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

// Helper functions for iterators over characters
pub trait SpanHelpers: Iterator<Item = char> {
    // Advances until the end of block (word/punctuation + whitespace group) and returns the
    // characters spanned
    fn to_end_of_block(&mut self) -> Vec<char>;
    // Advances until the end of word/punctuation group and returns the characters spanned
    fn to_end_of_word(&mut self) -> Vec<char>;
    //Returns the index at the current [word/punctuation + whitespace] group
    fn at_boundary(&mut self) -> bool;
}

pub type NewlineCheck = for<'r> fn(&'r (usize, char)) -> bool;

impl<I: Iterator<Item = char>> SpanHelpers for I {
    fn to_end_of_block(&mut self) -> Vec<char> {
        // We first extract the head and any newlines, then proceed until a category boundary
        enum Phase { HeadAndNewlines, StartOfBlock, FindBoundary, };
        let mut vec = Vec::<char>::new();
        let mut phase = Phase::HeadAndNewlines;
        let mut characters = self.peekable();

        while let Some(peek) = characters.peek() {
            match phase {
                Phase::HeadAndNewlines => {
                    vec.push(characters.next().unwrap());
                    if !matches!(characters.peek(), Some('\n')) {
                        phase = Phase::StartOfBlock
                    }
                },
                Phase::StartOfBlock => {
                    vec.push(characters.next().unwrap());
                    phase = Phase::FindBoundary;
                }
                Phase::FindBoundary => {
                    let last = vec.last().unwrap();
                    let is_boundary = ((categorize(*last) != categorize(*peek))
                        && (is_end_of_line(*peek) || !peek.is_whitespace()));
                    if is_boundary {
                        break;
                    } else {
                        vec.push(characters.next().unwrap());
                    }
                },
            }
        };
        vec
    }

    fn to_end_of_word(&mut self) -> Vec<char> {
        // We first extract the head and any newlines, then proceed until a word boundary
        enum Phase { HeadAndNewlines, StartOfBlock, FindBoundary, };
        let mut vec = Vec::<char>::new();
        let mut phase = Phase::HeadAndNewlines;
        let mut characters = self.peekable();

        while let Some(peek) = characters.peek() {
            match phase {
                Phase::HeadAndNewlines => {
                    vec.push(characters.next().unwrap());
                    if !matches!(characters.peek(), Some('\n')) {
                        phase = Phase::StartOfBlock
                    }
                },
                Phase::StartOfBlock => {
                    vec.push(characters.next().unwrap());
                    phase = Phase::FindBoundary;
                }
                Phase::FindBoundary => {
                    let last = vec.last().unwrap();
                    let is_boundary = ((categorize(*last) != categorize(*peek))
                                       && (!last.is_whitespace() || is_end_of_line(*peek)));
                    if is_boundary {
                        break;
                    } else {
                        vec.push(characters.next().unwrap());
                    }
                },
            }
        };
        vec
    }

    fn at_boundary(&mut self) -> bool {
        matches!(
            (self.next(), self.next()),
            (Some(a), Some(b)) if categorize(a) != categorize(b)
        )
    }
}

pub fn distance<A: Into<usize>, B: Into<usize>>(a: A, b: B) -> usize {
    let (a, b) = (a.into(), b.into());
    a.saturating_sub(b).max(b.saturating_sub(a))
}
