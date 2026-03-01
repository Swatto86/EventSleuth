//! Extended actions for [`EventSleuthApp`]: export, keyboard shortcuts,
//! export message processing, About dialog, .evtx import, live tail,
//! and filter preset management.
//!
//! These are `impl` blocks on the app struct, split out from `app.rs`
//! to keep file sizes manageable (< 400 lines each).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::app::EventSleuthApp;
use crate::core::event_reader;
use crate::util::constants;

// ── Export actions ──────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Export currently filtered events to CSV via a native save dialog.
    ///
    /// Runs on a background thread and sends a completion message back
    /// via `export_rx` so the UI can display feedback.
    pub fn export_csv(&mut self) {
        if self.export_rx.is_some() {
            self.export_message = Some((
                "Export already in progress".into(),
                std::time::Instant::now(),
            ));
            return;
        }

        let events = self.filtered_event_list();
        if events.is_empty() {
            self.export_message = Some(("No events to export".into(), std::time::Instant::now()));
            return;
        }

        let (tx, rx) = crossbeam_channel::bounded::<String>(1);
        self.export_rx = Some(rx);

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name("EventSleuth_export.csv")
                .save_file()
            {
                match crate::export::csv_export::export_csv(&events, &path) {
                    Ok(()) => {
                        let _ = tx.send(format!("Exported {} events to CSV", events.len()));
                    }
                    Err(e) => {
                        tracing::error!("CSV export failed: {}", e);
                        let _ = tx.send(format!("CSV export failed: {e}"));
                    }
                }
            }
        });
    }

    /// Export currently filtered events to JSON via a native save dialog.
    ///
    /// Runs on a background thread and sends a completion message back
    /// via `export_rx` so the UI can display feedback.
    pub fn export_json(&mut self) {
        if self.export_rx.is_some() {
            self.export_message = Some((
                "Export already in progress".into(),
                std::time::Instant::now(),
            ));
            return;
        }

        let events = self.filtered_event_list();
        if events.is_empty() {
            self.export_message = Some(("No events to export".into(), std::time::Instant::now()));
            return;
        }

        let (tx, rx) = crossbeam_channel::bounded::<String>(1);
        self.export_rx = Some(rx);

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("EventSleuth_export.json")
                .save_file()
            {
                match crate::export::json_export::export_json(&events, &path) {
                    Ok(()) => {
                        let _ = tx.send(format!("Exported {} events to JSON", events.len()));
                    }
                    Err(e) => {
                        tracing::error!("JSON export failed: {}", e);
                        let _ = tx.send(format!("JSON export failed: {e}"));
                    }
                }
            }
        });
    }

    /// Process export completion messages from background threads.
    ///
    /// Called once per frame. Checks the `export_rx` channel for messages
    /// and clears stale export messages after a timeout.
    pub fn process_export_messages(&mut self) {
        if let Some(rx) = &self.export_rx {
            match rx.try_recv() {
                Ok(msg) => {
                    self.export_message = Some((msg, std::time::Instant::now()));
                    self.export_rx = None;
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // Sender dropped without sending (user cancelled the save dialog).
                    // Clear the receiver so future exports are not permanently blocked.
                    self.export_rx = None;
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // Still waiting for the background thread — nothing to do.
                }
            }
        }
        // Clear export message after 4 seconds
        if let Some((_, instant)) = &self.export_message {
            if instant.elapsed() > std::time::Duration::from_secs(4) {
                self.export_message = None;
            }
        }
    }
}

// ── Keyboard shortcuts ──────────────────────────────────────────────────

impl EventSleuthApp {
    /// Handle global keyboard shortcuts.
    ///
    /// - **F5**: Refresh (re-query selected sources)
    /// - **Escape**: Close open dialogs / clear selection
    /// - **Up/Down arrows**: Navigate event table selection
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // F5 = Refresh
            if i.key_pressed(egui::Key::F5) {
                self.start_loading();
            }

            // Escape = Close dialogs, then clear selection
            if i.key_pressed(egui::Key::Escape) {
                if self.show_about {
                    self.show_about = false;
                } else if self.show_channel_selector {
                    self.show_channel_selector = false;
                } else {
                    self.selected_event_idx = None;
                }
            }

            // Arrow keys for event navigation
            if i.key_pressed(egui::Key::ArrowDown) {
                if let Some(idx) = self.selected_event_idx {
                    if idx + 1 < self.filtered_indices.len() {
                        self.selected_event_idx = Some(idx + 1);
                    }
                } else if !self.filtered_indices.is_empty() {
                    self.selected_event_idx = Some(0);
                }
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                if let Some(idx) = self.selected_event_idx {
                    if idx > 0 {
                        self.selected_event_idx = Some(idx - 1);
                    }
                }
            }
        });
    }
}

