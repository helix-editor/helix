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
        let mut vec = vec![];
        strs.into_iter().for_each(|s| {
            vec.push(
                s.to_string().replace(
                    "$file",
                    self.path
                        .clone()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default(),
                ),
            );
        });
        log::debug!("injected: {:#?}", vec);
        vec
    }
}
