use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::FutureExt;
use helix_core::chars::char_is_word;
use helix_core::syntax::LanguageServerFeature;
use helix_core::Transaction;
use helix_event::{
    cancelable_future, cancelation, register_hook, send_blocking, CancelRx, CancelTx,
};
use helix_lsp::lsp;
use helix_lsp::util::pos_to_lsp_pos;
use helix_stdx::rope::{self, RopeSliceExt};
use helix_view::document::{Mode, SavePoint};
use helix_view::handlers::lsp::CompletionEvent;
use helix_view::{DocumentId, Editor, ViewId};
use once_cell::sync::Lazy;
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;
use tokio_stream::StreamExt;

use crate::commands;
use crate::compositor::Compositor;
use crate::config::Config;
use crate::events::{OnModeSwitch, PostCommand, PostInsertChar};
use crate::job::{dispatch, dispatch_blocking};
use crate::keymap::MappableCommand;
use crate::ui::editor::InsertEvent;
use crate::ui::lsp::SignatureHelp;
use crate::ui::{self, CompletionItem, LspCompletionItem, PathCompletionItem, PathKind, Popup};

use super::Handlers;
pub use resolve::ResolveHandler;
mod resolve;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TriggerKind {
    Auto,
    TriggerChar,
    Manual,
}

#[derive(Debug, Clone, Copy)]
struct Trigger {
    pos: usize,
    view: ViewId,
    doc: DocumentId,
    kind: TriggerKind,
}

#[derive(Debug)]
pub(super) struct CompletionHandler {
    /// currently active trigger which will cause a
    /// completion request after the timeout
    trigger: Option<Trigger>,
    /// A handle for currently active completion request.
    /// This can be used to determine whether the current
    /// request is still active (and new triggers should be
    /// ignored) and can also be used to abort the current
    /// request (by dropping the handle)
    request: Option<CancelTx>,
    config: Arc<ArcSwap<Config>>,
}

