use ropey::{Rope, RopeSlice};

use crate::chars::{categorize_char, char_is_line_ending, char_is_whitespace, CharCategory};
use crate::movement::{self, Direction};
use crate::surround;
use crate::Range;

fn this_word_end_pos(slice: RopeSlice, mut pos: usize) -> usize {
    this_word_bound_pos(slice, pos, Direction::Forward)
}

fn this_word_start_pos(slice: RopeSlice, mut pos: usize) -> usize {
    this_word_bound_pos(slice, pos, Direction::Backward)
}

fn this_word_bound_pos(slice: RopeSlice, mut pos: usize, direction: Direction) -> usize {
    let iter = match direction {
        Direction::Forward => slice.chars_at(pos + 1),
        Direction::Backward => {
            let mut iter = slice.chars_at(pos);
            iter.reverse();
            iter
        }
    };

    match categorize_char(slice.char(pos)) {
        CharCategory::Eol | CharCategory::Whitespace => pos,
        category => {
            for peek in iter {
                let curr_category = categorize_char(peek);
                if curr_category != category
                    || curr_category == CharCategory::Eol
                    || curr_category == CharCategory::Whitespace
                {
                    return pos;
                }
                pos = match direction {
                    Direction::Forward => pos + 1,
                    Direction::Backward => pos.saturating_sub(1),
                }
            }
            pos
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextObject {
    Around,
    Inside,
}

// count doesn't do anything yet
pub fn textobject_word(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    count: usize,
) -> Range {
    let this_word_start = this_word_start_pos(slice, range.head);
    let this_word_end = this_word_end_pos(slice, range.head);

    let (anchor, head);
    match textobject {
        TextObject::Inside => {
            anchor = this_word_start;
            head = this_word_end;
        }
        TextObject::Around => {
            if slice
                .get_char(this_word_end + 1)
                .map_or(true, char_is_line_ending)
            {
                head = this_word_end;
                if slice
                    .get_char(this_word_start - 1)
                    .map_or(true, char_is_line_ending)
                {
                    // single word on a line
                    anchor = this_word_start;
                } else {
                    // last word on a line, select the whitespace before it too
                    anchor = movement::move_prev_word_end(slice, range, count).head;
                }
            } else if char_is_whitespace(slice.char(range.head)) {
                // select whole whitespace and next word
                head = movement::move_next_word_end(slice, range, count).head;
                anchor = movement::backwards_skip_while(slice, range.head, |c| c.is_whitespace())
                    .map(|p| p + 1) // p is first *non* whitespace char, so +1 to get whitespace pos
                    .unwrap_or(0);
            } else {
                head = movement::move_next_word_start(slice, range, count).head;
                anchor = this_word_start;
            }
        }
    };
    Range::new(anchor, head)
}

pub fn textobject_paragraph(
    slice: RopeSlice,
    range: Range,
    textobject: TextObject,
    count: usize,
) -> Range {
    Range::point(0)
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
            TextObject::Inside => Range::new(anchor + 1, head - 1),
            TextObject::Around => Range::new(anchor, head),
        })
        .unwrap_or(range)
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
                vec![(0, Inside, (0, 5)), (0, Around, (0, 6))],
            ),
            (
                "cursor at middle of word",
                vec![
                    (13, Inside, (10, 15)),
                    (10, Inside, (10, 15)),
                    (15, Inside, (10, 15)),
                    (13, Around, (10, 16)),
                    (10, Around, (10, 16)),
                    (15, Around, (10, 16)),
                ],
            ),
            (
                "cursor between word whitespace",
                vec![(6, Inside, (6, 6)), (6, Around, (6, 13))],
            ),
            (
                "cursor on word before newline\n",
                vec![
                    (22, Inside, (22, 28)),
                    (28, Inside, (22, 28)),
                    (25, Inside, (22, 28)),
                    (22, Around, (21, 28)),
                    (28, Around, (21, 28)),
                    (25, Around, (21, 28)),
                ],
            ),
            (
                "cursor on newline\nnext line",
                vec![(17, Inside, (17, 17)), (17, Around, (17, 22))],
            ),
            (
                "cursor on word after newline\nnext line",
                vec![
                    (29, Inside, (29, 32)),
                    (30, Inside, (29, 32)),
                    (32, Inside, (29, 32)),
                    (29, Around, (29, 33)),
                    (30, Around, (29, 33)),
                    (32, Around, (29, 33)),
                ],
            ),
            (
                "cursor on #$%:;* punctuation",
                vec![
                    (13, Inside, (10, 15)),
                    (10, Inside, (10, 15)),
                    (15, Inside, (10, 15)),
                    (13, Around, (10, 16)),
                    (10, Around, (10, 16)),
                    (15, Around, (10, 16)),
                ],
            ),
            (
                "cursor on punc%^#$:;.tuation",
                vec![
                    (14, Inside, (14, 20)),
                    (20, Inside, (14, 20)),
                    (17, Inside, (14, 20)),
                    (14, Around, (14, 20)),
                    // FIXME: edge case
                    // (20, Around, (14, 20)),
                    (17, Around, (14, 20)),
                ],
            ),
            (
                "cursor in   extra whitespace",
                vec![
                    (9, Inside, (9, 9)),
                    (10, Inside, (10, 10)),
                    (11, Inside, (11, 11)),
                    (9, Around, (9, 16)),
                    (10, Around, (9, 16)),
                    (11, Around, (9, 16)),
                ],
            ),
            (
                "cursor at end of doc",
                vec![(19, Inside, (17, 19)), (19, Around, (16, 19))],
            ),
        ];

        for (sample, scenario) in tests {
            let doc = Rope::from(*sample);
            let slice = doc.slice(..);
            for &case in scenario {
                let (pos, objtype, expected_range) = case;
                let result = textobject_word(slice, Range::point(pos), objtype, 1);
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
    fn test_textobject_surround() {
        // (text, [(cursor position, textobject, final range, count), ...])
        let tests = &[
            (
                "simple (single) surround pairs",
                vec![
                    (3, Inside, (3, 3), '(', 1),
                    (7, Inside, (8, 13), ')', 1),
                    (10, Inside, (8, 13), '(', 1),
                    (14, Inside, (8, 13), ')', 1),
                    (3, Around, (3, 3), '(', 1),
                    (7, Around, (7, 14), ')', 1),
                    (10, Around, (7, 14), '(', 1),
                    (14, Around, (7, 14), ')', 1),
                ],
            ),
            (
                "samexx 'single' surround pairs",
                vec![
                    (3, Inside, (3, 3), '\'', 1),
                    // FIXME: surround doesn't work when *on* same chars pair
                    // (7, Inner, (8, 13), '\'', 1),
                    (10, Inside, (8, 13), '\'', 1),
                    // (14, Inner, (8, 13), '\'', 1),
                    (3, Around, (3, 3), '\'', 1),
                    // (7, Around, (7, 14), '\'', 1),
                    (10, Around, (7, 14), '\'', 1),
                    // (14, Around, (7, 14), '\'', 1),
                ],
            ),
            (
                "(nested (surround (pairs)) 3 levels)",
                vec![
                    (0, Inside, (1, 34), '(', 1),
                    (6, Inside, (1, 34), ')', 1),
                    (8, Inside, (9, 24), '(', 1),
                    (8, Inside, (9, 34), ')', 2),
                    (20, Inside, (9, 24), '(', 2),
                    (20, Inside, (1, 34), ')', 3),
                    (0, Around, (0, 35), '(', 1),
                    (6, Around, (0, 35), ')', 1),
                    (8, Around, (8, 25), '(', 1),
                    (8, Around, (8, 35), ')', 2),
                    (20, Around, (8, 25), '(', 2),
                    (20, Around, (0, 35), ')', 3),
                ],
            ),
            (
                "(mixed {surround [pair] same} line)",
                vec![
                    (2, Inside, (1, 33), '(', 1),
                    (9, Inside, (8, 27), '{', 1),
                    (18, Inside, (18, 21), '[', 1),
                    (2, Around, (0, 34), '(', 1),
                    (9, Around, (7, 28), '{', 1),
                    (18, Around, (17, 22), '[', 1),
                ],
            ),
            (
                "(stepped (surround) pairs (should) skip)",
                vec![(22, Inside, (1, 38), '(', 1), (22, Around, (0, 39), '(', 1)],
            ),
            (
                "[surround pairs{\non different]\nlines}",
                vec![
                    (7, Inside, (1, 28), '[', 1),
                    (15, Inside, (16, 35), '{', 1),
                    (7, Around, (0, 29), '[', 1),
                    (15, Around, (15, 36), '{', 1),
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
