use std::fmt::Display;

use ropey::{iter::Chars, RopeSlice};
use tree_sitter::{Node, QueryCursor};

use crate::chars::{categorize_char, char_is_line_ending, char_is_whitespace, CharCategory};
use crate::graphemes::{
    next_grapheme_boundary, nth_prev_grapheme_boundary, prev_grapheme_boundary,
};
use crate::movement::Direction;
use crate::surround;
use crate::syntax::LanguageConfiguration;
use crate::Range;

fn chars_from_direction<'a>(slice: &'a RopeSlice, pos: usize, direction: Direction) -> Chars<'a> {
    match direction {
        Direction::Forward => slice.chars_at(pos),
        Direction::Backward => {
            let mut iter = slice.chars_at(pos);
            iter.reverse();
            iter
        }
    }
}

fn find_word_boundary(slice: RopeSlice, mut pos: usize, direction: Direction, long: bool) -> usize {
    use CharCategory::{Eol, Whitespace};

    let iter = chars_from_direction(&slice, pos, direction);

    let mut prev_category = match direction {
        Direction::Forward if pos == 0 => Whitespace,
        Direction::Forward => categorize_char(slice.char(pos - 1)),
        Direction::Backward if pos == slice.len_chars() => Whitespace,
        Direction::Backward => categorize_char(slice.char(pos)),
    };

    for ch in iter {
        match categorize_char(ch) {
            Eol | Whitespace => return pos,
            category => {
                if !long && category != prev_category && pos != 0 && pos != slice.len_chars() {
                    return pos;
                } else {
                    match direction {
                        Direction::Forward => pos += 1,
                        Direction::Backward => pos = pos.saturating_sub(1),
                    }
                    prev_category = category;
                }
            }
        }
    }

    pos
}

pub fn find_paragraph_boundary(slice: RopeSlice, mut pos: usize, direction: Direction) -> usize {
    // if pos is at non-empty line ending or when going forward move one character left
    if (!char_is_line_ending(slice.char(prev_grapheme_boundary(slice, pos)))
        || direction == Direction::Forward)
        && char_is_line_ending(slice.char(pos.min(slice.len_chars().saturating_sub(1))))
    {
        pos = pos.saturating_sub(1);
    }

    let prev_line_ending = match direction {
        Direction::Forward => {
            char_is_line_ending(slice.char(nth_prev_grapheme_boundary(slice, pos, 2)))
                && char_is_line_ending(slice.char(prev_grapheme_boundary(slice, pos)))
        }
        Direction::Backward if pos == slice.len_chars() => true,
        Direction::Backward => {
            char_is_line_ending(slice.char(prev_grapheme_boundary(slice, pos)))
                && char_is_line_ending(slice.char(pos))
        }
    };

    // keep finding for two consecutive different line ending
    // have to subtract later since we take past one or more cycle
    // TODO swap this to use grapheme so \r\n works
    let mut found = true;
    let iter = chars_from_direction(&slice, pos, direction).take_while(|&c| {
        let now = prev_line_ending == char_is_line_ending(c);
        let ret = found || now; // stops when both is different
        found = now;
        ret
    });
    let count = iter.count();
    // count will be subtracted by extra whitespace due to interator
    match direction {
        Direction::Forward if pos + count == slice.len_chars() => slice.len_chars(),
        // subtract by 1 due to extra \n when going forward
        Direction::Forward if prev_line_ending => pos + count.saturating_sub(1),
        Direction::Forward => pos + count,
        // iterator exhausted so it should be 0
        Direction::Backward if pos.saturating_sub(count) == 0 => 0,
        // subtract by 2 because it starts with \n and have 2 extra \n when going backwards
        Direction::Backward if prev_line_ending => pos.saturating_sub(count.saturating_sub(2)),
        // subtract by 1 due to extra \n when going backward
        Direction::Backward => pos.saturating_sub(count.saturating_sub(1)),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextObject {
    Around,
    Inside,
}

impl Display for TextObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Around => "around",
            Self::Inside => "inside",
        })
    }
}

