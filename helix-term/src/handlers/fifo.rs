use helix_event::register_hook;
use helix_view::{events::FifoReceived, handlers::Handlers};

use crate::job;

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut FifoReceived<'_>| {
        let doc = event.doc.clone();
        let text = String::from(event.text);
        job::dispatch_blocking(move |editor, _| {
            let doc = editor.document_mut(doc).unwrap();
            doc.apply_directly(&text);
        });
        Ok(())
    });
}
