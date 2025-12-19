use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use helix_core::file_watcher::{events_from_paths, EventType, FileSystemDidChange};
use helix_event::{dispatch, register_hook, send_blocking};
use helix_view::editor::Config;
use helix_view::events::ConfigDidChange;
use helix_view::handlers::{AutoReloadEvent, Handlers};
use helix_view::{DocumentId, Editor};
use tokio::time::Instant;

use crate::compositor::Compositor;
use crate::ui::{Prompt, PromptEvent};
use crate::{job, ui};

/// Handler for FileSystemDidChange events (from filesentry or polling)
struct ReloadHandler {
    enable: AtomicBool,
    prompt_if_modified: AtomicBool,
}

impl ReloadHandler {
    pub fn refresh_config(&self, config: &Config) {
        self.enable
            .store(config.auto_reload.enable, atomic::Ordering::Relaxed);
        self.prompt_if_modified.store(
            config.auto_reload.prompt_if_modified,
            atomic::Ordering::Relaxed,
        );
    }

    fn on_file_did_change(&self, event: &mut FileSystemDidChange) {
        if !self.enable.load(atomic::Ordering::Relaxed) {
            return;
        }
        let fs_events = event.fs_events.clone();
        if !fs_events
            .iter()
            .any(|event| event.ty == EventType::Modified)
        {
            return;
        }
        let prompt_if_modified = self.prompt_if_modified.load(atomic::Ordering::Relaxed);
        job::dispatch_blocking(move |editor, compositor| {
            let mut vcs_reload = false;

            for fs_event in &*fs_events {
                if fs_event.ty != EventType::Modified {
                    continue;
                }
                vcs_reload |= editor.diff_providers.needs_reload(fs_event);

                let Some(doc_id) = editor.document_id_by_path(fs_event.path.as_std_path()) else {
                    continue;
                };

                handle_document_change(editor, compositor, doc_id, prompt_if_modified);
            }

            if vcs_reload {
                reload_vcs_diffs(editor);
            }
        });
    }
}

/// Handler for polling-based change detection for unwatched files.
/// Polls documents not covered by the file watcher (e.g., outside workspace or in ignored dirs).
/// Co-Authored-By: Anthony Rubick <68485672+AnthonyMichaelTDM@users.noreply.github.com>
#[derive(Debug)]
pub(super) struct PollHandler;

impl PollHandler {
    pub fn new() -> Self {
        PollHandler
    }
}

impl helix_event::AsyncHook for PollHandler {
    type Event = AutoReloadEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _existing_debounce: Option<Instant>,
    ) -> Option<Instant> {
        match event {
            AutoReloadEvent::PollAfter { interval } => {
                Some(Instant::now() + Duration::from_millis(interval))
            }
        }
    }

    fn finish_debounce(&mut self) {
        job::dispatch_blocking(move |editor, _compositor| {
            let config = editor.config();
            if !config.auto_reload.enable || !config.auto_reload.poll.enable {
                return;
            }

            let poll_interval = config.auto_reload.poll.interval;

            // Check unwatched documents for external modifications
            let modified_paths: Vec<PathBuf> = editor
                .documents()
                .filter_map(|doc| {
                    let path = doc.path()?;
                    // Skip documents that are being watched by filesentry
                    if editor.file_watcher.is_watching(path) {
                        return None;
                    }
                    let mtime = path.metadata().ok()?.modified().ok()?;
                    if mtime != doc.last_saved_time {
                        Some(path.to_path_buf())
                    } else {
                        None
                    }
                })
                .collect();

            // Poll extra watched paths (e.g., VCS HEAD files outside workspace)
            let extra_path_changes = editor.file_watcher.poll_extra_paths();

            // Dispatch changes through the FileSystemDidChange hook
            let all_changed: Vec<PathBuf> = modified_paths
                .into_iter()
                .chain(extra_path_changes)
                .collect();
            if !all_changed.is_empty() {
                let events = events_from_paths(all_changed);
                dispatch(FileSystemDidChange { fs_events: events });
            }

            // Schedule next poll
            send_blocking(
                &editor.handlers.auto_reload,
                AutoReloadEvent::PollAfter {
                    interval: poll_interval,
                },
            );
        });
    }
}

