//! Utility functions to traverse the unicode graphemes of a `Rope`'s text contents.
//!
//! Based on <https://github.com/cessen/led/blob/c4fa72405f510b7fd16052f90a598c429b3104a6/src/graphemes.rs>
use ropey::{str_utils::byte_to_char_idx, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::UnicodeWidthStr;

use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::{slice, str};

use crate::chars::{char_is_whitespace, char_is_word};
use crate::LineEnding;

#[inline]
pub fn tab_width_at(visual_x: usize, tab_width: u16) -> usize {
    tab_width as usize - (visual_x % tab_width as usize)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grapheme<'a> {
    Newline,
    Tab { width: usize },
    Other { g: GraphemeStr<'a> },
}

impl<'a> Grapheme<'a> {
    pub fn new_decoration(g: &'static str) -> Grapheme<'a> {
        assert_ne!(g, "\t");
        Grapheme::new(g.into(), 0, 0)
    }

    pub fn new(g: GraphemeStr<'a>, visual_x: usize, tab_width: u16) -> Grapheme<'a> {
        match g {
            g if g == "\t" => Grapheme::Tab {
                width: tab_width_at(visual_x, tab_width),
            },
            _ if LineEnding::from_str(&g).is_some() => Grapheme::Newline,
            _ => Grapheme::Other { g },
        }
    }

    pub fn change_position(&mut self, visual_x: usize, tab_width: u16) {
        if let Grapheme::Tab { width } = self {
            *width = tab_width_at(visual_x, tab_width)
        }
    }

    /// Returns the a visual width of this grapheme,
    #[inline]
    pub fn width(&self) -> usize {
        match *self {
            // width is not cached because we are dealing with
            // ASCII almost all the time which already has a fastpath
            // it's okay to convert to u16 here because no codepoint has a width larger
            // than 2 and graphemes are usually atmost two visible codepoints wide
            Grapheme::Other { ref g } => grapheme_width(g),
            Grapheme::Tab { width } => width,
            Grapheme::Newline => 1,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        !matches!(&self, Grapheme::Other { g } if !g.chars().next().is_some_and(char_is_whitespace))
    }

    // TODO currently word boundaries are used for softwrapping.
    // This works best for programming languages and well for prose.
    // This could however be improved in the future by considering unicode
    // character classes but
    pub fn is_word_boundary(&self) -> bool {
        !matches!(&self, Grapheme::Other { g,.. } if g.chars().next().is_some_and(char_is_word))
    }
}

impl Display for Grapheme<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Grapheme::Newline => write!(f, " "),
            Grapheme::Tab { width } => {
                for _ in 0..width {
                    write!(f, " ")?;
                }
                Ok(())
            }
            Grapheme::Other { ref g } => {
                write!(f, "{g}")
            }
        }
    }
}

#[must_use]
pub fn grapheme_width(g: &str) -> usize {
    if g.as_bytes()[0] <= 127 {
        // Fast-path ascii.
        // Point 1: theoretically, ascii control characters should have zero
        // width, but in our case we actually want them to have width: if they
        // show up in text, we want to treat them as textual elements that can
        // be edited.  So we can get away with making all ascii single width
        // here.
        // Point 2: we're only examining the first codepoint here, which means
        // we're ignoring graphemes formed with combining characters.  However,
        // if it starts with ascii, it's going to be a single-width grapeheme
        // regardless, so, again, we can get away with that here.
        // Point 3: we're only examining the first _byte_.  But for utf8, when
        // checking for ascii range values only, that works.
        1
    } else {
        // We use max(1) here because all grapeheme clusters--even illformed
        // ones--should have at least some width so they can be edited
        // properly.
        // TODO properly handle unicode width for all codepoints
        // example of where unicode width is currently wrong: 🤦🏼‍♂️ (taken from https://hsivonen.fi/string-length/)
        UnicodeWidthStr::width(g).max(1)
    }
}

// NOTE: for byte indexing versions of these functions see `RopeSliceExt`'s
// `floor_grapheme_boundary` and `ceil_grapheme_boundary` and the rope grapheme iterators.

#[must_use]
pub fn nth_prev_grapheme_boundary(slice: RopeSlice, char_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let mut byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the previous grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.prev_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return 0,
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    let (a, b, c, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                    chunk = a;
                    chunk_byte_idx = b;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

/// Finds the previous grapheme boundary before the given char position.
#[must_use]
#[inline(always)]
pub fn prev_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> usize {
    nth_prev_grapheme_boundary(slice, char_idx, 1)
}

#[must_use]
pub fn nth_next_grapheme_boundary(slice: RopeSlice, char_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let mut byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the nth next grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.next_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return slice.len_chars(),
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte_idx += chunk.len();
                    let (a, _, c, _) = slice.chunk_at_byte(chunk_byte_idx);
                    chunk = a;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

/// Finds the next grapheme boundary after the given char position.
#[must_use]
#[inline(always)]
pub fn next_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> usize {
    nth_next_grapheme_boundary(slice, char_idx, 1)
}

/// Returns the passed char index if it's already a grapheme boundary,
/// or the next grapheme boundary char index if not.
#[must_use]
#[inline]
pub fn ensure_grapheme_boundary_next(slice: RopeSlice, char_idx: usize) -> usize {
    if char_idx == 0 {
        char_idx
    } else {
        next_grapheme_boundary(slice, char_idx - 1)
    }
}

/// Returns the passed char index if it's already a grapheme boundary,
/// or the prev grapheme boundary char index if not.
#[must_use]
#[inline]
pub fn ensure_grapheme_boundary_prev(slice: RopeSlice, char_idx: usize) -> usize {
    if char_idx == slice.len_chars() {
        char_idx
    } else {
        prev_grapheme_boundary(slice, char_idx + 1)
    }
}

/// A highly compressed Cow<'a, str> that holds
/// atmost u31::MAX bytes and is readonly
pub struct GraphemeStr<'a> {
    ptr: NonNull<u8>,
    len: u32,
    phantom: PhantomData<&'a str>,
}

impl GraphemeStr<'_> {
    const MASK_OWNED: u32 = 1 << 31;

    fn compute_len(&self) -> usize {
        (self.len & !Self::MASK_OWNED) as usize
    }
}

impl Deref for GraphemeStr<'_> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let bytes = slice::from_raw_parts(self.ptr.as_ptr(), self.compute_len());
            str::from_utf8_unchecked(bytes)
        }
    }
}

