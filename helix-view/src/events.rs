use helix_core::{ChangeSet, Rope};
use helix_event::events;
use helix_lsp::LanguageServerId;

use crate::{Document, DocumentId, Editor, ViewId};

events! {
    DocumentDidOpen<'a> {
        editor: &'a mut Editor,
        doc: DocumentId
    }
    DocumentDidChange<'a> {
        doc: &'a mut Document,
        view: ViewId,
        old_text: &'a Rope,
        changes: &'a ChangeSet,
        ghost_transaction: bool
    }
    DocumentDidClose<'a> {
        editor: &'a mut Editor,
        doc: Document
    }
    SelectionDidChange<'a> { doc: &'a mut Document, view: ViewId }
    DiagnosticsDidChange<'a> { editor: &'a mut Editor, doc: DocumentId }
    // called **after** a document loses focus (but not when its closed)
    DocumentFocusLost<'a> { editor: &'a mut Editor, doc: DocumentId }

    LanguageServerInitialized<'a> {
        editor: &'a mut Editor,
        server_id: LanguageServerId
    }
    LanguageServerExited<'a> {
        editor: &'a mut Editor,
        server_id: LanguageServerId
    }
}
