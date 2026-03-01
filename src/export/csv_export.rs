//! CSV export for filtered event records.
//!
//! Writes all currently filtered events to a CSV file with standard columns.

use crate::core::event_record::EventRecord;
use crate::util::error::EventSleuthError;
use crate::util::time::format_table_timestamp;
use std::path::Path;

/// Export the given events to a CSV file at `path`.
///
/// Columns: Timestamp, Level, EventID, Provider, Computer, Channel, Message.
///
/// # Errors
/// Returns [`EventSleuthError::Export`] if the file cannot be created or written.
pub fn export_csv(events: &[EventRecord], path: &Path) -> Result<(), EventSleuthError> {
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
