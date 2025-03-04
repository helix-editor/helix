use std::{collections::HashSet, mem, time::Duration};

use crate::job;

use super::Handlers;

use helix_core::{syntax::LanguageServerFeature, text_annotations::InlineAnnotation};
use helix_event::{cancelable_future, register_hook, send_blocking};
use helix_lsp::lsp;
use helix_view::{
    document::{DocumentInlayHints, DocumentInlayHintsId},
    events::{
        DocumentDidChange, DocumentDidOpen, LanguageServerExited, LanguageServerInitialized,
        SelectionDidChange,
    },
    handlers::lsp::InlayHintEvent,
    DocumentId, Editor, ViewId,
};
use tokio::time::Instant;

#[derive(Debug, Default)]
pub(super) struct InlayHintHandler {
    views: HashSet<ViewId>,
    docs: HashSet<DocumentId>,
}

const DOCUMENT_CHANGE_DEBOUNCE: Duration = Duration::from_millis(500);
const VIEWPORT_SCROLL_DEBOUNCE: Duration = Duration::from_millis(100);

impl helix_event::AsyncHook for InlayHintHandler {
    type Event = InlayHintEvent;

    fn handle_event(&mut self, event: Self::Event, timeout: Option<Instant>) -> Option<Instant> {
        match event {
            InlayHintEvent::DocumentChanged(doc) => {
                self.docs.insert(doc);
                Some(Instant::now() + DOCUMENT_CHANGE_DEBOUNCE)
            }
            InlayHintEvent::ViewportScrolled(view) => {
                self.views.insert(view);
                let mut new_timeout = Instant::now() + VIEWPORT_SCROLL_DEBOUNCE;
                if let Some(timeout) = timeout {
                    new_timeout = new_timeout.max(timeout);
                }
                Some(new_timeout)
            }
        }
    }

    fn finish_debounce(&mut self) {
        let mut views = mem::take(&mut self.views);
        let docs = mem::take(&mut self.docs);

        job::dispatch_blocking(move |editor, _compositor| {
            // Drop any views which have been closed.
            views.retain(|&view| editor.tree.contains(view));
            // Add any views that show documents which changed.
            views.extend(
                editor
                    .tree
                    .views()
                    .filter_map(|(view, _)| docs.contains(&view.doc).then_some(view.id)),
            );

            for view in views {
                let doc = editor.tree.get(view).doc;
                let is_scroll = !docs.contains(&doc);
                request_inlay_hints_for_view(editor, view, doc, is_scroll);
            }
        });
    }
}

fn request_inlay_hints_for_view(
    editor: &mut Editor,
    view_id: ViewId,
    doc_id: DocumentId,
    is_scroll: bool,
) {
    if !editor.config().lsp.display_inlay_hints {
        return;
    }
    let Some(doc) = editor.documents.get_mut(&doc_id) else {
        return;
    };
    let Some(view) = editor.tree.try_get(view_id) else {
        return;
    };
    let Some(language_server) = doc
        .language_servers_with_feature(LanguageServerFeature::InlayHints)
        .next()
    else {
        return;
    };

    let rope = doc.text();
    let text = rope.slice(..);
    let len_lines = text.len_lines();
    let view_height = view.inner_height();
    let first_visible_line =
        text.char_to_line(doc.view_offset(view_id).anchor.min(text.len_chars()));
    let first_line = first_visible_line.saturating_sub(view_height);
    let last_line = first_visible_line
        .saturating_add(view_height.saturating_mul(2))
        .min(len_lines);
    let new_doc_inlay_hints_id = DocumentInlayHintsId {
        first_line,
        last_line,
    };
    // If the view was updated by scrolling (rather than changing) and the viewport still has the
    // the same position, we can reuse the hints.
    if is_scroll
        && doc
            .inlay_hints(view_id)
            .is_some_and(|hint| hint.id == new_doc_inlay_hints_id)
    {
        return;
    }
    let offset_encoding = language_server.offset_encoding();
    let range = helix_lsp::util::range_to_lsp_range(
        rope,
        helix_core::Range::new(text.line_to_char(first_line), text.line_to_char(last_line)),
        offset_encoding,
    );
    let future = language_server
        .text_document_range_inlay_hints(doc.identifier(), range, None)
        .expect("language server must return Some if it supports inlay hints");
    let controller = doc.inlay_hint_controllers.entry(view_id).or_default();
    let cancel = controller.restart();

    tokio::spawn(async move {
        match cancelable_future(future, cancel).await {
            Some(Ok(res)) => {
                job::dispatch(move |editor, _compositor| {
                    attach_inlay_hints(
                        editor,
                        view_id,
                        doc_id,
                        new_doc_inlay_hints_id,
                        offset_encoding,
                        res,
                    );
                })
                .await
            }
            Some(Err(err)) => log::error!("inlay hint request failed: {err}"),
            None => (),
        }
    });
}

