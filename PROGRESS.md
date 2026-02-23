# EventSleuth — Progress Tracker

> Last updated: 2026-02-23

## Build Status

- **Debug build:** ✅ Compiles — zero errors, zero warnings
- **Release build:** ✅ Compiles — optimised, LTO, stripped symbols
- **Unit tests:** ✅ 18/18 passing
- **App launches:** ✅ GUI window opens, events load from Application/System sources
- **CI/CD:** ✅ GitHub Actions workflow for automated release builds
- **Single instance:** ✅ Named mutex prevents duplicate instances
- **Clippy:** ✅ Zero warnings on application code

---

## Specification Checklist

Tracking against [EventSleuth-Specification.md](EventSleuth-Specification.md).

### Core Functionality

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | Source enumeration discovers all logs | ✅ Done | `core/channel_enumerator.rs` — uses `EvtOpenChannelEnum`/`EvtNextChannelPath` |
| 2 | Background thread reads events without blocking UI | ✅ Done | `core/event_reader.rs` — spawns reader thread, sends batches via `crossbeam-channel` |
| 3 | Events parsed from XML into typed `EventRecord` structs | ✅ Done | `core/xml_parser.rs` — parses System, EventData, UserData |
| 4 | Virtual-scrolled table handles 100k+ events smoothly | ✅ Done | `ui/event_table.rs` — uses `egui_extras::TableBuilder` with `body.rows()` |
| 5 | Filter by Event ID (with ranges/negation) | ✅ Done | `core/filter.rs` — supports `1001`, `4000-4999`, `!1001` syntax |
| 6 | Filter by Level | ✅ Done | Checkbox per severity level |
| 7 | Filter by Provider | ✅ Done | Case-insensitive substring match |
| 8 | Filter by Text (free-form search) | ✅ Done | Searches message, provider, channel, event data, raw XML |
| 9 | Filter by Time range | ✅ Done | Manual input + quick presets (1h, 24h, 7d, 30d) |
| 10 | Detail panel shows all event fields | ✅ Done | `ui/detail_panel.rs` — header grid, message, event data table |
| 11 | Detail panel shows formatted event data table | ✅ Done | Name/Value grid from `<EventData>` or `<UserData>` |
| 12 | Detail panel shows raw XML | ✅ Done | Tabs: Details / XML with monospace rendering |
| 13 | CSV export with native save dialog | ✅ Done | `export/csv_export.rs` — uses `csv` + `rfd` crates |
| 14 | JSON export with native save dialog | ✅ Done | `export/json_export.rs` — pretty-printed via `serde_json` |
| 15 | Colour-coded severity levels | ✅ Done | `ui/theme.rs` — Critical/Error/Warning/Info/Verbose each have distinct colours |
| 16 | Graceful handling of access denied (Security log) | ✅ Done | Error shown in status bar, other sources continue |
| 17 | Comprehensive error handling — no panics | ✅ Done | `thiserror` enum, no `unwrap()` in prod paths |
| 18 | Every file < 400 lines | ✅ Done | Largest file is ~340 lines |
| 19 | Every public item documented | ✅ Done | `///` doc comments on all pub functions/structs |
| 20 | Dark theme with professional colour palette | ✅ Done | Custom dark theme applied on startup |
| 21 | Emoji polish throughout UI | ✅ Done | Toolbar, filters, status bar, detail panel, About dialog |
| 22 | Tooltips on all filter inputs | ✅ Done | Hover text with usage examples on every text field |
| 23 | Single-instance enforcement | ✅ Done | Named mutex (`Global\EventSleuth_SingleInstance`) with MessageBox notification |
| 24 | No console window in release | ✅ Done | `#![windows_subsystem = "windows"]` in release builds |
| 25 | GitHub URL in About dialog | Done | Clickable hyperlink to repository |
| 26 | Keyboard shortcuts (F5, Esc, arrows) | Done | F5=refresh, Esc=close dialogs, Up/Down=navigate events |
| 27 | Export success/failure feedback | Done | Toast message in status bar for 4 seconds after export |
| 28 | "Today" time preset | Done | Filter from midnight local time today |
| 29 | Zero clippy warnings on app code | Done | All 5 prior warnings resolved |
| 30 | Safe UTF-8 truncation | Done | Char-safe slicing in table and detail panel |
| 31 | Filter debouncing (~150ms) | Done | Text fields debounced via `Instant` timer; checkboxes/buttons instant |
| 32 | .evtx file import | Done | Open .evtx files via `EvtQueryFilePath` flag; toolbar "Open .evtx" button |
| 33 | Search match highlighting | Done | Detail panel message, event data, and XML view highlight search matches via `LayoutJob` |
| 34 | Window size/position persistence | Done | eframe `persistence` feature saves window geometry between sessions |
| 35 | User preference persistence | Done | Dark/light mode and selected channels restored on startup |
| 36 | Saved filter presets | Done | Named presets saved/loaded/deleted via eframe storage; UI in filter panel |
| 37 | Live tail / auto-refresh | Done | Toggle in toolbar; polls every 5s for new events, appends without clearing |
| 38 | Security log elevation banner | Done | Prominent warning banner above event table when Security access is denied |

