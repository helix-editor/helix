use std::fmt::Display;
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::{slice, str};

use regex_cursor::engines::meta::Regex;

use crate::tree_sitter::Grammar;

macro_rules! bail {
    ($($args:tt)*) => {{
        return Err(format!($($args)*))
    }}
}

macro_rules! ensure {
    ($cond: expr, $($args:tt)*) => {{
        if !$cond {
            return Err(format!($($args)*))
        }
    }}
}

#[derive(Debug)]
enum TextPredicateCaptureKind {
    EqString(u32),
    EqCapture(u32),
    MatchString(Regex),
    AnyString(Box<[Box<str>]>),
}

struct TextPredicateCapture {
    capture_idx: u32,
    kind: TextPredicateCaptureKind,
    negated: bool,
    match_all: bool,
}

pub enum QueryData {}
pub struct Query {
    raw: NonNull<QueryData>,
    num_captures: u32,
}

impl Query {
    /// Create a new query from a string containing one or more S-expression
    /// patterns.
    ///
    /// The query is associated with a particular grammar, and can only be run
    /// on syntax nodes parsed with that grammar. References to Queries can be
    /// shared between multiple threads.
    pub fn new(grammar: Grammar, source: &str, path: impl AsRef<Path>) -> Result<Self, ParseError> {
        assert!(
            source.len() <= i32::MAX as usize,
            "TreeSitter queries must be smaller then 2 GiB (is {})",
            source.len() as f64 / 1024.0 / 1024.0 / 1024.0
        );
        let mut error_offset = 0u32;
        let mut error_kind = RawQueryError::None;
        let bytes = source.as_bytes();

        // Compile the query.
        let ptr = unsafe {
            ts_query_new(
                grammar,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut error_offset,
                &mut error_kind,
            )
        };

        let Some(raw) = ptr else {
            let offset = error_offset as usize;
            let error_word = || {
                source[offset..]
                    .chars()
                    .take_while(|&c| c.is_alphanumeric() || matches!(c, '_' | '-'))
                    .collect()
            };
            let err = match error_kind {
                RawQueryError::NodeType => {
                    let node: String = error_word();
                    ParseError::InvalidNodeType {
                        location: ParserErrorLocation::new(
                            source,
                            path.as_ref(),
                            offset,
                            node.chars().count(),
                        ),
                        node,
                    }
                }
                RawQueryError::Field => {
                    let field = error_word();
                    ParseError::InvalidFieldName {
                        location: ParserErrorLocation::new(
                            source,
                            path.as_ref(),
                            offset,
                            field.chars().count(),
                        ),
                        field,
                    }
                }
                RawQueryError::Capture => {
                    let capture = error_word();
                    ParseError::InvalidCaptureName {
                        location: ParserErrorLocation::new(
                            source,
                            path.as_ref(),
                            offset,
                            capture.chars().count(),
                        ),
                        capture,
                    }
                }
                RawQueryError::Syntax => ParseError::SyntaxError(ParserErrorLocation::new(
                    source,
                    path.as_ref(),
                    offset,
                    0,
                )),
                RawQueryError::Structure => ParseError::ImpossiblePattern(
                    ParserErrorLocation::new(source, path.as_ref(), offset, 0),
                ),
                RawQueryError::None => {
                    unreachable!("tree-sitter returned a null pointer but did not set an error")
                }
                RawQueryError::Language => unreachable!("should be handled at grammar load"),
            };
            return Err(err)
        };

        // I am not going to bother with safety comments here, all of these are
        // safe as long as TS is not buggy because raw is a properly constructed query
        let num_captures = unsafe { ts_query_capture_count(raw) };

        Ok(Query { raw, num_captures })
    }

    fn parse_predicates(&mut self) {
        let pattern_count = unsafe { ts_query_pattern_count(self.raw) };

        let mut text_predicates = Vec::with_capacity(pattern_count as usize);
        let mut property_predicates = Vec::with_capacity(pattern_count as usize);
        let mut property_settings = Vec::with_capacity(pattern_count as usize);
        let mut general_predicates = Vec::with_capacity(pattern_count as usize);

        for i in 0..pattern_count {}
    }

