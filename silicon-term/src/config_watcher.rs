use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

pub struct ConfigWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    rx: mpsc::Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    watched_paths: Vec<PathBuf>,
}

impl ConfigWatcher {
    pub fn new() -> Option<Self> {
        let (tx, rx) = mpsc::channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), tx).ok()?;

        let mut watched_paths = Vec::new();

        // Watch global config directory
        let global_config = silicon_loader::config_dir();
        if global_config.exists()
            && debouncer
                .watcher()
                .watch(&global_config, notify::RecursiveMode::NonRecursive)
                .is_ok()
        {
            watched_paths.push(global_config.join("init.lua"));
        }

        // Watch workspace config directory
        let (workspace_root, _) = silicon_loader::find_workspace();
        let workspace_config_dir = workspace_root.join(".silicon");
        if workspace_config_dir.exists()
            && debouncer
                .watcher()
                .watch(&workspace_config_dir, notify::RecursiveMode::NonRecursive)
                .is_ok()
        {
            watched_paths.push(workspace_config_dir.join("init.lua"));
        }

        if watched_paths.is_empty() {
            return None;
        }

        Some(ConfigWatcher {
            _debouncer: debouncer,
            rx,
            watched_paths,
        })
    }

    /// Check for config file changes (non-blocking).
    /// Returns true if a watched config file was modified.
    pub fn poll(&self) -> bool {
        let mut changed = false;
        while let Ok(Ok(events)) = self.rx.try_recv() {
            for event in &events {
                if self.watched_paths.contains(&event.path) {
                    changed = true;
                }
            }
        }
        changed
    }
}
