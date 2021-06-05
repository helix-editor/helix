use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::{coords_at_pos, pos_at_coords, ChangeSet, Position, Range, Rope, RopeSlice, Selection};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

pub fn move_horizontally(
    text: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    extend: bool,
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
    Range::new(if extend { range.anchor } else { pos }, pos)
}

pub fn move_vertically(
    text: RopeSlice,
    range: Range,
    dir: Direction,
    count: usize,
    extend: bool,
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

    let mut range = Range::new(if extend { range.anchor } else { pos }, pos);
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
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut end, |ch| ch.is_ascii_punctuation());
        }

        skip_over_next(slice, &mut end, is_horiz_blank);
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

        with_end = skip_over_prev(slice, &mut end, is_horiz_blank);

        // refetch
        let ch = slice.char(end);

        if is_word(ch) {
            with_end = skip_over_prev(slice, &mut end, is_word);
        } else if ch.is_ascii_punctuation() {
            with_end = skip_over_prev(slice, &mut end, |ch| ch.is_ascii_punctuation());
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

        skip_over_next(slice, &mut end, is_horiz_blank);

        // refetch
        let ch = slice.char(end);

        if is_word(ch) {
            skip_over_next(slice, &mut end, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut end, |ch| ch.is_ascii_punctuation());
        }
    }

    Some(Range::new(begin, end - 1))
}

// ---- util ------------

// used for by-word movement

pub(crate) fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub(crate) fn is_horiz_blank(ch: char) -> bool {
    matches!(ch, ' ' | '\t')
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Category {
    Whitespace,
    Eol,
    Word,
    Punctuation,
    Unknown,
}

pub(crate) fn categorize(ch: char) -> Category {
    if ch == '\n' {
        Category::Eol
    } else if ch.is_ascii_whitespace() {
        Category::Whitespace
    } else if is_word(ch) {
        Category::Word
    } else if ch.is_ascii_punctuation() {
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
    use super::*;

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let slice = text.slice(..);
        let pos = pos_at_coords(slice, (0, 4).into());

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                slice,
                move_vertically(slice, range, Direction::Forward, 1, false).head
            ),
            (1, 2).into()
        );
    }
}
