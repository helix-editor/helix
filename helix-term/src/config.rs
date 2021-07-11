use serde::Deserialize;

use crate::keymap::Keymaps;

#[cfg(test)]
use crate::commands::Command;

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default)]
    pub lsp: LspConfig,
    #[serde(default)]
    pub keys: Keymaps,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LspConfig {
    pub display_messages: bool,
}

#[test]
fn parsing_keymaps_config_file() {
    use crate::keymap::KeyNode;
    use helix_core::hashmap;
    use helix_view::{
        document::Mode,
        input::KeyEvent,
        keyboard::{KeyCode, KeyModifiers},
    };

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
            keys: Keymaps(hashmap! {
                Mode::Insert => hashmap! {
                    KeyEvent {
                        code: KeyCode::Char('y'),
                        modifiers: KeyModifiers::NONE,
                    } => KeyNode::KeyCommand(Command::move_line_down),
                    KeyEvent {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL,
                    } => KeyNode::KeyCommand(Command::delete_selection),
                },
                Mode::Normal => hashmap! {
                    KeyEvent {
                        code: KeyCode::F(12),
                        modifiers: KeyModifiers::ALT,
                    } => KeyNode::KeyCommand(Command::move_next_word_end),
                },
            }),
            ..Default::default()
        }
    );
}
