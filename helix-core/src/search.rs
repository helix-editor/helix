use crate::{line_ending::line_end_char_index, movement::Direction, RopeSlice};

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

#[derive(Copy, Clone)]
pub enum PairMatcher<'a> {
    Char(char),
    LineEnding(&'a str),
}

pub fn find_nth_pair(
    text: RopeSlice,
    pair_matcher_left: PairMatcher,
    pair_matcher_right: PairMatcher,
    pos: usize,
    n: usize,
    direction: Direction,
) -> Option<usize> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let is_forward = direction == Direction::Forward;
    let direction_multiplier = if is_forward { 1 } else { -1 };

    match (pair_matcher_left, pair_matcher_right) {
        (PairMatcher::Char(ch_left), PairMatcher::Char(ch_right)) => {
            let chars = text.chars_at(pos);

            let mut chars = if is_forward {
                chars.peekable()
            } else {
                chars.reversed().peekable()
            };

            let mut offset = 0;

            for _ in 0..n {
                loop {
                    let ch_next = chars.next()?;
                    let ch_peek = chars.peek()?;

                    offset += 1;

                    let matches_char = if is_forward {
                        ch_left == ch_next && ch_right == *ch_peek
                    } else {
                        ch_right == ch_next && ch_left == *ch_peek
                    };

                    if matches_char {
                        break;
                    }
                }
            }

            let offs = offset * direction_multiplier;
            let new_pos: usize = (pos as isize + offs)
                .try_into()
                .expect("Character offset cannot exceed character count");

            Some(new_pos - 1)
        }
        (PairMatcher::Char(ch_left), PairMatcher::LineEnding(eol)) => {
            let start_line = text.char_to_line(pos);
            let start_line = if pos >= line_end_char_index(&text, start_line) {
                // if our cursor is currently on a character just before the eol, or on the eol
                // we start searching from the next line, instead of from the current line.
                start_line + eol.len()
            } else {
                start_line
            };

            let mut lines = if is_forward {
                text.lines_at(start_line).enumerate()
            } else {
                text.lines_at(start_line).reversed().enumerate()
            };

            if !is_forward {
                // skip the line we are currently on when going backward
                lines.next();
            }

            let mut matched_count = 0;
            for (traversed_lines, _line) in lines {
                let current_line = (start_line as isize
                    + (traversed_lines as isize * direction_multiplier))
                    as usize;

                let ch_opposite_eol_i = if is_forward {
                    line_end_char_index(&text, current_line).saturating_sub(eol.len())
                } else {
                    text.line_to_char(current_line)
                };

                let ch_opposite_eol = text.char(ch_opposite_eol_i);

                if ch_opposite_eol == ch_left {
                    matched_count += 1;
                    if matched_count == n {
                        return Some(ch_opposite_eol_i - if is_forward { 0 } else { 1 });
                    }
                }
            }

            None
        }
        (PairMatcher::LineEnding(eol), PairMatcher::Char(ch_right)) => {
            // Search starting from the beginning of the next or previous line
            let start_line = text.char_to_line(pos) + (is_forward as usize);

            let lines = if is_forward {
                text.lines_at(start_line).enumerate()
            } else {
                text.lines_at(start_line).reversed().enumerate()
            };

            let mut matched_count = 0;
            for (traversed_lines, _line) in lines {
                let current_line = (start_line as isize
                    + (traversed_lines as isize * direction_multiplier))
                    as usize;

                let ch_opposite_eol_i = if is_forward {
                    // eol, THEN character at the beginning of the current line
                    text.line_to_char(current_line)
                } else {
                    // character at the end of the previous line, THEN eol
                    line_end_char_index(&text, current_line - 1) - eol.len()
                };

                let ch_opposite_eol = text.get_char(ch_opposite_eol_i)?;

                if ch_opposite_eol == ch_right {
                    matched_count += 1;
                    if matched_count == n {
                        return Some(ch_opposite_eol_i - (is_forward as usize));
                    }
                }
            }

            None
        }
        (PairMatcher::LineEnding(eol), PairMatcher::LineEnding(_)) => {
            // Search starting from the beginning of the
            // line after the current one
            let start_line = text.char_to_line(pos) + 1;

            let mut lines = if is_forward {
                text.lines_at(start_line).enumerate()
            } else {
                text.lines_at(start_line).reversed().enumerate()
            };

            if !is_forward {
                // skip the line we are currently on when going backward
                lines.next();
            }

            let mut matched_count = 0;
            for (traversed_lines, _line) in lines {
                let current_line = (start_line as isize
                    + (traversed_lines as isize * direction_multiplier))
                    as usize;
                let current_line = text.line_to_char(current_line);
                let current_line_end = current_line + eol.len();
                if text.slice(current_line..current_line_end).as_str()? == eol {
                    matched_count += 1;
                    if matched_count == n {
                        return Some(current_line - eol.len());
                    }
                }
            }

            None
        }
    }
}
