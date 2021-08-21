use ropey::RopeSlice;

use crate::chars::{categorize_char, char_is_whitespace, CharCategory};
use crate::graphemes::next_grapheme_boundary;
use crate::movement::Direction;
use crate::surround;
use crate::SelectionRange;

fn find_word_boundary(slice: RopeSlice, mut pos: usize, direction: Direction) -> usize {
    use CharCategory::{Eol, Whitespace};

    let iter = match direction {
        Direction::Forward => slice.chars_at(pos),
        Direction::Backward => {
            let mut iter = slice.chars_at(pos);
            iter.reverse();
            iter
        }
    };

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
                if category != prev_category && pos != 0 && pos != slice.len_chars() {
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextObject {
    Around,
    Inside,
}

// count doesn't do anything yet
pub fn textobject_word(
    slice: RopeSlice,
    range: SelectionRange,
    textobject: TextObject,
    _count: usize,
) -> SelectionRange {
    let pos = range.cursor(slice);

    let word_start = find_word_boundary(slice, pos, Direction::Backward);
    let word_end = match slice.get_char(pos).map(categorize_char) {
        None | Some(CharCategory::Whitespace | CharCategory::Eol) => pos,
        _ => find_word_boundary(slice, pos + 1, Direction::Forward),
    };

    // Special case.
    if word_start == word_end {
        return SelectionRange::new(word_start, word_end);
    }

    match textobject {
        TextObject::Inside => SelectionRange::new(word_start, word_end),
        TextObject::Around => {
            let whitespace_count_right = slice
                .chars_at(word_end)
                .take_while(|c| char_is_whitespace(*c))
                .count();

            if whitespace_count_right > 0 {
                SelectionRange::new(word_start, word_end + whitespace_count_right)
            } else {
                let whitespace_count_left = {
                    let mut iter = slice.chars_at(word_start);
                    iter.reverse();
                    iter.take_while(|c| char_is_whitespace(*c)).count()
                };
                SelectionRange::new(word_start - whitespace_count_left, word_end)
            }
        }
    }
}

pub fn textobject_surround(
    slice: RopeSlice,
    range: SelectionRange,
    textobject: TextObject,
    ch: char,
    count: usize,
) -> SelectionRange {
    surround::find_nth_pairs_pos(slice, ch, range.head, count)
        .map(|(anchor, head)| match textobject {
            TextObject::Inside => SelectionRange::new(next_grapheme_boundary(slice, anchor), head),
            TextObject::Around => SelectionRange::new(anchor, next_grapheme_boundary(slice, head)),
        })
        .unwrap_or(range)
}

#[cfg(test)]
mod test {
    use super::TextObject::*;
    use super::*;

    use crate::SelectionRange;
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
                let result = textobject_word(slice, SelectionRange::point(pos), objtype, 1);
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
                let result = textobject_surround(slice, SelectionRange::point(pos), objtype, ch, count);
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
