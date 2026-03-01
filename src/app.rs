//! Top-level application state and construction.
//!
//! `EventSleuthApp` owns all UI state, the loaded event list, filter
//! configuration, and communication channels with the background reader
//! thread. Rendering is delegated to panel sub-modules in `ui/`.
//! The frame-by-frame update loop lives in [`crate::app_update`].

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crossbeam_channel::Receiver;

use crate::core::channel_enumerator;
use crate::core::event_reader::ReaderMessage;
use crate::core::event_record::EventRecord;
use crate::core::filter::FilterState;
use crate::core::filter_preset::FilterPreset;
use crate::ui::stats_panel::EventStats;
use crate::util::constants;

// ── Enums ───────────────────────────────────────────────────────────────

/// Which column the event table is currently sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Timestamp,
    Level,
    EventId,
    Provider,
    Message,
}

/// Which tab is active in the detail panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Details,
    Xml,
}

/// Controls which columns are visible in the event table.
///
/// Persisted to eframe storage so the user's preference survives restarts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColumnVisibility {
    /// Show the Timestamp column.
    pub timestamp: bool,
    /// Show the Level column.
    pub level: bool,
    /// Show the Event ID column.
    pub event_id: bool,
    /// Show the Provider column.
    pub provider: bool,
    /// Show the Message column.
    pub message: bool,
    /// Show the Channel/Source column.
    pub channel: bool,
    /// Show the Computer column.
    pub computer: bool,
}

impl Default for ColumnVisibility {
    /// Default: show the standard five columns, hide Channel and Computer.
    fn default() -> Self {
        Self {
            timestamp: true,
            level: true,
            event_id: true,
            provider: true,
            message: true,
            channel: false,
            computer: false,
        }
    }
}

// ── App state ───────────────────────────────────────────────────────────

/// Central application state for EventSleuth.
///
/// All fields are accessible to the UI rendering methods (defined in
/// `ui/*.rs` via `impl EventSleuthApp` blocks).
pub struct EventSleuthApp {
    // ── Channel management ──────────────────────────────────────
    /// All discovered event log channels on the system.
    pub channels: Vec<String>,
    /// Search string for filtering the channel list in the popup.
    pub channel_search: String,
    /// Channels the user has selected to query.
    pub selected_channels: Vec<String>,
    /// Whether the channel selector popup is open.
    pub show_channel_selector: bool,

    // ── Event storage ───────────────────────────────────────────
    /// Master list of all loaded events (unsorted, unfiltered).
    pub all_events: Vec<EventRecord>,
    /// Indices into `all_events` that match the current filter, in
    /// display order (sorted).
    pub filtered_indices: Vec<usize>,
    /// Index into `filtered_indices` of the currently selected row.
    pub selected_event_idx: Option<usize>,
    /// Flag: re-compute `filtered_indices` on the next frame.
    pub needs_refilter: bool,

    // ── Filter ──────────────────────────────────────────────────
    /// All active filter criteria.
    pub filter: FilterState,

    // ── Sorting ─────────────────────────────────────────────────
    /// Current sort column.
    pub sort_column: SortColumn,
    /// `true` = ascending, `false` = descending.
    pub sort_ascending: bool,

    // ── Background reader ───────────────────────────────────────
    /// Receiver end of the channel from the reader thread.
    pub reader_rx: Option<Receiver<ReaderMessage>>,
    /// Shared flag to request cancellation of the reader thread.
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// `true` while a reader thread is running.
    pub is_loading: bool,

    // ── Status ──────────────────────────────────────────────────
    /// Human-readable status text shown in the status bar.
    pub status_text: String,
    /// How long the last query took.
    pub query_elapsed: Option<std::time::Duration>,
    /// Total events read so far during the current load.
    pub progress_count: usize,
    /// Name of the channel currently being read.
    pub progress_channel: String,

    // ── Errors ──────────────────────────────────────────────────
    /// Errors from the last read operation: `(channel, message)`.
    pub errors: Vec<(String, String)>,

    // ── Detail panel ────────────────────────────────────────────
    /// Active tab in the detail pane.
    pub detail_tab: DetailTab,

    // ── Dialogs ─────────────────────────────────────────────────
    /// Whether the About dialog is open.
    pub show_about: bool,

    // ── Theme ───────────────────────────────────────────────────
    /// `true` = dark mode (default), `false` = light mode.
    pub dark_mode: bool,

