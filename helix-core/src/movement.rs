use std::iter;

use crate::{
    coords_at_pos,
    graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes},
    pos_at_coords, ChangeSet, Position, Range, Rope, RopeSlice, Selection,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SelectionBehaviour {
    Extend,
    Displace,
}

pub trait SliceIndexHelpers {
    fn outside(&self, text: RopeSlice) -> bool;
    fn inside(&self, text: RopeSlice) -> bool;
    /// The next character after this belongs
    /// to a different `Category`
    fn is_boundary(&self, text: RopeSlice) -> bool;
    fn is(&self, category: Category, text: RopeSlice) -> bool;
    fn category(&self, text: RopeSlice) -> Option<Category>;
    fn end_of_block(&self, text: RopeSlice) -> Self;
    fn skip_newlines(&self, text: RopeSlice) -> Self;
}

impl SliceIndexHelpers for usize {
    fn inside(&self, text: RopeSlice) -> bool {
        *self < text.len_chars()
    }

    fn outside(&self, text: RopeSlice) -> bool {
        !self.inside(text)
    }

    fn is_boundary(&self, text: RopeSlice) -> bool {
        (self + 1).inside(text)
            && (categorize(text.char(*self)) != categorize(text.char(self + 1)))
    }

    fn is(&self, category: Category, text: RopeSlice) -> bool {
        self.inside(text) && categorize(text.char(*self)) == category
    }

    fn category(&self, text: RopeSlice) -> Option<Category> {
        self.inside(text).then(||categorize(text.char(*self)))
    }

    // End of a word/punctuation group followed by any amount of whitespace.
    fn end_of_block(&self, slice: RopeSlice) -> Self {
        (*self..slice.len_chars().saturating_sub(1))
            .skip_while(|i| is_end_of_line(slice.char(*i)))
            .find(|pos| pos.is_boundary(slice)
                  && !(slice.char(*pos + 1).is_whitespace()
                       && !is_end_of_line(slice.char(*pos + 1)))
            )
            .unwrap_or(slice.len_chars().saturating_sub(1))
    }

    fn skip_newlines(&self, text: RopeSlice) -> Self {
        (*self..text.len_chars().saturating_sub(1))
            .find(|i| !is_end_of_line(text.char(*i)))
            .unwrap_or(text.len_chars().saturating_sub(1))
    }
}

pub fn move_horizontally(
    text: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    behaviour: SelectionBehaviour,
) -> Range {
    let pos = range.head;
    let line = text.char_to_line(pos);
    // TODO: we can optimize clamping by passing in RopeSlice limited to current line. that way
    // we stop calculating past start/end of line.
    let pos = match dir {
        Direction::Backward => {
            let start = text.line_to_char(line);
            nth_prev_grapheme_boundary(text, pos, count).max(start)
        }
        Direction::Forward => {
            // Line end is pos at the start of next line - 1
            let end = text.line_to_char(line + 1).saturating_sub(1);
            nth_next_grapheme_boundary(text, pos, count).min(end)
        }
    };
    let anchor = match behaviour {
        SelectionBehaviour::Extend => range.anchor,
        SelectionBehaviour::Displace => pos,
    };
    Range::new(anchor, pos)
}

pub fn move_vertically(
    text: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    behaviour: SelectionBehaviour,
) -> Range {
    let Position { row, col } = coords_at_pos(text, range.head);

    let horiz = range.horiz.unwrap_or(col as u32);

    let new_line = match dir {
        Direction::Backward => row.saturating_sub(count),
        Direction::Forward => std::cmp::min(
            row.saturating_add(count),
            text.len_lines().saturating_sub(2),
        ),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = text.line(new_line).len_chars().saturating_sub(2);

    let new_col = std::cmp::min(horiz as usize, new_line_len);

    let pos = pos_at_coords(text, Position::new(new_line, new_col));

    let anchor = match behaviour {
        SelectionBehaviour::Extend => range.anchor,
        SelectionBehaviour::Displace => pos,
    };

    let mut range = Range::new(anchor, pos);
    range.horiz = Some(horiz);
    range
}

pub fn move_next_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    let movement = |range: Range| -> Option<Range> {
        let after_head = (range.head + 1).skip_newlines(slice);
        (after_head + 1).inside(slice).then(|| {
            let boundary = range.head.is_boundary(slice);
            let new_anchor = if boundary { after_head } else { range.head };
            let new_head = (range.head + 1).end_of_block(slice);
            Some(Range::new(new_anchor, new_head))
        })?
    };
    (0..count).fold(range, |range, _| movement(range).unwrap_or(range))
}

