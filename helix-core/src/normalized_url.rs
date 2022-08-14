//! [`NormalizedUrl`]s are used to make comparisons more logical for URLs, notably those holding
//! Windows paths.
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::path::{Path, PathBuf};

use url::Url;

/// Normalize a [`Url`] to make it comparable across idiosyncracies from different tools.
///
/// # Example case
///
/// Helix gets paths from other sources, like LSPs. In such a case, Helix can't know in advance for
/// every source what the case will be for the prefix component on Windows (`C:` or `c:`). Since
/// this can hinder comparisons, especially in [`Url`]s, this structure gives access to the opposite
/// casing of the `Url` and uses it to compare.
#[derive(Debug, Clone)]
pub struct NormalizedUrl {
    base: Url,
    /// When appropriate, this contains the base URL with the drive letter in the opposite case.
    ///
    /// `file://C:/my/windows/path` in `base` means this is `Some(file://c:/my/windows/path)`.
    windows_normalized: Option<Url>,
}

impl NormalizedUrl {
    #[inline]
    pub fn new(base: Url) -> Self {
        Self {
            windows_normalized: windows_drive_opposite_case(&base),
            base,
        }
    }

    /// Inspired from [`Url::from_file_path()`] but will attempt to canonicalize the path before
    /// building the normalized URL
    pub fn from_file_path(path: &Path) -> Option<Self> {
        let path: Cow<'_, Path> = if path.is_absolute() {
            Cow::Borrowed(path)
        } else {
            Cow::Owned(crate::path::get_canonicalized_path(path).ok()?)
        };

        // If we managed to canonicalize, this should never fail else its a bug
        let base = Url::from_file_path(&path).unwrap();
        Some(Self::new(base))
    }

    /// Access the `Url` stored as the base of the normalized one.
    ///
    /// If you have the possibility to consume `self` because it's not used later, consider using
    /// [`Self::into_base()`], especially if you are cloning after the current `.base()` call.
    pub fn as_base(&self) -> &Url {
        &self.base
    }

    /// See [`Url::path()`]
    pub fn path(&self) -> &str {
        self.base.path()
    }

    /// See [`Url::to_file_path()`]
    #[allow(clippy::result_unit_err)]
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        self.base.to_file_path()
    }

    /// See [`Url::path_segments()`]
    pub fn path_segments(&self) -> Option<std::str::Split<'_, char>> {
        self.base.path_segments()
    }

    /// Consumes the normalized URL to return the base url given on construction.
    ///
    /// If only a reference is needed, use [`Self::as_base()`].
    pub fn into_base(self) -> Url {
        self.base
    }
}

impl PartialEq<Url> for NormalizedUrl {
    #[inline]
    fn eq(&self, other: &Url) -> bool {
        self.base.eq(other)
            || self
                .windows_normalized
                .as_ref()
                .map_or(false, |x| x.eq(other))
    }
}

impl PartialEq<NormalizedUrl> for Url {
    #[inline]
    fn eq(&self, other: &NormalizedUrl) -> bool {
        other.eq(self)
    }
}

impl PartialEq<Self> for NormalizedUrl {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.eq(&other.base)
    }
}

impl Eq for NormalizedUrl {}

impl PartialOrd<Self> for NormalizedUrl {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.base.partial_cmp(&other.base)
    }
}

impl Ord for NormalizedUrl {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.base.cmp(&other.base)
    }
}

impl PartialOrd<Url> for NormalizedUrl {
    #[inline]
    fn partial_cmp(&self, other: &Url) -> Option<Ordering> {
        self.base.partial_cmp(other)
    }
}

impl PartialOrd<NormalizedUrl> for Url {
    #[inline]
    fn partial_cmp(&self, other: &NormalizedUrl) -> Option<Ordering> {
        self.partial_cmp(&other.base)
    }
}

impl From<Url> for NormalizedUrl {
    #[inline]
    fn from(base: Url) -> Self {
        Self::new(base)
    }
}

impl fmt::Display for NormalizedUrl {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.fmt(f)
    }
}

/// Find if there is a Windows drive in the URL's path and invert its case if present. Returns
/// `None` if there is nothing to do.
///
/// If `base.scheme()` is not `"file"` this will return `None`.
fn windows_drive_opposite_case(base: &Url) -> Option<Url> {
    if base.scheme() != "file" {
        return None;
    }

    let base_path = base.path();

    let p = match base_path.as_bytes() {
        [b'/', drive_letter, b':', b'/', ..] if drive_letter.is_ascii_alphabetic() => {
            let mut p = base_path.to_owned();
            // Safety: we're accessing the second byte, which we know exists, to replace
            // it with another ASCII character so we're not creating invalid UTF-8
            let sl = unsafe { p.as_bytes_mut() };
            sl[1] = switch_drive_letter_case(drive_letter);
            p
        }
        [b'/', drive_letter, b':'] if drive_letter.is_ascii_alphabetic() => {
            let c = char::from(switch_drive_letter_case(drive_letter));
            format!("/{}:", c)
        }
        _ => return None,
    };

    let mut s = base.clone();
    s.set_path(&p);
    Some(s)
}

