use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use futures_util::Future;
use helix_core::completion::CompletionProvider;
use helix_core::syntax::config::LanguageServerFeature;
use helix_event::{cancelable_future, TaskController, TaskHandle};
use helix_lsp::lsp;
use helix_lsp::lsp::{CompletionContext, CompletionTriggerKind};
use helix_lsp::util::pos_to_lsp_pos;
use helix_stdx::rope::RopeSliceExt;
use helix_view::document::{Mode, SavePoint};
use helix_view::handlers::completion::{CompletionEvent, ResponseContext};
use helix_view::{Document, DocumentId, Editor, ViewId};
use tokio::task::JoinSet;
use tokio::time::{timeout_at, Instant};

use crate::compositor::Compositor;
use crate::config::Config;
use crate::handlers::completion::item::CompletionResponse;
use crate::handlers::completion::path::path_completion;
use crate::handlers::completion::{
    handle_response, replace_completions, show_completion, CompletionItems,
};
use crate::job::{dispatch, dispatch_blocking};
use crate::ui;
use crate::ui::editor::InsertEvent;

use super::word;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum TriggerKind {
    Auto,
    TriggerChar,
    Manual,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Trigger {
    pub(super) pos: usize,
    pub(super) view: ViewId,
    pub(super) doc: DocumentId,
    pub(super) kind: TriggerKind,
}

#[derive(Debug)]
pub struct CompletionHandler {
    /// The currently active trigger which will cause a completion request after the timeout.
    trigger: Option<Trigger>,
    in_flight: Option<Trigger>,
    task_controller: TaskController,
    config: Arc<ArcSwap<Config>>,
}

impl CompletionHandler {
    pub fn new(config: Arc<ArcSwap<Config>>) -> CompletionHandler {
        Self {
            config,
            task_controller: TaskController::new(),
            trigger: None,
            in_flight: None,
        }
    }
}

impl helix_event::AsyncHook for CompletionHandler {
    type Event = CompletionEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _old_timeout: Option<Instant>,
    ) -> Option<Instant> {
        if self.in_flight.is_some() && !self.task_controller.is_running() {
            self.in_flight = None;
        }
        match event {
            CompletionEvent::AutoTrigger {
                cursor: trigger_pos,
                doc,
                view,
            } => {
                // Technically it shouldn't be possible to switch views/documents in insert mode
                // but people may create weird keymaps/use the mouse so let's be extra careful.
                if self
                    .trigger
                    .or(self.in_flight)
                    .map_or(true, |trigger| trigger.doc != doc || trigger.view != view)
                {
                    self.trigger = Some(Trigger {
                        pos: trigger_pos,
                        view,
                        doc,
                        kind: TriggerKind::Auto,
                    });
                }
            }
            CompletionEvent::TriggerChar { cursor, doc, view } => {
                // immediately request completions and drop all auto completion requests
                self.task_controller.cancel();
                self.trigger = Some(Trigger {
                    pos: cursor,
                    view,
                    doc,
                    kind: TriggerKind::TriggerChar,
                });
            }
            CompletionEvent::ManualTrigger { cursor, doc, view } => {
                // immediately request completions and drop all auto completion requests
                self.trigger = Some(Trigger {
                    pos: cursor,
                    view,
                    doc,
                    kind: TriggerKind::Manual,
                });
                // stop debouncing immediately and request the completion
                self.finish_debounce();
                return None;
            }
            CompletionEvent::Cancel => {
                self.trigger = None;
                self.task_controller.cancel();
            }
            CompletionEvent::DeleteText { cursor } => {
                // if we deleted the original trigger, abort the completion
                if matches!(self.trigger.or(self.in_flight), Some(Trigger{ pos, .. }) if cursor < pos)
                {
                    self.trigger = None;
                    self.task_controller.cancel();
                }
            }
        }
        self.trigger.map(|trigger| {
            // if the current request was closed forget about it
            // otherwise immediately restart the completion request
            let timeout = if trigger.kind == TriggerKind::Auto {
                self.config.load().editor.completion_timeout
            } else {
                // we want almost instant completions for trigger chars
                // and restarting completion requests. The small timeout here mainly
                // serves to better handle cases where the completion handler
                // may fall behind (so multiple events in the channel) and macros
                Duration::from_millis(5)
            };
            Instant::now() + timeout
        })
    }

    fn finish_debounce(&mut self) {
        let trigger = self.trigger.take().expect("debounce always has a trigger");
        self.in_flight = Some(trigger);
        let handle = self.task_controller.restart();
        dispatch_blocking(move |editor, compositor| {
            request_completions(trigger, handle, editor, compositor)
        });
    }
}

