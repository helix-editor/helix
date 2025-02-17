use crate::RopeSlice;

// TODO: switch to std::str::Pattern when it is stable.
pub trait CharMatcher {
    fn char_match(&self, ch: char) -> bool;
}

impl CharMatcher for char {
    fn char_match(&self, ch: char) -> bool {
        *self == ch
    }
}

impl<F: Fn(&char) -> bool> CharMatcher for F {
    fn char_match(&self, ch: char) -> bool {
        (*self)(&ch)
    }
}

pub fn find_nth_next<M: CharMatcher>(
    text: RopeSlice,
    char_matcher: M,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);

    for _ in 0..n {
        loop {
            let c = chars.next()?;

            pos += 1;

            if char_matcher.char_match(c) {
                break;
            }
        }
    }

    Some(pos - 1)
}

pub fn find_nth_prev(text: RopeSlice, ch: char, mut pos: usize, n: usize) -> Option<usize> {
    if pos == 0 || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);

    for _ in 0..n {
        loop {
            let c = chars.prev()?;

            pos -= 1;

            if c == ch {
                break;
            }
        }
    }

    Some(pos)
}

pub fn find_nth_next_pair<M: CharMatcher>(
    text: RopeSlice,
    char_matcher_left: M,
    char_matcher_right: M,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos).peekable();

    for _ in 0..n {
        loop {
            let ch_left = chars.next()?;
            let ch_right = chars.peek()?;

            pos += 1;

            if char_matcher_left.char_match(ch_left) && char_matcher_right.char_match(*ch_right) {
                break;
            }
        }
    }

    Some(pos - 1)
}

pub fn find_nth_prev_pair<M: CharMatcher>(
    text: RopeSlice,
    char_matcher_left: M,
    char_matcher_right: M,
    mut pos: usize,
    n: usize,
) -> Option<usize> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos).reversed().peekable();

    for _ in 0..n {
        loop {
            let ch_right = chars.next()?;
            let ch_left = chars.peek()?;

            pos -= 1;

            if char_matcher_left.char_match(*ch_left) && char_matcher_right.char_match(ch_right) {
                break;
            }
        }
    }

    Some(pos - 1)
}
