use helix_event::register_hook;
use helix_view::events::{DocumentDidChange, DocumentFocusLost, SelectionDidChange};
use helix_view::handlers::Handlers;

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if let Some(snippet) = &event.doc.active_snippet {
            if !snippet.is_valid(event.doc.selection(event.view)) {
                event.doc.active_snippet = None;
            }
        }
        Ok(())
    });
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if let Some(snippet) = &mut event.doc.active_snippet {
            let invalid = snippet.map(event.changes);
            if invalid {
                event.doc.active_snippet = None;
            }
        }
        Ok(())
    });
    register_hook!(move |event: &mut DocumentFocusLost<'_>| {
        let editor = &mut event.editor;
        doc_mut!(editor).active_snippet = None;
        Ok(())
    });
}
