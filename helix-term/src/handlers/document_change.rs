use helix_event::AsyncHook;
use helix_view::handlers::lsp::DocumentChangeEvent;
use tokio::time::Instant;

use crate::job;

#[derive(Default)]
pub(super) struct DocumentChangeHandler;

impl AsyncHook for DocumentChangeHandler {
    type Event = DocumentChangeEvent;

    fn handle_event(&mut self, event: Self::Event, timeout: Option<Instant>) -> Option<Instant> {
        job::dispatch_blocking(move |editor, _| {
            for language_server_id in event.language_servers {
                let Some(language_server) = editor.language_server_by_id(language_server_id) else {
                    continue;
                };

                language_server.text_document_did_change(
                    event.text_document.clone(),
                    &event.old_text,
                    &event.text,
                    &event.changes,
                );
            }
        });

        timeout
    }

    fn finish_debounce(&mut self) {}
}
