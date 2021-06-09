use std::iter;

use crate::{
    coords_at_pos,
    graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary},
    pos_at_coords, Position, Range, RopeSlice,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Movement {
    Extend,
    Move,
}

pub fn move_horizontally(
    slice: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    behaviour: Movement,
) -> Range {
    let pos = range.head;
    let line = slice.char_to_line(pos);
    // TODO: we can optimize clamping by passing in RopeSlice limited to current line. that way
    // we stop calculating past start/end of line.
    let pos = match dir {
        Direction::Backward => {
            let start = slice.line_to_char(line);
            nth_prev_grapheme_boundary(slice, pos, count).max(start)
        }
        Direction::Forward => {
            // Line end is pos at the start of next line - 1
            let end = slice.line_to_char(line + 1).saturating_sub(1);
            nth_next_grapheme_boundary(slice, pos, count).min(end)
        }
    };
    let anchor = match behaviour {
        Movement::Extend => range.anchor,
        Movement::Move => pos,
    };
    Range::new(anchor, pos)
}

pub fn move_vertically(
    slice: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    behaviour: Movement,
) -> Range {
    let Position { row, col } = coords_at_pos(slice, range.head);

    let horiz = range.horiz.unwrap_or(col as u32);

    let new_line = match dir {
        Direction::Backward => row.saturating_sub(count),
        Direction::Forward => std::cmp::min(
            row.saturating_add(count),
            slice.len_lines().saturating_sub(2),
        ),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = slice.line(new_line).len_chars().saturating_sub(2);

    let new_col = std::cmp::min(horiz as usize, new_line_len);

    let pos = pos_at_coords(slice, Position::new(new_line, new_col));

    let anchor = match behaviour {
        Movement::Extend => range.anchor,
        Movement::Move => pos,
    };

    let mut range = Range::new(anchor, pos);
    range.horiz = Some(horiz);
    range
}

pub fn move_next_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    let movement = |range: Range| -> Option<Range> {
        let after_head = (range.head + 1).skip_newlines(slice);
        (after_head + 1).inside(slice).then(|| {
            let new_anchor = if range.head.is_boundary(slice) {
                after_head
            } else {
                range.head.skip_newlines(slice)
            };
            let new_head = (range.head + 1).end_of_block(slice);
            Some(Range::new(new_anchor, new_head))
        })?
    };
    (0..count).fold(range, |range, _| movement(range).unwrap_or(range))
}

pub fn move_prev_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    let movement = |range: Range| -> Option<Range> {
        (range.head > 0 && range.head.inside(slice)).then(|| {
            let new_anchor = if range.head.saturating_sub(1).is_boundary(slice) {
                (range.head.saturating_sub(1)).backwards_skip_newlines(slice)
            } else {
                range.head.backwards_skip_newlines(slice)
            };
            let new_head = range.head.saturating_sub(1).start_of_block(slice);
            Some(Range::new(new_anchor, new_head))
        })?
    };
    (0..count).fold(range, |range, _| movement(range).unwrap_or(range))
}

pub fn move_next_word_end(slice: RopeSlice, range: Range, count: usize) -> Range {
    let movement = |range: Range| -> Option<Range> {
        let after_head = (range.head + 1).skip_newlines(slice);
        (after_head + 1).inside(slice).then(|| {
            let new_anchor = if range.head.is_boundary(slice) {
                after_head
            } else {
                range.head.skip_newlines(slice)
            };
            let new_head = (range.head + 1).end_of_word(slice);
            Some(Range::new(new_anchor, new_head))
        })?
    };
    (0..count).fold(range, |range, _| movement(range).unwrap_or(range))
}

// ---- util ------------

