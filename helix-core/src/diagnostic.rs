#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Diagnostic {
    pub range: Range,
    pub line: usize,
    pub message: String,
    pub severity: Option<Severity>,
}
