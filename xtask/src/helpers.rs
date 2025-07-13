use std::path::{Path, PathBuf};

use crate::path;
use helix_term::health::TsFeature;

/// Get the list of languages that support a particular tree-sitter
/// based feature.
pub fn ts_lang_support(feat: TsFeature) -> Vec<String> {
    let queries_dir = path::ts_queries();

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

// naive implementation, but suffices for our needs
pub fn find_files(dir: &Path, filename: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_dir() {
                Some(find_files(&path, filename))
            } else if path.file_name()?.to_string_lossy() == filename {
                Some(vec![path])
            } else {
                None
            }
        })
        .flatten()
        .collect()
}
