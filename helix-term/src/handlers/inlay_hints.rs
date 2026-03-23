use helix_core::{
    syntax::config::LanguageServerFeature, text_annotations::InlineAnnotation, Range,
};
use helix_event::{cancelable_future, register_hook};
use helix_lsp::{lsp, OffsetEncoding};
use helix_view::{
    document::{DocumentInlayHints, DocumentInlayHintsId},
    events::{
        ConfigDidChange, DocumentDidChange, DocumentDidOpen, LanguageServerExited,
        LanguageServerInitialized, SelectionDidChange,
    },
    handlers::{lsp::InlayHintsEvent, Handlers},
    Document, DocumentId, Editor, View, ViewId,
};

use crate::job;

#[derive(Debug, Default)]
pub(super) struct InlayHintsHandler;

impl helix_event::AsyncHook for InlayHintsHandler {
    type Event = InlayHintsEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        job::dispatch_blocking(move |editor, _| match event {
            InlayHintsEvent::RefreshVisibleViews => refresh_visible_inlay_hints(editor),
            InlayHintsEvent::RefreshDocument { document_id } => {
                request_all_document_inlay_hints(editor, document_id)
            }
            InlayHintsEvent::RefreshView {
                document_id,
                view_id,
            } => request_inlay_hints(editor, document_id, view_id),
        });

        None
    }

    fn finish_debounce(&mut self) {}
}

struct InlayHintsRequest {
    id: DocumentInlayHintsId,
    range: lsp::Range,
    offset_encoding: OffsetEncoding,
}

fn visible_inlay_hints_id(view: &View, doc: &Document) -> DocumentInlayHintsId {
    let doc_text = doc.text();
    let len_lines = doc_text.len_lines();
    let view_height = view.inner_height();
    let first_visible_line =
        doc_text.char_to_line(doc.view_offset(view.id).anchor.min(doc_text.len_chars()));
    let first_line = first_visible_line.saturating_sub(view_height);
    let last_line = first_visible_line
        .saturating_add(view_height.saturating_mul(2))
        .min(len_lines);

    DocumentInlayHintsId {
        first_line,
        last_line,
    }
}

fn build_inlay_hints_request(
    view: &View,
    doc: &Document,
    offset_encoding: OffsetEncoding,
) -> InlayHintsRequest {
    let id = visible_inlay_hints_id(view, doc);
    let doc_text = doc.text();
    let doc_slice = doc_text.slice(..);
    let first_char = doc_slice.line_to_char(id.first_line);
    let last_char = doc_slice.line_to_char(id.last_line);
    let range = helix_lsp::util::range_to_lsp_range(
        doc_text,
        Range::new(first_char, last_char),
        offset_encoding,
    );

    InlayHintsRequest {
        id,
        range,
        offset_encoding,
    }
}

fn request_inlay_hints(editor: &mut Editor, document_id: DocumentId, view_id: ViewId) {
    if !editor.config().lsp.display_inlay_hints {
        return;
    }

    let Some(view) = editor.tree.try_get(view_id) else {
        return;
    };
    let Some(doc) = editor.document(document_id) else {
        return;
    };
    let current_id = visible_inlay_hints_id(view, doc);
    let Some(language_server) = doc
        .language_servers_with_feature(LanguageServerFeature::InlayHints)
        .next()
    else {
        if let Some(doc) = editor.document_mut(document_id) {
            doc.set_inlay_hints(view_id, DocumentInlayHints::empty_with_id(current_id));
        }
        return;
    };
    let request = build_inlay_hints_request(view, doc, language_server.offset_encoding());
    let Some(future) =
        language_server.text_document_range_inlay_hints(doc.identifier(), request.range, None)
    else {
        return;
    };

    let cancel = match editor.document_mut(document_id) {
        Some(doc) => doc.inlay_hint_controller(view_id).restart(),
        None => return,
    };

    tokio::spawn(async move {
        match cancelable_future(future, &cancel).await {
            Some(Ok(response)) => {
                job::dispatch(move |editor, _| {
                    apply_inlay_hints_response(editor, document_id, view_id, request, response);
                })
                .await;
            }
            Some(Err(err)) => log::error!("inlay hints request failed: {err}"),
            None => {}
        }
    });
}

fn refresh_visible_inlay_hints(editor: &mut Editor) {
    let requests: Vec<_> = editor
        .tree
        .views()
        .filter_map(|(view, _)| {
            let doc = editor.document(view.doc)?;

            (doc.inlay_hints(view.id).map(|hints| hints.id)
                != Some(visible_inlay_hints_id(view, doc)))
            .then_some((view.doc, view.id))
        })
        .collect();

    for (document_id, view_id) in requests {
        request_inlay_hints(editor, document_id, view_id);
    }
}

