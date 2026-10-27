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
    visuals.override_text_color = None;

    set_widget_colors(&mut visuals, palette);

    ctx.set_visuals(visuals);
}

#[derive(Clone, Copy)]
struct MotionPreference {
    reduced: bool,
    normal_light: f32,
    normal_dark: f32,
}

pub(crate) fn apply_motion_preference(ctx: &Context, reduced: bool) {
    let id = egui::Id::new("handshake.motion-preference");
    let previous = ctx.data(|data| data.get_temp::<MotionPreference>(id));
    if previous.is_some_and(|state| state.reduced == reduced)
        || (previous.is_none() && !reduced)
    {
        return;
    }
    let (normal_light, normal_dark) = match previous {
        Some(state) if state.reduced => (state.normal_light, state.normal_dark),
        _ => (
            ctx.style_of(egui::Theme::Light).animation_time,
            ctx.style_of(egui::Theme::Dark).animation_time,
        ),
    };
    ctx.style_mut_of(egui::Theme::Light, |style| {
        style.animation_time = if reduced { 0.0 } else { normal_light };
    });
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.animation_time = if reduced { 0.0 } else { normal_dark };
    });
    ctx.clear_animations();
    ctx.data_mut(|data| data.insert_temp(id, MotionPreference {
        reduced, normal_light, normal_dark,
    }));
}

fn set_widget_colors(visuals: &mut Visuals, palette: &HsPalette) {
    let w = &mut visuals.widgets;

    w.noninteractive.bg_fill = palette.bg;
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text);

    w.inactive.bg_fill = palette.surface;
    w.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);

    w.active.bg_fill = palette.accent;
    w.active.fg_stroke = Stroke::new(
        1.0_f32,
        if palette.is_dark() { palette.surface } else { palette.text },
    );

    w.hovered.bg_fill = palette.accent_soft;
    w.hovered.fg_stroke = Stroke::new(1.5_f32, palette.accent);
}

/// Equality probe used by the theme tests: does `ctx`'s installed visuals' `panel_fill`
/// match the expected background? (Lets the headless test prove `apply_to_ctx` actually
/// reached the context without a GPU/pixel readback.)
pub fn installed_panel_fill(ctx: &Context) -> Color32 {
    ctx.style().visuals.panel_fill
}

#[cfg(test)]
mod motion_tests {
    use super::*;

    #[test]
    fn reduce_motion_updates_both_themes_and_restores_each_duration() {
        let ctx = Context::default();
        ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.17);
        ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.23);
        apply_motion_preference(&ctx, false);
        assert_eq!(ctx.style_of(egui::Theme::Light).animation_time, 0.17);
        apply_motion_preference(&ctx, true);
        assert_eq!(ctx.style_of(egui::Theme::Light).animation_time, 0.0);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.0);
        apply_motion_preference(&ctx, true);
        apply_motion_preference(&ctx, false);
        assert_eq!(ctx.style_of(egui::Theme::Light).animation_time, 0.17);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.23);
        ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.31);
        apply_motion_preference(&ctx, true);
        apply_motion_preference(&ctx, false);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.31);
    }

    #[test]
    fn reduce_motion_is_applied_independently_to_each_context() {
        let root = Context::default();
        let detached = Context::default();
        let capture = Context::default();
        let normal = capture.style_of(egui::Theme::Dark).animation_time;
        apply_motion_preference(&root, true);
        assert_eq!(detached.style_of(egui::Theme::Dark).animation_time, normal);
        for ctx in [&detached, &capture] {
            apply_motion_preference(ctx, true);
            assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.0);
        }
        apply_motion_preference(&root, false);
        assert_eq!(root.style_of(egui::Theme::Dark).animation_time, normal);
        assert_eq!(detached.style_of(egui::Theme::Dark).animation_time, 0.0);
        assert_eq!(capture.style_of(egui::Theme::Dark).animation_time, 0.0);
    }

    #[test]
    fn account_reset_can_restore_timing_before_context_data_is_discarded() {
        let ctx = Context::default();
        ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.17);
        ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.23);
        apply_motion_preference(&ctx, true);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.0);
        apply_motion_preference(&ctx, false);
        ctx.data_mut(|data| data.clear());
        apply_motion_preference(&ctx, false);
        assert_eq!(ctx.style_of(egui::Theme::Light).animation_time, 0.17);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.23);
    }
}
