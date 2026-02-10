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
    EvtClose, EvtFormatMessage, EvtNext, EvtOpenPublisherMetadata, EvtQuery, EvtRender,
    EvtFormatMessageEvent, EvtQueryChannelPath, EvtQueryReverseDirection, EvtRenderEventXml,
    EVT_HANDLE,
};

use crate::core::event_record::EventRecord;
use crate::core::xml_parser::parse_event_xml;
use crate::util::constants::*;
use crate::util::error::EventSleuthError;

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
pub fn spawn_reader_thread(
    channels: Vec<String>,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("event-reader".into())
        .spawn(move || {
            reader_thread_main(channels, time_from, time_to, sender, cancel);
        })
        .expect("Failed to spawn event reader thread")
}

/// Main loop of the reader thread. Iterates over channels, reads events,
/// and sends results to the UI.
fn reader_thread_main(
    channels: Vec<String>,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: Sender<ReaderMessage>,
    cancel: Arc<AtomicBool>,
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

        match read_channel(
            channel,
            time_from,
            time_to,
            &sender,
            &cancel,
            &mut publisher_cache,
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

/// Read all events from a single channel and send them in batches.
///
/// Returns the number of events successfully read from this channel.
fn read_channel(
    channel: &str,
    time_from: Option<chrono::DateTime<chrono::Utc>>,
    time_to: Option<chrono::DateTime<chrono::Utc>>,
    sender: &Sender<ReaderMessage>,
    cancel: &Arc<AtomicBool>,
    publisher_cache: &mut HashMap<String, EVT_HANDLE>,
) -> Result<usize, EventSleuthError> {
    let xpath = build_xpath_query(time_from, time_to);
    let channel_wide = to_wide(channel);
    let xpath_wide = to_wide(&xpath);

    tracing::debug!("Querying channel '{}' with XPath: {}", channel, xpath);

    // SAFETY: We pass properly null-terminated UTF-16 strings. The session
    // handle is None (local machine). Flags request channel-path mode with
    // reverse (newest-first) ordering.
    let query_handle = unsafe {
        EvtQuery(
            None,
            PCWSTR(channel_wide.as_ptr()),
            PCWSTR(xpath_wide.as_ptr()),
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        )
    }
    .map_err(|e| EventSleuthError::WindowsApi {
        hr: e.code().0 as u32,
        context: format!("EvtQuery on channel '{channel}'"),
    })?;

    let mut count = 0usize;
    let mut handles = vec![0isize; EVT_BATCH_SIZE];

    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        // Safety limit: don't load more than MAX_EVENTS_PER_CHANNEL
        if count >= MAX_EVENTS_PER_CHANNEL {
            tracing::info!(
                "Hit event limit ({}) for channel '{}'",
                MAX_EVENTS_PER_CHANNEL,
                channel
            );
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
                // ERROR_NO_MORE_ITEMS (259 / 0x103) = normal end
                if code == 259 || code == 0x80070103 {
                    break;
                }
                // ERROR_TIMEOUT = 0x000005B4 (1460) — try again or break
                if code == 1460 || code == 0x800705B4 {
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
            _ => {}
        }

        // Process the batch of returned event handles
        let mut batch = Vec::with_capacity(returned as usize);
        for &event_handle in &handles[..returned as usize] {
            // Render the event to XML
            let xml = match render_event_xml(event_handle) {
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
            let formatted_msg = try_format_message(event_handle, &xml, publisher_cache);

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

/// Render a single event handle to an XML string via `EvtRender`.
fn render_event_xml(event_handle: isize) -> Result<String, EventSleuthError> {
    let mut buffer: Vec<u16> = vec![0; EVT_RENDER_BUFFER_SIZE];
    let mut buffer_used = 0u32;
    let mut property_count = 0u32;

    // SAFETY: event_handle is valid, buffer is properly sized.
    // EvtRenderEventXml renders the event as a null-terminated UTF-16 string.
    let result = unsafe {
        EvtRender(
            None,
            EVT_HANDLE(event_handle),
            EvtRenderEventXml.0 as u32,
            (buffer.len() * 2) as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut buffer_used,
            &mut property_count,
        )
    };

    if let Err(e) = result {
        let code = e.code().0 as u32;
        // ERROR_INSUFFICIENT_BUFFER (122) — grow and retry
        if code == 122 || code == 0x8007007A {
            let needed = (buffer_used as usize / 2) + 1;
            buffer.resize(needed, 0);
            // SAFETY: retrying with larger buffer
            unsafe {
                EvtRender(
                    None,
                    EVT_HANDLE(event_handle),
                    EvtRenderEventXml.0 as u32,
                    (buffer.len() * 2) as u32,
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut buffer_used,
                    &mut property_count,
                )
            }
            .map_err(|e| EventSleuthError::WindowsApi {
                hr: e.code().0 as u32,
                context: "EvtRender retry".into(),
            })?;
        } else {
            return Err(EventSleuthError::WindowsApi {
                hr: code,
                context: "EvtRender".into(),
            });
        }
    }

    // Convert UTF-16 to String. buffer_used is in bytes.
    let used_u16 = buffer_used as usize / 2;
    let end = if used_u16 > 0 && buffer[used_u16 - 1] == 0 {
        used_u16 - 1 // strip null terminator
    } else {
        used_u16
    };

    Ok(String::from_utf16_lossy(&buffer[..end]))
}

/// Attempt to format the event message via `EvtFormatMessage`.
///
/// Returns `Some(message)` on success, `None` if formatting fails (common
/// for events from uninstalled providers). Caches publisher metadata handles
/// in `publisher_cache`.
fn try_format_message(
    event_handle: isize,
    xml: &str,
    publisher_cache: &mut HashMap<String, EVT_HANDLE>,
) -> Option<String> {
    // Extract provider name from XML to look up publisher metadata.
    // A lightweight extraction to avoid full XML parse just for the name.
    let provider = extract_provider_name(xml)?;

    // Check cache — a cached value of 0 means we already failed for this provider.
    let pub_handle = match publisher_cache.get(&provider) {
        Some(&h) if h.0 != 0 => h,
        Some(_) => return None, // Known failure
        None => {
            // Open publisher metadata and cache the result
            let provider_wide = to_wide(&provider);
            // SAFETY: provider_wide is a valid null-terminated UTF-16 string.
            let result = unsafe {
                EvtOpenPublisherMetadata(None, PCWSTR(provider_wide.as_ptr()), None, 0, 0)
            };
            match result {
                Ok(h) => {
                    publisher_cache.insert(provider.clone(), h);
                    h
                }
                Err(_) => {
                    publisher_cache.insert(provider.clone(), EVT_HANDLE(0));
                    return None;
                }
            }
        }
    };

    // Format the message
    let mut buffer: Vec<u16> = vec![0; 4096];
    let mut used = 0u32;

    // SAFETY: pub_handle and event_handle are valid handles.
    // EvtFormatMessageEvent formats the event's primary message string.
    let result = unsafe {
        EvtFormatMessage(
            pub_handle,
            EVT_HANDLE(event_handle),
            0,
            None,
            EvtFormatMessageEvent.0 as u32,
            Some(buffer.as_mut_slice()),
            &mut used,
        )
    };

    match result {
        Ok(()) => {
            let end = if used > 0 { used as usize - 1 } else { 0 };
            let msg = String::from_utf16_lossy(&buffer[..end]);
            let trimmed = msg.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(e) => {
            let code = e.code().0 as u32;
            // ERROR_INSUFFICIENT_BUFFER — retry with larger buffer
            if code == 122 || code == 0x8007007A {
                buffer.resize(used as usize + 1, 0);
                // SAFETY: retrying with larger buffer
                let retry = unsafe {
                    EvtFormatMessage(
                        pub_handle,
                        EVT_HANDLE(event_handle),
                        0,
                        None,
                        EvtFormatMessageEvent.0 as u32,
                        Some(buffer.as_mut_slice()),
                        &mut used,
                    )
                };
                if retry.is_ok() {
                    let end = if used > 0 { used as usize - 1 } else { 0 };
                    let msg = String::from_utf16_lossy(&buffer[..end]);
                    let trimmed = msg.trim().to_string();
                    if !trimmed.is_empty() {
                        return Some(trimmed);
                    }
                }
            }
            None
        }
    }
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
fn extract_provider_name(xml: &str) -> Option<String> {
    let marker = "Provider Name=\"";
    let start = xml.find(marker)? + marker.len();
    let end = xml[start..].find('"')? + start;
    Some(xml[start..end].to_string())
}

/// Convert a `&str` to a null-terminated UTF-16 vector.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