fn request_all_document_inlay_hints(editor: &mut Editor, document_id: DocumentId) {
    let view_ids: Vec<_> = editor
        .tree
        .views()
        .filter_map(|(view, _)| (view.doc == document_id).then_some(view.id))
        .collect();

    for view_id in view_ids {
        request_inlay_hints(editor, document_id, view_id);
    }
}

fn apply_inlay_hints_response(
    editor: &mut Editor,
    document_id: DocumentId,
    view_id: ViewId,
    request: InlayHintsRequest,
    response: Option<Vec<lsp::InlayHint>>,
) {
    if !editor.config().lsp.display_inlay_hints {
        return;
    }

    let Some(view) = editor.tree.try_get(view_id) else {
        return;
    };
    let current_id = match editor.document(document_id) {
        Some(doc) => visible_inlay_hints_id(view, doc),
        None => return,
    };
    if current_id != request.id {
        return;
    }

    let Some(doc) = editor.document_mut(document_id) else {
        return;
    };

    let mut hints = match response {
        Some(hints) if !hints.is_empty() => hints,
        _ => {
            doc.set_inlay_hints(view_id, DocumentInlayHints::empty_with_id(request.id));
            return;
        }
    };

    hints.sort_by_key(|hint| hint.position);

    let mut padding_before_inlay_hints = Vec::new();
    let mut type_inlay_hints = Vec::new();
    let mut parameter_inlay_hints = Vec::new();
    let mut other_inlay_hints = Vec::new();
    let mut padding_after_inlay_hints = Vec::new();

    let doc_text = doc.text();
    let inlay_hints_length_limit = doc.config.load().lsp.inlay_hints_length_limit;

    for hint in hints {
        let char_idx =
            match helix_lsp::util::lsp_pos_to_pos(doc_text, hint.position, request.offset_encoding)
            {
                Some(pos) => pos,
                None => continue,
            };

        let mut label = match hint.label {
            lsp::InlayHintLabel::String(s) => s,
            lsp::InlayHintLabel::LabelParts(parts) => parts
                .into_iter()
                .map(|part| part.value)
                .collect::<Vec<_>>()
                .join(""),
        };

        if let Some(limit) = inlay_hints_length_limit {
            use helix_core::unicode::{segmentation::UnicodeSegmentation, width::UnicodeWidthStr};

            let width = label.width();
            let limit = limit.get().into();
            if width > limit {
                let mut floor_boundary = 0;
                let mut acc = 0;
                for (idx, grapheme_cluster) in label.grapheme_indices(true) {
                    acc += grapheme_cluster.width();

                    if acc > limit {
                        floor_boundary = idx;
                        break;
                    }
                }

                label.truncate(floor_boundary);
                label.push('…');
            }
        }

        let target = match hint.kind {
            Some(lsp::InlayHintKind::TYPE) => &mut type_inlay_hints,
            Some(lsp::InlayHintKind::PARAMETER) => &mut parameter_inlay_hints,
            _ => &mut other_inlay_hints,
        };

        if let Some(true) = hint.padding_left {
            padding_before_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
        }

        target.push(InlineAnnotation::new(char_idx, label));

        if let Some(true) = hint.padding_right {
            padding_after_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
        }
    }

    doc.set_inlay_hints(
        view_id,
        DocumentInlayHints {
            id: request.id,
            type_inlay_hints,
            parameter_inlay_hints,
            other_inlay_hints,
            padding_before_inlay_hints,
            padding_after_inlay_hints,
        },
    );
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if event.editor.config().lsp.display_inlay_hints {
            helix_event::send_blocking(
                &tx,
                InlayHintsEvent::RefreshDocument {
                    document_id: event.doc,
                },
            );
        }
        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.doc.config.load().lsp.display_inlay_hints && !event.ghost_transaction {
            event.doc.cancel_all_inlay_hint_requests();
            helix_event::send_blocking(
                &tx,
                InlayHintsEvent::RefreshDocument {
                    document_id: event.doc.id(),
                },
            );
        }
        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if event.doc.config.load().lsp.display_inlay_hints {
            event.doc.cancel_inlay_hint_request(event.view);
            helix_event::send_blocking(
                &tx,
                InlayHintsEvent::RefreshView {
                    document_id: event.doc.id(),
                    view_id: event.view,
                },
            );
        }
        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        if event.editor.config().lsp.display_inlay_hints {
            helix_event::send_blocking(&tx, InlayHintsEvent::RefreshVisibleViews);
        }
        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.reset_all_inlay_hints();
            }
        }

        if event.editor.config().lsp.display_inlay_hints {
            helix_event::send_blocking(&tx, InlayHintsEvent::RefreshVisibleViews);
        }
        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        if !event.new.lsp.display_inlay_hints {
            for doc in event.editor.documents_mut() {
                doc.reset_all_inlay_hints();
            }
            return Ok(());
        }

        helix_event::send_blocking(&tx, InlayHintsEvent::RefreshVisibleViews);
        Ok(())
    });
}
