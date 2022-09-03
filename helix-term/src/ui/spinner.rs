use std::{collections::HashMap, time::Instant};

use helix_view::editor::BRAILLE_SPINNER_STRINGS;

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    default: Spinner,
    inner: HashMap<usize, Spinner>,
}

impl ProgressSpinners {
    pub fn get(&self, id: usize) -> Option<&Spinner> {
        self.inner.get(&id)
    }

    pub fn get_or_create(&mut self, id: usize) -> &mut Spinner {
        self.inner.entry(id).or_insert_with(|| self.default.clone())
    }

    pub fn new(default: Spinner) -> Self {
        Self {
            default,
            inner: HashMap::new(),
        }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::dots(80)
    }
}

#[derive(Clone, Debug)]
pub struct Spinner {
    frames: Vec<String>,
    count: usize,
    start: Option<Instant>,
    interval: u64,
}

impl Spinner {
    /// Creates a new spinner with `frames` and `interval`.
    /// If either the frames count or interval is zero, create an empty spinner
    /// that won't display anything.
    pub fn new(frames: Vec<String>, interval: u64) -> Self {
        let count = frames.len();
        if count == 0 || interval == 0 {
            // disable the spinner
            return Self {
                frames: vec!["".to_string()],
                count: 1,
                interval: 80, // this doesn't matter if count == 1
                start: None,
            };
        }

        Self {
            frames,
            count,
            interval,
            start: None,
        }
    }

    pub fn dots(interval: u64) -> Self {
        Self::new(
            BRAILLE_SPINNER_STRINGS
                .into_iter()
                .map(String::from)
                .collect(),
            interval,
        )
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    pub fn frame(&self) -> Option<&str> {
        let idx = if self.count > 1 {
            (self
                .start
                .map(|time| Instant::now().duration_since(time))?
                .as_millis()
                / self.interval as u128) as usize
                % self.count
        } else {
            self.start.and(Some(0))?
        };
        self.frames.get(idx).map(|s| s.as_str())
    }

    pub fn stop(&mut self) {
        self.start = None;
    }

    pub fn is_stopped(&self) -> bool {
        self.start.is_none()
    }
}
