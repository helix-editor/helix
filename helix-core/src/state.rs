use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::{Position, Rope, RopeSlice, Selection, SelectionRange, Syntax};
use anyhow::Error;

use std::path::PathBuf;

#[derive(Copy, Clone)]
pub enum Mode {
    Normal,
    Insert,
}

/// A state represents the current editor state of a single buffer.
pub struct State {
    /// Path to file on disk.
    pub(crate) path: Option<PathBuf>,
    pub(crate) doc: Rope,
    pub(crate) selection: Selection,
    pub(crate) mode: Mode,

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
        }
    }

    pub fn load(path: PathBuf) -> Result<Self, Error> {
        use std::{env, fs::File, io::BufReader, path::PathBuf};
        let _current_dir = env::current_dir()?;

        let doc = Rope::from_reader(BufReader::new(File::open(path.clone())?))?;

        // TODO: create if not found

        let mut state = Self::new(doc);
        state.path = Some(path);

        let language = helix_syntax::get_language(&helix_syntax::LANG::Rust);

        let mut highlight_config = crate::syntax::HighlightConfiguration::new(
            language,
            &std::fs::read_to_string(
                "../helix-syntax/languages/tree-sitter-rust/queries/highlights.scm",
            )
            .unwrap(),
            &std::fs::read_to_string(
                "../helix-syntax/languages/tree-sitter-rust/queries/injections.scm",
            )
            .unwrap(),
            "", // locals.scm
        )
        .unwrap();

        // TODO: config.configure(scopes) is now delayed, is that ok?

        // TODO: get_language is called twice
        let syntax = Syntax::new(helix_syntax::LANG::Rust, &state.doc, highlight_config);

        state.syntax = Some(syntax);

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
            // TODO: clamp movement to line, prevent moving onto \n at the end
            (Direction::Backward, Granularity::Character) => {
                nth_prev_grapheme_boundary(&text.slice(..), pos, count)
            }
            (Direction::Forward, Granularity::Character) => {
                nth_next_grapheme_boundary(&text.slice(..), pos, count)
            }
            (_, Granularity::Line) => move_vertically(&text.slice(..), dir, pos, count),
            _ => pos,
        }
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
            SelectionRange::new(pos, pos)
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
            SelectionRange::new(range.anchor, pos)
        })
    }
}

/// Coordinates are a 0-indexed line and column pair.
pub type Coords = (usize, usize); // line, col

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
