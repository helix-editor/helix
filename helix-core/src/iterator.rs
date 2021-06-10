use std::iter::SkipWhile;

use ropey::RopeSlice;

use crate::movement::{categorize, is_end_of_line};

/// Returns a forward and backwards iterator over (usize, char), where the first element
/// always corresponds to the absolute index of the character in the slice.
pub fn enumerated_chars<'a>(
    slice: &'a RopeSlice,
    index: usize,
) -> impl Iterator<Item = (usize, char)> + 'a + Clone {
    // Single call to the API to ensure everything after is a cheap clone.
    let chars = slice.chars_at(index);
    (index..).zip(chars.clone())
}

pub fn backwards_enumerated_chars<'a>(
    slice: &'a RopeSlice,
    index: usize,
) -> impl Iterator<Item = (usize, char)> + 'a + Clone {
    // Single call to the API to ensure everything after is a cheap clone.
    let mut chars = slice.chars_at(index);
    chars.next();
    (0..=index).rev().zip(std::iter::from_fn(move || chars.prev()))
}

// Helper functions for iterators over (usize, char) tuples
// (necessary to iterate over ropes efficiently while retaining
// the index).
pub trait EnumeratedChars: Iterator<Item = (usize, char)> {
    //Returns the index at the current [word/punctuation + whitespace] group
    fn end_of_block(&mut self) -> Option<usize>;
    fn end_of_word(&mut self) -> Option<usize>;
    fn current_position(&mut self) -> Option<usize>;
    fn last_position(&mut self) -> Option<usize>;
    fn at_boundary(&mut self) -> bool;
}

pub trait NewlineTraversal: Sized {
    fn skip_newlines(&mut self) -> SkipWhile<&mut Self, NewlineCheck>;
}

pub type NewlineCheck = for<'r> fn(&'r (usize, char)) -> bool;

impl<I: Clone + Iterator<Item = (usize, char)>> EnumeratedChars for I {
    fn end_of_block(&mut self) -> Option<usize> {
        let after_newline = self.clone().skip_while(|(pos, c)| is_end_of_line(*c));
        let after_newline_zip = self.skip_while(|(pos, c)| is_end_of_line(*c)).skip(1);
        let mut pairs = after_newline.zip(after_newline_zip);
        pairs
            .find_map(|((a_pos, a), (_, b))| {
                ((categorize(a) != categorize(b)) && (is_end_of_line(b) || !b.is_whitespace()))
                    .then(|| a_pos)
            })
            .or_else(|| self.last_position())
    }

    fn end_of_word(&mut self) -> Option<usize> {
        let after_newline = self.clone().skip_while(|(_, c)| is_end_of_line(*c));
        let mut pairs = after_newline.clone().zip(after_newline.skip(1));
        pairs
            .find_map(|((a_pos, a), (_, b))| {
                ((categorize(a) != categorize(b)) && (!a.is_whitespace() || is_end_of_line(b)))
                    .then(|| a_pos)
            })
            .or_else(|| self.last_position())
    }

    fn last_position(&mut self) -> Option<usize> {
        self.last().map(|(pos, _)| pos)
    }

    fn current_position(&mut self) -> Option<usize> {
        self.next().map(|(pos, _)| pos)
    }

    fn at_boundary(&mut self) -> bool {
        matches!(
            (self.next(), self.next()),
            (Some((_, a)), Some((_, b))) if categorize(a) != categorize(b)
        )
    }
}

impl<I: Clone + Iterator<Item = (usize, char)>> NewlineTraversal for I {
    fn skip_newlines(&mut self) -> SkipWhile<&mut Self, NewlineCheck> {
        self.skip_while(|(_, c)| is_end_of_line(*c))
    }
}

