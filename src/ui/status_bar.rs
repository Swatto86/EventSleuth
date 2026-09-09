//! Bottom status bar: event counts, query time, and loading status.
//!
//! The error indicator uses a coloured badge so operators notice problems
//! even if the toolbar is busy.

use crate::app::EventSleuthApp;
use crate::ui::theme;
use crate::util::time::format_duration;

/// Which message the status bar shows in its single status slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusDisplay {
    Loading,
    /// An export is in flight (save dialog open or file being written).
    Exporting,
    /// The result of the last export.
    ExportMessage,
    LiveTail,
    StatusText,
    Ready,
}

/// Pick the status-bar message, in priority order.
///
/// A running export must be visible while it happens, otherwise a long write
/// leaves the window looking inert; it therefore outranks the stale result of
/// the previous export.
pub(crate) fn status_display(
    is_loading: bool,
    export_pending: bool,
    has_export_message: bool,
    live_tail: bool,
    has_status_text: bool,
) -> StatusDisplay {
    if is_loading {
        StatusDisplay::Loading
    } else if export_pending {
        StatusDisplay::Exporting
    } else if has_export_message {
        StatusDisplay::ExportMessage
    } else if live_tail {
        StatusDisplay::LiveTail
    } else if has_status_text {
        StatusDisplay::StatusText
    } else {
        StatusDisplay::Ready
    }
}

