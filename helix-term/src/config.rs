use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie};
use helix_loader::merge_toml_values;
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
use toml::de::Error as TomlError;

// Config loading error
#[derive(Debug)]
pub enum ConfigLoadError {
    BadConfig(TomlError),
    Error(IOError),
}

impl Default for ConfigLoadError {
    fn default() -> Self {
        ConfigLoadError::Error(IOError::new(std::io::ErrorKind::NotFound, "place holder"))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadWorkspaceConfig {
    #[default]
    Never,
    Always,
}

// Deserializable raw config struct
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ConfigRaw {
    pub load_workspace_config: Option<LoadWorkspaceConfig>,
    pub theme: Option<String>,
    pub keys: Option<HashMap<Mode, KeyTrie>>,
    pub editor: Option<toml::Value>,
}

impl Default for ConfigRaw {
    fn default() -> ConfigRaw {
        Self {
            load_workspace_config: Some(LoadWorkspaceConfig::default()),
            theme: None,
            keys: Some(keymap::default()),
            editor: None,
        }
    }
}

impl ConfigRaw {
    fn load(file: PathBuf) -> Result<Self, ConfigLoadError> {
        let source = fs::read_to_string(file).map_err(ConfigLoadError::Error)?;
        toml::from_str(&source).map_err(ConfigLoadError::BadConfig)
    }

    fn merge(self, other: ConfigRaw, trust: bool) -> Self {
        ConfigRaw {
            load_workspace_config: match trust {
                true =>  other.load_workspace_config.or(self.load_workspace_config),
                false => self.load_workspace_config,
            },
            theme: other.theme.or(self.theme),
            keys: match (self.keys, other.keys) {
                (Some(a), Some(b)) => Some(merge_keys(a, b)),
                (opt_a, opt_b) => opt_a.or(opt_b),
            },
            editor: match (self.editor, other.editor) {
                (Some(a), Some(b)) => Some(merge_toml_values(a, b, 3)),
                (opt_a, opt_b) => opt_a.or(opt_b),
            }
        }
    }
}

// Final config struct
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub load_workspace_config: LoadWorkspaceConfig,
    pub theme: Option<String>,
    pub keys: HashMap<Mode, KeyTrie>,
    pub editor: helix_view::editor::Config,
}

impl Default for Config {
    fn default() -> Config {
        let raw = ConfigRaw::default();
        Self {
            load_workspace_config: raw.load_workspace_config.unwrap_or_default(),
            theme: raw.theme,
            keys: raw.keys.unwrap_or_else(|| keymap::default()),
            editor: helix_view::editor::Config::default(),
        }
    }
}

impl Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::BadConfig(err) => err.fmt(f),
            ConfigLoadError::Error(err) => err.fmt(f),
        }
    }
}

impl TryFrom<ConfigRaw> for Config {
    type Error = ConfigLoadError;

    fn try_from(config: ConfigRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            load_workspace_config: config.load_workspace_config.unwrap_or_default(),
            theme: config.theme,
            keys: config.keys.unwrap_or_else(|| keymap::default()),
            editor: config.editor
                .map(|e| e.try_into()).transpose()
                .map_err(ConfigLoadError::BadConfig)?
                .unwrap_or_default(),
        })
    }
}

impl Config {
    pub fn load() -> Result<Config, ConfigLoadError> {
        let default = ConfigRaw::default();
        let global = default.merge(ConfigRaw::load(helix_loader::config_file())?, true);

        match global.load_workspace_config {
            Some(LoadWorkspaceConfig::Always) => {
                match ConfigRaw::load(helix_loader::workspace_config_file()) {
                    Ok(workspace) => Ok(global.merge(workspace, false)),
                    Err(ConfigLoadError::Error(_)) => Ok(global),
                    error => error,
                }?
            },
            _ => global,
       }.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Config {
        fn load_test(file: &str) -> Config {
            let raw: ConfigRaw = toml::from_str(file).unwrap();
            ConfigRaw::default().merge(raw, true).try_into().unwrap()
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

        let keys = merge_keys(
            keymap::default(),
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
}
