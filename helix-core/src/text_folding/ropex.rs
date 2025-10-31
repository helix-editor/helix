//! Implements fold-oriented methods for `RopeSlice`.

use crate::ropey::iter::{Chars, Lines};
use crate::RopeSlice;

use helix_stdx::rope::{RopeGraphemes, RopeSliceExt};

use super::FoldAnnotations;

pub trait RopeSliceFoldExt<'a> {
    /// Similar to the native `chars` method.
    fn folded_chars(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedChars<'a>;

    /// Similar to the extended `graphemes` method in the `RopeSliceExt` trait.
    fn folded_graphemes(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedGraphemes<'a>;

    /// Similar to the native `lines` method.
    fn folded_lines(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedLines<'a>;

    /// Similar to the native `chars_at` method.
    fn folded_chars_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
    ) -> FoldedChars<'a>;

    /// Similar to the extended `graphemes_at` method in the `RopeSliceExt` trait.
    fn folded_graphemes_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        byte_idx: usize,
    ) -> FoldedGraphemes<'a>;

    /// Similar to the native `lines_at` method.
    fn folded_lines_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
    ) -> FoldedLines<'a>;

    fn next_folded_char(&self, annotations: &'a FoldAnnotations<'a>, char_idx: usize) -> usize;
    fn next_folded_grapheme(&self, annotations: &'a FoldAnnotations<'a>, byte_idx: usize) -> usize;
    fn next_folded_line(&self, annotations: &'a FoldAnnotations<'a>, line_idx: usize) -> usize;
    fn prev_folded_char(&self, annotations: &'a FoldAnnotations<'a>, char_idx: usize) -> usize;
    fn prev_folded_grapheme(&self, annotations: &'a FoldAnnotations<'a>, byte_idx: usize) -> usize;
    fn prev_folded_line(&self, annotations: &'a FoldAnnotations<'a>, line_idx: usize) -> usize;
    fn nth_next_folded_char(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
        count: usize,
    ) -> usize;
    fn nth_next_folded_grapheme(
        &self,
        annotatins: &'a FoldAnnotations<'a>,
        byte_idx: usize,
        count: usize,
    ) -> usize;
    fn nth_next_folded_line(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
        count: usize,
    ) -> usize;
    fn nth_prev_folded_char(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
        count: usize,
    ) -> usize;
    fn nth_prev_folded_grapheme(
        &self,
        annotatins: &'a FoldAnnotations<'a>,
        byte_idx: usize,
        count: usize,
    ) -> usize;
    fn nth_prev_folded_line(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
        count: usize,
    ) -> usize;
}

