use std::collections::HashMap;

use helix_core::chars::char_is_word;
use helix_core::completion::CompletionProvider;
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{register_hook, TaskHandle};
use helix_lsp::lsp;
use helix_stdx::rope::RopeSliceExt;
use helix_view::document::Mode;
use helix_view::handlers::completion::{CompletionEvent, ResponseContext};
use helix_view::Editor;
use tokio::task::JoinSet;

use crate::commands;
use crate::compositor::Compositor;
use crate::events::{OnModeSwitch, PostCommand, PostInsertChar};
use crate::handlers::completion::request::{request_incomplete_completion_list, Trigger};
use crate::job::dispatch;
use crate::keymap::MappableCommand;
use crate::ui::lsp::signature_help::SignatureHelp;
use crate::ui::{self, Popup};

use super::Handlers;

pub use item::{CompletionItem, CompletionItems, CompletionResponse, LspCompletionItem};
pub use request::CompletionHandler;
pub use resolve::ResolveHandler;

mod item;
mod path;
mod request;
mod resolve;
mod word;

async fn handle_response(
    requests: &mut JoinSet<CompletionResponse>,
    is_incomplete: bool,
) -> Option<CompletionResponse> {
    loop {
        let response = requests.join_next().await?.unwrap();
        if !is_incomplete && !response.context.is_incomplete && response.items.is_empty() {
            continue;
        }
        return Some(response);
    }
}

async fn replace_completions(
    handle: TaskHandle,
    mut requests: JoinSet<CompletionResponse>,
    is_incomplete: bool,
) {
    while let Some(mut response) = handle_response(&mut requests, is_incomplete).await {
        let handle = handle.clone();
        dispatch(move |editor, compositor| {
            let editor_view = compositor.find::<ui::EditorView>().unwrap();
            let Some(completion) = &mut editor_view.completion else {
                return;
            };
            if handle.is_canceled() {
                log::info!("dropping outdated completion response");
                return;
            }

            completion.replace_provider_completions(&mut response, is_incomplete);
            if completion.is_empty() {
                editor_view.clear_completion(editor);
                // clearing completions might mean we want to immediately re-request them (usually
                // this occurs if typing a trigger char)
                trigger_auto_completion(editor, false);
            } else {
                editor
                    .handlers
                    .completions
                    .active_completions
                    .insert(response.provider, response.context);
            }
        })
        .await;
    }
}

fn show_completion(
    editor: &mut Editor,
    compositor: &mut Compositor,
    mut items: Vec<CompletionItem>,
    context: HashMap<CompletionProvider, ResponseContext>,
    trigger: Trigger,
) {
    let (view, doc) = current_ref!(editor);
    // check if the completion request is stale.
    //
    // Completions are completed asynchronously and therefore the user could
    //switch document/view or leave insert mode. In all of thoise cases the
    // completion should be discarded
    if editor.mode != Mode::Insert || view.id != trigger.view || doc.id() != trigger.doc {
        return;
    }

    let size = compositor.size();
    let ui = compositor.find::<ui::EditorView>().unwrap();
    if ui.completion.is_some() {
        return;
    }
    word::retain_valid_completions(trigger, doc, view.id, &mut items);
    editor.handlers.completions.active_completions = context;

    let completion_area = ui.set_completion(editor, items, trigger.pos, size);
    let signature_help_area = compositor
        .find_id::<Popup<SignatureHelp>>(SignatureHelp::ID)
        .map(|signature_help| signature_help.area(size, editor));
    // Delete the signature help popup if they intersect.
    if matches!((completion_area, signature_help_area),(Some(a), Some(b)) if a.intersects(b)) {
        compositor.remove(SignatureHelp::ID);
    }
}

