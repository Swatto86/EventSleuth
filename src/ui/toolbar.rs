//! Top toolbar: channel selector, refresh, and export buttons.

use crate::app::EventSleuthApp;
use crate::ui::theme;

impl EventSleuthApp {
    /// Render the top toolbar within the given `Ui` region.
    ///
    /// Contains the channel selector button, refresh / cancel controls,
    /// and export dropdown.
    pub fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // ── Channel selector button ─────────────────────────────
            let channel_label = if self.selected_channels.is_empty() {
                "Select Channels…".to_string()
            } else if self.selected_channels.len() == 1 {
                self.selected_channels[0].clone()
            } else {
                format!("{} channels", self.selected_channels.len())
            };

            if ui
                .button(format!("# {channel_label}"))
                .on_hover_text("Choose which log channels to query")
                .clicked()
            {
                self.show_channel_selector = !self.show_channel_selector;
            }

            ui.separator();

            // ── Refresh / Cancel ────────────────────────────────────
            if self.is_loading {
                ui.spinner();
                if ui.button("Stop").clicked() {
                    self.cancel_loading();
                }
            } else {
                let refresh = ui
                    .button("Refresh")
                    .on_hover_text("Re-query selected channels");
                if refresh.clicked() {
                    self.start_loading();
                }
            }

            ui.separator();

            // ── Export dropdown ──────────────────────────────────────
            ui.menu_button("Export", |ui| {
                if ui.button("Export to CSV…").clicked() {
                    self.export_csv();
                    ui.close_menu();
                }
                if ui.button("Export to JSON…").clicked() {
                    self.export_json();
                    ui.close_menu();
                }
            });

            // ── Right-aligned app title + about ───────────────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("(i)")
                    .on_hover_text("About EventSleuth")
                    .clicked()
                {
                    self.show_about = true;
                }
                ui.label(
                    egui::RichText::new("EventSleuth")
                        .color(theme::ACCENT)
                        .strong()
                        .size(16.0),
                );
            });
        });
    }

    /// Render the channel selector popup window (if visible).
    ///
    /// Shows a searchable list of all discovered channels with checkboxes.
    pub fn render_channel_selector(&mut self, ctx: &egui::Context) {
        if !self.show_channel_selector {
            return;
        }

        let mut open = true;
        egui::Window::new("Select Channels")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(350.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // Search box
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.channel_search);
                });

                ui.separator();

                // Quick select / deselect
                ui.horizontal(|ui| {
                    if ui.small_button("Common").clicked() {
                        self.selected_channels =
                            crate::core::channel_enumerator::common_channels(&self.channels);
                    }
                    if ui.small_button("All").clicked() {
                        self.selected_channels = self.channels.clone();
                    }
                    if ui.small_button("None").clicked() {
                        self.selected_channels.clear();
                    }
                });

                ui.separator();

                // Channel list with checkboxes
                let search_lower = self.channel_search.to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for channel in &self.channels.clone() {
                            // Filter by search
                            if !search_lower.is_empty()
                                && !channel.to_lowercase().contains(&search_lower)
                            {
                                continue;
                            }

                            let mut selected = self.selected_channels.contains(channel);
                            if ui.checkbox(&mut selected, channel).changed() {
                                if selected {
                                    self.selected_channels.push(channel.clone());
                                } else {
                                    self.selected_channels.retain(|c| c != channel);
                                }
                            }
                        }
                    });
            });

        if !open {
            self.show_channel_selector = false;
        }
    }
}
