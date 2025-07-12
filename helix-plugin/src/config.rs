
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub entrypoint: PathBuf,
    #[serde(default)]
    pub activation: Activation,
}

#[derive(Debug, Deserialize, Default)]
pub struct Activation {
    #[serde(default)]
    pub on_command: Vec<String>,
    #[serde(default)]
    pub on_language: Vec<String>,
    #[serde(default)]
    pub on_event: Vec<String>,
}

impl PluginManifest {
    pub fn load_from(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = toml::from_str(&content)?;
        Ok(manifest)
    }
}
