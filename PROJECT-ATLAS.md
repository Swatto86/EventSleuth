# EventSleuth - Project Atlas

> **Last updated**: 2026-03-01 (v1.0.2)

## System Purpose

**EventSleuth** is a fast, modern Windows Event Log viewer and analyser built in Rust with eframe/egui. It provides a desktop-native alternative to the built-in Windows Event Viewer, offering virtual-scrolled display of 100k+ events, composable filters, CSV/JSON export, live-tail monitoring, and .evtx file import.

## Domain Concepts

| Concept | Definition |
|---------|-----------|
| **Channel** | A Windows Event Log channel (e.g. `Application`, `System`, `Security`). Discovered at startup via `EvtOpenChannelEnum`. |
| **EventRecord** | Canonical representation of a single event: 20+ fields including timestamp, level, event ID, provider, message, XML, etc. |
| **FilterState** | Composite in-memory filter: level, event ID (ranges/negation), provider, text/regex match, time range. Applied client-side after batch loading. |
| **FilterPreset** | Serialisable named snapshot of a `FilterState`, persisted via eframe storage. |
| **ReaderMessage** | Typed enum sent from background reader thread to UI: `EventBatch`, `Progress`, `Complete`, `Error`. |
| **Live Tail** | Periodic re-query (5s interval) for events newer than the most recent loaded timestamp. Appends without clearing existing data. |
| **Bookmark** | User-pinned event by index into `all_events`. Bookmarks are cleared on reload/import because indices change. |

## Architectural Boundaries

```
src/
  main.rs              Entry point, single-instance guard, tracing init, eframe launch
  app.rs               Central state struct (EventSleuthApp), construction, preference restore
  app_update.rs        eframe::App impl: update loop, message processing, filtering, sorting
  app_actions.rs       Export, keyboard shortcuts, About dialog, .evtx import, live tail, presets

  core/                Domain logic (Windows Evt* API wrappers)
    channel_enumerator Channel discovery (EvtOpenChannelEnum / EvtNextChannelPath)
    event_reader       Background thread: EvtQuery -> EvtNext -> batch send via crossbeam
    event_format       EvtRender (XML) + EvtFormatMessage (message) with retry-on-buffer-grow
    xml_parser         roxmltree: XML string -> EventRecord
    event_record       Canonical EventRecord struct
    filter             FilterState: criteria + matches() with short-circuit
    filter_preset      Named filter presets (Serialize/Deserialize)
    filter_tests       26 unit tests for filter logic

  ui/                  Rendering (impl blocks on EventSleuthApp)
    toolbar            Source selector, refresh, export, import, live tail, columns, theme, about
    filter_panel       Collapsible sections for each filter dimension
    event_table        Virtual-scrolled table via egui_extras::TableBuilder
    detail_panel       Details tab (grid + message) + XML tab with search highlighting
    stats_panel        Floating statistics window: severity breakdown, top providers, histogram
    status_bar         Event counts, query time, loading spinner, error badges
    theme              Dark/light palettes, level colours, spacing constants

  export/              File export logic
    csv_export         CSV via csv crate + rfd dialogs
    json_export        JSON via serde_json + rfd dialogs

  util/                Cross-cutting utilities
    constants          All magic numbers and app metadata
    error              thiserror-based EventSleuthError enum with 6 variants
    time               Timestamp formatting and user input parsing

  lib.rs               Library crate re-exports (core, export, util) for integration tests

tests/                 Integration tests
  integration.rs       Module root: constants, errors, filters, exports, time
  constants_validation Constant range and consistency checks
  error_types          Error construction, Display, Send+Sync proofs
  filter_roundtrip     FilterState <-> FilterPreset serialisation roundtrip
  export_validation    Export path pre-flight validation
  time_utils           Timestamp formatting and parsing edge cases
```

### Boundary Rules