impl CompletionHandler {
    pub fn new(config: Arc<ArcSwap<Config>>) -> CompletionHandler {
        Self {
            config,
            request: None,
            trigger: None,
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
        match event {
            CompletionEvent::AutoTrigger {
                cursor: trigger_pos,
                doc,
                view,
            } => {
                // techically it shouldn't be possible to switch views/documents in insert mode
                // but people may create weird keymaps/use the mouse so lets be extra careful
                if self
                    .trigger
                    .as_ref()
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
                self.request = None;
                self.trigger = Some(Trigger {
                    pos: cursor,
                    view,
                    doc,
                    kind: TriggerKind::TriggerChar,
                });
            }
            CompletionEvent::ManualTrigger { cursor, doc, view } => {
                // immediately request completions and drop all auto completion requests
                self.request = None;
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
                self.request = None;
            }
            CompletionEvent::DeleteText { cursor } => {
                // if we deleted the original trigger, abort the completion
                if matches!(self.trigger, Some(Trigger{ pos, .. }) if cursor < pos) {
                    self.trigger = None;
                    self.request = None;
                }
            }
        }
        self.trigger.map(|trigger| {
            // if the current request was closed forget about it
            // otherwise immediately restart the completion request
            let cancel = self.request.take().map_or(false, |req| !req.is_closed());
            let timeout = if trigger.kind == TriggerKind::Auto && !cancel {
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
        let (tx, rx) = cancelation();
        self.request = Some(tx);
        dispatch_blocking(move |editor, compositor| {
            request_completion(trigger, rx, editor, compositor)
        });
    }
}

fn request_completion(
    mut trigger: Trigger,
    cancel: CancelRx,
    editor: &mut Editor,
    compositor: &mut Compositor,
) {
    let (view, doc) = current!(editor);

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
    // this looks odd... Why are we not using the trigger position from
    // the `trigger` here? Won't that mean that the trigger char doesn't get
    // send to the LS if we type fast enougn? Yes that is true but it's
    // not actually a problem. The LSP will resolve the completion to the identifier
    // anyway (in fact sending the later position is necessary to get the right results
    // from LSPs that provide incomplete completion list). We rely on trigger offset
    // and primary cursor matching for multi-cursor completions so this is definitely
    // necessary from our side too.
    trigger.pos = cursor;
    let trigger_text = text.slice(..cursor);

    let mut seen_language_servers = HashSet::new();
    let mut futures: FuturesUnordered<_> = doc
        .language_servers_with_feature(LanguageServerFeature::Completion)
        .filter(|ls| seen_language_servers.insert(ls.id()))
        .map(|ls| {
            let language_server_id = ls.id();
            let offset_encoding = ls.offset_encoding();
            let pos = pos_to_lsp_pos(text, cursor, offset_encoding);
            let doc_id = doc.identifier();
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

            let completion_response = ls.completion(doc_id, pos, None, context).unwrap();
            async move {
                let json = completion_response.await?;
                let response: Option<lsp::CompletionResponse> = serde_json::from_value(json)?;
                let items = match response {
                    Some(lsp::CompletionResponse::Array(items)) => items,
                    // TODO: do something with is_incomplete
                    Some(lsp::CompletionResponse::List(lsp::CompletionList {
                        is_incomplete: _is_incomplete,
                        items,
                    })) => items,
                    None => Vec::new(),
                }
                .into_iter()
                .map(|item| {
                    CompletionItem::Lsp(LspCompletionItem {
                        item,
                        provider: language_server_id,
                        resolved: false,
                    })
                })
                .collect();
                anyhow::Ok(items)
            }
            .boxed()
        })
        .chain(path_completion(cursor, text.clone(), doc))
        .collect();

    let future = async move {
        let mut items = Vec::new();
        while let Some(lsp_items) = futures.next().await {
            match lsp_items {
                Ok(mut lsp_items) => items.append(&mut lsp_items),
                Err(err) => {
                    log::debug!("completion request failed: {err:?}");
                }
            };
        }
        items
    };

    let savepoint = doc.savepoint(view);

    let ui = compositor.find::<ui::EditorView>().unwrap();
    ui.last_insert.1.push(InsertEvent::RequestCompletion);
    tokio::spawn(async move {
        let items = cancelable_future(future, cancel).await.unwrap_or_default();
        if items.is_empty() {
            return;
        }
        dispatch(move |editor, compositor| {
            show_completion(editor, compositor, items, trigger, savepoint)
        })
        .await
    });
}

fn path_completion(
    cursor: usize,
    text: helix_core::Rope,
    doc: &helix_view::Document,
) -> Option<BoxFuture<'static, anyhow::Result<Vec<CompletionItem>>>> {
    if !doc.supports_path_completion() {
        return None;
    }

    // TODO find a good regex for most use cases (especially Windows, which is not yet covered...)
    // currently only one path match per line is possible in unix
    static PATH_REGEX: Lazy<rope::Regex> =
        Lazy::new(|| rope::Regex::new(r"((?:~|\$HOME|\$\{HOME\})?(?:\.{0,2}/)+.*)").unwrap());

    let cur_line = text.char_to_line(cursor);
    let start = text.line_to_char(cur_line).max(cursor.saturating_sub(1000));
    let line_until_cursor = text.slice(..).regex_input_at(start..cursor);

    let (dir_path, typed_file_name) = PATH_REGEX
        .find_iter(line_until_cursor)
        .last() // this is a workaround for missing "match end of slice" support in `rope::Regex`
        .and_then(|matched_path| {
            let end = text.byte_to_char(matched_path.end());
            if end != cursor {
                return None;
            }
            let start = text.byte_to_char(matched_path.start());
            let matched_path = &text.slice(start..end).to_string();

            // resolve home dir (~/, $HOME/, ${HOME}/) on unix
            #[cfg(unix)]
            let mut path = {
                static HOME_DIR: Lazy<Option<std::ffi::OsString>> =
                    Lazy::new(|| std::env::var_os("HOME"));

                let home_resolved_path = if let Some(home) = &*HOME_DIR {
                    let first_separator_after_home = if matched_path.starts_with("~/") {
                        Some(1)
                    } else if matched_path.starts_with("$HOME") {
                        Some(5)
                    } else if matched_path.starts_with("${HOME}") {
                        Some(7)
                    } else {
                        None
                    };
                    if let Some(start_char) = first_separator_after_home {
                        let mut path = home.to_owned();
                        path.push(&matched_path[start_char..]);
                        path
                    } else {
                        matched_path.into()
                    }
                } else {
                    matched_path.into()
                };
                PathBuf::from(home_resolved_path)
            };
            #[cfg(not(unix))]
            let mut path = PathBuf::from(matched_path);

            if path.is_relative() {
                if let Some(doc_path) = doc.path().and_then(|dp| dp.parent()) {
                    path = doc_path.join(path);
                } else if let Ok(work_dir) = std::env::current_dir() {
                    path = work_dir.join(path);
                }
            }
            let ends_with_slash = match matched_path.chars().last() {
                Some(std::path::MAIN_SEPARATOR) => true,
                None => return None,
                _ => false,
            };
            // check if there are chars after the last slash, and if these chars represent a directory
            match std::fs::metadata(path.clone()).ok() {
                Some(m) if m.is_dir() && ends_with_slash => {
                    Some((PathBuf::from(path.as_path()), None))
                }
                _ if !ends_with_slash => path.parent().map(|parent_path| {
                    (
                        PathBuf::from(parent_path),
                        path.file_name().and_then(|f| f.to_str().map(String::from)),
                    )
                }),
                _ => None,
            }
        })?;

    // The async file accessor functions of tokio were considered, but they were a bit slower
    // and less ergonomic than just using the std functions in a separate "thread"
    let future = tokio::task::spawn_blocking(move || {
        let Some(read_dir) = std::fs::read_dir(&dir_path).ok() else {
            return Vec::new();
        };

        read_dir
            .filter_map(Result::ok)
            .filter_map(|dir_entry| dir_entry.metadata().ok().map(|md| (dir_entry.path(), md)))
            .map(|(path, md)| {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let full_path = path.canonicalize().unwrap_or_default();
                let full_path_name = full_path.to_string_lossy().into_owned();

                let is_dir = full_path.is_dir();

                let kind = if md.is_symlink() {
                    PathKind::Link
                } else if is_dir {
                    PathKind::Folder
                } else {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::FileTypeExt;
                        if md.file_type().is_block_device() {
                            PathKind::Block
                        } else if md.file_type().is_socket() {
                            PathKind::Socket
                        } else if md.file_type().is_char_device() {
                            PathKind::CharacterDevice
                        } else if md.file_type().is_fifo() {
                            PathKind::Fifo
                        } else {
                            PathKind::File
                        }
                    }
                    #[cfg(not(unix))]
                    PathKind::File
                };

                let resolved = if kind == PathKind::Link {
                    "resolved "
                } else {
                    ""
                };

                let documentation = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::prelude::PermissionsExt;
                        let mode = md.permissions().mode();

                        let perms = [
                            (libc::S_IRUSR, 'r'),
                            (libc::S_IWUSR, 'w'),
                            (libc::S_IXUSR, 'x'),
                            (libc::S_IRGRP, 'r'),
                            (libc::S_IWGRP, 'w'),
                            (libc::S_IXGRP, 'x'),
                            (libc::S_IROTH, 'r'),
                            (libc::S_IWOTH, 'w'),
                            (libc::S_IXOTH, 'x'),
                        ]
                        .into_iter()
                        .fold(
                            String::with_capacity(9),
                            |mut acc, (p, s)| {
                                // This cast is necessary on some platforms such as macos as `mode_t` is u16 there
                                #[allow(clippy::unnecessary_cast)]
                                acc.push(if mode & (p as u32) > 0 { s } else { '-' });
                                acc
                            },
                        );

                        // TODO it would be great to be able to individually color the documentation,
                        // but this will likely require a custom doc implementation (i.e. not `lsp::Documentation`)
                        // and/or different rendering in completion.rs
                        format!(
                            "type: `{kind}`\n\
                             permissions: `[{perms}]`\n\
                             {resolved}full path: `{full_path_name}`",
                        )
                    }
                    #[cfg(not(unix))]
                    {
                        format!(
                            "type: `{kind}`\n\
                             {resolved}full path: `{full_path_name}`",
                        )
                    }
                };

                let edit_diff = typed_file_name.as_ref().map(|f| f.len()).unwrap_or(0);

                let transaction = Transaction::change(
                    &text,
                    std::iter::once((cursor - edit_diff, cursor, Some(file_name.as_str().into()))),
                );

                CompletionItem::Path(PathCompletionItem {
                    kind,
                    item: helix_core::CompletionItem {
                        transaction,
                        label: file_name,
                        documentation,
                    },
                })
            })
            .collect::<Vec<_>>()
    });

