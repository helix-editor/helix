use std::iter::Peekable;
use std::sync::Arc;

use helix_core::Rope;
use helix_event::RenderLockGuard;
use imara_diff::Algorithm;
use parking_lot::{RwLock, RwLockReadGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::diff::worker::DiffWorker;

pub use imara_diff::Hunk;

mod line_cache;
mod worker;

/// A rendering lock passed to the differ the prevents redraws from occurring
struct RenderLock {
    pub lock: RenderLockGuard,
    pub timeout: Option<Instant>,
}

struct Event {
    text: Rope,
    is_base: bool,
    render_lock: Option<RenderLock>,
}

#[derive(Clone, Debug, Default)]
struct DiffInner {
    diff_base: Rope,
    doc: Rope,
    hunks: Vec<Hunk>,
}

/// Representation of a diff that can be updated.
#[derive(Clone, Debug)]
pub struct DiffHandle {
    channel: UnboundedSender<Event>,
    diff: Arc<RwLock<DiffInner>>,
    inverted: bool,
}

impl DiffHandle {
    pub fn new(diff_base: Rope, doc: Rope) -> DiffHandle {
        DiffHandle::new_with_handle(diff_base, doc).0
    }

    fn new_with_handle(diff_base: Rope, doc: Rope) -> (DiffHandle, JoinHandle<()>) {
        let (sender, receiver) = unbounded_channel();
        let diff: Arc<RwLock<DiffInner>> = Arc::default();
        let worker = DiffWorker {
            channel: receiver,
            diff: diff.clone(),
            diff_finished_notify: Arc::default(),
            diff_alloc: imara_diff::Diff::default(),
        };
        let handle = tokio::spawn(worker.run(diff_base, doc));
        let differ = DiffHandle {
            channel: sender,
            diff,
            inverted: false,
        };
        (differ, handle)
    }

    /// Switch base and modified texts' roles
    pub fn invert(&mut self) {
        self.inverted = !self.inverted;
    }

    /// Load the actual diff
    pub fn load(&self) -> Diff {
        Diff {
            diff: self.diff.read(),
            inverted: self.inverted,
        }
    }

    /// Updates the document associated with this redraw handle
    /// This function is only intended to be called from within the rendering loop
    /// if called from elsewhere it may fail to acquire the render lock and panic
    pub fn update_document(&self, doc: Rope, block: bool) -> bool {
        let lock = helix_event::lock_frame();
        let timeout = if block {
            None
        } else {
            Some(Instant::now() + tokio::time::Duration::from_millis(SYNC_DIFF_TIMEOUT))
        };
        self.update_document_impl(doc, self.inverted, Some(RenderLock { lock, timeout }))
    }

    /// Updates the base text of the diff. Returns if the update was successful.
    pub fn update_diff_base(&self, diff_base: Rope) -> bool {
        self.update_document_impl(diff_base, !self.inverted, None)
    }

    fn update_document_impl(
        &self,
        text: Rope,
        is_base: bool,
        render_lock: Option<RenderLock>,
    ) -> bool {
        let event = Event {
            text,
            is_base,
            render_lock,
        };
        self.channel.send(event).is_ok()
    }
}

/// synchronous debounce value should be low
/// so we can update synchronously most of the time
const DIFF_DEBOUNCE_TIME_SYNC: u64 = 1;
/// maximum time that rendering should be blocked until the diff finishes
const SYNC_DIFF_TIMEOUT: u64 = 12;
const DIFF_DEBOUNCE_TIME_ASYNC: u64 = 96;
const ALGORITHM: Algorithm = Algorithm::Histogram;
const MAX_DIFF_LINES: usize = 64 * u16::MAX as usize;
// cap average line length to 128 for files with MAX_DIFF_LINES
const MAX_DIFF_BYTES: usize = MAX_DIFF_LINES * 128;

/// A list of changes in a file sorted in ascending
/// non-overlapping order
#[derive(Debug)]
pub struct Diff<'a> {
    diff: RwLockReadGuard<'a, DiffInner>,
    inverted: bool,
}

