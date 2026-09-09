//! CSV export for filtered event records.
//!
//! Writes all currently filtered events to a CSV file with standard columns.
//! Performs pre-flight validation (Rule 17) before writing.

use crate::core::event_record::EventRecord;
use crate::util::error::EventSleuthError;
use crate::util::time::format_table_timestamp;
use std::path::Path;

/// Validate that the export destination is writable before starting.
///
/// Checks: parent directory exists, parent directory is writable (by
/// creating a temporary probe file).
pub fn validate_export_path(path: &Path) -> Result<(), EventSleuthError> {
    let parent = path
        .parent()
        .ok_or_else(|| EventSleuthError::Export("Export path has no parent directory".into()))?;

    if !parent.exists() {
        return Err(EventSleuthError::Export(format!(
            "Directory does not exist: {}. Create the directory first.",
            parent.display()
        )));
    }

    // Probe writability with an unpredictable, exclusively-created file so a
    // pre-planted file or hardlink at a guessable path can never be truncated
    // or deleted by the probe.
    let unique = format!(
        ".eventsleuth_write_probe_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let probe = parent.join(unique);
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(EventSleuthError::Export(format!(
            "Cannot write to directory {}: {}. Check permissions.",
            parent.display(),
            e
        ))),
    }
}

/// Neutralise spreadsheet formula injection in an exported CSV field.
///
/// Event Log content is attacker-influenceable, so any value that would be
/// interpreted as a formula by Excel / LibreOffice / Sheets is prefixed with
/// a literal apostrophe, which forces the cell to be treated as text.
pub fn sanitize_csv_field(value: &str) -> String {
    match value.chars().next() {
        Some('=') | Some('+') | Some('-') | Some('@') | Some('\t') | Some('\r') => {
            let mut out = String::with_capacity(value.len() + 1);
            out.push('\'');
            out.push_str(value);
            out
        }
        _ => value.to_owned(),
    }
}

/// Export the given events to a CSV file at `path`.
///
/// Columns: Timestamp, Level, EventID, Provider, Computer, Channel, Message.
///
/// # Pre-flight (Rule 17)
/// Validates that the target directory exists and is writable before writing.
///
/// # Errors
/// Returns [`EventSleuthError::Export`] if validation fails or the file
/// cannot be created or written.
pub fn export_csv(events: &[EventRecord], path: &Path) -> Result<(), EventSleuthError> {
    validate_export_path(path)?;
    let mut writer = csv::Writer::from_path(path)
        .map_err(|e| EventSleuthError::Export(format!("Failed to create CSV file: {e}")))?;

    // Write header row
    writer
        .write_record([
            "Timestamp",
            "Level",
            "EventID",
            "Provider",
            "Computer",
            "Channel",
            "Message",
        ])
        .map_err(|e| EventSleuthError::Export(format!("Failed to write CSV header: {e}")))?;

    // Write each event as a row
    for event in events {
        writer
            .write_record([
                format_table_timestamp(&event.timestamp),
                event.level_name.clone(),
                event.event_id.to_string(),
                sanitize_csv_field(&event.provider_name),
                sanitize_csv_field(&event.computer),
                sanitize_csv_field(&event.channel),
                sanitize_csv_field(event.display_message()),
            ])
            .map_err(|e| EventSleuthError::Export(format!("Failed to write CSV row: {e}")))?;
    }

    writer
        .flush()
        .map_err(|e| EventSleuthError::Export(format!("Failed to flush CSV: {e}")))?;

    tracing::info!(
        "Exported {} events to CSV: {}",
        events.len(),
        path.display()
    );
    Ok(())
}
