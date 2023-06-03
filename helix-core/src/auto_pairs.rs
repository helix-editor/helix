//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{graphemes, movement::Direction, Change, Range, Rope, Tendril};
use std::collections::HashMap;

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
    pub fn new<'a, V: 'a, A>(pairs: V) -> Self
    where
        V: IntoIterator<Item = A>,
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

// [TODO]
// * delete implementation where it erases the whole bracket (|) -> |
// * change to multi character pairs to handle cases like placing the cursor in the
//   middle of triple quotes, and more exotic pairs like Jinja's {% %}

#[must_use]
pub fn hook_insert(
    doc: &Rope,
    range: &Range,
    ch: char,
    pairs: &AutoPairs,
) -> Option<(Change, Range)> {
    log::trace!("autopairs hook range: {:#?}", range);

    if let Some(pair) = pairs.get(ch) {
        if pair.same() {
            return handle_insert_same(doc, range, pair);
        } else if pair.open == ch {
            return handle_insert_open(doc, range, pair);
        } else if pair.close == ch {
            // && char_at pos == close
            return handle_insert_close(doc, range, pair);
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
fn get_next_range(doc: &Rope, start_range: &Range, len_inserted: usize) -> Range {
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
        return Range::new(start_range.anchor + 1, start_range.head + 1);
    }

    let doc_slice = doc.slice(..);
    let single_grapheme = start_range.is_single_grapheme(doc_slice);

    // just skip over graphemes
    if len_inserted == 0 {
        let end_anchor = if single_grapheme {
            graphemes::next_grapheme_boundary(doc_slice, start_range.anchor)

        // even for backward inserts with multiple grapheme selections,
        // we want the anchor to stay where it is so that the relative
        // selection does not change, e.g.:
        //
        // foo([) wor]d -> insert ) -> foo()[ wor]d
        } else {
            start_range.anchor
        };

        return Range::new(
            end_anchor,
            graphemes::next_grapheme_boundary(doc_slice, start_range.head),
        );
    }

    // trivial case: only inserted a single-char opener, just move the selection
    if len_inserted == 1 {
        let end_anchor = if single_grapheme || start_range.direction() == Direction::Backward {
            start_range.anchor + 1
        } else {
            start_range.anchor
        };

        return Range::new(end_anchor, start_range.head + 1);
    }

    // If the head = 0, then we must be in insert mode with a backward
    // cursor, which implies the head will just move
    let end_head = if start_range.head == 0 || start_range.direction() == Direction::Backward {
        start_range.head + 1
    } else {
        // We must have a forward cursor, which means we must move to the
        // other end of the grapheme to get to where the new characters
        // are inserted, then move the head to where it should be
        let prev_bound = graphemes::prev_grapheme_boundary(doc_slice, start_range.head);
        log::trace!("prev_bound: {}, len_inserted: {}", prev_bound, len_inserted);

        prev_bound + len_inserted
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
                start_range.anchor
            }
        }

        (_, Direction::Backward) => {
            if single_grapheme {
                // if we're backward, then the head is at the first char
                // of the typed char, so we need to add the length of
                // the closing char
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.anchor) + len_inserted
            } else {
                // when we are inserting in front of a selection, we need to move
                // the anchor over by however many characters were inserted overall
                start_range.anchor + len_inserted
            }
        }
    };

    Range::new(end_anchor, end_head)
}

fn handle_insert_open(doc: &Rope, range: &Range, pair: &Pair) -> Option<(Change, Range)> {
    let cursor = range.cursor(doc.slice(..));
    let next_char = doc.get_char(cursor);
    let len_inserted;

    // Since auto pairs are currently limited to single chars, we're either
    // inserting exactly one or two chars. When arbitrary length pairs are
    // added, these will need to be changed.
    let change = match next_char {
        Some(_) if !pair.should_close(doc, range) => {
            return None;
        }
        _ => {
            // insert open & close
            let pair_str = Tendril::from_iter([pair.open, pair.close]);
            len_inserted = 2;
            (cursor, cursor, Some(pair_str))
        }
    };

    let next_range = get_next_range(doc, range, len_inserted);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}

fn handle_insert_close(doc: &Rope, range: &Range, pair: &Pair) -> Option<(Change, Range)> {
    let cursor = range.cursor(doc.slice(..));
    let next_char = doc.get_char(cursor);

    let change = if next_char == Some(pair.close) {
        // return transaction that moves past close
        (cursor, cursor, None) // no-op
    } else {
        return None;
    };

    let next_range = get_next_range(doc, range, 0);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}

/// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_insert_same(doc: &Rope, range: &Range, pair: &Pair) -> Option<(Change, Range)> {
    let cursor = range.cursor(doc.slice(..));
    let mut len_inserted = 0;
    let next_char = doc.get_char(cursor);

    let change = if next_char == Some(pair.open) {
        // return transaction that moves past close
        (cursor, cursor, None) // no-op
    } else {
        if !pair.should_close(doc, range) {
            return None;
        }

        let pair_str = Tendril::from_iter([pair.open, pair.close]);
        len_inserted = 2;
        (cursor, cursor, Some(pair_str))
    };

    let next_range = get_next_range(doc, range, len_inserted);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}
