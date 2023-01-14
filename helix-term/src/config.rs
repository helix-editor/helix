use crate::keymap::{default::{default, self}, merge_keys, Keymap};
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use anyhow::{Error, anyhow};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default = "default")]
    pub keys: HashMap<Mode, Keymap>,
    #[serde(default)]
    pub editor: helix_view::editor::Config,
}

impl Config {
    pub fn merged() -> Result<Self, Error> {
        let config_string = std::fs::read_to_string(helix_loader::config_file())?;
        toml::from_str(&config_string)
            .map(|config: Config| config.merge_in_default_keymap()) 
            .map_err(|error| anyhow!("{}", error))
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            theme: None,
            keys: default::default(),
            editor: helix_view::editor::Config::default(),
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
