use crate::keymap::{default::default, merge_keys, Keymap};
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Error as IOError;
use std::path::PathBuf;
use toml::de::Error as TomlError;

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
    pub fn load(config_path: PathBuf) -> Result<Config, ConfigLoadError> {
        match std::fs::read_to_string(config_path) {
            Ok(config) => toml::from_str(&config)
                .map(merge_keys)
                .map_err(ConfigLoadError::BadConfig),
            Err(err) => Err(ConfigLoadError::Error(err)),
        }
    }

    pub fn load_default() -> Result<Config, ConfigLoadError> {
        Config::load(helix_loader::config_file())
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
    fn parsing_menus() {
        use crate::keymap;
        use crate::keymap::Keymap;
        use helix_core::hashmap;
        use helix_view::document::Mode;

        let sample_keymaps = r#"
            [keys.normal]
            f = { f = "file_picker", c = "wclose" }
            b = { label = "buffer", b = "buffer_picker", n = "goto_next_buffer" }
        "#;

        assert_eq!(
            toml::from_str::<Config>(sample_keymaps).unwrap(),
            Config {
                keys: hashmap! {
                    Mode::Normal => Keymap::new(keymap!({ "Normal mode"
                        "f" => { ""
                            "f" => file_picker,
                            "c" => wclose,
                        },
                        "b" => { "buffer"
                            "b" => buffer_picker,
                            "n" => goto_next_buffer,
                        },
                    })),
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn parsing_typable_commands() {
        use crate::keymap;
        use crate::keymap::MappableCommand;
        use helix_view::document::Mode;
        use helix_view::input::KeyEvent;
        use std::str::FromStr;

        let sample_keymaps = r#"
            [keys.normal]
            o = { label = "Edit Config", command = ":open ~/.config" }
            c = ":buffer-close" 
        "#;

        let config = toml::from_str::<Config>(sample_keymaps).unwrap();

        let tree = config.keys.get(&Mode::Normal).unwrap().root();

        if let keymap::KeyTrie::Node(node) = tree {
            let open_node = node.get(&KeyEvent::from_str("o").unwrap()).unwrap();

            if let keymap::KeyTrie::Leaf(MappableCommand::Typable { doc, .. }) = open_node {
                assert_eq!(doc, "Edit Config");
            } else {
                panic!("Edit Config did not parse to typable command");
            }

            let close_node = node.get(&KeyEvent::from_str("c").unwrap()).unwrap();
            if let keymap::KeyTrie::Leaf(MappableCommand::Typable { doc, .. }) = close_node {
                assert_eq!(doc, ":buffer-close []");
            } else {
                panic!(":buffer-close command did not parse to typable command");
            }
        } else {
            panic!("Config did not parse to trie");
        }
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
