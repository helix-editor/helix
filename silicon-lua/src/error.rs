use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LuaConfigError {
    #[error("Lua error: {0}")]
    Lua(#[from] mlua::Error),

    #[error("Config validation error: {0}")]
    Validation(String),

    #[error("No config file found")]
    NotFound,

    #[error("TOML config detected at {0}. Lua config (init.lua) expected.")]
    TomlDetected(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
