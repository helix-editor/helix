pub mod config;
pub mod grammar;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

pub static RUNTIME_DIR: once_cell::sync::Lazy<std::path::PathBuf> =
    once_cell::sync::Lazy::new(runtime_dir);

pub fn runtime_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        return dir.into();
    }

    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // this is the directory of the crate being run by cargo, we need the workspace path so we take the parent
        let path = std::path::PathBuf::from(dir).parent().unwrap().join(RT_DIR);
        log::debug!("runtime dir: {}", path.to_string_lossy());
        return path;
    }

    const RT_DIR: &str = "runtime";
    let conf_dir = config_dir().join(RT_DIR);
    if conf_dir.exists() {
        return conf_dir;
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

pub fn local_config_dirs() -> Vec<std::path::PathBuf> {
    let directories = find_root_impl(None, &[".helix".to_string()])
        .into_iter()
        .map(|path| path.join(".helix"))
        .collect();
    log::debug!("Located configuration folders: {:?}", directories);
    directories
}

pub fn cache_dir() -> std::path::PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.cache_dir();
    path.push("helix");
    path
}

pub fn config_file() -> std::path::PathBuf {
    config_dir().join("config.toml")
}

pub fn lang_config_file() -> std::path::PathBuf {
    config_dir().join("languages.toml")
}

pub fn log_file() -> std::path::PathBuf {
    cache_dir().join("helix.log")
}

pub fn find_root_impl(root: Option<&str>, root_markers: &[String]) -> Vec<std::path::PathBuf> {
    let current_dir = std::env::current_dir().expect("unable to determine current directory");
    let mut directories = Vec::new();

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
        // don't go higher than repo
        if ancestor.join(".git").is_dir() {
            // Use workspace if detected from marker
            directories.push(ancestor.to_path_buf());
            break;
        } else if root_markers
            .iter()
            .any(|marker| ancestor.join(marker).exists())
        {
            directories.push(ancestor.to_path_buf());
        }
    }
    directories
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

        let base: Value = toml::from_slice(include_bytes!("../../languages.toml"))
            .expect("Couldn't parse built-in languages config");
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

        let base: Value = toml::from_slice(include_bytes!("../../languages.toml"))
            .expect("Couldn't parse built-in languages config");
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
}
