//! WP-KERNEL-011 MT-006 — split / resize divider LIVE proof (egui_kittest headless harness).
//!
//! The pure layout / clamp / keyboard / SetValue arithmetic is unit-tested in
//! `src/split_layout.rs`. This file proves the *live* behavior the contract's `proof_targets`
//! require, which can only be observed by running the real `HandshakeApp` through the same egui +
//! AccessKit path the out-of-process Windows UIA adapter uses:
//!
//! 1. The 2x2 grid renders for several frames with no panic.
//! 2. The live AccessKit tree contains EXACTLY two `Role::Splitter` nodes with author_ids
//!    `divider-horizontal` / `divider-vertical`, each carrying a numeric value in [0.2, 0.8].
//! 3. An AccessKit `SetValue(0.0)` action on the horizontal divider clamps the live value to 0.2
//!    (SPLIT_MIN), proving an agent cannot collapse a pane to zero size end-to-end (not just in the
//!    pure helper).

use egui::accesskit::{self, Action, ActionData, ActionRequest, NodeId};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::split_layout::{
    DIVIDER_H_AUTHOR_ID, DIVIDER_H_NODE_ID, DIVIDER_V_AUTHOR_ID, DIVIDER_V_NODE_ID, SPLIT_MAX,
    SPLIT_MIN, SPLIT_STEP,
};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// (author_id, role, numeric_value) for every Splitter node in the live consumer-side tree.
fn live_splitters(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<f64>)> {
    use egui_kittest::kittest::NodeT;
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.role() == accesskit::Role::Splitter {
            found.push((
                ak.author_id().unwrap_or_default().to_owned(),
                format!("{:?}", ak.role()),
                ak.numeric_value(),
            ));
        }
    }
    found
}

/// The live numeric value of one Splitter divider by author_id.
fn divider_value(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> f64 {
    let splitters = live_splitters(harness);
    splitters
        .iter()
        .find(|(a, _, _)| a == author_id)
        .unwrap_or_else(|| panic!("divider '{author_id}' present; got {splitters:?}"))
        .2
        .unwrap_or_else(|| panic!("divider '{author_id}' carries a numeric value"))
}

/// The center pixel position of a live divider's hit-rect, read from the divider node's on-screen
/// rect (kittest derives this from the AccessKit bounding box egui populated from the `ui.interact`
/// response rect). Used to drive a REAL pointer drag onto the exact divider, rather than guessing
/// pixel coordinates from panel geometry. Looked up by the divider's stable AccessKit label.
fn divider_center(harness: &Harness<'_, HandshakeApp>, label: &str) -> egui::Pos2 {
    use egui_kittest::kittest::Queryable;
    harness.get_by_label(label).rect().center()
}

#[test]
fn grid_renders_five_frames_with_two_splitters_in_range() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    // proof_target: render for 5 frames with no panic.
    harness.run_steps(5);

    let splitters = live_splitters(&harness);
    println!("LIVE splitters: {splitters:?}");
    assert_eq!(splitters.len(), 2, "exactly two Splitter nodes; got {splitters:?}");

    let author_ids: Vec<&str> = splitters.iter().map(|(a, _, _)| a.as_str()).collect();
    assert!(
        author_ids.contains(&DIVIDER_H_AUTHOR_ID),
        "horizontal divider present; got {author_ids:?}"
    );
    assert!(
        author_ids.contains(&DIVIDER_V_AUTHOR_ID),
        "vertical divider present; got {author_ids:?}"
    );

    for (author_id, role, value) in &splitters {
        assert_eq!(role, "Splitter", "{author_id} role");
        let v = value.unwrap_or_else(|| panic!("{author_id} carries a numeric value"));
        assert!(
            (SPLIT_MIN as f64..=SPLIT_MAX as f64).contains(&v),
            "{author_id} value {v} in [{SPLIT_MIN}, {SPLIT_MAX}]"
        );
    }
    println!("PASS: 2x2 grid rendered 5 frames; two Splitter nodes present, values in range");
}

