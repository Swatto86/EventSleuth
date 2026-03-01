//! Background event log reader thread.
//!
//! Queries Windows Event Log channels using the modern Evt* API on a
//! background thread. Parsed [`EventRecord`] batches are sent to the UI
//! via a [`crossbeam_channel`] sender. The UI polls the receiving end
//! each frame with non-blocking `try_recv`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;
use windows::core::PCWSTR;
use windows::Win32::System::EventLog::{
    EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryFilePath, EvtQueryReverseDirection,
    EVT_HANDLE,
};

use super::event_format::{render_event_xml, try_format_message};
use crate::core::event_record::EventRecord;
use crate::core::xml_parser::parse_event_xml;
use crate::util::constants::*;
use crate::util::error::EventSleuthError;

/// HRESULT codes considered transient (worth retrying).
///
/// These correspond to conditions that may resolve on their own:
/// busy handles, RPC timeouts, service restarts, file locks.
const TRANSIENT_HRESULTS: &[u32] = &[
    0x800706BA, // RPC server unavailable
    0x800706BB, // RPC server too busy
    0x800706BF, // RPC connection aborted
    0x80070020, // ERROR_SHARING_VIOLATION (file lock)
    0x8007045B, // ERROR_SHUTDOWN_IN_PROGRESS
    0x00000005, // ERROR_ACCESS_DENIED (transient during service restart)
    0x80070015, // ERROR_NOT_READY
    1460,       // ERROR_TIMEOUT (raw Win32)
    0x800705B4, // ERROR_TIMEOUT (HRESULT)
];

/// Check whether a Windows error code is considered transient.
fn is_transient_error(code: u32) -> bool {
    TRANSIENT_HRESULTS.contains(&code)
}

/// Messages sent from the background reader thread to the UI thread.
#[derive(Debug)]
pub enum ReaderMessage {
    /// A batch of parsed events ready to append to the display list.
    EventBatch(Vec<EventRecord>),
    /// Progress update: total events read so far and current channel name.
    Progress { count: usize, channel: String },
    /// Reading is complete for all requested channels.
    Complete {
        total: usize,
        elapsed: std::time::Duration,
    },
    /// An error occurred reading a specific channel. Non-fatal — other
    /// channels continue.
    Error { channel: String, error: String },
}

/// Spawn a background thread that reads events from the given channels.
///
/// Events are sent as batches via the `sender` channel. Set `cancel` to
/// `true` (via `AtomicBool`) to request graceful termination.
///
/// # Arguments
/// - `channels`: Channel names to query (e.g. `["Application", "System"]`)
/// - `time_from` / `time_to`: Optional time bounds pushed into the XPath query
/// - `sender`: Channel sender for [`ReaderMessage`] batches
/// - `cancel`: Shared flag to signal cancellation
/// - `max_events`: Maximum events per channel before stopping
pub fn spawn_reader_thread(
    channels: Vec<String>,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
    max_events: usize,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("event-reader".into())
        .spawn(move || {
            reader_thread_main(channels, time_from, time_to, sender, cancel, max_events);
        })
        .expect("Failed to spawn event reader thread")
}

/// Spawn a background thread that reads events from a local `.evtx` file.
///
/// Uses `EvtQueryFilePath` instead of `EvtQueryChannelPath` so that the
/// Evt* API reads directly from a file on disk rather than a live channel.
pub fn spawn_file_reader_thread(
    file_path: std::path::PathBuf,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
    max_events: usize,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("evtx-reader".into())
        .spawn(move || {
            file_reader_thread_main(file_path, time_from, time_to, sender, cancel, max_events);
        })
        .expect("Failed to spawn .evtx file reader thread")
}