fn request_completions(
    mut trigger: Trigger,
    handle: TaskHandle,
    editor: &mut Editor,
    compositor: &mut Compositor,
) {
    let (view, doc) = current_ref!(editor);

    if compositor
        .find::<ui::EditorView>()
        .unwrap()
        .completion
        .is_some()
        || editor.mode != Mode::Insert
    {
        return;
    }

    let text = doc.text();
    let cursor = doc.selection(view.id).primary().cursor(text.slice(..));
    if trigger.view != view.id || trigger.doc != doc.id() || cursor < trigger.pos {
        return;
    }
    // This looks odd... Why are we not using the trigger position from the `trigger` here? Won't
    // that mean that the trigger char doesn't get send to the language server if we type fast
    // enough? Yes that is true but it's not actually a problem. The language server will resolve
    // the completion to the identifier anyway (in fact sending the later position is necessary to
    // get the right results from language servers that provide incomplete completion list). We
    // rely on the trigger offset and primary cursor matching for multi-cursor completions so this
    // is definitely necessary from our side too.
    trigger.pos = cursor;
    let doc = doc_mut!(editor, &doc.id());
    let savepoint = doc.savepoint(view);
    let text = doc.text();
    let trigger_text = text.slice(..cursor);

    let mut seen_language_servers = HashSet::new();
    let language_servers: Vec<_> = doc
        .language_servers_with_feature(LanguageServerFeature::Completion)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .collect();
    let mut requests = JoinSet::new();
    for (priority, ls) in language_servers.iter().enumerate() {
        let context = if trigger.kind == TriggerKind::Manual {
            lsp::CompletionContext {
                trigger_kind: lsp::CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }
        } else {
            let trigger_char =
                ls.capabilities()
                    .completion_provider
                    .as_ref()
                    .and_then(|provider| {
                        provider
                            .trigger_characters
                            .as_deref()?
                            .iter()
                            .find(|&trigger| trigger_text.ends_with(trigger))
                    });

            if trigger_char.is_some() {
                lsp::CompletionContext {
                    trigger_kind: lsp::CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: trigger_char.cloned(),
                }
            } else {
                lsp::CompletionContext {
                    trigger_kind: lsp::CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }
            }
        };
        requests.spawn(request_completions_from_language_server(
            ls,
            doc,
            view.id,
            context,
            -(priority as i8),
            savepoint.clone(),
        ));
    }
    if let Some(path_completion_request) = path_completion(
        doc.selection(view.id).clone(),
        doc,
        handle.clone(),
        savepoint.clone(),
    ) {
        requests.spawn_blocking(path_completion_request);
    }
    if let Some(word_completion_request) =
        word::completion(editor, trigger, handle.clone(), savepoint)
    {
        requests.spawn_blocking(word_completion_request);
    }

    let ui = compositor.find::<ui::EditorView>().unwrap();
    ui.last_insert.1.push(InsertEvent::RequestCompletion);
    let handle_ = handle.clone();
    let request_completions = async move {
        let mut context = HashMap::new();
        let Some(mut response) = handle_response(&mut requests, false).await else {
            return;
        };

        let mut items: Vec<_> = Vec::new();
        response.take_items(&mut items);
        context.insert(response.provider, response.context);
        let deadline = Instant::now() + Duration::from_millis(100);
        loop {
            let Some(mut response) = timeout_at(deadline, handle_response(&mut requests, false))
                .await
                .ok()
                .flatten()
            else {
                break;
            };
            response.take_items(&mut items);
            context.insert(response.provider, response.context);
        }
        dispatch(move |editor, compositor| {
            show_completion(editor, compositor, items, context, trigger)
        })
        .await;
        if !requests.is_empty() {
            replace_completions(handle_, requests, false).await;
        }
    };
    tokio::spawn(cancelable_future(request_completions, handle));
}

fn request_completions_from_language_server(
    ls: &helix_lsp::Client,
    doc: &Document,
    view: ViewId,
    context: lsp::CompletionContext,
    priority: i8,
    savepoint: Arc<SavePoint>,
) -> impl Future<Output = CompletionResponse> {
    let provider = ls.id();
    let offset_encoding = ls.offset_encoding();
    let text = doc.text();
    let cursor = doc.selection(view).primary().cursor(text.slice(..));
    let pos = pos_to_lsp_pos(text, cursor, offset_encoding);
    let doc_id = doc.identifier();

    // it's important that this is before the async block (and that this is not an async function)
    // to ensure the request is dispatched right away before any new edit notifications
    let completion_response = ls.completion(doc_id, pos, None, context).unwrap();
    async move {
        let response: Option<lsp::CompletionResponse> = completion_response
            .await
            .inspect_err(|err| log::error!("completion request failed: {err}"))
            .ok()
            .flatten();
        let (mut items, is_incomplete) = match response {
            Some(lsp::CompletionResponse::Array(items)) => (items, false),
            Some(lsp::CompletionResponse::List(lsp::CompletionList {
                is_incomplete,
                items,
            })) => (items, is_incomplete),
            None => (Vec::new(), false),
        };
        items.sort_by(|item1, item2| {
            let sort_text1 = item1.sort_text.as_deref().unwrap_or(&item1.label);
            let sort_text2 = item2.sort_text.as_deref().unwrap_or(&item2.label);
            sort_text1.cmp(sort_text2)
        });
        CompletionResponse {
            items: CompletionItems::Lsp(items),
            context: ResponseContext {
                is_incomplete,
                priority,
                savepoint,
            },
            provider: CompletionProvider::Lsp(provider),
        }
    }
}

pub fn request_incomplete_completion_list(editor: &mut Editor, handle: TaskHandle) {
    let handler = &mut editor.handlers.completions;
    let mut requests = JoinSet::new();
    let mut savepoint = None;
    for (&provider, context) in &handler.active_completions {
        if !context.is_incomplete {
            continue;
        }
        let CompletionProvider::Lsp(ls_id) = provider else {
            log::error!("non-lsp incomplete completion lists");
            continue;
        };
        let Some(ls) = editor.language_servers.get_by_id(ls_id) else {
            continue;
        };
        let (view, doc) = current!(editor);
        let savepoint = savepoint.get_or_insert_with(|| doc.savepoint(view)).clone();
        let request = request_completions_from_language_server(
            ls,
            doc,
            view.id,
            CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
                trigger_character: None,
            },
            context.priority,
            savepoint,
        );
        requests.spawn(request);
    }
    if !requests.is_empty() {
        tokio::spawn(replace_completions(handle, requests, true));
    }
}
