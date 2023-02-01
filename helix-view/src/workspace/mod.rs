pub mod undo;

use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

use anyhow::{Context, Result};
use helix_core::path::path_as_bytes;
use sha1_smol::Sha1;

pub struct Workspace {
    path: PathBuf,
    lock: Option<FileLock>,
}

impl Workspace {
    // TODO: Allow custom session names to be passed.
    pub fn new() -> Result<Self> {
        let path = std::env::current_dir()?;
        let bytes = path_as_bytes(path);
        let hash = Sha1::from(bytes).digest().to_string();
        let path = helix_loader::cache_dir().join("workspaces").join(hash);
        Ok(Self { path, lock: None })
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn get(&mut self, path: &str) -> Result<File> {
        if self.lock.is_none() {
            let lock = FileLock::shared(self.path.join(".helix.lock"))?;
            lock.lock()?;

            self.lock = Some(lock);
        }
        let path = self.path.join(path);

        OpenOptions::new()
            .read(true)
            .open(path)
            .context("failed to open file")
    }

    pub fn get_mut(&mut self, path: &str) -> Result<File> {
        if self.lock.is_none() {
            let lock = FileLock::exclusive(self.path.join(".helix.lock"))?;
            lock.lock()?;

            self.lock = Some(lock);
        }
        let path = self.path.join(path);

        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .context("failed to open file")
    }
}

pub struct FileLock {
    file: File,
    shared: bool,
}

impl FileLock {
    pub fn exclusive(path: PathBuf) -> Result<Self> {
        let file = Self::open_lock(path)?;
        Ok(Self {
            file,
            shared: false,
        })
    }

    pub fn shared(path: PathBuf) -> Result<Self> {
        let file = Self::open_lock(path)?;
        Ok(Self { file, shared: true })
    }

    pub fn get(&self) -> Result<&File> {
        self.lock()?;
        Ok(&self.file)
    }

    pub fn get_mut(&mut self) -> Result<&mut File> {
        self.lock()?;
        Ok(&mut self.file)
    }

    fn lock(&self) -> Result<()> {
        sys::lock(&self.file, self.shared)
    }

    fn open_lock(path: PathBuf) -> std::io::Result<File> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::DirBuilder::new().recursive(true).create(parent)?;
            }
        }
        OpenOptions::new().write(true).create(true).open(path)
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = sys::unlock(&self.file);
    }
}

// `sys` impls from https://github.com/rust-lang/cargo/blob/fc2242a8c5606be36aecfd61dd464422271dad9d/src/cargo/util/flock.rs
#[cfg(unix)]
mod sys {
    use anyhow::Result;
    use std::fs::File;
    use std::io::Error;
    use std::os::unix::io::AsRawFd;

    pub(super) fn unlock(file: &File) -> Result<()> {
        flock(file, libc::LOCK_UN)
    }

    pub(super) fn lock(file: &File, shared: bool) -> Result<()> {
        let flag = if shared { libc::LOCK_SH } else { libc::LOCK_EX };
        flock(file, flag)
    }

    fn flock(file: &File, flag: libc::c_int) -> Result<()> {
        let ret = unsafe { libc::flock(file.as_raw_fd(), flag) };
        if ret < 0 {
            anyhow::bail!(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{fs::File, io::Error, os::windows::prelude::AsRawHandle, path::Path};

    use winapi::um::{
        fileapi::{LockFileEx, UnlockFile},
        minwinbase::LOCKFILE_EXCLUSIVE_LOCK,
    };

    /// Blocks until the lock is acquired.
    pub(super) fn lock(file: &File, shared: bool) -> anyhow::Result<()> {
        let flag = if shared { 0 } else { LOCKFILE_EXCLUSIVE_LOCK };
        unsafe {
            let mut overlapped = std::mem::zeroed();
            let ret = LockFileEx(file.as_raw_handle(), flag, 0, !0, !0, &mut overlapped);
            if ret == 0 {
                anyhow::bail!(Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    pub(super) fn unlock(file: &File) -> std::io::Result<()> {
        unsafe {
            let ret = UnlockFile(file.as_raw_handle(), 0, 0, !0, !0);
            if ret == 0 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}
