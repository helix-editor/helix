use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use gix::filter::plumbing::driver::apply::Delay;
use std::collections::HashSet;
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

/// Validates that a given diff base revision exists and is accessible.
/// This is used to check that a user-provided revision (branch, tag, commit hash, etc.)
/// can be resolved before attempting to use it for diff operations.
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

/// Get the absolute path to the repository root (working tree) for a given file.
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
    f: impl FnMut(Result<FileChange>) -> bool,
) -> Result<()> {
    let repo = &open_repo(cwd)?.to_thread_local();
    match diff_base_revision {
        Some(diff_base_revision) => status_with_base(&repo, diff_base_revision, f),
        None => status(&repo, f),
    }
}

fn open_repo(path: &Path) -> Result<ThreadSafeRepository> {
    // custom open options
    let mut git_open_opts_map = gix::sec::trust::Mapping::<gix::open::Options>::default();

    // On windows various configuration options are bundled as part of the installations
    // This path depends on the install location of git and therefore requires some overhead to lookup
    // This is basically only used on windows and has some overhead hence it's disabled on other platforms.
    // `gitoxide` doesn't use this as default
    let config = gix::open::permissions::Config {
        system: true,
        git: true,
        user: true,
        env: true,
        includes: true,
        git_binary: cfg!(windows),
    };
    // change options for config permissions without touching anything else
    git_open_opts_map.reduced = git_open_opts_map
        .reduced
        .permissions(gix::open::Permissions {
            config,
            ..gix::open::Permissions::default_for_level(gix::sec::Trust::Reduced)
        });
    git_open_opts_map.full = git_open_opts_map.full.permissions(gix::open::Permissions {
        config,
        ..gix::open::Permissions::default_for_level(gix::sec::Trust::Full)
    });

    let open_options = gix::discover::upwards::Options {
        dot_git_only: true,
        ..Default::default()
    };

    let res = ThreadSafeRepository::discover_with_environment_overrides_opts(
        path,
        open_options,
        git_open_opts_map,
    )?;

    Ok(res)
}

/// Emulates the result of running `git status` from the command line.
fn status(repo: &Repository, mut f: impl FnMut(Result<FileChange>) -> bool) -> Result<()> {
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
        let Ok(item) = item.map_err(|err| f(Err(err.into()))) else {
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
        if !f(Ok(change)) {
            break;
        }
    }

    Ok(())
}

/// Emulates `git status` but compares against a specific base revision instead of HEAD.
/// This shows files that differ between the working tree and the specified base revision.
fn status_with_base(
    repo: &Repository,
    diff_base_revision: &str,
    mut f: impl FnMut(Result<FileChange>) -> bool,
) -> Result<()> {
    let base_commit = resolve_diff_base_commit(repo, Some(diff_base_revision))?;
    let head_commit = repo.head_commit()?;
    if base_commit.id == head_commit.id {
        return status(repo, f);
    }

    let work_dir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("working tree not found"))?
        .to_path_buf();

    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 1000,
        ..Default::default()
    };
    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;

    // Collect all file paths that might have changed relative to work_dir.
    // This includes files changed between base and head (committed),
    // plus files with unstaged changes.
    let mut candidate_rel_paths = HashSet::new();

    // Add files from base->head diff
    base_tree
        .changes()?
        .options(|options| {
            options.track_rewrites(Some(rewrites.clone()));
        })
        .for_each_to_obtain_tree(&head_tree, |change| {
            if let Some((path, _)) = tree_change_to_file_change(&work_dir, change)? {
                // path is absolute, convert to relative
                if let Ok(rel_path) = path.strip_prefix(&work_dir) {
                    candidate_rel_paths.insert(rel_path.to_path_buf());
                } else {
                    // Fallback: use the path as-is if strip_prefix fails
                    // This can happen with symlinks or non-normalized paths
                    log::debug!("Failed to strip prefix from path: {}", path.display());
                }
            }
            Ok::<_, anyhow::Error>(std::ops::ControlFlow::Continue(()))
        })?;

    // Add files with unstaged changes
    status(repo, |change| {
        if let Ok(change) = change {
            // change.path() is absolute, convert to relative
            if let Ok(rel_path) = change.path().strip_prefix(&work_dir) {
                candidate_rel_paths.insert(rel_path.to_path_buf());
            } else {
                // Fallback: try to get relative path
                log::debug!(
                    "Failed to strip prefix from path: {}",
                    change.path().display()
                );
            }
        }
        true
    })?;

    // For each candidate path, check if working tree differs from base
    for rel_path in &candidate_rel_paths {
        let abs_path = work_dir.join(rel_path);

        // Get the file content from base
        let base_content = match get_file_at_commit(repo, &base_commit, &abs_path) {
            Ok(content) => content,
            Err(err) => {
                // File doesn't exist in base or error reading it
                log::debug!(
                    "Error getting file at commit for {}: {}",
                    abs_path.display(),
                    err
                );
                // Check if it exists in working tree
                if abs_path.exists() {
                    // New file - show it
                    let change = FileChange::Modified { path: abs_path };
                    if !f(Ok(change)) {
                        return Ok(());
                    }
                }
                // If it doesn't exist in either, skip it
                continue;
            }
        };

        // Get the working tree content
        let working_content = match std::fs::read(&abs_path) {
            Ok(content) => content,
            Err(err) => {
                // File doesn't exist in working tree - it was deleted
                log::debug!(
                    "Error reading working tree file {}: {}",
                    abs_path.display(),
                    err
                );
                // Check if it existed in base
                if !base_content.is_empty() {
                    // File was deleted from working tree but existed in base
                    let change = FileChange::Deleted { path: abs_path };
                    if !f(Ok(change)) {
                        return Ok(());
                    }
                }
                continue;
            }
        };

        // Compare
        if base_content != working_content {
            // File is different from base - show it
            let change = FileChange::Modified { path: abs_path };
            if !f(Ok(change)) {
                return Ok(());
            }
        }
        // else: file matches base, don't show it (it was reverted)
    }

    Ok(())
}

