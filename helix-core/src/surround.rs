use std::fmt::Display;

use crate::{search, Range, Selection};
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

#[derive(Debug, PartialEq)]
pub enum Error {
    PairNotFound,
    CursorOverlap,
    RangeExceedsText,
    CursorOnAmbiguousPair,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Error::PairNotFound => "Surround pair not found around all cursors",
            Error::CursorOverlap => "Cursors overlap for a single surround pair range",
            Error::RangeExceedsText => "Cursor range exceeds text length",
            Error::CursorOnAmbiguousPair => "Cursor on ambiguous surround pair",
        })
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Given any char in [PAIRS], return the open and closing chars. If not found in
/// [PAIRS] return (ch, ch).
///
/// ```
/// use helix_core::surround::get_pair;
///
/// assert_eq!(get_pair('['), ('[', ']'));
/// assert_eq!(get_pair('}'), ('{', '}'));
/// assert_eq!(get_pair('"'), ('"', '"'));
/// ```
pub fn get_pair(ch: char) -> (char, char) {
    PAIRS
        .iter()
        .find(|(open, close)| *open == ch || *close == ch)
        .copied()
        .unwrap_or((ch, ch))
}

pub fn find_nth_closest_pairs_pos(
    text: RopeSlice,
    range: Range,
    n: usize,
) -> Result<(usize, usize)> {
    let is_open_pair = |ch| PAIRS.iter().any(|(open, _)| *open == ch);
    let is_close_pair = |ch| PAIRS.iter().any(|(_, close)| *close == ch);

    let mut stack = Vec::with_capacity(2);
    let pos = range.cursor(text);

    for ch in text.chars_at(pos) {
        if is_open_pair(ch) {
            // Track open pairs encountered so that we can step over
            // the corresponding close pairs that will come up further
            // down the loop. We want to find a lone close pair whose
            // open pair is before the cursor position.
            stack.push(ch);
            continue;
        } else if is_close_pair(ch) {
            let (open, _) = get_pair(ch);
            if stack.last() == Some(&open) {
                stack.pop();
                continue;
            } else {
                // In the ideal case the stack would be empty here and the
                // current character would be the close pair that we are
                // looking for. It could also be the case that the pairs
                // are unbalanced and we encounter a close pair that doesn't
                // close the last seen open pair. In either case use this
                // char as the auto-detected closest pair.
                return find_nth_pairs_pos(text, ch, range, n);
            }
        }
    }

    Err(Error::PairNotFound)
}

/// Find the position of surround pairs of `ch` which can be either a closing
/// or opening pair. `n` will skip n - 1 pairs (eg. n=2 will discard (only)
/// the first pair found and keep looking)
pub fn find_nth_pairs_pos(
    text: RopeSlice,
    ch: char,
    range: Range,
    n: usize,
) -> Result<(usize, usize)> {
    if text.len_chars() < 2 {
        return Err(Error::PairNotFound);
    }
    if range.to() >= text.len_chars() {
        return Err(Error::RangeExceedsText);
    }

    let (open, close) = get_pair(ch);
    let pos = range.cursor(text);

    let (open, close) = if open == close {
        if Some(open) == text.get_char(pos) {
            // Cursor is directly on match char. We return no match
            // because there's no way to know which side of the char
            // we should be searching on.
            return Err(Error::CursorOnAmbiguousPair);
        }
        (
            search::find_nth_prev(text, open, pos, n),
            search::find_nth_next(text, close, pos, n),
        )
    } else {
        (
            find_nth_open_pair(text, open, close, pos, n),
            find_nth_close_pair(text, open, close, pos, n),
        )
    };

    Option::zip(open, close).ok_or(Error::PairNotFound)
}

fn find_nth_open_pair(
    text: RopeSlice,
    open: char,
    close: char,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    let mut chars = text.chars_at(pos + 1);

    // Adjusts pos for the first iteration, and handles the case of the
    // cursor being *on* the close character which will get falsely stepped over
    // if not skipped here
    if chars.prev()? == open {
        return Some(pos);
    }

    for _ in 0..n {
        let mut step_over: usize = 0;

        loop {
            let c = chars.prev()?;
            pos = pos.saturating_sub(1);

            // ignore other surround pairs that are enclosed *within* our search scope
            if c == close {
                step_over += 1;
            } else if c == open {
                if step_over == 0 {
                    break;
                }

                step_over = step_over.saturating_sub(1);
            }
        }
    }

    Some(pos)
}

fn find_nth_close_pair(
    text: RopeSlice,
    open: char,
    close: char,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    if pos >= text.len_chars() {
        return None;
    }

    let mut chars = text.chars_at(pos);

    if chars.next()? == close {
        return Some(pos);
    }

    for _ in 0..n {
        let mut step_over: usize = 0;

        loop {
            let c = chars.next()?;
            pos += 1;

            if c == open {
                step_over += 1;
            } else if c == close {
                if step_over == 0 {
                    break;
                }

                step_over = step_over.saturating_sub(1);
            }
        }
    }

    Some(pos)
}

