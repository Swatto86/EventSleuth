//! XML parser for Windows Event Log entries.
//!
//! Converts the raw XML string returned by `EvtRender` into a typed
//! [`EventRecord`]. Uses `roxmltree` for fast, zero-allocation-friendly
//! XML parsing.

use crate::core::event_record::EventRecord;
use crate::util::error::EventSleuthError;
use chrono::{DateTime, Utc};

/// Parse a raw event XML string (from `EvtRender`) into an [`EventRecord`].
///
/// The XML follows the Windows Event Log schema:
/// ```xml
/// <Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
///   <System>
///     <Provider Name="..." />
///     <EventID>1001</EventID>
///     <Level>2</Level>
///     <TimeCreated SystemTime="2024-01-15T10:23:45.1234567Z" />
///     ...
///   </System>
///   <EventData>
///     <Data Name="key">value</Data>
///     ...
///   </EventData>
/// </Event>
/// ```
///
/// # Arguments
/// - `xml`: The raw XML string from `EvtRender`.
/// - `channel`: The channel name this event was queried from (used as fallback
///   if the XML doesn't contain a `<Channel>` element).
/// - `formatted_message`: The message string from `EvtFormatMessage`, if available.
///
/// # Errors
/// Returns [`EventSleuthError::XmlParse`] if the XML is malformed or missing
/// required elements.
pub fn parse_event_xml(
    xml: &str,
    channel: &str,
    formatted_message: Option<String>,
) -> Result<EventRecord, EventSleuthError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| EventSleuthError::XmlParse(format!("Failed to parse XML: {e}")))?;

    let root = doc.root_element();

    // Find the <System> element (may be namespace-qualified)
    let system = find_child(&root, "System")
        .ok_or_else(|| EventSleuthError::XmlParse("Missing <System> element".into()))?;

    // Provider name
    let provider_name = find_child(&system, "Provider")
        .and_then(|p| p.attribute("Name").map(String::from))
        .unwrap_or_default();

    // Event ID — may have a Qualifiers attribute; we want the text content
    let event_id: u32 = find_child(&system, "EventID")
        .and_then(|e| e.text())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);

    // Level
    let level: u8 = find_child(&system, "Level")
        .and_then(|e| e.text())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);

    // TimeCreated
    let timestamp = find_child(&system, "TimeCreated")
        .and_then(|e| e.attribute("SystemTime"))
        .and_then(parse_system_time)
        .unwrap_or_else(Utc::now);

    // Computer
    let computer = find_child(&system, "Computer")
        .and_then(|e| e.text())
        .unwrap_or("")
        .to_string();

    // Channel (from XML, falling back to the parameter)
    let xml_channel = find_child(&system, "Channel")
        .and_then(|e| e.text())
        .unwrap_or("")
        .to_string();
    let channel = if xml_channel.is_empty() {
        channel.to_string()
    } else {
        xml_channel
    };

    // Process ID and Thread ID from <Execution>
    let (process_id, thread_id) = find_child(&system, "Execution")
        .map(|e| {
            let pid = e
                .attribute("ProcessID")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0u32);
            let tid = e
                .attribute("ThreadID")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0u32);
            (pid, tid)
        })
        .unwrap_or((0, 0));

    // Task
    let task: u16 = find_child(&system, "Task")
        .and_then(|e| e.text())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);

    // Opcode
    let opcode: u8 = find_child(&system, "Opcode")
        .and_then(|e| e.text())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);

    // Keywords (hex string like "0x8000000000000000")
    let keywords: u64 = find_child(&system, "Keywords")
        .and_then(|e| e.text())
        .and_then(|t| {
            let t = t.trim().trim_start_matches("0x").trim_start_matches("0X");
            u64::from_str_radix(t, 16).ok()
        })
        .unwrap_or(0);

    // Correlation Activity ID
    let activity_id = find_child(&system, "Correlation")
        .and_then(|e| e.attribute("ActivityID"))
        .map(String::from);

    // User SID
    let user_sid = find_child(&system, "Security")
        .and_then(|e| e.attribute("UserID"))
        .map(String::from);

    // Parse <EventData> or <UserData>
    let event_data = parse_event_data(&root);

    // Build message: prefer formatted message, then construct from event data
    let message = formatted_message.unwrap_or_else(|| {
        // Fallback: concatenate event data values
        if event_data.is_empty() {
            String::new()
        } else {
            event_data
                .iter()
                .map(|(k, v)| {
                    if k.is_empty() {
                        v.clone()
                    } else {
                        format!("{k}: {v}")
                    }
                })
                .collect::<Vec<_>>()
                .join("; ")
        }
    });

    let level_name = EventRecord::level_to_name(level).to_string();

    Ok(EventRecord {
        raw_xml: xml.to_string(),
        channel,
        event_id,
        level,
        level_name,
        provider_name,
        timestamp,
        computer,
        message,
        process_id,
        thread_id,
        task,
        opcode,
        keywords,
        activity_id,
        user_sid,
        event_data,
    })
}

