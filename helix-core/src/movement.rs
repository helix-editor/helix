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
        Direction::Forward => {
            std::cmp::min(row.saturating_add(count), text.len_lines().saturating_sub(2))
        }
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

pub fn move_next_word_start(slice: RopeSlice, mut begin: usize, count: usize) -> Option<Range> {
    let mut end = begin;

    for _ in 0..count {
        if begin + 1 == slice.len_chars() {
            return None;
        }

        let mut ch = slice.char(begin);
        let next = slice.char(begin + 1);

        // if we're at the end of a word, or on whitespce right before new one
        if categorize(ch) != categorize(next) {
            begin += 1;
        }

        if !skip_over_next(slice, &mut begin, |ch| ch == '\n') {
            return None;
        };
        ch = slice.char(begin);

        end = begin + 1;

        if is_word(ch) {
            skip_over_next(slice, &mut end, is_word);
        } else if is_punctuation(ch) {
            skip_over_next(slice, &mut end, is_punctuation);
        }

        skip_over_next(slice, &mut end, char::is_whitespace);
    }

    Some(Range::new(begin, end - 1))
}

pub fn move_prev_word_start(slice: RopeSlice, mut begin: usize, count: usize) -> Option<Range> {
    let mut with_end = false;
    let mut end = begin;

    for _ in 0..count {
        if begin == 0 {
            return None;
        }

        let ch = slice.char(begin);
        let prev = slice.char(begin - 1);

        if categorize(ch) != categorize(prev) {
            begin -= 1;
        }

        // return if not skip while?
        skip_over_prev(slice, &mut begin, |ch| ch == '\n');

        end = begin;

        with_end = skip_over_prev(slice, &mut end, char::is_whitespace);

        // refetch
        let ch = slice.char(end);

        if is_word(ch) {
            with_end = skip_over_prev(slice, &mut end, is_word);
        } else if is_punctuation(ch) {
            with_end = skip_over_prev(slice, &mut end, is_punctuation);
        }
    }

    Some(Range::new(begin, if with_end { end } else { end + 1 }))
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

        if !skip_over_next(slice, &mut begin, |ch| ch == '\n') {
            return None;
        };

        end = begin;

        skip_over_next(slice, &mut end, char::is_whitespace);

        // refetch
        let ch = slice.char(end);

        if is_word(ch) {
            skip_over_next(slice, &mut end, is_word);
        } else if is_punctuation(ch) {
            skip_over_next(slice, &mut end, is_punctuation);
        }
    }

    Some(Range::new(begin, end - 1))
}

// ---- util ------------

// used for by-word movement

#[inline]
pub(crate) fn is_word(ch: char) -> bool { ch.is_alphanumeric() || ch == '_' }

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
pub(crate) enum Category {
    Whitespace,
    Eol,
    Word,
    Punctuation,
    Unknown,
}

#[inline]
pub(crate) fn categorize(ch: char) -> Category {
    if ch == '\n' {
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
/// Returns true if there are more characters left after the new position.
pub fn skip_over_next<F>(slice: RopeSlice, pos: &mut usize, fun: F) -> bool
where
    F: Fn(char) -> bool,
{
    let mut chars = slice.chars_at(*pos);

    #[allow(clippy::while_let_on_iterator)]
    while let Some(ch) = chars.next() {
        if !fun(ch) {
            break;
        }
        *pos += 1;
    }
    chars.next().is_some()
}

#[inline]
/// Returns true if the final pos matches the predicate.
pub fn skip_over_prev<F>(slice: RopeSlice, pos: &mut usize, fun: F) -> bool
where
    F: Fn(char) -> bool,
{
    // need to +1 so that prev() includes current char
    let mut chars = slice.chars_at(*pos + 1);

    #[allow(clippy::while_let_on_iterator)]
    while let Some(ch) = chars.prev() {
        if !fun(ch) {
            break;
        }
        *pos = pos.saturating_sub(1);
    }
    fun(slice.char(*pos))
}

#[cfg(test)]
mod test {
    use std::array::IntoIter;

    use super::*;

    const SINGLE_LINE_SAMPLE: &str = "This is a simple alphabetic line";
    const MULTILINE_SAMPLE: &str = "\
        Multiline\n\
        text sample\n\
        which\n\
        is merely alphabetic\n\
        and whitespaced\n\
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
                move_vertically(slice, range, Direction::Forward, 1, SelectionBehaviour::Displace)
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
            range =
                move_horizontally(slice, range, direction, amount, SelectionBehaviour::Displace);
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
            range =
                move_horizontally(slice, range, direction, amount, SelectionBehaviour::Displace);
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
            range = move_vertically(slice, range, direction, amount, SelectionBehaviour::Displace);
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
                Axis::H => {
                    move_horizontally(slice, range, direction, amount, SelectionBehaviour::Displace)
                }
                Axis::V => {
                    move_vertically(slice, range, direction, amount, SelectionBehaviour::Displace)
                }
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
                Axis::H => {
                    move_horizontally(slice, range, direction, amount, SelectionBehaviour::Displace)
                }
                Axis::V => {
                    move_vertically(slice, range, direction, amount, SelectionBehaviour::Displace)
                }
            };
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn word_moves_through_multiline_text() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());
        let mut range = Range::single(position);

        enum Move { NextStart, NextEnd, PrevStart }

        let moves_and_expected_coordinates = IntoIter::new([
            ((Move::NextStart, 1), (0, 9)), // Multilin_
            ((Move::NextStart, 1), (1, 4)), // text_sample
            ((Move::NextStart, 1), (1, 11)), // text sampl_
            ((Move::PrevStart, 1), (1, 5)), // text _ample
            ((Move::PrevStart, 1), (1, 0)), // _ext sample
            ((Move::NextEnd, 1), (1, 3)), // tex_ sample
            ((Move::NextEnd, 1), (1, 10)), // text sampl_
            // FIXME
            ((Move::PrevStart, 2), (1, 0)), // _ext sample
            ((Move::NextStart, 3), (1, 0)), // _ext sample
            ((Move::NextStart, 3), (2, 0)), // _hich
        ]);

        for ((direction, count), coordinates) in moves_and_expected_coordinates {
            range = match direction {
                Move::NextStart => move_next_word_start(slice, range.head, count).unwrap(),
                Move::NextEnd => move_next_word_end(slice, range.head, count).unwrap(),
                Move::PrevStart => move_prev_word_start(slice, range.head, count).unwrap(),
            };
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
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
