use std::ops::{Bound, RangeBounds};

pub use regex_cursor::engines::meta::{Builder as RegexBuilder, Regex};
pub use regex_cursor::regex_automata::util::syntax::Config;
use regex_cursor::Input as RegexInput;
use ropey::{ChunkCursor, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

pub const LINE_TYPE: ropey::LineType = ropey::LineType::LF_CR;

pub trait RopeSliceExt<'a>: Sized {
    fn ends_with(self, text: &str) -> bool;
    fn starts_with(self, text: &str) -> bool;
    fn regex_input(self) -> RegexInput<ChunkCursor<'a>>;
    fn regex_input_at_bytes<R: RangeBounds<usize>>(
        self,
        byte_range: R,
    ) -> RegexInput<ChunkCursor<'a>>;
    #[deprecated = "use regex_input_at_bytes instead"]
    fn regex_input_at<R: RangeBounds<usize>>(self, char_range: R) -> RegexInput<ChunkCursor<'a>>;
    fn first_non_whitespace_char(self) -> Option<usize>;
    fn last_non_whitespace_char(self) -> Option<usize>;
    /// Finds the closest byte index not exceeding `byte_idx` which lies on a grapheme cluster
    /// boundary.
    ///
    /// If `byte_idx` already lies on a grapheme cluster boundary then it is returned as-is. When
    /// `byte_idx` lies between two grapheme cluster boundaries, this function returns the byte
    /// index of the lesser / earlier / left-hand-side boundary.
    ///
    /// `byte_idx` does not need to be aligned to a character boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::{RopeSlice, Rope};
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("\r\n"); // U+000D U+000A, hex: 0d 0a
    /// let text = text.slice(..);
    /// assert_eq!(text.floor_grapheme_boundary(0), 0);
    /// assert_eq!(text.floor_grapheme_boundary(1), 0);
    /// assert_eq!(text.floor_grapheme_boundary(2), 2);
    /// ```
    fn floor_grapheme_boundary(self, byte_idx: usize) -> usize;
    fn prev_grapheme_boundary(self, byte_idx: usize) -> usize {
        self.nth_prev_grapheme_boundary(byte_idx, 1)
    }
    fn nth_prev_grapheme_boundary(self, byte_idx: usize, n: usize) -> usize;
    /// Finds the closest byte index not exceeding `byte_idx` which lies on a grapheme cluster
    /// boundary.
    ///
    /// If `byte_idx` already lies on a grapheme cluster boundary then it is returned as-is. When
    /// `byte_idx` lies between two grapheme cluster boundaries, this function returns the byte
    /// index of the greater / later / right-hand-side boundary.
    ///
    /// `byte_idx` does not need to be aligned to a character boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::{RopeSlice, Rope};
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("\r\n"); // U+000D U+000A, hex: 0d 0a
    /// let text = text.slice(..);
    /// assert_eq!(text.ceil_grapheme_boundary(0), 0);
    /// assert_eq!(text.ceil_grapheme_boundary(1), 2);
    /// assert_eq!(text.ceil_grapheme_boundary(2), 2);
    /// ```
    fn ceil_grapheme_boundary(self, byte_idx: usize) -> usize;
    fn next_grapheme_boundary(self, byte_idx: usize) -> usize {
        self.nth_next_grapheme_boundary(byte_idx, 1)
    }
    fn nth_next_grapheme_boundary(self, byte_idx: usize, n: usize) -> usize;
    /// Checks whether the `byte_idx` lies on a grapheme cluster boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::{RopeSlice, Rope};
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("\r\n"); // U+000D U+000A, hex: 0d 0a
    /// let text = text.slice(..);
    /// assert!(text.is_grapheme_boundary(0));
    /// assert!(!text.is_grapheme_boundary(1));
    /// assert!(text.is_grapheme_boundary(2));
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_grapheme_boundary(self, byte_idx: usize) -> bool;
    /// Returns an iterator over the grapheme clusters in the slice.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::{RopeSlice, Rope};
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// let graphemes: Vec<_> = text.slice(..).graphemes().collect();
    /// assert_eq!(graphemes.as_slice(), &["😶‍🌫️", "🏴‍☠️", "🖼️"]);
    /// ```
    fn graphemes(self) -> RopeGraphemes<'a>;
    /// Returns an iterator over the grapheme clusters in the slice, reversed.
    ///
    /// The returned iterator starts at the end of the slice and ends at the beginning of the
    /// slice.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::{RopeSlice, Rope};
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// let graphemes: Vec<_> = text.slice(..).graphemes_rev().collect();
    /// assert_eq!(graphemes.as_slice(), &["🖼️", "🏴‍☠️", "😶‍🌫️"]);
    /// ```
    fn graphemes_rev(self) -> RevRopeGraphemes<'a>;
}

