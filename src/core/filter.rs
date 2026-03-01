//! In-memory filtering logic for EventSleuth.
//!
//! The [`FilterState`] struct holds all user-configurable filter criteria.
//! Filtering is performed in-memory against the loaded event list, with
//! checks ordered cheapest-first for short-circuit efficiency.
//!
//! [`FilterPreset`] is the serialisable subset of a filter configuration,
//! suitable for persisting named presets to disk or eframe storage.

use crate::core::event_record::EventRecord;
use std::collections::HashSet;

// ── Serialisable filter preset ──────────────────────────────────────────

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
            ..FilterState::default()
        };
        state.parse_event_ids();
        state.parse_time_range();
        state
    }
}

/// Holds all active filter criteria.
///
/// Applied in-memory against loaded events. All fields default to "pass all"
/// so that an empty `FilterState` matches every event.
#[derive(Debug, Clone)]
pub struct FilterState {
    /// Raw text from the Event ID input field.
    /// Supports: comma-separated IDs (`1001, 4625`), ranges (`4000-4999`),
    /// and negation (`!1001`).
    pub event_id_input: String,

    /// Parsed set of Event IDs to *include*. Computed from `event_id_input`.
    /// Empty means "include all".
    pub include_ids: HashSet<u32>,

    /// Parsed set of Event IDs to *exclude*. Computed from `event_id_input`.
    pub exclude_ids: HashSet<u32>,

    /// Which severity levels are enabled. Index 0..=5 corresponds to
    /// LogAlways, Critical, Error, Warning, Informational, Verbose.
    /// `true` = show events at that level.
    pub levels: [bool; 6],

    /// Provider/source name substring filter (case-insensitive).
    pub provider_filter: String,

    /// Free-form text search — matched against message, provider name,
    /// event data values, and raw XML.
    pub text_search: String,

    /// Pre-computed lowercase version of `text_search` for efficient
    /// case-insensitive matching. Updated by [`update_search_cache`].
    pub text_search_lower: String,

    /// Pre-computed lowercase version of `provider_filter` for efficient
    /// case-insensitive matching. Updated by [`update_search_cache`].
    pub provider_filter_lower: String,

    /// Start of time range filter. `None` = no lower bound.
    pub time_from_input: String,

    /// End of time range filter. `None` = no upper bound.
    pub time_to_input: String,

    /// Parsed start timestamp (computed from `time_from_input`).
    pub time_from: Option<chrono::DateTime<chrono::Utc>>,

    /// Parsed end timestamp (computed from `time_to_input`).
    pub time_to: Option<chrono::DateTime<chrono::Utc>>,

    /// Whether text search is case-sensitive.
    pub case_sensitive: bool,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            event_id_input: String::new(),
            include_ids: HashSet::new(),
            exclude_ids: HashSet::new(),
            // All levels enabled by default
            levels: [true; 6],
            provider_filter: String::new(),
            text_search: String::new(),
            text_search_lower: String::new(),
            provider_filter_lower: String::new(),
            time_from_input: String::new(),
            time_to_input: String::new(),
            time_from: None,
            time_to: None,
            case_sensitive: false,
        }
    }
}

impl FilterState {
    /// Re-parse the raw `event_id_input` string into the `include_ids` and
    /// `exclude_ids` sets. Call this whenever the input field changes.
    ///
    /// Supported syntax:
    /// - `1001` — single ID
    /// - `1001, 4625, 7036` — comma-separated
    /// - `4000-4999` — inclusive range
    /// - `!1001` — exclude this ID
    pub fn parse_event_ids(&mut self) {
        self.include_ids.clear();
        self.exclude_ids.clear();

        for token in self.event_id_input.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }

            let (negate, token) = if let Some(rest) = token.strip_prefix('!') {
                (true, rest.trim())
            } else {
                (false, token)
            };

