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

            // ── Source selector button ─────────────────────────
            let channel_label = if self.selected_channels.is_empty() {
                "Select Sources…".to_string()
            } else if self.selected_channels.len() == 1 {
                self.selected_channels[0].clone()
            } else {
                format!("{} sources", self.selected_channels.len())
            };

            if ui
                .button(format!("📋 {channel_label}"))
                .on_hover_text("Choose which log sources to query")
                .clicked()
            {
                self.show_channel_selector = !self.show_channel_selector;
            }

            ui.separator();

            // ── Refresh / Cancel ────────────────────────────────────
            if self.is_loading {
                ui.spinner();
                if ui.button("⏹ Stop").clicked() {
                    self.cancel_loading();
                }
            } else {
                let refresh = ui
                    .button("🔄 Refresh")
                    .on_hover_text("Re-query selected sources");
                if refresh.clicked() {
                    self.start_loading();
                }
            }

            ui.separator();

            // ── Export dropdown ──────────────────────────────────────
            ui.menu_button("📤 Export", |ui| {
                if ui.button("📄 Export to CSV...").clicked() {
                    self.export_csv();
                    ui.close_menu();
                }
                if ui.button("📋 Export to JSON...").clicked() {
                    self.export_json();
                    ui.close_menu();
                }
            });

            ui.separator();

            // ── Import .evtx ────────────────────────────────────────
            if ui
                .button("📂 Open .evtx")
                .on_hover_text("Import events from a local .evtx file")
                .clicked()
            {
                self.import_evtx();
            }

            ui.separator();

            // ── Live tail toggle ────────────────────────────────────
            let tail_label = if self.live_tail { "⏸ Pause Tail" } else { "▶ Live Tail" };
            let tail_btn = ui
                .selectable_label(self.live_tail, tail_label)
                .on_hover_text("Auto-refresh every 5 seconds to show new events");
            if tail_btn.clicked() {
                self.live_tail = !self.live_tail;
                if self.live_tail {
                    self.last_tail_time = None; // trigger an immediate query
                }
            }

            // ── Right-aligned app title + about + theme toggle ─────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let about_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("ℹ").size(14.0)
                    )
                    .min_size(egui::vec2(22.0, 22.0)),
                );
                if about_btn
                    .on_hover_text("About EventSleuth")
                    .clicked()
                {
                    self.show_about = true;
                }

                // Theme toggle
                let theme_icon = if self.dark_mode { "☀" } else { "🌙" };
                let theme_tooltip = if self.dark_mode { "Switch to light mode" } else { "Switch to dark mode" };
                let theme_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(theme_icon).size(14.0)
                    )
                    .min_size(egui::vec2(22.0, 22.0)),
                );
                if theme_btn
                    .on_hover_text(theme_tooltip)
                    .clicked()
                {
                    self.dark_mode = !self.dark_mode;
                    if self.dark_mode {
                        theme::apply_dark_theme(ui.ctx());
                    } else {
                        theme::apply_light_theme(ui.ctx());
                    }
                }

                ui.label(
                    egui::RichText::new("🔍 EventSleuth")
                        .color(theme::ACCENT)
                        .strong()
                        .size(16.0),
                );
            });
        });
    }

    /// Render the source selector popup window (if visible).
    ///
    /// Shows a searchable list of all discovered sources with checkboxes.
    pub fn render_channel_selector(&mut self, ctx: &egui::Context) {
        if !self.show_channel_selector {
            return;
        }

        let mut open = true;
        egui::Window::new("📋 Select Sources")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(350.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // Search box
                ui.horizontal(|ui| {
                    ui.label("🔎 Search:");
                    let ch_search = ui.text_edit_singleline(&mut self.channel_search);
                    ch_search.on_hover_text("Type to filter the source list below.\nExample: \"Security\" or \"Microsoft\"");
                });

                ui.separator();

                // Quick select / deselect
                ui.horizontal(|ui| {
                    if ui.small_button("⭐ Common").clicked() {
                        self.selected_channels =
                            crate::core::channel_enumerator::common_channels(&self.channels);
                    }
                    if ui.small_button("☑️ All").clicked() {
                        self.selected_channels = self.channels.clone();
                    }
                    if ui.small_button("✖ None").clicked() {
                        self.selected_channels.clear();
                    }
                });

                ui.separator();

                // Source list with checkboxes
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