impl<'a> RopeSliceExt<'a> for RopeSlice<'a> {
    fn ends_with(self, text: &str) -> bool {
        let len = self.len();
        if len < text.len() {
            return false;
        }
        self.try_slice(len - text.len()..)
            .is_ok_and(|end| end == text)
    }

    fn starts_with(self, text: &str) -> bool {
        let len = self.len();
        if len < text.len() {
            return false;
        }
        self.try_slice(..text.len())
            .is_ok_and(|start| start == text)
    }

    fn regex_input(self) -> RegexInput<ChunkCursor<'a>> {
        RegexInput::new(self)
    }

    fn regex_input_at<R: RangeBounds<usize>>(self, char_range: R) -> RegexInput<ChunkCursor<'a>> {
        let start_bound = match char_range.start_bound() {
            Bound::Included(&val) => Bound::Included(self.char_to_byte_idx(val)),
            Bound::Excluded(&val) => Bound::Excluded(self.char_to_byte_idx(val)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let end_bound = match char_range.end_bound() {
            Bound::Included(&val) => Bound::Included(self.char_to_byte_idx(val)),
            Bound::Excluded(&val) => Bound::Excluded(self.char_to_byte_idx(val)),
            Bound::Unbounded => Bound::Unbounded,
        };
        self.regex_input_at_bytes((start_bound, end_bound))
    }
    fn regex_input_at_bytes<R: RangeBounds<usize>>(
        self,
        byte_range: R,
    ) -> RegexInput<ChunkCursor<'a>> {
        let input = match byte_range.start_bound() {
            Bound::Included(&pos) | Bound::Excluded(&pos) => {
                RegexInput::new(self.chunk_cursor_at(pos))
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

    fn floor_grapheme_boundary(self, mut byte_idx: usize) -> usize {
        if byte_idx >= self.len() {
            return self.len();
        }

        byte_idx = self.ceil_char_boundary(byte_idx + 1);

        let mut chunk_cursor = self.chunk_cursor_at(byte_idx);
        let mut cursor = GraphemeCursor::new(byte_idx, self.len(), true);
        loop {
            match cursor.prev_boundary(chunk_cursor.chunk(), chunk_cursor.byte_offset()) {
                Ok(None) => return 0,
                Ok(Some(boundary)) => return boundary,
                Err(GraphemeIncomplete::PrevChunk) => assert!(chunk_cursor.prev()),
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = self.chunk(n - 1).0;
                    cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }

    fn nth_prev_grapheme_boundary(self, mut byte_idx: usize, n: usize) -> usize {
        byte_idx = self.floor_char_boundary(byte_idx);

        let mut chunk_cursor = self.chunk_cursor_at(byte_idx);
        let mut cursor = GraphemeCursor::new(byte_idx, self.len(), true);
        for _ in 0..n {
            loop {
                match cursor.prev_boundary(chunk_cursor.chunk(), chunk_cursor.byte_offset()) {
                    Ok(None) => return 0,
                    Ok(Some(boundary)) => {
                        byte_idx = boundary;
                        break;
                    }
                    Err(GraphemeIncomplete::PrevChunk) => assert!(chunk_cursor.prev()),
                    Err(GraphemeIncomplete::PreContext(n)) => {
                        let ctx_chunk = self.chunk(n - 1).0;
                        cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                    }
                    _ => unreachable!(),
                }
            }
        }
        byte_idx
    }

    fn ceil_grapheme_boundary(self, mut byte_idx: usize) -> usize {
        if byte_idx >= self.len() {
            return self.len();
        }

        if byte_idx == 0 {
            return 0;
        }

        byte_idx = self.floor_char_boundary(byte_idx - 1);

        let mut chunk_cursor = self.chunk_cursor_at(byte_idx);
        let mut cursor = GraphemeCursor::new(byte_idx, self.len(), true);
        loop {
            match cursor.next_boundary(chunk_cursor.chunk(), chunk_cursor.byte_offset()) {
                Ok(None) => return self.len(),
                Ok(Some(boundary)) => return boundary,
                Err(GraphemeIncomplete::NextChunk) => assert!(chunk_cursor.next()),
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = self.chunk(n - 1).0;
                    cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }

    fn nth_next_grapheme_boundary(self, mut byte_idx: usize, n: usize) -> usize {
        byte_idx = self.ceil_char_boundary(byte_idx);

        let mut chunk_cursor = self.chunk_cursor_at(byte_idx);
        let mut cursor = GraphemeCursor::new(byte_idx, self.len(), true);
        for _ in 0..n {
            loop {
                match cursor.prev_boundary(chunk_cursor.chunk(), chunk_cursor.byte_offset()) {
                    Ok(None) => return 0,
                    Ok(Some(boundary)) => {
                        byte_idx = boundary;
                        break;
                    }
                    Err(GraphemeIncomplete::NextChunk) => assert!(chunk_cursor.next()),
                    Err(GraphemeIncomplete::PreContext(n)) => {
                        let ctx_chunk = self.chunk(n - 1).0;
                        cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                    }
                    _ => unreachable!(),
                }
            }
        }
        byte_idx
    }

    fn is_grapheme_boundary(self, byte_idx: usize) -> bool {
        // The byte must lie on a character boundary to lie on a grapheme cluster boundary.
        if !self.is_char_boundary(byte_idx) {
            return false;
        }

        let (chunk, chunk_byte_idx) = self.chunk(byte_idx);
        let mut cursor = GraphemeCursor::new(byte_idx, self.len(), true);
        loop {
            match cursor.is_boundary(chunk, chunk_byte_idx) {
                Ok(n) => return n,
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let (ctx_chunk, ctx_byte_start) = self.chunk(n - 1);
                    cursor.provide_context(ctx_chunk, ctx_byte_start);
                }
                Err(_) => unreachable!(),
            }
        }
    }

    fn graphemes(self) -> RopeGraphemes<'a> {
        RopeGraphemes {
            chunk_cursor: self.chunk_cursor(),
            text: self,
            cursor: GraphemeCursor::new(0, self.len(), true),
        }
    }

    fn graphemes_rev(self) -> RevRopeGraphemes<'a> {
        RevRopeGraphemes {
            chunk_cursor: self.chunk_cursor_at(self.len()),
            text: self,
            cursor: GraphemeCursor::new(self.len(), self.len(), true),
        }
    }
}

/// An iterator over the graphemes of a `RopeSlice`.
#[derive(Debug, Clone)]
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunk_cursor: ChunkCursor<'a>,
    cursor: GraphemeCursor,
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.chunk_cursor.chunk(), self.chunk_cursor.byte_offset())
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => assert!(self.chunk_cursor.next()),
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx) = self.text.chunk(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.chunk_cursor.byte_offset() {
            Some(self.text.slice(a..b))
        } else {
            let a2 = a - self.chunk_cursor.byte_offset();
            let b2 = b - self.chunk_cursor.byte_offset();
            Some((&self.chunk_cursor.chunk()[a2..b2]).into())
        }
    }
}

