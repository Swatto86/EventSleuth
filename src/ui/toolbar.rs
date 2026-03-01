//! Top toolbar: channel selector, refresh, and export buttons.

use crate::app::EventSleuthApp;
use crate::ui::theme;

impl EventSleuthApp {
    /// Render the top toolbar within the given `Ui` region.
    ///
    /// Contains the channel selector button, refresh / cancel controls,
    /// export dropdown, .evtx import, live-tail toggle, and utility
    /// buttons (theme, about).  Keyboard-shortcut hints are shown in
    /// every tooltip so users can discover them organically.
    pub fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = theme::TOOLBAR_GROUP_SPACING;

            // ── Source selector button ─────────────────────────
            let channel_label = if self.selected_channels.is_empty() {
                "Select Sources...".to_string()
            } else if self.selected_channels.len() == 1 {
                self.selected_channels[0].clone()
            } else {
                format!("{} sources", self.selected_channels.len())
            };

            if ui
                .button(format!("\u{1F4CB} {channel_label}"))
                .on_hover_text("Choose which log sources to query")
                .clicked()
            {
                self.show_channel_selector = !self.show_channel_selector;
            }

            ui.separator();

            // ── Refresh / Cancel ────────────────────────────────────
            if self.is_loading {
                ui.spinner();
                if ui
                    .button("\u{23F9} Stop")
                    .on_hover_text("Cancel the current query (Esc)")
                    .clicked()
                {
                    self.cancel_loading();
                }
            } else {
                let refresh = ui
                    .button("\u{1F504} Refresh")
                    .on_hover_text("Re-query selected sources (F5 or Ctrl+R)");
                if refresh.clicked() {
                    self.start_loading();
                }
            }

            ui.separator();

            // ── Export dropdown (disabled when nothing to export) ────
            let has_events = !self.filtered_indices.is_empty();
            ui.add_enabled_ui(has_events, |ui| {
                ui.menu_button("\u{1F4E4} Export", |ui| {
                    if ui.button("\u{1F4C4} Export to CSV...").clicked() {
                        self.export_csv();
                        ui.close_menu();
                    }
                    if ui.button("\u{1F4CB} Export to JSON...").clicked() {
                        self.export_json();
                        ui.close_menu();
                    }
                })
                .response
                .on_hover_text(if has_events {
                    "Export filtered events to a file"
                } else {
                    "Load events first to enable export"
                })
                .on_disabled_hover_text("No events to export");
            });

            ui.separator();

            // ── Import .evtx ────────────────────────────────────────
            if ui
                .button("\u{1F4C2} Open .evtx")
                .on_hover_text("Import events from a local .evtx file")
                .clicked()
            {
                self.import_evtx();
            }

            ui.separator();

            // ── Live tail toggle ────────────────────────────────────
            let tail_text = if self.live_tail {
                "\u{23F8} Pause Tail"
            } else {
                "\u{25B6} Live Tail"
            };
            let tail_btn =
                egui::Button::new(egui::RichText::new(tail_text).color(if self.live_tail {
                    theme::accent(self.dark_mode)
                } else {
                    theme::text_primary(self.dark_mode)
                }));
            if ui
                .add(tail_btn)
                .on_hover_text(if self.live_tail {
                    "Stop auto-refreshing for new events"
                } else {
                    "Auto-refresh every 5 s to show new events"
                })
                .clicked()
            {
                self.live_tail = !self.live_tail;
                if self.live_tail {
                    self.last_tail_time = None; // trigger an immediate query
                }
            }

            // ── Active-filter count badge ───────────────────────────
            let active_count = self.filter.active_count();
            if active_count > 0 {
                ui.add_space(2.0);
                theme::badge(
                    ui,
                    active_count,
                    theme::accent(self.dark_mode),
                    egui::Color32::WHITE,
                );
                ui.label(
                    egui::RichText::new("filters")
                        .color(theme::text_dim(self.dark_mode))
                        .small(),
                );
            }

            ui.separator();

            // ── Statistics button ───────────────────────────────────
            let stats_text = "\u{1F4CA} Stats";
            let stats_btn =
                egui::Button::new(egui::RichText::new(stats_text).color(if self.show_stats {
                    theme::accent(self.dark_mode)
                } else {
                    theme::text_primary(self.dark_mode)
                }));
            let has_events_for_stats = !self.all_events.is_empty();
            if ui
                .add_enabled(has_events_for_stats, stats_btn)
                .on_hover_text("Show event statistics summary")
                .on_disabled_hover_text("Load events first")
                .clicked()
            {
                self.show_stats = !self.show_stats;
                if self.show_stats {
                    self.stats_dirty = true;
                }
            }

            // ── Column visibility dropdown ──────────────────────────
            ui.menu_button("\u{1F4CB} Columns", |ui| {
                ui.label(
                    egui::RichText::new("Visible columns")
                        .color(theme::text_secondary(self.dark_mode))
                        .strong(),
                );
                ui.separator();
                ui.checkbox(&mut self.column_visibility.timestamp, "Timestamp");
                ui.checkbox(&mut self.column_visibility.level, "Level");
                ui.checkbox(&mut self.column_visibility.event_id, "Event ID");
                ui.checkbox(&mut self.column_visibility.provider, "Provider");
                ui.checkbox(&mut self.column_visibility.channel, "Channel");
                ui.checkbox(&mut self.column_visibility.computer, "Computer");
                ui.checkbox(&mut self.column_visibility.message, "Message");
                ui.separator();
                if ui.small_button("Reset to defaults").clicked() {
                    self.column_visibility = crate::app::ColumnVisibility::default();
                    ui.close_menu();
                }
            })
            .response
            .on_hover_text("Show or hide table columns");

