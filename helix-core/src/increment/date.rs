use regex::Regex;

use std::borrow::Cow;
use std::cmp;

use ropey::RopeSlice;

use crate::{Range, Tendril};

use chrono::{Datelike, Duration, NaiveDate};

use super::Increment;

fn ndays_in_month(year: i32, month: u32) -> u32 {
    // The first day of the next month...
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let d = NaiveDate::from_ymd(y, m, 1);

    // ...is preceded by the last day of the original month.
    d.pred().day()
}

fn add_days(date: NaiveDate, amount: i64) -> Option<NaiveDate> {
    date.checked_add_signed(Duration::days(amount))
}

fn add_months(date: NaiveDate, amount: i64) -> Option<NaiveDate> {
    let month = date.month0() as i64 + amount;
    let year = date.year() + i32::try_from(month / 12).ok()?;
    let year = if month.is_negative() { year - 1 } else { year };

    // Normalize month
    let month = month % 12;
    let month = if month.is_negative() {
        month + 13
    } else {
        month + 1
    } as u32;

    let day = cmp::min(date.day(), ndays_in_month(year, month));

    Some(NaiveDate::from_ymd(year, month, day))
}

fn add_years(date: NaiveDate, amount: i64) -> Option<NaiveDate> {
    let year = i32::try_from(date.year() as i64 + amount).ok()?;
    let ndays = ndays_in_month(year, date.month());

    if date.day() > ndays {
        let d = NaiveDate::from_ymd(year, date.month(), ndays);
        Some(d.succ())
    } else {
        date.with_year(year)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Format {
    regex: &'static str,
    separator: char,
}

// Only support formats that aren't region specific.
static FORMATS: &[Format] = &[
    Format {
        regex: r"(\d{4})-(\d{2})-(\d{2})",
        separator: '-',
    },
    Format {
        regex: r"(\d{4})/(\d{2})/(\d{2})",
        separator: '/',
    },
];

const DATE_LENGTH: usize = 10;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DateField {
    Year,
    Month,
    Day,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DateIncrementor {
    date: NaiveDate,
    range: Range,
    field: DateField,
    format: Format,
}

impl DateIncrementor {
    pub fn from_range(text: RopeSlice, range: Range) -> Option<DateIncrementor> {
        let range = if range.is_empty() {
            if range.anchor < text.len_bytes() {
                // Treat empty range as a cursor range.
                range.put_cursor(text, range.anchor + 1, true)
            } else {
                // The range is empty and at the end of the text.
                return None;
            }
        } else {
            range
        };

        let from = range.from().saturating_sub(DATE_LENGTH);
        let to = (range.from() + DATE_LENGTH).min(text.len_chars());

        let (from_in_text, to_in_text) = (range.from() - from, range.to() - from);
        let text: Cow<str> = text.slice(from..to).into();

        FORMATS.iter().find_map(|&format| {
            let re = Regex::new(format.regex).ok()?;
            let captures = re.captures(&text)?;

            let date = captures.get(0)?;
            let offset = range.from() - from_in_text;
            let range = Range::new(date.start() + offset, date.end() + offset);

            let (year, month, day) = (captures.get(1)?, captures.get(2)?, captures.get(3)?);
            let (year_range, month_range, day_range) = (year.range(), month.range(), day.range());

            let field = if year_range.contains(&from_in_text)
                && year_range.contains(&(to_in_text - 1))
            {
                DateField::Year
            } else if month_range.contains(&from_in_text) && month_range.contains(&(to_in_text - 1))
            {
                DateField::Month
            } else if day_range.contains(&from_in_text) && day_range.contains(&(to_in_text - 1)) {
                DateField::Day
            } else {
                return None;
            };

            let date = NaiveDate::from_ymd_opt(
                year.as_str().parse::<i32>().ok()?,
                month.as_str().parse::<u32>().ok()?,
                day.as_str().parse::<u32>().ok()?,
            )?;

            Some(DateIncrementor {
                date,
                field,
                range,
                format,
            })
        })
    }
}

impl Increment for DateIncrementor {
    fn increment(&self, amount: i64) -> (Range, Tendril) {
        let date = match self.field {
            DateField::Year => add_years(self.date, amount),
            DateField::Month => add_months(self.date, amount),
            DateField::Day => add_days(self.date, amount),
        }
        .unwrap_or(self.date);

        (
            self.range,
            format!(
                "{:04}{}{:02}{}{:02}",
                date.year(),
                self.format.separator,
                date.month(),
                self.format.separator,
                date.day()
            )
            .into(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_create_incrementor_for_year_with_dashes() {
        let rope = Rope::from_str("2021-11-15");

        for cursor in 0..=3 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Year,
                    format: FORMATS[0],
                })
            );
        }
    }

    #[test]
    fn test_create_incrementor_for_month_with_dashes() {
        let rope = Rope::from_str("2021-11-15");

        for cursor in 5..=6 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Month,
                    format: FORMATS[0],
                })
            );
        }
    }

    #[test]
    fn test_create_incrementor_for_day_with_dashes() {
        let rope = Rope::from_str("2021-11-15");

        for cursor in 8..=9 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Day,
                    format: FORMATS[0],
                })
            );
        }
    }

    #[test]
    fn test_try_create_incrementor_on_dashes() {
        let rope = Rope::from_str("2021-11-15");

        for &cursor in &[4, 7] {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None,);
        }
    }

    #[test]
    fn test_create_incrementor_for_year_with_slashes() {
        let rope = Rope::from_str("2021/11/15");

        for cursor in 0..=3 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Year,
                    format: FORMATS[1],
                })
            );
        }
    }

    #[test]
    fn test_create_incrementor_for_month_with_slashes() {
        let rope = Rope::from_str("2021/11/15");

        for cursor in 5..=6 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Month,
                    format: FORMATS[1],
                })
            );
        }
    }

    #[test]
    fn test_create_incrementor_for_day_with_slashes() {
        let rope = Rope::from_str("2021/11/15");

        for cursor in 8..=9 {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range),
                Some(DateIncrementor {
                    date: NaiveDate::from_ymd(2021, 11, 15),
                    range: Range::new(0, 10),
                    field: DateField::Day,
                    format: FORMATS[1],
                })
            );
        }
    }

    #[test]
    fn test_try_create_incrementor_on_slashes() {
        let rope = Rope::from_str("2021/11/15");

        for &cursor in &[4, 7] {
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None,);
        }
    }

    #[test]
    fn test_date_surrounded_by_spaces() {
        let rope = Rope::from_str("   2021-11-15  ");
        let range = Range::new(3, 4);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(3, 13),
                field: DateField::Year,
                format: FORMATS[0],
            })
        );
    }

    #[test]
    fn test_date_in_single_quotes() {
        let rope = Rope::from_str("date = '2021-11-15'");
        let range = Range::new(10, 11);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(8, 18),
                field: DateField::Year,
                format: FORMATS[0],
            })
        );
    }

    #[test]
    fn test_date_in_double_quotes() {
        let rope = Rope::from_str("let date = \"2021-11-15\";");
        let range = Range::new(12, 13);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(12, 22),
                field: DateField::Year,
                format: FORMATS[0],
            })
        );
    }

    #[test]
    fn test_date_cursor_one_right_of_date() {
        let rope = Rope::from_str("2021-11-15 ");
        let range = Range::new(10, 11);
        assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_date_cursor_one_left_of_number() {
        let rope = Rope::from_str(" 2021-11-15");
        let range = Range::new(0, 1);
        assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
    }

    #[test]
    fn test_date_empty_range_at_beginning() {
        let rope = Rope::from_str("2021-11-15");
        let range = Range::point(0);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(0, 10),
                field: DateField::Year,
                format: FORMATS[0],
            })
        );
    }

    #[test]
    fn test_date_empty_range_at_in_middle() {
        let rope = Rope::from_str("2021-11-15");
        let range = Range::point(5);
        assert_eq!(
            DateIncrementor::from_range(rope.slice(..), range),
            Some(DateIncrementor {
                date: NaiveDate::from_ymd(2021, 11, 15),
                range: Range::new(0, 10),
                field: DateField::Month,
                format: FORMATS[0],
            })
        );
    }

    #[test]
    fn test_date_empty_range_at_end() {
        let rope = Rope::from_str("2021-11-15");
        let range = Range::point(10);
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
            let range = Range::new(0, 1);

            assert_eq!(DateIncrementor::from_range(rope.slice(..), range), None);
        }
    }

    #[test]
    fn test_increment_dates() {
        let tests = [
            // (original, cursor, amount, expected)
            ("2020-02-28", 0, 1, "2021-02-28"),
            ("2020-02-29", 0, 1, "2021-03-01"),
            ("2020-01-31", 5, 1, "2020-02-29"),
            ("2020-01-20", 5, 1, "2020-02-20"),
            ("2021-01-01", 5, -1, "2020-12-01"),
            ("2021-01-31", 5, -2, "2020-11-30"),
            ("2020-02-28", 8, 1, "2020-02-29"),
            ("2021-02-28", 8, 1, "2021-03-01"),
            ("2021-02-28", 0, -1, "2020-02-28"),
            ("2021-03-01", 0, -1, "2020-03-01"),
            ("2020-02-29", 5, -1, "2020-01-29"),
            ("2020-02-20", 5, -1, "2020-01-20"),
            ("2020-02-29", 8, -1, "2020-02-28"),
            ("2021-03-01", 8, -1, "2021-02-28"),
            ("1980/12/21", 8, 100, "1981/03/31"),
            ("1980/12/21", 8, -100, "1980/09/12"),
            ("1980/12/21", 8, 1000, "1983/09/17"),
            ("1980/12/21", 8, -1000, "1978/03/27"),
        ];

        for (original, cursor, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                expected.into()
            );
        }
    }
}
