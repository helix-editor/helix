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
/// Order of detection is based on `match-closest-root` option:
/// * If true, search for top-most folder containing either a root marker or a git repository root
/// * If false, search for closest folder containing a root marker or a git a repository root
/// * Use current working directory as fallback
pub fn find_root(
    root: Option<&str>,
    root_markers: &[String],
    match_closest_root: bool,
) -> std::path::PathBuf {
    let current_dir = std::env::current_dir().expect("unable to determine current directory");

    let root = match root {
        Some(root) => {
            let root = std::path::Path::new(root);
            if root.is_absolute() {
                root.to_path_buf()
            } else {
                current_dir.join(root)
            }
        }
        None => current_dir.clone(),
    };

    let mut found_marker = None;

    for ancestor in root.ancestors() {
        if root_markers
            .iter()
            .any(|marker| ancestor.join(marker).exists())
        {
            found_marker = Some(ancestor);

            // If `match_closest_root` is set to true,
            // stop looking for top level matches.
            if match_closest_root {
                break;
            }
        }

        if ancestor.join(".git").is_dir() {
            // Marker is repo root if no root marker was detected yet
            if found_marker.is_none() {
                found_marker = Some(ancestor);
            }
            // Don't go higher than repo if we're in one
            break;
        }
    }

    // Return the found marker or the current_dir as fallback
    found_marker.map_or(current_dir, |a| a.to_path_buf())
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

pub use line_ending::{LineEnding, DEFAULT_LINE_ENDING};
pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