/// Helper to get file content at a specific commit.
/// Returns empty vector if file doesn't exist at that commit.
fn get_file_at_commit(repo: &Repository, commit: &Commit, path: &Path) -> Result<Vec<u8>> {
    let repo_dir = repo.workdir().context("repo has no worktree")?;
    let rel_path = path.strip_prefix(repo_dir)?;
    let tree = commit.tree()?;
    let Some(tree_entry) = tree.lookup_entry_by_path(rel_path)? else {
        return Ok(Vec::new());
    };

    match tree_entry.mode().kind() {
        EntryKind::Blob | EntryKind::BlobExecutable => {
            let obj = tree_entry.object()?;
            Ok(obj.data.to_vec())
        }
        _ => Ok(Vec::new()), // Not a blob (directory, symlink, etc.)
    }
}

/// Convert a git tree diff change into a FileChange.
/// Handles additions, deletions, modifications, and renames.
fn tree_change_to_file_change(
    work_dir: &Path,
    change: gix::object::tree::diff::Change<'_, '_, '_>,
) -> Result<Option<(PathBuf, FileChange)>> {
    let change = match change {
        gix::object::tree::diff::Change::Addition {
            location,
            entry_mode,
            ..
        } => {
            if !picker_tracks_entry_kind(entry_mode.kind()) {
                return Ok(None);
            }
            let path = work_dir.join(location.to_path()?);
            (path.clone(), FileChange::Modified { path })
        }
        gix::object::tree::diff::Change::Deletion {
            location,
            entry_mode,
            ..
        } => {
            if !picker_tracks_entry_kind(entry_mode.kind()) {
                return Ok(None);
            }
            let path = work_dir.join(location.to_path()?);
            (path.clone(), FileChange::Deleted { path })
        }
        gix::object::tree::diff::Change::Modification {
            location,
            previous_entry_mode,
            entry_mode,
            ..
        } => {
            if !picker_tracks_entry_kind(previous_entry_mode.kind())
                || !picker_tracks_entry_kind(entry_mode.kind())
            {
                return Ok(None);
            }
            let path = work_dir.join(location.to_path()?);
            (path.clone(), FileChange::Modified { path })
        }
        gix::object::tree::diff::Change::Rewrite {
            source_location,
            source_entry_mode,
            location,
            entry_mode,
            copy,
            ..
        } => {
            if copy
                || !picker_tracks_entry_kind(source_entry_mode.kind())
                || !picker_tracks_entry_kind(entry_mode.kind())
            {
                return Ok(None);
            }
            let from_path = work_dir.join(source_location.to_path()?);
            let to_path = work_dir.join(location.to_path()?);
            (to_path.clone(), FileChange::Renamed { from_path, to_path })
        }
    };

    Ok(Some(change))
}

/// Returns true if the picker should track this entry kind.
/// We only track regular files and executables, not directories or submodules.
fn picker_tracks_entry_kind(kind: EntryKind) -> bool {
    !matches!(kind, EntryKind::Tree | EntryKind::Commit)
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

/// Resolve a diff base revision to a commit object.
/// If no revision is provided, returns the HEAD commit.
/// Supports all git revision syntax (branch names, tags, commit hashes, HEAD~, etc.).
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
