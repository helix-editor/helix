use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use std::path::Path;
use std::sync::Arc;

use gix::objs::tree::EntryMode;
use gix::sec::trust::DefaultForLevel;
use gix::{Commit, ObjectId, Repository, ThreadSafeRepository};

use crate::DiffProvider;

#[cfg(test)]
mod test;

pub struct Git;

impl Git {
    fn open_repo(path: &Path, ceiling_dir: Option<&Path>) -> Result<ThreadSafeRepository> {
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
            ceiling_dirs: ceiling_dir
                .map(|dir| vec![dir.to_owned()])
                .unwrap_or_default(),
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
}

impl DiffProvider for Git {
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        // TODO cache repository lookup

        let repo_dir = file.parent().context("file has no parent directory")?;
        let repo = Git::open_repo(repo_dir, None)
            .context("failed to open git repo")?
            .to_thread_local();
        let head = repo.head_commit()?;
        let file_oid = find_file_in_commit(&repo, &head, file)?;

        let file_object = repo.find_object(file_oid)?;
        let mut data = file_object.detach().data;
        // convert LF to CRLF if configured to avoid showing every line as changed
        if repo
            .config_snapshot()
            .boolean("core.autocrlf")
            .unwrap_or(false)
        {
            let mut normalized_file = Vec::with_capacity(data.len());
            let mut at_cr = false;
            for &byte in &data {
                if byte == b'\n' {
                    // if this is a LF instead of a CRLF (last byte was not a CR)
                    // insert a new CR to generate a CRLF
                    if !at_cr {
                        normalized_file.push(b'\r');
                    }
                }
                at_cr = byte == b'\r';
                normalized_file.push(byte)
            }
            data = normalized_file
        }
        Ok(data)
    }

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());
        let repo_dir = file.parent().context("file has no parent directory")?;
        let repo = Git::open_repo(repo_dir, None)
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
}

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(repo: &Repository, commit: &Commit, file: &Path) -> Result<ObjectId> {
    let repo_dir = repo.work_dir().context("repo has no worktree")?;
    let rel_path = file.strip_prefix(repo_dir)?;
    let tree = commit.tree()?;
    let tree_entry = tree
        .lookup_entry_by_path(rel_path)?
        .context("file is untracked")?;
    match tree_entry.mode() {
        // not a file, everything is new, do not show diff
        mode @ (EntryMode::Tree | EntryMode::Commit | EntryMode::Link) => {
            bail!("entry at {} is not a file but a {mode:?}", file.display())
        }
        // found a file
        EntryMode::Blob | EntryMode::BlobExecutable => Ok(tree_entry.object_id()),
    }
}
