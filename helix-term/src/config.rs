use crate::keymap::{default::default, merge_keys, Keymap};
use helix_view::document::Mode;
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

    // Load a merged config from configuration and $PWD/.helix/config.toml
    pub fn load_merged_config() -> Config {
        let root_config: Config = std::fs::read_to_string(helix_loader::config_file())
            .ok()
            .and_then(|config| toml::from_str(&config).ok())
            .unwrap_or_else(|| {
                eprintln!("Bad config: {:?}", helix_loader::config_file());
                Config::halt_and_confirm("default");
                Config::default()
            });

        // Load each config file
        let local_config_values = helix_loader::local_config_dirs()
            .into_iter()
            .map(|path| path.join("config.toml"))
            .chain([helix_loader::config_file()])
            .filter_map(|file| Config::load_config_toml_values(&root_config, file));

        // Merge configs and return, or alert user of error and load default
        match local_config_values.reduce(|a, b| helix_loader::merge_toml_values(b, a, 3)) {
            Some(conf) => conf.try_into().unwrap_or_default(),
            None => root_config,
        }
    }

    // Load a specific config file if allowed by config
    // Stay with toml::Values as they can be merged
    pub fn load_config_toml_values(
        root_config: &Config,
        config_path: std::path::PathBuf,
    ) -> Option<toml::Value> {
        if config_path.exists()
            && (config_path == helix_loader::config_file()
                || root_config.editor.security.load_local_config)
        {
            log::debug!("Load config: {:?}", config_path);
            let bytes = std::fs::read(&config_path);
            let cfg: Option<toml::Value> = match bytes {
                Ok(bytes) => {
                    let cfg = toml::from_slice(&bytes);
                    match cfg {
                        Ok(cfg) => Some(cfg),
                        Err(e) => {
                            eprintln!("Toml parse error for {:?}: {}", &config_path, e);
                            Config::halt_and_confirm("loaded");
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Could not read {:?}: {}", &config_path, e);
                    Config::halt_and_confirm("loaded");
                    None
                }
            };
            cfg
        } else {
            None
        }
    }

    fn halt_and_confirm(config_type: &'static str) {
        eprintln!("Press <ENTER> to continue with {} config", config_type);
        let mut tmp = String::new();
        let _ = std::io::stdin().read_line(&mut tmp);
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
}
