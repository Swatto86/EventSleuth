# EventSleuth — Production Specification & Build Prompt

## Overview

**EventSleuth** is a native Windows desktop application written in **Rust** using the **egui** (via `eframe`) UI framework. It serves as a fast, friendly, and filterable Windows Event Log viewer — a modern alternative to the built-in Event Viewer (`eventvwr.msc`). The application must read from the Windows Event Log API directly, present results in a responsive UI, and support powerful filtering across all available log channels.

---

## Core Objectives

1. **Speed** — Query and render tens of thousands of event log entries without freezing the UI. All log reading must happen on background threads; the UI thread must never block.
2. **Discoverability** — Automatically enumerate every available event log channel on the system (Application, System, Security, Setup, Forwarded Events, and all operational/analytic channels under `Microsoft-Windows-*`).
3. **Filtering** — Provide composable, real-time filters: Event ID, severity/level, source/provider, time range, free-form text search across all fields (message, provider, event data XML).
4. **Detail View** — Display the full event record at a glance: formatted message, raw XML event data, all standard fields, and any `<EventData>` or `<UserData>` key/value pairs.
5. **Production Quality** — Heavily inline-commented, clean separation of concerns, no monolithic files, easily extensible and maintainable codebase.

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| Language | Rust (stable) | Target `x86_64-pc-windows-msvc` |
| UI Framework | `eframe` / `egui` | Immediate-mode GUI, glow (OpenGL) backend |
| Event Log API | `windows` crate (`Win32::System::EventLog`) | Use the **EvtQuery / EvtNext / EvtRender** family (modern Evt* API, NOT the legacy `OpenEventLog` API) |
| XML Parsing | `quick-xml` or `roxmltree` | Parse the rendered event XML returned by `EvtRender` |
| Async/Threading | `std::thread` + `crossbeam-channel` (or `std::sync::mpsc`) | Background reader thread communicates with UI via channels |
| Time | `chrono` | Parse and display `SystemTime` timestamps |
| Serialisation | `serde` + `serde_json` | For export and internal config |

---

## Architecture & Project Structure

Organise the codebase with clear separation of concerns. **No file should exceed ~400 lines.** Each module has a single responsibility.

```
EventSleuth/
├── Cargo.toml
├── build.rs                      # Embed Windows app manifest + icon
├── assets/
│   ├── icon.ico                  # Application icon
│   └── app.manifest              # UAC elevation manifest (asInvoker or requireAdministrator)
│
├── src/
│   ├── main.rs                   # Entry point — initialise eframe, launch app
│   │
│   ├── app.rs                    # Top-level `eframe::App` implementation
│   │                             #   - Owns all UI state
│   │                             #   - Delegates to panel modules for rendering
│   │                             #   - Handles channel messages from background threads
│   │
│   ├── ui/
│   │   ├── mod.rs                # Re-exports all UI sub-modules
│   │   ├── toolbar.rs            # Top toolbar: channel selector, refresh, export buttons
│   │   ├── filter_panel.rs       # Left/top filter panel: all filter controls
│   │   ├── event_table.rs        # Central scrollable table of events (virtual scrolling)
│   │   ├── detail_panel.rs       # Right/bottom detail pane: full event view
│   │   ├── status_bar.rs         # Bottom status bar: event count, query time, progress
│   │   └── theme.rs              # Colour palette, spacing constants, style helpers
│   │
│   ├── core/
│   │   ├── mod.rs                # Re-exports
│   │   ├── event_record.rs       # `EventRecord` struct — the canonical parsed event
│   │   ├── event_reader.rs       # Background thread logic: enumerate channels, query events
│   │   ├── channel_enumerator.rs # Enumerate all available log channels via EvtOpenChannelEnum
│   │   ├── xml_parser.rs         # Parse raw event XML into `EventRecord`
│   │   └── filter.rs             # `FilterState` struct + filtering logic (applied in-memory)
│   │
│   ├── export/
│   │   ├── mod.rs
│   │   ├── csv_export.rs         # Export filtered results to CSV
│   │   └── json_export.rs        # Export filtered results to JSON
│   │
│   └── util/
│       ├── mod.rs
│       ├── error.rs              # Unified error types (`thiserror`)
│       ├── constants.rs          # App-wide constants (batch sizes, default limits)
│       └── time.rs               # Timestamp formatting helpers
```

