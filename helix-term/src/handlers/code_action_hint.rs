use std::{collections::HashSet, time::Duration};

use futures_util::stream::FuturesUnordered;
use helix_event::{cancelable_future, register_hook, send_blocking, AsyncHook};
use helix_lsp::lsp::{CodeAction, CodeActionOrCommand, CodeActionTriggerKind};
use helix_view::{
    events::{
        ConfigDidChange, DiagnosticsDidChange, DocumentDidChange, DocumentDidOpen,
        LanguageServerExited, LanguageServerInitialized, SelectionDidChange,
    },
    handlers::{lsp::CodeActionHintEvent, Handlers},
    DocumentId, Editor, ViewId,
};
use tokio::time::Instant;
use tokio_stream::StreamExt;

use crate::{commands::code_actions_for_range, job};

#[derive(Debug, Default)]
pub(super) struct Handler {
    doc_ids: HashSet<(DocumentId, ViewId)>,
}

impl AsyncHook for Handler {
    type Event = CodeActionHintEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        self.doc_ids.insert((event.document_id, event.view_id));
        Some(Instant::now() + Duration::from_millis(200))
    }

    fn finish_debounce(&mut self) {
        let ids = std::mem::take(&mut self.doc_ids);
        job::dispatch_blocking(move |editor, _| {
            for (doc_id, view_id) in ids {
                request_code_action_hint(editor, doc_id, view_id);
            }
        })
    }
}

fn request_code_action_hint(editor: &mut Editor, doc_id: DocumentId, view_id: ViewId) {
    if !editor.config().code_action_hint() {
        return;
    }

    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    doc.ensure_view_init(view_id);

    let selection_range = doc.selection(view_id).primary();
    let mut futures: FuturesUnordered<_> =
        code_actions_for_range(doc, selection_range, None, CodeActionTriggerKind::AUTOMATIC)
            .into_iter()
            .map(|(request, _)| async move {
                let Some(mut actions) = request.await? else {
                    return anyhow::Ok(Vec::new());
                };

                // remove disabled code actions
                actions.retain(|action| {
                    matches!(
                        action,
                        CodeActionOrCommand::Command(_)
                            | CodeActionOrCommand::CodeAction(CodeAction { disabled: None, .. })
                    )
                });

                Ok(actions)
            })
            .collect();

    if futures.is_empty() {
        doc.clear_code_action_hints(view_id);
        return;
    };

    let cancel = doc.code_action_controller(view_id).restart();

    tokio::spawn(async move {
        let mut actions = Vec::new();

        loop {
            match cancelable_future(futures.next(), &cancel).await {
                Some(output) => match output {
                    Some(Ok(mut lsp_items)) => actions.append(&mut lsp_items),
                    Some(Err(err)) => log::error!("while gathering code actions: {err}"),
                    None => break,
                },
                // The request was cancelled.
                None => return,
            }
        }

        job::dispatch(move |editor, _| {
            apply_code_action_hint(editor, doc_id, view_id, actions);
        })
        .await;
    });
}

fn apply_code_action_hint(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    code_actions: Vec<CodeActionOrCommand>,
) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };
    if code_actions.is_empty() {
        doc.clear_code_action_hints(view_id);
        return;
    }
    doc.set_code_action_hints(view_id);
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if event.doc.config.load().code_action_hint() {
            let doc_id = event.doc.id();
            let view_id = event.view;
            send_blocking(
                &tx,
                CodeActionHintEvent {
                    document_id: doc_id,
                    view_id,
                },
            );
        }
        Ok(())
    });

    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if !event.editor.config().code_action_hint() {
            return Ok(());
        }
        let view_id = event.editor.tree.focus;
        if event.editor.tree.try_get(view_id).is_none() {
            return Ok(());
        }
        send_blocking(
            &tx,
            CodeActionHintEvent {
                document_id: event.doc,
                view_id,
            },
        );
        Ok(())
    });

    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut DiagnosticsDidChange<'_>| {
        if event.editor.config().code_action_hint() {
            let doc_id = event.doc;
            let views: Vec<_> = event
                .editor
                .tree
                .views()
                .map(|(view, _)| (view.id, view.doc))
                .collect();
            for (view_id, view_doc) in views {
                if view_doc == doc_id {
                    send_blocking(
                        &tx,
                        CodeActionHintEvent {
                            document_id: doc_id,
                            view_id,
                        },
                    );
                }
            }
        }
        Ok(())
    });

    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.doc.config.load().code_action_hint() && !event.ghost_transaction {
            let doc_id = event.doc.id();
            let view_id = event.view;
            send_blocking(
                &tx,
                CodeActionHintEvent {
                    document_id: doc_id,
                    view_id,
                },
            );
        }
        Ok(())
    });

    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        if !event.editor.config().code_action_hint() {
            return Ok(());
        }
        let view_id = event.editor.tree.focus;
        let Some(view) = event.editor.tree.try_get(view_id) else {
            return Ok(());
        };
        let doc_id = view.doc;
        send_blocking(
            &tx,
            CodeActionHintEvent {
                document_id: doc_id,
                view_id,
            },
        );
        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerExited<'_>| {
        for doc in event.editor.documents_mut() {
            if doc.supports_language_server(event.server_id) {
                doc.clear_all_code_action_hints();
            }
        }
        Ok(())
    });

    let tx = handlers.code_action_hint.clone();
    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        // When code action hints are turned on, request them immediately
        // for the focused view instead of waiting for the next selection change.
        if !event.old.code_action_hint() && event.new.code_action_hint() {
            let view_id = event.editor.tree.focus;
            let Some(view) = event.editor.tree.try_get(view_id) else {
                return Ok(());
            };

            send_blocking(
                &tx,
                CodeActionHintEvent {
                    document_id: view.doc,
                    view_id,
                },
            );
            return Ok(());
        }

        // When code action hints are turned off, clear any that were
        // previously rendered across open documents.
        if event.old.code_action_hint() && !event.new.code_action_hint() {
            for doc in event.editor.documents_mut() {
                doc.clear_all_code_action_hints();
            }
        }
        Ok(())
    });
}
