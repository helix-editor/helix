#![allow(unused)]
pub mod auto_pairs;
pub mod comment;
pub mod diagnostic;
pub mod graphemes;
mod history;
pub mod indent;
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
pub fn runtime_dir() -> std::path::PathBuf {
    // runtime env var || dir where binary is located
    std::env::var("HELIX_RUNTIME")
        .map(|path| path.into())
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|path| path.to_path_buf()))
                .unwrap()
        })
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

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

pub use ropey::{Rope, RopeSlice};

pub use tendril::StrTendril as Tendril;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use position::{coords_at_pos, pos_at_coords, Position};
pub use selection::{Range, Selection};
pub use smallvec::SmallVec;
pub use syntax::Syntax;

pub use diagnostic::Diagnostic;
pub use history::History;
pub use state::State;

pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
