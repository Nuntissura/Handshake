//! MT-025 V4 canonical Argus proof for the mounted wiki projection overlay editor.
//!
//! This is deliberately a managed-SurrealDB/current-source product proof, not a direct-seeded panel
//! harness. It creates source Loom blocks and a wiki projection through the production HTTP API, lets
//! `WikiPagePaneMount` load that projection through `LoomWikiClient`, and drives Edit, SetValue, Cancel,
//! Edit, SetValue, and Save through the localhost canonical Argus transport. Save is accepted only after
//! the POST overlay receipt is confirmed by the follow-up GET and the unchanged source projection.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};
use sha2::{Digest, Sha256};

#[cfg(feature = "integration")]
#[path = "backend_proof_support/mod.rs"]
mod backend_proof_support;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, WIKI_PAGE_PANE_LABEL};
use handshake_native::graph::wiki_page_panel::{
    action_status_author_id, cancel_author_id, content_author_id, edit_area_author_id,
    edit_author_id, metadata_author_id, overlay_author_id, save_author_id, title_author_id,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let path = Path::new(local);
        assert!(
            !path.exists(),
            "CX-212E: no repo-local '{local}' artifact directory may exist: {}",
            path.display()
        );
    }
}

fn retype_pane_a_to_wiki(app: &mut HandshakeApp, projection_id: &str) {
    let pane_type: PaneType = placeholder_pane_type(WIKI_PAGE_PANE_LABEL);
    {
        let registry = app.pane_registry();
        registry.lock().expect("registry").insert(PaneRecord::new(
            PaneId::from("pane-a"),
            pane_type.clone(),
            DEFAULT_PROJECT_ID,
            Some(projection_id.to_owned()),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    if let Some(bar) = app.tab_bar_states_mut().get_mut(&PaneId::from("pane-a")) {
        let mut tab = handshake_native::tab_bar::TabState::new(pane_type);
        tab.content_id = Some(projection_id.to_owned());
        bar.tabs = vec![tab];
        bar.active_index = 0;
    }
}

#[cfg(feature = "integration")]
fn wait_for_loaded_wiki(harness: &mut Harness<'_, HandshakeApp>, projection_id: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        harness.run_steps(1);
        let loaded = harness
            .state()
            .mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().map(|(identity, panel)| {
                    identity.projection_id == projection_id
                        && panel.page.is_some()
                        && panel.error.is_none()
                })
            })
            .unwrap_or(false);
        if loaded {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "mounted wiki projection did not load from the current-source backend"
        );
        std::thread::yield_now();
    }
}

fn receipt<'a>(snapshot: &'a serde_json::Value, receipt_id: u64) -> &'a serde_json::Value {
    snapshot["action_receipts"]
        .as_array()
        .and_then(|receipts| {
            receipts
                .iter()
                .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
        })
        .expect("fresh Argus inspection contains the action receipt")
}

fn terminal_detail(receipt: &serde_json::Value) -> serde_json::Value {
    let observer: serde_json::Value = serde_json::from_str(
        receipt["observed_value"]
            .as_str()
            .expect("terminal receipt carries the exact observer token"),
    )
    .expect("observer token is JSON");
    serde_json::from_str(
        observer["terminal_detail"]
            .as_str()
            .expect("observer token carries terminal detail"),
    )
    .expect("terminal detail is deterministic JSON")
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> String {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("read proof input {}: {error}", path.display()));
    sha256_bytes(&bytes)
}

fn modified_unix_ms(path: &Path) -> u128 {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .expect("proof artifact mtime is after Unix epoch")
        .as_millis()
}

fn exact_action_terminal(
    tree: &serde_json::Value,
    receipt_id: u64,
    action: &str,
    workspace_id: &str,
    projection_id: &str,
) -> bool {
    let action_receipt = receipt(tree, receipt_id);
    if action_receipt["status"] != "applied" {
        return false;
    }
    let detail = terminal_detail(action_receipt);
    detail["schema"] == "handshake.wiki-action-terminal/v1"
        && detail["outcome"] == "applied"
        && detail["action"] == action
        && detail["workspace_id"] == workspace_id
        && detail["projection_id"] == projection_id
        && detail["pane_generation"].as_u64().is_some()
        && detail["edit_mode_generation"].as_u64().is_some()
        && detail["action_generation"].as_u64().is_some()
        && detail["draft_identity"]
            .as_str()
            .is_some_and(|identity| identity.starts_with("wiki-draft:"))
        && detail["draft_sha256"].as_str().map(str::len) == Some(64)
        && detail["source_content_sha256"].as_str().map(str::len) == Some(64)
        && detail["source_projection_revision"].as_str().is_some()
        && detail["source_staleness_hash"].as_str().is_some()
}

