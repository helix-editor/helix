use std::{collections::HashSet, time::Duration};

use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, register_hook};
use helix_view::{
    events::{DocumentDidChange, DocumentDidOpen, LanguageServerExited, LanguageServerInitialized},
    handlers::{lsp::CodeLensesEvent, Handlers},
    DocumentId, Editor,
};
use tokio::time::Instant;

use crate::job;

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

/// Requests code lenses for a specific document.
///
/// NOTE: currently supports only a single language server, the first one supporting
/// code lenses is used.
pub fn request_document_code_lenses(editor: &mut Editor, doc_id: DocumentId) {
    if !editor.config().lsp.display_code_lenses {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let cancel = doc.code_lenses_controller.restart();

    let language_server = doc
        .language_servers_with_feature(LanguageServerFeature::CodeLens)
        .next();
    let language_server = match language_server {
        Some(language_server) => language_server,
        None => {
            return;
        }
    };

    let text = doc.text().clone();
    let offset_encoding = language_server.offset_encoding();
    let future = language_server.code_lens(doc.identifier()).unwrap();

    tokio::spawn(async move {
        let lenses: Vec<_> = match cancelable_future(future, &cancel).await {
            Some(Ok(Some(items))) => items
                .into_iter()
                .filter_map(|lens| {
                    let _pos =
                        helix_lsp::util::lsp_pos_to_pos(&text, lens.range.start, offset_encoding)?;
                    Some(lens)
                })
                .collect(),
            Some(Ok(None)) => Vec::new(),
            Some(Err(err)) => {
                log::error!("code lenses request failed: {err}");
                return;
            }
            None => return,
        };

        job::dispatch(move |editor, _| {
            let Some(doc) = editor.documents.get_mut(&doc_id) else {
                return;
            };

            if lenses.is_empty() {
                doc.code_lenses.clear();
                return;
            }

            doc.set_code_lenses(lenses);
        })
        .await;
    });
}

pub(super) fn register_hooks(handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        // when a document is initially opened, request code lenses for it
        request_document_code_lenses(event.editor, event.doc);
        Ok(())
    });

    let tx = handlers.code_lenses.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        event.changes.update_positions(
            event
                .doc
                .code_lenses
                .iter_mut()
                .map(|lens| (&mut lens.char_idx, helix_core::Assoc::After)),
        );

        // Avoid re-requesting code lenses if the change is a ghost transaction (completion)
        // because the language server will not know about the updates to the document and will
        // give out-of-date locations.
        if !event.ghost_transaction {
            // Cancel the ongoing request, if present.
            event.doc.code_lenses_controller.cancel();
            helix_event::send_blocking(&tx, CodeLensesEvent(event.doc.id()));
        }

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
        // Clear and re-request all code lenses when a server exits.
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
