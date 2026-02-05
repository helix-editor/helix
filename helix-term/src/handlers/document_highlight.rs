use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, register_hook};
use helix_lsp::{lsp, util::lsp_range_to_range, OffsetEncoding};
use helix_view::{
    events::{
        ConfigDidChange, DocumentDidChange, DocumentDidOpen, LanguageServerExited,
        LanguageServerInitialized, SelectionDidChange,
    },
    handlers::Handlers,
    DocumentId, Editor, ViewId,
};

use crate::job;

fn request_document_highlights(editor: &mut Editor, doc_id: DocumentId, view_id: ViewId) {
    if !editor.config().lsp.auto_document_highlight {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    doc.ensure_view_init(view_id);

    let Some(language_server) = doc
        .language_servers_with_feature(LanguageServerFeature::DocumentHighlight)
        .next()
    else {
        doc.clear_document_highlights(view_id);
        return;
    };

    let offset_encoding = language_server.offset_encoding();
    let pos = doc.position(view_id, offset_encoding);
    let Some(future) =
        language_server.text_document_document_highlight(doc.identifier(), pos, None)
    else {
        doc.clear_document_highlights(view_id);
        return;
    };

    let text = doc.text().clone();
    let cancel = doc.document_highlight_controller(view_id).restart();

    tokio::spawn(async move {
        let response = match cancelable_future(future, &cancel).await {
            Some(Ok(response)) => response,
            Some(Err(err)) => {
                log::error!("document highlight request failed: {err}");
                return;
            }
            None => return,
        };

        let ranges = response
            .map(|highlights| document_highlight_ranges(&text, offset_encoding, highlights))
            .unwrap_or_default();

        job::dispatch(move |editor, _| {
            apply_document_highlights(editor, doc_id, view_id, ranges);
        })
        .await;
    });
}

fn document_highlight_ranges(
    text: &helix_core::Rope,
    offset_encoding: OffsetEncoding,
    highlights: Vec<lsp::DocumentHighlight>,
) -> Vec<std::ops::Range<usize>> {
    let slice = text.slice(..);
    let mut ranges: Vec<_> = highlights
        .into_iter()
        .filter_map(|highlight| lsp_range_to_range(text, highlight.range, offset_encoding))
        .map(|range| range.min_width_1(slice))
        .filter_map(|range| {
            let start = range.from();
            let end = range.to();
            (start < end).then_some(start..end)
        })
        .collect();

    ranges.sort_by(|a, b| (a.start, a.end).cmp(&(b.start, b.end)));

    let mut merged: Vec<std::ops::Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        if let Some(last) = merged.last_mut() {
            if range.start <= last.end {
                if range.end > last.end {
                    last.end = range.end;
                }
                continue;
            }
        }
        merged.push(range);
    }

    merged
}

fn apply_document_highlights(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    ranges: Vec<std::ops::Range<usize>>,
) {
    if !editor.config().lsp.auto_document_highlight {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    if !doc.has_language_server_with_feature(LanguageServerFeature::DocumentHighlight) {
        doc.clear_document_highlights(view_id);
        return;
    }

    if ranges.is_empty() {
        doc.clear_document_highlights(view_id);
        return;
    }

    doc.set_document_highlights(view_id, ranges);
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if event.doc.config.load().lsp.auto_document_highlight {
            let doc_id = event.doc.id();
            let view_id = event.view;
            job::dispatch_blocking(move |editor, _| {
                request_document_highlights(editor, doc_id, view_id);
            });
        }
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if !event.editor.config().lsp.auto_document_highlight {
            return Ok(());
        }
        let view_id = event.editor.tree.focus;
        if event.editor.tree.try_get(view_id).is_none() {
            return Ok(());
        }
        request_document_highlights(event.editor, event.doc, view_id);
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.doc.config.load().lsp.auto_document_highlight && !event.ghost_transaction {
            let doc_id = event.doc.id();
            let view_id = event.view;
            job::dispatch_blocking(move |editor, _| {
                request_document_highlights(editor, doc_id, view_id);
            });
        }
        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        if !event.editor.config().lsp.auto_document_highlight {
            return Ok(());
        }
        let view_id = event.editor.tree.focus;
        let Some(view) = event.editor.tree.try_get(view_id) else {
            return Ok(());
        };
        let doc_id = view.doc;
        request_document_highlights(event.editor, doc_id, view_id);
        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.clear_all_document_highlights();
            }
        }
        Ok(())
    });

    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        if event.new.lsp.auto_document_highlight {
            return Ok(());
        }
        for doc in event.editor.documents_mut() {
            doc.clear_all_document_highlights();
        }
        Ok(())
    });
}
