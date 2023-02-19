use std::path::Path;

use anyhow::Result;

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
    fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>>;

    fn get_changed_files(&self, cwd: &Path) -> Result<Vec<FileChange>>;
}

#[doc(hidden)]
pub struct Dummy;
impl DiffProvider for Dummy {
    fn get_diff_base(&self, _file: &Path) -> Option<Vec<u8>> {
        None
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
            .find_map(|provider| provider.get_diff_base(file))
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
