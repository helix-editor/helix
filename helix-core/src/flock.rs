use std::fs::{File, OpenOptions};
use std::io::Result;
use std::path::Path;

pub struct FileLock {
    file: File,
    shared: bool,
}

impl FileLock {
    pub fn exclusive<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = Self::open_lock(path)?;
        Ok(Self {
            file,
            shared: false,
        })
    }

    pub fn shared<P: AsRef<Path>>(path: P) -> Result<Self> {
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

    fn open_lock<P: AsRef<Path>>(path: P) -> Result<File> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.exists() {
                std::fs::DirBuilder::new().recursive(true).create(parent)?;
            }
        }
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
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
    use std::fs::File;
    use std::io::Error;
    use std::io::Result;
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
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{
        fs::File,
        io::{Error, Result},
        os::windows::prelude::AsRawHandle,
        path::Path,
    };

    use winapi::um::{
        fileapi::{LockFileEx, UnlockFile},
        minwinbase::LOCKFILE_EXCLUSIVE_LOCK,
    };

    /// Blocks until the lock is acquired.
    pub(super) fn lock(file: &File, shared: bool) -> Result<()> {
        let flag = if shared { 0 } else { LOCKFILE_EXCLUSIVE_LOCK };
        unsafe {
            let mut overlapped = std::mem::zeroed();
            let ret = LockFileEx(file.as_raw_handle(), flag, 0, !0, !0, &mut overlapped);
            if ret == 0 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    pub(super) fn unlock(file: &File) -> Result<()> {
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
