use std::fmt::Display;

use crate::Document;

pub fn inject_environment<T>(strs: T, doc: &Document) -> Vec<String>
where
    T: Iterator,
    <T as Iterator>::Item: Display,
{
    let path = doc.path().and_then(|p| p.to_str()).unwrap_or_else(|| {
        log::error!("No $path found for document: {:?}", doc.url());
        "ENV:NO_PATH"
    });
    let pwd = std::env::current_dir()
        .unwrap_or_default()
        .into_os_string()
        .into_string()
        .unwrap_or_else(|err| {
            log::error!("No $pwd found for: {:?}", err);
            "ENV:NO_PATH".to_string()
        });

    strs.map(|s| s.to_string().replace("$file", path).replace("$pwd", &pwd))
        .collect()
}
