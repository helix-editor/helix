use crate::keymap::{default::default, keymaps::Keymaps, Keymap};
use helix_view::document::Mode;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Display, io::Error as IOError};
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
    use crate::{
        commands::MappableCommand,
        config::Config,
        keymap::{
            default,
            keymaps::{KeymapResult, Keymaps},
            keytrienode::KeyTrieNode,
            macros::*,
            Keymap,
        },
    };
    use arc_swap::access::Constant;
    use helix_core::hashmap;
    use helix_view::document::Mode;

    #[test]
    fn parsing_keymaps_config_file() {
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
        assert_eq!(default_keys, default::default());

        // From the Default trait
        let default_keys = Config::default().keys;
        assert_eq!(default_keys, default::default());
    }

    #[test]
    fn merge_partial_keys() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "i" => normal_mode,
                        "无" => insert_mode,
                        "z" => jump_backward,
                        "g" => { "Merge into goto mode"
                            "$" => goto_line_end,
                            "g" => delete_char_forward,
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = Keymaps::merge_with_default(config.clone());
        assert_ne!(config, merged_config);

        let mut keymap = Keymaps::new(Box::new(Constant(merged_config.keys.clone())));
        assert_eq!(
            keymap.get(Mode::Normal, key!('i')),
            KeymapResult::Matched(MappableCommand::normal_mode),
            "New mappable command should ovveride default."
        );
        assert_eq!(
            keymap.get(Mode::Normal, key!('无')),
            KeymapResult::Matched(MappableCommand::insert_mode),
            "New mappable command should be present in merged keymap."
        );
        // Assumes that z is a node in the default keymap
        assert_eq!(
            keymap.get(Mode::Normal, key!('z')),
            KeymapResult::Matched(MappableCommand::jump_backward),
            "New Mappable command should replace default sub keytrie."
        );

        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('$')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::goto_line_end),
            "Mappable command should be present in merged keytrie."
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('g')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::delete_char_forward),
            "Mappable command should replace default in merged keytrie."
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('e')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::goto_last_line),
            "Mappable commands from default keytrie should still be present in merged merged keytrie unless overridden."
        );

        assert!(merged_config.keys.get(&Mode::Normal).unwrap().len() > 1);
        assert!(merged_config.keys.get(&Mode::Insert).unwrap().len() > 0);
    }

    #[test]
    fn merges_with_default_keymap_config() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "space" => { ""
                            "s" => { ""
                                "v" => vsplit,
                                "c" => hsplit,
                            },
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = Keymaps::merge_with_default(config.clone());
        assert_ne!(config, merged_config);
        let keymap_normal = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        assert_eq!(
            keymap_normal
                .root_node
                .traverse(&[key!(' '), key!('s'), key!('v')])
                .unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::vsplit),
            "Mappable command should be present in merged keytrie."
        );
    }
}
