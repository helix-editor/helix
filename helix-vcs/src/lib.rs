mod git;
mod rope;

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

pub use git::Git;

// TODO: Move to helix_core once we have a generic diff mode
#[derive(Copy, Clone, Debug)]
pub enum LineDiff {
    Added,
    Deleted,
    Modified,
}

/// Maps line numbers to changes
pub type LineDiffs = HashMap<usize, LineDiff>;

pub type RepoRoot = PathBuf;

#[derive(Debug, Default)]
pub struct Registry {
    inner: HashMap<RepoRoot, Rc<RefCell<Git>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discover_from_path(&mut self, file: &Path) -> Option<Rc<RefCell<Git>>> {
        let cached_root = self.inner.keys().find(|root| file.starts_with(root));
        match cached_root {
            Some(root) => self.inner.get(root).cloned(),
            None => {
                let repo = Git::discover_from_path(file)?;
                let root = repo.root().to_path_buf();
                let repo = Rc::new(RefCell::new(repo));
                self.inner.insert(root, Rc::clone(&repo));
                Some(repo)
            }
        }
    }
}
