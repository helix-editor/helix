use std::collections::HashMap;
use std::time::Instant;

const FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
// Using multiples of 2 allow compiling down to simpler instructions
const INTERVAL: u128 = 128;

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

    pub fn spinning(&self) -> bool {
        !self.inner.is_empty()
    }

    pub fn frame(&self, id: usize) -> Option<&str> {
        let start = self.inner.get(&id)?;
        let idx =
            (Instant::now().duration_since(*start).as_millis() / INTERVAL) as usize % FRAMES.len();

        Some(FRAMES[idx])
    }
}
