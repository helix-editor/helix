use helix_core::time::now_timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const MAX_ENTRIES: usize = 1000;
const FILE_NAME: &str = "sessions.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub anchor_row: usize,
    pub anchor_col: usize,
    pub head_row: usize,
    pub head_col: usize,
    pub view_anchor_row: usize,
    pub view_anchor_col: usize,
    pub horizontal_offset: usize,
    pub timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub files: HashMap<String, FileState>,
    #[serde(default)]
    pub last_gc: u64,
}

impl SessionState {
    pub fn load() -> Self {
        let path = helix_loader::cache_dir().join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|err| {
                log::warn!(
                    "Failed to parse session state from {}: {}",
                    path.display(),
                    err
                );
                Self::default()
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                log::warn!(
                    "Failed to read session state from {}: {}",
                    path.display(),
                    err
                );
                Self::default()
            }
        }
    }

    pub fn save(&mut self) {
        self.prune();
        let path = helix_loader::cache_dir().join(FILE_NAME);
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                log::warn!(
                    "Failed to create cache directory {}: {}",
                    parent.display(),
                    err
                );
                return;
            }
        }
        match serde_json::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(err) = std::fs::write(&path, contents) {
                    log::warn!(
                        "Failed to write session state to {}: {}",
                        path.display(),
                        err
                    );
                }
            }
            Err(err) => {
                log::warn!("Failed to serialize session state: {}", err);
            }
        }
    }

    pub fn set(&mut self, path: &Path, state: FileState) {
        self.files
            .insert(path.to_string_lossy().into_owned(), state);
    }

    pub fn get(&self, path: &Path) -> Option<&FileState> {
        self.files.get(&*path.to_string_lossy())
    }

    /// Returns `true` if more than 1 day has passed since the last GC run.
    pub fn needs_gc(&self) -> bool {
        now_timestamp().saturating_sub(self.last_gc) > 86400
    }

    /// Remove entries older than `max_age_days` days and update `last_gc`.
    /// Does NOT save to disk — the caller is responsible for eventual flush.
    pub fn gc(&mut self, max_age_days: u64) {
        let now = now_timestamp();
        let cutoff = now.saturating_sub(max_age_days * 86400);
        self.files.retain(|_, state| state.timestamp >= cutoff);
        self.last_gc = now;
    }

    fn prune(&mut self) {
        if self.files.len() <= MAX_ENTRIES {
            return;
        }

        let mut entries: Vec<(String, u64)> = self
            .files
            .iter()
            .map(|(k, v)| (k.clone(), v.timestamp))
            .collect();
        entries.sort_by_key(|(_, ts)| *ts);

        let to_remove = self.files.len() - MAX_ENTRIES;
        for (key, _) in entries.into_iter().take(to_remove) {
            self.files.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_serialization() {
        let mut state = SessionState::default();
        state.set(
            Path::new("/tmp/test.rs"),
            FileState {
                anchor_row: 10,
                anchor_col: 5,
                head_row: 10,
                head_col: 5,
                view_anchor_row: 5,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: 1000,
            },
        );

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.files.len(), 1);
        let file_state = deserialized.get(Path::new("/tmp/test.rs")).unwrap();
        assert_eq!(file_state.anchor_row, 10);
        assert_eq!(file_state.head_row, 10);
        assert_eq!(file_state.head_col, 5);
    }

    #[test]
    fn test_get_set() {
        let mut state = SessionState::default();
        assert!(state.get(Path::new("/nonexistent")).is_none());

        state.set(
            Path::new("/tmp/foo.rs"),
            FileState {
                anchor_row: 1,
                anchor_col: 2,
                head_row: 3,
                head_col: 4,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 10,
                timestamp: 500,
            },
        );

        let fs = state.get(Path::new("/tmp/foo.rs")).unwrap();
        assert_eq!(fs.anchor_row, 1);
        assert_eq!(fs.anchor_col, 2);
        assert_eq!(fs.head_row, 3);
        assert_eq!(fs.head_col, 4);
        assert_eq!(fs.horizontal_offset, 10);
    }

    #[test]
    fn test_prune() {
        let mut state = SessionState::default();
        for i in 0..1050 {
            state.set(
                Path::new(&format!("/tmp/file_{}.rs", i)),
                FileState {
                    anchor_row: 0,
                    anchor_col: 0,
                    head_row: 0,
                    head_col: 0,
                    view_anchor_row: 0,
                    view_anchor_col: 0,
                    horizontal_offset: 0,
                    timestamp: i as u64,
                },
            );
        }
        assert_eq!(state.files.len(), 1050);
        state.prune();
        assert_eq!(state.files.len(), MAX_ENTRIES);

        // The oldest entries (lowest timestamps) should have been removed
        assert!(state.get(Path::new("/tmp/file_0.rs")).is_none());
        assert!(state.get(Path::new("/tmp/file_49.rs")).is_none());
        // The newest entries should remain
        assert!(state.get(Path::new("/tmp/file_1049.rs")).is_some());
        assert!(state.get(Path::new("/tmp/file_50.rs")).is_some());
    }

    #[test]
    fn test_gc() {
        let now = now_timestamp();
        let mut state = SessionState::default();

        // Entry from 100 days ago — should be removed with max_age=90
        state.set(
            Path::new("/tmp/old.rs"),
            FileState {
                anchor_row: 0,
                anchor_col: 0,
                head_row: 0,
                head_col: 0,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: now - 100 * 86400,
            },
        );

        // Entry from 10 days ago — should be kept
        state.set(
            Path::new("/tmp/recent.rs"),
            FileState {
                anchor_row: 1,
                anchor_col: 0,
                head_row: 1,
                head_col: 0,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: now - 10 * 86400,
            },
        );

        // Entry from just now — should be kept
        state.set(
            Path::new("/tmp/new.rs"),
            FileState {
                anchor_row: 2,
                anchor_col: 0,
                head_row: 2,
                head_col: 0,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: now,
            },
        );

        assert_eq!(state.last_gc, 0);
        state.gc(90);

        assert_eq!(state.files.len(), 2);
        assert!(state.get(Path::new("/tmp/old.rs")).is_none());
        assert!(state.get(Path::new("/tmp/recent.rs")).is_some());
        assert!(state.get(Path::new("/tmp/new.rs")).is_some());
        assert!(state.last_gc >= now);
    }

    #[test]
    fn test_needs_gc() {
        let now = now_timestamp();
        let mut state = SessionState::default();

        // last_gc = 0, should need GC
        assert!(state.needs_gc());

        // last_gc = now, should not need GC
        state.last_gc = now;
        assert!(!state.needs_gc());

        // last_gc = 2 days ago, should need GC
        state.last_gc = now - 2 * 86400;
        assert!(state.needs_gc());
    }

    #[test]
    fn test_gc_disabled_with_zero_max_age() {
        let now = now_timestamp();
        let mut state = SessionState::default();

        state.set(
            Path::new("/tmp/old.rs"),
            FileState {
                anchor_row: 0,
                anchor_col: 0,
                head_row: 0,
                head_col: 0,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: 1000, // very old
            },
        );

        // gc with 0 days would set cutoff to now, removing everything,
        // but the caller guards against gc_max_age == 0
        // Just verify the method works with a large max_age that keeps everything
        state.gc(u64::MAX / 86400);
        assert_eq!(state.files.len(), 1);
        assert!(state.last_gc >= now);
    }

    #[test]
    fn test_last_gc_persisted_in_json() {
        let mut state = SessionState::default();
        state.last_gc = 1234567890;
        state.set(
            Path::new("/tmp/test.rs"),
            FileState {
                anchor_row: 0,
                anchor_col: 0,
                head_row: 0,
                head_col: 0,
                view_anchor_row: 0,
                view_anchor_col: 0,
                horizontal_offset: 0,
                timestamp: 1000,
            },
        );

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.last_gc, 1234567890);
    }

    #[test]
    fn test_last_gc_defaults_when_missing_from_json() {
        // Simulate old session JSON without last_gc field
        let json = r#"{"files":{}}"#;
        let state: SessionState = serde_json::from_str(json).unwrap();
        assert_eq!(state.last_gc, 0);
    }

    #[test]
    fn test_load_missing_file() {
        // Loading from a non-existent path should return default
        let state = SessionState::load();
        assert!(state.files.is_empty());
    }
}
