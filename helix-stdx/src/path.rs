pub use etcetera::home_dir;

use std::{
    borrow::Cow,
    ffi::OsString,
    path::{Component, Path, PathBuf, MAIN_SEPARATOR_STR},
};

use crate::env::current_working_dir;

/// Replaces users home directory from `path` with tilde `~` if the directory
/// is available, otherwise returns the path unchanged.
pub fn fold_home_dir<'a, P>(path: P) -> Cow<'a, Path>
where
    P: Into<Cow<'a, Path>>,
{
    let path = path.into();
    if let Ok(home) = home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            let mut path = OsString::with_capacity(2 + stripped.as_os_str().len());
            path.push("~");
            path.push(MAIN_SEPARATOR_STR);
            path.push(stripped);
            return Cow::Owned(PathBuf::from(path));
        }
    }

    path
}

/// Expands tilde `~` into users home directory if available, otherwise returns the path
/// unchanged. The tilde will only be expanded when present as the first component of the path
/// and only slash follows it.
pub fn expand_tilde<'a, P>(path: P) -> Cow<'a, Path>
where
    P: Into<Cow<'a, Path>>,
{
    let path = path.into();
    let mut components = path.components();
    if let Some(Component::Normal(c)) = components.next() {
        if c == "~" {
            if let Ok(mut buf) = home_dir() {
                buf.push(components);
                return Cow::Owned(buf);
            }
        }
    }

    path
}

/// Normalize a path without resolving symlinks.
// Strategy: start from the first component and move up. Cannonicalize previous path,
// join component, cannonicalize new path, strip prefix and join to the final result.
pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
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
            #[cfg(not(windows))]
            Component::ParentDir => {
                ret.pop();
            }
            #[cfg(windows)]
            Component::ParentDir => {
                if let Some(head) = ret.components().next_back() {
                    match head {
                        Component::Prefix(_) | Component::RootDir => {}
                        Component::CurDir => unreachable!(),
                        // If we left previous component as ".." it means we met a symlink before and we can't pop path.
                        Component::ParentDir => {
                            ret.push("..");
                        }
                        Component::Normal(_) => {
                            if ret.is_symlink() {
                                ret.push("..");
                            } else {
                                ret.pop();
                            }
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            Component::Normal(c) => {
                ret.push(c);
            }
            #[cfg(windows)]
            Component::Normal(c) => 'normal: {
                use std::fs::canonicalize;

                let new_path = ret.join(c);
                if new_path.is_symlink() {
                    ret = new_path;
                    break 'normal;
                }
                let (can_new, can_old) = (canonicalize(&new_path), canonicalize(&ret));
                match (can_new, can_old) {
                    (Ok(can_new), Ok(can_old)) => {
                        let striped = can_new.strip_prefix(can_old);
                        ret.push(striped.unwrap_or_else(|_| c.as_ref()));
                    }
                    _ => ret.push(c),
                }
            }
        }
    }
    dunce::simplified(&ret).to_path_buf()
}

/// Returns the canonical, absolute form of a path with all intermediate components normalized.
///
/// This function is used instead of [`std::fs::canonicalize`] because we don't want to verify
/// here if the path exists, just normalize it's components.
pub fn canonicalize(path: impl AsRef<Path>) -> PathBuf {
    let path = expand_tilde(path.as_ref());
    let path = if path.is_relative() {
        Cow::Owned(current_working_dir().join(path))
    } else {
        path
    };

    normalize(path)
}

pub fn get_relative_path<'a, P>(path: P) -> Cow<'a, Path>
where
    P: Into<Cow<'a, Path>>,
{
    let path = path.into();
    if path.is_absolute() {
        let cwdir = normalize(current_working_dir());
        if let Ok(stripped) = normalize(&path).strip_prefix(cwdir) {
            return Cow::Owned(PathBuf::from(stripped));
        }

        return fold_home_dir(path);
    }

    path
}

/// Returns a truncated filepath where the basepart of the path is reduced to the first
/// char of the folder and the whole filename appended.
///
/// Also strip the current working directory from the beginning of the path.
/// Note that this function does not check if the truncated path is unambiguous.
///
/// ```
///    use helix_stdx::path::get_truncated_path;
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
pub fn get_truncated_path(path: impl AsRef<Path>) -> PathBuf {
    let cwd = current_working_dir();
    let path = path.as_ref();
    let path = path.strip_prefix(cwd).unwrap_or(path);
    let file = path.file_name().unwrap_or_default();
    let base = path.parent().unwrap_or_else(|| Path::new(""));
    let mut ret = PathBuf::with_capacity(file.len());
    // A char can't be directly pushed to a PathBuf
    let mut first_char_buffer = String::new();
    for d in base {
        let Some(first_char) = d.to_string_lossy().chars().next() else {
            break;
        };
        first_char_buffer.push(first_char);
        ret.push(&first_char_buffer);
        first_char_buffer.clear();
    }
    ret.push(file);
    ret
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        path::{Component, Path},
    };

    use crate::path;

    #[test]
    fn expand_tilde() {
        for path in ["~", "~/foo"] {
            let expanded = path::expand_tilde(Path::new(path));

            let tilde = Component::Normal(OsStr::new("~"));

            let mut component_count = 0;
            for component in expanded.components() {
                // No tilde left.
                assert_ne!(component, tilde);
                component_count += 1;
            }

            // The path was at least expanded to something.
            assert_ne!(component_count, 0);
        }
    }
}
