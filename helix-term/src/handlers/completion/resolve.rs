use std::sync::Arc;

use helix_lsp::lsp;
use tokio::sync::mpsc::Sender;
use tokio::time::{Duration, Instant};

use helix_event::{send_blocking, AsyncHook, TaskController, TaskHandle};
use helix_view::Editor;

use super::LspCompletionItem;
use crate::handlers::completion::CompletionItem;
use crate::job;

/// A hook for resolving incomplete completion items.
///
/// From the [LSP spec](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion):
///
/// > If computing full completion items is expensive, servers can additionally provide a
/// > handler for the completion item resolve request. ...
/// > A typical use case is for example: the `textDocument/completion` request doesn't fill
/// > in the `documentation` property for returned completion items since it is expensive
/// > to compute. When the item is selected in the user interface then a
/// > 'completionItem/resolve' request is sent with the selected completion item as a parameter.
/// > The returned completion item should have the documentation property filled in.
pub struct ResolveHandler {
    last_request: Option<Arc<LspCompletionItem>>,
    resolver: Sender<ResolveRequest>,
}

impl ResolveHandler {
    pub fn new() -> ResolveHandler {
        ResolveHandler {
            last_request: None,
            resolver: ResolveTimeout::default().spawn(),
        }
    }

    pub fn ensure_item_resolved(&mut self, editor: &mut Editor, item: &mut LspCompletionItem) {
        if item.resolved {
            return;
        }
        // We consider an item to be fully resolved if it has non-empty, none-`None` details,
        // docs and additional text-edits. Ideally we could use `is_some` instead of this
        // check but some language servers send values like `Some([])` for additional text
        // edits although the items need to be resolved. This is probably a consequence of
        // how `null` works in the JavaScript world.
        let is_resolved = item
            .item
            .documentation
            .as_ref()
            .is_some_and(|docs| match docs {
                lsp::Documentation::String(text) => !text.is_empty(),
                lsp::Documentation::MarkupContent(markup) => !markup.value.is_empty(),
            })
            && item
                .item
                .detail
                .as_ref()
                .is_some_and(|detail| !detail.is_empty())
            && item
                .item
                .additional_text_edits
                .as_ref()
                .is_some_and(|edits| !edits.is_empty());
        if is_resolved {
            item.resolved = true;
            return;
        }
        if self.last_request.as_deref().is_some_and(|it| it == item) {
            return;
        }
        let Some(ls) = editor.language_servers.get_by_id(item.provider).cloned() else {
            item.resolved = true;
            return;
        };
        if matches!(
            ls.capabilities().completion_provider,
            Some(lsp::CompletionOptions {
                resolve_provider: Some(true),
                ..
            })
        ) {
            let item = Arc::new(item.clone());
            self.last_request = Some(item.clone());
            send_blocking(&self.resolver, ResolveRequest { item, ls })
        } else {
            item.resolved = true;
        }
    }
}

struct ResolveRequest {
    item: Arc<LspCompletionItem>,
    ls: Arc<helix_lsp::Client>,
}

#[derive(Default)]
struct ResolveTimeout {
    next_request: Option<ResolveRequest>,
    in_flight: Option<Arc<LspCompletionItem>>,
    task_controller: TaskController,
}

impl AsyncHook for ResolveTimeout {
    type Event = ResolveRequest;

    fn handle_event(
        &mut self,
        request: Self::Event,
        timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        if self
            .next_request
            .as_ref()
            .is_some_and(|old_request| old_request.item == request.item)
        {
            timeout
        } else if self
            .in_flight
            .as_ref()
            .is_some_and(|old_request| old_request.item == request.item.item)
        {
            self.next_request = None;
            None
        } else {
            self.next_request = Some(request);
            Some(Instant::now() + Duration::from_millis(150))
        }
    }

    fn finish_debounce(&mut self) {
        let Some(request) = self.next_request.take() else {
            return;
        };
        let token = self.task_controller.restart();
        self.in_flight = Some(request.item.clone());
        tokio::spawn(request.execute(token));
    }
}

impl ResolveRequest {
    async fn execute(self, cancel: TaskHandle) {
        let future = self.ls.resolve_completion_item(&self.item.item);
        let Some(resolved_item) = helix_event::cancelable_future(future, cancel).await else {
            return;
        };
        job::dispatch(move |_, compositor| {
            if let Some(completion) = &mut compositor
                .find::<crate::ui::EditorView>()
                .unwrap()
                .completion
            {
                let resolved_item = CompletionItem::Lsp(match resolved_item {
                    Ok(item) => LspCompletionItem {
                        item,
                        resolved: true,
                        ..*self.item
                    },
                    Err(err) => {
                        log::error!("completion resolve request failed: {err}");
                        // set item to resolved so we don't request it again
                        // we could also remove it but that oculd be odd ui
                        let mut item = (*self.item).clone();
                        item.resolved = true;
                        item
                    }
                });
                completion.replace_item(&*self.item, resolved_item);
            };
        })
        .await
    }
}
