//! WP-KERNEL-012 MT-121 — the MCP navigation snapshot publishes VIEWPORT-RELATIVE bounds.
//! WP-KERNEL-012 MT-135 — and it publishes the memory-backed popups that are actually open.
//!
//! ## MT-135 (the second defect, fixed here)
//!
//! MT-121 sized the capture viewport, which fixed coordinates only. The capture pass still ran on a
//! brand-new `egui::Context` whose `egui::Memory` was empty, and popup open state lives in exactly
//! that memory — so a context menu the operator could see was absent from the published snapshot and
//! a model driving Argus could not see, target, or steer it. `refresh_mcp_snapshot` now projects the
//! live context's single open-popup entry (id + anchor position) into the capture context via
//! `accessibility::popup_projection`. The MT-121 known-gap witness below is inverted accordingly, and
//! the `mt135_*` tests add the positive bounds proof, the closed-menu negative proof, and the manual
//! self-consistency check.
//!
//! ## The defect these tests exist to keep out
//!
//! `HandshakeApp::refresh_mcp_snapshot` renders the model-vision capture pass on an ISOLATED
//! `egui::Context`. It used to run that pass with `egui::RawInput::default()`, whose `screen_rect` is
//! `None`; egui then substitutes a synthetic 10000x10000 viewport
//! (`egui-0.33.3/src/input_state/mod.rs:368`). Every `UiNodeBounds` published through `argus.inspect`
//! was therefore measured against a window that does not exist — a model asked whether a control is
//! clipped, overlapping, or off-pane got a confidently wrong answer, and any HBR-VIS geometry gate
//! built on those coordinates was a false-proof generator.
//!
//! ## Why these assertions cannot pass under the old behaviour (AC-121-2, anti-vacuity)
//!
//! Every assertion here is anchored to the DECLARED viewport that the snapshot itself publishes, and
//! the live-window case pins that viewport to the harness window size. Restore the unsized context and
//! the shell's right/bottom-docked chrome lays out near x=10000/y=10000, which is decisively outside a
//! 1400x900 window — `bounds_outside_declared_viewport` reports every escapee by id. There is no
//! configuration of the pre-fix code in which these tests are silently satisfied.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::{SnapshotViewport, UiTreeSnapshot, ViewportSource};
use handshake_native::app::{HandshakeApp, HealthDisplayState, SNAPSHOT_FALLBACK_VIEWPORT};
use handshake_native::backend_client::HealthInfo;
use handshake_native::manual_content_editors::{
    editors_manual_section, AGENT_TOOL_REFERENCE_HEADING,
};

/// The harness window size. Deliberately far below egui's 10000pt unsized default so an unsized
/// capture cannot accidentally satisfy containment.
const HARNESS_W: f32 = 1400.0;
const HARNESS_H: f32 = 900.0;

/// A real, always-mounted shell chrome control with a stable `author_id` — the named surface used for
/// the PT-121-3 published-vs-rendered comparison.
const THEME_TOGGLE_AUTHOR_ID: &str = "shell.chrome.theme-toggle";

/// A pane-header context-menu leaf. Reachable only through a pointer secondary click, so it lives in
/// `egui::Memory` on the LIVE context — the AC-121-4 witness surface.
const PANE_POP_OUT_AUTHOR_ID: &str = "ctx-menu.pane.pop_out";

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// The live shell driven through egui_kittest at a KNOWN window size. `Harness` sets
/// `RawInput::screen_rect` from `with_size`, so `HandshakeApp::frame_ctx` (captured on the first
/// `ui()` pass) carries a real, non-default viewport — the production-equivalent live-window case.
fn shell_harness<'a>() -> Harness<'a, HandshakeApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(HARNESS_W, HARNESS_H))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness
}

/// Every node whose published bounds ORIGIN falls outside the snapshot's own declared viewport,
/// rendered as a readable one-line report per escapee.
///
/// Containment is tested on the origin rather than the full rect because a legitimately clipped child
/// of a scroll area may extend past the visible edge while still being a correct viewport-relative
/// measurement. An UNSIZED capture fails this anyway: its widgets do not merely overhang the edge,
/// their origins are placed thousands of points beyond the window.
fn bounds_outside_declared_viewport(
    snapshot: &UiTreeSnapshot,
    viewport: &SnapshotViewport,
) -> Vec<String> {
    snapshot
        .iter_nodes()
        .filter_map(|node| node.bounds.map(|bounds| (node, bounds)))
        .filter(|(_, bounds)| !viewport.contains_origin(bounds))
        .map(|(node, bounds)| {
            format!(
                "  id={} role={} bounds=(x={:.3}, y={:.3}, w={:.3}, h={:.3})",
                node.id, node.role, bounds.x, bounds.y, bounds.w, bounds.h
            )
        })
        .collect()
}

