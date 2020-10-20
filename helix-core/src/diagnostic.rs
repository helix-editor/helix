use crate::Range;

pub struct Diagnostic {
    pub range: (usize, usize),
    pub line: usize,
    pub message: String,
}
