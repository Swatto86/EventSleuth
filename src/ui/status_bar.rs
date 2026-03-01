//! Bottom status bar: event counts, query time, and loading status.
//!
//! The error indicator uses a coloured badge so operators notice problems
//! even if the toolbar is busy.

use crate::app::EventSleuthApp;
use crate::ui::theme;
use crate::util::time::format_duration;

impl EventSleuthApp {
    /// Render the status bar at the bottom of the window.
    ///
    /// Shows: filtered/total counts | query time | status indicator | errors.
    pub fn render_status_bar(&self, ui: &mut egui::Ui) {
        let dark = self.dark_mode;
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
            if self.is_loading {
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
            } else if let Some((ref msg, _)) = self.export_message {
                ui.label(egui::RichText::new(msg.as_str()).color(theme::accent(dark)));
            } else if self.live_tail {
                let since = self
                    .last_tail_time
                    .map(|t| format!("{}s ago", t.elapsed().as_secs()))
                    .unwrap_or_else(|| "starting".into());
                ui.label(
                    egui::RichText::new(format!("Live tail (last: {since})"))
                        .color(theme::accent(dark)),
                );
            } else {
                ui.label(egui::RichText::new("Ready").color(theme::accent_dim(dark)));
            }

            // ── Errors indicator (right-aligned, with badge) ────────
            if !self.errors.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count = self.errors.len();
                    theme::badge(ui, count, theme::error_badge_bg(dark), egui::Color32::WHITE);
                    let response = ui.label(
                        egui::RichText::new(format!(
                            "\u{26A0} {}",
                            if count == 1 { "error" } else { "errors" }
                        ))
                        .color(theme::level_color(2, dark)),
                    );
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
    }
}
