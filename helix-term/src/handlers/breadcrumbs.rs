use helix_event::register_hook;
use helix_view::{
    events::{DocumentSymbolsDidChange, SelectionDidChange},
    handlers::Handlers,
    Editor,
};

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentSymbolsDidChange<'_>| {
        let doc_id = event.doc;
        let Editor {
            tree, documents, ..
        } = &mut *event.editor;
        if let Some(doc) = documents.get_mut(&doc_id) {
            // Update each open view of the document and update.
            for (view, _) in tree.views().filter(|(view, _)| view.doc == doc_id) {
                doc.update_breadcrumbs_for_view_inlined(view.id);
            }
        }
        Ok(())
    });

    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        event.doc.update_breadcrumbs_for_view_inlined(event.view);
        Ok(())
    });
}
