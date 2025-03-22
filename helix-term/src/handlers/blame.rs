use std::time::Duration;

use helix_event::register_hook;
use helix_vcs::FileBlame;
use helix_view::{
    events::DocumentDidOpen,
    handlers::{BlameEvent, Handlers},
    DocumentId,
};
use tokio::{sync::oneshot, time::Instant};

use crate::job;

#[derive(Default)]
pub struct BlameHandler {
    worker: Option<oneshot::Receiver<anyhow::Result<FileBlame>>>,
    doc_id: DocumentId,
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
        let (completion_tx, completion_rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = FileBlame::try_new(event.path);
            let _ = completion_tx.send(result);
        });

        self.worker = Some(completion_rx);

        Some(Instant::now() + Duration::from_millis(50))
    }

    fn finish_debounce(&mut self) {
        let doc_id = self.doc_id;
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
                })
                .await;
            });
        }
    }
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.blame.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        helix_event::send_blocking(
            &tx,
            BlameEvent {
                path: event.path.to_path_buf(),
                doc_id: event.doc,
            },
        );
        Ok(())
    });
}
