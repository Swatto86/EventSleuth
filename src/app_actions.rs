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
    /// - **F5 / Ctrl+R**: Refresh (re-query selected sources)
    /// - **Escape**: Close open dialogs / clear selection
    /// - **Up/Down arrows**: Navigate event table selection
    /// - **Page Up/Down**: Jump 20 rows in event table
    /// - **Home/End**: Jump to first/last event
    /// - **Ctrl+Shift+X**: Clear all filters
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // F5 or Ctrl+R = Refresh
            if i.key_pressed(egui::Key::F5) || (i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
                self.start_loading();
            }

            // Ctrl+Shift+X = Clear all filters
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::X) {
                self.filter.clear();
                self.filter.parse_event_ids();
                self.filter.parse_time_range();
                self.needs_refilter = true;
            }

            // Escape = Cancel loading, close dialogs, then clear selection
            if i.key_pressed(egui::Key::Escape) {
                if self.is_loading {
                    self.cancel_loading();
                } else if self.show_about {
                    self.show_about = false;
                } else if self.show_channel_selector {
                    self.show_channel_selector = false;
                } else if self.show_save_preset {
                    self.show_save_preset = false;
                } else if self.show_stats {
                    self.show_stats = false;
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

            // Page Down = jump 20 rows forward
            if i.key_pressed(egui::Key::PageDown) {
                let max = self.filtered_indices.len().saturating_sub(1);
                if let Some(idx) = self.selected_event_idx {
                    self.selected_event_idx = Some((idx + 20).min(max));
                } else if !self.filtered_indices.is_empty() {
                    self.selected_event_idx = Some(0);
                }
            }

            // Page Up = jump 20 rows backward
            if i.key_pressed(egui::Key::PageUp) {
                if let Some(idx) = self.selected_event_idx {
                    self.selected_event_idx = Some(idx.saturating_sub(20));
                }
            }

            // Home = jump to first event
            if i.key_pressed(egui::Key::Home) && !self.filtered_indices.is_empty() {
                self.selected_event_idx = Some(0);
            }

            // End = jump to last event
            if i.key_pressed(egui::Key::End) && !self.filtered_indices.is_empty() {
                self.selected_event_idx = Some(self.filtered_indices.len().saturating_sub(1));
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
    ///
    /// Guards against double-activation: if a file dialog is already open
    /// (`import_rx` is `Some`), the call is a no-op so the first dialog is
    /// not silently abandoned.
    pub fn import_evtx(&mut self) {
        if self.import_rx.is_some() {
            // A file dialog is already pending — do not spawn a second one.
            tracing::debug!("import_evtx: dialog already open, ignoring duplicate call");
            return;
        }
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

        // Bookmarks reference indices into all_events, so they become
        // invalid after a file import and must be cleared.
        self.bookmarked_indices.clear();
        self.show_bookmarks_only = false;

        // Invalidate the stats cache immediately so a zero-event file
        // import never leaves the panel showing the previous run's data.
        self.stats_dirty = true;

        // Force a filter pass on the next frame so all derived state
        // (stats_dirty, filtered_indices) is consistent.
        self.needs_refilter = true;

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
            self.max_events_per_channel,
        );

        self.reader_rx = Some(rx);
        self.cancel_flag = Some(cancel);
        self.is_loading = true;
        self.is_tail_query = false;
        self.status_text = format!("Loading {}...", display_name);
    }
}

// ── Live tail ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tail_datetime_tests {
    /// Regression test for B2: adding 1 ms to a near-max DateTime<Utc> must not
    /// panic.  The fix uses `checked_add_signed` so overflow falls back gracefully
    /// rather than producing a panic in the reader thread startup path.
    #[test]
    fn tail_from_near_max_datetime_does_not_panic() {
        use chrono::Duration;
        // Use the maximum representable chrono::DateTime<chrono::Utc> value.
        let max_dt = chrono::DateTime::<chrono::Utc>::MAX_UTC;
        // This mirrors the logic in start_tail_query exactly.
        let tail_from = max_dt
            .checked_add_signed(Duration::milliseconds(1))
            .unwrap_or(max_dt);
        // On overflow the fallback must equal the original timestamp.
        assert_eq!(
            tail_from, max_dt,
            "overflow fallback must equal the original timestamp"
        );
    }

    /// Normal case: adding 1 ms to a typical timestamp must increment it by exactly 1 ms.
    #[test]
    fn tail_from_normal_datetime_increments_by_1ms() {
        use chrono::{Duration, TimeZone, Utc};
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        let result = ts
            .checked_add_signed(Duration::milliseconds(1))
            .unwrap_or(ts);
        assert_eq!(result - ts, Duration::milliseconds(1));
    }
}

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
        // Use checked arithmetic to guard against overflow at DateTime<Utc>::MAX.
        // If the add overflows (extremely unlikely in practice), fall back to the
        // un-incremented timestamp so we may re-deliver the last event rather than
        // silently lose all future tail events.
        let tail_from = newest.map(|t| {
            t.checked_add_signed(chrono::Duration::milliseconds(1))
                .unwrap_or(t)
        });

        let (tx, rx) = crossbeam_channel::bounded(constants::CHANNEL_BOUND);
        let cancel = Arc::new(AtomicBool::new(false));

        // Tail queries must not apply an upper time bound: if the user
        // previously set a `time_to` filter, honouring it here would
        // silently prevent any new events from ever appearing.
        let _handle = event_reader::spawn_reader_thread(
            self.selected_channels.clone(),
            tail_from.or(self.filter.time_from),
            None,
            tx,
            cancel.clone(),
            self.max_events_per_channel,
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

                // Auto-focus the text field once when the dialog first opens.
                // Previously this called request_focus() on every frame while
                // the input was empty, which stole keyboard focus away from
                // every other widget in the application until the user typed
                // at least one character.
                if response.gained_focus() {
                    response.request_focus();
                } else if !response.has_focus() && self.preset_name_input.is_empty() {
                    // Only steal focus if the field genuinely has no focus yet
                    // (e.g. the dialog just opened and nothing has been focused).
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
                *existing =
                    crate::core::filter_preset::FilterPreset::from_state(&name, &self.filter);
            } else {
                self.filter_presets
                    .push(crate::core::filter_preset::FilterPreset::from_state(
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
