use crate::{Range, Rope, Selection, State, Tendril};
use std::borrow::Cow;
use std::convert::TryFrom;

/// (from, to, replacement)
pub type Change = (usize, usize, Option<Tendril>);

// TODO: pub(crate)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Move cursor by n characters.
    Retain(usize),
    /// Delete n characters.
    Delete(usize),
    /// Insert text at position.
    Insert(Tendril),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
}

// ChangeSpec = Change | ChangeSet | Vec<Change>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub(crate) changes: Vec<Operation>,
    /// The required document length. Will refuse to apply changes unless it matches.
    len: usize,
}

impl ChangeSet {
    #[must_use]
    pub fn new(doc: &Rope) -> Self {
        let len = doc.len_chars();
        Self {
            changes: vec![Operation::Retain(len)],
            len,
        }
    }

    // TODO: from iter
    //

    #[doc(hidden)] // used by lsp to convert to LSP changes
    pub fn changes(&self) -> &[Operation] {
        &self.changes
    }

    #[must_use]
    fn len_after(&self) -> usize {
        use Operation::*;

        let mut len = 0;
        for change in &self.changes {
            match change {
                Retain(i) => len += i,
                Insert(s) => len += s.chars().count(),
                Delete(_) => (),
            }
        }
        len
    }

    /// Combine two changesets together.
    /// In other words,  If `this` goes `docA` → `docB` and `other` represents `docB` → `docC`, the
    /// returned value will represent the change `docA` → `docC`.
    pub fn compose(self, other: ChangeSet) -> Self {
        debug_assert!(self.len_after() == other.len);

        let len = self.changes.len();

        let mut changes_a = self.changes.into_iter();
        let mut changes_b = other.changes.into_iter();

        let mut head_a = changes_a.next();
        let mut head_b = changes_b.next();

        let mut changes: Vec<Operation> = Vec::with_capacity(len); // TODO: max(a, b), shrink_to_fit() afterwards

        loop {
            use std::cmp::Ordering;
            use Operation::*;
            match (head_a, head_b) {
                // we are done
                (None, None) => {
                    break;
                }
                // deletion in A
                (Some(change @ Delete(..)), b) => {
                    changes.push(change);
                    head_a = changes_a.next();
                    head_b = b;
                }
                // insertion in B
                (a, Some(Insert(current))) => {
                    // merge onto previous insert if possible
                    // TODO: do these as operations on a changeset
                    if let Some(Insert(prev)) = changes.last_mut() {
                        prev.push_tendril(&current);
                    } else {
                        changes.push(Insert(current));
                    }
                    head_a = a;
                    head_b = changes_b.next();
                }
                (None, _) | (_, None) => return unreachable!(),
                (Some(Retain(i)), Some(Retain(j))) => match i.cmp(&j) {
                    Ordering::Less => {
                        changes.push(Retain(i));
                        head_a = changes_a.next();
                        head_b = Some(Retain(j - i));
                    }
                    Ordering::Equal => {
                        changes.push(Retain(i));
                        head_a = changes_a.next();
                        head_b = changes_b.next();
                    }
                    Ordering::Greater => {
                        changes.push(Retain(j));
                        head_a = Some(Retain(i - j));
                        head_b = changes_b.next();
                    }
                },
                (Some(Insert(mut s)), Some(Delete(j))) => {
                    let len = s.chars().count();
                    match len.cmp(&j) {
                        Ordering::Less => {
                            head_a = changes_a.next();
                            head_b = Some(Delete(j - len));
                        }
                        Ordering::Equal => {
                            head_a = changes_a.next();
                            head_b = changes_b.next();
                        }
                        Ordering::Greater => {
                            // figure out the byte index of the truncated string end
                            let (pos, _) = s.char_indices().nth(len - j).unwrap();
                            s.pop_front(s.len() as u32 - pos as u32);
                            head_a = Some(Insert(s));
                            head_b = changes_b.next();
                        }
                    }
                }
                (Some(Insert(mut s)), Some(Retain(j))) => {
                    let len = s.chars().count();
                    match len.cmp(&j) {
                        Ordering::Less => {
                            changes.push(Insert(s));
                            head_a = changes_a.next();
                            head_b = Some(Retain(j - len));
                        }
                        Ordering::Equal => {
                            changes.push(Insert(s));
                            head_a = changes_a.next();
                            head_b = changes_b.next();
                        }
                        Ordering::Greater => {
                            // figure out the byte index of the truncated string end
                            let (pos, _) = s.char_indices().nth(j).unwrap();
                            let pos = pos as u32;
                            changes.push(Insert(s.subtendril(0, pos)));
                            head_a = Some(Insert(s.subtendril(pos, s.len() as u32 - pos)));
                            head_b = changes_b.next();
                        }
                    }
                }
                (Some(Retain(i)), Some(Delete(j))) => match i.cmp(&j) {
                    Ordering::Less => {
                        changes.push(Delete(i));
                        head_a = changes_a.next();
                        head_b = Some(Delete(j - i));
                    }
                    Ordering::Equal => {
                        changes.push(Delete(j));
                        head_a = changes_a.next();
                        head_b = changes_b.next();
                    }
                    Ordering::Greater => {
                        changes.push(Delete(j));
                        head_a = Some(Retain(i - j));
                        head_b = changes_b.next();
                    }
                },
            };
        }

        Self {
            len: self.len,
            changes,
        }
    }

