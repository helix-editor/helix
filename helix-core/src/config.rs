/// Syntax configuration loader based on built-in languages.toml.
pub fn default_syntax_loader() -> crate::syntax::Configuration {
    helix_loader::config::default_lang_config()
        .try_into()
        .expect("Could not serialize built-in languages.toml")
}
/// Syntax configuration loader based on user configured languages.toml.
pub fn user_syntax_loader() -> Result<crate::syntax::Configuration, toml::de::Error> {
    helix_loader::config::user_lang_config()?.try_into()
}