---

## Data Model

### `EventRecord` (core/event_record.rs)

This is the central data structure. Every event log entry is parsed into this struct.

```rust
/// Represents a single parsed Windows Event Log entry.
/// All fields are extracted from the XML rendered by EvtRender.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EventRecord {
    /// Raw XML string as returned by EvtRender (retained for detail view)
    pub raw_xml: String,

    /// The log channel this event came from (e.g. "Application", "System",
    /// "Microsoft-Windows-Sysmon/Operational")
    pub channel: String,

    /// Event ID (the numeric identifier for this event type)
    pub event_id: u32,

    /// Severity level: 0=LogAlways, 1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose
    pub level: u8,

    /// Human-readable level name
    pub level_name: String,

    /// The event provider/source name
    pub provider_name: String,

    /// Timestamp of the event (UTC)
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// The computer name where the event was generated
    pub computer: String,

    /// The formatted/rendered message string (if available)
    pub message: String,

    /// Process ID that generated the event
    pub process_id: u32,

    /// Thread ID
    pub thread_id: u32,

    /// Task category
    pub task: u16,

    /// Opcode
    pub opcode: u8,

    /// Keywords bitmask
    pub keywords: u64,

    /// Correlation Activity ID (if present)
    pub activity_id: Option<String>,

    /// User SID (if present)
    pub user_sid: Option<String>,

    /// Parsed key-value pairs from <EventData> or <UserData>
    pub event_data: Vec<(String, String)>,
}
```

### `FilterState` (core/filter.rs)

```rust
/// Holds all active filter criteria. Applied in-memory against the loaded events.
#[derive(Debug, Clone, Default)]
pub struct FilterState {
    /// Comma-separated or individual Event IDs to include (empty = all)
    pub event_ids: String,

    /// Minimum severity level to show (0=all, 1=Critical only, 2=Error+, etc.)
    pub min_level: u8,

    /// Maximum severity level to show
    pub max_level: u8,

    /// Provider/source name substring filter
    pub provider_filter: String,

    /// Free-form text search — matched against message, provider, raw XML, event data values
    pub text_search: String,

    /// Start of time range filter (None = no lower bound)
    pub time_from: Option<chrono::DateTime<chrono::Utc>>,

    /// End of time range filter (None = no upper bound)
    pub time_to: Option<chrono::DateTime<chrono::Utc>>,

    /// Whether text search is case-sensitive
    pub case_sensitive: bool,
}
```

Implement a `FilterState::matches(&self, event: &EventRecord) -> bool` method that evaluates all criteria with short-circuit logic (cheapest checks first: level, event_id, then provider, then text search).

---

## Event Log Reading Strategy

### Channel Enumeration (core/channel_enumerator.rs)

- Use `EvtOpenChannelEnum` / `EvtNextChannelPath` to discover every log channel on the system.
- Present channels in a searchable, categorised tree or flat list in the UI.
- Default selection: `Application`, `System`, `Security`.

### Event Querying (core/event_reader.rs)

- Use `EvtQuery` with `EvtQueryChannelPath` to open a query handle against a channel.
- Use an **XPath filter** in the query where possible (especially for time range) to reduce the volume of events returned from the API before in-memory filtering.
- Use `EvtNext` in a loop to fetch events in **batches** (e.g. 100–500 at a time).
- For each event handle, call `EvtRender` with `EvtRenderEventXml` to get the full XML string.
- Optionally also call `EvtFormatMessage` to get the provider-formatted message string (falls back to raw event data if formatting fails).
- **Parse** the XML into an `EventRecord` using the `xml_parser` module.
- Send completed `Vec<EventRecord>` batches to the UI thread via a channel.

### Threading Model

