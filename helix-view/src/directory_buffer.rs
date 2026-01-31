use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectorySort {
    TypeThenNameAsc,
    TypeThenNameDesc,
    NameAsc,
    NameDesc,
}

impl Default for DirectorySort {
    fn default() -> Self {
        Self::TypeThenNameAsc
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub relative_path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct DirectoryBufferState {
    pub root: PathBuf,
    pub entries: Vec<DirectoryEntry>,
    pub show_hidden: bool,
    pub sort: DirectorySort,
    pub delete_to_trash: bool,
}
