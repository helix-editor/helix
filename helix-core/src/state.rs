use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::{Rope, RopeSlice, Selection, SelectionRange};
use anyhow::Error;

use std::path::PathBuf;

pub enum Mode {
    Normal,
    Insert,
}

/// A state represents the current editor state of a single buffer.
pub struct State {
    /// Path to file on disk.
    pub path: Option<PathBuf>,
    pub doc: Rope,
    pub selection: Selection,
    pub mode: Mode,
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
        }
    }

    pub fn load(path: PathBuf) -> Result<Self, Error> {
        use std::{env, fs::File, io::BufReader, path::PathBuf};
        let _current_dir = env::current_dir()?;

        let doc = Rope::from_reader(BufReader::new(File::open(path.clone())?))?;

        // TODO: create if not found

        let mut state = Self::new(doc);
        state.path = Some(path);

        Ok(state)
    }

    // TODO: doc/selection accessors

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
        sel: Selection,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        // TODO: move all selections according to normal cursor move semantics by collapsing it
        // into cursors and moving them vertically

        let ranges = sel.ranges.into_iter().map(|range| {
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
        });

        Selection::new(ranges.collect(), sel.primary_index)
        // TODO: update selection in state via transaction
    }

    pub fn extend_selection(
        &self,
        sel: Selection,
        dir: Direction,
        granularity: Granularity,
        count: usize,
    ) -> Selection {
        let ranges = sel.ranges.into_iter().map(|range| {
            let pos = self.move_pos(range.head, dir, granularity, count);
            SelectionRange::new(range.anchor, pos)
        });

        Selection::new(ranges.collect(), sel.primary_index)
        // TODO: update selection in state via transaction
    }
}

/// Coordinates are a 0-indexed line and column pair.
type Coords = (usize, usize); // line, col

/// Convert a character index to (line, column) coordinates.
pub fn coords_at_pos(text: &RopeSlice, pos: usize) -> Coords {
    let line = text.char_to_line(pos);
    let line_start = text.line_to_char(line);
    let col = text.slice(line_start..pos).len_chars();
    (line, col)
}

/// Convert (line, column) coordinates to a character index.
pub fn pos_at_coords(text: &RopeSlice, coords: Coords) -> usize {
    let (line, col) = coords;
    let line_start = text.line_to_char(line);
    nth_next_grapheme_boundary(text, line_start, col)
}

fn move_vertically(text: &RopeSlice, dir: Direction, pos: usize, count: usize) -> usize {
    let (line, col) = coords_at_pos(text, pos);

    let new_line = match dir {
        Direction::Backward => line.saturating_sub(count),
        Direction::Forward => std::cmp::min(line.saturating_add(count), text.len_lines() - 1),
    };

    // convert to 0-indexed, subtract another 1 because len_chars() counts \n
    let new_line_len = text.line(new_line).len_chars().saturating_sub(2);

    let new_col = if new_line_len < col {
        // TODO: preserve horiz here
        new_line_len
    } else {
        col
    };

    pos_at_coords(text, (new_line, new_col))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_coords_at_pos() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        assert_eq!(coords_at_pos(&text.slice(..), 0), (0, 0));
        // TODO: what is the coordinate of newline?
        assert_eq!(coords_at_pos(&text.slice(..), 5), (0, 5)); // position on \n
        assert_eq!(coords_at_pos(&text.slice(..), 6), (1, 0)); // position on w
        assert_eq!(coords_at_pos(&text.slice(..), 7), (1, 1)); // position on o
        assert_eq!(coords_at_pos(&text.slice(..), 10), (1, 4)); // position on d
    }

    #[test]
    fn test_pos_at_coords() {
        let text = Rope::from("ḧëḷḷö\nẅöṛḷḋ");
        assert_eq!(pos_at_coords(&text.slice(..), (0, 0)), 0);
        assert_eq!(pos_at_coords(&text.slice(..), (0, 5)), 5); // position on \n
        assert_eq!(pos_at_coords(&text.slice(..), (1, 0)), 6); // position on w
        assert_eq!(pos_at_coords(&text.slice(..), (1, 1)), 7); // position on o
        assert_eq!(pos_at_coords(&text.slice(..), (1, 4)), 10); // position on d
    }

    #[test]
    fn test_vertical_move() {
        let text = Rope::from("abcd\nefg\nwrs");
        let pos = pos_at_coords(&text.slice(..), (0, 4));
        let slice = text.slice(..);

        assert_eq!(
            coords_at_pos(&slice, move_vertically(&slice, Direction::Forward, pos, 1)),
            (1, 2)
        );
    }
}
