use crate::job;
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, register_hook};
use helix_lsp::lsp::DocumentSymbolResponse;
use helix_view::{
    events::{DocumentDidChange, DocumentDidOpen, LanguageServerInitialized},
    handlers::Handlers,
    DocumentId, Editor,
};

fn request_document_symbols(editor: &mut Editor, doc_id: DocumentId) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let Some(language_server) = doc
        // Get the first LSP Server that supports `DocumentSymbols`.
        .language_servers_with_feature(LanguageServerFeature::DocumentSymbols)
        .next()
    else {
        doc.clear_document_symbols();
        return;
    };

    let offset_encoding = language_server.offset_encoding();
    let Some(future) = language_server.document_symbols(doc.identifier()) else {
        return;
    };
    let cancel = doc.symbols_controller.restart();

    tokio::spawn(async move {
        let Some(Ok(Some(response))) = cancelable_future(future, &cancel).await else {
            return;
        };

        job::dispatch(move |editor, _| {
            if let Some(doc) = editor.document_mut(doc_id) {
                match response {
                    DocumentSymbolResponse::Nested(symbols) => {
                        doc.set_document_symbols(symbols, offset_encoding);
                    }
                    // We cannot get structural information from flat responses.
                    DocumentSymbolResponse::Flat(_) => doc.clear_document_symbols(),
                }
            }
        })
        .await;
    });
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        request_document_symbols(event.editor, event.doc);
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if !event.ghost_transaction {
            let doc_id = event.doc.id();
            job::dispatch_blocking(move |editor, _| {
                request_document_symbols(editor, doc_id);
            });
        }
        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let view_id = event.editor.tree.focus;
        if let Some(view) = event.editor.tree.try_get(view_id) {
            request_document_symbols(event.editor, view.doc);
        }
        Ok(())
    });
}
