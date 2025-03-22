use std::time::{Instant, SystemTime};

use once_cell::sync::Lazy;

const SECOND: i64 = 1;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

/// Like `std::time::SystemTime::now()` but does not cause a syscall on every invocation.
///
/// There is just one syscall at the start of the program, subsequent invocations are
/// much cheaper and use the monotonic clock instead of trigerring a syscall.
fn now() -> SystemTime {
    static START_INSTANT: Lazy<Instant> = Lazy::new(Instant::now);
    static START_SYSTEM_TIME: Lazy<SystemTime> = Lazy::new(SystemTime::now);

    *START_SYSTEM_TIME + START_INSTANT.elapsed()
}

/// Formats a timestamp into a human-readable relative time string.
///
/// # Arguments
///
/// * `seconds` - Seconds since UNIX epoch (UTC)
/// * `timezone_offset` - Timezone offset in seconds
///
/// # Returns
///
/// A String representing the relative time (e.g., "4 years ago")
pub fn format_relative_time(seconds: i64, timezone_offset: i32) -> String {
    let now = now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let local_seconds = seconds + timezone_offset as i64;
    let local_now = now as i64 + timezone_offset as i64;

    let diff = local_now - local_seconds;

    let label = if diff.is_positive() {
        "ago"
    } else {
        "from now"
    };

    let (value, unit) = if diff >= YEAR {
        let years = diff / YEAR;
        (years, if years == 1 { "year" } else { "years" })
    } else if diff >= MONTH {
        let months = diff / MONTH;
        (months, if months == 1 { "month" } else { "months" })
    } else if diff >= DAY {
        let days = diff / DAY;
        (days, if days == 1 { "day" } else { "days" })
    } else if diff >= HOUR {
        let hours = diff / HOUR;
        (hours, if hours == 1 { "hour" } else { "hours" })
    } else if diff >= MINUTE {
        let minutes = diff / MINUTE;
        (minutes, if minutes == 1 { "minute" } else { "minutes" })
    } else {
        let seconds = diff / SECOND;
        (seconds, if seconds == 1 { "second" } else { "seconds" })
    };

    format!("{value} {unit} {label}")
}
