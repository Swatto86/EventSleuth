//! Detail panel: displays all fields of the currently selected event.
//!
//! Provides two tabs — **Details** (formatted view with event data table)
//! and **XML** (raw XML string in a monospaced scrollable area).

use crate::app::{DetailTab, EventSleuthApp};
use crate::ui::theme;
use crate::util::time::format_detail_timestamp;

impl EventSleuthApp {
    /// Render the bottom detail panel for the currently selected event.
    pub fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        let event = match self.selected_event() {
            Some(e) => e.clone(),
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("👆 Select an event to view details")
                            .color(theme::TEXT_DIM),
                    );
                });
                return;
            }
        };

        // ── Tab bar ─────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.detail_tab,
                DetailTab::Details,
                egui::RichText::new("📝 Details").strong(),
            );
            ui.selectable_value(
                &mut self.detail_tab,
                DetailTab::Xml,
                egui::RichText::new("📄 XML").strong(),
            );

            // Copy buttons on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("📋 Copy XML").clicked() {
                    ui.ctx().copy_text(event.raw_xml.clone());
                }
                if ui.small_button("📋 Copy Message").clicked() {
                    ui.ctx().copy_text(event.message.clone());
                }
            });
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.detail_tab {
                DetailTab::Details => self.render_detail_formatted(ui, &event),
                DetailTab::Xml => self.render_detail_xml(ui, &event),
            }
        });
    }

    /// Render the formatted details view: header fields, message, event data.
    fn render_detail_formatted(&self, ui: &mut egui::Ui, event: &crate::core::event_record::EventRecord) {
        let level_color = theme::level_color(event.level);

        // ── Header grid ─────────────────────────────────────────────
        egui::Grid::new("detail_header_grid")
            .num_columns(4)
            .striped(false)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                // Row 1
                ui.label(egui::RichText::new("Event ID").color(theme::TEXT_DIM));
                ui.label(event.event_id.to_string());
                ui.label(egui::RichText::new("Level").color(theme::TEXT_DIM));
                ui.label(egui::RichText::new(&event.level_name).color(level_color));
                ui.end_row();

                // Row 2
                ui.label(egui::RichText::new("Provider").color(theme::TEXT_DIM));
                ui.label(&event.provider_name);
                ui.label(egui::RichText::new("Channel").color(theme::TEXT_DIM));
                ui.label(&event.channel);
                ui.end_row();

                // Row 3
                ui.label(egui::RichText::new("Timestamp").color(theme::TEXT_DIM));
                ui.label(format_detail_timestamp(&event.timestamp));
                ui.label(egui::RichText::new("Computer").color(theme::TEXT_DIM));
                ui.label(&event.computer);
                ui.end_row();

                // Row 4
                ui.label(egui::RichText::new("Process ID").color(theme::TEXT_DIM));
                ui.label(event.process_id.to_string());
                ui.label(egui::RichText::new("Thread ID").color(theme::TEXT_DIM));
                ui.label(event.thread_id.to_string());
                ui.end_row();

                // Row 5 (optional fields)
                if let Some(ref sid) = event.user_sid {
                    ui.label(egui::RichText::new("User SID").color(theme::TEXT_DIM));
                    ui.label(sid);
                } else {
                    ui.label("");
                    ui.label("");
                }
                if let Some(ref aid) = event.activity_id {
                    ui.label(egui::RichText::new("Activity ID").color(theme::TEXT_DIM));
                    ui.label(aid);
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();
            });

        ui.add_space(8.0);

        // ── Message ─────────────────────────────────────────────────
        ui.label(egui::RichText::new("💬 Message").color(theme::ACCENT).strong());
        ui.separator();

        let msg = event.display_message();
        if msg.is_empty() || msg == "(no message)" {
            ui.label(
                egui::RichText::new("(no formatted message available)")
                    .color(theme::TEXT_DIM)
                    .italics(),
            );
        } else {
            // Format the message with word-wrap and readable styling
            ui.label(
                egui::RichText::new(msg)
                    .color(theme::TEXT_PRIMARY)
                    .size(13.0),
            );
        }

        // ── Event Data table ────────────────────────────────────────
        if !event.event_data.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("📊 Event Data")
                    .color(theme::ACCENT)
                    .strong(),
            );
            ui.separator();

            egui::Grid::new("event_data_grid")
                .num_columns(2)
                .striped(true)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    // Header row
                    ui.label(egui::RichText::new("Name").color(theme::TEXT_DIM).strong());
                    ui.label(egui::RichText::new("Value").color(theme::TEXT_DIM).strong());
                    ui.end_row();

                    for (key, value) in &event.event_data {
                        ui.label(egui::RichText::new(key).color(theme::TEXT_SECONDARY));
                        // Wrap long values
                        let display = if value.len() > 500 {
                            format!("{}… ({} chars)", &value[..500], value.len())
                        } else {
                            value.clone()
                        };
                        ui.label(&display);
                        ui.end_row();
                    }
                });
        }
    }

    /// Render the raw XML view with monspace font in a scrollable area.
    fn render_detail_xml(&self, ui: &mut egui::Ui, event: &crate::core::event_record::EventRecord) {
        ui.label(
            egui::RichText::new(&event.raw_xml)
                .monospace()
                .size(12.0)
                .color(theme::TEXT_SECONDARY),
        );
    }
}
