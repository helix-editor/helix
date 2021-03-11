use crate::Range;

pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct Diagnostic {
    pub range: (usize, usize),
    pub line: usize,
    pub message: String,
    pub severity: Option<Severity>,
}