pub fn move_prev_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    let movement = |mut range: Range| -> Option<Range> {
        range.anchor = if range.head.saturating_sub(1).is_boundary(slice) {
            range.head.saturating_sub(1)
        } else {
            range.head
        };

        range.anchor = backwards_skip_while(slice, range.anchor, is_end_of_line).unwrap_or(0);
        let category = range.anchor.category(slice)?;
        range.head = backwards_skip_while(slice, range.anchor, |c| categorize(c) == category)
            .map(|h| h + 1)
            .unwrap_or(0);
        Some(range)
    };
    (0..count).fold(range, |range, _| movement(range).unwrap_or(range))
}

pub fn move_next_word_end(slice: RopeSlice, mut begin: usize, count: usize) -> Option<Range> {
    let mut end = begin;

    for _ in 0..count {
        if begin + 2 >= slice.len_chars() {
            return None;
        }

        let ch = slice.char(begin);
        let next = slice.char(begin + 1);

        if categorize(ch) != categorize(next) {
            begin += 1;
        }

        begin = skip_while(slice, begin, |ch| ch == '\n')?;

        end = begin;

        end = skip_while(slice, end, char::is_whitespace)?;

        // refetch
        let ch = slice.char(end);

        if is_word(ch) {
            end = skip_while(slice, end, is_word)?;
        } else if is_punctuation(ch) {
            end = skip_while(slice, end, is_punctuation)?;
        }
    }

    Some(Range::new(begin, end - 1))
}

// ---- util ------------