// count doesn't do anything yet
pub fn textobject_word(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    _count: usize,
    long: bool,
) -> Range {
    let pos = range.cursor(slice);

    let word_start = find_word_boundary(slice, pos, Direction::Backward, long);
    let word_end = match slice.get_char(pos).map(categorize_char) {
        None | Some(CharCategory::Whitespace | CharCategory::Eol) => pos,
        _ => find_word_boundary(slice, pos + 1, Direction::Forward, long),
    };

    // Special case.
    if word_start == word_end {
        return Range::new(word_start, word_end);
    }

    match textobject {
        TextObject::Inside => Range::new(word_start, word_end),
        TextObject::Around => {
            let whitespace_count_right = slice
                .chars_at(word_end)
                .take_while(|c| char_is_whitespace(*c))
                .count();

            if whitespace_count_right > 0 {
                Range::new(word_start, word_end + whitespace_count_right)
            } else {
                let whitespace_count_left = {
                    let mut iter = slice.chars_at(word_start);
                    iter.reverse();
                    iter.take_while(|c| char_is_whitespace(*c)).count()
                };
                Range::new(word_start - whitespace_count_left, word_end)
            }
        }
    }
}

pub fn textobject_paragraph(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    _count: usize,
) -> Range {
    let pos = range.cursor(slice);

    let paragraph_start = find_paragraph_boundary(slice, pos, Direction::Backward);
    let paragraph_end = match slice.get_char(pos) {
        Some(_) => find_paragraph_boundary(slice, pos + 1, Direction::Forward),
        None => pos,
    };

    match textobject {
        TextObject::Inside => Range::new(paragraph_start, paragraph_end),
        TextObject::Around => Range::new(
            // if it is at the end of the document and only matches newlines,
            // it search backward one step
            if slice.get_char(paragraph_start.saturating_sub(1)).is_some()
                && slice.get_char(paragraph_end).is_none()
            {
                find_paragraph_boundary(
                    slice,
                    paragraph_start.saturating_sub(1),
                    Direction::Backward,
                )
            } else {
                paragraph_start
            },
            match slice.get_char(paragraph_end) {
                Some(_) => find_paragraph_boundary(slice, paragraph_end + 1, Direction::Forward),
                None => paragraph_end,
            },
        ),
    }
}

pub fn textobject_surround(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    ch: char,
    count: usize,
) -> Range {
    surround::find_nth_pairs_pos(slice, ch, range.head, count)
        .map(|(anchor, head)| match textobject {
            TextObject::Inside => Range::new(next_grapheme_boundary(slice, anchor), head),
            TextObject::Around => Range::new(anchor, next_grapheme_boundary(slice, head)),
        })
        .unwrap_or(range)
}

/// Transform the given range to select text objects based on tree-sitter.
/// `object_name` is a query capture base name like "function", "class", etc.
/// `slice_tree` is the tree-sitter node corresponding to given text slice.
pub fn textobject_treesitter(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    object_name: &str,
    slice_tree: Node,
    lang_config: &LanguageConfiguration,
    _count: usize,
) -> Range {
    let get_range = move || -> Option<Range> {
        let byte_pos = slice.char_to_byte(range.cursor(slice));

        let capture_name = format!("{}.{}", object_name, textobject); // eg. function.inner
        let mut cursor = QueryCursor::new();
        let node = lang_config
            .textobject_query()?
            .capture_nodes(&capture_name, slice_tree, slice, &mut cursor)?
            .filter(|node| node.byte_range().contains(&byte_pos))
            .min_by_key(|node| node.byte_range().len())?;

        let len = slice.len_bytes();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        if start_byte >= len || end_byte >= len {
            return None;
        }

        let start_char = slice.byte_to_char(start_byte);
        let end_char = slice.byte_to_char(end_byte);

        Some(Range::new(start_char, end_char))
    };
    get_range().unwrap_or(range)
}

#[cfg(test)]
mod test {
    use super::TextObject::*;
    use super::*;

    use crate::Range;
    use ropey::Rope;

