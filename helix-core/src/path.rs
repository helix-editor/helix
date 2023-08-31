use etcetera::home_dir;
use std::path::{Component, Path, PathBuf};

/// Replaces users home directory from `path` with tilde `~` if the directory
/// is available, otherwise returns the path unchanged.
pub fn fold_home_dir(path: &Path) -> PathBuf {
    if let Ok(home) = home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return PathBuf::from("~").join(stripped);
        }
    }

    path.to_path_buf()
}

/// Expands tilde `~` into users home directory if available, otherwise returns the path
/// unchanged. The tilde will only be expanded when present as the first component of the path
/// and only slash follows it.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    if let Some(Component::Normal(c)) = components.peek() {
        if c == &"~" {
            if let Ok(home) = home_dir() {
                // it's ok to unwrap, the path starts with `~`
                return home.join(path.strip_prefix("~").unwrap());
            }
        }
    }

    path.to_path_buf()
}

/// Normalize a path, removing things like `.` and `..`.
///
/// CAUTION: This does not resolve symlinks (unlike
/// [`std::fs::canonicalize`]). This may cause incorrect or surprising
/// behavior at times. This should be used carefully. Unfortunately,
/// [`std::fs::canonicalize`] can be hard to use correctly, since it can often
/// fail, or on Windows returns annoying device paths. This is a problem Cargo
/// needs to improve on.
/// Copied from cargo: <https://github.com/rust-lang/cargo/blob/070e459c2d8b79c5b2ac5218064e7603329c92ae/crates/cargo-util/src/paths.rs#L81>
pub fn get_normalized_path(path: &Path) -> PathBuf {
    // normalization strategy is to canonicalize first ancestor path that exists (i.e., canonicalize as much as possible),
    // then run handrolled normalization on the non-existent remainder
    let (base, path) = path
        .ancestors()
        .find_map(|base| {
            let canonicalized_base = dunce::canonicalize(base).ok()?;
            let remainder = path.strip_prefix(base).ok()?.into();
            Some((canonicalized_base, remainder))
        })
        .unwrap_or_else(|| (PathBuf::new(), PathBuf::from(path)));

    if path.as_os_str().is_empty() {
        return base;
    }

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    base.join(ret)
}

/// Returns the canonical, absolute form of a path with all intermediate components normalized.
///
/// This function is used instead of `std::fs::canonicalize` because we don't want to verify
/// here if the path exists, just normalize it's components.
pub fn get_canonicalized_path(path: &Path) -> PathBuf {
    let path = expand_tilde(path);
    let path = if path.is_relative() {
        helix_loader::current_working_dir().join(path)
    } else {
        path
    };

    get_normalized_path(path.as_path())
}

pub fn get_relative_path(path: &Path) -> PathBuf {
    let path = PathBuf::from(path);
    let path = if path.is_absolute() {
        let cwdir = get_normalized_path(&helix_loader::current_working_dir());
        get_normalized_path(&path)
            .strip_prefix(cwdir)
            .map(PathBuf::from)
            .unwrap_or(path)
    } else {
        path
    };
    fold_home_dir(&path)
}

/// Returns a truncated filepath where the basepart of the path is reduced to the first
/// char of the folder and the whole filename appended.
///
/// Also strip the current working directory from the beginning of the path.
/// Note that this function does not check if the truncated path is unambiguous.
///
/// ```   
///    use helix_core::path::get_truncated_path;
///    use std::path::Path;
///
///    assert_eq!(
///         get_truncated_path("/home/cnorris/documents/jokes.txt").as_path(),
///         Path::new("/h/c/d/jokes.txt")
///     );
///     assert_eq!(
///         get_truncated_path("jokes.txt").as_path(),
///         Path::new("jokes.txt")
///     );
///     assert_eq!(
///         get_truncated_path("/jokes.txt").as_path(),
///         Path::new("/jokes.txt")
///     );
///     assert_eq!(
///         get_truncated_path("/h/c/d/jokes.txt").as_path(),
///         Path::new("/h/c/d/jokes.txt")
///     );
///     assert_eq!(get_truncated_path("").as_path(), Path::new(""));
/// ```
///
pub fn get_truncated_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let cwd = helix_loader::current_working_dir();
    let path = path
        .as_ref()
        .strip_prefix(cwd)
        .unwrap_or_else(|_| path.as_ref());
    let file = path.file_name().unwrap_or_default();
    let base = path.parent().unwrap_or_else(|| Path::new(""));
    let mut ret = PathBuf::new();
    for d in base {
        ret.push(
            d.to_string_lossy()
                .chars()
                .next()
                .unwrap_or_default()
                .to_string(),
        );
    }
    ret.push(file);
    ret
}
