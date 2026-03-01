//! Application-wide constants for EventSleuth.
//!
//! Centralising magic numbers and configuration defaults here keeps the rest
//! of the codebase clean and makes tuning straightforward.

/// Number of event handles to request per `EvtNext` call.
/// Larger batches reduce API call overhead; 200 is a good balance between
/// memory and throughput.
pub const EVT_BATCH_SIZE: usize = 200;

/// Timeout in milliseconds passed to `EvtNext`. Using `INFINITE` (u32::MAX)
/// would block the reader thread; a finite timeout lets us check cancellation.
pub const EVT_NEXT_TIMEOUT_MS: u32 = 1000;

/// Maximum number of events to load per channel before stopping.
/// Acts as a safety valve to prevent runaway memory usage on channels
/// with millions of entries. Users can increase this via future settings.
pub const MAX_EVENTS_PER_CHANNEL: usize = 500_000;

/// Default channels selected on first launch.
#[allow(dead_code)]
pub const DEFAULT_CHANNELS: &[&str] = &["Application", "System"];

/// Buffer size (in `u16` units) for `EvtRender` output.
/// 8 KB (16 KB raw) is enough for the vast majority of events; the buffer
/// grows on demand for larger events and the allocation is reused across
/// all events in a channel read.
pub const EVT_RENDER_BUFFER_SIZE: usize = 8_192;

/// Size of the channel used to send batches from the reader thread to the UI.
/// Bounded to apply back-pressure if the UI falls behind. 256 lets the
/// reader run well ahead of the UI without stalling on send.
pub const CHANNEL_BOUND: usize = 256;

/// Row height in the virtual-scrolled event table (in logical pixels).
pub const TABLE_ROW_HEIGHT: f32 = 22.0;

/// How many events to accumulate in a batch before sending to the UI.
/// Smaller batches = more responsive UI updates; larger = less overhead.
#[allow(dead_code)]
pub const UI_BATCH_SIZE: usize = 500;

/// Application display name used in titles, dialogs, etc.
pub const APP_NAME: &str = "EventSleuth";

/// Application version string.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository URL.
pub const APP_GITHUB_URL: &str = "https://github.com/Swatto86/EventSleuth";

/// Buffer size (in `u16` units) for `EvtFormatMessage` output.
/// 2 KB (4 KB raw) covers most formatted message strings; the buffer
/// grows on demand and is reused across events.
pub const EVT_FORMAT_BUFFER_SIZE: usize = 2_048;

/// Debounce delay for text-based filter inputs (milliseconds).
/// Prevents excessive re-filtering while the user is still typing.
pub const FILTER_DEBOUNCE_MS: u64 = 150;

/// Interval between live-tail refresh queries (seconds).
pub const LIVE_TAIL_INTERVAL_SECS: u64 = 5;

/// Maximum number of errors to retain in the error list.
pub const MAX_ERRORS: usize = 200;

/// HRESULT code for E_ACCESSDENIED from the Windows API.
#[allow(dead_code)]
pub const HRESULT_ACCESS_DENIED: u32 = 0x80070005;
