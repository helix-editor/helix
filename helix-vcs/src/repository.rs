use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::Branch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepositoryProvider {
    #[cfg(feature = "git")]
    Git,
    #[allow(dead_code)]
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    provider: RepositoryProvider,
    work_dir: PathBuf,
}

impl Repository {
    #[cfg(feature = "git")]
    pub(crate) fn new(provider: RepositoryProvider, work_dir: PathBuf) -> Self {
        Self { provider, work_dir }
    }

    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    pub fn branches(&self) -> Result<Vec<Branch>> {
        match self.provider {
            #[cfg(feature = "git")]
            RepositoryProvider::Git => crate::git::branches(self.work_dir()),
            RepositoryProvider::None => bail!("No VCS support compiled in"),
        }
    }

    pub fn switch_branch(&self, _branch: &Branch) -> Result<()> {
        match self.provider {
            #[cfg(feature = "git")]
            RepositoryProvider::Git => crate::git::switch_branch(self.work_dir(), _branch),
            RepositoryProvider::None => bail!("No VCS support compiled in"),
        }
    }
}
