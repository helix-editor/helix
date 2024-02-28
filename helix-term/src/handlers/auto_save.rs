use arc_swap::access::Access;

use helix_event::{cancelation, register_hook, send_blocking, CancelRx, CancelTx};
use helix_view::{
    editor::SaveStyle,
    events::DocumentDidChange,
    handlers::{
        lsp::{AutoSaveEvent, AutoSaveInvoked},
        Handlers,
    }, Editor,
};
use tokio::time::Instant;

use crate::job;

#[derive(Debug)]
enum State {
    Open,
    Closed,
    Pending { request: CancelTx },
}

#[derive(Debug)]
pub(super) struct AutoSaveHandler {
    trigger: Option<AutoSaveInvoked>,
    state: State,
}

impl AutoSaveHandler {
    pub fn new() -> AutoSaveHandler {
        AutoSaveHandler {
            trigger: None,
            state: State::Closed,
        }
    }
}

impl helix_event::AsyncHook for AutoSaveHandler {
    type Event = AutoSaveEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        timeout: Option<tokio::time::Instant>,
    ) -> Option<Instant> {
        match event {
            AutoSaveEvent::Invoked => todo!(),
            AutoSaveEvent::Cancel => {
                self.state = State::Closed;
                None
            },
        }
    }

    fn finish_debounce(&mut self) {
        let invocation = self.trigger.take().unwrap();
        let (tx, rx) = cancelation();
        self.state = State::Pending { request: tx };
        job::dispatch_blocking(move |editor, _| request_auto_save(editor, invocation, rx))
    }
}

fn request_auto_save(
    editor: &mut Editor,
    invoked: AutoSaveInvoked,
    cancel: CancelRx,
) {

}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.auto_save.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        let config = event.doc.config.load();
        if config.auto_save && config.save_style == SaveStyle::AfterDelay {
            send_blocking(&tx, AutoSaveEvent::Cancel);
            send_blocking(&tx, AutoSaveEvent::Invoked);
        }
        Ok(())
    });
}
