use std::{
    collections::HashMap,
    fs::File,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock},
};

use fs2::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{data_dir, ensure_parent_dir, is_workspace, state_dir};
use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum SimpleDbError {
    #[error("Couldn't deserialize database: {}", .0)]
    Deserialize(#[from] toml::de::Error),
    #[error("Couldn't serialize database: {}", .0)]
    Serialize(#[from] toml::ser::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, SimpleDbError>;
/// A simple file-backed database which is cached in memory.
/// It supports concurrent writes, however, the cache won't update itself after a write from another process.
/// It is optimized mostly for reading. Writing is expensive.
struct SimpleDb<T> {
    path: PathBuf,
    db: OnceLock<ArcSwap<T>>,
}

impl<T: Default + DeserializeOwned + Serialize> SimpleDb<T> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        ensure_parent_dir(path.as_ref());
        Self {
            path: path.as_ref().to_path_buf(),
            db: OnceLock::new(),
        }
    }
    fn lock(&self) -> Result<File> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("lock"))?;
        file.lock_exclusive()?;
        Ok(file)
    }

    fn read(&self) -> Result<T> {
        Ok(match std::fs::read_to_string(&self.path) {
            Ok(s) => toml::from_str(&s)?,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    T::default()
                } else {
                    return Err(e.into());
                }
            }
        })
    }

    pub fn sync_cache(&self) -> Result<Arc<T>> {
        let db = Arc::new(self.read()?);
        let arc = self.db.get_or_init(|| Arc::clone(&db).into());
        arc.store(Arc::clone(&db));
        Ok(db)
    }

    pub fn inspect<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Some(db) = self.db.get() {
            Ok(f(db.load().as_ref()))
        } else {
            let db = self.sync_cache()?;
            Ok(f(db.as_ref()))
        }
    }

    pub fn modify<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let _lock = self.lock()?;
        let mut db = self.read()?;
        let r = f(&mut db);
        let toml_updated = toml::to_string(&db)?;
        let mut tmp = if let Some(parent) = self.path.parent() {
            tempfile::NamedTempFile::new_in(parent)
        } else {
            tempfile::NamedTempFile::new()
        }?;
        // atomically ensure that the file is always valid
        tmp.write_all(toml_updated.as_bytes())?;
        tmp.as_file().sync_data()?;
        tmp.persist(&self.path)
            .map_err(Into::<std::io::Error>::into)?;
        // we could go even further here and fsync the directory, but data loss isn't that important

        let db = Arc::new(db);
        let arc = self.db.get_or_init(|| Arc::clone(&db).into());
        arc.store(db);
        Ok(r)
    }
}

static TRUST_DB: LazyLock<SimpleDb<TrustDb>> = LazyLock::new(|| SimpleDb::new(trust_db_file()));

#[derive(Serialize, Deserialize, Default)]
struct TrustDb {
    trust: Option<HashMap<PathBuf, Trust>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum Trust {
    Trusted,
    Untrusted,
}

impl TrustDb {
    fn is_workspace_trusted(&self, path: impl AsRef<Path>) -> Option<bool> {
        let path = helix_stdx::path::canonicalize(path);
        self.trust.as_ref().and_then(|map| {
            path.ancestors().find_map(|path| {
                if is_workspace(path) || path.is_file() {
                    map.get(path).map(|trust| matches!(trust, Trust::Trusted))
                } else {
                    None
                }
            })
        })
    }

    fn set_trust(&mut self, path: impl AsRef<Path>, trust: Trust) -> bool {
        let path = helix_stdx::path::canonicalize(path);
        self.trust.get_or_insert(HashMap::new()).insert(path, trust) != Some(trust)
    }
}

fn trust_db_file() -> PathBuf {
    state_dir().unwrap_or(data_dir()).join("trust_db.toml")
}

/// check if the workspace is trusted. If the result is Ok(None) it implies that the path does not exist in the database.
pub fn is_workspace_trusted(path: impl AsRef<Path>) -> Result<Option<bool>> {
    TRUST_DB.inspect(|db| db.is_workspace_trusted(path))
}

/// Set the trust of a path. If the result is Ok, it returns true if the path was newly changed to the value of trust.
pub fn set_trust(path: impl AsRef<Path>, trust: Trust) -> Result<bool> {
    TRUST_DB.modify(|db| db.set_trust(path, trust))
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;

    use super::*;
    #[test]
    fn trust() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("trust_db.toml");
        let db = SimpleDb::<TrustDb>::new(db_path.clone());
        let some_path = dir.path().join("file.py");
        std::fs::write(&some_path, "# this is needed for .is_file() to return true").unwrap();
        assert_eq!(
            db.inspect(|db| db.is_workspace_trusted(&some_path))
                .unwrap(),
            None
        );
        assert_eq!(
            db.modify(|db| { db.set_trust(&some_path, Trust::Untrusted) })
                .unwrap(),
            true
        );
        assert_eq!(
            db.modify(|db| { db.set_trust(&some_path, Trust::Untrusted) })
                .unwrap(),
            false,
        );
        assert_eq!(
            db.inspect(|db| db.is_workspace_trusted(&some_path))
                .unwrap(),
            Some(false)
        );
        assert_eq!(
            db.modify(|db| db.set_trust(&some_path, Trust::Trusted))
                .unwrap(),
            true
        );
        assert_eq!(
            db.inspect(|db| db.is_workspace_trusted(&some_path))
                .unwrap(),
            Some(true)
        );
    }
}