/// Find position of surround characters around every cursor. Returns None
/// if any positions overlap. Note that the positions are in a flat Vec.
/// Use get_surround_pos().chunks(2) to get matching pairs of surround positions.
/// `ch` can be either closing or opening pair. If `ch` is None, surround pairs
/// are automatically detected around each cursor (note that this may result
/// in them selecting different surround characters for each selection).
pub fn get_surround_pos(
    text: RopeSlice,
    selection: &Selection,
    ch: Option<char>,
    skip: usize,
) -> Result<Vec<usize>> {
    let mut change_pos = Vec::new();

    for &range in selection {
        let (open_pos, close_pos) = match ch {
            Some(ch) => find_nth_pairs_pos(text, ch, range, skip)?,
            None => find_nth_closest_pairs_pos(text, range, skip)?,
        };
        if change_pos.contains(&open_pos) || change_pos.contains(&close_pos) {
            return Err(Error::CursorOverlap);
        }
        change_pos.extend_from_slice(&[open_pos, close_pos]);
    }
    Ok(change_pos)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Range;

    use ropey::Rope;
    use smallvec::SmallVec;

    #[allow(clippy::type_complexity)]
    fn check_find_nth_pair_pos(
        text: &str,
        cases: Vec<(usize, char, usize, Result<(usize, usize)>)>,
    ) {
        let doc = Rope::from(text);
        let slice = doc.slice(..);

        for (cursor_pos, ch, n, expected_range) in cases {
            let range = find_nth_pairs_pos(slice, ch, (cursor_pos, cursor_pos + 1).into(), n);
            assert_eq!(
                range, expected_range,
                "Expected {:?}, got {:?}",
                expected_range, range
            );
        }
    }

    #[test]
    fn test_find_nth_pairs_pos() {
        check_find_nth_pair_pos(
            "some (text) here",
            vec![
                // cursor on [t]ext
                (6, '(', 1, Ok((5, 10))),
                (6, ')', 1, Ok((5, 10))),
                // cursor on so[m]e
                (2, '(', 1, Err(Error::PairNotFound)),
                // cursor on bracket itself
                (5, '(', 1, Ok((5, 10))),
                (10, '(', 1, Ok((5, 10))),
            ],
        );
    }

    #[test]
    fn test_find_nth_pairs_pos_skip() {
        check_find_nth_pair_pos(
            "(so (many (good) text) here)",
            vec![
                // cursor on go[o]d
                (13, '(', 1, Ok((10, 15))),
                (13, '(', 2, Ok((4, 21))),
                (13, '(', 3, Ok((0, 27))),
            ],
        );
    }

    #[test]
    fn test_find_nth_pairs_pos_same() {
        check_find_nth_pair_pos(
            "'so 'many 'good' text' here'",
            vec![
                // cursor on go[o]d
                (13, '\'', 1, Ok((10, 15))),
                (13, '\'', 2, Ok((4, 21))),
                (13, '\'', 3, Ok((0, 27))),
                // cursor on the quotes
                (10, '\'', 1, Err(Error::CursorOnAmbiguousPair)),
            ],
        )
    }

    #[test]
    fn test_find_nth_pairs_pos_step() {
        check_find_nth_pair_pos(
            "((so)((many) good (text))(here))",
            vec![
                // cursor on go[o]d
                (15, '(', 1, Ok((5, 24))),
                (15, '(', 2, Ok((0, 31))),
            ],
        )
    }

    #[test]
    fn test_find_nth_pairs_pos_mixed() {
        check_find_nth_pair_pos(
            "(so [many {good} text] here)",
            vec![
                // cursor on go[o]d
                (13, '{', 1, Ok((10, 15))),
                (13, '[', 1, Ok((4, 21))),
                (13, '(', 1, Ok((0, 27))),
            ],
        )
    }

    #[test]
    fn test_get_surround_pos() {
        let doc = Rope::from("(some) (chars)\n(newline)");
        let slice = doc.slice(..);
        let selection = Selection::new(
            SmallVec::from_slice(&[Range::point(2), Range::point(9), Range::point(20)]),
            0,
        );

        // cursor on s[o]me, c[h]ars, newl[i]ne
        assert_eq!(
            get_surround_pos(slice, &selection, Some('('), 1)
                .unwrap()
                .as_slice(),
            &[0, 5, 7, 13, 15, 23]
        );
    }

    #[test]
    fn test_get_surround_pos_bail() {
        let doc = Rope::from("[some]\n(chars)xx\n(newline)");
        let slice = doc.slice(..);

        let selection =
            Selection::new(SmallVec::from_slice(&[Range::point(2), Range::point(9)]), 0);
        // cursor on s[o]me, c[h]ars
        assert_eq!(
            get_surround_pos(slice, &selection, Some('('), 1),
            Err(Error::PairNotFound) // different surround chars
        );

        let selection = Selection::new(
            SmallVec::from_slice(&[Range::point(14), Range::point(24)]),
            0,
        );
        // cursor on [x]x, newli[n]e
        assert_eq!(
            get_surround_pos(slice, &selection, Some('('), 1),
            Err(Error::PairNotFound) // overlapping surround chars
        );

        let selection =
            Selection::new(SmallVec::from_slice(&[Range::point(2), Range::point(3)]), 0);
        // cursor on s[o][m]e
        assert_eq!(
            get_surround_pos(slice, &selection, Some('['), 1),
            Err(Error::CursorOverlap)
        );
    }
}
