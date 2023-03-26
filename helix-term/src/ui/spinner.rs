use std::{collections::HashMap, time::Instant};

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    inner: HashMap<usize, Spinner>,
}

impl ProgressSpinners {
    pub fn start(&mut self, id: usize) {
        self.inner
            .entry(id)
            .or_insert_with(Spinner::default)
            .start();
    }

    pub fn stop(&mut self, id: usize) {
        self.inner.entry(id).or_insert_with(Spinner::default).stop();
    }

    pub fn frame(&self, id: usize) -> Option<&str> {
        self.inner.get(&id).and_then(|spinner| spinner.frame())
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::dots(80)
    }
}

#[derive(Debug)]
struct Spinner {
    frames: Vec<&'static str>,
    count: usize,
    start: Option<Instant>,
    interval: u64,
}

impl Spinner {
    /// Creates a new spinner with `frames` and `interval`.
    /// Expects the frames count and interval to be greater than 0.
    fn new(frames: Vec<&'static str>, interval: u64) -> Self {
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

    fn dots(interval: u64) -> Self {
        Self::new(vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], interval)
    }

    fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    fn frame(&self) -> Option<&str> {
        let idx = (self
            .start
            .map(|time| Instant::now().duration_since(time))?
            .as_millis()
            / self.interval as u128) as usize
            % self.count;

        self.frames.get(idx).copied()
    }

    fn stop(&mut self) {
        self.start = None;
    }
}
