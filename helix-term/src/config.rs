use crate::keymap::{default::default, merge_keys, Keymap};
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Display;
use std::io::Error as IOError;
use std::path::PathBuf;
use toml::de::Error as TomlError;

use helix_view::editor::ok_or_default;

// NOTE: The fields in this struct use the deserializer ok_or_default to continue parsing when
// there is an error. In that case, it will use the default value.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    #[serde(default, deserialize_with = "ok_or_default")]
    pub theme: Option<String>,
    #[serde(default = "default", deserialize_with = "ok_or_default")]
    pub keys: HashMap<Mode, Keymap>,
    #[serde(default, deserialize_with = "ok_or_default")]
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
    pub fn load(
        config_path: PathBuf,
        ignored_keys: &mut BTreeSet<String>,
    ) -> Result<Config, ConfigLoadError> {
        match std::fs::read_to_string(config_path) {
            Ok(config) => {
                serde_ignored::deserialize(&mut toml::Deserializer::new(&config), |path| {
                    ignored_keys.insert(path.to_string());
                })
                .map(merge_keys)
                .map_err(ConfigLoadError::BadConfig)
            }
            Err(err) => Err(ConfigLoadError::Error(err)),
        }
    }

    pub fn load_default(ignored_keys: &mut BTreeSet<String>) -> Result<Config, ConfigLoadError> {
        Config::load(helix_loader::config_file(), ignored_keys)
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

    #[test]
    fn partial_config_parsing() {
        use crate::keymap;
        use crate::keymap::Keymap;
        use helix_core::hashmap;
        use helix_view::document::Mode;

        let sample_keymaps = r#"
            theme = false

            [editor]
            line-number = false
            mous = "false"
            scrolloff = 7

            [editor.search]
            smart-case = false

            [keys.insert]
            y = "move_line_down"
            SC-a = "delete_selection"

            [keys.normal]
            A-F12 = "move_next_word_end"
        "#;

        let mut editor = helix_view::editor::Config::default();
        editor.search.smart_case = false;
        editor.scrolloff = 7;

        assert_eq!(
            toml::from_str::<Config>(sample_keymaps).unwrap(),
            Config {
                keys: hashmap! {
                    Mode::Insert => Keymap::new(keymap!({ "Insert mode"
                        "y" => move_line_down,
                    })),
                    Mode::Normal => Keymap::new(keymap!({ "Normal mode"
                        "A-F12" => move_next_word_end,
                    })),
                },
                editor,
                ..Default::default()
            }
        );
    }
}