/// Main loop of the reader thread. Iterates over channels, reads events,
/// and sends results to the UI.
fn reader_thread_main(
    channels: Vec<String>,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
    max_events: usize,
) {
    let start = Instant::now();
    let mut total = 0usize;

    // Cache publisher metadata handles to avoid re-opening per event.
    // Key = provider name, Value = handle (EVT_HANDLE(0) = failed/not-cached).
    let mut publisher_cache: HashMap<String, EVT_HANDLE> = HashMap::new();

    for channel in &channels {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let flags = EvtQueryChannelPath.0 | EvtQueryReverseDirection.0;
        match read_channel(
            channel,
            flags,
            time_from,
            time_to,
            &sender,
            &cancel,
            &mut publisher_cache,
            max_events,
        ) {
            Ok(count) => {
                total += count;
                let _ = sender.send(ReaderMessage::Progress {
                    count: total,
                    channel: channel.clone(),
                });
            }
            Err(e) => {
                tracing::warn!("Error reading channel '{}': {}", channel, e);
                let _ = sender.send(ReaderMessage::Error {
                    channel: channel.clone(),
                    error: e.to_string(),
                });
            }
        }
    }

    // Close all cached publisher metadata handles.
    for (name, handle) in publisher_cache.drain() {
        if handle.0 != 0 {
            // SAFETY: handle is a valid publisher metadata handle
            // that we opened with EvtOpenPublisherMetadata.
            unsafe {
                let _ = EvtClose(handle);
            }
            tracing::trace!("Closed publisher metadata for '{}'", name);
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(
        "Reader complete: {} events from {} channels in {:.2}s",
        total,
        channels.len(),
        elapsed.as_secs_f64()
    );
    let _ = sender.send(ReaderMessage::Complete { total, elapsed });
}

/// Main loop for reading events from a local `.evtx` file.
fn file_reader_thread_main(
    file_path: std::path::PathBuf,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
    max_events: usize,
) {
    let start = Instant::now();
    let mut publisher_cache: HashMap<String, EVT_HANDLE> = HashMap::new();

    let display_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "evtx".into());

    let path_str = file_path.to_string_lossy().into_owned();
    let flags = EvtQueryFilePath.0 | EvtQueryReverseDirection.0;

    let total = match read_channel(
        &path_str,
        flags,
        time_from,
        time_to,
        &sender,
        &cancel,
        &mut publisher_cache,
        max_events,
    ) {
        Ok(count) => {
            let _ = sender.send(ReaderMessage::Progress {
                count,
                channel: display_name.clone(),
            });
            count
        }
        Err(e) => {
            tracing::warn!("Error reading file '{}': {}", display_name, e);
            let _ = sender.send(ReaderMessage::Error {
                channel: display_name.clone(),
                error: e.to_string(),
            });
            0
        }
    };

    // Close cached publisher handles
    for (name, handle) in publisher_cache.drain() {
        if handle.0 != 0 {
            unsafe {
                let _ = EvtClose(handle);
            }
            tracing::trace!("Closed publisher metadata for '{}'", name);
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(
        "File reader complete: {} events from '{}' in {:.2}s",
        total,
        display_name,
        elapsed.as_secs_f64()
    );
    let _ = sender.send(ReaderMessage::Complete { total, elapsed });
}

/// Read all events from a single channel and send them in batches.
///
/// Returns the number of events successfully read from this channel.
/// The `query_flags` parameter controls whether this is a live channel
/// query (`EvtQueryChannelPath`) or a file query (`EvtQueryFilePath`).
#[allow(clippy::too_many_arguments)]
fn read_channel(
    channel: &str,
    query_flags: u32,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: &Sender<ReaderMessage>,
    cancel: &Arc<AtomicBool>,
    publisher_cache: &mut HashMap<String, EVT_HANDLE>,
    max_events: usize,
) -> Result<usize, EventSleuthError> {
    let xpath = build_xpath_query(time_from, time_to);
    let channel_wide = to_wide(channel);
    let xpath_wide = to_wide(&xpath);

    tracing::debug!("Querying channel '{}' with XPath: {}", channel, xpath);

    // Open the query with retry for transient failures (Rule 11).
    let query_handle = retry_transient(|| {
        // SAFETY: We pass properly null-terminated UTF-16 strings. The session
        // handle is None (local machine). Flags are provided by the caller.
        unsafe {
            EvtQuery(
                None,
                PCWSTR(channel_wide.as_ptr()),
                PCWSTR(xpath_wide.as_ptr()),
                query_flags,
            )
        }
        .map_err(|e| EventSleuthError::WindowsApi {
            hr: e.code().0 as u32,
            context: format!("EvtQuery on channel '{channel}'"),
        })
    })?;

    let mut count = 0usize;
    let mut handles = vec![0isize; EVT_BATCH_SIZE];

    // Reusable buffers shared across all events in this channel read,
    // eliminating per-event heap allocations for EvtRender/EvtFormatMessage.
    let mut render_buf: Vec<u16> = vec![0; EVT_RENDER_BUFFER_SIZE];
    let mut format_buf: Vec<u16> = vec![0; EVT_FORMAT_BUFFER_SIZE];

    // Retry counter for EvtNext timeouts. The Event Log service can be
    // temporarily slow under load; a timeout on `EvtNext` does not mean
    // there are no more events — retrying is the correct response (Rule 11).
    let mut timeout_retries = 0u32;

    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        // Safety limit: don't load more than max_events
        if count >= max_events {
            tracing::info!("Hit event limit ({}) for channel '{}'", max_events, channel);
            break;
        }

        let mut returned = 0u32;

        // SAFETY: query_handle is valid, handles array has EVT_BATCH_SIZE
        // slots, returned will receive the actual count.
        let result = unsafe {
            EvtNext(
                query_handle,
                &mut handles,
                EVT_NEXT_TIMEOUT_MS,
                0,
                &mut returned,
            )
        };

        match result {
            Ok(()) if returned == 0 => break,
            Err(e) => {
                let code = e.code().0 as u32;
                // ERROR_NO_MORE_ITEMS (259 / 0x103) = normal end of results
                if code == 259 || code == 0x80070103 {
                    break;
                }
                // ERROR_TIMEOUT (1460 / 0x800705B4): the Event Log service
                // was slow responding.  Retry up to MAX_RETRY_ATTEMPTS times
                // (with a small sleep so we don't spin) before giving up.
                // Previously this immediately broke the loop, silently
                // truncating the channel read on busy systems.
                if code == 1460 || code == 0x800705B4 {
                    timeout_retries += 1;
                    if timeout_retries <= MAX_RETRY_ATTEMPTS {
                        let delay_ms = RETRY_BASE_DELAY_MS * (1u64 << (timeout_retries - 1));
                        tracing::debug!(
                            "EvtNext timeout on '{}' (retry {}/{}), waiting {}ms",
                            channel,
                            timeout_retries,
                            MAX_RETRY_ATTEMPTS,
                            delay_ms,
                        );
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        continue;
                    }
                    tracing::warn!(
                        "EvtNext timed out after {} retries on channel '{}', read may be incomplete",
                        MAX_RETRY_ATTEMPTS,
                        channel,
                    );
                    break;
                }
                // Close query handle before returning error
                unsafe {
                    let _ = EvtClose(query_handle);
                }
                return Err(EventSleuthError::WindowsApi {
                    hr: code,
                    context: format!("EvtNext on channel '{channel}'"),
                });
            }
            _ => {
                // Successful batch received — reset the timeout retry counter.
                timeout_retries = 0;
            }
        }

        // Process the batch of returned event handles
        let mut batch = Vec::with_capacity(returned as usize);
        for &event_handle in &handles[..returned as usize] {
            // Render the event to XML
            let xml = match render_event_xml(event_handle, &mut render_buf) {
                Ok(xml) => xml,
                Err(e) => {
                    tracing::trace!("Failed to render event XML: {}", e);
                    // SAFETY: event_handle is valid from EvtNext
                    unsafe {
                        let _ = EvtClose(EVT_HANDLE(event_handle));
                    }
                    continue;
                }
            };

            // Try to format the message via EvtFormatMessage
            let formatted_msg =
                try_format_message(event_handle, &xml, publisher_cache, &mut format_buf);

            // Parse XML into an EventRecord
            match parse_event_xml(&xml, channel, formatted_msg) {
                Ok(record) => batch.push(record),
                Err(e) => {
                    tracing::trace!("Failed to parse event XML: {}", e);
                }
            }

            // SAFETY: we're done with this event handle
            unsafe {
                let _ = EvtClose(EVT_HANDLE(event_handle));
            }
        }

        count += batch.len();
        if !batch.is_empty() {
            let _ = sender.send(ReaderMessage::EventBatch(batch));
        }
    }

    // SAFETY: query_handle is valid and we're done with it
    unsafe {
        let _ = EvtClose(query_handle);
    }

    tracing::debug!("Read {} events from channel '{}'", count, channel);
    Ok(count)
}

/// Build an XPath query string for server-side pre-filtering.
///
/// Pushes time range predicates into the XPath to reduce the volume of
/// events returned by the API before in-memory filtering.
fn build_xpath_query(
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    let mut conditions = Vec::new();

    if let Some(from) = time_from {
        conditions.push(format!(
            "TimeCreated[@SystemTime >= '{}']",
            from.format("%Y-%m-%dT%H:%M:%S%.3fZ")
        ));
    }
    if let Some(to) = time_to {
        conditions.push(format!(
            "TimeCreated[@SystemTime <= '{}']",
            to.format("%Y-%m-%dT%H:%M:%S%.3fZ")
        ));
    }

    if conditions.is_empty() {
        "*".to_string()
    } else {
        format!("*[System[{}]]", conditions.join(" and "))
    }
}

/// Quick extraction of the `Provider Name` attribute from raw event XML.
///
/// Avoids a full XML parse just to get the provider name for publisher
/// metadata lookup. Looks for `Provider Name="..."` in the string.
pub(super) fn extract_provider_name(xml: &str) -> Option<String> {
    let marker = "Provider Name=\"";
    let start = xml.find(marker)? + marker.len();
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_string())
}

/// Convert a `&str` to a null-terminated UTF-16 vector.
pub(super) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Retry a fallible operation with capped exponential backoff for transient
/// Windows API errors (Rule 11).
///
/// Attempts the operation up to [`MAX_RETRY_ATTEMPTS`] times. On each
/// transient failure the thread sleeps for `RETRY_BASE_DELAY_MS * 2^attempt`
/// milliseconds before retrying. Permanent errors are returned immediately.
fn retry_transient<T, F>(mut op: F) -> Result<T, EventSleuthError>
where
    F: FnMut() -> Result<T, EventSleuthError>,
{
    let mut attempt = 0u32;
    loop {
        match op() {
            Ok(val) => return Ok(val),
            Err(e) => {
                let transient = matches!(&e, EventSleuthError::WindowsApi { hr, .. } if is_transient_error(*hr));
                attempt += 1;
                if !transient || attempt > MAX_RETRY_ATTEMPTS {
                    if transient {
                        tracing::warn!(
                            "Transient error persisted after {} retries: {}",
                            attempt - 1,
                            e
                        );
                    }
                    return Err(e);
                }
                // Delay sequence: 50ms -> 100ms -> 200ms (base * 2^(attempt-1))
                let delay_ms = RETRY_BASE_DELAY_MS * (1u64 << (attempt - 1));
                tracing::debug!(
                    "Transient error (retry {}/{}), retrying in {}ms: {}",
                    attempt,
                    MAX_RETRY_ATTEMPTS,
                    delay_ms,
                    e
                );
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_xpath_no_filters() {
        let xpath = build_xpath_query(None, None);
        assert_eq!(xpath, "*");
    }

    #[test]
    fn test_build_xpath_with_time_from() {
        use chrono::TimeZone;
        let from = chrono::Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let xpath = build_xpath_query(Some(from), None);
        assert!(xpath.contains("TimeCreated"));
        assert!(xpath.contains("2024-01-15"));
    }

    #[test]
    fn test_extract_provider_name() {
        let xml = r#"<Event><System><Provider Name="TestProvider" /></System></Event>"#;
        assert_eq!(extract_provider_name(xml), Some("TestProvider".into()));
    }

    #[test]
    fn test_extract_provider_name_missing() {
        let xml = "<Event><System></System></Event>";
        assert_eq!(extract_provider_name(xml), None);
    }

    #[test]
    fn test_to_wide() {
        let wide = to_wide("AB");
        assert_eq!(wide, vec![0x41, 0x42, 0x00]);
    }
}