/// Handler for document changes detected by filesentry or polling
fn handle_document_change(
    editor: &mut Editor,
    compositor: &mut Compositor,
    doc_id: DocumentId,
    prompt_if_modified: bool,
) {
    let scrolloff = editor.config().scrolloff;

    let doc = doc_mut!(editor, &doc_id);
    let Some(path) = doc.path().cloned() else {
        return;
    };

    let mtime = match path.metadata() {
        Ok(meta) => meta.modified().unwrap_or(SystemTime::now()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return,
        Err(_) => SystemTime::now(),
    };

    if mtime == doc.last_saved_time {
        return;
    }

    if doc.is_modified() {
        if prompt_if_modified {
            let path_str = doc
                .relative_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "[scratch]".into());
            prompt_reload_modified(compositor, doc_id, path_str);
        } else {
            let msg = format!(
                "{} changed externally but has unsaved changes, use :reload to refresh",
                doc.relative_path().unwrap().display()
            );
            editor.set_warning(msg);
        }
    } else {
        let view = view_mut!(editor);
        match doc.reload(view, &editor.diff_providers) {
            Ok(_) => {
                view.ensure_cursor_in_view(doc, scrolloff);
                let msg = format!(
                    "{} reloaded (external changes)",
                    doc.relative_path().unwrap().display()
                );
                editor.set_status(msg);
            }
            Err(err) => {
                let doc = doc!(editor, &doc_id);
                let msg = format!(
                    "{} auto-reload failed: {err}",
                    doc.relative_path().unwrap().display()
                );
                editor.set_error(msg);
            }
        }
    }
}

/// Reload VCS diffs for all documents
fn reload_vcs_diffs(editor: &mut Editor) {
    for doc in editor.documents.values_mut() {
        let Some(path) = doc.path() else {
            continue;
        };
        match editor.diff_providers.get_diff_base(path) {
            Some(diff_base) => doc.set_diff_base(diff_base),
            None => doc.diff_handle = None,
        }
    }
}

/// Shows a prompt asking the user whether to reload a modified document.
/// Co-Authored-By: Anthony Rubick <68485672+AnthonyMichaelTDM@users.noreply.github.com>
fn prompt_reload_modified(compositor: &mut Compositor, doc_id: DocumentId, path_str: String) {
    let prompt = Prompt::new(
        Cow::Owned(format!(
            "{path_str} changed externally (unsaved changes exist). Press Enter to reload, Esc to ignore: "
        )),
        None,
        ui::completers::none,
        move |cx, _input, event| {
            match event {
                PromptEvent::Validate => {
                    let scrolloff = cx.editor.config().scrolloff;
                    let doc = doc_mut!(cx.editor, &doc_id);
                    let view = view_mut!(cx.editor);
                    match doc.reload(view, &cx.editor.diff_providers) {
                        Ok(_) => {
                            view.ensure_cursor_in_view(doc, scrolloff);
                            cx.editor.set_status(format!("{path_str} reloaded"));
                        }
                        Err(err) => {
                            cx.editor
                                .set_error(format!("{path_str} reload failed: {err}"));
                        }
                    }
                }
                PromptEvent::Abort => {
                    cx.editor
                        .set_status(format!("{path_str} external changes ignored"));
                }
                PromptEvent::Update => {}
            }
        },
    );
    compositor.push(Box::new(prompt));
}

pub(super) fn register_hooks(handlers: &Handlers, config: &Config) {
    // Register handler for FileSystemDidChange events (from filesentry)
    let handler = Arc::new(ReloadHandler {
        enable: config.auto_reload.enable.into(),
        prompt_if_modified: config.auto_reload.prompt_if_modified.into(),
    });
    let handler_ = handler.clone();
    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        handler_.refresh_config(event.new);
        Ok(())
    });
    register_hook!(move |event: &mut FileSystemDidChange| {
        handler.on_file_did_change(event);
        Ok(())
    });

    // Start polling if enabled
    if config.auto_reload.enable && config.auto_reload.poll.enable {
        send_blocking(
            &handlers.auto_reload,
            AutoReloadEvent::PollAfter {
                interval: config.auto_reload.poll.interval,
            },
        );
    }
}
