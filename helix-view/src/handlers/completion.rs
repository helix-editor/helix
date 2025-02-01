use std::{collections::HashMap, sync::Arc};

use helix_core::completion::CompletionProvider;
use helix_event::{send_blocking, TaskController};

use crate::{document::SavePoint, DocumentId, ViewId};

use tokio::sync::mpsc::Sender;

pub struct CompletionHandler {
    event_tx: Sender<CompletionEvent>,
    pub active_completions: HashMap<CompletionProvider, ResponseContext>,
    pub request_controller: TaskController,
}

impl CompletionHandler {
    pub fn new(event_tx: Sender<CompletionEvent>) -> Self {
        Self {
            event_tx,
            active_completions: HashMap::new(),
            request_controller: TaskController::new(),
        }
    }

    pub fn event(&self, event: CompletionEvent) {
        send_blocking(&self.event_tx, event);
    }
}

pub struct ResponseContext {
    /// Whether the completion response is marked as "incomplete."
    ///
    /// This is used by LSP. When completions are "incomplete" and you continue typing, the
    /// completions should be recomputed by the server instead of filtered.
    pub is_incomplete: bool,
    pub priority: i8,
    pub savepoint: Arc<SavePoint>,
}

pub enum CompletionEvent {
    /// Auto completion was triggered by typing a word char
    AutoTrigger {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// Auto completion was triggered by typing a trigger char
    /// specified by the LSP
    TriggerChar {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// A completion was manually requested (c-x)
    ManualTrigger {
        cursor: usize,
        doc: DocumentId,
        view: ViewId,
    },
    /// Some text was deleted and the cursor is now at `pos`
    DeleteText { cursor: usize },
    /// Invalidate the current auto completion trigger
    Cancel,
}