```
┌─────────────────┐          channel (crossbeam)         ┌──────────────┐
│  Background      │  ──── Vec<EventRecord> batches ────▶ │  UI Thread   │
│  Reader Thread   │  ──── ProgressUpdate messages ─────▶ │  (egui)      │
│                  │  ◀──── CancelSignal ──────────────── │              │
└─────────────────┘                                       └──────────────┘
```

Define an enum for messages:

```rust
pub enum ReaderMessage {
    /// A batch of parsed events ready to display
    EventBatch(Vec<EventRecord>),
    /// Progress update: (events_read_so_far, channel_name)
    Progress { count: usize, channel: String },
    /// Reading is complete for all requested channels
    Complete { total: usize, elapsed: std::time::Duration },
    /// An error occurred reading a channel
    Error { channel: String, error: String },
}
```

The UI polls the receiving end of the channel every frame (non-blocking `try_recv` loop) and appends incoming events to its master list.

---

## UI Design Specification

### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  [Channel Selector ▼]  [🔄 Refresh]  [📤 Export ▼]   EventSleuth│ ← Toolbar
├────────────────────┬─────────────────────────────────────────────┤
│  FILTERS           │  EVENT TABLE (virtual scroll)               │
│                    │                                             │
│  Event ID: [____]  │  Timestamp  | Level | ID   | Source | Msg  │
│  Level: [▼ All   ] │  ─────────────────────────────────────────  │
│  Provider: [____]  │  2024-01-15 | Error | 1001 | App..  | The  │
│  Search: [_______] │  2024-01-15 | Warn  | 4625 | Sec..  | An   │
│  Time From: [____] │  2024-01-15 | Info  | 7036 | Ser..  | The  │
│  Time To:   [____] │  ...                                        │
│  ☐ Case Sensitive  │                                             │
│  [Apply] [Clear]   │                                             │
├────────────────────┴─────────────────────────────────────────────┤
│  DETAIL PANEL                                                    │
│                                                                  │
│  Event ID: 1001          Level: Error         Provider: AppHang  │
│  Timestamp: 2024-01-15 10:23:45 UTC          Computer: DESKTOP  │
│  Process ID: 4532        Thread ID: 7890                         │
│                                                                  │
│  Message:                                                        │
│  The program explorer.exe stopped interacting with Windows and   │
│  was closed. [...]                                               │
│                                                                  │
│  Event Data:                                                     │
│  ┌─────────────┬────────────────────────────┐                    │
│  │ Name        │ Value                      │                    │
│  ├─────────────┼────────────────────────────┤                    │
│  │ ProgramName │ explorer.exe               │                    │
│  │ HangTime    │ 10000                      │                    │
│  └─────────────┴────────────────────────────┘                    │
│                                                                  │
│  [📋 Copy XML]  [📋 Copy Message]                                │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Showing 1,247 of 45,832 events  |  Query: 1.2s  |  Ready ✓     │ ← Status Bar
└──────────────────────────────────────────────────────────────────┘
```

### Visual Design

- Use a **dark theme** as default with a custom colour palette (not stock egui).
- Severity levels should be **colour-coded**:
  - Critical: bright red (#FF4444)
  - Error: red/orange (#E06C60)
  - Warning: amber/yellow (#E0A840)
  - Informational: blue/grey (#7AA2D4)
  - Verbose: dim grey (#888888)
- Selected row should be clearly highlighted.
- Alternating row colours for readability.
- Monospace font for XML/raw data sections.

### Virtual Scrolling / Performance

The event table **must** use virtual scrolling (egui's `ScrollArea` with computed row ranges) — only render visible rows. With potentially 100,000+ events loaded, rendering all rows would destroy performance.

Suggested approach:
- Know the total number of filtered events and the row height.
- Calculate which row indices are visible given the current scroll offset.
- Only call UI layout code for those rows.
- egui's `ScrollArea::show_rows()` is ideal for this.

### Table Sorting

- Clicking a column header sorts by that column (toggle ascending/descending).
- Default sort: timestamp descending (newest first).
- Sorting must be fast — sort the filtered index, not the full dataset.

---

## Filter Implementation Details

Filters should be **reactive** — as the user types, filter results live (with a small debounce of ~150ms to avoid per-keystroke re-filtering of large datasets). Alternatively, provide an explicit **[Apply]** button for users who prefer deliberate filtering.

### Event ID Filter

- Accept a comma-separated list: `1001, 4625, 7036`
- Accept ranges: `4000-4999`
- Accept negation: `!1001` (exclude)
- Parse into a `HashSet<u32>` for O(1) lookup.

### Level Filter

- Dropdown or checkbox group: Critical, Error, Warning, Informational, Verbose.
- Multiple selection allowed.

### Text Search

- Search across: `message`, `provider_name`, `event_data` values, and optionally `raw_xml`.
- Case-insensitive by default, with a toggle.
- Highlight matching text in the detail panel.

### Time Range

- Date/time picker fields (or text input parsing with `chrono`).
- Quick presets: Last 1 hour, Last 24 hours, Last 7 days, Last 30 days, Today.

---

## Export (export/)

### CSV Export (csv_export.rs)

- Use the `csv` crate.
- Export all currently filtered events.
- Columns: Timestamp, Level, EventID, Provider, Computer, Message, Channel.
- Open a native file save dialog (`rfd` crate).

### JSON Export (json_export.rs)

- Use `serde_json`.
- Export filtered events as an array of `EventRecord` objects.
- Pretty-printed by default.

---

## Error Handling (util/error.rs)

Use `thiserror` for a unified error enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EventSleuthError {
    #[error("Windows API error: {context} (HRESULT: 0x{hr:08X})")]
    WindowsApi { hr: u32, context: String },

    #[error("XML parse error: {0}")]
    XmlParse(String),

    #[error("Channel enumeration failed: {0}")]
    ChannelEnum(String),

    #[error("Export failed: {0}")]
    Export(String),

    #[error("Filter parse error: {0}")]
    FilterParse(String),
}
```

