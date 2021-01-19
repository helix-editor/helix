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
        Position {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ordering() {
        // (0, 5) is less than (1, 0)
        assert!(Position::new(0, 5) < Position::new(1, 0));
    }
}
