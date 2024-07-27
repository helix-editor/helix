mod grammar;
mod parser;
pub mod query;
mod query_cursor;
mod query_match;
mod ropey;
mod syntax_tree;
mod syntax_tree_node;

use std::ops;

pub use grammar::Grammar;
pub use parser::{Parser, ParserInputRaw};
pub use query::{Capture, Pattern, Query, QueryStr};
pub use query_cursor::{InactiveQueryCursor, MatchedNode, MatchedNodeIdx, QueryCursor, QueryMatch};
pub use ropey::RopeTsInput;
pub use syntax_tree::{InputEdit, SyntaxTree};
pub use syntax_tree_node::SyntaxTreeNode;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub row: u32,
    pub col: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Range {
    pub start_point: Point,
    pub end_point: Point,
    pub start_byte: u32,
    pub end_byte: u32,
}

pub trait TsInput {
    type Cursor: regex_cursor::Cursor;
    fn cursor_at(&mut self, offset: usize) -> &mut Self::Cursor;
    fn eq(&mut self, range1: ops::Range<usize>, range2: ops::Range<usize>) -> bool;
}

pub trait IntoTsInput {
    type TsInput: TsInput;
    fn into_ts_input(self) -> Self::TsInput;
}
