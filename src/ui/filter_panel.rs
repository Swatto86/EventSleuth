//! Left-side filter panel: Event ID, level, provider, text search,
//! time range, case sensitivity toggle, apply/clear, time presets,
//! and saved filter preset management.

use crate::app::EventSleuthApp;
use crate::ui::theme;

impl EventSleuthApp {
    /// Render the filter panel within the given `Ui` region.
    ///
    /// All filter inputs modify `self.filter`. Text-field changes are
    /// **debounced** (150 ms) so the filter is not recomputed on every
    /// keystroke. Checkbox / button changes are applied immediately.
    pub fn render_filter_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("🔍 Filters").color(theme::ACCENT));

        // ── Preset controls (inline) ────────────────────────────────
        ui.horizontal(|ui| {
            ui.menu_button("📂 Presets", |ui| {
                if self.filter_presets.is_empty() {
                    ui.label(
                        egui::RichText::new("No saved presets")
                            .color(theme::TEXT_DIM)
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
                            if ui.small_button("🗑").on_hover_text("Delete this preset").clicked() {
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
                if ui.button("💾 Save current...").clicked() {
                    self.show_save_preset = true;
                    ui.close_menu();
                }
            });
        });

        ui.separator();

        // Track whether any immediate (non-debounced) change occurs
        let mut changed = false;
        // Track whether any text field was edited (debounced)
        let mut text_changed = false;

        // ── Event ID ────────────────────────────────────────────────
        ui.label("🆔 Event ID");
        let eid_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.event_id_input)
                .hint_text("e.g. 1001, 4000-4999, !7036")
                .desired_width(f32::INFINITY),
        );
        let eid_changed = eid_response.changed();
        eid_response.on_hover_text("Filter by Event ID.\nExamples:\n  1001 - single ID\n  4000-4999 - range\n  1001, 4624 - multiple IDs\n  !7036 - exclude an ID");
        if eid_changed {
            text_changed = true;
        }
        ui.add_space(4.0);

        // ── Severity levels ─────────────────────────────────────────
        ui.label("📊 Level");
        let level_names = ["⚙️ LogAlways", "🔴 Critical", "🟠 Error", "🟡 Warning", "🔵 Info", "⚪ Verbose"];
        let level_colors = [
            theme::LEVEL_DEFAULT,
            theme::LEVEL_CRITICAL,
            theme::LEVEL_ERROR,
            theme::LEVEL_WARNING,
            theme::LEVEL_INFO,
            theme::LEVEL_VERBOSE,
        ];
        for i in 0..=5 {
            let label = egui::RichText::new(level_names[i]).color(level_colors[i]);
            if ui.checkbox(&mut self.filter.levels[i], label).changed() {
                changed = true;
            }
        }
        ui.add_space(4.0);

        // ── Provider ────────────────────────────────────────────────
        ui.label("🏷️ Provider");
        let prov_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.provider_filter)
                .hint_text("Substring match")
                .desired_width(f32::INFINITY),
        );
        if prov_response.changed() {
            text_changed = true;
        }
        prov_response.on_hover_text("Filter events by provider name.\nMatches any provider containing the text you type.\nExample: \"Microsoft\" matches \"Microsoft-Windows-Security-Auditing\"");
        ui.add_space(4.0);

        // ── Text search ─────────────────────────────────────────────
        ui.label("🔎 Search");
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.text_search)
                .hint_text("Search all fields")
                .desired_width(f32::INFINITY),
        );
        if search_response.changed() {
            text_changed = true;
        }
        search_response.on_hover_text("Full-text search across all event fields:\nMessage, Provider, Event ID, Event Data, etc.\nUse the checkbox below for case-sensitive matching.");
        ui.checkbox(&mut self.filter.case_sensitive, "Case sensitive");
        ui.add_space(4.0);

        // ── Time range ──────────────────────────────────────────────
        ui.label("🕐 Time From");
        let tfrom_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.time_from_input)
                .hint_text("YYYY-MM-DD HH:MM:SS")
                .desired_width(f32::INFINITY),
        );
        if tfrom_response.changed() {
            text_changed = true;
        }
        tfrom_response.on_hover_text("Show events from this time onward.\nFormat: YYYY-MM-DD HH:MM:SS\nExample: 2026-02-10 09:00:00\nOr use the Quick Presets below.");
        ui.label("🕐 Time To");
        let tto_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.time_to_input)
                .hint_text("YYYY-MM-DD HH:MM:SS")
                .desired_width(f32::INFINITY),
        );
        if tto_response.changed() {
            text_changed = true;
        }
        tto_response.on_hover_text("Show events up to this time.\nFormat: YYYY-MM-DD HH:MM:SS\nExample: 2026-02-10 17:00:00\nLeave empty for no upper bound.");

        ui.add_space(4.0);

        // ── Time presets ────────────────────────────────────────────
        ui.label(egui::RichText::new("⚡ Quick presets").color(theme::TEXT_SECONDARY));
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("1h").clicked() {
                self.filter.apply_time_preset(1);
                changed = true;
            }
            if ui.small_button("24h").clicked() {
                self.filter.apply_time_preset(24);
                changed = true;
            }
            if ui.small_button("7d").clicked() {
                self.filter.apply_time_preset(24 * 7);
                changed = true;
            }
            if ui.small_button("30d").clicked() {
                self.filter.apply_time_preset(24 * 30);
                changed = true;
            }
            if ui.small_button("Today").clicked() {
                self.filter.apply_today_preset();
                changed = true;
            }
            if ui.small_button("All").clicked() {
                self.filter.time_from = None;
                self.filter.time_to = None;
                self.filter.time_from_input.clear();
                self.filter.time_to_input.clear();
                changed = true;
            }
        });

        ui.add_space(8.0);
        ui.separator();

        // ── Apply / Clear buttons ───────────────────────────────────
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("✅ Apply").color(theme::ACCENT))
                .clicked()
            {
                changed = true;
            }
            if ui.button("🗑️ Clear").clicked() {
                self.filter.clear();
                changed = true;
            }
        });

        // ── Show active filter count ────────────────────────────────
        if !self.filter.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("🟢 Filters active")
                    .color(theme::ACCENT)
                    .small(),
            );
        }

        // ── Apply changes ───────────────────────────────────────────
        // Immediate changes (checkboxes, buttons): parse + refilter now.
        if changed {
            self.filter.parse_event_ids();
            self.filter.parse_time_range();
            self.needs_refilter = true;
        }

        // Debounced changes (text fields): reset the debounce timer.
        // The actual parse + refilter happens in the update() loop after
        // FILTER_DEBOUNCE_MS of inactivity.
        if text_changed {
            self.debounce_timer = Some(std::time::Instant::now());
        }
    }
}
