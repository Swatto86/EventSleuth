//! Bottom status bar: event counts, query time, and loading status.

use crate::app::EventSleuthApp;
use crate::ui::theme;
use crate::util::time::format_duration;

impl EventSleuthApp {
    /// Render the status bar at the bottom of the window.
    ///
    /// Shows: filtered/total counts | query time | status indicator.
    pub fn render_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            // ── Event count ─────────────────────────────────────────
            let filtered = self.filtered_indices.len();
            let total = self.all_events.len();
            let count_text = if filtered == total {
                format!("{} events", total)
            } else {
                format!("Showing {} of {} events", filtered, total)
            };
            ui.label(egui::RichText::new(count_text).color(theme::TEXT_SECONDARY));

            ui.separator();

            // ── Query time ──────────────────────────────────────────
            if let Some(elapsed) = self.query_elapsed {
                ui.label(
                    egui::RichText::new(format!("Query: {}", format_duration(elapsed)))
                        .color(theme::TEXT_DIM),
                );
                ui.separator();
            }

            // ── Loading status ──────────────────────────────────────
            if self.is_loading {
                ui.spinner();
                let progress = format!(
                    "Loading… {} events ({})",
                    self.progress_count, self.progress_channel
                );
                ui.label(egui::RichText::new(progress).color(theme::TEXT_SECONDARY));
            } else {
                ui.label(egui::RichText::new("Ready").color(theme::ACCENT_DIM));
            }

            // ── Errors indicator ────────────────────────────────────────
            if !self.errors.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let err_text = format!("!! {} error(s)", self.errors.len());
                    let response =
                        ui.label(egui::RichText::new(err_text).color(theme::LEVEL_WARNING));
                    // Show error details on hover
                    response.on_hover_ui(|ui| {
                        for (ch, msg) in &self.errors {
                            ui.label(
                                egui::RichText::new(format!("{ch}: {msg}"))
                                    .color(theme::LEVEL_ERROR)
                                    .small(),
                            );
                        }
                    });
                });
            }
        });
    }
}
