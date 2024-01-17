use helix_event::{cancelable_future, CancelRx};
use helix_lsp::{lsp, util::range_to_lsp_range};
use helix_view::{handlers::lsp::SemanticTokensEvent, Document, DocumentId, Editor, View, ViewId};

use crate::job;

const TIMEOUT: u64 = 250;

struct State {
    doc_id: DocumentId,
    view_id: ViewId,
}

#[derive(Debug)]
pub(super) struct SemanticTokensHandler {}

impl helix_event::AsyncHook for SemanticTokensHandler {
    type Event = SemanticTokensEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        None
    }

    fn finish_debounce(&mut self) {
        job::dispatch_blocking(|editor, compositor| {
            request_semantic_tokens(editor, compositor);
        });
    }
}

fn request_semantic_tokens(editor: &mut Editor, _compositor: &mut crate::compositor::Compositor) {
    // TODO: Check config for semantic tokens
    for (view, _) in editor.tree.views() {
        let Some(doc) = editor.documents.get(&view.doc) else {
            continue;
        };

        let state = State {
            doc_id: doc.id(),
            view_id: view.id,
        };

        tokio::spawn(async move {
            job::dispatch(move |editor, _compositor| {
                request_semantic_tokens_for_view(editor, state);
            })
            .await;
        });
        // if let Some(callback) = request_semantic_tokens_for_view(view, doc) {
        //     job::dispatch_blocking(move |editor, compositor| {
        //         tokio::spawn(async move {
        //             request_semantic_tokens_for_view(, )
        //         })
        //     });
        // }
    }
}

// TODO: Change from Option maybe
fn request_semantic_tokens_for_view(editor: &mut Editor, state: State) -> Option<()> {
    let doc = editor.document(state.doc_id)?;
    let view = editor.tree.try_get(state.view_id)?;
    let text = doc.text();

    // Compute range to request semantic tokens for.
    let view_height = view.inner_height();
    let first_visible_line = text.char_to_line(view.offset.anchor.min(text.len_chars()));
    let first_line = first_visible_line.saturating_sub(view_height);
    let last_line = first_visible_line
        .saturating_add(view_height.saturating_mul(2))
        .min(text.len_lines());
    let start = text.line_to_char(first_line);
    let end = text.line_to_char(last_line);

    let future = doc
        .language_servers_with_feature(helix_core::syntax::LanguageServerFeature::SemanticTokens)
        .find_map(|ls| {
            let range = range_to_lsp_range(
                text,
                helix_core::Range::new(start, end),
                ls.offset_encoding(),
            );
            let id = doc.identifier();
            ls.text_document_semantic_tokens(id, range, None)
        });

    todo!()
}

// fn request_semantic_tokens(editor: &mut Editor, cancel: CancelRx) {
//     let (view, doc) = current!(editor);
//     let text = doc.text();

//     // Compute range to request semantic tokens for.
//     let view_height = view.inner_height();
//     let first_visible_line = text.char_to_line(view.offset.anchor.min(text.len_chars()));
//     let first_line = first_visible_line.saturating_sub(view_height);
//     let last_line = first_visible_line
//         .saturating_add(view_height.saturating_mul(2))
//         .min(text.len_lines());
//     let start = text.line_to_char(first_line);
//     let end = text.line_to_char(last_line);

//     let future = doc
//         .language_servers_with_feature(helix_core::syntax::LanguageServerFeature::SemanticTokens)
//         .find_map(|ls| {
//             let range = range_to_lsp_range(
//                 text,
//                 helix_core::Range::new(start, end),
//                 ls.offset_encoding(),
//             );
//             let id = doc.identifier();
//             ls.text_document_semantic_tokens(id, range, None)
//         });
//     let Some(future) = future else {
//         editor.set_error("No configured language server supports semantic tokens");
//         return;
//     };

//     tokio::spawn(async move {
//         match cancelable_future(future, cancel).await {
//             Some(Ok(res)) => {
//                 job::dispatch(move |editor, compositor| {
//                     compute_semantic_tokens(editor, compositor, res)
//                 })
//                 .await
//             }
//             Some(Err(err)) => log::error!("semantic tokens request failed: {err}"),
//             None => (),
//         }
//     });
// }

// fn compute_semantic_tokens(
//     editor: &mut Editor,
//     compositor: &mut crate::compositor::Compositor,
//     response: Option<lsp::SemanticTokens>,
// ) {
//     let (view, doc) = current_ref!(editor);
//     if editor.mode != Mode::Insert || view.id != trigger.view || doc.id() != trigger.doc {
//         return;
//     }

//     todo!()
// }
