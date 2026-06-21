//! Logging support for `hx`.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal `log::Log` implementation: a level filter plus a single line-buffered sink.
struct Logger {
    level: log::LevelFilter,
    sink: Mutex<Box<dyn Write + Send>>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Ok(mut sink) = self.sink.lock() {
            let _ = writeln!(
                sink,
                "{} {} [{}] {}",
                log_timestamp(),
                record.target(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        if let Ok(mut sink) = self.sink.lock() {
            let _ = sink.flush();
        }
    }
}

fn install(
    level: log::LevelFilter,
    sink: Box<dyn Write + Send>,
) -> Result<(), log::SetLoggerError> {
    log::set_boxed_logger(Box::new(Logger {
        level,
        sink: Mutex::new(sink),
    }))?;
    log::set_max_level(level);
    Ok(())
}

/// Install the global logger writing to `path` (created if absent, appended to).
pub fn init_file(level: log::LevelFilter, path: &Path) -> std::io::Result<()> {
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    install(level, Box::new(file)).map_err(std::io::Error::other)
}

/// Install the global logger writing to stdout (used by integration tests).
#[cfg(feature = "integration")]
pub fn init_stdout(level: log::LevelFilter) {
    let _ = install(level, Box::new(std::io::stdout()));
}

/// RFC3339-style UTC timestamp for a log line: `YYYY-MM-DDTHH:MM:SS.mmm`.
pub fn log_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_timestamp(now.as_secs(), now.subsec_millis())
}

fn format_timestamp(secs: u64, millis: u32) -> String {
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let (hour, min, sec) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}")
}

/// Howard Hinnant's `civil_from_days`: days since the Unix epoch (1970-01-01,
/// UTC) to `(year, month, day)`. Exact for the whole proleptic Gregorian range.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn timestamp_matches_known_instants() {
        // epoch
        assert_eq!(format_timestamp(0, 0), "1970-01-01T00:00:00.000");
        // next day, and time-of-day + millis
        assert_eq!(
            format_timestamp(86_400 + 3661, 7),
            "1970-01-02T01:01:01.007"
        );
        // a leap day: 2024-02-29T12:30:45.123 UTC == 1709209845 s
        assert_eq!(
            format_timestamp(1_709_209_845, 123),
            "2024-02-29T12:30:45.123"
        );
        // 2000-03-01 (the algorithm's era boundary)
        assert_eq!(format_timestamp(951_868_800, 0), "2000-03-01T00:00:00.000");
    }
}
