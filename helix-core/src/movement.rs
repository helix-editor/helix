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
            // subtract another 1 because the line ends with \n
            let end = text.line_to_char(line + 1).saturating_sub(2);
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
        Direction::Forward => std::cmp::min(row.saturating_add(count), text.len_lines() - 1),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = text.line(new_line).len_chars().saturating_sub(2);

    let new_col = std::cmp::min(horiz as usize, new_line_len);

    let pos = pos_at_coords(text, Position::new(new_line, new_col));

    let mut range = Range::new(if extend { range.anchor } else { pos }, pos);
    range.horiz = Some(horiz);
    range
}

pub fn move_next_word_start(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos + 1 == slice.len_chars() {
            return pos;
        }

        let mut ch = slice.char(pos);
        let next = slice.char(pos + 1);

        // if we're at the end of a word, or on whitespce right before new one
        if categorize(ch) != categorize(next) {
            pos += 1;
            ch = next;
        }

        if is_word(ch) {
            skip_over_next(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }

        // TODO: don't include newline?
        skip_over_next(slice, &mut pos, |ch| ch.is_ascii_whitespace());
    }

    pos
}

pub fn move_prev_word_start(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos == 0 {
            return pos;
        }

        let ch = slice.char(pos);
        let prev = slice.char(pos - 1);

        if categorize(ch) != categorize(prev) {
            pos -= 1;
        }

        // match (category c1, category c2) => {
        //  if c1 != c2 {
        //  }
        // }

        // TODO: skip while eol

        // TODO: don't include newline?
        skip_over_prev(slice, &mut pos, |ch| ch.is_ascii_whitespace());

        // refetch
        let ch = slice.char(pos);

        if is_word(ch) {
            skip_over_prev(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_prev(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }
        pos = pos.saturating_add(1)
    }

    pos
}

pub fn move_next_word_end(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos + 1 == slice.len_chars() {
            return pos;
        }

        let ch = slice.char(pos);
        let next = slice.char(pos + 1);

        if categorize(ch) != categorize(next) {
            pos += 1;
        }

        // TODO: don't include newline?
        skip_over_next(slice, &mut pos, |ch| ch.is_ascii_whitespace());

        // refetch
        let ch = slice.char(pos);

        if is_word(ch) {
            skip_over_next(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }
        pos -= 1
    }

    pos
}

// ---- util ------------

// used for by-word movement

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[derive(Debug, Eq, PartialEq)]
enum Category {
    Whitespace,
    Eol,
    Word,
    Punctuation,
}
fn categorize(ch: char) -> Category {
    if ch == '\n' {
        Category::Eol
    } else if ch.is_ascii_whitespace() {
        Category::Whitespace
    } else if ch.is_ascii_punctuation() {
        Category::Punctuation
    } else if ch.is_ascii_alphanumeric() {
        Category::Word
    } else {
        unreachable!()
    }
}

#[inline]
pub fn skip_over_next<F>(slice: RopeSlice, pos: &mut usize, fun: F)
where
    F: Fn(char) -> bool,
{
    let mut chars = slice.chars_at(*pos);

    for ch in chars {
        if !fun(ch) {
            break;
        }
        *pos += 1;
    }
}

#[inline]
pub fn skip_over_prev<F>(slice: RopeSlice, pos: &mut usize, fun: F)
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
