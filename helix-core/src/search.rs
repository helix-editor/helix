use crate::RopeSlice;

pub fn find_nth_next(text: RopeSlice, ch: char, mut pos: usize, n: usize) -> Option<usize> {
    // start searching right after pos
    let mut chars = text.chars_at(pos + 1);

    for _ in 0..n {
        loop {
            let c = chars.next()?;

            pos += 1;

            if c == ch {
                break;
            }
        }
    }

    Some(pos)
}

pub fn find_nth_prev(text: RopeSlice, ch: char, mut pos: usize, n: usize) -> Option<usize> {
    // start searching right before pos
    let mut chars = text.chars_at(pos.saturating_sub(1));

    for _ in 0..n {
        loop {
            let c = chars.prev()?;

            pos = pos.saturating_sub(1);

            if c == ch {
                break;
            }
        }
    }

    Some(pos)
}
