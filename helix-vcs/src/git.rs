use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use gix::filter::plumbing::driver::apply::Delay;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use gix::objs::tree::EntryKind;
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
        let data = file_object.detach().data;
        // Get the actual data that git would make out of the git object.
        // This will apply the user's git config or attributes like crlf conversions.
        if let Some(work_dir) = repo.work_dir() {
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
        .lookup_entry_by_path(rel_path, &mut Vec::new())?
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
