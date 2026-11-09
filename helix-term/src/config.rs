use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie};
use helix_view::{document::Mode, theme};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
use std::path::PathBuf;
use toml::de::Error as TomlError;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub theme: Option<theme::Config>,
    pub keys: HashMap<Mode, KeyTrie>,
    pub editor: helix_view::editor::Config,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigRaw {
    pub theme: Option<theme::Config>,
    pub keys: Option<HashMap<Mode, KeyTrie>>,
    pub editor: Option<toml::Value>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            theme: None,
            keys: keymap::default(),
            editor: helix_view::editor::Config::default(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigLoadError {
    BadConfig(TomlError),
    BadConfigFile { path: PathBuf, error: TomlError },
    Error(IOError),
}

impl Default for ConfigLoadError {
    fn default() -> Self {
        ConfigLoadError::Error(IOError::new(std::io::ErrorKind::NotFound, "place holder"))
    }
}

impl Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::BadConfig(err) => err.fmt(f),
            ConfigLoadError::BadConfigFile { path, error } => {
                write!(f, "{}: {}", path.display(), error)
            }
            ConfigLoadError::Error(err) => err.fmt(f),
        }
    }
}

impl Config {
    pub fn load(
        global: Result<&String, ConfigLoadError>,
        local: Result<String, ConfigLoadError>,
    ) -> Result<Config, ConfigLoadError> {
        let global_config: Result<ConfigRaw, ConfigLoadError> =
            global.and_then(|file| toml::from_str(file).map_err(ConfigLoadError::BadConfig));
        let local_config: Result<ConfigRaw, ConfigLoadError> =
            local.and_then(|file| toml::from_str(&file).map_err(ConfigLoadError::BadConfig));
        let res = match (global_config, local_config) {
            (Ok(global), Ok(local)) => {
                let mut keys = keymap::default();
                if let Some(global_keys) = global.keys {
                    merge_keys(&mut keys, global_keys)
                }
                if let Some(local_keys) = local.keys {
                    merge_keys(&mut keys, local_keys)
                }

                let editor = match (global.editor, local.editor) {
                    (None, None) => helix_view::editor::Config::default(),
                    (None, Some(val)) | (Some(val), None) => {
                        val.try_into().map_err(ConfigLoadError::BadConfig)?
                    }
                    (Some(global), Some(local)) => Self::merge_config_values(global, local)
                        .try_into()
                        .map_err(ConfigLoadError::BadConfig)?,
                };

                Config {
                    theme: local.theme.or(global.theme),
                    keys,
                    editor,
                }
            }
            // if any configs are invalid return that first
            (_, Err(ConfigLoadError::BadConfig(err)))
            | (Err(ConfigLoadError::BadConfig(err)), _) => {
                return Err(ConfigLoadError::BadConfig(err))
            }
            (Ok(config), Err(_)) | (Err(_), Ok(config)) => {
                let mut keys = keymap::default();
                if let Some(keymap) = config.keys {
                    merge_keys(&mut keys, keymap);
                }
                Config {
                    theme: config.theme,
                    keys,
                    editor: config.editor.map_or_else(
                        || Ok(helix_view::editor::Config::default()),
                        |val| val.try_into().map_err(ConfigLoadError::BadConfig),
                    )?,
                }
            }

            // these are just two io errors return the one for the global config
            (Err(err), Err(_)) => return Err(err),
        };

        Ok(res)
    }

    fn load_merged_global_string() -> Result<String, ConfigLoadError> {
        Self::load_merged_global_value().and_then(|val| {
            toml::to_string(&val)
                .map_err(|err| ConfigLoadError::Error(IOError::other(err.to_string())))
        })
    }

    /// Deep-merge tables for `config.d`. Unlike `merge_toml_values`, nested
    /// keymap prefixes (`[keys.normal.space]`) and statusline mode tables are
    /// merged rather than replaced wholesale; arrays/scalars are replaced.
    fn merge_config_values(left: toml::Value, right: toml::Value) -> toml::Value {
        use toml::Value;

        match (left, right) {
            (Value::Table(mut left), Value::Table(right)) => {
                for (key, right_value) in right {
                    let merged = match left.remove(&key) {
                        Some(left_value) => Self::merge_config_values(left_value, right_value),
                        None => right_value,
                    };
                    left.insert(key, merged);
                }
                Value::Table(left)
            }
            (_, right_value) => right_value,
        }
    }

