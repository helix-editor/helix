use crate::keymap::{merge_keys, Keymaps};
use anyhow::{Error, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub theme: Option<String>,
    #[serde(default)]
    pub lsp: LspConfig,
    #[serde(default)]
    pub keys: Keymaps,
    #[serde(default)]
    pub editor: helix_view::editor::Config,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LspConfig {
    pub display_messages: bool,
}

impl Config {
    pub fn load(config_path: PathBuf) -> Result<Config, Error> {
        match std::fs::read_to_string(config_path) {
            Ok(config) => Result::Ok(toml::from_str(&config)
                .map(merge_keys)
                .unwrap_or_else(|err| {
                    eprintln!("Bad config: {}", err);
                    eprintln!("Press <ENTER> to continue with default config");
                    use std::io::Read;
                    // This waits for an enter press.
                    let _ = std::io::stdin().read(&mut []);
                    Config::default()
                })),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Result::Ok(Config::default()),
            Err(err) => return Err(Error::new(err)),
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
}
