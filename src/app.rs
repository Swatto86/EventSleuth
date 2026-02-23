//! Top-level application state and `eframe::App` implementation.
//!
//! `EventSleuthApp` owns all UI state, the loaded event list, filter
//! configuration, and communication channels with the background reader
//! thread. Rendering is delegated to panel sub-modules in `ui/`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Receiver;

use crate::core::channel_enumerator;
use crate::core::event_reader::{self, ReaderMessage};
use crate::core::event_record::EventRecord;
use crate::core::filter::{FilterPreset, FilterState};
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
}

// ── Construction ────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Create a new `EventSleuthApp` and apply the custom theme.
    ///
    /// Enumerates available log channels (synchronous — typically fast)
    /// and auto-starts loading the default channels.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::theme::apply_theme(&cc.egui_ctx);
        Self::install_system_fonts(&cc.egui_ctx);

        // Enumerate channels — this is fast (< 100ms typically)
        let channels = match channel_enumerator::enumerate_channels() {
            Ok(ch) => ch,
            Err(e) => {
                tracing::error!("Failed to enumerate channels: {}", e);
                vec![
                    "Application".into(),
                    "System".into(),
                    "Security".into(),
                ]
            }
        };

        let selected = channel_enumerator::common_channels(&channels);

        let mut app = Self {
            channels,
            channel_search: String::new(),
            selected_channels: selected,
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
            if let Some(presets) = eframe::get_value::<Vec<FilterPreset>>(storage, "filter_presets") {
                app.filter_presets = presets;
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
        let symbol_path = r"C:\Windows\Fonts\seguisym.ttf";
        if let Ok(data) = std::fs::read(symbol_path) {
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
        let emoji_path = r"C:\Windows\Fonts\seguiemj.ttf";
        if let Ok(data) = std::fs::read(emoji_path) {
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

// ── Core logic ──────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Start (or restart) loading events from the selected channels.
    ///
    /// Cancels any in-progress load, clears existing data, and spawns
    /// a new reader background thread.
    pub fn start_loading(&mut self) {
        // Cancel any existing reader
        self.cancel_loading();

        if self.selected_channels.is_empty() {
            self.status_text = "No sources selected".into();
            return;
        }

        // Clear previous results
        self.all_events.clear();
        self.filtered_indices.clear();
        self.selected_event_idx = None;
        self.errors.clear();
        self.query_elapsed = None;
        self.progress_count = 0;
        self.progress_channel.clear();

        // Create communication channel and cancellation flag
        let (tx, rx) =
            crossbeam_channel::bounded::<ReaderMessage>(constants::CHANNEL_BOUND);
        let cancel = Arc::new(AtomicBool::new(false));

        // Spawn background reader thread
        let _handle = event_reader::spawn_reader_thread(
            self.selected_channels.clone(),
            self.filter.time_from,
            self.filter.time_to,
            tx,
            cancel.clone(),
        );

        self.reader_rx = Some(rx);
        self.cancel_flag = Some(cancel);
        self.is_loading = true;
        self.status_text = "Loading…".into();
    }

    /// Request cancellation of the current reader thread.
    pub fn cancel_loading(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
        self.is_loading = false;
        self.reader_rx = None;
        self.cancel_flag = None;
    }

    /// Poll the reader channel for incoming messages and process them.
    ///
    /// Called once per frame. Non-blocking — uses `try_recv` in a loop
    /// to drain all available messages.
    fn process_messages(&mut self) {
        let rx = match &self.reader_rx {
            Some(rx) => rx.clone(),
            None => return,
        };

        // Drain all available messages this frame
        let mut received_events = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ReaderMessage::EventBatch(batch) => {
                    self.all_events.extend(batch);
                    received_events = true;
                }
                ReaderMessage::Progress { count, channel } => {
                    self.progress_count = count;
                    self.progress_channel = channel;
                }
                ReaderMessage::Complete { total, elapsed } => {
                    self.is_loading = false;
                    self.reader_rx = None;
                    self.cancel_flag = None;
                    if self.is_tail_query {
                        // Tail query: only update status if new events arrived
                        if total > 0 {
                            self.status_text = format!("{} new events (live tail)", total);
                            tracing::info!("Tail complete: {} new events", total);
                        }
                        self.is_tail_query = false;
                    } else {
                        self.query_elapsed = Some(elapsed);
                        self.status_text = format!("Loaded {} events", total);
                        tracing::info!("Load complete: {} events", total);
                    }
                }
                ReaderMessage::Error { channel, error } => {
                    if self.errors.len() < constants::MAX_ERRORS {
                        self.errors.push((channel, error));
                    }
                }
            }
        }

        if received_events {
            self.needs_refilter = true;
        }
    }

    /// Rebuild `filtered_indices` by applying the current filter to all events.
    pub fn apply_filter(&mut self) {
        self.filtered_indices = self
            .all_events
            .iter()
            .enumerate()
            .filter(|(_, event)| self.filter.matches(event))
            .map(|(i, _)| i)
            .collect();

        self.sort_events();

        // Clamp selection to valid range
        if let Some(idx) = self.selected_event_idx {
            if idx >= self.filtered_indices.len() {
                self.selected_event_idx = None;
            }
        }

        self.needs_refilter = false;
    }

    /// Sort `filtered_indices` by the current sort column and direction.
    pub fn sort_events(&mut self) {
        let events = &self.all_events;
        let col = self.sort_column;
        let asc = self.sort_ascending;

        self.filtered_indices.sort_by(|&a, &b| {
            let ea = &events[a];
            let eb = &events[b];
            let ord = match col {
                SortColumn::Timestamp => ea.timestamp.cmp(&eb.timestamp),
                SortColumn::Level => ea.level.cmp(&eb.level),
                SortColumn::EventId => ea.event_id.cmp(&eb.event_id),
                SortColumn::Provider => ea.provider_name.cmp(&eb.provider_name),
                SortColumn::Message => ea.message.cmp(&eb.message),
            };
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    /// Get a reference to the currently selected event, if any.
    pub fn selected_event(&self) -> Option<&EventRecord> {
        let vis_idx = self.selected_event_idx?;
        let event_idx = *self.filtered_indices.get(vis_idx)?;
        self.all_events.get(event_idx)
    }

    /// Collect the filtered events into a cloned `Vec` for export.
    ///
    /// Cloning is necessary because export happens on a background thread
    /// (for the file dialog) and can't hold references to `self`.
    pub fn filtered_event_list(&self) -> Vec<EventRecord> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| self.all_events.get(idx).cloned())
            .collect()
    }

    /// Check whether any error from the Security channel indicates
    /// an access-denied failure (requires elevation).
    pub fn has_security_access_error(&self) -> bool {
        self.errors.iter().any(|(ch, err)| {
            ch == "Security"
                && (err.contains("80070005")
                    || err.contains("00000005")
                    || err.to_lowercase().contains("access"))
        })
    }

    /// Poll the import file-selection channel for a user-chosen .evtx path.
    fn process_import_selection(&mut self) {
        let path = {
            let rx = match &self.import_rx {
                Some(rx) => rx,
                None => return,
            };
            match rx.try_recv() {
                Ok(p) => p,
                Err(_) => return,
            }
        };
        self.import_rx = None;
        self.start_loading_evtx(&path);
    }
}

// ── eframe::App implementation ──────────────────────────────────────────

impl eframe::App for EventSleuthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process messages from the reader thread
        self.process_messages();

        // 2. Process export completion messages
        self.process_export_messages();

        // 3. Process .evtx import file selection
        self.process_import_selection();

        // 4. Debounce: apply filter after FILTER_DEBOUNCE_MS of inactivity
        if let Some(timer) = self.debounce_timer {
            let debounce = std::time::Duration::from_millis(constants::FILTER_DEBOUNCE_MS);
            if timer.elapsed() >= debounce {
                self.filter.parse_event_ids();
                self.filter.parse_time_range();
                self.needs_refilter = true;
                self.debounce_timer = None;
            } else {
                ctx.request_repaint_after(debounce);
            }
        }

        // 5. Re-filter if needed
        if self.needs_refilter {
            self.apply_filter();
        }

        // 6. Keep repainting while loading (to poll messages)
        if self.is_loading {
            ctx.request_repaint();
        }

        // 7. Live tail: periodic re-query for new events
        if self.live_tail && !self.is_loading {
            let should_tail = match self.last_tail_time {
                Some(t) => t.elapsed() >= std::time::Duration::from_secs(constants::LIVE_TAIL_INTERVAL_SECS),
                None => true,
            };
            if should_tail {
                self.start_tail_query();
                self.last_tail_time = Some(std::time::Instant::now());
            }
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        // 8. Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // ── Top toolbar ─────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .exact_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_toolbar(ui);
            });

        // ── Bottom status bar ───────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                self.render_status_bar(ui);
            });

        // ── Bottom detail panel ─────────────────────────────────────
        egui::TopBottomPanel::bottom("detail_panel")
            .resizable(true)
            .default_height(250.0)
            .min_height(100.0)
            .show(ctx, |ui| {
                self.render_detail_panel(ui);
            });

        // ── Left filter panel ───────────────────────────────────────
        egui::SidePanel::left("filter_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(160.0)
            .max_width(350.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_filter_panel(ui);
                });
            });

        // ── Central event table ─────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            // Security elevation banner
            if self.has_security_access_error() {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(60, 40, 10))
                    .inner_margin(egui::Margin::same(6))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("⚠ Security log access denied.")
                                    .color(crate::ui::theme::LEVEL_WARNING)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("Run EventSleuth as Administrator to view Security events.")
                                    .color(crate::ui::theme::TEXT_SECONDARY),
                            );
                        });
                    });
                ui.add_space(4.0);
            }
            self.render_event_table(ui);
        });

        // ── Floating popups ─────────────────────────────────────────
        self.render_channel_selector(ctx);
        self.render_about_dialog(ctx);
        self.render_save_preset_dialog(ctx);
    }

    /// Persist user preferences to eframe storage on shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "dark_mode", &self.dark_mode);
        eframe::set_value(storage, "selected_channels", &self.selected_channels);
        eframe::set_value(storage, "filter_presets", &self.filter_presets);
    }
}
