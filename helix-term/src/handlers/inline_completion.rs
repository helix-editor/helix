use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::{
    syntax::config::LanguageServerFeature,
    text_annotations::{InlineAnnotation, Overlay},
    Range,
};
use helix_event::{register_hook, send_blocking};
use helix_lsp::{lsp, util::lsp_range_to_range};
use helix_view::{
    document::{InlineCompletion, Mode},
    events::DocumentDidChange,
    handlers::Handlers,
};
use crate::events::OnModeSwitch;
use helix_core::unicode::segmentation::UnicodeSegmentation;
use tokio::time::Instant;

use crate::{config::Config, job};

pub(super) struct InlineCompletionHandler {
    config: Arc<ArcSwap<Config>>,
}

impl InlineCompletionHandler {
    pub fn new(config: Arc<ArcSwap<Config>>) -> Self {
        Self { config }
    }
}

impl helix_event::AsyncHook for InlineCompletionHandler {
    type Event = ();

    fn handle_event(&mut self, _: Self::Event, _: Option<Instant>) -> Option<Instant> {
        Some(Instant::now() + self.config.load().editor.inline_completion_timeout)
    }

    fn finish_debounce(&mut self) {
        trigger_inline_completion(lsp::InlineCompletionTriggerKind::Automatic);
    }
}

/// Request inline completion from LSP servers. Called by debounce handler (auto)
/// or directly by manual trigger command.
pub fn trigger_inline_completion(trigger_kind: lsp::InlineCompletionTriggerKind) {
    job::dispatch_blocking(move |editor, _| {
        // Only trigger in insert mode
        if editor.mode != Mode::Insert {
            return;
        }
        let (view, doc) = current!(editor);
        // DocumentId is monotonic; ViewId uses slotmap versioning (reused slots get new version).
        let doc_id = doc.id();
        let view_id = view.id;
        let doc_version = doc.version();
        // Capture tab_width for ghost text processing
        let tab_width = doc.tab_width();

        for ls in doc.language_servers_with_feature(LanguageServerFeature::InlineCompletion) {
            let pos = doc.position(view.id, ls.offset_encoding());
            let context = lsp::InlineCompletionContext {
                trigger_kind,
                selected_completion_info: None,
            };
            let Some(fut) = ls.inline_completion(doc.identifier(), pos, context, None) else {
                continue;
            };

            let offset_encoding = ls.offset_encoding();
            tokio::spawn(async move {
                let Ok(Some(resp)) = fut.await else { return };
                let items = match resp {
                    lsp::InlineCompletionResponse::Array(v) => v,
                    lsp::InlineCompletionResponse::List(l) => l.items,
                };
                if items.is_empty() {
                    return;
                }

                job::dispatch(move |editor, _| {
                    // User may have left insert mode, switched view/doc, or edited the document
                    let (view, doc) = current!(editor);
                    if editor.mode != Mode::Insert
                        || view.id != view_id
                        || doc.id() != doc_id
                        || doc.version() != doc_version
                    {
                        return;
                    }
                    let text = doc.text();
                    let cursor = doc.selection(view.id).primary().cursor(text.slice(..));

                    let completions: Vec<_> = items
                        .into_iter()
                        .filter_map(|item| {
                            let replace_range = item
                                .range
                                .and_then(|r| lsp_range_to_range(text, r, offset_encoding))
                                .unwrap_or_else(|| Range::point(cursor));

                            // Discard if cursor moved outside range (e.g., arrow keys don't change version)
                            if !replace_range.contains_range(&Range::point(cursor)) {
                                return None;
                            }

                            // Skip already-typed chars
                            let skip = cursor.saturating_sub(replace_range.from());
                            let ghost_text: String = item.insert_text.chars().skip(skip).collect();

                            if ghost_text.is_empty() {
                                return None;
                            }

                            // Process ghost text: expand tabs and split into lines
                            let tab_spaces: String = " ".repeat(tab_width);
                            let mut lines: Vec<String> = ghost_text
                                .split('\n')
                                .map(|line| line.replace('\t', &tab_spaces))
                                .collect();

                            let first_line = lines.remove(0);

                            // Check if cursor is at EOL (on newline or past end)
                            let at_eol = text.get_char(cursor).is_none_or(|c| c == '\n');

                            let (first_char_overlay, rest_of_line_annotation, eol_ghost_text) =
                                if at_eol {
                                    // At EOL: use Decoration to render first line (doesn't shift cursor)
                                    let eol_text = if !first_line.is_empty() {
                                        Some(first_line)
                                    } else {
                                        None
                                    };
                                    (None, None, eol_text)
                                } else {
                                    // Mid-line: overlay first char, annotate rest
                                    let mut graphemes = first_line.graphemes(true);

                                    // First grapheme becomes Overlay (appears ON block cursor)
                                    let first_char_overlay = graphemes
                                        .next()
                                        .map(|g| Overlay::new(cursor, g.to_string()));

                                    // Rest of first line becomes InlineAnnotation (at cursor+1, shifts content)
                                    let rest: String = graphemes.collect();
                                    let rest_of_line_annotation = if !rest.is_empty() {
                                        Some(InlineAnnotation::new(cursor + 1, rest))
                                    } else {
                                        None
                                    };
                                    (first_char_overlay, rest_of_line_annotation, None)
                                };

                            Some(InlineCompletion {
                                ghost_text,
                                replace_range,
                                cursor_char_idx: cursor,
                                first_char_overlay,
                                rest_of_line_annotation,
                                eol_ghost_text,
                                additional_lines: lines,
                            })
                        })
                        .collect();

                    for completion in completions {
                        doc.inline_completions.push(completion);
                    }

                    // Rebuild annotation caches
                    doc.inline_completions.rebuild_annotations(
                        &mut doc.inline_completion_overlay,
                        &mut doc.inline_completion_annotations,
                    );
                })
                .await;
            });
        }
    });
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.inline_completions.clone();

    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        // Clear stale completion: it was computed for the previous document state
        event.doc.inline_completions.take_and_clear();
        // Also clear annotation caches
        event.doc.inline_completion_overlay.clear();
        event.doc.inline_completion_annotations.clear();
        // Ignore changes caused by a preview being displayed
        if event.ghost_transaction {
            return Ok(());
        }

        if event.doc.config.load().inline_completion_auto_trigger {
            send_blocking(&tx, ());
        }
        Ok(())
    });

    // Clear inline completions when leaving insert mode
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.old_mode == Mode::Insert && event.new_mode != Mode::Insert {
            let (_, doc) = current!(event.cx.editor);
            doc.inline_completions.take_and_clear();
            doc.inline_completion_overlay.clear();
            doc.inline_completion_annotations.clear();
        }
        Ok(())
    });
}