impl Drop for GraphemeStr<'_> {
    fn drop(&mut self) {
        if self.len & Self::MASK_OWNED != 0 {
            // free allocation
            unsafe {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    self.ptr.as_ptr(),
                    self.compute_len(),
                )));
            }
        }
    }
}

impl<'a> From<&'a str> for GraphemeStr<'a> {
    fn from(g: &'a str) -> Self {
        GraphemeStr {
            ptr: unsafe { NonNull::new_unchecked(g.as_bytes().as_ptr() as *mut u8) },
            len: i32::try_from(g.len()).unwrap() as u32,
            phantom: PhantomData,
        }
    }
}

impl From<String> for GraphemeStr<'_> {
    fn from(g: String) -> Self {
        let len = g.len();
        let ptr = Box::into_raw(g.into_bytes().into_boxed_slice()) as *mut u8;
        GraphemeStr {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            len: (i32::try_from(len).unwrap() as u32) | Self::MASK_OWNED,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<Cow<'a, str>> for GraphemeStr<'a> {
    fn from(g: Cow<'a, str>) -> Self {
        match g {
            Cow::Borrowed(g) => g.into(),
            Cow::Owned(g) => g.into(),
        }
    }
}

impl<T: Deref<Target = str>> PartialEq<T> for GraphemeStr<'_> {
    fn eq(&self, other: &T) -> bool {
        self.deref() == other.deref()
    }
}
impl PartialEq<str> for GraphemeStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.deref() == other
    }
}
impl Eq for GraphemeStr<'_> {}
impl Debug for GraphemeStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}
impl Display for GraphemeStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.deref(), f)
    }
}
impl Clone for GraphemeStr<'_> {
    fn clone(&self) -> Self {
        self.deref().to_owned().into()
    }
}
