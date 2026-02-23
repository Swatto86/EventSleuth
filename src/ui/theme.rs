//! Colour palette and style helpers for EventSleuth's dark theme.
//!
//! Defines the custom colour scheme used throughout the application.
//! Severity levels are colour-coded per the specification.

use egui::Color32;

// ── Background colours ──────────────────────────────────────────────────

/// Main window background.
pub const BG_DARK: Color32 = Color32::from_rgb(30, 30, 46);

/// Panel / sidebar background.
pub const BG_PANEL: Color32 = Color32::from_rgb(36, 36, 54);

/// Even rows in the event table.
pub const BG_TABLE_ROW_EVEN: Color32 = Color32::from_rgb(32, 32, 48);

/// Odd rows in the event table.
#[allow(dead_code)]
pub const BG_TABLE_ROW_ODD: Color32 = Color32::from_rgb(38, 38, 56);

/// Currently selected / highlighted row.
pub const BG_SELECTED: Color32 = Color32::from_rgb(55, 55, 95);

// ── Text colours ────────────────────────────────────────────────────────

/// Primary text colour.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(205, 205, 215);

/// Secondary / muted text.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 160);

/// Dim text (hints, placeholders).
pub const TEXT_DIM: Color32 = Color32::from_rgb(100, 100, 120);

// ── Severity level colours ──────────────────────────────────────────────

/// Critical (level 1) — bright red.
pub const LEVEL_CRITICAL: Color32 = Color32::from_rgb(255, 68, 68);

/// Error (level 2) — red-orange.
pub const LEVEL_ERROR: Color32 = Color32::from_rgb(224, 108, 96);

/// Warning (level 3) — amber.
pub const LEVEL_WARNING: Color32 = Color32::from_rgb(224, 168, 64);

/// Informational (level 4) — blue-grey.
pub const LEVEL_INFO: Color32 = Color32::from_rgb(122, 162, 212);

/// Verbose (level 5) — dim grey.
pub const LEVEL_VERBOSE: Color32 = Color32::from_rgb(136, 136, 136);

/// Fallback for unknown levels.
pub const LEVEL_DEFAULT: Color32 = Color32::from_rgb(170, 170, 170);

// ── Accent colours ──────────────────────────────────────────────────────

/// Primary accent (teal).
pub const ACCENT: Color32 = Color32::from_rgb(80, 200, 220);

/// Dimmer accent for secondary highlights.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(60, 150, 170);

/// Background colour for search-match highlighting.
pub const HIGHLIGHT_BG: Color32 = Color32::from_rgba_premultiplied(200, 170, 0, 70);

/// Text colour for search-match highlighted segments.
pub const HIGHLIGHT_TEXT: Color32 = Color32::from_rgb(255, 220, 80);

// ── Helpers ─────────────────────────────────────────────────────────────

/// Return the colour associated with a numeric severity level.
pub fn level_color(level: u8) -> Color32 {
    match level {
        1 => LEVEL_CRITICAL,
        2 => LEVEL_ERROR,
        3 => LEVEL_WARNING,
        4 => LEVEL_INFO,
        5 => LEVEL_VERBOSE,
        _ => LEVEL_DEFAULT,
    }
}

/// Apply the EventSleuth dark theme to the given egui context.
///
/// Should be called once during initialisation (in `App::new`).
pub fn apply_theme(ctx: &egui::Context) {
    apply_dark_theme(ctx);
}

/// Apply the EventSleuth dark theme.
pub fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Background tones
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_DARK;
    visuals.faint_bg_color = BG_TABLE_ROW_EVEN;

    // Override all text to our primary colour
    visuals.override_text_color = Some(TEXT_PRIMARY);

    // Widget resting state
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 65);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(40, 40, 60);

    // Widget hover state
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 80);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);

    // Widget active state
    visuals.widgets.active.bg_fill = Color32::from_rgb(65, 65, 95);

    // Non-interactive backgrounds
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);

    // Selection
    visuals.selection.bg_fill = BG_SELECTED;
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);

    // Window appearance
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 70));

    ctx.set_visuals(visuals);
}

/// Apply the EventSleuth light theme.
pub fn apply_light_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();

    // Background tones — light palette
    visuals.panel_fill = Color32::from_rgb(245, 245, 248);
    visuals.window_fill = Color32::from_rgb(250, 250, 252);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_rgb(238, 238, 242);

    // Text
    visuals.override_text_color = Some(Color32::from_rgb(40, 40, 50));

    // Widget resting state
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(225, 225, 232);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 100));
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(230, 230, 236);

    // Widget hover state
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(210, 210, 220);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(40, 40, 50));

    // Widget active state
    visuals.widgets.active.bg_fill = Color32::from_rgb(195, 195, 210);

    // Non-interactive backgrounds
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(240, 240, 244);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 120));

    // Selection
    visuals.selection.bg_fill = Color32::from_rgb(180, 215, 235);
    visuals.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(40, 160, 180));

    // Window appearance
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 210));

    ctx.set_visuals(visuals);
}
