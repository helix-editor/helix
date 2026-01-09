use std::str::from_utf8;

/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let config = [
        crate::config_dir(),
        crate::find_workspace().0.join(".helix"),
    ]
    .into_iter()
    .map(|path| path.join("languages.toml"))
    .filter_map(|file| {
        std::fs::read_to_string(file)
            .map(|config| toml::from_str(&config))
            .ok()
    })
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .fold(default_lang_config(), |a, b| {
        crate::merge_toml_values(a, b, 3)
    });

    Ok(config)
}

/// Default built-in auto-pairs.toml.
pub fn default_auto_pairs_config() -> toml::Value {
    let default_config = include_bytes!("../../auto-pairs.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in auto-pairs.toml to valid toml")
}

/// Load auto-pairs config, merged with user overrides.
///
/// Priority (lowest to highest):
/// 1. Built-in auto-pairs.toml (embedded)
/// 2. User ~/.config/helix/auto-pairs.toml
/// 3. Workspace .helix/auto-pairs.toml
///
/// Note: Explicit `auto-pairs` in languages.toml takes precedence over all of these.
pub fn auto_pairs_config() -> Result<toml::Value, toml::de::Error> {
    let config = [
        crate::config_dir(),
        crate::find_workspace().0.join(".helix"),
    ]
    .into_iter()
    .map(|path| path.join("auto-pairs.toml"))
    .filter_map(|file| {
        std::fs::read_to_string(file)
            .map(|config| toml::from_str(&config))
            .ok()
    })
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .fold(default_auto_pairs_config(), |a, b| {
        // Merge at depth 1: language sections replace entirely
        crate::merge_toml_values(a, b, 1)
    });

    Ok(config)
}