/// Private helpers to help manipulate slice indices
pub trait SliceIndexHelpers {
    fn outside(&self, slice: RopeSlice) -> bool;
    fn inside(&self, slice: RopeSlice) -> bool;
    /// The next character after this belongs
    /// to a different `Category`
    fn is_boundary(&self, slice: RopeSlice) -> bool;
    fn category(&self, slice: RopeSlice) -> Option<Category>;
    /// Returns the start of a word/punctuation group followed by any amount of whitespace.
    fn start_of_block(&self, slice: RopeSlice) -> Self;
    /// Returns the end of a word/punctuation group followed by any amount of whitespace.
    fn end_of_block(&self, slice: RopeSlice) -> Self;
    /// Returns the end of a word/punctuation group.
    fn end_of_word(&self, slice: RopeSlice) -> Self;
    fn skip_newlines(&self, slice: RopeSlice) -> Self;
    fn backwards_skip_newlines(&self, slice: RopeSlice) -> Self;
    fn is_whitespace(&self, slice: RopeSlice) -> bool;
    fn is_end_of_line(&self, slice: RopeSlice) -> bool;
}

impl SliceIndexHelpers for usize {
    fn inside(&self, slice: RopeSlice) -> bool {
        *self < slice.len_chars()
    }

    fn outside(&self, slice: RopeSlice) -> bool {
        !self.inside(slice)
    }

    fn is_boundary(&self, slice: RopeSlice) -> bool {
        (self + 1).inside(slice)
            && (categorize(slice.char(*self)) != categorize(slice.char(self + 1)))
    }

    fn category(&self, slice: RopeSlice) -> Option<Category> {
        self.inside(slice).then(|| categorize(slice.char(*self)))
    }

    fn end_of_word(&self, slice: RopeSlice) -> Self {
        // Scan the entire slice
        (*self..slice.len_chars())
            // Skip any initial newlines, as they must be skipped over for
            // the purposes of word movement
            .skip_while(|i| is_end_of_line(slice.char(*i)))
            // Find the first boundary that doesn't start from whitespace
            .find(|pos| {
                pos.is_boundary(slice)
                    && (!pos.is_whitespace(slice) || (pos + 1).is_end_of_line(slice))
            })
            // If not found, return the end of the range
            .unwrap_or_else(|| slice.len_chars().saturating_sub(1))
    }

    fn end_of_block(&self, slice: RopeSlice) -> Self {
        // Scan the entire slice
        (*self..slice.len_chars())
            // Skip any initial newlines, as they must be skipped over for
            // the purposes of word movement
            .skip_while(|pos| pos.is_end_of_line(slice))
            // Find the first boundary that doesn't go into whitespace or EOL
            .find(|pos| {
                pos.is_boundary(slice)
                    &&((pos + 1).is_end_of_line(slice) || !slice.char(*pos + 1).is_whitespace())
            })
            // If not found, return the end of the range
            .unwrap_or_else(|| slice.len_chars().saturating_sub(1))
    }

    fn start_of_block(&self, slice: RopeSlice) -> Self {
        // Scan the entire slice backwards, skipping any initial newlines,
        // as they must be skipped over for the purposes of word movement
        (0..=self.backwards_skip_newlines(slice))
            .rev()
            // Skip any and all whitespace that isn't preceded by newlines
            // (Whitespace preceded by a newline forms a block)
            .skip_while(|pos| {
                is_strict_whitespace(slice.char(*pos))
                    && !pos.saturating_sub(1).is_end_of_line(slice)
            })
            // Find the first boundary
            .find(|pos| pos.saturating_sub(1).is_boundary(slice))
            .unwrap_or(0)
    }

    fn skip_newlines(&self, slice: RopeSlice) -> Self {
        skip_while(slice, *self, is_end_of_line).unwrap_or_else(|| slice.len_chars().saturating_sub(1))
    }

    fn backwards_skip_newlines(&self, slice: RopeSlice) -> Self {
        backwards_skip_while(slice, *self, is_end_of_line).unwrap_or(0)
    }

    fn is_whitespace(&self, slice: RopeSlice) -> bool {
        slice.char(*self).is_whitespace()
    }

    fn is_end_of_line(&self, slice: RopeSlice) -> bool {
        is_end_of_line(slice.char(*self))
    }
}

