use arc_swap::ArcSwap;
use std::{path::Path, sync::Arc};

use anyhow::{bail, Result};

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

    fn get_changed_files(&self, cwd: &Path) -> Result<Vec<FileChange>>;
}

#[doc(hidden)]
pub struct Dummy;
impl DiffProvider for Dummy {
    fn get_diff_base(&self, _file: &Path) -> Result<Vec<u8>> {
        bail!("helix was compiled without git support")
    }

    fn get_current_head_name(&self, _file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        bail!("helix was compiled without git support")
    }

    fn get_changed_files(&self, _cwd: &Path) -> Result<Vec<FileChange>> {
        anyhow::bail!("dummy diff provider")
    }
}

pub struct DiffProviderRegistry {
    providers: Vec<Box<dyn DiffProvider>>,
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

    pub fn get_changed_files(&self, cwd: &Path) -> Result<Vec<FileChange>> {
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
        let git: Box<dyn DiffProvider> = Box::new(Git);
        let providers = vec![git];
        DiffProviderRegistry { providers }
    }
}
