//! Sapling integration through `sl`, including repositories using the legacy `.hg` directory.
//!
//! Invoking the installed CLI also supports private backends and extensions. Since these can
//! execute repository-local code, commands must only run in trusted workspaces.

use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use anyhow::{bail, ensure, Context, Result};
use arc_swap::ArcSwap;

use crate::FileChange;

#[cfg(test)]
mod test;

pub fn get_diff_base(file: &Path, trust_full: bool) -> Result<Vec<u8>> {
    ensure!(trust_full, "Sapling requires a trusted workspace");
    // Resolve existing symlinks, but allow newly created or deleted files as well.
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let repo = find_repo(file.parent().context("file has no parent")?)?;
    let relative = file.strip_prefix(&repo)?;

    if let Ok(base) = cat(&repo, relative) {
        return Ok(base);
    }

    // A newly added file has an empty base; an untracked file has no diff base. Copies and
    // renames use the source's committed contents so subsequent edits get meaningful gutters.
    let output = output(
        command(&repo)
            .args([
                "status",
                "--added",
                "--copies",
                "--template",
                STATUS_TEMPLATE,
                "--",
            ])
            .arg(file_pattern(relative)),
    )?;
    for entry in parse_status(&output)? {
        if entry.path == relative && entry.status == b'A' {
            return match entry.copy {
                Some(source) => cat(&repo, &source),
                None => Ok(Vec::new()),
            };
        }
    }
    bail!("file has no Sapling diff base: {}", file.display())
}

fn cat(repo: &Path, file: &Path) -> Result<Vec<u8>> {
    // Intersect the literal path with its parent's immediate files: `path:` alone also
    // includes descendants if a committed directory was replaced by a working-copy file.
    let mut parent = OsString::from("rootfilesin:");
    parent.push(file.parent().context("file has no parent")?);
    output(
        command(repo)
            .args(["cat", "--rev", ".", "--include"])
            .arg(parent)
            .arg("--")
            .arg(file_pattern(file)),
    )
}

pub fn get_current_head_name(file: &Path, trust_full: bool) -> Result<Arc<ArcSwap<Box<str>>>> {
    ensure!(trust_full, "Sapling requires a trusted workspace");
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let repo = find_repo(file.parent().context("file has no parent")?)?;
    let data = output(command(&repo).args([
        "log",
        "--rev",
        ".",
        "--template",
        "{if(activebookmark, activebookmark, node|short)}",
    ]))?;
    let name = String::from_utf8(data).context("Sapling head is not UTF-8")?;
    ensure!(!name.is_empty(), "Sapling returned an empty head name");
    Ok(Arc::new(ArcSwap::from_pointee(name.into_boxed_str())))
}

pub fn for_each_changed_file(
    cwd: &Path,
    trust_full: bool,
    callback: impl Fn(Result<FileChange>) -> bool,
) -> Result<()> {
    ensure!(trust_full, "Sapling requires a trusted workspace");
    let repo = find_repo(cwd)?;
    let status = output(command(&repo).args([
        "status",
        "--modified",
        "--added",
        "--removed",
        "--deleted",
        "--unknown",
        "--copies",
        "--template",
        STATUS_TEMPLATE,
    ]))?;
    let resolved =
        output(command(&repo).args(["resolve", "--list", "--template", "{status}\\0{path}\\0"]))?;
    for change in changes(&repo, &status, &resolved)? {
        if !callback(Ok(change)) {
            break;
        }
    }
    Ok(())
}

fn find_repo(path: &Path) -> Result<PathBuf> {
    ensure!(path.is_absolute(), "Sapling requires an absolute path");
    for parent in path.ancestors() {
        if parent.join(".sl").is_dir() || parent.join(".hg").is_dir() {
            return Ok(parent.to_path_buf());
        }
        // A Git repository nested inside a Sapling repository belongs to the Git provider.
        if parent.join(".git").exists() {
            break;
        }
    }
    bail!("no Sapling repository found")
}

