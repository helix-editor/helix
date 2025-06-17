//! Functions and types for working with [RopeSlice]
use std::fmt;
use std::ops::{Bound, RangeBounds};

pub use regex_cursor::engines::meta::{Builder as RegexBuilder, Regex};
pub use regex_cursor::regex_automata::util::syntax::Config;
use regex_cursor::{Input as RegexInput, RopeyCursor};
use ropey::iter::Chunks;
use ropey::RopeSlice;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// Additional utility functions for [RopeSlice]
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
    /// Finds the closest byte index not exceeding `byte_idx` which lies on a character boundary.
    ///
    /// If `byte_idx` already lies on a character boundary then it is returned as-is. When
    /// `byte_idx` lies between two character boundaries, this function returns the byte index of
    /// the lesser / earlier / left-hand-side boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("⌚"); // three bytes: e2 8c 9a
    /// assert_eq!(text.floor_char_boundary(0), 0);
    /// assert_eq!(text.floor_char_boundary(1), 0);
    /// assert_eq!(text.floor_char_boundary(2), 0);
    /// assert_eq!(text.floor_char_boundary(3), 3);
    /// ```
    fn floor_char_boundary(self, byte_idx: usize) -> usize;
    /// Finds the closest byte index not below `byte_idx` which lies on a character boundary.
    ///
    /// If `byte_idx` already lies on a character boundary then it is returned as-is. When
    /// `byte_idx` lies between two character boundaries, this function returns the byte index of
    /// the greater / later / right-hand-side boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("⌚"); // three bytes: e2 8c 9a
    /// assert_eq!(text.ceil_char_boundary(0), 0);
    /// assert_eq!(text.ceil_char_boundary(1), 3);
    /// assert_eq!(text.ceil_char_boundary(2), 3);
    /// assert_eq!(text.ceil_char_boundary(3), 3);
    /// ```
    fn ceil_char_boundary(self, byte_idx: usize) -> usize;
    /// Checks whether the given `byte_idx` lies on a character boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("⌚"); // three bytes: e2 8c 9a
    /// assert!(text.is_char_boundary(0));
    /// assert!(!text.is_char_boundary(1));
    /// assert!(!text.is_char_boundary(2));
    /// assert!(text.is_char_boundary(3));
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn is_char_boundary(self, byte_idx: usize) -> bool;
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
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("\r\n"); // U+000D U+000A, hex: 0d 0a
    /// assert_eq!(text.floor_grapheme_boundary(0), 0);
    /// assert_eq!(text.floor_grapheme_boundary(1), 0);
    /// assert_eq!(text.floor_grapheme_boundary(2), 2);
    /// ```
    fn floor_grapheme_boundary(self, byte_idx: usize) -> usize;
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
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("\r\n"); // U+000D U+000A, hex: 0d 0a
    /// assert_eq!(text.ceil_grapheme_boundary(0), 0);
    /// assert_eq!(text.ceil_grapheme_boundary(1), 2);
    /// assert_eq!(text.ceil_grapheme_boundary(2), 2);
    /// ```
    fn ceil_grapheme_boundary(self, byte_idx: usize) -> usize;
    /// Checks whether the `byte_idx` lies on a grapheme cluster boundary.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("\r\n"); // U+000D U+000A, hex: 0d 0a
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
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("😶‍🌫️🏴‍☠️🖼️");
    /// let graphemes: Vec<_> = text.graphemes().collect();
    /// assert_eq!(graphemes.as_slice(), &["😶‍🌫️", "🏴‍☠️", "🖼️"]);
    /// ```
    fn graphemes(self) -> RopeGraphemes<'a> {
        self.graphemes_at(0)
    }
    /// Returns an iterator over the grapheme clusters in the slice, reversed.
    ///
    /// The returned iterator starts at the end of the slice and ends at the beginning of the
    /// slice.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::RopeSlice;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = RopeSlice::from("😶‍🌫️🏴‍☠️🖼️");
    /// let graphemes: Vec<_> = text.graphemes_rev().collect();
    /// assert_eq!(graphemes.as_slice(), &["🖼️", "🏴‍☠️", "😶‍🌫️"]);
    /// ```
    fn graphemes_rev(self) -> RopeGraphemes<'a>;
    /// Returns an iterator over the grapheme clusters in the slice at the given byte index.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::Rope;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// // 14 is the byte index of the pirate flag's starting cluster boundary.
    /// let graphemes: Vec<_> = text.slice(..).graphemes_at(14).collect();
    /// assert_eq!(graphemes.as_slice(), &["🏴‍☠️", "🖼️"]);
    /// // 27 is the byte index of the pirate flag's ending cluster boundary.
    /// let graphemes: Vec<_> = text.slice(..).graphemes_at(27).reversed().collect();
    /// assert_eq!(graphemes.as_slice(), &["🏴‍☠️", "😶‍🌫️"]);
    /// ```
    fn graphemes_at(self, byte_idx: usize) -> RopeGraphemes<'a>;
    /// Returns an iterator over the grapheme clusters in a rope and the byte index where each
    /// grapheme cluster starts.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::Rope;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// let slice = text.slice(..);
    /// let graphemes: Vec<_> = slice.grapheme_indices_at(0).collect();
    /// assert_eq!(
    ///   graphemes.as_slice(),
    ///   &[(0, "😶‍🌫️".into()), (14, "🏴‍☠️".into()), (27, "🖼️".into())]
    /// );
    /// let graphemes: Vec<_> = slice.grapheme_indices_at(slice.len_bytes()).reversed().collect();
    /// assert_eq!(
    ///   graphemes.as_slice(),
    ///   &[(27, "🖼️".into()), (14, "🏴‍☠️".into()), (0, "😶‍🌫️".into())]
    /// );
    /// ```
    fn grapheme_indices_at(self, byte_idx: usize) -> RopeGraphemeIndices<'a>;
    /// Finds the byte index of the next grapheme boundary after `byte_idx`.
    ///
    /// If the byte index lies on the last grapheme cluster in the slice then this function
    /// returns `RopeSlice::len_bytes`.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::Rope;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// let slice = text.slice(..);
    /// let mut byte_idx = 0;
    /// assert_eq!(slice.graphemes_at(byte_idx).next(), Some("😶‍🌫️".into()));
    /// byte_idx = slice.next_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).next(), Some("🏴‍☠️".into()));
    ///
    /// // If `byte_idx` does not lie on a character or grapheme boundary then this function is
    /// // functionally the same as `ceil_grapheme_boundary`.
    /// assert_eq!(slice.next_grapheme_boundary(byte_idx - 1), byte_idx);
    /// assert_eq!(slice.next_grapheme_boundary(byte_idx - 2), byte_idx);
    /// assert_eq!(slice.next_grapheme_boundary(byte_idx + 1), slice.next_grapheme_boundary(byte_idx));
    /// assert_eq!(slice.next_grapheme_boundary(byte_idx + 2), slice.next_grapheme_boundary(byte_idx));
    ///
    /// byte_idx = slice.next_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).next(), Some("🖼️".into()));
    /// byte_idx = slice.next_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).next(), None);
    /// assert_eq!(byte_idx, slice.len_bytes());
    /// ```
    fn next_grapheme_boundary(self, byte_idx: usize) -> usize {
        self.nth_next_grapheme_boundary(byte_idx, 1)
    }
    /// Finds the byte index of the `n`th grapheme cluster after the given `byte_idx`.
    ///
    /// If there are fewer than `n` grapheme clusters after `byte_idx` in the rope then this
    /// function returns `RopeSlice::len_bytes`.
    ///
    /// This is functionally equivalent to calling `next_grapheme_boundary` `n` times but is more
    /// efficient.
    fn nth_next_grapheme_boundary(self, byte_idx: usize, n: usize) -> usize;
    /// Finds the byte index of the previous grapheme boundary before `byte_idx`.
    ///
    /// If the byte index lies on the first grapheme cluster in the slice then this function
    /// returns zero.
    ///
    /// # Example
    ///
    /// ```
    /// # use ropey::Rope;
    /// # use helix_stdx::rope::RopeSliceExt;
    /// let text = Rope::from_str("😶‍🌫️🏴‍☠️🖼️");
    /// let slice = text.slice(..);
    /// let mut byte_idx = text.len_bytes();
    /// assert_eq!(slice.graphemes_at(byte_idx).prev(), Some("🖼️".into()));
    /// byte_idx = slice.prev_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).prev(), Some("🏴‍☠️".into()));
    ///
    /// // If `byte_idx` does not lie on a character or grapheme boundary then this function is
    /// // functionally the same as `floor_grapheme_boundary`.
    /// assert_eq!(slice.prev_grapheme_boundary(byte_idx + 1), byte_idx);
    /// assert_eq!(slice.prev_grapheme_boundary(byte_idx + 2), byte_idx);
    /// assert_eq!(slice.prev_grapheme_boundary(byte_idx - 1), slice.prev_grapheme_boundary(byte_idx));
    /// assert_eq!(slice.prev_grapheme_boundary(byte_idx - 2), slice.prev_grapheme_boundary(byte_idx));
    ///
    /// byte_idx = slice.prev_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).prev(), Some("😶‍🌫️".into()));
    /// byte_idx = slice.prev_grapheme_boundary(byte_idx);
    /// assert_eq!(slice.graphemes_at(byte_idx).prev(), None);
    /// assert_eq!(byte_idx, 0);
    /// ```
    fn prev_grapheme_boundary(self, byte_idx: usize) -> usize {
        self.nth_prev_grapheme_boundary(byte_idx, 1)
    }
    /// Finds the byte index of the `n`th grapheme cluster before the given `byte_idx`.
    ///
    /// If there are fewer than `n` grapheme clusters before `byte_idx` in the rope then this
    /// function returns zero.
    ///
    /// This is functionally equivalent to calling `prev_grapheme_boundary` `n` times but is more
    /// efficient.
    fn nth_prev_grapheme_boundary(self, byte_idx: usize, n: usize) -> usize;
}

