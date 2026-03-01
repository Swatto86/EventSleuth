//! Windows Event Log message rendering and formatting.
//!
//! Provides low-level helpers that call `EvtRender` (XML serialisation) and
//! `EvtFormatMessage` (human-readable message formatting) via the Windows
//! Evt* API. Isolated from the reader thread logic to keep individual
//! module sizes manageable.

use std::collections::HashMap;

use windows::core::PCWSTR;
use windows::Win32::System::EventLog::{
    EvtFormatMessage, EvtFormatMessageEvent, EvtOpenPublisherMetadata, EvtRender,
    EvtRenderEventXml, EVT_HANDLE,
};

use crate::util::constants::*;
use crate::util::error::EventSleuthError;

use super::event_reader::{extract_provider_name, to_wide};

/// Render a single event handle to an XML string via `EvtRender`.
///
/// Uses a caller-provided reusable buffer to avoid per-event heap allocation.
/// The buffer grows if needed and retains its size for subsequent calls.
pub(super) fn render_event_xml(
    event_handle: isize,
    buffer: &mut Vec<u16>,
) -> Result<String, EventSleuthError> {
    // Ensure minimum capacity; the buffer is reused across events.
    if buffer.len() < EVT_RENDER_BUFFER_SIZE {
        buffer.resize(EVT_RENDER_BUFFER_SIZE, 0);
    }
    let mut buffer_used = 0u32;
    let mut property_count = 0u32;

    // SAFETY: event_handle is valid, buffer is properly sized.
    // EvtRenderEventXml renders the event as a null-terminated UTF-16 string.
    let result = unsafe {
        EvtRender(
            None,
            EVT_HANDLE(event_handle),
            EvtRenderEventXml.0,
            (buffer.len() * 2) as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut buffer_used,
            &mut property_count,
        )
    };

    if let Err(e) = result {
        let code = e.code().0 as u32;
        // ERROR_INSUFFICIENT_BUFFER — HRESULT 0x8007007A: grow buffer and retry.
        // Note: windows-rs errors always surface as HRESULTs (0x8007xxxx);
        // the raw Win32 code 122 can never appear here, only the HRESULT form.
        if code == 0x8007007A {
            let needed = (buffer_used as usize / 2) + 1;
            buffer.resize(needed, 0);
            // SAFETY: retrying with larger buffer
            unsafe {
                EvtRender(
                    None,
                    EVT_HANDLE(event_handle),
                    EvtRenderEventXml.0,
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
/// in `publisher_cache`. Uses a caller-provided reusable buffer to avoid
/// per-event heap allocation.
pub(super) fn try_format_message(
    event_handle: isize,
    xml: &str,
    publisher_cache: &mut HashMap<String, EVT_HANDLE>,
    buffer: &mut Vec<u16>,
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

    // Format the message (reuse caller-provided buffer)
    if buffer.len() < EVT_FORMAT_BUFFER_SIZE {
        buffer.resize(EVT_FORMAT_BUFFER_SIZE, 0);
    }
    let mut used = 0u32;

    // SAFETY: pub_handle and event_handle are valid handles.
    // EvtFormatMessageEvent formats the event's primary message string.
    let result = unsafe {
        EvtFormatMessage(
            pub_handle,
            EVT_HANDLE(event_handle),
            0,
            None,
            EvtFormatMessageEvent.0,
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
            // ERROR_INSUFFICIENT_BUFFER — HRESULT 0x8007007A: retry with larger buffer.
            // Note: windows-rs errors always surface as HRESULTs (0x8007xxxx);
            // the raw Win32 code 122 can never appear here, only the HRESULT form.
            if code == 0x8007007A {
                buffer.resize(used as usize + 1, 0);
                // SAFETY: retrying with larger buffer
                let retry = unsafe {
                    EvtFormatMessage(
                        pub_handle,
                        EVT_HANDLE(event_handle),
                        0,
                        None,
                        EvtFormatMessageEvent.0,
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
