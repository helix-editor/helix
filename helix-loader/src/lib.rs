pub mod config;
pub mod grammar;

use helix_stdx::{env::current_working_dir, path};
use std::collections::HashSet;
use toml::Value;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};
use std::path::{Path, PathBuf};

pub const VERSION_AND_GIT_HASH: &str = env!("VERSION_AND_GIT_HASH");

static RUNTIME_DIRS: once_cell::sync::Lazy<Vec<PathBuf>> =
    once_cell::sync::Lazy::new(prioritize_runtime_dirs);

static CONFIG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();

static LOG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();

pub fn initialize_config_file(specified_file: Option<PathBuf>) {
    let config_file = specified_file.unwrap_or_else(default_config_file);
    ensure_parent_dir(&config_file);
    CONFIG_FILE.set(config_file).ok();
}

pub fn initialize_log_file(specified_file: Option<PathBuf>) {
    let log_file = specified_file.unwrap_or_else(default_log_file);
    ensure_parent_dir(&log_file);
    LOG_FILE.set(log_file).ok();
}

/// A list of runtime directories from highest to lowest priority
///
/// The priority is:
///
/// 1. sibling directory to `CARGO_MANIFEST_DIR` (if environment variable is set)
/// 2. subdirectory of user config directory (always included)
/// 3. `HELIX_RUNTIME` (if environment variable is set)
/// 4. `HELIX_DEFAULT_RUNTIME` (if environment variable is set *at build time*)
/// 5. subdirectory of path to helix executable (always included)
///
/// Postcondition: returns at least two paths (they might not exist).
fn prioritize_runtime_dirs() -> Vec<PathBuf> {
    const RT_DIR: &str = "runtime";
    // Adding higher priority first
    let mut rt_dirs = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // this is the directory of the crate being run by cargo, we need the workspace path so we take the parent
        let path = PathBuf::from(dir).parent().unwrap().join(RT_DIR);
        log::debug!("runtime dir: {}", path.to_string_lossy());
        rt_dirs.push(path);
    }

    let conf_rt_dir = config_dir().join(RT_DIR);
    rt_dirs.push(conf_rt_dir);

    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        let dir = path::expand_tilde(Path::new(&dir));
        rt_dirs.push(path::normalize(dir));
    }

    // If this variable is set during build time, it will always be included
    // in the lookup list. This allows downstream packagers to set a fallback
    // directory to a location that is conventional on their distro so that they
    // need not resort to a wrapper script or a global environment variable.
    if let Some(dir) = std::option_env!("HELIX_DEFAULT_RUNTIME") {
        rt_dirs.push(dir.into());
    }

    // fallback to location of the executable being run
    // canonicalize the path in case the executable is symlinked
    let exe_rt_dir = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .and_then(|path| path.parent().map(|path| path.to_path_buf().join(RT_DIR)))
        .unwrap();
    rt_dirs.push(exe_rt_dir);
    rt_dirs
}

/// Runtime directories ordered from highest to lowest priority
///
/// All directories should be checked when looking for files.
///
/// Postcondition: returns at least one path (it might not exist).
pub fn runtime_dirs() -> &'static [PathBuf] {
    &RUNTIME_DIRS
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise None.
fn find_runtime_file(rel_path: &Path) -> Option<PathBuf> {
    RUNTIME_DIRS.iter().find_map(|rt_dir| {
        let path = rt_dir.join(rel_path);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    })
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise the path to the final attempt
/// that failed.
pub fn runtime_file(rel_path: impl AsRef<Path>) -> PathBuf {
    find_runtime_file(rel_path.as_ref()).unwrap_or_else(|| {
        RUNTIME_DIRS
            .last()
            .map(|dir| dir.join(rel_path))
            .unwrap_or_default()
    })
}

pub fn config_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("helix");
    path
}

pub fn cache_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the cache directory!");
    let mut path = strategy.cache_dir();
    path.push("helix");
    path
}

