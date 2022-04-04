use helix_view::document::Mode;
use helix_view::keymap::{default::default, Keymap};
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

/// Merge default config keys with user overwritten keys for custom user config.
pub fn merge_keys(mut config: Config) -> Config {
    let mut delta = std::mem::replace(&mut config.keys, default());
    for (mode, keys) in &mut config.keys {
        keys.merge(delta.remove(mode).unwrap_or_default())
    }
    config
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
        use helix_core::hashmap;
        use helix_view::document::Mode;
        use helix_view::keymap::{self, Keymap};

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

    use arc_swap::access::Constant;
    use helix_core::hashmap;

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
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);

        let mut keymap = Keymaps::new(Box::new(Constant(merged_config.keys.clone())));
        assert_eq!(
            keymap.get(Mode::Normal, key!('i')),
            KeymapResult::Matched(MappableCommand::normal_mode),
            "Leaf should replace leaf"
        );
        assert_eq!(
            keymap.get(Mode::Normal, key!('无')),
            KeymapResult::Matched(MappableCommand::insert_mode),
            "New leaf should be present in merged keymap"
        );
        // Assumes that z is a node in the default keymap
        assert_eq!(
            keymap.get(Mode::Normal, key!('z')),
            KeymapResult::Matched(MappableCommand::jump_backward),
            "Leaf should replace node"
        );

        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('$')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::goto_line_end),
            "Leaf should be present in merged subnode"
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('g')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::delete_char_forward),
            "Leaf should replace old leaf in merged subnode"
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('e')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::goto_last_line),
            "Old leaves in subnode should be present in merged node"
        );

        assert!(merged_config.keys.get(&Mode::Normal).unwrap().len() > 1);
        assert!(merged_config.keys.get(&Mode::Insert).unwrap().len() > 0);
    }

    #[test]
    fn order_should_be_set() {
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
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);
        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Make sure mapping works
        assert_eq!(
            keymap
                .root()
                .search(&[key!(' '), key!('s'), key!('v')])
                .unwrap(),
            &KeyTrie::Leaf(MappableCommand::vsplit),
            "Leaf should be present in merged subnode"
        );
        // Make sure an order was set during merge
        let node = keymap.root().search(&[helix_view::key!(' ')]).unwrap();
        assert!(!node.node().unwrap().order().is_empty())
    }
}
