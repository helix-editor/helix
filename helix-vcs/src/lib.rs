use anyhow::{bail, Result};
use arc_swap::ArcSwap;
use std::{path::Path, sync::Arc};

#[cfg(feature = "git")]
pub use git::Git;
#[cfg(not(feature = "git"))]
pub use Dummy as Git;

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, FileChange, Hunk};

pub trait DiffProvider {
    /// Returns the data that a diff should be computed against
    /// if this provider is used.
    /// The data is returned as raw byte without any decoding or encoding performed
    /// to ensure all file encodings are handled correctly.
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>>;

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>>;

    fn get_changed_files(&self, cwd: &Path)
        -> Result<Box<dyn Iterator<Item = Result<FileChange>>>>;
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Dummy;
impl DiffProvider for Dummy {
    fn get_diff_base(&self, _file: &Path) -> Result<Vec<u8>> {
        bail!("helix was compiled without git support")
    }

    fn get_current_head_name(&self, _file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        bail!("helix was compiled without git support")
    }

    fn get_changed_files(
        &self,
        _cwd: &Path,
    ) -> Result<Box<dyn Iterator<Item = Result<FileChange>>>> {
        anyhow::bail!("dummy diff provider")
    }
}

impl From<Dummy> for DiffProviderImpls {
    fn from(value: Dummy) -> Self {
        DiffProviderImpls::Dummy(value)
    }
}

#[derive(Clone)]
pub struct DiffProviderRegistry {
    providers: Vec<DiffProviderImpls>,
}

impl DiffProviderRegistry {
    pub fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>> {
        self.providers
            .iter()
            .find_map(|provider| match provider.get_diff_base(file) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::info!("{err:#?}");
                    log::info!("failed to open diff base for {}", file.display());
                    None
                }
            })
    }

    pub fn get_current_head_name(&self, file: &Path) -> Option<Arc<ArcSwap<Box<str>>>> {
        self.providers
            .iter()
            .find_map(|provider| match provider.get_current_head_name(file) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::info!("{err:#?}");
                    log::info!("failed to obtain current head name for {}", file.display());
                    None
                }
            })
    }

    pub fn get_changed_files(
        &self,
        cwd: &Path,
    ) -> Result<Box<dyn Iterator<Item = Result<FileChange>>>> {
        self.providers
            .iter()
            .find_map(|provider| provider.get_changed_files(cwd).ok())
            .ok_or_else(|| anyhow::anyhow!("no diff provider returns success"))
    }
}

impl Default for DiffProviderRegistry {
    fn default() -> Self {
        // currently only git is supported
        // TODO make this configurable when more providers are added
        let providers = vec![Git.into()];
        DiffProviderRegistry { providers }
    }
}

/// A union type that includes all types that implement [DiffProvider]. We need this type to allow
/// cloning [DiffProviderRegistry] as `Clone` cannot be used in trait objects (or use `dyn-clone`?).
#[derive(Clone)]
pub enum DiffProviderImpls {
    Dummy(Dummy),
    #[cfg(feature = "git")]
    Git(Git),
}

impl DiffProvider for DiffProviderImpls {
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>> {
        match self {
            Self::Dummy(inner) => inner.get_diff_base(file),
            #[cfg(feature = "git")]
            Self::Git(inner) => inner.get_diff_base(file),
        }
    }

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        match self {
            Self::Dummy(inner) => inner.get_current_head_name(file),
            #[cfg(feature = "git")]
            Self::Git(inner) => inner.get_current_head_name(file),
        }
    }

    fn get_changed_files(
        &self,
        cwd: &Path,
    ) -> Result<Box<dyn Iterator<Item = Result<FileChange>>>> {
        match self {
            Self::Dummy(inner) => inner.get_changed_files(cwd),
            #[cfg(feature = "git")]
            Self::Git(inner) => inner.get_changed_files(cwd),
        }
    }
}
