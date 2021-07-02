use ropey::{Rope, RopeSlice};

use crate::chars::{categorize_char, char_is_line_ending, char_is_whitespace, CharCategory};
use crate::movement;
use crate::surround;
use crate::Range;

fn this_word_end_pos(slice: RopeSlice, mut pos: usize) -> usize {
    match categorize_char(slice.char(pos)) {
        CharCategory::Eol | CharCategory::Whitespace => pos,
        category => {
            for c in slice.chars_at(pos) {
                let curr_category = categorize_char(c);
                if curr_category != category
                    || curr_category == CharCategory::Eol
                    || curr_category == CharCategory::Whitespace
                {
                    return pos.saturating_sub(1);
                }
                pos += 1;
            }
            pos.saturating_sub(1)
        }
    }
}

fn this_word_start_pos(slice: RopeSlice, mut pos: usize) -> usize {
    match categorize_char(slice.char(pos)) {
        CharCategory::Eol | CharCategory::Whitespace => pos,
        category => {
            let mut iter = slice.chars_at(pos + 1);
            for c in std::iter::from_fn(|| iter.prev()) {
                let curr_category = categorize_char(c);
                if curr_category != category
                    || curr_category == CharCategory::Eol
                    || curr_category == CharCategory::Whitespace
                {
                    return pos + 1;
                }
                pos = pos.saturating_sub(1);
            }
            pos
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextObject {
    Around,
    Inner,
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
        TextObject::Inner => {
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
            TextObject::Inner => Range::new(anchor + 1, head - 1),
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
        let doc = Rope::from("text with chars\nmore lines\n$!@%word   next some");
        let slice = doc.slice(..);

        // text, cursor head, textobject, final range
        let tests = &[
            (
                "cursor at beginning of doc",
                vec![(0, Inner, (0, 5)), (0, Around, (0, 6))],
            ),
            (
                "cursor at middle of word",
                vec![
                    (13, Inner, (10, 15)),
                    (10, Inner, (10, 15)),
                    (15, Inner, (10, 15)),
                    (13, Around, (10, 16)),
                    (10, Around, (10, 16)),
                    (15, Around, (10, 16)),
                ],
            ),
            (
                "cursor between word whitespace",
                vec![(6, Inner, (6, 6)), (6, Around, (6, 13))],
            ),
            (
                "cursor on word before newline\n",
                vec![
                    (22, Inner, (22, 28)),
                    (28, Inner, (22, 28)),
                    (25, Inner, (22, 28)),
                    (22, Around, (21, 28)),
                    (28, Around, (21, 28)),
                    (25, Around, (21, 28)),
                ],
            ),
            (
                "cursor on newline\nnext line",
                vec![(17, Inner, (17, 17)), (17, Around, (17, 22))],
            ),
            (
                "cursor on word after newline\nnext line",
                vec![
                    (29, Inner, (29, 32)),
                    (30, Inner, (29, 32)),
                    (32, Inner, (29, 32)),
                    (29, Around, (29, 33)),
                    (30, Around, (29, 33)),
                    (32, Around, (29, 33)),
                ],
            ),
            (
                "cursor on #$%:;* punctuation",
                vec![
                    (13, Inner, (10, 15)),
                    (10, Inner, (10, 15)),
                    (15, Inner, (10, 15)),
                    (13, Around, (10, 16)),
                    (10, Around, (10, 16)),
                    (15, Around, (10, 16)),
                ],
            ),
            (
                "cursor on punc%^#$:;.tuation",
                vec![
                    (14, Inner, (14, 20)),
                    (20, Inner, (14, 20)),
                    (17, Inner, (14, 20)),
                    (14, Around, (14, 20)),
                    // FIXME: edge case
                    // (20, Around, (14, 20)),
                    (17, Around, (14, 20)),
                ],
            ),
            (
                "cursor in   extra whitespace",
                vec![
                    (9, Inner, (9, 9)),
                    (10, Inner, (10, 10)),
                    (11, Inner, (11, 11)),
                    (9, Around, (9, 16)),
                    (10, Around, (9, 16)),
                    (11, Around, (9, 16)),
                ],
            ),
            (
                "cursor at end of doc",
                vec![(19, Inner, (17, 19)), (19, Around, (16, 19))],
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
}
