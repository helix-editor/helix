use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

/// A generic pointer to a file location.
///
/// Currently this type only supports paths to local files.
///
/// Cloning this type is cheap: the internal representation uses an Arc.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Uri {
    File(Arc<Path>),
}

impl Uri {
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UriParseError {
    source: String,
    kind: UriParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriParseErrorKind {
    UnsupportedScheme(String),
    MalformedUri,
}

impl fmt::Display for UriParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            UriParseErrorKind::UnsupportedScheme(scheme) => {
                write!(f, "unsupported scheme '{scheme}' in URI {}", self.source)
            }
            UriParseErrorKind::MalformedUri => {
                write!(
                    f,
                    "unable to convert malformed URI to file path: {}",
                    self.source
                )
            }
        }
    }
}

impl std::error::Error for UriParseError {}

impl FromStr for Uri {
    type Err = UriParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use std::ffi::OsStr;
        #[cfg(any(unix, target_os = "redox"))]
        use std::os::unix::prelude::OsStrExt;
        #[cfg(target_os = "wasi")]
        use std::os::wasi::prelude::OsStrExt;

        let Some((scheme, rest)) = s.split_once("://") else {
            return Err(Self::Err {
                source: s.to_string(),
                kind: UriParseErrorKind::MalformedUri,
            });
        };

        if scheme != "file" {
            return Err(Self::Err {
                source: s.to_string(),
                kind: UriParseErrorKind::UnsupportedScheme(scheme.to_string()),
            });
        }

        // Assert there is no query or fragment in the URI.
        if s.find(['?', '#']).is_some() {
            return Err(Self::Err {
                source: s.to_string(),
                kind: UriParseErrorKind::MalformedUri,
            });
        }

        let mut bytes = Vec::new();
        bytes.extend(percent_encoding::percent_decode(rest.as_bytes()));
        Ok(PathBuf::from(OsStr::from_bytes(&bytes)).into())
    }
}

impl TryFrom<&str> for Uri {
    type Error = UriParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unknown_scheme() {
        let uri = "csharp://metadata/foo/barBaz.cs";
        assert_eq!(
            uri.parse::<Uri>(),
            Err(UriParseError {
                source: uri.to_string(),
                kind: UriParseErrorKind::UnsupportedScheme("csharp".to_string()),
            })
        );
    }
}