#[test]
fn set_value_zero_clamps_live_horizontal_divider_to_split_min() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    // Dispatch the same AccessKit action an out-of-process agent would: SetValue(0.0) targeting the
    // horizontal divider's stable NodeId. egui surfaces this via input.accesskit_action_requests,
    // which the divider consumes (apply_set_value -> clamp). This is the END-TO-END clamp proof:
    // the value is clamped in the live frame, not merely in the pure helper.
    harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target: NodeId(DIVIDER_H_NODE_ID),
        data: Some(ActionData::NumericValue(0.0)),
    }));
    harness.run();

    let splitters = live_splitters(&harness);
    let (_, _, value) = splitters
        .iter()
        .find(|(a, _, _)| a == DIVIDER_H_AUTHOR_ID)
        .expect("horizontal divider present");
    let v = value.expect("horizontal divider numeric value");
    assert!(
        (v - SPLIT_MIN as f64).abs() < 1e-4,
        "SetValue(0.0) must clamp the live horizontal divider to SPLIT_MIN (0.2); got {v}"
    );
    println!("PASS: live SetValue(0.0) on '{DIVIDER_H_AUTHOR_ID}' clamped to {v} (SPLIT_MIN)");
}

#[test]
fn set_value_high_clamps_live_vertical_divider_to_split_max() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target: NodeId(DIVIDER_V_NODE_ID),
        data: Some(ActionData::NumericValue(0.95)),
    }));
    harness.run();

    let splitters = live_splitters(&harness);
    let (_, _, value) = splitters
        .iter()
        .find(|(a, _, _)| a == DIVIDER_V_AUTHOR_ID)
        .expect("vertical divider present");
    let v = value.expect("vertical divider numeric value");
    assert!(
        (v - SPLIT_MAX as f64).abs() < 1e-4,
        "SetValue(0.95) must clamp the live vertical divider to SPLIT_MAX (0.8); got {v}"
    );
    println!("PASS: live SetValue(0.95) on '{DIVIDER_V_AUTHOR_ID}' clamped to {v} (SPLIT_MAX)");
}

// ── FIX-3: LIVE input-path proofs (real pointer drag + real keyboard), not arithmetic ──────────────
//
// AC-2/3/4 must be proven through the REAL widget code (egui `Response::dragged()` /
// `has_focus()` + arrow keys), not just the pure clamp helpers. These tests drive the live
// `HandshakeApp` through the same egui input pipeline the desktop app uses.

/// AC-2/AC-3 (LIVE): a real pointer drag on the HORIZONTAL divider's hit-rect changes
/// `weights.horizontal` (the divider's live numeric value) and clamps within [0.2, 0.8]. The drag is
/// dispatched onto the divider's actual on-screen center (read from its AccessKit bounds), so it
/// exercises `response.dragged()` + `drag_delta()` in the real widget, not arithmetic.
#[test]
fn live_pointer_drag_horizontal_divider_changes_weight_and_clamps() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let before = divider_value(&harness, DIVIDER_H_AUTHOR_ID);
    // The horizontal line spans the full width; grab it WELL AWAY from x=center (where the vertical
    // divider crosses it) so the drag lands on the horizontal divider, not the crossing point.
    let h_center = divider_center(&harness, "Horizontal split divider");
    let center = egui::pos2(h_center.x * 0.25, h_center.y);
    println!("LIVE drag horizontal: grab point = {center:?} (line y={})", h_center.y);

    // Drive the drag one frame PER pointer event (`step`, not `run`). egui hit-tests at the START of
    // a frame using the pointer position recorded at the END of the previous frame, so each pointer
    // event needs a following settle frame for `hovered()`/`dragged()` to catch up. We therefore
    // emit one settle `step()` after each pointer event. `run` would collapse the whole gesture into
    // a single repaint-to-stable pass and lose the incremental deltas.
    harness.hover_at(center);
    harness.step();
    harness.step(); // settle: hit-test now sees the pointer over the divider
    harness.drag_at(center); // press at the divider
    harness.step();
    harness.step();
    for s in 1..=8 {
        harness.hover_at(egui::pos2(center.x, center.y + s as f32 * 10.0)); // held-button move down
        harness.step();
        harness.step();
    }
    harness.drop_at(egui::pos2(center.x, center.y + 80.0));
    harness.step();

    let after = divider_value(&harness, DIVIDER_H_AUTHOR_ID);
    println!("LIVE drag horizontal: before={before} after={after}");
    assert!(
        after > before,
        "dragging the horizontal divider DOWN must increase weights.horizontal (live): \
         before={before} after={after}"
    );
    assert!(
        (SPLIT_MIN as f64..=SPLIT_MAX as f64).contains(&after),
        "live dragged value stays clamped in [{SPLIT_MIN}, {SPLIT_MAX}]; got {after}"
    );
    println!("PASS: live pointer drag moved + clamped the horizontal divider through the real widget");
}

