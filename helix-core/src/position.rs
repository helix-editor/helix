/// Represents a single point in a text buffer. Zero indexed.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn is_zero(self) -> bool {
        self.row == 0 && self.col == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ordering() {
        // (0, 5) is less than (1, 0 w v f)
        assert!(Position::new(0, 5) < Position::new(1, 0));
    }
}
