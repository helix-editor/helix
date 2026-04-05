//! Jujutsu works with several backends and could add new ones in the future. Private builds of
//! it could also have private backends. Those make it hard to use `jj-lib` since it won't have
//! access to newer or private backends and fail to compute the diffs for them.
//!
//! Instead in case there *is* a diff to base ourselves on, we copy it to a tempfile or just use the
//! current file if not.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;

use crate::FileChange;

pub(super) fn get_diff_base(repo: &Path, file: &Path) -> Result<Vec<u8>> {
    let file_relative_to_root = file
        .strip_prefix(repo)
        .context("failed to strip JJ repo root path from file")?;

    let tmpfile = tempfile::NamedTempFile::with_prefix("helix-jj-diff-")
        .context("could not create tempfile to save jj diff base")?;
    let tmppath = tmpfile.path();

    let copy_bin = if cfg!(windows) { "copy.exe" } else { "cp" };

    let status = Command::new("jj")
        // Ensure we're working in the expected repository.
        .arg("--repository")
        .arg(repo)
        .args([
            // Do not commit changes: this avoids Helix updating the JJ state every time this runs.
            "--ignore-working-copy",
            "diff",
            // Ensuring no configuration option will interfere.
            "--color",
            "never",
            "--no-pager",
            // Work with current revision only.
            "--revision",
            "@",
            // Pass custom diff configuration.
            "--config",
        ])
        // Copy the temporary file provided by jujutsu to a temporary path of our own,
        // because the `$left` directory is deleted when `jj` finishes executing.
        .arg(format!(
            "ui.diff-formatter=['{exe}', '$left/{base}', '{target}']",
            exe = copy_bin,
            base = file_relative_to_root.display(),
            // Where to copy the jujutsu-provided file
            target = tmppath.display(),
        ))
        // Restrict the diff to the current file
        .arg(file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to execute `jj diff` to get diff base")?;

    let use_jj_path = status.success() && std::fs::metadata(tmppath).is_ok_and(|m| m.len() > 0);
    // If the copy call inside `jj diff` succeeded, the tempfile is the one containing the base
    // else it's just the original file (so no diff). We check for size since `jj` can return
    // 0-sized files when there are no diffs to present for the file.
    let diff_base_path = if use_jj_path { tmppath } else { file };

    // If the command succeeded, it means we either copied the jujutsu base or defaulted to the
    // current file, so there should always be something to read and compare to.
    std::fs::read(diff_base_path).context("could not read jj diff base from the target")
}

pub(crate) fn get_current_head_name(repo: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    // See <https://github.com/martinvonz/jj/blob/main/docs/templates.md>
    //
    // This will produce the following:
    //
    //     quvlrxss
    //     kmlpqmrv main-1
    //     sxrrsnun main-2 main-3*
    //     kzqnuykl
    //
    // There will be a `*` when a bookmark has been modified compared to its remote.
    //
    // We use a short ID with 8 characters because in practice the change ID is extremely unlikely
    // to conflict since we only consider mutable commits (like most jj commands will do by default)
    // and this leaves space for bookmarks to appear in the status bar even on narrower screens.
    let template = r#"change_id.short(8) ++ " " ++ bookmarks ++ "\n""#;

    let out = Command::new("jj")
        // Ensure we're working in the expected repository.
        .arg("--repository")
        .arg(repo)
        .args([
            // Do not commit changes: this avoids Helix updating the JJ state every time this runs.
            "--ignore-working-copy",
            "log",
            // Ensuring no configuration option will interfere.
            "--color",
            "never",
            "--no-graph",
            "--no-pager",
            // Includes from last immutable revision to current change.
            "--revisions",
            "mutable()-::@",
            "--template",
            template,
        ])
        .output()?;

    anyhow::ensure!(out.status.success(), "`jj log` executed but failed");

    let output = String::from_utf8(out.stdout).context("`jj log` did not output valid UTF-8")?;
    let head_text = extract_head_name(&output)?;

    Ok(Arc::new(ArcSwap::from_pointee(head_text.into())))
}

/// Helper function to make the extracting logic testable
fn extract_head_name(output: &str) -> Result<String> {
    let mut lines = output.lines();
    let mut next = || lines.next().and_then(|line| line.split_once(' '));

    let (rev, exact_bookmarks) = next()
        // Contrary to git, if a JJ repo exists, it always has at least two revisions:
        // the root (zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz), which cannot be focused, and the current
        // one, which exists even for brand new repos.
        .context("should always find at least one line")?;

    let head_text = if !exact_bookmarks.is_empty() {
        // Parentheses: bookmarks are exactly on current change.
        format!("{rev} ({exact_bookmarks})")
    } else {
        let mutable_ancestor_bookmarks = std::iter::from_fn(next)
            .map(|e| e.1)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if mutable_ancestor_bookmarks.is_empty() {
            // Found no bookmarks amongst mutable ancestors.
            rev.to_string()
        } else {
            // Angle brackets: bookmarks are on mutable ancestors.
            format!("{rev} [{}]", mutable_ancestor_bookmarks.join(" ").trim())
        }
    };

    Ok(head_text)
}

pub(crate) fn for_each_changed_file(
    repo: &Path,
    callback: impl Fn(Result<FileChange>) -> bool,
) -> Result<()> {
    // First we get conflict via another command because we have to, `jj diff` cannot list them and
    // `jj status` does not support templating.
    let out = Command::new("jj")
        // Ensure we're working in the expected repository.
        .arg("--repository")
        .arg(repo)
        .args([
            // Do not commit changes: this avoids Helix updating the JJ state every time this runs.
            "--ignore-working-copy",
            "file",
            "list",
            // Ensuring no configuration option will interfere.
            "--color",
            "never",
            "--no-pager",
            // Work with current revision only.
            "--revision",
            "@",
            // List per-file diff types, do not show diff itself.
            "--template",
            "if(conflict, path ++ \" //\n\")",
        ])
        .output()?;

    anyhow::ensure!(out.status.success(), "`jj file list` executed but failed");

    for entry in split_double_slash(&out.stdout, true) {
        if entry.is_empty() {
            continue;
        }

        let path = make_pathbuf(entry);

        if !callback(Ok(FileChange::Conflict { path })) {
            return Ok(());
        }
    }

    // The forward slash is the only character that is disallowed in both Unix and Windows paths,
    // meaning `//` cannot ever appear in them on any platform.
    //
    // <https://jj-vcs.github.io/jj/latest/templates/#treediffentry-type>
    //
    // Lines will be of the following format (examples)
    //
    // ```
    // modified // conflict.txt // conflict // conflict.txt // file //\n
    // added // added file.rs //  // added file.rs // symlink //\n
    // renamed // renamed.nix // file // after-rename.nix // file //\n
    // removed // testing.ts // file // testing.ts //  //\n
    // ```
    //
    // Note we use `//\n` as the end delimiter to allow for files that contains `\n` in their name.
    //
    // For the file types, we will only concern ourselves with `file` and `symlink`, anything else
    // will get dropped just like `git.rs` does.
    let template = concat!(
        // First, print the status, it will determinate some of our parsing.
        // One of "modified", "added", "removed", "copied", or "renamed".
        "status ++ ",
        "\" // \" ++",
        "source.path() ++ ",
        "\" // \" ++ ",
        // <https://jj-vcs.github.io/jj/latest/templates/#treeentry-type>
        "source.file_type() ++ ",
        "\" // \" ++ ",
        "target.path() ++ ",
        "\" // \" ++ ",
        // <https://jj-vcs.github.io/jj/latest/templates/#treeentry-type>
        "target.file_type() ++ ",
        "\" //\\n\"",
    );

    let out = Command::new("jj")
        // Ensure we're working in the expected repository.
        .arg("--repository")
        .arg(repo)
        .args([
            // Do not commit changes: this avoids Helix updating the JJ state every time this runs.
            "--ignore-working-copy",
            "diff",
            // Ensuring no configuration option will interfere.
            "--color",
            "never",
            "--no-pager",
            // Work with current revision only.
            "--revision",
            "@",
            // List per-file diff types, do not show diff itself.
            "--template",
            template,
        ])
        .output()?;

    anyhow::ensure!(out.status.success(), "`jj diff` executed but failed");

    for entry in split_double_slash(&out.stdout, true) {
        if entry.is_empty() {
            continue;
        }

        let Some(change) = entry_to_change(entry) else {
            continue;
        };

        if !callback(Ok(change)) {
            return Ok(());
        }
    }

    Ok(())
}

pub(crate) fn open_repo(repo_path: &Path) -> Result<()> {
    assert!(
        repo_path.join(".jj").exists(),
        "no .jj where one was expected: {}",
        repo_path.display(),
    );

    let status = Command::new("jj")
        .args(["--ignore-working-copy", "root"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("not a valid JJ repo")
    }
}

/// Associate a status to a `FileChange`.
///
/// Gets something like `modified // conflict.txt // conflict // conflict.txt // file` as input.
fn entry_to_change(entry: &[u8]) -> Option<FileChange> {
    let mut sections = split_double_slash(entry, false);

    let kind = sections.next()?;

    let source_path = sections.next()?;
    let source_file_type = sections.next()?;

    let target_path = sections.next()?;
    let target_file_type = sections.next()?;

    // Never generated in practice but let's be thourough in case that changes.
    // <https://github.com/jj-vcs/jj/issues/7264>
    if target_file_type == b"conflict" {
        return Some(FileChange::Conflict {
            path: make_pathbuf(target_path),
        });
    }

    let file_types = [
        // The empty file type is used when the file didn't exist before or doesn't exist now,
        // e.g. when added or removed.
        "".as_bytes(),
        "conflict".as_bytes(),
        "file".as_bytes(),
        "symlink".as_bytes(),
    ];
    if !file_types.contains(&source_file_type) || !file_types.contains(&target_file_type) {
        return None;
    }

    let change = match kind {
        b"added" | b"copied" => FileChange::Untracked {
            path: make_pathbuf(target_path),
        },
        b"modified" => FileChange::Modified {
            path: make_pathbuf(target_path),
        },
        b"removed" => FileChange::Deleted {
            path: make_pathbuf(target_path),
        },
        b"renamed" => FileChange::Renamed {
            from_path: make_pathbuf(source_path),
            to_path: make_pathbuf(target_path),
        },
        _ => return None,
    };

    Some(change)
}

#[cfg(any(unix, target_os = "wasi"))]
fn make_pathbuf(sl: &[u8]) -> PathBuf {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    PathBuf::from(std::ffi::OsStr::from_bytes(sl))
}

// Imperfect fallback for platforms where we don't know about an always-correct method.
// In practice, non-UTF8 paths are vanishingly rare and should not be an issue for anyone running a
// Rust binary like Helix.
#[cfg(not(any(unix, target_os = "wasi")))]
fn make_pathbuf(sl: &[u8]) -> PathBuf {
    let s = String::from_utf8_lossy(sl);
    PathBuf::from(s.into_owned())
}

/// Split a byte slice on either ` // ` or ` //\n` depending on `with_newline`.
fn split_double_slash(slice: &[u8], with_newline: bool) -> impl Iterator<Item = &[u8]> {
    let mut done = false;
    let mut rest = slice;
    let needle = if with_newline { " //\n" } else { " // " }.as_bytes();
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let result = match memchr::memmem::find(rest, needle) {
            Some(pos) => {
                // We use the non-panicking variants to avoid adding the panic machinery here when
                // we know it won't ever panic in practice (unless there is a bug in memchr, which
                // is unlikely given how much the crate is used).
                let (before, after) = rest.split_at_checked(pos).unwrap_or_default();
                rest = after.get(4..).unwrap_or_default();
                before
            }
            None => {
                done = true;
                rest
            }
        };
        Some(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_double_slash_no_newline() {
        let input = b"modified // test.rs // file // test.rs // file //\n";
        let expected = [
            "modified".as_bytes(),
            "test.rs".as_bytes(),
            "file".as_bytes(),
            "test.rs".as_bytes(),
            "file //\n".as_bytes(), // Not trimmed since we're not splitting on newlines
        ];

        let result = split_double_slash(input, false).collect::<Vec<_>>();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_split_double_slash_with_newline() {
        let input = concat!(
            "modified // test.rs // file // test.rs // file //\n",
            "modified // test.rs // file // test.rs // file //\n",
        )
        .as_bytes();
        let expected = [
            "modified // test.rs // file // test.rs // file".as_bytes(),
            "modified // test.rs // file // test.rs // file".as_bytes(),
            // We expect an empty slice after the last split
            &[],
        ];

        let result = split_double_slash(input, true).collect::<Vec<_>>();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_entry_to_change() {
        let p = "helix-vcs/src/lib.rs";
        let pb = PathBuf::from(p);

        let entry = |kind, (t1, t2)| {
            entry_to_change(format!("{kind} // {p} // {t1} // {p} // {t2}").as_bytes())
        };

        for types in [
            ("conflict", "file"),
            ("conflict", "symlink"),
            ("file", "file"),
            ("file", "symlink"),
            ("symlink", "file"),
            ("symlink", "symlink"),
        ] {
            assert_eq!(
                entry("modified", types).unwrap(),
                FileChange::Modified { path: pb.clone() }
            );
        }

        for types in [("", "file"), ("", "symlink")] {
            assert_eq!(
                entry("added", types).unwrap(),
                FileChange::Untracked { path: pb.clone() }
            );
        }

        for types in [
            ("file", "file"),
            ("file", "symlink"),
            ("symlink", "file"),
            ("symlink", "symlink"),
        ] {
            assert_eq!(
                entry("copied", types).unwrap(),
                FileChange::Untracked { path: pb.clone() }
            );
        }

        for types in [("conflict", ""), ("file", ""), ("symlink", "")] {
            assert_eq!(
                entry("removed", types).unwrap(),
                FileChange::Deleted { path: pb.clone() }
            );
        }

        for types in [
            ("", "conflict"),
            ("conflict", "conflict"),
            ("file", "conflict"),
            ("symlink", "conflict"),
        ] {
            assert_eq!(
                entry("conflict", types).unwrap(),
                FileChange::Conflict { path: pb.clone() }
            );
        }

        for invalid_kind in ["invalid", ""] {
            assert_eq!(entry(invalid_kind, ("file", "file")), None);
        }

        for invalid_types in [
            ("tree", "file"),
            ("submodule", "file"),
            ("abcdef", "file"),
            ("file", "tree"),
            ("file", "submodule"),
            ("file", "abcdef"),
        ] {
            assert_eq!(entry("modified", invalid_types), None);
        }
    }

    #[test]
    fn test_extract_head_name() {
        // No bookmarks.
        let result = extract_head_name("abcdefgh \nijklmnop \n").unwrap();
        assert_eq!(result, "abcdefgh");

        // Single exact bookmark.
        let result = extract_head_name("abcdefgh bookmark*\nijklmnop other-bookmark*\n").unwrap();
        assert_eq!(result, "abcdefgh (bookmark*)");

        // Multiple exact bookmarks.
        let result = extract_head_name(concat!(
            "abcdefgh bookmark bookmark-v2\n",
            "ijklmnop other-ookmark\n",
        ))
        .unwrap();
        assert_eq!(result, "abcdefgh (bookmark bookmark-v2)");

        // Single inexact bookmark.
        let result = extract_head_name("abcdefgh \nijklmnop other-bookmark\n").unwrap();
        assert_eq!(result, "abcdefgh [other-bookmark]");

        // Multiple inexact bookmarks.
        let result = extract_head_name("abcdefgh \nijklmnop bookmark* bookmark-v2\n").unwrap();
        assert_eq!(result, "abcdefgh [bookmark* bookmark-v2]");
    }
}