    /// Given another change set starting in the same document, maps this
    /// change set over the other, producing a new change set that can be
    /// applied to the document produced by applying `other`. When
    /// `before` is `true`, order changes as if `this` comes before
    /// `other`, otherwise (the default) treat `other` as coming first.
    ///
    /// Given two changes `A` and `B`, `A.compose(B.map(A))` and
    /// `B.compose(A.map(B, true))` will produce the same document. This
    /// provides a basic form of [operational
    /// transformation](https://en.wikipedia.org/wiki/Operational_transformation),
    /// and can be used for collaborative editing.
    pub fn map(self, _other: Self) -> Self {
        unimplemented!()
    }

    /// Returns a new changeset that reverts this one. Useful for `undo` implementation.
    /// The document parameter expects the original document before this change was applied.
    pub fn invert(&self, original_doc: &Rope) -> Self {
        assert!(original_doc.len_chars() == self.len);

        let mut changes = Vec::with_capacity(self.changes.len());
        let mut pos = 0;
        let mut len = 0;

        for change in &self.changes {
            use Operation::*;
            match change {
                Retain(n) => {
                    changes.push(Retain(*n));
                    pos += n;
                    len += n;
                }
                Delete(n) => {
                    let text = Cow::from(original_doc.slice(pos..pos + *n));
                    changes.push(Insert(Tendril::from_slice(&text)));
                    pos += n;
                }
                Insert(s) => {
                    let chars = s.chars().count();
                    changes.push(Delete(chars));
                    len += chars;
                }
            }
        }

        Self { changes, len }
    }

    /// Returns true if applied successfully.
    pub fn apply(&self, text: &mut Rope) -> bool {
        if text.len_chars() != self.len {
            return false;
        }

        let mut pos = 0;

        for change in &self.changes {
            use Operation::*;
            match change {
                Retain(n) => {
                    pos += n;
                }
                Delete(n) => {
                    text.remove(pos..pos + *n);
                    // pos += n;
                }
                Insert(s) => {
                    text.insert(pos, s);
                    pos += s.chars().count();
                }
            }
        }
        true
    }

