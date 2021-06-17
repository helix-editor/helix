use serde::Deserialize;

use crate::commands::Command;
use crate::keymap::Keymaps;

#[derive(Debug, PartialEq, Deserialize)]
pub struct GlobalConfig {
    pub lsp_progress: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self { lsp_progress: true }
    }
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub global: GlobalConfig,
    pub keys: Keymaps,
}

#[test]
fn parsing_keymaps_config_file() {
    use helix_core::hashmap;
    use helix_view::document::Mode;
    use helix_view::input::{KeyCode, KeyEvent, KeyModifiers};

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
            global: Default::default(),
            keys: Keymaps(hashmap! {
                Mode::Insert => hashmap! {
                    KeyEvent {
                        code: KeyCode::Char('y'),
                        modifiers: KeyModifiers::NONE,
                    } => Command::move_line_down,
                    KeyEvent {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL,
                    } => Command::delete_selection,
                },
                Mode::Normal => hashmap! {
                    KeyEvent {
                        code: KeyCode::F(12),
                        modifiers: KeyModifiers::ALT,
                    } => Command::move_next_word_end,
                },
            })
        }
    );
}
