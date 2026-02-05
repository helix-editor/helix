use std::{collections::HashSet, time::Duration};

use futures_util::{stream::FuturesOrdered, StreamExt};
use helix_core::{syntax::config::LanguageServerFeature, Assoc};
use helix_event::{cancelable_future, register_hook};
use helix_view::{
    document::DocumentLink,
    events::{DocumentDidChange, DocumentDidOpen, LanguageServerExited, LanguageServerInitialized},
    handlers::{lsp::DocumentLinksEvent, Handlers},
    DocumentId, Editor,
};
use tokio::time::Instant;

use crate::job;

#[derive(Default)]
pub(super) struct DocumentLinksHandler {
    docs: HashSet<DocumentId>,
}

const DOCUMENT_CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

impl helix_event::AsyncHook for DocumentLinksHandler {
    type Event = DocumentLinksEvent;

    fn handle_event(&mut self, event: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        let DocumentLinksEvent(doc_id) = event;
        self.docs.insert(doc_id);
        Some(Instant::now() + DOCUMENT_CHANGE_DEBOUNCE)
    }

    fn finish_debounce(&mut self) {
        let docs = std::mem::take(&mut self.docs);

        job::dispatch_blocking(move |editor, _compositor| {
            for doc in docs {
                request_document_links(editor, doc);
            }
        });
    }
}

/// Request document links for a specific document and cache them for navigation.
fn request_document_links(editor: &mut Editor, doc_id: DocumentId) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let cancel = doc.document_link_controller.restart();

    let mut seen_language_servers = HashSet::new();
    let mut futures: FuturesOrdered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::DocumentLinks)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .filter_map(|language_server| {
            let text = doc.text().clone();
            let offset_encoding = language_server.offset_encoding();
            let language_server_id = language_server.id();
            let future = language_server.text_document_document_link(doc.identifier(), None)?;

            Some(async move {
                let links = future.await?.unwrap_or_default();
                let links: Vec<_> = links
                    .into_iter()
                    .filter_map(|link| {
                        let start = helix_lsp::util::lsp_pos_to_pos(
                            &text,
                            link.range.start,
                            offset_encoding,
                        )?;
                        let end = helix_lsp::util::lsp_pos_to_pos(
                            &text,
                            link.range.end,
                            offset_encoding,
                        )?;
                        if start > end {
                            return None;
                        }
                        Some(DocumentLink {
                            start,
                            end,
                            link,
                            language_server_id,
                        })
                    })
                    .collect();
                anyhow::Ok(links)
            })
        })
        .collect();

    if futures.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let mut all_links = Vec::new();
        loop {
            match cancelable_future(futures.next(), &cancel).await {
                Some(Some(Ok(items))) => all_links.extend(items),
                Some(Some(Err(err))) => log::error!("document link request failed: {err}"),
                Some(None) => break,
                None => return,
            }
        }

        job::dispatch(move |editor, _| attach_document_links(editor, doc_id, all_links)).await;
    });
}

fn attach_document_links(editor: &mut Editor, doc_id: DocumentId, mut links: Vec<DocumentLink>) {
    let Some(doc) = editor.documents.get_mut(&doc_id) else {
        return;
    };

    if links.is_empty() {
        doc.document_links.clear();
        return;
    }

    links.sort_by_key(|link| (link.start, link.end));
    doc.document_links = links;
}

pub(super) fn register_hooks(handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        request_document_links(event.editor, event.doc);
        Ok(())
    });

    let tx = handlers.document_links.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        event
            .changes
            .update_positions(event.doc.document_links.iter_mut().flat_map(|link| {
                std::iter::once((&mut link.start, Assoc::After))
                    .chain(std::iter::once((&mut link.end, Assoc::After)))
            }));

        if !event.ghost_transaction {
            event.doc.document_link_controller.cancel();
            helix_event::send_blocking(&tx, DocumentLinksEvent(event.doc.id()));
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let doc_ids: Vec<_> = event.editor.documents().map(|doc| doc.id()).collect();

        for doc_id in doc_ids {
            request_document_links(event.editor, doc_id);
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.document_links.clear();
            }
        }

        let doc_ids: Vec<_> = event.editor.documents().map(|doc| doc.id()).collect();

        for doc_id in doc_ids {
            request_document_links(event.editor, doc_id);
        }

        Ok(())
    });
}
