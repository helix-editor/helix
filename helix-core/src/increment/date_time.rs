use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use once_cell::sync::Lazy;
use regex::Regex;
use ropey::RopeSlice;

use std::borrow::Cow;
use std::cmp;
use std::fmt::Write;

use super::Increment;
use crate::{Range, Tendril};

#[derive(Debug, PartialEq, Eq)]
pub struct DateTimeIncrementor {
    date_time: NaiveDateTime,
    range: Range,
    fmt: &'static str,
    field: DateField,
}

impl DateTimeIncrementor {
    pub fn from_range(text: RopeSlice, range: Range) -> Option<DateTimeIncrementor> {
        let range = if range.is_empty() {
            if range.anchor < text.len_chars() {
                // Treat empty range as a cursor range.
                range.put_cursor(text, range.anchor + 1, true)
            } else {
                // The range is empty and at the end of the text.
                return None;
            }
        } else {
            range
        };

        FORMATS.iter().find_map(|format| {
            let from = range.from().saturating_sub(format.max_len);
            let to = (range.from() + format.max_len).min(text.len_chars());

            let (from_in_text, to_in_text) = (range.from() - from, range.to() - from);
            let text: Cow<str> = text.slice(from..to).into();

            let captures = format.regex.captures(&text)?;
            if captures.len() - 1 != format.fields.len() {
                return None;
            }

            let date_time = captures.get(0)?;
            let offset = range.from() - from_in_text;
            let range = Range::new(date_time.start() + offset, date_time.end() + offset);

            let field = captures
                .iter()
                .skip(1)
                .enumerate()
                .find_map(|(i, capture)| {
                    let capture = capture?;
                    let capture_range = capture.range();

                    if capture_range.contains(&from_in_text)
                        && capture_range.contains(&(to_in_text - 1))
                    {
                        Some(format.fields[i])
                    } else {
                        None
                    }
                })?;

            let has_date = format.fields.iter().any(|f| f.unit.is_date());
            let has_time = format.fields.iter().any(|f| f.unit.is_time());

            let date_time = &text[date_time.start()..date_time.end()];
            let date_time = match (has_date, has_time) {
                (true, true) => NaiveDateTime::parse_from_str(date_time, format.fmt).ok()?,
                (true, false) => {
                    let date = NaiveDate::parse_from_str(date_time, format.fmt).ok()?;

                    date.and_hms_opt(0, 0, 0).unwrap()
                }
                (false, true) => {
                    let time = NaiveTime::parse_from_str(date_time, format.fmt).ok()?;

                    NaiveDate::from_ymd_opt(0, 1, 1).unwrap().and_time(time)
                }
                (false, false) => return None,
            };

            Some(DateTimeIncrementor {
                date_time,
                range,
                fmt: format.fmt,
                field,
            })
        })
    }
}

impl Increment for DateTimeIncrementor {
    fn increment(&self, amount: i64) -> (Range, Tendril) {
        let date_time = match self.field.unit {
            DateUnit::Years => add_years(self.date_time, amount),
            DateUnit::Months => add_months(self.date_time, amount),
            DateUnit::Days => add_duration(self.date_time, Duration::days(amount)),
            DateUnit::Hours => add_duration(self.date_time, Duration::hours(amount)),
            DateUnit::Minutes => add_duration(self.date_time, Duration::minutes(amount)),
            DateUnit::Seconds => add_duration(self.date_time, Duration::seconds(amount)),
            DateUnit::AmPm => toggle_am_pm(self.date_time),
        }
        .unwrap_or(self.date_time);

        (self.range, date_time.format(self.fmt).to_string().into())
    }
}

