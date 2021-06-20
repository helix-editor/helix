use anyhow::{Error, Result};
use std::{collections::HashMap, str::FromStr};

use serde::{de::Error as SerdeError, Deserialize, Serialize};

use crate::keymap::{parse_keymaps, Keymaps};

pub struct GlobalConfig {
    pub theme: Option<String>,
    pub lsp_progress: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            lsp_progress: true,
            theme: None,
        }
    }
}

#[derive(Default)]
pub struct Config {
    pub global: GlobalConfig,
    pub keymaps: Keymaps,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlConfig {
    theme: Option<String>,
    lsp_progress: Option<bool>,
    keys: Option<HashMap<String, HashMap<String, String>>>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = TomlConfig::deserialize(deserializer)?;
        Ok(Self {
            global: GlobalConfig {
                lsp_progress: config.lsp_progress.unwrap_or(true),
                theme: config.theme,
            },
            keymaps: config
                .keys
                .map(|r| parse_keymaps(&r))
                .transpose()
                .map_err(|e| D::Error::custom(format!("Error deserializing keymap: {}", e)))?
                .unwrap_or_else(Keymaps::default),
        })
    }
}