impl<'a> RopeSliceExt<'a> for RopeSlice<'a> {
    fn ends_with(self, text: &str) -> bool {
        let len = self.len_bytes();
        if len < text.len() {
            return false;
        }
        self.get_byte_slice(len - text.len()..)
            .is_some_and(|end| end == text)
    }

    fn starts_with(self, text: &str) -> bool {
        let len = self.len_bytes();
        if len < text.len() {
            return false;
        }
        self.get_byte_slice(..text.len())
            .is_some_and(|start| start == text)
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

    // These three are adapted from std:

    fn floor_char_boundary(self, byte_idx: usize) -> usize {
        if byte_idx >= self.len_bytes() {
            self.len_bytes()
        } else {
            let offset = self
                .bytes_at(byte_idx + 1)
                .reversed()
                .take(4)
                .position(is_utf8_char_boundary)
                // A char can only be four bytes long so we are guaranteed to find a boundary.
                .unwrap();

            byte_idx - offset
        }
    }

    fn ceil_char_boundary(self, byte_idx: usize) -> usize {
        if byte_idx > self.len_bytes() {
            self.len_bytes()
        } else {
            let upper_bound = self.len_bytes().min(byte_idx + 4);
            self.bytes_at(byte_idx)
                .position(is_utf8_char_boundary)
                .map_or(upper_bound, |pos| pos + byte_idx)
        }
    }

    fn is_char_boundary(self, byte_idx: usize) -> bool {
        if byte_idx == 0 {
            return true;
        }

        if byte_idx >= self.len_bytes() {
            byte_idx == self.len_bytes()
        } else {
            is_utf8_char_boundary(self.bytes_at(byte_idx).next().unwrap())
        }
    }

    fn floor_grapheme_boundary(self, mut byte_idx: usize) -> usize {
        if byte_idx >= self.len_bytes() {
            return self.len_bytes();
        }

        byte_idx = self.ceil_char_boundary(byte_idx + 1);

        let (mut chunk, mut chunk_byte_idx, _, _) = self.chunk_at_byte(byte_idx);

        let mut cursor = GraphemeCursor::new(byte_idx, self.len_bytes(), true);

        loop {
            match cursor.prev_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return 0,
                Ok(Some(boundary)) => return boundary,
                Err(GraphemeIncomplete::PrevChunk) => {
                    let (ch, ch_byte_idx, _, _) = self.chunk_at_byte(chunk_byte_idx - 1);
                    chunk = ch;
                    chunk_byte_idx = ch_byte_idx;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = self.chunk_at_byte(n - 1).0;
                    cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }

    fn ceil_grapheme_boundary(self, mut byte_idx: usize) -> usize {
        if byte_idx >= self.len_bytes() {
            return self.len_bytes();
        }

        if byte_idx == 0 {
            return 0;
        }

        byte_idx = self.floor_char_boundary(byte_idx - 1);

        let (mut chunk, mut chunk_byte_idx, _, _) = self.chunk_at_byte(byte_idx);

        let mut cursor = GraphemeCursor::new(byte_idx, self.len_bytes(), true);

        loop {
            match cursor.next_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return self.len_bytes(),
                Ok(Some(boundary)) => return boundary,
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte_idx += chunk.len();
                    chunk = self.chunk_at_byte(chunk_byte_idx).0;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = self.chunk_at_byte(n - 1).0;
                    cursor.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }

    fn is_grapheme_boundary(self, byte_idx: usize) -> bool {
        // The byte must lie on a character boundary to lie on a grapheme cluster boundary.
        if !self.is_char_boundary(byte_idx) {
            return false;
        }

        let (chunk, chunk_byte_idx, _, _) = self.chunk_at_byte(byte_idx);

        let mut cursor = GraphemeCursor::new(byte_idx, self.len_bytes(), true);

        loop {
            match cursor.is_boundary(chunk, chunk_byte_idx) {
                Ok(n) => return n,
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let (ctx_chunk, ctx_byte_start, _, _) = self.chunk_at_byte(n - 1);
                    cursor.provide_context(ctx_chunk, ctx_byte_start);
                }
                Err(_) => unreachable!(),
            }
        }
    }

    fn graphemes_rev(self) -> RopeGraphemes<'a> {
        self.graphemes_at(self.len_bytes()).reversed()
    }

    fn graphemes_at(self, byte_idx: usize) -> RopeGraphemes<'a> {
        // Bounds check
        assert!(byte_idx <= self.len_bytes());

        let (mut chunks, chunk_byte_idx, _, _) = self.chunks_at_byte(byte_idx);
        let current_chunk = chunks.next().unwrap_or("");

        RopeGraphemes {
            text: self,
            chunks,
            current_chunk,
            chunk_byte_idx,
            cursor: GraphemeCursor::new(byte_idx, self.len_bytes(), true),
            is_reversed: false,
        }
    }

    fn grapheme_indices_at(self, byte_idx: usize) -> RopeGraphemeIndices<'a> {
        // Bounds check
        assert!(byte_idx <= self.len_bytes());
        RopeGraphemeIndices {
            front_offset: byte_idx,
            iter: self.graphemes_at(byte_idx),
            is_reversed: false,
        }
    }

    fn nth_next_grapheme_boundary(self, mut byte_idx: usize, n: usize) -> usize {
        // Bounds check
        assert!(byte_idx <= self.len_bytes());

        byte_idx = self.floor_char_boundary(byte_idx);

        // Get the chunk with our byte index in it.
        let (mut chunk, mut chunk_byte_idx, _, _) = self.chunk_at_byte(byte_idx);

        // Set up the grapheme cursor.
        let mut gc = GraphemeCursor::new(byte_idx, self.len_bytes(), true);

        // Find the nth next grapheme cluster boundary.
        for _ in 0..n {
            loop {
                match gc.next_boundary(chunk, chunk_byte_idx) {
                    Ok(None) => return self.len_bytes(),
                    Ok(Some(boundary)) => {
                        byte_idx = boundary;
                        break;
                    }
                    Err(GraphemeIncomplete::NextChunk) => {
                        chunk_byte_idx += chunk.len();
                        let (a, _, _, _) = self.chunk_at_byte(chunk_byte_idx);
                        chunk = a;
                    }
                    Err(GraphemeIncomplete::PreContext(n)) => {
                        let ctx_chunk = self.chunk_at_byte(n - 1).0;
                        gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                    }
                    _ => unreachable!(),
                }
            }
        }

        byte_idx
    }

    fn nth_prev_grapheme_boundary(self, mut byte_idx: usize, n: usize) -> usize {
        // Bounds check
        assert!(byte_idx <= self.len_bytes());

        byte_idx = self.ceil_char_boundary(byte_idx);

        // Get the chunk with our byte index in it.
        let (mut chunk, mut chunk_byte_idx, _, _) = self.chunk_at_byte(byte_idx);

        // Set up the grapheme cursor.
        let mut gc = GraphemeCursor::new(byte_idx, self.len_bytes(), true);

        for _ in 0..n {
            loop {
                match gc.prev_boundary(chunk, chunk_byte_idx) {
                    Ok(None) => return 0,
                    Ok(Some(boundary)) => {
                        byte_idx = boundary;
                        break;
                    }
                    Err(GraphemeIncomplete::PrevChunk) => {
                        let (a, b, _, _) = self.chunk_at_byte(chunk_byte_idx - 1);
                        chunk = a;
                        chunk_byte_idx = b;
                    }
                    Err(GraphemeIncomplete::PreContext(n)) => {
                        let ctx_chunk = self.chunk_at_byte(n - 1).0;
                        gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                    }
                    _ => unreachable!(),
                }
            }
        }

        byte_idx
    }
}

