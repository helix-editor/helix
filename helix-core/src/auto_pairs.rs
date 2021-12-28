//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use std::{collections::HashMap, rc::Rc};

use crate::{movement::Direction, Range, Rope, Selection, Tendril, Transaction};
use log::debug;
use smallvec::SmallVec;

// Heavily based on https://github.com/codemirror/closebrackets/

pub const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('\'', '\''),
    ('"', '"'),
    ('`', '`'),
];

pub type PairPredicate = Box<dyn Fn(&Rope, &Range) -> bool>;

/// The type that represents the collection of auto pairs,
/// keyed by the opener.
pub struct AutoPairs(HashMap<char, Rc<AutoPair>>);

/// Represents the config for a particular pairing.
pub struct AutoPair {
    open: char,
    close: char,
    conditions: Vec<PairPredicate>,
}

impl AutoPair {
    /// true if open == close
    pub fn same(&self) -> bool {
        self.open == self.close
    }

    /// true if all of the pair's conditions hold for the given document and range
    pub fn should_close(&self, doc: &Rope, range: &Range) -> bool {
        self.conditions.iter().all(|cond| cond(doc, range))
    }
}

impl AutoPairs {
    /// Make a new AutoPairs set with the given pairs and default conditions.
    pub fn new<'a, V: 'a>(pairs: V) -> Self
    where
        V: IntoIterator<Item = &'a (char, char)>,
    {
        let mut auto_pairs = HashMap::new();

        for (open, close) in pairs.into_iter() {
            let mut conditions: Vec<PairPredicate> = vec![Box::new(Self::next_is_not_alpha)];

            if open == close {
                conditions.push(Box::new(Self::prev_is_not_alpha));
            }

            let auto_pair = Rc::new(AutoPair {
                open: *open,
                close: *close,
                conditions,
            });

            auto_pairs.insert(*open, auto_pair.clone());

            if open != close {
                auto_pairs.insert(*close, auto_pair);
            }
        }

        Self(auto_pairs)
    }

    pub fn get(&self, ch: char) -> Option<&AutoPair> {
        self.0.get(&ch).map(|rc| rc.as_ref())
    }

