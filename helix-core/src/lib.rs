pub mod auto_pairs;
pub mod chars;
pub mod comment;
pub mod diagnostic;
pub mod diff;
pub mod graphemes;
pub mod history;
pub mod indent;
pub mod line_ending;
pub mod macros;
pub mod match_brackets;
pub mod movement;
pub mod numbers;
pub mod object;
pub mod path;
mod position;
pub mod register;
pub mod search;
pub mod selection;
mod state;
pub mod surround;
pub mod syntax;
pub mod textobject;
mod transaction;

pub mod unicode {
    pub use unicode_general_category as category;
    pub use unicode_segmentation as segmentation;
    pub use unicode_width as width;
}

static RUNTIME_DIR: once_cell::sync::Lazy<std::path::PathBuf> =
    once_cell::sync::Lazy::new(runtime_dir);

pub fn find_first_non_whitespace_char(line: RopeSlice) -> Option<usize> {
    line.chars().position(|ch| !ch.is_whitespace())
}

/// Find `.git` root.
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

pub fn runtime_dir() -> std::path::PathBuf {
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

// right overrides left
pub fn merge_toml_values(left: toml::Value, right: toml::Value) -> toml::Value {
    use toml::Value;

    fn get_name(v: &Value) -> Option<&str> {
        v.get("name").and_then(Value::as_str)
    }

    match (left, right) {
        (Value::Array(mut left_items), Value::Array(right_items)) => {
            left_items.reserve(right_items.len());
            for rvalue in right_items {
                let lvalue = get_name(&rvalue)
                    .and_then(|rname| left_items.iter().position(|v| get_name(v) == Some(rname)))
                    .map(|lpos| left_items.remove(lpos));
                let mvalue = match lvalue {
                    Some(lvalue) => merge_toml_values(lvalue, rvalue),
                    None => rvalue,
                };
                left_items.push(mvalue);
            }
            Value::Array(left_items)
        }
        (Value::Table(mut left_map), Value::Table(right_map)) => {
            for (rname, rvalue) in right_map {
                match left_map.remove(&rname) {
                    Some(lvalue) => {
                        let merged_value = merge_toml_values(lvalue, rvalue);
                        left_map.insert(rname, merged_value);
                    }
                    None => {
                        left_map.insert(rname, rvalue);
                    }
                }
            }
            Value::Table(left_map)
        }
        // Catch everything else we didn't handle, and use the right value
        (_, value) => value,
    }
}

#[cfg(test)]
mod merge_toml_tests {
    use super::merge_toml_values;

    #[test]
    fn language_tomls() {
        use toml::Value;

        const USER: &str = "
        [[language]]
        name = \"nix\"
        test = \"bbb\"
        indent = { tab-width = 4, unit = \"    \", test = \"aaa\" }
        ";

        let base: Value = toml::from_slice(include_bytes!("../../languages.toml"))
            .expect("Couldn't parse built-in langauges config");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(base, user);
        let languages = merged.get("language").unwrap().as_array().unwrap();
        let nix = languages
            .iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "nix")
            .unwrap();
        let nix_indent = nix.get("indent").unwrap();

        // We changed tab-width and unit in indent so check them if they are the new values
        assert_eq!(
            nix_indent.get("tab-width").unwrap().as_integer().unwrap(),
            4
        );
        assert_eq!(nix_indent.get("unit").unwrap().as_str().unwrap(), "    ");
        // We added a new keys, so check them
        assert_eq!(nix.get("test").unwrap().as_str().unwrap(), "bbb");
        assert_eq!(nix_indent.get("test").unwrap().as_str().unwrap(), "aaa");
        // We didn't change comment-token so it should be same
        assert_eq!(nix.get("comment-token").unwrap().as_str().unwrap(), "#");
    }
}

pub use etcetera::home_dir;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

pub use ropey::{Rope, RopeBuilder, RopeSlice};

pub use tendril::StrTendril as Tendril;

#[doc(inline)]
pub use {regex, tree_sitter};

pub use graphemes::RopeGraphemes;
pub use position::{coords_at_pos, pos_at_coords, visual_coords_at_pos, Position};
pub use selection::{Range, Selection};
pub use smallvec::SmallVec;
pub use syntax::Syntax;

pub use diagnostic::Diagnostic;
pub use state::State;

pub use line_ending::{LineEnding, DEFAULT_LINE_ENDING};
pub use transaction::{Assoc, Change, ChangeSet, Operation, Transaction};
