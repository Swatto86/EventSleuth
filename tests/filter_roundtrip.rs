//! Integration tests for filter logic roundtrip.

use eventsleuth::core::filter::FilterState;
use eventsleuth::core::filter_preset::FilterPreset;

#[test]
fn default_filter_matches_everything() {
    let filter = FilterState::default();
    // A default filter should not have any active criteria
    assert!(filter.event_id_input.is_empty());
    assert!(filter.text_search.is_empty());
    // All levels enabled by default
    assert!(filter.levels.iter().all(|&l| l));
}

#[test]
fn preset_roundtrip_preserves_state() {
    let filter = FilterState {
        event_id_input: "1000-2000, !1500".to_string(),
        text_search: "test query".to_string(),
        levels: [false, false, true, true, false, false],
        use_regex: false,
        ..Default::default()
    };

    let preset = FilterPreset::from_state("test_preset", &filter);
    assert_eq!(preset.name, "test_preset");

    let restored = preset.to_filter_state();
    assert_eq!(restored.event_id_input, "1000-2000, !1500");
    assert_eq!(restored.text_search, "test query");
    assert!(restored.levels[2]); // Error
    assert!(restored.levels[3]); // Warning
    assert!(!restored.levels[4]); // Info should be off
    assert!(!restored.use_regex);
}

#[test]
fn preset_serialization_roundtrip() {
    let filter = FilterState {
        text_search: "serde test".to_string(),
        levels: [false, true, false, false, false, false],
        ..Default::default()
    };

    let preset = FilterPreset::from_state("serde_test", &filter);

    // Serialize to JSON
    let json = serde_json::to_string(&preset).expect("serialize");

    // Deserialize back
    let restored: FilterPreset = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.name, "serde_test");

    let state = restored.to_filter_state();
    assert_eq!(state.text_search, "serde test");
    assert!(state.levels[1]); // Critical
}