// ── About dialog ────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Render the About dialog window.
    pub fn render_about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }

        let mut open = true;
        egui::Window::new("About")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([320.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("EventSleuth")
                            .color(crate::ui::theme::accent(self.dark_mode))
                            .strong()
                            .size(20.0),
                    );
                    ui.label(
                        egui::RichText::new(format!("v{}", crate::util::constants::APP_VERSION))
                            .color(crate::ui::theme::text_secondary(self.dark_mode)),
                    );
                    ui.add_space(8.0);
                    ui.label("A fast, filterable Windows Event Log viewer");
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("Developer: Swatto")
                            .color(crate::ui::theme::text_secondary(self.dark_mode)),
                    );
                    ui.add_space(4.0);
                    ui.hyperlink_to(
                        "github.com/Swatto86/EventSleuth",
                        crate::util::constants::APP_GITHUB_URL,
                    );
                    ui.add_space(8.0);
                });
            });

        if !open {
            self.show_about = false;
        }
    }
}

// ── .evtx file import ──────────────────────────────────────────────────

impl EventSleuthApp {
    /// Open a native file dialog (on a background thread) to select an
    /// `.evtx` file. The chosen path is sent back via `import_rx`.
    pub fn import_evtx(&mut self) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.import_rx = Some(rx);

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Event Log Files", &["evtx"])
                .set_title("Open .evtx File")
                .pick_file()
            {
                let _ = tx.send(path);
            }
        });
    }

    /// Begin loading events from a local `.evtx` file.
    ///
    /// Cancels any in-progress load, clears existing data, and spawns
    /// a file reader thread.
    pub fn start_loading_evtx(&mut self, path: &std::path::Path) {
        self.cancel_loading();
        self.live_tail = false;

        self.all_events.clear();
        self.filtered_indices.clear();
        self.selected_event_idx = None;
        self.errors.clear();
        self.query_elapsed = None;
        self.progress_count = 0;
        self.progress_channel.clear();

        let (tx, rx) = crossbeam_channel::bounded(constants::CHANNEL_BOUND);
        let cancel = Arc::new(AtomicBool::new(false));

        let display_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "evtx file".into());

        let _handle = event_reader::spawn_file_reader_thread(
            path.to_path_buf(),
            self.filter.time_from,
            self.filter.time_to,
            tx,
            cancel.clone(),
        );

        self.reader_rx = Some(rx);
        self.cancel_flag = Some(cancel);
        self.is_loading = true;
        self.is_tail_query = false;
        self.status_text = format!("Loading {}...", display_name);
    }
}

// ── Live tail ───────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Start a tail query that appends new events (does NOT clear existing data).
    ///
    /// Queries from 1ms after the newest loaded event timestamp forward.
    pub fn start_tail_query(&mut self) {
        if self.is_loading || self.selected_channels.is_empty() {
            return;
        }

        // Find the newest timestamp in the current data
        let newest = self.all_events.iter().map(|e| e.timestamp).max();
        let tail_from = newest.map(|t| t + chrono::Duration::milliseconds(1));

        let (tx, rx) = crossbeam_channel::bounded(constants::CHANNEL_BOUND);
        let cancel = Arc::new(AtomicBool::new(false));

        let _handle = event_reader::spawn_reader_thread(
            self.selected_channels.clone(),
            tail_from.or(self.filter.time_from),
            self.filter.time_to,
            tx,
            cancel.clone(),
        );

        self.reader_rx = Some(rx);
        self.cancel_flag = Some(cancel);
        self.is_loading = true;
        self.is_tail_query = true;
    }
}

// ── Save-preset dialog ─────────────────────────────────────────────────

impl EventSleuthApp {
    /// Render the "Save Filter Preset" dialog window.
    pub fn render_save_preset_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_save_preset {
            return;
        }

        let mut should_save = false;
        let mut should_close = false;

        egui::Window::new("Save Filter Preset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([280.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Preset name:");
                let response = ui.text_edit_singleline(&mut self.preset_name_input);

                // Auto-focus the text field
                if response.gained_focus() || self.preset_name_input.is_empty() {
                    response.request_focus();
                }

                // Enter key to confirm
                if response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.preset_name_input.trim().is_empty()
                {
                    should_save = true;
                }

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let name_valid = !self.preset_name_input.trim().is_empty();
                    if ui
                        .add_enabled(name_valid, egui::Button::new("Save"))
                        .clicked()
                    {
                        should_save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_save {
            let name = self.preset_name_input.trim().to_owned();
            // Replace existing preset with the same name, or append
            if let Some(existing) = self.filter_presets.iter_mut().find(|p| p.name == name) {
                *existing = crate::core::filter::FilterPreset::from_state(&name, &self.filter);
            } else {
                self.filter_presets
                    .push(crate::core::filter::FilterPreset::from_state(
                        &name,
                        &self.filter,
                    ));
            }
            self.preset_name_input.clear();
            self.show_save_preset = false;
            tracing::info!("Saved filter preset: {}", name);
        }

        if should_close {
            self.show_save_preset = false;
            self.preset_name_input.clear();
        }
    }
}
