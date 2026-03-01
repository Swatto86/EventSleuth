//! Integration tests for time utilities.

use eventsleuth::util::time::{format_duration, format_table_timestamp, parse_datetime_input};

#[test]
fn format_duration_sub_second() {
    let d = std::time::Duration::from_millis(350);
    let s = format_duration(d);
    assert!(
        s.contains("0.4") || s.contains("0.3"),
        "Expected ~0.3-0.4s, got: {s}"
    );
}

#[test]
fn format_duration_seconds() {
    let d = std::time::Duration::from_secs(5);
    let s = format_duration(d);
    assert_eq!(s, "5.0s");
}

#[test]
fn format_duration_minutes() {
    let d = std::time::Duration::from_secs(90);
    let s = format_duration(d);
    assert!(s.contains("1.5m"), "Expected 1.5m, got: {s}");
}

#[test]
fn format_table_timestamp_has_expected_format() {
    use chrono::TimeZone;
    let ts = chrono::Utc
        .with_ymd_and_hms(2024, 6, 15, 14, 30, 0)
        .unwrap();
    let formatted = format_table_timestamp(&ts);
    // Should contain the date portion regardless of timezone
    assert!(
        formatted.contains("2024"),
        "Should contain year: {formatted}"
    );
    assert!(
        formatted.contains("06") || formatted.contains("6"),
        "Should contain month"
    );
}

#[test]
fn parse_datetime_full() {
    let result = parse_datetime_input("2024-06-15 14:30:00");
    assert!(result.is_some(), "Should parse full datetime");
}

#[test]
fn parse_datetime_no_seconds() {
    let result = parse_datetime_input("2024-06-15 14:30");
    assert!(result.is_some(), "Should parse datetime without seconds");
}

#[test]
fn parse_datetime_date_only() {
    let result = parse_datetime_input("2024-06-15");
    assert!(result.is_some(), "Should parse date-only input");
}

#[test]
fn parse_datetime_empty_returns_none() {
    assert!(parse_datetime_input("").is_none());
    assert!(parse_datetime_input("   ").is_none());
}

#[test]
fn parse_datetime_invalid_returns_none() {
    assert!(parse_datetime_input("not-a-date").is_none());
    assert!(parse_datetime_input("2024-13-40").is_none());
}

#[test]
fn parse_datetime_whitespace_trimmed() {
    let result = parse_datetime_input("  2024-06-15  ");
    assert!(result.is_some(), "Should trim whitespace");
}
