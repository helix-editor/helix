use std::iter;

use ropey::iter::Chars;
use tree_sitter::{Node, QueryCursor};

use crate::{
    chars::{categorize_char, char_is_line_ending, CharCategory},
    graphemes::{
        next_grapheme_boundary, nth_next_grapheme_boundary, nth_prev_grapheme_boundary,
        prev_grapheme_boundary,
    },
    line_ending::rope_is_line_ending,
    pos_at_visual_coords,
    syntax::LanguageConfiguration,
    textobject::TextObject,
    visual_coords_at_pos, Position, Range, RopeSlice,
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
    _: usize,
) -> Range {
    let pos = range.cursor(slice);

    // Compute the new position.
    let new_pos = match dir {
        Direction::Forward => nth_next_grapheme_boundary(slice, pos, count),
        Direction::Backward => nth_prev_grapheme_boundary(slice, pos, count),
    };

    // Compute the final new range.
    range.put_cursor(slice, new_pos, behaviour == Movement::Extend)
}

pub fn move_vertically(
    slice: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    behaviour: Movement,
    tab_width: usize,
) -> Range {
    let pos = range.cursor(slice);

    // Compute the current position's 2d coordinates.
    let Position { row, col } = visual_coords_at_pos(slice, pos, tab_width);
    let horiz = range.horiz.unwrap_or(col as u32);

    // Compute the new position.
    let new_row = match dir {
        Direction::Forward => (row + count).min(slice.len_lines().saturating_sub(1)),
        Direction::Backward => row.saturating_sub(count),
    };
    let new_col = col.max(horiz as usize);
    let new_pos = pos_at_visual_coords(slice, Position::new(new_row, new_col), tab_width);

    // Special-case to avoid moving to the end of the last non-empty line.
    if behaviour == Movement::Extend && slice.line(new_row).len_chars() == 0 {
        return range;
    }

    let mut new_range = range.put_cursor(slice, new_pos, behaviour == Movement::Extend);
    new_range.horiz = Some(horiz);
    new_range
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

pub fn move_prev_word_end(slice: RopeSlice, range: Range, count: usize) -> Range {
    word_move(slice, range, count, WordMotionTarget::PrevWordEnd)
}

fn word_move(slice: RopeSlice, range: Range, count: usize, target: WordMotionTarget) -> Range {
    let is_prev = matches!(
        target,
        WordMotionTarget::PrevWordStart
            | WordMotionTarget::PrevLongWordStart
            | WordMotionTarget::PrevWordEnd
    );

    // Special-case early-out.
    if (is_prev && range.head == 0) || (!is_prev && range.head == slice.len_chars()) {
        return range;
    }

    // Prepare the range appropriately based on the target movement
    // direction.  This is addressing two things at once:
    //
    //   1. Block-cursor semantics.
    //   2. The anchor position being irrelevant to the output result.
    #[allow(clippy::collapsible_else_if)] // Makes the structure clearer in this case.
    let start_range = if is_prev {
        if range.anchor < range.head {
            Range::new(range.head, prev_grapheme_boundary(slice, range.head))
        } else {
            Range::new(next_grapheme_boundary(slice, range.head), range.head)
        }
    } else {
        if range.anchor < range.head {
            Range::new(prev_grapheme_boundary(slice, range.head), range.head)
        } else {
            Range::new(range.head, next_grapheme_boundary(slice, range.head))
        }
    };

    // Do the main work.
    (0..count).fold(start_range, |r, _| {
        slice.chars_at(r.head).range_to_target(target, r)
    })
}

pub fn move_prev_paragraph(
    slice: RopeSlice,
    range: Range,
    count: usize,
    behavior: Movement,
) -> Range {
    let mut line = range.cursor_line(slice);
    let first_char = slice.line_to_char(line) == range.cursor(slice);
    let prev_line_empty = rope_is_line_ending(slice.line(line.saturating_sub(1)));
    let curr_line_empty = rope_is_line_ending(slice.line(line));
    let prev_empty_to_line = prev_line_empty && !curr_line_empty;

    // skip character before paragraph boundary
    if prev_empty_to_line && !first_char {
        line += 1;
    }
    let mut lines = slice.lines_at(line);
    lines.reverse();
    let mut lines = lines.map(rope_is_line_ending).peekable();
    for _ in 0..count {
        while lines.next_if(|&e| e).is_some() {
            line -= 1;
        }
        while lines.next_if(|&e| !e).is_some() {
            line -= 1;
        }
    }

    let head = slice.line_to_char(line);
    let anchor = if behavior == Movement::Move {
        // exclude first character after paragraph boundary
        if prev_empty_to_line && first_char {
            range.cursor(slice)
        } else {
            range.head
        }
    } else {
        range.put_cursor(slice, head, true).anchor
    };
    Range::new(anchor, head)
}

pub fn move_next_paragraph(
    slice: RopeSlice,
    range: Range,
    count: usize,
    behavior: Movement,
) -> Range {
    let mut line = range.cursor_line(slice);
    let last_char =
        prev_grapheme_boundary(slice, slice.line_to_char(line + 1)) == range.cursor(slice);
    let curr_line_empty = rope_is_line_ending(slice.line(line));
    let next_line_empty =
        rope_is_line_ending(slice.line(slice.len_lines().saturating_sub(1).min(line + 1)));
    let curr_empty_to_line = curr_line_empty && !next_line_empty;

    // skip character after paragraph boundary
    if curr_empty_to_line && last_char {
        line += 1;
    }
    let mut lines = slice.lines_at(line).map(rope_is_line_ending).peekable();
    for _ in 0..count {
        while lines.next_if(|&e| !e).is_some() {
            line += 1;
        }
        while lines.next_if(|&e| e).is_some() {
            line += 1;
        }
    }
    let head = slice.line_to_char(line);
    let anchor = if behavior == Movement::Move {
        if curr_empty_to_line && last_char {
            range.head
        } else {
            range.cursor(slice)
        }
    } else {
        range.put_cursor(slice, head, true).anchor
    };
    Range::new(anchor, head)
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
    let mut chars_starting_from_next = slice.chars_at(pos);
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
    PrevWordEnd,
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

impl CharHelpers for Chars<'_> {
    /// Note: this only changes the anchor of the range if the head is effectively
    /// starting on a boundary (either directly or after skipping newline characters).
    /// Any other changes to the anchor should be handled by the calling code.
    fn range_to_target(&mut self, target: WordMotionTarget, origin: Range) -> Range {
        let is_prev = matches!(
            target,
            WordMotionTarget::PrevWordStart
                | WordMotionTarget::PrevLongWordStart
                | WordMotionTarget::PrevWordEnd
        );

        // Reverse the iterator if needed for the motion direction.
        if is_prev {
            self.reverse();
        }

        // Function to advance index in the appropriate motion direction.
        let advance: &dyn Fn(&mut usize) = if is_prev {
            &|idx| *idx = idx.saturating_sub(1)
        } else {
            &|idx| *idx += 1
        };

        // Initialize state variables.
        let mut anchor = origin.anchor;
        let mut head = origin.head;
        let mut prev_ch = {
            let ch = self.prev();
            if ch.is_some() {
                self.next();
            }
            ch
        };

        // Skip any initial newline characters.
        while let Some(ch) = self.next() {
            if char_is_line_ending(ch) {
                prev_ch = Some(ch);
                advance(&mut head);
            } else {
                self.prev();
                break;
            }
        }
        if prev_ch.map(char_is_line_ending).unwrap_or(false) {
            anchor = head;
        }

        // Find our target position(s).
        let head_start = head;
        #[allow(clippy::while_let_on_iterator)] // Clippy's suggestion to fix doesn't work here.
        while let Some(next_ch) = self.next() {
            if prev_ch.is_none() || reached_target(target, prev_ch.unwrap(), next_ch) {
                if head == head_start {
                    anchor = head;
                } else {
                    break;
                }
            }
            prev_ch = Some(next_ch);
            advance(&mut head);
        }

        // Un-reverse the iterator if needed.
        if is_prev {
            self.reverse();
        }

        Range::new(anchor, head)
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
        _ => false,
    }
}

fn reached_target(target: WordMotionTarget, prev_ch: char, next_ch: char) -> bool {
    match target {
        WordMotionTarget::NextWordStart | WordMotionTarget::PrevWordEnd => {
            is_word_boundary(prev_ch, next_ch)
                && (char_is_line_ending(next_ch) || !next_ch.is_whitespace())
        }
        WordMotionTarget::NextWordEnd | WordMotionTarget::PrevWordStart => {
            is_word_boundary(prev_ch, next_ch)
                && (!prev_ch.is_whitespace() || char_is_line_ending(next_ch))
        }
        WordMotionTarget::NextLongWordStart => {
            is_long_word_boundary(prev_ch, next_ch)
                && (char_is_line_ending(next_ch) || !next_ch.is_whitespace())
        }
        WordMotionTarget::NextLongWordEnd | WordMotionTarget::PrevLongWordStart => {
            is_long_word_boundary(prev_ch, next_ch)
                && (!prev_ch.is_whitespace() || char_is_line_ending(next_ch))
        }
    }
}

pub fn goto_treesitter_object(
    slice: RopeSlice,
    range: Range,
    object_name: &str,
    dir: Direction,
    slice_tree: Node,
    lang_config: &LanguageConfiguration,
    count: usize,
) -> Range {
    let get_range = move |range: Range| -> Option<Range> {
        let byte_pos = slice.char_to_byte(range.cursor(slice));

        let cap_name = |t: TextObject| format!("{}.{}", object_name, t);
        let mut cursor = QueryCursor::new();
        let nodes = lang_config.textobject_query()?.capture_nodes_any(
            &[
                &cap_name(TextObject::Movement),
                &cap_name(TextObject::Around),
                &cap_name(TextObject::Inside),
            ],
            slice_tree,
            slice,
            &mut cursor,
        )?;

        let node = match dir {
            Direction::Forward => nodes
                .filter(|n| n.start_byte() > byte_pos)
                .min_by_key(|n| n.start_byte())?,
            Direction::Backward => nodes
                .filter(|n| n.start_byte() < byte_pos)
                .max_by_key(|n| n.start_byte())?,
        };

        let len = slice.len_bytes();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        if start_byte >= len || end_byte >= len {
            return None;
        }

        let start_char = slice.byte_to_char(start_byte);
        let end_char = slice.byte_to_char(end_byte);

        // head of range should be at beginning
        Some(Range::new(end_char, start_char))
    };
    (0..count).fold(range, |range, _| get_range(range).unwrap_or(range))
}

#[cfg(test)]
mod test {
    use ropey::Rope;

    use crate::{coords_at_pos, pos_at_coords};

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
        let pos = pos_at_coords(slice, (0, 4).into(), true);

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                slice,
                move_vertically(slice, range, Direction::Forward, 1, Movement::Move, 4).head
            ),
            (1, 3).into()
        );
    }

    #[test]
    fn horizontal_moves_through_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);

        let mut range = Range::point(position);

        let moves_and_expected_coordinates = [
            ((Direction::Forward, 1usize), (0, 1)), // T|his is a simple alphabetic line
            ((Direction::Forward, 2usize), (0, 3)), // Thi|s is a simple alphabetic line
            ((Direction::Forward, 0usize), (0, 3)), // Thi|s is a simple alphabetic line
            ((Direction::Forward, 999usize), (0, 32)), // This is a simple alphabetic line|
            ((Direction::Forward, 999usize), (0, 32)), // This is a simple alphabetic line|
            ((Direction::Backward, 999usize), (0, 0)), // |This is a simple alphabetic line
        ];

        for ((direction, amount), coordinates) in moves_and_expected_coordinates {
            range = move_horizontally(slice, range, direction, amount, Movement::Move, 0);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into())
        }
    }

    #[test]
    fn horizontal_moves_through_multiline_text() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);

        let mut range = Range::point(position);

        let moves_and_expected_coordinates = [
            ((Direction::Forward, 11usize), (1, 1)), // Multiline\nt|ext sample\n...
            ((Direction::Backward, 1usize), (1, 0)), // Multiline\n|text sample\n...
            ((Direction::Backward, 5usize), (0, 5)), // Multi|line\ntext sample\n...
            ((Direction::Backward, 999usize), (0, 0)), // |Multiline\ntext sample\n...
            ((Direction::Forward, 3usize), (0, 3)),  // Mul|tiline\ntext sample\n...
            ((Direction::Forward, 0usize), (0, 3)),  // Mul|tiline\ntext sample\n...
            ((Direction::Backward, 0usize), (0, 3)), // Mul|tiline\ntext sample\n...
            ((Direction::Forward, 999usize), (5, 0)), // ...and whitespaced\n|
            ((Direction::Forward, 999usize), (5, 0)), // ...and whitespaced\n|
        ];

        for ((direction, amount), coordinates) in moves_and_expected_coordinates {
            range = move_horizontally(slice, range, direction, amount, Movement::Move, 0);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn selection_extending_moves_in_single_line_text() {
        let text = Rope::from(SINGLE_LINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);

        let mut range = Range::point(position);
        let original_anchor = range.anchor;

        let moves = [
            (Direction::Forward, 1usize),
            (Direction::Forward, 5usize),
            (Direction::Backward, 3usize),
        ];

        for (direction, amount) in moves {
            range = move_horizontally(slice, range, direction, amount, Movement::Extend, 0);
            assert_eq!(range.anchor, original_anchor);
        }
    }

    #[test]
    fn vertical_moves_in_single_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);
        let mut range = Range::point(position);
        let moves_and_expected_coordinates = [
            ((Direction::Forward, 1usize), (1, 0)),
            ((Direction::Forward, 2usize), (3, 0)),
            ((Direction::Forward, 1usize), (4, 0)),
            ((Direction::Backward, 999usize), (0, 0)),
            ((Direction::Forward, 4usize), (4, 0)),
            ((Direction::Forward, 0usize), (4, 0)),
            ((Direction::Backward, 0usize), (4, 0)),
            ((Direction::Forward, 5), (5, 0)),
            ((Direction::Forward, 999usize), (5, 0)),
        ];

        for ((direction, amount), coordinates) in moves_and_expected_coordinates {
            range = move_vertically(slice, range, direction, amount, Movement::Move, 4);
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn vertical_moves_jumping_column() {
        let text = Rope::from(MULTILINE_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);
        let mut range = Range::point(position);

        enum Axis {
            H,
            V,
        }
        let moves_and_expected_coordinates = [
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
            ((Axis::V, Direction::Forward, 4usize), (4, 8)),
            ((Axis::V, Direction::Forward, 999usize), (5, 0)),
        ];

        for ((axis, direction, amount), coordinates) in moves_and_expected_coordinates {
            range = match axis {
                Axis::H => move_horizontally(slice, range, direction, amount, Movement::Move, 0),
                Axis::V => move_vertically(slice, range, direction, amount, Movement::Move, 4),
            };
            assert_eq!(coords_at_pos(slice, range.head), coordinates.into());
            assert_eq!(range.head, range.anchor);
        }
    }

    #[test]
    fn multibyte_character_wide_column_jumps() {
        let text = Rope::from(MULTIBYTE_CHARACTER_SAMPLE);
        let slice = text.slice(..);
        let position = pos_at_coords(slice, (0, 0).into(), true);
        let mut range = Range::point(position);

        // FIXME: The behaviour captured in this test diverges from both Kakoune and Vim. These
        // will attempt to preserve the horizontal position of the cursor, rather than
        // placing it at the same character index.
        enum Axis {
            H,
            V,
        }
        let moves_and_expected_coordinates = [
            // Places cursor at the fourth kana.
            ((Axis::H, Direction::Forward, 4), (0, 4)),
            // Descent places cursor at the 8th character.
            ((Axis::V, Direction::Forward, 1usize), (1, 8)),
            // Moving back 2 characters.
            ((Axis::H, Direction::Backward, 2usize), (1, 6)),
            // Jumping back up 1 line.
            ((Axis::V, Direction::Backward, 1usize), (0, 3)),
        ];

        for ((axis, direction, amount), coordinates) in moves_and_expected_coordinates {
            range = match axis {
                Axis::H => move_horizontally(slice, range, direction, amount, Movement::Move, 0),
                Axis::V => move_vertically(slice, range, direction, amount, Movement::Move, 4),
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
        let tests = [
            ("Basic forward motion stops at the first space",
                vec![(1, Range::new(0, 0), Range::new(0, 6))]),
            (" Starting from a boundary advances the anchor",
                vec![(1, Range::new(0, 0), Range::new(1, 10))]),
            ("Long       whitespace gap is bridged by the head",
                vec![(1, Range::new(0, 0), Range::new(0, 11))]),
            ("Previous anchor is irrelevant for forward motions",
                vec![(1, Range::new(12, 0), Range::new(0, 9))]),
            ("    Starting from whitespace moves to last space in sequence",
                vec![(1, Range::new(0, 0), Range::new(0, 4))]),
            ("Starting from mid-word leaves anchor at start position and moves head",
                vec![(1, Range::new(3, 3), Range::new(3, 9))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 29))]),
            ("Jumping\n    into starting whitespace selects the spaces before 'into'",
                vec![(1, Range::new(0, 7), Range::new(8, 12))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 12)),
                    (1, Range::new(0, 12), Range::new(12, 15)),
                    (1, Range::new(12, 15), Range::new(15, 18))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 6)),
                    (1, Range::new(0, 6), Range::new(6, 10)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 2))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 8)),
                    (1, Range::new(0, 8), Range::new(10, 14)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects whitespace.",
                vec![
                    (1, Range::new(0, 9), Range::new(13, 16)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("oh oh oh two character words!",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 3)),
                    (1, Range::new(0, 3), Range::new(3, 6)),
                    (1, Range::new(0, 2), Range::new(1, 3)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(17, 20)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(32, 41)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(5, 5)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 4)),
                    (1, Range::new(1, 4), Range::new(5, 8)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 6)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_next_long_words() {
        let tests = [
            ("Basic forward motion stops at the first space",
                vec![(1, Range::new(0, 0), Range::new(0, 6))]),
            (" Starting from a boundary advances the anchor",
                vec![(1, Range::new(0, 0), Range::new(1, 10))]),
            ("Long       whitespace gap is bridged by the head",
                vec![(1, Range::new(0, 0), Range::new(0, 11))]),
            ("Previous anchor is irrelevant for forward motions",
                vec![(1, Range::new(12, 0), Range::new(0, 9))]),
            ("    Starting from whitespace moves to last space in sequence",
                vec![(1, Range::new(0, 0), Range::new(0, 4))]),
            ("Starting from mid-word leaves anchor at start position and moves head",
                vec![(1, Range::new(3, 3), Range::new(3, 9))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 29))]),
            ("Jumping\n    into starting whitespace selects the spaces before 'into'",
                vec![(1, Range::new(0, 7), Range::new(8, 12))]),
            ("alphanumeric.!,and.?=punctuation are not treated any differently than alphanumerics",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 33)),
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 6)),
                    (1, Range::new(0, 6), Range::new(6, 10)),
                ]),
            (".._.._ punctuation is joined by underscores into a single word, as it behaves like alphanumerics",
                vec![(1, Range::new(0, 0), Range::new(0, 7))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 8)),
                    (1, Range::new(0, 8), Range::new(10, 14)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects whitespace.",
                vec![
                    (1, Range::new(0, 9), Range::new(13, 16)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("oh oh oh two character words!",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 3)),
                    (1, Range::new(0, 3), Range::new(3, 6)),
                    (1, Range::new(0, 1), Range::new(0, 3)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(17, 20)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(32, 41)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(5, 5)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 4)),
                    (1, Range::new(1, 4), Range::new(5, 8)),
                ]),
            ("ヒー..リクス multibyte characters behave as normal characters, including their interaction with punctuation",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 8)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_long_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_previous_words() {
        let tests = [
            ("Basic backward motion from the middle of a word",
                vec![(1, Range::new(3, 3), Range::new(4, 0))]),

            // // Why do we want this behavior?  The current behavior fails this
            // // test, but seems better and more consistent.
            // ("Starting from after boundary retreats the anchor",
            //     vec![(1, Range::new(0, 9), Range::new(8, 0))]),

            ("    Jump to start of a word preceded by whitespace",
                vec![(1, Range::new(5, 5), Range::new(6, 4))]),
            ("    Jump to start of line from start of word preceded by whitespace",
                vec![(1, Range::new(4, 4), Range::new(4, 0))]),
            ("Previous anchor is irrelevant for backward motions",
                vec![(1, Range::new(12, 5), Range::new(6, 0))]),
            ("    Starting from whitespace moves to first space in sequence",
                vec![(1, Range::new(0, 4), Range::new(4, 0))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 20), Range::new(20, 0))]),
            ("Jumping\n    \nback through a newline selects whitespace",
                vec![(1, Range::new(0, 13), Range::new(12, 8))]),
            ("Jumping to start of word from the end selects the word",
                vec![(1, Range::new(6, 7), Range::new(7, 0))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(29, 30), Range::new(30, 21)),
                    (1, Range::new(30, 21), Range::new(21, 18)),
                    (1, Range::new(21, 18), Range::new(18, 15))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 10), Range::new(10, 6)),
                    (1, Range::new(10, 6), Range::new(6, 0)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 6), Range::new(5, 3))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 10), Range::new(8, 0)),
                ]),
            ("Jumping    \n\n\n\n\nback from within a newline group selects previous block",
                vec![
                    (1, Range::new(0, 13), Range::new(11, 0)),
                ]),
            ("Failed motions do not modify the range",
                vec![
                    (0, Range::new(3, 0), Range::new(3, 0)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(18, 18), Range::new(9, 0)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(40, 40), Range::new(10, 0)),
                ]),
            ("", // Edge case of moving backwards in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving backwards in all newlines
                vec![
                    (1, Range::new(5, 5), Range::new(0, 0)),
                ]),
            ("   \n   \nJumping back through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 8), Range::new(7, 4)),
                    (1, Range::new(7, 4), Range::new(3, 0)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 6), Range::new(6, 0)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_prev_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_start_of_previous_long_words() {
        let tests = [
            (
                "Basic backward motion from the middle of a word",
                vec![(1, Range::new(3, 3), Range::new(4, 0))],
            ),

            // // Why do we want this behavior?  The current behavior fails this
            // // test, but seems better and more consistent.
            // ("Starting from after boundary retreats the anchor",
            //     vec![(1, Range::new(0, 9), Range::new(8, 0))]),

            (
                "    Jump to start of a word preceded by whitespace",
                vec![(1, Range::new(5, 5), Range::new(6, 4))],
            ),
            (
                "    Jump to start of line from start of word preceded by whitespace",
                vec![(1, Range::new(3, 4), Range::new(4, 0))],
            ),
            ("Previous anchor is irrelevant for backward motions",
                vec![(1, Range::new(12, 5), Range::new(6, 0))]),
            (
                "    Starting from whitespace moves to first space in sequence",
                vec![(1, Range::new(0, 4), Range::new(4, 0))],
            ),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 20), Range::new(20, 0))]),
            (
                "Jumping\n    \nback through a newline selects whitespace",
                vec![(1, Range::new(0, 13), Range::new(12, 8))],
            ),
            (
                "Jumping to start of word from the end selects the word",
                vec![(1, Range::new(6, 7), Range::new(7, 0))],
            ),
            (
                "alphanumeric.!,and.?=punctuation are treated exactly the same",
                vec![(1, Range::new(29, 30), Range::new(30, 0))],
            ),
            (
                "...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 10), Range::new(10, 6)),
                    (1, Range::new(10, 6), Range::new(6, 0)),
                ],
            ),
            (".._.._ punctuation is joined by underscores into a single block",
                vec![(1, Range::new(0, 6), Range::new(6, 0))]),
            (
                "Newlines\n\nare bridged seamlessly.",
                vec![(1, Range::new(0, 10), Range::new(8, 0))],
            ),
            (
                "Jumping    \n\n\n\n\nback from within a newline group selects previous block",
                vec![(1, Range::new(0, 13), Range::new(11, 0))],
            ),
            (
                "Failed motions do not modify the range",
                vec![(0, Range::new(3, 0), Range::new(3, 0))],
            ),
            (
                "Multiple motions at once resolve correctly",
                vec![(3, Range::new(19, 19), Range::new(9, 0))],
            ),
            (
                "Excessive motions are performed partially",
                vec![(999, Range::new(40, 40), Range::new(10, 0))],
            ),
            (
                "", // Edge case of moving backwards in empty string
                vec![(1, Range::new(0, 0), Range::new(0, 0))],
            ),
            (
                "\n\n\n\n\n", // Edge case of moving backwards in all newlines
                vec![(1, Range::new(5, 5), Range::new(0, 0))],
            ),
            ("   \n   \nJumping back through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 8), Range::new(7, 4)),
                    (1, Range::new(7, 4), Range::new(3, 0)),
                ]),
            ("ヒーリ..クス multibyte characters behave as normal characters, including when interacting with punctuation",
                vec![
                    (1, Range::new(0, 8), Range::new(8, 0)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_prev_long_word_start(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_end_of_next_words() {
        let tests = [
            ("Basic forward motion from the start of a word to the end of it",
                vec![(1, Range::new(0, 0), Range::new(0, 5))]),
            ("Basic forward motion from the end of a word to the end of the next",
                vec![(1, Range::new(0, 5), Range::new(5, 13))]),
            ("Basic forward motion from the middle of a word to the end of it",
                vec![(1, Range::new(2, 2), Range::new(2, 5))]),
            ("    Jumping to end of a word preceded by whitespace",
                vec![(1, Range::new(0, 0), Range::new(0, 11))]),

            // // Why do we want this behavior?  The current behavior fails this
            // // test, but seems better and more consistent.
            // (" Starting from a boundary advances the anchor",
            //     vec![(1, Range::new(0, 0), Range::new(1, 9))]),

            ("Previous anchor is irrelevant for end of word motion",
                vec![(1, Range::new(12, 2), Range::new(2, 8))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 28))]),
            ("Jumping\n    into starting whitespace selects up to the end of next word",
                vec![(1, Range::new(0, 7), Range::new(8, 16))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 12)),
                    (1, Range::new(0, 12), Range::new(12, 15)),
                    (1, Range::new(12, 15), Range::new(15, 18))
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 3)),
                    (1, Range::new(0, 3), Range::new(3, 9)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 2))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 8)),
                    (1, Range::new(0, 8), Range::new(10, 13)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects to end of next word.",
                vec![
                    (1, Range::new(0, 8), Range::new(13, 20)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(16, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(31, 41)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(5, 5)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 4)),
                    (1, Range::new(1, 4), Range::new(5, 8)),
                ]),
            ("ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 5)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_word_end(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_end_of_previous_words() {
        let tests = [
            ("Basic backward motion from the middle of a word",
                vec![(1, Range::new(9, 9), Range::new(10, 5))]),
            ("Starting from after boundary retreats the anchor",
                vec![(1, Range::new(0, 14), Range::new(13, 8))]),
            ("Jump     to end of a word succeeded by whitespace",
                vec![(1, Range::new(11, 11), Range::new(11, 4))]),
            ("    Jump to start of line from end of word preceded by whitespace",
                vec![(1, Range::new(8, 8), Range::new(8, 0))]),
            ("Previous anchor is irrelevant for backward motions",
                vec![(1, Range::new(26, 12), Range::new(13, 8))]),
            ("    Starting from whitespace moves to first space in sequence",
                vec![(1, Range::new(0, 4), Range::new(4, 0))]),
            ("Test identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 25), Range::new(25, 4))]),
            ("Jumping\n    \nback through a newline selects whitespace",
                vec![(1, Range::new(0, 13), Range::new(12, 8))]),
            ("Jumping to start of word from the end selects the whole word",
                vec![(1, Range::new(16, 16), Range::new(16, 10))]),
            ("alphanumeric.!,and.?=punctuation are considered 'words' for the purposes of word motion",
                vec![
                    (1, Range::new(30, 30), Range::new(31, 21)),
                    (1, Range::new(31, 21), Range::new(21, 18)),
                    (1, Range::new(21, 18), Range::new(18, 15))
                ]),

            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 10), Range::new(9, 3)),
                    (1, Range::new(9, 3), Range::new(3, 0)),
                ]),
            (".._.._ punctuation is not joined by underscores into a single block",
                vec![(1, Range::new(0, 5), Range::new(5, 3))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 10), Range::new(8, 0)),
                ]),
            ("Jumping    \n\n\n\n\nback from within a newline group selects previous block",
                vec![
                    (1, Range::new(0, 13), Range::new(11, 7)),
                ]),
            ("Failed motions do not modify the range",
                vec![
                    (0, Range::new(3, 0), Range::new(3, 0)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(24, 24), Range::new(16, 8)),
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
                    (1, Range::new(5, 5), Range::new(0, 0)),
                ]),
            ("   \n   \nJumping back through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 8), Range::new(7, 4)),
                    (1, Range::new(7, 4), Range::new(3, 0)),
                ]),
            ("Test ヒーリクス multibyte characters behave as normal characters",
                vec![
                    (1, Range::new(0, 10), Range::new(10, 4)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_prev_word_end(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_end_of_next_long_words() {
        let tests = [
            ("Basic forward motion from the start of a word to the end of it",
                vec![(1, Range::new(0, 0), Range::new(0, 5))]),
            ("Basic forward motion from the end of a word to the end of the next",
                vec![(1, Range::new(0, 5), Range::new(5, 13))]),
            ("Basic forward motion from the middle of a word to the end of it",
                vec![(1, Range::new(2, 2), Range::new(2, 5))]),
            ("    Jumping to end of a word preceded by whitespace",
                vec![(1, Range::new(0, 0), Range::new(0, 11))]),

            // // Why do we want this behavior?  The current behavior fails this
            // // test, but seems better and more consistent.
            // (" Starting from a boundary advances the anchor",
            //     vec![(1, Range::new(0, 0), Range::new(1, 9))]),

            ("Previous anchor is irrelevant for end of word motion",
                vec![(1, Range::new(12, 2), Range::new(2, 8))]),
            ("Identifiers_with_underscores are considered a single word",
                vec![(1, Range::new(0, 0), Range::new(0, 28))]),
            ("Jumping\n    into starting whitespace selects up to the end of next word",
                vec![(1, Range::new(0, 7), Range::new(8, 16))]),
            ("alphanumeric.!,and.?=punctuation are treated the same way",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 32)),
                ]),
            ("...   ... punctuation and spaces behave as expected",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 3)),
                    (1, Range::new(0, 3), Range::new(3, 9)),
                ]),
            (".._.._ punctuation is joined by underscores into a single block",
                vec![(1, Range::new(0, 0), Range::new(0, 6))]),
            ("Newlines\n\nare bridged seamlessly.",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 8)),
                    (1, Range::new(0, 8), Range::new(10, 13)),
                ]),
            ("Jumping\n\n\n\n\n\n   from newlines to whitespace selects to end of next word.",
                vec![
                    (1, Range::new(0, 9), Range::new(13, 20)),
                ]),
            ("A failed motion does not modify the range",
                vec![
                    (3, Range::new(37, 41), Range::new(37, 41)),
                ]),
            ("Multiple motions at once resolve correctly",
                vec![
                    (3, Range::new(0, 0), Range::new(16, 19)),
                ]),
            ("Excessive motions are performed partially",
                vec![
                    (999, Range::new(0, 0), Range::new(31, 41)),
                ]),
            ("", // Edge case of moving forward in empty string
                vec![
                    (1, Range::new(0, 0), Range::new(0, 0)),
                ]),
            ("\n\n\n\n\n", // Edge case of moving forward in all newlines
                vec![
                    (1, Range::new(0, 0), Range::new(5, 5)),
                ]),
            ("\n   \n   \n Jumping through alternated space blocks and newlines selects the space blocks",
                vec![
                    (1, Range::new(0, 0), Range::new(1, 4)),
                    (1, Range::new(1, 4), Range::new(5, 8)),
                ]),
            ("ヒーリ..クス multibyte characters behave as normal characters, including  when they interact with punctuation",
                vec![
                    (1, Range::new(0, 0), Range::new(0, 7)),
                ]),
        ];

        for (sample, scenario) in tests {
            for (count, begin, expected_end) in scenario.into_iter() {
                let range = move_next_long_word_end(Rope::from(sample).slice(..), begin, count);
                assert_eq!(range, expected_end, "Case failed: [{}]", sample);
            }
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_prev_paragraph_single() {
        let tests = [
            ("#[|]#", "#[|]#"),
            ("#[s|]#tart at\nfirst char\n", "#[|s]#tart at\nfirst char\n"),
            ("start at\nlast char#[\n|]#", "#[|start at\nlast char\n]#"),
            (
                "goto\nfirst\n\n#[p|]#aragraph",
                "#[|goto\nfirst\n\n]#paragraph",
            ),
            (
                "goto\nfirst\n#[\n|]#paragraph",
                "#[|goto\nfirst\n\n]#paragraph",
            ),
            (
                "goto\nsecond\n\np#[a|]#ragraph",
                "goto\nsecond\n\n#[|pa]#ragraph",
            ),
            (
                "here\n\nhave\nmultiple\nparagraph\n\n\n\n\n#[|]#",
                "here\n\n#[|have\nmultiple\nparagraph\n\n\n\n\n]#",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection =
                selection.transform(|r| move_prev_paragraph(text.slice(..), r, 1, Movement::Move));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_prev_paragraph_double() {
        let tests = [
            (
                "on#[e|]#\n\ntwo\n\nthree\n\n",
                "#[|one]#\n\ntwo\n\nthree\n\n",
            ),
            (
                "one\n\ntwo\n\nth#[r|]#ee\n\n",
                "one\n\n#[|two\n\nthr]#ee\n\n",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection =
                selection.transform(|r| move_prev_paragraph(text.slice(..), r, 2, Movement::Move));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_prev_paragraph_extend() {
        let tests = [
            (
                "one\n\n#[|two\n\n]#three\n\n",
                "#[|one\n\ntwo\n\n]#three\n\n",
            ),
            (
                "#[|one\n\ntwo\n\n]#three\n\n",
                "#[|one\n\ntwo\n\n]#three\n\n",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection = selection
                .transform(|r| move_prev_paragraph(text.slice(..), r, 1, Movement::Extend));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_next_paragraph_single() {
        let tests = [
            ("#[|]#", "#[|]#"),
            ("#[s|]#tart at\nfirst char\n", "#[start at\nfirst char\n|]#"),
            ("start at\nlast char#[\n|]#", "start at\nlast char#[\n|]#"),
            (
                "a\nb\n\n#[g|]#oto\nthird\n\nparagraph",
                "a\nb\n\n#[goto\nthird\n\n|]#paragraph",
            ),
            (
                "a\nb\n#[\n|]#goto\nthird\n\nparagraph",
                "a\nb\n\n#[goto\nthird\n\n|]#paragraph",
            ),
            (
                "a\nb#[\n|]#\ngoto\nsecond\n\nparagraph",
                "a\nb#[\n\n|]#goto\nsecond\n\nparagraph",
            ),
            (
                "here\n\nhave\n#[m|]#ultiple\nparagraph\n\n\n\n\n",
                "here\n\nhave\n#[multiple\nparagraph\n\n\n\n\n|]#",
            ),
            (
                "#[t|]#ext\n\n\nafter two blank lines\n\nmore text\n",
                "#[text\n\n\n|]#after two blank lines\n\nmore text\n",
            ),
            (
                "#[text\n\n\n|]#after two blank lines\n\nmore text\n",
                "text\n\n\n#[after two blank lines\n\n|]#more text\n",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection =
                selection.transform(|r| move_next_paragraph(text.slice(..), r, 1, Movement::Move));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_next_paragraph_double() {
        let tests = [
            (
                "one\n\ntwo\n\nth#[r|]#ee\n\n",
                "one\n\ntwo\n\nth#[ree\n\n|]#",
            ),
            (
                "on#[e|]#\n\ntwo\n\nthree\n\n",
                "on#[e\n\ntwo\n\n|]#three\n\n",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection =
                selection.transform(|r| move_next_paragraph(text.slice(..), r, 2, Movement::Move));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }

    #[test]
    fn test_behaviour_when_moving_to_next_paragraph_extend() {
        let tests = [
            (
                "one\n\n#[two\n\n|]#three\n\n",
                "one\n\n#[two\n\nthree\n\n|]#",
            ),
            (
                "one\n\n#[two\n\nthree\n\n|]#",
                "one\n\n#[two\n\nthree\n\n|]#",
            ),
        ];

        for (before, expected) in tests {
            let (s, selection) = crate::test::print(before);
            let text = Rope::from(s.as_str());
            let selection = selection
                .transform(|r| move_next_paragraph(text.slice(..), r, 1, Movement::Extend));
            let actual = crate::test::plain(&s, selection);
            assert_eq!(actual, expected, "\nbefore: `{:?}`", before);
        }
    }
}
