use std::error::Error;
use std::iter::zip;
use std::ops::Range;
use std::ptr::NonNull;
use std::{fmt, slice};

use crate::tree_sitter::query::property::QueryProperty;
use crate::tree_sitter::query::{
    Capture, Pattern, PatternData, Query, QueryData, QueryStr, UserPredicate,
};
use crate::tree_sitter::query_cursor::MatchedNode;
use crate::tree_sitter::TsInput;

use regex_cursor::engines::meta::Regex;
use regex_cursor::Cursor;

macro_rules! bail {
    ($($args:tt)*) => {{
        return Err(InvalidPredicateError {msg: format!($($args)*).into() })
    }}
}

macro_rules! ensure {
    ($cond: expr, $($args:tt)*) => {{
        if !$cond {
            return Err(InvalidPredicateError { msg: format!($($args)*).into() })
        }
    }}
}

#[derive(Debug)]
pub(super) enum TextPredicateKind {
    EqString(QueryStr),
    EqCapture(Capture),
    MatchString(Regex),
    AnyString(Box<[QueryStr]>),
}

#[derive(Debug)]
pub(crate) struct TextPredicate {
    capture: Capture,
    kind: TextPredicateKind,
    negated: bool,
    match_all: bool,
}

fn input_matches_str<I: TsInput>(str: &str, range: Range<usize>, input: &mut I) -> bool {
    if str.len() != range.len() {
        return false;
    }
    let mut str = str.as_bytes();
    let cursor = input.cursor_at(range.start);
    let start_in_chunk = range.start - cursor.offset();
    if range.end - cursor.offset() <= cursor.chunk().len() {
        // hotpath
        return &cursor.chunk()[start_in_chunk..range.end - cursor.offset()] == str;
    }
    if cursor.chunk()[start_in_chunk..] != str[..cursor.chunk().len() - start_in_chunk] {
        return false;
    }
    str = &str[..cursor.chunk().len() - start_in_chunk];
    while cursor.advance() {
        if str.len() <= cursor.chunk().len() {
            return &cursor.chunk()[..range.end - cursor.offset()] == str;
        }
        if &str[..cursor.chunk().len()] != cursor.chunk() {
            return false;
        }
        str = &str[cursor.chunk().len()..]
    }
    // buggy cursor/invalid range
    false
}

fn inputs_match<I: TsInput>(str: &str, range: Range<usize>, input: &mut I) -> bool {
    if str.len() != range.len() {
        return false;
    }
    let mut str = str.as_bytes();
    let cursor = input.cursor_at(range.start);
    let start_in_chunk = range.start - cursor.offset();
    if range.end - cursor.offset() <= cursor.chunk().len() {
        // hotpath
        return &cursor.chunk()[start_in_chunk..range.end - cursor.offset()] == str;
    }
    if cursor.chunk()[start_in_chunk..] != str[..cursor.chunk().len() - start_in_chunk] {
        return false;
    }
    str = &str[..cursor.chunk().len() - start_in_chunk];
    while cursor.advance() {
        if str.len() <= cursor.chunk().len() {
            return &cursor.chunk()[..range.end - cursor.offset()] == str;
        }
        if &str[..cursor.chunk().len()] != cursor.chunk() {
            return false;
        }
        str = &str[cursor.chunk().len()..]
    }
    // buggy cursor/invalid range
    false
}

impl TextPredicate {
    /// handlers match_all and negated
    fn satisfied_helper(&self, mut nodes: impl Iterator<Item = bool>) -> bool {
        if self.match_all {
            nodes.all(|matched| matched != self.negated)
        } else {
            nodes.any(|matched| matched != self.negated)
        }
    }

    pub fn satsified<I: TsInput>(
        &self,
        input: &mut I,
        matched_nodes: &[MatchedNode],
        query: &Query,
    ) -> bool {
        let mut capture_nodes = matched_nodes
            .iter()
            .filter(|matched_node| matched_node.capture == self.capture);
        match self.kind {
            TextPredicateKind::EqString(str) => self.satisfied_helper(capture_nodes.map(|node| {
                let range = node.syntax_node.byte_range();
                input_matches_str(query.get_string(str), range.clone(), input)
            })),
            TextPredicateKind::EqCapture(other_capture) => {
                let mut other_nodes = matched_nodes
                    .iter()
                    .filter(|matched_node| matched_node.capture == other_capture);

                let res = self.satisfied_helper(zip(&mut capture_nodes, &mut other_nodes).map(
                    |(node1, node2)| {
                        let range1 = node1.syntax_node.byte_range();
                        let range2 = node2.syntax_node.byte_range();
                        input.eq(range1, range2)
                    },
                ));
                let consumed_all = capture_nodes.next().is_none() && other_nodes.next().is_none();
                res && (!self.match_all || consumed_all)
            }
            TextPredicateKind::MatchString(ref regex) => {
                self.satisfied_helper(capture_nodes.map(|node| {
                    let range = node.syntax_node.byte_range();
                    let input = regex_cursor::Input::new(input.cursor_at(range.start)).range(range);
                    regex.is_match(input)
                }))
            }
            TextPredicateKind::AnyString(ref strings) => {
                let strings = strings.iter().map(|&str| query.get_string(str));
                self.satisfied_helper(capture_nodes.map(|node| {
                    let range = node.syntax_node.byte_range();
                    strings
                        .clone()
                        .filter(|str| str.len() == range.len())
                        .any(|str| input_matches_str(str, range.clone(), input))
                }))
            }
        }
    }
}

