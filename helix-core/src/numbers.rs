use std::borrow::Cow;

use ropey::{Rope, RopeSlice};

pub struct NumberInfo {
    pub start: usize,
    pub end: usize,
    pub value: i64,
    pub radix: u32,
}

/// If there is a number at `char_idx`, return the text range, value and radix.
pub fn number_at(text: &Rope, char_idx: usize) -> Option<NumberInfo> {
    let line = text.char_to_line(char_idx);
    let line_start = text.line_to_char(line);
    let line = text.line(line);
    let line_len = line.len_chars();

    let mut pos = char_idx - line_start;
    let mut range_and_radix = None;

    // Search from the cursor until a number is found or we reach the end of the line.
    while range_and_radix.is_none() && pos < line_len {
        pos += line.chars_at(pos).take_while(|c| !c.is_digit(16)).count();

        range_and_radix = if let Some((start, end)) = hex_number_range(&line, pos) {
            Some((start, end, 16))
        } else if let Some((start, end)) = octal_number_range(&line, pos) {
            Some((start, end, 8))
        } else if let Some((start, end)) = binary_number_range(&line, pos) {
            Some((start, end, 2))
        } else if let Some((start, end)) = decimal_number_range(&line, pos) {
            // We don't want to treat the '0' of the prefixes "0x", "0o", and "0b" as a number itself, so check for that here.
            if end - start == 1 && line.char(start) == '0' && start + 2 < line_len {
                let (c1, c2) = (line.char(start + 1), line.char(start + 2));
                if c1 == 'x' && c2.is_digit(16)
                    || c1 == 'o' && c2.is_digit(8)
                    || c1 == 'b' && c2.is_digit(2)
                {
                    pos += 2;
                    continue;
                }
            }

            Some((start, end, 10))
        } else {
            pos += 1;
            None
        };
    }

    if let Some((start, end, radix)) = range_and_radix {
        let number_text: Cow<str> = line.slice(start..end).into();
        let value = i128::from_str_radix(&number_text, radix).ok()?;
        if (value.is_positive() && value.leading_zeros() < 64)
            || (value.is_negative() && value.leading_ones() < 64)
        {
            return None;
        }
        let value = value as i64;
        Some(NumberInfo {
            start: line_start + start,
            end: line_start + end,
            value,
            radix,
        })
    } else {
        None
    }
}

/// Return the start and end of the decimal number at `pos` if there is one.
fn decimal_number_range(text: &RopeSlice, pos: usize) -> Option<(usize, usize)> {
    if pos >= text.len_chars() {
        return None;
    }
    let pos = pos + 1;
    let mut chars = text.chars_at(pos);
    chars.reverse();
    let decimal_start = pos - chars.take_while(|c| c.is_digit(10)).count();

    if decimal_start < pos {
        let decimal_end = decimal_start
            + text
                .chars_at(decimal_start)
                .take_while(|c| c.is_digit(10))
                .count();

        // Handle negative numbers
        if decimal_start > 0 && text.char(decimal_start - 1) == '-' {
            Some((decimal_start - 1, decimal_end))
        } else {
            Some((decimal_start, decimal_end))
        }
    } else {
        None
    }
}

/// Return the start and end of the hexidecimal number at `pos` if there is one.
/// Hexidecimal numbers must be prefixed with "0x". The prefix will not be included in the range.
fn hex_number_range(text: &RopeSlice, pos: usize) -> Option<(usize, usize)> {
    prefixed_number_range(text, pos, 16, 'x')
}

/// Return the start and end of the octal number at `pos` if there is one.
/// Octal numbers must be prefixed with "0o". The prefix will not be included in the range.
fn octal_number_range(text: &RopeSlice, pos: usize) -> Option<(usize, usize)> {
    prefixed_number_range(text, pos, 8, 'o')
}

/// Return the start and end of the binary number at `pos` if there is one.
/// Binary numbers must be prefixed with "0b". The prefix will not be included in the range.
fn binary_number_range(text: &RopeSlice, pos: usize) -> Option<(usize, usize)> {
    prefixed_number_range(text, pos, 2, 'b')
}

/// Return the start and end of the number at `pos` if there is one with the given `radix` and `prefix_char`.
/// The number must be prefixed with `'0' + prefix_char`. The prefix will not be included in the range.
fn prefixed_number_range(
    text: &RopeSlice,
    pos: usize,
    radix: u32,
    prefix_char: char,
) -> Option<(usize, usize)> {
    if pos >= text.len_chars() {
        return None;
    }
    let pos = pos + 1;
    let mut chars = text.chars_at(pos);
    chars.reverse();
    let start = pos - chars.take_while(|c| c.is_digit(radix)).count();
    let is_num = start < pos
        && start >= 2
        && text.char(start - 2) == '0'
        && text.char(start - 1) == prefix_char;

    if is_num {
        let end = pos + text.chars_at(pos).take_while(|c| c.is_digit(radix)).count();
        Some((start, end))
    } else {
        None
    }
}