/// Find a direct child element by local name, ignoring namespace.
fn find_child<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    local_name: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == local_name)
}

/// Parse the `SystemTime` attribute from `<TimeCreated>`.
///
/// Windows uses ISO 8601 format with varying precision:
/// - `2024-01-15T10:23:45.1234567Z`
/// - `2024-01-15T10:23:45.123Z`
/// - `2024-01-15T10:23:45Z`
fn parse_system_time(s: &str) -> Option<DateTime<Utc>> {
    // Try parsing with fractional seconds (chrono handles variable precision)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Windows sometimes emits 7-digit fractional seconds which RFC3339 doesn't
    // handle. Truncate to 6 digits (microseconds) and retry.
    if let Some(dot_pos) = s.find('.') {
        if let Some(z_pos) = s.find('Z') {
            let frac = &s[dot_pos + 1..z_pos];
            if frac.len() > 6 {
                let truncated = format!("{}.{}Z", &s[..dot_pos], &frac[..6]);
                if let Ok(dt) = DateTime::parse_from_rfc3339(&truncated) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }
    }

    // Last resort: try NaiveDateTime parsing
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }

    None
}

/// Parse `<EventData>` or `<UserData>` child elements into key-value pairs.
///
/// Handles two common patterns:
/// 1. `<Data Name="key">value</Data>` — named data items
/// 2. `<Data>value</Data>` — unnamed data items (name defaults to "Data_N")
fn parse_event_data(root: &roxmltree::Node) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    // Try <EventData> first
    if let Some(event_data) = find_child(root, "EventData") {
        let mut unnamed_idx = 0usize;
        for child in event_data.children().filter(|n| n.is_element()) {
            let name = child
                .attribute("Name")
                .map(String::from)
                .unwrap_or_else(|| {
                    unnamed_idx += 1;
                    format!("Data_{unnamed_idx}")
                });
            let value = collect_text(&child);
            pairs.push((name, value));
        }
        return pairs;
    }

    // Try <UserData>
    if let Some(user_data) = find_child(root, "UserData") {
        // UserData typically has a single wrapper element containing the data
        for wrapper in user_data.children().filter(|n| n.is_element()) {
            for child in wrapper.children().filter(|n| n.is_element()) {
                let name = child.tag_name().name().to_string();
                let value = collect_text(&child);
                pairs.push((name, value));
            }
        }
    }

    pairs
}

/// Collect all text content from a node and its descendants.
fn collect_text(node: &roxmltree::Node) -> String {
    let mut text = String::new();
    for desc in node.descendants() {
        if desc.is_text() {
            if let Some(t) = desc.text() {
                text.push_str(t);
            }
        }
    }
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
  <System>
    <Provider Name="TestProvider" />
    <EventID>1001</EventID>
    <Level>2</Level>
    <Task>0</Task>
    <Opcode>0</Opcode>
    <Keywords>0x80000000000000</Keywords>
    <TimeCreated SystemTime="2024-01-15T10:23:45.1234567Z" />
    <Execution ProcessID="4532" ThreadID="7890" />
    <Channel>Application</Channel>
    <Computer>DESKTOP-TEST</Computer>
    <Security UserID="S-1-5-21-123" />
  </System>
  <EventData>
    <Data Name="ProgramName">explorer.exe</Data>
    <Data Name="HangTime">10000</Data>
  </EventData>
</Event>"#;

    #[test]
    fn test_parse_basic_event() {
        let record = parse_event_xml(SAMPLE_XML, "Application", None).unwrap();
        assert_eq!(record.event_id, 1001);
        assert_eq!(record.level, 2);
        assert_eq!(record.provider_name, "TestProvider");
        assert_eq!(record.computer, "DESKTOP-TEST");
        assert_eq!(record.process_id, 4532);
        assert_eq!(record.thread_id, 7890);
        assert_eq!(record.event_data.len(), 2);
        assert_eq!(
            record.event_data[0],
            ("ProgramName".into(), "explorer.exe".into())
        );
        assert_eq!(record.user_sid, Some("S-1-5-21-123".into()));
    }

    #[test]
    fn test_parse_system_time_7_digits() {
        let dt = parse_system_time("2024-01-15T10:23:45.1234567Z");
        assert!(dt.is_some());
    }

    #[test]
    fn test_parse_system_time_3_digits() {
        let dt = parse_system_time("2024-01-15T10:23:45.123Z");
        assert!(dt.is_some());
    }
}
