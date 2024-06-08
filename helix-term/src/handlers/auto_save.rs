use std::time::Duration;

use anyhow::Ok;
use arc_swap::access::Access;

use helix_event::{register_hook, send_blocking};
use helix_view::{events::DocumentDidChange, handlers::Handlers, Editor};
use tokio::time::Instant;

use crate::{
    commands, compositor,
    job::{self, Jobs},
};

#[derive(Debug)]
pub(super) struct AutoSaveHandler;

impl AutoSaveHandler {
    pub fn new() -> AutoSaveHandler {
        AutoSaveHandler
    }
}

impl helix_event::AsyncHook for AutoSaveHandler {
    type Event = u64;

    fn handle_event(
        &mut self,
        timeout: Self::Event,
        _: Option<tokio::time::Instant>,
    ) -> Option<Instant> {
        Some(Instant::now() + Duration::from_millis(timeout))
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
        if config.auto_save.after_delay.enable {
            send_blocking(&tx, config.auto_save.after_delay.timeout);
        }
        Ok(())
    });
}