    #[test]
    fn test_textobject_word() {
        // (text, [(cursor position, textobject, final range), ...])
        let tests = &[
            (
                "cursor at beginning of doc",
                vec![(0, Inside, (0, 6)), (0, Around, (0, 7))],
            ),
            (
                "cursor at middle of word",
                vec![
                    (13, Inside, (10, 16)),
                    (10, Inside, (10, 16)),
                    (15, Inside, (10, 16)),
                    (13, Around, (10, 17)),
                    (10, Around, (10, 17)),
                    (15, Around, (10, 17)),
                ],
            ),
            (
                "cursor between word whitespace",
                vec![(6, Inside, (6, 6)), (6, Around, (6, 6))],
            ),
            (
                "cursor on word before newline\n",
                vec![
                    (22, Inside, (22, 29)),
                    (28, Inside, (22, 29)),
                    (25, Inside, (22, 29)),
                    (22, Around, (21, 29)),
                    (28, Around, (21, 29)),
                    (25, Around, (21, 29)),
                ],
            ),
            (
                "cursor on newline\nnext line",
                vec![(17, Inside, (17, 17)), (17, Around, (17, 17))],
            ),
            (
                "cursor on word after newline\nnext line",
                vec![
                    (29, Inside, (29, 33)),
                    (30, Inside, (29, 33)),
                    (32, Inside, (29, 33)),
                    (29, Around, (29, 34)),
                    (30, Around, (29, 34)),
                    (32, Around, (29, 34)),
                ],
            ),
            (
                "cursor on #$%:;* punctuation",
                vec![
                    (13, Inside, (10, 16)),
                    (10, Inside, (10, 16)),
                    (15, Inside, (10, 16)),
                    (13, Around, (10, 17)),
                    (10, Around, (10, 17)),
                    (15, Around, (10, 17)),
                ],
            ),
            (
                "cursor on punc%^#$:;.tuation",
                vec![
                    (14, Inside, (14, 21)),
                    (20, Inside, (14, 21)),
                    (17, Inside, (14, 21)),
                    (14, Around, (14, 21)),
                    (20, Around, (14, 21)),
                    (17, Around, (14, 21)),
                ],
            ),
            (
                "cursor in   extra whitespace",
                vec![
                    (9, Inside, (9, 9)),
                    (10, Inside, (10, 10)),
                    (11, Inside, (11, 11)),
                    (9, Around, (9, 9)),
                    (10, Around, (10, 10)),
                    (11, Around, (11, 11)),
                ],
            ),
            (
                "cursor on word   with extra whitespace",
                vec![(11, Inside, (10, 14)), (11, Around, (10, 17))],
            ),
            (
                "cursor at end with extra   whitespace",
                vec![(28, Inside, (27, 37)), (28, Around, (24, 37))],
            ),
            (
                "cursor at end of doc",
                vec![(19, Inside, (17, 20)), (19, Around, (16, 20))],
            ),
        ];

        for (sample, scenario) in tests {
            let doc = Rope::from(*sample);
            let slice = doc.slice(..);
            for &case in scenario {
                let (pos, objtype, expected_range) = case;
                let result = textobject_word(slice, Range::point(pos), objtype, 1, false);
                assert_eq!(
                    result,
                    expected_range.into(),
                    "\nCase failed: {:?} - {:?}",
                    sample,
                    case
                );
            }
        }
    }

    #[test]
    fn test_textobject_paragraph() {
        // (text, [(cursor position, textobject, final range), ...])
        let tests = &[
            ("\n", vec![(0, Inside, (0, 1)), (0, Around, (0, 1))]),
            (
                "p1\np1\n\np2\np2\n\n",
                vec![
                    (0, Inside, (0, 6)),
                    (0, Around, (0, 7)),
                    (1, Inside, (0, 6)),
                    (1, Around, (0, 7)),
                    (2, Inside, (0, 6)),
                    (2, Around, (0, 7)),
                    (3, Inside, (0, 6)),
                    (3, Around, (0, 7)),
                    (4, Inside, (0, 6)),
                    (4, Around, (0, 7)),
                    (5, Inside, (0, 6)),
                    (5, Around, (0, 7)),
                    (6, Inside, (6, 7)),
                    (6, Around, (6, 13)),
                    (7, Inside, (7, 13)),
                    (7, Around, (7, 14)),
                    (8, Inside, (7, 13)),
                    (8, Around, (7, 14)),
                    (9, Inside, (7, 13)),
                    (9, Around, (7, 14)),
                    (10, Inside, (7, 13)),
                    (10, Around, (7, 14)),
                    (11, Inside, (7, 13)),
                    (11, Around, (7, 14)),
                    (12, Inside, (7, 13)),
                    (12, Around, (7, 14)),
                    (13, Inside, (13, 14)),
                    (13, Around, (7, 14)),
                ],
            ),
        ];

        for (sample, scenario) in tests {
            let doc = Rope::from(*sample);
            let slice = doc.slice(..);
            for &case in scenario {
                let (pos, objtype, expected_range) = case;
                let result = textobject_paragraph(slice, Range::point(pos), objtype, 1);
                assert_eq!(
                    result,
                    expected_range.into(),
                    "\nCase failed: {:?} - {:?}",
                    sample,
                    case,
                );
            }
        }
    }

