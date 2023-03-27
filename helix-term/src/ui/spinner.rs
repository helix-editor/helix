use std::collections::HashMap;
use std::time::Instant;

// These two options coupled together makes it just nice for the progress to
// spin one turn in almost ~1s (1024ms), given that we display 128ms per frame
// with 8 items in total, we render every 16ms (~60fps), so it renders new
// progress icon every 8 frames.
const FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
// Using multiples of 2 allow compiling down to simpler instructions
const INTERVAL: u128 = 128;

#[derive(Debug, Default)]
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

    /// Check if spinning is needed, only needed to do this check for interval
    /// but not the usual render to reduce the number of render needed.
    pub fn spinning(&self, last_render: Instant) -> bool {
        // only render if spinner should change after last render
        !self.inner.is_empty() && last_render.elapsed().as_millis() > INTERVAL
    }

    pub fn frame(&self, id: usize) -> Option<&str> {
        let start = self.inner.get(&id)?;
        let idx = (start.elapsed().as_millis() / INTERVAL) as usize % FRAMES.len();

        Some(FRAMES[idx])
    }
}
