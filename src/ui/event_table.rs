//! Central event table with virtual scrolling and sortable columns.
//!
//! Uses `egui_extras::TableBuilder` for column layout, which provides
//! built-in virtual scrolling via its `body.rows()` method — only
//! visible rows are laid out, keeping performance smooth with 100k+ events.

use crate::app::{EventSleuthApp, SortColumn};
use crate::ui::theme;
use crate::util::constants::TABLE_ROW_HEIGHT;
use crate::util::time::format_table_timestamp;
use egui_extras::{Column, TableBuilder};

impl EventSleuthApp {
    /// Render the virtual-scrolled event table in the central panel.
    ///
    /// Columns: Timestamp, Level, ID, Provider, Message.
    /// Clicking a header sorts by that column (toggle asc/desc).
    /// Clicking a row selects it and shows details.
    pub fn render_event_table(&mut self, ui: &mut egui::Ui) {
        let row_count = self.filtered_indices.len();

        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(145.0).clip(true)) // Timestamp
            .column(Column::auto().at_least(85.0).clip(true)) // Level
            .column(Column::auto().at_least(55.0)) // Event ID
            .column(Column::auto().at_least(140.0).clip(true)) // Provider
            .column(Column::remainder().clip(true)) // Message
            .sense(egui::Sense::click());

        table
            .header(22.0, |mut header| {
                // Timestamp column header
                header.col(|ui| {
                    self.render_sort_header(ui, SortColumn::Timestamp, "Timestamp");
                });
                // Level column header
                header.col(|ui| {
                    self.render_sort_header(ui, SortColumn::Level, "Level");
                });
                // Event ID column header
                header.col(|ui| {
                    self.render_sort_header(ui, SortColumn::EventId, "ID");
                });
                // Provider column header
                header.col(|ui| {
                    self.render_sort_header(ui, SortColumn::Provider, "Provider");
                });
                // Message column header
                header.col(|ui| {
                    self.render_sort_header(ui, SortColumn::Message, "Message");
                });
            })
            .body(|body| {
                body.rows(TABLE_ROW_HEIGHT, row_count, |mut row| {
                    let visible_idx = row.index();
                    if visible_idx >= self.filtered_indices.len() {
                        return;
                    }
                    let event_idx = self.filtered_indices[visible_idx];
                    let event = &self.all_events[event_idx];
                    let is_selected = self.selected_event_idx == Some(visible_idx);
                    let dark = self.dark_mode;
                    let level_color = theme::level_color(event.level, dark);

                    // If selected, set the row colour
                    row.set_selected(is_selected);

                    // Timestamp
                    row.col(|ui| {
                        ui.label(
                            egui::RichText::new(format_table_timestamp(&event.timestamp))
                                .color(theme::text_secondary(dark))
                                .small(),
                        );
                    });

                    // Level (colour-coded)
                    row.col(|ui| {
                        ui.label(egui::RichText::new(&event.level_name).color(level_color));
                    });

                    // Event ID
                    row.col(|ui| {
                        ui.label(event.event_id.to_string());
                    });

                    // Provider
                    row.col(|ui| {
                        ui.label(
                            egui::RichText::new(&event.provider_name)
                                .color(theme::text_secondary(dark)),
                        );
                    });

                    // Message (truncated to one line)
                    row.col(|ui| {
                        let msg = event.display_message();
                        // Truncate long messages for the table view (char-safe)
                        let display = if msg.chars().count() > 200 {
                            let end = msg.char_indices()
                                .nth(200)
                                .map(|(i, _)| i)
                                .unwrap_or(msg.len());
                            format!("{}...", &msg[..end])
                        } else {
                            msg.to_string()
                        };
                        ui.label(&display);
                    });

                    // Handle row click → select
                    if row.response().clicked() {
                        self.selected_event_idx = Some(visible_idx);
                    }
                });
            });
    }

    /// Render a sortable column header button.
    ///
    /// Shows an arrow indicator for the current sort column and toggles
    /// direction on click.
    fn render_sort_header(&mut self, ui: &mut egui::Ui, column: SortColumn, label: &str) {
        let is_current = self.sort_column == column;
        let arrow = if is_current {
            if self.sort_ascending { " ▲" } else { " ▼" }
        } else {
            ""
        };

        let text = format!("{label}{arrow}");
        let dark = self.dark_mode;
        let rich = if is_current {
            egui::RichText::new(text).color(theme::accent(dark)).strong()
        } else {
            egui::RichText::new(text).color(theme::text_primary(dark))
        };

        if ui.button(rich).clicked() {
            if is_current {
                // Toggle direction
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = column;
                // Default: newest first for timestamp, ascending for others
                self.sort_ascending = column != SortColumn::Timestamp;
            }
            self.sort_events();
        }
    }
}