#[inline]
pub(crate) fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[inline]
pub(crate) fn is_end_of_line(ch: char) -> bool {
    ch == '\n'
}

#[inline]
// Whitespace, but not end of line
pub(crate) fn is_strict_whitespace(ch: char) -> bool {
    ch.is_whitespace() && !is_end_of_line(ch)
}

#[inline]
pub(crate) fn is_punctuation(ch: char) -> bool {
    use unicode_general_category::{get_general_category, GeneralCategory};

    matches!(
        get_general_category(ch),
        GeneralCategory::OtherPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
    )
}

#[derive(Debug, Eq, PartialEq)]
pub enum Category {
    Whitespace,
    Eol,
    Word,
    Punctuation,
    Unknown,
}

#[inline]
pub(crate) fn categorize(ch: char) -> Category {
    if is_end_of_line(ch) {
        Category::Eol
    } else if ch.is_whitespace() {
        Category::Whitespace
    } else if is_word(ch) {
        Category::Word
    } else if is_punctuation(ch) {
        Category::Punctuation
    } else {
        Category::Unknown
    }
}

#[inline]
/// Returns first index that doesn't satisfy a given predicate when
/// advancing the character index.
///
/// Returns none if all characters satisfy the predicate.
pub fn skip_while<F>(slice: RopeSlice, pos: usize, fun: F) -> Option<usize>
where
    F: Fn(char) -> bool,
{
    if pos.outside(slice) {
        None
    } else {
        let mut chars = slice.chars_at(pos).enumerate();
        chars.find_map(|(i, c)| if !fun(c) { Some(pos + i) } else { None })
    }
}