// copied from std
#[inline]
const fn is_utf8_char_boundary(b: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (b as i8) >= -0x40
}

/// An iterator over the graphemes of a `RopeSlice`.
///
/// This iterator is cursor-like: rather than implementing DoubleEndedIterator it can be reversed
/// like a cursor. This style matches `Bytes` and `Chars` iterator types in Ropey and is more
/// natural and useful for wrapping `GraphemeCursor`.
#[derive(Clone)]
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    current_chunk: &'a str,
    /// Byte index of the start of the current chunk.
    chunk_byte_idx: usize,
    cursor: GraphemeCursor,
    is_reversed: bool,
}

impl fmt::Debug for RopeGraphemes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RopeGraphemes")
            .field("text", &self.text)
            .field("chunks", &self.chunks)
            .field("current_chunk", &self.current_chunk)
            .field("chunk_byte_idx", &self.chunk_byte_idx)
            // .field("cursor", &self.cursor)
            .field("is_reversed", &self.is_reversed)
            .finish()
    }
}

impl<'a> RopeGraphemes<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<RopeSlice<'a>> {
        if self.is_reversed {
            self.prev_impl()
        } else {
            self.next_impl()
        }
    }

    pub fn prev(&mut self) -> Option<RopeSlice<'a>> {
        if self.is_reversed {
            self.next_impl()
        } else {
            self.prev_impl()
        }
    }

    pub fn reverse(&mut self) {
        self.is_reversed = !self.is_reversed;
    }

    #[must_use]
    pub fn reversed(mut self) -> Self {
        self.reverse();
        self
    }

    fn next_impl(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.current_chunk, self.chunk_byte_idx)
            {
                Ok(None) => return None,
                Ok(Some(boundary)) => {
                    b = boundary;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.chunk_byte_idx += self.current_chunk.len();
                    self.current_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.chunk_byte_idx {
            Some(self.text.byte_slice(a..b))
        } else {
            let a2 = a - self.chunk_byte_idx;
            let b2 = b - self.chunk_byte_idx;
            Some((&self.current_chunk[a2..b2]).into())
        }
    }

    fn prev_impl(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .prev_boundary(self.current_chunk, self.chunk_byte_idx)
            {
                Ok(None) => return None,
                Ok(Some(boundary)) => {
                    b = boundary;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    self.current_chunk = self.chunks.prev().unwrap_or("");
                    self.chunk_byte_idx -= self.current_chunk.len();
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a >= self.chunk_byte_idx + self.current_chunk.len() {
            Some(self.text.byte_slice(b..a))
        } else {
            let a2 = a - self.chunk_byte_idx;
            let b2 = b - self.chunk_byte_idx;
            Some((&self.current_chunk[b2..a2]).into())
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        RopeGraphemes::next(self)
    }
}

/// An iterator over the grapheme clusters in a rope and the byte indices where each grapheme
/// cluster starts.
///
/// This iterator wraps `RopeGraphemes` and is also cursor-like. Use `reverse` or `reversed` to
/// toggle the direction of the iterator. See [RopeGraphemes].
#[derive(Debug, Clone)]
pub struct RopeGraphemeIndices<'a> {
    front_offset: usize,
    iter: RopeGraphemes<'a>,
    is_reversed: bool,
}

impl<'a> RopeGraphemeIndices<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<(usize, RopeSlice<'a>)> {
        if self.is_reversed {
            self.prev_impl()
        } else {
            self.next_impl()
        }
    }

    pub fn prev(&mut self) -> Option<(usize, RopeSlice<'a>)> {
        if self.is_reversed {
            self.next_impl()
        } else {
            self.prev_impl()
        }
    }

    pub fn reverse(&mut self) {
        self.is_reversed = !self.is_reversed;
    }

    #[must_use]
    pub fn reversed(mut self) -> Self {
        self.reverse();
        self
    }

    fn next_impl(&mut self) -> Option<(usize, RopeSlice<'a>)> {
        let slice = self.iter.next()?;
        let idx = self.front_offset;
        self.front_offset += slice.len_bytes();
        Some((idx, slice))
    }

    fn prev_impl(&mut self) -> Option<(usize, RopeSlice<'a>)> {
        let slice = self.iter.prev()?;
        self.front_offset -= slice.len_bytes();
        Some((self.front_offset, slice))
    }
}

