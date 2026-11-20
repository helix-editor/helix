//! A minimal RFC3986 URI type backed by a string.
//!
//! We only care to convert between `file://` URIs and `PathBuf`, so we don't need a fully
//! featured URL/URI crate. Also, LSP uses RFC3986 rather than WHATWG, and the two have different
//! percent encoding rules. Encoding follows RFC3986 (percent-encode everything outside `pchar`),
//! not the WHATWG URL rules the `url` crate implements. Some language servers are strict about
//! this (e.g. Deno): they reject unescaped `[`/`]` in paths (valid in WHATWG but not RFC3986).

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use percent_encoding::{percent_decode, percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The set of bytes percent-encoded in a path. RFC3986 allows `pchar`
/// (`unreserved` / `sub-delims` / `:` / `@`) plus `/` as the separator
/// unescaped; everything else (and every non-ASCII byte) is encoded.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    // unreserved: ALPHA / DIGIT (already kept) / "-" / "." / "_" / "~"
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    // sub-delims
    .remove(b'!')
    .remove(b'$')
    .remove(b'&')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    // pchar extras and the path separator
    .remove(b':')
    .remove(b'@')
    .remove(b'/');

/// An RFC3986 URI.
///
/// The URI is stored verbatim and only interpreted when a `file://` path is actually needed.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Url(String);

/// Error returned when a string is not a valid absolute URI.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid URI: relative URL without a base")
    }
}

impl std::error::Error for ParseError {}

impl Url {
    /// Parse an absolute URI. Mirrors `url::Url::parse`'s rejection of relative
    /// references: the input must begin with a valid scheme (`ALPHA *( ALPHA /
    /// DIGIT / "+" / "-" / "." ) ":"`).
    pub fn parse(input: &str) -> Result<Url, ParseError> {
        let colon = input.find(':').ok_or(ParseError)?;
        let scheme = &input[..colon];
        let mut bytes = scheme.bytes();
        let valid = bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
            && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
        if valid {
            Ok(Url(input.to_string()))
        } else {
            Err(ParseError)
        }
    }

    /// The full URI as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The scheme (the part before the first `:`), e.g. `file`.
    pub fn scheme(&self) -> &str {
        match self.0.find(':') {
            Some(i) => &self.0[..i],
            None => "",
        }
    }

    /// The (still percent-encoded) path component, matching `url::Url::path`.
    pub fn path(&self) -> &str {
        let after_scheme = match self.0.find(':') {
            Some(i) => &self.0[i + 1..],
            None => self.0.as_str(),
        };
        // Skip an `//authority` component if present.
        let after_authority = match after_scheme.strip_prefix("//") {
            Some(rest) => match rest.find('/') {
                Some(i) => &rest[i..],
                None => "",
            },
            None => after_scheme,
        };
        let end = after_authority
            .find(['?', '#'])
            .unwrap_or(after_authority.len());
        &after_authority[..end]
    }

    /// Build a `file://` URI from an absolute filesystem path.
    #[allow(clippy::result_unit_err)]
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(());
        }
        let mut serialization = String::from("file://");
        serialize_path(&mut serialization, path)?;
        Ok(Url(serialization))
    }

    /// Like [`Url::from_file_path`], but ensures a trailing slash so the URI
    /// denotes a directory.
    #[allow(clippy::result_unit_err)]
    pub fn from_directory_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut url = Url::from_file_path(path)?;
        if !url.0.ends_with('/') {
            url.0.push('/');
        }
        Ok(url)
    }

    /// Convert a `file://` URI back to a filesystem path.
    #[allow(clippy::result_unit_err)]
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        // Schemes are case-insensitive (RFC3986 §3.1), so accept any casing.
        // The scheme is `file`, so it is always 5 bytes (`file:`) regardless of
        // case, which keeps the slice below valid.
        if !self.scheme().eq_ignore_ascii_case("file") {
            return Err(());
        }
        let rest = self.0["file:".len()..].strip_prefix("//").ok_or(())?;
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        let local_host = authority.is_empty() || authority.eq_ignore_ascii_case("localhost");

        #[cfg(not(windows))]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            if !local_host {
                return Err(());
            }
            let bytes = percent_decode(path.as_bytes()).collect::<Vec<u8>>();
            if bytes.is_empty() {
                return Err(());
            }
            Ok(PathBuf::from(OsStr::from_bytes(&bytes)))
        }
        #[cfg(windows)]
        {
            let decoded = percent_decode(path.as_bytes())
                .decode_utf8()
                .map_err(|_| ())?;
            if local_host {
                // `/C:/dir/file` -> `C:\dir\file`
                let path = decoded.strip_prefix('/').unwrap_or(&decoded);
                if path.is_empty() {
                    return Err(());
                }
                Ok(PathBuf::from(path.replace('/', "\\")))
            } else {
                // UNC: `file://server/share/...` -> `\\server\share\...`
                Ok(PathBuf::from(format!(
                    "\\\\{}{}",
                    authority,
                    decoded.replace('/', "\\")
                )))
            }
        }
    }
}