pub fn trigger_auto_completion(editor: &Editor, trigger_char_only: bool) {
    let config = editor.config.load();
    if !config.auto_completion {
        return;
    }
    let (view, doc): (&helix_view::View, &helix_view::Document) = current_ref!(editor);
    let mut text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(text);
    text = doc.text().slice(..cursor);

    let is_trigger_char = doc
        .language_servers_with_feature(LanguageServerFeature::Completion)
        .any(|ls| {
            matches!(&ls.capabilities().completion_provider, Some(lsp::CompletionOptions {
                        trigger_characters: Some(triggers),
                        ..
                    }) if triggers.iter().any(|trigger| text.ends_with(trigger)))
        });

    let cursor_char = text
        .get_bytes_at(text.len_bytes())
        .and_then(|t| t.reversed().next());

    #[cfg(windows)]
    let is_path_completion_trigger = matches!(cursor_char, Some(b'/' | b'\\'));
    #[cfg(not(windows))]
    let is_path_completion_trigger = matches!(cursor_char, Some(b'/'));

    let handler = &editor.handlers.completions;
    if is_trigger_char || (is_path_completion_trigger && doc.path_completion_enabled()) {
        handler.event(CompletionEvent::TriggerChar {
            cursor,
            doc: doc.id(),
            view: view.id,
        });
        return;
    }

    let is_auto_trigger = !trigger_char_only
        && doc
            .text()
            .chars_at(cursor)
            .reversed()
            .take(config.completion_trigger_len as usize)
            .all(char_is_word);

    if is_auto_trigger {
        handler.event(CompletionEvent::AutoTrigger {
            cursor,
            doc: doc.id(),
            view: view.id,
        });
    }
}

fn update_completion_filter(cx: &mut commands::Context, c: Option<char>) {
    cx.callback.push(Box::new(move |compositor, cx| {
        let editor_view = compositor.find::<ui::EditorView>().unwrap();
        if let Some(completion) = &mut editor_view.completion {
            completion.update_filter(c);
            if completion.is_empty() || c.is_some_and(|c| !char_is_word(c)) {
                editor_view.clear_completion(cx.editor);
                // clearing completions might mean we want to immediately rerequest them (usually
                // this occurs if typing a trigger char)
                if c.is_some() {
                    trigger_auto_completion(cx.editor, false);
                }
            } else {
                let handle = cx.editor.handlers.completions.request_controller.restart();
                request_incomplete_completion_list(cx.editor, handle)
            }
        }
    }))
}

fn clear_completions(cx: &mut commands::Context) {
    cx.callback.push(Box::new(|compositor, cx| {
        let editor_view = compositor.find::<ui::EditorView>().unwrap();
        editor_view.clear_completion(cx.editor);
    }))
}

fn completion_post_command_hook(
    PostCommand { command, cx }: &mut PostCommand<'_, '_>,
) -> anyhow::Result<()> {
    if cx.editor.mode == Mode::Insert {
        if cx.editor.last_completion.is_some() {
            match command {
                MappableCommand::Static {
                    name: "delete_word_forward" | "delete_char_forward" | "completion",
                    ..
                } => (),
                MappableCommand::Static {
                    name: "delete_char_backward",
                    ..
                } => update_completion_filter(cx, None),
                _ => clear_completions(cx),
            }
        } else {
            let event = match command {
                MappableCommand::Static {
                    name: "delete_char_backward" | "delete_word_forward" | "delete_char_forward",
                    ..
                } => {
                    let (view, doc) = current!(cx.editor);
                    let primary_cursor = doc
                        .selection(view.id)
                        .primary()
                        .cursor(doc.text().slice(..));
                    CompletionEvent::DeleteText {
                        cursor: primary_cursor,
                    }
                }
                // hacks: some commands are handeled elsewhere and we don't want to
                // cancel in that case
                MappableCommand::Static {
                    name: "completion" | "insert_mode" | "append_mode",
                    ..
                } => return Ok(()),
                _ => CompletionEvent::Cancel,
            };
            cx.editor.handlers.completions.event(event);
        }
    }
    Ok(())
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut PostCommand<'_, '_>| completion_post_command_hook(event));

    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.old_mode == Mode::Insert {
            event
                .cx
                .editor
                .handlers
                .completions
                .event(CompletionEvent::Cancel);
            clear_completions(event.cx);
        } else if event.new_mode == Mode::Insert {
            trigger_auto_completion(event.cx.editor, false)
        }
        Ok(())
    });

    register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
        if event.cx.editor.last_completion.is_some() {
            update_completion_filter(event.cx, Some(event.c))
        } else {
            trigger_auto_completion(event.cx.editor, false);
        }
        Ok(())
    });
}
