use crate::keymap::{default::default, Keymap, keymaps::Keymaps};
use helix_view::document::Mode;
use std::{
    fmt::Display,
    collections::HashMap,
    io::Error as IOError,
};
use toml::de::Error as TomlError;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default = "default")]
    pub keys: HashMap<Mode, Keymap>,
    #[serde(default)]
    pub editor: helix_view::editor::Config,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            theme: None,
            keys: default(),
            editor: helix_view::editor::Config::default(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigLoadError { 
    BadConfig(TomlError),
    Error(IOError),
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
    // REFACTOR? code similar to config assignment in main.rs, 
    pub fn load_default() -> Result<Config, ConfigLoadError> {
        match std::fs::read_to_string(helix_loader::config_file()) {
            Ok(config) => toml::from_str(&config)
                .map(Keymaps::merge_with_default)
                .map_err(ConfigLoadError::BadConfig),
            Err(err) => Err(ConfigLoadError::Error(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_keymaps_config_file() {
        use crate::keymap;
        use crate::keymap::Keymap;
        use helix_core::hashmap;
        use helix_view::document::Mode;

        let sample_keymaps = r#"
            [keys.insert]
            y = "move_line_down"
            S-C-a = "delete_selection"

            [keys.normal]
            A-F12 = "move_next_word_end"
        "#;

        assert_eq!(
            toml::from_str::<Config>(sample_keymaps).unwrap(),
            Config {
                keys: hashmap! {
                    Mode::Insert => Keymap::new(keymap!({ "Insert mode"
                        "y" => move_line_down,
                        "S-C-a" => delete_selection,
                    })),
                    Mode::Normal => Keymap::new(keymap!({ "Normal mode"
                        "A-F12" => move_next_word_end,
                    })),
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn keys_resolve_to_correct_defaults() {
        // From serde default
        let default_keys = toml::from_str::<Config>("").unwrap().keys;
        assert_eq!(default_keys, default());

        // From the Default trait
        let default_keys = Config::default().keys;
        assert_eq!(default_keys, default());
    }
}