All Windows API calls should be wrapped in helper functions that convert `HRESULT` / `GetLastError` into this error type with context.

---

## UAC / Permissions

- The Security log requires **administrator** privileges.
- By default, run **without elevation** (`asInvoker` manifest) so Application/System logs work for any user.
- If a user selects the Security channel and reading fails with access denied, show a clear warning in the UI: *"Reading the Security log requires running EventSleuth as Administrator."*
- Do **not** force elevation for the whole app just because one channel needs it.

---

## Cargo.toml Dependencies

```toml
[package]
name = "eventsleuth"
version = "0.1.0"
edition = "2021"
description = "A fast, filterable Windows Event Log viewer"

[dependencies]
eframe = { version = "0.31", features = ["default"] }
egui = "0.31"
egui_extras = { version = "0.31", features = ["datepicker"] }
windows = { version = "0.58", features = [
    "Win32_System_EventLog",
    "Win32_Foundation",
    "Win32_Security",
] }
chrono = { version = "0.4", features = ["serde"] }
quick-xml = "0.36"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"
crossbeam-channel = "0.5"
thiserror = "2"
rfd = "0.15"                      # Native file dialogs
tracing = "0.1"                   # Structured logging
tracing-subscriber = "0.3"

[build-dependencies]
winresource = "0.1"               # Embed icon + manifest

[profile.release]
opt-level = 3
lto = true
strip = "symbols"
```

> **Note:** Pin exact versions at build time. The versions above are indicative — use the latest compatible versions.

---

## Key Implementation Notes

### Windows Evt* API Usage Pattern

The core loop in `event_reader.rs` should follow this pattern (pseudocode):

