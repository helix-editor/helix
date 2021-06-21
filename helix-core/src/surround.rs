use crate::{search, Selection};
use ropey::RopeSlice;

pub const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('<', '>'),
    ('«', '»'),
    ('「', '」'),
    ('（', '）'),
];

/// Given any char in [PAIRS], return the open and closing chars. If not found in
/// [PAIRS] return (ch, ch).
pub fn get_pair(ch: char) -> (char, char) {
    PAIRS
        .iter()
        .find(|(open, close)| *open == ch || *close == ch)
        .copied()
        .unwrap_or((ch, ch))
}

/// Find the position of surround pairs of `ch` which can be either a closing
/// or opening pair. `n` will skip n - 1 pairs (eg. n=2 will discard (only)
/// the first pair found and keep looking)
pub fn find_nth_pairs_pos(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
) -> Option<(usize, usize)> {
    let (open, close) = get_pair(ch);
    let open_pos = search::find_nth_prev(text, open, pos, n, true)?;
    let close_pos = search::find_nth_next(text, close, pos, n, true)?;

    Some((open_pos, close_pos))
}

/// Find position of surround characters around every cursor. Returns None
/// if any positions overlap. Note that the positions are in a flat Vec.
/// Use get_surround_pos().chunks(2) to get matching pairs of surround positions.
/// `ch` can be either closing or opening pair.
pub fn get_surround_pos(
    text: RopeSlice,
    selection: &Selection,
    ch: char,
    skip: usize,
) -> Option<Vec<usize>> {
    let mut change_pos = Vec::new();

    for range in selection {
        let head = range.head;

        match find_nth_pairs_pos(text, ch, head, skip) {
            Some((open_pos, close_pos)) => {
                if change_pos.contains(&open_pos) || change_pos.contains(&close_pos) {
                    return None;
                }
                change_pos.extend_from_slice(&[open_pos, close_pos]);
            }
            None => return None,
        }
    }
    Some(change_pos)
}
