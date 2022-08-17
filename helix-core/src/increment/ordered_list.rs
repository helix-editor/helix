use super::Increment;
use ropey::RopeSlice;
use std::string::String;

use crate::{
    textobject::{textobject_word, TextObject},
    Range, Tendril,
};

#[derive(Debug, PartialEq, Eq)]
pub struct OrderedListWalker<'a> {
    walkway: &'a Vec<String>,
    index: usize,
    is_capitalized: bool,
    range: Range,
}

impl<'a> OrderedListWalker<'a> {
    pub fn from_range(
        text: RopeSlice,
        range: Range,
        config: &'a [Vec<String>],
    ) -> Option<OrderedListWalker<'a>> {
        let range = textobject_word(text, range, TextObject::Inside, 1, false);
        let word: String = text.slice(range.from()..range.to()).chars().collect();
        if word.is_empty() {
            // no word found
            return None;
        }
        let lower_case_word: String = word.to_lowercase();
        for (_i, walkway) in config.iter().enumerate() {
            for (index, w) in walkway.iter().enumerate() {
                if !w.is_empty() && lower_case_word.eq(w.to_lowercase().as_str()) {
                    let is_capitalized: bool = word.chars().next().unwrap().is_uppercase();
                    return Some(OrderedListWalker {
                        walkway,
                        index,
                        is_capitalized,
                        range,
                    });
                }
            }
        }
        None
    }
}

impl<'a> Increment for OrderedListWalker<'a> {
    fn increment(&self, amount: i64) -> (Range, Tendril) {
        let pos: usize =
            (self.index as i64 + amount).rem_euclid(self.walkway.len() as i64) as usize;
        let mut s: String = self.walkway.get(pos).unwrap().into();
        if self.is_capitalized {
            // https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust
            s = s.chars().next().unwrap().to_uppercase().to_string()
                + s.chars().skip(1).collect::<String>().as_str();
        }
        (self.range, s.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_ordered_from_range() {
        let rope = Rope::from_str("Test text true more text.");
        let range = Range::point(12);
        let walkway_one = vec!["true".to_owned(), "false".to_owned()];
        let walkway_two = vec!["ja".to_owned(), "nee".to_owned()];
        let config = vec![walkway_two, walkway_one.to_owned()];
        assert_eq!(
            OrderedListWalker::from_range(rope.slice(..), range, &config),
            Some(OrderedListWalker {
                range: Range::new(10, 14),
                walkway: &walkway_one,
                index: 0,
                is_capitalized: false,
            })
        );
        let range = Range::point(10);
        assert_eq!(
            OrderedListWalker::from_range(rope.slice(..), range, &config),
            Some(OrderedListWalker {
                range: Range::new(10, 14),
                walkway: &walkway_one,
                index: 0,
                is_capitalized: false,
            })
        );
        let range = Range::point(13);
        assert_eq!(
            OrderedListWalker::from_range(rope.slice(..), range, &config),
            Some(OrderedListWalker {
                range: Range::new(10, 14),
                walkway: &walkway_one,
                index: 0,
                is_capitalized: false,
            })
        );
        let range = Range::point(14);
        assert_eq!(
            OrderedListWalker::from_range(rope.slice(..), range, &config),
            None,
        );
        let range = Range::point(9);
        assert_eq!(
            OrderedListWalker::from_range(rope.slice(..), range, &config),
            None,
        );
    }

    #[test]
    #[ignore]
    fn test_ordered_increment() {
        let walkway_one = vec!["true".to_owned(), "false".to_owned()];
        let walkway_two = vec!["ja".to_owned(), "nee".to_owned()];
        let config = vec![walkway_two, walkway_one];

        let tests = [
            ("false", "false", 2),
            ("false", "true", 1),
            ("false", "true", -1),
            ("false", "false", -2),
            ("False", "True", 1),
            ("True", "False", -1),
            ("Ja", "Nee", 3),
        ];

        for (original, expected, amount) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                OrderedListWalker::from_range(rope.slice(..), range, &config)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }
}
