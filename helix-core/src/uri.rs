use std::{
    fmt,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

// uses NonZeroUsize so Option<DocumentId> takes the same space
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentId(NonZeroUsize);

impl DocumentId {
    pub const MAX: Self = Self(unsafe { NonZeroUsize::new_unchecked(usize::MAX) });

    pub fn next(&self) -> Self {
        // Safety: adding 1 from 1 is fine, probably impossible to reach usize max
        Self(unsafe { NonZeroUsize::new_unchecked(self.0.get() + 1) })
    }
}

impl Default for DocumentId {
    fn default() -> DocumentId {
        // Safety: 1 is non-zero
        DocumentId(unsafe { NonZeroUsize::new_unchecked(1) })
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A generic pointer to a file location.
///
/// Cloning this type is cheap: the internal representation uses an Arc or data which is Copy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Uri {
    File(Arc<Path>),
    Scratch(DocumentId),
}

impl Uri {
    // This clippy allow mirrors url::Url::from_file_path
    #[allow(clippy::result_unit_err)]
    pub fn to_url(&self) -> Result<url::Url, ()> {
        match self {
            Uri::File(path) => url::Url::from_file_path(path),
            Uri::Scratch(_) => Err(()),
        }
    }

    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Scratch(_) => None,
        }
    }
}

impl From<PathBuf> for Uri {
    fn from(path: PathBuf) -> Self {
        Self::File(path.into())
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) => write!(f, "{}", path.display()),
            Self::Scratch(id) => write!(f, "[scratch {id}]"),
        }
    }
}

#[derive(Debug)]
pub struct UrlConversionError {
    source: url::Url,
    kind: UrlConversionErrorKind,
}

#[derive(Debug)]
pub enum UrlConversionErrorKind {
    UnsupportedScheme,
    UnableToConvert,
}

impl fmt::Display for UrlConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            UrlConversionErrorKind::UnsupportedScheme => {
                write!(
                    f,
                    "unsupported scheme '{}' in URL {}",
                    self.source.scheme(),
                    self.source
                )
            }
            UrlConversionErrorKind::UnableToConvert => {
                write!(f, "unable to convert URL to file path: {}", self.source)
            }
        }
    }
}

impl std::error::Error for UrlConversionError {}

fn convert_url_to_uri(url: &url::Url) -> Result<Uri, UrlConversionErrorKind> {
    if url.scheme() == "file" {
        url.to_file_path()
            .map(|path| Uri::File(helix_stdx::path::normalize(path).into()))
            .map_err(|_| UrlConversionErrorKind::UnableToConvert)
    } else {
        Err(UrlConversionErrorKind::UnsupportedScheme)
    }
}

impl TryFrom<url::Url> for Uri {
    type Error = UrlConversionError;

    fn try_from(url: url::Url) -> Result<Self, Self::Error> {
        convert_url_to_uri(&url).map_err(|kind| Self::Error { source: url, kind })
    }
}

impl TryFrom<&url::Url> for Uri {
    type Error = UrlConversionError;

    fn try_from(url: &url::Url) -> Result<Self, Self::Error> {
        convert_url_to_uri(url).map_err(|kind| Self::Error {
            source: url.clone(),
            kind,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use url::Url;

    #[test]
    fn unknown_scheme() {
        let url = Url::parse("csharp:/metadata/foo/bar/Baz.cs").unwrap();
        assert!(matches!(
            Uri::try_from(url),
            Err(UrlConversionError {
                kind: UrlConversionErrorKind::UnsupportedScheme,
                ..
            })
        ));
    }
}