    fn parse_predicate(&self, pattern_index: u32) -> Result<(), String> {
        let mut text_predicates = Vec::new();
        let mut property_predicates = Vec::new();
        let mut property_settings = Vec::new();
        let mut general_predicates = Vec::new();
        for predicate in self.predicates(pattern_index) {
            let predicate = unsafe { Predicate::new(self, predicate)? };

            // Build a predicate for each of the known predicate function names.
            match predicate.operator_name {
                "eq?" | "not-eq?" | "any-eq?" | "any-not-eq?" => {
                    predicate.check_arg_count(2)?;
                    let capture_idx = predicate.get_arg(0, PredicateArg::Capture)?;
                    let (arg2, arg2_kind) = predicate.get_any_arg(1);

                    let negated = matches!(predicate.operator_name, "not-eq?" | "not-any-eq?");
                    let match_all = matches!(predicate.operator_name, "eq?" | "not-eq?");
                    let kind = match arg2_kind {
                        PredicateArg::Capture => TextPredicateCaptureKind::EqCapture(arg2),
                        PredicateArg::String => TextPredicateCaptureKind::EqString(arg2),
                    };
                    text_predicates.push(TextPredicateCapture {
                        capture_idx,
                        kind,
                        negated,
                        match_all,
                    });
                }

                "match?" | "not-match?" | "any-match?" | "any-not-match?" => {
                    predicate.check_arg_count(2)?;
                    let capture_idx = predicate.get_arg(0, PredicateArg::Capture)?;
                    let regex = predicate.get_str_arg(1)?;

                    let negated =
                        matches!(predicate.operator_name, "not-match?" | "any-not-match?");
                    let match_all = matches!(predicate.operator_name, "match?" | "not-match?");
                    let regex = match Regex::new(regex) {
                        Ok(regex) => regex,
                        Err(err) => bail!("invalid regex '{regex}', {err}"),
                    };
                    text_predicates.push(TextPredicateCapture {
                        capture_idx,
                        kind: TextPredicateCaptureKind::MatchString(regex),
                        negated,
                        match_all,
                    });
                }

                "set!" => property_settings.push(Self::parse_property(
                    row,
                    operator_name,
                    &capture_names,
                    &string_values,
                    &p[1..],
                )?),

                "is?" | "is-not?" => property_predicates.push((
                    Self::parse_property(
                        row,
                        operator_name,
                        &capture_names,
                        &string_values,
                        &p[1..],
                    )?,
                    operator_name == "is?",
                )),

                "any-of?" | "not-any-of?" => {
                    if p.len() < 2 {
                        return Err(predicate_error(row, format!(
                                "Wrong number of arguments to #any-of? predicate. Expected at least 1, got {}.",
                                p.len() - 1
                            )));
                    }
                    if p[1].type_ != TYPE_CAPTURE {
                        return Err(predicate_error(row, format!(
                                "First argument to #any-of? predicate must be a capture name. Got literal \"{}\".",
                                string_values[p[1].value_id as usize],
                            )));
                    }

                    let is_positive = operator_name == "any-of?";
                    let mut values = Vec::new();
                    for arg in &p[2..] {
                        if arg.type_ == TYPE_CAPTURE {
                            return Err(predicate_error(row, format!(
                                    "Arguments to #any-of? predicate must be literals. Got capture @{}.",
                                    capture_names[arg.value_id as usize],
                                )));
                        }
                        values.push(string_values[arg.value_id as usize]);
                    }
                    text_predicates.push(TextPredicateCapture::AnyString(
                        p[1].value_id,
                        values
                            .iter()
                            .map(|x| (*x).to_string().into())
                            .collect::<Vec<_>>()
                            .into(),
                        is_positive,
                    ));
                }

                _ => general_predicates.push(QueryPredicate {
                    operator: operator_name.to_string().into(),
                    args: p[1..]
                        .iter()
                        .map(|a| {
                            if a.type_ == TYPE_CAPTURE {
                                QueryPredicateArg::Capture(a.value_id)
                            } else {
                                QueryPredicateArg::String(
                                    string_values[a.value_id as usize].to_string().into(),
                                )
                            }
                        })
                        .collect(),
                }),
            }
        }

        text_predicates_vec.push(text_predicates.into());
        property_predicates_vec.push(property_predicates.into());
        property_settings_vec.push(property_settings.into());
        general_predicates_vec.push(general_predicates.into());
    }

