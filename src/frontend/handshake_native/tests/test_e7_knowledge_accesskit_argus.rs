//! WP-KERNEL-012 MT-042 V4 canonical Argus/live-PostgreSQL proof.
#![cfg(all(feature = "integration", feature = "wgpu_screenshots"))]

use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicU64, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, ArgusObservation, CanonicalArgusDriver};

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use handshake_native::accessibility::knowledge_action_registry::{
    canvas_card_author_id, collection_lane_author_id, graph_edge_author_id,
};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend::knowledge_documents::{
    CreateDocumentRequest, HskDocumentHeaders, KnowledgeDocumentsClient,
};
use handshake_native::backend_client::{BlockViewClient, BlockViewOpCell, HealthInfo};
use handshake_native::command_registry::{CMD_VIEW_CANVAS, CMD_VIEW_GRAPH};
use handshake_native::graph::block_collection_view::{
    kanban_card_author_id, BlockViewDefinition, BlockViewGroupBy, BlockViewKind,
};
use handshake_native::graph::graph_view::{node_author_id, RETRY_AUTHOR_ID};
use handshake_native::pane_registry::PaneId;
use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

fn run_id() -> String {
    format!(
        "run-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    )
}

fn artifact_dir(run_id: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_PROOF_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate lives below the worktree collection")
                .join("Handshake_Artifacts")
                .join("handshake-test")
        });
    root.join("wp-kernel-012-mt-042-v4").join(run_id)
}

fn sha256(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            std::fs::read(path).unwrap_or_else(|error| {
                panic!("read proof artifact {}: {error}", path.display())
            })
        )
    )
}

fn write_json(dir: &Path, name: &str, value: &serde_json::Value) -> serde_json::Value {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    serde_json::json!({"path": path, "sha256": sha256(&path)})
}

fn capture(harness: &mut Harness<'_, HandshakeApp>, dir: &Path, name: &str) -> serde_json::Value {
    let path = dir.join(name);
    let image = harness
        .render()
        .unwrap_or_else(|error| panic!("render {name}: {error}"));
    assert!(image.width() > 0 && image.height() > 0);
    let dimensions = [image.width(), image.height()];
    image
        .save(&path)
        .unwrap_or_else(|error| panic!("save {}: {error}", path.display()));
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    serde_json::json!({"path": path, "sha256": sha256(&path), "dimensions": dimensions})
}

fn capture_with_tree(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    dir: &Path,
    stem: &str,
) -> Vec<serde_json::Value> {
    let tree = inspect_clean(argus, harness);
    vec![
        write_json(dir, &format!("{stem}-tree.json"), &tree),
        capture(harness, dir, &format!("{stem}.png")),
    ]
}

fn inspect_clean(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
) -> serde_json::Value {
    settle_incidental_fems_for_capture(harness);
    let tree = argus.inspect(harness);
    assert_no_modal_contamination(&tree);
    tree
}

