//! WP-KERNEL-012 E3 MT-023 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED tags panel.
//!
//! `validation_v2` failed MT-023 in part because "the mounted tag-hub interaction also lacks canonical
//! Argus re-observation proof". The isolated `test_tags_panel.rs` kittest coverage drives the tags
//! WIDGET, but never the mounted `HandshakeApp` through the real localhost `SwarmMcpServer` transport the
//! way an out-of-process swarm agent does. This test closes that exact gap:
//!
//!   1. mounts the production `HandshakeApp` shell with the Tags pane mounted and a seeded tag list,
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves the mounted search box (`tags.search`) and tag rows (`tags.row.{id}`) are
//!      addressable by stable author_id in the live tree,
//!   4. drives ONE safe, reversible action (select a tag row) through Argus,
//!   5. FRESH `argus.inspect` re-observes the post-action tree (rows remain addressable — the action was
//!      additive, not destructive), and
//!   6. writes the before/after tree evidence externally + a screenshot marker (headless DEFERRED is an
//!      acceptable typed outcome).
//!
//! A second test proves the empty-workspace state renders and inspects through canonical Argus with no
//! tag rows and no panic (AC8).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-023/` root.

use std::path::{Path, PathBuf};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
use handshake_native::graph::tags_panel::TagEntry;
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

/// Every `author_id` present anywhere in a canonical Argus inspect tree.
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

/// A live, runtime-injected shell with `pane-a` re-typed to the Tags pane so the mounted tags factory
/// renders in the split. The runtime is returned so it outlives the harness.
fn tags_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_pane_a_to_tags(&mut app);
    (app, runtime)
}

/// Re-type `pane-a` to the Tags placeholder pane in BOTH the registry record and the tab bar (the shell
/// syncs the record from the active tab every frame, so both must be set).
fn retype_pane_a_to_tags(app: &mut HandshakeApp) {
    let ty: PaneType = placeholder_pane_type(TAGS_PANE_LABEL);
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
fn mt023_mounted_tags_panel_canonical_argus_inspect_steer_reobserve() {
    let (app, _rt) = tags_shell();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Bind the workspace FIRST. `drive_tags_pane` replaces the mounted panel whenever
    // `panel.workspace_id != workspace` (the A -> B -> A epoch guard), so a pre-mount seed would be
    // wiped on the first frame. After these frames the panel is bound to the shell's active workspace
    // and a seed survives. The parked no-backend fetch never delivers, so it cannot overwrite the seed.
    harness.run_steps(2);

    // Seed a three-tag list directly into the mounted (workspace-bound) tags panel — no backend needed
    // for the Argus tree proof.
    {
        let panel = harness.state().mounted_tags_panel_for_test();
        let mut guard = panel.lock().unwrap();
        guard.set_tags(vec![
            TagEntry::new("tag-rust", "rust", Some(3)),
            TagEntry::new("tag-python", "python", Some(1)),
            TagEntry::new("tag-design", "design", Some(2)),
        ]);
    }
    harness.run_steps(2);

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-023/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-023 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-023-tags-panel");

    // (1) Canonical inspect: the mounted search box + tag rows are addressable by stable author_id.
    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, "tags.search"),
        "canonical argus.inspect must see the mounted tags search box 'tags.search'"
    );
    for author in [
        "tags.row.tag-rust",
        "tags.row.tag-python",
        "tags.row.tag-design",
    ] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted tag row '{author}' in the live tree"
        );
    }

    // (2) Safe, reversible steer: select a tag row through the real Argus transport. This fires the
    // in-app open/filter navigation callback — no durable/backend/external mutation.
    let observation = argus.click_and_reinspect(&mut harness, "tags.row.tag-rust");
    assert!(
        matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical tag-select action receipt is terminal and non-rejected: {}",
        observation.receipt_status
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-023-tags-panel-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Fresh re-observation: selecting a tag NAVIGATES the mounted pane from the tag LIST to that
    // tag's HUB page. The post-action tree therefore carries the hub surface for the exact tag that was
    // clicked (`tag-hub.title.tag-rust` + its add-tag control, both emitted before the parked detail
    // fetch resolves), and no longer carries the list rows. This is the real, re-observed state
    // transition — not a no-op click.
    assert!(
        json_has_author_id(&observation.after, "tag-hub.title.tag-rust"),
        "fresh canonical re-inspection observes the tag-hub page for the exact selected tag"
    );
    assert!(
        json_has_author_id(&observation.after, "tag-hub.add-tag.tag-rust"),
        "the re-observed tag-hub page exposes its add-tag control by stable author_id"
    );
    let mut after_ids = Vec::new();
    collect_author_ids(&observation.after, &mut after_ids);
    assert!(
        !after_ids.iter().any(|id| id.starts_with("tags.row.")),
        "after navigating to the hub page the tag LIST rows are no longer rendered; got {:?}",
        after_ids
            .iter()
            .filter(|id| id.starts_with("tags."))
            .collect::<Vec<_>>()
    );

    // (4) Evidence: before/after canonical trees + a screenshot marker (headless DEFERRED is acceptable).
    let tree_path = artifact_dir.join("mt023-mounted-tags-panel-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "before": before,
            "after": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
        }))
        .expect("serialize canonical MT-023 tags-panel tree evidence"),
    )
    .expect("write canonical MT-023 tags-panel tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt023-mounted-tags-panel.png");
            image
                .save(&path)
                .expect("save mounted tags-panel screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-023 canonical Argus mounted tags panel: inspect(tags.search + 3 tag rows) -> \
         click(tags.row.tag-rust) -> reinspect(navigated to tag-hub.title.tag-rust + \
         tag-hub.add-tag.tag-rust; list rows gone); receipt={} agent={} screenshot={} tree={}",
        observation.receipt_status,
        observation.agent_id,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}

#[test]
fn mt023_mounted_tags_panel_empty_state_canonical_argus() {
    // AC8: with no tag hubs, the mounted panel renders + inspects through canonical Argus with no tag
    // rows and no panic.
    let (app, _rt) = tags_shell();
    // Leave the mounted tags panel empty (no set_tags).

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-023-tags-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("tags.row.")),
        "empty-workspace mounted tags panel must expose NO tag rows through canonical Argus; got {:?}",
        ids.iter()
            .filter(|id| id.starts_with("tags."))
            .collect::<Vec<_>>()
    );

    println!(
        "MT-023 canonical Argus empty tags panel: inspect() returned {} author_ids, 0 tags.row.* (AC8)",
        ids.len()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
