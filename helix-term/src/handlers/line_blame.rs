use std::{collections::HashSet, path::PathBuf, time::Duration};

use helix_event::{register_hook, send_blocking, AsyncHook};
use helix_view::{
    events::{ConfigDidChange, DocumentDidChange, DocumentDidOpen, SelectionDidChange},
    handlers::Handlers,
    DocumentId, Editor, ViewId,
};
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;

use crate::job;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct LineBlameEvent {
    doc_id: DocumentId,
    view_id: ViewId,
}

#[derive(Default)]
pub(super) struct LineBlameHandler {
    requests: HashSet<LineBlameEvent>,
}

const LINE_BLAME_DEBOUNCE: Duration = Duration::from_millis(150);

fn line_is_blank(text: helix_core::RopeSlice, line: usize) -> bool {
    line >= text.len_lines() || text.line(line).chars().all(char::is_whitespace)
}

impl AsyncHook for LineBlameHandler {
    type Event = LineBlameEvent;

    fn handle_event(&mut self, event: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        self.requests.insert(event);
        Some(Instant::now() + LINE_BLAME_DEBOUNCE)
    }

    fn finish_debounce(&mut self) {
        let requests = std::mem::take(&mut self.requests);
        job::dispatch_blocking(move |editor, _| {
            for request in requests {
                request_line_blame(editor, request.doc_id, request.view_id);
            }
        });
    }
}

fn request_line_blame(editor: &mut Editor, doc_id: DocumentId, view_id: ViewId) {
    let diff_providers = editor.diff_providers.clone();
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    if !doc.config.load().inline_blame {
        doc.clear_line_blame(view_id);
        return;
    }

    let Some(path) = doc.path().map(ToOwned::to_owned) else {
        doc.clear_line_blame(view_id);
        return;
    };

    let Some(selection) = doc.selections().get(&view_id) else {
        return;
    };

    let text = doc.text().slice(..);
    let line = selection.primary().cursor_line(text);
    if line_is_blank(text, line) {
        doc.clear_line_blame(view_id);
        return;
    }

    let contents = doc.is_modified().then(|| text.to_string());
    let version = doc.version();

    tokio::spawn(async move {
        let blame_path = path.clone();
        let blame = tokio::task::spawn_blocking(move || {
            diff_providers.get_line_blame(&blame_path, contents.as_deref(), line)
        })
        .await
        .ok()
        .flatten();

        job::dispatch(move |editor, _| {
            apply_line_blame(editor, doc_id, view_id, path, version, line, blame);
        })
        .await;
    });
}

fn apply_line_blame(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    path: PathBuf,
    version: i32,
    line: usize,
    blame: Option<helix_vcs::BlameLine>,
) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    if !doc.config.load().inline_blame {
        doc.clear_line_blame(view_id);
        return;
    }

    if doc.version() != version || doc.path() != Some(path.as_path()) {
        return;
    }

    let Some(selection) = doc.selections().get(&view_id) else {
        return;
    };

    if selection.primary().cursor_line(doc.text().slice(..)) != line {
        return;
    }
    if line_is_blank(doc.text().slice(..), line) {
        doc.clear_line_blame(view_id);
        return;
    }

    match blame {
        Some(blame) => doc.set_line_blame(
            view_id,
            line,
            format!("  {}, {}", blame.author(), blame.timestamp()),
        ),
        None => doc.clear_line_blame(view_id),
    }
}

fn queue_line_blame(tx: &Sender<LineBlameEvent>, doc_id: DocumentId, view_id: ViewId) {
    send_blocking(tx, LineBlameEvent { doc_id, view_id });
}

pub(super) fn register_hooks(_handlers: &Handlers, tx: Sender<LineBlameEvent>) {
    let line_blame = tx.clone();
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if event.doc.config.load().inline_blame {
            let line = event
                .doc
                .selection(event.view)
                .primary()
                .cursor_line(event.doc.text().slice(..));
            if line_is_blank(event.doc.text().slice(..), line) {
                event.doc.clear_line_blame(event.view);
                return Ok(());
            }
            if !event.doc.has_line_blame_for_line(event.view, line) {
                event.doc.clear_line_blame(event.view);
                queue_line_blame(&line_blame, event.doc.id(), event.view);
            }
        }
        Ok(())
    });

    let line_blame = tx.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.doc.config.load().inline_blame && !event.ghost_transaction {
            event.doc.clear_line_blame(event.view);
            let line = event
                .doc
                .selection(event.view)
                .primary()
                .cursor_line(event.doc.text().slice(..));
            if !line_is_blank(event.doc.text().slice(..), line) {
                queue_line_blame(&line_blame, event.doc.id(), event.view);
            }
        }
        Ok(())
    });

    let line_blame = tx.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if !event.editor.config().inline_blame {
            return Ok(());
        }

        let view_id = event.editor.tree.focus;
        if event.editor.tree.try_get(view_id).is_some() {
            queue_line_blame(&line_blame, event.doc, view_id);
        }
        Ok(())
    });

    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        if !event.old.inline_blame && event.new.inline_blame {
            let view_id = event.editor.tree.focus;
            let Some(view) = event.editor.tree.try_get(view_id) else {
                return Ok(());
            };
            queue_line_blame(&tx, view.doc, view_id);
            return Ok(());
        }

        if event.old.inline_blame && !event.new.inline_blame {
            for doc in event.editor.documents_mut() {
                doc.clear_all_line_blames();
            }
        }

        Ok(())
    });
}
