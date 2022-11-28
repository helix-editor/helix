use std::ops::Range;
use std::sync::Arc;

use helix_core::Rope;
use imara_diff::Algorithm;
use parking_lot::{Mutex, MutexGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

use crate::diff::worker::DiffWorker;

mod line_cache;
mod worker;

type RedrawHandle = Arc<(Notify, RwLock<()>)>;

// The order of enum variants is used by the PartialOrd
// derive macro, DO NOT REORDER
#[derive(PartialEq, PartialOrd)]
enum RenderStrategy {
    Async,
    SyncWithTimeout,
    Sync,
}

struct Event {
    text: Rope,
    is_base: bool,
    render_strategy: RenderStrategy,
}

#[derive(Clone, Debug)]
pub struct DiffHandle {
    channel: UnboundedSender<Event>,
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
            redraw_handle,
            difff_finished_notify: Arc::default(),
        };
        let handle = tokio::spawn(worker.run(diff_base, doc));
        let differ = DiffHandle {
            channel: sender,
            hunks,
            inverted: false,
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

    pub fn update_document(&self, doc: Rope, block: bool) -> bool {
        let mode = if block {
            RenderStrategy::Sync
        } else {
            RenderStrategy::SyncWithTimeout
        };
        self.update_document_impl(doc, self.inverted, mode)
    }

    pub fn update_diff_base(&self, diff_base: Rope) -> bool {
        self.update_document_impl(diff_base, !self.inverted, RenderStrategy::Async)
    }

    fn update_document_impl(&self, text: Rope, is_base: bool, mode: RenderStrategy) -> bool {
        let event = Event {
            text,
            is_base,
            render_strategy: mode,
        };
        self.channel.send(event).is_ok()
    }
}

// TODO configuration
/// synchronous debounce value should be low
/// so we can update synchronously most of the time
const DIFF_DEBOUNCE_TIME_SYNC: u64 = 1;
/// maximum time that rendering should be blocked until the diff finishes
const SYNC_DIFF_TIMEOUT: u64 = 50;
const DIFF_DEBOUNCE_TIME_ASYNC: u64 = 100;
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
    /// because lines larger than `i32::MAX` are not supported by imara-diff anways.
    /// Has some nice properties where it usually is not necessary to check for `None` seperatly:
    /// Empty ranges fail contains checks and also fails smaller than checks.
    pub const NONE: Hunk = Hunk {
        before: u32::MAX..u32::MAX,
        after: u32::MAX..u32::MAX,
    };

    /// Inverts a change so that `before` becomes `after` and `after` becomes `before`
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
