use std::borrow::Cow;

use ropey::RopeSlice;

use crate::{
    textobject::{textobject_word, TextObject},
    Range,
};

#[derive(Debug, PartialEq, Eq)]
pub struct NumberInfo {
    pub range: Range,
    pub value: i64,
    pub radix: u32,
}

/// Return information about number under cursor if there is one.
pub fn number_at(text: RopeSlice, range: Range) -> Option<NumberInfo> {
    let word_range = textobject_word(text, range, TextObject::Inside, 1, true);
    let word: Cow<str> = text.slice(word_range.from()..word_range.to()).into();
    let (radix, prefixed) = if word.starts_with("0x") {
        (16, true)
    } else if word.starts_with("0o") {
        (8, true)
    } else if word.starts_with("0b") {
        (2, true)
    } else {
        (10, false)
    };

    let number = if prefixed { &word[2..] } else { &word };

    let value = i128::from_str_radix(&number, radix).ok()?;
    if (value.is_positive() && value.leading_zeros() < 64)
        || (value.is_negative() && value.leading_ones() < 64)
    {
        return None;
    }

    let value = value as i64;
    Some(NumberInfo {
        range: word_range,
        value,
        radix,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_decimal_at_point() {
        let rope = Rope::from_str("Test text 12345 more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 15),
                value: 12345,
                radix: 10,
            })
        );
    }

    #[test]
    fn test_uppercase_hexadecimal_at_point() {
        let rope = Rope::from_str("Test text 0x123ABCDEF more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 21),
                value: 0x123ABCDEF,
                radix: 16,
            })
        );
    }

    #[test]
    fn test_lowercase_hexadecimal_at_point() {
        let rope = Rope::from_str("Test text 0xfa3b4e more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 18),
                value: 0xfa3b4e,
                radix: 16,
            })
        );
    }

    #[test]
    fn test_octal_at_point() {
        let rope = Rope::from_str("Test text 0o1074312 more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 19),
                value: 0o1074312,
                radix: 8,
            })
        );
    }

    #[test]
    fn test_binary_at_point() {
        let rope = Rope::from_str("Test text 0b10111010010101 more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 26),
                value: 0b10111010010101,
                radix: 2,
            })
        );
    }

    #[test]
    fn test_negative_decimal_at_point() {
        let rope = Rope::from_str("Test text -54321 more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 16),
                value: -54321,
                radix: 10,
            })
        );
    }

    #[test]
    fn test_decimal_with_leading_zeroes_at_point() {
        let rope = Rope::from_str("Test text 000045326 more text.");
        let range = Range::point(12);
        assert_eq!(
            number_at(rope.slice(..), range),
            Some(NumberInfo {
                range: Range::new(10, 19),
                value: 45326,
                radix: 10,
            })
        );
    }

    #[test]
    fn test_not_a_number_point() {
        let rope = Rope::from_str("Test text 45326 more text.");
        let range = Range::point(6);
        assert_eq!(number_at(rope.slice(..), range), None);
    }

    #[test]
    fn test_number_too_large_at_point() {
        let rope = Rope::from_str("Test text 0xFFFFFFFFFFFFFFFFF more text.");
        let range = Range::point(12);
        assert_eq!(number_at(rope.slice(..), range), None);
    }
}
