use crate::movement::{categorize, is_horiz_blank, is_word, skip_over_prev};
use ropey::RopeSlice;

#[must_use]
pub fn nth_prev_word_boundary(slice: RopeSlice, mut char_idx: usize, count: usize) -> usize {
    let mut with_end = false;

    for _ in 0..count {
        if char_idx == 0 {
            break;
        }

        // return if not skip while?
        skip_over_prev(slice, &mut char_idx, |ch| ch == '\n');

        with_end = skip_over_prev(slice, &mut char_idx, is_horiz_blank);

        // refetch
        let ch = slice.char(char_idx);

        if is_word(ch) {
            with_end = skip_over_prev(slice, &mut char_idx, is_word);
        } else if ch.is_ascii_punctuation() {
            with_end = skip_over_prev(slice, &mut char_idx, |ch| ch.is_ascii_punctuation());
        }
    }

    if with_end {
        char_idx
    } else {
        char_idx + 1
    }
}
