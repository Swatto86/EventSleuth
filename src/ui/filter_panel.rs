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
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.event_id_input)
                .hint_text("e.g. 1001, 4000-4999, !7036")
                .desired_width(f32::INFINITY),
        );
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
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.provider_filter)
                .hint_text("Substring match")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);

        // ── Text search ─────────────────────────────────────────────
        ui.label("🔎 Search");
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.text_search)
                .hint_text("Search all fields")
                .desired_width(f32::INFINITY),
        );
        ui.checkbox(&mut self.filter.case_sensitive, "Case sensitive");
        ui.add_space(4.0);

        // ── Time range ──────────────────────────────────────────────
        ui.label("🕐 Time From");
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.time_from_input)
                .hint_text("YYYY-MM-DD HH:MM:SS")
                .desired_width(f32::INFINITY),
        );
        ui.label("🕐 Time To");
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.time_to_input)
                .hint_text("YYYY-MM-DD HH:MM:SS")
                .desired_width(f32::INFINITY),
        );

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
