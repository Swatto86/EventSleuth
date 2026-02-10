# EventSleuth — Progress Tracker

> Last updated: 2026-02-10

## Build Status

- **Debug build:** ✅ Compiles — zero errors, zero warnings
- **Release build:** ✅ Compiles — optimised, LTO, stripped symbols
- **Unit tests:** ✅ 18/18 passing
- **App launches:** ✅ GUI window opens, events load from Application/System channels
- **CI/CD:** ✅ GitHub Actions workflow for automated release builds

---

## Specification Checklist

Tracking against [EventSleuth-Specification.md](EventSleuth-Specification.md).

### Core Functionality

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | Channel enumeration discovers all logs | ✅ Done | `core/channel_enumerator.rs` — uses `EvtOpenChannelEnum`/`EvtNextChannelPath` |
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
| 16 | Graceful handling of access denied (Security log) | ✅ Done | Error shown in status bar, other channels continue |
| 17 | Comprehensive error handling — no panics | ✅ Done | `thiserror` enum, no `unwrap()` in prod paths |
| 18 | Every file < 400 lines | ✅ Done | Largest file is ~340 lines |
| 19 | Every public item documented | ✅ Done | `///` doc comments on all pub functions/structs |
| 20 | Dark theme with professional colour palette | ✅ Done | Custom dark theme applied on startup |

### UI Layout

| Component | Status | File |
|-----------|--------|------|
| Top toolbar (channel selector, refresh, export) | ✅ Done | `ui/toolbar.rs` |
| Channel selector popup (searchable, checkboxes) | ✅ Done | `ui/toolbar.rs` |
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
| Safety limit per channel | ✅ Done | 500,000 max events |
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
│   ├── main.rs                         ✅  (entry point, tracing init, eframe launch)
│   ├── app.rs                          ✅  (App state, eframe::App impl, message processing)
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

## Recent Fixes

| Fix | Description |
|-----|-------------|
| System font fallbacks | Loads "Segoe UI Symbol" and "Segoe UI Emoji" from Windows as fallback fonts, so arrow indicators (▲/▼), emoji, and other Unicode symbols render correctly in egui. |
| Embedded icon | `load_app_icon()` now uses `include_bytes!("../assets/icon.ico")` instead of runtime file I/O. The icon is baked into the exe at compile time, so it works regardless of CWD. Fixes titlebar/taskbar showing default egui icon. |
| Emoji glyphs → ASCII | egui's default font does not include emoji — all emoji characters (📋🔄📤⏹✓⚠ℹ🔍) were rendering as squares. Replaced with ASCII equivalents throughout the UI. |
| About dialog | Added an `(i)` button in the toolbar that opens a modal About dialog showing app name, version, and "Developer: Swatto". |
| CI/CD release pipeline | Added `update-application.ps1` (version bump + tag + push) and `.github/workflows/release.yml` (GitHub Actions: build release exe, create GitHub Release with single portable exe). Mirrors the DiskSleuth release pattern. |

---

## Known Limitations / Remaining Polish

These are **not** blockers — the app is functional. Listed for future improvement:

| Item | Priority | Notes |
|------|----------|-------|
| Filter debouncing (~150ms) | Low | Currently requires Apply button; no per-keystroke filtering |
| Text search match highlighting in detail panel | Low | Search works but matching text isn't highlighted |
| Date/time picker widget | Low | Uses text input instead of `egui_extras::DatePickerButton` |
| Column resizing persistence | Low | Column widths reset on restart |
| Security log elevation prompt/UX | Low | Shows error in status bar; could add a more prominent banner |

---

## Future Extensibility (Designed For, Not Implemented)

Per spec — the architecture supports adding these without major refactoring:

- [ ] Remote computer event log querying (EvtQuery session handle)
- [ ] Saved filter presets (serialize FilterState to JSON)
- [ ] Bookmarked/pinned events
- [ ] `.evtx` file import (EvtQueryFilePath flag)
- [ ] Regex text search
- [ ] Column customisation (show/hide/reorder)
- [ ] Event correlation by Activity ID

---

## How to Build & Run

```powershell
# Debug
cargo build
cargo run

# Release
cargo build --release
.\target\release\eventsleuth.exe

# Tests
cargo test
```