impl Query {
    pub(super) fn parse_pattern_predicates(
        &mut self,
        pattern: Pattern,
        mut custom_predicate: impl FnMut(Pattern, UserPredicate) -> Result<(), InvalidPredicateError>,
    ) -> Result<PatternData, InvalidPredicateError> {
        let text_predicate_start = self.text_predicates.len() as u32;

        let predicate_steps = unsafe {
            let mut len = 0u32;
            let raw_predicates = ts_query_predicates_for_pattern(self.raw, pattern.0, &mut len);
            (len != 0)
                .then(|| slice::from_raw_parts(raw_predicates, len as usize))
                .unwrap_or_default()
        };
        let predicates = predicate_steps
            .split(|step| step.kind == PredicateStepKind::Done)
            .filter(|predicate| !predicate.is_empty());

        for predicate in predicates {
            let predicate = unsafe { Predicate::new(self, predicate)? };

            match predicate.name() {
                "eq?" | "not-eq?" | "any-eq?" | "any-not-eq?" => {
                    predicate.check_arg_count(2)?;
                    let capture_idx = predicate.capture_arg(0)?;
                    let arg2 = predicate.arg(1);

                    let negated = matches!(predicate.name(), "not-eq?" | "not-any-eq?");
                    let match_all = matches!(predicate.name(), "eq?" | "not-eq?");
                    let kind = match arg2 {
                        PredicateArg::Capture(capture) => TextPredicateKind::EqCapture(capture),
                        PredicateArg::String(str) => TextPredicateKind::EqString(str),
                    };
                    self.text_predicates.push(TextPredicate {
                        capture: capture_idx,
                        kind,
                        negated,
                        match_all,
                    });
                }

                "match?" | "not-match?" | "any-match?" | "any-not-match?" => {
                    predicate.check_arg_count(2)?;
                    let capture_idx = predicate.capture_arg(0)?;
                    let regex = predicate.query_str_arg(1)?.get(self);

                    let negated = matches!(predicate.name(), "not-match?" | "any-not-match?");
                    let match_all = matches!(predicate.name(), "match?" | "not-match?");
                    let regex = match Regex::new(regex) {
                        Ok(regex) => regex,
                        Err(err) => bail!("invalid regex '{regex}', {err}"),
                    };
                    self.text_predicates.push(TextPredicate {
                        capture: capture_idx,
                        kind: TextPredicateKind::MatchString(regex),
                        negated,
                        match_all,
                    });
                }

                "set!" => {
                    let property = QueryProperty::parse(&predicate)?;
                    custom_predicate(
                        pattern,
                        UserPredicate::SetProperty {
                            key: property.key.get(&self),
                            val: property.val.map(|val| val.get(&self)),
                        },
                    )?
                }
                "is-not?" | "is?" => {
                    let property = QueryProperty::parse(&predicate)?;
                    custom_predicate(
                        pattern,
                        UserPredicate::IsPropertySet {
                            negate: predicate.name() == "is-not?",
                            key: property.key.get(&self),
                            val: property.val.map(|val| val.get(&self)),
                        },
                    )?
                }

                "any-of?" | "not-any-of?" => {
                    predicate.check_min_arg_count(1)?;
                    let capture = predicate.capture_arg(0)?;
                    let negated = predicate.name() == "not-any-of?";
                    let values: Result<_, InvalidPredicateError> = (1..predicate.num_args())
                        .map(|i| predicate.query_str_arg(i))
                        .collect();
                    self.text_predicates.push(TextPredicate {
                        capture,
                        kind: TextPredicateKind::AnyString(values?),
                        negated,
                        match_all: false,
                    });
                }

                // is and is-not are better handeled as custom predicates since interpreting is context dependent
                // "is?" => property_predicates.push((QueryProperty::parse(&predicate), false)),
                // "is-not?" => property_predicates.push((QueryProperty::parse(&predicate), true)),
                _ => custom_predicate(pattern, UserPredicate::Other(predicate))?,
            }
        }
        Ok(PatternData {
            text_predicates: text_predicate_start..self.text_predicates.len() as u32,
        })
    }
}

pub enum PredicateArg {
    Capture(Capture),
    String(QueryStr),
}

pub struct Predicate<'a> {
    pub name: QueryStr,
    args: &'a [PredicateStep],
    query: &'a Query,
}

