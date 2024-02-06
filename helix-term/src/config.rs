use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie};
use helix_loader::merge_toml_values;
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
use std::path::Path;
use toml::de::Error as TomlError;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub theme: Option<String>,
    pub keys: HashMap<Mode, KeyTrie>,
    pub editor: helix_view::editor::Config,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigRaw {
    pub theme: Option<String>,
    pub keys: Option<HashMap<Mode, KeyTrie>>,
    pub editor: Option<toml::Value>,
}

impl ConfigRaw {
    fn merge(global: ConfigRaw, local: ConfigRaw) -> Result<ConfigRaw, ConfigLoadError> {
        let keys = match (global.keys, local.keys) {
            (None, None) => None,
            (Some(keys), None) | (None, Some(keys)) => Some(keys),
            (Some(mut global_keys), Some(local_keys)) => {
                merge_keys(&mut global_keys, local_keys);
                Some(global_keys)
            }
        };

        let editor = match (global.editor, local.editor) {
            (None, None) => None,
            (None, Some(val)) | (Some(val), None) => {
                val.try_into().map_err(ConfigLoadError::BadConfig)?
            }
            (Some(global), Some(local)) => merge_toml_values(global, local, 3)
                .try_into()
                .map_err(ConfigLoadError::BadConfig)?,
        };

        Ok(ConfigRaw {
            theme: local.theme.or(global.theme),
            keys,
            editor,
        })
    }
}

impl TryFrom<ConfigRaw> for Config {
    type Error = ConfigLoadError;
    fn try_from(config: ConfigRaw) -> Result<Self, Self::Error> {
        // merge raw config into defaults
        let mut keys = keymap::default();
        if let Some(config_keys) = config.keys {
            merge_keys(&mut keys, config_keys)
        }
        let editor = config
            .editor
            .map(|value| value.try_into())
            .transpose()
            .map_err(ConfigLoadError::BadConfig)?
            .unwrap_or_default();

        Ok(Self {
            // workspace_config: config.workspace_config.unwrap_or_default(),
            theme: config.theme,
            keys,
            editor,
        })
    }
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
            ConfigLoadError::Error(err) => err.fmt(f),
        }
    }
}

impl Config {
    pub fn load_default() -> Result<Config, ConfigLoadError> {
        fn load(path: &Path) -> Result<ConfigRaw, ConfigLoadError> {
            fs::read_to_string(path)
                .map_err(ConfigLoadError::Error)
                .and_then(|file| toml::from_str(&file).map_err(ConfigLoadError::BadConfig))
        }

        let global = load(&helix_loader::config_file())?;
        let workspace = load(&helix_loader::workspace_config_file());

        if let Ok(workspace) = workspace {
            let config = ConfigRaw::merge(global, workspace)?;
            config.try_into()
        } else {
            global.try_into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Config {
        fn load_test(config: &str) -> Config {
            let config: ConfigRaw = toml::from_str(config).unwrap();
            config.try_into().unwrap()
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
}
