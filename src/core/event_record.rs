//! Canonical data structure for a single Windows Event Log entry.
//!
//! Every event log entry is parsed from the XML rendered by `EvtRender` into
//! this struct. It carries both the structured fields and the original raw XML
//! for display in the detail panel.

use chrono::{DateTime, Utc};

/// Represents a single parsed Windows Event Log entry.
///
/// All fields are extracted from the XML rendered by `EvtRender`.
/// The struct is `Clone` (for UI selection) and `serde::Serialize` (for export).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EventRecord {
    /// Raw XML string as returned by `EvtRender` — retained for the detail view.
    pub raw_xml: String,

    /// The log channel this event came from (e.g. `"Application"`, `"System"`,
    /// `"Microsoft-Windows-Sysmon/Operational"`).
    pub channel: String,

    /// Event ID — the numeric identifier for this event type.
    pub event_id: u32,

    /// Severity level:
    /// - 0 = LogAlways
    /// - 1 = Critical
    /// - 2 = Error
    /// - 3 = Warning
    /// - 4 = Informational
    /// - 5 = Verbose
    pub level: u8,

    /// Human-readable level name derived from the numeric level.
    pub level_name: String,

    /// The event provider / source name.
    pub provider_name: String,

    /// Timestamp of the event in UTC.
    pub timestamp: DateTime<Utc>,

    /// The computer name where the event was generated.
    pub computer: String,

    /// The formatted / rendered message string. May be empty if the provider
    /// metadata is unavailable on this machine.
    pub message: String,

    /// Process ID that generated the event.
    pub process_id: u32,

    /// Thread ID that generated the event.
    pub thread_id: u32,

    /// Task category value.
    pub task: u16,

    /// Opcode value.
    pub opcode: u8,

    /// Keywords bitmask.
    pub keywords: u64,

    /// Correlation Activity ID, if present.
    pub activity_id: Option<String>,

    /// User SID string, if present.
    pub user_sid: Option<String>,

    /// Parsed key-value pairs from `<EventData>` or `<UserData>`.
    /// Each entry is `(name, value)`.
    pub event_data: Vec<(String, String)>,
}

impl EventRecord {
    /// Returns the human-readable level name for a given numeric level.
    ///
    /// Maps the standard ETW level values to display strings. Unknown values
    /// are formatted as `"Level(N)"`.
    pub fn level_to_name(level: u8) -> &'static str {
        match level {
            0 => "LogAlways",
            1 => "Critical",
            2 => "Error",
            3 => "Warning",
            4 => "Information",
            5 => "Verbose",
            _ => "Unknown",
        }
    }

    /// Returns a one-line summary suitable for the table's message column.
    ///
    /// If the formatted message is empty, falls back to the first event data
    /// value or a placeholder.
    pub fn display_message(&self) -> &str {
        if !self.message.is_empty() {
            &self.message
        } else if let Some((_, val)) = self.event_data.first() {
            val.as_str()
        } else {
            "(no message)"
        }
    }
}
