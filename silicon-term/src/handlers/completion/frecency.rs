use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static FRECENCY: Lazy<FrecencyIndex> = Lazy::new(|| {
    let data_dir = silicon_loader::data_dir();
    FrecencyIndex::load(&data_dir)
});

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrecencyKey {
    pub label: String,
    pub kind: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrecencyEntry {
    count: u32,
    last_used: u64,
}

pub struct FrecencyIndex {
    entries: Mutex<HashMap<FrecencyKey, FrecencyEntry>>,
    dirty: Mutex<bool>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl FrecencyIndex {
    fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("completion_history.json");
        let entries = fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default();
        FrecencyIndex {
            entries: Mutex::new(entries),
            dirty: Mutex::new(false),
        }
    }

    pub fn record(&self, key: FrecencyKey) {
        let now = now_secs();
        {
            let mut entries = self.entries.lock().unwrap();
            let entry = entries.entry(key).or_insert(FrecencyEntry {
                count: 0,
                last_used: now,
            });
            entry.count = entry.count.saturating_add(1);
            entry.last_used = now;
        }
        *self.dirty.lock().unwrap() = true;
    }

    /// Returns a multiplier in `1.0..=3.0` for the given completion.
    /// Unknown items return 1.0.
    pub fn score(&self, label: &str, kind: &str, language: &str) -> f32 {
        let entries = self.entries.lock().unwrap();
        let key = FrecencyKey {
            label: label.to_string(),
            kind: kind.to_string(),
            language: language.to_string(),
        };
        let Some(entry) = entries.get(&key) else {
            return 1.0;
        };

        let now = now_secs();

        // 7-day half-life: recency_weight = 2^(-(now - last_used) / 604800)
        let age_secs = now.saturating_sub(entry.last_used) as f64;
        let recency_weight = (2.0_f64).powf(-age_secs / 604800.0) as f32;

        // multiplier = (1.0 + log2(count+1) * recency_weight).min(3.0)
        let count_factor = ((entry.count + 1) as f32).log2();
        (1.0_f32 + count_factor * recency_weight).min(3.0)
    }

    pub fn flush(&self, data_dir: &Path) {
        if !*self.dirty.lock().unwrap() {
            return;
        }

        let mut entries = self.entries.lock().unwrap();

        // Prune entries older than 30 days or with zero count
        let now = now_secs();
        let thirty_days: u64 = 30 * 24 * 3600;
        entries.retain(|_, entry| {
            entry.count > 0 && now.saturating_sub(entry.last_used) < thirty_days
        });

        // Cap at 5000 entries, keeping most recently used
        if entries.len() > 5000 {
            let mut items: Vec<_> = entries.drain().collect();
            items.sort_by(|a, b| b.1.last_used.cmp(&a.1.last_used));
            items.truncate(5000);
            *entries = items.into_iter().collect();
        }

        let path = data_dir.join("completion_history.json");
        let tmp_path = data_dir.join("completion_history.json.tmp");

        if let Ok(data) = serde_json::to_string(&*entries) {
            let _ = fs::create_dir_all(data_dir);
            if fs::write(&tmp_path, &data).is_ok() {
                let _ = fs::rename(&tmp_path, &path);
            }
        }

        *self.dirty.lock().unwrap() = false;
    }
}
