use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use git::index::{entry::Mode, State};
use git::objs::tree::EntryMode;
use git::{prelude::FindExt, sec::trust::DefaultForLevel};
use git::{Commit, ObjectId, Repository, ThreadSafeRepository};
use git_repository as git;
use ignore::WalkBuilder;
use sha1::Digest;

use crate::{DiffProvider, FileChange};

#[cfg(test)]
mod test;

pub struct Git;

/// A subset of `git_repository::objs::tree::EntryMode` that actually makes sense for tree nodes.
#[derive(Hash, PartialEq, Eq)]
enum FileEntryMode {
    Blob,
    BlobExecutable,
    Link,
}

#[derive(Default)]
struct RawChanges {
    additions: Vec<RawAddition>,
    deletions: HashMap<ObjectId, Vec<RawDeletion>>,
    modifications: Vec<RawModification>,
}

#[derive(Hash, PartialEq, Eq)]
struct RawAddition {
    entry_mode: FileEntryMode,
    oid: ObjectId,
    path: PathBuf,
}

#[derive(Hash, PartialEq, Eq)]
struct RawDeletion {
    entry_mode: FileEntryMode,
    oid: ObjectId,
    path: PathBuf,
}

#[allow(unused)]
struct RawModification {
    previous_entry_mode: FileEntryMode,
    previous_oid: ObjectId,

    entry_mode: FileEntryMode,
    oid: ObjectId,

    path: PathBuf,
}

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

    /// Emulates the result of running `git status` from the command line.
    fn status(repo: &Repository) -> Result<Vec<FileChange>> {
        let autocrlf = repo
            .config_snapshot()
            .boolean("core.autocrlf")
            .unwrap_or(false);

        let work_dir = repo
            .work_dir()
            .ok_or_else(|| anyhow::anyhow!("working tree not found"))?;

        // TODO: allow diffing against another ref
        let head_tree = repo.head_commit()?.tree()?;
        let head_state = State::from_tree(&head_tree.id, |oid, buf| {
            repo.objects.find_tree_iter(oid, buf).ok()
        })?;

        let mut head_tree_set = HashSet::new();
        let mut submodule_paths = vec![];

        let mut raw_changes = RawChanges::default();

        for item in head_state.entries() {
            let full_path = work_dir.join(&PathBuf::from(item.path(&head_state).to_string()));

            if item.mode == Mode::COMMIT {
                submodule_paths.push(full_path);
            } else {
                let old_entry_mode = match item.mode {
                    Mode::FILE => FileEntryMode::Blob,
                    Mode::FILE_EXECUTABLE => FileEntryMode::BlobExecutable,
                    Mode::SYMLINK => FileEntryMode::Link,
                    _ => anyhow::bail!("unexpected entry mode"),
                };

                match git_meta_from_path(&full_path, autocrlf)? {
                    Some((new_entry_mode, new_oid)) => {
                        // On Windows, physical files are _always_ inferred as `Blob`. We simply don't
                        // compare the entry mode as it's pointless.
                        let entry_mode_changed = {
                            #[cfg(unix)]
                            {
                                new_entry_mode != old_entry_mode
                            }

                            #[cfg(not(unix))]
                            {
                                false
                            }
                        };

                        if entry_mode_changed || new_oid != item.id {
                            raw_changes.add_modification(RawModification {
                                previous_entry_mode: old_entry_mode,
                                previous_oid: item.id,
                                entry_mode: new_entry_mode,
                                oid: new_oid,
                                path: full_path.clone(),
                            });
                        }
                    }
                    None => {
                        raw_changes.add_deletion(RawDeletion {
                            entry_mode: old_entry_mode,
                            oid: item.id,
                            path: full_path.clone(),
                        });
                    }
                }

                head_tree_set.insert(full_path);
            }
        }

        // Looks for untracked files by walking the fs and probing the (cached) head tree
        // TODO: use build_parallel() to speed this up
        for entry in WalkBuilder::new(work_dir)
            .hidden(false)
            .ignore(false)
            .filter_entry(move |entry| {
                entry.file_name() != ".git"
                    && !submodule_paths
                        .iter()
                        .any(|submodule| entry.path().starts_with(submodule))
            })
            .build()
        {
            let entry = entry?;
            if !entry.file_type().map_or(false, |ft| ft.is_dir()) {
                let full_path = entry.path();
                let meta = git_meta_from_path(full_path, autocrlf)?
                    .ok_or_else(|| anyhow::anyhow!("file moved between checks"))?;
                if !head_tree_set.contains(full_path) {
                    raw_changes.add_addition(RawAddition {
                        entry_mode: meta.0,
                        oid: meta.1,
                        path: full_path.to_path_buf(),
                    })
                }
            }
        }

        Ok(raw_changes.into())
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

    fn get_changed_files(&self, cwd: &Path) -> Result<Vec<FileChange>> {
        Self::status(
            &Self::open_repo(cwd, None)
                .ok_or_else(|| anyhow::anyhow!("no Git repository found"))?
                .to_thread_local(),
        )
    }
}

impl RawChanges {
    pub fn add_addition(&mut self, addition: RawAddition) {
        self.additions.push(addition);
    }

    pub fn add_deletion(&mut self, deletion: RawDeletion) {
        match self.deletions.entry(deletion.oid) {
            Entry::Occupied(entry) => {
                entry.into_mut().push(deletion);
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![deletion]);
            }
        }
    }

    pub fn add_modification(&mut self, modification: RawModification) {
        self.modifications.push(modification);
    }
}

