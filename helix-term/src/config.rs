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
    #[serde(default)]
    pub paths: helix_loader::Paths,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            theme: None,
            keys: default(),
            editor: helix_view::editor::Config::default(),
            paths: helix_loader::Paths::default(),
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

    #[test]
    fn directories_resolve_to_correct_defaults() {
        // From serde default
        let paths = toml::from_str::<Config>("").unwrap().paths;
        assert_eq!(paths, helix_loader::Paths::default());

        // From the Default trait
        let paths = Config::default().paths;
        assert_eq!(paths, helix_loader::Paths::default());
    }

    #[test]
    fn partialy_specified_directories_resolve_correctly() {
        use helix_loader::Path;
        use std::path::PathBuf;

        const CONFIG: &str = r#"
            [paths]
            log-file = "../rel/path/log.file"
            grammar-dir = "/somewhere/else"
        "#;
        let defaults = helix_loader::Paths::default();
        let paths = toml::from_str::<Config>(CONFIG).unwrap().paths;

        assert_eq!(
            paths.get(&Path::LogFile),
            PathBuf::from("../rel/path/log.file")
        );
        assert_eq!(
            paths.get(&Path::GrammarDir),
            PathBuf::from("/somewhere/else")
        );

        assert_eq!(
            paths.get(&Path::LanguageFile),
            defaults.get(&Path::LanguageFile)
        );
        assert_eq!(paths.get(&Path::ThemeDir), defaults.get(&Path::ThemeDir));
        assert_eq!(paths.get(&Path::QueryDir), defaults.get(&Path::QueryDir));
    }

    #[test]
    fn invalid_path_key_specified() {
        const CONFIG: &str = r#"
            [paths]
            log-dir = "../rel/path/log.file"
            grammar-file = "/somewhere/else"
        "#;
        let paths = toml::from_str::<Config>(CONFIG);

        assert!(paths.is_err())
    }
}