    fn load_merged_global_value() -> Result<toml::Value, ConfigLoadError> {
        let config_file = helix_loader::config_file();
        let config_dir = helix_loader::config_dir();

        // Load `config.d` next to the main file. A custom `--config` gets its
        // own sibling directory (matching normal `-c` isolation); the default
        // config also gets `~/.config/helix/config.d`. Deduplicate so a custom
        // path inside the default dir is not scanned twice.
        let mut module_dirs: Vec<PathBuf> = Vec::new();
        if let Some(parent) = config_file.parent() {
            module_dirs.push(parent.join("config.d"));
        }
        let default_config = config_dir.join("config.toml");
        let default_d = config_dir.join("config.d");
        if config_file == default_config && !module_dirs.iter().any(|p| p == &default_d) {
            module_dirs.push(default_d);
        }

        let mut merged = toml::Value::Table(Default::default());
        let mut found_any = false;

        // Main config.toml first (if it exists)
        match fs::read_to_string(&config_file) {
            Ok(content) => {
                let val: toml::Value =
                    toml::from_str(&content).map_err(|error| ConfigLoadError::BadConfigFile {
                        path: config_file.clone(),
                        error,
                    })?;
                merged = Self::merge_config_values(merged, val);
                found_any = true;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(ConfigLoadError::Error(err)),
        }

        // Then each `*.toml` in `config.d`, sorted for determinism
        for dir in module_dirs {
            if !dir.is_dir() {
                continue;
            }
            let mut entries: Vec<PathBuf> = match fs::read_dir(&dir) {
                Ok(rd) => rd
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.is_file() && p.extension().map(|e| e == "toml").unwrap_or(false))
                    .collect(),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(ConfigLoadError::Error(err)),
            };
            entries.sort();
            for path in entries {
                let content = fs::read_to_string(&path).map_err(ConfigLoadError::Error)?;
                let val: toml::Value =
                    toml::from_str(&content).map_err(|error| ConfigLoadError::BadConfigFile {
                        path: path.clone(),
                        error,
                    })?;
                merged = Self::merge_config_values(merged, val);
                found_any = true;
            }
        }

        if !found_any {
            return Err(ConfigLoadError::Error(IOError::new(
                std::io::ErrorKind::NotFound,
                format!("no config found at {} or config.d", config_file.display()),
            )));
        }

        Ok(merged)
    }

