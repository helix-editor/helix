use std::cell::Cell;
use std::num::NonZeroUsize;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;
use std::time::Duration;

use helix_event::{request_redraw, send_blocking, AsyncHook};
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;

use crate::{Document, DocumentId, ViewId};

#[derive(Debug)]
pub enum DiagnosticEvent {
    CursorLineChanged { generation: usize },
    Refresh,
}

struct DiagnosticTimeout {
    active_generation: Arc<AtomicUsize>,
    generation: usize,
}

const TIMEOUT: Duration = Duration::from_millis(350);

impl AsyncHook for DiagnosticTimeout {
    type Event = DiagnosticEvent;

    fn handle_event(
        &mut self,
        event: DiagnosticEvent,
        timeout: Option<Instant>,
    ) -> Option<Instant> {
        match event {
            DiagnosticEvent::CursorLineChanged { generation } => {
                if generation > self.generation {
                    self.generation = generation;
                    Some(Instant::now() + TIMEOUT)
                } else {
                    timeout
                }
            }
            DiagnosticEvent::Refresh if timeout.is_some() => Some(Instant::now() + TIMEOUT),
            DiagnosticEvent::Refresh => None,
        }
    }

    fn finish_debounce(&mut self) {
        if self.active_generation.load(atomic::Ordering::Relaxed) < self.generation {
            self.active_generation
                .store(self.generation, atomic::Ordering::Relaxed);
            request_redraw();
        }
    }
}

pub struct DiagnosticsHandler {
    active_generation: Arc<AtomicUsize>,
    generation: Cell<usize>,
    last_doc: Cell<DocumentId>,
    last_cursor_line: Cell<usize>,
    pub active: bool,
    pub events: Sender<DiagnosticEvent>,
}

// make sure we never share handlers across multiple views this is a stop
// gap solution. We just shouldn't be cloneing a view to begin with (we do
// for :hsplit/vsplit) and really this should not be view specific to begin with
// but to fix that larger architecutre changes are needed
impl Clone for DiagnosticsHandler {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl DiagnosticsHandler {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let active_generation = Arc::new(AtomicUsize::new(0));
        let events = DiagnosticTimeout {
            active_generation: active_generation.clone(),
            generation: 0,
        }
        .spawn();
        Self {
            active_generation,
            generation: Cell::new(0),
            events,
            last_doc: Cell::new(DocumentId(NonZeroUsize::new(usize::MAX).unwrap())),
            last_cursor_line: Cell::new(usize::MAX),
            active: true,
        }
    }
}

impl DiagnosticsHandler {
    pub fn immediately_show_diagnostic(&self, doc: &Document, view: ViewId) {
        self.last_doc.set(doc.id());
        let cursor_line = doc
            .selection(view)
            .primary()
            .cursor_line(doc.text().slice(..));
        self.last_cursor_line.set(cursor_line);
        self.active_generation
            .store(self.generation.get(), atomic::Ordering::Relaxed);
    }
    pub fn show_cursorline_diagnostics(&self, doc: &Document, view: ViewId) -> bool {
        if !self.active || !doc.config.load().enable_diagnostics {
            return false;
        }
        let cursor_line = doc
            .selection(view)
            .primary()
            .cursor_line(doc.text().slice(..));
        if self.last_cursor_line.get() == cursor_line && self.last_doc.get() == doc.id() {
            let active_generation = self.active_generation.load(atomic::Ordering::Relaxed);
            self.generation.get() == active_generation
        } else {
            self.last_doc.set(doc.id());
            self.last_cursor_line.set(cursor_line);
            self.generation.set(self.generation.get() + 1);
            send_blocking(
                &self.events,
                DiagnosticEvent::CursorLineChanged {
                    generation: self.generation.get(),
                },
            );
            false
        }
    }
}
