//! Recent workspace history.
//!
//! Records workspace roots opened by the user so they can be listed and
//! jumped to later (`:workspace-history`).
//!
//! ## Storage
//!
//! A single JSON file at `data_dir()/workspace_history.json`, holding an array
//! of entries (most recent first):
//!
//! ```json
//! [{"path": "/home/user/proj", "last_seen": 1728450000}]
//! ```
//!
//! Writes are atomic (temp file + rename) and best-effort: IO failures are
//! logged and never surfaced, so history bookkeeping can never break `:cd`
//! or editor startup.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::data_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub last_seen: u64,
}

pub fn history_file() -> PathBuf {
    data_dir().join("workspace_history.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn canonicalize(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Load recorded entries, most recent first. Returns an empty vec when no
/// history exists yet or the file cannot be parsed.
pub fn load() -> Vec<Entry> {
    load_from(&history_file())
}

/// Record `workspace` as recently used: upsert, prune missing directories,
/// cap at `max_entries`, and persist atomically. Never fails.
pub fn record(workspace: &Path, enable: bool, max_entries: usize) {
    record_to(&history_file(), workspace, enable, max_entries, now_unix())
}

/// Forget a single workspace root. Never fails.
pub fn remove(path: &Path) {
    let file = history_file();
    let path = canonicalize(path);
    let entries: Vec<Entry> = load_from(&file)
        .into_iter()
        .filter(|e| e.path != path)
        .collect();
    save_to(&file, &entries);
}

/// Clear all recorded history. Never fails.
pub fn clear() {
    if let Err(err) = fs::remove_file(history_file()) {
        if err.kind() != std::io::ErrorKind::NotFound {
            log::warn!("could not clear workspace history: {err}");
        }
    }
}

fn load_from(file: &Path) -> Vec<Entry> {
    let contents = match fs::read_to_string(file) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            log::warn!("could not read workspace history: {err}");
            return Vec::new();
        }
    };
    let mut entries = match serde_json::from_str::<Vec<Entry>>(&contents) {
        Ok(entries) => entries,
        Err(_) => {
            log::warn!("ignoring unparsable workspace history file");
            return Vec::new();
        }
    };
    entries.sort_by_key(|e| std::cmp::Reverse(e.last_seen));
    entries
}

fn record_to(file: &Path, workspace: &Path, enable: bool, max_entries: usize, now: u64) {
    if !enable || max_entries == 0 {
        return;
    }
    let path = canonicalize(workspace);
    let mut entries = load_from(file);
    // Drop entries whose directories no longer exist (except the one being
    // recorded) so stale paths don't accumulate forever.
    entries.retain(|e| e.path == path || e.path.exists());
    match entries.iter_mut().find(|e| e.path == path) {
        Some(entry) => {
            entry.last_seen = now;
        }
        None => entries.push(Entry {
            path,
            last_seen: now,
        }),
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.last_seen));
    entries.truncate(max_entries);
    save_to(file, &entries);
}

fn save_to(file: &Path, entries: &[Entry]) {
    if let Some(parent) = file.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            log::warn!("could not create workspace history dir: {err}");
            return;
        }
    }
    let tmp = file.with_extension(format!("tmp.{}", std::process::id()));
    let write_result = serde_json::to_string(entries)
        .map_err(|err| err.to_string())
        .and_then(|json| fs::write(&tmp, json).map_err(|err| err.to_string()));
    if let Err(err) = write_result {
        log::warn!("could not write workspace history: {err}");
        let _ = fs::remove_file(&tmp);
        return;
    }
    if let Err(err) = fs::rename(&tmp, file) {
        log::warn!("could not persist workspace history: {err}");
        let _ = fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_file_in(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("workspace_history.json")
    }

    #[test]
    fn load_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_from(&history_file_in(&dir)).is_empty());
    }

    #[test]
    fn load_broken_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = history_file_in(&dir);
        fs::write(&file, "{ not json").unwrap();
        assert!(load_from(&file).is_empty());
    }

    #[test]
    fn record_upsert_bumps_recency() {
        let dir = tempfile::tempdir().unwrap();
        let file = history_file_in(&dir);
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        record_to(&file, &a, true, 10, 100);
        record_to(&file, &b, true, 10, 200);
        record_to(&file, &a, true, 10, 300);

        let entries = load_from(&file);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, canonicalize(&a));
        assert_eq!(entries[0].last_seen, 300);
        assert_eq!(entries[1].path, canonicalize(&b));
    }

    #[test]
    fn record_truncates_to_max_entries() {
        let dir = tempfile::tempdir().unwrap();
        let file = history_file_in(&dir);
        for name in ["a", "b", "c"] {
            let path = dir.path().join(name);
            fs::create_dir_all(&path).unwrap();
        }
        record_to(&file, &dir.path().join("a"), true, 2, 100);
        record_to(&file, &dir.path().join("b"), true, 2, 200);
        record_to(&file, &dir.path().join("c"), true, 2, 300);

        let entries = load_from(&file);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, canonicalize(&dir.path().join("c")));
        assert_eq!(entries[1].path, canonicalize(&dir.path().join("b")));
    }

    #[test]
    fn record_prunes_missing_directories() {
        let dir = tempfile::tempdir().unwrap();
        let file = history_file_in(&dir);
        let gone = dir.path().join("gone");
        fs::create_dir_all(&gone).unwrap();
        record_to(&file, &gone, true, 10, 100);
        assert_eq!(load_from(&file).len(), 1);

        fs::remove_dir_all(&gone).unwrap();
        let other = dir.path().join("other");
        fs::create_dir_all(&other).unwrap();
        record_to(&file, &other, true, 10, 200);

        let entries = load_from(&file);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, canonicalize(&other));
    }

    #[test]
    fn record_disabled_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let file = history_file_in(&dir);
        record_to(&file, dir.path(), false, 10, 100);
        assert!(!file.exists());
    }
}