#[cfg(not(windows))]
fn serialize_path(out: &mut String, path: &Path) -> Result<(), ()> {
    use std::os::unix::ffi::OsStrExt;
    // The path is absolute, so it begins with `/`; percent-encode it while
    // preserving the `/` separators (they are excluded from `PATH`).
    out.extend(percent_encode(path.as_os_str().as_bytes(), PATH));
    Ok(())
}

#[cfg(windows)]
fn serialize_path(out: &mut String, path: &Path) -> Result<(), ()> {
    use std::path::{Component, Prefix};
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {
                // `C:` -> `/C:`
                out.push('/');
                out.push_str(&prefix.as_os_str().to_string_lossy());
            }
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                // `\\server\share` -> `//server/share` (authority + first seg)
                out.pop(); // drop one `/` from the `file://` we were appended to
                out.push_str(&server.to_string_lossy());
                out.push('/');
                out.extend(percent_encode(share.to_string_lossy().as_bytes(), PATH));
            }
            _ => return Err(()),
        },
        _ => return Err(()),
    }
    for component in components {
        match component {
            Component::RootDir => {}
            Component::Normal(seg) => {
                out.push('/');
                out.extend(percent_encode(seg.to_string_lossy().as_bytes(), PATH));
            }
            Component::CurDir => out.push_str("/."),
            Component::ParentDir => out.push_str("/.."),
            Component::Prefix(_) => return Err(()),
        }
    }
    Ok(())
}

impl FromStr for Url {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Url::parse(s)
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for Url {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Url {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Store the URI as-is. Path conversion is lazy.
        Ok(Url(String::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_and_path() {
        let url = Url::parse("file:///home/user/main.rs").unwrap();
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), "/home/user/main.rs");

        let url = Url::parse("csharp:/metadata/foo/Baz.cs").unwrap();
        assert_eq!(url.scheme(), "csharp");
        assert_eq!(url.path(), "/metadata/foo/Baz.cs");

        // authority present
        let url = Url::parse("file://host/path?q#frag").unwrap();
        assert_eq!(url.path(), "/path");
    }

    #[test]
    fn parse_rejects_relative() {
        assert!(Url::parse("src/main.rs").is_err());
        assert!(Url::parse("just text").is_err());
        assert!(Url::parse("https://example.com").is_ok());
    }

    #[cfg(not(windows))]
    #[test]
    fn file_path_round_trip() {
        for path in [
            "/home/user/main.rs",
            "/tmp/a b.txt",           // space
            "/tmp/[test]/x.ts",       // brackets (Deno RFC3986 fix)
            "/tmp/c#/Program.cs",     // '#'
            "/home/üser/café.txt",    // non-ASCII
            "/weird/100%/qu?ery&x=1", // '%', '?', '&'
        ] {
            let url = Url::from_file_path(path).unwrap();
            assert_eq!(url.to_file_path().unwrap(), PathBuf::from(path), "{}", url);
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn rfc3986_encoding() {
        let url = Url::from_file_path("/tmp/[test]/a b.ts").unwrap();
        // brackets and space are percent-encoded; '/' and unreserved are not
        assert_eq!(url.as_str(), "file:///tmp/%5Btest%5D/a%20b.ts");
    }

    #[cfg(not(windows))]
    #[test]
    fn directory_path_has_trailing_slash() {
        let url = Url::from_directory_path("/home/user").unwrap();
        assert_eq!(url.as_str(), "file:///home/user/");
    }

    #[test]
    fn from_file_path_rejects_relative() {
        assert!(Url::from_file_path("relative/path").is_err());
    }

    #[test]
    fn non_file_scheme_has_no_path() {
        assert!(Url::parse("untitled:foo").unwrap().to_file_path().is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn to_file_path_accepts_uppercase_scheme() {
        // Schemes are case-insensitive (RFC3986 §3.1); the `url` crate lowercased
        // them on parse, so a mixed-case `file` scheme must still resolve.
        for uri in ["FILE:///home/user/x.rs", "File:///home/user/x.rs"] {
            let url = Url::parse(uri).unwrap();
            assert_eq!(
                url.to_file_path().unwrap(),
                PathBuf::from("/home/user/x.rs"),
                "{uri}"
            );
        }
    }

    #[test]
    fn serde_is_opaque() {
        let url = Url::parse("file:///a/b.rs").unwrap();
        let json = serde_json::to_string(&url).unwrap();
        assert_eq!(json, "\"file:///a/b.rs\"");
        let back: Url = serde_json::from_str(&json).unwrap();
        assert_eq!(url, back);
    }
}
