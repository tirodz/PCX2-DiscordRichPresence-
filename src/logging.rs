//! Minimal file logger for the background helper.
//!
//! The installed application has no console, so a small log file next to
//! the executable is the only place to look when something does not work.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 512 * 1024;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("helper.log")))
        .unwrap_or_else(|| PathBuf::from("helper.log"))
}

pub fn init() {
    let path = path();

    // Start a fresh log when the previous one grew too large.
    if let Ok(metadata) = fs::metadata(&path) {
        if metadata.len() > MAX_LOG_BYTES {
            let _ = fs::remove_file(&path);
        }
    }

    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        if let Ok(mut guard) = LOG_FILE.lock() {
            *guard = Some(file);
        }
    }
}

pub fn info(message: &str) {
    write("INFO", message);
}

pub fn error(message: &str) {
    write("ERROR", message);
}

fn write(level: &str, message: &str) {
    let line = format!("{} {level:5} {message}", timestamp());
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }
    eprintln!("{line}");
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_timestamp(secs)
}

/// Format unix seconds as `YYYY-MM-DD HH:MM:SSZ` (UTC) without pulling in
/// a date-time crate.
fn format_timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let day_secs = secs % 86_400;
    let hour = day_secs / 3_600;
    let minute = (day_secs % 3_600) / 60;
    let second = day_secs % 60;

    // Civil date from day count, Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00:00Z");
        assert_eq!(format_timestamp(1_700_000_000), "2023-11-14 22:13:20Z");
        assert_eq!(format_timestamp(1_757_635_200), "2025-09-12 00:00:00Z");
        assert_eq!(format_timestamp(951_782_400), "2000-02-29 00:00:00Z");
    }
}