fn command(repo: &Path) -> Command {
    let mut command = Command::new("sl");
    command
        .current_dir(repo)
        .args(["--repository"])
        .arg(repo)
        .args(["--noninteractive", "--pager", "never", "--color", "never"])
        // Plain mode disables aliases and formatting overrides, but retains backend config.
        .env("SL_PLAIN", "1")
        .env("HGPLAIN", "1")
        .env_remove("SL_PLAINEXCEPT")
        .env_remove("HGPLAINEXCEPT")
        .stdin(Stdio::null());
    command
}

fn output(command: &mut Command) -> Result<Vec<u8>> {
    let output = command
        .output()
        .context("failed to execute Sapling (`sl`)")?;
    ensure!(
        output.status.success(),
        "Sapling command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn file_pattern(file: &Path) -> OsString {
    // Paths must be literal even if they contain glob syntax, a pattern prefix or a leading '-'.
    let mut pattern = OsString::from("path:");
    pattern.push(file);
    pattern
}

const STATUS_TEMPLATE: &str = "{status}\\0{path}\\0{copy}\\0";

struct StatusEntry {
    status: u8,
    path: PathBuf,
    copy: Option<PathBuf>,
}

fn parse_status(data: &[u8]) -> Result<Vec<StatusEntry>> {
    let mut fields = nul_fields(data)?;
    let mut entries = Vec::new();
    while let Some(status) = fields.next() {
        ensure!(status.len() == 1, "invalid Sapling status");
        let path = make_path(fields.next().context("missing status path")?)?;
        let copy = fields.next().context("missing copy source")?;
        let copy = if copy.is_empty() {
            None
        } else {
            Some(make_path(copy)?)
        };
        entries.push(StatusEntry {
            status: status[0],
            path,
            copy,
        });
    }
    Ok(entries)
}

fn changes(repo: &Path, status: &[u8], resolved: &[u8]) -> Result<Vec<FileChange>> {
    let entries = parse_status(status)?;
    let mut changes = Vec::new();
    let mut conflicts = HashSet::new();
    let mut fields = nul_fields(resolved)?;
    while let Some(status) = fields.next() {
        ensure!(
            matches!(status, b"U" | b"R"),
            "invalid Sapling resolve status: {status:?}"
        );
        let path = make_path(fields.next().context("missing resolve path")?)?;
        if status == b"U" && conflicts.insert(path.clone()) {
            changes.push(FileChange::Conflict {
                path: repo.join(path),
            });
        }
    }

    let removed: HashSet<_> = entries
        .iter()
        .filter(|entry| entry.status == b'R')
        .map(|entry| &entry.path)
        .collect();
    let renamed: HashSet<_> = entries
        .iter()
        .filter(|entry| entry.status == b'A' && !conflicts.contains(&entry.path))
        .filter_map(|entry| entry.copy.as_ref())
        .filter(|source| removed.contains(source))
        .collect();

    for entry in &entries {
        if conflicts.contains(&entry.path) {
            continue;
        }
        let path = repo.join(&entry.path);
        let change = match entry.status {
            b'M' => FileChange::Modified { path },
            b'A' => match entry
                .copy
                .as_ref()
                .filter(|source| removed.contains(source))
            {
                Some(source) => FileChange::Renamed {
                    from_path: repo.join(source),
                    to_path: path,
                },
                None => FileChange::Untracked { path },
            },
            b'?' => FileChange::Untracked { path },
            b'R' if renamed.contains(&entry.path) => continue,
            b'R' | b'!' => FileChange::Deleted { path },
            _ => continue,
        };
        changes.push(change);
    }
    Ok(changes)
}

fn nul_fields(data: &[u8]) -> Result<impl Iterator<Item = &[u8]>> {
    ensure!(
        data.is_empty() || data.ends_with(&[0]),
        "unterminated Sapling output"
    );
    Ok(data
        .split_inclusive(|&byte| byte == 0)
        .map(|field| &field[..field.len() - 1]))
}

fn make_path(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    };
    #[cfg(not(unix))]
    let path = PathBuf::from(std::str::from_utf8(bytes).context("Sapling path is not UTF-8")?);
    ensure!(
        !path.as_os_str().is_empty()
            && path.components().all(|c| matches!(c, Component::Normal(_))),
        "invalid repository-relative Sapling path"
    );
    Ok(path)
}