fn attach_inlay_hints(
    editor: &mut Editor,
    view_id: ViewId,
    doc_id: DocumentId,
    id: DocumentInlayHintsId,
    offset_encoding: helix_lsp::OffsetEncoding,
    response: Option<Vec<lsp::InlayHint>>,
) {
    if !editor.config().lsp.display_inlay_hints || editor.tree.try_get(view_id).is_none() {
        return;
    }

    let Some(doc) = editor.documents.get_mut(&doc_id) else {
        return;
    };

    let mut hints = match response {
        Some(hints) if !hints.is_empty() => hints,
        _ => {
            doc.set_inlay_hints(view_id, DocumentInlayHints::empty_with_id(id));
            return;
        }
    };

    // Most language servers will already send them sorted but ensure this is the case to
    // avoid errors on our end.
    hints.sort_by_key(|inlay_hint| inlay_hint.position);

    let mut padding_before_inlay_hints = Vec::new();
    let mut type_inlay_hints = Vec::new();
    let mut parameter_inlay_hints = Vec::new();
    let mut other_inlay_hints = Vec::new();
    let mut padding_after_inlay_hints = Vec::new();

    let doc_text = doc.text();

    for hint in hints {
        let char_idx =
            match helix_lsp::util::lsp_pos_to_pos(doc_text, hint.position, offset_encoding) {
                Some(pos) => pos,
                // Skip inlay hints that have no "real" position
                None => continue,
            };

        let label = match hint.label {
            lsp::InlayHintLabel::String(s) => s,
            lsp::InlayHintLabel::LabelParts(parts) => parts
                .into_iter()
                .map(|p| p.value)
                .collect::<Vec<_>>()
                .join(""),
        };

        let inlay_hints_vec = match hint.kind {
            Some(lsp::InlayHintKind::TYPE) => &mut type_inlay_hints,
            Some(lsp::InlayHintKind::PARAMETER) => &mut parameter_inlay_hints,
            // We can't warn on unknown kind here since LSPs are free to set it or not, for
            // example Rust Analyzer does not: every kind will be `None`.
            _ => &mut other_inlay_hints,
        };

        if let Some(true) = hint.padding_left {
            padding_before_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
        }

        inlay_hints_vec.push(InlineAnnotation::new(char_idx, label));

        if let Some(true) = hint.padding_right {
            padding_after_inlay_hints.push(InlineAnnotation::new(char_idx, " "));
        }
    }

    doc.set_inlay_hints(
        view_id,
        DocumentInlayHints {
            id,
            type_inlay_hints,
            parameter_inlay_hints,
            other_inlay_hints,
            padding_before_inlay_hints,
            padding_after_inlay_hints,
        },
    );
}

pub(super) fn register_hooks(handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        // When a document is initially opened, request inlay hints for it.
        let views: Vec<_> = event
            .editor
            .tree
            .views()
            .filter_map(|(view, _)| (view.doc == event.doc).then_some(view.id))
            .collect();
        for view in views {
            request_inlay_hints_for_view(event.editor, view, event.doc, false);
        }

        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        // Update the inlay hint annotations' positions, helping ensure they are displayed in the
        // proper place.
        let apply_inlay_hint_changes = |annotations: &mut Vec<InlineAnnotation>| {
            event.changes.update_positions(
                annotations
                    .iter_mut()
                    .map(|annotation| (&mut annotation.char_idx, helix_core::Assoc::After)),
            );
        };

        for (_view_id, text_annotation) in event.doc.inlay_hints_mut() {
            let DocumentInlayHints {
                id: _,
                type_inlay_hints,
                parameter_inlay_hints,
                other_inlay_hints,
                padding_before_inlay_hints,
                padding_after_inlay_hints,
            } = text_annotation;

            apply_inlay_hint_changes(padding_before_inlay_hints);
            apply_inlay_hint_changes(type_inlay_hints);
            apply_inlay_hint_changes(parameter_inlay_hints);
            apply_inlay_hint_changes(other_inlay_hints);
            apply_inlay_hint_changes(padding_after_inlay_hints);
        }

        if !event.ghost_transaction {
            if let Some(controller) = event.doc.inlay_hint_controllers.get_mut(&event.view) {
                controller.cancel();
            }
            // TODO: ideally we should only send this if the document is visible.
            send_blocking(&tx, InlayHintEvent::DocumentChanged(event.doc.id()));
        }

        Ok(())
    });

    let tx = handlers.inlay_hints.clone();
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if let Some(controller) = event.doc.inlay_hint_controllers.get_mut(&event.view) {
            controller.cancel();
        }
        // Ideally this would only trigger an update if the viewport changed...
        send_blocking(&tx, InlayHintEvent::ViewportScrolled(event.view));

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let views: Vec<_> = event
            .editor
            .tree
            .views()
            .map(|(view, _)| (view.id, view.doc))
            .collect();
        for (view, doc) in views {
            request_inlay_hints_for_view(event.editor, view, doc, false);
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        // Clear and re-request all annotations when a server exits.
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.reset_all_inlay_hints();
            }
        }

        let views: Vec<_> = event
            .editor
            .tree
            .views()
            .map(|(view, _)| (view.id, view.doc))
            .collect();
        for (view, doc) in views {
            request_inlay_hints_for_view(event.editor, view, doc, false);
        }

        Ok(())
    });
}
