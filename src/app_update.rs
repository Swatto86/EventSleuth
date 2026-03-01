//! Frame-by-frame update loop and core processing logic.
//!
//! Contains the [`eframe::App`] implementation for `EventSleuthApp`,
//! plus the background-message processing, filtering, sorting, and
//! selection helpers that the update loop depends on.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::app::{EventSleuthApp, SortColumn};
use crate::core::event_reader::{self, ReaderMessage};
use crate::core::event_record::EventRecord;
use crate::util::constants;

// ── Core logic ──────────────────────────────────────────────────────────

impl EventSleuthApp {
    /// Start (or restart) loading events from the selected channels.
    ///
    /// Cancels any in-progress load, clears existing data, and spawns
    /// a new reader background thread.
    pub fn start_loading(&mut self) {
        // Cancel any existing reader
        self.cancel_loading();

        // Reset the tail flag so a full load is never misidentified as a
        // tail completion in `process_messages` (fixes incorrect status
        // text when the user refreshes while live-tail is running).
        self.is_tail_query = false;

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

        // Bookmarks reference indices into all_events, so they become
        // invalid after a reload and must be cleared.
        self.bookmarked_indices.clear();
        self.show_bookmarks_only = false;

        // Invalidate the stats cache immediately so a zero-event query
        // never leaves the panel showing the previous run's data.
        self.stats_dirty = true;

        // Force a filter pass (even with no events) so that all derived
        // state (stats_dirty, filtered_indices) is consistent on
        // the first frame after loading starts.
        self.needs_refilter = true;

        // Create communication channel and cancellation flag
        let (tx, rx) = crossbeam_channel::bounded::<ReaderMessage>(constants::CHANNEL_BOUND);
        let cancel = Arc::new(AtomicBool::new(false));

        // Spawn background reader thread
        let max_ev = self.max_events_per_channel;
        let _handle = event_reader::spawn_reader_thread(
            self.selected_channels.clone(),
            self.filter.time_from,
            self.filter.time_to,
            tx,
            cancel.clone(),
            max_ev,
        );

        self.reader_rx = Some(rx);
        self.cancel_flag = Some(cancel);
        self.is_loading = true;
        self.status_text = "Loading...".into();
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
    pub(crate) fn process_messages(&mut self) {
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

                    // Guard against unbounded memory growth during live-tail.
                    //
                    // A full load is bounded by `max_events_per_channel` * channels,
                    // but each tail poll appends without removing anything.  On a
                    // busy system this can exhaust memory over extended sessions.
                    //
                    // When the cap is hit we evict the oldest events from the front
                    // of `all_events` (cheapest option: O(n) drain).  After eviction:
                    //  • `filtered_indices` is invalidated and rebuilt on the next
                    //    frame via `needs_refilter = true`.
                    //  • `selected_event_idx` is cleared to avoid a stale visual
                    //    highlight that would point to the wrong event after eviction.
                    //  • `bookmarked_indices` are cleared because they are raw indices
                    //    into `all_events` whose values shift after the drain.  We
                    //    cannot remap them cheaply without a reverse lookup map.
                    if self.is_tail_query && self.all_events.len() > constants::MAX_TOTAL_EVENTS_CAP
                    {
                        let evict = self.all_events.len() - constants::MAX_TOTAL_EVENTS_CAP;
                        self.all_events.drain(0..evict);
                        self.filtered_indices.clear();
                        self.selected_event_idx = None;
                        if !self.bookmarked_indices.is_empty() {
                            self.bookmarked_indices.clear();
                            self.show_bookmarks_only = false;
                            tracing::debug!(
                                "Cleared bookmarks after evicting {} oldest events \
                                 (live-tail cap {} reached)",
                                evict,
                                constants::MAX_TOTAL_EVENTS_CAP,
                            );
                        }
                        tracing::debug!(
                            "Evicted {} oldest events to stay within live-tail cap of {}",
                            evict,
                            constants::MAX_TOTAL_EVENTS_CAP,
                        );
                    }

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
                    // Always invalidate the stats cache when a query finishes,
                    // including the zero-event case where no EventBatch
                    // messages arrived and needs_refilter was never set.
                    self.stats_dirty = true;
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
    ///
    /// Reuses the existing `filtered_indices` allocation to avoid a heap
    /// allocation on every filter pass (significant for repeated filtering
    /// during text search with debounce).
    pub fn apply_filter(&mut self) {
        // Remember which underlying event was selected so we can restore
        // the highlight after the filtered/sorted index list changes.
        let prev_event_idx = self
            .selected_event_idx
            .and_then(|vis| self.filtered_indices.get(vis).copied());

        self.filtered_indices.clear();
        self.filtered_indices.extend(
            self.all_events
                .iter()
                .enumerate()
                .filter(|(i, event)| {
                    // When bookmarks-only mode is active, skip non-bookmarked events.
                    if self.show_bookmarks_only && !self.bookmarked_indices.contains(i) {
                        return false;
                    }
                    self.filter.matches(event)
                })
                .map(|(i, _)| i),
        );

        self.sort_events();

        // Restore selection: find the previously-selected event in the
        // new filtered list. Falls back to clamping if the event was
        // filtered out.
        if let Some(ev_idx) = prev_event_idx {
            self.selected_event_idx = self.filtered_indices.iter().position(|&i| i == ev_idx);
        }

        // Clamp selection to valid range (covers the case where the
        // previously-selected event was filtered out).
        if let Some(idx) = self.selected_event_idx {
            if idx >= self.filtered_indices.len() {
                self.selected_event_idx = if self.filtered_indices.is_empty() {
                    None
                } else {
                    Some(self.filtered_indices.len() - 1)
                };
            }
        }

        self.needs_refilter = false;
        self.stats_dirty = true;
    }

    /// Sort `filtered_indices` by the current sort column and direction.
    ///
    /// Uses `sort_unstable_by` for better performance on index slices
    /// (no stability guarantees needed for indices; avoids temporary allocation).
    pub fn sort_events(&mut self) {
        let events = &self.all_events;
        let col = self.sort_column;
        let asc = self.sort_ascending;

        self.filtered_indices.sort_unstable_by(|&a, &b| {
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
    ///
    /// Delegates to the pure helper [`security_access_error_in_list`] so
    /// the logic can be tested without constructing a full `EventSleuthApp`.
    pub fn has_security_access_error(&self) -> bool {
        security_access_error_in_list(&self.errors)
    }

    /// Poll the import file-selection channel for a user-chosen .evtx path.
    pub(crate) fn process_import_selection(&mut self) {
        let path = {
            let rx = match &self.import_rx {
                Some(rx) => rx,
                None => return,
            };
            match rx.try_recv() {
                Ok(p) => p,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // Sender dropped without sending (user cancelled the file dialog).
                    self.import_rx = None;
                    return;
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // Still waiting for the user to pick a file.
                    return;
                }
            }
        };
        self.import_rx = None;
        self.start_loading_evtx(&path);
    }
}

// ── Security banner helper (pure, testable) ─────────────────────────────

/// Returns `true` when `errors` contains a Security-channel entry whose
/// message indicates an access-denied failure.
///
/// Checks for the HRESULT `0x80070005` string representation (`"80070005"`)
/// produced by `EventSleuthError::WindowsApi`, or a generic "access"
/// substring for broader safety.  The old check for `"00000005"` (raw Win32
/// code) has been removed: `EventSleuthError` always formats errors as
/// HRESULTs and the raw Win32 code string can never appear in the output.
pub(crate) fn security_access_error_in_list(errors: &[(String, String)]) -> bool {
    errors.iter().any(|(ch, err)| {
        ch == "Security" && (err.contains("80070005") || err.to_lowercase().contains("access"))
    })
}

#[cfg(test)]
mod security_banner_tests {
    use super::security_access_error_in_list;

    fn e(ch: &str, msg: &str) -> (String, String) {
        (ch.to_owned(), msg.to_owned())
    }

    /// Regression test for Bug 1: HRESULT 0x80070005 must trigger the banner.
    #[test]
    fn hresult_access_denied_triggers_banner() {
        let errors = vec![e(
            "Security",
            "Windows API error: EvtQuery on channel 'Security' (HRESULT: 0x80070005)",
        )];
        assert!(
            security_access_error_in_list(&errors),
            "HRESULT 0x80070005 in Security channel error must trigger security banner"
        );
    }

    /// Regression test for Bug 1: the previously dead-code check `"00000005"`
    /// (raw Win32 code) must NOT independently trigger the banner.
    #[test]
    fn raw_win32_code_string_alone_does_not_trigger_banner() {
        // A message containing only the "00000005" raw code string (no HRESULT
        // prefix, no "access" substring) must NOT trigger the banner.
        let errors = vec![e("Security", "error code: 00000005")];
        assert!(
            !security_access_error_in_list(&errors),
            "Raw Win32 code string '00000005' alone must NOT trigger the security banner"
        );
    }

    /// Non-Security channels must never trigger the banner, even on access errors.
    #[test]
    fn non_security_channel_never_triggers_banner() {
        let errors = vec![e(
            "Application",
            "Windows API error: EvtQuery (HRESULT: 0x80070005)",
        )];
        assert!(
            !security_access_error_in_list(&errors),
            "Access-denied on a non-Security channel must not trigger the security banner"
        );
    }

    /// Generic "access denied" text in a Security-channel error triggers the banner.
    #[test]
    fn generic_access_text_triggers_banner() {
        let errors = vec![e("Security", "Access denied: channel cannot be opened")];
        assert!(
            security_access_error_in_list(&errors),
            "Access-denied text in Security channel error must trigger the security banner"
        );
    }

    /// Empty error list must not trigger the banner.
    #[test]
    fn empty_errors_no_banner() {
        assert!(
            !security_access_error_in_list(&[]),
            "Empty error list must not trigger the security banner"
        );
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
            let elapsed = timer.elapsed();
            if elapsed >= debounce {
                self.filter.parse_event_ids();
                self.filter.parse_time_range();
                self.needs_refilter = true;
                self.debounce_timer = None;
            } else {
                ctx.request_repaint_after(debounce - elapsed);
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
                Some(t) => {
                    t.elapsed()
                        >= std::time::Duration::from_secs(constants::LIVE_TAIL_INTERVAL_SECS)
                }
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
            .exact_height(38.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_toolbar(ui);
            });

        // ── Bottom status bar ───────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(28.0)
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
            .default_width(230.0)
            .min_width(180.0)
            .max_width(380.0)
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
                    .fill(crate::ui::theme::security_banner_bg(self.dark_mode))
                    .inner_margin(egui::Margin::same(6))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("\u{26A0} Security log access denied.")
                                    .color(crate::ui::theme::level_color(3, self.dark_mode))
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Run EventSleuth as Administrator to view Security events.",
                                )
                                .color(crate::ui::theme::text_secondary(self.dark_mode)),
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
        self.render_stats_panel(ctx);
    }

    /// Return the clear colour used before each frame render.
    ///
    /// Matches the themed background so the GPU clear is the same
    /// colour as the app background, eliminating any flash.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.dark_mode {
            crate::ui::theme::BG_DARK.to_normalized_gamma_f32()
        } else {
            crate::ui::theme::BG_LIGHT.to_normalized_gamma_f32()
        }
    }

    /// Persist user preferences to eframe storage on shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "dark_mode", &self.dark_mode);
        eframe::set_value(storage, "selected_channels", &self.selected_channels);
        eframe::set_value(storage, "filter_presets", &self.filter_presets);
        eframe::set_value(
            storage,
            "max_events_per_channel",
            &self.max_events_per_channel,
        );
        eframe::set_value(storage, "column_visibility", &self.column_visibility);
    }
}
