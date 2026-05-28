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

    if pos > text.len() {
        return None;
    }
    let mut chars = text.chars_at(pos);

    match direction {
        Direction::Forward => loop {
            let c = chars.next()?;
            if char_matcher.char_match(c) {
                n -= 1;
                if n == 0 {
                    return Some(pos);
                }
            }
            pos += c.len_utf8();
        },
        Direction::Backward => loop {
            let c = chars.prev()?;
            pos -= c.len_utf8();
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
        // Bytes: a(0) a(1) ' '(2) ⌚(3..6) a(6) a(7) ' '(8) \r(9) \n(10) ' '(11) a(12) a(13)
        let text = RopeSlice::from("aa ⌚aa \r\n aa");

        // Forward direction (start at byte 7 — the second 'a' after ⌚)
        assert_eq!(find_nth_char(1, text, 'a', 7, Direction::Forward), Some(7));
        assert_eq!(find_nth_char(2, text, 'a', 7, Direction::Forward), Some(12));
        assert_eq!(find_nth_char(3, text, 'a', 7, Direction::Forward), Some(13));
        assert_eq!(find_nth_char(4, text, 'a', 7, Direction::Forward), None);

        // Backward direction
        assert_eq!(find_nth_char(1, text, 'a', 7, Direction::Backward), Some(6));
        assert_eq!(find_nth_char(2, text, 'a', 7, Direction::Backward), Some(1));
        assert_eq!(find_nth_char(3, text, 'a', 7, Direction::Backward), Some(0));
        assert_eq!(find_nth_char(4, text, 'a', 7, Direction::Backward), None);

        // Edge cases
        assert_eq!(find_nth_char(0, text, 'a', 7, Direction::Forward), None); // n = 0
        assert_eq!(find_nth_char(1, text, 'x', 7, Direction::Forward), None); // Not found
        assert_eq!(find_nth_char(1, text, 'a', 20, Direction::Forward), None); // Beyond text
        assert_eq!(find_nth_char(1, text, 'a', 0, Direction::Backward), None); // At start going backward
    }

    /// Regression for byte/char confusion in `find_nth_char`: stepping must advance
    /// by `len_utf8`, not 1, otherwise the returned byte index falls inside a
    /// multi-byte codepoint.
    #[test]
    fn test_find_nth_char_multibyte() {
        // "ëaëaë" — ë is U+00EB (2 bytes). Byte layout:
        //   ë(0..2) a(2) ë(3..5) a(5) ë(6..8)
        let text = RopeSlice::from("ëaëaë");
        assert_eq!(text.len(), 8);

        // Forward from start: 'a' at byte 2, then byte 5.
        assert_eq!(find_nth_char(1, text, 'a', 0, Direction::Forward), Some(2));
        assert_eq!(find_nth_char(2, text, 'a', 0, Direction::Forward), Some(5));

        // 'ë' at bytes 0, 3, 6.
        assert_eq!(find_nth_char(1, text, 'ë', 0, Direction::Forward), Some(0));
        assert_eq!(find_nth_char(2, text, 'ë', 0, Direction::Forward), Some(3));
        assert_eq!(find_nth_char(3, text, 'ë', 0, Direction::Forward), Some(6));

        // Backward from end: 'a' at byte 5, then byte 2.
        assert_eq!(find_nth_char(1, text, 'a', 8, Direction::Backward), Some(5));
        assert_eq!(find_nth_char(2, text, 'a', 8, Direction::Backward), Some(2));

        // Backward from end: 'ë' at bytes 6, 3, 0.
        assert_eq!(find_nth_char(1, text, 'ë', 8, Direction::Backward), Some(6));
        assert_eq!(find_nth_char(2, text, 'ë', 8, Direction::Backward), Some(3));
        assert_eq!(find_nth_char(3, text, 'ë', 8, Direction::Backward), Some(0));
    }
}
