//! Jujutsu works with several backends and could add new ones in the future. Private builds of
//! it could also have private backends. Those make it hard to use `jj-lib` since it won't have
//! access to newer or private backends and fail to compute the diffs for them.
//!
//! Instead in case there *is* a diff to base ourselves on, we copy it to a tempfile or just use the
//! current file if not.

use std::path::Path;
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
        .arg("--repository")
        .arg(repo)
        .args([
            "--ignore-working-copy",
            "diff",
            "--revision",
            "@",
            "--config-toml",
        ])
        // Copy the temporary file provided by jujutsu to a temporary path of our own,
        // because the `$left` directory is deleted when `jj` finishes executing.
        .arg(format!(
            "ui.diff.tool = ['{exe}', '$left/{base}', '{target}']",
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
        .context("failed to execute jj diff command")?;

    let use_jj_path = status.success() && std::fs::metadata(tmppath).map_or(false, |m| m.len() > 0);
    // If the copy call inside `jj diff` succeeded, the tempfile is the one containing the base
    // else it's just the original file (so no diff). We check for size since `jj` can return
    // 0-sized files when there are no diffs to present for the file.
    let diff_base_path = if use_jj_path { tmppath } else { file };

    // If the command succeeded, it means we either copied the jujutsu base or the current file,
    // so there should always be something to read and compare to.
    std::fs::read(diff_base_path).context("could not read jj diff base from the target")
}

pub(crate) fn get_current_head_name(repo: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    // See <https://github.com/martinvonz/jj/blob/main/docs/templates.md>
    //
    // This will produce the following:
    //
    // - If there are no branches:    `vyvqwlmsvnlkmqrvqktpuluvuknuxpmm`
    // - If there is a single branch: `vyvqwlmsvnlkmqrvqktpuluvuknuxpmm (master)`
    // - If there are 2+ branches:    `vyvqwlmsvnlkmqrvqktpuluvuknuxpmm (master, jj-diffs)`
    //
    // Always using the long id makes it easy to share it with others, which would not be the
    // case for shorter ones: they could have a local change that renders it ambiguous.
    let template = r#"separate(" ", change_id, surround("(", ")", branches.join(", ")))"#;

    let out = Command::new("jj")
        .arg("--repository")
        .arg(repo)
        .args([
            "--ignore-working-copy",
            "log",
            "--color",
            "never",
            "--revisions",
            "@", // Only display the current revision
            "--no-graph",
            "--no-pager",
            "--template",
            template,
        ])
        .output()?;

    if !out.status.success() {
        anyhow::bail!("jj log command executed but failed");
    }

    let out = String::from_utf8(out.stdout)?;

    let rev = out
        .lines()
        .next()
        // Contrary to git, if a JJ repo exists, it always has at least two revisions:
        // the root (zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz), which cannot be focused, and the current
        // one, which exists even for brand new repos.
        .context("should always find at least one line")?;

    Ok(Arc::new(ArcSwap::from_pointee(rev.into())))
}

pub(crate) fn for_each_changed_file(
    repo: &Path,
    callback: impl Fn(Result<FileChange>) -> bool,
) -> Result<()> {
    let out = Command::new("jj")
        .arg("--repository")
        .arg(repo)
        .args([
            "--ignore-working-copy",
            "diff",
            "--color",
            "never",
            "--revision",
            "@", // Only display the current revision
            "--no-pager",
            "--types",
        ])
        .output()?;

    if !out.status.success() {
        anyhow::bail!("jj log command executed but failed");
    }

    let out = String::from_utf8(out.stdout)?;

    for line in out.lines() {
        let Some((status, path)) = line.split_once(' ') else {
            continue;
        };

        let Some(change) = status_to_change(status, path) else {
            continue;
        };

        if !callback(Ok(change)) {
            break;
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
fn status_to_change(status: &str, path: &str) -> Option<FileChange> {
    if let rename @ Some(_) = find_rename(path) {
        return rename;
    }

    // Syntax: <https://github.com/martinvonz/jj/blob/f9cfa5c9ce0eacd38e961c954e461e5e73067d22/cli/src/diff_util.rs#L97-L101>
    Some(match status {
        "FF" | "LL" | "CF" | "CL" | "FL" | "LF" => FileChange::Modified { path: path.into() },
        "-F" | "-L" => FileChange::Untracked { path: path.into() },
        "F-" | "L-" => FileChange::Deleted { path: path.into() },
        "FC" | "LC" => FileChange::Conflict { path: path.into() },
        // We ignore gitsubmodules here since they not interesting in the context of
        // a file editor.
        _ => return None,
    })
}

fn find_rename(path: &str) -> Option<FileChange> {
    let (start, rest) = path.split_once('{')?;
    let (from, rest) = rest.split_once(" => ")?;
    let (to, end) = rest.split_once('}')?;

    Some(FileChange::Renamed {
        from_path: format!("{start}{from}{end}").into(),
        to_path: format!("{start}{to}{end}").into(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_status_to_change() {
        let p = "helix-vcs/src/lib.rs";
        let pb = PathBuf::from(p);

        for s in ["FF", "LL", "CF", "CL", "FL", "LF"] {
            assert_eq!(
                status_to_change(s, p).unwrap(),
                FileChange::Modified { path: pb.clone() }
            );
        }
        for s in ["-F", "-L"] {
            assert_eq!(
                status_to_change(s, p).unwrap(),
                FileChange::Untracked { path: pb.clone() }
            );
        }
        for s in ["F-", "L-"] {
            assert_eq!(
                status_to_change(s, p).unwrap(),
                FileChange::Deleted { path: pb.clone() }
            );
        }
        for s in ["FC", "LC"] {
            assert_eq!(
                status_to_change(s, p).unwrap(),
                FileChange::Conflict { path: pb.clone() }
            );
        }
        for s in ["GG", "LG", "ARO", "", " ", "  "] {
            assert_eq!(status_to_change(s, p), None);
        }
    }

    #[test]
    fn test_find_rename() {
        fn check(path: &str, expected: Option<(&str, &str)>) {
            let result = find_rename(path);

            assert_eq!(
                result,
                expected.map(|(f, t)| FileChange::Renamed {
                    from_path: f.into(),
                    to_path: t.into()
                })
            )
        }

        // No renames
        check("helix-term/Cargo.toml", None);
        check("helix-term/src/lib.rs", None);

        // Rename of first element in path
        check(
            "{helix-term => helix-term2}/Cargo.toml",
            Some(("helix-term/Cargo.toml", "helix-term2/Cargo.toml")),
        );
        // Rename of final element in path
        check(
            "helix-term/{Cargo.toml => Cargo.toml2}",
            Some(("helix-term/Cargo.toml", "helix-term/Cargo.toml2")),
        );
        // Rename of a single dir in the middle
        check(
            "helix-term/{src => src2}/lib.rs",
            Some(("helix-term/src/lib.rs", "helix-term/src2/lib.rs")),
        );
        // Rename of two dirs in the middle
        check(
            "helix-term/{src/ui => src2/ui2}/text.rs",
            Some(("helix-term/src/ui/text.rs", "helix-term/src2/ui2/text.rs")),
        );
    }
}
