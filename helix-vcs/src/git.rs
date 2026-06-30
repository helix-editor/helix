use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use gix::filter::plumbing::driver::apply::Delay;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gix::bstr::ByteSlice;
use gix::diff::Rewrites;
use gix::dir::entry::Status;
use gix::objs::tree::EntryKind;
use gix::sec::trust::DefaultForLevel;
use gix::status::{
    index_worktree::Item,
    plumbing::index_as_worktree::{Change, EntryStatus},
    UntrackedFiles,
};
use gix::{Commit, ObjectId, Repository, ThreadSafeRepository};

use crate::FileChange;

#[cfg(test)]
mod test;

#[inline]
fn get_repo_dir(path: &Path) -> Result<&Path> {
    if path.is_dir() {
        Ok(path)
    } else {
        path.parent().context("path has no parent directory")
    }
}

pub fn get_diff_base(file: &Path, diff_base_revision: Option<&str>) -> Result<Vec<u8>> {
    debug_assert!(!file.exists() || file.is_file());
    debug_assert!(file.is_absolute());
    let file = gix::path::realpath(file).context("resolve symlinks")?;

    // TODO cache repository lookup

    let repo_dir = get_repo_dir(&file)?;
    let repo = open_repo(repo_dir)
        .context("failed to open git repo")?
        .to_thread_local();
    let diff_base = resolve_diff_base_commit(&repo, diff_base_revision)?;
    let file_oid = match find_file_in_commit(&repo, &diff_base, &file)? {
        Some(file_oid) => file_oid,
        None if diff_base_revision.is_some() => return Ok(Vec::new()),
        None => bail!("file is untracked"),
    };

    let file_object = repo.find_object(file_oid)?;
    let data = file_object.detach().data;
    // Get the actual data that git would make out of the git object.
    // This will apply the user's git config or attributes like crlf conversions.
    //
    // The whole filter pipeline still runs in untrusted (`Trust::Reduced`) mode so built-in
    // conversions like autocrlf keep working, but gix drops `filter.*.clean` / `filter.*.smudge`
    // drivers defined in untrusted (repository-local) config, so those external programs are not
    // executed unless the workspace was explicitly trusted. This relies on `open_repo` forcing the
    // trust level instead of letting gix re-derive it from `.git` ownership; see the note there.
    if let Some(work_dir) = repo.workdir() {
        let rela_path = file.strip_prefix(work_dir)?;
        let rela_path = gix::path::try_into_bstr(rela_path)?;
        let (mut pipeline, _) = repo.filter_pipeline(None)?;
        let mut worktree_outcome =
            pipeline.convert_to_worktree(&data, rela_path.as_ref(), Delay::Forbid)?;
        let mut buf = Vec::with_capacity(data.len());
        worktree_outcome.read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        Ok(data)
    }
}

pub fn ensure_diff_base(file: &Path, diff_base_revision: &str) -> Result<()> {
    debug_assert!(file.is_absolute());
    let file = gix::path::realpath(file).context("resolve symlinks")?;
    let repo_dir = get_repo_dir(&file)?;
    let repo = open_repo(repo_dir)
        .context("failed to open git repo")?
        .to_thread_local();
    resolve_diff_base_commit(&repo, Some(diff_base_revision))?;
    Ok(())
}

pub fn get_repo_root(file: &Path) -> Result<PathBuf> {
    debug_assert!(file.is_absolute());
    let file = gix::path::realpath(file).context("resolve symlinks")?;
    let repo_dir = get_repo_dir(&file)?;
    let repo = open_repo(repo_dir)
        .context("failed to open git repo")?
        .to_thread_local();
    let work_dir = repo.workdir().context("repo has no worktree")?;
    gix::path::realpath(work_dir).context("resolve repo worktree")
}