fn exact_value(tree: &serde_json::Value, author_id: &str, value: &str) -> bool {
    json_node_by_author_id(tree, author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        == Some(value)
}

fn write_atomic(path: &Path, bytes: &[u8]) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("atomic evidence path has a UTF-8 file name");
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    std::fs::write(&temporary, bytes)
        .unwrap_or_else(|error| panic!("write temporary proof {}: {error}", temporary.display()));
    std::fs::rename(&temporary, path).unwrap_or_else(|error| {
        panic!(
            "atomically publish proof {} -> {}: {error}",
            temporary.display(),
            path.display()
        )
    });
}

#[test]
#[cfg(feature = "integration")]
fn mt025_mounted_wiki_current_source_pg_gpu_argus_edit_cancel_save_readback() {
    let mut live = backend_proof_support::require_live_backend();
    let run_id = format!("run-{}", uuid::Uuid::new_v4());
    let backend_binding = live.owned_backend_binding_receipt();
    let workspace_id = live.workspace_id.clone();
    let source_title = format!("MT-025 Argus source {}", uuid::Uuid::new_v4());
    let source = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({"content_type": "note", "title": source_title}),
    );
    let source_id = source["block_id"]
        .as_str()
        .expect("created source block has block_id")
        .to_owned();
    let compiled = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/wiki"),
        &serde_json::json!({
            "title": "MT-025 canonical Argus projection",
            "block_ids": [&source_id]
        }),
    );
    let projection_id = compiled["projection_id"]
        .as_str()
        .expect("compiled projection has projection_id")
        .to_owned();
    let projection_path = format!("/workspaces/{workspace_id}/loom/wiki/{projection_id}");
    let overlays_path = format!("{projection_path}/overlays");
    let source_before = live.get_json(&projection_path);
    assert_eq!(source_before["projection_id"], projection_id);
    assert!(source_before["rendered_content"]
        .as_str()
        .is_some_and(|content| content.contains(&source_title)));
    assert_eq!(
        live.get_json(&overlays_path).as_array().map(Vec::len),
        Some(0)
    );

    let runtime = tokio::runtime::Runtime::new().expect("wiki app runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: None,
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.set_wiki_backend_base_url_for_test(live.base.clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    retype_pane_a_to_wiki(&mut app, &projection_id);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 850.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_loaded_wiki(&mut harness, &projection_id);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-025-v4");

    let before = argus.inspect(&mut harness);
    for author in [
        title_author_id(&projection_id),
        metadata_author_id(&projection_id),
        content_author_id(&projection_id),
        edit_author_id(&projection_id),
        action_status_author_id(&projection_id),
    ] {
        assert!(
            json_has_author_id(&before, &author),
            "canonical Argus must see mounted node {author}"
        );
    }

    // Cancel is a first-class no-write terminal action, not an indeterminate local state change.
    let enter_cancel = argus.click_and_reinspect(&mut harness, &edit_author_id(&projection_id));
    assert_eq!(enter_cancel.receipt_status, "applied");
    harness.run_steps(1);
    let edit_cancel_terminal =
        argus.assert_latest_terminal_predicate(&mut harness, "edit-open-before-cancel", |tree| {
            exact_action_terminal(
                tree,
                enter_cancel.receipt_id,
                "edit",
                &workspace_id,
                &projection_id,
            ) && json_has_author_id(tree, &edit_area_author_id(&projection_id))
                && json_has_author_id(tree, &cancel_author_id(&projection_id))
                && json_has_author_id(tree, &save_author_id(&projection_id))
        });
    let cancelled_draft = "MT-025 exact cancelled draft";
    let cancel_value = argus.set_value_and_reinspect(
        &mut harness,
        &edit_area_author_id(&projection_id),
        cancelled_draft,
    );
    let cancel_value_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "cancel-draft-exact-value-visible",
        |tree| {
            matches!(
                receipt(tree, cancel_value.receipt_id)["status"].as_str(),
                Some("applied" | "indeterminate")
            ) && exact_value(tree, &edit_area_author_id(&projection_id), cancelled_draft)
        },
    );
    let cancelled = argus.click_and_reinspect(&mut harness, &cancel_author_id(&projection_id));
    assert_eq!(cancelled.receipt_status, "applied");
    harness.run_steps(1);
    let cancel_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "cancel-discards-draft-without-write",
        |tree| {
            exact_action_terminal(
                tree,
                cancelled.receipt_id,
                "cancel",
                &workspace_id,
                &projection_id,
            ) && !json_has_author_id(tree, &edit_area_author_id(&projection_id))
                && json_has_author_id(tree, &content_author_id(&projection_id))
                && terminal_detail(receipt(tree, cancelled.receipt_id))["write_count"] == 0
                && terminal_detail(receipt(tree, cancelled.receipt_id))["no_write"] == true
                && terminal_detail(receipt(tree, cancelled.receipt_id))["extra"]["draft_discarded"]
                    == true
                && terminal_detail(receipt(tree, cancelled.receipt_id))["extra"]["edit_closed"]
                    == true
        },
    );
    let cancel_detail = terminal_detail(receipt(&cancel_terminal, cancelled.receipt_id));
    assert_eq!(cancel_detail["action"], "cancel");
    assert_eq!(cancel_detail["write_count"], 0);
    assert_eq!(cancel_detail["no_write"], true);
    assert_eq!(cancel_detail["extra"]["draft_discarded"], true);
    assert_eq!(cancel_detail["extra"]["edit_closed"], true);
    assert_eq!(
        live.get_json(&overlays_path).as_array().map(Vec::len),
        Some(0),
        "Cancel performs no canonical write"
    );
    let source_after_cancel = live.get_json(&projection_path);
    for field in ["updated_at", "staleness_hash", "rendered_content"] {
        assert_eq!(
            source_after_cancel[field], source_before[field],
            "Cancel leaves original source field {field} authoritative"
        );
    }

    // Save must terminate Applied only after the POST receipt is found unchanged in a fresh GET.
    harness.run_steps(1);
    let enter_save = argus.click_and_reinspect(&mut harness, &edit_author_id(&projection_id));
    assert_eq!(enter_save.receipt_status, "applied");
    harness.run_steps(1);
    let edit_save_terminal =
        argus.assert_latest_terminal_predicate(&mut harness, "edit-open-before-save", |tree| {
            exact_action_terminal(
                tree,
                enter_save.receipt_id,
                "edit",
                &workspace_id,
                &projection_id,
            ) && json_has_author_id(tree, &edit_area_author_id(&projection_id))
                && json_has_author_id(tree, &cancel_author_id(&projection_id))
                && json_has_author_id(tree, &save_author_id(&projection_id))
        });
    let saved_draft = format!("MT-025 persisted Argus overlay {}", uuid::Uuid::new_v4());
    let save_value = argus.set_value_and_reinspect(
        &mut harness,
        &edit_area_author_id(&projection_id),
        &saved_draft,
    );
    let save_value_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "save-draft-exact-value-visible",
        |tree| {
            matches!(
                receipt(tree, save_value.receipt_id)["status"].as_str(),
                Some("applied" | "indeterminate")
            ) && exact_value(tree, &edit_area_author_id(&projection_id), &saved_draft)
        },
    );
    let saved = argus.click_and_reinspect(&mut harness, &save_author_id(&projection_id));
    assert_eq!(
        saved.receipt_status, "applied",
        "Save is Applied only after canonical persisted readback"
    );
    let overlays = live.get_json(&overlays_path);
    let overlay = overlays
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["annotation"] == saved_draft))
        .expect("fresh backend GET contains the exact saved overlay");
    let overlay_id = overlay["overlay_id"].as_str().expect("overlay id");
    let source_after_save = live.get_json(&projection_path);
    for field in ["updated_at", "staleness_hash", "rendered_content"] {
        assert_eq!(
            source_after_save[field], source_before[field],
            "overlay Save leaves original source field {field} immutable"
        );
    }

    harness.run_steps(1);
    let save_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "save-persisted-readback-and-source-immutable",
        |tree| {
            exact_action_terminal(
                tree,
                saved.receipt_id,
                "save",
                &workspace_id,
                &projection_id,
            ) && json_has_author_id(tree, &content_author_id(&projection_id))
                && json_has_author_id(tree, &overlay_author_id(overlay_id))
                && terminal_detail(receipt(tree, saved.receipt_id))["write_count"] == 1
                && terminal_detail(receipt(tree, saved.receipt_id))["overlay_id"]
                    == overlay["overlay_id"]
                && terminal_detail(receipt(tree, saved.receipt_id))["overlay_readback_revision"]
                    == overlay["updated_at"]
                && terminal_detail(receipt(tree, saved.receipt_id))["extra"]
                    ["persisted_and_read_back"]
                    == true
        },
    );
    let save_receipt = receipt(&save_terminal, saved.receipt_id);
    let save_detail = terminal_detail(save_receipt);
    assert_eq!(save_detail["action"], "save");
    assert_eq!(save_detail["workspace_id"], workspace_id);
    assert_eq!(save_detail["projection_id"], projection_id);
    assert!(save_detail["pane_generation"].as_u64().is_some());
    assert!(save_detail["action_generation"].as_u64().is_some());
    assert!(save_detail["edit_mode_generation"].as_u64().is_some());
    assert_eq!(save_detail["draft_sha256"].as_str().map(str::len), Some(64));
    assert_eq!(
        save_detail["source_projection_revision"],
        source_before["updated_at"]
    );
    assert_eq!(
        save_detail["source_staleness_hash"],
        source_before["staleness_hash"]
    );
    assert_eq!(save_detail["write_count"], 1);
    assert_eq!(save_detail["overlay_id"], overlay["overlay_id"]);
    assert_eq!(save_detail["overlay_created_at"], overlay["created_at"]);
    assert_eq!(
        save_detail["overlay_persisted_revision"],
        overlay["updated_at"]
    );
    assert_eq!(
        save_detail["overlay_readback_revision"],
        overlay["updated_at"]
    );
    assert_eq!(save_detail["extra"]["persisted_and_read_back"], true);

    let source_content_sha256 = sha256_bytes(
        source_before["rendered_content"]
            .as_str()
            .expect("source rendered_content")
            .as_bytes(),
    );
    assert_eq!(save_detail["source_content_sha256"], source_content_sha256);

    let artifact_dir = backend_proof_support::external_artifact_root()
        .join("wp-kernel-012-mt-025")
        .join("canonical-argus-v4-runs")
        .join(&run_id);
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-025 artifact directory");
    let screenshot_path = artifact_dir.join("mt025-wiki-save-readback.png");
    let screenshot_image = harness
        .render()
        .expect("MT-025 canonical proof requires an actual WGPU frame");
    let screenshot_dimensions = [screenshot_image.width(), screenshot_image.height()];
    screenshot_image
        .save(&screenshot_path)
        .expect("save canonical GPU frame");
    let screenshot_bytes = std::fs::read(&screenshot_path).expect("reread canonical GPU PNG");
    assert!(
        screenshot_bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
        "canonical GPU frame has the exact PNG signature"
    );
    let screenshot_sha256 = sha256_bytes(&screenshot_bytes);
    let screenshot_mtime_unix_ms = modified_unix_ms(&screenshot_path);
    let screenshot_outcome = harness
        .last_screenshot_outcome()
        .expect("screenshot harness exposes the durable GPU outcome")
        .clone();
    assert_eq!(screenshot_outcome.status, "CAPTURED");
    assert!(screenshot_outcome.gpu_screenshot_enabled);

    argus.finish();
    live.assert_cleanup();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_files = [
        "src/graph/wiki_page_panel.rs",
        "src/backend_client.rs",
        "src/mcp/action.rs",
        "tests/test_wiki_page_panel_argus.rs",
    ];
    let source_hashes = source_files
        .into_iter()
        .map(|relative| {
            let path = manifest_dir.join(relative);
            (
                relative.to_owned(),
                serde_json::Value::String(sha256_file(&path)),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let evidence = serde_json::json!({
        "schema": "handshake.mt025-canonical-argus-v4-evidence.v1",
        "run_id": run_id,
        "backend_binding": backend_binding,
        "source_files_sha256": source_hashes,
        "source_content_sha256": source_content_sha256,
        "workspace_id": workspace_id,
        "projection_id": projection_id,
        "terminal_trees": {
            "read_only_before": before,
            "edit_before_cancel": edit_cancel_terminal,
            "cancel_draft_value": cancel_value_terminal,
            "cancel": cancel_terminal,
            "edit_before_save": edit_save_terminal,
            "save_draft_value": save_value_terminal,
            "save": save_terminal,
        },
        "receipts": {
            "edit_before_cancel": receipt(&edit_cancel_terminal, enter_cancel.receipt_id),
            "cancel_draft_value": receipt(&cancel_value_terminal, cancel_value.receipt_id),
            "cancel": receipt(&cancel_terminal, cancelled.receipt_id),
            "edit_before_save": receipt(&edit_save_terminal, enter_save.receipt_id),
            "save_draft_value": receipt(&save_value_terminal, save_value.receipt_id),
            "save": save_receipt,
        },
        "cancel_terminal_detail": cancel_detail,
        "save_terminal_detail": save_detail,
        "canonical_overlay": overlay,
        "source_before": source_before,
        "source_after_cancel": source_after_cancel,
        "source_after_save": source_after_save,
        "screenshot": {
            "path": screenshot_path,
            "sha256": screenshot_sha256,
            "mtime_unix_ms": screenshot_mtime_unix_ms,
            "png_signature_hex": "89504e470d0a1a0a",
            "dimensions": screenshot_dimensions,
            "harness_run_id": screenshot_outcome.run_id,
            "outcome_id": screenshot_outcome.outcome_id,
            "scenario_id": screenshot_outcome.scenario_id,
            "status": screenshot_outcome.status,
            "gpu_screenshot_enabled": screenshot_outcome.gpu_screenshot_enabled,
            "harness_frame_path": screenshot_outcome.frame_path,
        },
        "cleanup": {
            "argus_finished": true,
            "workspace_deleted": true,
            "owned_backend_reaped": true,
        },
    });
    let evidence_path = artifact_dir.join("mt025-wiki-terminal-receipts.json");
    write_atomic(
        &evidence_path,
        &serde_json::to_vec_pretty(&evidence).expect("serialize MT-025 canonical evidence"),
    );
    let evidence_bytes = std::fs::read(&evidence_path).expect("reread published MT-025 evidence");
    let evidence_reread: serde_json::Value =
        serde_json::from_slice(&evidence_bytes).expect("published MT-025 evidence is valid JSON");
    assert_eq!(evidence_reread["run_id"], run_id);
    assert_eq!(evidence_reread["screenshot"]["sha256"], screenshot_sha256);
    assert_eq!(evidence_reread["receipts"]["cancel"]["status"], "applied");
    assert_eq!(evidence_reread["receipts"]["save"]["status"], "applied");

    let evidence_sha256 = sha256_bytes(&evidence_bytes);
    let evidence_mtime_unix_ms = modified_unix_ms(&evidence_path);
    let manifest_path = artifact_dir.join("manifest.json");
    let manifest = serde_json::json!({
        "schema": "handshake.mt025-canonical-argus-v4-manifest.v1",
        "run_id": run_id,
        "evidence_path": evidence_path,
        "evidence_sha256": evidence_sha256,
        "evidence_mtime_unix_ms": evidence_mtime_unix_ms,
        "screenshot_path": screenshot_path,
        "screenshot_sha256": screenshot_sha256,
        "screenshot_mtime_unix_ms": screenshot_mtime_unix_ms,
        "png_signature_hex": "89504e470d0a1a0a",
        "backend_binary_sha256": backend_binding["backend_binary_sha256"],
        "published_after_argus_finish_and_pg_cleanup": true,
    });
    write_atomic(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).expect("serialize MT-025 proof manifest"),
    );
    let manifest_reread: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path).expect("reread MT-025 proof manifest"),
    )
    .expect("published MT-025 manifest is valid JSON");
    assert_eq!(manifest_reread["run_id"], run_id);
    assert_eq!(manifest_reread["evidence_sha256"], evidence_sha256);
    assert_eq!(manifest_reread["screenshot_sha256"], screenshot_sha256);

    println!(
        "MT-025 V4 current-source SurrealDB Argus: run={} cancel={} save={} overlay={} screenshot={} evidence={} manifest={}",
        run_id,
        cancelled.receipt_status,
        saved.receipt_status,
        overlay_id,
        screenshot_path.display(),
        evidence_path.display(),
        manifest_path.display(),
    );
    assert_no_local_artifact_dir();
}
