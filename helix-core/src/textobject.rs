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

        // initial, textobject, final
        let cases = &[
            // cursor at [t]ext
            ((0, 0), Inner, (0, 3)),  // [with]
            ((0, 0), Around, (0, 4)), // [with ]
            // cursor at w[i]th
            ((6, 6), Inner, (5, 8)),  // [with]
            ((6, 6), Around, (5, 9)), // [with ]
            // cursor at text[ ]with
            ((4, 4), Inner, (4, 4)),  // no change
            ((4, 4), Around, (4, 8)), // [ with]
            // cursor at [c]hars
            ((10, 10), Inner, (10, 14)), // [chars]
            ((10, 10), Around, (9, 14)), // [ chars]
            // cursor at char[s]
            ((14, 14), Inner, (10, 14)), // [chars]
            ((14, 14), Around, (9, 14)), // [ chars]
            // cursor at chars[\n]more
            ((15, 15), Inner, (15, 15)),  // no change
            ((15, 15), Around, (15, 20)), // [\nmore ]
            // cursor at [m]ore
            ((16, 16), Inner, (16, 19)),  // [more]
            ((16, 16), Around, (16, 20)), // [more ]
            // cursor at $!@[%]
            ((30, 30), Inner, (27, 30)), // [$!@%]
            // ((30, 30), Around, (27, 30)), // [$!@%]
            // cursor at word [ ] next
            ((36, 36), Inner, (36, 36)),  // no change
            ((36, 36), Around, (35, 41)), // word[   next]
            // cursor at som[e]
            ((46, 46), Inner, (43, 46)),  // [some]
            ((46, 46), Around, (42, 46)), // [ some]
        ];

        for &case in cases {
            let (before, textobject, after) = case;
            let before = Range::new(before.0, before.1);
            let expected = Range::new(after.0, after.1);
            let result = textobject_word(slice, before, textobject, 1);
            assert_eq!(expected, result, "\n{:?}", case);
        }
    }
}
