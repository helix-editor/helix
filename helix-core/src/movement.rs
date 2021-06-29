use std::iter::{self, from_fn, Peekable, SkipWhile};

use ropey::iter::Chars;

use crate::{
    chars::{
        categorize_char, char_is_line_ending, char_is_punctuation, char_is_whitespace,
        char_is_word, CharCategory,
    },
    coords_at_pos,
    graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary},
    line_ending::{get_line_ending, line_end_char_index},
    pos_at_coords, Position, Range, RopeSlice,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
            let end_char_idx = line_end_char_index(&slice, line);
            nth_next_grapheme_boundary(slice, pos, count).min(end_char_idx)
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

    // Length of the line sans line-ending.
    let new_line_len = {
        let line = slice.line(new_line);
        line.len_chars() - get_line_ending(&line).map(|le| le.len_chars()).unwrap_or(0)
    };

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
    word_move(slice, range, count, WordMotionTarget::NextWordStart)
}

pub fn move_next_word_end(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::NextWordEnd)
}

pub fn move_prev_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::PrevWordStart)
}

pub fn move_next_long_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::NextLongWordStart)
}

pub fn move_next_long_word_end(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::NextLongWordEnd)
}

pub fn move_prev_long_word_start(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::PrevLongWordStart)
}

fn word_move(slice: RopeSlice, mut range: Range, count: usize, target: WordMotionTarget) -> Range {
    (0..count).fold(range, |range, _| {
        slice.chars_at(range.head).range_to_target(target, range)
    })
}

// ---- util ------------

#[inline]
/// Returns first index that doesn't satisfy a given predicate when
/// advancing the character index.
///
/// Returns none if all characters satisfy the predicate.
pub fn skip_while<F>(slice: RopeSlice, pos: usize, fun: F) -> Option<usize>
where
    F: Fn(char) -> bool,
{
    let mut chars = slice.chars_at(pos).enumerate();
    chars.find_map(|(i, c)| if !fun(c) { Some(pos + i) } else { None })
}

#[inline]
/// Returns first index that doesn't satisfy a given predicate when
/// retreating the character index, saturating if all elements satisfy
/// the condition.
pub fn backwards_skip_while<F>(slice: RopeSlice, pos: usize, fun: F) -> Option<usize>
where
    F: Fn(char) -> bool,
{
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

/// Possible targets of a word motion
#[derive(Copy, Clone, Debug)]
pub enum WordMotionTarget {
    NextWordStart,
    NextWordEnd,
    PrevWordStart,
    // A "Long word" (also known as a WORD in vim/kakoune) is strictly
    // delimited by whitespace, and can consist of punctuation as well
    // as alphanumerics.
    NextLongWordStart,
    NextLongWordEnd,
    PrevLongWordStart,
}

pub trait CharHelpers {
    fn range_to_target(&mut self, target: WordMotionTarget, origin: Range) -> Range;
}

enum WordMotionPhase {
    Start,
    SkipNewlines,
    ReachTarget,
}

impl CharHelpers for Chars<'_> {
    fn range_to_target(&mut self, target: WordMotionTarget, origin: Range) -> Range {
        let range = origin;
        // Characters are iterated forward or backwards depending on the motion direction.
        let characters: Box<dyn Iterator<Item = char>> = match target {
            WordMotionTarget::PrevWordStart | WordMotionTarget::PrevLongWordStart => {
                self.next();
                Box::new(from_fn(|| self.prev()))
            }
            _ => Box::new(self),
        };

        // Index advancement also depends on the direction.
        let advance: &dyn Fn(&mut usize) = match target {
            WordMotionTarget::PrevWordStart | WordMotionTarget::PrevLongWordStart => &|u| *u = u.saturating_sub(1),
            _ => &|u| *u += 1,
        };

        let mut characters = characters.peekable();
        let mut phase = WordMotionPhase::Start;
        let mut head = origin.head;
        let mut anchor: Option<usize> = None;
        let is_boundary =
            |a: char, b: Option<char>| categorize_char(a) != categorize_char(b.unwrap_or(a));
        while let Some(peek) = characters.peek().copied() {
            phase = match phase {
                WordMotionPhase::Start => {
                    characters.next();
                    if characters.peek().is_none() {
                        break; // We're at the end, so there's nothing to do.
                    }
                    // Anchor may remain here if the head wasn't at a boundary
                    if !is_boundary(peek, characters.peek().copied()) && !char_is_line_ending(peek)
                    {
                        anchor = Some(head);
                    }
                    // First character is always skipped by the head
                    advance(&mut head);
                    WordMotionPhase::SkipNewlines
                }
                WordMotionPhase::SkipNewlines => {
                    if char_is_line_ending(peek) {
                        characters.next();
                        if characters.peek().is_some() {
                            advance(&mut head);
                        }
                        WordMotionPhase::SkipNewlines
                    } else {
                        WordMotionPhase::ReachTarget
                    }
                }
                WordMotionPhase::ReachTarget => {
                    characters.next();
                    anchor = anchor.or(Some(head));
                    if reached_target(target, peek, characters.peek()) {
                        break;
                    } else {
                        advance(&mut head);
                    }
                    WordMotionPhase::ReachTarget
                }
            }
        }
        Range::new(anchor.unwrap_or(origin.anchor), head)
    }
}