impl EventSleuthApp {
    /// Render the status bar at the bottom of the window.
    ///
    /// Shows: filtered/total counts | query time | status indicator | errors.
    pub fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        let dark = self.dark_mode;
        // Applied after the layout closure, which borrows `self` immutably.
        let mut toggle_errors = false;
        ui.horizontal_centered(|ui| {
            // ── Event count ─────────────────────────────────────────
            let filtered = self.filtered_indices.len();
            let total = self.all_events.len();
            let count_text = if filtered == total {
                format!("{} events", total)
            } else {
                format!("{} of {} events", filtered, total)
            };
            ui.label(egui::RichText::new(count_text).color(theme::text_secondary(dark)));

            ui.separator();

            // ── Query time ──────────────────────────────────────────
            if let Some(elapsed) = self.query_elapsed {
                ui.label(
                    egui::RichText::new(format!("Query: {}", format_duration(elapsed)))
                        .color(theme::text_dim(dark)),
                );
                ui.separator();
            }

            // ── Loading status ──────────────────────────────────────
            match status_display(
                self.is_loading,
                self.export_rx.is_some(),
                self.export_message.is_some(),
                self.live_tail,
                !self.status_text.is_empty(),
            ) {
                StatusDisplay::Loading => {
                    ui.spinner();
                    let progress = if self.is_tail_query {
                        "Checking for new events...".to_string()
                    } else {
                        format!(
                            "Loading... {} events ({})",
                            self.progress_count, self.progress_channel
                        )
                    };
                    ui.label(egui::RichText::new(progress).color(theme::text_secondary(dark)));
                }
                StatusDisplay::Exporting => {
                    ui.spinner();
                    ui.label(
                        egui::RichText::new("Exporting\u{2026}").color(theme::text_secondary(dark)),
                    );
                }
                StatusDisplay::ExportMessage => {
                    if let Some((ref msg, _)) = self.export_message {
                        ui.label(egui::RichText::new(msg.as_str()).color(theme::accent(dark)));
                    }
                }
                StatusDisplay::LiveTail => {
                    let since = self
                        .last_tail_time
                        .map(|t| format!("{}s ago", t.elapsed().as_secs()))
                        .unwrap_or_else(|| "starting".into());
                    ui.label(
                        egui::RichText::new(format!("Live tail (last: {since})"))
                            .color(theme::accent(dark)),
                    );
                }
                StatusDisplay::StatusText => {
                    // Status messages set by the app ("Loaded N events",
                    // "No sources selected", reader-crash diagnostics, ...).
                    ui.label(
                        egui::RichText::new(self.status_text.as_str())
                            .color(theme::accent_dim(dark)),
                    );
                }
                StatusDisplay::Ready => {
                    ui.label(egui::RichText::new("Ready").color(theme::accent_dim(dark)));
                }
            }

            // ── Bookmarks-discarded notice ──────────────────────────
            if let Some((n, when)) = self.bookmark_notice {
                if when.elapsed() < std::time::Duration::from_secs(10) {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "\u{2B50} {} bookmark(s) cleared by reload",
                            n
                        ))
                        .color(theme::level_color(3, dark)),
                    );
                }
            }

            // ── Errors indicator (right-aligned, with badge) ────────
            if !self.errors.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count = self.errors.len();
                    theme::badge(ui, count, theme::error_badge_bg(dark), egui::Color32::WHITE);
                    // A button (not a label) so the indicator is focusable and
                    // activatable from the keyboard: the hover tooltip below is
                    // an extra, not the only way to read the error detail.
                    let response = ui.button(
                        egui::RichText::new(format!(
                            "\u{26A0} {}",
                            if count == 1 { "error" } else { "errors" }
                        ))
                        .color(theme::level_color(2, dark)),
                    );
                    if response.clicked() {
                        toggle_errors = true;
                    }
                    response.on_hover_ui(|ui| {
                        ui.label(
                            egui::RichText::new("Errors from the last query:")
                                .color(theme::text_secondary(dark))
                                .strong(),
                        );
                        ui.separator();
                        for (ch, msg) in &self.errors {
                            ui.label(
                                egui::RichText::new(format!("{ch}: {msg}"))
                                    .color(theme::level_color(2, dark))
                                    .small(),
                            );
                        }
                    });
                });
            }
        });

        if toggle_errors {
            self.show_errors = !self.show_errors;
        }
    }

    /// Render the error-details window (opened from the status-bar error button).
    pub fn render_errors_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_errors {
            return;
        }
        let dark = self.dark_mode;
        let mut open = true;
        egui::Window::new("\u{26A0} Query Errors")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                if self.errors.is_empty() {
                    ui.label(
                        egui::RichText::new("No errors from the last query.")
                            .color(theme::text_secondary(dark)),
                    );
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (ch, msg) in &self.errors {
                        ui.label(
                            egui::RichText::new(format!("{ch}: {msg}"))
                                .color(theme::level_color(2, dark)),
                        );
                        ui.separator();
                    }
                });
                if ui.button("Copy all").clicked() {
                    let all: Vec<String> = self
                        .errors
                        .iter()
                        .map(|(ch, msg)| format!("{ch}: {msg}"))
                        .collect();
                    ui.ctx().copy_text(all.join("\n"));
                }
            });
        if !open {
            self.show_errors = false;
        }
    }
}

#[cfg(test)]
mod status_display_tests {
    use super::{status_display, StatusDisplay};

    /// Regression test: an in-flight export must be shown, rather than leaving
    /// the previous export's message (or "Loaded N events") on screen while the
    /// file is written.
    #[test]
    fn pending_export_shows_a_busy_state() {
        assert_eq!(
            status_display(false, true, false, false, true),
            StatusDisplay::Exporting
        );
    }

    /// The busy state outranks the previous export's result message.
    #[test]
    fn pending_export_outranks_stale_export_message() {
        assert_eq!(
            status_display(false, true, true, false, false),
            StatusDisplay::Exporting
        );
    }

    /// Once the export finishes its result is shown.
    #[test]
    fn finished_export_shows_its_message() {
        assert_eq!(
            status_display(false, false, true, false, false),
            StatusDisplay::ExportMessage
        );
    }

    /// A running query still takes priority over everything else.
    #[test]
    fn loading_takes_priority() {
        assert_eq!(
            status_display(true, true, true, true, true),
            StatusDisplay::Loading
        );
    }

    /// With nothing happening the bar reads "Ready".
    #[test]
    fn idle_is_ready() {
        assert_eq!(
            status_display(false, false, false, false, false),
            StatusDisplay::Ready
        );
    }
}