// used for by-word movement

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
                move_vertically(
                    slice,
                    range,
                    Direction::Forward,
                    1,
                    SelectionBehaviour::Displace
                )
                .head
            ),
            (1, 2).into()
        );
    }

    #[test]
    fn horizontal_moves_through_single_line_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::single(position);

        let moves_and_expected_coordinates = [
            ((Direction::Forward, 1usize), (0, 1)),
            ((Direction::Forward, 2usize), (0, 3)),
            ((Direction::Forward, 0usize), (0, 3)),
            ((Direction::Forward, 999usize), (0, 31)),
            ((Direction::Forward, 999usize), (0, 31)),
            ((Direction::Backward, 999usize), (0, 0)),
        ];

        for ((direction, amount), coordinates) in IntoIter::new(moves_and_expected_coordinates) {
            range = move_horizontally(
                slice,
                range,
                direction,
                amount,
                SelectionBehaviour::Displace,
            );
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into())
        }
    }

    #[test]
    fn horizontal_moves_through_single_line_in_multiline_text() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::single(position);

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
            range = move_horizontally(
                slice,
                range,
                direction,
                amount,
                SelectionBehaviour::Displace,
            );
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn selection_extending_moves_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::single(position);
        let original_anchor = range.anchor;

        let moves = IntoIter::new([
            (Direction::Forward, 1usize),
            (Direction::Forward, 5usize),
            (Direction::Backward, 3usize),
        ]);

        for (direction, amount) in moves {
            range = move_horizontally(slice, range, direction, amount, SelectionBehaviour::Extend);
            assert_eq!(range.anchor, original_anchor);
        }
    }

    #[test]
    fn vertical_moves_in_single_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = dbg!(&text).slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::single(position);
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
            range = move_vertically(
                slice,
                range,
                direction,
                amount,
                SelectionBehaviour::Displace,
            );
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn vertical_moves_jumping_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::single(position);

        enum Axis {
            H,
            V,
        };
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
                Axis::H => move_horizontally(
                    slice,
                    range,
                    direction,
                    amount,
                    SelectionBehaviour::Displace,
                ),
                Axis::V => move_vertically(
                    slice,
                    range,
                    direction,
                    amount,
                    SelectionBehaviour::Displace,
                ),
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
        let mut range = Range::single(position);

        // FIXME: The behaviour captured in this test diverges from both Kakoune and Vim. These
        // will attempt to preserve the horizontal position of the cursor, rather than
        // placing it at the same character index.
        enum Axis {
            H,
            V,
        };
        let moves_and_expected_coordinates = IntoIter::new([
            // Places cursor at the fourth kana
            ((Axis::H, Direction::Forward, 4), (0, 4)),
            // Descent places cursor at the fourth character.
            ((Axis::V, Direction::Forward, 1usize), (1, 4)),
        ]);

        for ((axis, direction, amount), coordinates) in moves_and_expected_coordinates {
            range = match axis {
                Axis::H => move_horizontally(
                    slice,
                    range,
                    direction,
                    amount,
                    SelectionBehaviour::Displace,
                ),
                Axis::V => move_vertically(
                    slice,
                    range,
                    direction,
                    amount,
                    SelectionBehaviour::Displace,
                ),
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
                vec![(Motion::NextStart(1), Range::new(0, 0), Range::new(0, 5))]),
            ("Long       whitespace gap is bridged by the head",
                vec![(Motion::NextStart(1), Range::new(0, 0), Range::new(0, 10))]),
            ("Previous anchor is irrelevant for forward motions",
                vec![(Motion::NextStart(1), Range::new(12, 0), Range::new(0, 8))]),
            ("    Starting from whitespace moves to last space in sequence",
                vec![(Motion::NextStart(1), Range::new(0, 0), Range::new(0, 3))]),
            ("Starting from mid-word leaves anchor at start position and moves head",
                vec![(Motion::NextStart(1), Range::new(3, 3), Range::new(3, 8))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(Motion::NextStart(1), Range::new(0, 0), Range::new(0, 28))]),
            ("Jumping\n    into starting whitespace selects the spaces before 'into'",
                vec![(Motion::NextStart(1), Range::new(0, 6), Range::new(8, 11))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 11)),
                    (Motion::NextStart(1), Range::new(0, 11), Range::new(12, 14)),
                    (Motion::NextStart(1), Range::new(12, 14), Range::new(15, 17))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 5)),
                    (Motion::NextStart(1), Range::new(0, 5), Range::new(6, 9)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(Motion::NextStart(1), Range::new(0, 0), Range::new(0, 1))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 7)),
                    (Motion::NextStart(1), Range::new(0, 7), Range::new(10, 13)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (Motion::NextStart(3), Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (Motion::NextStart(3), Range::new(0, 0), Range::new(17, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (Motion::NextStart(999), Range::new(0, 0), Range::new(32, 40)),
                ]),
            // TODO Consider whether this is desirable. Rather than silently failing,
            // it may be worth improving the API so it returns expressive results.
            ("Attempting to move from outside bounds fails without panic",
                vec![
                    (Motion::NextStart(1), Range::new(9999, 9999), Range::new(9999, 9999)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(1, 3)),
                    (Motion::NextStart(1), Range::new(1, 3), Range::new(5, 7)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (Motion::NextStart(1), Range::new(0, 0), Range::new(0, 5)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (motion, begin, expected_end) in scenario.into_iter() {
                let range = match motion {
                    Motion::NextStart(count) => {
                        move_next_word_start(Rope::from(sample).slice(..), begin, count)
                    }
                    Motion::NextEnd(count) => todo!(), //move_next_word_end(Rope::from(sample).slice(..), begin, count),
                    Motion::PrevStart(count) => {
                        move_prev_word_start(Rope::from(sample).slice(..), begin, count)
                    }
                };
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_previous_words() {
        let tests = array::IntoIter::new([
            ("Basic backward motion from the middle of a word",
                vec![(Motion::PrevStart(1), Range::new(3, 3), Range::new(3, 0))]),
            ("    Jump to start of a word preceded by whitespace",
                vec![(Motion::PrevStart(1), Range::new(5, 5), Range::new(5, 4))]),
            ("    Jump to start of line from start of word preceded by whitespace",
                vec![(Motion::PrevStart(1), Range::new(4, 4), Range::new(3, 0))]),
            ("Previous anchor is irrelevant for backward motions",
                vec![(Motion::PrevStart(1), Range::new(12, 5), Range::new(5, 0))]),
            ("    Starting from whitespace moves to first space in sequence",
                vec![(Motion::PrevStart(1), Range::new(0, 3), Range::new(3, 0))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(Motion::PrevStart(1), Range::new(0, 20), Range::new(20, 0))]),
            ("Jumping\n    \nback through a newline selects whitespace",
                vec![(Motion::PrevStart(1), Range::new(0, 13), Range::new(11, 8))]),
            ("Jumping to start of word from the end selects the word",
                vec![(Motion::PrevStart(1), Range::new(6, 6), Range::new(6, 0))]),

            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (Motion::PrevStart(1), Range::new(30, 30), Range::new(30, 21)),
                    (Motion::PrevStart(1), Range::new(30, 21), Range::new(20, 18)),
                    (Motion::PrevStart(1), Range::new(20, 18), Range::new(17, 15))
                ]),
        ]);

        for (sample, scenario) in tests {
            for (motion, begin, expected_end) in scenario.into_iter() {
                let range = match motion {
                    Motion::NextStart(count) => {
                        move_next_word_start(Rope::from(sample).slice(..), begin, count)
                    }
                    Motion::NextEnd(count) => todo!(), //move_next_word_end(Rope::from(sample).slice(..), begin, count),
                    Motion::PrevStart(count) => {
                        move_prev_word_start(Rope::from(sample).slice(..), begin, count)
                    }
                };
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_categorize() {
        const WORD_TEST_CASE: &'static str =
            "_hello_world_あいうえおー1234567890１２３４５６７８９０";
        const PUNCTUATION_TEST_CASE: &'static str = "!\"#$%&\'()*+,-./:;<=>?@[\\]^`{|}~！”＃＄％＆’（）＊＋、。：；＜＝＞？＠「」＾｀｛｜｝～";
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
