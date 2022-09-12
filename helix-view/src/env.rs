use std::{fmt::Display, path::PathBuf};

use crate::Document;
pub struct Env {
    path: Option<PathBuf>,
}
impl Env {
    pub fn for_document(doc: &Document) -> Self {
        Env {
            path: doc.get_path(),
        }
    }
    pub fn for_path(path: Option<PathBuf>) -> Self {
        Env { path }
    }
    pub fn inject_into<T>(&self, strs: T) -> Vec<String>
    where
        T: Iterator,
        <T as Iterator>::Item: Display,
    {
        let path = self
            .path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or_default();

        strs.map(|s| s.to_string().replace("$file", path)).collect()
    }
}