    pub fn load_default() -> Result<Config, ConfigLoadError> {
        let global_config = Self::load_merged_global_string()?;
        let local_config = fs::read_to_string(helix_loader::workspace_config_file())
            .map_err(ConfigLoadError::Error);

        let phony_config = ConfigLoadError::Error(IOError::other("hacky placeholder"));
        let global_parsed = Config::load(Ok(&global_config), Err(phony_config))?;

        // We need to build a transient `WorkspaceTrust` just to ask whether the workspace is
        // trusted enough to load its `.helix/config.toml`. The persisted-trust file on disk is the
        // source of truth either way; this transient instance has an empty cache and is dropped
        // after the check.
        let trust = helix_loader::workspace_trust::WorkspaceTrust::new(
            (&global_parsed.editor.workspace_trust).into(),
        );
        if trust
            .query_current(helix_loader::workspace_trust::TrustQuery::LocalConfig)
            .is_trusted()
        {
            let mut merged = Config::load(Ok(&global_config), local_config)?;
            // editor.workspace-trust is global/user-scope only. Without this override, a
            // workspace's `.helix/config.toml` could set `level = "insecure"`; once the user trusted
            // *that* workspace, refresh_config would re-load with the override merged in and from
            // then on every subsequent workspace in the session would be implicitly trusted. Pin
            // the gate's own configuration to the global file.
            merged.editor.workspace_trust = global_parsed.editor.workspace_trust;
            Ok(merged)
        } else {
            Ok(global_parsed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Config {
        fn load_test(config: &str) -> Config {
            Config::load(Ok(&config.to_owned()), Err(ConfigLoadError::default())).unwrap()
        }
    }

    #[test]
    fn parsing_keymaps_config_file() {
        use crate::keymap;
        use helix_core::hashmap;
        use helix_view::document::Mode;

        let sample_keymaps = r#"
            [keys.insert]
            y = "move_line_down"
            S-C-a = "delete_selection"

            [keys.normal]
            A-F12 = "move_next_word_end"
        "#;

        let mut keys = keymap::default();
        merge_keys(
            &mut keys,
            hashmap! {
                Mode::Insert => keymap!({ "Insert mode"
                    "y" => move_line_down,
                    "S-C-a" => delete_selection,
                }),
                Mode::Normal => keymap!({ "Normal mode"
                    "A-F12" => move_next_word_end,
                }),
            },
        );

        assert_eq!(
            Config::load_test(sample_keymaps),
            Config {
                keys,
                ..Default::default()
            }
        );
    }

    #[test]
    fn keys_resolve_to_correct_defaults() {
        // From serde default
        let default_keys = Config::load_test("").keys;
        assert_eq!(default_keys, keymap::default());

        // From the Default trait
        let default_keys = Config::default().keys;
        assert_eq!(default_keys, keymap::default());
    }

    #[test]
    fn modular_merge_combines_keys() {
        // Simulate `config.toml` + `config.d/vim.toml` merging.
        let base = r#"
            [keys.normal]
            K = "hover"
        "#;
        let vim = r#"
            [keys.normal]
            w = ["move_next_word_start", "move_char_right", "collapse_selection"]
            C-h = "select_prev_sibling"
        "#;
        let base_val: toml::Value = toml::from_str(base).unwrap();
        let vim_val: toml::Value = toml::from_str(vim).unwrap();
        let merged = helix_loader::merge_toml_values(base_val, vim_val, 3);
        let merged_str = toml::to_string(&merged).unwrap();
        let cfg = Config::load_test(&merged_str);
        use helix_view::document::Mode;
        // Both K and vim's w/C-h should be present alongside defaults.
        // Helix's default `w` is `move_next_word_start`, vim overrides it to a
        // sequence, so the merged value should contain the vim sequence.
        // We can't introspect the trie easily, but we can at least check that
        // the config parsed and that the merged string contains both keys.
        assert!(merged_str.contains("hover"));
        assert!(merged_str.contains("select_prev_sibling"));
        assert!(cfg.keys.contains_key(&Mode::Normal));
    }

    #[test]
    fn modular_merge_keeps_nested_tables() {
        let base: toml::Value = toml::from_str(
            r#"
            [keys.normal.space]
            o = ":obsidian-quick-switch"
            [editor.statusline.mode]
            normal = "NORMAL"
            insert = "INSERT"
            "#,
        )
        .unwrap();
        let module: toml::Value = toml::from_str(
            r#"
            [keys.normal.space]
            f = "file_picker"
            [editor.statusline.mode]
            normal = "N"
            "#,
        )
        .unwrap();

        let merged = Config::merge_config_values(base, module);
        let space = merged
            .get("keys")
            .and_then(|v| v.get("normal"))
            .and_then(|v| v.get("space"))
            .unwrap();
        assert!(space.get("o").is_some(), "nested keymap prefix was dropped");
        assert!(space.get("f").is_some());
        let mode = merged
            .get("editor")
            .and_then(|v| v.get("statusline"))
            .and_then(|v| v.get("mode"))
            .unwrap();
        assert_eq!(mode.get("normal").unwrap().as_str(), Some("N"));
        assert_eq!(mode.get("insert").unwrap().as_str(), Some("INSERT"));
    }

    #[test]
    fn config_file_errors_include_the_path() {
        let error = toml::from_str::<toml::Value>("[broken").unwrap_err();
        let error = ConfigLoadError::BadConfigFile {
            path: PathBuf::from("/config/config.d/vim.toml"),
            error,
        };
        let message = error.to_string();
        assert!(message.contains("/config/config.d/vim.toml"));
        assert!(message.contains("TOML parse error") || message.contains("expected"));
    }
}