    #[test]
    fn test_textobject_surround() {
        // (text, [(cursor position, textobject, final range, count), ...])
        let tests = &[
            (
                "simple (single) surround pairs",
                vec![
                    (3, Inside, (3, 3), '(', 1),
                    (7, Inside, (8, 14), ')', 1),
                    (10, Inside, (8, 14), '(', 1),
                    (14, Inside, (8, 14), ')', 1),
                    (3, Around, (3, 3), '(', 1),
                    (7, Around, (7, 15), ')', 1),
                    (10, Around, (7, 15), '(', 1),
                    (14, Around, (7, 15), ')', 1),
                ],
            ),
            (
                "samexx 'single' surround pairs",
                vec![
                    (3, Inside, (3, 3), '\'', 1),
                    (7, Inside, (7, 7), '\'', 1),
                    (10, Inside, (8, 14), '\'', 1),
                    (14, Inside, (14, 14), '\'', 1),
                    (3, Around, (3, 3), '\'', 1),
                    (7, Around, (7, 7), '\'', 1),
                    (10, Around, (7, 15), '\'', 1),
                    (14, Around, (14, 14), '\'', 1),
                ],
            ),
            (
                "(nested (surround (pairs)) 3 levels)",
                vec![
                    (0, Inside, (1, 35), '(', 1),
                    (6, Inside, (1, 35), ')', 1),
                    (8, Inside, (9, 25), '(', 1),
                    (8, Inside, (9, 35), ')', 2),
                    (20, Inside, (9, 25), '(', 2),
                    (20, Inside, (1, 35), ')', 3),
                    (0, Around, (0, 36), '(', 1),
                    (6, Around, (0, 36), ')', 1),
                    (8, Around, (8, 26), '(', 1),
                    (8, Around, (8, 36), ')', 2),
                    (20, Around, (8, 26), '(', 2),
                    (20, Around, (0, 36), ')', 3),
                ],
            ),
            (
                "(mixed {surround [pair] same} line)",
                vec![
                    (2, Inside, (1, 34), '(', 1),
                    (9, Inside, (8, 28), '{', 1),
                    (18, Inside, (18, 22), '[', 1),
                    (2, Around, (0, 35), '(', 1),
                    (9, Around, (7, 29), '{', 1),
                    (18, Around, (17, 23), '[', 1),
                ],
            ),
            (
                "(stepped (surround) pairs (should) skip)",
                vec![(22, Inside, (1, 39), '(', 1), (22, Around, (0, 40), '(', 1)],
            ),
            (
                "[surround pairs{\non different]\nlines}",
                vec![
                    (7, Inside, (1, 29), '[', 1),
                    (15, Inside, (16, 36), '{', 1),
                    (7, Around, (0, 30), '[', 1),
                    (15, Around, (15, 37), '{', 1),
                ],
            ),
        ];

        for (sample, scenario) in tests {
            let doc = Rope::from(*sample);
            let slice = doc.slice(..);
            for &case in scenario {
                let (pos, objtype, expected_range, ch, count) = case;
                let result = textobject_surround(slice, Range::point(pos), objtype, ch, count);
                assert_eq!(
                    result,
                    expected_range.into(),
                    "\nCase failed: {:?} - {:?}",
                    sample,
                    case
                );
            }
        }
    }
}
