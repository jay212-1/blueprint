//! Colours and visual style, the same as the other Showtime Suite tools:
//! dark, with the same meaning colours everywhere (green = good, blue =
//! active/selected, amber = attention, red = error).

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(12, 14, 17);
pub const PANEL: Color32 = Color32::from_rgb(17, 19, 24);
pub const CARD: Color32 = Color32::from_rgb(24, 27, 34);
pub const LINE: Color32 = Color32::from_rgb(44, 48, 58);
pub const TEXT: Color32 = Color32::from_rgb(232, 235, 239);
pub const MUTED: Color32 = Color32::from_rgb(140, 146, 160);
pub const OK: Color32 = Color32::from_rgb(34, 197, 94);
pub const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_BG: Color32 = Color32::from_rgb(23, 42, 82);
pub const WARN: Color32 = Color32::from_rgb(250, 204, 21);
pub const ERROR: Color32 = Color32::from_rgb(239, 68, 68);
/// Blueprint's own colour (launcher tile, icon).
pub const BRAND: Color32 = Color32::from_rgb(56, 152, 236);

/// Colour names of the project (`bp_core::model::COLORS`) -> swatch.
pub fn color(name: &str) -> Option<Color32> {
    Some(match name {
        "red" => Color32::from_rgb(239, 68, 68),
        "orange" => Color32::from_rgb(249, 115, 22),
        "yellow" => Color32::from_rgb(234, 179, 8),
        "green" => Color32::from_rgb(34, 197, 94),
        "blue" => Color32::from_rgb(59, 130, 246),
        "purple" => Color32::from_rgb(168, 85, 247),
        _ => return None,
    })
}

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(10, 11, 14);
    v.faint_bg_color = Color32::from_rgb(28, 31, 39);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 42, 52);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 57, 70);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(8);
    v.selection.bg_fill = Color32::from_rgb(37, 72, 140);
    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 26.0;
    for (s, size, family) in [
        (TextStyle::Body, 14.0, FontFamily::Proportional),
        (TextStyle::Button, 14.0, FontFamily::Proportional),
        (TextStyle::Small, 11.5, FontFamily::Proportional),
        (TextStyle::Heading, 20.0, FontFamily::Proportional),
        (TextStyle::Monospace, 13.5, FontFamily::Monospace),
    ] {
        style.text_styles.insert(s, FontId::new(size, family));
    }
    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::Theme::Dark);
}
