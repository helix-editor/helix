use std::collections::HashSet;

use crate::syntax::{
    Configuration, LanguageServerFeatures, Loader, LoaderError,
};

static ENABLE_COPILOT: once_cell::sync::OnceCell<bool> = once_cell::sync::OnceCell::new();

/// Language configuration based on built-in languages.toml.
pub fn default_lang_config() -> Configuration {
    helix_loader::config::default_lang_config()
        .try_into()
        .expect("Could not deserialize built-in languages.toml")
}

/// Language configuration loader based on built-in languages.toml.
pub fn default_lang_loader() -> Loader {
    let mut config = default_lang_config();
    if ENABLE_COPILOT.get().map_or(false, |v| *v) {
        append_copilot_lsp_to_language_configs(&mut config);
    }
    Loader::new(config).expect("Could not compile loader for default config")
}

#[derive(Debug)]
pub enum LanguageLoaderError {
    DeserializeError(toml::de::Error),
    LoaderError(LoaderError),
}

impl std::fmt::Display for LanguageLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializeError(err) => write!(f, "Failed to parse language config: {err}"),
            Self::LoaderError(err) => write!(f, "Failed to compile language config: {err}"),
        }
    }
}

impl std::error::Error for LanguageLoaderError {}

/// Language configuration based on user configured languages.toml.
pub fn user_lang_config() -> Result<Configuration, toml::de::Error> {
    helix_loader::config::user_lang_config()?.try_into()
}

/// Language configuration loader based on user configured languages.toml.
pub fn user_lang_loader() -> Result<Loader, LanguageLoaderError> {
    let mut config: Configuration = helix_loader::config::user_lang_config()
        .map_err(LanguageLoaderError::DeserializeError)?
        .try_into()
        .map_err(LanguageLoaderError::DeserializeError)?;

    if ENABLE_COPILOT.get().map_or(false, |v| *v) {
        append_copilot_lsp_to_language_configs(&mut config);
    }

    Loader::new(config).map_err(LanguageLoaderError::LoaderError)
}

fn append_copilot_lsp_to_language_configs(config: &mut Configuration) {
    let copilot_ls = LanguageServerFeatures {
        name: "copilot".into(),
        only: HashSet::new(),
        excluded: HashSet::new(),
    };
    for lan_config in config.language.iter_mut() {
        lan_config.language_servers.push(copilot_ls.clone());
    }
}

pub fn initialize_enable_copilot(enable_copilot: bool) {
    ENABLE_COPILOT.set(enable_copilot).ok();
}
