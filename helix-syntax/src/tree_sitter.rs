mod grammar;
mod parser;
mod query;
mod ropey;
mod syntax_tree;
mod syntax_tree_node;

pub use grammar::Grammar;
pub use parser::{Parser, ParserInputRaw};
pub use syntax_tree::{InputEdit, SyntaxTree};
pub use syntax_tree_node::SyntaxTreeNode;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub row: u32,
    pub column: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Range {
    pub start_point: Point,
    pub end_point: Point,
    pub start_byte: u32,
    pub end_byte: u32,
}
