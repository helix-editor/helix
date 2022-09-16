use std::fmt::Display;

use crate::Document;

pub fn inject_environment<T>(strs: T, doc: &Document) -> Vec<String>
where
    T: Iterator,
    <T as Iterator>::Item: Display,
{
    let path = doc.path().and_then(|p| p.to_str()).unwrap_or_default();

    strs.map(|s| s.to_string().replace("$file", path)).collect()
}