pub fn config_file() -> PathBuf {
    CONFIG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn log_file() -> PathBuf {
    LOG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn workspace_config_file() -> PathBuf {
    find_workspace().0.join(".helix").join("config.toml")
}

pub fn lang_config_file() -> PathBuf {
    config_dir().join("languages.toml")
}

pub fn default_log_file() -> PathBuf {
    cache_dir().join("helix.log")
}

/// Merge two TOML documents, merging values from `incoming` onto `base`
///
/// `merge_depth` sets the nesting depth up to which values are merged instead
/// of overridden.
///
/// When a table exists in both `base` and `incoming`, the merged table consists of
/// all keys in `base`'s table unioned with all keys in `incoming` with the values
/// of `incoming` being merged recursively onto values of `base`.
///
/// `crate::merge_toml_values(a, b, 3)` combines, for example:
///
/// b:
/// ```toml
/// [[language]]
/// name = "toml"
/// language-server = { command = "taplo", args = ["lsp", "stdio"] }
/// ```
/// a:
/// ```toml
/// [[language]]
/// language-server = { command = "/usr/bin/taplo" }
/// ```
///
/// into:
/// ```toml
/// [[language]]
/// name = "toml"
/// language-server = { command = "/usr/bin/taplo" }
/// ```
///
/// thus it overrides the third depth-level of b with values of a if they exist,
/// but otherwise merges their values
pub fn merge_toml_values(
    base: toml::Value,
    incoming: toml::Value,
    merge_depth: usize,
) -> toml::Value {
    use toml::Value;

    fn get_name(v: &Value) -> Option<&str> {
        v.get("name").and_then(Value::as_str)
    }

    match (base, incoming) {
        (Value::Array(mut base_items), Value::Array(incoming_items)) => {
            // Attempt merge with list modifiers if any are present
            let needs_list_modifier_merge = incoming_items.iter().any(|val| {
                if let Value::String(s) = val {
                    s == "..." || s.starts_with('!')
                } else {
                    false
                }
            }) || incoming_items.iter().all(|val| val.is_str());

            if needs_list_modifier_merge {
                Value::Array(merge_list_with_modifiers(base_items, incoming_items))
            } else if merge_depth > 0 {
                // Existing logic for merging arrays of tables based on 'name' key
                base_items.reserve(incoming_items.len());
                for incoming_value in incoming_items {
                    let existing_value_option = get_name(&incoming_value)
                        .and_then(|incoming_name| {
                            base_items
                                .iter()
                                .position(|v| get_name(v) == Some(incoming_name))
                        })
                        .map(|base_pos| base_items.remove(base_pos));

                    let merged_value = match existing_value_option {
                        Some(base_value) => {
                            merge_toml_values(base_value, incoming_value, merge_depth - 1)
                        }
                        None => incoming_value,
                    };
                    base_items.push(merged_value);
                }
                Value::Array(base_items)
            } else {
                Value::Array(incoming_items)
            }
        }
        (Value::Table(mut base_map), Value::Table(incoming_map)) => {
            if merge_depth > 0 {
                for (incoming_name, incoming_value) in incoming_map {
                    match base_map.remove(&incoming_name) {
                        Some(base_value) => {
                            let merged_value =
                                merge_toml_values(base_value, incoming_value, merge_depth - 1);
                            base_map.insert(incoming_name, merged_value);
                        }
                        None => {
                            base_map.insert(incoming_name, incoming_value);
                        }
                    }
                }
                Value::Table(base_map)
            } else {
                Value::Table(incoming_map)
            }
        }
        // Catch everything else we didn't handle, and use the right value
        (_, value) => value,
    }
}

fn merge_list_with_modifiers(base_list: Vec<Value>, incoming_list: Vec<Value>) -> Vec<Value> {
    let mut final_list = Vec::new();
    let mut excluded_items = HashSet::new();
    let mut append_at_idx: Option<usize> = None;

    for r_val in incoming_list {
        if let Value::String(s) = r_val {
            if s == "..." {
                if append_at_idx.is_some() {
                    log::warn!(
                        "Multiple '...' found in list. Only the first one will be considered."
                    );
                    continue;
                }
                append_at_idx = Some(final_list.len());
            } else if let Some(name) = s.strip_prefix('!') {
                excluded_items.insert(name.to_string());
            } else {
                final_list.push(Value::String(s));
            }
        } else {
            // Non-string values are passed through directly,
            // but modifiers only apply to strings.
            final_list.push(r_val);
        }
    }

    // combine
    if let Some(idx) = append_at_idx {
        // remove excluded and already present items from base list
        let mut processed_base_list = Vec::new();
        for l_val in base_list {
            if let Value::String(s) = l_val {
                if !excluded_items.contains(&s)
                    && !final_list
                        .iter()
                        .any(|item| item.is_str() && item.as_str().is_some_and(|v| v == &s))
                {
                    processed_base_list.push(Value::String(s));
                }
            } else {
                processed_base_list.push(l_val);
            }
        }

        final_list.splice(idx..idx, processed_base_list);
        final_list
    } else {
        // follow previous behavior when no ... is specified
        // final_list here is just left
        final_list
    }
}

/// Finds the current workspace folder.
/// Used as a ceiling dir for LSP root resolution, the filepicker and potentially as a future filewatching root
///
/// This function starts searching the FS upward from the CWD
/// and returns the first directory that contains either `.git`, `.svn`, `.jj` or `.helix`.
/// If no workspace was found returns (CWD, true).
/// Otherwise (workspace, false) is returned
pub fn find_workspace() -> (PathBuf, bool) {
    let current_dir = current_working_dir();
    find_workspace_in(current_dir)
}

pub fn find_workspace_in(dir: impl AsRef<Path>) -> (PathBuf, bool) {
    let dir = dir.as_ref();
    for ancestor in dir.ancestors() {
        if ancestor.join(".git").exists()
            || ancestor.join(".svn").exists()
            || ancestor.join(".jj").exists()
            || ancestor.join(".helix").exists()
        {
            return (ancestor.to_owned(), false);
        }
    }

    (dir.to_owned(), true)
}

fn default_config_file() -> PathBuf {
    config_dir().join("config.toml")
}

fn ensure_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).ok();
        }
    }
}

