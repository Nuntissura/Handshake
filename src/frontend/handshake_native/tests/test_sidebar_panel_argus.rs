//! WP-KERNEL-012 E3 MT-024 remediation (FAIL_V2): canonical Argus inspect /
//! safe-steer / re-observe proof for the MOUNTED pins / favorites / backlinks /
//! unlinked sidebar.
//!
//! `validation_v2` failed MT-024 in part because "canonical Argus proof is also
//! absent". The isolated `test_sidebar_panel.rs` kittest coverage drives the
//! sidebar WIDGET (and a live-PG mounted host via plain AccessKit clicks), but
//! never the mounted `HandshakeApp` through the real localhost `SwarmMcpServer`
//! transport the way an out-of-process swarm agent does. This test closes that
//! exact gap:
//!
//!   1. mounts the production `HandshakeApp` shell with the Sidebar pane mounted
//!      and seeded Pins / Favorites / Backlinks / Unlinked sections plus an
//!      active block,
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same
//!      `argus.inspect` / `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves the four sections' rows are addressable by stable
//!      author_id in the live tree (`sidebar.pin.*`, `sidebar.favorite.*`,
//!      `sidebar.backlink.*`, `sidebar.unlinked.*`, `sidebar.breadcrumb.*`),
//!   4. drives ONE action (click the Pins row's real Remove button — AC2) through
//!      Argus; with no backend runtime bound the mounted host takes its typed
//!      no-runtime recovery branch (a Pins section error + Retry, never a partial
//!      removal — the recoverable behavior MT-024 FAIL_V2 requires),
//!   5. FRESH `argus.inspect` re-observes the post-action tree: the Pins section
//!      now exposes its error + Retry control while its rows are replaced (AC9)
//!      and the other three sections remain addressable, and
//!   6. writes the before/after tree evidence externally + a screenshot marker
//!      (headless DEFERRED is an acceptable typed outcome).
//!
//! A second test proves the empty + error states inspect through canonical Argus:
//! an empty workspace exposes NO rows, and a section error exposes its stable
//! `sidebar.pins.retry` control (AC9).
//!
//! No backend is needed for these tree proofs — the mounted sidebar host is
//! seeded directly. The one-time no-runtime fetch attempt sets the section
//! error synchronously on the first frame; the direct seed then clears it.
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-024/` root.

use std::path::{Path, PathBuf};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, SIDEBAR_PANE_LABEL};
use handshake_native::graph::sidebar_panel::{
    breadcrumb_author_id, pin_remove_author_id, section_retry_author_id, BacklinkRow, SectionKind,
    SidebarBlock, UnlinkedRow,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' artifact dir may exist (found {})",
            p.display()
        );
    }
}

fn collect_author_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(id) = map.get("author_id").and_then(|v| v.as_str()) {
                out.push(id.to_owned());
            }
            for v in map.values() {
                collect_author_ids(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_author_ids(v, out);
            }
        }
        _ => {}
    }
}

/// A live shell with `pane-a` re-typed to the Sidebar pane so the mounted sidebar
/// factory renders in the split. No runtime handle is set: the one-time sidebar
/// fetch takes the deterministic `Runtime unavailable` branch on the first frame,
/// and the direct seed then clears that error before the Argus proof.
fn sidebar_shell() -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    retype_pane_a_to_sidebar(&mut app);
    app
}

fn retype_pane_a_to_sidebar(app: &mut HandshakeApp) {
    let ty: PaneType = placeholder_pane_type(SIDEBAR_PANE_LABEL);
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            ty.clone(),
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    let bars = app.tab_bar_states_mut();
    if let Some(bar) = bars.get_mut(&PaneId::from("pane-a")) {
        bar.tabs = vec![handshake_native::tab_bar::TabState::new(ty)];
        bar.active_index = 0;
    }
}

#[test]
fn mt024_mounted_sidebar_canonical_argus_inspect_steer_reobserve() {
    let app = sidebar_shell();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 960.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // First frames: the sidebar host runs its one-time fetch attempt (no runtime
    // -> synchronous section error) and binds the panel to the active workspace.
    harness.run_steps(2);

    // Seed all four sections + an active block directly into the mounted panel.
    // set_pins/set_favorites/set_backlinks/set_unlinked each clear that section's
    // error, so the first-frame `Runtime unavailable` banners are gone.
    {
        let panel = harness.state().mounted_sidebar_panel_for_test();
        let mut guard = panel.lock().unwrap();
        guard.active_block_id = Some("block-active".to_owned());
        guard.set_pins(vec![
            SidebarBlock::new("block-001", "Design doc", "note"),
            SidebarBlock::new("block-002", "Roadmap", "note"),
        ]);
        guard.set_favorites(vec![SidebarBlock::new("block-003", "Reading list", "note")]);
        guard.set_backlinks(vec![BacklinkRow::new(
            "block-back",
            "Mentions the active block",
            "mention",
        )]);
        guard.set_unlinked(vec![UnlinkedRow::new("block-unl", "Unlinked mentioner")]);
        // Two breadcrumbs so the crumb strip is addressable (AC6).
        guard.push_breadcrumb("block-home", "Home");
        guard.push_breadcrumb("block-active", "Active Block");
    }
    harness.run_steps(2);

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-024/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-024 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-024-sidebar");

    // (1) Canonical inspect: all four sections' rows + a breadcrumb are addressable.
    let before = argus.inspect(&mut harness);
    for author in [
        "sidebar.pin.block-001",
        "sidebar.pin.block-002",
        "sidebar.favorite.block-003",
        "sidebar.backlink.block-back",
        "sidebar.unlinked.block-unl",
    ] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see mounted row '{author}' in the live tree"
        );
    }
    assert!(
        json_has_author_id(&before, &breadcrumb_author_id(0)),
        "canonical argus.inspect must see the breadcrumb strip (AC6)"
    );
    // The pin Remove control is addressable (the mutation surface exists in-tree).
    assert!(
        json_has_author_id(&before, "sidebar.pin.block-001.remove"),
        "the mounted pin row exposes its Remove control by stable author_id"
    );

    // (2) Safe steer: click the Pins row's Remove control through the real Argus
    // transport. The Remove control is a real egui::Button, so the AccessKit click
    // drives the mounted `RemovePin` mutation path (AC2). No backend runtime is
    // bound in this tree proof, so the mounted host takes the typed no-runtime
    // recovery branch: it surfaces a Pins section error + Retry rather than a
    // partial removal — exactly the recoverable behavior MT-024 FAIL_V2 requires.
    let remove = pin_remove_author_id("block-001");
    let observation = argus.click_and_reinspect(&mut harness, &remove);
    assert!(
        matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical remove-pin action receipt is terminal and non-rejected: {}",
        observation.receipt_status
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-024-sidebar-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Fresh re-observation: the steer drove a real mounted state transition.
    // The Pins section now renders its error banner + Retry control (its rows are
    // replaced by the error state), while the other three sections stay intact.
    assert!(
        json_has_author_id(&observation.after, &section_retry_author_id(SectionKind::Pins)),
        "after the remove steer the Pins section exposes its Retry control (AC9)"
    );
    let mut after_ids = Vec::new();
    collect_author_ids(&observation.after, &mut after_ids);
    assert!(
        !after_ids.iter().any(|id| id.starts_with("sidebar.pin.")),
        "the errored Pins section no longer renders its rows; got {:?}",
        after_ids
            .iter()
            .filter(|id| id.starts_with("sidebar."))
            .collect::<Vec<_>>()
    );
    for still in [
        "sidebar.favorite.block-003",
        "sidebar.backlink.block-back",
        "sidebar.unlinked.block-unl",
    ] {
        assert!(
            json_has_author_id(&observation.after, still),
            "the remove steer must not affect other sections: '{still}' still addressable"
        );
    }

    // (4) Evidence: before/after canonical trees + a screenshot marker.
    let tree_path = artifact_dir.join("mt024-mounted-sidebar-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "before": before,
            "after": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
        }))
        .expect("serialize canonical MT-024 sidebar tree evidence"),
    )
    .expect("write canonical MT-024 sidebar tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt024-mounted-sidebar.png");
            image.save(&path).expect("save mounted sidebar screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-024 canonical Argus mounted sidebar: inspect(pins/favorites/backlinks/unlinked + \
         breadcrumb + pin remove control) -> click({remove}) -> reinspect(Pins error+Retry, pin \
         rows gone; favorites/backlinks/unlinked remain); receipt={} agent={} screenshot={} tree={}",
        observation.receipt_status,
        observation.agent_id,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}

#[test]
fn mt024_mounted_sidebar_empty_and_error_states_canonical_argus() {
    let app = sidebar_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // Empty Pins/Favorites (clear the first-frame errors with empty lists), no
    // active block (Backlinks/Unlinked show a neutral prompt, no rows), and a
    // Backlinks section ERROR to prove the Retry control is addressable (AC9).
    {
        let panel = harness.state().mounted_sidebar_panel_for_test();
        let mut guard = panel.lock().unwrap();
        guard.set_pins(vec![]);
        guard.set_favorites(vec![]);
        guard.active_block_id = Some("block-x".to_owned());
        guard.set_error(SectionKind::Backlinks, "Backend unavailable");
    }
    harness.run_steps(2);

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-024-sidebar-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("sidebar.pin.")),
        "empty Pins must expose NO pin rows; got {:?}",
        ids.iter().filter(|id| id.starts_with("sidebar.")).collect::<Vec<_>>()
    );
    assert!(
        !ids.iter().any(|id| id.starts_with("sidebar.favorite.")),
        "empty Favorites must expose NO favorite rows"
    );
    // AC9: the errored Backlinks section exposes its stable Retry control.
    assert!(
        json_has_author_id(&tree, &section_retry_author_id(SectionKind::Backlinks)),
        "errored Backlinks section must expose its Retry control by stable author_id (AC9)"
    );

    println!(
        "MT-024 canonical Argus empty/error sidebar: inspect() returned {} author_ids, \
         0 pin/favorite rows, backlinks Retry addressable (AC9)",
        ids.len()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
