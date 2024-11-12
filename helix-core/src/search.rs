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

pub fn find_nth_next_tag(
    text: RopeSlice,
    tag: &str,
    mut pos: usize,
    n: usize,
) -> Option<Vec<usize>> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);

    let tag = format!("</{tag}>");
    let len = tag.len();

    for _ in 0..n {
        loop {
            let c = chars.next()?;
            let cloned_chars = chars.clone();
            let stri: String = cloned_chars.take(len).collect();

            pos += 1;

            if stri == tag {
                break;
            }
        }
    }

    let range: Vec<usize> = (pos - 1..pos + len - 1).collect();

    Some(range)
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

pub fn find_nth_prev_tag(
    text: RopeSlice,
    tag: &str,
    mut pos: usize,
    n: usize,
) -> Option<Vec<usize>> {
    if pos == 0 || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos).reversed();

    let tag = format!("<{tag}>");
    let len = tag.len();

    for _ in 0..n {
        loop {
            let c = chars.next()?;
            let cloned_chars = chars.clone();
            let stri: String = cloned_chars.take(len).collect();

            pos -= 1;

            if stri == tag {
                break;
            }
        }
    }

    let range: Vec<usize> = (pos..pos + len).collect();

    Some(range)
}
