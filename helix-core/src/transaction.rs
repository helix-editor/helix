use ropey::RopeSlice;
use smallvec::SmallVec;

use crate::{chars::char_is_word, Range, Rope, Selection, Tendril};
use std::{borrow::Cow, iter::once};

/// (from, to, replacement)
pub type Change = (usize, usize, Option<Tendril>);
pub type Deletion = (usize, usize);

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

impl Operation {
    /// The number of characters affected by the operation.
    pub fn len_chars(&self) -> usize {
        match self {
            Self::Retain(n) | Self::Delete(n) => *n,
            Self::Insert(s) => s.chars().count(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
    /// Acts like `After` if a word character is inserted
    /// after the position, otherwise acts like `Before`
    AfterWord,
    /// Acts like `Before` if a word character is inserted
    /// before the position, otherwise acts like `After`
    BeforeWord,
    /// Acts like `Before` but if the position is within an exact replacement
    /// (exact size) the offset to the start of the replacement is kept
    BeforeSticky,
    /// Acts like `After` but if the position is within an exact replacement
    /// (exact size) the offset to the start of the replacement is kept
    AfterSticky,
}

impl Assoc {
    /// Whether to stick to gaps
    fn stay_at_gaps(self) -> bool {
        !matches!(self, Self::BeforeWord | Self::AfterWord)
    }

    fn insert_offset(self, s: &str) -> usize {
        let chars = s.chars().count();
        match self {
            Assoc::After | Assoc::AfterSticky => chars,
            Assoc::AfterWord => s.chars().take_while(|&c| char_is_word(c)).count(),
            // return position before inserted text
            Assoc::Before | Assoc::BeforeSticky => 0,
            Assoc::BeforeWord => chars - s.chars().rev().take_while(|&c| char_is_word(c)).count(),
        }
    }

    pub fn sticky(self) -> bool {
        matches!(self, Assoc::BeforeSticky | Assoc::AfterSticky)
    }
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
    pub fn new(doc: RopeSlice) -> Self {
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
    pub(crate) fn delete(&mut self, n: usize) {
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

    pub(crate) fn insert(&mut self, fragment: Tendril) {
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

    pub(crate) fn retain(&mut self, n: usize) {
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

    /// Map a (mostly) *sorted* list of positions through the changes.
    ///
    /// This is equivalent to updating each position with `map_pos`:
    ///
    /// ``` no-compile
    /// for (pos, assoc) in positions {
    ///     *pos = changes.map_pos(*pos, assoc);
    /// }
    /// ```
    /// However this function is significantly faster for sorted lists running
    /// in `O(N+M)` instead of `O(NM)`. This function also handles unsorted/
    /// partially sorted lists. However, in that case worst case complexity is
    /// again `O(MN)`.  For lists that are often/mostly sorted (like the end of diagnostic ranges)
    /// performance is usally close to `O(N + M)`
    pub fn update_positions<'a>(&self, positions: impl Iterator<Item = (&'a mut usize, Assoc)>) {
        use Operation::*;

        let mut positions = positions.peekable();

        let mut old_pos = 0;
        let mut new_pos = 0;
        let mut iter = self.changes.iter().enumerate().peekable();

        'outer: loop {
            macro_rules! map {
                ($map: expr, $i: expr) => {
                    loop {
                        let Some((pos, assoc)) = positions.peek_mut() else {
                            return;
                        };
                        if **pos < old_pos {
                            // Positions are not sorted, revert to the last Operation that
                            // contains this position and continue iterating from there.
                            // We can unwrap here since `pos` can not be negative
                            // (unsigned integer) and iterating backwards to the start
                            // should always move us back to the start
                            for (i, change) in self.changes[..$i].iter().enumerate().rev() {
                                match change {
                                    Retain(i) => {
                                        old_pos -= i;
                                        new_pos -= i;
                                    }
                                    Delete(i) => {
                                        old_pos -= i;
                                    }
                                    Insert(ins) => {
                                        new_pos -= ins.chars().count();
                                    }
                                }
                                if old_pos <= **pos {
                                    iter = self.changes[i..].iter().enumerate().peekable();
                                }
                            }
                            debug_assert!(old_pos <= **pos, "Reverse Iter across changeset works");
                            continue 'outer;
                        }
                        #[allow(clippy::redundant_closure_call)]
                        let Some(new_pos) = $map(**pos, *assoc) else {
                            break;
                        };
                        **pos = new_pos;
                        positions.next();
                    }
                };
            }

            let Some((i, change)) = iter.next() else {
                map!(
                    |pos, _| (old_pos == pos).then_some(new_pos),
                    self.changes.len()
                );
                break;
            };

            let len = match change {
                Delete(i) | Retain(i) => *i,
                Insert(_) => 0,
            };
            let mut old_end = old_pos + len;

            match change {
                Retain(_) => {
                    map!(
                        |pos, _| (old_end > pos).then_some(new_pos + (pos - old_pos)),
                        i
                    );
                    new_pos += len;
                }
                Delete(_) => {
                    // in range
                    map!(|pos, _| (old_end > pos).then_some(new_pos), i);
                }
                Insert(s) => {
                    // a subsequent delete means a replace, consume it
                    if let Some((_, Delete(len))) = iter.peek() {
                        iter.next();

                        old_end = old_pos + len;
                        // in range of replaced text
                        map!(
                            |pos, assoc: Assoc| (old_end > pos).then(|| {
                                // at point or tracking before
                                if pos == old_pos && assoc.stay_at_gaps() {
                                    new_pos
                                } else {
                                    let ins = assoc.insert_offset(s);
                                    // if the deleted and inserted text have the exact same size
                                    // keep the relative offset into the new text
                                    if *len == ins && assoc.sticky() {
                                        new_pos + (pos - old_pos)
                                    } else {
                                        new_pos + assoc.insert_offset(s)
                                    }
                                }
                            }),
                            i
                        );
                    } else {
                        // at insert point
                        map!(
                            |pos, assoc: Assoc| (old_pos == pos).then(|| {
                                // return position before inserted text
                                new_pos + assoc.insert_offset(s)
                            }),
                            i
                        );
                    }

                    new_pos += s.chars().count();
                }
            }
            old_pos = old_end;
        }
        let out_of_bounds: Vec<_> = positions.collect();

        panic!("Positions {out_of_bounds:?} are out of range for changeset len {old_pos}!",)
    }

    /// Map a position through the changes.
    ///
    /// `assoc` indicates which side to associate the position with. `Before` will keep the
    /// position close to the character before, and will place it before insertions over that
    /// range, or at that point. `After` will move it forward, placing it at the end of such
    /// insertions.
    pub fn map_pos(&self, mut pos: usize, assoc: Assoc) -> usize {
        self.update_positions(once((&mut pos, assoc)));
        pos
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
            changes: ChangeSet::new(doc.slice(..)),
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

    /// Generate a transaction from a set of potentially overlapping changes. The `change_ranges`
    /// iterator yield the range (of removed text) in the old document for each edit. If any change
    /// overlaps with a range overlaps with a previous range then that range is ignored.
    ///
    /// The `process_change` callback is called for each edit that is not ignored (in the order
    /// yielded by `changes`) and should return the new text that the associated range will be
    /// replaced with.
    ///
    /// To make this function more flexible the iterator can yield additional data for each change
    /// that is passed to `process_change`
    pub fn change_ignore_overlapping<T>(
        doc: &Rope,
        change_ranges: impl Iterator<Item = (usize, usize, T)>,
        mut process_change: impl FnMut(usize, usize, T) -> Option<Tendril>,
    ) -> Self {
        let mut last = 0;
        let changes = change_ranges.filter_map(|(from, to, data)| {
            if from < last {
                return None;
            }
            let tendril = process_change(from, to, data);
            last = to;
            Some((from, to, tendril))
        });
        Self::change(doc, changes)
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
            // Verify ranges are correct
            debug_assert!(
                from <= to,
                "Edit end must end before it starts (should {from} <= {to})"
            );

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

    /// Generate a transaction from a set of potentially overlapping deletions
    /// by merging overlapping deletions together.
    pub fn delete<I>(doc: &Rope, deletions: I) -> Self
    where
        I: Iterator<Item = Deletion>,
    {
        let len = doc.len_chars();

        let (lower, upper) = deletions.size_hint();
        let size = upper.unwrap_or(lower);
        let mut changeset = ChangeSet::with_capacity(2 * size + 1); // rough estimate

        let mut last = 0;
        for (mut from, to) in deletions {
            if last > to {
                continue;
            }
            if last > from {
                from = last
            }
            debug_assert!(
                from <= to,
                "Edit end must end before it starts (should {from} <= {to})"
            );
            // Retain from last "to" to current "from"
            changeset.retain(from - last);
            changeset.delete(to - from);
            last = to;
        }

        changeset.retain(len - last);

        Self::from(changeset)
    }

    pub fn insert_at_eof(mut self, text: Tendril) -> Transaction {
        self.changes.insert(text);
        self
    }

    /// Generate a transaction with a change per selection range.
    pub fn change_by_selection<F>(doc: &Rope, selection: &Selection, f: F) -> Self
    where
        F: FnMut(&Range) -> Change,
    {
        Self::change(doc, selection.iter().map(f))
    }

    pub fn change_by_selection_ignore_overlapping(
        doc: &Rope,
        selection: &Selection,
        mut change_range: impl FnMut(&Range) -> (usize, usize),
        mut create_tendril: impl FnMut(usize, usize) -> Option<Tendril>,
    ) -> (Transaction, Selection) {
        let mut last_selection_idx = None;
        let mut new_primary_idx = None;
        let mut ranges: SmallVec<[Range; 1]> = SmallVec::new();
        let process_change = |change_start, change_end, (idx, range): (usize, &Range)| {
            // update the primary idx
            if idx == selection.primary_index() {
                new_primary_idx = Some(idx);
            } else if new_primary_idx.is_none() {
                if idx > selection.primary_index() {
                    new_primary_idx = last_selection_idx;
                } else {
                    last_selection_idx = Some(idx);
                }
            }
            ranges.push(*range);
            create_tendril(change_start, change_end)
        };
        let transaction = Self::change_ignore_overlapping(
            doc,
            selection.iter().enumerate().map(|range| {
                let (change_start, change_end) = change_range(range.1);
                (change_start, change_end, range)
            }),
            process_change,
        );

        (
            transaction,
            Selection::new(ranges, new_primary_idx.unwrap_or(0)),
        )
    }

    /// Generate a transaction with a deletion per selection range.
    /// Compared to using `change_by_selection` directly these ranges may overlap.
    /// In that case they are merged
    pub fn delete_by_selection<F>(doc: &Rope, selection: &Selection, f: F) -> Self
    where
        F: FnMut(&Range) -> Deletion,
    {
        Self::delete(doc, selection.iter().map(f))
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

impl Iterator for ChangeIterator<'_> {
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
    use crate::history::State;

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
        // unsorted selection
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
        let mut positions = [4, 2];
        cs.update_positions(positions.iter_mut().map(|pos| (pos, Assoc::After)));
        assert_eq!(positions, [4, 2]);
        // stays at word boundary
        let cs = ChangeSet {
            changes: vec![
                Retain(2), // <space><space>
                Insert(" ab".into()),
                Retain(2), // cd
                Insert("de ".into()),
            ],
            len: 4,
            len_after: 10,
        };
        assert_eq!(cs.map_pos(2, Assoc::BeforeWord), 3);
        assert_eq!(cs.map_pos(4, Assoc::AfterWord), 9);
        let cs = ChangeSet {
            changes: vec![
                Retain(1), // <space>
                Insert(" b".into()),
                Delete(1), // c
                Retain(1), // d
                Insert("e ".into()),
                Delete(1), // <space>
            ],
            len: 5,
            len_after: 7,
        };
        assert_eq!(cs.map_pos(1, Assoc::BeforeWord), 2);
        assert_eq!(cs.map_pos(3, Assoc::AfterWord), 5);
        let cs = ChangeSet {
            changes: vec![
                Retain(1), // <space>
                Insert("a".into()),
                Delete(2), // <space>b
                Retain(1), // d
                Insert("e".into()),
                Delete(1), // f
                Retain(1), // <space>
            ],
            len: 5,
            len_after: 7,
        };
        assert_eq!(cs.map_pos(2, Assoc::BeforeWord), 1);
        assert_eq!(cs.map_pos(4, Assoc::AfterWord), 4);
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
        let mut state = State {
            doc: "".into(),
            selection: Selection::point(0),
        };
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
        let a = ChangeSet::new(empty.slice(..));

        let mut b = ChangeSet::new(empty.slice(..));
        b.insert("a".into());

        let changes = a.compose(b);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert("a".into())]);
    }

    #[test]
    fn combine_with_utf8() {
        const TEST_CASE: &str = "Hello, これはヘリックスエディターです！";

        let empty = Rope::from("");
        let a = ChangeSet::new(empty.slice(..));

        let mut b = ChangeSet::new(empty.slice(..));
        b.insert(TEST_CASE.into());

        let changes = a.compose(b);

        use Operation::*;
        assert_eq!(changes.changes, &[Insert(TEST_CASE.into())]);
        assert_eq!(changes.len_after, TEST_CASE.chars().count());
    }
}