            // Check for range syntax: "4000-4999"
            if let Some((start_str, end_str)) = token.split_once('-') {
                if let (Ok(start), Ok(end)) = (
                    start_str.trim().parse::<u32>(),
                    end_str.trim().parse::<u32>(),
                ) {
                    let (lo, hi) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    // Cap range to prevent accidental huge allocations
                    let capped_hi = hi.min(lo + 100_000);
                    for id in lo..=capped_hi {
                        if negate {
                            self.exclude_ids.insert(id);
                        } else {
                            self.include_ids.insert(id);
                        }
                    }
                }
            } else if let Ok(id) = token.parse::<u32>() {
                if negate {
                    self.exclude_ids.insert(id);
                } else {
                    self.include_ids.insert(id);
                }
            }
        }

        // Update cached lowercase search strings for case-insensitive matching.
        self.update_search_cache();
    }

    /// Refresh the cached lowercase versions of text search fields.
    ///
    /// Call this after modifying `text_search` or `provider_filter` to keep
    /// the derived caches in sync.
    pub fn update_search_cache(&mut self) {
        self.text_search_lower = self.text_search.to_lowercase();
        self.provider_filter_lower = self.provider_filter.to_lowercase();
    }

    /// Re-parse the time range input strings into `time_from` / `time_to`.
    pub fn parse_time_range(&mut self) {
        self.time_from = crate::util::time::parse_datetime_input(&self.time_from_input);
        self.time_to = crate::util::time::parse_datetime_input(&self.time_to_input);
    }

    /// Test whether the given event matches **all** active filter criteria.
    ///
    /// Checks are ordered cheapest-first for short-circuit efficiency:
    /// 1. Level (array lookup)
    /// 2. Event ID (hash set lookup)
    /// 3. Time range (comparison)
    /// 4. Provider substring
    /// 5. Text search (most expensive)
    pub fn matches(&self, event: &EventRecord) -> bool {
        // 1. Level filter — O(1) array index
        let level_idx = (event.level as usize).min(5);
        if !self.levels[level_idx] {
            return false;
        }

        // 2. Event ID filter — O(1) hash lookup
        if !self.include_ids.is_empty() && !self.include_ids.contains(&event.event_id) {
            return false;
        }
        if self.exclude_ids.contains(&event.event_id) {
            return false;
        }

        // 3. Time range — O(1) comparison
        if let Some(ref from) = self.time_from {
            if event.timestamp < *from {
                return false;
            }
        }
        if let Some(ref to) = self.time_to {
            if event.timestamp > *to {
                return false;
            }
        }

        // 4. Provider substring — O(n) where n = provider name length
        if !self.provider_filter.is_empty() {
            let provider_lower = event.provider_name.to_lowercase();
            if !provider_lower.contains(self.provider_filter_lower.as_str()) {
                return false;
            }
        }

        // 5. Text search — most expensive, checked last
        if !self.text_search.is_empty() {
            let matches = if self.case_sensitive {
                self.text_search_case_sensitive(event)
            } else {
                self.text_search_case_insensitive(event)
            };
            if !matches {
                return false;
            }
        }

        true
    }

    /// Case-sensitive text search across event fields.
    fn text_search_case_sensitive(&self, event: &EventRecord) -> bool {
        let q = &self.text_search;
        if event.message.contains(q) {
            return true;
        }
        if event.provider_name.contains(q) {
            return true;
        }
        if event.channel.contains(q) {
            return true;
        }
        for (k, v) in &event.event_data {
            if k.contains(q) || v.contains(q) {
                return true;
            }
        }
        if event.raw_xml.contains(q) {
            return true;
        }
        false
    }

    /// Case-insensitive text search across event fields.
    ///
    /// Uses `text_search_lower` (cached by `parse_event_ids`) to avoid
    /// re-allocating the lowered search term once per event.
    fn text_search_case_insensitive(&self, event: &EventRecord) -> bool {
        let q = self.text_search_lower.as_str();
        if event.message.to_lowercase().contains(q) {
            return true;
        }
        if event.provider_name.to_lowercase().contains(q) {
            return true;
        }
        if event.channel.to_lowercase().contains(q) {
            return true;
        }
        for (k, v) in &event.event_data {
            if k.to_lowercase().contains(q) || v.to_lowercase().contains(q) {
                return true;
            }
        }
        // raw_xml search is expensive, do it last
        if event.raw_xml.to_lowercase().contains(q) {
            return true;
        }
        false
    }

    /// Returns `true` if all filters are at their default (pass-all) state.
    pub fn is_empty(&self) -> bool {
        self.event_id_input.is_empty()
            && self.levels.iter().all(|&v| v)
            && self.provider_filter.is_empty()
            && self.text_search.is_empty()
            && self.time_from.is_none()
            && self.time_to.is_none()
    }

    /// Reset all filters to their default (pass-all) state.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Apply a time preset relative to now.
    pub fn apply_time_preset(&mut self, hours: i64) {
        let now = chrono::Utc::now();
        let from = now - chrono::Duration::hours(hours);
        self.time_from = Some(from);
        self.time_to = None;
        self.time_from_input = from.format("%Y-%m-%d %H:%M:%S").to_string();
        self.time_to_input.clear();
    }

    /// Apply a "Today" preset: from midnight local time today to now.
    pub fn apply_today_preset(&mut self) {
        let today_local = chrono::Local::now().date_naive().and_hms_opt(0, 0, 0);
        if let Some(naive) = today_local {
            use chrono::TimeZone;
            if let Some(local_dt) = chrono::Local.from_local_datetime(&naive).earliest() {
                let from_utc = local_dt.with_timezone(&chrono::Utc);
                self.time_from = Some(from_utc);
                self.time_to = None;
                self.time_from_input = from_utc.format("%Y-%m-%d %H:%M:%S").to_string();
                self.time_to_input.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