impl<'a> RopeSliceFoldExt<'a> for RopeSlice<'a> {
    fn folded_chars(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedChars<'a> {
        FoldedChars {
            inner: FoldedTextItems::new(*self, annotations, 0),
        }
    }

    fn folded_graphemes(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedGraphemes<'a> {
        FoldedGraphemes {
            inner: FoldedTextItems::new(*self, annotations, 0),
        }
    }

    fn folded_lines(&self, annotations: &'a FoldAnnotations<'a>) -> FoldedLines<'a> {
        FoldedLines {
            inner: FoldedTextItems::new(*self, annotations, 0),
        }
    }

    fn folded_chars_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
    ) -> FoldedChars<'a> {
        FoldedChars {
            inner: FoldedTextItems::new(*self, annotations, char_idx),
        }
    }

    fn folded_graphemes_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        byte_idx: usize,
    ) -> FoldedGraphemes<'a> {
        FoldedGraphemes {
            inner: FoldedTextItems::new(*self, annotations, byte_idx),
        }
    }

    fn folded_lines_at(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
    ) -> FoldedLines<'a> {
        FoldedLines {
            inner: FoldedTextItems::new(*self, annotations, line_idx),
        }
    }

    fn next_folded_char(&self, annotations: &'a FoldAnnotations<'a>, char_idx: usize) -> usize {
        self.nth_next_folded_char(annotations, char_idx, 1)
    }

    fn next_folded_grapheme(&self, annotations: &'a FoldAnnotations<'a>, byte_idx: usize) -> usize {
        self.nth_next_folded_grapheme(annotations, byte_idx, 1)
    }

    fn next_folded_line(&self, annotations: &'a FoldAnnotations<'a>, line_idx: usize) -> usize {
        self.nth_next_folded_line(annotations, line_idx, 1)
    }

    fn prev_folded_char(&self, annotations: &'a FoldAnnotations<'a>, char_idx: usize) -> usize {
        self.nth_prev_folded_char(annotations, char_idx, 1)
    }

    fn prev_folded_grapheme(&self, annotations: &'a FoldAnnotations<'a>, byte_idx: usize) -> usize {
        self.nth_prev_folded_grapheme(annotations, byte_idx, 1)
    }

    fn prev_folded_line(&self, annotations: &'a FoldAnnotations<'a>, line_idx: usize) -> usize {
        self.nth_prev_folded_line(annotations, line_idx, 1)
    }

    fn nth_next_folded_char(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
        mut count: usize,
    ) -> usize {
        if count == 0 {
            return char_idx;
        }

        let mut chars = self.folded_chars_at(annotations, char_idx);

        // consume the initial char
        chars.next();

        // the initial char can be folded
        if chars.last_idx().unwrap_or(self.len_chars()) != char_idx {
            count -= 1;
        }

        chars.by_ref().take(count).for_each(|_| ());
        chars
            .last_idx()
            .unwrap_or(self.len_chars().saturating_sub(1))
    }

    fn nth_next_folded_grapheme(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        byte_idx: usize,
        mut count: usize,
    ) -> usize {
        if count == 0 {
            return byte_idx;
        }

        let mut graphemes = self.folded_graphemes_at(annotations, byte_idx);

        // consume the initial grapheme
        graphemes.next();

        // the initial grapheme can be folded
        if graphemes.last_idx().unwrap_or(self.len_bytes()) != byte_idx {
            count -= 1;
        }

        graphemes.by_ref().take(count).for_each(|_| ());
        graphemes
            .last_idx()
            .unwrap_or(self.len_bytes().saturating_sub(1))
    }

    fn nth_next_folded_line(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
        mut count: usize,
    ) -> usize {
        if count == 0 {
            return line_idx;
        }

        let mut lines = self.folded_lines_at(annotations, line_idx);

        // consume the initial line
        lines.next();

        // the initial line can be folded
        if lines.last_idx().unwrap_or(self.len_lines()) != line_idx {
            count -= 1;
        }

        lines.by_ref().take(count).for_each(|_| ());
        lines
            .last_idx()
            .unwrap_or(self.len_lines().saturating_sub(1))
    }

    fn nth_prev_folded_char(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        char_idx: usize,
        count: usize,
    ) -> usize {
        if count == 0 {
            return char_idx;
        }

        let mut chars = self.folded_chars_at(annotations, char_idx).reversed();

        chars.by_ref().take(count).for_each(|_| ());
        chars.last_idx().unwrap_or(0)
    }

    fn nth_prev_folded_grapheme(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        byte_idx: usize,
        count: usize,
    ) -> usize {
        if count == 0 {
            return byte_idx;
        }

        let mut graphemes = self.folded_graphemes_at(annotations, byte_idx).reversed();

        graphemes.by_ref().take(count).for_each(|_| ());
        graphemes.last_idx().unwrap_or(0)
    }

    fn nth_prev_folded_line(
        &self,
        annotations: &'a FoldAnnotations<'a>,
        line_idx: usize,
        count: usize,
    ) -> usize {
        if count == 0 {
            return line_idx;
        }

        let mut lines = self.folded_lines_at(annotations, line_idx).reversed();

        lines.by_ref().take(count).for_each(|_| ());
        lines.last_idx().unwrap_or(0)
    }
}

macro_rules! FoldedWrapper {
    ($Name:ident, $TextItems:ident) => {
        pub struct $Name<'a> {
            inner: FoldedTextItems<'a, $TextItems<'a>>,
        }

        impl<'a> $Name<'a> {
            pub fn reverse(&mut self) {
                self.inner.is_reversed = !self.inner.is_reversed;
            }

            pub fn reversed(mut self) -> Self {
                self.reverse();
                self
            }

            pub fn prev(&mut self) -> Option<<Self as Iterator>::Item> {
                self.inner.prev()
            }

            pub fn last_idx(&self) -> Option<usize> {
                self.inner.last_idx
            }
        }

        impl<'a> Iterator for $Name<'a> {
            type Item = <$TextItems<'a> as Iterator>::Item;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }
        }
    };
}

FoldedWrapper!(FoldedChars, Chars);
FoldedWrapper!(FoldedGraphemes, RopeGraphemes);
FoldedWrapper!(FoldedLines, Lines);

struct FoldedTextItems<'a, Items> {
    items: Items,
    slice: RopeSlice<'a>,
    annotations: &'a FoldAnnotations<'a>,
    idx: usize,
    last_idx: Option<usize>,
    is_reversed: bool,
}