impl<'a> Iterator for RopeGraphemeIndices<'a> {
    type Item = (usize, RopeSlice<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        RopeGraphemeIndices::next(self)
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
    fn char_boundaries() {
        let ascii = RopeSlice::from("ascii");
        // When the given index lies on a character boundary, the index should not change.
        for byte_idx in 0..=ascii.len_bytes() {
            assert_eq!(ascii.floor_char_boundary(byte_idx), byte_idx);
            assert_eq!(ascii.ceil_char_boundary(byte_idx), byte_idx);
            assert!(ascii.is_char_boundary(byte_idx));
        }

        // This is a polyfill of a method of this trait which was replaced by ceil_char_boundary.
        // It returns the _character index_ of the given byte index, rounding up if it does not
        // already lie on a character boundary.
        fn byte_to_next_char(slice: RopeSlice, byte_idx: usize) -> usize {
            slice.byte_to_char(slice.ceil_char_boundary(byte_idx))
        }

        for i in 0..=6 {
            assert_eq!(byte_to_next_char(RopeSlice::from("foobar"), i), i);
        }
        for char_idx in 0..10 {
            let len = "😆".len();
            assert_eq!(
                byte_to_next_char(RopeSlice::from("😆😆😆😆😆😆😆😆😆😆"), char_idx * len),
                char_idx
            );
            for i in 1..=len {
                assert_eq!(
                    byte_to_next_char(RopeSlice::from("😆😆😆😆😆😆😆😆😆😆"), char_idx * len + i),
                    char_idx + 1
                );
            }
        }
    }

    #[test]
    fn grapheme_boundaries() {
        let ascii = RopeSlice::from("ascii");
        // When the given index lies on a grapheme boundary, the index should not change.
        for byte_idx in 0..=ascii.len_bytes() {
            assert_eq!(ascii.floor_char_boundary(byte_idx), byte_idx);
            assert_eq!(ascii.ceil_char_boundary(byte_idx), byte_idx);
            assert!(ascii.is_grapheme_boundary(byte_idx));
        }

        // 🏴‍☠️: U+1F3F4 U+200D U+2620 U+FE0F
        // 13 bytes, hex: f0 9f 8f b4 + e2 80 8d + e2 98 a0 + ef b8 8f
        let g = RopeSlice::from("🏴‍☠️\r\n");
        let emoji_len = "🏴‍☠️".len();
        let end = g.len_bytes();

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