            // ── Right-aligned app title + about + theme toggle + shortcuts ──
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let about_btn = ui.add(
                    egui::Button::new(egui::RichText::new("\u{2139}").size(14.0))
                        .min_size(egui::vec2(theme::ICON_BTN_SIZE, theme::ICON_BTN_SIZE)),
                );
                if about_btn.on_hover_text("About EventSleuth").clicked() {
                    self.show_about = true;
                }

                // Theme toggle
                let theme_icon = if self.dark_mode {
                    "\u{2600}"
                } else {
                    "\u{1F319}"
                };
                let theme_tooltip = if self.dark_mode {
                    "Switch to light mode"
                } else {
                    "Switch to dark mode"
                };
                let theme_btn = ui.add(
                    egui::Button::new(egui::RichText::new(theme_icon).size(14.0))
                        .min_size(egui::vec2(theme::ICON_BTN_SIZE, theme::ICON_BTN_SIZE)),
                );
                if theme_btn.on_hover_text(theme_tooltip).clicked() {
                    self.dark_mode = !self.dark_mode;
                    if self.dark_mode {
                        theme::apply_dark_theme(ui.ctx());
                    } else {
                        theme::apply_light_theme(ui.ctx());
                    }
                }

                // Keyboard shortcuts reference tooltip
                let kb_btn = ui.add(
                    egui::Button::new(egui::RichText::new("\u{2328}").size(14.0))
                        .min_size(egui::vec2(theme::ICON_BTN_SIZE, theme::ICON_BTN_SIZE)),
                );
                kb_btn.on_hover_ui(|ui| {
                    ui.label(
                        egui::RichText::new("Keyboard Shortcuts")
                            .color(theme::accent(self.dark_mode))
                            .strong(),
                    );
                    ui.separator();
                    let shortcuts = [
                        ("F5 / Ctrl+R", "Refresh sources"),
                        ("Escape", "Close dialog / deselect"),
                        ("\u{2191} / \u{2193}", "Navigate events"),
                        ("Page Up / Down", "Jump 20 events"),
                        ("Home / End", "First / last event"),
                        ("Ctrl+Shift+X", "Clear all filters"),
                    ];
                    egui::Grid::new("shortcuts_grid")
                        .num_columns(2)
                        .spacing([12.0, 2.0])
                        .show(ui, |ui| {
                            for (key, desc) in &shortcuts {
                                ui.label(
                                    egui::RichText::new(*key)
                                        .color(theme::text_primary(self.dark_mode))
                                        .strong()
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(*desc)
                                        .color(theme::text_secondary(self.dark_mode))
                                        .small(),
                                );
                                ui.end_row();
                            }
                        });
                });

                ui.label(
                    egui::RichText::new("\u{1F50D} EventSleuth")
                        .color(theme::accent(self.dark_mode))
                        .strong()
                        .size(16.0),
                );
            });
        });
    }

    /// Render the source selector popup window (if visible).
    ///
    /// Shows a searchable list of all discovered sources with checkboxes.
    /// The channel list is iterated by index to avoid cloning the entire
    /// `Vec<String>` on every frame.
    pub fn render_channel_selector(&mut self, ctx: &egui::Context) {
        if !self.show_channel_selector {
            return;
        }

        let mut open = true;
        egui::Window::new("\u{1F4CB} Select Sources")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(350.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // Search box
                ui.horizontal(|ui| {
                    ui.label("\u{1F50E}");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.channel_search)
                            .hint_text("Filter sources...")
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.separator();

                // Quick select / deselect
                ui.horizontal(|ui| {
                    if ui
                        .small_button("\u{2B50} Common")
                        .on_hover_text("Select the most commonly useful channels")
                        .clicked()
                    {
                        self.selected_channels =
                            crate::core::channel_enumerator::common_channels(&self.channels);
                    }
                    if ui
                        .small_button("\u{2611}\u{FE0F} All")
                        .on_hover_text("Select every available channel")
                        .clicked()
                    {
                        self.selected_channels = self.channels.clone();
                    }
                    if ui
                        .small_button("\u{2716} None")
                        .on_hover_text("Deselect all channels")
                        .clicked()
                    {
                        self.selected_channels.clear();
                    }

                    // Show selection summary
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}/{}",
                                self.selected_channels.len(),
                                self.channels.len()
                            ))
                            .color(theme::text_dim(self.dark_mode))
                            .small(),
                        );
                    });
                });

                ui.separator();

                // Source list with checkboxes (iterate by index to avoid clone)
                let search_lower = self.channel_search.to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        let mut toggle: Option<(usize, bool)> = None;
                        for idx in 0..self.channels.len() {
                            let channel = &self.channels[idx];
                            if !search_lower.is_empty()
                                && !channel.to_lowercase().contains(&search_lower)
                            {
                                continue;
                            }

                            let mut selected = self.selected_channels.contains(channel);
                            if ui.checkbox(&mut selected, channel.as_str()).changed() {
                                toggle = Some((idx, selected));
                            }
                        }
                        // Apply toggle outside borrow
                        if let Some((idx, selected)) = toggle {
                            let channel = self.channels[idx].clone();
                            if selected {
                                self.selected_channels.push(channel);
                            } else {
                                self.selected_channels.retain(|c| c != &self.channels[idx]);
                            }
                        }
                    });
            });

        if !open {
            self.show_channel_selector = false;
            self.channel_search.clear();
        }
    }
}
