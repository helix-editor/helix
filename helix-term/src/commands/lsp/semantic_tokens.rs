//! Semantic tokens computations for documents.
//!
//! The tokens are then used in highlighting.
use std::future::Future;
use std::sync::Arc;

use helix_lsp::lsp;
use helix_view::document::DocumentSemanticTokens;
use helix_view::editor::Editor;
use helix_view::{Document, View};

pub fn compute_semantic_tokens_for_all_views(editor: &mut Editor, jobs: &mut crate::job::Jobs) {
    if !editor.config().lsp.enable_semantic_tokens_highlighting {
        return;
    }

    for (view, _) in editor.tree.views() {
        let doc = match editor.documents.get(&view.doc) {
            Some(doc) => doc,
            None => continue,
        };
        if let Some(callback) = compute_semantic_tokens_for_view(view, doc) {
            jobs.callback(callback);
        }
    }
}

pub(crate) fn compute_semantic_tokens_for_view(
    view: &View,
    doc: &Document,
) -> Option<std::pin::Pin<Box<impl Future<Output = Result<crate::job::Callback, anyhow::Error>>>>> {
    let language_server = doc.language_server()?;
    let capabilities = language_server.capabilities();

    let view_id = view.id;
    let doc_id = view.doc;

    let lsp_support_ranges = match capabilities.semantic_tokens_provider.as_ref()? {
        lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(opt) => opt.range?,
        lsp::SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(opt) => {
            opt.semantic_tokens_options.range?
        }
    };

    if !lsp_support_ranges {
        return None;
    }

    let doc_text = doc.text();
    let len_lines = doc_text.len_lines();

    // Compute ~3 times the current view height of semantic tokens, that way some scrolling
    // will not show half the view with thel and half without while still being faster
    // than computing all the hints for the full file (which could be dozens of time
    // longer than the view is).
    let view_height = view.inner_height();
    let first_visible_line = doc_text.char_to_line(view.offset.anchor);
    let first_line = first_visible_line.saturating_sub(view_height);
    let last_line = first_visible_line
        .saturating_add(view_height.saturating_mul(2))
        .min(len_lines);

    if !doc.semantic_tokens_outdated
        && doc.semantic_tokens(view_id).map_or(false, |dst| {
            dst.first_line == first_line && dst.last_line == last_line
        })
    {
        return None;
    }

    let doc_slice = doc_text.slice(..);
    let first_char_in_range = doc_slice.line_to_char(first_line);
    let last_char_in_range = doc_slice.line_to_char(last_line);

    let range = helix_lsp::util::range_to_lsp_range(
        doc_text,
        helix_core::Range::new(first_char_in_range, last_char_in_range),
        language_server.offset_encoding(),
    );

    let future = language_server.text_document_semantic_tokens(doc.identifier(), range, None)?;

    let callback = super::super::make_job_callback(
        future,
        move |editor, _compositor, response: Option<lsp::SemanticTokensRangeResult>| {
            // The config was modified or the window was closed while the request was in flight
            if !editor.config().lsp.enable_semantic_tokens_highlighting
                || editor.tree.try_get(view_id).is_none()
            {
                return;
            }

            // Add annotations to relevant document, not the current one (it may have changed in between)
            let doc = match editor.documents.get_mut(&doc_id) {
                Some(doc) => doc,
                None => return,
            };

            let mut dst = DocumentSemanticTokens {
                first_line,
                last_line,
                tokens: Vec::new(),
            };

            // Immutable borrow of doc inside, conflicts with the `set_semantic_tokens` at the end
            {
                let (ls, data) = match (doc.language_server(), response) {
                    (
                        Some(ls),
                        Some(
                            lsp::SemanticTokensRangeResult::Tokens(lsp::SemanticTokens {
                                data,
                                ..
                            })
                            | lsp::SemanticTokensRangeResult::Partial(
                                lsp::SemanticTokensPartialResult { data },
                            ),
                        ),
                    ) if !data.is_empty() => (ls, data),
                    _ => {
                        doc.set_semantic_tokens(
                            view_id,
                            DocumentSemanticTokens {
                                first_line,
                                last_line,
                                tokens: Vec::new(),
                            },
                        );
                        doc.semantic_tokens_outdated = false;
                        return;
                    }
                };

                let offset_encoding = ls.offset_encoding();
                let types_legend = ls.types_legend();
                let modifiers_legend = ls.modifiers_legend();

                let doc_text = doc.text();

                let mut line = 0_u32;
                let mut character = 0;

                for token in data {
                    line = line.saturating_add(token.delta_line);
                    character = if token.delta_line > 0 {
                        token.delta_start
                    } else {
                        character.saturating_add(token.delta_start)
                    };

                    let start = lsp::Position { line, character };
                    let end = lsp::Position {
                        line,
                        character: character.saturating_add(token.length),
                    };

                    let range = match helix_lsp::util::lsp_range_to_range(
                        doc_text,
                        lsp::Range { start, end },
                        offset_encoding,
                    ) {
                        Some(r) => r,
                        None => continue,
                    };

                    let token_type = match types_legend.get(token.token_type as usize) {
                        Some(ty) => Arc::clone(ty),
                        None => continue,
                    };

                    let mut tokens_for_range =
                        Vec::with_capacity(token.token_modifiers_bitset.count_ones() as usize + 1);
                    tokens_for_range.push(token_type);

                    for i in 0..u32::BITS {
                        let mask = 1 << i;

                        if token.token_modifiers_bitset & mask != 0 {
                            if let Some(mo) = modifiers_legend.get(i as usize) {
                                tokens_for_range.push(Arc::clone(mo));
                            }
                        }
                    }

                    dst.tokens.push((range, tokens_for_range));
                }
            }

            doc.set_semantic_tokens(view_id, dst);
        },
    );

    Some(callback)
}
