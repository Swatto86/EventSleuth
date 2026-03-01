//! Tests for the in-memory filtering logic.

use super::*;
use chrono::Utc;

fn make_event(id: u32, level: u8, provider: &str, message: &str) -> EventRecord {
    EventRecord {
        raw_xml: String::new(),
        channel: "Application".into(),
        event_id: id,
        level,
        level_name: EventRecord::level_to_name(level).into(),
        provider_name: provider.into(),
        timestamp: Utc::now(),
        computer: "TEST-PC".into(),
        message: message.into(),
        process_id: 0,
        thread_id: 0,
        task: 0,
        opcode: 0,
        keywords: 0,
        activity_id: None,
        user_sid: None,
        event_data: vec![],
    }
}

#[test]
fn test_default_matches_all() {
    let f = FilterState::default();
    let e = make_event(1001, 2, "TestProvider", "some message");
    assert!(f.matches(&e));
}

#[test]
fn test_event_id_include() {
    let mut f = FilterState::default();
    f.event_id_input = "1001, 1002".into();
    f.parse_event_ids();

    assert!(f.matches(&make_event(1001, 4, "P", "m")));
    assert!(f.matches(&make_event(1002, 4, "P", "m")));
    assert!(!f.matches(&make_event(9999, 4, "P", "m")));
}

#[test]
fn test_event_id_exclude() {
    let mut f = FilterState::default();
    f.event_id_input = "!1001".into();
    f.parse_event_ids();

    assert!(!f.matches(&make_event(1001, 4, "P", "m")));
    assert!(f.matches(&make_event(9999, 4, "P", "m")));
}

#[test]
fn test_event_id_range() {
    let mut f = FilterState::default();
    f.event_id_input = "100-105".into();
    f.parse_event_ids();

    assert!(f.matches(&make_event(100, 4, "P", "m")));
    assert!(f.matches(&make_event(103, 4, "P", "m")));
    assert!(f.matches(&make_event(105, 4, "P", "m")));
    assert!(!f.matches(&make_event(106, 4, "P", "m")));
}

#[test]
fn test_level_filter() {
    let mut f = FilterState::default();
    f.levels = [false, false, true, false, false, false]; // only Error
    assert!(f.matches(&make_event(1, 2, "P", "m"))); // Error
    assert!(!f.matches(&make_event(1, 4, "P", "m"))); // Info
}

#[test]
fn test_text_search_case_insensitive() {
    let mut f = FilterState::default();
    f.text_search = "explorer".into();
    f.parse_event_ids(); // updates text_search_lower cache
    assert!(f.matches(&make_event(1, 4, "P", "Explorer.exe crashed")));
    assert!(!f.matches(&make_event(1, 4, "P", "Nothing here")));
}

// ── Regex search tests ──────────────────────────────────────────

#[test]
fn test_regex_search_basic_pattern() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = r"crash(ed|ing)".into();
    f.parse_event_ids();
    assert!(f.matches(&make_event(1, 4, "P", "Explorer.exe crashed")));
    assert!(f.matches(&make_event(1, 4, "P", "App is crashing")));
    assert!(!f.matches(&make_event(1, 4, "P", "App is running")));
}

#[test]
fn test_regex_search_case_insensitive_default() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.case_sensitive = false;
    f.text_search = r"ERROR".into();
    f.parse_event_ids();
    assert!(f.matches(&make_event(1, 4, "P", "An error occurred")));
    assert!(f.matches(&make_event(1, 4, "P", "ERROR in module")));
}

#[test]
fn test_regex_search_case_sensitive() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.case_sensitive = true;
    f.text_search = r"ERROR".into();
    f.parse_event_ids();
    assert!(!f.matches(&make_event(1, 4, "P", "An error occurred")));
    assert!(f.matches(&make_event(1, 4, "P", "ERROR in module")));
}

#[test]
fn test_regex_invalid_pattern_matches_nothing() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = r"[invalid(".into();
    f.parse_event_ids();
    // Invalid regex should compile to None, so text_search_regex returns false
    assert!(!f.matches(&make_event(1, 4, "P", "anything")));
}

#[test]
fn test_regex_matches_provider_name() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = r"^Microsoft".into();
    f.parse_event_ids();
    assert!(f.matches(&make_event(1, 4, "Microsoft-Windows-Kernel", "m")));
    assert!(!f.matches(&make_event(1, 4, "OtherProvider", "m")));
}

#[test]
fn test_regex_empty_search_matches_all() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = String::new();
    f.parse_event_ids();
    // Empty search text should match all events regardless of regex mode
    assert!(f.matches(&make_event(1, 4, "P", "anything")));
}

#[test]
fn test_regex_search_matches_channel() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = r"^Application$".into();
    f.parse_event_ids();
    // Channel field is "Application" from make_event
    assert!(f.matches(&make_event(1001, 4, "P", "m")));
}

// ── Preset round-trip test ──────────────────────────────────────

#[test]
fn test_preset_preserves_regex_flag() {
    let mut f = FilterState::default();
    f.use_regex = true;
    f.text_search = r"\d+".into();
    f.parse_event_ids();

    let preset = crate::core::filter_preset::FilterPreset::from_state("test", &f);
    assert!(preset.use_regex);

    let restored = preset.to_filter_state();
    assert!(restored.use_regex);
    assert_eq!(restored.text_search, r"\d+");
}
