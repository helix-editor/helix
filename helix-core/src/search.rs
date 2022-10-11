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
    let mut count = 0; 

    while count < n {
        let c = chars.next()?;
        if char_matcher.char_match(c) {
            count += 1;
        }
        pos += 1; 
    }

    Some(pos - 1)
}

pub fn find_nth_prev<M: CharMatcher>(
    text: RopeSlice,
    char_matcher: M,
    mut pos: usize,
    n: usize
) -> Option<usize> {
    if pos == 0 || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);
    let mut count = 0; 

    while count < n {
        let c = chars.prev()?;
        if char_matcher.char_match(c) {
            count += 1; 
        }
        pos -= 1; 
    }

    Some(pos)
}