    Some(async move { Ok(future.await?) }.boxed())
}

fn show_completion(
    editor: &mut Editor,
    compositor: &mut Compositor,
    items: Vec<CompletionItem>,
    trigger: Trigger,
    savepoint: Arc<SavePoint>,
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

    let completion_area = ui.set_completion(editor, savepoint, items, trigger.pos, size);
    let signature_help_area = compositor
        .find_id::<Popup<SignatureHelp>>(SignatureHelp::ID)
        .map(|signature_help| signature_help.area(size, editor));
    // Delete the signature help popup if they intersect.
    if matches!((completion_area, signature_help_area),(Some(a), Some(b)) if a.intersects(b)) {
        compositor.remove(SignatureHelp::ID);
    }
}

pub fn trigger_auto_completion(
    tx: &Sender<CompletionEvent>,
    editor: &Editor,
    trigger_char_only: bool,
) {
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

    let trigger_path_completion =
        text.ends_with(std::path::MAIN_SEPARATOR_STR) && doc.supports_path_completion();

    if is_trigger_char || trigger_path_completion {
        send_blocking(
            tx,
            CompletionEvent::TriggerChar {
                cursor,
                doc: doc.id(),
                view: view.id,
            },
        );
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
        send_blocking(
            tx,
            CompletionEvent::AutoTrigger {
                cursor,
                doc: doc.id(),
                view: view.id,
            },
        );
    }
}

