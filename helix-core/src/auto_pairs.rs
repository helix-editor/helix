//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{movement::Direction, Change, Deletion, Range, Rope, Tendril};
use helix_stdx::rope::RopeSliceExt;
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
        let next_char = doc.get_char(cursor).ok();
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
    } else if ch.is_whitespace() {
        return handle_insert_whitespace(doc, range, ch, pairs);
    }

    None
}

#[must_use]
pub fn hook_delete(doc: &Rope, range: &Range, pairs: &AutoPairs) -> Option<(Deletion, Range)> {
    log::trace!("autopairs delete hook range: {:#?}", range);

    let text = doc.slice(..);
    let cursor = range.cursor(text);

    let cur = doc.get_char(cursor).ok()?;
    let prev = prev_char(doc, cursor)?;

    // check for whitespace surrounding a pair
    if doc.len() >= 4 && prev.is_whitespace() && cur.is_whitespace() {
        let second_prev = doc.get_char(text.nth_prev_grapheme_boundary(cursor, 2)).ok()?;
        let second_next = doc.get_char(text.next_grapheme_boundary(cursor)).ok()?;
        log::debug!("second_prev: {}, second_next: {}", second_prev, second_next);

        if let Some(pair) = pairs.get(second_prev) {
            if pair.open == second_prev && pair.close == second_next {
                return handle_delete(doc, range);
            }
        }
    }

    let pair = pairs.get(cur)?;

    if pair.open != prev || pair.close != cur {
        return None;
    }

    handle_delete(doc, range)
}

pub fn handle_delete(doc: &Rope, range: &Range) -> Option<(Deletion, Range)> {
    let text = doc.slice(..);
    let cursor = range.cursor(text);

    let end_next = text.next_grapheme_boundary(cursor);
    let end_prev = text.prev_grapheme_boundary(cursor);

    let delete = (end_prev, end_next);
    let size_delete = end_next - end_prev;
    let next_head = text.next_grapheme_boundary(range.head) - size_delete;

    // if the range is a single grapheme cursor, we do not want to shrink the
    // range, just move it, so we only subtract the size of the closing pair char
    let next_anchor = match (range.direction(), range.is_single_grapheme(text)) {
        // single grapheme forward needs to move, but only the width of the
        // character under the cursor, which is the closer
        (Direction::Forward, true) => range.anchor - (end_next - cursor),
        (Direction::Backward, true) => range.anchor - (cursor - end_prev),

        (Direction::Forward, false) => range.anchor,
        (Direction::Backward, false) => range.anchor - size_delete,
    };

    let next_range = Range::new(next_anchor, next_head);

    log::trace!(
        "auto pair delete: {:?}, range: {:?}, next_range: {:?}, text len: {}",
        delete,
        range,
        next_range,
        text.len()
    );

    Some((delete, next_range))
}

fn handle_insert_whitespace(
    doc: &Rope,
    range: &Range,
    ch: char,
    pairs: &AutoPairs,
) -> Option<(Change, Range)> {
    let text = doc.slice(..);
    let cursor = range.cursor(text);
    let cur = doc.get_char(cursor).ok()?;
    let prev = prev_char(doc, cursor)?;
    let pair = pairs.get(cur)?;

    if pair.open != prev || pair.close != cur {
        return None;
    }

    let whitespace_pair = Pair {
        open: ch,
        close: ch,
    };

    handle_insert_same(doc, range, &whitespace_pair)
}

fn prev_char(doc: &Rope, pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }

    // `pos - 1` would land mid-codepoint when the preceding char is multi-byte,
    // so walk the char iterator instead.
    doc.slice(..).chars_at(pos).prev()
}

