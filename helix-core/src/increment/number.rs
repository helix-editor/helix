use std::borrow::Cow;

use ropey::RopeSlice;

use super::Increment;

use crate::{
    textobject::{textobject_word, TextObject},
    Range, Tendril,
};

#[derive(Debug, PartialEq, Eq)]
pub struct NumberIncrementor<'a> {
    value: i64,
    radix: u32,
    range: Range,

    text: RopeSlice<'a>,
}

impl<'a> NumberIncrementor<'a> {
    /// Return information about number under rang if there is one.
    pub fn from_range(text: RopeSlice, range: Range) -> Option<NumberIncrementor> {
        // If the cursor is on the minus sign of a number we want to get the word textobject to the
        // right of it.
        let range = if range.to() < text.len_chars()
            && range.to() - range.from() <= 1
            && text.char(range.from()) == '-'
        {
            Range::new(range.from() + 1, range.to() + 1)
        } else {
            range
        };

        let range = textobject_word(text, range, TextObject::Inside, 1, false);

        // If there is a minus sign to the left of the word object, we want to include it in the range.
        let range = if range.from() > 0 && text.char(range.from() - 1) == '-' {
            range.extend(range.from() - 1, range.from())
        } else {
            range
        };

        let word: String = text
            .slice(range.from()..range.to())
            .chars()
            .filter(|&c| c != '_')
            .collect();
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

        let value = i128::from_str_radix(number, radix).ok()?;
        if (value.is_positive() && value.leading_zeros() < 64)
            || (value.is_negative() && value.leading_ones() < 64)
        {
            return None;
        }

        let value = value as i64;
        Some(NumberIncrementor {
            range,
            value,
            radix,
            text,
        })
    }
}

