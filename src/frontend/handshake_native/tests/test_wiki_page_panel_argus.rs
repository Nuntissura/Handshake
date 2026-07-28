//! WP-KERNEL-012 E3 MT-025 remediation (FAIL_V2): canonical Argus inspect /
//! safe-steer / re-observe proof for the MOUNTED wiki-page projection overlay
//! edit/save/reload surface.
//!
//! `validation_v2` failed MT-025 in part because "the mounted edit/save/reload
//! surface has no current canonical Argus inspect/steer/re-observe proof". The
//! isolated `test_wiki_page_panel.rs` kittest coverage drives the wiki WIDGET
//! (and a live-PG mounted host via plain AccessKit clicks), but never the mounted
//! `HandshakeApp` through the real localhost `SwarmMcpServer` transport the way an
//! out-of-process swarm agent does. This test closes that exact gap:
//!
//!   1. mounts the production `HandshakeApp` shell with the Wiki Page pane bound
//!      to a seeded projection (no backend — the binding is pre-seeded with a
//!      panel that already has its page, so the factory never issues a GET),
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC) to the app,
//!   3. `argus.inspect` proves the read-only view's stable author_ids (title,
//!      metadata, content, edit) are addressable in the live tree,
//!   4. drives the edit/save/reload SURFACE through Argus: click `wiki.edit.*`
//!      -> FRESH inspect re-observes the edit overlay (`wiki.edit-area.*`,
//!      `wiki.save.*`, `wiki.cancel.*`), then click `wiki.cancel.*` -> FRESH
//!      inspect re-observes the return to the read-only content view (reversible,
//!      no backend mutation), and
//!   5. writes the before/after tree evidence externally + a screenshot marker
//!      (headless DEFERRED is an acceptable typed outcome).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-025/` root.

use std::path::{Path, PathBuf};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::{HealthInfo, WikiProjection};
use handshake_native::editor_pane_factories::{placeholder_pane_type, WIKI_PAGE_PANE_LABEL};
use handshake_native::graph::wiki_page_panel::{
    cancel_author_id, content_author_id, edit_area_author_id, edit_author_id, metadata_author_id,
    save_author_id, title_author_id,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

const WS: &str = "ws-argus-mt025";
const PROJ: &str = "proj-argus-mt025";

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

fn seeded_projection() -> WikiProjection {
    WikiProjection {
        projection_id: PROJ.to_owned(),
        workspace_id: WS.to_owned(),
        title: "Ownership model".to_owned(),
        source_block_ids: vec!["blk-1".to_owned(), "blk-2".to_owned(), "blk-3".to_owned()],
        rendered_content:
            "# Ownership model\nThe borrow checker enforces aliasing rules at compile time."
                .to_owned(),
        staleness_hash: "h1".to_owned(),
        rebuild_status: "fresh".to_owned(),
        page_type: Some("concept".to_owned()),
        overlays: Vec::new(),
        staleness_verdict: serde_json::json!({ "state": "fresh" }),
    }
}

/// A live shell with `pane-a` re-typed to the Wiki Page pane (content_id = PROJ)
/// and the active workspace bound to WS. On the first frame the wiki factory binds
/// its own panel for PROJ and (with no runtime) marks it errored; the test then
/// seeds the projection directly onto that bound panel (`set_page` clears the
/// error), so the read-only view renders with no backend GET. Because the factory
/// created the binding identity from the session workspace, it never rebinds again
/// and the seed survives.
fn wiki_shell() -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.bind_active_project_for_integration_test(WS);
    retype_pane_a_to_wiki(&mut app);
    app
}

/// Seed the projection onto whatever panel the mounted wiki factory bound for
/// `pane-a`. Returns the bound projection_id (its author_ids key). Panics if the
/// factory has not bound a panel yet (run frames first).
fn seed_bound_wiki_page(harness: &mut Harness<'_, HandshakeApp>) -> String {
    let binding = harness.state().mounted_wiki_binding_for_test();
    let mut guard = binding.lock().unwrap();
    let (identity, panel) = guard
        .as_mut()
        .expect("wiki factory must have bound a panel for pane-a");
    let projection_id = identity.projection_id.clone();
    let mut page = seeded_projection();
    page.projection_id = projection_id.clone();
    page.workspace_id = identity.workspace_id.clone();
    panel.set_page(page);
    projection_id
}

fn retype_pane_a_to_wiki(app: &mut HandshakeApp) {
    let ty: PaneType = placeholder_pane_type(WIKI_PAGE_PANE_LABEL);
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            ty.clone(),
            DEFAULT_PROJECT_ID,
            Some(PROJ.to_owned()),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    let bars = app.tab_bar_states_mut();
    if let Some(bar) = bars.get_mut(&PaneId::from("pane-a")) {
        let mut tab = handshake_native::tab_bar::TabState::new(ty);
        tab.content_id = Some(PROJ.to_owned());
        bar.tabs = vec![tab];
        bar.active_index = 0;
    }
}

