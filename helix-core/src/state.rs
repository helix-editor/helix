use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::syntax::LOADER;
use crate::{Position, Range, Rope, RopeSlice, Selection, Syntax};
use anyhow::Error;

use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal,
    Insert,
    Goto,
}

/// A state represents the current editor state of a single buffer.
pub struct State {
    // TODO: fields should be private but we need to refactor commands.rs first
    /// Path to file on disk.
    pub path: Option<PathBuf>,
    pub doc: Rope,
    pub selection: Selection,
    pub mode: Mode,

    pub restore_cursor: bool,

    //
    pub syntax: Option<Syntax>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Granularity {
    Character,
    Word,
    Line,
    // LineBoundary
}

impl State {
    #[must_use]
    pub fn new(doc: Rope) -> Self {
        Self {
            path: None,
            doc,
            selection: Selection::single(0, 0),
            mode: Mode::Normal,
            syntax: None,
            restore_cursor: false,
        }
    }

    // TODO: passing scopes here is awkward
    pub fn load(path: PathBuf, scopes: &[String]) -> Result<Self, Error> {
        use std::{env, fs::File, io::BufReader, path::PathBuf};
        let _current_dir = env::current_dir()?;

        let doc = Rope::from_reader(BufReader::new(File::open(path.clone())?))?;

        // TODO: create if not found

        let mut state = Self::new(doc);

        if let Some(language_config) = LOADER.language_config_for_file_name(path.as_path()) {
            let highlight_config = language_config.highlight_config(scopes).unwrap().unwrap();
            // TODO: config.configure(scopes) is now delayed, is that ok?

            let syntax = Syntax::new(&state.doc, highlight_config.clone());

            state.syntax = Some(syntax);
        };

        state.path = Some(path);

        Ok(state)
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

    #[inline]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[inline]
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
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

    pub fn move_pos(
        &self,
        pos: usize,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> usize {
        let text = &self.doc;
        match (dir, granularity) {
            (Direction::Backward, Granularity::Character) => {
                // Clamp to line
                let line = text.char_to_line(pos);
                let start = text.line_to_char(line);
                std::cmp::max(
                    nth_prev_grapheme_boundary(&text.slice(..), pos, count),
                    start,
                )
            }
            (Direction::Forward, Granularity::Character) => {
                // Clamp to line
                let line = text.char_to_line(pos);
                // Line end is pos at the start of next line - 1
                // subtract another 1 because the line ends with \n
                let end = text.line_to_char(line + 1).saturating_sub(2);
                std::cmp::min(nth_next_grapheme_boundary(&text.slice(..), pos, count), end)
            }
            (Direction::Forward, Granularity::Word) => {
                Self::move_next_word_start(&text.slice(..), pos)
            }
            (Direction::Backward, Granularity::Word) => {
                Self::move_prev_word_start(&text.slice(..), pos)
            }
            (_, Granularity::Line) => move_vertically(&text.slice(..), dir, pos, count),
            _ => pos,
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
        // TODO: move all selections according to normal cursor move semantics by collapsing it
        // into cursors and moving them vertically

        self.selection.transform(|range| {
            // let pos = if !range.is_empty() {
            //     // if selection already exists, bump it to the start or end of current select first
            //     if dir == Direction::Backward {
            //         range.from()
            //     } else {
            //         range.to()
            //     }
            // } else {
            let pos = self.move_pos(range.head, dir, granularity, count);
            // };
            Range::new(pos, pos)
        })
    }

    pub fn extend_selection(
        &self,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        self.selection.transform(|range| {
            let pos = self.move_pos(range.head, dir, granularity, count);
            Range::new(range.anchor, pos)
        })
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

fn move_vertically(text: &RopeSlice, dir: Direction, pos: usize, count: usize) -> usize {
    let Position { row, col } = coords_at_pos(text, pos);

    let new_line = match dir {
        Direction::Backward => row.saturating_sub(count),
        Direction::Forward => std::cmp::min(row.saturating_add(count), text.len_lines() - 1),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = text.line(new_line).len_chars().saturating_sub(2);

    let new_col = if new_line_len < col {
        // TODO: preserve horiz here
        new_line_len
    } else {
        col
    };

    pos_at_coords(text, Position::new(new_line, new_col))
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

        assert_eq!(
            coords_at_pos(&slice, move_vertically(&slice, Direction::Forward, pos, 1)),
            (1, 2).into()
        );
    }
}
