use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileState {
    Missing,
    Present {
        modified: Option<SystemTime>,
        len: u64,
        readonly: bool,
        #[cfg(unix)]
        device: u64,
        #[cfg(unix)]
        inode: u64,
        #[cfg(unix)]
        changed: (i64, i64),
    },
}

impl FileState {
    fn read(path: &Path) -> Self {
        match fs::metadata(path) {
            Ok(metadata) => Self::Present {
                modified: metadata.modified().ok(),
                len: metadata.len(),
                readonly: metadata.permissions().readonly(),
                #[cfg(unix)]
                device: metadata.dev(),
                #[cfg(unix)]
                inode: metadata.ino(),
                #[cfg(unix)]
                changed: (metadata.ctime(), metadata.ctime_nsec()),
            },
            Err(_) => Self::Missing,
        }
    }
}

/// Watches the parent directories of open files.
///
/// Watching directories instead of files is important because atomic saves replace the file and
/// would otherwise detach the watch from the new inode. Each file is watched directly as well so
/// changes through symlinks and hardlinks are reported. `FileState` filters duplicate backend
/// notifications and notifications caused by Helix's own writes.
pub(crate) struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: UnboundedReceiver<notify::Result<Event>>,
    watched_files: HashMap<PathBuf, FileState>,
    watched_file_handles: HashSet<PathBuf>,
    watched_directories: HashMap<PathBuf, usize>,
}

impl FileWatcher {
    pub(crate) fn new() -> notify::Result<Self> {
        let (sender, receiver) = unbounded_channel();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })?;

        Ok(Self {
            watcher,
            receiver,
            watched_files: HashMap::new(),
            watched_file_handles: HashSet::new(),
            watched_directories: HashMap::new(),
        })
    }

    pub(crate) fn watch(&mut self, path: &Path) -> notify::Result<()> {
        if self.watched_files.contains_key(path) {
            self.refresh(path);
            return Ok(());
        }

        let Some(parent) = path.parent() else {
            return Ok(());
        };

        match self.watched_directories.get_mut(parent) {
            Some(count) => *count += 1,
            None => {
                self.watcher.watch(parent, RecursiveMode::NonRecursive)?;
                self.watched_directories.insert(parent.to_owned(), 1);
            }
        }

        self.watched_files
            .insert(path.to_owned(), FileState::read(path));
        self.watch_file_handle(path);
        Ok(())
    }

    pub(crate) fn unwatch(&mut self, path: &Path) {
        if self.watched_files.remove(path).is_none() {
            return;
        }

        self.unwatch_file_handle(path);

        let Some(parent) = path.parent() else {
            return;
        };

        let remove_parent = match self.watched_directories.get_mut(parent) {
            Some(1) => true,
            Some(count) => {
                *count -= 1;
                false
            }
            None => false,
        };

        if remove_parent {
            self.watched_directories.remove(parent);
            if let Err(err) = self.watcher.unwatch(parent) {
                log::debug!("failed to stop watching {}: {err}", parent.display());
            }
        }
    }

    /// Records the current disk state after a write performed by Helix.
    pub(crate) fn refresh(&mut self, path: &Path) {
        if let Some(state) = self.watched_files.get_mut(path) {
            *state = FileState::read(path);
            self.unwatch_file_handle(path);
            self.watch_file_handle(path);
        }
    }

    fn watch_file_handle(&mut self, path: &Path) {
        if !path.exists() || self.watched_file_handles.contains(path) {
            return;
        }

        match self.watcher.watch(path, RecursiveMode::NonRecursive) {
            Ok(()) => {
                self.watched_file_handles.insert(path.to_owned());
            }
            Err(err) => log::debug!("failed to watch file {} directly: {err}", path.display()),
        }
    }

    fn unwatch_file_handle(&mut self, path: &Path) {
        if !self.watched_file_handles.remove(path) {
            return;
        }
        if let Err(err) = self.watcher.unwatch(path) {
            log::debug!(
                "failed to stop watching file {} directly: {err}",
                path.display()
            );
        }
    }

    pub(crate) async fn recv(&mut self) -> Option<Vec<PathBuf>> {
        loop {
            match self.receiver.recv().await? {
                Ok(event) => {
                    let paths = self.changed_paths(event);
                    if !paths.is_empty() {
                        return Some(paths);
                    }
                }
                Err(err) => log::warn!("file watcher error: {err}"),
            }
        }
    }

    fn changed_paths(&mut self, event: Event) -> Vec<PathBuf> {
        if matches!(event.kind, EventKind::Access(_)) {
            return Vec::new();
        }

        let mut candidates = HashSet::new();
        if event.paths.is_empty() {
            candidates.extend(self.watched_files.keys().cloned());
        }
        for event_path in event.paths {
            let event_path = helix_stdx::path::canonicalize(event_path);
            if self.watched_files.contains_key(&event_path) {
                candidates.insert(event_path);
            } else if self.watched_directories.contains_key(&event_path) {
                candidates.extend(
                    self.watched_files
                        .keys()
                        .filter(|path| path.parent() == Some(event_path.as_path()))
                        .cloned(),
                );
            }
        }

        candidates.retain(|path| {
            let new_state = FileState::read(path);
            let state = self
                .watched_files
                .get_mut(path)
                .expect("candidate paths are watched");
            if *state == new_state {
                false
            } else {
                *state = new_state;
                true
            }
        });

        candidates.into_iter().collect()
    }
}
