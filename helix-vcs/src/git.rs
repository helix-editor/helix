use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use gix::filter::plumbing::driver::apply::Delay;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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

use crate::{
    repository::RepositoryProvider, Branch, BranchKind, FileChange, Repository as VcsRepository,
};

#[cfg(test)]
mod test;

const BRANCH_FORMAT: &str =
    "%(HEAD)%00%(refname)%00%(refname:short)%00%(objectname:short)%00%(upstream:short)%00%(upstream:track)";

#[inline]
fn get_repo_dir(file: &Path) -> Result<&Path> {
    file.parent().context("file has no parent directory")
}

pub fn get_diff_base(file: &Path) -> Result<Vec<u8>> {
    debug_assert!(!file.exists() || file.is_file());
    debug_assert!(file.is_absolute());
    let file = gix::path::realpath(file).context("resolve symlinks")?;

    // TODO cache repository lookup

    let repo_dir = get_repo_dir(&file)?;
    let repo = open_repo(repo_dir)
        .context("failed to open git repo")?
        .to_thread_local();
    let head = repo.head_commit()?;
    let file_oid = find_file_in_commit(&repo, &head, &file)?;

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

pub fn for_each_changed_file(cwd: &Path, f: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
    status(&open_repo(cwd)?.to_thread_local(), f)
}

pub fn repository(cwd: &Path) -> Result<VcsRepository> {
    let work_dir = repo_workdir(cwd).with_context(|| {
        format!(
            "current directory is not inside a git worktree: {}",
            cwd.display()
        )
    })?;

    Ok(VcsRepository::new(RepositoryProvider::Git, work_dir))
}

pub fn branches(work_dir: &Path) -> Result<Vec<Branch>> {
    let output = git_output(
        work_dir,
        &[
            "branch",
            "--all",
            "--sort=refname",
            "--format",
            BRANCH_FORMAT,
        ],
    )?;

    let branches = output
        .lines()
        .filter_map(parse_branch_line)
        .collect::<Vec<_>>();

    Ok(branches)
}

pub fn switch_branch(work_dir: &Path, branch: &Branch) -> Result<()> {
    if branch.is_current() {
        return Ok(());
    }

    ensure_clean_worktree(work_dir)?;

    match branch.kind() {
        BranchKind::Local => {
            git_run(work_dir, &["switch", branch.name()])?;
        }
        BranchKind::Remote => {
            let local_branch = local_name_for_remote_branch(branch.name())?;

            if local_branch_exists(work_dir, local_branch)? {
                git_run(work_dir, &["switch", local_branch])?;
            } else {
                git_run(work_dir, &["switch", "--track", branch.name()])?;
            }
        }
    }

    Ok(())
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

fn repo_workdir(cwd: &Path) -> Result<PathBuf> {
    let repo = open_repo(cwd)?.to_thread_local();
    let work_dir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("working tree not found"))?;

    Ok(std::fs::canonicalize(work_dir).unwrap_or_else(|_| work_dir.to_path_buf()))
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = git_command(cwd, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_run(cwd: &Path, args: &[&str]) -> Result<()> {
    git_command(cwd, args).map(|_| ())
}

fn git_command(cwd: &Path, args: &[&str]) -> Result<Output> {
    let output = git_command_output(cwd, args)?;
    if !output.status.success() {
        bail!("{}", git_failure_message(args, &output));
    }

    Ok(output)
}

fn git_command_output(cwd: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "false")
        .output()
        .with_context(|| format!("failed to run `git {}`", args.join(" ")))
}

fn git_failure_message(args: &[&str], output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let message = if stderr.is_empty() {
        format!("exit status {}", output.status)
    } else {
        stderr.to_string()
    };
    format!("git {} failed: {message}", args.join(" "))
}

struct BranchRecord<'a> {
    head_marker: &'a str,
    refname: &'a str,
    name: &'a str,
    head: &'a str,
    upstream: &'a str,
    tracking: &'a str,
}

impl<'a> BranchRecord<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let mut fields = line.split('\0');
        Some(Self {
            head_marker: fields.next()?,
            refname: fields.next()?,
            name: fields.next()?.trim(),
            head: fields.next()?.trim(),
            upstream: fields.next()?.trim(),
            tracking: fields.next()?.trim(),
        })
    }
}

fn parse_branch_line(line: &str) -> Option<Branch> {
    let record = BranchRecord::parse(line)?;
    let kind = branch_kind(record.refname)?;

    Some(Branch::new(
        record.name.to_string(),
        kind,
        record.head_marker == "*",
        record.head.to_string(),
        non_empty_string(record.upstream),
        tracking_status(record.tracking),
    ))
}

fn branch_kind(refname: &str) -> Option<BranchKind> {
    let kind = if refname.starts_with("refs/heads/") {
        BranchKind::Local
    } else if refname.starts_with("refs/remotes/") {
        if refname.ends_with("/HEAD") {
            return None;
        }
        BranchKind::Remote
    } else {
        return None;
    };

    Some(kind)
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn tracking_status(value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn local_name_for_remote_branch(branch: &str) -> Result<&str> {
    branch
        .split_once('/')
        .map(|(_, local)| local)
        .filter(|local| !local.is_empty())
        .context("remote branch name does not include a remote")
}

fn ensure_clean_worktree(cwd: &Path) -> Result<()> {
    let output = git_output(
        cwd,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    if let Some(first_change) = output.lines().next() {
        bail!(
            "worktree has uncommitted changes; commit, stash, or clean before switching branches (first change: {first_change})"
        );
    }
    Ok(())
}

fn local_branch_exists(cwd: &Path, branch: &str) -> Result<bool> {
    let refname = format!("refs/heads/{branch}");
    let args = ["show-ref", "--verify", "--quiet", refname.as_str()];
    let output = git_command_output(cwd, &args)?;

    if output.status.success() {
        return Ok(true);
    }

    if output.status.code() == Some(1) {
        return Ok(false);
    }

    bail!("{}", git_failure_message(&args, &output))
}

/// Emulates the result of running `git status` from the command line.
fn status(repo: &Repository, f: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
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

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(repo: &Repository, commit: &Commit, file: &Path) -> Result<ObjectId> {
    let repo_dir = repo.workdir().context("repo has no worktree")?;
    let rel_path = file.strip_prefix(repo_dir)?;
    let tree = commit.tree()?;
    let tree_entry = tree
        .lookup_entry_by_path(rel_path)?
        .context("file is untracked")?;
    match tree_entry.mode().kind() {
        // not a file, everything is new, do not show diff
        mode @ (EntryKind::Tree | EntryKind::Commit | EntryKind::Link) => {
            bail!("entry at {} is not a file but a {mode:?}", file.display())
        }
        // found a file
        EntryKind::Blob | EntryKind::BlobExecutable => Ok(tree_entry.object_id()),
    }
}
