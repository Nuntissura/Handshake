//! WP-KERNEL-012 E3 MT-023 V4 remediation: canonical Argus inspect / safe-steer / authoritative
//! re-observe proof for the MOUNTED tags panel against real SurrealDB.
//!
//! `validation_v4` observed visible tag-list -> hub navigation but rejected the indeterminate click
//! receipt because the post-state was not causally bound to the exact authoritative hub-membership
//! request. This test drives the mounted `HandshakeApp` through the real localhost `SwarmMcpServer`
//! transport the way an out-of-process swarm agent does and closes that exact gap:
//!
//!   1. creates an isolated real-SurrealDB workspace and seeds three tag hubs plus one real tag edge,
//!      then mounts the production `HandshakeApp` Tags pane against the current backend,
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves the mounted search box (`tags.search`) and tag rows (`tags.row.{id}`) are
//!      addressable by stable author_id in the live tree,
//!   4. drives ONE safe, reversible action (select a tag row) through Argus,
//!   5. waits for the exact authoritative hub-membership request and requires an Applied receipt whose
//!      observer semantic binds the source/destination hub plus workspace/completion generations,
//!   6. FRESH `argus.inspect` binds that receipt to the loaded hub title + persisted member while list
//!      rows are absent, and
//!   7. proves workspace/backend cleanup before writing the before/after tree and required GPU screenshot.
//!
//! A second test proves the empty-workspace state renders and inspects through canonical Argus with no
//! tag rows and no panic (AC8).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-023/` root.

use std::path::{Path, PathBuf};

#[cfg(feature = "integration")]
mod interconnect_support;

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
use handshake_native::graph::tags_panel::{
    hub_member_author_id, hub_title_author_id, tag_row_author_id, TAG_NAVIGATION_OBSERVER_AUTHOR_ID,
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
    retype_pane_a_to_tags(&mut app, DEFAULT_PROJECT_ID);
    (app, runtime)
}

/// Re-type `pane-a` to the Tags placeholder pane in BOTH the registry record and the tab bar (the shell
/// syncs the record from the active tab every frame, so both must be set).
fn retype_pane_a_to_tags(app: &mut HandshakeApp, project_id: &str) {
    let ty: PaneType = placeholder_pane_type(TAGS_PANE_LABEL);
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            ty.clone(),
            project_id,
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

#[cfg(feature = "integration")]
struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204 | 404),
            "managed-SurrealDB workspace cleanup returned HTTP {status}"
        );
        self.cleaned = true;
    }
}

#[cfg(feature = "integration")]
impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

fn terminal_navigation_is_causal(
    tree: &serde_json::Value,
    receipt_id: u64,
    workspace_id: &str,
    workspace_generation: u64,
    completion_generation: u64,
    hub_id: &str,
    member_id: &str,
) -> bool {
    let receipt_applied = tree["action_receipts"].as_array().is_some_and(|receipts| {
        receipts.iter().any(|receipt| {
            receipt["receipt_id"].as_u64() == Some(receipt_id)
                && receipt["status"].as_str() == Some("applied")
        })
    });
    let Some(observer_value) = json_node_by_author_id(tree, TAG_NAVIGATION_OBSERVER_AUTHOR_ID)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    let Ok(observer): Result<serde_json::Value, _> = serde_json::from_str(observer_value) else {
        return false;
    };
    let Some(semantic_raw) = observer["semantic_value"].as_str() else {
        return false;
    };
    let Ok(semantic): Result<serde_json::Value, _> = serde_json::from_str(semantic_raw) else {
        return false;
    };
    let expected_row = tag_row_author_id(hub_id);

    receipt_applied
        && observer["state"].as_str() == Some("applied")
        && observer["pending_target"].as_str() == Some(expected_row.as_str())
        && semantic["source_tag_id"].as_str() == Some(hub_id)
        && semantic["destination_tag_hub_id"].as_str() == Some(hub_id)
        && semantic["workspace_id"].as_str() == Some(workspace_id)
        && semantic["workspace_generation"].as_u64() == Some(workspace_generation)
        && semantic["completion_generation"].as_u64() == Some(completion_generation)
        && observer["generation"].as_u64() == Some(completion_generation)
        && semantic["completion_kind"].as_str()
            == Some("authoritative-hub-membership-query-complete")
        && json_has_author_id(tree, &hub_title_author_id(hub_id))
        && json_has_author_id(tree, &hub_member_author_id(member_id))
        && !tree.to_string().contains("Loading members")
}

