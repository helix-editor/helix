#![allow(unused)]
pub mod graphemes;
mod history;
pub mod macros;
mod position;
pub mod selection;
pub mod state;
pub mod syntax;
mod transaction;

pub use ropey::{Rope, RopeSlice};

pub use tendril::StrTendril as Tendril;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use position::Position;
pub use selection::Range;
pub use selection::Selection;
pub use syntax::Syntax;

pub use history::History;
pub use state::State;

pub use transaction::{Assoc, Change, ChangeSet, Transaction};