/// The bounds the REAL rendered frame carries for `author_id`, read straight off the harness's live
/// AccessKit tree (the same tree the platform UIA adapter receives). This is the frame the operator
/// sees — not a second capture pass — so comparing it to the published snapshot is a true
/// published-vs-rendered check.
fn rendered_frame_bounds(
    harness: &Harness<'_, HandshakeApp>,
    author_id: &str,
) -> Option<egui::accesskit::Rect> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        // `raw_bounds` is the node's OWN untransformed rect — exactly the value
        // `accessibility::snapshot::leaf_node` reads via `accesskit::Node::bounds`, so the two sides of
        // this comparison are the same measurement taken from two different frames.
        .and_then(|node| node.accesskit_node().raw_bounds())
}

/// PT-121-1 — bounds published through the MCP snapshot lie inside the viewport the snapshot declares,
/// and that viewport is the LIVE window rather than egui's unsized default.
#[test]
fn mt121_published_bounds_lie_within_the_declared_live_viewport() {
    let mut harness = shell_harness();
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();

    let viewport = snapshot
        .viewport
        .expect("the MCP capture path always declares the viewport its bounds were laid out against");
    assert_eq!(
        viewport.source,
        ViewportSource::LiveWindow,
        "a shell that has rendered a frame measures the live window, not a fallback: {viewport:?}"
    );
    assert_eq!(
        (viewport.w, viewport.h),
        (HARNESS_W, HARNESS_H),
        "the declared viewport is the live window size, not egui's 10000x10000 unsized default: {viewport:?}"
    );

    let escapees = bounds_outside_declared_viewport(&snapshot, &viewport);
    assert!(
        escapees.is_empty(),
        "{} of {} published nodes carry bounds outside the declared {}x{} viewport — the snapshot is \
describing a window that does not exist:\n{}",
        escapees.len(),
        snapshot.widget_count,
        viewport.w,
        viewport.h,
        escapees.join("\n")
    );
}

