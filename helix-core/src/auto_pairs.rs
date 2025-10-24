//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{graphemes, movement::Direction, Range, Rope, Selection, Tendril, Transaction};
use std::collections::HashMap;

use smallvec::SmallVec;

// Heavily based on https://github.com/codemirror/closebrackets/
pub const DEFAULT_PAIRS: &[(&str, &str)] = &[
    ("(", ")"),
    ("{", "}"),
    ("[", "]"),
    ("'", "'"),
    ("\"", "\""),
    ("`", "`"),
];

/// The type that represents the collection of auto pairs,
/// keyed by both opener and closer.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoPairs(HashMap<char, AutoPairsNode>);

#[derive(Debug, Clone, PartialEq)]
struct AutoPairsInternal(HashMap<Option<char>, AutoPairsNode>);

#[derive(Debug, Clone, PartialEq)]
enum AutoPairsNode {
    Terminal(Pair),
    Internal(AutoPairsInternal),
}

impl AutoPairs {
    pub fn new<'a, V, A>(pairs: V) -> Self
    where
        V: IntoIterator<Item = A> + 'a,
        A: Into<Pair>,
    {
        pairs
            .into_iter()
            .map(Into::<Pair>::into)
            .fold(AutoPairs::empty(), |acc, pair| {
                acc.inserting(pair.clone(), pair.open.chars().rev())
                    .inserting(pair.clone(), pair.close.chars())
            })
    }

    fn empty() -> Self {
        AutoPairs(HashMap::new())
    }

    fn inserting<T: Iterator<Item = char>>(self, pair: Pair, mut source: T) -> Self {
        match (self, source.next()) {
            (autopairs, None) => autopairs,
            (AutoPairs(mut hash_map), Some(ch)) => {
                let ch_next = source.next();
                match (hash_map.remove(&ch), ch_next) {
                    (Some(old_node), _) => {
                        let _ = hash_map.insert(ch, old_node.inserting(pair, source, ch_next));
                    }
                    (None, Some(_)) => {
                        let _ = hash_map.insert(
                            ch,
                            AutoPairsNode::Internal(
                                AutoPairsInternal::empty().inserting(pair, source, ch_next),
                            ),
                        );
                    }
                    (None, None) => {
                        let _ = hash_map.insert(ch, AutoPairsNode::Terminal(pair));
                    }
                };
                AutoPairs(hash_map)
            }
        }
    }

    pub fn get(&self, doc: &Rope, cursor: usize, ch: char) -> Option<&Pair> {
        let AutoPairs(hash_map) = self;
        let pairs_internal = hash_map.get(&ch)?;

        match pairs_internal {
            AutoPairsNode::Terminal(pair) => Some(pair),
            AutoPairsNode::Internal(auto_pairs_internal) => auto_pairs_internal
                .get(iterate_backwards_from_index(doc, cursor))
                .or_else(|| auto_pairs_internal.get(iterate_forward_from_index(doc, cursor))),
        }
    }
}

impl Default for AutoPairs {
    fn default() -> Self {
        AutoPairs::new(DEFAULT_PAIRS.iter())
    }
}

impl AutoPairsInternal {
    fn empty() -> Self {
        Self(HashMap::new())
    }

    fn inserting<T: Iterator<Item = char>>(
        self,
        pair: Pair,
        mut tail: T,
        head: Option<char>,
    ) -> Self {
        match (self, head) {
            (AutoPairsInternal(mut hash_map), None) => {
                let _ = hash_map.insert(None, AutoPairsNode::Terminal(pair));
                AutoPairsInternal(hash_map)
            }
            (AutoPairsInternal(mut hash_map), Some(ch)) => match hash_map.remove(&Some(ch)) {
                Some(AutoPairsNode::Terminal(old_pair)) => {
                    let new_head = tail.next();
                    let rec = AutoPairsNode::Terminal(old_pair).inserting(pair, tail, new_head);
                    let _ = hash_map.insert(Some(ch), rec);
                    AutoPairsInternal(hash_map)
                }
                Some(AutoPairsNode::Internal(AutoPairsInternal(old_hash_map))) => {
                    let new_head = tail.next();
                    let rec = AutoPairsNode::Internal(
                        AutoPairsInternal(old_hash_map).inserting(pair, tail, new_head),
                    );
                    let _ = hash_map.insert(Some(ch), rec);
                    AutoPairsInternal(hash_map)
                }
                None => {
                    let new_head = tail.next();
                    let rec = AutoPairsNode::Internal(
                        AutoPairsInternal::empty().inserting(pair, tail, new_head),
                    );
                    let _ = hash_map.insert(Some(ch), rec);
                    AutoPairsInternal(hash_map)
                }
            },
        }
    }

