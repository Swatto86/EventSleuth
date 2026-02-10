//! Timestamp formatting helpers for EventSleuth.
//!
//! Provides consistent date/time display across the entire UI.

use chrono::{DateTime, Local, Utc};

/// Format a UTC timestamp for display in the event table.
///
/// Shows local time in `YYYY-MM-DD HH:MM:SS` format. This is the compact
/// format used in table rows where horizontal space is limited.
pub fn format_table_timestamp(ts: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = ts.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format a UTC timestamp for the detail panel.
///
/// Shows full precision including milliseconds and the UTC offset,
/// e.g. `2024-01-15 10:23:45.123 +00:00`.
pub fn format_detail_timestamp(ts: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = ts.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string()
}

/// Format a `std::time::Duration` into a human-readable string.
///
/// Used in the status bar to show query elapsed time.
/// Examples: `0.3s`, `1.2s`, `45.6s`.
pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 0.01 {
        format!("{:.1}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{secs:.1}s")
    } else {
        let mins = secs / 60.0;
        format!("{mins:.1}m")
    }
}

/// Parse a date-time string from user input into a UTC `DateTime`.
///
/// Accepts several common formats:
/// - `YYYY-MM-DD`
/// - `YYYY-MM-DD HH:MM`
/// - `YYYY-MM-DD HH:MM:SS`
///
/// Input is interpreted as **local time** and converted to UTC.
pub fn parse_datetime_input(input: &str) -> Option<DateTime<Utc>> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Try full datetime with seconds
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return local_naive_to_utc(naive);
    }

    // Try datetime without seconds
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M") {
        return local_naive_to_utc(naive);
    }

    // Try date only (midnight)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let naive = date.and_hms_opt(0, 0, 0)?;
        return local_naive_to_utc(naive);
    }

    None
}

/// Convert a naive local datetime to UTC.
fn local_naive_to_utc(naive: chrono::NaiveDateTime) -> Option<DateTime<Utc>> {
    use chrono::TimeZone;
    let local = Local.from_local_datetime(&naive).earliest()?;
    Some(local.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_millis() {
        let d = std::time::Duration::from_millis(5);
        let s = format_duration(d);
        assert!(s.contains("ms"), "Expected ms, got: {s}");
    }

    #[test]
    fn test_format_duration_seconds() {
        let d = std::time::Duration::from_millis(1200);
        let s = format_duration(d);
        assert_eq!(s, "1.2s");
    }

    #[test]
    fn test_parse_datetime_date_only() {
        let result = parse_datetime_input("2024-06-15");
        assert!(result.is_some(), "Should parse date-only input");
    }

    #[test]
    fn test_parse_datetime_empty() {
        assert!(parse_datetime_input("").is_none());
    }
}
