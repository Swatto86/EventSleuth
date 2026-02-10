//! Unified error types for EventSleuth.
//!
//! All fallible operations throughout the codebase return `Result<T, EventSleuthError>`.
//! This ensures consistent error reporting and clean propagation via the `?` operator.

/// Unified error type used throughout EventSleuth.
///
/// Each variant captures enough context to produce an actionable message for
/// the user or for log output.
#[derive(Debug, thiserror::Error)]
pub enum EventSleuthError {
    /// A Windows API call failed. `hr` is the raw HRESULT code and `context`
    /// describes which operation triggered the failure.
    #[error("Windows API error: {context} (HRESULT: 0x{hr:08X})")]
    WindowsApi {
        /// The raw HRESULT error code from the Windows API.
        hr: u32,
        /// Human-readable description of the operation that failed.
        context: String,
    },

    /// XML returned by `EvtRender` could not be parsed.
    #[error("XML parse error: {0}")]
    XmlParse(String),

    /// Channel enumeration via `EvtOpenChannelEnum` / `EvtNextChannelPath` failed.
    #[error("Channel enumeration failed: {0}")]
    ChannelEnum(String),

    /// Export (CSV or JSON) failed — typically an I/O error.
    #[error("Export failed: {0}")]
    Export(String),

    /// A user-supplied filter expression could not be parsed.
    #[error("Filter parse error: {0}")]
    #[allow(dead_code)]
    FilterParse(String),

    /// Catch-all for I/O errors (file writes, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias used throughout the crate.
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, EventSleuthError>;

/// Convert a raw Windows `HRESULT` (or `GetLastError` code) into an
/// [`EventSleuthError::WindowsApi`] with the given context string.
///
/// # Example
/// ```ignore
/// windows_err(0x80070005, "EvtQuery on Security channel")
/// ```
#[allow(dead_code)]
pub fn windows_err(hr: u32, context: impl Into<String>) -> EventSleuthError {
    EventSleuthError::WindowsApi {
        hr,
        context: context.into(),
    }
}
