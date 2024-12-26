use crate::keymap;
use crate::keymap::{merge_keys, KeyTrie};
use helix_loader::merge_toml_values;
use helix_view::commands::custom::CustomTypableCommand;
use helix_view::document::Mode;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::io::Error as IOError;
use std::sync::Arc;
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
    commands: Option<Commands>,
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
            (Ok(mut global), Ok(local)) => {
                let mut keys = keymap::default();
                if let Some(global_keys) = global.keys {
                    merge_keys(&mut keys, global_keys)
                }
                if let Some(local_keys) = local.keys {
                    merge_keys(&mut keys, local_keys)
                }

                let mut editor = match (global.editor, local.editor) {
                    (None, None) => helix_view::editor::Config::default(),
                    (None, Some(val)) | (Some(val), None) => {
                        val.try_into().map_err(ConfigLoadError::BadConfig)?
                    }
                    (Some(global), Some(local)) => merge_toml_values(global, local, 3)
                        .try_into()
                        .map_err(ConfigLoadError::BadConfig)?,
                };

                // Merge locally defined commands, overwriting global space commands if encountered
                if let Some(lcommands) = local.commands {
                    if let Some(gcommands) = &mut global.commands {
                        for (name, details) in lcommands.commands {
                            gcommands.commands.insert(name, details);
                        }
                    } else {
                        global.commands = Some(lcommands);
                    }
                }

                // If any commands were defined anywhere, add to editor
                if let Some(commands) = global.commands {
                    editor.commands.commands = commands
                        .commands
                        .into_iter()
                        .map(|(name, details)| details.into_custom_command(name))
                        .collect();
                }

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

                let mut editor = config.editor.map_or_else(
                    || Ok(helix_view::editor::Config::default()),
                    |val| val.try_into().map_err(ConfigLoadError::BadConfig),
                )?;

                // Add custom commands
                if let Some(commands) = config.commands {
                    editor.commands.commands = commands
                        .commands
                        .into_iter()
                        .map(|(name, details)| details.into_custom_command(name))
                        .collect();
                }

                Config {
                    theme: config.theme,
                    keys,
                    editor,
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

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
struct Commands {
    #[serde(flatten)]
    commands: HashMap<String, CommandTomlType>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
enum CommandTomlType {
    Single(String),
    Multiple(Vec<String>),
    Detailed {
        commands: Vec<String>,
        desc: Option<String>,
        accepts: Option<String>,
        completer: Option<String>,
    },
}

impl CommandTomlType {
    fn into_custom_command(self, name: String) -> CustomTypableCommand {
        let name = name.trim_start_matches(':');
        match self {
            Self::Single(command) => CustomTypableCommand {
                name: Arc::from(name),
                desc: None,
                commands: vec![Arc::from(command.trim_start_matches(':'))].into(),
                accepts: None,
                completer: None,
            },
            Self::Multiple(commands) => CustomTypableCommand {
                name: Arc::from(name),
                desc: None,
                commands: commands
                    .into_iter()
                    .map(|command| Arc::from(command.trim_start_matches(':')))
                    .collect::<Vec<_>>()
                    .into(),
                accepts: None,
                completer: None,
            },
            Self::Detailed {
                commands,
                desc,
                accepts,
                completer,
            } => CustomTypableCommand {
                name: Arc::from(name),
                desc: desc.map(Arc::from),
                commands: commands
                    .into_iter()
                    .map(|command| Arc::from(command.trim_start_matches(':')))
                    .collect::<Vec<_>>()
                    .into(),
                accepts: accepts.map(|accepts| Arc::from(accepts.trim_start_matches(':'))),
                completer: completer.map(|completer| Arc::from(completer.trim_start_matches(':'))),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    impl Config {
        fn load_test(config: &str) -> Result<Config, ConfigLoadError> {
            Config::load(Ok(config.to_owned()), Err(ConfigLoadError::default()))
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
            Config::load_test(sample_keymaps).unwrap(),
            Config {
                keys,
                ..Default::default()
            }
        );
    }

    #[test]
    fn keys_resolve_to_correct_defaults() {
        // From serde default
        let default_keys = Config::load_test("").unwrap().keys;
        assert_eq!(default_keys, keymap::default());

        // From the Default trait
        let default_keys = Config::default().keys;
        assert_eq!(default_keys, keymap::default());
    }

    #[test]
    fn should_deserialize_custom_commands() {
        let config = r#"
[commands]
":wq" = [":write", "quit"]
":w" = ":write --force"
":wcd!" = { commands = [':write --force %{arg}', ':cd %sh{ %{arg} | path dirname }'], desc = "writes buffer to disk forcefully, then changes to its directory", accepts = "<path>", completer = ":write" }
"#;

        if let Err(err) = Config::load_test(config) {
            panic!("{err:#?}")
        };
    }

    // TODO: See if this capabiliy till be allowed
    //     #[test]
    //     fn should_deserialize_custom_commands_into_keys() {
    //         let config = r#"
    // [keys.normal.space]
    // g = ":lg"

    // [commands]
    // ":lg" = ""
    // "#;

    //         if let Err(err) = Config::load_test(config) {
    //             panic!("{err:#?}")
    //         };
    //     }
}
