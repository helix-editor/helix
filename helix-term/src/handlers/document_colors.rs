use std::{collections::HashSet, time::Duration};

use helix_core::{syntax::LanguageServerFeature, text_annotations::InlineAnnotation};
use helix_event::{cancelable_future, register_hook, TaskController};
use helix_lsp::{
    lsp::{self, ColorInformation},
    OffsetEncoding,
};
use helix_view::{
    document::DocumentColorSwatches,
    events::{DocumentDidChange, DocumentDidOpen, LanguageServerExited, LanguageServerInitialized},
    handlers::{lsp::DocumentColorsEvent, Handlers},
    DocumentId, Editor, Theme, ViewId,
};
use tokio::time::Instant;

use crate::job;

#[derive(Default)]
pub(super) struct DocumentColorsHandler {
    views: HashSet<ViewId>,
    docs: HashSet<DocumentId>,
}

impl DocumentColorsHandler {
    pub fn new() -> Self {
        Self::default()
    }
}

const DOCUMENT_CHANGE_DEBOUNCE: Duration = Duration::from_millis(50);

impl helix_event::AsyncHook for DocumentColorsHandler {
    type Event = DocumentColorsEvent;

    fn handle_event(&mut self, event: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        let DocumentColorsEvent(doc_id) = event;
        self.docs.insert(doc_id);
        Some(Instant::now() + DOCUMENT_CHANGE_DEBOUNCE)
    }

    fn finish_debounce(&mut self) {
        let mut views = std::mem::take(&mut self.views);
        let docs = std::mem::take(&mut self.docs);

        job::dispatch_blocking(move |editor, _compositor| {
            editor
                .handlers
                .document_colors
                .active_requests
                .retain(|_, controller| controller.is_running());

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
                request_document_colors_for_view(editor, view, doc);
            }
        });
    }
}

fn request_document_colors_for_view(editor: &mut Editor, view_id: ViewId, doc_id: DocumentId) {
    if !editor.config().lsp.display_color_swatches {
        return;
    }

    // Cancel any ongoing events for this view.
    if let Some(mut controller) = editor
        .handlers
        .document_colors
        .active_requests
        .remove(&view_id)
    {
        controller.cancel();
    }

    let Some(doc) = editor.document(doc_id) else {
        return;
    };

    let Some(language_server) = doc
        .language_servers_with_feature(LanguageServerFeature::DocumentColors)
        .next()
    else {
        return;
    };

    let offset_encoding = language_server.offset_encoding();

    let future = language_server
        .text_document_colors(doc.identifier(), None)
        .expect("language server must return Some if it supports document colors");

    let mut controller = TaskController::new();
    let cancel = controller.restart();
    editor
        .handlers
        .document_colors
        .active_requests
        .insert(view_id, controller);

    tokio::spawn(async move {
        match cancelable_future(future, cancel).await {
            Some(Ok(response)) => {
                job::dispatch(move |editor, _| {
                    attach_document_colors(editor, view_id, doc_id, response, offset_encoding)
                })
                .await;
            }
            Some(Err(err)) => log::error!("document color request failed: {err}"),
            None => (),
        }
    });
}

fn attach_document_colors(
    editor: &mut Editor,
    view_id: ViewId,
    doc_id: DocumentId,
    response: Option<Vec<lsp::ColorInformation>>,
    offset_encoding: OffsetEncoding,
) {
    if !editor.config().lsp.display_color_swatches || editor.tree.try_get(view_id).is_none() {
        return;
    }

    let Some(doc) = editor.documents.get_mut(&doc_id) else {
        return;
    };

    let mut doc_colors = match response {
        Some(colors) if !colors.is_empty() => colors,
        _ => {
            doc.set_color_swatches(view_id, DocumentColorSwatches::default());
            return;
        }
    };

    // Most language servers will already send them sorted but ensure this is the case to avoid errors on our end
    doc_colors.sort_unstable_by_key(|color| color.range.start);

    let mut color_swatches = Vec::with_capacity(doc_colors.len());
    let mut color_swatches_padding = Vec::with_capacity(doc_colors.len());
    let mut colors = Vec::with_capacity(doc_colors.len());

    let doc_text = doc.text();

    for ColorInformation { range, color } in doc_colors {
        let swatch_idx =
            match helix_lsp::util::lsp_pos_to_pos(doc_text, range.start, offset_encoding) {
                Some(pos) => pos,
                // Skip color swatches that have no "real" position
                None => continue,
            };
        color_swatches_padding.push(InlineAnnotation::new(swatch_idx, " "));
        color_swatches.push(InlineAnnotation::new(swatch_idx, "■"));
        colors.push(Theme::rgb_highlight(
            (color.red * 255.) as u8,
            (color.green * 255.) as u8,
            (color.blue * 255.) as u8,
        ));
    }

    let swatches = DocumentColorSwatches {
        color_swatches,
        colors,
        color_swatches_padding,
    };

    doc.set_color_swatches(view_id, swatches)
}

pub(super) fn register_hooks(handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        // when a document is initially opened, request colors for it
        let views: Vec<_> = event
            .editor
            .tree
            .views()
            .filter_map(|(view, _)| (view.doc == event.doc).then_some(view.id))
            .collect();

        for view in views {
            request_document_colors_for_view(event.editor, view, event.doc);
        }

        Ok(())
    });

    // Once these events carry a reference to the Editor then this `tx` method can be dropped
    // and we can use `DocumentColorHandler::event` instead.
    let tx = handlers.document_colors.tx().clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        // Update the color swatch' positions, helping ensure they are displayed in the
        // proper place.
        let apply_color_swatch_changes = |annotations: &mut Vec<InlineAnnotation>| {
            event.changes.update_positions(
                annotations
                    .iter_mut()
                    .map(|annotation| (&mut annotation.char_idx, helix_core::Assoc::After)),
            );
        };

        for (_view_id, color_swatch) in event.doc.color_swatches_mut() {
            let DocumentColorSwatches {
                color_swatches,
                colors: _colors,
                color_swatches_padding,
            } = color_swatch;

            apply_color_swatch_changes(color_swatches);
            apply_color_swatch_changes(color_swatches_padding);
        }

        // TODO: ideally we should only send this if the document is visible.
        helix_event::send_blocking(&tx, DocumentColorsEvent(event.doc.id()));

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
            request_document_colors_for_view(event.editor, view, doc);
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        // Clear and re-request all color swatches when a server exits.
        for doc in event.editor.documents_mut() {
            if doc
                .language_servers()
                .any(|server| server.id() == event.server_id)
            {
                doc.reset_all_color_swatches();
            }
        }

        let views: Vec<_> = event
            .editor
            .tree
            .views()
            .map(|(view, _)| (view.id, view.doc))
            .collect();
        for (view, doc) in views {
            request_document_colors_for_view(event.editor, view, doc);
        }

        Ok(())
    });
}
