//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{graphemes, movement::Direction, Range, Rope, Selection, Tendril, Transaction};
use std::collections::HashMap;

use smallvec::SmallVec;

// Heavily based on https://github.com/codemirror/closebrackets/
pub const DEFAULT_PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('\'', '\''),
    ('"', '"'),
    ('`', '`'),
];

/// The type that represents the collection of auto pairs,
/// keyed by both opener and closer.
#[derive(Debug, Clone)]
pub struct AutoPairs(HashMap<char, Pair>);

/// Represents the config for a particular pairing.
#[derive(Debug, Clone, Copy)]
pub struct Pair {
    pub open: char,
    pub close: char,
}

impl Pair {
    /// true if open == close
    pub fn same(&self) -> bool {
        self.open == self.close
    }

    /// true if all of the pair's conditions hold for the given document and range
    pub fn should_close(&self, doc: &Rope, range: &Range) -> bool {
        let mut should_close = Self::next_is_not_alpha(doc, range);

        if self.same() {
            should_close &= Self::prev_is_not_alpha(doc, range);
        }

        should_close
    }

    pub fn next_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        next_char.map(|c| !c.is_alphanumeric()).unwrap_or(true)
    }

    pub fn prev_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        let prev_char = prev_char(doc, cursor);
        prev_char.map(|c| !c.is_alphanumeric()).unwrap_or(true)
    }
}

impl From<&(char, char)> for Pair {
    fn from(&(open, close): &(char, char)) -> Self {
        Self { open, close }
    }
}

impl From<(&char, &char)> for Pair {
    fn from((open, close): (&char, &char)) -> Self {
        Self {
            open: *open,
            close: *close,
        }
    }
}

impl AutoPairs {
    /// Make a new AutoPairs set with the given pairs and default conditions.
    pub fn new<'a, V, A>(pairs: V) -> Self
    where
        V: IntoIterator<Item = A> + 'a,
        A: Into<Pair>,
    {
        let mut auto_pairs = HashMap::new();

        for pair in pairs.into_iter() {
            let auto_pair = pair.into();

            auto_pairs.insert(auto_pair.open, auto_pair);

            if auto_pair.open != auto_pair.close {
                auto_pairs.insert(auto_pair.close, auto_pair);
            }
        }

        Self(auto_pairs)
    }

    pub fn get(&self, ch: char) -> Option<&Pair> {
        self.0.get(&ch)
    }
}

impl Default for AutoPairs {
    fn default() -> Self {
        AutoPairs::new(DEFAULT_PAIRS.iter())
    }
}

// insert hook:
// Fn(doc, selection, char) => Option<Transaction>
// problem is, we want to do this per range, so we can call default handler for some ranges
// so maybe ret Vec<Option<Change>>
// but we also need to be able to return transactions...
//
// to simplify, maybe return Option<Transaction> and just reimplement the default

// [TODO]
// * delete implementation where it erases the whole bracket (|) -> |
// * change to multi character pairs to handle cases like placing the cursor in the
//   middle of triple quotes, and more exotic pairs like Jinja's {% %}

#[must_use]
pub fn hook(doc: &Rope, selection: &Selection, ch: char, pairs: &AutoPairs) -> Option<Transaction> {
    log::trace!("autopairs hook selection: {:#?}", selection);

    if let Some(pair) = pairs.get(ch) {
        if pair.same() {
            return Some(handle_same(doc, selection, pair));
        } else if pair.open == ch {
            return Some(handle_open(doc, selection, pair));
        } else if pair.close == ch {
            // && char_at pos == close
            return Some(handle_close(doc, selection, pair));
        }
    }

    None
}

fn prev_char(doc: &Rope, pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }

    doc.get_char(pos - 1)
}