### UI Layout

| Component | Status | File |
|-----------|--------|------|
| Top toolbar (source selector, refresh, export) | ✅ Done | `ui/toolbar.rs` |
| Source selector popup (searchable, checkboxes) | ✅ Done | `ui/toolbar.rs` |
| Left filter panel | ✅ Done | `ui/filter_panel.rs` |
| Central event table (virtual scroll, sortable) | ✅ Done | `ui/event_table.rs` |
| Bottom detail panel (Details + XML tabs) | ✅ Done | `ui/detail_panel.rs` |
| Bottom status bar | ✅ Done | `ui/status_bar.rs` |

### Table Features

| Feature | Status | Notes |
|---------|--------|-------|
| Column sorting (click header) | ✅ Done | Timestamp, Level, ID, Provider, Message |
| Sort direction toggle (▲/▼) | ✅ Done | Visual indicator on active column |
| Default sort: timestamp descending | ✅ Done | Newest first |
| Selected row highlighting | ✅ Done | Via `row.set_selected()` |
| Alternating row colours | ✅ Done | Via `TableBuilder::striped(true)` |

### Threading & Performance

| Feature | Status | Notes |
|---------|--------|-------|
| Background reader thread | ✅ Done | `std::thread` + `crossbeam-channel` |
| Cancellation support | ✅ Done | `AtomicBool` cancel flag |
| XPath pre-filtering (time range) | ✅ Done | Pushes time bounds into `EvtQuery` |
| Batched `EvtNext` calls | ✅ Done | 200 handles per batch |
| Publisher metadata caching | ✅ Done | `HashMap<String, EVT_HANDLE>` per provider |
| Safety limit per source | ✅ Done | 500,000 max events |
| `EvtFormatMessage` with fallback | ✅ Done | Falls back to event data concatenation |

---

## Project Structure

```
EventSleuth/
├── Cargo.toml                          ✅
├── build.rs                            ✅  (icon generation + manifest embedding)
├── update-application.ps1              ✅  (version bump, tag, push to trigger release)
├── .github/
│   └── workflows/
│       └── release.yml                 ✅  (CI build + GitHub Release with single exe)
├── assets/
│   ├── app.manifest                    ✅  (asInvoker, DPI aware)
│   └── icon.ico                        ✅  (auto-generated on first build)
├── src/
│   ├── main.rs                         ✅  (entry point, single-instance check, tracing init, eframe launch)
│   ├── app.rs                          ✅  (App state, eframe::App impl, message processing)
│   ├── app_actions.rs                  ✅  (Export actions, keyboard shortcuts, About dialog)
│   ├── core/
│   │   ├── mod.rs                      ✅
│   │   ├── event_record.rs             ✅  (EventRecord struct)
│   │   ├── event_reader.rs             ✅  (background reader, ReaderMessage enum)
│   │   ├── channel_enumerator.rs       ✅  (EvtOpenChannelEnum)
│   │   ├── xml_parser.rs              ✅  (XML → EventRecord)
│   │   └── filter.rs                   ✅  (FilterState + matching logic)
│   ├── ui/
│   │   ├── mod.rs                      ✅
│   │   ├── toolbar.rs                  ✅
│   │   ├── filter_panel.rs             ✅
│   │   ├── event_table.rs             ✅
│   │   ├── detail_panel.rs             ✅
│   │   ├── status_bar.rs              ✅
│   │   └── theme.rs                    ✅
│   ├── export/
│   │   ├── mod.rs                      ✅
│   │   ├── csv_export.rs              ✅
│   │   └── json_export.rs             ✅
│   └── util/
│       ├── mod.rs                      ✅
│       ├── error.rs                    ✅
│       ├── constants.rs               ✅
│       └── time.rs                     ✅
```

