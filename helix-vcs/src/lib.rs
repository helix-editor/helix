use anyhow::Result;
use arc_swap::ArcSwap;
use std::{collections::HashMap, path::Path, sync::Arc};

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, Hunk};

mod status;

pub use status::FileChange;

#[derive(Default, Clone)]
pub struct DiffProviderRegistry {
    /// Repository root path mapped to their provider.
    ///
    /// When a root path cannot be found after having called `add_file`, it means there is no
    /// provider to speak of.
    providers: HashMap<Arc<Path>, DiffProvider>,
    /// Count the number of files added for a specific provider path.
    /// Providers themselves don't care about that, this is handled entirely in `Self::add_file`,
    /// without knowledge from the `Self::add_file_<provider>` methods.
    ///
    /// Note: it *could* happen that a provider for a path is changed without the number of
    /// associated files changing, e.g deleting a .git/ and initializing a .jj/ repo.
    counters: HashMap<Arc<Path>, u32>,
}

/// Diff-related methods
impl DiffProviderRegistry {
    pub fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>> {
        match self.provider_for(file)?.get_diff_base(file) {
            Ok(diff_base) => Some(diff_base),
            Err(err) => {
                log::debug!("{err:#?}");
                log::debug!("failed to open diff base for {}", file.display());
                None
            }
        }
    }

    pub fn get_current_head_name(&self, file: &Path) -> Option<Arc<ArcSwap<Box<str>>>> {
        match self.provider_for(file)?.get_current_head_name() {
            Ok(head_name) => Some(head_name),
            Err(err) => {
                log::debug!("{err:#?}");
                log::debug!("failed to obtain current head name for {}", file.display());
                None
            }
        }
    }

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file(
        self,
        cwd: Arc<Path>,
        f: impl Fn(Result<FileChange>) -> bool + Send + 'static,
    ) {
        tokio::task::spawn_blocking(move || {
            let Some(diff_provider) = self.provider_for(&cwd) else {
                return;
            };
            if let Err(err) = diff_provider.for_each_changed_file(&f) {
                f(Err(err));
            }
        });
    }
}

/// Creation and update methods
#[cfg_attr(not(feature = "git"), allow(unused))]
impl DiffProviderRegistry {
    /// Register a provider (if any is found) for the given path.
    pub fn add(&mut self, path: &Path) {
        let Some((repo_path, provider)) = get_possible_provider(path) else {
            // Do nothing here: there is no path to use and so the actual methods to get infos
            // like `get_diff_base` just won't do anything since they won't find a source to
            // work with.
            log::debug!("Found no potential diff provider for {}", path.display());
            // Note: if a `.<vcs>/` dir is deleted, we may end up in a situation where we lose track
            // of a now unused provider. This is acceptable because it doesn't happen that often in
            // practice and people can just reload to force an update.
            //
            // If it becomes an issue in the future, we could fix it by recomputing the providers
            // for each stored paths here.
            return;
        };

        let result = match provider {
            #[cfg(feature = "git")]
            PossibleDiffProvider::Git => self.add_file_git(repo_path),
        };

        match result {
            Ok((key, prov)) => {
                // Increase the count for this path.
                let count = self.counters.entry(key).or_default();
                let created = *count == 0;
                *count += 1;

                // Only log at info level when adding a new provider
                if created {
                    log::info!(
                        "Added {prov:?} (repo: {}) from {}",
                        repo_path.display(),
                        path.display()
                    )
                } else {
                    log::debug!(
                        "Reused {prov:?} (repo: {}) for {}",
                        repo_path.display(),
                        path.display()
                    );
                }
            }
            Err(err) => log::debug!(
                "Failed to open repo at {} for {}: {:?}",
                repo_path.display(),
                path.display(),
                err
            ),
        }
    }

    /// Reload the provider for the given path.
    pub fn reload(&mut self, path: &Path) {
        self.remove(path);
        self.add(path);
    }

