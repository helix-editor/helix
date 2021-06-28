use ropey::{Rope, RopeSlice};

use crate::chars::{char_is_line_ending, char_is_whitespace};
use crate::movement;
use crate::surround;
use crate::Range;

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
    let this_word_start = movement::move_this_word_start(slice, range, count).head;
    let this_word_end = movement::move_this_word_end(slice, range, count).head;

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
                anchor = movement::move_this_word_prev_bound(slice, range, count).head;
            } else {
                head = movement::move_next_word_start(slice, range, count).head;
                anchor = movement::move_this_word_start(slice, range, count).head;
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
        let doc = Rope::from("text with chars\nmore lines\n$!@%word   next ");
        let slice = doc.slice(..);

        // initial, textobject, final
        let cases = &[
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
