use std::path::{Path, PathBuf};

pub enum FileChange {
    Untracked {
        path: PathBuf,
    },
    Modified {
        path: PathBuf,
    },
    Conflict {
        path: PathBuf,
    },
    Deleted {
        path: PathBuf,
    },
    Renamed {
        from_path: PathBuf,
        to_path: PathBuf,
    },
}

impl FileChange {
    pub fn path(&self) -> &Path {
        match self {
            Self::Untracked { path } => path,
            Self::Modified { path } => path,
            Self::Conflict { path } => path,
            Self::Deleted { path } => path,
            Self::Renamed { to_path, .. } => to_path,
        }
    }
}
