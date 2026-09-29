use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant as StdInstant},
};

use helix_event::{register_hook, send_blocking, AsyncHook};
use helix_view::{
    events::{DocumentDidChange, DocumentDidClose},
    DocumentId, Editor,
};
use tokio::{sync::mpsc, time::Instant};

use crate::job;

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(50);
const MIN_DEBOUNCE: Duration = Duration::from_millis(15);
const MAX_DEBOUNCE: Duration = Duration::from_millis(300);
const DEBOUNCE_PAD: Duration = Duration::from_millis(20);

#[derive(Default)]
struct SyntaxHandler {
    pending: HashSet<DocumentId>,
    last_parse: Arc<Mutex<HashMap<DocumentId, Duration>>>,
}

enum SyntaxEvent {
    Change(DocumentId),
    Close(DocumentId),
}

impl SyntaxHandler {
    fn debounce_for(&self, doc: DocumentId) -> Duration {
        let last = self
            .last_parse
            .lock()
            .ok()
            .and_then(|times| times.get(&doc).copied());
        parse_debounce(last)
    }
}

fn parse_debounce(last_parse: Option<Duration>) -> Duration {
    let Some(parse) = last_parse else {
        return DEFAULT_DEBOUNCE;
    };
    parse
        .saturating_add(DEBOUNCE_PAD)
        .clamp(MIN_DEBOUNCE, MAX_DEBOUNCE)
}

impl AsyncHook for SyntaxHandler {
    type Event = SyntaxEvent;

    fn handle_event(&mut self, event: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        match event {
            SyntaxEvent::Change(doc) => {
                self.pending.insert(doc);
                Some(Instant::now() + self.debounce_for(doc))
            }
            SyntaxEvent::Close(doc) => {
                self.pending.remove(&doc);
                if let Ok(mut times) = self.last_parse.lock() {
                    times.remove(&doc);
                }
                None
            }
        }
    }

    fn finish_debounce(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        let last_parse = Arc::clone(&self.last_parse);
        job::dispatch_blocking(move |editor, _| update_syntax(editor, pending, last_parse));
    }
}

fn update_syntax(
    editor: &mut Editor,
    pending: HashSet<DocumentId>,
    last_parse: Arc<Mutex<HashMap<DocumentId, Duration>>>,
) {
    let loader = editor.syn_loader.load();
    for doc_id in pending {
        let Some(doc) = editor.document_mut(doc_id) else {
            continue;
        };
        let diff = doc.syntax_pending_changes().clone();
        let Some(mut syntax) = doc.syntax.take() else {
            continue;
        };
        let start = StdInstant::now();
        let update_result = syntax.update(
            doc.syntax_text_snapshot().slice(..),
            doc.text().slice(..),
            &diff,
            &loader,
        );
        if let Err(err) = update_result {
            log::error!(
                target: "helix::syntax",
                "parse failed for {}: {err}",
                doc.display_name()
            );
        } else {
            doc.syntax = Some(syntax);
            let elapsed = start.elapsed();
            doc.commit_syntax_text_snapshot();
            if let Ok(mut times) = last_parse.lock() {
                times.insert(doc_id, elapsed);
            }
            log::debug!(
                target: "helix::syntax",
                "debounced parse ok for {} in {elapsed:?}, next debounce {:?}",
                doc.display_name(),
                parse_debounce(Some(elapsed))
            );
        }
    }
    helix_event::request_redraw();
}

pub fn spawn() {
    let handler = SyntaxHandler::default();
    let tx = handler.spawn();
    register_hooks(&tx);
}

fn register_hooks(tx: &mpsc::Sender<SyntaxEvent>) {
    let change_tx = tx.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.ghost_transaction || event.doc.syntax().is_none() {
            return Ok(());
        }
        send_blocking(&change_tx, SyntaxEvent::Change(event.doc.id()));
        Ok(())
    });

    let close_tx = tx.clone();
    register_hook!(move |event: &mut DocumentDidClose<'_>| {
        send_blocking(&close_tx, SyntaxEvent::Close(event.doc.id()));
        Ok(())
    });
}
