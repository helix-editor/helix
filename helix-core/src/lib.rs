pub use encoding_rs as encoding;

pub mod auto_pairs;
pub mod chars;
pub mod comment;
pub mod config;
pub mod diagnostic;
pub mod diff;
pub mod graphemes;
pub mod history;
pub mod increment;
pub mod indent;
pub mod line_ending;
pub mod macros;
pub mod match_brackets;
pub mod movement;
pub mod object;
pub mod path;
mod position;
pub mod register;
pub mod search;
pub mod selection;
pub mod shellwords;
mod state;
pub mod surround;
pub mod syntax;
pub mod test;
pub mod textobject;
mod transaction;
pub mod wrap;

pub mod unicode {
    pub use unicode_general_category as category;
    pub use unicode_segmentation as segmentation;
    pub use unicode_width as width;
}

pub fn find_first_non_whitespace_char(line: RopeSlice) -> Option<usize> {
    line.chars().position(|ch| !ch.is_whitespace())
}

/// Find project root.
///
/// Order of detection:
/// * Top-most folder containing a root marker in current git repository
/// * Git repository root if no marker detected
/// * Top-most folder containing a root marker if not git repository detected
/// * Current working directory as fallback
pub fn find_root(root: Option<&str>, root_markers: &[String]) -> Option<std::path::PathBuf> {
    helix_loader::find_root_impl(root, root_markers)
        .first()
        .cloned()
}

pub use ropey::{str_utils, Rope, RopeBuilder, RopeSlice};

// pub use tendril::StrTendril as Tendril;
pub use smartstring::SmartString;

pub type Tendril = SmartString<smartstring::LazyCompact>;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use graphemes::RopeGraphemes;
pub use position::{
    coords_at_pos, pos_at_coords, pos_at_visual_coords, visual_coords_at_pos, Position,
};
pub use selection::{Range, Selection};
pub use smallvec::{smallvec, SmallVec};
pub use syntax::Syntax;

pub use diagnostic::Diagnostic;
pub use state::State;

pub use line_ending::{LineEnding, DEFAULT_LINE_ENDING};
pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
