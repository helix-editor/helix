use std::mem::swap;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use helix_core::{Rope, RopeSlice};
use imara_diff::intern::InternedInput;
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{timeout_at, Duration, Instant};

use crate::diff::{Event, ALGORITHM, DIFF_DEBOUNCE_TIME};

use super::line_cache::InternedRopeLines;
use super::Hunk;

#[cfg(test)]
mod test;

pub(super) struct DiffWorker {
    pub channel: UnboundedReceiver<Event>,
    pub hunks: Arc<Mutex<Vec<Hunk>>>,
    pub new_hunks: Vec<Hunk>,
    pub notify: Arc<AtomicBool>,
}

impl DiffWorker {
    pub async fn run(mut self, diff_base: Rope, doc: Rope) {
        let mut interner = InternedRopeLines::new(diff_base, doc);
        if let Some(lines) = interner.interned_lines() {
            self.perform_diff(lines);
        }
        self.apply_hunks();
        while let Some(event) = self.channel.recv().await {
            let mut accumulator = EventAccumulator::new();
            accumulator.handle_event(event);
            accumulator
                .accumulate_debounced_events(&mut self.channel)
                .await;

            if let Some(new_base) = accumulator.diff_base {
                interner.update_diff_base(new_base, accumulator.doc)
            } else {
                interner.update_doc(accumulator.doc.unwrap())
            }

            if let Some(lines) = interner.interned_lines() {
                self.perform_diff(lines);
            }
            self.apply_hunks();
        }
    }

    /// update the hunks (used by the gutter) by replacing it with `self.new_hunks`.
    /// `self.new_hunks` is always empty after this function runs.
    /// To improve performance this function tries to reuse the allocation of the old diff previously stored in `self.line_diffs`
    fn apply_hunks(&mut self) {
        swap(&mut *self.hunks.lock(), &mut self.new_hunks);
        self.notify.store(false, Ordering::Relaxed);
        self.new_hunks.clear();
    }

    fn perform_diff(&mut self, input: &InternedInput<RopeSlice>) {
        imara_diff::diff(ALGORITHM, input, |before: Range<u32>, after: Range<u32>| {
            self.new_hunks.push(Hunk { before, after })
        })
    }
}

struct EventAccumulator {
    diff_base: Option<Rope>,
    doc: Option<Rope>,
}

impl EventAccumulator {
    fn new() -> EventAccumulator {
        EventAccumulator {
            diff_base: None,
            doc: None,
        }
    }
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::UpdateDocument(doc) => self.doc = Some(doc),
            Event::UpdateDiffBase(new_diff_base) => self.diff_base = Some(new_diff_base),
        }
    }
    async fn accumulate_debounced_events(&mut self, channel: &mut UnboundedReceiver<Event>) {
        let debounce = Duration::from_millis(DIFF_DEBOUNCE_TIME);
        let timeout = Instant::now() + debounce;
        while let Ok(Some(event)) = timeout_at(timeout, channel.recv()).await {
            self.handle_event(event)
        }
    }
}