/// calculate what the resulting range should be for an auto pair insertion
fn get_next_range(doc: &Rope, start_range: &Range, offset: usize, len_inserted: usize) -> Range {
    // When the character under the cursor changes due to complete pair
    // insertion, we must look backward a grapheme and then add the length
    // of the insertion to put the resulting cursor in the right place, e.g.
    //
    // foo[\r\n] - anchor: 3, head: 5
    // foo([)]\r\n - anchor: 4, head: 5
    //
    // foo[\r\n] - anchor: 3, head: 5
    // foo'[\r\n] - anchor: 4, head: 6
    //
    // foo([)]\r\n - anchor: 4, head: 5
    // foo()[\r\n] - anchor: 5, head: 7
    //
    // [foo]\r\n - anchor: 0, head: 3
    // [foo(])\r\n - anchor: 0, head: 5

    // inserting at the very end of the document after the last newline
    if start_range.head == doc.len_chars() && start_range.anchor == doc.len_chars() {
        return Range::new(
            start_range.anchor + offset + 1,
            start_range.head + offset + 1,
        );
    }

    let doc_slice = doc.slice(..);
    let single_grapheme = start_range.is_single_grapheme(doc_slice);

    // just skip over graphemes
    if len_inserted == 0 {
        let end_anchor = if single_grapheme {
            graphemes::next_grapheme_boundary(doc_slice, start_range.anchor) + offset

        // even for backward inserts with multiple grapheme selections,
        // we want the anchor to stay where it is so that the relative
        // selection does not change, e.g.:
        //
        // foo([) wor]d -> insert ) -> foo()[ wor]d
        } else {
            start_range.anchor + offset
        };

        return Range::new(
            end_anchor,
            graphemes::next_grapheme_boundary(doc_slice, start_range.head) + offset,
        );
    }

    // trivial case: only inserted a single-char opener, just move the selection
    if len_inserted == 1 {
        let end_anchor = if single_grapheme || start_range.direction() == Direction::Backward {
            start_range.anchor + offset + 1
        } else {
            start_range.anchor + offset
        };

        return Range::new(end_anchor, start_range.head + offset + 1);
    }

    // If the head = 0, then we must be in insert mode with a backward
    // cursor, which implies the head will just move
    let end_head = if start_range.head == 0 || start_range.direction() == Direction::Backward {
        start_range.head + offset + 1
    } else {
        // We must have a forward cursor, which means we must move to the
        // other end of the grapheme to get to where the new characters
        // are inserted, then move the head to where it should be
        let prev_bound = graphemes::prev_grapheme_boundary(doc_slice, start_range.head);
        log::trace!(
            "prev_bound: {}, offset: {}, len_inserted: {}",
            prev_bound,
            offset,
            len_inserted
        );
        prev_bound + offset + len_inserted
    };

    let end_anchor = match (start_range.len(), start_range.direction()) {
        // if we have a zero width cursor, it shifts to the same number
        (0, _) => end_head,

        // If we are inserting for a regular one-width cursor, the anchor
        // moves with the head. This is the fast path for ASCII.
        (1, Direction::Forward) => end_head - 1,
        (1, Direction::Backward) => end_head + 1,

        (_, Direction::Forward) => {
            if single_grapheme {
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.head) + 1

            // if we are appending, the anchor stays where it is; only offset
            // for multiple range insertions
            } else {
                start_range.anchor + offset
            }
        }

        (_, Direction::Backward) => {
            if single_grapheme {
                // if we're backward, then the head is at the first char
                // of the typed char, so we need to add the length of
                // the closing char
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.anchor)
                    + len_inserted
                    + offset
            } else {
                // when we are inserting in front of a selection, we need to move
                // the anchor over by however many characters were inserted overall
                start_range.anchor + offset + len_inserted
            }
        }
    };

    Range::new(end_anchor, end_head)
}

fn handle_open(doc: &Rope, selection: &Selection, pair: &Pair) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        let len_inserted;

        // Since auto pairs are currently limited to single chars, we're either
        // inserting exactly one or two chars. When arbitrary length pairs are
        // added, these will need to be changed.
        let change = match next_char {
            Some(_) if !pair.should_close(doc, start_range) => {
                len_inserted = 1;
                let mut tendril = Tendril::new();
                tendril.push(pair.open);
                (cursor, cursor, Some(tendril))
            }
            _ => {
                // insert open & close
                let pair_str = Tendril::from_iter([pair.open, pair.close]);
                len_inserted = 2;
                (cursor, cursor, Some(pair_str))
            }
        };

        let next_range = get_next_range(doc, start_range, offs, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    log::debug!("auto pair transaction: {:#?}", t);
    t
}

fn handle_close(doc: &Rope, selection: &Selection, pair: &Pair) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        let mut len_inserted = 0;

        let change = if next_char == Some(pair.close) {
            // return transaction that moves past close
            (cursor, cursor, None) // no-op
        } else {
            len_inserted = 1;
            let mut tendril = Tendril::new();
            tendril.push(pair.close);
            (cursor, cursor, Some(tendril))
        };

        let next_range = get_next_range(doc, start_range, offs, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    log::debug!("auto pair transaction: {:#?}", t);
    t
}

/// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_same(doc: &Rope, selection: &Selection, pair: &Pair) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let mut len_inserted = 0;
        let next_char = doc.get_char(cursor);

        let change = if next_char == Some(pair.open) {
            //  return transaction that moves past close
            (cursor, cursor, None) // no-op
        } else {
            let mut pair_str = Tendril::new();
            pair_str.push(pair.open);

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if pair.should_close(doc, start_range) {
                pair_str.push(pair.close);
            }

            len_inserted += pair_str.chars().count();
            (cursor, cursor, Some(pair_str))
        };

        let next_range = get_next_range(doc, start_range, offs, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    log::debug!("auto pair transaction: {:#?}", t);
    t
}
