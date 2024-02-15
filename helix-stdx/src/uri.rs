use std::path::PathBuf;

use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum FilePathError<'a> {
    #[error("unsupported scheme in URI: {0}")]
    UnsupportedScheme(&'a Url),
    #[error("unable to convert URI to file path: {0}")]
    UnableToConvert(&'a Url),
}

/// Converts a [`Url`] into a [`PathBuf`].
///
/// Unlike [`Url::to_file_path`], this method respects the uri's scheme
/// and returns `Ok(None)` if the scheme was not "file".
pub fn uri_to_file_path(uri: &Url) -> Result<PathBuf, FilePathError> {
    if uri.scheme() == "file" {
        uri.to_file_path()
            .map_err(|_| FilePathError::UnableToConvert(uri))
    } else {
        Err(FilePathError::UnsupportedScheme(uri))
    }
}