pub fn get_current_head_name(file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    debug_assert!(!file.exists() || file.is_file());
    debug_assert!(file.is_absolute());
    let file = gix::path::realpath(file).context("resolve symlinks")?;

    let repo_dir = get_repo_dir(&file)?;
    let repo = open_repo(repo_dir)
        .context("failed to open git repo")?
        .to_thread_local();
    let head_ref = repo.head_ref()?;
    let head_commit = repo.head_commit()?;

    let name = match head_ref {
        Some(reference) => reference.name().shorten().to_string(),
        None => head_commit.id.to_hex_with_len(8).to_string(),
    };

    Ok(Arc::new(ArcSwap::from_pointee(name.into_boxed_str())))
}

pub fn for_each_changed_file(
    cwd: &Path,
    diff_base_revision: Option<&str>,
    mut f: impl FnMut(Result<FileChange>) -> bool,
) -> Result<()> {
    let repo = &open_repo(cwd)?.to_thread_local();
    match diff_base_revision {
        Some(diff_base_revision) => status_with_base(&repo, diff_base_revision, &mut f),
        None => status(&repo, &mut f),
    }
}

fn open_repo(path: &Path) -> Result<ThreadSafeRepository> {
    // Default to Trust::Full for git diff base operations
    // This is acceptable for the gitbase-picker functionality which doesn't require
    // workspace trust security checks
    let trust = gix::sec::Trust::Full;

    // On Windows various configuration options are bundled as part of the git installation. The
    // lookup is expensive; only do it there.
    let config = gix::open::permissions::Config {
        system: true,
        git: true,
        user: true,
        env: true,
        includes: true,
        git_binary: cfg!(windows),
    };

    let permissions = gix::open::Permissions {
        config,
        ..gix::open::Permissions::default_for_level(trust)
    };

    let discover_options = gix::discover::upwards::Options {
        dot_git_only: true,
        ..Default::default()
    };
    let (repo_path, _trust_from_ownership) = gix::discover::upwards_opts(path, discover_options)
        .context("failed to discover git repo")?;
    let (git_dir, _work_dir) = repo_path.into_repository_and_work_tree_directories();

    let options = gix::open::Options::default()
        .permissions(permissions)
        // `git_dir` is the discovered `.git` directory (or a linked-worktree git dir), so open it
        // as-is rather than letting gix append `.git` again.
        .open_path_as_is(true)
        .with(trust);

    Ok(ThreadSafeRepository::open_opts(git_dir, options)?)
}

/// Emulates the result of running `git status` from the command line.
fn status(repo: &Repository, f: &mut impl FnMut(Result<FileChange>) -> bool) -> Result<()> {
    let work_dir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("working tree not found"))?
        .to_path_buf();

    let status_platform = repo
        .status(gix::progress::Discard)?
        // Here we discard the `status.showUntrackedFiles` config, as it makes little sense in
        // our case to not list new (untracked) files. We could have respected this config
        // if the default value weren't `Collapsed` though, as this default value would render
        // the feature unusable to many.
        .untracked_files(UntrackedFiles::Files)
        // Turn on file rename detection, which is off by default.
        .index_worktree_rewrites(Some(Rewrites {
            copies: None,
            percentage: Some(0.5),
            limit: 1000,
            ..Default::default()
        }));

    // No filtering based on path
    let empty_patterns = vec![];

    let status_iter = status_platform.into_index_worktree_iter(empty_patterns)?;

    for item in status_iter {
        let Ok(item) = item.map_err(|err| (*f)(Err(err.into()))) else {
            continue;
        };
        let change = match item {
            Item::Modification {
                rela_path, status, ..
            } => {
                let path = work_dir.join(rela_path.to_path()?);
                match status {
                    EntryStatus::Conflict { .. } => FileChange::Conflict { path },
                    EntryStatus::Change(Change::Removed) => FileChange::Deleted { path },
                    EntryStatus::Change(Change::Modification { .. }) => {
                        FileChange::Modified { path }
                    }
                    // Files marked with `git add --intent-to-add`. Such files
                    // still show up as new in `git status`, so it's appropriate
                    // to show them the same way as untracked files in the
                    // "changed file" picker. One example of this being used
                    // is Jujutsu, a Git-compatible VCS. It marks all new files
                    // with `--intent-to-add` automatically.
                    EntryStatus::IntentToAdd => FileChange::Untracked { path },
                    _ => continue,
                }
            }
            Item::DirectoryContents { entry, .. } if entry.status == Status::Untracked => {
                FileChange::Untracked {
                    path: work_dir.join(entry.rela_path.to_path()?),
                }
            }
            Item::Rewrite {
                source,
                dirwalk_entry,
                ..
            } => FileChange::Renamed {
                from_path: work_dir.join(source.rela_path().to_path()?),
                to_path: work_dir.join(dirwalk_entry.rela_path.to_path()?),
            },
            _ => continue,
        };
        if !(*f)(Ok(change)) {
            break;
        }
    }

    Ok(())
}

