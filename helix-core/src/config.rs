use crate::merge_toml_values;

/// Default bultin-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    toml::from_slice(include_bytes!("../../languages.toml"))
        .expect("Could not parse bultin-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let def_lang_conf = default_lang_config();
    let data = std::fs::read(crate::config_dir().join("languages.toml"));
    let user_lang_conf = match data {
        Ok(raw) => {
            let value = toml::from_slice(&raw)?;
            merge_toml_values(def_lang_conf, value)
        }
        Err(_) => def_lang_conf,
    };

    Ok(user_lang_conf)
}

/// Syntax configuration loader based on built-in languages.toml.
pub fn default_syntax_loader() -> crate::syntax::Configuration {
    default_lang_config()
        .try_into()
        .expect("Could not serialize built-in language.toml")
}
/// Syntax configuration loader based on user configured languages.toml.
pub fn user_syntax_loader() -> Result<crate::syntax::Configuration, toml::de::Error> {
    user_lang_config()?.try_into()
}
