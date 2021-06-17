use anyhow::{Error, Result};
use std::{collections::HashMap, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::keymap::{parse_keymaps, Keymaps};

#[derive(Default)]
pub struct Config {
    pub keymaps: Keymaps,
}

#[derive(Serialize, Deserialize)]
struct TomlConfig {
    keys: Option<HashMap<String, HashMap<String, String>>>,
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let toml_config: TomlConfig = toml::from_str(&s)?;
        Ok(Self {
            keymaps: toml_config
                .keys
                .map(|r| parse_keymaps(&r))
                .transpose()?
                .unwrap_or_else(Keymaps::default),
        })
    }
}