#[cfg(test)]
mod merge_toml_tests {
    use std::str;

    use super::merge_toml_values;
    use toml::Value;

    #[test]
    fn language_toml_map_merges() {
        const USER: &str = r#"
        [[language]]
        name = "nix"
        test = "bbb"
        indent = { tab-width = 4, unit = "    ", test = "aaa" }
        "#;

        let base = include_bytes!("../../languages.toml");
        let base = str::from_utf8(base).expect("Couldn't parse built-in languages config");
        let base: Value = toml::from_str(base).expect("Couldn't parse built-in languages config");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(base, user, 3);
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

    #[test]
    fn language_toml_nested_array_merges() {
        const USER: &str = r#"
        [[language]]
        name = "typescript"
        language-server = { command = "deno", args = ["lsp"] }
        "#;

        let base = include_bytes!("../../languages.toml");
        let base = str::from_utf8(base).expect("Couldn't parse built-in languages config");
        let base: Value = toml::from_str(base).expect("Couldn't parse built-in languages config");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(base, user, 3);
        let languages = merged.get("language").unwrap().as_array().unwrap();
        let ts = languages
            .iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "typescript")
            .unwrap();
        assert_eq!(
            ts.get("language-server")
                .unwrap()
                .get("args")
                .unwrap()
                .as_array()
                .unwrap(),
            &vec![Value::String("lsp".into())]
        )
    }