/// calculate what the resulting range should be for an auto pair insertion
///
/// `offset` is the byte length of the inserted opening character (where the
/// cursor should land, between the open and close) and `len_inserted` is the
/// total byte length inserted (open + close). For ASCII pairs these are 1 and
/// 2, but both must be byte-aware so the resulting cursor never lands inside a
/// multi-byte codepoint.
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
    if start_range.head == doc.len() && start_range.anchor == doc.len() {
        return Range::new(start_range.anchor + offset, start_range.head + offset);
    }

    let doc_slice = doc.slice(..);
    let single_grapheme = start_range.is_single_grapheme(doc_slice);

    // just skip over graphemes
    if len_inserted == 0 {
        let end_anchor = if single_grapheme {
            doc_slice.next_grapheme_boundary(start_range.anchor)

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
            doc_slice.next_grapheme_boundary(start_range.head),
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
        start_range.head + offset
    } else {
        // We must have a forward cursor, which means we must move to the
        // other end of the grapheme to get to where the new characters
        // are inserted, then move the head to where it should be
        let prev_bound = doc_slice.prev_grapheme_boundary(start_range.head);
        log::trace!("prev_bound: {}, len_inserted: {}", prev_bound, len_inserted);

        prev_bound + len_inserted
    };

    let end_anchor = match (start_range.len(), start_range.direction()) {
        // if we have a zero width cursor, it shifts to the same number
        (0, _) => end_head,

        // If we are inserting for a regular one-width cursor, the anchor
        // moves with the head. This is the fast path for ASCII.
        (1, Direction::Forward) => end_head - offset,
        (1, Direction::Backward) => end_head + (len_inserted - offset),

        (_, Direction::Forward) => {
            if single_grapheme {
                doc.slice(..).prev_grapheme_boundary(start_range.head) + offset

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
                doc.slice(..).prev_grapheme_boundary(start_range.anchor) + len_inserted
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
    let next_char = doc.get_char(cursor).ok();
    let len_inserted;

    let change = match next_char {
        Some(_) if !pair.should_close(doc, range) => {
            return None;
        }
        _ => {
            // insert open & close
            let pair_str = Tendril::from_iter([pair.open, pair.close]);
            len_inserted = pair.open.len_utf8() + pair.close.len_utf8();
            (cursor, cursor, Some(pair_str))
        }
    };

    let next_range = get_next_range(doc, range, pair.open.len_utf8(), len_inserted);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}

fn handle_insert_close(doc: &Rope, range: &Range, pair: &Pair) -> Option<(Change, Range)> {
    let cursor = range.cursor(doc.slice(..));
    let next_char = doc.get_char(cursor).ok();

    let change = if next_char == Some(pair.close) {
        // return transaction that moves past close
        (cursor, cursor, None) // no-op
    } else {
        return None;
    };

    // Nothing is inserted (the cursor just skips over the existing close), so
    // the offset is irrelevant — the `len_inserted == 0` path steps by grapheme.
    let next_range = get_next_range(doc, range, 0, 0);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}

/// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_insert_same(doc: &Rope, range: &Range, pair: &Pair) -> Option<(Change, Range)> {
    let cursor = range.cursor(doc.slice(..));
    let mut len_inserted = 0;
    let next_char = doc.get_char(cursor).ok();

    let change = if next_char == Some(pair.open) {
        // return transaction that moves past close
        (cursor, cursor, None) // no-op
    } else {
        if !pair.should_close(doc, range) {
            return None;
        }

        let pair_str = Tendril::from_iter([pair.open, pair.close]);
        len_inserted = pair.open.len_utf8() + pair.close.len_utf8();
        (cursor, cursor, Some(pair_str))
    };

    let next_range = get_next_range(doc, range, pair.open.len_utf8(), len_inserted);
    let result = (change, next_range);

    log::debug!("auto pair change: {:#?}", &result);

    Some(result)
}

#[cfg(test)]
mod test {
    use super::*;

    /// Regression: `prev_char` must walk by char (not byte) so that a multi-byte
    /// char preceding `pos` is correctly returned instead of returning a mid-
    /// codepoint error from `get_char(pos - 1)`.
    #[test]
    fn test_prev_char_multibyte() {
        // Bytes: 「(0..3) a(3) 」(4..7)
        let doc = Rope::from("「a」");
        assert_eq!(prev_char(&doc, 0), None);
        assert_eq!(prev_char(&doc, 3), Some('「'));
        assert_eq!(prev_char(&doc, 4), Some('a'));
        assert_eq!(prev_char(&doc, 7), Some('」'));
    }

    /// Regression: auto-pair insertion of a pair whose open and/or close char is
    /// multi-byte must account for the actual byte length of the inserted text.
    /// Previously hardcoded to `len_inserted = 2`, which placed the cursor in
    /// the middle of a multi-byte open character.
    #[test]
    fn test_handle_insert_open_multibyte_pair() {
        // Pair 「」 has 3-byte open and 3-byte close.
        let pair = Pair {
            open: '「',
            close: '」',
        };
        let doc = Rope::from("");
        let range = Range::point(0);
        let (change, _next_range) = handle_insert_open(&doc, &range, &pair).unwrap();
        assert_eq!(change.0, 0);
        assert_eq!(change.1, 0);
        let inserted = change.2.expect("expected an insertion");
        assert_eq!(inserted.as_bytes().len(), 6);
    }
}
