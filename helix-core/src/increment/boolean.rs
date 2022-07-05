use super::Increment;
use ropey::RopeSlice;

use crate::{
    textobject::{textobject_word, TextObject},
    Range, Tendril,
};

#[derive(Debug, PartialEq, Eq)]
pub struct BooleanIncrementor<'a> {
    incremented: &'a str,
    range: Range,
}

impl<'a> BooleanIncrementor<'a> {
    /// Return information about boolean under range if there is one.
    pub fn from_range(text: RopeSlice, range: Range) -> Option<BooleanIncrementor> {
        let range = textobject_word(text, range, TextObject::Inside, 1, false);
        let word: String = text.slice(range.from()..range.to()).chars().collect();

        let incremented = match word.as_str() {
            "false" => "true",
            "true" => "false",
            "False" => "True",
            "True" => "False",
            "FALSE" => "TRUE",
            "TRUE" => "FALSE",
            _ => return None,
        };

        Some(BooleanIncrementor { incremented, range })
    }
}

impl<'a> Increment for BooleanIncrementor<'a> {
    fn increment(&self, _amount: i64) -> (Range, Tendril) {
        (self.range, self.incremented.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_boolean_from_range() {
        let rope = Rope::from_str("Test text true more text.");
        let range = Range::point(12);
        assert_eq!(
            BooleanIncrementor::from_range(rope.slice(..), range),
            Some(BooleanIncrementor {
                range: Range::new(10, 14),
                incremented: "false",
            })
        );
        let range = Range::point(10);
        assert_eq!(
            BooleanIncrementor::from_range(rope.slice(..), range),
            Some(BooleanIncrementor {
                range: Range::new(10, 14),
                incremented: "false",
            })
        );
        let range = Range::point(13);
        assert_eq!(
            BooleanIncrementor::from_range(rope.slice(..), range),
            Some(BooleanIncrementor {
                range: Range::new(10, 14),
                incremented: "false",
            })
        );
        let range = Range::point(14);
        assert_eq!(BooleanIncrementor::from_range(rope.slice(..), range), None,);
        let range = Range::point(9);
        assert_eq!(BooleanIncrementor::from_range(rope.slice(..), range), None,);
    }

    #[test]
    fn test_boolean_increment() {
        let tests = [
            ("false", "true"),
            ("true", "false"),
            ("False", "True"),
            ("True", "False"),
            ("FALSE", "TRUE"),
            ("TRUE", "FALSE"),
        ];

        for (original, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                BooleanIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(1)
                    .1,
                Tendril::from(expected)
            );
        }
    }
}