/// PT-121-3 — for one real, named surface, the bounds published to models MATCH the bounds in the
/// rendered frame. This is the assertion that would have caught the MT-119 PT-119-3 false positive
/// (an inspect-tree right edge of 2605px inside a 2560px window while the GPU frame showed the control
/// comfortably inside its pane).
#[test]
fn mt121_published_bounds_match_the_rendered_frame_for_a_real_surface() {
    let mut harness = shell_harness();
    let rendered = rendered_frame_bounds(&harness, THEME_TOGGLE_AUTHOR_ID)
        .expect("the theme toggle is mounted in the live rendered frame");

    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let viewport = snapshot.viewport.expect("declared viewport");
    let node = snapshot
        .find_unique_by_author_id(THEME_TOGGLE_AUTHOR_ID)
        .expect("the theme toggle is uniquely addressable in the published snapshot");
    let published = node
        .bounds
        .expect("a mounted interactive control publishes bounds");

    assert!(
        viewport.contains_rect(&published),
        "published theme-toggle rect {published:?} must lie wholly inside the declared viewport \
{viewport:?}"
    );

    // Same window, same layout inputs, so the published rect must reproduce the rendered rect. The
    // tolerance covers egui's UI-grid rounding only, not a different layout.
    let tolerance = 1.0_f32;
    let rendered_x = rendered.x0 as f32;
    let rendered_y = rendered.y0 as f32;
    let rendered_w = (rendered.x1 - rendered.x0) as f32;
    let rendered_h = (rendered.y1 - rendered.y0) as f32;
    let deltas = [
        ("x", published.x, rendered_x),
        ("y", published.y, rendered_y),
        ("w", published.w, rendered_w),
        ("h", published.h, rendered_h),
    ];
    let mismatches: Vec<String> = deltas
        .iter()
        .filter(|(_, published, rendered)| (published - rendered).abs() > tolerance)
        .map(|(axis, published, rendered)| {
            format!("{axis}: published={published:.3} rendered={rendered:.3}")
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "published bounds must describe the rendered frame for '{THEME_TOGGLE_AUTHOR_ID}'.\n  \
published=(x={:.3}, y={:.3}, w={:.3}, h={:.3})\n  rendered=(x={rendered_x:.3}, y={rendered_y:.3}, \
w={rendered_w:.3}, h={rendered_h:.3})\n  mismatched: {}",
        published.x,
        published.y,
        published.w,
        published.h,
        mismatches.join(", ")
    );

    // Evidence line for the MT-121 before/after record (visible with --nocapture).
    println!(
        "MT-121 PT-121-3 viewport={:?} source={:?} published=(x={:.3}, y={:.3}, w={:.3}, h={:.3}) \
rendered=(x={rendered_x:.3}, y={rendered_y:.3}, w={rendered_w:.3}, h={rendered_h:.3})",
        (viewport.w, viewport.h),
        viewport.source,
        published.x,
        published.y,
        published.w,
        published.h
    );
}

/// AC-121-1 — with no live window viewport available, the capture uses the shell's DECLARED default
/// window size and says so. It must never silently fall through to egui's 10000x10000 default.
#[test]
fn mt121_headless_capture_declares_the_documented_fallback_viewport() {
    let mut app = ok_app();
    let snapshot = app.capture_mcp_snapshot_for_navigation();

    let viewport = snapshot.viewport.expect("a headless capture still declares its viewport");
    assert_eq!(
        viewport.source,
        ViewportSource::DeclaredFallback,
        "a capture with no live window must report the fallback honestly: {viewport:?}"
    );
    assert_eq!(
        (viewport.w, viewport.h),
        (SNAPSHOT_FALLBACK_VIEWPORT.x, SNAPSHOT_FALLBACK_VIEWPORT.y),
        "the fallback is the shell's own declared default window size"
    );
    assert!(
        viewport.w < 10_000.0 && viewport.h < 10_000.0,
        "the fallback is never egui's unsized 10000x10000 viewport: {viewport:?}"
    );

    let escapees = bounds_outside_declared_viewport(&snapshot, &viewport);
    assert!(
        escapees.is_empty(),
        "{} of {} published nodes escape the declared fallback {}x{} viewport:\n{}",
        escapees.len(),
        snapshot.widget_count,
        viewport.w,
        viewport.h,
        escapees.join("\n")
    );
}

/// AC-121-4 / AC-135-1 — an `egui::Memory`-backed pane context menu that is OPEN in the live window is
/// published to models.
///
/// ## History (why this test reads as an inversion)
///
/// MT-121 sized the capture viewport, which fixed COORDINATES only. `RawInput::screen_rect` feeds
/// `InputState`, not `Memory`: popup/context-menu open state lives in the LIVE context's `Memory`, and
/// the capture pass builds a brand-new `egui::Context` that starts with an empty one. Under MT-121 this
/// test therefore asserted the DEFECT — the pane menu was present in the rendered frame and absent from
/// the published snapshot — and it was left in the suite as a known-gap witness.
///
/// **WP-KERNEL-012 MT-135 repaired that defect** by projecting the live context's open-popup memory
/// state (id + anchor position) into the capture context in
/// `HandshakeApp::refresh_mcp_snapshot`, via `accessibility::popup_projection`. The assertion below is
/// the INVERSION: the menu must now be visible to the capture pass. It no longer documents the defect
/// as expected behaviour.
#[test]
fn mt121_memory_backed_context_menu_remains_invisible_to_the_sized_capture_pass() {
    let mut harness = shell_harness();
    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();

    let live_leaf_present = harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(PANE_POP_OUT_AUTHOR_ID));
    assert!(
        live_leaf_present,
        "precondition: the pointer-opened pane menu IS in the live rendered frame"
    );

    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let viewport = snapshot.viewport.expect("declared viewport");
    assert_eq!(
        viewport.source,
        ViewportSource::LiveWindow,
        "the capture pass is sized from the live window: {viewport:?}"
    );
    assert!(
        snapshot.find_by_author_id(PANE_POP_OUT_AUTHOR_ID).is_some(),
        "AC-135-1: '{PANE_POP_OUT_AUTHOR_ID}' is open in the live rendered frame, so it MUST appear in \
the published MCP snapshot. If this fails, the MT-135 open-popup projection into the capture context \
has regressed and models are blind to context menus again."
    );
    println!(
        "MT-135 AC-135-1 sized capture viewport={:?} source={:?}; memory-backed leaf \
'{PANE_POP_OUT_AUTHOR_ID}' live=true published=true (fresh-context popup defect repaired by MT-135)",
        (viewport.w, viewport.h),
        viewport.source
    );
}