static FORMATS: Lazy<Vec<Format>> = Lazy::new(|| {
    vec![
        Format::new("%Y-%m-%d %H:%M:%S"), // 2021-11-24 07:12:23
        Format::new("%Y/%m/%d %H:%M:%S"), // 2021/11/24 07:12:23
        Format::new("%Y-%m-%d %H:%M"),    // 2021-11-24 07:12
        Format::new("%Y/%m/%d %H:%M"),    // 2021/11/24 07:12
        Format::new("%Y-%m-%d"),          // 2021-11-24
        Format::new("%Y/%m/%d"),          // 2021/11/24
        Format::new("%a %b %d %Y"),       // Wed Nov 24 2021
        Format::new("%d-%b-%Y"),          // 24-Nov-2021
        Format::new("%Y %b %d"),          // 2021 Nov 24
        Format::new("%b %d, %Y"),         // Nov 24, 2021
        Format::new("%-I:%M:%S %P"),      // 7:21:53 am
        Format::new("%-I:%M %P"),         // 7:21 am
        Format::new("%-I:%M:%S %p"),      // 7:21:53 AM
        Format::new("%-I:%M %p"),         // 7:21 AM
        Format::new("%H:%M:%S"),          // 23:24:23
        Format::new("%H:%M"),             // 23:24
    ]
});

#[derive(Debug)]
struct Format {
    fmt: &'static str,
    fields: Vec<DateField>,
    regex: Regex,
    max_len: usize,
}

impl Format {
    fn new(fmt: &'static str) -> Self {
        let mut remaining = fmt;
        let mut fields = Vec::new();
        let mut regex = String::new();
        let mut max_len = 0;

        while let Some(i) = remaining.find('%') {
            let after = &remaining[i + 1..];
            let mut chars = after.chars();
            let c = chars.next().unwrap();

            let spec_len = if c == '-' {
                1 + chars.next().unwrap().len_utf8()
            } else {
                c.len_utf8()
            };

            let specifier = &after[..spec_len];
            let field = DateField::from_specifier(specifier).unwrap();
            fields.push(field);
            max_len += field.max_len + remaining[..i].len();
            regex += &remaining[..i];
            write!(regex, "({})", field.regex).unwrap();
            remaining = &after[spec_len..];
        }

        let regex = Regex::new(&regex).unwrap();

        Self {
            fmt,
            fields,
            regex,
            max_len,
        }
    }
}

impl PartialEq for Format {
    fn eq(&self, other: &Self) -> bool {
        self.fmt == other.fmt && self.fields == other.fields && self.max_len == other.max_len
    }
}

