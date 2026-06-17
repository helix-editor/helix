//! Spell checking as a non-LSP diagnostic source.
//!
//! This is the editor-side state for the spell checker. The detection logic (the debounced hook,
//! dictionary loading and the word checking itself) lives in `helix-term`'s spelling handler, which
//! drives this state through [`SpellingEvent`]s and the editor's dictionaries.

use std::collections::{HashMap, HashSet};

use helix_core::{ChangeSet, Rope, SpellingLanguage};
use helix_event::{TaskController, TaskHandle};
use tokio::sync::mpsc::Sender;

use crate::DocumentId;

#[derive(Debug)]
pub struct SpellingHandler {
    pub event_tx: Sender<SpellingEvent>,
    /// In-flight full-document checks, keyed by document. Starting a new full check for a document
    /// cancels the previous one (incremental checks run synchronously and need no cancellation).
    pub requests: HashMap<DocumentId, TaskController>,
    /// Languages whose dictionary is currently being loaded, so the same one isn't loaded twice
    /// concurrently.
    pub loading_dictionaries: HashSet<SpellingLanguage>,
}

impl SpellingHandler {
    pub fn new(event_tx: Sender<SpellingEvent>) -> Self {
        Self {
            event_tx,
            requests: HashMap::new(),
            loading_dictionaries: HashSet::new(),
        }
    }

    /// Registers a new in-flight full check for `document`, cancelling any previous one, and
    /// returns a handle the background task uses to observe cancellation.
    pub fn open_request(&mut self, document: DocumentId) -> TaskHandle {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        self.requests.insert(document, controller);
        handle
    }
}

#[derive(Debug)]
pub enum SpellingEvent {
    /// A dictionary finished loading; (re-)check the open documents that use it.
    DictionaryLoaded { language: SpellingLanguage },
    /// A document was opened; check it in full.
    DocumentOpened { doc: DocumentId },
    /// A document changed; re-check the regions around the change (or rescan, see the term-side
    /// handler). Carries the snapshot needed to recompute the affected regions off the main loop.
    DocumentChanged {
        doc: DocumentId,
        old_text: Rope,
        text: Rope,
        changes: ChangeSet,
        version: i32,
    },
}
