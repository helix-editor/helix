use std::path::{Path, PathBuf};

use crate::paths;
use helix_core::syntax::Configuration as LangConfig;
use helix_term::health::TsFeature;

/// Get the list of languages that support a particular tree-sitter
/// based feature.
pub fn ts_lang_support(feat: TsFeature) -> Vec<String> {
    let queries_dir = paths::ts_queries();

    find_files(&queries_dir, feat.runtime_filename())
        .iter()
        .map(|f| {
            // .../helix/runtime/queries/python/highlights.scm
            let tail = f.strip_prefix(&queries_dir).unwrap(); // python/highlights.scm
            let lang = tail.components().next().unwrap(); // python
            lang.as_os_str().to_string_lossy().to_string()
        })
        .collect()
}

/// Get the list of languages that have any form of tree-sitter
/// queries defined in the runtime directory.
pub fn langs_with_ts_queries() -> Vec<String> {
    std::fs::read_dir(paths::ts_queries())
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .file_type()
                .ok()?
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().to_string())
        })
        .collect()
}

// naive implementation, but suffices for our needs
pub fn find_files(dir: &Path, filename: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_dir() {
                Some(find_files(&path, filename))
            } else {
                (path.file_name()?.to_string_lossy() == filename).then(|| vec![path])
            }
        })
        .flatten()
        .collect()
}

pub fn lang_config() -> LangConfig {
    let bytes = std::fs::read(paths::lang_config()).unwrap();
    toml::from_slice(&bytes).unwrap()
}