fn update_completions(cx: &mut commands::Context, c: Option<char>) {
    cx.callback.push(Box::new(move |compositor, cx| {
        let editor_view = compositor.find::<ui::EditorView>().unwrap();
        if let Some(completion) = &mut editor_view.completion {
            completion.update_filter(c);
            if completion.is_empty() {
                editor_view.clear_completion(cx.editor);
                // clearing completions might mean we want to immediately rerequest them (usually
                // this occurs if typing a trigger char)
                if c.is_some() {
                    trigger_auto_completion(&cx.editor.handlers.completions, cx.editor, false);
                }
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
    tx: &Sender<CompletionEvent>,
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
                } => update_completions(cx, None),
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
            send_blocking(tx, event);
        }
    }
    Ok(())
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.completions.clone();
    register_hook!(move |event: &mut PostCommand<'_, '_>| completion_post_command_hook(&tx, event));

    let tx = handlers.completions.clone();
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.old_mode == Mode::Insert {
            send_blocking(&tx, CompletionEvent::Cancel);
            clear_completions(event.cx);
        } else if event.new_mode == Mode::Insert {
            trigger_auto_completion(&tx, event.cx.editor, false)
        }
        Ok(())
    });

    let tx = handlers.completions.clone();
    register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
        if event.cx.editor.last_completion.is_some() {
            update_completions(event.cx, Some(event.c))
        } else {
            trigger_auto_completion(&tx, event.cx.editor, false);
        }
        Ok(())
    });
}