impl<'a, Items: TextItems<'a>> FoldedTextItems<'a, Items> {
    fn new(slice: RopeSlice<'a>, annotations: &'a FoldAnnotations<'a>, idx: usize) -> Self {
        Items::reset_pos(annotations, idx);
        Self {
            items: Items::at(slice, idx),
            slice,
            annotations,
            idx,
            last_idx: None,
            is_reversed: false,
        }
    }

    #[inline(always)]
    fn prev(&mut self) -> Option<Items::Item> {
        if !self.is_reversed {
            self.prev_impl()
        } else {
            self.next_impl()
        }
    }

    fn prev_impl(&mut self) -> Option<Items::Item> {
        if self.idx == 0 {
            self.last_idx = None;
            return None;
        }

        self.idx -= 1;
        if let Some(position) = Items::consume_prev(self.annotations, self.idx) {
            self.idx = position;
            self.items = Items::at(self.slice, self.idx);

            return self.prev_impl();
        }

        self.last_idx = Some(self.idx);

        Some(
            self.items
                .prev_impl()
                .expect("The `idx` field must equal the item index."),
        )
    }

    fn next_impl(&mut self) -> Option<Items::Item> {
        if self.idx == Items::len(self.slice) {
            self.last_idx = None;
            return None;
        }

        if let Some(position) = Items::consume_next(self.annotations, self.idx) {
            self.idx = position + 1;
            self.items = Items::at(self.slice, self.idx);

            return self.next_impl();
        }

        self.last_idx = Some(self.idx);

        let result = self
            .items
            .next_impl()
            .expect("The `idx` field must equal the item index.");
        self.idx += 1;

        Some(result)
    }
}

impl<'a, Items: TextItems<'a>> Iterator for FoldedTextItems<'a, Items> {
    type Item = Items::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_reversed {
            self.next_impl()
        } else {
            self.prev_impl()
        }
    }
}

trait TextItems<'a>: Iterator {
    fn at(slice: RopeSlice<'a>, idx: usize) -> Self;
    fn reset_pos(annotations: &FoldAnnotations, idx: usize);
    fn len(slice: RopeSlice) -> usize;
    fn prev_impl(&mut self) -> Option<Self::Item>;
    fn next_impl(&mut self) -> Option<Self::Item>;
    fn consume_prev(annotations: &FoldAnnotations, idx: usize) -> Option<usize>;
    fn consume_next(annotations: &FoldAnnotations, idx: usize) -> Option<usize>;
}

impl<'a> TextItems<'a> for Chars<'a> {
    fn at(slice: RopeSlice<'a>, char_idx: usize) -> Self {
        slice.chars_at(char_idx)
    }

    fn reset_pos(annotations: &FoldAnnotations, char_idx: usize) {
        annotations.reset_pos(char_idx, |fold| fold.start.char)
    }

    fn len(slice: RopeSlice) -> usize {
        slice.len_chars()
    }

    fn prev_impl(&mut self) -> Option<Self::Item> {
        self.prev()
    }

    fn next_impl(&mut self) -> Option<Self::Item> {
        self.next()
    }

    fn consume_prev(annotations: &FoldAnnotations, char_idx: usize) -> Option<usize> {
        annotations
            .consume_prev(char_idx, |fold| fold.end.char)
            .map(|fold| fold.start.char)
    }

    fn consume_next(annotations: &FoldAnnotations, char_idx: usize) -> Option<usize> {
        annotations
            .consume_next(char_idx, |fold| fold.start.char)
            .map(|fold| fold.end.char)
    }
}

impl<'a> TextItems<'a> for RopeGraphemes<'a> {
    fn at(slice: RopeSlice<'a>, byte_idx: usize) -> Self {
        slice.graphemes_at(byte_idx)
    }

    fn reset_pos(annotations: &FoldAnnotations, byte_idx: usize) {
        annotations.reset_pos(byte_idx, |fold| fold.start.byte)
    }

    fn len(slice: RopeSlice) -> usize {
        slice.len_bytes()
    }

    fn prev_impl(&mut self) -> Option<Self::Item> {
        self.prev()
    }

    fn next_impl(&mut self) -> Option<Self::Item> {
        self.next()
    }

    fn consume_prev(annotations: &FoldAnnotations, byte_idx: usize) -> Option<usize> {
        annotations
            .consume_prev(byte_idx, |fold| fold.end.byte)
            .map(|fold| fold.start.byte)
    }

    fn consume_next(annotations: &FoldAnnotations, byte_idx: usize) -> Option<usize> {
        annotations
            .consume_next(byte_idx, |fold| fold.start.byte)
            .map(|fold| fold.end.byte)
    }
}

impl<'a> TextItems<'a> for Lines<'a> {
    fn at(slice: RopeSlice<'a>, line_idx: usize) -> Self {
        slice.lines_at(line_idx)
    }

    fn reset_pos(annotations: &FoldAnnotations, line_idx: usize) {
        annotations.reset_pos(line_idx, |fold| fold.start.line)
    }

    fn len(slice: RopeSlice) -> usize {
        slice.len_lines()
    }

    fn prev_impl(&mut self) -> Option<Self::Item> {
        self.prev()
    }

    fn next_impl(&mut self) -> Option<Self::Item> {
        self.next()
    }

    fn consume_prev(annotations: &FoldAnnotations, line_idx: usize) -> Option<usize> {
        annotations
            .consume_prev(line_idx, |fold| fold.end.line)
            .map(|fold| fold.start.line)
    }

    fn consume_next(annotations: &FoldAnnotations, line_idx: usize) -> Option<usize> {
        annotations
            .consume_next(line_idx, |fold| fold.start.line)
            .map(|fold| fold.end.line)
    }
}
