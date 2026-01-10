use crate::{
    auto_pairs::{AutoPairsRegistry, AutoPairsRegistryError},
    syntax::{
        config::{Configuration, LanguageConfiguration},
        Loader, LoaderError,
    },
};
use helix_loader::config::AutoPairsConfigError;

/// Language configuration based on built-in languages.toml.
pub fn default_lang_config() -> Configuration {
    helix_loader::config::default_lang_config()
        .try_into()
        .expect("Could not deserialize built-in languages.toml")
}

/// Language configuration loader based on built-in languages.toml.
pub fn default_lang_loader() -> Loader {
    Loader::new(default_lang_config()).expect("Could not compile loader for default config")
}

#[derive(Debug)]
pub enum LanguageLoaderError {
    DeserializeError(toml::de::Error),
    ConfigError(toml::de::Error, String),
    LoaderError(LoaderError),
    AutoPairsError(AutoPairsRegistryError),
    AutoPairsConfigError(AutoPairsConfigError),
}

impl std::fmt::Display for LanguageLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializeError(err) => write!(f, "Failed to parse language config: {err}"),
            Self::ConfigError(err, context) => {
                write!(f, "Failed to parse language config {context}: {err}")
            }
            Self::LoaderError(err) => write!(f, "Failed to compile language config: {err}"),
            Self::AutoPairsError(err) => write!(f, "Failed to load auto-pairs config: {err}"),
            Self::AutoPairsConfigError(err) => {
                write!(f, "Failed to load auto-pairs config: {err}")
            }
        }
    }
}

impl std::error::Error for LanguageLoaderError {}

/// Language configuration based on user configured languages.toml.
pub fn user_lang_config() -> Result<Configuration, toml::de::Error> {
    helix_loader::config::user_lang_config()?.try_into()
}

/// Load the auto-pairs registry from auto-pairs.toml.
pub fn auto_pairs_registry() -> Result<AutoPairsRegistry, LanguageLoaderError> {
    let config_val = helix_loader::config::auto_pairs_config()
        .map_err(LanguageLoaderError::AutoPairsConfigError)?;
    AutoPairsRegistry::from_toml(&config_val).map_err(LanguageLoaderError::AutoPairsError)
}

/// Language configuration loader based on user configured languages.toml.
pub fn user_lang_loader() -> Result<Loader, LanguageLoaderError> {
    let config_val =
        helix_loader::config::user_lang_config().map_err(LanguageLoaderError::DeserializeError)?;
    let config = config_val.clone().try_into().map_err(|e| {
        if let Some(languages) = config_val.get("language").and_then(|v| v.as_array()) {
            for lang in languages.iter() {
                let res: Result<LanguageConfiguration, _> = lang.clone().try_into();
                if let Err(inner_err) = res {
                    let context = match lang.get("name") {
                        Some(name) => format!("for language {}", name),
                        None => "for unknown language".to_owned(),
                    };
                    return LanguageLoaderError::ConfigError(inner_err, context);
                }
            }
        }
        LanguageLoaderError::ConfigError(e, String::new())
    })?;

    let registry = auto_pairs_registry()?;
    Loader::new_with_auto_pairs(config, registry).map_err(LanguageLoaderError::LoaderError)
}
