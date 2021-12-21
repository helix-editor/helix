//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

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

// [TODO] build this dynamically in language config. see #992
const OPEN_BEFORE: &str = "([{'\":;,> \n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}";
const CLOSE_BEFORE: &str = ")]}'\":;,> \n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}"; // includes space and newlines

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
pub fn hook(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
    debug!("autopairs hook selection: {:#?}", selection);

    for &(open, close) in PAIRS {
        if open == ch {
            if open == close {
                return Some(handle_same(doc, selection, open, CLOSE_BEFORE, OPEN_BEFORE));
            } else {
                return Some(handle_open(doc, selection, open, close, CLOSE_BEFORE));
            }
        }

        if close == ch {
            // && char_at pos == close
            return Some(handle_close(doc, selection, open, close));
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
        (1, Some(Direction::Forward)) => end_head - 1,
        (1, Some(Direction::Backward)) => end_head + 1,

        // if we are appending, the anchor stays where it is; only offset
        // for multiple range insertions
        (_, Some(Direction::Forward)) => start_range.anchor + offset,

        // when we are inserting in front of a selection, we need to move
        // the anchor over by however many characters were inserted overall
        (_, Some(Direction::Backward)) => start_range.anchor + offset + len_inserted,

        // None means 0 width
        _ => unreachable!(),
    };

    Range::new(end_anchor, end_head)
}

fn handle_open(
    doc: &Rope,
    selection: &Selection,
    open: char,
    close: char,
    close_before: &str,
) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        let len_inserted;

        let change = match next_char {
            Some(ch) if !close_before.contains(ch) => {
                len_inserted = open.len_utf8();
                (cursor, cursor, Some(Tendril::from_char(open)))
            }
            // None | Some(ch) if close_before.contains(ch) => {}
            _ => {
                // insert open & close
                let pair = Tendril::from_iter([open, close]);
                len_inserted = open.len_utf8() + close.len_utf8();
                (cursor, cursor, Some(pair))
            }
        };

        let next_range = get_next_range(start_range, offs, open, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

fn handle_close(doc: &Rope, selection: &Selection, _open: char, close: char) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let next_char = doc.get_char(cursor);
        let mut len_inserted = 0;

        let change = if next_char == Some(close) {
            // return transaction that moves past close
            (cursor, cursor, None) // no-op
        } else {
            len_inserted += close.len_utf8();
            (cursor, cursor, Some(Tendril::from_char(close)))
        };

        let next_range = get_next_range(start_range, offs, close, len_inserted);
        end_ranges.push(next_range);
        offs += len_inserted;

        change
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

/// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_same(
    doc: &Rope,
    selection: &Selection,
    token: char,
    close_before: &str,
    open_before: &str,
) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let cursor = start_range.cursor(doc.slice(..));
        let mut len_inserted = 0;

        let next_char = doc.get_char(cursor);
        let prev_char = prev_char(doc, cursor);

        let change = if next_char == Some(token) {
            //  return transaction that moves past close
            (cursor, cursor, None) // no-op
        } else {
            let mut pair = Tendril::with_capacity(2 * token.len_utf8() as u32);
            pair.push_char(token);

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if (next_char.is_none() || close_before.contains(next_char.unwrap()))
                && (prev_char.is_none() || open_before.contains(prev_char.unwrap()))
            {
                pair.push_char(token);
            }

            len_inserted += pair.len();
            (cursor, cursor, Some(pair))
        };

        let next_range = get_next_range(start_range, offs, token, len_inserted);
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
        expected_doc: &Rope,
        expected_sel: &Selection,
    ) {
        let trans = hook(&in_doc, &in_sel, ch).unwrap();
        let mut actual_doc = in_doc.clone();
        assert!(trans.apply(&mut actual_doc));
        assert_eq!(expected_doc, &actual_doc);
        assert_eq!(expected_sel, trans.selection().unwrap());
    }

    fn test_hooks_with_pairs<I, F, R>(
        in_doc: &Rope,
        in_sel: &Selection,
        pairs: I,
        get_expected_doc: F,
        actual_sel: &Selection,
    ) where
        I: IntoIterator<Item = &'static (char, char)>,
        F: Fn(char, char) -> R,
        R: Into<Rope>,
        Rope: From<R>,
    {
        pairs.into_iter().for_each(|(open, close)| {
            test_hooks(
                in_doc,
                in_sel,
                *open,
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

            test_hooks(&doc, &sel, *close, &doc, &expected_sel);
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

            test_hooks(&doc, &sel, *close, &doc, &expected_sel);
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

            test_hooks(&doc, &sel, *open, &expected_doc, &expected_sel);
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

            test_hooks(&doc, &sel, *open, &expected_doc, &expected_sel);
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

                test_hooks(&doc, &sel, *inner_open, &expected_doc, &expected_sel);
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

                test_hooks(&doc, &sel, *inner_open, &expected_doc, &expected_sel);
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
            test_hooks(&doc, &sel, *close, &expected_doc, &expected_sel);
        }
    }

    /// foo[ wor]d -> insert ( -> foo([) wor]d
    #[test]
    fn test_insert_open_trailing_word_with_selection() {
        test_hooks_with_pairs(
            &Rope::from("foo word"),
            &Selection::single(7, 3),
            differing_pairs(),
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
            |open, close| format!("word{}{}", open, close),
            &expected_sel,
        );

        test_hooks_with_pairs(
            &doc,
            &sel,
            matching_pairs(),
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
            |open, close| format!("{}{}\n", open, close),
            &Selection::single(1, 2),
        );
    }
}
