use std::collections::HashMap;
use std::time::Instant;

// These two options coupled together makes it just nice for the progress to
// spin one turn in almost ~1s (1024ms), given that we display 128ms per frame
// with 8 items in total, we render every 16ms (~60fps), so it renders new
// progress icon every 8 frames.
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
