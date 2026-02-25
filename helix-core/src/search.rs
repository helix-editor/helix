use crate::movement::Direction;
use crate::RopeSlice;

// TODO: switch to std::str::Pattern when it is stable.
pub trait CharMatcher {
    fn char_match(&self, ch: char) -> bool;
}

impl CharMatcher for char {
    fn char_match(&self, ch: char) -> bool {
        *self == ch
    }
}

impl<F: Fn(&char) -> bool> CharMatcher for F {
    fn char_match(&self, ch: char) -> bool {
        (*self)(&ch)
    }
}

// Finds the positions of the nth matching character in given direction
// starting from the pos gap-index (see Range struct for explanation)
pub fn find_nth_char<M: CharMatcher>(
    mut n: usize,
    text: RopeSlice,
    char_matcher: M,
    mut pos: usize,
    direction: Direction,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut chars = text.get_chars_at(pos)?;

    match direction {
        Direction::Forward => loop {
            let c = chars.next()?;
            if char_matcher.char_match(c) {
                n -= 1;
                if n == 0 {
                    return Some(pos);
                }
            }
            pos += 1;
        },
        Direction::Backward => loop {
            let c = chars.prev()?;
            pos -= 1;
            if char_matcher.char_match(c) {
                n -= 1;
                if n == 0 {
                    return Some(pos);
                }
            }
        },
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::movement::Direction;

    #[test]
    fn test_find_nth_char() {
        let text = RopeSlice::from("aa ⌚aa \r\n aa");

        // Forward direction
        assert_eq!(find_nth_char(1, text, 'a', 5, Direction::Forward), Some(5));
        assert_eq!(find_nth_char(2, text, 'a', 5, Direction::Forward), Some(10));
        assert_eq!(find_nth_char(3, text, 'a', 5, Direction::Forward), Some(11));
        assert_eq!(find_nth_char(4, text, 'a', 5, Direction::Forward), None);

        // Backward direction
        assert_eq!(find_nth_char(1, text, 'a', 5, Direction::Backward), Some(4));
        assert_eq!(find_nth_char(2, text, 'a', 5, Direction::Backward), Some(1));
        assert_eq!(find_nth_char(3, text, 'a', 5, Direction::Backward), Some(0));
        assert_eq!(find_nth_char(4, text, 'a', 5, Direction::Backward), None);

        // Edge cases
        assert_eq!(find_nth_char(0, text, 'a', 5, Direction::Forward), None); // n = 0
        assert_eq!(find_nth_char(1, text, 'x', 5, Direction::Forward), None); // Not found
        assert_eq!(find_nth_char(1, text, 'a', 20, Direction::Forward), None); // Beyond text
        assert_eq!(find_nth_char(1, text, 'a', 0, Direction::Backward), None); // At start going backward
    }
}