```rust
// 1. Build XPath query for server-side filtering (time range at minimum)
let xpath = build_xpath_query(&filter);

// 2. Open query handle
let query_handle = EvtQuery(None, channel_name, xpath, EvtQueryChannelPath | EvtQueryReverseDirection)?;

// 3. Fetch in batches
let mut handles = vec![EVT_HANDLE::default(); BATCH_SIZE];
loop {
    let mut returned = 0u32;
    let ok = EvtNext(query_handle, &mut handles, TIMEOUT_MS, 0, &mut returned);
    if !ok || returned == 0 { break; }

    let batch: Vec<EventRecord> = handles[..returned as usize]
        .iter()
        .filter_map(|&h| {
            let xml = render_event_xml(h).ok()?;
            let record = parse_event_xml(&xml, &channel_name).ok()?;
            EvtClose(h); // Always close individual event handles
            Some(record)
        })
        .collect();

    sender.send(ReaderMessage::EventBatch(batch))?;

    // Check for cancellation
    if cancel_flag.load(Ordering::Relaxed) { break; }
}

// 4. Clean up
EvtClose(query_handle);
```

### XPath Pre-Filtering

Where possible, push filters down into the XPath query passed to `EvtQuery` to reduce data transferred from the event log service:

```xpath
*[System[TimeCreated[@SystemTime >= '2024-01-01T00:00:00.000Z']]]
*[System[(Level = 1) or (Level = 2)]]
*[System[EventID = 1001]]
```

Multiple predicates can be combined. This is critical for performance on large logs.

### Message Formatting

`EvtFormatMessage` requires a publisher metadata handle (`EvtOpenPublisherMetadata`). Cache these handles per provider name (in a `HashMap<String, EVT_HANDLE>`) to avoid re-opening them for every event. Close them all on shutdown.

If `EvtFormatMessage` fails (common for events from uninstalled providers), fall back to displaying the raw `<EventData>` contents.

---

## Coding Standards

1. **Every public function and struct** must have a `///` doc comment explaining purpose, parameters, and return value.
2. **Inline comments** on any non-obvious logic — especially Windows API calls, unsafe blocks, and bitwise operations.
3. **No `unwrap()` in production code paths** — use `?` operator, `map_err`, or explicit error handling. `unwrap()` is acceptable only in tests or provably infallible cases with a comment explaining why.
4. **All `unsafe` blocks** must have a `// SAFETY:` comment explaining the invariants.
5. **No file over ~400 lines** — split into sub-modules if approaching this limit.
6. **Consistent naming**: `snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE` for constants.
7. **Use `tracing`** for structured logging throughout (debug/info/warn/error levels).

---

## Testing Strategy

- **Unit tests** in each module for parsing, filtering, and data transformation logic.
- **Integration tests** that query real event logs (marked `#[ignore]` for CI, run manually on Windows).
- **Mock data** in `tests/fixtures/` — sample event XML strings for parser tests.

---

## Future Extensibility (Design For But Don't Implement)

The architecture should make the following future features straightforward to add:

- **Remote computer event log querying** (EvtQuery supports remote machines).
- **Saved filter presets** (serialise `FilterState` to JSON config file).
- **Bookmarked events** (star/pin events for later reference).
- **Log file import** (`.evtx` file parsing via `EvtQuery` with `EvtQueryFilePath`).
- **Regex support** in text search.
- **Column customisation** (show/hide columns, reorder).
- **Event correlation** (group related events by Activity ID).

---

## Build & Run

```powershell
# Debug build
cargo build

# Release build (optimised, stripped)
cargo build --release

# Run (may need elevation for Security log access)
.\target\release\eventsleuth.exe
```

---

## Summary Checklist

- [ ] Channel enumeration discovers all logs
- [ ] Background thread reads events without blocking UI
- [ ] Events parsed from XML into typed `EventRecord` structs
- [ ] Virtual-scrolled table handles 100k+ events smoothly
- [ ] Filter by: Event ID (with ranges/negation), Level, Provider, Text, Time range
- [ ] Detail panel shows all event fields + formatted event data table + raw XML
- [ ] CSV and JSON export with native save dialog
- [ ] Colour-coded severity levels throughout
- [ ] Graceful handling of access denied (Security log)
- [ ] Comprehensive error handling — no panics in production paths
- [ ] Every file < 400 lines, every public item documented
- [ ] Dark theme with professional colour palette
