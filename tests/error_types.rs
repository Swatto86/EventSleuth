//! Integration tests for error type construction and display.

use eventsleuth::util::error::{windows_err, EventSleuthError};

#[test]
fn windows_api_error_displays_hex_hresult() {
    let err = windows_err(0x80070005, "EvtQuery on Security");
    let msg = err.to_string();
    assert!(
        msg.contains("80070005"),
        "Error message should contain hex HRESULT: {msg}"
    );
    assert!(
        msg.contains("EvtQuery on Security"),
        "Error message should contain context: {msg}"
    );
}

#[test]
fn xml_parse_error_preserves_message() {
    let err = EventSleuthError::XmlParse("unexpected EOF at line 42".into());
    let msg = err.to_string();
    assert!(
        msg.contains("unexpected EOF"),
        "Should contain detail: {msg}"
    );
}

#[test]
fn export_error_preserves_message() {
    let err = EventSleuthError::Export("disk full".into());
    let msg = err.to_string();
    assert!(msg.contains("disk full"), "Should contain detail: {msg}");
}

#[test]
fn io_error_converts() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
    let err: EventSleuthError = io_err.into();
    let msg = err.to_string();
    assert!(msg.contains("no access"), "Should preserve IO error: {msg}");
}

#[test]
fn channel_enum_error_displays() {
    let err = EventSleuthError::ChannelEnum("handle invalid".into());
    let msg = err.to_string();
    assert!(
        msg.contains("handle invalid"),
        "Should contain detail: {msg}"
    );
}

#[test]
fn error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    // EventSleuthError should be thread-safe for crossbeam channels
    assert_send_sync::<EventSleuthError>();
}