fn is_word_boundary(a: char, b: char) -> bool {
    categorize_char(a) != categorize_char(b)
}

fn is_long_word_boundary(a: char, b: char) -> bool {
    match (categorize_char(a), categorize_char(b)) {
        (CharCategory::Word, CharCategory::Punctuation)
            | (CharCategory::Punctuation, CharCategory::Word) => false,
        (a, b) if a != b => true,
        _ => false
    }
}

fn reached_target(target: WordMotionTarget, peek: char, next_peek: Option<&char>) -> bool {
    let next_peek = match next_peek {
        Some(next_peek) => next_peek,
        None => return true,
    };

    match target {
        WordMotionTarget::NextWordStart => {
            (is_word_boundary(peek, *next_peek)
                && (char_is_line_ending(*next_peek) || !next_peek.is_whitespace()))
        }
        WordMotionTarget::NextWordEnd | WordMotionTarget::PrevWordStart => {
            (is_word_boundary(peek, *next_peek)
                && (!peek.is_whitespace() || char_is_line_ending(*next_peek)))
        }
        WordMotionTarget::NextLongWordStart => {
            (is_long_word_boundary(peek, *next_peek)
                && (char_is_line_ending(*next_peek) || !next_peek.is_whitespace()))
        }
        WordMotionTarget::NextLongWordEnd | WordMotionTarget::PrevLongWordStart => {
            (is_long_word_boundary(peek, *next_peek)
                && (!peek.is_whitespace() || char_is_line_ending(*next_peek)))
        }
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
            (1, 3).into()
        );
    }

    #[test]
    fn horizontal_moves_through_single_line_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into());

        let mut range = Range::point(position);

        let moves_and_expected_coordinates = [
            ((Direction::Forward, 1usize), (0, 1)), // T|his is a simple alphabetic line
            ((Direction::Forward, 2usize), (0, 3)), // Thi|s is a simple alphabetic line
            ((Direction::Forward, 0usize), (0, 3)), // Thi|s is a simple alphabetic line
            ((Direction::Forward, 999usize), (0, 32)), // This is a simple alphabetic line|
            ((Direction::Forward, 999usize), (0, 32)), // This is a simple alphabetic line|
            ((Direction::Backward, 999usize), (0, 0)), // |This is a simple alphabetic line
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
            ((Direction::Forward, 1usize), (0, 1)),    // M|ultiline\n
            ((Direction::Forward, 2usize), (0, 3)),    // Mul|tiline\n
            ((Direction::Backward, 6usize), (0, 0)),   // |Multiline\n
            ((Direction::Backward, 999usize), (0, 0)), // |Multiline\n
            ((Direction::Forward, 3usize), (0, 3)),    // Mul|tiline\n
            ((Direction::Forward, 0usize), (0, 3)),    // Mul|tiline\n
            ((Direction::Backward, 0usize), (0, 3)),   // Mul|tiline\n
            ((Direction::Forward, 999usize), (0, 9)),  // Multiline|\n
            ((Direction::Forward, 999usize), (0, 9)),  // Multiline|\n
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
            ((Axis::V, Direction::Forward, 1usize), (2, 5)),
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

    #[test]
    #[should_panic]
    fn nonsensical_ranges_panic_on_forward_movement_attempt_in_debug_mode() {
        move_next_word_start(Rope::from("Sample").slice(..), Range::point(99999999), 1);
    }

    #[test]
    #[should_panic]
    fn nonsensical_ranges_panic_on_forward_to_end_movement_attempt_in_debug_mode() {
        move_next_word_end(Rope::from("Sample").slice(..), Range::point(99999999), 1);
    }

    #[test]
    #[should_panic]
    fn nonsensical_ranges_panic_on_backwards_movement_attempt_in_debug_mode() {
        move_prev_word_start(Rope::from("Sample").slice(..), Range::point(99999999), 1);
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
            ("oh oh oh two character words!",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 2)),
                    (1, Range::new(0, 2), Range::new(3, 5)),
                    (1, Range::new(0, 1), Range::new(2, 2)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(17, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(32, 40)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 4)),
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
    fn test_behaviour_when_moving_to_start_of_next_long_words() {
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
            ("alphanumeric.!,and.?=punctuation are not treated any differently than alphanumerics",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 32)),
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 5)),
                    (1, Range::new(0, 5), Range::new(6, 9)),
                ]),
            (".._.._ punctuation is joined by underscores into a single word, as it behaves like alphanumerics",
                vec![(1, Range::new(0, 0), Range::new(0, 6))]),
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
            ("oh oh oh two character words!",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 2)),
                    (1, Range::new(0, 2), Range::new(3, 5)),
                    (1, Range::new(0, 1), Range::new(2, 2)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(17, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(32, 40)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 4)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 3)),
                    (1, Range::new(1, 3), Range::new(5, 7)),
                ]),
            ("ヒー..リクス multibyte characters behave as normal characters, including their interaction with punctuation",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 7)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_long_word_start(Rope::from(sample).slice(..), begin, count);
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
            ("", // Edge case of moving backwards in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving backwards in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
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
    fn test_behaviour_when_moving_to_start_of_previous_long_words() {
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
            ("alphanumeric.!,and.?=punctuation are treated exactly the same",
                vec![
                    (1, Range::new(30, 30), Range::new(30, 0)),
                ]),

            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 10), Range::new(9, 6)),
                    (1, Range::new(9, 6), Range::new(5, 0)),
                ]),
            (".._.._ punctuation is joined by underscores into a single block",
                vec![(1, Range::new(0, 5), Range::new(4, 0))]),
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
            ("", // Edge case of moving backwards in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving backwards in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("   \n   \nJumping back through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 7), Range::new(6, 4)),
                    (1, Range::new(6, 4), Range::new(2, 0)),
                ]),
            ("ヒーリ..クス multibyte characters behave as normal characters, including when interacting with punctuation",
                vec![
                    (1, Range::new(0, 7), Range::new(6, 0)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_prev_long_word_start(Rope::from(sample).slice(..), begin, count);
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
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 4)),
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
    fn test_behaviour_when_moving_to_end_of_next_long_words() {
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
            ("alphanumeric.!,and.?=punctuation are treated the same way",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 31)),
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 2)),
                    (1, Range::new(0, 2), Range::new(3, 8)),
                ]),
            (".._.._ punctuation is joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 5))]),
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
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(0, 4)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 3)),
                    (1, Range::new(1, 3), Range::new(5, 7)),
                ]),
            ("ヒーリ..クス multibyte characters behave as normal characters, including  when they interact with punctuation",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 6)),
                ]),
        ]);

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_long_word_end(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }
}