    fn get<T: Iterator<Item = char>>(&self, doc_chars: T) -> Option<&Pair> {
        let mut last_feasible: Option<&Pair> = None;
        let mut internal = self;
        for doc_ch_next in doc_chars {
            last_feasible = internal.get_none_or_unwrap().or(last_feasible);
            match internal.mapping().get(&Some(doc_ch_next)) {
                Some(AutoPairsNode::Terminal(pair)) => last_feasible = Some(pair),
                Some(AutoPairsNode::Internal(next_internal)) => internal = next_internal,
                None => break,
            }
        }
        internal.get_none_or_unwrap().or(last_feasible)
    }

    fn get_none_or_unwrap(&self) -> Option<&Pair> {
        let AutoPairsInternal(hash_map) = self;
        hash_map.get(&None).map(|node| match node {
            AutoPairsNode::Terminal(pair) => pair,
            AutoPairsNode::Internal(_) => unreachable!(),
        })
    }

    fn mapping(&self) -> &HashMap<Option<char>, AutoPairsNode> {
        &self.0
    }
}

impl AutoPairsNode {
    fn inserting<T: Iterator<Item = char>>(
        self,
        pair: Pair,
        mut tail: T,
        head: Option<char>,
    ) -> Self {
        match (self, head) {
            (AutoPairsNode::Terminal(old_pair), None) => {
                AutoPairsNode::Internal(AutoPairsInternal(HashMap::from([
                    (None, AutoPairsNode::Terminal(old_pair)),
                    (None, AutoPairsNode::Terminal(pair)),
                ])))
            }
            (AutoPairsNode::Terminal(old_pair), Some(ch)) => {
                let next_head = tail.next();
                let rec = AutoPairsNode::Internal(
                    AutoPairsInternal::empty().inserting(pair, tail, next_head),
                );

                AutoPairsNode::Internal(AutoPairsInternal(HashMap::from([
                    (None, AutoPairsNode::Terminal(old_pair)),
                    (Some(ch), rec),
                ])))
            }
            (AutoPairsNode::Internal(internal), _) => {
                AutoPairsNode::Internal(internal.inserting(pair, tail, head))
            }
        }
    }
}

/// Represents the config for a particular pairing.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub open: String,
    pub close: String,
}

impl Pair {
    /// true if open == close
    pub fn same(&self) -> bool {
        self.open == self.close
    }

    /// true if all of the pair's conditions hold for the given document and range
    pub fn should_close(&self, doc: &Rope, range: &Range) -> bool {
        let mut should_close = Self::next_is_not_alpha(self, doc, range);

        if self.same() {
            should_close &= Self::prev_is_not_alpha(self, doc, range);
        }

        should_close
    }

    pub fn next_is_not_alpha(&self, doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        self.first_after_pair(doc, cursor)
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true)
    }

    pub fn prev_is_not_alpha(&self, doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        self.first_before_pair(doc, cursor)
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true)
    }

    fn first_after_pair(&self, doc: &Rope, cursor: usize) -> Option<char> {
        let mut document_slice = iterate_forward_from_index(doc, cursor);
        for c in self.close.chars() {
            let doc_ch = document_slice.next();
            if doc_ch.is_none_or(|doc_ch| doc_ch != c) {
                return doc_ch;
            }
        }
        None
    }
    fn first_before_pair(&self, doc: &Rope, cursor: usize) -> Option<char> {
        let mut document_slice = iterate_backwards_from_index(doc, cursor);
        for c in self.open.chars().rev() {
            let doc_ch = document_slice.next();
            if doc_ch.is_none_or(|doc_ch| doc_ch != c) {
                return doc_ch;
            }
        }
        None
    }

    pub fn is_open_match_extending_with_char(&self, doc: &Rope, cursor: usize, ch: char) -> bool {
        let chars_open = self.open.chars().rev();
        let document_slice = iterate_backwards_from_index(doc, cursor);

        Self::after_char_matches_source(document_slice, ch, chars_open)
    }

    pub fn is_close_next_match(&self, doc: &Rope, cursor: usize) -> bool {
        let close_chars = self.close.chars();
        let document_slice = iterate_forward_from_index(doc, cursor);

        Self::matches_source(document_slice, close_chars)
    }

    fn last_char_of_open(&self) -> char {
        self.open.chars().next_back().unwrap()
    }

    fn after_char_matches_source<S: IntoIterator<Item = char>, G: IntoIterator<Item = char>>(
        ground: G,
        ch: char,
        source: S,
    ) -> bool {
        let mut chars = source.into_iter();
        let first_in_close = chars.next();
        if Some(ch) != first_in_close {
            return false;
        }

        Self::matches_source(ground, chars)
    }

    fn matches_source<S: IntoIterator<Item = char>, G: IntoIterator<Item = char>>(
        ground: G,
        source: S,
    ) -> bool {
        let mut ground = ground.into_iter();
        let chars = source.into_iter();

        for str_char in chars {
            let Some(src_char) = ground.next() else {
                return false;
            };
            if src_char != str_char {
                return false;
            }
        }
        true
    }
}

