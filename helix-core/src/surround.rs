use std::fmt::Display;

use crate::{movement::Direction, search, Range, Selection};
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

#[derive(Debug, PartialEq, Eq)]
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
    mut skip: usize,
) -> Result<(usize, usize)> {
    let is_open_pair = |ch| PAIRS.iter().any(|(open, _)| *open == ch);
    let is_close_pair = |ch| PAIRS.iter().any(|(_, close)| *close == ch);

    let mut stack = Vec::with_capacity(2);
    let pos = range.from();
    let mut close_pos = pos.saturating_sub(1);

    for ch in text.chars_at(pos) {
        close_pos += 1;

        if is_open_pair(ch) {
            // Track open pairs encountered so that we can step over
            // the corresponding close pairs that will come up further
            // down the loop. We want to find a lone close pair whose
            // open pair is before the cursor position.
            stack.push(ch);
            continue;
        }

        if !is_close_pair(ch) {
            // We don't care if this character isn't a brace pair item,
            // so short circuit here.
            continue;
        }

        let (open, close) = get_pair(ch);

        if stack.last() == Some(&open) {
            // If we are encountering the closing pair for an opener
            // we just found while traversing, then its inside the
            // selection and should be skipped over.
            stack.pop();
            continue;
        }

        match find_nth_open_pair(text, open, close, close_pos, 1) {
            // Before we accept this pair, we want to ensure that the
            // pair encloses the range rather than just the cursor.
            Some(open_pos)
                if open_pos <= pos.saturating_add(1)
                    && close_pos >= range.to().saturating_sub(1) =>
            {
                // Since we have special conditions for when to
                // accept, we can't just pass the skip parameter on
                // through to the find_nth_*_pair methods, so we
                // track skips manually here.
                if skip > 1 {
                    skip -= 1;
                    continue;
                }

                return match range.direction() {
                    Direction::Forward => Ok((open_pos, close_pos)),
                    Direction::Backward => Ok((close_pos, open_pos)),
                };
            }
            _ => continue,
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

    #[test]
    fn test_get_surround_pos() {
        #[rustfmt::skip]
        let (doc, selection, expectations) =
            rope_with_selections_and_expectations(
                "(some) (chars)\n(newline)",
                "_ ^  _ _ ^   _\n_    ^  _"
            );

        assert_eq!(
            get_surround_pos(doc.slice(..), &selection, Some('('), 1).unwrap(),
            expectations
        );
    }

    #[test]
    fn test_get_surround_pos_bail_different_surround_chars() {
        #[rustfmt::skip]
        let (doc, selection, _) =
            rope_with_selections_and_expectations(
                "[some]\n(chars)xx\n(newline)",
                "  ^   \n  ^      \n         "
            );

        assert_eq!(
            get_surround_pos(doc.slice(..), &selection, Some('('), 1),
            Err(Error::PairNotFound)
        );
    }

    #[test]
    fn test_get_surround_pos_bail_overlapping_surround_chars() {
        #[rustfmt::skip]
        let (doc, selection, _) =
            rope_with_selections_and_expectations(
                "[some]\n(chars)xx\n(newline)",
                "      \n       ^ \n      ^  "
            );

        assert_eq!(
            get_surround_pos(doc.slice(..), &selection, Some('('), 1),
            Err(Error::PairNotFound) // overlapping surround chars
        );
    }

    #[test]
    fn test_get_surround_pos_bail_cursor_overlap() {
        #[rustfmt::skip]
        let (doc, selection, _) =
            rope_with_selections_and_expectations(
                "[some]\n(chars)xx\n(newline)",
                "  ^^  \n         \n         "
            );

        assert_eq!(
            get_surround_pos(doc.slice(..), &selection, Some('['), 1),
            Err(Error::CursorOverlap)
        );
    }

    #[test]
    fn test_find_nth_pairs_pos_quote_success() {
        #[rustfmt::skip]
        let (doc, selection, expectations) =
            rope_with_selections_and_expectations(
                "some 'quoted text' on this 'line'\n'and this one'",
                "     _        ^  _               \n              "
            );

        assert_eq!(2, expectations.len());
        assert_eq!(
            find_nth_pairs_pos(doc.slice(..), '\'', selection.primary(), 1)
                .expect("find should succeed"),
            (expectations[0], expectations[1])
        )
    }

    #[test]
    fn test_find_nth_pairs_pos_nested_quote_success() {
        #[rustfmt::skip]
        let (doc, selection, expectations) =
            rope_with_selections_and_expectations(
                "some 'nested 'quoted' text' on this 'line'\n'and this one'",
                "     _           ^        _               \n              "
            );

        assert_eq!(2, expectations.len());
        assert_eq!(
            find_nth_pairs_pos(doc.slice(..), '\'', selection.primary(), 2)
                .expect("find should succeed"),
            (expectations[0], expectations[1])
        )
    }

    #[test]
    fn test_find_nth_pairs_pos_inside_quote_ambiguous() {
        #[rustfmt::skip]
        let (doc, selection, _) =
            rope_with_selections_and_expectations(
                "some 'nested 'quoted' text' on this 'line'\n'and this one'",
                "                    ^                     \n              "
            );

        assert_eq!(
            find_nth_pairs_pos(doc.slice(..), '\'', selection.primary(), 1),
            Err(Error::CursorOnAmbiguousPair)
        )
    }

    // Create a Rope and a matching Selection using a specification language.
    // ^ is a single-point selection.
    // _ is an expected index. These are returned as a Vec<usize> for use in assertions.
    fn rope_with_selections_and_expectations(
        text: &str,
        spec: &str,
    ) -> (Rope, Selection, Vec<usize>) {
        if text.len() != spec.len() {
            panic!("specification must match text length -- are newlines aligned?");
        }

        let rope = Rope::from(text);

        let selections: SmallVec<[Range; 1]> = spec
            .match_indices('^')
            .map(|(i, _)| Range::point(i))
            .collect();

        let expectations: Vec<usize> = spec.match_indices('_').map(|(i, _)| i).collect();

        (rope, Selection::new(selections, 0), expectations)
    }
}
