use crate::RopeSlice;

pub fn find_nth_next(
    text: RopeSlice,
    ch: char,
    mut pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    if pos >= text.len_chars() {
        return None;
    }

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

    if !inclusive {
        pos -= 1;
    }

    Some(pos)
}

pub fn find_nth_prev(
    text: RopeSlice,
    ch: char,
    mut pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    // start searching right before pos
    pos = pos.saturating_sub(1);
    let mut chars = text.chars_at(pos);

    for _ in 0..n {
        loop {
            let c = chars.prev()?;

            pos = pos.saturating_sub(1);

            if c == ch {
                break;
            }
        }
    }

    if !inclusive {
        pos += 1;
    }

    Some(pos)
}