/// Switch the case of the given ASCII char in `a..=z` to `A..=Z` and conversely.
///
/// If the given ASCII character is not a letter, returns it unchanged.
fn switch_drive_letter_case(c: &u8) -> u8 {
    match c {
        b'A'..=b'Z' => c.to_ascii_lowercase(),
        b'a'..=b'z' => c.to_ascii_uppercase(),
        x => *x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_drive_letter_works() {
        // 256 integers to test, might as well test them all
        for c in 0..=u8::MAX {
            let switched = switch_drive_letter_case(&c);

            if c.is_ascii_lowercase() {
                assert_eq!(c.to_ascii_uppercase(), switched);
            } else if c.is_ascii_uppercase() {
                assert_eq!(c.to_ascii_lowercase(), switched);
            } else {
                assert_eq!(c, switched);
            }
        }
    }

    #[test]
    fn opposite_windows_drive_case() {
        let a = Url::parse("https://example.com/C:/test/path").unwrap();
        assert_eq!(windows_drive_opposite_case(&a), None);

        let a = Url::parse("file://my/unix/path").unwrap();
        assert_eq!(windows_drive_opposite_case(&a), None);

        let a = Url::parse("file://C:/my/windows/path").unwrap();
        let b = Url::parse("file://c:/my/windows/path").unwrap();
        assert_eq!(windows_drive_opposite_case(&a), Some(b));
    }

    #[test]
    fn normalized_url_new() {
        let a = Url::parse("https://example.com/test/path").unwrap();
        assert_eq!(
            NormalizedUrl::new(a.clone()),
            NormalizedUrl {
                base: a,
                windows_normalized: None,
            }
        );

        let a = Url::parse("file://unix/test/path").unwrap();
        assert_eq!(
            NormalizedUrl::new(a.clone()),
            NormalizedUrl {
                base: a,
                windows_normalized: None,
            }
        );

        let a = Url::parse("file://C:/test/path").unwrap();
        let b = Url::parse("file://c:/test/path").unwrap();
        assert_eq!(
            NormalizedUrl::new(a.clone()),
            NormalizedUrl {
                base: a,
                windows_normalized: Some(b),
            }
        );
    }

    #[test]
    fn different_drive_letters_compare_equal() {
        let a = Url::parse("file://C:/my/path").unwrap();
        let b = Url::parse("file://c:/my/path").unwrap();

        // Here the two URLs are different but the normalized forms are equal
        assert_ne!(a, b);
        assert_eq!(NormalizedUrl::new(a), NormalizedUrl::new(b));

        let a = Url::parse("file:///C:/my/path").unwrap();
        let b = Url::parse("file:///c:/my/path").unwrap();

        assert_ne!(a, b);
        assert_eq!(NormalizedUrl::new(a), NormalizedUrl::new(b));
    }

    #[test]
    fn same_drive_letters_compare_equal() {
        let a = Url::parse("file://C:/my/path").unwrap();
        let b = Url::parse("file://C:/my/path").unwrap();

        assert_eq!(a, b);
        assert_eq!(NormalizedUrl::new(a), NormalizedUrl::new(b));

        let a = Url::parse("file:///C:/my/path").unwrap();
        let b = Url::parse("file:///C:/my/path").unwrap();

        assert_eq!(a, b);
        assert_eq!(NormalizedUrl::new(a), NormalizedUrl::new(b));
    }

    #[test]
    fn not_drive_letters_compare_different() {
        let a = Url::parse("file:///C:test/my/path").unwrap();
        let b = Url::parse("file:///c:test/my/path").unwrap();

        assert_ne!(a, b);
        assert_ne!(NormalizedUrl::new(a), NormalizedUrl::new(b));
    }

    #[test]
    fn same_unix_path_compare_equal() {
        let a = Url::parse("file:///my/unix/path").unwrap();
        let b = Url::parse("file:///my/unix/path").unwrap();

        assert_eq!(a, b);
        assert_eq!(NormalizedUrl::new(a), NormalizedUrl::new(b));
    }

    #[test]
    fn different_unix_path_compare_different() {
        let a = Url::parse("file:///My/unix/path").unwrap();
        let b = Url::parse("file:///my/unix/path").unwrap();

        assert_ne!(a, b);
        assert_ne!(NormalizedUrl::new(a), NormalizedUrl::new(b));
    }

    // Ensure the comparison result is the same as the original URL when
    // it's not a file scheme
    #[test]
    fn non_file_scheme_ignores_normalization() {
        let a = Url::parse("https:///C:/some/path").unwrap();
        let b = Url::parse("https:///c:/some/path").unwrap();

        assert_eq!(a == b, NormalizedUrl::new(a) == NormalizedUrl::new(b));

        let a = Url::parse("data:text/plain,My Super Path").unwrap();
        let b = Url::parse("data:text/plain,My Super Data").unwrap();

        assert_eq!(a == b, NormalizedUrl::new(a) == NormalizedUrl::new(b));
    }
}
