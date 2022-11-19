use std::mem::swap;
use std::ops::Range;
use std::sync::Arc;

use helix_core::{Rope, RopeSlice};
use imara_diff::intern::InternedInput;
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{Notify, RwLockReadGuard};
use tokio::time::{timeout, timeout_at, Duration, Instant};

use crate::diff::{
    Event, RedrawHandle, RenderStrategy, ALGORITHM, DIFF_DEBOUNCE_TIME_ASYNC,
    DIFF_DEBOUNCE_TIME_SYNC, SYNC_DIFF_TIMEOUT,
};

use super::line_cache::InternedRopeLines;
use super::Hunk;

#[cfg(test)]
mod test;

pub(super) struct DiffWorker {
    pub channel: UnboundedReceiver<Event>,
    pub hunks: Arc<Mutex<Vec<Hunk>>>,
    pub new_hunks: Vec<Hunk>,
    pub redraw_handle: RedrawHandle,
    pub difff_finished_notify: Arc<Notify>,
}

impl DiffWorker {
    async fn accumulate_events(&mut self, event: Event) -> (Option<Rope>, Option<Rope>) {
        let mut accumulator = EventAccumulator::new(&self.redraw_handle);
        accumulator.handle_event(event).await;
        accumulator
            .accumulate_debounced_events(
                &mut self.channel,
                self.redraw_handle.clone(),
                self.difff_finished_notify.clone(),
            )
            .await;
        (accumulator.doc, accumulator.diff_base)
    }

    pub async fn run(mut self, diff_base: Rope, doc: Rope) {
        let mut interner = InternedRopeLines::new(diff_base, doc);
        if let Some(lines) = interner.interned_lines() {
            self.perform_diff(lines);
        }
        self.apply_hunks();
        while let Some(event) = self.channel.recv().await {
            let (doc, diff_base) = self.accumulate_events(event).await;

            let process_accumulated_events = || {
                if let Some(new_base) = diff_base {
                    interner.update_diff_base(new_base, doc)
                } else {
                    interner.update_doc(doc.unwrap())
                }

                if let Some(lines) = interner.interned_lines() {
                    self.perform_diff(lines)
                }
            };

            // Calculating diffs is computationally expensive and should
            // not run inside an async function to avoid blocking other futures.
            // Note: tokio::task::block_in_place does not work during tests
            #[cfg(test)]
            process_accumulated_events();
            #[cfg(not(test))]
            tokio::task::block_in_place(process_accumulated_events);

            self.apply_hunks();
        }
    }

    /// update the hunks (used by the gutter) by replacing it with `self.new_hunks`.
    /// `self.new_hunks` is always empty after this function runs.
    /// To improve performance this function tries to reuse the allocation of the old diff previously stored in `self.line_diffs`
    fn apply_hunks(&mut self) {
        swap(&mut *self.hunks.lock(), &mut self.new_hunks);
        self.difff_finished_notify.notify_waiters();
        self.new_hunks.clear();
    }

    fn perform_diff(&mut self, input: &InternedInput<RopeSlice>) {
        imara_diff::diff(ALGORITHM, input, |before: Range<u32>, after: Range<u32>| {
            self.new_hunks.push(Hunk { before, after })
        })
    }
}

struct EventAccumulator<'a> {
    diff_base: Option<Rope>,
    doc: Option<Rope>,
    render_stratagey: RenderStrategy,
    redraw_handle: &'a RedrawHandle,
    render_lock: Option<RwLockReadGuard<'a, ()>>,
    timeout: Instant,
}

impl<'a> EventAccumulator<'a> {
    fn new(redraw_handle: &'a RedrawHandle) -> EventAccumulator<'a> {
        EventAccumulator {
            diff_base: None,
            doc: None,
            render_stratagey: RenderStrategy::Async,
            render_lock: None,
            redraw_handle,
            timeout: Instant::now(),
        }
    }

    async fn handle_event(&mut self, event: Event) {
        let dst = if event.is_base {
            &mut self.diff_base
        } else {
            &mut self.doc
        };

        *dst = Some(event.text);

        // always prefer the most synchronus requested render mode
        if event.render_strategy > self.render_stratagey {
            if self.render_lock.is_none() {
                self.timeout = Instant::now() + Duration::from_millis(SYNC_DIFF_TIMEOUT);
                self.render_lock = Some(self.redraw_handle.1.read().await);
            }
            self.render_stratagey = event.render_strategy
        }
    }

    async fn accumulate_debounced_events(
        &mut self,
        channel: &mut UnboundedReceiver<Event>,
        redraw_handle: RedrawHandle,
        diff_finished_notify: Arc<Notify>,
    ) {
        let async_debounce = Duration::from_millis(DIFF_DEBOUNCE_TIME_ASYNC);
        let sync_debounce = Duration::from_millis(DIFF_DEBOUNCE_TIME_SYNC);
        loop {
            let debounce = if self.render_stratagey == RenderStrategy::Async {
                async_debounce
            } else {
                sync_debounce
            };

            if let Ok(Some(event)) = timeout(debounce, channel.recv()).await {
                self.handle_event(event).await;
            } else {
                break;
            }
        }

        // setup task to trigger the rendering
        // with the choosen render stragey
        match self.render_stratagey {
            RenderStrategy::Async => {
                tokio::spawn(async move {
                    diff_finished_notify.notified().await;
                    redraw_handle.0.notify_one();
                });
            }
            RenderStrategy::SyncWithTimeout => {
                let timeout = self.timeout;
                tokio::spawn(async move {
                    let res = {
                        // Aquire a lock on the redraw handle.
                        // The lock will block the rendering from occuring while held.
                        // The rendering waits for the diff if it doesn't time out
                        let _render_guard = redraw_handle.1.read();
                        timeout_at(timeout, diff_finished_notify.notified()).await
                    };
                    if res.is_ok() {
                        // Diff finished in time we are done.
                        return;
                    }
                    // Diff failed to complete in time log the event
                    // and wait until the diff occurs to trigger an async redraw
                    log::warn!("Diff computation timed out, update of diffs might appear delayed");
                    diff_finished_notify.notified().await;
                    redraw_handle.0.notify_one();
                });
            }
            RenderStrategy::Sync => {
                tokio::spawn(async move {
                    // Aquire a lock on the redraw handle.
                    // The lock will block the rendering from occuring while held.
                    let _render_guard = redraw_handle.1.read();
                    diff_finished_notify.notified().await
                });
            }
        };
    }
}
