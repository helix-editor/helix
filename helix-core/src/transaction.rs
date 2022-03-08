use crate::{Range, Rope, Selection, Tendril};
use std::borrow::Cow;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub(crate) changes: Vec<Operation>,
    /// The required document length. Will refuse to apply changes unless it matches.
    len: usize,
    len_after: usize,
}

impl ChangeSet {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            changes: Vec::with_capacity(capacity),
            len: 0,
            len_after: 0,
        }
    }

    #[must_use]
    pub fn new(doc: &Rope) -> Self {
        let len = doc.len_chars();
        Self {
            changes: Vec::new(),
            len,
            len_after: len,
        }
    }

    // TODO: from iter

    #[doc(hidden)] // used by lsp to convert to LSP changes
    pub fn changes(&self) -> &[Operation] {
        &self.changes
    }

    // Changeset builder operations: delete/insert/retain
    fn delete(&mut self, n: usize) {
        use Operation::*;
        if n == 0 {
            return;
        }

        self.len += n;

        if let Some(Delete(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(Delete(n));
        }
    }

    fn insert(&mut self, fragment: Tendril) {
        use Operation::*;

        if fragment.is_empty() {
            return;
        }

        // Avoiding std::str::len() to account for UTF-8 characters.
        self.len_after += fragment.chars().count();

        let new_last = match self.changes.as_mut_slice() {
            [.., Insert(prev)] | [.., Insert(prev), Delete(_)] => {
                prev.push_str(&fragment);
                return;
            }
            [.., last @ Delete(_)] => std::mem::replace(last, Insert(fragment)),
            _ => Insert(fragment),
        };

        self.changes.push(new_last);
    }

    fn retain(&mut self, n: usize) {
        use Operation::*;
        if n == 0 {
            return;
        }

        self.len += n;
        self.len_after += n;

        if let Some(Retain(count)) = self.changes.last_mut() {
            *count += n;
        } else {
            self.changes.push(Retain(n));
        }
    }

    /// Combine two changesets together.
    /// In other words,  If `this` goes `docA` → `docB` and `other` represents `docB` → `docC`, the
    /// returned value will represent the change `docA` → `docC`.
    pub fn compose(self, other: Self) -> Self {
        assert!(self.len_after == other.len);

        // composing fails in weird ways if one of the sets is empty
        // a: [] len: 0 len_after: 1 | b: [Insert(Tendril<UTF8>(inline: "\n")), Retain(1)] len 1
        if self.changes.is_empty() {
            return other;
        }
        if other.changes.is_empty() {
            return self;
        }

        let len = self.changes.len();

        let mut changes_a = self.changes.into_iter();
        let mut changes_b = other.changes.into_iter();

        let mut head_a = changes_a.next();
        let mut head_b = changes_b.next();

        let mut changes = Self::with_capacity(len); // TODO: max(a, b), shrink_to_fit() afterwards

        loop {
            use std::cmp::Ordering;
            use Operation::*;
            match (head_a, head_b) {
                // we are done
                (None, None) => {
                    break;
                }
                // deletion in A
                (Some(Delete(i)), b) => {
                    changes.delete(i);
                    head_a = changes_a.next();
                    head_b = b;
                }
                // insertion in B
                (a, Some(Insert(current))) => {
                    changes.insert(current);
                    head_a = a;
                    head_b = changes_b.next();
                }
                (None, val) | (val, None) => unreachable!("({:?})", val),
                (Some(Retain(i)), Some(Retain(j))) => match i.cmp(&j) {
                    Ordering::Less => {
                        changes.retain(i);
                        head_a = changes_a.next();
                        head_b = Some(Retain(j - i));
                    }
                    Ordering::Equal => {
                        changes.retain(i);
                        head_a = changes_a.next();
                        head_b = changes_b.next();
                    }
                    Ordering::Greater => {
                        changes.retain(j);
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
                            // TODO: cover this with a test
                            // figure out the byte index of the truncated string end
                            let (pos, _) = s.char_indices().nth(j).unwrap();
                            s.replace_range(0..pos, "");
                            head_a = Some(Insert(s));
                            head_b = changes_b.next();
                        }
                    }
                }
                (Some(Insert(s)), Some(Retain(j))) => {
                    let len = s.chars().count();
                    match len.cmp(&j) {
                        Ordering::Less => {
                            changes.insert(s);
                            head_a = changes_a.next();
                            head_b = Some(Retain(j - len));
                        }
                        Ordering::Equal => {
                            changes.insert(s);
                            head_a = changes_a.next();
                            head_b = changes_b.next();
                        }
                        Ordering::Greater => {
                            // figure out the byte index of the truncated string end
                            let (pos, _) = s.char_indices().nth(j).unwrap();
                            let mut before = s;
                            let after = before.split_off(pos);

                            changes.insert(before);
                            head_a = Some(Insert(after));
                            head_b = changes_b.next();
                        }
                    }
                }
                (Some(Retain(i)), Some(Delete(j))) => match i.cmp(&j) {
                    Ordering::Less => {
                        changes.delete(i);
                        head_a = changes_a.next();
                        head_b = Some(Delete(j - i));
                    }
                    Ordering::Equal => {
                        changes.delete(j);
                        head_a = changes_a.next();
                        head_b = changes_b.next();
                    }
                    Ordering::Greater => {
                        changes.delete(j);
                        head_a = Some(Retain(i - j));
                        head_b = changes_b.next();
                    }
                },
            };
        }

        // starting len should still equal original starting len
        debug_assert!(changes.len == self.len);

        changes
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

        let mut changes = Self::with_capacity(self.changes.len());

        let mut pos = 0;

        for change in &self.changes {
            use Operation::*;
            match change {
                Retain(n) => {
                    changes.retain(*n);
                    pos += n;
                }
                Delete(n) => {
                    let text = Cow::from(original_doc.slice(pos..pos + *n));
                    changes.insert(Tendril::from(text.as_ref()));
                    pos += n;
                }
                Insert(s) => {
                    let chars = s.chars().count();
                    changes.delete(chars);
                }
            }
        }

        changes
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
        self.changes.is_empty() || self.changes == [Operation::Retain(self.len)]
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
            let mut old_end = old_pos + len;

            match change {
                Retain(_) => {
                    if old_end > pos {
                        return new_pos + (pos - old_pos);
                    }
                    new_pos += len;
                }
                Delete(_) => {
                    // in range
                    if old_end > pos {
                        return new_pos;
                    }
                }
                Insert(s) => {
                    let ins = s.chars().count();

                    // a subsequent delete means a replace, consume it
                    if let Some(Delete(len)) = iter.peek() {
                        iter.next();

                        old_end = old_pos + len;
                        // in range of replaced text
                        if old_end > pos {
                            // at point or tracking before
                            if pos == old_pos || assoc == Assoc::Before {
                                return new_pos;
                            } else {
                                // place to end of insert
                                return new_pos + ins;
                            }
                        }
                    } else {
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

    pub fn changes_iter(&self) -> ChangeIterator {
        ChangeIterator::new(self)
    }
}

/// Transaction represents a single undoable unit of changes. Several changes can be grouped into
/// a single transaction.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Transaction {
    changes: ChangeSet,
    selection: Option<Selection>,
}

impl Transaction {
    /// Create a new, empty transaction.
    pub fn new(doc: &Rope) -> Self {
        Self {
            changes: ChangeSet::new(doc),
            selection: None,
        }
    }

    /// Changes made to the buffer.
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    /// When set, explicitly updates the selection.
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Returns true if applied successfully.
    pub fn apply(&self, doc: &mut Rope) -> bool {
        if self.changes.is_empty() {
            return true;
        }

        // apply changes to the document
        self.changes.apply(doc)
    }

    /// Generate a transaction that reverts this one.
    pub fn invert(&self, original: &Rope) -> Self {
        let changes = self.changes.invert(original);

        Self {
            changes,
            selection: None,
        }
    }

    pub fn compose(mut self, other: Self) -> Self {
        self.changes = self.changes.compose(other.changes);
        // Other selection takes precedence
        self.selection = other.selection;
        self
    }

    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Generate a transaction from a set of changes.
    pub fn change<I>(doc: &Rope, changes: I) -> Self
    where
        I: Iterator<Item = Change>,
    {
        let len = doc.len_chars();

        let (lower, upper) = changes.size_hint();
        let size = upper.unwrap_or(lower);
        let mut changeset = ChangeSet::with_capacity(2 * size + 1); // rough estimate

        let mut last = 0;
        for (from, to, tendril) in changes {
            // Verify ranges are ordered and not overlapping
            debug_assert!(last <= from);

            // Retain from last "to" to current "from"
            changeset.retain(from - last);
            let span = to - from;
            match tendril {
                Some(text) => {
                    changeset.insert(text);
                    changeset.delete(span);
                }
                None => changeset.delete(span),
            }
            last = to;
        }

        changeset.retain(len - last);

        Self::from(changeset)
    }

    /// Generate a transaction with a change per selection range.
    pub fn change_by_selection<F>(doc: &Rope, selection: &Selection, f: F) -> Self
    where
        F: FnMut(&Range) -> Change,
    {
        Self::change(doc, selection.iter().map(f))
    }

    /// Insert text at each selection head.
    pub fn insert(doc: &Rope, selection: &Selection, text: Tendril) -> Self {
        Self::change_by_selection(doc, selection, |range| {
            (range.head, range.head, Some(text.clone()))
        })
    }

    pub fn changes_iter(&self) -> ChangeIterator {
        self.changes.changes_iter()
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

pub struct ChangeIterator<'a> {
    iter: std::iter::Peekable<std::slice::Iter<'a, Operation>>,
    pos: usize,
}

impl<'a> ChangeIterator<'a> {
    fn new(changeset: &'a ChangeSet) -> Self {
        let iter = changeset.changes.iter().peekable();
        Self { iter, pos: 0 }
    }
}

impl<'a> Iterator for ChangeIterator<'a> {
    type Item = Change;

    fn next(&mut self) -> Option<Self::Item> {
        use Operation::*;

        loop {
            match self.iter.next()? {
                Retain(len) => {
                    self.pos += len;
                }
                Delete(len) => {
                    let start = self.pos;
                    self.pos += len;
                    return Some((start, self.pos, None));
                }
                Insert(s) => {
                    let start = self.pos;
                    // a subsequent delete means a replace, consume it
                    if let Some(Delete(len)) = self.iter.peek() {
                        self.iter.next();

                        self.pos += len;
                        return Some((start, self.pos, Some(s.clone())));
                    } else {
                        return Some((start, start, Some(s.clone())));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::State;

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
            len_after: 15,
        };

        let b = ChangeSet {
            changes: vec![Delete(10), Insert("世orld".into()), Retain(5)],
            len: 15,
            len_after: 10,
        };

        let mut text = Rope::from("hello xz");

        // should probably return cloned text
        let composed = a.compose(b);
        assert_eq!(composed.len, 8);
        assert!(composed.apply(&mut text));
        assert_eq!(text, "世orld! abc");
    }

    #[test]
    fn invert() {
        use Operation::*;

        let changes = ChangeSet {
            changes: vec![Retain(4), Insert("test".into()), Delete(5), Retain(3)],
            len: 12,
            len_after: 11,
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
            len_after: 10,
        };

        assert_eq!(cs.map_pos(0, Assoc::Before), 0); // before insert region
        assert_eq!(cs.map_pos(4, Assoc::Before), 4); // at insert, track before
        assert_eq!(cs.map_pos(4, Assoc::After), 6); // at insert, track after
        assert_eq!(cs.map_pos(5, Assoc::Before), 7); // after insert region

        // maps deletes
        let cs = ChangeSet {
            changes: vec![Retain(4), Delete(4), Retain(4)],
            len: 12,
            len_after: 8,
        };
        assert_eq!(cs.map_pos(0, Assoc::Before), 0); // at start
        assert_eq!(cs.map_pos(4, Assoc::Before), 4); // before a delete
        assert_eq!(cs.map_pos(5, Assoc::Before), 4); // inside a delete
        assert_eq!(cs.map_pos(5, Assoc::After), 4); // inside a delete

        // TODO: delete tracking

        // stays inbetween replacements
        let cs = ChangeSet {
            changes: vec![
                Insert("ab".into()),
                Delete(2),
                Insert("cd".into()),
                Delete(2),
            ],
            len: 4,
            len_after: 4,
        };
        assert_eq!(cs.map_pos(2, Assoc::Before), 2);
        assert_eq!(cs.map_pos(2, Assoc::After), 2);
    }

    #[test]
    fn transaction_change() {
        let mut doc = Rope::from("hello world!\ntest 123");
        let transaction = Transaction::change(
            &doc,
            // (1, 1, None) is a useless 0-width delete that gets factored out
            vec![(1, 1, None), (6, 11, Some("void".into())), (12, 17, None)].into_iter(),
        );
        transaction.apply(&mut doc);
        assert_eq!(doc, Rope::from_str("hello void! 123"));
    }

    #[test]
    fn changes_iter() {
        let doc = Rope::from("hello world!\ntest 123");
        let changes = vec![(6, 11, Some("void".into())), (12, 17, None)];
        let transaction = Transaction::change(&doc, changes.clone().into_iter());
        assert_eq!(transaction.changes_iter().collect::<Vec<_>>(), changes);
    }

    #[test]
    fn optimized_composition() {
        let mut state = State::new("".into());
        let t1 = Transaction::insert(&state.doc, &state.selection, Tendril::from("h"));
        t1.apply(&mut state.doc);
        state.selection = state.selection.clone().map(t1.changes());
        let t2 = Transaction::insert(&state.doc, &state.selection, Tendril::from("e"));
        t2.apply(&mut state.doc);
        state.selection = state.selection.clone().map(t2.changes());
        let t3 = Transaction::insert(&state.doc, &state.selection, Tendril::from("l"));
        t3.apply(&mut state.doc);
        state.selection = state.selection.clone().map(t3.changes());
        let t4 = Transaction::insert(&state.doc, &state.selection, Tendril::from("l"));
        t4.apply(&mut state.doc);
        state.selection = state.selection.clone().map(t4.changes());
        let t5 = Transaction::insert(&state.doc, &state.selection, Tendril::from("o"));
        t5.apply(&mut state.doc);
        state.selection = state.selection.clone().map(t5.changes());

        assert_eq!(state.doc, Rope::from_str("hello"));

        // changesets as follows:
        // h
        // retain 1, e
        // retain 2, l

        let changes = t1
            .changes
            .compose(t2.changes)
            .compose(t3.changes)
            .compose(t4.changes)
            .compose(t5.changes);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert("hello".into())]);
        // instead of insert h, insert e, insert l, insert l, insert o
    }

    #[test]
    fn combine_with_empty() {
        let empty = Rope::from("");
        let a = ChangeSet::new(&empty);

        let mut b = ChangeSet::new(&empty);
        b.insert("a".into());

        let changes = a.compose(b);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert("a".into())]);
    }

    #[test]
    fn combine_with_utf8() {
        const TEST_CASE: &str = "Hello, これはヘリックスエディターです！";

        let empty = Rope::from("");
        let a = ChangeSet::new(&empty);

        let mut b = ChangeSet::new(&empty);
        b.insert(TEST_CASE.into());

        let changes = a.compose(b);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert(TEST_CASE.into())]);
        assert_eq!(changes.len_after, TEST_CASE.chars().count());
    }
}
