//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{Range, Rope, Selection, Tendril, Transaction};
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
// * do not reduce to cursors; use whole selections, and surround with pair
// * change to multi character pairs to handle cases like placing the cursor in the
//   middle of triple quotes, and more exotic pairs like Jinja's {% %}

#[must_use]
pub fn hook(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
    debug!("autopairs hook selection: {:#?}", selection);

    let cursors = selection.clone().cursors(doc.slice(..));

    for &(open, close) in PAIRS {
        if open == ch {
            if open == close {
                return Some(handle_same(doc, &cursors, open, CLOSE_BEFORE, OPEN_BEFORE));
            } else {
                return Some(handle_open(doc, &cursors, open, close, CLOSE_BEFORE));
            }
        }

        if close == ch {
            // && char_at pos == close
            return Some(handle_close(doc, &cursors, open, close));
        }
    }

    None
}

fn next_char(doc: &Rope, pos: usize) -> Option<char> {
    if pos >= doc.len_chars() {
        return None;
    }
    Some(doc.char(pos))
}

fn prev_char(doc: &Rope, mut pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }

    pos -= 1;

    next_char(doc, pos)
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
        let start_head = start_range.head;

        let next = next_char(doc, start_head);
        let end_head = start_head + offs + open.len_utf8();

        let end_anchor = if start_range.is_empty() {
            end_head
        } else {
            start_range.anchor + offs
        };

        end_ranges.push(Range::new(end_anchor, end_head));

        match next {
            Some(ch) if !close_before.contains(ch) => {
                offs += 1;
                // TODO: else return (use default handler that inserts open)
                (start_head, start_head, Some(Tendril::from_char(open)))
            }
            // None | Some(ch) if close_before.contains(ch) => {}
            _ => {
                // insert open & close
                let mut pair = Tendril::with_capacity(2);
                pair.push_char(open);
                pair.push_char(close);

                offs += 2;

                (start_head, start_head, Some(pair))
            }
        }
    });

    let t = transaction.with_selection(Selection::new(end_ranges, selection.primary_index()));
    debug!("auto pair transaction: {:#?}", t);
    t
}

fn handle_close(doc: &Rope, selection: &Selection, _open: char, close: char) -> Transaction {
    let mut end_ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |start_range| {
        let start_head = start_range.head;
        let next = next_char(doc, start_head);
        let end_head = start_head + offs + close.len_utf8();

        let end_anchor = if start_range.is_empty() {
            end_head
        } else {
            start_range.anchor + offs
        };

        end_ranges.push(Range::new(end_anchor, end_head));

        if next == Some(close) {
            // return transaction that moves past close
            (start_head, start_head, None) // no-op
        } else {
            offs += close.len_utf8();

            // TODO: else return (use default handler that inserts close)
            (start_head, start_head, Some(Tendril::from_char(close)))
        }
    });

    transaction.with_selection(Selection::new(end_ranges, selection.primary_index()))
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
        let start_head = start_range.head;
        let end_head = start_head + offs + token.len_utf8();

        let end_anchor = if start_range.is_empty() {
            end_head
        } else {
            start_range.anchor + offs
        };

        // if selection, retain anchor, if cursor, move over
        end_ranges.push(Range::new(end_anchor, end_head));

        let next = next_char(doc, start_head);
        let prev = prev_char(doc, start_head);

        if next == Some(token) {
            //  return transaction that moves past close
            (start_head, start_head, None) // no-op
        } else {
            let mut pair = Tendril::with_capacity(2);
            pair.push_char(token);

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if (next.is_none() || close_before.contains(next.unwrap()))
                && (prev.is_none() || open_before.contains(prev.unwrap()))
            {
                pair.push_char(token);
            }

            offs += pair.len();

            (start_head, start_head, Some(pair))
        }
    });

    transaction.with_selection(Selection::new(end_ranges, selection.primary_index()))
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
            &Selection::single(1, 1),
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
                smallvec!(Range::point(1), Range::point(4), Range::point(7),),
                0,
            ),
        );
    }

    // [TODO] broken until it works with selections
    /// fo[o] -> append ( -> fo[o(])
    #[ignore]
    #[test]
    fn test_append() {
        test_hooks_with_pairs(
            &Rope::from("foo"),
            &Selection::single(2, 4),
            PAIRS,
            |open, close| format!("foo{}{}", open, close),
            &Selection::single(2, 5),
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
                &Selection::point(2),
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
            // smallvec!(Range::new(3, 2), Range::new(6, 5), Range::new(9, 8),),
            smallvec!(Range::point(2), Range::point(5), Range::point(8),),
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
        let expected_sel = Selection::point(2);

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

    /// ([]) -> insert " -> ("[]")
    #[test]
    fn test_insert_nested_open_inside_pair() {
        let sel = Selection::single(2, 1);
        let expected_sel = Selection::point(2);

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
            &Selection::point(1),
        )
    }

    // [TODO] broken until it works with selections
    /// [wor]d -> insert ( -> ([wor]d
    #[test]
    #[ignore]
    fn test_insert_open_with_selection() {
        test_hooks_with_pairs(
            &Rope::from("word"),
            &Selection::single(0, 4),
            PAIRS,
            |open, _| format!("{}word", open),
            &Selection::single(1, 5),
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
        let expected_sel = Selection::point(5);

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
}