    /// `true` when the set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let len = self.changes.len();
        len == 0 || (len == 1 && self.changes[0] == Operation::Retain(self.len))
    }

    /// Map a position through the changes.
    ///
    /// `assoc` indicates which size to associate the position with. `Before` will keep the
    /// position close to the character before, and will place it before insertions over that
    /// range, or at that point. `After` will move it forward, placing it at the end of such
    /// insertions.
    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        use Operation::*;
        let mut old_pos = 0;
        let mut new_pos = 0;

        let mut iter = self.changes.iter().peekable();

        while let Some(change) = iter.next() {
            let len = match change {
                Delete(i) | Retain(i) => *i,
                Insert(_) => 0,
            };
            let old_end = old_pos + len;

            match change {
                Retain(_) => {
                    if old_end > pos {
                        return new_pos + (pos - old_pos);
                    }
                    new_pos += len;
                }
                Delete(_) => {
                    // a subsequent ins means a replace, consume it
                    let ins = if let Some(Insert(s)) = iter.peek() {
                        iter.next();
                        s.chars().count()
                    } else {
                        0
                    };

                    // in range
                    if old_end > pos {
                        // at point or tracking before
                        if pos == old_pos || assoc == Assoc::Before {
                            return new_pos;
                        } else {
                            // place to end of delete
                            return new_pos + ins;
                        }
                    }

                    new_pos += ins;
                }
                Insert(s) => {
                    let ins = s.chars().count();
                    // at insert point
                    if old_pos == pos {
                        // return position before inserted text
                        if assoc == Assoc::Before {
                            return new_pos;
                        } else {
                            // after text
                            return new_pos + ins;
                        }
                    }

                    new_pos += ins;
                }
            }
            old_pos = old_end;
        }

        if pos > old_pos {
            panic!(
                "Position {} is out of range for changeset len {}!",
                pos, old_pos
            )
        }
        new_pos
    }
}

/// Transaction represents a single undoable unit of changes. Several changes can be grouped into
/// a single transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Changes made to the buffer.
    pub(crate) changes: ChangeSet,
    /// When set, explicitly updates the selection.
    selection: Option<Selection>,
    // effects, annotations
    // scroll_into_view
}

impl Transaction {
    /// Create a new, empty transaction.
    pub fn new(state: &mut State) -> Self {
        Self {
            changes: ChangeSet::new(&state.doc),
            selection: None,
        }
    }

    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    /// Returns true if applied successfully.
    pub fn apply(&self, state: &mut State) -> bool {
        if !self.changes.is_empty() {
            // apply changes to the document
            if !self.changes.apply(&mut state.doc) {
                return false;
            }
        }

        // TODO: also avoid mapping the selection if not necessary

        // update the selection: either take the selection specified in the transaction, or map the
        // current selection through changes.
        state.selection = self
            .selection
            .clone()
            .unwrap_or_else(|| state.selection.clone().map(&self.changes));

        true
    }

    /// Generate a transaction that reverts this one.
    pub fn invert(&self, original: &State) -> Self {
        let changes = self.changes.invert(original.doc());
        // Store the current cursor position
        let selection = original.selection.clone();

        Self {
            changes,
            selection: Some(selection),
        }
    }

    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Generate a transaction from a set of changes.
    pub fn change<I>(state: &State, changes: I) -> Self
    where
        I: IntoIterator<Item = Change> + ExactSizeIterator,
    {
        let len = state.doc.len_chars();
        let mut acc = Vec::with_capacity(2 * changes.len() + 1);

        // TODO: verify ranges are ordered and not overlapping or change will panic.

        let mut last = 0;
        for (from, to, tendril) in changes {
            // Retain from last "to" to current "from"
            if from - last > 0 {
                acc.push(Operation::Retain(from - last));
            }
            let span = to - from;
            match tendril {
                Some(text) => {
                    if span > 0 {
                        acc.push(Operation::Delete(span));
                    }
                    acc.push(Operation::Insert(text));
                }
                None if span > 0 => acc.push(Operation::Delete(span)),
                // empty delete is useless
                None => (),
            }
            last = to;
        }

        let span = len - last;
        if span > 0 {
            acc.push(Operation::Retain(span));
        }

        Self::from(ChangeSet { changes: acc, len })
    }

    /// Generate a transaction with a change per selection range.
    pub fn change_by_selection<F>(state: &State, f: F) -> Self
    where
        F: FnMut(&Range) -> Change,
    {
        Self::change(state, state.selection.ranges().iter().map(f))
    }

    /// Insert text at each selection head.
    pub fn insert(state: &State, text: Tendril) -> Self {
        Self::change_by_selection(state, |range| (range.head, range.head, Some(text.clone())))
    }
}