impl<'a> Increment for NumberIncrementor<'a> {
    fn increment(&self, amount: i64) -> (Range, Tendril) {
        let old_text: Cow<str> = self.text.slice(self.range.from()..self.range.to()).into();
        let old_length = old_text.len();
        let new_value = self.value.wrapping_add(amount);

        // Get separator indexes from right to left.
        let separator_rtl_indexes: Vec<usize> = old_text
            .chars()
            .rev()
            .enumerate()
            .filter_map(|(i, c)| if c == '_' { Some(i) } else { None })
            .collect();

        let format_length = if self.radix == 10 {
            match (self.value.is_negative(), new_value.is_negative()) {
                (true, false) => old_length - 1,
                (false, true) => old_length + 1,
                _ => old_text.len(),
            }
        } else {
            old_text.len() - 2
        } - separator_rtl_indexes.len();

        let mut new_text = match self.radix {
            2 => format!("0b{:01$b}", new_value, format_length),
            8 => format!("0o{:01$o}", new_value, format_length),
            10 if old_text.starts_with('0') || old_text.starts_with("-0") => {
                format!("{:01$}", new_value, format_length)
            }
            10 => format!("{}", new_value),
            16 => {
                let (lower_count, upper_count): (usize, usize) =
                    old_text.chars().skip(2).fold((0, 0), |(lower, upper), c| {
                        (
                            lower + c.is_ascii_lowercase().then(|| 1).unwrap_or(0),
                            upper + c.is_ascii_uppercase().then(|| 1).unwrap_or(0),
                        )
                    });
                if upper_count > lower_count {
                    format!("0x{:01$X}", new_value, format_length)
                } else {
                    format!("0x{:01$x}", new_value, format_length)
                }
            }
            _ => unimplemented!("radix not supported: {}", self.radix),
        };

        // Add separators from original number.
        for &rtl_index in &separator_rtl_indexes {
            if rtl_index < new_text.len() {
                let new_index = new_text.len() - rtl_index;
                new_text.insert(new_index, '_');
            }
        }

        // Add in additional separators if necessary.
        if new_text.len() > old_length && !separator_rtl_indexes.is_empty() {
            let spacing = match separator_rtl_indexes.as_slice() {
                [.., b, a] => a - b - 1,
                _ => separator_rtl_indexes[0],
            };

            let prefix_length = if self.radix == 10 { 0 } else { 2 };
            if let Some(mut index) = new_text.find('_') {
                while index - prefix_length > spacing {
                    index -= spacing;
                    new_text.insert(index, '_');
                }
            }
        }

        (self.range, new_text.into())
    }
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
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 15),
                value: 12345,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_uppercase_hexadecimal_at_point() {
        let rope = Rope::from_str("Test text 0x123ABCDEF more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 21),
                value: 0x123ABCDEF,
                radix: 16,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_lowercase_hexadecimal_at_point() {
        let rope = Rope::from_str("Test text 0xfa3b4e more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 18),
                value: 0xfa3b4e,
                radix: 16,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_octal_at_point() {
        let rope = Rope::from_str("Test text 0o1074312 more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 19),
                value: 0o1074312,
                radix: 8,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_binary_at_point() {
        let rope = Rope::from_str("Test text 0b10111010010101 more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 26),
                value: 0b10111010010101,
                radix: 2,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_negative_decimal_at_point() {
        let rope = Rope::from_str("Test text -54321 more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 16),
                value: -54321,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_decimal_with_leading_zeroes_at_point() {
        let rope = Rope::from_str("Test text 000045326 more text.");
        let range = Range::point(12);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 19),
                value: 45326,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_negative_decimal_cursor_on_minus_sign() {
        let rope = Rope::from_str("Test text -54321 more text.");
        let range = Range::point(10);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(10, 16),
                value: -54321,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_number_under_range_start_of_rope() {
        let rope = Rope::from_str("100");
        let range = Range::point(0);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(0, 3),
                value: 100,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_number_under_range_end_of_rope() {
        let rope = Rope::from_str("100");
        let range = Range::point(2);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(0, 3),
                value: 100,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_number_surrounded_by_punctuation() {
        let rope = Rope::from_str(",100;");
        let range = Range::point(1);
        assert_eq!(
            NumberIncrementor::from_range(rope.slice(..), range),
            Some(NumberIncrementor {
                range: Range::new(1, 4),
                value: 100,
                radix: 10,
                text: rope.slice(..),
            })
        );
    }

    #[test]
    fn test_not_a_number_point() {
        let rope = Rope::from_str("Test text 45326 more text.");
        let range = Range::point(6);
        assert_eq!(NumberIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_number_too_large_at_point() {
        let rope = Rope::from_str("Test text 0xFFFFFFFFFFFFFFFFF more text.");
        let range = Range::point(12);
        assert_eq!(NumberIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_number_cursor_one_right_of_number() {
        let rope = Rope::from_str("100 ");
        let range = Range::point(3);
        assert_eq!(NumberIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_number_cursor_one_left_of_number() {
        let rope = Rope::from_str(" 100");
        let range = Range::point(0);
        assert_eq!(NumberIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_increment_basic_decimal_numbers() {
        let tests = [
            ("100", 1, "101"),
            ("100", -1, "99"),
            ("99", 1, "100"),
            ("100", 1000, "1100"),
            ("100", -1000, "-900"),
            ("-1", 1, "0"),
            ("-1", 2, "1"),
            ("1", -1, "0"),
            ("1", -2, "-1"),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                NumberIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }

    #[test]
    fn test_increment_basic_hexadecimal_numbers() {
        let tests = [
            ("0x0100", 1, "0x0101"),
            ("0x0100", -1, "0x00ff"),
            ("0x0001", -1, "0x0000"),
            ("0x0000", -1, "0xffffffffffffffff"),
            ("0xffffffffffffffff", 1, "0x0000000000000000"),
            ("0xffffffffffffffff", 2, "0x0000000000000001"),
            ("0xffffffffffffffff", -1, "0xfffffffffffffffe"),
            ("0xABCDEF1234567890", 1, "0xABCDEF1234567891"),
            ("0xabcdef1234567890", 1, "0xabcdef1234567891"),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                NumberIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }

    #[test]
    fn test_increment_basic_octal_numbers() {
        let tests = [
            ("0o0107", 1, "0o0110"),
            ("0o0110", -1, "0o0107"),
            ("0o0001", -1, "0o0000"),
            ("0o7777", 1, "0o10000"),
            ("0o1000", -1, "0o0777"),
            ("0o0107", 10, "0o0121"),
            ("0o0000", -1, "0o1777777777777777777777"),
            ("0o1777777777777777777777", 1, "0o0000000000000000000000"),
            ("0o1777777777777777777777", 2, "0o0000000000000000000001"),
            ("0o1777777777777777777777", -1, "0o1777777777777777777776"),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                NumberIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }

    #[test]
    fn test_increment_basic_binary_numbers() {
        let tests = [
            ("0b00000100", 1, "0b00000101"),
            ("0b00000100", -1, "0b00000011"),
            ("0b00000100", 2, "0b00000110"),
            ("0b00000100", -2, "0b00000010"),
            ("0b00000001", -1, "0b00000000"),
            ("0b00111111", 10, "0b01001001"),
            ("0b11111111", 1, "0b100000000"),
            ("0b10000000", -1, "0b01111111"),
            (
                "0b0000",
                -1,
                "0b1111111111111111111111111111111111111111111111111111111111111111",
            ),
            (
                "0b1111111111111111111111111111111111111111111111111111111111111111",
                1,
                "0b0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0b1111111111111111111111111111111111111111111111111111111111111111",
                2,
                "0b0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0b1111111111111111111111111111111111111111111111111111111111111111",
                -1,
                "0b1111111111111111111111111111111111111111111111111111111111111110",
            ),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                NumberIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }

    #[test]
    fn test_increment_with_separators() {
        let tests = [
            ("999_999", 1, "1_000_000"),
            ("1_000_000", -1, "999_999"),
            ("-999_999", -1, "-1_000_000"),
            ("0x0000_0000_0001", 0x1_ffff_0000, "0x0001_ffff_0001"),
            ("0x0000_0000_0001", 0x1_ffff_0000, "0x0001_ffff_0001"),
            ("0x0000_0000_0001", 0x1_ffff_0000, "0x0001_ffff_0001"),
            ("0x0000_0000", -1, "0xffff_ffff_ffff_ffff"),
            ("0x0000_0000_0000", -1, "0xffff_ffff_ffff_ffff"),
            ("0b01111111_11111111", 1, "0b10000000_00000000"),
            ("0b11111111_11111111", 1, "0b1_00000000_00000000"),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                NumberIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }
}