impl<O, C> From<&(O, C)> for Pair
where
    O: ToString,
    C: ToString,
{
    fn from((open, close): &(O, C)) -> Self {
        Self {
            open: open.to_string(),
            close: close.to_string(),
        }
    }
}

impl<O, C> From<(&O, &C)> for Pair
where
    O: ToString,
    C: ToString,
{
    fn from((open, close): (&O, &C)) -> Self {
        Self {
            open: open.to_string(),
            close: close.to_string(),
        }
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

#[must_use]
pub fn hook(doc: &Rope, selection: &Selection, ch: char, pairs: &AutoPairs) -> Option<Transaction> {
    log::trace!("autopairs hook selection: {:#?}", selection);
    let primary_cursor = selection.primary().cursor(doc.slice(..));

    if let Some(pair) = pairs.get(doc, primary_cursor, ch) {
        if pair.same() {
            return Some(handle_same(doc, selection, pair));
        } else if pair.is_open_match_extending_with_char(doc, primary_cursor, ch) {
            return Some(handle_open(doc, selection, pair));
        } else if pair.is_close_next_match(doc, primary_cursor) {
            return Some(handle_close(doc, selection, pair));
        }
    }
    None
}

fn iterate_backwards_from_index(doc: &Rope, index: usize) -> impl Iterator<Item = char> + use<'_> {
    doc.chars_at(index).reversed()
}

fn iterate_forward_from_index(doc: &Rope, index: usize) -> impl Iterator<Item = char> + use<'_> {
    doc.chars_at(index)
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
        prev_bound + offset + 2
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

        let change = match next_char {
            Some(_) if !pair.should_close(doc, start_range) => {
                len_inserted = 1;
                let mut tendril = Tendril::new();
                tendril.push(pair.last_char_of_open());
                (cursor, cursor, Some(tendril))
            }
            _ => {
                // insert open & close
                let change = {
                    let mut t = Tendril::new();
                    t.push(pair.last_char_of_open());
                    t.push_str(&pair.close);
                    t
                };
                len_inserted = 1 + pair.close.chars().count();
                (cursor, cursor, Some(change))
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
        let len_inserted;

        let change = if pair.is_close_next_match(doc, cursor) {
            len_inserted = 0;
            // return transaction that moves past close
            (cursor, cursor, None) // no-op
        } else {
            len_inserted = pair.close.chars().count();
            let mut tendril = Tendril::new();
            tendril.push_str(&pair.close);
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
        let len_inserted;

        let change = if pair.is_close_next_match(doc, cursor) {
            //  return transaction that moves past close
            len_inserted = 0;
            (cursor, cursor, None) // no-op
        } else {
            let mut pair_str = Tendril::new();
            pair_str.push(pair.last_char_of_open());

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if pair.should_close(doc, start_range) {
                pair_str.push_str(&pair.close);
            }

            len_inserted = pair_str.chars().count();
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

#[cfg(test)]
mod tests {
    use super::*;
    impl Pair {
        fn mock1() -> Self {
            Pair {
                open: "\\(".to_string(),
                close: "\\)".to_string(),
            }
        }
    }

    fn mock_rope1() -> Rope {
        Rope::from_str("a\\a\\)")
    }

    fn mock_rope2() -> Rope {
        Rope::from_str("``")
    }

    #[test]
    fn pair_matches() {
        let pair = Pair::mock1();
        let doc = mock_rope1();

        let cursor: usize = 2;
        assert!(
            pair.is_open_match_extending_with_char(&doc, cursor, '('),
            "pair: {pair:#?}\tdoc:{doc:#?}\tcursor: {cursor}"
        );

        let cursor: usize = 3;
        assert!(
            pair.is_close_next_match(&doc, cursor),
            "pair: {pair:#?}\tdoc:{doc:#?}\tcursor: {cursor}"
        );
    }

    #[test]
    fn autopairs_contains() {
        let autopairs = AutoPairs::default();
        let doc = mock_rope2();
        assert!(autopairs.get(&doc, 0, '`').is_some());
        assert!(autopairs.get(&doc, 1, '`').is_some());
        assert!(autopairs.get(&doc, 2, '`').is_some());
    }

    #[test]
    fn autopairs_creation() {
        let pairs = AutoPairs::new(&[("(", ")"), ("\\(", "\\)")]);
        let pairs_alt = AutoPairs::new(&[("\\(", "\\)"), ("(", ")")]);
        assert_eq!(pairs, pairs_alt, "left: {pairs:#?}\nright: {pairs_alt:#?}");
    }
}
