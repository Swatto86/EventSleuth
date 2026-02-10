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
use crate::core::filter::FilterState;
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

            status_text: "Starting…".into(),
            query_elapsed: None,
            progress_count: 0,
            progress_channel: String::new(),

            errors: Vec::new(),

            detail_tab: DetailTab::Details,

            show_about: false,
        };

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
            self.status_text = "No channels selected".into();
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
                    self.query_elapsed = Some(elapsed);
                    self.reader_rx = None;
                    self.cancel_flag = None;
                    self.status_text = format!("Loaded {} events", total);
                    tracing::info!("Load complete: {} events", total);
                }
                ReaderMessage::Error { channel, error } => {
                    self.errors.push((channel, error));
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

    /// Export currently filtered events to CSV via a native save dialog.
    pub fn export_csv(&self) {
        let events = self.filtered_event_list();
        if events.is_empty() {
            tracing::warn!("No events to export");
            return;
        }

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name("EventSleuth_export.csv")
                .save_file()
            {
                if let Err(e) = crate::export::csv_export::export_csv(&events, &path) {
                    tracing::error!("CSV export failed: {}", e);
                }
            }
        });
    }

    /// Export currently filtered events to JSON via a native save dialog.
    pub fn export_json(&self) {
        let events = self.filtered_event_list();
        if events.is_empty() {
            tracing::warn!("No events to export");
            return;
        }

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("EventSleuth_export.json")
                .save_file()
            {
                if let Err(e) = crate::export::json_export::export_json(&events, &path) {
                    tracing::error!("JSON export failed: {}", e);
                }
            }
        });
    }

    /// Collect the filtered events into a cloned `Vec` for export.
    ///
    /// Cloning is necessary because export happens on a background thread
    /// (for the file dialog) and can't hold references to `self`.
    fn filtered_event_list(&self) -> Vec<EventRecord> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| self.all_events.get(idx).cloned())
            .collect()
    }
}

// ── eframe::App implementation ──────────────────────────────────────────

impl eframe::App for EventSleuthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process messages from the reader thread
        self.process_messages();

        // 2. Re-filter if needed
        if self.needs_refilter {
            self.apply_filter();
        }

        // 3. Keep repainting while loading (to poll messages)
        if self.is_loading {
            ctx.request_repaint();
        }

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
            self.render_event_table(ui);
        });

        // ── Floating popups ─────────────────────────────────────────
        self.render_channel_selector(ctx);
        self.render_about_dialog(ctx);
    }
}

impl EventSleuthApp {
    /// Render the About dialog window.
    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }

        let mut open = true;
        egui::Window::new("About")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([260.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("EventSleuth")
                            .color(crate::ui::theme::ACCENT)
                            .strong()
                            .size(20.0),
                    );
                    ui.label(
                        egui::RichText::new(format!("v{}", crate::util::constants::APP_VERSION))
                            .color(crate::ui::theme::TEXT_SECONDARY),
                    );
                    ui.add_space(8.0);
                    ui.label("A fast, filterable Windows Event Log viewer");
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("Developer: Swatto")
                            .color(crate::ui::theme::TEXT_SECONDARY),
                    );
                    ui.add_space(8.0);
                });
            });

        if !open {
            self.show_about = false;
        }
    }
}
