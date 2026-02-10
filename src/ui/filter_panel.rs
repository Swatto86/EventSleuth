//! Left-side filter panel: Event ID, level, provider, text search,
//! time range, case sensitivity toggle, apply/clear, and time presets.

use crate::app::EventSleuthApp;
use crate::ui::theme;

impl EventSleuthApp {
    /// Render the filter panel within the given `Ui` region.
    ///
    /// All filter inputs modify `self.filter`. Changes are applied either
    /// on each keystroke (debounced) or when the user presses **Apply**.
    pub fn render_filter_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("🔍 Filters").color(theme::ACCENT));
        ui.separator();

        let mut changed = false;

        // ── Event ID ────────────────────────────────────────────────
        ui.label("🆔 Event ID");
        let eid_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.event_id_input)
                .hint_text("e.g. 1001, 4000-4999, !7036")
                .desired_width(f32::INFINITY),
        );
        eid_response.on_hover_text("Filter by Event ID.\nExamples:\n  1001 — single ID\n  4000-4999 — range\n  1001, 4624 — multiple IDs\n  !7036 — exclude an ID");
        ui.add_space(4.0);

        // ── Severity levels ─────────────────────────────────────────
        ui.label("📊 Level");
        let level_names = ["LogAlways", "🔴 Critical", "🟠 Error", "🟡 Warning", "🔵 Info", "⚪ Verbose"];
        let level_colors = [
            theme::LEVEL_DEFAULT,
            theme::LEVEL_CRITICAL,
            theme::LEVEL_ERROR,
            theme::LEVEL_WARNING,
            theme::LEVEL_INFO,
            theme::LEVEL_VERBOSE,
        ];
        for i in 1..=5 {
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
        prov_response.on_hover_text("Filter events by provider name.\nMatches any provider containing the text you type.\nExample: \"Microsoft\" matches \"Microsoft-Windows-Security-Auditing\"");
        ui.add_space(4.0);

        // ── Text search ─────────────────────────────────────────────
        ui.label("🔎 Search");
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.text_search)
                .hint_text("Search all fields")
                .desired_width(f32::INFINITY),
        );
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
        tfrom_response.on_hover_text("Show events from this time onward.\nFormat: YYYY-MM-DD HH:MM:SS\nExample: 2026-02-10 09:00:00\nOr use the Quick Presets below.");
        ui.label("🕐 Time To");
        let tto_response = ui.add(
            egui::TextEdit::singleline(&mut self.filter.time_to_input)
                .hint_text("YYYY-MM-DD HH:MM:SS")
                .desired_width(f32::INFINITY),
        );
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

        if changed {
            self.filter.parse_event_ids();
            self.filter.parse_time_range();
            self.needs_refilter = true;
        }
    }
}
