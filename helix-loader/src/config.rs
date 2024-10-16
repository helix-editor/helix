use std::str::from_utf8;
use std::path::PathBuf;

/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

fn merge_language_config(
    left: toml::Value, file: PathBuf,
) -> Result<toml::Value, toml::de::Error> {
    let right = std::fs::read_to_string(file).ok()
        .map(|c| toml::from_str(&c)).transpose()?;

    let config = match right {
        Some(right) => crate::merge_toml_values(left, right, 3),
        None => left,
    };

    Ok(config)
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let global = merge_language_config(default_lang_config(), crate::lang_config_file())?;

    let config = match global.get("workspace-config").and_then(|v| v.as_bool()) {
        Some(true) => merge_language_config(global, crate::workspace_lang_config_file())?,
        _ => global,
    };

    Ok(config)
}
