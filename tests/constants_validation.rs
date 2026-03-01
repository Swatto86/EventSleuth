//! Validates that compile-time constants are internally consistent.
#![allow(clippy::assertions_on_constants)]

use eventsleuth::util::constants::*;

#[test]
fn batch_size_is_positive() {
    assert!(EVT_BATCH_SIZE > 0, "EVT_BATCH_SIZE must be > 0");
}

#[test]
fn channel_bound_is_positive() {
    assert!(CHANNEL_BOUND > 0, "CHANNEL_BOUND must be > 0");
}

#[test]
fn max_events_is_reasonable() {
    assert!(
        MAX_EVENTS_PER_CHANNEL >= 1000,
        "MAX_EVENTS_PER_CHANNEL should be at least 1000"
    );
    assert!(
        MAX_EVENTS_PER_CHANNEL <= 100_000_000,
        "MAX_EVENTS_PER_CHANNEL should not exceed 100M"
    );
}

#[test]
fn max_errors_is_bounded() {
    assert!(MAX_ERRORS > 0, "MAX_ERRORS must be > 0");
    assert!(MAX_ERRORS <= 10_000, "MAX_ERRORS should be bounded");
}

#[test]
fn retry_constants_are_valid() {
    assert!(MAX_RETRY_ATTEMPTS >= 1, "Must retry at least once");
    assert!(MAX_RETRY_ATTEMPTS <= 10, "Excessive retries");
    assert!(RETRY_BASE_DELAY_MS > 0, "Base delay must be > 0");
    assert!(RETRY_BASE_DELAY_MS <= 1000, "Base delay too large");
}

#[test]
fn render_buffers_are_reasonable() {
    assert!(EVT_RENDER_BUFFER_SIZE >= 1024);
    assert!(EVT_FORMAT_BUFFER_SIZE >= 512);
}

#[test]
fn app_metadata_is_populated() {
    assert!(!APP_NAME.is_empty(), "APP_NAME must not be empty");
    assert!(!APP_VERSION.is_empty(), "APP_VERSION must not be empty");
    assert!(
        APP_GITHUB_URL.starts_with("https://"),
        "APP_GITHUB_URL must be HTTPS"
    );
}

#[test]
fn debounce_is_reasonable() {
    assert!(FILTER_DEBOUNCE_MS >= 50, "Debounce too low");
    assert!(FILTER_DEBOUNCE_MS <= 2000, "Debounce too high");
}

#[test]
fn live_tail_interval_is_reasonable() {
    assert!(LIVE_TAIL_INTERVAL_SECS >= 1, "Tail interval too low");
    assert!(LIVE_TAIL_INTERVAL_SECS <= 60, "Tail interval too high");
}
