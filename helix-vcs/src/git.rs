use std::path::Path;

use git::objs::tree::EntryMode;
use git::sec::trust::DefaultForLevel;
use git::{Commit, ObjectId, Repository, ThreadSafeRepository};
use git_repository as git;

use crate::DiffProvider;

#[cfg(test)]
mod test;

pub struct Git;

impl Git {
    fn open_repo(path: &Path, ceiling_dir: Option<&Path>) -> Option<ThreadSafeRepository> {
        // custom open options
        let mut git_open_opts_map = git::sec::trust::Mapping::<git::open::Options>::default();

        // On windows various configuration options are bundled as part of the installations
        // This path depends on the install location of git and therefore requires some overhead to lookup
        // This is basically only used on windows and has some overhead hence it's disabled on other platforms.
        // `gitoxide` doesn't use this as default
        let config = git::permissions::Config {
            system: true,
            git: true,
            user: true,
            env: true,
            includes: true,
            git_binary: cfg!(windows),
        };
        // change options for config permissions without touching anything else
        git_open_opts_map.reduced = git_open_opts_map.reduced.permissions(git::Permissions {
            config,
            ..git::Permissions::default_for_level(git::sec::Trust::Reduced)
        });
        git_open_opts_map.full = git_open_opts_map.full.permissions(git::Permissions {
            config,
            ..git::Permissions::default_for_level(git::sec::Trust::Full)
        });

        let mut open_options = git::discover::upwards::Options::default();
        if let Some(ceiling_dir) = ceiling_dir {
            open_options.ceiling_dirs = vec![ceiling_dir.to_owned()];
        }

        ThreadSafeRepository::discover_with_environment_overrides_opts(
            path,
            open_options,
            git_open_opts_map,
        )
        .ok()
    }
}

impl DiffProvider for Git {
    fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        // TODO cache repository lookup
        let repo = Git::open_repo(file.parent()?, None)?.to_thread_local();
        let head = repo.head_commit().ok()?;
        let file_oid = find_file_in_commit(&repo, &head, file)?;

        let file_object = repo.find_object(file_oid).ok()?;
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
        Some(data)
    }
}

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(repo: &Repository, commit: &Commit, file: &Path) -> Option<ObjectId> {
    let repo_dir = repo.work_dir()?;
    let rel_path = file.strip_prefix(repo_dir).ok()?;
    let tree = commit.tree().ok()?;
    let tree_entry = tree.lookup_entry_by_path(rel_path).ok()??;
    match tree_entry.mode() {
        // not a file, everything is new, do not show diff
        EntryMode::Tree | EntryMode::Commit | EntryMode::Link => None,
        // found a file
        EntryMode::Blob | EntryMode::BlobExecutable => Some(tree_entry.object_id()),
    }
}
