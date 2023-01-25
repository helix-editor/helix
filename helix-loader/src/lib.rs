pub mod grammar;
pub mod repo_paths;
pub mod ts_probe;

use anyhow::Error;
use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};
use std::path::{Path, PathBuf};

pub const VERSION_AND_GIT_HASH: &str = env!("VERSION_AND_GIT_HASH");

static LOG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();
static CONFIG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();
static RUNTIME_DIRS: once_cell::sync::Lazy<Vec<PathBuf>> =
    once_cell::sync::Lazy::new(set_runtime_dirs);

pub fn log_file() -> PathBuf {
    match LOG_FILE.get() {
        Some(log_path) => log_path.to_path_buf(),
        None => {
            setup_log_file(None);
            log_file()
        }
    }
}

// TODO: allow env var override
pub fn cache_dir() -> PathBuf {
    choose_base_strategy()
        .expect("Unable to determine system base directory specification!")
        .cache_dir()
        .join("helix")
}

pub fn config_file() -> PathBuf {
    match CONFIG_FILE.get() {
        Some(config_path) => config_path.to_path_buf(),
        None => {
            setup_config_file(None);
            config_file()
        }
    }
}

// TODO: allow env var override
pub fn user_config_dir() -> PathBuf {
    choose_base_strategy()
        .expect("Unable to determine system base directory specification!")
        .config_dir()
        .join("helix")
}

/// Returns a non-existent path relative to the local executable if none are found.
pub fn get_runtime_file(relative_path: &Path) -> PathBuf {
    get_runtime_dirs()
        .iter()
        .find_map(|runtime_dir| {
            let path = runtime_dir.join(relative_path);
            match path.exists() {
                true => Some(path),
                false => None,
            }
        })
        .unwrap_or_else(|| {
            get_runtime_dirs()
                .last()
                .expect("Path to local executable.")
                .join(relative_path)
        })
}

pub fn get_first_runtime_dir() -> &'static PathBuf {
    get_runtime_dirs()
        .first()
        .expect("should return at least one directory")
}

pub fn get_runtime_dirs() -> &'static [PathBuf] {
    &RUNTIME_DIRS
}

pub fn user_lang_config_file() -> PathBuf {
    user_config_dir().join("languages.toml")
}

pub fn setup_config_file(specified_file: Option<PathBuf>) {
    let config_file = specified_file.unwrap_or_else(|| {
        let config_dir = user_config_dir();
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).ok();
        }
        config_dir.join("config.toml")
    });
    CONFIG_FILE.set(config_file).unwrap();
}

pub fn setup_log_file(specified_file: Option<PathBuf>) {
    let log_file = specified_file.unwrap_or_else(|| {
        let log_dir = cache_dir();
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir).ok();
        }
        log_dir.join("helix.log")
    });
    LOG_FILE.set(log_file).ok();
}

/// Runtime directory location priority:
/// 1. Sibling directory to `CARGO_MANIFEST_DIR`, given that environment variable is set. (Often done by cargo)
// TODO: XDG_RUNTIME_DIR
/// 2. Under user config directory, given that it exists.
/// 3. `HELIX_RUNTIME`, given that the environment variable is set.
/// 4. Under path to helix executable, always included. However, it might not exist.
fn set_runtime_dirs() -> Vec<PathBuf> {
    let mut runtime_dirs = Vec::new();
    const RUNTIME_DIR_NAME: &str = "runtime";
    if std::env::var("CARGO_MANIFEST_DIR").is_ok() {
        let path = repo_paths::project_root().join(RUNTIME_DIR_NAME);
        log::debug!("runtime dir: {}", path.to_string_lossy());
        runtime_dirs.push(path);
    }

    let conf_dir = user_config_dir().join(RUNTIME_DIR_NAME);
    if conf_dir.exists() {
        runtime_dirs.push(conf_dir);
    }

    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        runtime_dirs.push(dir.into());
    }

    // canonicalize the path in case the executable is symlinked
    runtime_dirs.push(
        std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::canonicalize(path).ok())
            .and_then(|path| {
                path.parent()
                    .map(|path| path.to_path_buf().join(RUNTIME_DIR_NAME))
            })
            .unwrap(),
    );

    runtime_dirs
}

pub fn merged_config() -> Result<toml::Value, Error> {
    let config_paths: Vec<PathBuf> = local_config_dirs()
        .into_iter()
        .chain([user_config_dir()].into_iter())
        .map(|path| path.join("config.toml"))
        .collect();
    merge_toml_by_config_paths(config_paths)
}

/// Searces for language.toml in config path (user config) and in 'helix' directories
/// in opened git repository (local). Merge order:
/// local -> user config -> default/system
pub fn merged_lang_config() -> Result<toml::Value, Error> {
    let config_paths: Vec<PathBuf> = local_config_dirs()
        .into_iter()
        .chain([user_config_dir(), repo_paths::default_config_dir()].into_iter())
        .map(|path| path.join("languages.toml"))
        .collect();
    merge_toml_by_config_paths(config_paths)
}

