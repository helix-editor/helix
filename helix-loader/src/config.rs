use std::str::from_utf8;

/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
///
/// This includes workspace-local config from `.helix/languages.toml`.
/// Use `user_lang_config_trusted` if you need to control workspace config loading.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    user_lang_config_trusted(true)
}

/// User configured languages.toml file, with optional workspace config.
///
/// If `include_workspace` is false, only the global user config is loaded,
/// ignoring any `.helix/languages.toml` in the workspace.
pub fn user_lang_config_trusted(include_workspace: bool) -> Result<toml::Value, toml::de::Error> {
    let mut paths = vec![crate::config_dir()];

    if include_workspace {
        paths.push(crate::find_workspace().0.join(".helix"));
    }

    let config = paths
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
