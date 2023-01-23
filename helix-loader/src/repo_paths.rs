use std::path::{Path, PathBuf};

pub fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn book_gen() -> PathBuf {
    project_root().join("book/src/generated/")
}

pub fn ts_queries() -> PathBuf {
    project_root().join("runtime/queries")
}

pub fn themes() -> PathBuf {
    project_root().join("runtime/themes")
}

pub fn default_config_dir() -> PathBuf {
    // TODO: would be nice to move config files away from project root folder
    project_root()
}

pub fn default_lang_configs() -> PathBuf {
    default_config_dir().join("languages.toml")
}

pub fn default_theme() -> PathBuf {
    default_config_dir().join("theme.toml")
}

pub fn default_base16_theme() -> PathBuf {
    default_config_dir().join("base16_theme.toml")
}