#[inline]
/// Returns first index that doesn't satisfy a given predicate when
/// retreating the character index, saturating if all elements satisfy
/// the condition.
pub fn backwards_skip_while<F>(slice: RopeSlice, pos: usize, fun: F) -> Option<usize>
where
    F: Fn(char) -> bool,
{
    if pos.outside(slice) {
        None
    } else {
        let mut chars_starting_from_next = slice.chars_at(pos + 1);
        let mut backwards = iter::from_fn(|| chars_starting_from_next.prev()).enumerate();
        backwards.find_map(|(i, c)| {
            if !fun(c) {
                Some(pos.saturating_sub(i))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::array::{self, IntoIter};

    use ropey::Rope;

    use super::*;

    const SINGLE_LINE_SAMPLE: &str = "This is a simple alphabetic line";
    const MULTILINE_SAMPLE: &str = "\
        Multiline\n\
        text sample\n\
        which\n\
        is merely alphabetic\n\
        and whitespaced\n\
    ";

    const PUNCTUATION_SAMPLE: &str = "\
        Multiline, example    with,, some;
        ... punctuation!    \n
    ";

    const MULTIBYTE_CHARACTER_SAMPLE: &str = "\
        パーティーへ行かないか\n\
        The text above is Japanese\n\
    ";

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let slice = text.slice(..);
        let pos = pos_at_coords(slice, (0, 4).into());

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                slice,
                move_vertically(slice, range, Direction::Forward, 1, Movement::Move).head
            ),
            (1, 2).into()
        );
    }

    #[test]
    fn horizontal_moves_through_single_line_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::point(position);

        let moves_and_expected_coordinates = [
            ((Direction::Forward, 1usize), (0, 1)),
            ((Direction::Forward, 2usize), (0, 3)),
            ((Direction::Forward, 0usize), (0, 3)),
            ((Direction::Forward, 999usize), (0, 31)),
            ((Direction::Forward, 999usize), (0, 31)),
            ((Direction::Backward, 999usize), (0, 0)),
        ];

        for ((direction, amount), coordinates) in IntoIter::new(moves_and_expected_coordinates) {
            range = move_horizontally(slice, range, direction, amount, Movement::Move);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into())
        }
    }

    #[test]
    fn horizontal_moves_through_single_line_in_multiline_text() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::point(position);

        let moves_and_expected_coordinates = IntoIter::new([
            ((Direction::Forward, 1usize), (0, 1)),    // M_ltiline
            ((Direction::Forward, 2usize), (0, 3)),    // Mul_iline
            ((Direction::Backward, 6usize), (0, 0)),   // _ultiline
            ((Direction::Backward, 999usize), (0, 0)), // _ultiline
            ((Direction::Forward, 3usize), (0, 3)),    // Mul_iline
            ((Direction::Forward, 0usize), (0, 3)),    // Mul_iline
            ((Direction::Backward, 0usize), (0, 3)),   // Mul_iline
            ((Direction::Forward, 999usize), (0, 9)),  // Multilin_
            ((Direction::Forward, 999usize), (0, 9)),  // Multilin_
        ]);

        for ((direction, amount), coordinates) in moves_and_expected_coordinates {
            range = move_horizontally(slice, range, direction, amount, Movement::Move);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn selection_extending_moves_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::point(position);
        let original_anchor = range.anchor;

        let moves = IntoIter::new([
            (Direction::Forward, 1usize),
            (Direction::Forward, 5usize),
            (Direction::Backward, 3usize),
        ]);

        for (direction, amount) in moves {
            range = move_horizontally(slice, range, direction, amount, Movement::Extend);
            assert_eq!(range.anchor, original_anchor);
        }
    }

    #[test]
    fn vertical_moves_in_single_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = dbg!(&text).slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::point(position);
        let moves_and_expected_coordinates = IntoIter::new([
            ((Direction::Forward, 1usize), (1, 0)),
            ((Direction::Forward, 2usize), (3, 0)),
            ((Direction::Backward, 999usize), (0, 0)),
            ((Direction::Forward, 3usize), (3, 0)),
            ((Direction::Forward, 0usize), (3, 0)),
            ((Direction::Backward, 0usize), (3, 0)),
            ((Direction::Forward, 5), (4, 0)),
            ((Direction::Forward, 999usize), (4, 0)),
        ]);

        for ((direction, amount), coordinates) in moves_and_expected_coordinates {
            range = move_vertically(slice, range, direction, amount, Movement::Move);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn vertical_moves_jumping_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::point(position);

        enum Axis {
            H,
            V,
        }
        let moves_and_expected_coordinates = IntoIter::new([
            // Places cursor at the end of line
            ((Axis::H, Direction::Forward, 8usize), (0, 8)),
            // First descent preserves column as the target line is wider
            ((Axis::V, Direction::Forward, 1usize), (1, 8)),
            // Second descent clamps column as the target line is shorter
            ((Axis::V, Direction::Forward, 1usize), (2, 4)),
            // Third descent restores the original column
            ((Axis::V, Direction::Forward, 1usize), (3, 8)),
            // Behaviour is preserved even through long jumps
            ((Axis::V, Direction::Backward, 999usize), (0, 8)),
            ((Axis::V, Direction::Forward, 999usize), (4, 8)),
        ]);

        for ((axis, direction, amount), coordinates) in moves_and_expected_coordinates {
            range = match axis {
                Axis::H => move_horizontally(slice, range, direction, amount, Movement::Move),
                Axis::V => move_vertically(slice, range, direction, amount, Movement::Move),
            };
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn multibyte_character_column_jumps() {
        let text = Rope::from(MULTIBYTE_CHARACTER_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::point(position);

        // FIXME: The behaviour captured in this test diverges from both Kakoune and Vim. These
        // will attempt to preserve the horizontal position of the cursor, rather than
        // placing it at the same character index.
        enum Axis {
            H,
            V,
        }
        let moves_and_expected_coordinates = IntoIter::new([
            // Places cursor at the fourth kana
            ((Axis::H, Direction::Forward, 4), (0, 4)),
            // Descent places cursor at the fourth character.
            ((Axis::V, Direction::Forward, 1usize), (1, 4)),
        ]);

        for ((axis, direction, amount), coordinates) in moves_and_expected_coordinates {
            range = match axis {
                Axis::H => move_horizontally(slice, range, direction, amount, Movement::Move),
                Axis::V => move_vertically(slice, range, direction, amount, Movement::Move),
            };
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    enum Motion {
        NextStart(usize),
        NextEnd(usize),
        PrevStart(usize),
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_next_words() {
        let tests = array::IntoIter::new([
            ("Basic forward motion stops at the first space",
                vec![(1, Range::new(0, 0), Range::new(0, 5))]),
            (" Starting from a boundary advances the anchor",
                vec![(1, Range::new(0, 0), Range::new(1, 9))]),
            ("Long       whitespace gap is bridged by the head",
                vec![(1, Range::new(0, 0), Range::new(0, 10))]),
            ("Previous anchor is irrelevant for forward motions",
                vec![(1, Range::new(12, 0), Range::new(0, 8))]),
            ("    Starting from whitespace moves to last space in sequence",
                vec![(1, Range::new(0, 0), Range::new(0, 3))]),
            ("Starting from mid-word leaves anchor at start position and moves head",
                vec![(1, Range::new(3, 3), Range::new(3, 8))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 28))]),
            ("Jumping\n    into starting whitespace selects the spaces before 'into'",
                vec![(1, Range::new(0, 6), Range::new(8, 11))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 11)),
                    (1, Range::new(0, 11), Range::new(12, 14)),
                    (1, Range::new(12, 14), Range::new(15, 17))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 5)),
                    (1, Range::new(0, 5), Range::new(6, 9)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 1))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 7)),
                    (1, Range::new(0, 7), Range::new(10, 13)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects whitespace.",
                vec![
                    (1, Range::new(0, 8), Range::new(13, 15)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(17, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(32, 40)),
                ]),
            // TODO Consider whether this is desirable. Rather than silently failing,
            // it may be worth improving the API so it returns expressive results.
            ("Attempting to move from outside bounds fails without panic",
                vec![
                    (1, Range::new(9999, 9999), Range::new(9999, 9999)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 3)),
                    (1, Range::new(1, 3), Range::new(5, 7)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 5)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_previous_words() {
        let tests = array::IntoIter::new([
            ("Basic backward motion from the middle of a word",
                vec![(1, Range::new(3, 3), Range::new(3, 0))]),
            ("Starting from after boundary retreats the anchor",
                vec![(1, Range::new(0, 8), Range::new(7, 0))]),
            ("    Jump to start of a word preceded by whitespace",
                vec![(1, Range::new(5, 5), Range::new(5, 4))]),
            ("    Jump to start of line from start of word preceded by whitespace",
                vec![(1, Range::new(4, 4), Range::new(3, 0))]),
            ("Previous anchor is irrelevant for backward motions",
                vec![(1, Range::new(12, 5), Range::new(5, 0))]),
            ("    Starting from whitespace moves to first space in sequence",
                vec![(1, Range::new(0, 3), Range::new(3, 0))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 20), Range::new(20, 0))]),
            ("Jumping\n    \nback through a newline selects whitespace",
                vec![(1, Range::new(0, 13), Range::new(11, 8))]),
            ("Jumping to start of word from the end selects the word",
                vec![(1, Range::new(6, 6), Range::new(6, 0))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(30, 30), Range::new(30, 21)),
                    (1, Range::new(30, 21), Range::new(20, 18)),
                    (1, Range::new(20, 18), Range::new(17, 15))
                ]),

            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 10), Range::new(9, 6)),
                    (1, Range::new(9, 6), Range::new(5, 0)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 5), Range::new(4, 3))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 10), Range::new(7, 0)),
                ]),
            ("Jumping    \n\n\n\n\nback from within a newline group selects previous block",
                vec![
                    (1, Range::new(0, 13), Range::new(10, 0)),
                ]),
            ("Failed motions do not modify the range",
                vec![
                    (0, Range::new(3, 0), Range::new(3, 0)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(18, 18), Range::new(8, 0)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(40, 40), Range::new(9, 0)),
                ]),
            // TODO Consider whether this is desirable. Rather than silently failing,
            // it may be worth improving the API so it returns expressive results.
            ("Attempting to move from outside bounds fails without panic",
                vec![
                    (1, Range::new(9999, 9999), Range::new(9999, 9999)),
                ]),
            ("", // Edge case of moving backwards in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving backwards in all newlines
                vec![
                    (1, Range::new(0, 3), Range::new(0, 0)),
                ]),
            ("   \n   \nJumping back through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 7), Range::new(6, 4)),
                    (1, Range::new(6, 4), Range::new(2, 0)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 5), Range::new(4, 0)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_prev_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_end_of_next_words() {
        let tests = array::IntoIter::new([
            ("Basic forward motion from the start of a word to the end of it",
                vec![(1, Range::new(0, 0), Range::new(0, 4))]),
            ("Basic forward motion from the end of a word to the end of the next",
                vec![(1, Range::new(0, 4), Range::new(5, 12))]),
            ("Basic forward motion from the middle of a word to the end of it",
                vec![(1, Range::new(2, 2), Range::new(2, 4))]),
            ("    Jumping to end of a word preceded by whitespace",
                vec![(1, Range::new(0, 0), Range::new(0, 10))]),
            (" Starting from a boundary advances the anchor",
                vec![(1, Range::new(0, 0), Range::new(1, 8))]),
            ("Previous anchor is irrelevant for end of word motion",
                vec![(1, Range::new(12, 2), Range::new(2, 7))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 27))]),
            ("Jumping\n    into starting whitespace selects up to the end of next word",
                vec![(1, Range::new(0, 6), Range::new(8, 15))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 11)),
                    (1, Range::new(0, 11), Range::new(12, 14)),
                    (1, Range::new(12, 14), Range::new(15, 17))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 2)),
                    (1, Range::new(0, 2), Range::new(3, 8)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 1))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 7)),
                    (1, Range::new(0, 7), Range::new(10, 12)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects to end of next word.",
                vec![
                    (1, Range::new(0, 8), Range::new(13, 19)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(16, 18)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(31, 40)),
                ]),
            // TODO Consider whether this is desirable. Rather than silently failing,
            // it may be worth improving the API so it returns expressive results.
            ("Attempting to move from outside bounds fails without panic",
                vec![
                    (1, Range::new(9999, 9999), Range::new(9999, 9999)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 3)),
                    (1, Range::new(1, 3), Range::new(5, 7)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 4)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_word_end(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_categorize() {
        const WORD_TEST_CASE: &'static str =
            "_hello_world_あいうえおー1234567890１２３４５６７８９０";
        const PUNCTUATION_TEST_CASE: &'static str =
            "!\"#$%&\'()*+,-./:;<=>?@[\\]^`{|}~！”＃＄％＆’（）＊＋、。：；＜＝＞？＠「」＾｀｛｜｝～";
        const WHITESPACE_TEST_CASE: &'static str = "  　   ";

        assert_eq!(Category::Eol, categorize('\n'));

        for ch in WHITESPACE_TEST_CASE.chars() {
            assert_eq!(
                Category::Whitespace,
                categorize(ch),
                "Testing '{}', but got `{:?}` instead of `Category::Whitespace`",
                ch,
                categorize(ch)
            );
        }

        for ch in WORD_TEST_CASE.chars() {
            assert_eq!(
                Category::Word,
                categorize(ch),
                "Testing '{}', but got `{:?}` instead of `Category::Word`",
                ch,
                categorize(ch)
            );
        }

        for ch in PUNCTUATION_TEST_CASE.chars() {
            assert_eq!(
                Category::Punctuation,
                categorize(ch),
                "Testing '{}', but got `{:?}` instead of `Category::Punctuation`",
                ch,
                categorize(ch)
            );
        }
    }
}
