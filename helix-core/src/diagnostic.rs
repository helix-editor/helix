pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct Range {
    pub start: usize,
    pub end: usize,
}
pub struct Diagnostic {
    pub range: Range,
    pub line: usize,
    pub message: String,
    pub severity: Option<Severity>,
}