    /// Remove the given path from the provider cache. If it was the last one using it, this will
    /// free up the provider.
    pub fn remove(&mut self, path: &Path) {
        let Some((repo_path, _)) = get_possible_provider(path) else {
            return;
        };

        let Some(count) = self.counters.get_mut(repo_path) else {
            return;
        };

        *count -= 1;
        if *count == 0 {
            // Cleanup the provider when the last user disappears
            self.counters.remove(repo_path);
            self.providers.remove(repo_path);

            // While reallocating is costly, in most sessions of Helix there will be one main
            // workspace and sometimes a jump to some temporary one (for example from a jump-to-def
            // in an LSP) that will be closed after some time. We want to avoid keeping unused
            // RAM for this.
            self.providers.shrink_to_fit();
            self.counters.shrink_to_fit();
        }
    }

    /// Clears the saved providers completely.
    pub fn reset(&mut self) {
        self.providers = Default::default();
        self.counters = Default::default();
    }
}

/// Private methods
impl DiffProviderRegistry {
    fn provider_for(&self, path: &Path) -> Option<&DiffProvider> {
        let path = get_possible_provider(path)?.0;
        self.providers.get(path)
    }

    /// Add the git repo to the known providers *if* it isn't already known.
    #[cfg(feature = "git")]
    fn add_file_git(&mut self, repo_path: &Path) -> Result<(Arc<Path>, PossibleDiffProvider)> {
        // Don't build a git repo object if there is already one for that path.
        if let Some((key, DiffProvider::Git(_))) = self.providers.get_key_value(repo_path) {
            return Ok((Arc::clone(key), PossibleDiffProvider::Git));
        }

        match git::open_repo(repo_path) {
            Ok(repo) => {
                let key = Arc::from(repo_path);
                self.providers
                    .insert(Arc::clone(&key), DiffProvider::Git(repo));
                Ok((key, PossibleDiffProvider::Git))
            }
            Err(err) => Err(err),
        }
    }
}

/// A union type that includes all types that implement [DiffProvider]. We need this type to allow
/// cloning [DiffProviderRegistry] as `Clone` cannot be used in trait objects.
///
/// `Copy` is simply to ensure the `clone()` call is the simplest it can be.
#[derive(Clone)]
pub enum DiffProvider {
    #[cfg(feature = "git")]
    Git(gix::ThreadSafeRepository),
}

#[cfg_attr(not(feature = "git"), allow(unused))]
impl DiffProvider {
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>> {
        // We need the */ref else we're matching on a reference and Rust considers all references
        // inhabited. In our case
        match *self {
            #[cfg(feature = "git")]
            Self::Git(ref repo) => git::get_diff_base(repo, file),
        }
    }

    fn get_current_head_name(&self) -> Result<Arc<ArcSwap<Box<str>>>> {
        match *self {
            #[cfg(feature = "git")]
            Self::Git(ref repo) => git::get_current_head_name(repo),
        }
    }

    fn for_each_changed_file(&self, f: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
        match *self {
            #[cfg(feature = "git")]
            Self::Git(ref repo) => git::for_each_changed_file(repo, f),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PossibleDiffProvider {
    /// Possibly a git repo rooted at the stored path (i.e. `<path>/.git` exists)
    #[cfg(feature = "git")]
    Git,
}

/// Does *possible* diff provider auto detection. Returns the 'root' of the workspace
///
/// We say possible because this function doesn't open the actual repository to check if that's
/// actually the case.
fn get_possible_provider(path: &Path) -> Option<(&Path, PossibleDiffProvider)> {
    if cfg!(feature = "git") {
        #[cfg_attr(not(feature = "git"), allow(unused))]
        fn check_path(path: &Path) -> Option<(&Path, PossibleDiffProvider)> {
            #[cfg(feature = "git")]
            if path.join(".git").try_exists().ok()? {
                return Some((path, PossibleDiffProvider::Git));
            }

            None
        }

        for parent in path.ancestors() {
            if let Some(path_and_provider) = check_path(parent) {
                return Some(path_and_provider);
            }
        }
    }

    None
}
