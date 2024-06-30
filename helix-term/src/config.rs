use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie};
use helix_loader::merge_toml_values;
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
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
    pub fn load(
        global: Result<String, ConfigLoadError>,
        local: Result<String, ConfigLoadError>,
    ) -> Result<Config, ConfigLoadError> {
        let global_config: Result<ConfigRaw, ConfigLoadError> =
            global.and_then(|file| toml::from_str(&file).map_err(ConfigLoadError::BadConfig));
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
                    (Some(global), Some(local)) => merge_toml_values(global, local, 3)
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

    pub fn load_default() -> Result<Config, ConfigLoadError> {
        let global_config =
            fs::read_to_string(helix_loader::config_file()).map_err(ConfigLoadError::Error);
        let local_config = fs::read_to_string(helix_loader::workspace_config_file())
            .map_err(ConfigLoadError::Error);
        Config::load(global_config, local_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Config {
        fn load_test(config: &str) -> Config {
            Config::load(Ok(config.to_owned()), Err(ConfigLoadError::default())).unwrap()
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
    fn parsing_menus() {
        use crate::keymap;
        use helix_core::hashmap;
        use helix_view::document::Mode;

        let sample_keymaps = r#"
            [keys.normal]
            f = { f = "file_picker", c = "wclose" }
            b = { label = "buffer", b = "buffer_picker", n = "goto_next_buffer" }
        "#;

        let mut keys = keymap::default();
        merge_keys(
            &mut keys,
            hashmap! {
                Mode::Normal => keymap!({ "Normal mode"
                    "f" => { ""
                        "f" => file_picker,
                        "c" => wclose,
                    },
                    "b" => { "buffer"
                        "b" => buffer_picker,
                        "n" => goto_next_buffer,
                    },
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
            h = ["vsplit", "normal_mode", "swap_view_left"]
            j = {command = ["hsplit", "normal_mode", {}], label = "split down"}
        "#;

        let config = Config::load_test(sample_keymaps);

        let tree = config.keys.get(&Mode::Normal).unwrap();

        if let keymap::KeyTrie::Node(node) = tree {
            let open_node = node.get(&KeyEvent::from_str("o").unwrap()).unwrap();

            if let keymap::KeyTrie::MappableCommand(MappableCommand::Typable { doc, .. }) =
                open_node
            {
                assert_eq!(doc, "Edit Config");
            } else {
                panic!("Edit Config did not parse to typable command");
            }

            let close_node = node.get(&KeyEvent::from_str("c").unwrap()).unwrap();
            if let keymap::KeyTrie::MappableCommand(MappableCommand::Typable { doc, .. }) =
                close_node
            {
                assert_eq!(doc, ":buffer-close []");
            } else {
                panic!(":buffer-close command did not parse to typable command");
            }

            let split_left = node.get(&KeyEvent::from_str("h").unwrap()).unwrap();
            if let keymap::KeyTrie::Sequence(label, cmds) = split_left {
                assert_eq!(label, KeyTrie::DEFAULT_SEQUENCE_LABEL);
                assert_eq!(
                    *cmds,
                    vec![
                        MappableCommand::vsplit,
                        MappableCommand::normal_mode,
                        MappableCommand::swap_view_left
                    ]
                );
            }

            let split_down = node.get(&KeyEvent::from_str("j").unwrap()).unwrap();
            if let keymap::KeyTrie::Sequence(label, cmds) = split_down {
                assert_eq!(label, "split down");
                assert_eq!(
                    *cmds,
                    vec![
                        MappableCommand::hsplit,
                        MappableCommand::normal_mode,
                        MappableCommand::swap_view_down
                    ]
                );
            }
        } else {
            panic!("Config did not parse to trie");
        }
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
