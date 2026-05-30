//! `helix_vcs` provides editor-facing Version Control System (VCS) integration.
//! Currently `git` is the only supported provider, but this architecture allows
//! more providers to be added in the future.

use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "git")]
mod git;

mod branch;
mod diff;
mod repository;

pub use branch::{Branch, BranchKind};
pub use diff::{DiffHandle, Hunk};
pub use repository::Repository;

mod status;

pub use status::FileChange;

/// Contains all active VCS providers. Providers are compiled in via features. Currently only
/// `git` is supported.
#[derive(Clone)]
pub struct VcsProviderRegistry {
    providers: Vec<VcsProvider>,
}

/// Compatibility alias for existing diff-specific callers.
pub type DiffProviderRegistry = VcsProviderRegistry;

impl VcsProviderRegistry {
    /// Get the given file from the VCS. This provides the unedited document as a "base"
    /// for a diff to be created.
    pub fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>> {
        self.providers
            .iter()
            .find_map(|provider| match provider.get_diff_base(file) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to open diff base for {}", file.display());
                    None
                }
            })
    }

    /// Get the current name of the current [HEAD](https://stackoverflow.com/questions/2304087/what-is-head-in-git).
    pub fn get_current_head_name(&self, file: &Path) -> Option<Arc<ArcSwap<Box<str>>>> {
        self.providers
            .iter()
            .find_map(|provider| match provider.get_current_head_name(file) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to obtain current head name for {}", file.display());
                    None
                }
            })
    }

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file(
        self,
        cwd: PathBuf,
        f: impl Fn(Result<FileChange>) -> bool + Send + 'static,
    ) {
        tokio::task::spawn_blocking(move || {
            if self
                .providers
                .iter()
                .find_map(|provider| provider.for_each_changed_file(&cwd, &f).ok())
                .is_none()
            {
                f(Err(anyhow!("no diff provider returns success")));
            }
        });
    }

    /// Find the repository containing `cwd`.
    pub fn repository(&self, cwd: &Path) -> Result<Repository> {
        let mut last_err = None;
        for provider in &self.providers {
            match provider.repository(cwd) {
                Ok(repository) => return Ok(repository),
                Err(err) => {
                    if last_err.is_none() || !matches!(provider, VcsProvider::None) {
                        last_err = Some(err);
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("no VCS provider returns success")))
    }
}

impl Default for VcsProviderRegistry {
    fn default() -> Self {
        // currently only git is supported
        // TODO make this configurable when more providers are added
        let providers = vec![
            #[cfg(feature = "git")]
            VcsProvider::Git,
            VcsProvider::None,
        ];
        VcsProviderRegistry { providers }
    }
}

/// A union type that includes all supported VCS providers. We need this type to allow cloning
/// [VcsProviderRegistry] as `Clone` cannot be used in trait objects.
///
/// `Copy` is simply to ensure the `clone()` call is the simplest it can be.
#[derive(Copy, Clone)]
enum VcsProvider {
    #[cfg(feature = "git")]
    Git,
    None,
}

impl VcsProvider {
    fn get_diff_base(&self, _file: &Path) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_diff_base(_file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn get_current_head_name(&self, _file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_current_head_name(_file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn for_each_changed_file(
        &self,
        _cwd: &Path,
        _f: impl Fn(Result<FileChange>) -> bool,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::for_each_changed_file(_cwd, _f),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn repository(&self, _cwd: &Path) -> Result<Repository> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::repository(_cwd),
            Self::None => bail!("No VCS support compiled in"),
        }
    }
}
