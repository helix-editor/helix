pub mod grammar;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};

pub static RUNTIME_DIR: once_cell::sync::Lazy<std::path::PathBuf> =
    once_cell::sync::Lazy::new(runtime_dir);

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

pub fn config_file() -> std::path::PathBuf {
    config_dir().join("config.toml")
}

pub fn lang_config_file() -> std::path::PathBuf {
    config_dir().join("languages.toml")
}

pub fn log_file() -> std::path::PathBuf {
    cache_dir().join("helix.log")
}

/// Default bultin-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    toml::from_slice(include_bytes!("../../languages.toml"))
        .expect("Could not parse bultin-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let def_lang_conf = default_lang_config();

    // Checking if there is a language file where the executable is been run
    // This allows to have language definitions in each project
    let data = std::env::current_dir()
        .ok()
        .map(|path| path.join("languages.toml"))
        .and_then(|path| {
            if path.exists() {
                let data = std::fs::read(path);
                Some(data)
            } else {
                None
            }
        })
        .unwrap_or_else(|| std::fs::read(crate::config_dir().join("languages.toml")));

    let user_lang_conf = match data {
        Ok(raw) => {
            let value = toml::from_slice(&raw)?;
            merge_toml_values(def_lang_conf, value)
        }
        Err(_) => def_lang_conf,
    };

    Ok(user_lang_conf)
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
            .expect("Couldn't parse built-in languages config");
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
