use chrono::{Duration, NaiveDate};

use std::borrow::Cow;

use ropey::RopeSlice;

use crate::{
    textobject::{textobject_word, TextObject},
    Range, Tendril,
};

// Only support formats that aren't region specific.
static FORMATS: &[&str] = &["%Y-%m-%d", "%Y/%m/%d"];

// We don't want to parse ambiguous dates like 10/11/12 or 7/8/10.
// They must be YYYY-mm-dd or YYYY/mm/dd.
// So 2021-01-05 works, but 2021-1-5 doesn't.
const DATE_LENGTH: usize = 10;

#[derive(Debug, PartialEq, Eq)]
pub struct DateIncrementor {
    pub date: NaiveDate,
    pub range: Range,
    pub format: &'static str,
}

impl DateIncrementor {
    pub fn from_range(text: RopeSlice, range: Range) -> Option<DateIncrementor> {
        // Don't increment if the cursor is one right of the date text.
        if text.char(range.from()).is_whitespace() {
            return None;
        }

        let range = textobject_word(text, range, TextObject::Inside, 1, true);
        let text: Cow<str> = text.slice(range.from()..range.to()).into();

        let first = text.chars().next()?;
        let last = text.chars().next_back()?;

        // Allow date strings in quotes.
        let (range, text) = if first == last && (first == '"' || first == '\'') {
            (
                Range::new(range.from() + 1, range.to() - 1),
                Cow::from(&text[1..text.len() - 1]),
            )
        } else {
            (range, text)
        };

        if text.len() != DATE_LENGTH {
            return None;
        }

        FORMATS.iter().find_map(|format| {
            NaiveDate::parse_from_str(&text, format)
                .ok()
                .map(|date| DateIncrementor {
                    date,
                    range,
                    format,
                })
        })
    }

    pub fn incremented_text(&self, amount: i64) -> Tendril {
        let incremented_date = self.date + Duration::days(amount);
        incremented_date.format(self.format).to_string().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_date_dashes() {
        let rope = Rope::from_str("2021-11-15");
        let range = Range::point(0);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(0, 10),
                format: "%Y-%m-%d",
            })
        );
    }

    #[test]
    fn test_date_slashes() {
        let rope = Rope::from_str("2021/11/15");
        let range = Range::point(0);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(0, 10),
                format: "%Y/%m/%d",
            })
        );
    }

    #[test]
    fn test_date_surrounded_by_spaces() {
        let rope = Rope::from_str("   2021-11-15  ");
        let range = Range::point(10);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(3, 13),
                format: "%Y-%m-%d",
            })
        );
    }

    #[test]
    fn test_date_in_single_quotes() {
        let rope = Rope::from_str("date = '2021-11-15'");
        let range = Range::point(10);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(8, 18),
                format: "%Y-%m-%d",
            })
        );
    }

    #[test]
    fn test_date_in_double_quotes() {
        let rope = Rope::from_str("date = \"2021-11-15\"");
        let range = Range::point(10);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(8, 18),
                format: "%Y-%m-%d",
            })
        );
    }

    #[test]
    fn test_date_cursor_one_right_of_date() {
        let rope = Rope::from_str("2021-11-15 ");
        let range = Range::point(10);
        assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_date_cursor_one_left_of_number() {
        let rope = Rope::from_str(" 2021-11-15");
        let range = Range::point(0);
        assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_invalid_dates() {
        let tests = [
            "0000-00-00",
            "1980-2-21",
            "1980-12-1",
            "12345",
            "2020-02-30",
            "1999-12-32",
            "19-12-32",
            "1-2-3",
            "0000/00/00",
            "1980/2/21",
            "1980/12/1",
            "12345",
            "2020/02/30",
            "1999/12/32",
            "19/12/32",
            "1/2/3",
        ];

        for invalid in tests {
            let rope = Rope::from_str(invalid);
            let range = Range::point(0);

            assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
        }
    }

    #[test]
    fn test_increment_dates() {
        let tests = [
            ("1980-12-21", 1, "1980-12-22"),
            ("1980-12-21", -1, "1980-12-20"),
            ("1980-12-21", 100, "1981-03-31"),
            ("1980-12-21", -100, "1980-09-12"),
            ("1980-12-21", 1000, "1983-09-17"),
            ("1980-12-21", -1000, "1978-03-27"),
            ("1980/12/21", 1, "1980/12/22"),
            ("1980/12/21", -1, "1980/12/20"),
            ("1980/12/21", 100, "1981/03/31"),
            ("1980/12/21", -100, "1980/09/12"),
            ("1980/12/21", 1000, "1983/09/17"),
            ("1980/12/21", -1000, "1978/03/27"),
        ];

        for (original, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::point(0);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .incremented_text(amount),
                expected.into()
            );
        }
    }
}