    // ── Export feedback ─────────────────────────────────────────
    /// Receiver for export completion messages from background threads.
    pub export_rx: Option<crossbeam_channel::Receiver<String>>,
    /// Transient status message for export results (shown briefly).
    pub export_message: Option<(String, std::time::Instant)>,

    // ── Filter debounce ─────────────────────────────────────────
    /// Timestamp of the last text-field change in the filter panel.
    /// When set, the update loop waits [`constants::FILTER_DEBOUNCE_MS`]
    /// before applying the filter.
    pub debounce_timer: Option<std::time::Instant>,

    // ── Filter presets ──────────────────────────────────────────
    /// Saved named filter presets (persisted via eframe storage).
    pub filter_presets: Vec<FilterPreset>,
    /// Whether the "save preset" dialog is open.
    pub show_save_preset: bool,
    /// Text input for the new preset name.
    pub preset_name_input: String,

    // ── Live tail ───────────────────────────────────────────────
    /// When `true`, the app periodically re-queries for new events.
    pub live_tail: bool,
    /// Timestamp of the last live-tail refresh.
    pub last_tail_time: Option<std::time::Instant>,
    /// Whether the current in-flight query is a tail append (vs full load).
    pub is_tail_query: bool,

    // ── .evtx file import ───────────────────────────────────────
    /// Receiver for a file path selected by the user via the open dialog.
    pub import_rx: Option<crossbeam_channel::Receiver<std::path::PathBuf>>,

    // ── Statistics panel ────────────────────────────────────────
    /// Whether the statistics panel window is visible.
    pub show_stats: bool,
    /// Cached statistics snapshot, recomputed when events change.
    pub stats_cache: EventStats,
    /// Flag: recompute stats on the next frame when the panel is visible.
    pub stats_dirty: bool,

    // ── Regex search ────────────────────────────────────────────
    // (The `use_regex` flag lives in FilterState; no extra app fields.)

    // ── Configurable max events ─────────────────────────────────
    /// User-adjustable maximum events per channel. Overrides the
    /// compile-time [`constants::MAX_EVENTS_PER_CHANNEL`] default.
    pub max_events_per_channel: usize,
    /// Raw string binding for the max-events text field in the filter panel.
    ///
    /// Persisted as text so that the field shows whatever the user typed
    /// across frames, rather than being overwritten on every frame with
    /// the clamped integer value (Bug fix: ephemeral-binding reset bug).
    pub max_events_input: String,

    // ── Bookmarked / pinned events ──────────────────────────────
    /// Set of bookmarked event indices (into `all_events`).
    pub bookmarked_indices: std::collections::HashSet<usize>,
    /// Whether to show only bookmarked events in the table.
    pub show_bookmarks_only: bool,

    // ── Save-preset dialog focus tracking ───────────────────────
    /// `true` once the save-preset text field has received its initial
    /// auto-focus for the current dialog session.  Reset to `false` each
    /// time the dialog opens so focus is requested exactly once per session,
    /// preventing the field from stealing keyboard focus on every frame
    /// while it is empty (Bug fix: persistent focus-stealing in preset dialog).
    pub save_preset_focus_requested: bool,

    // ── Column visibility ───────────────────────────────────────
    /// Controls which columns are visible in the event table.
    pub column_visibility: ColumnVisibility,
}

// ── Pre-initialisation state (built before eframe::run_native) ──────

/// Data collected before `eframe::run_native()` so that the creator
/// closure is trivial (Rule 16: avoid white flash).
pub struct PreInitState {
    /// Discovered event log channels.
    pub channels: Vec<String>,
    /// Default channels to select on first launch.
    pub selected: Vec<String>,
}

impl PreInitState {
    /// Perform all expensive pre-launch work: channel enumeration, etc.
    ///
    /// Must be called **before** `eframe::run_native()`.
    pub fn build() -> Self {
        let channels = match channel_enumerator::enumerate_channels() {
            Ok(ch) => ch,
            Err(e) => {
                tracing::error!("Failed to enumerate channels: {}", e);
                vec!["Application".into(), "System".into(), "Security".into()]
            }
        };
        let selected = channel_enumerator::common_channels(&channels);
        Self { channels, selected }
    }
}

