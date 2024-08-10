use crate::path;
use helix_core::syntax::Configuration as LangConfig;

pub fn lang_config() -> LangConfig {
    let text = std::fs::read_to_string(path::lang_config()).unwrap();
    toml::from_str(&text).unwrap()
}
