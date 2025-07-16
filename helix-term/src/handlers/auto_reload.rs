use std::borrow::Cow;
use std::fs;
use std::sync::atomic::{self, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use helix_core::command_line::Args;
use helix_event::{register_hook, send_blocking};
use helix_view::document::Mode;
use helix_view::events::DocumentDidOpen;
use helix_view::handlers::{AutoReloadEvent, Handlers};
use helix_view::{Document, Editor};
use tokio::time::Instant;

use crate::compositor::Compositor;
use crate::events::OnModeSwitch;
use crate::job;
use crate::ui::{Prompt, PromptEvent};
use crate::{commands, ui};

#[derive(Debug)]
pub(super) struct AutoReloadHandler {
    reload_pending: Arc<AtomicBool>,
}

impl AutoReloadHandler {
    pub fn new() -> AutoReloadHandler {
        AutoReloadHandler {
            reload_pending: Default::default(),
        }
    }
}

impl helix_event::AsyncHook for AutoReloadHandler {
    type Event = AutoReloadEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        existing_debounce: Option<Instant>,
    ) -> Option<Instant> {
        match event {
            Self::Event::CheckForChanges { after } => {
                Some(Instant::now() + Duration::from_millis(after))
            }
            Self::Event::LeftInsertMode | Self::Event::EditorFocused => {
                if existing_debounce.is_some() {
                    // If the event happened more recently than the debounce, let the
                    // debounce run down before checking for changes.
                    existing_debounce
                } else {
                    // Otherwise if there is a reload pending, check immediately.
                    if self.reload_pending.load(Ordering::Relaxed) {
                        self.finish_debounce();
                    }
                    None
                }
            }
        }
    }

    fn finish_debounce(&mut self) {
        let reload_pending = self.reload_pending.clone();
        job::dispatch_blocking(move |editor, compositor| {
            if editor.mode() == Mode::Insert {
                // Avoid reloading while in insert mode since this mixes up
                // the modification indicator and prevents future saves.
                reload_pending.store(true, atomic::Ordering::Relaxed);
            } else {
                prompt_to_reload_if_needed(editor, compositor);
                reload_pending.store(false, atomic::Ordering::Relaxed);
            }
        });
    }
}

/// Requests a reload if any documents have been modified externally.
fn prompt_to_reload_if_needed(editor: &mut Editor, compositor: &mut Compositor) {
    let modified_docs = editor
        .documents()
        // Filter out documents that have unsaved changes.
        .filter(|doc| !doc.is_modified())
        // Get the documents that have been modified externally.
        .filter(has_document_been_externally_modified)
        .count();

    // If there are no externally modified documents, we can do nothing.
    if modified_docs == 0 {
        // Reset the debounce timer to allow for the next check.
        let config = editor.config.load();
        if config.auto_reload.periodic.enable {
            let interval = config.auto_reload.periodic.interval;
            send_blocking(
                &editor.handlers.auto_reload,
                AutoReloadEvent::CheckForChanges { after: interval },
            );
        }

        return;
    }

    let prompt = Prompt::new(
        Cow::Borrowed("Some files have been modified externally, press Enter to reload them."),
        None,
        ui::completers::none,
        |cx, _, event| {
            if event == PromptEvent::Update {
                return;
            }

            if let Err(err) =
                commands::typed::reload_all(cx, Args::default(), PromptEvent::Validate)
            {
                cx.editor
                    .set_error(format!("Failed to reload document: {err}"));
            } else {
                cx.editor.set_status("Reloaded modified documents");
            }

            // Reset the debounce timer to allow for the next check.
            let config = cx.editor.config.load();
            if config.auto_reload.periodic.enable {
                let interval = config.auto_reload.periodic.interval;
                send_blocking(
                    &cx.editor.handlers.auto_reload,
                    AutoReloadEvent::CheckForChanges { after: interval },
                );
            }
        },
    );
    // Show the prompt to the user.
    compositor.push(Box::new(prompt));
}

fn has_document_been_externally_modified(doc: &&Document) -> bool {
    let last_saved_time = doc.get_last_saved_time();
    let Some(path) = doc.path() else {
        return false;
    };

    // Check if the file has been modified externally
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified_time) = metadata.modified() {
            if modified_time > last_saved_time {
                return true;
            }
        }
    }
    false
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.auto_reload.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let config = event.editor.config.load();
        if config.auto_reload.periodic.enable {
            let interval = config.auto_reload.periodic.interval;
            send_blocking(&tx, AutoReloadEvent::CheckForChanges { after: interval });
        }
        Ok(())
    });

    let tx = handlers.auto_reload.clone();
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.old_mode == Mode::Insert {
            send_blocking(&tx, AutoReloadEvent::LeftInsertMode)
        }
        Ok(())
    });
}
