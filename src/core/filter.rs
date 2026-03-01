//! In-memory filtering logic for EventSleuth.
//!
//! The [`FilterState`] struct holds all user-configurable filter criteria.
//! Filtering is performed in-memory against the loaded event list, with
//! checks ordered cheapest-first for short-circuit efficiency.
//!
//! [`FilterPreset`] lives in the sibling [`super::filter_preset`] module
//! and is re-exported here for convenience.

use crate::core::event_record::EventRecord;
use std::collections::HashSet;

/// Compiled regex for text search, when regex mode is enabled.
///
/// Wrapped in `Option` because compilation may fail for invalid patterns.
type CompiledRegex = Option<regex::Regex>;

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

    /// Whether text search uses regex patterns instead of literal substrings.
    pub use_regex: bool,

    /// Compiled regex for the current `text_search` when `use_regex` is true.
    /// `None` if the pattern is empty or invalid.
    pub compiled_regex: CompiledRegex,
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
            use_regex: false,
            compiled_regex: None,
        }
    }
}

/// Case-insensitive substring search without heap allocation for ASCII content.
///
/// Assumes `needle_lower` is already fully lowercased. Uses a fast byte-level
/// comparison for ASCII-only haystacks (typical of Windows Event Log data),
/// falling back to `to_lowercase().contains()` only when non-ASCII is detected.
fn contains_case_insensitive(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    let n = needle_lower.as_bytes();
    let h = haystack.as_bytes();
    if n.len() > h.len() {
        return false;
    }
    // Fast path: byte-level ASCII comparison (zero-alloc, covers ~99% of event log data)
    let found = h
        .windows(n.len())
        .any(|w| w.iter().zip(n).all(|(a, b)| a.to_ascii_lowercase() == *b));
    if found {
        return true;
    }
    // Slow path: full Unicode lowering only if haystack contains non-ASCII
    if !haystack.is_ascii() {
        return haystack.to_lowercase().contains(needle_lower);
    }
    false
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
                    // Cap range to prevent accidental huge allocations.
                    // Use saturating_add to avoid u32 overflow when lo is large.
                    let capped_hi = hi.min(lo.saturating_add(100_000));
                    if capped_hi < hi {
                        tracing::warn!(
                            "Event ID range {}-{} capped to {}-{} (max 100,000 IDs per range)",
                            lo,
                            hi,
                            lo,
                            capped_hi,
                        );
                    }
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

        // Also refresh the derived text-search caches. This is the
        // canonical call-site; see `update_search_cache` for the full list
        // of fields it touches.
        self.update_search_cache();
    }

    /// Refresh the cached lowercase versions of text search fields.
    ///
    /// **Must** be called after modifying `text_search` or `provider_filter`
    /// to keep the derived caches in sync. Also recompiles the regex when
    /// `use_regex` is enabled. Currently also called by
    /// [`parse_event_ids`] as a convenience, but callers that change only
    /// the text fields (without touching Event IDs) should call this
    /// method directly.
    pub fn update_search_cache(&mut self) {
        self.text_search_lower = self.text_search.to_lowercase();
        self.provider_filter_lower = self.provider_filter.to_lowercase();

        // Compile regex if in regex mode
        if self.use_regex && !self.text_search.is_empty() {
            let pattern_result = if self.case_sensitive {
                regex::RegexBuilder::new(&self.text_search).build()
            } else {
                regex::RegexBuilder::new(&self.text_search)
                    .case_insensitive(true)
                    .build()
            };
            self.compiled_regex = pattern_result.ok();
        } else {
            self.compiled_regex = None;
        }
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

        // 4. Provider substring -- zero-alloc for ASCII via contains_case_insensitive
        if !self.provider_filter.is_empty()
            && !contains_case_insensitive(&event.provider_name, &self.provider_filter_lower)
        {
            return false;
        }

        // 5. Text search — most expensive, checked last
        if !self.text_search.is_empty() {
            let matches = if self.use_regex {
                self.text_search_regex(event)
            } else if self.case_sensitive {
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

    /// Regex-based text search across event fields.
    ///
    /// Uses the pre-compiled regex from [`compiled_regex`]. Returns `false`
    /// if the regex failed to compile (invalid pattern).
    fn text_search_regex(&self, event: &EventRecord) -> bool {
        let re = match &self.compiled_regex {
            Some(re) => re,
            None => return false, // invalid regex pattern
        };
        if re.is_match(&event.message) {
            return true;
        }
        if re.is_match(&event.provider_name) {
            return true;
        }
        if re.is_match(&event.channel) {
            return true;
        }
        for (k, v) in &event.event_data {
            if re.is_match(k) || re.is_match(v) {
                return true;
            }
        }
        if re.is_match(&event.raw_xml) {
            return true;
        }
        false
    }

    /// Case-insensitive text search across event fields.
    ///
    /// Uses `text_search_lower` (cached by `parse_event_ids`) to avoid
    /// re-allocating the lowered search term once per event.
    /// Case-insensitive text search across event fields.
    ///
    /// Uses [`contains_case_insensitive`] for zero-allocation matching on
    /// ASCII content (typical of Windows Event Log data). Fields are checked
    /// cheapest-first with early return on match.
    fn text_search_case_insensitive(&self, event: &EventRecord) -> bool {
        let q = self.text_search_lower.as_str();
        if contains_case_insensitive(&event.message, q) {
            return true;
        }
        if contains_case_insensitive(&event.provider_name, q) {
            return true;
        }
        if contains_case_insensitive(&event.channel, q) {
            return true;
        }
        for (k, v) in &event.event_data {
            if contains_case_insensitive(k, q) || contains_case_insensitive(v, q) {
                return true;
            }
        }
        // raw_xml search is expensive, do it last
        if contains_case_insensitive(&event.raw_xml, q) {
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

    /// Count how many distinct filter categories are currently active.
    ///
    /// Used by the toolbar badge to give users a quick glance at how many
    /// filters are narrowing the result set.
    pub fn active_count(&self) -> usize {
        let mut n = 0usize;
        if !self.event_id_input.is_empty() {
            n += 1;
        }
        if !self.levels.iter().all(|&v| v) {
            n += 1;
        }
        if !self.provider_filter.is_empty() {
            n += 1;
        }
        if !self.text_search.is_empty() {
            n += 1;
        }
        if self.time_from.is_some() || self.time_to.is_some() {
            n += 1;
        }
        n
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
        // Display as local time since parse_datetime_input interprets input as local.
        let local_from: chrono::DateTime<chrono::Local> = from.with_timezone(&chrono::Local);
        self.time_from_input = local_from.format("%Y-%m-%d %H:%M:%S").to_string();
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
                // Display as local time (midnight) since parse_datetime_input
                // interprets input as local.
                self.time_from_input = naive.format("%Y-%m-%d %H:%M:%S").to_string();
                self.time_to_input.clear();
            }
        }
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
