use crate::{Range, Rope, Selection, Tendril, Transaction};
use smallvec::SmallVec;

// Heavily based on https://github.com/codemirror/closebrackets/

const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('\'', '\''),
    ('"', '"'),
    ('`', '`'),
];

const CLOSE_BEFORE: &str = ")]}'\":;> \n"; // includes space and newline

// insert hook:
// Fn(doc, selection, char) => Option<Transaction>
// problem is, we want to do this per range, so we can call default handler for some ranges
// so maybe ret Vec<Option<Change>>
// but we also need to be able to return transactions...
//
// to simplify, maybe return Option<Transaction> and just reimplement the default

// TODO: delete implementation where it erases the whole bracket (|) -> |

pub fn hook(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
    for &(open, close) in PAIRS {
        if open == ch {
            let t = if open == close {
                return None;
                // handle_same()
            } else {
                handle_open(doc, selection, open, close, CLOSE_BEFORE)
            };
            return Some(t);
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

// TODO: if not cursor but selection, wrap on both sides of selection (surround)
fn handle_open(
    doc: &Rope,
    selection: &Selection,
    open: char,
    close: char,
    close_before: &str,
) -> Transaction {
    let mut ranges = SmallVec::with_capacity(selection.len());

    let mut transaction = Transaction::change_by_selection(doc, selection, |range| {
        let pos = range.head;
        let next = next_char(doc, pos);

        ranges.push(Range::new(range.anchor, pos + 1)); // pos + open

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

                (pos, pos, Some(pair))
            }
        }
    });

    transaction.with_selection(Selection::new(ranges, selection.primary_index()))
}

fn handle_close(doc: &Rope, selection: &Selection, _open: char, close: char) -> Transaction {
    let mut ranges = SmallVec::with_capacity(selection.len());

    let mut transaction = Transaction::change_by_selection(doc, selection, |range| {
        let pos = range.head;
        let next = next_char(doc, pos);

        ranges.push(Range::new(range.anchor, pos + 1)); // pos + close

        if next == Some(close) {
            //  return transaction that moves past close
            (pos, pos, None) // no-op
        } else {
            // TODO: else return (use default handler that inserts close)
            (pos, pos, Some(Tendril::from_char(close)))
        }
    });

    transaction.with_selection(Selection::new(ranges, selection.primary_index()))
}

// handle cases where open and close is the same, or in triples ("""docstring""")
fn handle_same() {
    // if not cursor but selection, wrap
    // let next = next char
}
