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
            selection: Selection::single(0, 0),
        }
    }

    // TODO: doc/selection accessors

    // TODO: be able to take either Rope or RopeSlice
    #[inline]
    pub fn doc(&self) -> &Rope {
        &self.doc
    }

    #[inline]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    // pub fn doc<R>(&self, range: R) -> RopeSlice
    // where
    //     R: std::ops::RangeBounds<usize>,
    // {
    //     self.doc.slice(range)
    // }

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

    // TODO: move that accepts a boundary matcher fn/list, we keep incrementing until we hit
    // a boundary

    // TODO: edits, does each keypress trigger a full command? I guess it's adding to the same
    // transaction
    // There should be three pieces of the state: current transaction, the original doc, "preview"
    // of the new state.
    // 1. apply the newly generated keypress as a transaction
    // 2. compose onto a ongoing transaction
    // 3. on insert mode leave, that transaction gets stored into undo history

    pub fn move_range(
        &self,
        range: Range,
        dir: Direction,
        granularity: Granularity,
        count: usize,
        extend: bool,
    ) -> Range {
        let text = &self.doc;
        let pos = range.head;
        match (dir, granularity) {
            (Direction::Backward, Granularity::Character) => {
                // Clamp to line
                let line = text.char_to_line(pos);
                let start = text.line_to_char(line);
                let pos = std::cmp::max(
                    nth_prev_grapheme_boundary(&text.slice(..), pos, count),
                    start,
                );
                Range::new(if extend { range.anchor } else { pos }, pos)
            }
            (Direction::Forward, Granularity::Character) => {
                // Clamp to line
                let line = text.char_to_line(pos);
                // Line end is pos at the start of next line - 1
                // subtract another 1 because the line ends with \n
                let end = text.line_to_char(line + 1).saturating_sub(2);
                let pos =
                    std::cmp::min(nth_next_grapheme_boundary(&text.slice(..), pos, count), end);
                Range::new(if extend { range.anchor } else { pos }, pos)
            }
            (_, Granularity::Line) => move_vertically(&text.slice(..), dir, range, count, extend),
        }
    }

    pub fn move_next_word_start(slice: &RopeSlice, mut pos: usize) -> usize {
        // TODO: confirm it's fine without using graphemes, I think it should be
        let ch = slice.char(pos);
        let next = slice.char(pos.saturating_add(1));
        if categorize(ch) != categorize(next) {
            pos += 1;
        }

        // refetch
        let ch = slice.char(pos);

        if is_word(ch) {
            skip_over_next(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }

        // TODO: don't include newline?
        skip_over_next(slice, &mut pos, |ch| ch.is_ascii_whitespace());

        pos
    }

    pub fn move_prev_word_start(slice: &RopeSlice, mut pos: usize) -> usize {
        // TODO: confirm it's fine without using graphemes, I think it should be
        let ch = slice.char(pos);
        let prev = slice.char(pos.saturating_sub(1)); // TODO: just return original pos if at start

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

        pos.saturating_add(1)
    }

    pub fn move_next_word_end(slice: &RopeSlice, mut pos: usize, _count: usize) -> usize {
        // TODO: confirm it's fine without using graphemes, I think it should be
        let ch = slice.char(pos);
        let next = slice.char(pos.saturating_add(1));
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

        pos.saturating_sub(1)
    }

    pub fn move_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        // move all selections according to normal cursor move semantics by collapsing it
        // into cursors and moving them vertically

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
pub fn coords_at_pos(text: &RopeSlice, pos: usize) -> Position {
    let line = text.char_to_line(pos);
    let line_start = text.line_to_char(line);
    let col = text.slice(line_start..pos).len_chars();
    Position::new(line, col)
}

/// Convert (line, column) coordinates to a character index.
pub fn pos_at_coords(text: &RopeSlice, coords: Position) -> usize {
    let Position { row, col } = coords;
    let line_start = text.line_to_char(row);
    nth_next_grapheme_boundary(text, line_start, col)
}

fn move_vertically(
    text: &RopeSlice,
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
    use std::convert::TryInto;
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
    EOL,
    Word,
    Punctuation,
}
fn categorize(ch: char) -> Category {
    if ch == '\n' {
        Category::EOL
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

fn skip_over_next<F>(slice: &RopeSlice, pos: &mut usize, fun: F)
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

fn skip_over_prev<F>(slice: &RopeSlice, pos: &mut usize, fun: F)
where
    F: Fn(char) -> bool,
{
    // need to +1 so that prev() includes current char
    let mut chars = slice.chars_at(*pos + 1);

    while let Some(ch) = chars.prev() {
        if !fun(ch) {
            break;
        }
        *pos -= 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_coords_at_pos() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        assert_eq!(coords_at_pos(&text.slice(..), 0), (0, 0).into());
        assert_eq!(coords_at_pos(&text.slice(..), 5), (0, 5).into()); // position on \n
        assert_eq!(coords_at_pos(&text.slice(..), 6), (1, 0).into()); // position on w
        assert_eq!(coords_at_pos(&text.slice(..), 7), (1, 1).into()); // position on o
        assert_eq!(coords_at_pos(&text.slice(..), 10), (1, 4).into()); // position on d
    }

    #[test]
    fn test_pos_at_coords() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        assert_eq!(pos_at_coords(&text.slice(..), (0, 0).into()), 0);
        assert_eq!(pos_at_coords(&text.slice(..), (0, 5).into()), 5); // position on \n
        assert_eq!(pos_at_coords(&text.slice(..), (1, 0).into()), 6); // position on w
        assert_eq!(pos_at_coords(&text.slice(..), (1, 1).into()), 7); // position on o
        assert_eq!(pos_at_coords(&text.slice(..), (1, 4).into()), 10); // position on d
    }

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let pos = pos_at_coords(&text.slice(..), (0, 4).into());
        let slice = text.slice(..);

        let range = Range::new(pos, pos);
        assert_eq!(
            coords_at_pos(
                &slice,
                move_vertically(&slice, Direction::Forward, range, 1).head
            ),
            (1, 2).into()
        );
    }
}
