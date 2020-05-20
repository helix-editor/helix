use crate::Position;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

// range traversal iters
