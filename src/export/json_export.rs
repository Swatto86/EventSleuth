//! JSON export for filtered event records.
//!
//! Serialises the event list as a pretty-printed JSON array using Serde.

use crate::core::event_record::EventRecord;
use crate::util::error::EventSleuthError;
use std::path::Path;

/// Export the given events to a JSON file at `path`.
///
/// Output is a pretty-printed JSON array of [`EventRecord`] objects.
///
/// # Errors
/// Returns [`EventSleuthError::Export`] if the file cannot be created or written.
pub fn export_json(events: &[EventRecord], path: &Path) -> Result<(), EventSleuthError> {
    let file = std::fs::File::create(path)
        .map_err(|e| EventSleuthError::Export(format!("Failed to create JSON file: {e}")))?;

    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, events)
        .map_err(|e| EventSleuthError::Export(format!("Failed to write JSON: {e}")))?;

    tracing::info!(
        "Exported {} events to JSON: {}",
        events.len(),
        path.display()
    );
    Ok(())
}