// ── Construction ────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Create an `EventSleuthApp` from pre-initialised state.
    ///
    /// The `PreInitState` is built before `eframe::run_native()` so this
    /// closure is trivial (font setup + struct construction only), which
    /// eliminates the startup white flash on Windows.
    pub fn from_pre_init(cc: &eframe::CreationContext<'_>, pre: PreInitState) -> Self {
        crate::ui::theme::apply_theme(&cc.egui_ctx);
        Self::install_system_fonts(&cc.egui_ctx);

        let mut app = Self {
            channels: pre.channels,
            channel_search: String::new(),
            selected_channels: pre.selected,
            show_channel_selector: false,

            all_events: Vec::new(),
            filtered_indices: Vec::new(),
            selected_event_idx: None,
            needs_refilter: false,

            filter: FilterState::default(),

            sort_column: SortColumn::Timestamp,
            sort_ascending: false, // newest first

            reader_rx: None,
            cancel_flag: None,
            is_loading: false,

            status_text: "Starting...".into(),
            query_elapsed: None,
            progress_count: 0,
            progress_channel: String::new(),

            errors: Vec::new(),

            detail_tab: DetailTab::Details,

            show_about: false,

            dark_mode: true,

            export_rx: None,
            export_message: None,

            debounce_timer: None,

            filter_presets: Vec::new(),
            show_save_preset: false,
            preset_name_input: String::new(),

            live_tail: false,
            last_tail_time: None,
            is_tail_query: false,

            import_rx: None,

            show_stats: false,
            stats_cache: EventStats::default(),
            stats_dirty: true,

            max_events_per_channel: constants::MAX_EVENTS_PER_CHANNEL,
            max_events_input: constants::MAX_EVENTS_PER_CHANNEL.to_string(),

            bookmarked_indices: std::collections::HashSet::new(),
            show_bookmarks_only: false,

            save_preset_focus_requested: false,

            column_visibility: ColumnVisibility::default(),
        };

        // ── Restore persisted preferences ──────────────────────────
        if let Some(storage) = cc.storage {
            if let Some(dark) = eframe::get_value::<bool>(storage, "dark_mode") {
                app.dark_mode = dark;
                if dark {
                    crate::ui::theme::apply_dark_theme(&cc.egui_ctx);
                } else {
                    crate::ui::theme::apply_light_theme(&cc.egui_ctx);
                }
            }
            if let Some(ch) = eframe::get_value::<Vec<String>>(storage, "selected_channels") {
                if !ch.is_empty() {
                    app.selected_channels = ch;
                }
            }
            if let Some(presets) = eframe::get_value::<Vec<FilterPreset>>(storage, "filter_presets")
            {
                app.filter_presets = presets;
            }
            if let Some(max_ev) = eframe::get_value::<usize>(storage, "max_events_per_channel") {
                app.max_events_per_channel = max_ev.clamp(1000, 10_000_000);
                // Sync the text binding so the field shows the restored value.
                app.max_events_input = app.max_events_per_channel.to_string();
            }
            if let Some(cv) = eframe::get_value::<ColumnVisibility>(storage, "column_visibility") {
                app.column_visibility = cv;
            }
        }

        // Auto-start loading default channels
        app.start_loading();

        app
    }

    /// Install Windows system fonts as fallbacks for emoji and symbol coverage.
    ///
    /// Loads "Segoe UI Emoji" and "Segoe UI Symbol" from the system fonts
    /// directory, appending them as fallbacks so that arrow glyphs (▲/▼),
    /// emoji, and other Unicode symbols render correctly.
    fn install_system_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // Segoe UI Symbol — geometric shapes, arrows, misc symbols
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
        let symbol_path = format!(r"{}\Fonts\seguisym.ttf", windir);
        if let Ok(data) = std::fs::read(&symbol_path) {
            fonts.font_data.insert(
                "segoe_ui_symbol".to_owned(),
                egui::FontData::from_owned(data).into(),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push("segoe_ui_symbol".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.push("segoe_ui_symbol".to_owned());
            }
        }

        // Segoe UI Emoji — colour emoji (rendered monochrome in egui)
        let emoji_path = format!(r"{}\Fonts\seguiemj.ttf", windir);
        if let Ok(data) = std::fs::read(&emoji_path) {
            fonts.font_data.insert(
                "segoe_ui_emoji".to_owned(),
                egui::FontData::from_owned(data).into(),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push("segoe_ui_emoji".to_owned());
            }
        }

        ctx.set_fonts(fonts);
    }
}
