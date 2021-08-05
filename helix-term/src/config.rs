use serde::Deserialize;

use crate::keymap::Keymaps;

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default)]
    pub lsp: LspConfig,
    #[serde(default)]
    pub keys: Keymaps,
    #[serde(default)]
    pub terminal: TerminalConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LspConfig {
    pub display_messages: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TerminalConfig {
    #[serde(default = "mouse_default")]
    pub mouse: bool,
    #[serde(default = "middle_click_paste_default")]
    pub middle_click_paste: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            mouse: true,
            middle_click_paste: true,
        }
    }
}

fn mouse_default() -> bool {
    true
}

fn middle_click_paste_default() -> bool {
    true
}

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
            keys: Keymaps(hashmap! {
                Mode::Insert => Keymap::new(keymap!({ "Insert mode"
                    "y" => move_line_down,
                    "S-C-a" => delete_selection,
                })),
                Mode::Normal => Keymap::new(keymap!({ "Normal mode"
                    "A-F12" => move_next_word_end,
                })),
            }),
            ..Default::default()
        }
    );
}
