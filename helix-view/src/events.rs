use helix_core::{ChangeSet, Rope};
use helix_event::events;

use crate::{Document, DocumentId, Editor, ViewId};

events! {
    DocumentDidChange<'a> {
        doc: &'a mut Document,
        view: ViewId,
        old_text: &'a Rope,
        changes: &'a ChangeSet,
        ghost_transaction: bool
    }
    SelectionDidChange<'a> { doc: &'a mut Document, view: ViewId }
    DiagnosticsDidChange<'a> { editor: &'a mut Editor, doc: DocumentId }
    // called **after** a document loses focus (but not when its closed)
    DocumentFocusLost<'a> { editor: &'a mut Editor, doc: DocumentId }
}
