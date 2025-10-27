use std::{
    collections::HashMap,
    fs::File,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::{data_dir, ensure_parent_dir, is_workspace};

#[derive(Serialize, Deserialize, Default)]
struct TrustDb {
    trust: Option<HashMap<PathBuf, Trust>>,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum Trust {
    Trusted,
    Untrusted,
}

impl TrustDb {
    fn is_workspace_trusted(&self, path: impl AsRef<Path>) -> Option<bool> {
        self.trust.as_ref().and_then(|map| {
            path.as_ref().ancestors().find_map(|path| {
                if is_workspace(path) || path.is_file() {
                    map.get(path).map(|trust| matches!(trust, Trust::Trusted))
                } else {
                    None
                }
            })
        })
    }

    fn lock() -> std::io::Result<File> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(trust_db_lock_file())?;
        file.lock_exclusive()?;
        Ok(file)
    }

    fn inspect<F, R>(f: F) -> std::io::Result<R>
    where
        F: FnOnce(TrustDb) -> R,
    {
        let lock = TrustDb::lock()?;
        let contents = match std::fs::read_to_string(trust_db_file()) {
            Ok(s) => s,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    toml::to_string(&TrustDb::default()).unwrap()
                } else {
                    return Err(e);
                }
            }
        };
        let toml: TrustDb = toml::from_str(&contents).unwrap_or_else(|_| {
            panic!(
                "Trust database is corrupted. Try to fix {} or delete it",
                trust_db_file().display()
            )
        });
        let r = f(toml);
        drop(lock);
        Ok(r)
    }

    fn modify<F, R>(f: F) -> std::io::Result<R>
    where
        F: FnOnce(&mut TrustDb) -> R,
    {
        let lock = TrustDb::lock()?;
        let contents = match std::fs::read_to_string(trust_db_file()) {
            Ok(s) => s,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    toml::to_string(&TrustDb::default()).unwrap()
                } else {
                    return Err(e);
                }
            }
        };
        let mut toml: TrustDb = toml::from_str(&contents).unwrap_or_else(|_| {
            panic!(
                "Trust database is corrupted. Try to fix {} or delete it",
                trust_db_file().display()
            )
        });
        let r = f(&mut toml);
        let toml_updated =
            toml::to_string(&toml).expect("toml serialization of trust database failed?");
        std::fs::write(trust_db_file(), toml_updated)?;
        drop(lock);
        Ok(r)
    }
}

fn trust_db_file() -> PathBuf {
    data_dir().join("trust_db.toml")
}

fn trust_db_lock_file() -> PathBuf {
    trust_db_file().with_extension("lock")
}

pub fn is_workspace_trusted(path: impl AsRef<Path>) -> std::io::Result<Option<bool>> {
    let Ok(path) = path.as_ref().canonicalize() else {
        return Ok(Some(false));
    };
    TrustDb::inspect(|db| db.is_workspace_trusted(path))
}

pub fn trust_path(path: impl AsRef<Path>) -> std::io::Result<bool> {
    let Ok(path) = path.as_ref().canonicalize() else {
        return Ok(false);
    };
    TrustDb::modify(|db| {
        db.trust
            .get_or_insert(HashMap::new())
            .insert(path, Trust::Trusted)
            != Some(Trust::Trusted)
    })
}

pub fn untrust_path(path: impl AsRef<Path>) -> std::io::Result<bool> {
    let Ok(path) = path.as_ref().canonicalize() else {
        return Ok(false);
    };
    TrustDb::modify(|db| {
        db.trust
            .get_or_insert(HashMap::new())
            .insert(path, Trust::Untrusted)
            != Some(Trust::Untrusted)
    })
}

pub fn initialize_trust_db() {
    ensure_parent_dir(&trust_db_file());
}
