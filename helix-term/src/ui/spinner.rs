use std::{collections::HashMap, time::Instant};

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    inner: HashMap<usize, Spinner>,
}

impl ProgressSpinners {
    pub fn get(&self, id: usize) -> Option<&Spinner> {
        self.inner.get(&id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Spinner> {
        self.inner.get_mut(&id)
    }

    pub fn get_or_create(&mut self, id: usize) -> &mut Spinner {
        self.inner.entry(id).or_default()
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::dots(100)
    }
}

#[derive(Debug)]
pub struct Spinner {
    frames: Vec<&'static str>,
    count: usize,
    last_frame: Instant,
    is_stopped: bool,
    interval: u64,
    idx: usize,
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
            last_frame: Instant::now(),
            is_stopped: true,
            interval,
            idx: 0,
        }
    }

    pub fn dots(interval: u64) -> Self {
        Self::new(vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], interval)
    }

    pub fn start(&mut self) {
        self.is_stopped = false;
    }

    pub fn frame(&mut self) -> Option<&str> {
        if self.is_stopped {
            None
        } else {
            if Instant::now().duration_since(self.last_frame).as_millis() as u64 >= self.interval {
                self.idx = (self.idx + 1) % self.count;
                self.last_frame = Instant::now();
            }
            self.frames.get(self.idx).copied()
        }
    }

    pub fn stop(&mut self) {
        self.is_stopped = true;
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped
    }
}
