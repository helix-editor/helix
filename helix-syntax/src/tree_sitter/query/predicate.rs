use std::error::Error;
use std::ptr::NonNull;
use std::{fmt, slice};

use crate::tree_sitter::query::property::QueryProperty;
use crate::tree_sitter::query::{Capture, Pattern, Query, QueryData, QueryStr};

use regex_cursor::engines::meta::Regex;

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

pub(super) struct TextPredicate {
    capture: Capture,
    kind: TextPredicateKind,
    negated: bool,
    match_all: bool,
}

impl Query {
    pub(super) fn parse_pattern_predicates(
        &mut self,
        pattern_index: u32,
        mut custom_predicate: impl FnMut(Predicate) -> Result<(), InvalidPredicateError>,
    ) -> Result<Pattern, InvalidPredicateError> {
        let text_predicate_start = self.text_predicates.len() as u32;
        let property_start = self.properties.len() as u32;

        let predicate_steps = unsafe {
            let mut len = 0u32;
            let raw_predicates = ts_query_predicates_for_pattern(self.raw, pattern_index, &mut len);
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
                    let regex = predicate.str_arg(1)?.get(self);

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

                "set!" => self.properties.push(QueryProperty::parse(&predicate)?),

                "any-of?" | "not-any-of?" => {
                    predicate.check_min_arg_count(1)?;
                    let capture = predicate.capture_arg(0)?;
                    let negated = predicate.name() == "not-any-of?";
                    let values: Result<_, InvalidPredicateError> = (1..predicate.num_args())
                        .map(|i| predicate.str_arg(i))
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
                _ => custom_predicate(predicate)?,
            }
        }
        Ok(Pattern {
            text_predicates: text_predicate_start..self.text_predicates.len() as u32,
            properties: property_start..self.properties.len() as u32,
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

    pub fn str_arg(&self, i: usize) -> Result<QueryStr, InvalidPredicateError> {
        match self.arg(i) {
            PredicateArg::String(str) => Ok(str),
            PredicateArg::Capture(capture) => bail!(
                "{i}. argument to #{} must be a literal, got capture @{:?}",
                self.name(),
                capture.name(self.query)
            ),
        }
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