fn status_with_base(
    repo: &Repository,
    diff_base_revision: &str,
    f: &mut impl FnMut(Result<FileChange>) -> bool,
) -> Result<()> {
    let _base_commit = match resolve_diff_base_commit(repo, Some(diff_base_revision)) {
        Ok(commit) => commit,
        Err(_) => return status(repo, f), // Fall back to regular status if base can't be resolved
    };
    
    let work_dir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("working tree not found"))?
        .to_path_buf();
    
    // Use git command line to get diff between base and HEAD
    let output = std::process::Command::new("git")
        .args(["diff", "--name-status", &format!("{}..HEAD", diff_base_revision)])
        .current_dir(&work_dir)
        .output()?;
    
    if !output.status.success() {
        return status(repo, f); // Fall back to regular status if git diff fails
    }
    
    let diff_output = String::from_utf8_lossy(&output.stdout);
    for line in diff_output.lines() {
        if line.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        
        let status = parts[0];
        let path = parts[1..].join(" ");
        let full_path = work_dir.join(path);
        
        let canonical_path = match full_path.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        
        let file_change = match status {
            "M" => FileChange::Modified { path: canonical_path },
            "A" => FileChange::Untracked { path: canonical_path },
            "D" => FileChange::Deleted { path: canonical_path },
            "R" | "C" => {
                // For renames/copies, we need both paths
                if parts.len() < 3 {
                    continue;
                }
                let from_path = work_dir.join(parts[1]);
                let to_path = work_dir.join(parts[2]);
                let from_canonical = match from_path.canonicalize() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let to_canonical = match to_path.canonicalize() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                FileChange::Renamed {
                    from_path: from_canonical,
                    to_path: to_canonical,
                }
            }
            _ => continue, // Skip unknown status types
        };
        
        if !f(Ok(file_change)) {
            break;
        }
    }
    
    Ok(())
}

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(
    repo: &Repository,
    commit: &Commit,
    file: &Path,
) -> Result<Option<ObjectId>> {
    let repo_dir = repo.workdir().context("repo has no worktree")?;
    let rel_path = file.strip_prefix(repo_dir)?;
    let tree = commit.tree()?;
    let Some(tree_entry) = tree.lookup_entry_by_path(rel_path)? else {
        return Ok(None);
    };
    match tree_entry.mode().kind() {
        // not a file, everything is new, do not show diff
        mode @ (EntryKind::Tree | EntryKind::Commit | EntryKind::Link) => {
            bail!("entry at {} is not a file but a {mode:?}", file.display())
        }
        // found a file
        EntryKind::Blob | EntryKind::BlobExecutable => Ok(Some(tree_entry.object_id())),
    }
}

fn resolve_diff_base_commit<'repo>(
    repo: &'repo Repository,
    diff_base_revision: Option<&str>,
) -> Result<Commit<'repo>> {
    let Some(diff_base_revision) = diff_base_revision else {
        return Ok(repo.head_commit()?);
    };

    // Try to use rev_parse which handles all kinds of references including HEAD~, master~2, etc.
    if let Ok(object_id) = repo.rev_parse_single(diff_base_revision) {
        return Ok(repo.find_commit(object_id)?);
    }

    bail!("could not resolve git diff base '{diff_base_revision}'")
}