    // --- Integration style tests for merge_toml_values with language-servers modifiers ---
    #[test]
    fn test_merge_toml_values_language_servers_exclude() {
        let base_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["rust-analyzer", "taplo"]
        "#;
        let user_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["!taplo", "custom-lsp"]
        "#;
        let base: Value = toml::from_str(base_toml).unwrap();
        let user: Value = toml::from_str(user_toml).unwrap();

        let merged = super::merge_toml_values(base, user, 3);
        let rust_lang = merged
            .get("language")
            .unwrap()
            .as_array()
            .unwrap()
            .into_iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "rust")
            .unwrap();
        let servers = rust_lang
            .get("language-servers")
            .unwrap()
            .as_array()
            .unwrap();
        let server_names: Vec<String> = servers
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(server_names, vec!["custom-lsp"]);
    }

    #[test]
    fn test_merge_toml_values_language_servers_append() {
        let base_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["rust-analyzer", "taplo"]
        "#;
        let user_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["custom-lsp-before", "...", "custom-lsp-after"]
        "#;
        let base: Value = toml::from_str(base_toml).unwrap();
        let user: Value = toml::from_str(user_toml).unwrap();

        let merged = super::merge_toml_values(base, user, 3);
        let rust_lang = merged
            .get("language")
            .unwrap()
            .as_array()
            .unwrap()
            .into_iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "rust")
            .unwrap();
        let servers = rust_lang
            .get("language-servers")
            .unwrap()
            .as_array()
            .unwrap();
        let server_names: Vec<String> = servers
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(
            server_names,
            vec![
                "custom-lsp-before",
                "rust-analyzer",
                "taplo",
                "custom-lsp-after"
            ]
        );
    }

    #[test]
    fn test_merge_toml_values_language_servers_append_implicit() {
        let base_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["rust-analyzer", "taplo"]
        "#;
        let user_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["custom-lsp"]
        "#;
        let base: Value = toml::from_str(base_toml).unwrap();
        let user: Value = toml::from_str(user_toml).unwrap();

        let merged = super::merge_toml_values(base, user, 3);
        let rust_lang = merged
            .get("language")
            .unwrap()
            .as_array()
            .unwrap()
            .into_iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "rust")
            .unwrap();
        let servers = rust_lang
            .get("language-servers")
            .unwrap()
            .as_array()
            .unwrap();
        let server_names: Vec<String> = servers
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(server_names, vec!["custom-lsp"]);
    }

    #[test]
    fn test_merge_toml_values_language_servers_exclude_and_append() {
        let base_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["rust-analyzer", "taplo", "clippy"]
        "#;
        let user_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["custom-lsp", "!taplo", "...", "!clippy", "another-custom"]
        "#;
        let base: Value = toml::from_str(base_toml).unwrap();
        let user: Value = toml::from_str(user_toml).unwrap();

        let merged = super::merge_toml_values(base, user, 3);
        let rust_lang = merged
            .get("language")
            .unwrap()
            .as_array()
            .unwrap()
            .into_iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "rust")
            .unwrap();
        let servers = rust_lang
            .get("language-servers")
            .unwrap()
            .as_array()
            .unwrap();
        let server_names: Vec<String> = servers
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(
            server_names,
            vec!["custom-lsp", "rust-analyzer", "another-custom"]
        );
    }

    #[test]
    fn test_merge_toml_values_language_servers_deduplication() {
        let base_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["rust-analyzer", "taplo"]
        "#;
        let user_toml = r#"
        [[language]]
        name = "rust"
        language-servers = ["custom-lsp", "rust-analyzer", "..."]
        "#;
        let base: Value = toml::from_str(base_toml).unwrap();
        let user: Value = toml::from_str(user_toml).unwrap();

        let merged = super::merge_toml_values(base, user, 3);
        let rust_lang = merged
            .get("language")
            .unwrap()
            .as_array()
            .unwrap()
            .into_iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "rust")
            .unwrap();
        let servers = rust_lang
            .get("language-servers")
            .unwrap()
            .as_array()
            .unwrap();
        let server_names: Vec<String> = servers
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert_eq!(server_names, vec!["custom-lsp", "rust-analyzer", "taplo"]);
    }
}
