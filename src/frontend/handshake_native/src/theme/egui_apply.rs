//! Maps an `HsPalette` onto egui's `Visuals` and installs it on the context.
//!
//! egui Visuals field list used below (RISK-5 / CONTROL-5): when bumping egui across a
//! minor version, re-check these exact fields against the new `egui::Visuals` /
//! `egui::style::Widgets` definitions and update this function. Pinned to the egui 0.33
//! family (MT-001 spike verdict).
//!   Visuals: window_fill, panel_fill, faint_bg_color, extreme_bg_color, hyperlink_color,
//!            selection.bg_fill, override_text_color
//!   Widgets (noninteractive/inactive/active/hovered): bg_fill, fg_stroke

use crate::theme::palette::HsPalette;
use egui::{Color32, Context, Stroke, Visuals};

/// Build egui `Visuals` from a resolved palette and install them on `ctx`.
///
/// Cheap to call every frame: `set_visuals` clones a small struct and does not allocate
/// layout. `HandshakeApp` additionally gates the call on a theme change (CONTROL-1) so the
/// common no-change frame skips it entirely.
pub fn apply_to_ctx(palette: &HsPalette, ctx: &Context) {
    let mut visuals: Visuals = if palette.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.window_fill = palette.surface;
    visuals.panel_fill = palette.bg;
    visuals.faint_bg_color = palette.surface;
    visuals.extreme_bg_color = palette.bg;
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.override_text_color = Some(palette.text);

    set_widget_colors(&mut visuals, palette);

    ctx.set_visuals(visuals);
}

fn set_widget_colors(visuals: &mut Visuals, palette: &HsPalette) {
    let w = &mut visuals.widgets;

    w.noninteractive.bg_fill = palette.bg;
    w.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);

    w.inactive.bg_fill = palette.surface;
    w.inactive.fg_stroke = Stroke::new(1.0, palette.text);

    w.active.bg_fill = palette.accent;
    w.active.fg_stroke = Stroke::new(1.0, palette.surface);

    w.hovered.bg_fill = palette.accent_soft;
    w.hovered.fg_stroke = Stroke::new(1.5, palette.accent);
}

/// Equality probe used by the theme tests: does `ctx`'s installed visuals' `panel_fill`
/// match the expected background? (Lets the headless test prove `apply_to_ctx` actually
/// reached the context without a GPU/pixel readback.)
pub fn installed_panel_fill(ctx: &Context) -> Color32 {
    ctx.style().visuals.panel_fill
}
