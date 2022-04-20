/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    toml::from_slice(include_bytes!("../../languages.toml"))
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let config = crate::local_config_dirs()
        .into_iter()
        .chain([crate::config_dir()].into_iter())
        .map(|path| path.join("languages.toml"))
        .filter_map(|file| {
            std::fs::read(&file)
                .map(|config| toml::from_slice(&config))
                .ok()
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .chain([default_lang_config()].into_iter())
        .fold(toml::Value::Table(toml::value::Table::default()), |a, b| {
            crate::merge_toml_values(b, a, false)
        });

    Ok(config)
}