impl Eq for Format {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct DateField {
    regex: &'static str,
    unit: DateUnit,
    max_len: usize,
}

impl DateField {
    fn from_specifier(specifier: &str) -> Option<Self> {
        match specifier {
            "Y" => Some(Self {
                regex: r"\d{4}",
                unit: DateUnit::Years,
                max_len: 5,
            }),
            "y" => Some(Self {
                regex: r"\d\d",
                unit: DateUnit::Years,
                max_len: 2,
            }),
            "m" => Some(Self {
                regex: r"[0-1]\d",
                unit: DateUnit::Months,
                max_len: 2,
            }),
            "d" => Some(Self {
                regex: r"[0-3]\d",
                unit: DateUnit::Days,
                max_len: 2,
            }),
            "-d" => Some(Self {
                regex: r"[1-3]?\d",
                unit: DateUnit::Days,
                max_len: 2,
            }),
            "a" => Some(Self {
                regex: r"Sun|Mon|Tue|Wed|Thu|Fri|Sat",
                unit: DateUnit::Days,
                max_len: 3,
            }),
            "A" => Some(Self {
                regex: r"Sunday|Monday|Tuesday|Wednesday|Thursday|Friday|Saturday",
                unit: DateUnit::Days,
                max_len: 9,
            }),
            "b" | "h" => Some(Self {
                regex: r"Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec",
                unit: DateUnit::Months,
                max_len: 3,
            }),
            "B" => Some(Self {
                regex: r"January|February|March|April|May|June|July|August|September|October|November|December",
                unit: DateUnit::Months,
                max_len: 9,
            }),
            "H" => Some(Self {
                regex: r"[0-2]\d",
                unit: DateUnit::Hours,
                max_len: 2,
            }),
            "M" => Some(Self {
                regex: r"[0-5]\d",
                unit: DateUnit::Minutes,
                max_len: 2,
            }),
            "S" => Some(Self {
                regex: r"[0-5]\d",
                unit: DateUnit::Seconds,
                max_len: 2,
            }),
            "I" => Some(Self {
                regex: r"[0-1]\d",
                unit: DateUnit::Hours,
                max_len: 2,
            }),
            "-I" => Some(Self {
                regex: r"1?\d",
                unit: DateUnit::Hours,
                max_len: 2,
            }),
            "P" => Some(Self {
                regex: r"am|pm",
                unit: DateUnit::AmPm,
                max_len: 2,
            }),
            "p" => Some(Self {
                regex: r"AM|PM",
                unit: DateUnit::AmPm,
                max_len: 2,
            }),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DateUnit {
    Years,
    Months,
    Days,
    Hours,
    Minutes,
    Seconds,
    AmPm,
}

impl DateUnit {
    fn is_date(self) -> bool {
        matches!(self, DateUnit::Years | DateUnit::Months | DateUnit::Days)
    }

    fn is_time(self) -> bool {
        matches!(
            self,
            DateUnit::Hours | DateUnit::Minutes | DateUnit::Seconds
        )
    }
}

fn ndays_in_month(year: i32, month: u32) -> u32 {
    // The first day of the next month...
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let d = NaiveDate::from_ymd_opt(y, m, 1).unwrap();

    // ...is preceded by the last day of the original month.
    d.pred_opt().unwrap().day()
}

fn add_months(date_time: NaiveDateTime, amount: i64) -> Option<NaiveDateTime> {
    let month = (date_time.month0() as i64).checked_add(amount)?;
    let year = date_time.year() + i32::try_from(month / 12).ok()?;
    let year = if month.is_negative() { year - 1 } else { year };

    // Normalize month
    let month = month % 12;
    let month = if month.is_negative() {
        month + 12
    } else {
        month
    } as u32
        + 1;

    let day = cmp::min(date_time.day(), ndays_in_month(year, month));

    NaiveDate::from_ymd_opt(year, month, day).map(|date| date.and_time(date_time.time()))
}

fn add_years(date_time: NaiveDateTime, amount: i64) -> Option<NaiveDateTime> {
    let year = i32::try_from((date_time.year() as i64).checked_add(amount)?).ok()?;
    let ndays = ndays_in_month(year, date_time.month());

    if date_time.day() > ndays {
        NaiveDate::from_ymd_opt(year, date_time.month(), ndays)
            .and_then(|date| date.succ_opt().map(|date| date.and_time(date_time.time())))
    } else {
        date_time.with_year(year)
    }
}

fn add_duration(date_time: NaiveDateTime, duration: Duration) -> Option<NaiveDateTime> {
    date_time.checked_add_signed(duration)
}

fn toggle_am_pm(date_time: NaiveDateTime) -> Option<NaiveDateTime> {
    if date_time.hour() < 12 {
        add_duration(date_time, Duration::hours(12))
    } else {
        add_duration(date_time, Duration::hours(-12))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_increment_date_times() {
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
            ("2021-11-24 07:12:23", 0, 1, "2022-11-24 07:12:23"),
            ("2021-11-24 07:12:23", 5, 1, "2021-12-24 07:12:23"),
            ("2021-11-24 07:12:23", 8, 1, "2021-11-25 07:12:23"),
            ("2021-11-24 07:12:23", 11, 1, "2021-11-24 08:12:23"),
            ("2021-11-24 07:12:23", 14, 1, "2021-11-24 07:13:23"),
            ("2021-11-24 07:12:23", 17, 1, "2021-11-24 07:12:24"),
            ("2021/11/24 07:12:23", 0, 1, "2022/11/24 07:12:23"),
            ("2021/11/24 07:12:23", 5, 1, "2021/12/24 07:12:23"),
            ("2021/11/24 07:12:23", 8, 1, "2021/11/25 07:12:23"),
            ("2021/11/24 07:12:23", 11, 1, "2021/11/24 08:12:23"),
            ("2021/11/24 07:12:23", 14, 1, "2021/11/24 07:13:23"),
            ("2021/11/24 07:12:23", 17, 1, "2021/11/24 07:12:24"),
            ("2021-11-24 07:12", 0, 1, "2022-11-24 07:12"),
            ("2021-11-24 07:12", 5, 1, "2021-12-24 07:12"),
            ("2021-11-24 07:12", 8, 1, "2021-11-25 07:12"),
            ("2021-11-24 07:12", 11, 1, "2021-11-24 08:12"),
            ("2021-11-24 07:12", 14, 1, "2021-11-24 07:13"),
            ("2021/11/24 07:12", 0, 1, "2022/11/24 07:12"),
            ("2021/11/24 07:12", 5, 1, "2021/12/24 07:12"),
            ("2021/11/24 07:12", 8, 1, "2021/11/25 07:12"),
            ("2021/11/24 07:12", 11, 1, "2021/11/24 08:12"),
            ("2021/11/24 07:12", 14, 1, "2021/11/24 07:13"),
            ("Wed Nov 24 2021", 0, 1, "Thu Nov 25 2021"),
            ("Wed Nov 24 2021", 4, 1, "Fri Dec 24 2021"),
            ("Wed Nov 24 2021", 8, 1, "Thu Nov 25 2021"),
            ("Wed Nov 24 2021", 11, 1, "Thu Nov 24 2022"),
            ("24-Nov-2021", 0, 1, "25-Nov-2021"),
            ("24-Nov-2021", 3, 1, "24-Dec-2021"),
            ("24-Nov-2021", 7, 1, "24-Nov-2022"),
            ("2021 Nov 24", 0, 1, "2022 Nov 24"),
            ("2021 Nov 24", 5, 1, "2021 Dec 24"),
            ("2021 Nov 24", 9, 1, "2021 Nov 25"),
            ("Nov 24, 2021", 0, 1, "Dec 24, 2021"),
            ("Nov 24, 2021", 4, 1, "Nov 25, 2021"),
            ("Nov 24, 2021", 8, 1, "Nov 24, 2022"),
            ("7:21:53 am", 0, 1, "8:21:53 am"),
            ("7:21:53 am", 3, 1, "7:22:53 am"),
            ("7:21:53 am", 5, 1, "7:21:54 am"),
            ("7:21:53 am", 8, 1, "7:21:53 pm"),
            ("7:21:53 AM", 0, 1, "8:21:53 AM"),
            ("7:21:53 AM", 3, 1, "7:22:53 AM"),
            ("7:21:53 AM", 5, 1, "7:21:54 AM"),
            ("7:21:53 AM", 8, 1, "7:21:53 PM"),
            ("7:21 am", 0, 1, "8:21 am"),
            ("7:21 am", 3, 1, "7:22 am"),
            ("7:21 am", 5, 1, "7:21 pm"),
            ("7:21 AM", 0, 1, "8:21 AM"),
            ("7:21 AM", 3, 1, "7:22 AM"),
            ("7:21 AM", 5, 1, "7:21 PM"),
            ("23:24:23", 1, 1, "00:24:23"),
            ("23:24:23", 3, 1, "23:25:23"),
            ("23:24:23", 6, 1, "23:24:24"),
            ("23:24", 1, 1, "00:24"),
            ("23:24", 3, 1, "23:25"),
        ];

        for (original, cursor, amount, expected) in tests {
            let rope = Rope::from_str(original);
            let range = Range::new(cursor, cursor + 1);
            assert_eq!(
                DateTimeIncrementor::from_range(rope.slice(..), range)
                    .unwrap()
                    .increment(amount)
                    .1,
                Tendril::from(expected)
            );
        }
    }

    #[test]
    fn test_invalid_date_times() {
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
            "123:456:789",
            "11:61",
            "2021-55-12 08:12:54",
        ];

        for invalid in tests {
            let rope = Rope::from_str(invalid);
            let range = Range::new(0, 1);

            assert_eq!(DateTimeIncrementor::from_range(rope.slice(..), range), None)
        }
    }
}
