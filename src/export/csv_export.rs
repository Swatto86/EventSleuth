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

    // Probe writability by creating a temporary file in the target directory.
    let probe = parent.join(".eventsleuth_write_probe");
    match std::fs::File::create(&probe) {
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
                &format_table_timestamp(&event.timestamp),
                &event.level_name,
                &event.event_id.to_string(),
                &event.provider_name,
                &event.computer,
                &event.channel,
                event.display_message(),
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
