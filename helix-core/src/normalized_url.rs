//! [`NormalizedUrl`]s are used to make comparisons more logical for URLs, notably those holding
//! Windows paths.
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
#[cfg(windows)]
use std::path::{Component, Prefix};
use std::path::{Path, PathBuf};

use url::Url;

/// Normalize a [`Url`] to make it comparable across idiosyncracies from different tools.
///
/// # Example case
///
/// Helix gets paths from other sources, like LSPs. In such a case, Helix can't know in advance for
/// every source what the case will be for the prefix component on Windows (`C:` or `c:`). Since
/// this can hinder comparisons, especially in [`Url`]s, this structure gives access to the opposite
/// casing of the `Url` on Windows and uses it to compare.
#[derive(Debug, Clone)]
pub struct NormalizedUrl {
    base: Url,
    /// This field is only available on Windows since differences in casing on other OSes are not
    /// limited to the prefix and should be dealt with differently.
    #[cfg(windows)]
    opposite_case: Option<Url>,
}

impl NormalizedUrl {
    pub fn from_file_path(path: &Path) -> Option<Self> {
        let path: Cow<'_, Path> = if path.is_absolute() {
            Cow::Borrowed(path)
        } else {
            Cow::Owned(crate::path::get_canonicalized_path(path).ok()?)
        };
        // If we managed to canonicalize, this should never fail else its a bug
        let base = Url::from_file_path(&path).unwrap();
        #[cfg(windows)]
        let opposite_case =
            path_prefix_opposite_casing(&path).map(|x| Url::from_file_path(&x).unwrap());

        Some(Self {
            base,
            #[cfg(windows)]
            opposite_case,
        })
    }

    /// Access the `Url` stored as the base of the normalized one
    pub fn base(&self) -> &Url {
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
}

impl PartialEq<Self> for NormalizedUrl {
    fn eq(&self, other: &Self) -> bool {
        // Since the other fields are derived from this one deterministically, we can simply
        // compare `.base` to know if two `NormalizedUrl`s are equal.
        self.base.eq(&other.base)
    }
}

impl Eq for NormalizedUrl {}

impl PartialOrd<Self> for NormalizedUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.base.partial_cmp(&other.base)
    }
}

impl Ord for NormalizedUrl {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base.cmp(&other.base)
    }
}

impl fmt::Display for NormalizedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.fmt(f)
    }
}

// Interactions with the original Url type are below

impl From<Url> for NormalizedUrl {
    fn from(u: Url) -> Self {
        let path = Path::new(u.path());
        Self::from_file_path(path).expect("URL failed to convert to absolute path")
    }
}

impl PartialEq<Url> for NormalizedUrl {
    #[cfg(windows)]
    fn eq(&self, other: &Url) -> bool {
        self.base.eq(other) || self.opposite_case.as_ref().map_or(false, |oc| oc.eq(other))
    }

    #[cfg(not(windows))]
    fn eq(&self, other: &Url) -> bool {
        self.base.eq(other)
    }
}

impl PartialOrd<Url> for NormalizedUrl {
    fn partial_cmp(&self, other: &Url) -> Option<Ordering> {
        self.base.partial_cmp(other)
    }
}

/// Utility function to get the opposite casing of a Windows path, if any
#[cfg(windows)]
fn path_prefix_opposite_casing(default_case_path: &Path) -> Option<PathBuf> {
    let mut components = default_case_path.components();
    let first_comp = components.next()?;

    match first_comp {
        Component::Prefix(prefix)
        // Anything else than `Disk` variants is not subject to this casing issue
        if matches!(prefix.kind(), Prefix::VerbatimDisk(_) | Prefix::Disk(_)) => {
            let cased_prefix = {
                // Hard to get data on this, but most drive letter should be uppercased by
                // default so lowercasing first means not having to then uppercase ?
                let mut prefix_str = prefix.as_os_str().to_ascii_lowercase();
                if prefix_str == prefix.as_os_str() {
                    prefix_str.make_ascii_uppercase();
                }
                prefix_str
            };

            // The `push`s should be free thanks to this
            let mut updated_case_path = PathBuf::with_capacity(default_case_path.as_os_str().len());
            updated_case_path.push(&cased_prefix);
            for cp in components {
                updated_case_path.push(cp.as_os_str());
            }
            Some(updated_case_path)
        },
        _ => None,
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;

    #[test]
    fn different_drives_case_are_equal() {
        let p1 = Path::new("C:\\one\\path\\Here");
        let p2 = Path::new("c:\\one\\path\\Here");

        assert_eq!(p1, p2);

        let norm = NormalizedUrl::new(p1).unwrap();
        let u = Url::from_file_path(p2).unwrap();

        assert_eq!(norm, u);
    }
}