/// An iterator over the graphemes of a `RopeSlice` in reverse.
#[derive(Debug, Clone)]
pub struct RevRopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunk_cursor: ChunkCursor<'a>,
    cursor: GraphemeCursor,
}

impl<'a> Iterator for RevRopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .prev_boundary(self.chunk_cursor.chunk(), self.chunk_cursor.byte_offset())
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => assert!(self.chunk_cursor.prev()),
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx) = self.text.chunk(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a >= self.chunk_cursor.byte_offset() + self.chunk_cursor.chunk().len() {
            Some(self.text.slice(b..a))
        } else {
            let a2 = a - self.chunk_cursor.byte_offset();
            let b2 = b - self.chunk_cursor.byte_offset();
            Some((&self.chunk_cursor.chunk()[b2..a2]).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use ropey::RopeSlice;

    use crate::rope::RopeSliceExt;

    #[test]
    fn starts_with() {
        assert!(RopeSlice::from("asdf").starts_with("a"));
    }

    #[test]
    fn ends_with() {
        assert!(RopeSlice::from("asdf").ends_with("f"));
    }

    #[test]
    fn grapheme_boundaries() {
        let ascii = RopeSlice::from("ascii");
        // When the given index lies on a grapheme boundary, the index should not change.
        for byte_idx in 0..=ascii.len() {
            assert_eq!(ascii.floor_char_boundary(byte_idx), byte_idx);
            assert_eq!(ascii.ceil_char_boundary(byte_idx), byte_idx);
            assert!(ascii.is_grapheme_boundary(byte_idx));
        }

        // 🏴‍☠️: U+1F3F4 U+200D U+2620 U+FE0F
        // 13 bytes, hex: f0 9f 8f b4 + e2 80 8d + e2 98 a0 + ef b8 8f
        let g = RopeSlice::from("🏴‍☠️\r\n");
        let emoji_len = "🏴‍☠️".len();
        let end = g.len();

        for byte_idx in 0..emoji_len {
            assert_eq!(g.floor_grapheme_boundary(byte_idx), 0);
        }
        for byte_idx in emoji_len..end {
            assert_eq!(g.floor_grapheme_boundary(byte_idx), emoji_len);
        }
        assert_eq!(g.floor_grapheme_boundary(end), end);

        assert_eq!(g.ceil_grapheme_boundary(0), 0);
        for byte_idx in 1..=emoji_len {
            assert_eq!(g.ceil_grapheme_boundary(byte_idx), emoji_len);
        }
        for byte_idx in emoji_len + 1..=end {
            assert_eq!(g.ceil_grapheme_boundary(byte_idx), end);
        }

        assert!(g.is_grapheme_boundary(0));
        assert!(g.is_grapheme_boundary(emoji_len));
        assert!(g.is_grapheme_boundary(end));
        for byte_idx in (1..emoji_len).chain(emoji_len + 1..end) {
            assert!(!g.is_grapheme_boundary(byte_idx));
        }
    }
}
