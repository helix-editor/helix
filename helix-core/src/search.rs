use helix_stdx::rope::RopeSliceExt;

use crate::{
    graphemes::{
        nth_next_folded_grapheme_boundary, nth_next_grapheme_boundary,
        nth_prev_folded_grapheme_boundary, nth_prev_grapheme_boundary,
    },
    text_folding::{ropex::RopeSliceFoldExt, FoldAnnotations},
    RopeSlice,
};

pub trait GraphemeMatcher {
    fn grapheme_match(&self, g: RopeSlice) -> bool;
}

impl GraphemeMatcher for char {
    fn grapheme_match(&self, g: RopeSlice) -> bool {
        g == RopeSlice::from(self.encode_utf8(&mut [0; 4]) as &str)
    }
}

impl<F: Fn(RopeSlice) -> bool> GraphemeMatcher for F {
    fn grapheme_match(&self, g: RopeSlice) -> bool {
        (*self)(g)
    }
}

pub fn find_nth_next(
    text: RopeSlice,
    matcher: impl GraphemeMatcher,
    pos: usize,
    mut n: usize,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut count = 0;
    for (i, g) in text.graphemes_at(pos).skip(1).enumerate() {
        if matcher.grapheme_match(g) {
            count = i + 1;
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }

    (n == 0).then(|| nth_next_grapheme_boundary(text, pos, count))
}

pub fn find_nth_prev(
    text: RopeSlice,
    matcher: impl GraphemeMatcher,
    pos: usize,
    mut n: usize,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut count = 0;
    for (i, g) in text.graphemes_at(pos).reversed().enumerate() {
        if matcher.grapheme_match(g) {
            count = i + 1;
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }

    (n == 0).then(|| (nth_prev_grapheme_boundary(text, pos, count)))
}

pub fn find_folded_nth_next(
    text: RopeSlice,
    annotations: &FoldAnnotations,
    matcher: impl GraphemeMatcher,
    pos: usize,
    mut n: usize,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut count = 0;
    for (i, g) in text
        .folded_graphemes_at(annotations, text.char_to_byte(pos))
        .skip(1)
        .enumerate()
    {
        if matcher.grapheme_match(g) {
            count = i + 1;
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }

    (n == 0).then(|| nth_next_folded_grapheme_boundary(text, annotations, pos, count))
}

pub fn find_folded_nth_prev(
    text: RopeSlice,
    annotations: &FoldAnnotations,
    matcher: impl GraphemeMatcher,
    pos: usize,
    mut n: usize,
) -> Option<usize> {
    if n == 0 {
        return None;
    }

    let mut count = 0;
    for (i, g) in text
        .folded_graphemes_at(annotations, text.char_to_byte(pos))
        .reversed()
        .enumerate()
    {
        if matcher.grapheme_match(g) {
            count = i + 1;
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }

    (n == 0).then(|| nth_prev_folded_grapheme_boundary(text, annotations, pos, count))
}
