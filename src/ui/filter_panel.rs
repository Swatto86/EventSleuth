//! Left-side filter panel: Event ID, level, provider, text search,
//! time range, case sensitivity toggle, apply/clear, time presets,
//! and saved filter preset management.
//!
//! Sections use `CollapsingHeader` so users can collapse areas they are
//! not actively using, reducing visual noise.  An active-filter banner
//! at the top summarises which filters are narrowing the result set.

use crate::app::EventSleuthApp;
use crate::ui::theme;

impl EventSleuthApp {
    /// Render the filter panel within the given `Ui` region.
    ///
    /// All filter inputs modify `self.filter`. Text-field changes are
    /// **debounced** (150 ms) so the filter is not recomputed on every
    /// keystroke. Checkbox / button changes are applied immediately.
    pub fn render_filter_panel(&mut self, ui: &mut egui::Ui) {
        let dark = self.dark_mode;

        // ── Active-filter summary banner ────────────────────────────
        // Shown only when at least one filter is active so users always
        // know the table is narrowed, plus a one-click clear.
        if !self.filter.is_empty() {
            egui::Frame::new()
                .fill(theme::filter_active_bg(dark))
                .inner_margin(egui::Margin::same(6))
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::badge(
                            ui,
                            self.filter.active_count(),
                            theme::accent(dark),
                            egui::Color32::WHITE,
                        );
                        ui.label(
                            egui::RichText::new("active filters")
                                .color(theme::accent(dark))
                                .small()
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("\u{2716} Clear all")
                                .on_hover_text("Reset every filter to its default")
                                .clicked()
                            {
                                self.filter.clear();
                                self.filter.parse_event_ids();
                                self.filter.parse_time_range();
                                self.needs_refilter = true;
                            }
                        });
                    });
                });
            ui.add_space(theme::SECTION_SPACING);
        }

        ui.heading(egui::RichText::new("\u{1F50D} Filters").color(theme::accent(dark)));

        // ── Preset controls (inline) ────────────────────────────────
        ui.horizontal(|ui| {
            ui.menu_button("\u{1F4C2} Presets", |ui| {
                if self.filter_presets.is_empty() {
                    ui.label(
                        egui::RichText::new("No saved presets")
                            .color(theme::text_dim(dark))
                            .italics(),
                    );
                } else {
                    let mut load_idx: Option<usize> = None;
                    let mut delete_idx: Option<usize> = None;
                    for (i, preset) in self.filter_presets.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(&preset.name).clicked() {
                                load_idx = Some(i);
                            }
                            if ui
                                .small_button("\u{1F5D1}")
                                .on_hover_text("Delete this preset")
                                .clicked()
                            {
                                delete_idx = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = load_idx {
                        self.filter = self.filter_presets[idx].to_filter_state();
                        self.needs_refilter = true;
                        ui.close_menu();
                    }
                    if let Some(idx) = delete_idx {
                        self.filter_presets.remove(idx);
                    }
                }
                ui.separator();
                if ui.button("\u{1F4BE} Save current...").clicked() {
                    self.show_save_preset = true;
                    ui.close_menu();
                }
            });
        });

        ui.add_space(theme::ITEM_SPACING);

        // Track whether any immediate (non-debounced) change occurs
        let mut changed = false;
        // Track whether any text field was edited (debounced)
        let mut text_changed = false;

        // ── Event ID ────────────────────────────────────────────────
        egui::CollapsingHeader::new(
            egui::RichText::new("\u{1F194} Event ID").strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            let eid_response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.event_id_input)
                    .hint_text("e.g. 1001, 4000-4999, !7036")
                    .desired_width(f32::INFINITY),
            );
            let eid_changed = eid_response.changed();
            eid_response.on_hover_text(
                "Filter by Event ID.\nExamples:\n  1001 - single ID\n  4000-4999 - range\n  1001, 4624 - multiple IDs\n  !7036 - exclude an ID",
            );
            if eid_changed {
                text_changed = true;
            }
        });

        ui.add_space(theme::ITEM_SPACING);

        // ── Severity levels ─────────────────────────────────────────
        egui::CollapsingHeader::new(egui::RichText::new("\u{1F4CA} Level").strong())
            .default_open(true)
            .show(ui, |ui| {
                let level_names = [
                    "\u{2699}\u{FE0F} LogAlways",
                    "\u{1F534} Critical",
                    "\u{1F7E0} Error",
                    "\u{1F7E1} Warning",
                    "\u{1F535} Info",
                    "\u{26AA} Verbose",
                ];
                let level_colors = theme::level_colors(dark);
                let all_on = self.filter.levels.iter().all(|&v| v);
                let none_on = self.filter.levels.iter().all(|&v| !v);
                // Quick toggles
                ui.horizontal(|ui| {
                    if ui
                        .small_button(if all_on { "Deselect all" } else { "Select all" })
                        .clicked()
                    {
                        let new_val = !all_on;
                        for l in &mut self.filter.levels {
                            *l = new_val;
                        }
                        changed = true;
                    }
                    if !none_on && !all_on {
                        // Show which levels are active as a hint
                        let active: Vec<&str> = self
                            .filter
                            .levels
                            .iter()
                            .enumerate()
                            .filter(|(_, v)| !**v)
                            .map(|(i, _)| match i {
                                0 => "LogAlways",
                                1 => "Critical",
                                2 => "Error",
                                3 => "Warning",
                                4 => "Info",
                                5 => "Verbose",
                                _ => "",
                            })
                            .collect();
                        if !active.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("{} hidden", active.len()))
                                    .color(theme::text_dim(dark))
                                    .small(),
                            );
                        }
                    }
                });
                for i in 0..=5 {
                    let label = egui::RichText::new(level_names[i]).color(level_colors[i]);
                    if ui.checkbox(&mut self.filter.levels[i], label).changed() {
                        changed = true;
                    }
                }
            });

        ui.add_space(theme::ITEM_SPACING);

        // ── Provider ────────────────────────────────────────────────
        egui::CollapsingHeader::new(
            egui::RichText::new("\u{1F3F7}\u{FE0F} Provider").strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            let prov_response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.provider_filter)
                    .hint_text("Substring match")
                    .desired_width(f32::INFINITY),
            );
            if prov_response.changed() {
                text_changed = true;
            }
            prov_response.on_hover_text(
                "Filter events by provider name.\nMatches any provider containing the text you type.\nExample: \"Microsoft\" matches \"Microsoft-Windows-Security-Auditing\"",
            );
        });

        ui.add_space(theme::ITEM_SPACING);

        // ── Text search ─────────────────────────────────────────────
        egui::CollapsingHeader::new(
            egui::RichText::new("\u{1F50E} Search").strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.text_search)
                    .hint_text("Search all fields")
                    .desired_width(f32::INFINITY),
            );
            if search_response.changed() {
                text_changed = true;
            }
            search_response.on_hover_text(
                "Full-text search across all event fields:\nMessage, Provider, Event ID, Event Data, etc.",
            );
            if ui
                .checkbox(&mut self.filter.case_sensitive, "Case sensitive")
                .changed()
            {
                changed = true;
            }
        });

        ui.add_space(theme::ITEM_SPACING);

        // ── Time range ──────────────────────────────────────────────
        egui::CollapsingHeader::new(
            egui::RichText::new("\u{1F550} Time Range").strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("From").color(theme::text_dim(dark)));
            let tfrom_response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.time_from_input)
                    .hint_text("YYYY-MM-DD HH:MM:SS")
                    .desired_width(f32::INFINITY),
            );
            if tfrom_response.changed() {
                text_changed = true;
            }
            tfrom_response.on_hover_text(
                "Show events from this time onward.\nFormat: YYYY-MM-DD HH:MM:SS\nExample: 2026-02-10 09:00:00",
            );

            ui.add_space(2.0);

            ui.label(egui::RichText::new("To").color(theme::text_dim(dark)));
            let tto_response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.time_to_input)
                    .hint_text("YYYY-MM-DD HH:MM:SS")
                    .desired_width(f32::INFINITY),
            );
            if tto_response.changed() {
                text_changed = true;
            }
            tto_response.on_hover_text(
                "Show events up to this time.\nLeave empty for no upper bound.",
            );

            ui.add_space(theme::ITEM_SPACING);

            // ── Time presets ────────────────────────────────────────
            ui.label(
                egui::RichText::new("Quick presets")
                    .color(theme::text_secondary(dark))
                    .small(),
            );
            egui::Grid::new("time_presets_grid")
                .num_columns(3)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    if ui.small_button("1 h").clicked() {
                        self.filter.apply_time_preset(1);
                        changed = true;
                    }
                    if ui.small_button("24 h").clicked() {
                        self.filter.apply_time_preset(24);
                        changed = true;
                    }
                    if ui.small_button("7 d").clicked() {
                        self.filter.apply_time_preset(24 * 7);
                        changed = true;
                    }
                    ui.end_row();
                    if ui.small_button("30 d").clicked() {
                        self.filter.apply_time_preset(24 * 30);
                        changed = true;
                    }
                    if ui.small_button("Today").clicked() {
                        self.filter.apply_today_preset();
                        changed = true;
                    }
                    if ui.small_button("All time").clicked() {
                        self.filter.time_from = None;
                        self.filter.time_to = None;
                        self.filter.time_from_input.clear();
                        self.filter.time_to_input.clear();
                        changed = true;
                    }
                    ui.end_row();
                });
        });

        ui.add_space(theme::SECTION_SPACING);
        ui.separator();

        // ── Apply / Clear buttons ───────────────────────────────────
        ui.horizontal(|ui| {
            if ui
                .button(
                    egui::RichText::new("\u{1F504} Re-query")
                        .color(theme::accent(dark)),
                )
                .on_hover_text(
                    "Re-read events from the selected sources with the\ncurrent time-range filter applied at the query level (F5)",
                )
                .clicked()
            {
                self.filter.parse_event_ids();
                self.filter.parse_time_range();
                self.start_loading();
            }
            if ui
                .button("\u{1F5D1}\u{FE0F} Clear")
                .on_hover_text("Reset all filters to their defaults")
                .clicked()
            {
                self.filter.clear();
                changed = true;
            }
        });

        // ── Apply changes ───────────────────────────────────────────
        // Immediate changes (checkboxes, buttons): parse + refilter now.
        if changed {
            self.filter.parse_event_ids();
            self.filter.parse_time_range();
            self.needs_refilter = true;
        }

        // Debounced changes (text fields): reset the debounce timer.
        if text_changed {
            self.debounce_timer = Some(std::time::Instant::now());
        }
    }
}
