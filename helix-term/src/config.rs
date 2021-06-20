use anyhow::{Error, Result};
use std::collections::HashMap;

use serde::{de::Error as SerdeError, Deserialize, Serialize};

use crate::keymap::{parse_keymaps, Keymaps};

#[derive(Default)]
pub struct Config {
    pub theme: Option<String>,
    pub lsp: LspConfig,
    pub keymaps: Keymaps,
}

#[derive(Default, Serialize, Deserialize)]
pub struct LspConfig {
    pub display_messages: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlConfig {
    theme: Option<String>,
    #[serde(default)]
    lsp: LspConfig,
    keys: Option<HashMap<String, HashMap<String, String>>>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = TomlConfig::deserialize(deserializer)?;
        Ok(Self {
            theme: config.theme,
            lsp: config.lsp,
            keymaps: config
                .keys
                .map(|r| parse_keymaps(&r))
                .transpose()
                .map_err(|e| D::Error::custom(format!("Error deserializing keymap: {}", e)))?
                .unwrap_or_else(Keymaps::default),
        })
    }
}
