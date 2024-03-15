use anyhow::Result;
use arc_swap::ArcSwap;
use std::{path::Path, sync::Arc};

#[cfg(feature = "git")]
mod git;

mod diff;
pub use diff::{DiffHandle, Hunk};

pub mod config;

pub trait DiffProvider {
    /// Returns the data that a diff should be computed against
    /// if this provider is used.
    /// The data is returned as raw byte without any decoding or encoding performed
    /// to ensure all file encodings are handled correctly.
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>>;
    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>>;
}

pub struct DiffProviderRegistry {
    /// Built from the list in the user configuration.
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
}

impl From<&config::Vcs> for DiffProviderRegistry {
    fn from(value: &config::Vcs) -> Self {
        fn mapper(p: &config::Provider) -> Box<dyn DiffProvider> {
            match p {
                #[cfg(feature = "git")]
                config::Provider::Git => Box::new(git::Git),
            }
        }

        Self {
            providers: value.providers.iter().map(mapper).collect(),
        }
    }
}
