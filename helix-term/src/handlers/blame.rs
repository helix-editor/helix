use std::time::Duration;

use helix_event::register_hook;
use helix_vcs::FileBlame;
use helix_view::{
    editor::InlineBlameBehaviour,
    events::{DocumentDidOpen, EditorConfigDidChange},
    handlers::{BlameEvent, Handlers},
    DocumentId,
};
use tokio::{sync::oneshot, time::Instant};

use crate::job;

#[derive(Default)]
pub struct BlameHandler {
    worker: Option<oneshot::Receiver<anyhow::Result<FileBlame>>>,
    doc_id: DocumentId,
    show_blame_for_line_in_statusline: Option<u32>,
}

impl helix_event::AsyncHook for BlameHandler {
    type Event = BlameEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        if let Some(worker) = &mut self.worker {
            if worker.try_recv().is_ok() {
                self.finish_debounce();
                return None;
            }
        }

        self.doc_id = event.doc_id;
        self.show_blame_for_line_in_statusline = event.line;
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = FileBlame::try_new(event.path);
            let _ = tx.send(result);
        });

        self.worker = Some(rx);

        Some(Instant::now() + Duration::from_millis(50))
    }

    fn finish_debounce(&mut self) {
        let doc_id = self.doc_id;
        let line_blame = self.show_blame_for_line_in_statusline;
        if let Some(worker) = self.worker.take() {
            tokio::spawn(async move {
                let Ok(result) = worker.await else {
                    return;
                };

                job::dispatch(move |editor, _| {
                    let Some(doc) = editor.document_mut(doc_id) else {
                        return;
                    };
                    doc.file_blame = Some(result);
                    if editor.config().inline_blame.behaviour == InlineBlameBehaviour::Disabled {
                        if let Some(line) = line_blame {
                            crate::commands::blame_line_impl(editor, doc_id, line);
                        } else {
                            editor.set_status("Blame for this file is now available")
                        }
                    }
                })
                .await;
            });
        }
    }
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.blame.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if event.editor.config().inline_blame.behaviour != InlineBlameBehaviour::Disabled {
            helix_event::send_blocking(
                &tx,
                BlameEvent {
                    path: event.path.to_path_buf(),
                    doc_id: event.doc,
                    line: None,
                },
            );
        }
        Ok(())
    });
    let tx = handlers.blame.clone();
    register_hook!(move |event: &mut EditorConfigDidChange<'_>| {
        if event.old_config.inline_blame.behaviour == InlineBlameBehaviour::Disabled
            && event.new_config.inline_blame.behaviour != InlineBlameBehaviour::Disabled
        {
            // request blame for all documents, since any of them could have
            // outdated blame
            for doc in event.editor.documents() {
                if let Some(path) = doc.path() {
                    helix_event::send_blocking(
                        &tx,
                        BlameEvent {
                            path: path.to_path_buf(),
                            doc_id: doc.id(),
                            line: None,
                        },
                    );
                }
            }
        }
        Ok(())
    });
}
