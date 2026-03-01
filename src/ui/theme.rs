//! Colour palette and style helpers for EventSleuth's dark and light themes.
//!
//! Defines the custom colour scheme used throughout the application.
//! Theme-aware accessor functions accept a `dark: bool` parameter so
//! every UI call site renders correctly in both modes.
//! Severity levels are colour-coded per the specification.

use egui::Color32;

// ── Background colours (dark) ───────────────────────────────────────────

/// Main window background (dark).
pub const BG_DARK: Color32 = Color32::from_rgb(30, 30, 46);

/// Panel / sidebar background (dark).
pub const BG_PANEL: Color32 = Color32::from_rgb(36, 36, 54);

/// Even rows in the event table (dark).
pub const BG_TABLE_ROW_EVEN: Color32 = Color32::from_rgb(32, 32, 48);

/// Odd rows in the event table (dark).
#[allow(dead_code)]
pub const BG_TABLE_ROW_ODD: Color32 = Color32::from_rgb(38, 38, 56);

/// Currently selected / highlighted row (dark).
pub const BG_SELECTED: Color32 = Color32::from_rgb(55, 55, 95);

// ── Background colours (light) ──────────────────────────────────────────

/// Main window background (light).
pub const BG_LIGHT: Color32 = Color32::from_rgb(245, 245, 248);

/// Panel / sidebar background (light).
pub const BG_PANEL_LIGHT: Color32 = Color32::from_rgb(240, 240, 244);

// ── Theme-aware colour accessors ────────────────────────────────────────
//
// Pass `true` for dark mode, `false` for light mode.

/// Primary text colour — high-contrast body text.
pub fn text_primary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(205, 205, 215)
    } else {
        Color32::from_rgb(40, 40, 50)
    }
}

/// Secondary / muted text — timestamps, providers, labels.
pub fn text_secondary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(140, 140, 160)
    } else {
        Color32::from_rgb(80, 80, 100)
    }
}

/// Dim text — hints, placeholders, field names.
pub fn text_dim(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(100, 100, 120)
    } else {
        Color32::from_rgb(120, 120, 138)
    }
}

/// Primary accent (teal) — headings, active sort headers, branding.
pub fn accent(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(80, 200, 220)
    } else {
        Color32::from_rgb(0, 125, 150)
    }
}

/// Dimmer accent — secondary highlights, "Ready" text.
pub fn accent_dim(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(60, 150, 170)
    } else {
        Color32::from_rgb(50, 115, 135)
    }
}

/// Background colour for search-match highlighting.
pub fn highlight_bg(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(120, 90, 0)
    } else {
        Color32::from_rgb(255, 225, 80)
    }
}

/// Text colour for search-match highlighted segments.
pub fn highlight_text(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(255, 255, 255)
    } else {
        Color32::from_rgb(30, 20, 0)
    }
}

/// Security-banner background fill.
pub fn security_banner_bg(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(60, 40, 10)
    } else {
        Color32::from_rgb(255, 245, 220)
    }
}

/// Return the colour associated with a numeric severity level.
///
/// Dark-mode colours are bright/saturated for dark backgrounds.
/// Light-mode colours are darkened for contrast on light backgrounds.
pub fn level_color(level: u8, dark: bool) -> Color32 {
    if dark {
        match level {
            1 => Color32::from_rgb(255, 68, 68),   // Critical — bright red
            2 => Color32::from_rgb(224, 108, 96),  // Error — red-orange
            3 => Color32::from_rgb(224, 168, 64),  // Warning — amber
            4 => Color32::from_rgb(122, 162, 212), // Info — blue-grey
            5 => Color32::from_rgb(136, 136, 136), // Verbose — dim grey
            _ => Color32::from_rgb(170, 170, 170), // Default
        }
    } else {
        match level {
            1 => Color32::from_rgb(185, 20, 20),   // Critical — dark red
            2 => Color32::from_rgb(175, 55, 40),   // Error — dark red-orange
            3 => Color32::from_rgb(155, 105, 0),   // Warning — dark amber
            4 => Color32::from_rgb(35, 90, 155),   // Info — dark blue
            5 => Color32::from_rgb(105, 105, 105), // Verbose — medium grey
            _ => Color32::from_rgb(115, 115, 115), // Default
        }
    }
}

/// Shorthand array of level colours for the filter-panel checkboxes.
pub fn level_colors(dark: bool) -> [Color32; 6] {
    [
        level_color(0, dark), // LogAlways / default
        level_color(1, dark), // Critical
        level_color(2, dark), // Error
        level_color(3, dark), // Warning
        level_color(4, dark), // Info
        level_color(5, dark), // Verbose
    ]
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
    visuals.override_text_color = Some(text_primary(true));

    // Widget resting state
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 65);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_secondary(true));
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(40, 40, 60);

    // Widget hover state
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 80);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_primary(true));

    // Widget active state
    visuals.widgets.active.bg_fill = Color32::from_rgb(65, 65, 95);

    // Non-interactive backgrounds
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_secondary(true));

    // Selection
    visuals.selection.bg_fill = BG_SELECTED;
    visuals.selection.stroke = egui::Stroke::new(1.0, accent(true));

    // Window appearance
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 70));

    ctx.set_visuals(visuals);
}

/// Apply the EventSleuth light theme.
pub fn apply_light_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();

    // Background tones — light palette
    visuals.panel_fill = BG_LIGHT;
    visuals.window_fill = Color32::from_rgb(250, 250, 252);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_rgb(238, 238, 242);

    // Text
    visuals.override_text_color = Some(text_primary(false));

    // Widget resting state
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(225, 225, 232);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_secondary(false));
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(230, 230, 236);

    // Widget hover state
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(210, 210, 220);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_primary(false));

    // Widget active state
    visuals.widgets.active.bg_fill = Color32::from_rgb(195, 195, 210);

    // Non-interactive backgrounds
    visuals.widgets.noninteractive.bg_fill = BG_PANEL_LIGHT;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_secondary(false));

    // Selection
    visuals.selection.bg_fill = Color32::from_rgb(180, 215, 235);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent(false));

    // Window appearance
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 210));

    ctx.set_visuals(visuals);
}
