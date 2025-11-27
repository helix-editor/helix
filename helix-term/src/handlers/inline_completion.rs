use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{register_hook, send_blocking};
use helix_lsp::{lsp, util::lsp_range_to_range};
use helix_view::{
    document::{InlineCompletion, Mode},
    events::DocumentDidChange,
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
        job::dispatch_blocking(move |editor, _| {
            // User may have left insert mode before debounce fired
            if editor.mode != Mode::Insert {
                return;
            }
            let (view, doc) = current!(editor);
            let doc_id = doc.id();
            let view_id = view.id;

            for ls in doc.language_servers_with_feature(LanguageServerFeature::InlineCompletion) {
                let pos = doc.position(view.id, ls.offset_encoding());
                let context = lsp::InlineCompletionContext {
                    trigger_kind: lsp::InlineCompletionTriggerKind::Automatic,
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
                    let Some(item) = items.into_iter().next() else {
                        return;
                    };

                    job::dispatch(move |editor, _| {
                        // User may have left insert mode while request was in flight
                        if editor.mode != Mode::Insert {
                            return;
                        }
                        let Some(doc) = editor.documents.get_mut(&doc_id) else {
                            return;
                        };
                        let text = doc.text();
                        let cursor = doc.selection(view_id).primary().cursor(text.slice(..));

                        let replace_range = item
                            .range
                            .and_then(|r| lsp_range_to_range(text, r, offset_encoding));

                        let offset = match replace_range {
                            Some(r) if cursor > r.to() => return, // stale
                            Some(r) => cursor.saturating_sub(r.from()),
                            None => 0,
                        };

                        if item
                            .insert_text
                            .get(offset..)
                            .is_some_and(|s| !s.is_empty())
                        {
                            doc.inline_completions.push(InlineCompletion::new(
                                cursor,
                                item.insert_text,
                                offset,
                                replace_range,
                            ));
                        }
                    })
                    .await;
                });
            }
        });
    }
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.inline_completions.clone();

    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        // Clear stale completion: it was computed for the previous document state
        event.doc.inline_completions.take_and_clear();
        // Ignore changes caused by a preview being displayed
        if event.ghost_transaction {
            return Ok(());
        }

        send_blocking(&tx, ());
        Ok(())
    });
}
