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

/// A document's diff base: the contents of the file at the base revision.
#[derive(Debug)]
pub struct DiffBase {
    pub content: Vec<u8>,
    /// Set when the requested revision could not be resolved and `content` was read from
    /// `HEAD` instead.
    ///
    /// Without this the fallback would be invisible: the gutter would quietly show changes
    /// against a revision the user did not ask for. It is a flag rather than a message
    /// because only the caller knows what was requested and how to phrase it.
    pub used_fallback: bool,
}

/// Contains all active diff providers. Diff providers are compiled in via features. Currently
/// only `git` is supported.
#[derive(Clone)]
pub struct DiffProviderRegistry {
    providers: Vec<DiffProvider>,
}

impl DiffProviderRegistry {
    /// Get the given file from the VCS as it is at `revision`. This provides the unedited
    /// document as a "base" for a diff to be created.
    ///
    /// `revision` is whatever the provider understands as naming a point in history; see the
    /// provider's own documentation for the syntax it accepts.
    pub fn get_diff_base(&self, file: &Path, revision: &str, trust_full: bool) -> Option<DiffBase> {
        self.providers.iter().find_map(|provider| {
            match provider.get_diff_base(file, revision, trust_full) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to open diff base for {}", file.display());
                    None
                }
            }
        })
    }

    /// Get the current name of the current [HEAD](https://stackoverflow.com/questions/2304087/what-is-head-in-git).
    pub fn get_current_head_name(
        &self,
        file: &Path,
        trust_full: bool,
    ) -> Option<Arc<ArcSwap<Box<str>>>> {
        self.providers.iter().find_map(|provider| {
            match provider.get_current_head_name(file, trust_full) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to obtain current head name for {}", file.display());
                    None
                }
            }
        })
    }

    /// Checks that `revision` names a point in history the repository containing `cwd` can be
    /// diffed against, so that a bad value can be rejected before it is applied.
    ///
    /// Unlike the other methods this reports the error instead of logging it: a provider that
    /// cannot make sense of the revision at all must not look like "this file has no changes".
    pub fn validate_diff_base_revision(
        &self,
        cwd: &Path,
        revision: &str,
        trust_full: bool,
    ) -> Result<()> {
        let mut last_err = anyhow!("no diff provider returns success");
        for provider in &self.providers {
            match provider.validate_diff_base_revision(cwd, revision, trust_full) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    /// Revisions that can be used as a diff base in the repository containing `cwd`, named the
    /// way the provider displays them.
    pub fn get_revisions(&self, cwd: &Path, trust_full: bool) -> Option<Vec<String>> {
        self.providers
            .iter()
            .find_map(|provider| match provider.get_revisions(cwd, trust_full) {
                Ok(res) => Some(res),
                Err(err) => {
                    log::debug!("{err:#?}");
                    log::debug!("failed to list revisions in {}", cwd.display());
                    None
                }
            })
    }

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file(
        self,
        cwd: PathBuf,
        trust_full: bool,
        f: impl Fn(Result<FileChange>) -> bool + Send + 'static,
    ) {
        tokio::task::spawn_blocking(move || {
            if self
                .providers
                .iter()
                .find_map(|provider| provider.for_each_changed_file(&cwd, trust_full, &f).ok())
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
/// Every method below dispatches with an exhaustive `match`, so a provider added here cannot
/// compile until it has decided what to do about `editor.diff-base`. A provider with no notion
/// of a revision must return an error rather than silently ignoring the setting.
///
/// `Copy` is simply to ensure the `clone()` call is the simplest it can be.
#[derive(Copy, Clone)]
enum DiffProvider {
    #[cfg(feature = "git")]
    Git,
    None,
}

impl DiffProvider {
    fn get_diff_base(&self, file: &Path, revision: &str, trust_full: bool) -> Result<DiffBase> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_diff_base(file, revision, trust_full),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn validate_diff_base_revision(
        &self,
        cwd: &Path,
        revision: &str,
        trust_full: bool,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::validate_diff_base_revision(cwd, revision, trust_full),
            Self::None => bail!("`editor.diff-base` is not supported without diff support"),
        }
    }

    fn get_revisions(&self, cwd: &Path, trust_full: bool) -> Result<Vec<String>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_revisions(cwd, trust_full),
            Self::None => bail!("listing revisions is not supported without diff support"),
        }
    }

    fn get_current_head_name(
        &self,
        file: &Path,
        trust_full: bool,
    ) -> Result<Arc<ArcSwap<Box<str>>>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_current_head_name(file, trust_full),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn for_each_changed_file(
        &self,
        cwd: &Path,
        trust_full: bool,
        f: impl Fn(Result<FileChange>) -> bool,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::for_each_changed_file(cwd, trust_full, f),
            Self::None => bail!("No diff support compiled in"),
        }
    }
}
