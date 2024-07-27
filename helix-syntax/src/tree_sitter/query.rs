use std::fmt::{self, Display};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::{slice, str};

use crate::tree_sitter::query::predicate::{InvalidPredicateError, Predicate, TextPredicate};
use crate::tree_sitter::Grammar;

mod predicate;
mod property;

pub enum UserPredicate<'a> {
    IsPropertySet {
        negate: bool,
        key: &'a str,
        val: Option<&'a str>,
    },
    SetProperty {
        key: &'a str,
        val: Option<&'a str>,
    },
    Other(Predicate<'a>),
}

impl Display for UserPredicate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UserPredicate::IsPropertySet { negate, key, val } => {
                let predicate = if negate { "is-not?" } else { "is?" };
                write!(f, " ({predicate} {key} {})", val.unwrap_or(""))
            }
            UserPredicate::SetProperty { key, val } => {
                write!(f, "(set! {key} {})", val.unwrap_or(""))
            }
            UserPredicate::Other(ref predicate) => {
                write!(f, "{}", predicate.name())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pattern(pub(crate) u32);

impl Pattern {
    pub const SENTINEL: Pattern = Pattern(u32::MAX);
    pub fn idx(&self) -> usize {
        self.0 as usize
    }
}

pub enum QueryData {}

#[derive(Debug)]
pub(super) struct PatternData {
    text_predicates: Range<u32>,
}

#[derive(Debug)]
pub struct Query {
    pub(crate) raw: NonNull<QueryData>,
    num_captures: u32,
    num_strings: u32,
    text_predicates: Vec<TextPredicate>,
    patterns: Box<[PatternData]>,
}

impl Query {
    /// Create a new query from a string containing one or more S-expression
    /// patterns.
    ///
    /// The query is associated with a particular grammar, and can only be run
    /// on syntax nodes parsed with that grammar. References to Queries can be
    /// shared between multiple threads.
    pub fn new(
        grammar: Grammar,
        source: &str,
        path: impl AsRef<Path>,
        mut custom_predicate: impl FnMut(Pattern, UserPredicate) -> Result<(), InvalidPredicateError>,
    ) -> Result<Self, ParseError> {
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
            return Err(err);
        };

        // I am not going to bother with safety comments here, all of these are
        // safe as long as TS is not buggy because raw is a properly constructed query
        let num_captures = unsafe { ts_query_capture_count(raw) };
        let num_strings = unsafe { ts_query_string_count(raw) };
        let num_patterns = unsafe { ts_query_pattern_count(raw) };

        let mut query = Query {
            raw,
            num_captures,
            num_strings,
            text_predicates: Vec::new(),
            patterns: Box::default(),
        };
        let patterns: Result<_, ParseError> = (0..num_patterns)
            .map(|pattern| {
                query
                    .parse_pattern_predicates(Pattern(pattern), &mut custom_predicate)
                    .map_err(|err| ParseError::InvalidPredicate {
                        message: err.msg.into(),
                        location: ParserErrorLocation::new(
                            source,
                            path.as_ref(),
                            unsafe { ts_query_start_byte_for_pattern(query.raw, pattern) as usize },
                            0,
                        ),
                    })
            })
            .collect();
        query.patterns = patterns?;
        Ok(query)
    }

    #[inline]
    fn get_string(&self, str: QueryStr) -> &str {
        let value_id = str.0;
        // need an assertions because the ts c api does not do bounds check
        assert!(value_id <= self.num_captures, "invalid value index");
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
    pub fn capture_name(&self, capture_idx: Capture) -> &str {
        let capture_idx = capture_idx.0;
        // need an assertions because the ts c api does not do bounds check
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

    #[inline]
    pub fn captures(&self) -> impl ExactSizeIterator<Item = (Capture, &str)> {
        (0..self.num_captures).map(|cap| (Capture(cap), self.capture_name(Capture(cap))))
    }

    #[inline]
    pub fn num_captures(&self) -> u32 {
        self.num_captures
    }

    #[inline]
    pub fn get_capture(&self, capture_name: &str) -> Option<Capture> {
        for capture in 0..self.num_captures {
            if capture_name == self.capture_name(Capture(capture)) {
                return Some(Capture(capture));
            }
        }
        None
    }

    pub(crate) fn pattern_text_predicates(&self, pattern_idx: u16) -> &[TextPredicate] {
        let range = self.patterns[pattern_idx as usize].text_predicates.clone();
        &self.text_predicates[range.start as usize..range.end as usize]
    }

    /// Get the byte offset where the given pattern starts in the query's
    /// source.
    #[doc(alias = "ts_query_start_byte_for_pattern")]
    #[must_use]
    pub fn start_byte_for_pattern(&self, pattern: Pattern) -> usize {
        assert!(
            pattern.0 < self.text_predicates.len() as u32,
            "Pattern index is {pattern_index} but the pattern count is {}",
            self.text_predicates.len(),
        );
        unsafe { ts_query_start_byte_for_pattern(self.raw, pattern.0) as usize }
    }

    /// Get the number of patterns in the query.
    #[must_use]
    pub fn pattern_count(&self) -> usize {
        unsafe { ts_query_pattern_count(self.raw) as usize }
    }
    /// Get the number of patterns in the query.
    #[must_use]
    pub fn patterns(&self) -> impl ExactSizeIterator<Item = Pattern> {
        (0..self.pattern_count() as u32).map(Pattern)
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        unsafe { ts_query_delete(self.raw) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Capture(u32);

impl Capture {
    pub fn name(self, query: &Query) -> &str {
        query.capture_name(self)
    }
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// A reference to a string stroed in a query
#[derive(Clone, Copy, Debug)]
pub struct QueryStr(u32);

impl QueryStr {
    pub fn get(self, query: &Query) -> &str {
        query.get_string(self)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "  --> {}:{}:{}",
            self.path.display(),
            self.line,
            self.column
        )?;
        let line = self.line.to_string();
        let prefix = format!(" {:width$} |", "", width = line.len());
        writeln!(f, "{prefix}")?;
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

extern "C" {
    /// Create a new query from a string containing one or more S-expression
    /// patterns. The query is associated with a particular language, and can
    /// only be run on syntax nodes parsed with that language. If all of the
    /// given patterns are valid, this returns a [`TSQuery`]. If a pattern is
    /// invalid, this returns `NULL`, and provides two pieces of information
    /// about the problem: 1. The byte offset of the error is written to
    /// the `error_offset` parameter. 2. The type of error is written to the
    /// `error_type` parameter.
    fn ts_query_new(
        grammar: Grammar,
        source: *const u8,
        source_len: u32,
        error_offset: &mut u32,
        error_type: &mut RawQueryError,
    ) -> Option<NonNull<QueryData>>;

    /// Delete a query, freeing all of the memory that it used.
    fn ts_query_delete(query: NonNull<QueryData>);

    /// Get the number of patterns, captures, or string literals in the query.
    fn ts_query_pattern_count(query: NonNull<QueryData>) -> u32;
    fn ts_query_capture_count(query: NonNull<QueryData>) -> u32;
    fn ts_query_string_count(query: NonNull<QueryData>) -> u32;

    /// Get the byte offset where the given pattern starts in the query's
    /// source. This can be useful when combining queries by concatenating their
    /// source code strings.
    fn ts_query_start_byte_for_pattern(query: NonNull<QueryData>, pattern_index: u32) -> u32;

    // fn ts_query_is_pattern_rooted(query: NonNull<QueryData>, pattern_index: u32) -> bool;
    // fn ts_query_is_pattern_non_local(query: NonNull<QueryData>, pattern_index: u32) -> bool;
    // fn ts_query_is_pattern_guaranteed_at_step(query: NonNull<QueryData>, byte_offset: u32) -> bool;
    /// Get the name and length of one of the query's captures, or one of the
    /// query's string literals. Each capture and string is associated with a
    /// numeric id based on the order that it appeared in the query's source.
    fn ts_query_capture_name_for_id(
        query: NonNull<QueryData>,
        index: u32,
        length: &mut u32,
    ) -> *const u8;

    fn ts_query_string_value_for_id(
        self_: NonNull<QueryData>,
        index: u32,
        length: &mut u32,
    ) -> *const u8;
}
