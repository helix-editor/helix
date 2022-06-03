use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobMatcher};
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use slotmap::SlotMap;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::task::JoinHandle;

pub use notify::{event as notify_event, Event};

// A list of directory names to ignore when found in the root of
// a workspace. These are directories that would otherwise generate
// a very large amount of events which could cause stuttering.
// i.e. `rust-analyzer` calls `cargo check` on every write
// which generates a large amount of fingerprint file changes in the
// `target` directory.
const IGNORE: &[&str] = &["target"];

/// Handle used to interact with the file watcher task.
pub struct FileWatcher {
    tx: UnboundedSender<WatcherEvent>,
    task: JoinHandle<()>,
}

enum WatcherEvent {
    Notify(notify::Event),
    Register(Watch, oneshot::Sender<WatchId>),
    Unregister(WatchId),
    AddWorkspace(PathBuf),
    RemoveWorkspace(PathBuf),
    Shutdown,
}

slotmap::new_key_type! {
    /// Token obtained from watch registration, needed for deregistration.
    pub struct WatchId;
}

/// A watch registered with a `FileWatcher`: `callback` is called with any
/// events from the workspace `workspace` that match the given filter.
struct Watch {
    callback: Box<Callback>,
    workspace: PathBuf,
    filter: Filter,
}

type Callback = dyn FnMut(&Event) + Send + 'static;

impl Watch {
    fn wants(&self, workspace: &Path, path: &Path) -> bool {
        workspace == self.workspace
            && match &self.filter {
                Filter::Glob(glob) => glob.is_match(path),
                Filter::Custom(f) => f(path),
                Filter::None => true,
            }
    }
}

// TODO: This could be replaced with just the filter closure, moving glob logic
// to the LSP side. Not sure what's preferable. Conversely, currently filtering
// which event kinds are actually wanted is done on the LSP side, which is a bit
// weird.
enum Filter {
    Glob(GlobMatcher),
    Custom(Box<CustomFilter>),
    None,
}

type CustomFilter = dyn Fn(&Path) -> bool + Send + 'static;

impl FileWatcher {
    /// Start the file watcher task, which allows registering a callback to
    /// filesystem events concerning paths matching a given filter.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, state) = Dispatcher::new()?;
        let task = tokio::spawn(state.run());

        Ok(Self { task, tx })
    }

    /// Register a callback that is called on any filesystem event in the
    /// monitored directory.
    pub async fn register<F>(&self, workspace: PathBuf, callback: F) -> anyhow::Result<WatchId>
    where
        F: FnMut(&Event) + Send + 'static,
    {
        self.register_filtered(workspace, Filter::None, Box::new(callback))
            .await
    }

    /// Register a callback that is called on filesystem events concerning paths
    /// matching the given glob string.
    pub async fn register_glob<F>(
        &self,
        workspace: PathBuf,
        glob: &str,
        callback: F,
    ) -> anyhow::Result<WatchId>
    where
        F: FnMut(&Event) + Send + 'static,
    {
        let filter = Filter::Glob(
            GlobBuilder::new(glob)
                .literal_separator(true)
                .build()?
                .compile_matcher(),
        );
        self.register_filtered(workspace, filter, Box::new(callback))
            .await
    }

    /// Register a callback that is called on filesystem events concerning paths
    /// matching a provided filter closure that returns true for desired paths.
    pub async fn register_custom<F, FF>(
        &self,
        workspace: PathBuf,
        filter: FF,
        callback: F,
    ) -> anyhow::Result<WatchId>
    where
        F: FnMut(&Event) + Send + 'static,
        FF: Fn(&Path) -> bool + Send + 'static,
    {
        self.register_filtered(
            workspace,
            Filter::Custom(Box::new(filter)),
            Box::new(callback),
        )
        .await
    }

    async fn register_filtered(
        &self,
        workspace: PathBuf,
        filter: Filter,
        callback: Box<Callback>,
    ) -> anyhow::Result<WatchId> {
        let (tx, rx) = oneshot::channel();

        let watch = Watch {
            filter,
            workspace,
            callback,
        };
        let ev = WatcherEvent::Register(watch, tx);
        if let Err(e) = self.tx.send(ev) {
            anyhow::bail!("file watcher channel error: {}", e);
        }

        Ok(rx.await?)
    }

    /// Unregister a callback given a `WatchId` obtained from its registration.
    pub fn unregister(&self, id: WatchId) {
        let _ = self.tx.send(WatcherEvent::Unregister(id));
    }

    // TODO: Currently `add_workspace` and `remove_workspace` fail silently
    // if given invalid workspace names.

    /// Add a workspace to the list of watched workspaces. Events are only emitted
    /// for watched workspaces.
    pub fn add_workspace(&self, workspace_root: PathBuf) {
        let _ = self.tx.send(WatcherEvent::AddWorkspace(workspace_root));
    }

    /// Remove a workspace from the list of watched workspaces.
    pub fn remove_workspace(&self, workspace_root: PathBuf) {
        let _ = self.tx.send(WatcherEvent::RemoveWorkspace(workspace_root));
    }

    // TODO: should this be in a drop function using tokio::spawn instead?
    pub async fn shutdown(self) -> anyhow::Result<()> {
        let _ = self.tx.send(WatcherEvent::Shutdown);
        Ok(self.task.await?)
    }
}

