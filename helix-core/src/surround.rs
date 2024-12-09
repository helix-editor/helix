use std::{collections::HashSet, fmt::Display};

use crate::{
    graphemes::next_grapheme_boundary,
    match_brackets::{
        find_matching_bracket, find_matching_bracket_fuzzy, get_pair, is_close_bracket,
        is_open_bracket,
    },
    movement::Direction,
    search, Range, Selection, Syntax,
};
use ropey::RopeSlice;

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

/// Finds the position of surround pairs of any [`crate::match_brackets::PAIRS`]
/// using tree-sitter when possible.
///
/// # Returns
///
/// Tuple `(anchor, head)`, meaning it is not always ordered.
pub fn find_nth_closest_pairs_pos(
    syntax: Option<&Syntax>,
    text: RopeSlice,
    range: Range,
    skip: usize,
) -> Result<(usize, usize)> {
    match syntax {
        Some(syntax) => find_nth_closest_pairs_ts(syntax, text, range, skip),
        None => find_nth_closest_pairs_plain(text, range, skip),
    }
}

fn find_nth_closest_pairs_ts(
    syntax: &Syntax,
    text: RopeSlice,
    range: Range,
    mut skip: usize,
) -> Result<(usize, usize)> {
    let mut opening = range.from();
    // We want to expand the selection if we are already on the found pair,
    // otherwise we would need to subtract "-1" from "range.to()".
    let mut closing = range.to();

    while skip > 0 {
        closing = find_matching_bracket_fuzzy(syntax, text, closing).ok_or(Error::PairNotFound)?;
        opening = find_matching_bracket(syntax, text, closing).ok_or(Error::PairNotFound)?;
        // If we're already on a closing bracket "find_matching_bracket_fuzzy" will return
        // the position of the opening bracket.
        if closing < opening {
            (opening, closing) = (closing, opening);
        }

        // In case found brackets are partially inside current selection.
        if range.from() < opening || closing < range.to() - 1 {
            closing = next_grapheme_boundary(text, closing);
        } else {
            skip -= 1;
            if skip != 0 {
                closing = next_grapheme_boundary(text, closing);
            }
        }
    }

    // Keep the original direction.
    if let Direction::Forward = range.direction() {
        Ok((opening, closing))
    } else {
        Ok((closing, opening))
    }
}

