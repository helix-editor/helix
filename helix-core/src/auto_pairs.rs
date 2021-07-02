use crate::{Range, Rope, Selection, Tendril, Transaction};
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

const CLOSE_BEFORE: &str = ")]}'\":;> \n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}"; // includes space and newlines

// insert hook:
// Fn(doc, selection, char) => Option<Transaction>
// problem is, we want to do this per range, so we can call default handler for some ranges
// so maybe ret Vec<Option<Change>>
// but we also need to be able to return transactions...
//
// to simplify, maybe return Option<Transaction> and just reimplement the default

// TODO: delete implementation where it erases the whole bracket (|) -> |

#[must_use]
pub fn hook(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
    for &(open, close) in PAIRS {
        if open == ch {
            if open == close {
                return handle_same(doc, selection, open);
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

// TODO: special handling for lifetimes in rust: if preceeded by & or < don't auto close '
// for example "&'a mut", or "fn<'a>"

fn next_char(doc: &Rope, pos: usize) -> Option<char> {
    if pos >= doc.len_chars() {
        return None;
    }
    Some(doc.char(pos))
}
// TODO: selections should be extended if range, moved if point.

// TODO: if not cursor but selection, wrap on both sides of selection (surround)
fn handle_open(
    doc: &Rope,
    selection: &Selection,
    open: char,
    close: char,
    close_before: &str,
) -> Transaction {
    let mut ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |range| {
        let pos = range.head;
        let next = next_char(doc, pos);

        let head = pos + offs + open.len_utf8();
        // if selection, retain anchor, if cursor, move over
        ranges.push(Range::new(
            if range.is_empty() {
                head
            } else {
                range.anchor + offs
            },
            head,
        ));

        match next {
            Some(ch) if !close_before.contains(ch) => {
                // TODO: else return (use default handler that inserts open)
                (pos, pos, Some(Tendril::from_char(open)))
            }
            // None | Some(ch) if close_before.contains(ch) => {}
            _ => {
                // insert open & close
                let mut pair = Tendril::with_capacity(2);
                pair.push_char(open);
                pair.push_char(close);

                offs += 2;

                (pos, pos, Some(pair))
            }
        }
    });

    transaction.with_selection(Selection::new(ranges, selection.primary_index()))
}

fn handle_close(doc: &Rope, selection: &Selection, _open: char, close: char) -> Transaction {
    let mut ranges = SmallVec::with_capacity(selection.len());

    let mut offs = 0;

    let transaction = Transaction::change_by_selection(doc, selection, |range| {
        let pos = range.head;
        let next = next_char(doc, pos);

        let head = pos + offs + close.len_utf8();
        // if selection, retain anchor, if cursor, move over
        ranges.push(Range::new(
            if range.is_empty() {
                head
            } else {
                range.anchor + offs
            },
            head,
        ));

        if next == Some(close) {
            //  return transaction that moves past close
            (pos, pos, None) // no-op
        } else {
            offs += close.len_utf8();

            // TODO: else return (use default handler that inserts close)
            (pos, pos, Some(Tendril::from_char(close)))
        }
    });

    transaction.with_selection(Selection::new(ranges, selection.primary_index()))
}

// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_same(_doc: &Rope, _selection: &Selection, _token: char) -> Option<Transaction> {
    // if not cursor but selection, wrap
    // let next = next char

    // if next == bracket {
    //   // if start of syntax node, insert token twice (new pair because node is complete)
    //   // elseif colsedBracketAt
    //      // is_triple == allow triple && next 3 is equal
    //      // cursor jump over
    // }
    //} else if allow_triple && followed by triple {
    //}
    //} else if next != word char && prev != bracket && prev != word char {
    // // condition checks for cases like I' where you don't want I'' (or I'm)
    //  insert pair ("")
    //}
    None
}