impl Diff<'_> {
    /// Returns the base [Rope] of the [Diff]
    pub fn diff_base(&self) -> &Rope {
        if self.inverted {
            &self.diff.doc
        } else {
            &self.diff.diff_base
        }
    }

    /// Returns the [Rope] being compared against
    pub fn doc(&self) -> &Rope {
        if self.inverted {
            &self.diff.diff_base
        } else {
            &self.diff.doc
        }
    }

    pub fn is_inverted(&self) -> bool {
        self.inverted
    }

    /// Returns the `Hunk` for the `n`th change in this file.
    /// if there is no `n`th change  `Hunk::NONE` is returned instead.
    pub fn nth_hunk(&self, n: u32) -> Hunk {
        match self.diff.hunks.get(n as usize) {
            Some(hunk) if self.inverted => hunk.invert(),
            Some(hunk) => hunk.clone(),
            None => Hunk::NONE,
        }
    }

    pub fn len(&self) -> u32 {
        self.diff.hunks.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gives the index of the first hunk after the given line, if one exists.
    pub fn next_hunk(&self, line: u32) -> Option<u32> {
        let hunk_range = if self.inverted {
            |hunk: &Hunk| hunk.before.clone()
        } else {
            |hunk: &Hunk| hunk.after.clone()
        };

        let res = self
            .diff
            .hunks
            .binary_search_by_key(&line, |hunk| hunk_range(hunk).start);

        match res {
            // Search found a hunk that starts exactly at this line, return the next hunk if it exists.
            Ok(pos) if pos + 1 == self.diff.hunks.len() => None,
            Ok(pos) => Some(pos as u32 + 1),

            // No hunk starts exactly at this line, so the search returns
            // the position where a hunk starting at this line should be inserted.
            // That position is exactly the position of the next hunk or the end
            // of the list if no such hunk exists
            Err(pos) if pos == self.diff.hunks.len() => None,
            Err(pos) => Some(pos as u32),
        }
    }

    /// Gives the index of the first hunk before the given line, if one exists.
    pub fn prev_hunk(&self, line: u32) -> Option<u32> {
        let hunk_range = if self.inverted {
            |hunk: &Hunk| hunk.before.clone()
        } else {
            |hunk: &Hunk| hunk.after.clone()
        };
        let res = self
            .diff
            .hunks
            .binary_search_by_key(&line, |hunk| hunk_range(hunk).end);

        match res {
            // Search found a hunk that ends exactly at this line (so it does not include the current line).
            // We can usually just return that hunk, however a special case for empty hunk is necessary
            // which represents a pure removal.
            // Removals are technically empty but are still shown as single line hunks
            // and as such we must jump to the previous hunk (if it exists) if we are already inside the removal
            Ok(pos) if !hunk_range(&self.diff.hunks[pos]).is_empty() => Some(pos as u32),

            // No hunk ends exactly at this line, so the search returns
            // the position where a hunk ending at this line should be inserted.
            // That position before this one is exactly the position of the previous hunk
            Err(0) | Ok(0) => None,
            Err(pos) | Ok(pos) => Some(pos as u32 - 1),
        }
    }

    /// Iterates over all hunks that intersect with the given line ranges.
    ///
    /// Hunks are returned at most once even when intersecting with multiple of the line
    /// ranges.
    pub fn hunks_intersecting_line_ranges<I>(&self, line_ranges: I) -> impl Iterator<Item = &Hunk>
    where
        I: Iterator<Item = (usize, usize)>,
    {
        HunksInLineRangesIter {
            hunks: &self.diff.hunks,
            line_ranges: line_ranges.peekable(),
            inverted: self.inverted,
            cursor: 0,
        }
    }

    /// Returns the index of the hunk containing the given line if it exists.
    pub fn hunk_at(&self, line: u32, include_removal: bool) -> Option<u32> {
        let hunk_range = if self.inverted {
            |hunk: &Hunk| hunk.before.clone()
        } else {
            |hunk: &Hunk| hunk.after.clone()
        };

        let res = self
            .diff
            .hunks
            .binary_search_by_key(&line, |hunk| hunk_range(hunk).start);

        match res {
            // Search found a hunk that starts exactly at this line, return it
            Ok(pos) => Some(pos as u32),

            // No hunk starts exactly at this line, so the search returns
            // the position where a hunk starting at this line should be inserted.
            // The previous hunk contains this hunk if it exists and doesn't end before this line
            Err(0) => None,
            Err(pos) => {
                let hunk = hunk_range(&self.diff.hunks[pos - 1]);
                if hunk.end > line || include_removal && hunk.start == line && hunk.is_empty() {
                    Some(pos as u32 - 1)
                } else {
                    None
                }
            }
        }
    }
}

pub struct HunksInLineRangesIter<'a, I: Iterator<Item = (usize, usize)>> {
    hunks: &'a [Hunk],
    line_ranges: Peekable<I>,
    inverted: bool,
    cursor: usize,
}

impl<'a, I: Iterator<Item = (usize, usize)>> Iterator for HunksInLineRangesIter<'a, I> {
    type Item = &'a Hunk;

    fn next(&mut self) -> Option<Self::Item> {
        let hunk_range = if self.inverted {
            |hunk: &Hunk| hunk.before.clone()
        } else {
            |hunk: &Hunk| hunk.after.clone()
        };

        loop {
            let (start_line, end_line) = self.line_ranges.peek()?;
            let hunk = self.hunks.get(self.cursor)?;

            if (hunk_range(hunk).end as usize) < *start_line {
                // If the hunk under the cursor comes before this range, jump the cursor
                // ahead to the next hunk that overlaps with the line range.
                self.cursor += self.hunks[self.cursor..]
                    .partition_point(|hunk| (hunk_range(hunk).end as usize) < *start_line);
            } else if (hunk_range(hunk).start as usize) <= *end_line {
                // If the hunk under the cursor overlaps with this line range, emit it
                // and move the cursor up so that the hunk cannot be emitted twice.
                self.cursor += 1;
                return Some(hunk);
            } else {
                // Otherwise, go to the next line range.
                self.line_ranges.next();
            }
        }
    }
}