/// AC-3 (LIVE): a real pointer drag FAR past the bottom clamps `weights.horizontal` to SPLIT_MAX
/// through the live widget (not the pure helper).
#[test]
fn live_pointer_drag_far_clamps_to_max() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let h_center = divider_center(&harness, "Horizontal split divider");
    // Grab away from the vertical-divider crossing (see sibling test).
    let center = egui::pos2(h_center.x * 0.25, h_center.y);
    harness.hover_at(center);
    harness.step();
    harness.step();
    harness.drag_at(center);
    harness.step();
    harness.step();
    // Drag well past the bottom edge of the 600px-tall window, one settle frame per move.
    for s in 1..=12 {
        harness.hover_at(egui::pos2(center.x, center.y + s as f32 * 100.0));
        harness.step();
        harness.step();
    }
    harness.drop_at(egui::pos2(center.x, center.y + 1300.0));
    harness.step();

    let after = divider_value(&harness, DIVIDER_H_AUTHOR_ID);
    assert!(
        (after - SPLIT_MAX as f64).abs() < 1e-3,
        "dragging far past the bottom must clamp the live horizontal divider to SPLIT_MAX (0.8); \
         got {after}"
    );
    println!("PASS: live over-drag clamped the horizontal divider to SPLIT_MAX ({after})");
}

/// AC-4 (LIVE): focus the VERTICAL divider (AccessKit Focus action), then press ArrowRight; the
/// live `weights.vertical` (its numeric value) increases by SPLIT_STEP through the real
/// `response.has_focus()` + arrow-key path. Proves keyboard resize works end-to-end, and that key
/// events are consumed via the focus gate.
#[test]
fn live_keyboard_arrow_resizes_focused_vertical_divider() {
    use egui_kittest::kittest::Queryable;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let before = divider_value(&harness, DIVIDER_V_AUTHOR_ID);

    // Focus the divider by its stable AccessKit author_id, then run so focus takes effect.
    harness.get_by_label("Vertical split divider").focus();
    harness.run();

    // ArrowRight on the vertical line grows weights.vertical by SPLIT_STEP (React axis "vertical").
    harness.key_press(egui::Key::ArrowRight);
    harness.run();

    let after = divider_value(&harness, DIVIDER_V_AUTHOR_ID);
    println!("LIVE keyboard vertical: before={before} after={after}");
    assert!(
        (after - (before + SPLIT_STEP as f64)).abs() < 1e-4,
        "ArrowRight on the focused vertical divider must add SPLIT_STEP to weights.vertical (live): \
         before={before} after={after} step={SPLIT_STEP}"
    );
    assert!(
        (SPLIT_MIN as f64..=SPLIT_MAX as f64).contains(&after),
        "live keyboard-resized value stays clamped; got {after}"
    );
    println!("PASS: live ArrowRight on the focused vertical divider stepped weights.vertical");
}

/// AC-4 (LIVE): the keyboard path is GATED on focus — an arrow key with NO divider focused must NOT
/// move any weight (red-team CONTROL: arrow keys never steal input when the divider is not focused).
#[test]
fn live_keyboard_arrow_without_focus_does_not_move_divider() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let before_h = divider_value(&harness, DIVIDER_H_AUTHOR_ID);
    let before_v = divider_value(&harness, DIVIDER_V_AUTHOR_ID);

    // No focus set on any divider -> arrow keys must be ignored by the dividers.
    harness.key_press(egui::Key::ArrowDown);
    harness.key_press(egui::Key::ArrowRight);
    harness.run();

    let after_h = divider_value(&harness, DIVIDER_H_AUTHOR_ID);
    let after_v = divider_value(&harness, DIVIDER_V_AUTHOR_ID);
    assert!((after_h - before_h).abs() < 1e-9, "unfocused arrow must not move horizontal divider");
    assert!((after_v - before_v).abs() < 1e-9, "unfocused arrow must not move vertical divider");
    println!("PASS: arrow keys are ignored while no divider is focused (focus-gated)");
}