- **core/** contains all Windows Evt* API interaction. The rest of the crate never calls Win32 event log APIs directly.
- **ui/** modules only add rendering `impl` blocks on `EventSleuthApp`; they do not own state.
- **export/** is self-contained and depends only on `core::event_record` and `util::*`.
- **util/** has zero dependencies on other crate modules.
- The single-instance mutex and MessageBox in `main.rs` are the only direct Win32 calls outside `core/`.

### Platform Coupling Acknowledgment

This application is inherently Windows-specific (its sole purpose is reading Windows Event Logs via the Evt* API). The `core/` module directly calls Windows APIs rather than abstracting behind a trait. This is an intentional architectural decision: an OS abstraction layer would add complexity with no practical benefit since EventSleuth will never run on non-Windows platforms. The coupling is confined to `core/` and `main.rs`.

## Entry Points / APIs / Extension Points

| Entry Point | Location | Purpose |
|-------------|----------|---------|
| `main()` | [src/main.rs](src/main.rs) | Process entry, single-instance guard, logging init, eframe launch |
| `PreInitState::build()` | [src/app.rs](src/app.rs) | Pre-launch channel enumeration (called before `run_native()`) |
| `EventSleuthApp::from_pre_init()` | [src/app.rs](src/app.rs) | App construction from pre-built state, theme install, preference restore |
| `eframe::App::update()` | [src/app_update.rs](src/app_update.rs) | Per-frame update loop: message processing, filtering, UI rendering |
| `spawn_reader_thread()` | [src/core/event_reader.rs](src/core/event_reader.rs) | Background channel reader entry |
| `spawn_file_reader_thread()` | [src/core/event_reader.rs](src/core/event_reader.rs) | Background .evtx file reader entry |

## Build / Test / CI / Release

### Build

```powershell
# Debug
cargo build

# Release (optimised, LTO, stripped)
cargo build --release
```

**Build script** ([build.rs](build.rs)): Generates a multi-resolution ICO icon programmatically and embeds it with the Windows manifest via `winresource`.

### Test

```powershell
cargo test
```

26 unit tests covering: filter logic, XML parsing, XPath generation, timestamp formatting, provider extraction.

31 integration tests in `tests/` cover: constants validation, error type construction, filter roundtrip, export validation, time utilities.

### CI

[.github/workflows/release.yml](.github/workflows/release.yml): Triggered on `v*` tag push. Jobs:
1. `create-release` (ubuntu): Extract tag annotation, create draft GitHub Release
2. `build` (windows): `cargo build --release`, upload `EventSleuth.exe` asset
3. `publish-release` (ubuntu): Promote draft to published

CI runs: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all-targets`, `cargo build --release`.

A separate [.github/workflows/ci.yml](.github/workflows/ci.yml) runs the same checks on every push/PR to `main`.

### Release

[update-application.ps1](update-application.ps1): Automated release pipeline.

```powershell
# Interactive
.\update-application.ps1

# Parameterised
.\update-application.ps1 -Version "1.1.0" -Notes "Added feature X"

# Force (skip duplicate version check)
.\update-application.ps1 -Version "1.0.2" -Notes "Hotfix" -Force
```

Steps: version validation -> Cargo.toml update -> pre-release build + test -> git commit -> delete old tags/releases -> create annotated tag -> push (triggers CI).

### Installer

[installer/eventsleuth.nsi](installer/eventsleuth.nsi): NSIS installer script.

```powershell
# Build installer (requires NSIS on PATH)
makensis installer\eventsleuth.nsi
```

Produces `EventSleuth-Setup.exe` with Start Menu shortcuts, per-user install, and clean uninstaller.

## Configuration

| Setting | Source | Default | Notes |
|---------|--------|---------|-------|
| Theme (dark/light) | eframe persistence | Dark | Toggled via toolbar |
| Selected channels | eframe persistence | Application, System, Security, Setup | Restored on startup |
| Filter presets | eframe persistence | Empty | User-created named presets |
| Max events/channel | eframe persistence | 500,000 | Configurable 1,000 - 10,000,000 |
| Column visibility | eframe persistence | 5 of 7 shown | Channel/Computer hidden by default |
| Log verbosity | `RUST_LOG` env var | `info` | Set `RUST_LOG=debug` or `RUST_LOG=trace` for diagnostics |
| Error log file | Automatic | `%LOCALAPPDATA%\EventSleuth\logs\eventsleuth.log` | Persistent structured log file |

### Debug Mode

EventSleuth uses the `tracing` crate with `tracing-subscriber` and env-filter.

**Activation**:
```powershell
# From command line (shows logs in terminal):
$env:RUST_LOG="debug"; .\EventSleuth.exe

# Trace level (maximum verbosity):
$env:RUST_LOG="trace"; .\EventSleuth.exe
```

**Output**: Logs are written to stderr (visible in terminal) AND to a persistent log file at `%LOCALAPPDATA%\EventSleuth\logs\eventsleuth.log`. The file logger always writes at `debug` level regardless of the `RUST_LOG` setting.

**Content at debug level**: Function entry/exit for key operations, decision points (filter match/reject), state transitions (loading -> complete), Windows API call parameters and results, timing information.

**Security**: No PII, secrets, tokens, or encryption keys are logged at any level.

## Constants & Resource Bounds

All magic numbers live in [src/util/constants.rs](src/util/constants.rs):

| Constant | Value | Purpose |
|----------|-------|---------|
| `EVT_BATCH_SIZE` | 200 | Event handles per EvtNext call |
| `EVT_NEXT_TIMEOUT_MS` | 1,000 | EvtNext timeout (ms) |
| `MAX_EVENTS_PER_CHANNEL` | 500,000 | Safety cap (user-adjustable) |
| `EVT_RENDER_BUFFER_SIZE` | 8,192 | u16 units for EvtRender |
| `EVT_FORMAT_BUFFER_SIZE` | 2,048 | u16 units for EvtFormatMessage |
| `CHANNEL_BOUND` | 256 | Crossbeam channel capacity |
| `FILTER_DEBOUNCE_MS` | 150 | Filter input debounce |
| `LIVE_TAIL_INTERVAL_SECS` | 5 | Live tail poll interval |
| `MAX_ERRORS` | 200 | Error list size cap |
| `MAX_RETRY_ATTEMPTS` | 3 | Transient error retry count |
| `RETRY_BASE_DELAY_MS` | 50 | Base delay for exponential backoff |

## Critical Invariants

1. **Single instance**: A named mutex (`Global\EventSleuth_SingleInstance`) ensures only one process runs at a time.
2. **Bounded channel**: The crossbeam channel between reader and UI is bounded (256 slots) for back-pressure.
3. **Error cap**: The error list is bounded at `MAX_ERRORS` (200) to prevent unbounded memory growth.
4. **Max events**: Each channel read is capped at `max_events_per_channel` (user-configurable, clamped 1,000 - 10,000,000).
5. **Cancel flag**: All reader threads check an `AtomicBool` cancel flag, enabling prompt cancellation.
6. **Thread safety**: Reader threads own all their Windows API handles and close them before exiting. The UI thread never touches Evt* handles.
7. **Bookmark invalidation**: Bookmarks are stored as indices into `all_events`. Any operation that clears `all_events` (reload, import) also clears bookmarks.
8. **Transient retry**: Windows API calls that return transient error codes are retried with capped exponential backoff (3 attempts, 50ms base).

## Runtime Dependencies

| Dependency | Minimum Version | Rationale |
|-----------|----------------|-----------|
| Windows 10 | 1903+ | Required for PerMonitorV2 DPI, modern Evt* API |
| VC++ Runtime | 2015+ | Rust MSVC toolchain dependency (usually pre-installed) |

No additional runtime installations are required. The application is distributed as a single executable.