/// Contains the `notify` watcher and state used in the file watching async task.
struct Dispatcher {
    inner: RecommendedWatcher,
    rx: UnboundedReceiver<WatcherEvent>,

    workspaces: HashMap<PathBuf, HashSet<PathBuf>>,
    watches: SlotMap<WatchId, Watch>,
}

impl Dispatcher {
    fn new() -> anyhow::Result<(UnboundedSender<WatcherEvent>, Self)> {
        let (tx, rx) = mpsc::unbounded_channel();

        let inner = {
            let tx = tx.clone();
            RecommendedWatcher::new(move |res: notify::Result<_>| {
                let _ = tx.send(WatcherEvent::Notify(res.unwrap()));
            })?
        };

        let res = Self {
            inner,
            rx,
            workspaces: HashMap::default(),
            watches: SlotMap::default(),
        };

        Ok((tx, res))
    }

    /// Future to run the file watcher task.
    async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            match event {
                WatcherEvent::Notify(event) => {
                    self.handle_raw_event(event);
                }
                WatcherEvent::Register(watch, tx) => {
                    let _ = tx.send(self.watches.insert(watch));
                }
                WatcherEvent::Unregister(id) => {
                    self.watches.remove(id);
                }
                WatcherEvent::AddWorkspace(path) => {
                    self.watch_workspace(path);
                }
                WatcherEvent::RemoveWorkspace(path) => {
                    if let Some(dirs) = self.workspaces.remove(&path) {
                        for dir in &dirs {
                            self.inner.unwatch(dir).unwrap();
                        }
                    }
                }
                WatcherEvent::Shutdown => break,
            }
        }
    }

    fn handle_raw_event(&mut self, event: notify::Event) {
        // Not interested in any events that don't have an associated path.
        let path = match event.paths.get(0) {
            Some(p) => p,
            None => return,
        };

        // Monitor events in root directory to ensure we set watches on new or
        // renamed directories. Note that `notify` seems to already "unwatch"
        // directories when they are removed or renamed.
        // TODO: what to do if a workspace root changes names?
        if let Some(parent) = path.parent() {
            // Must keep the list of watched directories per workspace up to date,
            // as well.
            if let Some(dirs) = self.workspaces.get_mut(parent) {
                match event.kind {
                    EventKind::Create(CreateKind::Folder) => {
                        self.inner.watch(path, RecursiveMode::Recursive).unwrap();
                        dirs.insert(path.to_owned());
                    }
                    EventKind::Remove(RemoveKind::Folder) => {
                        dirs.remove(path);
                    }
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                        // This event kind always involves two paths.
                        let new_path = event.paths.get(1).unwrap();
                        self.inner
                            .watch(new_path, RecursiveMode::Recursive)
                            .unwrap();
                        dirs.insert(new_path.to_owned());
                        dirs.remove(path);
                    }
                    _ => {}
                }
            }
        }

        // Find the root of the workspace that the event's primary path is located in.
        let workspace = match self.workspaces.keys().find(|ws| path.starts_with(ws)) {
            Some(root) => root,
            None => return,
        };

        // Dispatch event to registered callbacks.
        for watch in self
            .watches
            .values_mut()
            .filter(|w| w.wants(workspace, path))
        {
            (watch.callback)(&event);
        }
    }

    fn watch_workspace(&mut self, root: PathBuf) {
        // The set of individual directory watches issued, necessary for unwatching
        // the workspace.
        let mut watches = HashSet::new();

        // Watch root for directories and file changes. This is non-recursive
        // to allow for ignoring specific directories in the root directory.
        // Note that if directories are created or deleted after calling
        // `add_directory`, they must have watches set/unset on them in response
        // to events from this watch call.
        self.inner
            .watch(&root, RecursiveMode::NonRecursive)
            .unwrap();
        watches.insert(root.clone());

        // Watch the root's children directories recursively, ignoring those in
        // the ignore list.
        for entry in fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            // If we can't get the file type, just ignore it.
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| !IGNORE.contains(&s))
                    .unwrap_or(true) // Allow non-utf8 paths.
            })
        {
            self.inner
                .watch(&entry.path(), RecursiveMode::Recursive)
                .unwrap();
            watches.insert(entry.path());
        }

        self.workspaces.insert(root, watches);
    }
}
