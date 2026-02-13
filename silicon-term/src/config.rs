use crate::commands::MappableCommand;
use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie, KeyTrieNode};
use silicon_loader::merge_toml_values;
use silicon_lua::KeyBinding;
use silicon_view::{document::Mode, input::KeyEvent, theme};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
use toml::de::Error as TomlError;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub theme: Option<theme::Config>,
    pub keys: HashMap<Mode, KeyTrie>,
    pub editor: silicon_view::editor::Config,
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
            theme: Some(theme::Config::constant("onedark")),
            keys: keymap::default(),
            editor: silicon_view::editor::Config::default(),
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

/// Convert a `KeyBinding` intermediate representation to a `KeyTrie`.
fn convert_keybinding(binding: KeyBinding) -> Result<KeyTrie, String> {
    match binding {
        KeyBinding::Command(name) => name
            .parse::<MappableCommand>()
            .map(KeyTrie::MappableCommand)
            .map_err(|e| e.to_string()),
        KeyBinding::Sequence(names) => {
            let cmds: Result<Vec<MappableCommand>, _> = names
                .iter()
                .map(|n| n.parse::<MappableCommand>())
                .collect();
            cmds.map(KeyTrie::Sequence).map_err(|e| e.to_string())
        }
        KeyBinding::Node {
            label,
            is_sticky,
            map,
            order,
        } => {
            let mut converted_map: HashMap<KeyEvent, KeyTrie> = HashMap::new();
            let mut converted_order: Vec<KeyEvent> = Vec::new();
            for key_event in &order {
                if let Some(child) = map.get(key_event) {
                    match convert_keybinding(child.clone()) {
                        Ok(trie) => {
                            converted_map.insert(*key_event, trie);
                            converted_order.push(*key_event);
                        }
                        Err(e) => {
                            log::warn!("skipping keybinding for {}: {}", key_event, e);
                        }
                    }
                }
            }
            let mut node = KeyTrieNode::new(&label, converted_map, converted_order);
            node.is_sticky = is_sticky;
            Ok(KeyTrie::Node(node))
        }
    }
}

impl Config {
    pub fn from_lua(lua_config: silicon_lua::LuaConfig) -> Self {
        let mut keys = keymap::default();
        if !lua_config.keys.is_empty() {
            let mut user_keys = HashMap::new();
            for (mode, binding) in lua_config.keys {
                match convert_keybinding(binding) {
                    Ok(keytrie) => {
                        user_keys.insert(mode, keytrie);
                    }
                    Err(e) => log::warn!("invalid keybinding for {:?}: {}", mode, e),
                }
            }
            merge_keys(&mut keys, user_keys);
        }
        Config {
            theme: Some(theme::Config::constant("onedark")),
            keys,
            editor: lua_config.editor,
        }
    }

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
                    (None, None) => silicon_view::editor::Config::default(),
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
                        || Ok(silicon_view::editor::Config::default()),
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
            fs::read_to_string(silicon_loader::config_file()).map_err(ConfigLoadError::Error);
        let local_config = fs::read_to_string(silicon_loader::workspace_config_file())
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
        use silicon_core::hashmap;
        use silicon_view::document::Mode;

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
                theme: None,
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
