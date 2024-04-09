//! Jujutsu works with several backends and could add new ones in the future. Private builds of
//! it could also have private backends. Those make it hard to use `jj-lib` since it won't have
//! access to newer or private backends and fail to compute the diffs for them.
//!
//! Instead in case there *is* a diff to base ourselves on, we copy it to a tempfile or just use the
//! current file if not.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Ok, Result};
use arc_swap::ArcSwap;

use crate::FileChange;

pub(super) fn get_diff_base(file: &Path) -> Result<Vec<u8>> {
    let jj_root_dir = find_jj_root(file)?;

    // We extracted the `jj_root_dir` from the file itself, if stripping the prefix fails
    // something has gone very very wrong
    let file_rel_to_dot_jj = file
        .strip_prefix(jj_root_dir)
        .expect("failed to strip diff path from jj root dir");

    let tmpfile = tempfile::NamedTempFile::with_prefix("helix-jj-diff-")
        .context("could not create tempfile to save jj diff base")?;
    let tmppath = tmpfile.path();

    let copy_bin = if cfg!(windows) { "copy.exe" } else { "cp" };

    let status = Command::new("jj")
        .arg("--repository")
        .arg(jj_root_dir)
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
            base = file_rel_to_dot_jj.display(),
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

pub(super) fn get_current_head_name(file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    let jj_root_dir = find_jj_root(file)?;

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
        .arg(jj_root_dir)
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
        .context("should always find at least one line")?;

    Ok(Arc::new(ArcSwap::from_pointee(rev.into())))
}

pub(super) fn for_each_changed_file(
    cwd: &Path,
    callback: impl Fn(Result<FileChange>) -> bool,
) -> Result<()> {
    let jj_root_dir = find_jj_root(cwd)?;

    let out = Command::new("jj")
        .arg("--repository")
        .arg(jj_root_dir)
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
            "",
            "--types",
        ])
        .arg(cwd)
        .output()?;

    if !out.status.success() {
        anyhow::bail!("jj log command executed but failed");
    }

    let out = String::from_utf8(out.stdout)?;

    for line in out.lines() {
        let mut split = line.splitn(2, ' ');

        let Some(status) = split.next() else { continue; };
        let Some(path) = split.next() else { continue; };

        let Some(change) = status_to_change(status, path) else { continue };

        if !callback(Ok(change)) {
            break;
        }
    }

    Ok(())
}

/// Move up until we find the repository's root
fn find_jj_root(file: &Path) -> Result<&Path> {
    file.ancestors()
        .find(|p| p.join(".jj").exists())
        .context("no .jj dir found in parents")
}

/// Associate a status to a `FileChange`.
fn status_to_change(status: &str, path: &str) -> Option<FileChange> {
    // Syntax: <https://github.com/martinvonz/jj/blob/320f50e00fcbd0d3ce27feb1e14b8e36d76b658f/cli/src/diff_util.rs#L68>
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
}
