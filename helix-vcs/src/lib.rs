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

    pub fn needs_reload(&self, fs_event: &helix_core::file_watcher::Event) -> bool {
        self.providers
            .iter()
            .any(|provider| provider.needs_reload(fs_event))
    }

    /// Get paths that need to be watched for VCS state changes.
    /// These are paths like HEAD files that indicate branch/commit changes.
    /// The workspace path is used to determine if the VCS metadata is external.
    pub fn get_watched_paths(&self, workspace: &Path) -> Vec<PathBuf> {
        self.providers
            .iter()
            .filter_map(|provider| provider.get_watched_path(workspace))
            .collect()
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
    pub fn needs_reload(&self, fs_event: &helix_core::file_watcher::Event) -> bool {
        match self {
            #[cfg(feature = "git")]
            DiffProvider::Git => {
                let path = fs_event.path.as_std_path();
                // Check for regular .git/HEAD
                if path.ends_with(".git/HEAD") {
                    return true;
                }
                // Check for worktree HEAD at .git/worktrees/<name>/HEAD
                if path.file_name().is_some_and(|f| f == "HEAD") {
                    // Walk up the path to check for .git/worktrees pattern
                    if let Some(parent) = path.parent() {
                        if let Some(grandparent) = parent.parent() {
                            if grandparent.ends_with(".git/worktrees") {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            DiffProvider::None => false,
        }
    }

    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_diff_base(file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_current_head_name(file),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    fn for_each_changed_file(
        &self,
        cwd: &Path,
        f: impl Fn(Result<FileChange>) -> bool,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::for_each_changed_file(cwd, f),
            Self::None => bail!("No diff support compiled in"),
        }
    }

    /// Get the path to watch for VCS state changes (e.g., HEAD file).
    fn get_watched_path(&self, workspace: &Path) -> Option<PathBuf> {
        match self {
            #[cfg(feature = "git")]
            Self::Git => git::get_head_path(workspace),
            Self::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "git")]
    #[test]
    fn test_needs_reload_regular_git() {
        use helix_core::file_watcher::{CanonicalPathBuf, Event, EventType};
        use std::path::Path;

        let provider = DiffProvider::Git;

        // Regular .git/HEAD should trigger reload
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new("/home/user/repo/.git/HEAD")),
            ty: EventType::Modified,
        };
        assert!(provider.needs_reload(&event));

        // Other .git files should not trigger reload
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new("/home/user/repo/.git/config")),
            ty: EventType::Modified,
        };
        assert!(!provider.needs_reload(&event));
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_needs_reload_worktree_head() {
        use helix_core::file_watcher::{CanonicalPathBuf, Event, EventType};
        use std::path::Path;

        let provider = DiffProvider::Git;

        // Worktree HEAD at .git/worktrees/<name>/HEAD should trigger reload
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new(
                "/home/user/main-repo/.git/worktrees/my-worktree/HEAD",
            )),
            ty: EventType::Modified,
        };
        assert!(provider.needs_reload(&event));

        // Nested worktree name should also work
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new(
                "/home/user/main-repo/.git/worktrees/feature-branch/HEAD",
            )),
            ty: EventType::Modified,
        };
        assert!(provider.needs_reload(&event));

        // Non-HEAD files in worktrees should not trigger reload
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new(
                "/home/user/main-repo/.git/worktrees/my-worktree/index",
            )),
            ty: EventType::Modified,
        };
        assert!(!provider.needs_reload(&event));

        // HEAD files not in .git/worktrees should not trigger reload
        let event = Event {
            path: CanonicalPathBuf::assert_canonicalized(Path::new(
                "/home/user/other/worktrees/my-worktree/HEAD",
            )),
            ty: EventType::Modified,
        };
        assert!(!provider.needs_reload(&event));
    }
}
