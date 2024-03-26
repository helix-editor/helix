use std::ops::Range;
use std::sync::Arc;

use helix_core::Rope;
use helix_event::RenderLockGuard;
use imara_diff::Algorithm;
use parking_lot::{Mutex, MutexGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::diff::worker::DiffWorker;

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

#[derive(Clone, Debug)]
pub struct DiffHandle {
    channel: UnboundedSender<Event>,
    diff: Arc<Mutex<DiffInner>>,
    inverted: bool,
}

impl DiffHandle {
    pub fn new(diff_base: Rope, doc: Rope) -> DiffHandle {
        DiffHandle::new_with_handle(diff_base, doc).0
    }

    fn new_with_handle(diff_base: Rope, doc: Rope) -> (DiffHandle, JoinHandle<()>) {
        let (sender, receiver) = unbounded_channel();
        let diff: Arc<Mutex<DiffInner>> = Arc::default();
        let worker = DiffWorker {
            channel: receiver,
            diff: diff.clone(),
            new_hunks: Vec::default(),
            diff_finished_notify: Arc::default(),
        };
        let handle = tokio::spawn(worker.run(diff_base, doc));
        let differ = DiffHandle {
            channel: sender,
            diff,
            inverted: false,
        };
        (differ, handle)
    }

    pub fn invert(&mut self) {
        self.inverted = !self.inverted;
    }

    pub fn load(&self) -> Diff {
        Diff {
            diff: self.diff.lock(),
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

/// A single change in a file potentially spanning multiple lines
/// Hunks produced by the differs are always ordered by their position
/// in the file and non-overlapping.
/// Specifically for any two hunks `x` and `y` the following properties hold:
///
/// ``` no_compile
/// assert!(x.before.end <= y.before.start);
/// assert!(x.after.end <= y.after.start);
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Hunk {
    pub before: Range<u32>,
    pub after: Range<u32>,
}

impl Hunk {
    /// Can be used instead of `Option::None` for better performance
    /// because lines larger then `i32::MAX` are not supported by `imara-diff` anyways.
    /// Has some nice properties where it usually is not necessary to check for `None` separately:
    /// Empty ranges fail contains checks and also fails smaller then checks.
    pub const NONE: Hunk = Hunk {
        before: u32::MAX..u32::MAX,
        after: u32::MAX..u32::MAX,
    };

    /// Inverts a change so that `before`
    pub fn invert(&self) -> Hunk {
        Hunk {
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }

    pub fn is_pure_insertion(&self) -> bool {
        self.before.is_empty()
    }

    pub fn is_pure_removal(&self) -> bool {
        self.after.is_empty()
    }
}

/// A list of changes in a file sorted in ascending
/// non-overlapping order
#[derive(Debug)]
pub struct Diff<'a> {
    diff: MutexGuard<'a, DiffInner>,
    inverted: bool,
}

impl Diff<'_> {
    pub fn diff_base(&self) -> &Rope {
        if self.inverted {
            &self.diff.doc
        } else {
            &self.diff.diff_base
        }
    }

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