    fn predicates<'a>(
        &'a self,
        pattern_index: u32,
    ) -> impl Iterator<Item = &'a [PredicateStep]> + 'a {
        let predicate_steps = unsafe {
            let mut len = 0u32;
            let raw_predicates = ts_query_predicates_for_pattern(self.raw, pattern_index, &mut len);
            (len != 0)
                .then(|| slice::from_raw_parts(raw_predicates, len as usize))
                .unwrap_or_default()
        };
        predicate_steps
            .split(|step| step.kind == PredicateStepKind::Done)
            .filter(|predicate| !predicate.is_empty())
    }

    /// Safety: value_idx must be a valid string id (in bounds) for this query and pattern_index
    unsafe fn get_pattern_string(&self, value_id: u32) -> &str {
        unsafe {
            let mut len = 0;
            let ptr = ts_query_string_value_for_id(self.raw, value_id, &mut len);
            let data = slice::from_raw_parts(ptr, len as usize);
            // safety: we only allow passing valid str(ings) as arguments to query::new
            // name is always a substring of that. Treesitter does proper utf8 segmentation
            // so any substrings it produces are codepoint aligned and therefore valid utf8
            str::from_utf8_unchecked(data)
        }
    }

    #[inline]
    pub fn capture_name(&self, capture_idx: u32) -> &str {
        // this one needs an assertions because the ts c api is inconsisent
        // and unsafe, other functions do have checks and would return null
        assert!(capture_idx <= self.num_captures, "invalid capture index");
        let mut length = 0;
        unsafe {
            let ptr = ts_query_capture_name_for_id(self.raw, capture_idx, &mut length);
            let name = slice::from_raw_parts(ptr, length as usize);
            // safety: we only allow passing valid str(ings) as arguments to query::new
            // name is always a substring of that. Treesitter does proper utf8 segmentation
            // so any substrings it produces are codepoint aligned and therefore valid utf8
            str::from_utf8_unchecked(name)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParserErrorLocation {
    pub path: PathBuf,
    /// at which line the error occured
    pub line: usize,
    /// at which codepoints/columns the errors starts in the line
    pub column: usize,
    /// how many codepoints/columns the error takes up
    pub len: usize,
    line_content: String,
}

impl ParserErrorLocation {
    pub fn new(source: &str, path: &Path, offset: usize, len: usize) -> ParserErrorLocation {
        let (line, line_content) = source[..offset]
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .enumerate()
            .last()
            .unwrap_or((0, ""));
        let column = line_content.chars().count();
        ParserErrorLocation {
            path: path.to_owned(),
            line,
            column,
            len,
            line_content: line_content.to_owned(),
        }
    }
}

impl Display for ParserErrorLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "  --> {}:{}:{}",
            self.path.display(),
            self.line,
            self.column
        )?;
        let line = self.line.to_string();
        let prefix = format_args!(" {:width$} |", "", width = line.len());
        writeln!(f, "{prefix}");
        writeln!(f, " {line} | {}", self.line_content)?;
        writeln!(
            f,
            "{prefix}{:width$}{:^<len$}",
            "",
            "^",
            width = self.column,
            len = self.len
        )?;
        writeln!(f, "{prefix}")
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParseError {
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("invalid query syntax\n{0}")]
    SyntaxError(ParserErrorLocation),
    #[error("invalid node type {node:?}\n{location}")]
    InvalidNodeType {
        node: String,
        location: ParserErrorLocation,
    },
    #[error("invalid field name {field:?}\n{location}")]
    InvalidFieldName {
        field: String,
        location: ParserErrorLocation,
    },
    #[error("invalid capture name {capture:?}\n{location}")]
    InvalidCaptureName {
        capture: String,
        location: ParserErrorLocation,
    },
    #[error("{message}\n{location}")]
    InvalidPredicate {
        message: String,
        location: ParserErrorLocation,
    },
    #[error("invalid predicate\n{0}")]
    ImpossiblePattern(ParserErrorLocation),
}

#[repr(C)]
enum RawQueryError {
    None = 0,
    Syntax = 1,
    NodeType = 2,
    Field = 3,
    Capture = 4,
    Structure = 5,
    Language = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PredicateStepKind {
    Done = 0,
    Capture = 1,
    String = 2,
}

#[repr(C)]
struct PredicateStep {
    kind: PredicateStepKind,
    value_id: u32,
}

struct Predicate<'a> {
    operator_name: &'a str,
    args: &'a [PredicateStep],
    query: &'a Query,
}

