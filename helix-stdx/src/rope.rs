use std::ops::{Bound, RangeBounds};

pub use regex_cursor::engines::meta::{Builder as RegexBuilder, Regex};
pub use regex_cursor::regex_automata::util::syntax::Config;
use regex_cursor::{Input as RegexInput, RopeyCursor};
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
        self.get_byte_slice(..len - text.len())
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
}
