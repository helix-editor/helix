use crate::{Document, Editor, View};
use helix_core::{Selection, Transaction};

pub trait EditorFocus {
    fn focused_view(&self) -> &View;
    fn focused_document(&self) -> &Document;
    fn focused_view_doc(&self) -> (&View, &Document);
    fn focused_selection(&self) -> &Selection;
    fn apply_transaction_to_focused_view_doc(&mut self, transaction: &Transaction) -> bool;
}

impl EditorFocus for Editor {
    fn focused_view(&self) -> &View {
        self.tree.get(self.tree.focus)
    }

    fn focused_document(&self) -> &Document {
        let current_view = self.tree.get(self.tree.focus);
        &self.documents[&current_view.doc]
    }

    fn focused_view_doc(&self) -> (&View, &Document) {
        (self.focused_view(), self.focused_document())
    }

    fn focused_selection(&self) -> &Selection {
        let (view, doc) = self.focused_view_doc();
        doc.selection(view.id)
    }

    fn apply_transaction_to_focused_view_doc(&mut self, transaction: &Transaction) -> bool {
        let current_view = self.tree.get(self.tree.focus);
        self.documents
            .get_mut(&current_view.doc)
            .expect("Current document id in view should point to a document.")
            .apply(transaction, self.tree.get(self.tree.focus).id)
    }
}
