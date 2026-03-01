//! Bottom status bar: event counts, query time, and loading status.

use crate::app::EventSleuthApp;
use crate::ui::theme;
use crate::util::time::format_duration;

impl EventSleuthApp {
    /// Render the status bar at the bottom of the window.
    ///
    /// Shows: filtered/total counts | query time | status indicator.
    pub fn render_status_bar(&self, ui: &mut egui::Ui) {
        let dark = self.dark_mode;
        ui.horizontal_centered(|ui| {
            // ── Event count ─────────────────────────────────────────
            let filtered = self.filtered_indices.len();
            let total = self.all_events.len();
            let count_text = if filtered == total {
                format!("{} events", total)
            } else {
                format!("Showing {} of {} events", filtered, total)
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
                // Show export feedback briefly
                ui.label(egui::RichText::new(msg.as_str()).color(theme::accent(dark)));
            } else if self.live_tail {
                let since = self
                    .last_tail_time
                    .map(|t| format!("{}s ago", t.elapsed().as_secs()))
                    .unwrap_or_else(|| "starting".into());
                ui.label(
                    egui::RichText::new(format!("Live tail active (last check: {since})"))
                        .color(theme::accent(dark)),
                );
            } else {
                ui.label(egui::RichText::new("Ready").color(theme::accent_dim(dark)));
            }

            // ── Errors indicator ────────────────────────────────────────
            if !self.errors.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let err_text = format!("⚠ {} error(s)", self.errors.len());
                    let response =
                        ui.label(egui::RichText::new(err_text).color(theme::level_color(3, dark)));
                    // Show error details on hover
                    response.on_hover_ui(|ui| {
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
