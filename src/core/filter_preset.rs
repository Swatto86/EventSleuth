//! Serialisable filter preset for named filter configurations.
//!
//! [`FilterPreset`] captures the user-visible subset of [`super::filter::FilterState`]
//! and is serialised/deserialised via `serde` for persistent storage.

use super::filter::FilterState;

/// A named, serialisable snapshot of the user-visible filter fields.
///
/// Unlike [`FilterState`], this omits derived/parsed caches
/// (`include_ids`, `exclude_ids`, `time_from`, `time_to`) which are
/// recomputed from the input strings when the preset is loaded.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterPreset {
    /// Display name shown in the preset list.
    pub name: String,
    /// Raw event-ID input string (e.g. `"1001, 4000-4999"`).
    pub event_id_input: String,
    /// Enabled severity levels (index 0..=5).
    pub levels: [bool; 6],
    /// Provider substring filter.
    pub provider_filter: String,
    /// Free-form text search.
    pub text_search: String,
    /// Raw "time from" input string.
    pub time_from_input: String,
    /// Raw "time to" input string.
    pub time_to_input: String,
    /// Case-sensitive search flag.
    pub case_sensitive: bool,
    /// Whether text search uses regex instead of substring matching.
    pub use_regex: bool,
}

impl FilterPreset {
    /// Create a preset from the current [`FilterState`].
    pub fn from_state(name: &str, state: &FilterState) -> Self {
        Self {
            name: name.to_owned(),
            event_id_input: state.event_id_input.clone(),
            levels: state.levels,
            provider_filter: state.provider_filter.clone(),
            text_search: state.text_search.clone(),
            time_from_input: state.time_from_input.clone(),
            time_to_input: state.time_to_input.clone(),
            case_sensitive: state.case_sensitive,
            use_regex: state.use_regex,
        }
    }

    /// Convert this preset into a fully-parsed [`FilterState`].
    pub fn to_filter_state(&self) -> FilterState {
        let mut state = FilterState {
            event_id_input: self.event_id_input.clone(),
            levels: self.levels,
            provider_filter: self.provider_filter.clone(),
            text_search: self.text_search.clone(),
            time_from_input: self.time_from_input.clone(),
            time_to_input: self.time_to_input.clone(),
            case_sensitive: self.case_sensitive,
            use_regex: self.use_regex,
            ..FilterState::default()
        };
        state.parse_event_ids();
        state.parse_time_range();
        state
    }
}
