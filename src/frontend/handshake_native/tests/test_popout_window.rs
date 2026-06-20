//! WP-KERNEL-011 MT-008 — LIVE pop-out window + merge-back proof (egui_kittest).
//!
//! These tests drive the REAL `HandshakeApp` headlessly through egui_kittest (which enables
//! AccessKit and produces the same `TreeUpdate` the out-of-process Windows UIA adapter receives) and
//! prove the MT-008 acceptance criteria end-to-end:
//!
//! - triggering a pop-out on pane-a renders a `PopOutPlaceholder` tile in pane-a's grid rect with a
//!   `Button` whose stable `author_id` is `merge-back-pane-a`;
//! - the detached window's title is the contract's `"Handshake – <pane_type_label>"`;
//! - clicking the Merge Back button via an AccessKit `Click` (the out-of-process steering path) merges
//!   the pane back: the placeholder disappears and `is_popped_out("pane-a")` becomes false;
//! - the OS close-button path (`ViewportInfo::close_requested`) also merges the pane back, exercised
//!   through the exact seam the deferred viewport callback hits;
//! - the pane's record is NEVER removed from the registry across the whole cycle (single source of
//!   truth);
//! - the popped-out pane is still accessible (its `pane-a` node + a `popout-window-pane-a` window root
//!   node are present in the live tree while detached).
//!
//! ## Headless scope (honest)
//!
//! On a plain headless `egui::Context` (kittest), `Context::embed_viewports()` is `true`, so
//! `show_viewport_deferred` runs its callback EMBEDDED in the current frame rather than opening a real
//! OS window — eframe sets `embed_viewports == false` (real multi-window) only on the live
//! wgpu/winit backend. That makes the pop-out's *content*, *placeholder*, *merge-back logic*, and
//! *AccessKit nodes* fully drivable here. The one part that genuinely needs a real winit event loop —
//! the OS actually raising a second top-level window and the user clicking its native title-bar X — is
//! the only step not exercised headlessly. We do NOT fake it: the close -> merge-back LOGIC is proven
//! by driving the `request_close` seam directly (the exact call the deferred callback makes on
//! `close_requested()`), and the genuine "second native window appears" step is left to manual /
//! real-window verification. See `popout_window.rs` module docs.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneId;
use handshake_native::popout_window::{
    merge_back_author_id, popout_title_for, popout_window_author_id,
};
use std::sync::Arc;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

fn pid(s: &str) -> PaneId {
    Arc::from(s)
}

/// Collect every (author_id, role, label) triple from the live consumer-side AccessKit tree — the
/// same surface an out-of-process model reads. Mirrors the helper in `test_accesskit_ids.rs`.
fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    use egui_kittest::kittest::NodeT;
    let mut found = Vec::new();
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

fn registry_len(app: &HandshakeApp) -> usize {
    app.pane_registry()
        .lock()
        .expect("registry mutex")
        .len()
}

#[test]
fn pop_out_renders_placeholder_with_merge_back_button_then_merge_back_via_accesskit_click() {
    // End-to-end proof of the headline acceptance flow, in ONE test so the before/during/after
    // registry-size invariant is asserted across the whole cycle.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());

    // Frame 1: fresh shell. pane-a is docked; NO merge-back button exists yet.
    harness.run();
    let before_len = registry_len(harness.state());
    assert_eq!(before_len, 4, "registry seeds four panes");
    assert!(
        harness.query_by_label("Merge Back").is_none(),
        "no Merge Back button while nothing is popped out"
    );
    assert!(
        !harness.state().is_popped_out(&pid("pane-a")),
        "pane-a starts docked"
    );

    // Trigger the pop-out (the seam a future MT-019 pane-header action / out-of-process driver uses).
    harness.state_mut().request_pop_out(pid("pane-a"));

    // Render 2 frames so the request is applied and the placeholder + detached viewport render.
    harness.run();
    harness.run();

    // During pop-out: pane-a is popped out, and the placeholder Button is in the LIVE main-window tree
    // with the stable author_id `merge-back-pane-a` and Role::Button.
    assert!(
        harness.state().is_popped_out(&pid("pane-a")),
        "pane-a is popped out after the request is applied"
    );
    let nodes = live_author_nodes(&harness);
    let merge = nodes
        .iter()
        .find(|(a, _, _)| a == &merge_back_author_id("pane-a"))
        .unwrap_or_else(|| {
            panic!(
                "merge-back-pane-a missing from LIVE tree; author_ids found: {:?}",
                nodes.iter().map(|(a, _, _)| a).collect::<Vec<_>>()
            )
        });
    assert_eq!(merge.1, "Button", "merge-back node is Role::Button");

    // The placeholder text "(popped out)" is present and the Merge Back button is findable by label
    // (the out-of-process locate path).
    assert!(
        harness.query_by_label_contains("(popped out)").is_some(),
        "placeholder shows the '(popped out)' marker"
    );

    // Registry STILL holds pane-a while it is popped out (single source of truth, never removed).
    assert_eq!(
        registry_len(harness.state()),
        before_len,
        "registry retains all panes WHILE pane-a is popped out"
    );

    // Merge back via an AccessKit Click on the Merge Back button — the exact out-of-process steering
    // path a model would use (click_accesskit dispatches accesskit::Action::Click).
    harness.get_by_label("Merge Back").click_accesskit();

    // Render 2 more frames so the click is consumed, merge_back marks the pop-out closed, and the
    // post-show drain removes it.
    harness.run();
    harness.run();

    // After merge-back: pane-a is docked again, the placeholder/button are gone, registry unchanged.
    assert!(
        !harness.state().is_popped_out(&pid("pane-a")),
        "pane-a is no longer popped out after the Merge Back click"
    );
    assert!(
        harness.query_by_label("Merge Back").is_none(),
        "Merge Back button gone after merge-back"
    );
    assert_eq!(
        registry_len(harness.state()),
        before_len,
        "registry retains all panes AFTER merge-back (never removed during the cycle)"
    );
    println!("PASS: pop-out -> placeholder+merge-back-pane-a Button -> AccessKit Click -> merged back; registry stable at {before_len}");
}