fn settle_incidental_fems_for_capture(harness: &mut Harness<'_, HandshakeApp>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let cleared = harness
            .state_mut()
            .clear_incidental_fems_notice_for_integration_test();
        harness.run_steps(1);
        if cleared
            && harness
                .state_mut()
                .clear_incidental_fems_notice_for_integration_test()
        {
            // Publish one final tree after clearing the terminal notice. Re-checking after that frame
            // proves no deferred refresh immediately repopulated the unrelated overlay.
            harness.run_steps(1);
            if harness
                .state_mut()
                .clear_incidental_fems_notice_for_integration_test()
            {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "FEMS contamination did not clear"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn isolate_knowledge_proof_surface(app: &mut HandshakeApp) {
    // The default shell intentionally seeds Wiki and Runtime Chat beside the primary editor. They are
    // unrelated to MT-042 and can display honest local-only/EndpointMissing states that obscure both
    // pixels and AccessKit evidence. Keep the real shell and pane-a factory route, but remove the two
    // unrelated seeded panes from this fixture-owned layout before opening a knowledge surface.
    for pane in [PaneId::from("pane-b"), PaneId::from("pane-c")] {
        if let Some(tab_count) = app.tab_bar_states().get(&pane).map(|bar| bar.tabs.len()) {
            app.close_tab_indices_for_test(pane.clone(), (0..tab_count).collect());
            assert!(
                app.tab_bar_states()
                    .get(&pane)
                    .is_some_and(|bar| bar.tabs.is_empty()),
                "unrelated seeded pane {} did not close cleanly",
                pane.as_ref()
            );
        }
        app.tab_bar_states_mut().remove(&pane);
        app.pane_registry()
            .lock()
            .expect("pane registry mutex")
            .remove(&pane);
    }
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
}

fn pg_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn pg_scalar(sql: &str) -> String {
    interconnect_support::run_bounded_psql(sql, None)
        .trim()
        .to_owned()
}

fn process_absent(pid: u64) -> bool {
    #[cfg(target_os = "windows")]
    {
        let filter = format!("PID eq {pid}");
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output()
            .expect("query backend PID teardown");
        !String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        !std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn drive_until(
    harness: &mut Harness<'_, HandshakeApp>,
    proof: &str,
    condition: impl Fn(&HandshakeApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let _ = harness
            .state_mut()
            .clear_incidental_fems_notice_for_integration_test();
        harness.run_steps(2);
        if condition(harness.state()) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {proof}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn assert_no_modal_contamination(tree: &serde_json::Value) {
    let serialized = tree.to_string().to_ascii_lowercase();
    // Ban the MODAL, not the substring "fems".
    //
    // The property this guards is "no modal is open over the proof surface". A blanket substring ban
    // does not express that, and it stopped being true the moment the shell gained durable
    // click-completion observers whose author_ids merely MENTION fems:
    //   mt064.fems-proposal-flow-completion  (app.rs:309, projected unconditionally at :12672)
    //   mt065.fems-swarm-flow-completion     (app.rs:1577, projected unconditionally at :12683)
    // Both are shell-owned Role::Status nodes published every snapshot in the DEFAULT state
    // (generation 0, state ready), so this fired on the very first inspect_clean with nothing clicked
    // and no modal anywhere. Confirmed against a historical PASSING artifact set for this same test
    // (wp-kernel-012-mt-042-v4/run-51300-...), whose tree carries the identical family of durable
    // observers — mt033/mt034/mt035/mt036/mt042 — and zero occurrences of fems, because MT-064/065
    // did not exist yet. The observers and the assertion landed on divergent branches days apart.
    //
    // Matching author_id EXACTLY means a shell observer that merely names FEMS can no longer trip
    // this, while a genuinely mounted proposal dialog still does — it publishes this dialog id AND
    // "role":"dialog", which the check below independently catches.
    for modal_author_id in [
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_CANCEL_AUTHOR_ID,
    ] {
        assert!(
            !json_has_author_id(tree, modal_author_id),
            "FEMS modal `{modal_author_id}` contaminated proof tree"
        );
    }
    assert!(
        !serialized.contains("modal.scrim"),
        "modal scrim contaminated proof tree"
    );
    assert!(
        !serialized.contains("\"role\":\"dialog\""),
        "Dialog contaminated proof tree"
    );
    for forbidden in [
        "runtime-chat-status",
        "endpointmissing",
        "wikilink-alias-local-only-banner",
        "alias resolution is running local-only",
        "local-only",
        "backend aliases are unavailable",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "unrelated shell state `{forbidden}` contaminated proof tree"
        );
    }
}

fn author_value(tree: &serde_json::Value, author_id: &str) -> Option<String> {
    match tree {
        serde_json::Value::Object(map) => {
            if map.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                return map
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned);
            }
            map.values()
                .find_map(|value| author_value(value, author_id))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| author_value(value, author_id)),
        _ => None,
    }
}

fn create_document(
    runtime: &tokio::runtime::Runtime,
    client: &KnowledgeDocumentsClient,
    headers: &HskDocumentHeaders,
    workspace_id: &str,
    title: &str,
) -> String {
    let body = format!("{title} canonical runtime document body.");
    let created = runtime
        .block_on(client.create_document(
            headers,
            &CreateDocumentRequest {
                workspace_id: workspace_id.to_owned(),
                title: title.to_owned(),
                create_if_title_absent: false,
                content_json: Some(serde_json::json!({
                    "type": "doc",
                    "content": [
                        {
                            "type": "heading",
                            "attrs": {"level": 1},
                            "content": [{"type": "text", "text": title}]
                        },
                        {
                            "type": "paragraph",
                            "content": [{"type": "text", "text": body}]
                        }
                    ]
                })),
                schema_version: None,
                project_ref: None,
                folder_ref: None,
            },
        ))
        .expect("create canonical rich document");
    created.document["rich_document_id"]
        .as_str()
        .expect("rich document identity")
        .to_owned()
}

fn create_kanban(client: &BlockViewClient, workspace_id: &str, title: &str) -> String {
    let mut definition = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    definition.query.content_type = Some("note".to_owned());
    definition.group_by = Some(BlockViewGroupBy::Tag);
    let block_id = uuid::Uuid::new_v4().to_string();
    let cell: BlockViewOpCell = Arc::new(Mutex::new(None));
    let generation = Arc::new(AtomicU64::new(1));
    client.create_view(
        workspace_id,
        &block_id,
        title,
        &definition,
        generation,
        1,
        Arc::clone(&cell),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(delivery) = cell.lock().unwrap().clone() {
            return delivery.result.expect("create canonical Kanban");
        }
        assert!(Instant::now() < deadline, "Kanban create timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn record_latest(argus: &CanonicalArgusDriver, actions: &mut Vec<ArgusObservation>) {
    let observation = argus.latest_terminal_observation();
    assert_no_modal_contamination(&observation.after);
    actions.push(observation);
}

#[test]
fn mt042_v4_canonical_argus_complete_runtime_proof() {
    let run_id = run_id();
    let proof_dir = artifact_dir(&run_id);
    std::fs::create_dir_all(&proof_dir).expect("create unique MT-042 V4 proof directory");

    let mut live = interconnect_support::require_live_backend();
    let backend_binding = live.owned_backend_binding_receipt();
    let runtime_roots = live.owned_runtime_roots_for_proof();
    let empty_workspace_id = live.workspace_id.clone();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(empty_workspace_id.clone());
    isolate_knowledge_proof_surface(&mut app);
    assert!(app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(&mut harness, "empty graph", |app| {
        app.mounted_graph_view()
            .lock()
            .is_ok_and(|graph| !graph.loading && graph.error.is_none() && graph.nodes.is_empty())
    });

    let mut argus = CanonicalArgusDriver::bind(
        harness.state(),
        &format!("wp-kernel-012-mt-042-v4-{run_id}"),
    );
    let empty_tree = inspect_clean(&mut argus, &mut harness);
    assert!(!empty_tree.to_string().contains("graph.node."));
    let mut artifacts = capture_with_tree(&mut argus, &mut harness, &proof_dir, "01-empty-graph");

    let workspace = live.create_workspace(&format!("mt042-v4-{run_id}"));
    let workspace_id = workspace["id"].as_str().unwrap().to_owned();
    let empty_cleanup_status = live.delete_workspace(&empty_workspace_id);
    assert!(matches!(empty_cleanup_status, 200..=299 | 404));
    live.workspace_id = workspace_id.clone();

    let docs = KnowledgeDocumentsClient::with_client(reqwest::Client::new(), live.base.clone());
    let headers = HskDocumentHeaders::for_operator(format!("mt042-v4-{run_id}"), &run_id);
    let alpha = create_document(&runtime, &docs, &headers, &workspace_id, "MT-042 V4 Alpha");
    let beta = create_document(&runtime, &docs, &headers, &workspace_id, "MT-042 V4 Beta");
    let gamma = create_document(&runtime, &docs, &headers, &workspace_id, "MT-042 V4 Gamma");
    let delta = create_document(&runtime, &docs, &headers, &workspace_id, "MT-042 V4 Delta");
    let initial_edge_one = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": alpha,
            "target_block_id": beta,
            "edge_type": "mention",
            "created_by": "user"
        }),
    );
    let initial_edge_two = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": beta,
            "target_block_id": gamma,
            "edge_type": "mention",
            "created_by": "user"
        }),
    );
    let canvas = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title":"MT-042 V4 Canvas"}),
    );
    let canvas_id = canvas["block_id"].as_str().unwrap().to_owned();
    let requested_todo = format!("mt042-v4-todo-{run_id}");
    assert!(requested_todo.is_ascii() && requested_todo.len() <= 64);
    let todo = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({
            "block_id": requested_todo.clone(),
            "content_type":"tag_hub",
            "title":"mt042-v4-todo"
        }),
    )["block_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(todo, requested_todo);
    let requested_done = format!("mt042-v4-done-{run_id}");
    assert!(requested_done.is_ascii() && requested_done.len() <= 64);
    let done = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({
            "block_id": requested_done.clone(),
            "content_type":"tag_hub",
            "title":"mt042-v4-done"
        }),
    )["block_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(done, requested_done);
    live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": alpha,
            "target_block_id": todo,
            "edge_type":"tag",
            "created_by":"user"
        }),
    );
    let block_view_client = BlockViewClient::new(live.base.clone(), runtime.handle().clone());
    let kanban_id = create_kanban(&block_view_client, &workspace_id, "MT-042 V4 Kanban");

    harness
        .state_mut()
        .bind_active_project_for_integration_test(workspace_id.clone());
    isolate_knowledge_proof_surface(harness.state_mut());
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
    drive_until(&mut harness, "populated graph", |app| {
        app.mounted_graph_view().lock().is_ok_and(|graph| {
            !graph.loading
                && graph.error.is_none()
                && [
                    alpha.as_str(),
                    beta.as_str(),
                    gamma.as_str(),
                    delta.as_str(),
                ]
                .iter()
                .all(|id| graph.nodes.iter().any(|node| node.block_id == **id))
        })
    });
    let graph_tree = inspect_clean(&mut argus, &mut harness);
    for id in [&alpha, &beta, &gamma, &delta] {
        assert!(json_has_author_id(&graph_tree, &node_author_id(id)));
    }
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "02-populated-graph",
    ));

    let mut actions = Vec::new();
    let edge_count_before_malformed = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {}",
        pg_literal(&workspace_id)
    ));
    argus.click_with_payload_expect_typed_rejected_and_reinspect(
        &mut harness,
        "graph.add-edge",
        serde_json::json!({"source_id":alpha}),
        "requires non-empty source_id and target_id",
    );
    argus.assert_latest_terminal_predicate(&mut harness, "malformed-payload-rejected", |tree| {
        tree["action_receipts"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["status"] == "rejected"))
    });
    let edge_count_after_malformed = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {}",
        pg_literal(&workspace_id)
    ));
    assert_eq!(edge_count_after_malformed, edge_count_before_malformed);
    record_latest(&argus, &mut actions);

    argus.click_with_payload_expect_applied_and_reinspect(
        &mut harness,
        "graph.add-edge",
        serde_json::json!({"source_id":alpha,"target_id":gamma}),
    );
    let created_edge_id = harness
        .state()
        .mounted_graph_view()
        .lock()
        .unwrap()
        .edges
        .iter()
        .find(|edge| edge.source == alpha && edge.target == gamma)
        .and_then(|edge| edge.edge_id.clone())
        .expect("exact created edge in authoritative projection");
    let created_edge_author = graph_edge_author_id(&created_edge_id);
    let created_edge_present_after_add = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {} AND edge_id = {} AND source_block_id = {} AND target_block_id = {} AND edge_type = 'mention'",
        pg_literal(&workspace_id),
        pg_literal(&created_edge_id),
        pg_literal(&alpha),
        pg_literal(&gamma),
    ));
    assert_eq!(created_edge_present_after_add, "1");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-edge-created",
        serde_json::json!({"edge_id":created_edge_id}),
        |tree| json_has_author_id(tree, &created_edge_author),
    );
    record_latest(&argus, &mut actions);

    argus.click_with_payload_expect_applied_and_reinspect(
        &mut harness,
        "graph.remove-edge",
        serde_json::json!({"edge_id":created_edge_id}),
    );
    argus.assert_latest_terminal_predicate(&mut harness, "exact-edge-removed", |tree| {
        !json_has_author_id(tree, &created_edge_author)
    });
    let created_edge_absent_after_remove = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {} AND edge_id = {}",
        pg_literal(&workspace_id),
        pg_literal(&created_edge_id),
    ));
    assert_eq!(created_edge_absent_after_remove, "0");
    record_latest(&argus, &mut actions);
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "03-graph-edge-add-remove",
    ));

    let beta_author = node_author_id(&beta);
    let mut reading_modes =
        handshake_native::rich_editor::reading_mode::reading_mode_store(&harness.ctx);
    reading_modes.set(
        &beta,
        handshake_native::rich_editor::reading_mode::ViewMode::Reading,
    );
    handshake_native::rich_editor::reading_mode::write_reading_mode_store(
        &harness.ctx,
        &reading_modes,
    );
    argus.click_expect_applied_and_reinspect(&mut harness, &beta_author);
    let rich_author = format!("rich-editor.document.{beta}");
    let beta_body = "MT-042 V4 Beta canonical runtime document body.";
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-rich-document-opened",
        serde_json::json!({"rich_document_id":beta}),
        |tree| {
            let rendered = tree.to_string();
            json_has_author_id(tree, &rich_author)
                && rendered.contains("MT-042 V4 Beta")
                && rendered.contains(beta_body)
        },
    );
    record_latest(&argus, &mut actions);
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "04-opened-document",
    ));

    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
    drive_until(&mut harness, "graph remounted", |app| {
        app.mounted_graph_view()
            .lock()
            .is_ok_and(|graph| !graph.loading && graph.error.is_none())
    });
    let stale_before = inspect_clean(&mut argus, &mut harness);
    let edges_before_stale = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {}",
        pg_literal(&workspace_id)
    ));
    {
        let board = harness.state().mounted_canvas_board();
        let mut board = board.lock().unwrap();
        board.workspace_id = workspace_id.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_CANVAS));
    drive_until(&mut harness, "Canvas mounted", |app| {
        app.mounted_canvas_board()
            .lock()
            .is_ok_and(|board| !board.loading && board.error.is_none())
    });
    let stale_hidden = argus.click_from_snapshot_expect_rpc_rejected(
        &mut harness,
        "graph.add-edge",
        &stale_before,
        "no live widget",
    );
    let edges_after_stale = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {}",
        pg_literal(&workspace_id)
    ));
    assert_eq!(edges_after_stale, edges_before_stale);

    argus.click_with_payload_expect_applied_and_reinspect(
        &mut harness,
        "canvas.place-block",
        serde_json::json!({"block_id":alpha,"x":123.0,"y":234.0}),
    );
    let placement_id = harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .placements
        .iter()
        .find(|placement| placement.placed_block_id == alpha)
        .map(|placement| placement.placement_id.clone())
        .expect("authoritative Canvas contains exact placement");
    let placement_author = canvas_card_author_id(&placement_id);
    let placement_present_after_create = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_canvas_placements WHERE workspace_id = {} AND placement_id = {} AND placed_block_id = {}",
        pg_literal(&workspace_id),
        pg_literal(&placement_id),
        pg_literal(&alpha),
    ));
    assert_eq!(placement_present_after_create, "1");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-placement-created",
        serde_json::json!({"placement_id":placement_id}),
        |tree| json_has_author_id(tree, &placement_author),
    );
    record_latest(&argus, &mut actions);
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "05-canvas-place",
    ));

    argus.click_with_payload_expect_applied_and_reinspect(
        &mut harness,
        "canvas.remove-placement",
        serde_json::json!({"placement_id":placement_id}),
    );
    argus.assert_latest_terminal_predicate(&mut harness, "exact-placement-removed", |tree| {
        !json_has_author_id(tree, &placement_author)
    });
    let placement_absent_after_remove = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_canvas_placements WHERE workspace_id = {} AND placement_id = {}",
        pg_literal(&workspace_id),
        pg_literal(&placement_id),
    ));
    assert_eq!(placement_absent_after_remove, "0");
    record_latest(&argus, &mut actions);
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "06-canvas-remove",
    ));

    assert!(matches!(
        harness.state_mut().open_block_collection_view(&kanban_id),
        NavDispatchOutcome::Opened { .. }
    ));
    drive_until(&mut harness, "Kanban mounted", |app| {
        app.mounted_block_collection_view()
            .lock()
            .is_ok_and(|view| {
                view.view_block_id == kanban_id
                    && !view.loading
                    && !view.in_flight
                    && view.error.is_none()
            })
    });
    argus.click_with_payload_expect_applied_and_reinspect(
        &mut harness,
        "collection.kanban-move",
        serde_json::json!({"block_id":alpha,"from_lane":todo,"to_lane":done}),
    );
    let done_lane_author = collection_lane_author_id(&done);
    let todo_lane_author = collection_lane_author_id(&todo);
    let alpha_card_author = kanban_card_author_id(&alpha);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-kanban-lane-move",
        serde_json::json!({"block_id":alpha,"to_lane":done}),
        |tree| {
            author_value(tree, &done_lane_author).is_some_and(|value| value.contains(&alpha))
                && author_value(tree, &todo_lane_author).is_none_or(|value| !value.contains(&alpha))
                && author_value(tree, &alpha_card_author).as_deref() == Some(done.as_str())
        },
    );
    let source_tag_absent_after_move = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {} AND source_block_id = {} AND target_block_id = {} AND edge_type = 'tag'",
        pg_literal(&workspace_id), pg_literal(&alpha), pg_literal(&todo),
    ));
    let target_tag_present_after_move = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {} AND source_block_id = {} AND target_block_id = {} AND edge_type = 'tag'",
        pg_literal(&workspace_id), pg_literal(&alpha), pg_literal(&done),
    ));
    assert_eq!(source_tag_absent_after_move, "0");
    assert_eq!(target_tag_present_after_move, "1");
    record_latest(&argus, &mut actions);
    artifacts.extend(capture_with_tree(
        &mut argus,
        &mut harness,
        &proof_dir,
        "07-kanban-move",
    ));
    argus.finish_require_no_indeterminate();

    artifacts.push(write_json(&proof_dir, "stale-hidden.json", &stale_hidden));

    // Retry recovery: retain the exact error surface, rebind transport, then click Retry canonically.
    let (error_app, error_runtime) = {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }));
        app.set_backend_base_url_for_test("http://127.0.0.1:0", runtime.handle().clone());
        app.bind_active_project_for_integration_test(workspace_id.clone());
        isolate_knowledge_proof_surface(&mut app);
        assert!(app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH));
        (app, runtime)
    };
    let mut error_harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), error_app);
    drive_until(&mut error_harness, "graph backend error", |app| {
        app.mounted_graph_view()
            .lock()
            .is_ok_and(|graph| graph.error.is_some() && !graph.loading)
    });
    let mut error_argus = CanonicalArgusDriver::bind(
        error_harness.state(),
        &format!("wp-kernel-012-mt-042-v4-error-{run_id}"),
    );
    let _error_tree = inspect_clean(&mut error_argus, &mut error_harness);
    artifacts.extend(capture_with_tree(
        &mut error_argus,
        &mut error_harness,
        &proof_dir,
        "08-backend-error-before-retry",
    ));
    error_harness
        .state_mut()
        .set_backend_base_url_for_test(&live.base, error_runtime.handle().clone());
    error_argus.click_expect_applied_and_reinspect(&mut error_harness, RETRY_AUTHOR_ID);
    error_argus.assert_latest_terminal_predicate_with_evidence(
        &mut error_harness,
        "retry-recovers-and-disappears",
        serde_json::json!({"workspace_id":workspace_id}),
        |tree| !json_has_author_id(tree, RETRY_AUTHOR_ID),
    );
    let retry_observation = error_argus.latest_terminal_observation();
    assert_no_modal_contamination(&retry_observation.after);
    let retry_receipt = retry_observation.after["action_receipts"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["receipt_id"].as_u64() == Some(retry_observation.receipt_id))
        })
        .expect("Retry receipt retained");
    let observer_token: serde_json::Value = serde_json::from_str(
        retry_receipt["observed_value"]
            .as_str()
            .expect("Retry observer token"),
    )
    .expect("parse Retry observer token");
    let retry_detail: serde_json::Value = serde_json::from_str(
        observer_token["terminal_detail"]
            .as_str()
            .expect("Retry terminal detail"),
    )
    .expect("parse Retry terminal detail");
    assert!(
        retry_detail["request_generation_after"].as_u64().unwrap()
            > retry_detail["request_generation_before"].as_u64().unwrap()
    );
    assert!(retry_detail["terminal_error"].is_null());
    actions.push(retry_observation.clone());
    artifacts.push(write_json(
        &proof_dir,
        "retry-recovery-action.json",
        &serde_json::to_value(retry_observation).unwrap(),
    ));
    artifacts.extend(capture_with_tree(
        &mut error_argus,
        &mut error_harness,
        &proof_dir,
        "09-backend-retry-recovered",
    ));
    error_argus.finish_require_no_indeterminate();

    let actions_json = serde_json::to_value(&actions).unwrap();
    artifacts.push(write_json(
        &proof_dir,
        "canonical-actions.json",
        &actions_json,
    ));

    let exact_node_count = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = {} AND block_id IN ({},{},{},{})",
        pg_literal(&workspace_id),
        pg_literal(&alpha),
        pg_literal(&beta),
        pg_literal(&gamma),
        pg_literal(&delta),
    ));
    let exact_document_count = pg_scalar(&format!(
        "SELECT COUNT(*) FROM knowledge_rich_documents WHERE workspace_id = {} AND rich_document_id IN ({},{},{},{})",
        pg_literal(&workspace_id), pg_literal(&alpha), pg_literal(&beta), pg_literal(&gamma), pg_literal(&delta),
    ));
    let initial_edges_present = pg_scalar(&format!(
        "SELECT COUNT(*) FROM loom_edges WHERE workspace_id = {} AND edge_id IN ({},{}) AND edge_type = 'mention'",
        pg_literal(&workspace_id),
        pg_literal(initial_edge_one["edge_id"].as_str().unwrap()),
        pg_literal(initial_edge_two["edge_id"].as_str().unwrap()),
    ));
    assert_eq!(exact_node_count, "4");
    assert_eq!(exact_document_count, "4");
    assert_eq!(initial_edges_present, "2");
    let db_evidence = serde_json::json!({
        "workspace_id": workspace_id,
        "query_bindings": {"alpha":alpha,"beta":beta,"gamma":gamma,"delta":delta,"created_edge_id":created_edge_id,"placement_id":placement_id,"todo":todo,"done":done},
        "exact_node_count": exact_node_count,
        "exact_document_count": exact_document_count,
        "initial_edges_present": initial_edges_present,
        "created_edge_present_after_add": created_edge_present_after_add,
        "created_edge_absent_after_remove": created_edge_absent_after_remove,
        "placement_present_after_create": placement_present_after_create,
        "placement_absent_after_remove": placement_absent_after_remove,
        "source_tag_absent_after_move": source_tag_absent_after_move,
        "target_tag_present_after_move": target_tag_present_after_move,
        "edge_count_before_malformed": edge_count_before_malformed,
        "edge_count_after_malformed": edge_count_after_malformed,
        "edges_before_stale": edges_before_stale,
        "edges_after_stale": edges_after_stale,
    });
    artifacts.push(write_json(
        &proof_dir,
        "database-evidence.json",
        &db_evidence,
    ));

    let backend_pid = backend_binding["backend_pid"].as_u64().unwrap();
    let backend_url = reqwest::Url::parse(backend_binding["base_url"].as_str().unwrap()).unwrap();
    let backend_socket = format!(
        "{}:{}",
        backend_url.host_str().unwrap(),
        backend_url.port_or_known_default().unwrap()
    );
    let cleanup_status = live.delete_workspace(&workspace_id);
    assert!(matches!(cleanup_status, 200..=299 | 404));
    live.workspace_id.clear();
    live.assert_cleanup();
    drop(live);
    let runtime_root_teardown = runtime_roots
        .iter()
        .map(|path| serde_json::json!({"path":path,"exists_after_drop":path.exists()}))
        .collect::<Vec<_>>();
    assert!(runtime_roots.iter().all(|path| !path.exists()));
    assert!(process_absent(backend_pid));
    assert!(std::net::TcpStream::connect_timeout(
        &backend_socket.parse().unwrap(),
        Duration::from_millis(500)
    )
    .is_err());
    artifacts.push(write_json(
        &proof_dir,
        "teardown.json",
        &serde_json::json!({
            "backend_binding": backend_binding,
            "workspace_cleanup_status": cleanup_status,
            "runtime_roots": runtime_root_teardown,
            "backend_pid": backend_pid,
            "backend_process_absent": true,
            "backend_socket": backend_socket,
            "backend_socket_closed": true,
        }),
    ));

    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_files = [
        source_root.join("src/app.rs"),
        source_root.join("src/mcp/action.rs"),
        source_root.join("src/backend_client.rs"),
        source_root.join("src/graph/graph_view.rs"),
        source_root.join("src/graph/canvas_board.rs"),
        source_root.join("src/graph/block_collection_view.rs"),
        source_root.join("src/split_layout.rs"),
        source_root.join("src/manual_content_editors.rs"),
        source_root.join("tests/test_e7_knowledge_accesskit.rs"),
        source_root.join("tests/test_e7_knowledge_accesskit_argus.rs"),
        source_root.join("tests/native_gui_support/canonical_argus_driver.rs"),
        source_root.join("tests/interconnect_support/mod.rs"),
    ]
    .into_iter()
    .map(|path| serde_json::json!({"path":path,"sha256":sha256(&path)}))
    .collect::<Vec<_>>();
    let terminal_action_count = actions.len();
    let immediate_rpc_rejection_count = 1_usize;
    let indeterminate_count = actions
        .iter()
        .filter(|observation| observation.receipt_status == "indeterminate")
        .count();
    let unresolved_count = actions
        .iter()
        .filter(|observation| {
            matches!(observation.receipt_status.as_str(), "queued" | "dispatched")
        })
        .count();
    assert_eq!(indeterminate_count, 0);
    assert_eq!(unresolved_count, 0);
    let bundle = serde_json::json!({
        "schema_id": "hsk.wp-kernel-012.mt-042-v4-proof-bundle@1",
        "run_id": run_id,
        "status": "PASS",
        "command": "cargo test --manifest-path src/frontend/handshake_native/Cargo.toml --features integration,wgpu_screenshots --test test_e7_knowledge_accesskit_argus -- --nocapture --test-threads=1",
        "raw_suite": {
            "command": "cargo test --manifest-path src/frontend/handshake_native/Cargo.toml --features integration --test test_e7_knowledge_accesskit -- --nocapture --test-threads=1",
            "expected_case_count": 24,
            "result": null,
            "note": "recorded by the separately executed raw-suite command; this canonical test does not self-claim that external result"
        },
        "source_files": source_files,
        "backend_binding": backend_binding,
        "artifacts": artifacts,
        "terminal_action_count": terminal_action_count,
        "immediate_rpc_rejection_count": immediate_rpc_rejection_count,
        "action_count": terminal_action_count + immediate_rpc_rejection_count,
        "indeterminate_count": indeterminate_count,
        "unresolved_count": unresolved_count,
        "workspace_cleanup_status": cleanup_status,
        "runtime_roots_removed": true,
    });
    let bundle_path = proof_dir.join("proof-bundle.json");
    std::fs::write(&bundle_path, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
    println!("MT-042 V4 proof bundle: {}", bundle_path.display());

    for local in ["test_output", "tests/screenshots"] {
        assert!(
            !Path::new(local).exists(),
            "repo-local artifact dir {local}"
        );
    }
}
