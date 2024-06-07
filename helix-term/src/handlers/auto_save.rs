use std::time::Duration;

use anyhow::Ok;
use arc_swap::access::Access;

use helix_event::{register_hook, send_blocking};
use helix_view::{
    editor::DEFAULT_AUTO_SAVE_DELAY,
    events::DocumentDidChange,
    handlers::{lsp::AutoSaveEvent, Handlers},
    Editor,
};
use tokio::time::Instant;

use crate::{
    commands, compositor,
    job::{self, Jobs},
};

#[derive(Debug)]
enum State {
    Closed,
}

#[derive(Debug)]
pub(super) struct AutoSaveHandler {
    state: State,
}

impl AutoSaveHandler {
    pub fn new() -> AutoSaveHandler {
        AutoSaveHandler {
            state: State::Closed,
        }
    }
}

impl helix_event::AsyncHook for AutoSaveHandler {
    type Event = AutoSaveEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _: Option<tokio::time::Instant>,
    ) -> Option<Instant> {
        match event {
            AutoSaveEvent::Trigger(delay) => {
                if matches!(self.state, State::Closed) {
                    return Some(Instant::now() + Duration::from_millis(delay));
                }
            }
            AutoSaveEvent::Cancel => {
                self.state = State::Closed;
                return None;
            }
        }

        Some(Instant::now() + Duration::from_millis(DEFAULT_AUTO_SAVE_DELAY))
    }

    fn finish_debounce(&mut self) {
        job::dispatch_blocking(move |editor, _| request_auto_save(editor))
    }
}

fn request_auto_save(editor: &mut Editor) {
    let context = &mut compositor::Context {
        editor,
        scroll: Some(0),
        jobs: &mut Jobs::new(),
    };

    if let Err(e) = commands::typed::write_all_impl(context, false, false) {
        context.editor.set_error(format!("{}", e));
    }
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.auto_save.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        let config = event.doc.config.load();
        if let Some(delay) = config.auto_save.after_delay {
            send_blocking(&tx, AutoSaveEvent::Trigger(delay));
        }
        Ok(())
    });
}