pub fn local_config_dirs() -> Vec<PathBuf> {
    let current_dir = std::env::current_dir().expect("Unable to determine current directory.");
    let mut directories = Vec::new();
    for ancestor in current_dir.ancestors() {
        let potential_dir = ancestor.to_path_buf().join(".helix");
        if potential_dir.is_dir() {
            directories.push(potential_dir);
        }
        if ancestor.join(".git").exists() {
            break;
        }
    }
    log::debug!("Located local configuration folders: {:?}", directories);
    directories
}

fn merge_toml_by_config_paths(config_paths: Vec<PathBuf>) -> Result<toml::Value, Error> {
    let mut configs: Vec<toml::Value> = Vec::with_capacity(config_paths.len());
    for config_path in config_paths {
        if config_path.exists() {
            let config_string = std::fs::read_to_string(&config_path)?;
            let config = toml::from_str(&config_string)?;
            configs.push(config);
        }
    }

    Ok(configs
        .into_iter()
        .reduce(|a, b| merge_toml_values(b, a, 3))
        .expect("Supplied config paths should point to at least one valid config."))
}

/// Merge two TOML documents, merging values from `right` onto `left`
///
/// When an array exists in both `left` and `right`, `right`'s array is
/// used. When a table exists in both `left` and `right`, the merged table
/// consists of all keys in `left`'s table unioned with all keys in `right`
/// with the values of `right` being merged recursively onto values of
/// `left`.
///
/// `merge_toplevel_arrays` controls whether a top-level array in the TOML
/// document is merged instead of overridden. This is useful for TOML
/// documents that use a top-level array of values like the `languages.toml`,
/// where one usually wants to override or add to the array instead of
/// replacing it altogether.
///
/// For example:
///
/// left:
///   [[language]]
///   name = "toml"
///   language-server = { command = "taplo", args = ["lsp", "stdio"] }
///
/// right:
///   [[language]]
///   language-server = { command = "/usr/bin/taplo" }
///
/// result:
///   [[language]]
///   name = "toml"
///   language-server = { command = "/usr/bin/taplo" }
pub fn merge_toml_values(left: toml::Value, right: toml::Value, merge_depth: usize) -> toml::Value {
    use toml::Value;

    fn get_name(v: &Value) -> Option<&str> {
        v.get("name").and_then(Value::as_str)
    }

    match (left, right) {
        (Value::Array(mut left_items), Value::Array(right_items)) => {
            // The top-level arrays should be merged but nested arrays should
            // act as overrides. For the `languages.toml` config, this means
            // that you can specify a sub-set of languages in an overriding
            // `languages.toml` but that nested arrays like Language Server
            // arguments are replaced instead of merged.
            if merge_depth > 0 {
                left_items.reserve(right_items.len());
                for rvalue in right_items {
                    let lvalue = get_name(&rvalue)
                        .and_then(|rname| {
                            left_items.iter().position(|v| get_name(v) == Some(rname))
                        })
                        .map(|lpos| left_items.remove(lpos));
                    let mvalue = match lvalue {
                        Some(lvalue) => merge_toml_values(lvalue, rvalue, merge_depth - 1),
                        None => rvalue,
                    };
                    left_items.push(mvalue);
                }
                Value::Array(left_items)
            } else {
                Value::Array(right_items)
            }
        }
        (Value::Table(mut left_map), Value::Table(right_map)) => {
            if merge_depth > 0 {
                for (rname, rvalue) in right_map {
                    match left_map.remove(&rname) {
                        Some(lvalue) => {
                            let merged_value = merge_toml_values(lvalue, rvalue, merge_depth - 1);
                            left_map.insert(rname, merged_value);
                        }
                        None => {
                            left_map.insert(rname, rvalue);
                        }
                    }
                }
                Value::Table(left_map)
            } else {
                Value::Table(right_map)
            }
        }
        // Catch everything else we didn't handle, and use the right value
        (_, value) => value,
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

        // NOTE: Exact duplicate of helix_core::LanguageConfigurations::default()
        let default: Value = toml::from_str(
            &std::fs::read_to_string(crate::repo_paths::default_lang_configs()).unwrap(),
        )
        .expect("Failed to deserialize built-in languages.toml");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(default, user, 3);
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

        // NOTE: Exact duplicate of helix_core::LanguageConfigurations::default()
        let default: Value = toml::from_str(
            &std::fs::read_to_string(crate::repo_paths::default_lang_configs()).unwrap(),
        )
        .expect("Failed to deserialize built-in languages.toml");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(default, user, 3);
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
}
