#![allow(unused)]
pub mod auto_pairs;
pub mod chars;
pub mod comment;
pub mod diagnostic;
pub mod graphemes;
pub mod history;
pub mod indent;
pub mod line_ending;
pub mod macros;
pub mod match_brackets;
pub mod movement;
pub mod object;
mod position;
pub mod register;
pub mod search;
pub mod selection;
mod state;
pub mod syntax;
mod transaction;

static RUNTIME_DIR: once_cell::sync::Lazy<std::path::PathBuf> =
    once_cell::sync::Lazy::new(runtime_dir);

pub fn find_first_non_whitespace_char(line: RopeSlice) -> Option<usize> {
    line.chars().position(|ch| !ch.is_whitespace())
}

pub fn find_root(root: Option<&str>) -> Option<std::path::PathBuf> {
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
        None => current_dir,
    };

    for ancestor in root.ancestors() {
        // TODO: also use defined roots if git isn't found
        if ancestor.join(".git").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

#[cfg(not(embed_runtime))]
fn runtime_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        return dir.into();
    }

    const RT_DIR: &str = "runtime";
    let conf_dir = config_dir().join(RT_DIR);
    if conf_dir.exists() {
        return conf_dir;
    }

    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // this is the directory of the crate being run by cargo, we need the workspace path so we take the parent
        return std::path::PathBuf::from(dir).parent().unwrap().join(RT_DIR);
    }

    // fallback to location of the executable being run
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|path| path.to_path_buf().join(RT_DIR)))
        .unwrap()
}

pub fn config_dir() -> std::path::PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("helix");
    path
}

pub fn cache_dir() -> std::path::PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.cache_dir();
    path.push("helix");
    path
}

pub use etcetera::home_dir;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

pub use ropey::{Rope, RopeSlice};

pub use tendril::StrTendril as Tendril;

pub use unicode_general_category::get_general_category;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use graphemes::RopeGraphemes;
pub use position::{coords_at_pos, pos_at_coords, Position};
pub use selection::{Range, Selection};
pub use smallvec::SmallVec;
pub use syntax::Syntax;

pub use diagnostic::Diagnostic;
pub use state::State;

pub use line_ending::{
    auto_detect_line_ending, get_line_ending, rope_slice_to_line_ending, LineEnding,
    DEFAULT_LINE_ENDING, line_end
};
pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
