//! `helix_vcs` provides types for working with diffs from a Version Control System (VCS).
//! Currently `git` is the only supported provider for diffs, but this architecture allows
//! for other providers to be added in the future.

use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, Hunk};

mod status;

pub use status::FileChange;

/// Contains all active diff providers. Diff providers are compiled in via features. Currently
/// only `git` is supported.
#[derive(Clone)]
pub struct DiffProviderRegistry {
    providers: Vec<DiffProvider>,
}

impl DiffProviderRegistry {
    /// Get the given file from the VCS. This provides the unedited document as a "base"
    /// for a diff to be created.
    pub fn get_diff_base(&self, file: &Path, diff_base_revision: Option<&str>) -> Option<Vec<u8>> {
        self.providers.iter().find_map(|provider| {
            match provider.get_diff_base(file, diff_base_revision) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to open diff base for {}", file.display());
                    None
                }
            }
        })
    }

    pub fn get_repo_root(&self, file: &Path) -> Result<PathBuf> {
        let mut last_err = None;
        for provider in &self.providers {
            match provider.get_repo_root(file) {
                Ok(repo_root) => return Ok(repo_root),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to resolve repo root for {}", file.display());
                    last_err = Some(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("no diff provider returns success")))
    }

    pub fn ensure_diff_base(&self, file: &Path, diff_base_revision: &str) -> Result<()> {
        let mut last_err = None;
        for provider in &self.providers {
            match provider.ensure_diff_base(file, diff_base_revision) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!(
                        "failed to validate diff base '{}' for {}",
                        diff_base_revision,
                        file.display()
                    );
                    last_err = Some(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("no diff provider returns success")))
    }

    /// Get the current name of the current [HEAD](https://stackoverflow.com/questions/2304087/what-is-head-in-git).
    pub fn get_current_head_name(&self, file: &Path) -> Option<Arc<ArcSwap<Box<str>>>> {
        self.providers.iter().find_map(|provider| {
            match provider.get_current_head_name(file) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to obtain current head name for {}", file.display());
                    None
                }
            }
        })
    }

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file(
        self,
        cwd: PathBuf,
        diff_base_revision: Option<String>,
        mut f: impl FnMut(Result<FileChange>) -> bool + Send + 'static,
    ) {
        tokio::task::spawn_blocking(move || {
            if self
                .providers
                .iter()
                .find_map(|provider| {
                    provider
                        .for_each_changed_file(&cwd, diff_base_revision.as_deref(), &mut f)
                        .ok()
                })
                .is_none()
            {
                f(Err(anyhow!("no diff provider returns success")));
            }
        });
    }
}

impl Default for DiffProviderRegistry {
    fn default() -> Self {
        // currently only git is supported
        // TODO make this configurable when more providers are added
        let providers = vec![
            #[cfg(feature = "git")]
            DiffProvider::Git,
            DiffProvider::None,
        ];
        DiffProviderRegistry { providers }
    }
}

/// A union type that includes all types that implement [DiffProvider]. We need this type to allow
/// cloning [DiffProviderRegistry] as `Clone` cannot be used in trait objects.
///
/// `Copy` is simply to ensure the `clone()` call is the simplest it can be.
#[derive(Copy, Clone)]
enum DiffProvider {
    #[cfg(feature = "git")]
    Git,
    None,
}

impl DiffProvider {
    fn get_diff_base(&self, file: &Path, diff_base_revision: Option<&str>) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_diff_base(file, diff_base_revision),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn get_repo_root(&self, _file: &Path) -> Result<PathBuf> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_repo_root(_file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn ensure_diff_base(&self, _file: &Path, _diff_base_revision: &str) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::ensure_diff_base(_file, _diff_base_revision),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn get_current_head_name(
        &self,
        file: &Path,
    ) -> Result<Arc<ArcSwap<Box<str>>>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_current_head_name(file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn for_each_changed_file(
        &self,
        cwd: &Path,
        diff_base_revision: Option<&str>,
        f: impl FnMut(Result<FileChange>) -> bool,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::for_each_changed_file(cwd, diff_base_revision, f),
            Self::None => bail!("No diff support compiled in"),
        }
    }
}