#[test]
fn popped_out_pane_window_title_and_window_node_are_present() {
    // The detached window title is "Handshake – <pane_type_label>" and the pop-out exposes a root
    // Window AccessKit node (popout-window-pane-a) so the detached window is itself addressable.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().request_pop_out(pid("pane-a"));
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _)| a.as_str()).collect();

    // pane-a (Workspace) -> title "Handshake – Workspace". The window root node carries this as label.
    let expected_title = popout_title_for("Workspace");
    let window = nodes
        .iter()
        .find(|(a, _, _)| a == &popout_window_author_id("pane-a"))
        .unwrap_or_else(|| {
            panic!("popout-window-pane-a missing from LIVE tree; found {author_ids:?}")
        });
    assert_eq!(window.1, "Window", "pop-out root is Role::Window");
    assert_eq!(
        window.2.as_deref(),
        Some(expected_title.as_str()),
        "pop-out window label is the contract title 'Handshake – Workspace'"
    );

    // The popped-out pane is STILL accessible by its stable docked author_id (only the host changed).
    assert!(
        author_ids.contains(&"pane-a"),
        "popped-out pane-a remains accessible by its author_id; found {author_ids:?}"
    );
    println!("PASS: detached window title '{expected_title}' + Window node + pane-a still accessible");
}

#[test]
fn os_close_button_path_merges_pane_back() {
    // The OS close-button merge-back path (ViewportInfo::close_requested) converges on the same
    // open=false flag the Merge Back button sets. This test drives the APP's REAL popout_manager
    // through its own update loop: pop pane-b out, simulate the native close via the app's
    // `request_os_close` seam (the exact `close_requested -> request_close` wiring the immediate
    // viewport callback performs), run a frame so the app's `show_all` drain runs, and assert the
    // APP's `is_popped_out("pane-b")` flips to false — proving the close seam through the app, not a
    // throwaway manager. (The genuine "user clicked the native window X" step needs a real winit
    // window and is the documented manual-verification remainder.)
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    let before_len = registry_len(harness.state());

    harness.state_mut().request_pop_out(pid("pane-b"));
    harness.run();
    harness.run();
    assert!(harness.state().is_popped_out(&pid("pane-b")), "pane-b popped out");

    // Simulate the OS close button on the APP's real pop-out (same call the immediate viewport
    // callback makes on close_requested()). Returns true because a pop-out existed for pane-b.
    let existed = harness.state_mut().request_os_close(&pid("pane-b"));
    assert!(existed, "request_os_close found the live pane-b pop-out");

    // Run frames so the app's own update loop calls show_all, whose post-show drain removes the
    // closed entry — the pane returns to the main split through the app itself.
    harness.run();
    harness.run();

    assert!(
        !harness.state().is_popped_out(&pid("pane-b")),
        "pane-b merged back after the OS close seam fired, driven through the app's own show_all"
    );
    // The placeholder/merge-back UI is gone now that pane-b is docked again.
    assert!(
        harness.query_by_label("Merge Back").is_none(),
        "Merge Back button gone after OS-close merge-back"
    );
    // The app's registry is untouched throughout (single source of truth).
    assert_eq!(
        registry_len(harness.state()),
        before_len,
        "registry retains all panes across the OS-close merge-back cycle"
    );
    println!("PASS: app.request_os_close -> app's show_all drain merges pane-b back; registry stable at {before_len}");
}

