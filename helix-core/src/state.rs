use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::syntax::LOADER;
use crate::{ChangeSet, Diagnostic, Position, Range, Rope, RopeSlice, Selection, Syntax};
use anyhow::Error;

/// A state represents the current editor state of a single buffer.
#[derive(Clone)]
pub struct State {
    // TODO: fields should be private but we need to refactor commands.rs first
    pub doc: Rope,
    pub selection: Selection,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Granularity {
    Character,
    Line,
}

impl State {
    #[must_use]
    pub fn new(doc: Rope) -> Self {
        Self {
            doc,
            selection: Selection::point(0),
        }
    }

    // update/transact:
    // update(desc) => transaction ?  transaction.doc() for applied doc
    // transaction.apply(doc)
    // doc.transact(fn -> ... end)

    // replaceSelection (transaction that replaces selection)
    // changeByRange
    // changes
    // slice
    //
    // getters:
    // tabSize
    // indentUnit
    // languageDataAt()
    //
    // config:
    // indentation
    // tabSize
    // lineUnit
    // syntax
    // foldable
    // changeFilter/transactionFilter

    pub fn move_range(
        &self,
        range: Range,
        dir: Direction,
        granularity: Granularity,
        count: usize,
        extend: bool,
    ) -> Range {
        let text = self.doc.slice(..);
        let pos = range.head;
        let line = text.char_to_line(pos);
        // TODO: we can optimize clamping by passing in RopeSlice limited to current line. that way
        // we stop calculating past start/end of line.
        let pos = match (dir, granularity) {
            (Direction::Backward, Granularity::Character) => {
                let start = text.line_to_char(line);
                nth_prev_grapheme_boundary(text, pos, count).max(start)
            }
            (Direction::Forward, Granularity::Character) => {
                // Line end is pos at the start of next line - 1
                // subtract another 1 because the line ends with \n
                let end = text.line_to_char(line + 1).saturating_sub(2);
                nth_next_grapheme_boundary(text, pos, count).min(end)
            }
            (_, Granularity::Line) => return move_vertically(text, dir, range, count, extend),
        };
        Range::new(if extend { range.anchor } else { pos }, pos)
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

    pub fn move_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        self.selection
            .transform(|range| self.move_range(range, dir, granularity, count, false))
    }

    pub fn extend_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        self.selection
            .transform(|range| self.move_range(range, dir, granularity, count, true))
    }
}

/// Convert a character index to (line, column) coordinates.
pub fn coords_at_pos(text: RopeSlice, pos: usize) -> Position {
    let line = text.char_to_line(pos);
    let line_start = text.line_to_char(line);
    let col = RopeGraphemes::new(text.slice(line_start..pos)).count();
    Position::new(line, col)
}

/// Convert (line, column) coordinates to a character index.
pub fn pos_at_coords(text: RopeSlice, coords: Position) -> usize {
    let Position { row, col } = coords;
    let line_start = text.line_to_char(row);
    // line_start + col
    nth_next_grapheme_boundary(text, line_start, col)
}

fn move_vertically(
    text: RopeSlice,
    dir: Direction,
    range: Range,
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
    fn test_coords_at_pos() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        let slice = text.slice(..);
        // assert_eq!(coords_at_pos(slice, 0), (0, 0).into());
        // assert_eq!(coords_at_pos(slice, 5), (0, 5).into()); // position on \n
        // assert_eq!(coords_at_pos(slice, 6), (1, 0).into()); // position on w
        // assert_eq!(coords_at_pos(slice, 7), (1, 1).into()); // position on o
        // assert_eq!(coords_at_pos(slice, 10), (1, 4).into()); // position on d

        // test with grapheme clusters
        let text = Rope::from("a̐éö̲\r\n");
        let slice = text.slice(..);
        assert_eq!(coords_at_pos(slice, 0), (0, 0).into());
        assert_eq!(coords_at_pos(slice, 2), (0, 1).into());
        assert_eq!(coords_at_pos(slice, 4), (0, 2).into());
        assert_eq!(coords_at_pos(slice, 7), (0, 3).into());

        let text = Rope::from("किमपि");
        let slice = text.slice(..);
        assert_eq!(coords_at_pos(slice, 0), (0, 0).into());
        assert_eq!(coords_at_pos(slice, 2), (0, 1).into());
        assert_eq!(coords_at_pos(slice, 3), (0, 2).into());
        assert_eq!(coords_at_pos(slice, 5), (0, 3).into());
    }

    #[test]
    fn test_pos_at_coords() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        let slice = text.slice(..);
        assert_eq!(pos_at_coords(slice, (0, 0).into()), 0);
        assert_eq!(pos_at_coords(slice, (0, 5).into()), 5); // position on \n
        assert_eq!(pos_at_coords(slice, (1, 0).into()), 6); // position on w
        assert_eq!(pos_at_coords(slice, (1, 1).into()), 7); // position on o
        assert_eq!(pos_at_coords(slice, (1, 4).into()), 10); // position on d

        // test with grapheme clusters
        let text = Rope::from("a̐éö̲\r\n");
        let slice = text.slice(..);
        assert_eq!(pos_at_coords(slice, (0, 0).into()), 0);
        assert_eq!(pos_at_coords(slice, (0, 1).into()), 2);
        assert_eq!(pos_at_coords(slice, (0, 2).into()), 4);
        assert_eq!(pos_at_coords(slice, (0, 3).into()), 7); // \r\n is one char here
        assert_eq!(pos_at_coords(slice, (0, 4).into()), 9);
        let text = Rope::from("किमपि");
        // 2 - 1 - 2 codepoints
        // TODO: delete handling as per https://news.ycombinator.com/item?id=20058454
        let slice = text.slice(..);
        assert_eq!(pos_at_coords(slice, (0, 0).into()), 0);
        assert_eq!(pos_at_coords(slice, (0, 1).into()), 2);
        assert_eq!(pos_at_coords(slice, (0, 2).into()), 3);
        assert_eq!(pos_at_coords(slice, (0, 3).into()), 5); // eol
    }

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let slice = text.slice(..);
        let pos = pos_at_coords(slice, (0, 4).into());

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                slice,
                move_vertically(slice, Direction::Forward, range, 1, false).head
            ),
            (1, 2).into()
        );
    }
}
