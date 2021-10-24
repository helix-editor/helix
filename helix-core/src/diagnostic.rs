//! LSP diagnostic utility types.

/// Describes the severity level of a [`Diagnostic`].
#[derive(Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// A range of `char`s within the text.
#[derive(Debug)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

/// Corresponds to [`lsp_types::Diagnostic`](https://docs.rs/lsp-types/0.91.0/lsp_types/struct.Diagnostic.html)
#[derive(Debug)]
pub struct Diagnostic {
    pub range: Range,
    pub line: usize,
    pub message: String,
    pub severity: Option<Severity>,
}
