use std::ops::{Bound, RangeBounds};

pub use regex_cursor::engines::meta::{Builder as RegexBuilder, Regex};
pub use regex_cursor::regex_automata::util::syntax::Config;
use regex_cursor::{Input as RegexInput, RopeyCursor};
use ropey::str_utils::byte_to_char_idx;
use ropey::RopeSlice;

pub trait RopeSliceExt<'a>: Sized {
    fn ends_with(self, text: &str) -> bool;
    fn starts_with(self, text: &str) -> bool;
    fn regex_input(self) -> RegexInput<RopeyCursor<'a>>;
    fn regex_input_at_bytes<R: RangeBounds<usize>>(
        self,
        byte_range: R,
    ) -> RegexInput<RopeyCursor<'a>>;
    fn regex_input_at<R: RangeBounds<usize>>(self, char_range: R) -> RegexInput<RopeyCursor<'a>>;
    fn first_non_whitespace_char(self) -> Option<usize>;
    fn last_non_whitespace_char(self) -> Option<usize>;
    /// returns the char idx of `byte_idx`, if `byte_idx` is a char boundary
    /// this function behaves the same as `byte_to_char` but if `byte_idx` is
    /// not a valid char boundary (so within a char) this will return the next
    /// char index.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("😆");
    /// for i in 1..text.len_bytes() {
    ///     assert_eq!(text.byte_to_char(i), 0);
    ///     assert_eq!(text.byte_to_next_char(i), 1);
    /// }
    /// ```
    fn byte_to_next_char(self, byte_idx: usize) -> usize;
}

impl<'a> RopeSliceExt<'a> for RopeSlice<'a> {
    fn ends_with(self, text: &str) -> bool {
        let len = self.len_bytes();
        if len < text.len() {
            return false;
        }
        self.get_byte_slice(len - text.len()..)
            .map_or(false, |end| end == text)
    }

    fn starts_with(self, text: &str) -> bool {
        let len = self.len_bytes();
        if len < text.len() {
            return false;
        }
        self.get_byte_slice(..text.len())
            .map_or(false, |start| start == text)
    }

    fn regex_input(self) -> RegexInput<RopeyCursor<'a>> {
        RegexInput::new(self)
    }

    fn regex_input_at<R: RangeBounds<usize>>(self, char_range: R) -> RegexInput<RopeyCursor<'a>> {
        let start_bound = match char_range.start_bound() {
            Bound::Included(&val) => Bound::Included(self.char_to_byte(val)),
            Bound::Excluded(&val) => Bound::Excluded(self.char_to_byte(val)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let end_bound = match char_range.end_bound() {
            Bound::Included(&val) => Bound::Included(self.char_to_byte(val)),
            Bound::Excluded(&val) => Bound::Excluded(self.char_to_byte(val)),
            Bound::Unbounded => Bound::Unbounded,
        };
        self.regex_input_at_bytes((start_bound, end_bound))
    }
    fn regex_input_at_bytes<R: RangeBounds<usize>>(
        self,
        byte_range: R,
    ) -> RegexInput<RopeyCursor<'a>> {
        let input = match byte_range.start_bound() {
            Bound::Included(&pos) | Bound::Excluded(&pos) => {
                RegexInput::new(RopeyCursor::at(self, pos))
            }
            Bound::Unbounded => RegexInput::new(self),
        };
        input.range(byte_range)
    }
    fn first_non_whitespace_char(self) -> Option<usize> {
        self.chars().position(|ch| !ch.is_whitespace())
    }
    fn last_non_whitespace_char(self) -> Option<usize> {
        self.chars_at(self.len_chars())
            .reversed()
            .position(|ch| !ch.is_whitespace())
            .map(|pos| self.len_chars() - pos - 1)
    }

    /// returns the char idx of `byte_idx`, if `byte_idx` is
    /// a char boundary this function behaves the same as `byte_to_char`
    fn byte_to_next_char(self, mut byte_idx: usize) -> usize {
        let (chunk, chunk_byte_off, chunk_char_off, _) = self.chunk_at_byte(byte_idx);
        byte_idx -= chunk_byte_off;
        let is_char_boundary =
            is_utf8_char_boundary(chunk.as_bytes().get(byte_idx).copied().unwrap_or(0));
        chunk_char_off + byte_to_char_idx(chunk, byte_idx) + !is_char_boundary as usize
    }
}

// copied from std
#[inline]
const fn is_utf8_char_boundary(b: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (b as i8) >= -0x40
}

#[cfg(test)]
mod tests {
    use ropey::RopeSlice;

    use crate::rope::RopeSliceExt;

    #[test]
    fn next_char_at_byte() {
        for i in 0..=6 {
            assert_eq!(RopeSlice::from("foobar").byte_to_next_char(i), i);
        }
        for char_idx in 0..10 {
            let len = "😆".len();
            assert_eq!(
                RopeSlice::from("😆😆😆😆😆😆😆😆😆😆").byte_to_next_char(char_idx * len),
                char_idx
            );
            for i in 1..=len {
                assert_eq!(
                    RopeSlice::from("😆😆😆😆😆😆😆😆😆😆").byte_to_next_char(char_idx * len + i),
                    char_idx + 1
                );
            }
        }
    }

    #[test]
    fn starts_with() {
        assert!(RopeSlice::from("asdf").starts_with("a"));
    }

    #[test]
    fn ends_with() {
        assert!(RopeSlice::from("asdf").ends_with("f"));
    }
}
