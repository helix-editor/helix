pub mod state;
pub mod undo;

use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

use anyhow::{Context, Result};
use sha1_smol::Sha1;

// Needs to mimic borrowing rules.
// Allow multiple read-only references, and only one mutable reference w/ no read-only.
// Should not lock unless actively used. And should be unlocked automatically when all file handles are dropped.
pub struct Session {
    path: PathBuf,
    lock: Option<FileLock>,
}

impl Session {
    // TODO: Allow custom session names to be passed.
    pub fn new(path: PathBuf) -> Result<Self> {
        let bytes = sys::path_as_bytes(path.as_path());
        let hash = Sha1::from(bytes).digest().to_string();
        let path = helix_loader::cache_dir().join("sessions").join(hash);
        Ok(Self { path, lock: None })
    }

    pub fn get(&mut self, filename: String) -> Result<File> {
        if self.lock.is_none() {
            let lock = FileLock::shared(self.path.join(".helix.lock"))?;
            lock.lock()?;

            self.lock = Some(lock);
        }

        OpenOptions::new()
            .read(true)
            .open(self.path.join(filename))
            .context("failed to open file")
    }

    // TODO: Return a FileLockGuard instead.
    pub fn get_mut(&mut self, filename: String) -> Result<File> {
        if self.lock.is_none() {
            let lock = FileLock::exclusive(self.path.join(".helix.lock"))?;
            lock.lock()?;

            self.lock = Some(lock);
        }

        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.path.join(filename))
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

    pub fn lock(&self) -> Result<()> {
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
    use std::io::{Error, Result};
    use std::os::unix::io::AsRawFd;

    pub(super) fn unlock(file: &File) -> Result<()> {
        flock(file, libc::LOCK_UN)
    }

    pub(super) fn lock(file: &File) -> Result<()> {
        flock(file, libc::LOCK_EX)
    }

    #[cfg(not(target_os = "solaris"))]
    fn flock(file: &File, flag: libc::c_int) -> Result<()> {
        let ret = unsafe { libc::flock(file.as_raw_fd(), flag) };
        if ret < 0 {
            anyhow::bail!(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "solaris")]
    fn flock(file: &File, flag: libc::c_int) -> Result<()> {
        // Solaris lacks flock(), so try to emulate using fcntl()
        let mut flock = libc::flock {
            l_type: 0,
            l_whence: 0,
            l_start: 0,
            l_len: 0,
            l_sysid: 0,
            l_pid: 0,
            l_pad: [0, 0, 0, 0],
        };
        flock.l_type = if flag & libc::LOCK_UN != 0 {
            libc::F_UNLCK
        } else if flag & libc::LOCK_EX != 0 {
            libc::F_WRLCK
        } else if flag & libc::LOCK_SH != 0 {
            libc::F_RDLCK
        } else {
            panic!("unexpected flock() operation")
        };

        let mut cmd = libc::F_SETLKW;
        if (flag & libc::LOCK_NB) != 0 {
            cmd = libc::F_SETLK;
        }

        let ret = unsafe { libc::fcntl(file.as_raw_fd(), cmd, &flock) };

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

    pub(super) fn path_as_bytes(path: &Path) -> &[u8] {
        path.to_str().unwrap().as_bytes()
    }

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
