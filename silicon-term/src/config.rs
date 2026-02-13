use crate::commands::MappableCommand;
use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie, KeyTrieNode};
use silicon_lua::{KeyBinding, ThemeConfig};
use silicon_view::{document::Mode, input::KeyEvent, theme};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub theme: Option<theme::Config>,
    pub keys: HashMap<Mode, KeyTrie>,
    pub editor: silicon_view::editor::Config,
    /// Pre-parsed TOML data for a custom Lua-defined theme (`si.theme.define()`).
    pub custom_theme_data: Option<toml::Value>,
    /// Language config from Lua, to be merged with built-in `languages.toml`.
    pub language_config: Option<toml::Value>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            theme: Some(theme::Config::constant("onedark")),
            keys: keymap::default(),
            editor: silicon_view::editor::Config::default(),
            custom_theme_data: None,
            language_config: None,
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

        let (theme, custom_theme_data) = match lua_config.theme {
            Some(ThemeConfig::Named(name)) => (Some(theme::Config::constant(&name)), None),
            Some(ThemeConfig::Adaptive {
                light,
                dark,
                fallback,
            }) => (
                Some(theme::Config::adaptive(
                    &light,
                    &dark,
                    fallback.as_deref(),
                )),
                None,
            ),
            Some(ThemeConfig::Custom { name, spec }) => {
                let toml_data = silicon_lua::theme::theme_spec_to_toml(&spec);
                (Some(theme::Config::constant(&name)), Some(toml_data))
            }
            None => (None, None),
        };

        Config {
            theme,
            keys,
            editor: lua_config.editor,
            custom_theme_data,
            language_config: lua_config.language_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_resolve_to_correct_defaults() {
        let default_keys = Config::default().keys;
        assert_eq!(default_keys, keymap::default());
    }
}