fn find_nth_closest_pairs_plain(
    text: RopeSlice,
    range: Range,
    mut skip: usize,
) -> Result<(usize, usize)> {
    let mut stack = Vec::with_capacity(2);
    let pos = range.from();
    let mut close_pos = pos.saturating_sub(1);

    for ch in text.chars_at(pos) {
        close_pos += 1;

        if is_open_bracket(ch) {
            // Track open pairs encountered so that we can step over
            // the corresponding close pairs that will come up further
            // down the loop. We want to find a lone close pair whose
            // open pair is before the cursor position.
            stack.push(ch);
            continue;
        }

        if !is_close_bracket(ch) {
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

    // preserve original direction
    match range.direction() {
        Direction::Forward => Option::zip(open, close).ok_or(Error::PairNotFound),
        Direction::Backward => Option::zip(close, open).ok_or(Error::PairNotFound),
    }
}

fn find_nth_open_pair(
    text: RopeSlice,
    open: char,
    close: char,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    if pos >= text.len_chars() {
        return None;
    }

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
    syntax: Option<&Syntax>,
    text: RopeSlice,
    selection: &Selection,
    ch: Option<char>,
    skip: usize,
) -> Result<Vec<usize>> {
    let mut change_pos = Vec::new();

    for &range in selection {
        let (open_pos, close_pos) = {
            let range_raw = match ch {
                Some(ch) => find_nth_pairs_pos(text, ch, range, skip)?,
                None => find_nth_closest_pairs_pos(syntax, text, range, skip)?,
            };
            let range = Range::new(range_raw.0, range_raw.1);
            (range.from(), range.to())
        };
        if change_pos.contains(&open_pos) || change_pos.contains(&close_pos) {
            return Err(Error::CursorOverlap);
        }
        // ensure the positions are always paired in the forward direction
        change_pos.extend_from_slice(&[open_pos.min(close_pos), close_pos.max(open_pos)]);
    }
    Ok(change_pos)
}

/// Test whether a character would be considered a valid character if it was used for either JSX, HTML or XML tags
/// JSX tags may have `.` in them for scoping
/// HTML tags may have `-` in them if it's a custom element
/// Both JSX and HTML tags may have `_`
fn is_valid_tagname_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.'
}

/// Find the opening `<tag>` starting from `cursor_pos` and iterating until the beginning of the text.
/// Returns the Range of the tag's name (excluding the `<` and `>` characters.)
/// As well as the actual name of the tag
/// Additionally, it returns the last position where it stopped searching.
fn find_prev_tag(
    text: RopeSlice,
    mut cursor_pos: usize,
    skip: usize,
) -> Result<(Range, String, usize)> {
    if cursor_pos == 0 || skip == 0 {
        return Err(Error::RangeExceedsText);
    }

    let mut chars = text.chars_at(cursor_pos);

    loop {
        let prev_char = match chars.prev() {
            Some(ch) => ch,
            None => return Err(Error::PairNotFound),
        };
        cursor_pos -= 1;

        if prev_char == '>' {
            let mut possible_tag_name = String::new();
            loop {
                let current_char = match chars.prev() {
                    Some(ch) => ch,
                    None => return Err(Error::PairNotFound),
                };
                cursor_pos -= 1;
                if current_char == '<' {
                    let tag_name = possible_tag_name
                        .chars()
                        .rev()
                        .take_while(|&ch| is_valid_tagname_char(ch))
                        .collect::<String>();

                    let range = Range::new(cursor_pos + 1, cursor_pos + tag_name.len() + 1);
                    return Ok((range, tag_name, cursor_pos));
                }
                possible_tag_name.push(current_char);
            }
        }
    }
}

/// Find the closing `</tag>` starting from `pos` and iterating the end of the text.
/// Returns the Range of the tag's name (excluding the `</` and `>` characters.)
/// As well as the actual name of the tag and where it last stopped searching.
fn find_next_tag(
    text: RopeSlice,
    mut cursor_pos: usize,
    skip: usize,
) -> Result<(Range, String, usize)> {
    if cursor_pos >= text.len_chars() || skip == 0 {
        return Err(Error::RangeExceedsText);
    }

    let mut chars = text.chars_at(cursor_pos);

    // look forward and find something that looks like a closing tag, e.g. <html> and extract it's name so we get "html"
    loop {
        let next_char = match chars.next() {
            Some(ch) => ch,
            None => return Err(Error::PairNotFound),
        };
        cursor_pos += 1;
        if next_char == '<' {
            let char_after_that = match chars.next() {
                Some(ch) => ch,
                None => return Err(Error::PairNotFound),
            };
            cursor_pos += 1;
            if char_after_that == '/' {
                let mut possible_tag_name = String::new();
                loop {
                    let current_char = match chars.next() {
                        Some(ch) => ch,
                        None => return Err(Error::PairNotFound),
                    };
                    cursor_pos += 1;
                    if is_valid_tagname_char(current_char) {
                        possible_tag_name.push(current_char);
                    } else if current_char == '>' && possible_tag_name.len() != 0 {
                        let range =
                            Range::new(cursor_pos - possible_tag_name.len() - 1, cursor_pos - 1);

                        return Ok((range, possible_tag_name, cursor_pos));
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

/// Get the two sorted `Range`s corresponding to nth matching tags surrounding the cursor, as well as the name of the tags.
fn find_nth_nearest_tag(
    forward_text: RopeSlice,
    cursor_pos: usize,
    skip: usize,
) -> Result<((Range, Range), String)> {
    let backward_text = forward_text.clone();

    let mut forward_tags = vec![];
    let mut previous_forward_pos = cursor_pos;

    /// the maximum length of chars we will search forward and backward to find the tags, provided we don't hit the end or the beginning of the document
    const SEARCH_CHARS: usize = 2000;

    while {
        let is_within_offset = previous_forward_pos - cursor_pos < SEARCH_CHARS;
        let is_within_bounds = previous_forward_pos < forward_text.len_chars();

        is_within_offset && is_within_bounds
    } {
        match find_next_tag(forward_text, previous_forward_pos, skip) {
            Ok((forward_tag_range, forward_tag_name, forward_search_idx)) => {
                forward_tags.push((forward_tag_range, forward_tag_name));
                previous_forward_pos = forward_search_idx;
            }
            Err(err) => match err {
                Error::PairNotFound => {
                    break;
                }
                other_error => {
                    return Err(other_error);
                }
            },
        }
    }

    let mut backward_tags = vec![];
    let mut previous_backward_pos = cursor_pos;

    while {
        let is_within_offset = cursor_pos - previous_backward_pos < SEARCH_CHARS;
        let is_within_bounds = previous_backward_pos > 0;

        is_within_offset && is_within_bounds
    } {
        match find_prev_tag(backward_text, previous_backward_pos, skip) {
            Ok((backward_tag_range, backward_tag_name, backward_search_idx)) => {
                backward_tags.push((backward_tag_range, backward_tag_name));
                previous_backward_pos = backward_search_idx;
            }
            Err(err) => match err {
                Error::PairNotFound => {
                    break;
                }
                other_error => {
                    return Err(other_error);
                }
            },
        }
    }

    // only consider the tags which are in both collections.
    let backward_tag_names: HashSet<_> = backward_tags.iter().map(|(_, tag)| tag.clone()).collect();
    let forward_tag_names: HashSet<_> = forward_tags.iter().map(|(_, tag)| tag.clone()).collect();

    let common_tags: HashSet<_> = backward_tag_names
        .intersection(&forward_tag_names)
        .collect();

    let backward_tags: Vec<_> = backward_tags
        .into_iter()
        .filter(|(_, tag)| common_tags.contains(tag))
        .collect();

    let forward_tags: Vec<_> = forward_tags
        .into_iter()
        .filter(|(_, tag)| common_tags.contains(tag))
        .collect();

    // improperly ordered tags such as <div> <span> </div> </span> are ignored completely
    let matching_tags: Vec<((Range, String), (Range, String))> = forward_tags
        .into_iter()
        .zip(backward_tags)
        .filter(|((_, forward_tag_name), (_, backward_tag_name))| {
            forward_tag_name == backward_tag_name
        })
        .collect();

    // If the count overflows past the highest available outer tag, e.g. user types 100 but we can only select up to 4 nestings of tags -- simply select the last one available
    let access_index = if skip - 1 <= matching_tags.len() {
        skip - 1
    } else {
        matching_tags.len() - 1
    };

    if let Some(nth_matching_tags) = matching_tags.into_iter().nth(access_index) {
        let ((range_forward, tag_name), (range_backward, _tag_name)) = nth_matching_tags;

        Ok(((range_backward, range_forward), tag_name))
    } else {
        Err(Error::PairNotFound)
    }
}

/// Find position of surrounding <tag>s around every cursor as well as the tag's names.
/// Returns Err if any positions overlap. Note that the positions are in a flat Vec.
/// Use get_surround_pos_tag().chunks(2) to get matching pairs of surround positions.
pub fn get_surround_pos_tag(
    text: RopeSlice,
    selection: &Selection,
    skip: usize,
) -> Result<Vec<(Range, String)>> {
    let mut change_pos = vec![];

    for &range in selection {
        let cursor_pos = range.cursor(text);

        let ((prev_tag, next_tag), tag_name) = find_nth_nearest_tag(text, cursor_pos, skip)?;

        change_pos.push((prev_tag, tag_name.clone()));
        change_pos.push((next_tag, tag_name));
    }

    // sort all ranges by their beginning
    change_pos.sort_by(|&(a, _), (b, _)| a.from().cmp(&b.from()));

    // if the end of any range exceeds beginning of the next range, there is an overlap
    let has_overlaps = change_pos
        .windows(2)
        .any(|window| window[0].0.to() > window[1].0.from());

    if has_overlaps {
        Err(Error::CursorOverlap)
    } else {
        Ok(change_pos)
    }
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
            get_surround_pos(None, doc.slice(..), &selection, Some('('), 1).unwrap(),
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
            get_surround_pos(None, doc.slice(..), &selection, Some('('), 1),
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
            get_surround_pos(None, doc.slice(..), &selection, Some('('), 1),
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
            get_surround_pos(None, doc.slice(..), &selection, Some('['), 1),
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

    #[test]
    fn test_find_nth_closest_pairs_pos_index_range_panic() {
        #[rustfmt::skip]
        let (doc, selection, _) =
            rope_with_selections_and_expectations(
                "(a)c)",
                "^^^^^"
            );

        assert_eq!(
            find_nth_closest_pairs_pos(None, doc.slice(..), selection.primary(), 1),
            Err(Error::PairNotFound)
        )
    }

    #[test]
    fn test_find_surrounding_tag_simple() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<html> test </html>",
            " ____   ^     ____ ",
            vec!["html"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_with_extra_closing_tag() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<div> test </html> </div>",
            " ___    ^            ___ ",
            vec!["div"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_with_broken_tags() {
        let (doc, selection, _) = rope_with_selections_and_expectations_tags(
            "<span> <div> simple example </span> </html> </div>",
            "                    ^                             ",
            vec![],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            Err(Error::PairNotFound)
        );
    }

    #[test]
    fn test_find_surrounding_tag_with_many_tags() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<span> <div><html>  simple example </div> </html> </span>",
            " ____                      ^                        ____ ",
            vec!["span"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_with_nth_tag_newline() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<span> <div> test\n\n </div> </span>",
            " ____         ^  \n\n          ____ ",
            vec!["span"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 2),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_multiple_cursor() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<span> <div> </div> </span>\n\n <b> <a>     </a> </b>",
            " ____       ^         ____ \n\n  _       ^         _ ",
            vec!["span", "b"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 2),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_empty_document() {
        let (doc, selection, _) = rope_with_selections_and_expectations_tags(
            " hello world, wonderful world! ",
            "               ^               ",
            vec![],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            Err(Error::PairNotFound)
        );
    }

    #[test]
    fn test_find_surrounding_tag_unclosed_tag() {
        let (doc, selection, _) = rope_with_selections_and_expectations_tags(
            "this is an <div> Unclosed tag",
            " ^                           ",
            vec![],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            Err(Error::PairNotFound)
        );
    }

    #[test]
    fn test_find_surrounding_tag_nested_with_partial_overlap() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<div> <span> <p> Text </span> </p> </div>",
            "       ____  ^          ____             ",
            vec!["span"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_nested_same_tag_multiple_levels() {
        let (doc, selection, _) = rope_with_selections_and_expectations_tags(
            "<div> <div> <div> Nested </div> </div> </div>",
            " ___      ^                              ___ ",
            vec!["div"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 2),
            Err(Error::PairNotFound)
        );
    }

    #[test]
    fn test_find_surrounding_tag_self_closing_tags_ignored() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<div> <img /> <span> Text </span> </div>",
            " ___             ^                  ___ ",
            vec!["div"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    #[test]
    fn test_find_surrounding_tag_adjacent_tags() {
        let (doc, selection, expectations) = rope_with_selections_and_expectations_tags(
            "<div></div><span></span>",
            " ___ ^ ___              ",
            vec!["div"],
        );

        assert_eq!(
            get_surround_pos_tag(doc.slice(..), &selection, 1),
            expectations
        );
    }

    /// Create a Rope and a matching Selection using a specification language.
    /// ^ is a cursor position.
    /// Continuous _ denote start and end of ranges. These are returned as (Range, Range)
    /// for use within assertions.
    fn rope_with_selections_and_expectations_tags(
        text: &str,
        spec: &str,
        tag_names: Vec<&str>,
    ) -> (Rope, Selection, Result<Vec<(Range, String)>>) {
        if text.len() != spec.len() {
            panic!("specification must match text length -- are newlines aligned?");
        }

        let selections: SmallVec<[Range; 1]> = spec
            .match_indices('^')
            .map(|(i, _)| Range::point(i))
            .collect();

        let mut tag_names = tag_names
            .into_iter()
            .flat_map(|tag_name| vec![tag_name, tag_name]);

        let raw_ranges = spec
            .char_indices()
            .chain(std::iter::once((spec.len(), ' ')))
            .fold(Vec::new(), |mut groups, (i, c)| {
                match (groups.last_mut(), c) {
                    (Some((_start, end)), '_') if *end + 1 == i => {
                        // Extend current group
                        *end = i;
                    }
                    (Some((_start, end)), '_') if *end < i => {
                        // Start a new group after a gap
                        groups.push((i, i));
                    }
                    (None, '_') => {
                        // Start the first group
                        groups.push((i, i));
                    }
                    _ => {} // Ignore non-underscore characters
                }
                groups
            })
            .into_iter();

        let range_and_tags = raw_ranges
            .map(|(anchor, head)| {
                (
                    Range::new(anchor, head + 1),
                    String::from(tag_names.next().unwrap()),
                )
            })
            .collect();

        (
            Rope::from(text),
            Selection::new(selections, 0),
            Ok(range_and_tags),
        )
    }

    /// Create a Rope and a matching Selection using a specification language.
    /// ^ is a single-point selection.
    /// _ is an expected index. These are returned as a Vec<usize> for use in assertions.
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