/// Read the consumer-side AccessKit bounding box (width, height) for the node with the given
/// `author_id` from the LIVE tree — the same geometry an out-of-process AT/UIA client sees. Returns
/// `None` if the node is absent or carries no bounds this frame.
fn author_node_bounds(
    harness: &Harness<'_, HandshakeApp>,
    author_id: &str,
) -> Option<(f64, f64)> {
    use egui_kittest::kittest::NodeT;
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            // bounding_box() applies the node's transform chain (the geometry a consumer reads);
            // raw_bounds() is the pre-transform fallback if the chain yields nothing this frame.
            let r = ak.bounding_box().or_else(|| ak.raw_bounds())?;
            return Some((r.width(), r.height()));
        }
    }
    None
}

#[test]
fn popped_out_pane_body_owns_full_viewport_width() {
    // POSITIVE-CORRECTNESS companion to the deterministic fail-on-old unit test
    // `popout_window::tests::show_all_does_not_open_a_central_panel_so_body_owns_the_only_one`.
    //
    // The round-2 MAJOR was that `show_all` opened a body-less CentralPanel JUST to host the
    // window-root node, leaving the body fighting a second same-id CentralPanel for the central rect.
    // The fix emits the window-root node from a zero-interaction `egui::Area`, so the body's
    // CentralPanel is the SOLE one and owns the full central rect. Here we assert the externally
    // meaningful result: the popped-out pane's body lays out across the full detached-viewport width.
    //
    // Honest scope note: this assertion holds on BOTH the fixed and the round-1 structures headlessly,
    // because egui's `allocate_central_panel` deliberately does NOT shrink `available_rect`
    // (egui-0.33 panel.rs / pass_state.rs), so a second CentralPanel is not starved of *space* — the
    // real round-1 defect is two overlapping same-id CentralPanels, which is what the unit test catches
    // deterministically (fail-on-old). This test guards that the fix did not REGRESS the body's full
    // width (the body must still span the viewport, wider than a single 2x2 grid cell).
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());

    // The body-layout witness is the pane's tab-bar strip node (`tabbar-pane-a`), which the body's
    // CentralPanel lays out across the central rect (the Group pane node's bounds are not always
    // populated headlessly). The contract explicitly allows asserting "the pane's (or tab bar's) live
    // AccessKit node BOUNDS ... approximately the viewport size".
    let tabbar_author = "tabbar-pane-a";

    // Baseline: while DOCKED, pane-a's tab bar spans only its 2x2 grid cell.
    harness.run();
    let docked = author_node_bounds(&harness, tabbar_author)
        .expect("pane-a tab bar has live bounds while docked");
    assert!(
        docked.0 > 1.0 && docked.1 > 1.0,
        "sanity: docked pane-a tab bar has real layout bounds, got {docked:?}"
    );

    harness.state_mut().request_pop_out(pid("pane-a"));
    harness.run();
    harness.run();
    assert!(
        harness.state().is_popped_out(&pid("pane-a")),
        "pane-a is popped out"
    );

    let popped = author_node_bounds(&harness, tabbar_author).unwrap_or_else(|| {
        panic!("popped-out pane-a body did not lay out a tab bar with bounds")
    });

    // The body spans real layout width (not a sliver), and is WIDER than a single docked grid cell —
    // the detached viewport is the full window, so the body's tab bar must span > one 2x2 cell.
    assert!(
        popped.0 > 100.0,
        "popped-out pane-a tab bar must span real layout width (the sole CentralPanel owns the \
         central rect); got width {} (docked cell width was {})",
        popped.0,
        docked.0
    );
    assert!(
        popped.0 > docked.0,
        "popped-out tab bar width {} must exceed the docked grid-cell width {} (the detached \
         viewport is the full window, wider than one 2x2 cell)",
        popped.0,
        docked.0
    );
    println!(
        "PASS: popped-out pane-a tab bar width {} spans the full detached viewport (docked cell \
         width {}); the body's CentralPanel owns the central rect",
        popped.0, docked.0
    );
}

#[test]
fn pop_out_does_not_change_default_seed_live_tree() {
    // Regression guard: with NO pop-out active, the live tree must be exactly the MT-007 baseline (no
    // merge-back / popout-window nodes leak into the default shell). This protects the strict 21-node
    // assertion in test_accesskit_ids.rs from this MT.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    let nodes = live_author_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _)| a.as_str()).collect();
    assert!(
        !author_ids.iter().any(|a| a.starts_with("merge-back-")),
        "no merge-back nodes in the default seed; found {author_ids:?}"
    );
    assert!(
        !author_ids.iter().any(|a| a.starts_with("popout-window-")),
        "no popout-window nodes in the default seed; found {author_ids:?}"
    );
    println!("PASS: default-seed live tree carries no pop-out nodes ({} author_id nodes)", nodes.len());
}
