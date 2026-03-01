//! Channel enumeration via the Windows Evt* API.
//!
//! Discovers all available event log channels on the system using
//! `EvtOpenChannelEnum` and `EvtNextChannelPath`. This includes standard
//! channels (Application, System, Security) as well as all operational
//! and analytic channels under `Microsoft-Windows-*`.

use crate::util::error::EventSleuthError;
use windows::Win32::System::EventLog::{EvtClose, EvtNextChannelPath, EvtOpenChannelEnum};

/// Enumerate all available event log channels on the local system.
///
/// Returns a sorted list of channel path strings. Channels that cannot be
/// read (e.g. due to access restrictions) are still listed — access errors
/// are surfaced later when the user tries to query them.
///
/// # Errors
/// Returns [`EventSleuthError::ChannelEnum`] if the enumeration handle
/// cannot be opened.
pub fn enumerate_channels() -> Result<Vec<String>, EventSleuthError> {
    let mut channels = Vec::with_capacity(256);

    // SAFETY: EvtOpenChannelEnum with a null session handle opens a local
    // enumeration. The returned handle is valid until closed with EvtClose.
    let handle = unsafe { EvtOpenChannelEnum(None, 0) }
        .map_err(|e| EventSleuthError::ChannelEnum(format!("EvtOpenChannelEnum failed: {e}")))?;

    // Buffer for channel path strings (most are under 256 chars)
    let mut buffer = vec![0u16; 512];
    let mut used = 0u32;

    loop {
        // SAFETY: We pass a valid handle and a properly sized buffer.
        // EvtNextChannelPath writes the channel name as a null-terminated
        // UTF-16 string into the buffer.
        let result = unsafe { EvtNextChannelPath(handle, Some(buffer.as_mut_slice()), &mut used) };

        match result {
            Ok(()) => {
                // Convert UTF-16 to String. `used` includes the null terminator.
                let len = if used > 0 { used as usize - 1 } else { 0 };
                let name = String::from_utf16_lossy(&buffer[..len]);
                if !name.is_empty() {
                    channels.push(name);
                }
            }
            Err(e) => {
                let code = e.code().0 as u32;
                // ERROR_NO_MORE_ITEMS — HRESULT 0x80070103 = normal end of enumeration.
                // Note: windows-rs errors always surface as HRESULTs (0x8007xxxx);
                // the raw Win32 code 259 can never appear here.
                if code == 0x80070103 {
                    break;
                }
                // ERROR_INSUFFICIENT_BUFFER — HRESULT 0x8007007A: grow buffer and retry.
                // Note: raw Win32 code 122 can never appear; only HRESULT form matches.
                if code == 0x8007007A {
                    buffer.resize(used as usize + 64, 0);
                    continue;
                }
                // Any other error — log and break
                tracing::warn!("EvtNextChannelPath returned unexpected error: {e}");
                break;
            }
        }
    }

    // SAFETY: handle is valid and hasn't been closed yet.
    unsafe {
        let _ = EvtClose(handle);
    }

    // Sort alphabetically for presentation
    channels.sort_unstable_by_key(|a| a.to_lowercase());

    tracing::info!("Enumerated {} event log channels", channels.len());
    Ok(channels)
}

/// Categorise a channel name into a display group.
///
/// Used by the UI to organise channels into a tree-like structure.
/// Returns `("Category", "SubName")`.
#[allow(dead_code)]
pub fn categorise_channel(channel: &str) -> (&str, &str) {
    if channel.eq_ignore_ascii_case("Application")
        || channel.eq_ignore_ascii_case("System")
        || channel.eq_ignore_ascii_case("Security")
        || channel.eq_ignore_ascii_case("Setup")
    {
        return ("Windows Logs", channel);
    }
    if channel.starts_with("Microsoft-Windows-") {
        return ("Applications and Services Logs", channel);
    }
    ("Other", channel)
}

/// Returns the subset of channels that are commonly useful.
///
/// These are shown first / selected by default in the UI.
///
/// Pushes the **actual** channel name found in `all` (preserving the
/// casing returned by the OS) rather than a hardcoded string.  This
/// ensures that `selected_channels` values are always byte-identical to
/// their counterparts in `channels`, so the channel-selector checkboxes
/// are rendered correctly regardless of OS casing (e.g. `"SECURITY"`
/// vs `"Security"`).
pub fn common_channels(all: &[String]) -> Vec<String> {
    let common = ["Application", "System", "Security", "Setup"];
    let mut result = Vec::new();
    for name in &common {
        if let Some(found) = all.iter().find(|c| c.eq_ignore_ascii_case(name)) {
            result.push(found.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strv(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    /// `common_channels` must return the **actual** string from the list,
    /// not a hardcoded canonical form.
    /// Regression test for B3: previously the function always pushed
    /// `"Security"` even when the OS list contained `"SECURITY"`, causing
    /// the channel-selector checkbox to appear unchecked.
    #[test]
    fn test_common_channels_preserves_actual_casing() {
        let all = strv(&["application", "SYSTEM", "SECURITY", "Setup"]);
        let result = common_channels(&all);
        assert!(
            result.contains(&"application".to_string()),
            "must keep OS casing"
        );
        assert!(
            result.contains(&"SYSTEM".to_string()),
            "must keep OS casing"
        );
        assert!(
            result.contains(&"SECURITY".to_string()),
            "must keep OS casing"
        );
        assert!(result.contains(&"Setup".to_string()), "must keep OS casing");
    }

    /// Channels absent from the system list must not be added.
    #[test]
    fn test_common_channels_skips_absent_channels() {
        let all = strv(&["Application", "System"]);
        let result = common_channels(&all);
        assert_eq!(result.len(), 2);
        assert!(!result.iter().any(|c| c.eq_ignore_ascii_case("Security")));
        assert!(!result.iter().any(|c| c.eq_ignore_ascii_case("Setup")));
    }

    /// The returned names must be exact elements of the input `all` slice
    /// so that `selected_channels.contains(channel)` lookups work correctly.
    #[test]
    fn test_common_channels_names_are_in_all() {
        let all = strv(&["Application", "System", "Security", "Setup", "Other"]);
        let result = common_channels(&all);
        for name in &result {
            assert!(
                all.contains(name),
                "returned name '{name}' must be an element of the input slice"
            );
        }
    }
}