---

## Recent Changes

| Change | Description |
|--------|-------------|
| Keyboard shortcuts | F5 refreshes, Escape closes dialogs/clears selection, Up/Down navigates event list |
| Export feedback | Export success/failure shown in status bar for 4 seconds with event count |
| "Today" time preset | New quick-preset button filters from midnight local time today |
| Clippy clean | Resolved all 5 clippy warnings (unnecessary casts, sort_by) |
| Safe UTF-8 truncation | Fixed potential panics on multi-byte chars in table message and detail panel value truncation |
| File size compliance | Split `app.rs` (was 530 lines) into `app.rs` (383) + `app_actions.rs` (183) |
| Filter debouncing | Text fields debounce at ~150ms before triggering re-filter; checkboxes/buttons remain instant |
| .evtx file import | Open exported `.evtx` files via toolbar button; reads using `EvtQueryFilePath` flag |
| Search match highlighting | Detail panel highlights search matches in message, event data values, and XML with `LayoutJob` |
| Window persistence | Window size, position, dark/light mode, and selected channels restored between sessions |
| Saved filter presets | Named filter presets saved/loaded/deleted via Presets menu in filter panel |
| Live tail / auto-refresh | Toolbar toggle polls for new events every 5s and appends without clearing existing data |
| Security elevation banner | Prominent warning banner above event table when Security log access is denied |
| Theme-matched clear colour | `clear_color()` override returns the active theme background colour instead of default white |
| Sources terminology | Renamed all user-facing "Channels" references to "Sources" for clarity. |
| Single-instance enforcement | Uses a Windows named mutex to prevent multiple instances. Shows a MessageBox if already running. |
| Emoji UI polish | Added contextual emoji throughout: toolbar buttons, filter labels, status bar, detail panel tabs, About dialog. |
| Tooltips on all inputs | Every text field has a hover tooltip explaining usage with examples. |
| About dialog | Shows version, developer name, and clickable GitHub URL (`github.com/Swatto86/EventSleuth`). |
| No console window | `#![windows_subsystem = "windows"]` hides the console in release builds. |
| Compact About button | Replaced `(i)` text with a properly sized ℹ button. |
| Release binary name | `EventSleuth.exe` (PascalCase) via `[[bin]]` in Cargo.toml. |
| System font fallbacks | Loads "Segoe UI Symbol" and "Segoe UI Emoji" from Windows as fallback fonts for Unicode rendering. |
| Embedded icon | `include_bytes!` bakes icon into the exe at compile time. |
| CI/CD release pipeline | `update-application.ps1` + `.github/workflows/release.yml` for automated GitHub Releases. |

---

## Known Limitations / Remaining Polish

These are **not** blockers — the app is functional. Listed for future improvement:

| Item | Priority | Notes |
|------|----------|-------|
| Startup flash on launch | Low | Brief white flash during window creation; a known winit/Windows limitation with no reliable fix in eframe 0.31 |
| Date/time picker widget | Low | Uses text input instead of `egui_extras::DatePickerButton` (incompatible with eframe persistence) |
| Column resizing persistence | Low | Column widths reset on restart |

---

## Potential Enhancements

Ideas for future development, roughly prioritised:

| Enhancement | Priority | Description |
|-------------|----------|-------------|
| Remote computer querying | Medium | Query event logs on remote machines via `EvtQuery` session handles. |
| Regex text search | Low | Toggle between substring and regex matching in the search box. |
| Column customisation | Low | Show/hide/reorder table columns via a settings panel. |
| Column width persistence | Low | Save/restore column widths between sessions. |
| Event correlation by Activity ID | Low | Group related events by their Activity ID. |
| Bookmarked/pinned events | Low | Let users pin important events for reference. |
| Event statistics dashboard | Low | Show summary charts: events by level, top providers, events over time. |
| Export filtered only | Low | Option to export only the currently filtered/visible events. |
| Configurable max events | Low | Let users adjust the 500k per-source safety limit. |

---

## How to Build & Run

```powershell
# Debug
cargo build
cargo run

# Release
cargo build --release
.\target\release\EventSleuth.exe

# Tests
cargo test
```
