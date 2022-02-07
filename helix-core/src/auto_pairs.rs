//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{
    graphemes, movement::Direction, Range, Rope, RopeGraphemes, Selection, Tendril, Transaction,
};
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

fn is_single_grapheme(doc: &Rope, range: &Range) -> bool {
    let mut graphemes = RopeGraphemes::new(doc.slice(range.from()..range.to()));
    let first = graphemes.next();
    let second = graphemes.next();
    debug!("first: {:#?}, second: {:#?}", first, second);
    first.is_some() && second.is_none()
}

/// calculate what the resulting range should be for an auto pair insertion
fn get_next_range(
    doc: &Rope,
    start_range: &Range,
    offset: usize,
    typed_char: char,
    len_inserted: usize,
) -> Range {
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
            start_range.anchor + offset + typed_char.len_utf8(),
            start_range.head + offset + typed_char.len_utf8(),
        );
    }

    let single_grapheme = is_single_grapheme(doc, start_range);
    let doc_slice = doc.slice(..);

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
            start_range.anchor + offset + typed_char.len_utf8()
        } else {
            start_range.anchor + offset
        };

        return Range::new(
            end_anchor,
            start_range.head + offset + typed_char.len_utf8(),
        );
    }

    // If the head = 0, then we must be in insert mode with a backward
    // cursor, which implies the head will just move
    let end_head = if start_range.head == 0 || start_range.direction() == Direction::Backward {
        start_range.head + offset + typed_char.len_utf8()
    } else {
        // We must have a forward cursor, which means we must move to the
        // other end of the grapheme to get to where the new characters
        // are inserted, then move the head to where it should be
        let prev_bound = graphemes::prev_grapheme_boundary(doc_slice, start_range.head);
        debug!(
            "prev_bound: {}, offset: {}, len_inserted: {}",
            prev_bound, offset, len_inserted
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
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.head)
                    + typed_char.len_utf8()

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
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.anchor) + len_inserted
            } else {
                // when we are inserting in front of a selection, we need to move
                // the anchor over by however many characters were inserted overall
                start_range.anchor + offset + len_inserted
            }
        }
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
                let mut tendril = Tendril::new();
                tendril.push(open);
                (cursor, cursor, Some(tendril))
            }
            // None | Some(ch) if close_before.contains(ch) => {}
            _ => {
                // insert open & close
                let pair = Tendril::from_iter([open, close]);
                len_inserted = open.len_utf8() + close.len_utf8();
                (cursor, cursor, Some(pair))
            }
        };

        let next_range = get_next_range(doc, start_range, offs, open, len_inserted);
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
            let mut tendril = Tendril::new();
            tendril.push(close);
            (cursor, cursor, Some(tendril))
        };

        let next_range = get_next_range(doc, start_range, offs, close, len_inserted);
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
            let mut pair = Tendril::new();
            pair.push(token);

            // for equal pairs, don't insert both open and close if either
            // side has a non-pair char
            if (next_char.is_none() || close_before.contains(next_char.unwrap()))
                && (prev_char.is_none() || open_before.contains(prev_char.unwrap()))
            {
                pair.push(token);
            }

            len_inserted += pair.len();
            (cursor, cursor, Some(pair))
        };

        let next_range = get_next_range(doc, start_range, offs, token, len_inserted);
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

    const LINE_END: &str = crate::DEFAULT_LINE_ENDING.as_str();

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
        let trans = hook(in_doc, in_sel, ch).unwrap();
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
            &Rope::from(LINE_END),
            &Selection::single(1, 0),
            PAIRS,
            |open, close| format!("{}{}{}", open, close, LINE_END),
            &Selection::single(2, 1),
        );

        let empty_doc = Rope::from(format!("{line_end}{line_end}", line_end = LINE_END));

        test_hooks_with_pairs(
            &empty_doc,
            &Selection::single(empty_doc.len_chars(), LINE_END.len()),
            PAIRS,
            |open, close| {
                format!(
                    "{line_end}{open}{close}{line_end}",
                    open = open,
                    close = close,
                    line_end = LINE_END
                )
            },
            &Selection::single(LINE_END.len() + 2, LINE_END.len() + 1),
        );
    }

    #[test]
    fn test_insert_before_multi_code_point_graphemes() {
        test_hooks_with_pairs(
            &Rope::from(format!("hello 👨‍👩‍👧‍👦 goodbye{}", LINE_END)),
            &Selection::single(13, 6),
            PAIRS,
            |open, _| format!("hello {}👨‍👩‍👧‍👦 goodbye{}", open, LINE_END),
            &Selection::single(14, 7),
        );
    }

    #[test]
    fn test_insert_at_end_of_document() {
        test_hooks_with_pairs(
            &Rope::from(LINE_END),
            &Selection::single(LINE_END.len(), LINE_END.len()),
            PAIRS,
            |open, close| format!("{}{}{}", LINE_END, open, close),
            &Selection::single(LINE_END.len() + 1, LINE_END.len() + 1),
        );

        test_hooks_with_pairs(
            &Rope::from(format!("foo{}", LINE_END)),
            &Selection::single(3 + LINE_END.len(), 3 + LINE_END.len()),
            PAIRS,
            |open, close| format!("foo{}{}{}", LINE_END, open, close),
            &Selection::single(LINE_END.len() + 4, LINE_END.len() + 4),
        );
    }

    /// [] -> append ( -> ([])
    #[test]
    fn test_append_blank() {
        test_hooks_with_pairs(
            // this is what happens when you have a totally blank document and then append
            &Rope::from(format!("{line_end}{line_end}", line_end = LINE_END)),
            // before inserting the pair, the cursor covers all of both empty lines
            &Selection::single(0, LINE_END.len() * 2),
            PAIRS,
            |open, close| {
                format!(
                    "{line_end}{open}{close}{line_end}",
                    line_end = LINE_END,
                    open = open,
                    close = close
                )
            },
            // after inserting pair, the cursor covers the first new line and the open char
            &Selection::single(0, LINE_END.len() + 2),
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

    /// foo[] -> append to end of line ( -> foo([])
    #[test]
    fn test_append_single_cursor() {
        test_hooks_with_pairs(
            &Rope::from(format!("foo{}", LINE_END)),
            &Selection::single(3, 3 + LINE_END.len()),
            differing_pairs(),
            |open, close| format!("foo{}{}{}", open, close, LINE_END),
            &Selection::single(4, 5),
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

    /// ([)] -> insert ) -> ()[]
    #[test]
    fn test_insert_close_inside_pair() {
        for (open, close) in PAIRS {
            let doc = Rope::from(format!("{}{}{}", open, close, LINE_END));

            test_hooks(
                &doc,
                &Selection::single(2, 1),
                *close,
                &doc,
                &Selection::single(2 + LINE_END.len(), 2),
            );
        }
    }

    /// [(]) -> append ) -> [()]
    #[test]
    fn test_append_close_inside_pair() {
        for (open, close) in PAIRS {
            let doc = Rope::from(format!("{}{}{}", open, close, LINE_END));

            test_hooks(
                &doc,
                &Selection::single(0, 2),
                *close,
                &doc,
                &Selection::single(0, 2 + LINE_END.len()),
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

    /// foo([) wor]d -> insert ) -> foo()[ wor]d
    #[test]
    fn test_insert_close_inside_pair_trailing_word_with_selection() {
        for (open, close) in differing_pairs() {
            test_hooks(
                &Rope::from(format!("foo{}{} word{}", open, close, LINE_END)),
                &Selection::single(9, 4),
                *close,
                &Rope::from(format!("foo{}{} word{}", open, close, LINE_END)),
                &Selection::single(9, 5),
            )
        }
    }

    /// we want pairs that are *not* the same char to be inserted after
    /// a non-pair char, for cases like functions, but for pairs that are
    /// the same char, we want to *not* insert a pair to handle cases like "I'm"
    ///
    /// word[]  -> insert ( -> word([])
    /// word[]  -> insert ' -> word'[]
    #[test]
    fn test_insert_open_after_non_pair() {
        let doc = Rope::from(format!("word{}", LINE_END));
        let sel = Selection::single(5, 4);
        let expected_sel = Selection::single(6, 5);

        test_hooks_with_pairs(
            &doc,
            &sel,
            differing_pairs(),
            |open, close| format!("word{}{}{}", open, close, LINE_END),
            &expected_sel,
        );

        test_hooks_with_pairs(
            &doc,
            &sel,
            matching_pairs(),
            |open, _| format!("word{}{}", open, LINE_END),
            &expected_sel,
        );
    }
}