#[test]
#[cfg(feature = "integration")]
fn mt023_mounted_tags_panel_canonical_argus_inspect_steer_reobserve() {
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-023/canonical-argus");
    let tree_path = artifact_dir.join("mt023-mounted-tags-panel-argus.json");
    let screenshot_path = artifact_dir.join("mt023-mounted-tags-panel.png");
    for owned_path in [&tree_path, &screenshot_path] {
        match std::fs::remove_file(owned_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!(
                "remove stale MT-023 owned artifact {} before proof: {error}",
                owned_path.display()
            ),
        }
        assert!(
            !owned_path.exists(),
            "MT-023 proof starts without a stale owned artifact at {}",
            owned_path.display()
        );
    }
    let proof_started = std::time::SystemTime::now();

    let mut live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt023-argus-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    );
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };
    let seed_block = |content_type: &str, title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": content_type, "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("block create returns block_id")
            .to_owned()
    };
    let rust_hub = seed_block("tag_hub", "rust");
    let python_hub = seed_block("tag_hub", "python");
    let design_hub = seed_block("tag_hub", "design");
    let rust_member = seed_block("note", "MT-023 authoritative Rust member");
    live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": rust_member,
            "target_block_id": rust_hub,
            "edge_type": "tag",
            "created_by": "user"
        }),
    );
    drop(seed_block);

    let (mut app, _rt) = tags_shell();
    app.bind_active_project_for_integration_test(&workspace_id);
    app.set_tags_backend_base_url_for_test(live.base.clone());
    retype_pane_a_to_tags(&mut app, &workspace_id);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    for _ in 0..200 {
        harness.run_steps(1);
        let loaded = harness
            .state()
            .mounted_tags_panel_for_test()
            .lock()
            .map(|panel| {
                !panel.loading
                    && panel.error.is_none()
                    && panel.tags.len() == 3
                    && panel.tags.iter().all(|tag| tag.member_count.is_some())
            })
            .unwrap_or(false);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(
        harness
            .state()
            .mounted_tags_panel_for_test()
            .lock()
            .unwrap()
            .tags
            .len(),
        3,
        "real SurrealDB tag list settles with all three seeded tag hubs"
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-023-tags-panel");

    // (1) Canonical inspect: the mounted search box + tag rows are addressable by stable author_id.
    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, "tags.search"),
        "canonical argus.inspect must see the mounted tags search box 'tags.search'"
    );
    for author in [
        tag_row_author_id(&rust_hub),
        tag_row_author_id(&python_hub),
        tag_row_author_id(&design_hub),
    ] {
        assert!(
            json_has_author_id(&before, &author),
            "canonical argus.inspect must see the mounted tag row '{author}' in the live tree"
        );
    }
    let workspace_generation = harness.state().tags_workspace_epoch_for_test();
    let baseline_observer_value =
        json_node_by_author_id(&before, TAG_NAVIGATION_OBSERVER_AUTHOR_ID)
            .and_then(|node| node.get("value"))
            .and_then(serde_json::Value::as_str)
            .expect("pre-click tree carries the durable tag navigation observer");
    let baseline_observer: serde_json::Value = serde_json::from_str(baseline_observer_value)
        .expect("pre-click tag navigation observer is strict JSON");
    assert_eq!(baseline_observer["state"].as_str(), Some("ready"));
    let completion_generation = baseline_observer["generation"]
        .as_u64()
        .and_then(|generation| generation.checked_add(1))
        .expect("tag navigation observer generation can advance exactly once");

    // (2) Safe, reversible steer: select a tag row through the real Argus transport. This fires the
    // in-app open/filter navigation callback — no durable/backend/external mutation.
    let rust_row = tag_row_author_id(&rust_hub);
    let observation = argus.click_and_reinspect(&mut harness, &rust_row);
    assert_eq!(
        observation.receipt_status, "applied",
        "the authoritative tag navigation receipt must never be indeterminate"
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-023-tags-panel-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // The main frame paints before `drive_remediation_mounts` drains the completed membership request at
    // the end of `HandshakeApp::ui`; the following side-effect-free Argus inspect therefore sees updated
    // state even though `Harness::render` still retains that main frame's loading `FullOutput`. Repaint
    // the main harness once after Applied so the terminal tree and GPU screenshot prove the same loaded
    // hub frame.
    harness.run_steps(1);

    // (3) Fresh re-observation: selecting a tag NAVIGATES the mounted pane from the tag LIST to that
    // tag's HUB page. The post-action tree therefore carries the hub surface for the exact tag that was
    // clicked (`tag-hub.title.tag-rust` + its add-tag control, both emitted before the parked detail
    // fetch resolves), and no longer carries the list rows. This is the real, re-observed state
    // transition — not a no-op click.
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt023-tag-navigation-receipt-plus-loaded-hub-membership",
        serde_json::json!({
            "receipt_id": observation.receipt_id,
            "workspace_id": workspace_id,
            "workspace_generation": workspace_generation,
            "completion_generation": completion_generation,
            "source_tag_id": rust_hub,
            "destination_tag_hub_id": rust_hub,
            "expected_member_id": rust_member,
            "observer_author_id": TAG_NAVIGATION_OBSERVER_AUTHOR_ID,
        }),
        |tree| {
            terminal_navigation_is_causal(
                tree,
                observation.receipt_id,
                &workspace_id,
                workspace_generation,
                completion_generation,
                &rust_hub,
                &rust_member,
            )
        },
    );
    let mut after_ids = Vec::new();
    collect_author_ids(&terminal, &mut after_ids);
    assert!(
        !after_ids.iter().any(|id| id.starts_with("tags.row.")),
        "after navigating to the hub page the tag LIST rows are no longer rendered; got {:?}",
        after_ids
            .iter()
            .filter(|id| id.starts_with("tags."))
            .collect::<Vec<_>>()
    );

    // Capture terminal visual evidence in memory. No PASS-looking durable artifact is published until
    // SurrealDB/backend cleanup, Argus finalization, and local-artifact hygiene all succeed.
    let screenshot = harness
        .render()
        .expect("MT-023 V4 requires GPU rendering for the terminal hub screenshot");
    cleanup.assert_cleaned();
    drop(cleanup);
    live.assert_cleanup();
    argus.finish();
    assert_no_local_artifact_dir();

    // (4) Publish before/after canonical trees + the required GPU-rendered terminal screenshot last.
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-023 Argus artifact dir");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "proof_run_id": unique,
            "before": before,
            "after": terminal,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
        }))
        .expect("serialize canonical MT-023 tags-panel tree evidence"),
    )
    .expect("write canonical MT-023 tags-panel tree evidence externally");
    screenshot
        .save(&screenshot_path)
        .expect("save mounted tags-panel screenshot");
    let published_tree: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&tree_path).expect("read freshly published MT-023 tree evidence"),
    )
    .expect("freshly published MT-023 tree evidence is valid JSON");
    assert_eq!(
        published_tree["proof_run_id"].as_str(),
        Some(unique.as_str()),
        "published tree belongs to this exact proof run"
    );
    assert_eq!(
        published_tree["receipt_id"].as_u64(),
        Some(observation.receipt_id),
        "published tree carries this exact action receipt"
    );
    let screenshot_bytes =
        std::fs::read(&screenshot_path).expect("read freshly published MT-023 screenshot");
    assert!(
        screenshot_bytes.starts_with(b"\x89PNG\r\n\x1a\n") && screenshot_bytes.len() > 8,
        "published screenshot is a non-empty PNG from this proof run"
    );
    for owned_path in [&tree_path, &screenshot_path] {
        let modified = std::fs::metadata(owned_path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or_else(|error| {
                panic!(
                    "read publication timestamp for {}: {error}",
                    owned_path.display()
                )
            });
        assert!(
            modified >= proof_started,
            "owned artifact {} was freshly published after proof start",
            owned_path.display()
        );
    }
    println!(
        "MT-023 canonical Argus mounted tags panel: inspect(tags.search + 3 tag rows) -> \
         click({}) -> reinspect(loaded title={} + member={}; list rows gone); \
         receipt={} agent={} screenshot=CAPTURED {} tree={}",
        rust_row,
        hub_title_author_id(&rust_hub),
        hub_member_author_id(&rust_member),
        observation.receipt_status,
        observation.agent_id,
        screenshot_path.display(),
        tree_path.display()
    );
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
