use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "git")]
pub use git::Git;
#[cfg(not(feature = "git"))]
pub use Dummy as Git;

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, Hunk};

mod status;

pub use status::FileChange;

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Dummy;
impl Dummy {
    fn get_diff_base(&self, _file: &Path) -> Result<Vec<u8>> {
        bail!("helix was compiled without git support")
    }

    fn get_current_head_name(&self, _file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        bail!("helix was compiled without git support")
    }

    fn for_each_changed_file(
        &self,
        _cwd: &Path,
        _f: impl Fn(Result<FileChange>) -> bool,
    ) -> Result<()> {
        bail!("helix was compiled without git support")
    }
}

impl From<Dummy> for DiffProvider {
    fn from(value: Dummy) -> Self {
        DiffProvider::Dummy(value)
    }
}

#[derive(Clone)]
pub struct DiffProviderRegistry {
    providers: Vec<DiffProvider>,
}

impl DiffProviderRegistry {
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
/// cloning [DiffProviderRegistry] as `Clone` cannot be used in trait objects.
#[derive(Clone)]
pub enum DiffProvider {
    Dummy(Dummy),
    #[cfg(feature = "git")]
    Git(Git),
}

impl DiffProvider {
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

    fn for_each_changed_file(
        &self,
        cwd: &Path,
        f: impl Fn(Result<FileChange>) -> bool,
    ) -> Result<()> {
        match self {
            Self::Dummy(inner) => inner.for_each_changed_file(cwd, f),
            #[cfg(feature = "git")]
            Self::Git(inner) => inner.for_each_changed_file(cwd, f),
        }
    }
}
