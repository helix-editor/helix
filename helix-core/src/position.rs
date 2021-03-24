use crate::{
    graphemes::{nth_next_grapheme_boundary, RopeGraphemes},
    Rope, RopeSlice,
};

/// Represents a single point in a text buffer. Zero indexed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub const fn is_zero(self) -> bool {
        self.row == 0 && self.col == 0
    }

    // TODO: generalize
    pub fn traverse(self, text: &crate::Tendril) -> Self {
        let Self { mut row, mut col } = self;
        // TODO: there should be a better way here
        for ch in text.chars() {
            if ch == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Self { row, col }
    }
}

impl From<(usize, usize)> for Position {
    fn from(tuple: (usize, usize)) -> Self {
        Self {
            row: tuple.0,
            col: tuple.1,
        }
    }
}

impl From<Position> for tree_sitter::Point {
    fn from(pos: Position) -> Self {
        Self::new(pos.row, pos.col)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ordering() {
        // (0, 5) is less than (1, 0)
        assert!(Position::new(0, 5) < Position::new(1, 0));
    }

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
}
