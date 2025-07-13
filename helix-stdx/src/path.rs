//! Functions for working with [Path].

pub use etcetera::home_dir;
use once_cell::sync::Lazy;
use regex_cursor::{engines::meta::Regex, Input};
use ropey::RopeSlice;

use std::{
    borrow::Cow,
    ffi::OsString,
    ops::Range,
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
/// unchanged.
///
/// The tilde will only be expanded when present as the first component of the path
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
// Strategy: start from the first component and move up. Canonicalize previous path,
// join component, canonicalize new path, strip prefix and join to the final result.
pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().copied() {
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

/// Convert path into a relative path
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

fn path_component_regex(windows: bool) -> String {
    // TODO: support backslash path escape on windows (when using git bash for example)
    let space_escape = if windows { r"[\^`]\s" } else { r"[\\]\s" };
    // partially baesd on what's allowed in an url but with some care to avoid
    // false positives (like any kind of brackets or quotes)
    r"[\w@.\-+#$%?!,;~&]|".to_owned() + space_escape
}

/// Regex for delimited environment captures like `${HOME}`.
fn braced_env_regex(windows: bool) -> String {
    r"\$\{(?:".to_owned() + &path_component_regex(windows) + r"|[/:=])+\}"
}

fn compile_path_regex(
    prefix: &str,
    postfix: &str,
    match_single_file: bool,
    windows: bool,
) -> Regex {
    let first_component = format!(
        "(?:{}|(?:{}))",
        braced_env_regex(windows),
        path_component_regex(windows)
    );
    // For all components except the first we allow an equals so that `foo=/
    // bar/baz` does not include foo. This is primarily intended for url queries
    // (where an equals is never in the first component)
    let component = format!("(?:{first_component}|=)");
    let sep = if windows { r"[/\\]" } else { "/" };
    let url_prefix = r"[\w+\-.]+://??";
    let path_prefix = if windows {
        // single slash handles most windows prefixes (like\\server\...) but `\
        // \?\C:\..` (and C:\) needs special handling, since we don't allow : in path
        // components (so that colon separated paths and <path>:<line> work)
        r"\\\\\?\\\w:|\w:|\\|"
    } else {
        ""
    };
    let path_start = format!("(?:{first_component}+|~|{path_prefix}{url_prefix})");
    let optional = if match_single_file {
        format!("|{path_start}")
    } else {
        String::new()
    };
    let path_regex = format!(
        "{prefix}(?:{path_start}?(?:(?:{sep}{component}+)+{sep}?|{sep}){optional}){postfix}"
    );
    Regex::new(&path_regex).unwrap()
}

/// If `src` ends with a path then this function returns the part of the slice.
pub fn get_path_suffix(src: RopeSlice<'_>, match_single_file: bool) -> Option<RopeSlice<'_>> {
    let regex = if match_single_file {
        static REGEX: Lazy<Regex> = Lazy::new(|| compile_path_regex("", "$", true, cfg!(windows)));
        &*REGEX
    } else {
        static REGEX: Lazy<Regex> = Lazy::new(|| compile_path_regex("", "$", false, cfg!(windows)));
        &*REGEX
    };

    regex
        .find(Input::new(src))
        .map(|mat| src.byte_slice(mat.range()))
}

/// Returns an iterator of the **byte** ranges in src that contain a path.
pub fn find_paths(
    src: RopeSlice<'_>,
    match_single_file: bool,
) -> impl Iterator<Item = Range<usize>> + '_ {
    let regex = if match_single_file {
        static REGEX: Lazy<Regex> = Lazy::new(|| compile_path_regex("", "", true, cfg!(windows)));
        &*REGEX
    } else {
        static REGEX: Lazy<Regex> = Lazy::new(|| compile_path_regex("", "", false, cfg!(windows)));
        &*REGEX
    };
    regex.find_iter(Input::new(src)).map(|mat| mat.range())
}

/// Performs substitution of `~` and environment variables, see [`env::expand`](crate::env::expand) and [`expand_tilde`]
pub fn expand<T: AsRef<Path> + ?Sized>(path: &T) -> Cow<'_, Path> {
    let path = path.as_ref();
    let path = expand_tilde(path);
    match crate::env::expand(&*path) {
        Cow::Borrowed(_) => path,
        Cow::Owned(path) => PathBuf::from(path).into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        path::{Component, Path},
    };

    use regex_cursor::Input;
    use ropey::RopeSlice;

    use crate::path::{self, compile_path_regex};

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

    macro_rules! assert_match {
        ($regex: expr, $haystack: expr) => {
            let haystack = Input::new(RopeSlice::from($haystack));
            assert!(
                $regex.is_match(haystack),
                "regex should match {}",
                $haystack
            );
        };
    }
    macro_rules! assert_no_match {
        ($regex: expr, $haystack: expr) => {
            let haystack = Input::new(RopeSlice::from($haystack));
            assert!(
                !$regex.is_match(haystack),
                "regex should not match {}",
                $haystack
            );
        };
    }

    macro_rules! assert_matches {
        ($regex: expr, $haystack: expr, [$($matches: expr),*]) => {
            let src = $haystack;
            let matches: Vec<_> = $regex
                .find_iter(Input::new(RopeSlice::from(src)))
                .map(|it| &src[it.range()])
                .collect();
            assert_eq!(matches, vec![$($matches),*]);
        };
    }

    /// Linux-only path
    #[test]
    fn path_regex_unix() {
        // due to ambiguity with the `\` path separator we can't support space escapes `\ ` on windows
        let regex = compile_path_regex("^", "$", false, false);
        assert_match!(regex, "${FOO}/hello\\ world");
        assert_match!(regex, "${FOO}/\\ ");
    }

    /// Windows-only paths
    #[test]
    fn path_regex_windows() {
        let regex = compile_path_regex("^", "$", false, true);
        assert_match!(regex, "${FOO}/hello^ world");
        assert_match!(regex, "${FOO}/hello` world");
        assert_match!(regex, "${FOO}/^ ");
        assert_match!(regex, "${FOO}/` ");
        assert_match!(regex, r"foo\bar");
        assert_match!(regex, r"foo\bar");
        assert_match!(regex, r"..\bar");
        assert_match!(regex, r"..\");
        assert_match!(regex, r"C:\");
        assert_match!(regex, r"\\?\C:\foo");
        assert_match!(regex, r"\\server\foo");
    }

    /// Paths that should work on all platforms
    #[test]
    fn path_regex() {
        for windows in [false, true] {
            let regex = compile_path_regex("^", "$", false, windows);
            assert_no_match!(regex, "foo");
            assert_no_match!(regex, "");
            assert_match!(regex, "https://github.com/notifications/query=foo");
            assert_match!(regex, "file:///foo/bar");
            assert_match!(regex, "foo/bar");
            assert_match!(regex, "$HOME/foo");
            assert_match!(regex, "${FOO:-bar}/baz");
            assert_match!(regex, "foo/bar_");
            assert_match!(regex, "/home/bar");
            assert_match!(regex, "foo/");
            assert_match!(regex, "./");
            assert_match!(regex, "../");
            assert_match!(regex, "../..");
            assert_match!(regex, "./foo");
            assert_match!(regex, "./foo.rs");
            assert_match!(regex, "/");
            assert_match!(regex, "~/");
            assert_match!(regex, "~/foo");
            assert_match!(regex, "~/foo");
            assert_match!(regex, "~/foo/../baz");
            assert_match!(regex, "${HOME}/foo");
            assert_match!(regex, "$HOME/foo");
            assert_match!(regex, "/$FOO");
            assert_match!(regex, "/${FOO}");
            assert_match!(regex, "/${FOO}/${BAR}");
            assert_match!(regex, "/${FOO}/${BAR}/foo");
            assert_match!(regex, "/${FOO}/${BAR}");
            assert_match!(regex, "${FOO}/hello_$WORLD");
            assert_match!(regex, "${FOO}/hello_${WORLD}");
            let regex = compile_path_regex("", "", false, windows);
            assert_no_match!(regex, "");
            assert_matches!(
                regex,
                r#"${FOO}/hello_${WORLD}  ${FOO}/hello_${WORLD} foo("./bar", "/home/foo")""#,
                [
                    "${FOO}/hello_${WORLD}",
                    "${FOO}/hello_${WORLD}",
                    "./bar",
                    "/home/foo"
                ]
            );
            assert_matches!(
                regex,
                r#"--> helix-stdx/src/path.rs:427:13"#,
                ["helix-stdx/src/path.rs"]
            );
            assert_matches!(
                regex,
                r#"PATH=/foo/bar:/bar/baz:${foo:-/foo}/bar:${PATH}"#,
                ["/foo/bar", "/bar/baz", "${foo:-/foo}/bar"]
            );
            let regex = compile_path_regex("^", "$", true, windows);
            assert_no_match!(regex, "");
            assert_match!(regex, "foo");
            assert_match!(regex, "foo/");
            assert_match!(regex, "$FOO");
            assert_match!(regex, "${BAR}");
        }
    }
}