/// PT-135-2 / AC-135-3 — the projected context menu is not merely PRESENT, it is published with the
/// SAME bounds the rendered frame carries, and those bounds lie inside the declared viewport.
///
/// This is the assertion that keeps the projection honest against the MT-121 coordinate contract: a
/// projection that re-opened the popup without its stored anchor position, or that imported live
/// focus/scroll state and shifted layout, would put the menu somewhere the operator's window does not
/// show it — a worse falsehood than not publishing it at all.
#[test]
fn mt135_projected_context_menu_bounds_match_the_rendered_frame_inside_the_viewport() {
    let mut harness = shell_harness();
    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();

    let rendered = rendered_frame_bounds(&harness, PANE_POP_OUT_AUTHOR_ID)
        .expect("precondition: the pointer-opened pane menu leaf is in the live rendered frame");

    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let viewport = snapshot.viewport.expect("declared viewport");
    let node = snapshot
        .find_unique_by_author_id(PANE_POP_OUT_AUTHOR_ID)
        .expect(
            "AC-135-1: the open pane menu leaf is uniquely addressable in the published snapshot",
        );
    let published = node
        .bounds
        .expect("a projected, mounted menu item publishes bounds");

    assert!(
        viewport.contains_rect(&published),
        "AC-135-3: published pane-menu rect {published:?} must lie wholly inside the declared viewport \
{viewport:?}"
    );

    let tolerance = 1.0_f32;
    let rendered_x = rendered.x0 as f32;
    let rendered_y = rendered.y0 as f32;
    let rendered_w = (rendered.x1 - rendered.x0) as f32;
    let rendered_h = (rendered.y1 - rendered.y0) as f32;
    let deltas = [
        ("x", published.x, rendered_x),
        ("y", published.y, rendered_y),
        ("w", published.w, rendered_w),
        ("h", published.h, rendered_h),
    ];
    let mismatches: Vec<String> = deltas
        .iter()
        .filter(|(_, published, rendered)| (published - rendered).abs() > tolerance)
        .map(|(axis, published, rendered)| {
            format!("{axis}: published={published:.3} rendered={rendered:.3}")
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "AC-135-3: published bounds must describe the rendered frame for \
'{PANE_POP_OUT_AUTHOR_ID}'.\n  published=(x={:.3}, y={:.3}, w={:.3}, h={:.3})\n  \
rendered=(x={rendered_x:.3}, y={rendered_y:.3}, w={rendered_w:.3}, h={rendered_h:.3})\n  \
mismatched: {}",
        published.x,
        published.y,
        published.w,
        published.h,
        mismatches.join(", ")
    );

    // The whole-tree containment guard must still hold with a menu projected into the capture pass —
    // the projection must not push any node outside the declared viewport (MT-121 regression guard).
    let escapees = bounds_outside_declared_viewport(&snapshot, &viewport);
    assert!(
        escapees.is_empty(),
        "AC-135-3: {} of {} published nodes escape the declared {}x{} viewport while a context menu is \
projected:\n{}",
        escapees.len(),
        snapshot.widget_count,
        viewport.w,
        viewport.h,
        escapees.join("\n")
    );

    println!(
        "MT-135 PT-135-2 viewport={:?} source={:?} published=(x={:.3}, y={:.3}, w={:.3}, h={:.3}) \
rendered=(x={rendered_x:.3}, y={rendered_y:.3}, w={rendered_w:.3}, h={rendered_h:.3})",
        (viewport.w, viewport.h),
        viewport.source,
        published.x,
        published.y,
        published.w,
        published.h
    );
}

/// PT-135-3 / AC-135-5 — NEGATIVE PROOF. A context menu that is CLOSED must stay absent from the
/// snapshot.
///
/// Two closed states are checked, because they fail differently:
/// 1. never opened — a projection that unconditionally forced menus open would publish it here;
/// 2. opened and then dismissed with Escape — a projection that latched the open state, or that copied
///    a stale `egui::Memory` entry, would keep publishing a menu the operator already dismissed and
///    models would act on a surface that is not on screen.
#[test]
fn mt135_closed_context_menu_is_absent_from_the_published_snapshot() {
    let mut harness = shell_harness();

    let never_opened = harness.state_mut().capture_mcp_snapshot_for_navigation();
    assert!(
        never_opened
            .find_by_author_id(PANE_POP_OUT_AUTHOR_ID)
            .is_none(),
        "AC-135-5: a context menu that was never opened must not appear in the published snapshot"
    );

    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();
    let while_open = harness.state_mut().capture_mcp_snapshot_for_navigation();
    assert!(
        while_open
            .find_by_author_id(PANE_POP_OUT_AUTHOR_ID)
            .is_some(),
        "precondition for the dismissal half of this proof: the OPEN menu is published"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    let live_leaf_present = harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(PANE_POP_OUT_AUTHOR_ID));
    assert!(
        !live_leaf_present,
        "precondition: Escape dismissed the menu in the LIVE rendered frame"
    );

    let after_dismiss = harness.state_mut().capture_mcp_snapshot_for_navigation();
    assert!(
        after_dismiss
            .find_by_author_id(PANE_POP_OUT_AUTHOR_ID)
            .is_none(),
        "AC-135-5: a dismissed context menu must disappear from the published snapshot — the \
projection must track the live open/closed state, not latch it"
    );
    println!(
        "MT-135 PT-135-3 '{PANE_POP_OUT_AUTHOR_ID}' published: never_opened=false while_open=true \
after_escape=false"
    );
}

/// MT-135 UserManual self-consistency — the in-product Agent Tool Reference must describe the
/// capture pass's REAL context-menu visibility, proven in the same test run rather than asserted
/// against prose alone.
///
/// The manual previously told models that "pointer-opened context menus and other egui::Memory-backed
/// popups ... remain absent from the capture pass". That sentence was true under MT-121 and is false
/// now; a manual that keeps it would send models looking for a capture gap that no longer exists. This
/// test drives the live shell to establish the runtime truth (menu open => published, dismissed =>
/// absent) and only then checks that the manual states that same truth.
#[test]
fn mt135_manual_agent_tool_reference_matches_capture_pass_truth() {
    // 1. Runtime truth, taken from the live shell.
    let mut harness = shell_harness();
    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();
    let open_menu_is_published = harness
        .state_mut()
        .capture_mcp_snapshot_for_navigation()
        .find_by_author_id(PANE_POP_OUT_AUTHOR_ID)
        .is_some();
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    let closed_menu_is_published = harness
        .state_mut()
        .capture_mcp_snapshot_for_navigation()
        .find_by_author_id(PANE_POP_OUT_AUTHOR_ID)
        .is_some();
    assert!(
        open_menu_is_published && !closed_menu_is_published,
        "code truth for this test: open menu published={open_menu_is_published}, \
closed menu published={closed_menu_is_published}"
    );

    // 2. The manual entry a no-context model reads must state that same truth.
    let section = editors_manual_section();
    let body = section
        .topics
        .iter()
        .find(|topic| topic.heading == AGENT_TOOL_REFERENCE_HEADING)
        .map(|topic| topic.body.clone())
        .expect("the editors manual section carries the Agent Tool Reference topic");

    assert!(
        !body.contains("remain absent from the capture pass"),
        "the manual still documents the MT-121-era defect (context menus absent from the capture \
pass) while the runtime publishes them — a no-context model would be told to expect a gap that no \
longer exists"
    );
    for required in [
        "ctx-menu.surface.",
        "argus.click",
        "MT-135",
        "CLOSED or dismissed menu is absent",
    ] {
        assert!(
            body.contains(required),
            "the Agent Tool Reference must state '{required}' so a no-context model knows how to \
address an open menu and how to read its absence"
        );
    }
}

/// AC-121-1 — a repeated headless capture keeps reporting `DeclaredFallback`. The capture pass must not
/// adopt its own throwaway context as the "live" frame context and then re-publish that fallback as if
/// it had been measured from a window.
#[test]
fn mt121_repeat_headless_capture_does_not_launder_the_fallback_into_a_live_claim() {
    let mut app = ok_app();
    let _first = app.capture_mcp_snapshot_for_navigation();
    let second = app.capture_mcp_snapshot_for_navigation();

    let viewport = second.viewport.expect("declared viewport");
    assert_eq!(
        viewport.source,
        ViewportSource::DeclaredFallback,
        "the snapshot context is not a window and must never be reported as one: {viewport:?}"
    );
}