    pub fn next_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        next_char.map(|c| !c.is_alphabetic()).unwrap_or(true)
    }

    pub fn prev_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        let prev_char = prev_char(doc, cursor);
        prev_char.map(|c| !c.is_alphabetic()).unwrap_or(true)
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
pub fn hook(
    doc: &Rope,
    selection: &Selection,
    ch: char,
    pairs: Option<AutoPairs>,
) -> Option<Transaction> {
    debug!("autopairs hook selection: {:#?}", selection);
    let pairs = pairs.unwrap_or_else(|| AutoPairs::new(PAIRS));

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
fn get_next_range(
    start_range: &Range,
    offset: usize,
    typed_char: char,
    len_inserted: usize,
) -> Range {
    let end_head = start_range.head + offset + typed_char.len_utf8();

    let end_anchor = match (start_range.len(), start_range.direction()) {
        // if we have a zero width cursor, it shifts to the same number
        (0, _) => end_head,

        // if we are inserting for a regular one-width cursor, the anchor
        // moves with the head
        (1, Direction::Forward) => end_head - 1,
        (1, Direction::Backward) => end_head + 1,

        // if we are appending, the anchor stays where it is; only offset
        // for multiple range insertions
        (_, Direction::Forward) => start_range.anchor + offset,

        // when we are inserting in front of a selection, we need to move
        // the anchor over by however many characters were inserted overall
        (_, Direction::Backward) => start_range.anchor + offset + len_inserted,
    };

    Range::new(end_anchor, end_head)
}

fn handle_open(doc: &Rope, selection: &Selection, pair: &AutoPair) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        let len_inserted;

        let change = match next_char {
            Some(_) if !pair.should_close(doc, start_range) => {
                len_inserted = pair.open.len_utf8();
                (cursor, cursor, Some(Tendril::from_char(pair.open)))
            }
            _ => {
                // insert open & close
                let pair_str = Tendril::from_iter([pair.open, pair.close]);
                len_inserted = pair.open.len_utf8() + pair.close.len_utf8();
                (cursor, cursor, Some(pair_str))
            }
        };

        let next_range = get_next_range(start_range, offs, pair.open, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

fn handle_close(doc: &Rope, selection: &Selection, pair: &AutoPair) -> Transaction {
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
            len_inserted += pair.close.len_utf8();
            (cursor, cursor, Some(Tendril::from_char(pair.close)))
        };

        let next_range = get_next_range(start_range, offs, pair.close, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

/// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_same(doc: &Rope, selection: &Selection, pair: &AutoPair) -> Transaction {
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
            let mut pair_str = Tendril::with_capacity(2 * pair.open.len_utf8() as u32);
            pair_str.push_char(pair.open);

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if pair.should_close(doc, start_range) {
                pair_str.push_char(pair.open);
            }

            len_inserted += pair_str.len();
            (cursor, cursor, Some(pair_str))
        };

        let next_range = get_next_range(start_range, offs, pair.open, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

#[cfg(test)]
mod test {
    use super::*;
    use smallvec::smallvec;

    fn differing_pairs() -> impl Iterator<Item = &'static (char, char)> {
        PAIRS.iter().filter(|(open, close)| open != close)
    }

    fn matching_pairs() -> impl Iterator<Item = &'static (char, char)> {
        PAIRS.iter().filter(|(open, close)| open == close)
    }

    fn test_hooks(
        in_doc: &Rope,
        in_sel: &Selection,
        ch: char,
        pairs: Option<&[(char, char)]>,
        expected_doc: &Rope,
        expected_sel: &Selection,
    ) {
        let pairs = pairs
            .map(|p| AutoPairs::new(p))
            .or_else(|| Some(AutoPairs::new(PAIRS)));
        let trans = hook(&in_doc, &in_sel, ch, pairs).unwrap();
        let mut actual_doc = in_doc.clone();
        assert!(trans.apply(&mut actual_doc));
        assert_eq!(expected_doc, &actual_doc);
        assert_eq!(expected_sel, trans.selection().unwrap());
    }

    fn test_hooks_with_pairs<I, F, R>(
        in_doc: &Rope,
        in_sel: &Selection,
        test_pairs: I,
        pairs: Option<&[(char, char)]>,
        get_expected_doc: F,
        actual_sel: &Selection,
    ) where
        I: IntoIterator<Item = &'static (char, char)>,
        F: Fn(char, char) -> R,
        R: Into<Rope>,
        Rope: From<R>,
    {
        test_pairs.into_iter().for_each(|(open, close)| {
            test_hooks(
                in_doc,
                in_sel,
                *open,
                pairs,
                &Rope::from(get_expected_doc(*open, *close)),
                actual_sel,
            )
        });
    }

    // [] indicates range

    /// [] -> insert ( -> ([])
    #[test]
    fn test_insert_blank() {
        test_hooks_with_pairs(
            &Rope::new(),
            &Selection::single(1, 0),
            PAIRS,
            None,
            |open, close| format!("{}{}", open, close),
            &Selection::single(2, 1),
        );
    }

    /// [] -> append ( -> ([])
    #[test]
    fn test_append_blank() {
        test_hooks_with_pairs(
            // this is what happens when you have a totally blank document and then append
            &Rope::from("\n\n"),
            &Selection::single(0, 2),
            PAIRS,
            None,
            |open, close| format!("\n{}{}\n", open, close),
            &Selection::single(0, 3),
        );
    }

    /// []              ([])
    /// [] -> insert -> ([])
    /// []              ([])
    #[test]
    fn test_insert_blank_multi_cursor() {
        test_hooks_with_pairs(
            &Rope::from("\n\n\n"),
            &Selection::new(
                smallvec!(Range::new(1, 0), Range::new(2, 1), Range::new(3, 2),),
                0,
            ),
            PAIRS,
            None,
            |open, close| {
                format!(
                    "{open}{close}\n{open}{close}\n{open}{close}\n",
                    open = open,
                    close = close
                )
            },
            &Selection::new(
                smallvec!(Range::new(2, 1), Range::new(5, 4), Range::new(8, 7),),
                0,
            ),
        );
    }

    /// fo[o] -> append ( -> fo[o(])
    #[test]
    fn test_append() {
        test_hooks_with_pairs(
            &Rope::from("foo\n"),
            &Selection::single(2, 4),
            differing_pairs(),
            None,
            |open, close| format!("foo{}{}\n", open, close),
            &Selection::single(2, 5),
        );
    }

    /// fo[o]                fo[o(])
    /// fo[o] -> append ( -> fo[o(])
    /// fo[o]                fo[o(])
    #[test]
    fn test_append_multi() {
        test_hooks_with_pairs(
            &Rope::from("foo\nfoo\nfoo\n"),
            &Selection::new(
                smallvec!(Range::new(2, 4), Range::new(6, 8), Range::new(10, 12)),
                0,
            ),
            differing_pairs(),
            None,
            |open, close| {
                format!(
                    "foo{open}{close}\nfoo{open}{close}\nfoo{open}{close}\n",
                    open = open,
                    close = close
                )
            },
            &Selection::new(
                smallvec!(Range::new(2, 5), Range::new(8, 11), Range::new(14, 17)),
                0,
            ),
        );
    }

    /// ([]) -> insert ) -> ()[]
    #[test]
    fn test_insert_close_inside_pair() {
        for (open, close) in PAIRS {
            let doc = Rope::from(format!("{}{}", open, close));

            test_hooks(
                &doc,
                &Selection::single(2, 1),
                *close,
                None,
                &doc,
                &Selection::single(3, 2),
            );
        }
    }

    /// [(]) -> append ) -> [()]
    #[test]
    fn test_append_close_inside_pair() {
        for (open, close) in PAIRS {
            let doc = Rope::from(format!("{}{}\n", open, close));

            test_hooks(
                &doc,
                &Selection::single(0, 2),
                *close,
                None,
                &doc,
                &Selection::single(0, 3),
            );
        }
    }

    /// ([])                ()[]
    /// ([]) -> insert ) -> ()[]
    /// ([])                ()[]
    #[test]
    fn test_insert_close_inside_pair_multi_cursor() {
        let sel = Selection::new(
            smallvec!(Range::new(2, 1), Range::new(5, 4), Range::new(8, 7),),
            0,
        );

        let expected_sel = Selection::new(
            smallvec!(Range::new(3, 2), Range::new(6, 5), Range::new(9, 8),),
            0,
        );

        for (open, close) in PAIRS {
            let doc = Rope::from(format!(
                "{open}{close}\n{open}{close}\n{open}{close}\n",
                open = open,
                close = close
            ));

            test_hooks(&doc, &sel, *close, None, &doc, &expected_sel);
        }
    }

    /// [(])                [()]
    /// [(]) -> append ) -> [()]
    /// [(])                [()]
    #[test]
    fn test_append_close_inside_pair_multi_cursor() {
        let sel = Selection::new(
            smallvec!(Range::new(0, 2), Range::new(3, 5), Range::new(6, 8),),
            0,
        );

        let expected_sel = Selection::new(
            smallvec!(Range::new(0, 3), Range::new(3, 6), Range::new(6, 9),),
            0,
        );

        for (open, close) in PAIRS {
            let doc = Rope::from(format!(
                "{open}{close}\n{open}{close}\n{open}{close}\n",
                open = open,
                close = close
            ));

            test_hooks(&doc, &sel, *close, None, &doc, &expected_sel);
        }
    }

    /// ([]) -> insert ( -> (([]))
    #[test]
    fn test_insert_open_inside_pair() {
        let sel = Selection::single(2, 1);
        let expected_sel = Selection::single(3, 2);

        for (open, close) in differing_pairs() {
            let doc = Rope::from(format!("{}{}", open, close));
            let expected_doc = Rope::from(format!(
                "{open}{open}{close}{close}",
                open = open,
                close = close
            ));

            test_hooks(&doc, &sel, *open, None, &expected_doc, &expected_sel);
        }
    }

    /// [word(]) -> append ( -> [word((]))
    #[test]
    fn test_append_open_inside_pair() {
        let sel = Selection::single(0, 6);
        let expected_sel = Selection::single(0, 7);

        for (open, close) in differing_pairs() {
            let doc = Rope::from(format!("word{}{}", open, close));
            let expected_doc = Rope::from(format!(
                "word{open}{open}{close}{close}",
                open = open,
                close = close
            ));

            test_hooks(&doc, &sel, *open, None, &expected_doc, &expected_sel);
        }
    }

    /// ([]) -> insert " -> ("[]")
    #[test]
    fn test_insert_nested_open_inside_pair() {
        let sel = Selection::single(2, 1);
        let expected_sel = Selection::single(3, 2);

        for (outer_open, outer_close) in differing_pairs() {
            let doc = Rope::from(format!("{}{}", outer_open, outer_close,));

            for (inner_open, inner_close) in matching_pairs() {
                let expected_doc = Rope::from(format!(
                    "{}{}{}{}",
                    outer_open, inner_open, inner_close, outer_close
                ));

                test_hooks(&doc, &sel, *inner_open, None, &expected_doc, &expected_sel);
            }
        }
    }

    /// [(]) -> append " -> [("]")
    #[test]
    fn test_append_nested_open_inside_pair() {
        let sel = Selection::single(0, 2);
        let expected_sel = Selection::single(0, 3);

        for (outer_open, outer_close) in differing_pairs() {
            let doc = Rope::from(format!("{}{}", outer_open, outer_close,));

            for (inner_open, inner_close) in matching_pairs() {
                let expected_doc = Rope::from(format!(
                    "{}{}{}{}",
                    outer_open, inner_open, inner_close, outer_close
                ));

                test_hooks(&doc, &sel, *inner_open, None, &expected_doc, &expected_sel);
            }
        }
    }

    /// []word -> insert ( -> ([]word
    #[test]
    fn test_insert_open_before_non_pair() {
        test_hooks_with_pairs(
            &Rope::from("word"),
            &Selection::single(1, 0),
            PAIRS,
            None,
            |open, _| format!("{}word", open),
            &Selection::single(2, 1),
        )
    }

    /// [wor]d -> insert ( -> ([wor]d
    #[test]
    fn test_insert_open_with_selection() {
        test_hooks_with_pairs(
            &Rope::from("word"),
            &Selection::single(3, 0),
            PAIRS,
            None,
            |open, _| format!("{}word", open),
            &Selection::single(4, 1),
        )
    }

    /// [wor]d -> append ) -> [wor)]d
    #[test]
    fn test_append_close_inside_non_pair_with_selection() {
        let sel = Selection::single(0, 4);
        let expected_sel = Selection::single(0, 5);

        for (_, close) in PAIRS {
            let doc = Rope::from("word");
            let expected_doc = Rope::from(format!("wor{}d", close));
            test_hooks(&doc, &sel, *close, None, &expected_doc, &expected_sel);
        }
    }

    /// foo[ wor]d -> insert ( -> foo([) wor]d
    #[test]
    fn test_insert_open_trailing_word_with_selection() {
        test_hooks_with_pairs(
            &Rope::from("foo word"),
            &Selection::single(7, 3),
            differing_pairs(),
            None,
            |open, close| format!("foo{}{} word", open, close),
            &Selection::single(9, 4),
        )
    }

    /// we want pairs that are *not* the same char to be inserted after
    /// a non-pair char, for cases like functions, but for pairs that are
    /// the same char, we want to *not* insert a pair to handle cases like "I'm"
    ///
    /// word[]  -> insert ( -> word([])
    /// word[]  -> insert ' -> word'[]
    #[test]
    fn test_insert_open_after_non_pair() {
        let doc = Rope::from("word");
        let sel = Selection::single(5, 4);
        let expected_sel = Selection::single(6, 5);

        test_hooks_with_pairs(
            &doc,
            &sel,
            differing_pairs(),
            None,
            |open, close| format!("word{}{}", open, close),
            &expected_sel,
        );

        test_hooks_with_pairs(
            &doc,
            &sel,
            matching_pairs(),
            None,
            |open, _| format!("word{}", open),
            &expected_sel,
        );
    }

    /// appending with only a cursor should stay a cursor
    ///
    /// [] -> append to end "foo -> "foo[]"
    #[test]
    fn test_append_single_cursor() {
        test_hooks_with_pairs(
            &Rope::from("\n"),
            &Selection::single(0, 1),
            PAIRS,
            None,
            |open, close| format!("{}{}\n", open, close),
            &Selection::single(1, 2),
        );
    }
}