impl<'a> Predicate<'a> {
    unsafe fn new(
        query: &'a Query,
        predicate: &'a [PredicateStep],
    ) -> Result<Predicate<'a>, String> {
        ensure!(
            predicate[0].kind == PredicateStepKind::String,
            "expected predicate to start with a function name. Got @{}.",
            query.capture_name(predicate[0].value_id)
        );
        let operator_name = query.get_pattern_string(predicate[0].value_id);
        Ok(Predicate {
            operator_name,
            args: &predicate[1..],
            query,
        })
    }
    pub fn check_arg_count(&self, n: usize) -> Result<(), String> {
        ensure!(
            self.args.len() == n,
            "expected {n} arguments for #{}, got {}",
            self.operator_name,
            self.args.len()
        );
        Ok(())
    }

    pub fn get_arg(&self, i: usize, expect: PredicateArg) -> Result<u32, String> {
        let (val, actual) = self.get_any_arg(i);
        match (actual, expect) {
            (PredicateArg::Capture, PredicateArg::String) => bail!(
                "{i}. argument to #{} expected a capture, got literal {val:?}",
                self.operator_name
            ),
            (PredicateArg::String, PredicateArg::Capture) => bail!(
                "{i}. argument to #{} must be a literal, got capture @{val:?}",
                self.operator_name
            ),
            _ => (),
        };
        Ok(val)
    }
    pub fn get_str_arg(&self, i: usize) -> Result<&'a str, String> {
        let arg = self.get_arg(i, PredicateArg::String)?;
        unsafe { Ok(self.query.get_pattern_string(arg)) }
    }

    pub fn get_any_arg(&self, i: usize) -> (u32, PredicateArg) {
        match self.args[i].kind {
            PredicateStepKind::String => unsafe { (self.args[i].value_id, PredicateArg::String) },
            PredicateStepKind::Capture => (self.args[i].value_id, PredicateArg::Capture),
            PredicateStepKind::Done => unreachable!(),
        }
    }
}

enum PredicateArg {
    Capture,
    String,
}

extern "C" {
    /// Create a new query from a string containing one or more S-expression
    /// patterns. The query is associated with a particular language, and can
    /// only be run on syntax nodes parsed with that language. If all of the
    /// given patterns are valid, this returns a [`TSQuery`]. If a pattern is
    /// invalid, this returns `NULL`, and provides two pieces of information
    /// about the problem: 1. The byte offset of the error is written to
    /// the `error_offset` parameter. 2. The type of error is written to the
    /// `error_type` parameter.
    pub fn ts_query_new(
        grammar: Grammar,
        source: *const u8,
        source_len: u32,
        error_offset: &mut u32,
        error_type: &mut RawQueryError,
    ) -> Option<NonNull<QueryData>>;

    /// Delete a query, freeing all of the memory that it used.
    pub fn ts_query_delete(query: NonNull<QueryData>);

    /// Get the number of patterns, captures, or string literals in the query.
    pub fn ts_query_pattern_count(query: NonNull<QueryData>) -> u32;
    pub fn ts_query_capture_count(query: NonNull<QueryData>) -> u32;
    pub fn ts_query_string_count(query: NonNull<QueryData>) -> u32;

    /// Get the byte offset where the given pattern starts in the query's
    /// source. This can be useful when combining queries by concatenating their
    /// source code strings.
    pub fn ts_query_start_byte_for_pattern(query: NonNull<QueryData>, pattern_index: u32) -> u32;

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
    pub fn ts_query_predicates_for_pattern(
        query: NonNull<QueryData>,
        pattern_index: u32,
        step_count: &mut u32,
    ) -> *const PredicateStep;

    pub fn ts_query_is_pattern_rooted(query: NonNull<QueryData>, pattern_index: u32) -> bool;
    pub fn ts_query_is_pattern_non_local(query: NonNull<QueryData>, pattern_index: u32) -> bool;
    pub fn ts_query_is_pattern_guaranteed_at_step(
        query: NonNull<QueryData>,
        byte_offset: u32,
    ) -> bool;
    /// Get the name and length of one of the query's captures, or one of the
    /// query's string literals. Each capture and string is associated with a
    /// numeric id based on the order that it appeared in the query's source.
    pub fn ts_query_capture_name_for_id(
        query: NonNull<QueryData>,
        index: u32,
        length: &mut u32,
    ) -> *const u8;

    pub fn ts_query_string_value_for_id(
        self_: NonNull<QueryData>,
        index: u32,
        length: &mut u32,
    ) -> *const u8;
}
