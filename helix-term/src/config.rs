use anyhow::{Error, Result};
use std::{collections::HashMap, str::FromStr};

use serde::{de::Error as SerdeError, Deserialize, Serialize};

use crate::keymap::{parse_keymaps, Keymaps};

#[derive(Default)]
pub struct Config {
    pub keymaps: Keymaps,
}

#[derive(Serialize, Deserialize)]
struct TomlConfig {
    keys: Option<HashMap<String, HashMap<String, String>>>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = TomlConfig::deserialize(deserializer)?;
        Ok(Self {
            keymaps: config
                .keys
                .map(|r| parse_keymaps(&r))
                .transpose()
                .map_err(|e| D::Error::custom(format!("Error deserializing keymap: {}", e)))?
                .unwrap_or_else(Keymaps::default),
        })
    }
}
