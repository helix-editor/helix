use crate::{ChangeSet, Rope, State, Transaction};
use smallvec::{smallvec, SmallVec};

/// Undo-tree style history store.
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct History {
    revisions: Vec<Revision>,
    cursor: usize,
}

#[derive(Debug)]
struct Revision {
    parent: usize,
    children: SmallVec<[(usize, Transaction); 1]>,
    /// The transaction to revert to previous state.
    revert: Transaction,
    // selection before, selection after?
}

impl Default for History {
    fn default() -> Self {
        // Add a dummy root revision with empty transaction
        Self {
            revisions: vec![Revision {
                parent: 0,
                children: SmallVec::new(),
                revert: Transaction::from(ChangeSet::new(&Rope::new())),
            }],
            cursor: 0,
        }
    }
}

impl History {
    pub fn commit_revision(&mut self, transaction: &Transaction, original: &State) {
        // TODO: could store a single transaction, if deletes also stored the text they delete
        let revert = transaction
            .invert(&original.doc)
            // Store the current cursor position
            .with_selection(original.selection.clone());

        let new_cursor = self.revisions.len();
        self.revisions.push(Revision {
            parent: self.cursor,
            children: SmallVec::new(),
            revert,
        });

        // add a reference to the parent
        self.revisions
            .get_mut(self.cursor)
            .unwrap() // TODO: get_unchecked_mut
            .children
            .push((new_cursor, transaction.clone()));

        self.cursor = new_cursor;
    }

    #[inline]
    pub fn current_revision(&self) -> usize {
        self.cursor
    }

    #[inline]
    pub const fn at_root(&self) -> bool {
        self.cursor == 0
    }

    pub fn undo(&mut self) -> Option<&Transaction> {
        if self.at_root() {
            // We're at the root of undo, nothing to do.
            return None;
        }

        let current_revision = &self.revisions[self.cursor];

        self.cursor = current_revision.parent;

        Some(&current_revision.revert)
    }

    pub fn redo(&mut self) -> Option<&Transaction> {
        let current_revision = &self.revisions[self.cursor];

        // for now, simply pick the latest child (linear undo / redo)
        if let Some((index, transaction)) = current_revision.children.last() {
            self.cursor = *index;

            return Some(&transaction);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut history = History::default();
        let doc = Rope::from("hello");
        let mut state = State::new(doc);

        let transaction1 =
            Transaction::change(&state.doc, vec![(5, 5, Some(" world!".into()))].into_iter());

        // Need to commit before applying!
        history.commit_revision(&transaction1, &state);
        transaction1.apply(&mut state.doc);
        assert_eq!("hello world!", state.doc);

        // ---

        let transaction2 =
            Transaction::change(&state.doc, vec![(6, 11, Some("世界".into()))].into_iter());

        // Need to commit before applying!
        history.commit_revision(&transaction2, &state);
        transaction2.apply(&mut state.doc);
        assert_eq!("hello 世界!", state.doc);

        // ---
        fn undo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.undo() {
                transaction.apply(&mut state.doc);
            }
        }
        fn redo(history: &mut History, state: &mut State) {
            if let Some(transaction) = history.redo() {
                transaction.apply(&mut state.doc);
            }
        }

        undo(&mut history, &mut state);
        assert_eq!("hello world!", state.doc);
        redo(&mut history, &mut state);
        assert_eq!("hello 世界!", state.doc);
        undo(&mut history, &mut state);
        undo(&mut history, &mut state);
        assert_eq!("hello", state.doc);

        // undo at root is a no-op
        undo(&mut history, &mut state);
        assert_eq!("hello", state.doc);
    }
}
