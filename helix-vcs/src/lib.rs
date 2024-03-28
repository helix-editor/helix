use anyhow::{bail, Result};
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

pub trait DiffProvider {
    /// Returns the data that a diff should be computed against
    /// if this provider is used.
    /// The data is returned as raw byte without any decoding or encoding performed
    /// to ensure all file encodings are handled correctly.
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>>;

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>>;

    /// Returns `Err` in case of an _initialization_ failure. Iteration errors must be reported via
    /// `on_err` instead.
    fn for_each_changed_file<FC, FE>(&self, cwd: &Path, on_change: FC, on_err: FE) -> Result<()>
    where
        FC: Fn(FileChange) -> bool,
        FE: Fn(anyhow::Error);
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

    fn for_each_changed_file<FC, FE>(&self, _cwd: &Path, _on_item: FC, _on_err: FE) -> Result<()>
    where
        FC: Fn(FileChange) -> bool,
        FE: Fn(anyhow::Error),
    {
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

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file<FC, FE>(self, cwd: PathBuf, on_change: FC, on_err: FE)
    where
        FC: Fn(FileChange) -> bool + Clone + Send + 'static,
        FE: Fn(anyhow::Error) + Clone + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            if self
                .providers
                .iter()
                .find_map(|provider| {
                    provider
                        .for_each_changed_file(&cwd, on_change.clone(), on_err.clone())
                        .ok()
                })
                .is_none()
            {
                on_err(anyhow::anyhow!("no diff provider returns success"))
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

    fn for_each_changed_file<FC, FE>(&self, cwd: &Path, on_change: FC, on_err: FE) -> Result<()>
    where
        FC: Fn(FileChange) -> bool,
        FE: Fn(anyhow::Error),
    {
        match self {
            Self::Dummy(inner) => inner.for_each_changed_file(cwd, on_change, on_err),
            #[cfg(feature = "git")]
            Self::Git(inner) => inner.for_each_changed_file(cwd, on_change, on_err),
        }
    }
}
