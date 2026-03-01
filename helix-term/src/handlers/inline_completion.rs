use std::sync::Arc;

use crate::events::OnModeSwitch;
use arc_swap::ArcSwap;
use helix_core::{syntax::config::LanguageServerFeature, text_annotations::Overlay, Range};
use helix_event::{register_hook, send_blocking};
use helix_lsp::{lsp, util::lsp_range_to_range};
use helix_view::{
    document::{InlineCompletion, Mode},
    events::{DocumentDidChange, SelectionDidChange},
    handlers::Handlers,
};
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

                            let at_eol = text.get_char(cursor).is_none_or(|c| c == '\n');

                            // Process ghost text: expand tabs and split into lines
                            let tab_spaces: String = " ".repeat(tab_width);
                            let mut lines: Vec<String> = ghost_text
                                .split('\n')
                                .map(|line| line.replace('\t', &tab_spaces))
                                .collect();

                            let first_line = lines.remove(0);

                            // Get rest of line for mid-line cases
                            let line_end = text.line_to_char(text.char_to_line(cursor) + 1);
                            let rest_of_line: String = text
                                .slice(cursor..line_end)
                                .chars()
                                .take_while(|c| *c != '\n')
                                .collect();

                            let (overlays, overflow_text, eol_ghost_text, additional_lines) =
                                if at_eol {
                                    // At EOL: use Decoration to render first line
                                    let eol_text = if !first_line.is_empty() {
                                        Some(first_line)
                                    } else {
                                        None
                                    };
                                    (Vec::new(), None, eol_text, lines)
                                } else {
                                    // Mid-line: use overlays for first line (no cursor shift)
                                    // Additional lines (if any) rendered as virtual lines below
                                    let is_multiline = !lines.is_empty();

                                    let after_cursor: String =
                                        rest_of_line.chars().skip(1).collect();

                                    // Trim matching suffix for display only (single-line only)
                                    let mut display_first_line = first_line.clone();
                                    if !is_multiline {
                                        for suffix_len in (1..=after_cursor.len()).rev() {
                                            if let Some(suffix) = after_cursor.get(..suffix_len) {
                                                if display_first_line.ends_with(suffix) {
                                                    let new_len =
                                                        display_first_line.len() - suffix.len();
                                                    display_first_line.truncate(new_len);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    // Build preview:
                                    // - Single-line: first line + after_cursor (shows full result)
                                    // - Multi-line: first line padded with spaces to cover rest_of_line
                                    //   (rest_of_line is pushed down to last ghost line, so blank it here)
                                    let preview = if is_multiline {
                                        let rest_len = rest_of_line.chars().count();
                                        let first_len = display_first_line.chars().count();
                                        if first_len < rest_len {
                                            // Pad with spaces to cover rest_of_line
                                            format!(
                                                "{}{}",
                                                display_first_line,
                                                " ".repeat(rest_len - first_len)
                                            )
                                        } else {
                                            display_first_line
                                        }
                                    } else {
                                        format!("{}{}", display_first_line, after_cursor)
                                    };

                                    // Create overlay for each position
                                    let mut overlays = Vec::new();
                                    for (i, preview_char) in preview.chars().enumerate() {
                                        if i >= rest_of_line.chars().count() {
                                            break;
                                        }
                                        overlays.push(Overlay::new(
                                            cursor + i,
                                            preview_char.to_string(),
                                        ));
                                    }

                                    // Overflow: preview chars beyond rest_of_line
                                    let overflow: String = preview
                                        .chars()
                                        .skip(rest_of_line.chars().count())
                                        .collect();
                                    let overflow_text = if !overflow.is_empty() {
                                        Some(overflow)
                                    } else {
                                        None
                                    };

                                    // For multi-line, append rest_of_line to last line ("pushed down")
                                    // But only if ghost text doesn't already contain rest_of_line
                                    // (if it does, ghost text is replacing cursor position content)
                                    let additional_lines = if is_multiline {
                                        let mut result = lines;
                                        let ghost_contains_rest = first_line
                                            .contains(&rest_of_line)
                                            || result.iter().any(|l| l.contains(&rest_of_line));
                                        if !ghost_contains_rest {
                                            if let Some(last) = result.last_mut() {
                                                last.push_str(&rest_of_line);
                                            }
                                        }
                                        result
                                    } else {
                                        lines
                                    };

                                    (overlays, overflow_text, None, additional_lines)
                                };

                            Some(InlineCompletion {
                                ghost_text,
                                replace_range,
                                cursor_char_idx: cursor,
                                overlays,
                                overflow_text,
                                eol_ghost_text,
                                additional_lines,
                            })
                        })
                        .collect();

                    for completion in completions {
                        doc.inline_completions.push(completion);
                    }

                    // Rebuild overlay cache
                    doc.inline_completions
                        .rebuild_overlays(&mut doc.inline_completion_overlays);
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
        // Also clear overlay cache
        event.doc.inline_completion_overlays.clear();
        // Ignore changes caused by a preview being displayed
        if event.ghost_transaction {
            return Ok(());
        }

        if event.doc.config.load().inline_completion_auto_trigger {
            send_blocking(&tx, ());
        }
        Ok(())
    });

    // Clear inline completions when cursor moves (e.g., arrow keys)
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        event.doc.inline_completions.take_and_clear();
        event.doc.inline_completion_overlays.clear();
        Ok(())
    });

    // Clear inline completions when leaving insert mode
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.old_mode == Mode::Insert && event.new_mode != Mode::Insert {
            let (_, doc) = current!(event.cx.editor);
            doc.inline_completions.take_and_clear();
            doc.inline_completion_overlays.clear();
        }
        Ok(())
    });
}
