use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use std::fmt::Display;
use std::str::FromStr;
use std::{path::Path, sync::Arc};

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, Hunk};

/// Diff source for a file.
///
/// The one selected for each file is set on opening the file.
// TODO: provide a command to set the diff source for a file
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DiffSource {
    /// No diffs computations.
    #[default]
    None,
    /// Diffs are computed against the on-disk version of the file.
    File,
    /// Diff are computed from the in-tree version last registered in git.
    #[cfg(feature = "git")]
    Git,
}

pub type DiffHead = Arc<ArcSwap<Box<str>>>;

impl DiffSource {
    /// Auto detection of the diff source to use for a file.
    pub fn auto_detect(file: &Path) -> Self {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        #[cfg(feature = "git")]
        if let Some(parent) = file.parent() {
            if git::open_repo(parent).is_ok() {
                return Self::Git;
            }
        }

        Self::File
    }

    /// Returns the data that a diff should be computed against.
    ///
    /// The data is returned as raw bytes without any decoding or encoding performed
    /// to ensure all file encodings are handled correctly.
    pub fn get_diff_base(&self, file: &Path) -> Result<Option<Vec<u8>>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        match self {
            Self::None => Ok(None),
            Self::File => std::fs::read(file)
                .context("Failed to read file for diff base")
                .map(Some),
            #[cfg(feature = "git")]
            Self::Git => git::get_diff_base(file).map(Some),
        }
    }

    pub fn get_current_head_name(&self, file: &Path) -> Option<Result<DiffHead>> {
        match self {
            Self::None => None,
            Self::File => None,
            #[cfg(feature = "git")]
            Self::Git => Some(git::get_current_head_name(file)),
        }
    }
}

impl FromStr for DiffSource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(Self::None),
            "file" => Ok(Self::File),
            #[cfg(feature = "git")]
            "git" => Ok(Self::Git),
            s => bail!("invalid diff source '{s}', pick one of 'none', 'file' or 'git'"),
        }
    }
}

impl Display for DiffSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::File => "file",
            #[cfg(feature = "git")]
            Self::Git => "git",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_source_parse() {
        assert_eq!(DiffSource::from_str("none").unwrap(), DiffSource::None);
        assert_eq!(DiffSource::from_str("file").unwrap(), DiffSource::File);
        #[cfg(feature = "git")]
        assert_eq!(DiffSource::from_str("git").unwrap(), DiffSource::Git);

        assert!(DiffSource::from_str("Git").is_err());
        assert!(DiffSource::from_str("NONE").is_err());
        assert!(DiffSource::from_str("fIlE").is_err());
    }

    #[test]
    fn test_diff_source_display() {
        assert_eq!(DiffSource::None.to_string(), "none");
        assert_eq!(DiffSource::File.to_string(), "file");
        #[cfg(feature = "git")]
        assert_eq!(DiffSource::Git.to_string(), "git");
    }
}
