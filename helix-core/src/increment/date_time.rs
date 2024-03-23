use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt::Write;

/// Increment a Date or DateTime
///
/// If just a Date is selected the day will be incremented.
/// If a DateTime is selected the second will be incremented.
pub fn increment(selected_text: &str, amount: i64) -> Option<String> {
    if selected_text.is_empty() {
        return None;
    }

    FORMATS.iter().find_map(|format| {
        let captures = format.regex.captures(selected_text)?;
        if captures.len() - 1 != format.fields.len() {
            return None;
        }

        let date_time = captures.get(0)?;
        let has_date = format.fields.iter().any(|f| f.unit.is_date());
        let has_time = format.fields.iter().any(|f| f.unit.is_time());
        let date_time = &selected_text[date_time.start()..date_time.end()];
        match (has_date, has_time) {
            (true, true) => {
                let date_time = NaiveDateTime::parse_from_str(date_time, format.fmt).ok()?;
                Some(
                    date_time
                        .checked_add_signed(Duration::try_minutes(amount)?)?
                        .format(format.fmt)
                        .to_string(),
                )
            }
            (true, false) => {
                let date = NaiveDate::parse_from_str(date_time, format.fmt).ok()?;
                Some(
                    date.checked_add_signed(Duration::try_days(amount)?)?
                        .format(format.fmt)
                        .to_string(),
                )
            }
            (false, true) => {
                let time = NaiveTime::parse_from_str(date_time, format.fmt).ok()?;
                let (adjusted_time, _) =
                    time.overflowing_add_signed(Duration::try_minutes(amount)?);
                Some(adjusted_time.format(format.fmt).to_string())
            }
            (false, false) => None,
        }
    })
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
        let mut regex = "^".to_string();
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
        regex += "$";

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_increment_date_times() {
        let tests = [
            // (original, cursor, amount, expected)
            ("2020-02-28", 1, "2020-02-29"),
            ("2020-02-29", 1, "2020-03-01"),
            ("2020-01-31", 1, "2020-02-01"),
            ("2020-01-20", 1, "2020-01-21"),
            ("2021-01-01", -1, "2020-12-31"),
            ("2021-01-31", -2, "2021-01-29"),
            ("2020-02-28", 1, "2020-02-29"),
            ("2021-02-28", 1, "2021-03-01"),
            ("2021-03-01", -1, "2021-02-28"),
            ("2020-02-29", -1, "2020-02-28"),
            ("2020-02-20", -1, "2020-02-19"),
            ("2021-03-01", -1, "2021-02-28"),
            ("1980/12/21", 100, "1981/03/31"),
            ("1980/12/21", -100, "1980/09/12"),
            ("1980/12/21", 1000, "1983/09/17"),
            ("1980/12/21", -1000, "1978/03/27"),
            ("2021-11-24 07:12:23", 1, "2021-11-24 07:13:23"),
            ("2021-11-24 07:12", 1, "2021-11-24 07:13"),
            ("Wed Nov 24 2021", 1, "Thu Nov 25 2021"),
            ("24-Nov-2021", 1, "25-Nov-2021"),
            ("2021 Nov 24", 1, "2021 Nov 25"),
            ("Nov 24, 2021", 1, "Nov 25, 2021"),
            ("7:21:53 am", 1, "7:22:53 am"),
            ("7:21:53 AM", 1, "7:22:53 AM"),
            ("7:21 am", 1, "7:22 am"),
            ("23:24:23", 1, "23:25:23"),
            ("23:24", 1, "23:25"),
            ("23:59", 1, "00:00"),
            ("23:59:59", 1, "00:00:59"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
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
            assert_eq!(increment(invalid, 1), None)
        }
    }
}
