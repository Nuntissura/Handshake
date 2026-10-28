//! Native host motion preference and complete theme-toggle presentation.

use egui::Context;

#[derive(Clone, Copy)]
struct MotionPreference {
    reduced: bool,
    normal_light: f32,
    normal_dark: f32,
}

pub fn apply_motion_preference(ctx: &Context, reduced: bool) {
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

/// Draw the complete toggle using the caller's stable identity and target label.
/// The caller attaches its author ID and handles the returned live response.
pub fn theme_toggle(ui: &mut egui::Ui, id: egui::Id, label: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(14.0),
        ui.visuals().text_color(),
    );
    let padding = ui.spacing().button_padding;
    let desired = galley.size() + padding * 2.0;
    let (_auto, rect) = ui.allocate_space(desired);
    let response = ui.interact(rect, id, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let hover = ui.ctx().animate_bool_responsive(id.with("hover-fill"), response.hovered());
        let fill = if !ui.is_enabled() || response.is_pointer_button_down_on() || response.has_focus() {
            visuals.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill.lerp_to_gamma(
                ui.visuals().widgets.hovered.bg_fill, hover,
            )
        };
        ui.painter()
            .rect_filled(rect, visuals.corner_radius, fill);
        let text_pos = rect.center() - galley.size() * 0.5;
        ui.painter().galley(text_pos, galley, visuals.text_color());
    }

    // Emit Role::Button + label + Action::Click into the AccessKit tree for this id.
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
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