impl From<ChangeSet> for Transaction {
    fn from(changes: ChangeSet) -> Self {
        Self {
            changes,
            selection: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn composition() {
        use Operation::*;

        let a = ChangeSet {
            changes: vec![
                Retain(5),
                Insert(" test!".into()),
                Retain(1),
                Delete(2),
                Insert("abc".into()),
            ],
            len: 8,
        };

        let b = ChangeSet {
            changes: vec![Delete(10), Insert("world".into()), Retain(5)],
            len: 15,
        };

        let mut text = Rope::from("hello xz");

        // should probably return cloned text
        let composed = a.compose(b);
        assert_eq!(composed.len, 8);
        assert!(composed.apply(&mut text));
        assert_eq!(text, "world! abc");
    }

    #[test]
    fn invert() {
        use Operation::*;

        let changes = ChangeSet {
            changes: vec![Retain(4), Delete(5), Insert("test".into()), Retain(3)],
            len: 12,
        };

        let doc = Rope::from("世界3 hello xz");
        let revert = changes.invert(&doc);

        let mut doc2 = doc.clone();
        changes.apply(&mut doc2);

        // a revert is different
        assert_ne!(changes, revert);
        assert_ne!(doc, doc2);

        // but inverting a revert will give us the original
        assert_eq!(changes, revert.invert(&doc2));

        // applying a revert gives us back the original
        revert.apply(&mut doc2);
        assert_eq!(doc, doc2);
    }

    #[test]
    fn map_pos() {
        use Operation::*;

        // maps inserts
        let cs = ChangeSet {
            changes: vec![Retain(4), Insert("!!".into()), Retain(4)],
            len: 8,
        };

        assert_eq!(cs.map_pos(0, Assoc::Before), 0); // before insert region
        assert_eq!(cs.map_pos(4, Assoc::Before), 4); // at insert, track before
        assert_eq!(cs.map_pos(4, Assoc::After), 6); // at insert, track after
        assert_eq!(cs.map_pos(5, Assoc::Before), 7); // after insert region

        // maps deletes
        let cs = ChangeSet {
            changes: vec![Retain(4), Delete(4), Retain(4)],
            len: 12,
        };
        assert_eq!(cs.map_pos(0, Assoc::Before), 0); // at start
        assert_eq!(cs.map_pos(4, Assoc::Before), 4); // before a delete
        assert_eq!(cs.map_pos(5, Assoc::Before), 4); // inside a delete
        assert_eq!(cs.map_pos(5, Assoc::After), 4); // inside a delete

        // TODO: delete tracking

        // stays inbetween replacements
        let cs = ChangeSet {
            changes: vec![
                Delete(2),
                Insert("ab".into()),
                Delete(2),
                Insert("cd".into()),
            ],
            len: 4,
        };
        assert_eq!(cs.map_pos(2, Assoc::Before), 2);
        assert_eq!(cs.map_pos(2, Assoc::After), 2);
    }

    #[test]
    fn transaction_change() {
        let mut state = State::new("hello world!\ntest 123".into());
        let transaction = Transaction::change(
            &state,
            // (1, 1, None) is a useless 0-width delete
            vec![(1, 1, None), (6, 11, Some("void".into())), (12, 17, None)].into_iter(),
        );
        transaction.apply(&mut state);
        assert_eq!(state.doc, Rope::from_str("hello void! 123"));
    }

    #[test]
    fn insert_composition() {
        let mut state = State::new("".into());
        let t1 = Transaction::insert(&state, Tendril::from_char('h'));
        t1.apply(&mut state);
        let t2 = Transaction::insert(&state, Tendril::from_char('e'));
        t2.apply(&mut state);
        let t3 = Transaction::insert(&state, Tendril::from_char('l'));
        t3.apply(&mut state);
        let t4 = Transaction::insert(&state, Tendril::from_char('l'));
        t4.apply(&mut state);
        let t5 = Transaction::insert(&state, Tendril::from_char('o'));
        t5.apply(&mut state);

        assert_eq!(state.doc, Rope::from_str("hello"));

        // changesets as follows:
        // h
        // retain 1, e
        // retain 2, l

        let mut changes = t1
            .changes
            .compose(t2.changes)
            .compose(t3.changes)
            .compose(t4.changes)
            .compose(t5.changes);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert("hello".into())]);
    }
}
