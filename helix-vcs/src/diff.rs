use std::ops::Range;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use helix_core::Rope;
use imara_diff::Algorithm;
use parking_lot::{Mutex, MutexGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;

use crate::diff::worker::DiffWorker;

mod line_cache;
mod worker;

enum Event {
    UpdateDocument(Rope),
    UpdateDiffBase(Rope),
}

#[derive(Clone, Debug)]
pub struct DiffHandle {
    channel: UnboundedSender<Event>,
    hunks: Arc<Mutex<Vec<Hunk>>>,
    inverted: bool,
}

impl DiffHandle {
    pub fn new(diff_base: Rope, doc: Rope, redraw_handle: Arc<AtomicBool>) -> DiffHandle {
        DiffHandle::new_with_handle(diff_base, doc, redraw_handle).0
    }

    fn new_with_handle(
        diff_base: Rope,
        doc: Rope,
        notify: Arc<AtomicBool>,
    ) -> (DiffHandle, JoinHandle<()>) {
        let (sender, receiver) = unbounded_channel();
        let hunks: Arc<Mutex<Vec<Hunk>>> = Arc::default();
        let worker = DiffWorker {
            channel: receiver,
            hunks: hunks.clone(),
            new_hunks: Vec::default(),
            notify,
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

    pub fn update_document(&self, doc: Rope) -> bool {
        if self.inverted {
            self.update_diff_base_impl(doc)
        } else {
            self.update_document_impl(doc)
        }
    }

    pub fn update_diff_base(&self, diff_base: Rope) -> bool {
        if self.inverted {
            self.update_document_impl(diff_base)
        } else {
            self.update_diff_base_impl(diff_base)
        }
    }

    pub fn update_document_impl(&self, doc: Rope) -> bool {
        self.channel.send(Event::UpdateDocument(doc)).is_ok()
    }

    pub fn update_diff_base_impl(&self, diff_base: Rope) -> bool {
        self.channel.send(Event::UpdateDiffBase(diff_base)).is_ok()
    }
}

// TODO configuration
const DIFF_DEBOUNCE_TIME: u64 = 100;
const ALGORITHM: Algorithm = Algorithm::Histogram;
const MAX_DIFF_LINES: usize = u16::MAX as usize;
// cap average line length to 128 for files with MAX_DIFF_LINES
const MAX_DIFF_BYTES: usize = MAX_DIFF_LINES * 128;

/// A single change in a file potentially sppaning multiple lines
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
    /// because lines larger then `i32::MAX` are not supported by imara-diff anways.
    /// Has some nice properties where it usually is not necessary to check for `None` seperatly:
    /// Empty ranges fail contains checks and also faills smaller then checks.
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
