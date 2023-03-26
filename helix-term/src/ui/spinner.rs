use std::collections::HashMap;
use std::time::Instant;

const FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
const INTERVAL: u128 = 80;

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    inner: HashMap<usize, Instant>,
}

impl ProgressSpinners {
    pub fn start(&mut self, id: usize) {
        self.inner.entry(id).or_insert_with(Instant::now);
    }

    pub fn stop(&mut self, id: usize) {
        self.inner.remove(&id);
    }

    pub fn frame(&self, id: usize) -> Option<&str> {
        let start = self.inner.get(&id)?;
        let idx =
            (Instant::now().duration_since(*start).as_millis() / INTERVAL) as usize % FRAMES.len();

        Some(FRAMES[idx])
    }
}
