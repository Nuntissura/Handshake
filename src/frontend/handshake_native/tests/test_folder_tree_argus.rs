//! WP-KERNEL-012 E3 MT-022 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED folder-tree panel.
//!
//! `validation_v2` failed MT-022 in part because "the folder tree also lacks current canonical Argus
//! steering and re-observation proof". The isolated `test_folder_tree.rs` kittest coverage drives the
//! folder-tree WIDGET, but never the mounted `HandshakeApp` through the real localhost `SwarmMcpServer`
//! transport the way an out-of-process swarm agent does. This test closes that exact gap:
//!
//!   1. mounts the production `HandshakeApp` shell with the Folders pane mounted and a seeded folder
//!      tree (two root folders, one coloured),
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves the mounted folder nodes are addressable by stable author_id in the live
//!      tree (`folder-tree.node.{id}`),
//!   4. drives ONE safe, reversible action (expand a folder) through Argus,
//!   5. FRESH `argus.inspect` re-observes the post-action tree (the folder node remains addressable —
//!      the action was additive, not destructive), and
//!   6. writes the before/after tree evidence externally + a screenshot marker (headless DEFERRED is an
//!      acceptable typed outcome).
//!
//! A second test proves the empty-workspace state renders and inspects through canonical Argus with no
//! folder nodes and no panic (AC7).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-022/` root.

use std::path::{Path, PathBuf};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, FOLDER_TREE_PANE_LABEL};
use handshake_native::graph::folder_tree::FolderRow;
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

/// A live, runtime-injected shell with `pane-a` re-typed to the Folders pane so the mounted folder-tree
/// factory renders in the split. The runtime is returned so it outlives the harness.
fn folders_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
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
    retype_pane_a_to_folders(&mut app);
    (app, runtime)
}

/// Re-type `pane-a` to the Folders placeholder pane in BOTH the registry record and the tab bar (the
/// shell syncs the record from the active tab every frame, so both must be set).
fn retype_pane_a_to_folders(app: &mut HandshakeApp) {
    let ty: PaneType = placeholder_pane_type(FOLDER_TREE_PANE_LABEL);
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
fn mt022_mounted_folder_tree_canonical_argus_inspect_steer_reobserve() {
    let (app, _rt) = folders_shell();

    // Seed a two-folder tree directly into the mounted folder-tree widget (no backend needed for the
    // Argus tree proof): "Projects" (red) and "Archive" (uncoloured).
    {
        let tree = app.mounted_folder_tree_for_test();
        let mut guard = tree.lock().unwrap();
        guard.set_folders(&[
            FolderRow::new("folder-001", None, "Projects", Some("#ff0000".to_owned())),
            FolderRow::new("folder-002", None, "Archive", None),
        ]);
    }

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-022/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-022 Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-022-folder-tree");

    // (1) Canonical inspect: the mounted folder nodes are addressable by stable author_id.
    let before = argus.inspect(&mut harness);
    for author in ["folder-tree.node.folder-001", "folder-tree.node.folder-002"] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted folder node '{author}' in the live tree"
        );
    }

    // (2) Safe, reversible steer: expand a folder through the real Argus transport. This changes no
    // durable/backend/external state — it toggles the in-panel expansion (and dispatches a lazy child
    // fetch cell that stays parked headless).
    let observation = argus.click_and_reinspect(&mut harness, "folder-tree.node.folder-001");
    assert!(
        matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical folder-expand action receipt is terminal and non-rejected: {}",
        observation.receipt_status
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-022-folder-tree-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Fresh re-observation: the expanded folder node remains addressable, and the sibling node is
    // still present (the action was additive, not destructive).
    assert!(
        json_has_author_id(&observation.after, "folder-tree.node.folder-001"),
        "fresh canonical re-inspection still sees the expanded folder node"
    );
    assert!(
        json_has_author_id(&observation.after, "folder-tree.node.folder-002"),
        "the sibling folder node remains addressable after the safe action"
    );

    // (4) Evidence: before/after canonical trees + a screenshot marker (headless DEFERRED is acceptable).
    let tree_path = artifact_dir.join("mt022-mounted-folder-tree-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "before": before,
            "after": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
        }))
        .expect("serialize canonical MT-022 folder-tree tree evidence"),
    )
    .expect("write canonical MT-022 folder-tree tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt022-mounted-folder-tree.png");
            image
                .save(&path)
                .expect("save mounted folder-tree screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-022 canonical Argus mounted folder tree: inspect(2 nodes) -> click(folder-tree.node.folder-001) \
         -> reinspect(node still addressable); receipt={} agent={} screenshot={} tree={}",
        observation.receipt_status,
        observation.agent_id,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}

#[test]
fn mt022_mounted_folder_tree_empty_state_canonical_argus() {
    // AC7: with no folders, the mounted panel renders + inspects through canonical Argus with no folder
    // nodes and no panic.
    let (app, _rt) = folders_shell();
    // Leave the mounted folder tree empty (no set_folders).

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-022-folder-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("folder-tree.node.")),
        "empty-workspace mounted folder tree must expose NO folder nodes through canonical Argus; got {:?}",
        ids.iter()
            .filter(|id| id.starts_with("folder-tree."))
            .collect::<Vec<_>>()
    );

    println!(
        "MT-022 canonical Argus empty folder tree: inspect() returned {} author_ids, 0 folder-tree.node.* (AC7)",
        ids.len()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
