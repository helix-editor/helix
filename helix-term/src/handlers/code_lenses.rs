use std::{collections::HashSet, time::Duration};

use futures_util::{stream::FuturesOrdered, StreamExt};
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, register_hook};
use helix_view::{
    events::{DocumentDidChange, DocumentDidOpen, LanguageServerExited, LanguageServerInitialized},
    handlers::{lsp::CodeLensesEvent, Handlers},
    DocumentId, Editor,
};
use tokio::time::Instant;

use crate::job;

// TODO(matoous): use to update code lenses positions on document changes
#[derive(Default)]
#[allow(dead_code)]
pub(super) struct DocumentCodeLensesHandler {
    docs: HashSet<DocumentId>,
}

// TODO(matoous): use to update debounce document changes when udpating positions of code lenses
// TODO: share with color swatches and possibly other annotations
#[allow(dead_code)]
const DOCUMENT_CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

impl helix_event::AsyncHook for DocumentCodeLensesHandler {
    type Event = CodeLensesEvent;

    fn handle_event(&mut self, event: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        let CodeLensesEvent(doc_id) = event;
        self.docs.insert(doc_id);
        Some(Instant::now() + DOCUMENT_CHANGE_DEBOUNCE)
    }

    fn finish_debounce(&mut self) {
        let docs = std::mem::take(&mut self.docs);

        job::dispatch_blocking(move |editor, _compositor| {
            for doc in docs {
                request_document_code_lenses(editor, doc);
            }
        });
    }
}

pub fn request_document_code_lenses(editor: &mut Editor, doc_id: DocumentId) {
    if !editor.config().lsp.display_code_lenses {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let cancel = doc.color_swatch_controller.restart();

    let mut seen_language_servers = HashSet::new();
    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::CodeLens)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .map(|language_server| {
            let text = doc.text().clone();
            let offset_encoding = language_server.offset_encoding();
            let future = language_server.code_lens(doc.identifier()).unwrap();

            async move {
                let lenses: Vec<_> = future
                    .await?
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|lens| {
                        let _pos = helix_lsp::util::lsp_pos_to_pos(
                            &text,
                            lens.range.start,
                            offset_encoding,
                        )?;
                        Some(lens)
                    })
                    .collect();
                anyhow::Ok(lenses)
            }
        })
        .collect();

    if futures.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let mut code_lenses = Vec::new();
        loop {
            match cancelable_future(futures.next(), &cancel).await {
                Some(Some(Ok(items))) => code_lenses.extend(items),
                Some(Some(Err(err))) => log::error!("document color request failed: {err}"),
                Some(None) => break,
                // The request was cancelled.
                None => return,
            }
        }
        job::dispatch(move |editor, _| {
            let Some(doc) = editor.documents.get_mut(&doc_id) else {
                return;
            };

            if code_lenses.is_empty() {
                doc.code_lenses.clear();
                return;
            }

            doc.code_lenses = code_lenses;
        })
        .await;
    });
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        // when a document is initially opened, request colors for it
        request_document_code_lenses(event.editor, event.doc);

        Ok(())
    });

    register_hook!(move |_event: &mut DocumentDidChange<'_>| {
        // TODO: update code lenses positions, same as with e.g. document colors
        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let doc_ids: Vec<_> = event.editor.documents().map(|doc| doc.id()).collect();

        for doc_id in doc_ids {
            request_document_code_lenses(event.editor, doc_id);
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        // Clear and re-request all color swatches when a server exits.
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.code_lenses.clear();
            }
        }

        let doc_ids: Vec<_> = event.editor.documents().map(|doc| doc.id()).collect();

        for doc_id in doc_ids {
            request_document_code_lenses(event.editor, doc_id);
        }

        Ok(())
    });
}
