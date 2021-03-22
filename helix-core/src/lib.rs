#![allow(unused)]
pub mod auto_pairs;
pub mod comment;
pub mod diagnostic;
pub mod graphemes;
mod history;
pub mod indent;
pub mod macros;
pub mod movement;
pub mod object;
mod position;
pub mod register;
pub mod search;
pub mod selection;
pub mod state;
pub mod syntax;
mod transaction;

pub(crate) fn find_first_non_whitespace_char2(line: RopeSlice) -> Option<usize> {
    // find first non-whitespace char
    for (start, ch) in line.chars().enumerate() {
        // TODO: could use memchr with chunks?
        if ch != ' ' && ch != '\t' && ch != '\n' {
            return Some(start);
        }
    }

    None
}
pub(crate) fn find_first_non_whitespace_char(text: RopeSlice, line_num: usize) -> Option<usize> {
    let line = text.line(line_num);
    let mut start = text.line_to_char(line_num);

    // find first non-whitespace char
    for ch in line.chars() {
        // TODO: could use memchr with chunks?
        if ch != ' ' && ch != '\t' && ch != '\n' {
            return Some(start);
        }
        start += 1;
    }

    None
}

pub use ropey::{Rope, RopeSlice};

pub use tendril::StrTendril as Tendril;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use position::{coords_at_pos, pos_at_coords, Position};
pub use selection::{Range, Selection};
pub use syntax::Syntax;

pub use diagnostic::Diagnostic;
pub use history::History;
pub use state::State;

pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
