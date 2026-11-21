use crate::job;
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, register_hook};
use helix_lsp::lsp::DocumentSymbolResponse;
use helix_view::{
    events::{
        ConfigDidChange, DocumentDidChange, DocumentDidOpen, DocumentSymbolsUpdated,
        LanguageServerExited, LanguageServerInitialized, SelectionDidChange,
    },
    handlers::Handlers,
    DocumentId, Editor,
};

fn request_document_symbols(editor: &mut Editor, doc_id: DocumentId) {
    if !editor.config().breadcrumb.enable {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let Some(language_server) = doc
        // Get the first LSP Server that supports `DocumentSymbols`.
        .language_servers_with_feature(LanguageServerFeature::DocumentSymbols)
        .next()
    else {
        return;
    };

    let offset_encoding = language_server.offset_encoding();
    let Some(future) = language_server.document_symbols(doc.identifier()) else {
        return;
    };
    let cancel = doc.document_symbols_controller.restart();

    tokio::spawn(async move {
        let Some(Ok(Some(response))) = cancelable_future(future, &cancel).await else {
            return;
        };

        job::dispatch(move |editor, _| {
            if let Some(doc) = editor.document_mut(doc_id) {
                match response {
                    DocumentSymbolResponse::Nested(symbols) => {
                        doc.set_document_symbols(symbols, offset_encoding);
                        // WARN: Must call after setting new symbols, otherwise
                        // the symbols might not match to the real cursor position.
                        helix_event::dispatch(DocumentSymbolsUpdated {
                            editor,
                            doc: doc_id,
                        });
                    }
                    // TODO: Using the `Location`, it should be possible to map cursor
                    // to a hierarchical tree?
                    DocumentSymbolResponse::Flat(_) => {}
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
            // Cancel the ongoing request, if present.
            event.doc.document_symbols_controller.cancel();
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

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.clear_document_symbols();
            }
        }
        Ok(())
    });

    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        if !event.old.breadcrumb.enable && event.new.breadcrumb.enable {
            let view_id = event.editor.tree.focus;
            if let Some(view) = event.editor.tree.try_get(view_id) {
                let doc_id = view.doc;
                request_document_symbols(event.editor, doc_id);
            }
            return Ok(());
        }

        if event.old.breadcrumb.enable && !event.new.breadcrumb.enable {
            for doc in event.editor.documents_mut() {
                doc.clear_document_symbols();
            }
        }

        Ok(())
    });

    register_hook!(move |event: &mut DocumentSymbolsUpdated<'_>| {
        let doc_id = event.doc;
        let Editor {
            tree, documents, ..
        } = &mut *event.editor;
        if let Some(doc) = documents.get_mut(&doc_id) {
            // Update each open view of the document and update.
            for (view, _) in tree.views().filter(|(view, _)| view.doc == doc_id) {
                doc.update_breadcrumbs_for_view_inlined(view.id);
            }
        }
        Ok(())
    });

    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        event.doc.update_breadcrumbs_for_view_inlined(event.view);
        Ok(())
    });
}
