use crate::graphemes::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, RopeGraphemes};
use crate::{coords_at_pos, pos_at_coords, ChangeSet, Position, Range, Rope, RopeSlice, Selection};

pub fn move_next_word_start(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos + 1 == slice.len_chars() {
            return pos;
        }

        let mut ch = slice.char(pos);
        let next = slice.char(pos + 1);

        // if we're at the end of a word, or on whitespce right before new one
        if categorize(ch) != categorize(next) {
            pos += 1;
            ch = next;
        }

        if is_word(ch) {
            skip_over_next(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }

        // TODO: don't include newline?
        skip_over_next(slice, &mut pos, |ch| ch.is_ascii_whitespace());
    }

    pos
}

pub fn move_prev_word_start(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos == 0 {
            return pos;
        }

        let ch = slice.char(pos);
        let prev = slice.char(pos - 1);

        if categorize(ch) != categorize(prev) {
            pos -= 1;
        }

        // match (category c1, category c2) => {
        //  if c1 != c2 {
        //  }
        // }

        // TODO: skip while eol

        // TODO: don't include newline?
        skip_over_prev(slice, &mut pos, |ch| ch.is_ascii_whitespace());

        // refetch
        let ch = slice.char(pos);

        if is_word(ch) {
            skip_over_prev(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_prev(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }
        pos = pos.saturating_add(1)
    }

    pos
}

pub fn move_next_word_end(slice: RopeSlice, mut pos: usize, count: usize) -> usize {
    for _ in 0..count {
        if pos + 1 == slice.len_chars() {
            return pos;
        }

        let ch = slice.char(pos);
        let next = slice.char(pos + 1);

        if categorize(ch) != categorize(next) {
            pos += 1;
        }

        // TODO: don't include newline?
        skip_over_next(slice, &mut pos, |ch| ch.is_ascii_whitespace());

        // refetch
        let ch = slice.char(pos);

        if is_word(ch) {
            skip_over_next(slice, &mut pos, is_word);
        } else if ch.is_ascii_punctuation() {
            skip_over_next(slice, &mut pos, |ch| ch.is_ascii_punctuation());
        }
        pos -= 1
    }

    pos
}

// ---- util ------------

// used for by-word movement

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[derive(Debug, Eq, PartialEq)]
enum Category {
    Whitespace,
    Eol,
    Word,
    Punctuation,
}
fn categorize(ch: char) -> Category {
    if ch == '\n' {
        Category::Eol
    } else if ch.is_ascii_whitespace() {
        Category::Whitespace
    } else if ch.is_ascii_punctuation() {
        Category::Punctuation
    } else if ch.is_ascii_alphanumeric() {
        Category::Word
    } else {
        unreachable!()
    }
}

#[inline]
pub fn skip_over_next<F>(slice: RopeSlice, pos: &mut usize, fun: F)
where
    F: Fn(char) -> bool,
{
    let mut chars = slice.chars_at(*pos);

    for ch in chars {
        if !fun(ch) {
            break;
        }
        *pos += 1;
    }
}

#[inline]
pub fn skip_over_prev<F>(slice: RopeSlice, pos: &mut usize, fun: F)
where
    F: Fn(char) -> bool,
{
    // need to +1 so that prev() includes current char
    let mut chars = slice.chars_at(*pos + 1);

    while let Some(ch) = chars.prev() {
        if !fun(ch) {
            break;
        }
        *pos = pos.saturating_sub(1);
    }
}
