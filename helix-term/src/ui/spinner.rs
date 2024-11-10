use std::{collections::HashMap, time::Instant};

use helix_lsp::LanguageServerId;

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    inner: HashMap<LanguageServerId, Spinner>,
}

impl ProgressSpinners {
    pub fn get(&self, id: LanguageServerId) -> Option<&Spinner> {
        self.inner.get(&id)
    }

    pub fn get_or_create(&mut self, id: LanguageServerId) -> &mut Spinner {
        self.inner.entry(id).or_default()
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::dots(80)
    }
}

#[derive(Debug)]
pub struct Spinner {
    frames: Vec<&'static str>,
    count: usize,
    start: Option<Instant>,
    interval: u64,
}

impl Spinner {
    /// Creates a new spinner with `frames` and `interval`.
    /// Expects the frames count and interval to be greater than 0.
    pub fn new(frames: Vec<&'static str>, interval: u64) -> Self {
        let count = frames.len();
        assert!(count > 0);
        assert!(interval > 0);

        Self {
            frames,
            count,
            interval,
            start: None,
        }
    }

    pub fn dots(interval: u64) -> Self {
        Self::new(vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], interval)
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    pub fn frame(&self) -> Option<&str> {
        let idx = (self
            .start
            .map(|time| Instant::now().duration_since(time))?
            .as_millis()
            / self.interval as u128) as usize
            % self.count;

        self.frames.get(idx).copied()
    }

    pub fn stop(&mut self) {
        self.start = None;
    }

    pub fn is_stopped(&self) -> bool {
        self.start.is_none()
    }
}
