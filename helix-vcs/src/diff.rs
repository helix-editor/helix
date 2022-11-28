use std::ops::Range;
use std::sync::Arc;

use helix_core::Rope;
use imara_diff::Algorithm;
use parking_lot::{Mutex, MutexGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{Notify, OwnedRwLockReadGuard, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::diff::worker::DiffWorker;

mod line_cache;
mod worker;

type RedrawHandle = (Arc<Notify>, Arc<RwLock<()>>);

/// A rendering lock passed to the differ the prevents redraws from occurring
struct RenderLock {
    pub lock: OwnedRwLockReadGuard<()>,
    pub timeout: Option<Instant>,
}

struct Event {
    text: Rope,
    is_base: bool,
    render_lock: Option<RenderLock>,
}

#[derive(Clone, Debug)]
pub struct DiffHandle {
    channel: UnboundedSender<Event>,
    render_lock: Arc<RwLock<()>>,
    hunks: Arc<Mutex<Vec<Hunk>>>,
    inverted: bool,
}

impl DiffHandle {
    pub fn new(diff_base: Rope, doc: Rope, redraw_handle: RedrawHandle) -> DiffHandle {
        DiffHandle::new_with_handle(diff_base, doc, redraw_handle).0
    }

    fn new_with_handle(
        diff_base: Rope,
        doc: Rope,
        redraw_handle: RedrawHandle,
    ) -> (DiffHandle, JoinHandle<()>) {
        let (sender, receiver) = unbounded_channel();
        let hunks: Arc<Mutex<Vec<Hunk>>> = Arc::default();
        let worker = DiffWorker {
            channel: receiver,
            hunks: hunks.clone(),
            new_hunks: Vec::default(),
            redraw_notify: redraw_handle.0,
            diff_finished_notify: Arc::default(),
        };
        let handle = tokio::spawn(worker.run(diff_base, doc));
        let differ = DiffHandle {
            channel: sender,
            hunks,
            inverted: false,
            render_lock: redraw_handle.1,
        };
        (differ, handle)
    }

    pub fn invert(&mut self) {
        self.inverted = !self.inverted;
    }

    pub fn hunks(&self) -> FileHunks {
        FileHunks {
            hunks: self.hunks.lock(),
            inverted: self.inverted,
        }
    }

    /// Updates the document associated with this redraw handle
    /// This function is only intended to be called from within the rendering loop
    /// if called from elsewhere it may fail to acquire the render lock and panic
    pub fn update_document(&self, doc: Rope, block: bool) -> bool {
        // unwrap is ok here because the rendering lock is
        // only exclusively locked during redraw.
        // This function is only intended to be called
        // from the core rendering loop where no redraw can happen in parallel
        let lock = self.render_lock.clone().try_read_owned().unwrap();
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
pub struct FileHunks<'a> {
    hunks: MutexGuard<'a, Vec<Hunk>>,
    inverted: bool,
}

impl FileHunks<'_> {
    pub fn is_inverted(&self) -> bool {
        self.inverted
    }

    /// Returns the `Hunk` for the `n`th change in this file.
    /// if there is no `n`th change  `Hunk::NONE` is returned instead.
    pub fn nth_hunk(&self, n: u32) -> Hunk {
        match self.hunks.get(n as usize) {
            Some(hunk) if self.inverted => hunk.invert(),
            Some(hunk) => hunk.clone(),
            None => Hunk::NONE,
        }
    }

    pub fn len(&self) -> u32 {
        self.hunks.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
