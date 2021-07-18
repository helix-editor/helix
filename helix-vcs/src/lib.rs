use std::collections::HashMap;
use std::path::Path;

mod git;
use git::Git;

#[derive(Copy, Clone, Debug)]
pub enum LineChange {
    Added,
    RemovedAbove,
    RemovedBelow,
    Modified,
}
impl LineChange {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineChange::Added => &"▌",
            LineChange::RemovedAbove => &"▘",
            LineChange::RemovedBelow => &"▖",
            LineChange::Modified => &"▐",
        }
    }
}

pub type LineChanges = HashMap<usize, LineChange>;

//#[derive(Clone)]
pub enum VCS {
    Git(Git),
}
impl VCS {
    pub fn from_path(filename: &Path) -> Option<Self> {
        Some(VCS::Git(Git::from_path(filename)?))
    }
    pub fn get_line_changes(&self) -> Option<&LineChanges> {
        match self {
            VCS::Git(git) => git.line_changes.as_ref(),
        }
    }
    pub fn diff(&mut self) {
        match self {
            VCS::Git(git) => git.diff(),
        }
    }
}