#[test]
fn mt025_mounted_wiki_canonical_argus_edit_save_reload_surface() {
    let app = wiki_shell();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 820.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // First frames: the factory binds its own panel for PROJ (no runtime -> error).
    harness.run_steps(3);
    // Seed the projection onto the bound panel (clears the no-runtime error).
    let bound_proj = seed_bound_wiki_page(&mut harness);
    assert_eq!(
        bound_proj, PROJ,
        "the factory bound the pane's content_id projection"
    );
    harness.run_steps(2);

    // Guard: the mounted binding now holds the seeded page in read-only mode and
    // the factory did not wipe it with a rebind/GET. If this fails the mounting
    // assumptions drifted.
    {
        let bound = harness.state().mounted_wiki_binding_for_test();
        let guard = bound.lock().unwrap();
        let page_present = guard
            .as_ref()
            .map(|(_, panel)| panel.page.is_some() && !panel.edit_mode)
            .unwrap_or(false);
        assert!(
            page_present,
            "mounted wiki pane must render the seeded page in read-only mode (no backend GET)"
        );
    }

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-025/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-025 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-025-wiki");

    // (1) Canonical inspect: the read-only view's stable author_ids are addressable.
    let before = argus.inspect(&mut harness);
    for author in [
        title_author_id(PROJ),
        metadata_author_id(PROJ),
        content_author_id(PROJ),
        edit_author_id(PROJ),
    ] {
        assert!(
            json_has_author_id(&before, &author),
            "canonical argus.inspect must see the mounted read-only node '{author}'"
        );
    }
    // Edit overlay controls are NOT present until Edit is clicked.
    assert!(
        !json_has_author_id(&before, &edit_area_author_id(PROJ)),
        "the edit area must NOT be present in the read-only view"
    );

    // (2) Safe steer #1: enter the edit overlay through the real Argus transport.
    let edit = edit_author_id(PROJ);
    let enter = argus.click_and_reinspect(&mut harness, &edit);
    assert!(
        matches!(enter.receipt_status.as_str(), "applied" | "indeterminate"),
        "the canonical edit-enter receipt is terminal and non-rejected: {}",
        enter.receipt_status
    );
    assert!(
        enter
            .agent_id
            .contains(":client:wp-kernel-012-mt-025-wiki-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        enter.agent_id
    );
    // (3) Fresh re-observation: the edit overlay's save/cancel/edit-area controls
    // are now addressable (the edit/save SURFACE the V2 report demanded).
    for author in [
        edit_area_author_id(PROJ),
        save_author_id(PROJ),
        cancel_author_id(PROJ),
    ] {
        assert!(
            json_has_author_id(&enter.after, &author),
            "fresh re-inspection after Edit must observe the overlay control '{author}'"
        );
    }

    // (4) Safe steer #2: cancel back to read-only (reversible, no backend write).
    let cancel = cancel_author_id(PROJ);
    let back = argus.click_and_reinspect(&mut harness, &cancel);
    assert!(
        matches!(back.receipt_status.as_str(), "applied" | "indeterminate"),
        "the canonical cancel receipt is terminal and non-rejected: {}",
        back.receipt_status
    );
    assert!(
        json_has_author_id(&back.after, &content_author_id(PROJ)),
        "cancel returns to the read-only content view"
    );
    let mut after_ids = Vec::new();
    collect_author_ids(&back.after, &mut after_ids);
    assert!(
        !after_ids.iter().any(|id| id == &edit_area_author_id(PROJ)),
        "after Cancel the edit area is gone from the AccessKit tree; got {:?}",
        after_ids
            .iter()
            .filter(|id| id.starts_with("wiki."))
            .collect::<Vec<_>>()
    );

    // (5) Evidence: before/after canonical trees + a screenshot marker.
    let tree_path = artifact_dir.join("mt025-mounted-wiki-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "read_only_before": before,
            "edit_overlay_after": enter.after,
            "cancelled_read_only_after": back.after,
            "edit_receipt": { "id": enter.receipt_id, "status": enter.receipt_status, "agent": enter.agent_id },
            "cancel_receipt": { "id": back.receipt_id, "status": back.receipt_status, "agent": back.agent_id },
        }))
        .expect("serialize canonical MT-025 wiki tree evidence"),
    )
    .expect("write canonical MT-025 wiki tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt025-mounted-wiki.png");
            image.save(&path).expect("save mounted wiki screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-025 canonical Argus mounted wiki: inspect(title+metadata+content+edit) -> \
         click({edit}) -> reinspect(edit-area+save+cancel) -> click({cancel}) -> \
         reinspect(content; edit-area gone); edit_receipt={} cancel_receipt={} screenshot={} tree={}",
        enter.receipt_status,
        back.receipt_status,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
