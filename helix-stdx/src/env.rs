use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::RwLock,
};

static CWD: RwLock<Option<PathBuf>> = RwLock::new(None);

// Get the current working directory.
// This information is managed internally as the call to std::env::current_dir
// might fail if the cwd has been deleted.
pub fn current_working_dir() -> PathBuf {
    if let Some(path) = &*CWD.read().unwrap() {
        return path.clone();
    }

    let path = std::env::current_dir()
        .map(crate::path::normalize)
        .expect("Couldn't determine current working directory");
    let mut cwd = CWD.write().unwrap();
    *cwd = Some(path.clone());

    path
}

pub fn set_current_working_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = crate::path::canonicalize(path);
    std::env::set_current_dir(&path)?;
    let mut cwd = CWD.write().unwrap();
    *cwd = Some(path);
    Ok(())
}

pub fn env_var_is_set(env_var_name: &str) -> bool {
    std::env::var_os(env_var_name).is_some()
}

pub fn binary_exists<T: AsRef<OsStr>>(binary_name: T) -> bool {
    which::which(binary_name).is_ok()
}

pub fn which<T: AsRef<OsStr>>(
    binary_name: T,
) -> Result<std::path::PathBuf, ExecutableNotFoundError> {
    which::which(binary_name.as_ref()).map_err(|err| ExecutableNotFoundError {
        command: binary_name.as_ref().to_string_lossy().into_owned(),
        inner: err,
    })
}

#[derive(Debug)]
pub struct ExecutableNotFoundError {
    command: String,
    inner: which::Error,
}

impl std::fmt::Display for ExecutableNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "command '{}' not found: {}", self.command, self.inner)
    }
}

impl std::error::Error for ExecutableNotFoundError {}

#[cfg(test)]
mod tests {
    use super::{current_working_dir, set_current_working_dir};

    #[test]
    fn current_dir_is_set() {
        let new_path = dunce::canonicalize(std::env::temp_dir()).unwrap();
        let cwd = current_working_dir();
        assert_ne!(cwd, new_path);

        set_current_working_dir(&new_path).expect("Couldn't set new path");

        let cwd = current_working_dir();
        assert_eq!(cwd, new_path);
    }
}
