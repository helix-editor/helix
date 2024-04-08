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

pub fn lang_config() -> PathBuf {
    project_root().join("languages.toml")
}