impl<'a> Predicate<'a> {
    unsafe fn new(
        query: &'a Query,
        predicate: &'a [PredicateStep],
    ) -> Result<Predicate<'a>, InvalidPredicateError> {
        ensure!(
            predicate[0].kind == PredicateStepKind::String,
            "expected predicate to start with a function name. Got @{}.",
            Capture(predicate[0].value_id).name(query)
        );
        let operator_name = QueryStr(predicate[0].value_id);
        Ok(Predicate {
            name: operator_name,
            args: &predicate[1..],
            query,
        })
    }

    pub fn name(&self) -> &str {
        self.name.get(self.query)
    }

    pub fn check_arg_count(&self, n: usize) -> Result<(), InvalidPredicateError> {
        ensure!(
            self.args.len() == n,
            "expected {n} arguments for #{}, got {}",
            self.name(),
            self.args.len()
        );
        Ok(())
    }

    pub fn check_min_arg_count(&self, n: usize) -> Result<(), InvalidPredicateError> {
        ensure!(
            n <= self.args.len(),
            "expected at least {n} arguments for #{}, got {}",
            self.name(),
            self.args.len()
        );
        Ok(())
    }

    pub fn check_max_arg_count(&self, n: usize) -> Result<(), InvalidPredicateError> {
        ensure!(
            self.args.len() <= n,
            "expected at most {n} arguments for #{}, got {}",
            self.name(),
            self.args.len()
        );
        Ok(())
    }

    pub fn query_str_arg(&self, i: usize) -> Result<QueryStr, InvalidPredicateError> {
        match self.arg(i) {
            PredicateArg::String(str) => Ok(str),
            PredicateArg::Capture(capture) => bail!(
                "{i}. argument to #{} must be a literal, got capture @{:?}",
                self.name(),
                capture.name(self.query)
            ),
        }
    }

    pub fn str_arg(&self, i: usize) -> Result<&str, InvalidPredicateError> {
        Ok(self.query_str_arg(i)?.get(self.query))
    }

    pub fn num_args(&self) -> usize {
        self.args.len()
    }

    pub fn capture_arg(&self, i: usize) -> Result<Capture, InvalidPredicateError> {
        match self.arg(i) {
            PredicateArg::Capture(capture) => Ok(capture),
            PredicateArg::String(str) => bail!(
                "{i}. argument to #{} expected a capture, got literal {:?}",
                self.name(),
                str.get(self.query)
            ),
        }
    }

    pub fn arg(&self, i: usize) -> PredicateArg {
        self.args[i].try_into().unwrap()
    }

    pub fn args(&self) -> impl Iterator<Item = PredicateArg> + '_ {
        self.args.iter().map(|&arg| arg.try_into().unwrap())
    }
}

#[derive(Debug)]
pub struct InvalidPredicateError {
    pub(super) msg: Box<str>,
}

impl From<String> for InvalidPredicateError {
    fn from(value: String) -> Self {
        InvalidPredicateError {
            msg: value.into_boxed_str(),
        }
    }
}

impl<'a> From<&'a str> for InvalidPredicateError {
    fn from(value: &'a str) -> Self {
        InvalidPredicateError { msg: value.into() }
    }
}

impl fmt::Display for InvalidPredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for InvalidPredicateError {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PredicateStepKind {
    Done = 0,
    Capture = 1,
    String = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PredicateStep {
    kind: PredicateStepKind,
    value_id: u32,
}

impl TryFrom<PredicateStep> for PredicateArg {
    type Error = ();

    fn try_from(step: PredicateStep) -> Result<Self, Self::Error> {
        match step.kind {
            PredicateStepKind::String => Ok(PredicateArg::String(QueryStr(step.value_id))),
            PredicateStepKind::Capture => Ok(PredicateArg::Capture(Capture(step.value_id))),
            PredicateStepKind::Done => Err(()),
        }
    }
}

extern "C" {
    /// Get all of the predicates for the given pattern in the query. The
    /// predicates are represented as a single array of steps. There are three
    /// types of steps in this array, which correspond to the three legal values
    /// for the `type` field: - `TSQueryPredicateStepTypeCapture` - Steps with
    /// this type represent names    of captures. Their `value_id` can be used
    /// with the   [`ts_query_capture_name_for_id`] function to obtain the name
    /// of the capture. - `TSQueryPredicateStepTypeString` - Steps with this
    /// type represent literal    strings. Their `value_id` can be used with the
    /// [`ts_query_string_value_for_id`] function to obtain their string value.
    /// - `TSQueryPredicateStepTypeDone` - Steps with this type are *sentinels*
    /// that represent the end of an individual predicate. If a pattern has two
    /// predicates, then there will be two steps with this `type` in the array.
    fn ts_query_predicates_for_pattern(
        query: NonNull<QueryData>,
        pattern_index: u32,
        step_count: &mut u32,
    ) -> *const PredicateStep;

}
