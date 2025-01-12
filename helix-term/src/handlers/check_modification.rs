use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Ok;

use helix_event::{register_hook, send_blocking};
use helix_view::{
    events::DocumentDidChange,
    handlers::{CheckModificationEvent, Handlers},
    DocumentId,
};
use tokio::time::Instant;

use crate::job;

/// CheckModificationHandler reacts to user changes in the editor by checking if there
/// has been an external change on the filesystem for the same file since the last save.
#[derive(Debug)]
pub(super) struct CheckModificationHandler {
    /// Documents that should be polled.
    docs: Arc<Mutex<HashSet<DocumentId>>>,
}

impl CheckModificationHandler {
    pub fn new() -> CheckModificationHandler {
        CheckModificationHandler {
            docs: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

// Strike a balance between spamming stat syscalls and informing the user relatively soon
// after they start making changes that there's a conflict.
const DEBOUNCE_INTERVAL: Duration = Duration::from_secs(2);

impl helix_event::AsyncHook for CheckModificationHandler {
    type Event = CheckModificationEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        existing_debounce: Option<tokio::time::Instant>,
    ) -> Option<Instant> {
        {
            let mu = self.docs.clone();
            let mut docs = mu.lock().unwrap();
            docs.insert(event.doc_id);
        }
        existing_debounce.or_else(|| Some(Instant::now() + DEBOUNCE_INTERVAL))
    }

    fn finish_debounce(&mut self) {
        let docs = {
            let mu = self.docs.clone();
            let mut docs = mu.lock().unwrap();
            docs.drain().collect::<Vec<DocumentId>>()
        };
        job::dispatch_blocking(move |editor, _| {
            for doc_id in docs {
                editor.check_external_modification(doc_id);
            }
        });
    }
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.check_modification.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        send_blocking(
            &tx,
            CheckModificationEvent {
                doc_id: event.doc.id(),
            },
        );
        Ok(())
    });
}