impl From<RawChanges> for Vec<FileChange> {
    // Unlike Git, we only look for pure renames at the moment.
    // TODO: detect renames with minor changes
    fn from(mut raw: RawChanges) -> Self {
        let mut status_entries = vec![];

        let additions_left = if !raw.additions.is_empty() && !raw.deletions.is_empty() {
            let mut unmatched_additions = vec![];

            for add in raw.additions.into_iter() {
                let matched_deletions = match raw.deletions.entry(add.oid) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(_) => {
                        unmatched_additions.push(add);
                        continue;
                    }
                };

                // Impossible to have an empty vec inside
                let chosen_deletion = matched_deletions.pop().expect("unexpected empty vec");
                if matched_deletions.is_empty() {
                    raw.deletions.remove(&add.oid);
                }

                status_entries.push(FileChange::Renamed {
                    from_path: chosen_deletion.path.to_owned(),
                    to_path: add.path.to_owned(),
                });
            }

            unmatched_additions
        } else {
            raw.additions
        };

        additions_left
            .into_iter()
            .for_each(|item| status_entries.push(FileChange::Untracked { path: item.path }));
        raw.deletions
            .values()
            .into_iter()
            .flat_map(|val| val.iter())
            .for_each(|item| {
                status_entries.push(FileChange::Deleted {
                    path: item.path.to_owned(),
                })
            });
        raw.modifications
            .into_iter()
            .for_each(|item| status_entries.push(FileChange::Modified { path: item.path }));

        status_entries
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

fn git_meta_from_path(
    path: &Path,
    autocrlf: bool,
) -> Result<Option<(FileEntryMode, ObjectId)>, std::io::Error> {
    // Windows doesn't support symlinks. This block runs fine but is just wasting CPU cycles.
    #[cfg(not(windows))]
    match path.symlink_metadata() {
        Ok(meta) => {
            if meta.is_symlink() {
                let link_content = std::fs::read_link(path)?;
                let link_content = link_content.to_string_lossy();
                let link_content = link_content.as_bytes();

                let mut hasher = sha1::Sha1::default();
                hasher.update(b"blob ");
                hasher.update(format!("{}", link_content.len()).as_bytes());
                hasher.update(b"\0");
                hasher.update(link_content);

                let hash: [u8; 20] = hasher.finalize().into();

                return Ok(Some((FileEntryMode::Link, ObjectId::from(hash))));
            }
        }
        Err(_) => return Ok(None),
    };

    // Not a symlink for sure from this point
    Ok(match path.metadata() {
        Ok(meta) => {
            if meta.is_file() {
                let entry_mode = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::prelude::PermissionsExt;
                        if meta.permissions().mode() & 0o111 != 0 {
                            FileEntryMode::BlobExecutable
                        } else {
                            FileEntryMode::Blob
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        FileEntryMode::Blob
                    }
                };

                let oid = {
                    let mut file = std::fs::File::open(path)?;

                    // `git::features::hash::Sha1` doesn't implement `Write` so we use the
                    // underlying crate directly for max perf.
                    let mut hasher = sha1::Sha1::default();
                    hasher.update(b"blob ");

                    if autocrlf {
                        // When autocrlf is on, we either have to fit the whole file into memory,
                        // or we read the file twice. Either way is not optimal. How should we
                        // handle this?
                        //
                        // With the current implementation, there's no way we can handle huge files
                        // that do not fit into memory. Maybe we can set a size limit? Anything
                        // over a certain size will simply be read twice: once for getting the
                        // normalized size, and once for the hasher updates?
                        const BUFFER_SIZE: usize = 8 * 1024;
                        let mut buffer = [0u8; BUFFER_SIZE];

                        let mut len = file.read(&mut buffer)?;
                        if content_inspector::inspect(&buffer[..len])
                            == content_inspector::ContentType::BINARY
                        {
                            // No CRLF handling! We update the part already read + the remaining
                            // content in the file.
                            hasher.update(format!("{}", meta.len()).as_bytes());
                            hasher.update(b"\0");

                            hasher.update(&buffer[..len]);
                            std::io::copy(&mut file, &mut hasher)?;
                        } else {
                            // It's a text file. CRLF transformation as planned.
                            let mut normalized_file = Vec::with_capacity(meta.len() as usize);
                            let mut was_cr = false;

                            loop {
                                buffer[..len].iter().for_each(|byte| {
                                    if was_cr && *byte == b'\n' {
                                        normalized_file.pop();
                                    }
                                    normalized_file.push(*byte);
                                    was_cr = *byte == b'\r';
                                });

                                if len < BUFFER_SIZE {
                                    break;
                                }
                                len = file.read(&mut buffer)?;
                            }

                            hasher.update(format!("{}", normalized_file.len()).as_bytes());
                            hasher.update(b"\0");

                            hasher.update(&normalized_file);
                        }
                    } else {
                        hasher.update(format!("{}", meta.len()).as_bytes());
                        hasher.update(b"\0");

                        std::io::copy(&mut file, &mut hasher)?;
                    }

                    let hash: [u8; 20] = hasher.finalize().into();
                    ObjectId::from(hash)
                };

                Some((entry_mode, oid))
            } else {
                // It's a non-symlink folder. Git doesn't track folders. Same as deletion.
                None
            }
        }
        Err(_) => None,
    })
}
