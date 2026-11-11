//! WP-KERNEL-012 E3 MT-022 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED folder-tree panel.
//!
//! `validation_v2` failed MT-022 in part because "the folder tree also lacks current canonical Argus
//! steering and re-observation proof". The isolated `test_folder_tree.rs` kittest coverage drives the
//! folder-tree WIDGET, but never the mounted `HandshakeApp` through the real localhost `SwarmMcpServer`
//! transport the way an out-of-process swarm agent does. This test closes that exact gap:
//!
//!   1. creates an isolated real-SurrealDB workspace with two folders plus one member block, then
//!      mounts the production `HandshakeApp` Folders pane against the current-source backend,
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves the mounted folder nodes are addressable by stable author_id in the live
//!      tree (`folder-tree.node.{id}`),
//!   4. clicks one folder through Argus and holds the receipt Pending across the real lazy child fetch,
//!   5. re-observes exact selected/expanded/generation/child-list state and requires raw Applied, and
//!   6. verifies fixture cleanup + canonical finish before publishing tree/screenshot evidence.
//!
//! A second test proves the empty-workspace state renders and inspects through canonical Argus with no
//! folder nodes and no panic (AC7).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-022/` root.
#![cfg(feature = "integration")]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use handshake_diag_ring::ring::{DiagRingReaderV2, DiagRingWriterV2};
use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity, DEFAULT_CAPACITY};
use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::diagnostics::gui_events::GuiDiagnosticPhase;
use handshake_native::diagnostics::recorder::install_v2;
use handshake_native::editor_pane_factories::{placeholder_pane_type, FOLDER_TREE_PANE_LABEL};
use handshake_native::graph::folder_tree::{node_author_id, status_author_id};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn install_mounted_diagnostics_ring() -> &'static (PathBuf, [u8; 16]) {
    static RING: OnceLock<(PathBuf, [u8; 16])> = OnceLock::new();
    RING.get_or_init(|| {
        let dir = external_artifact_dir("wp-kernel-012-mt-022/canonical-argus");
        std::fs::create_dir_all(&dir).expect("create external MT-022 diagnostics directory");
        let path = dir.join(format!("cabinet-child-read-{}.ring", uuid::Uuid::now_v7()));
        let nonce = *uuid::Uuid::new_v4().as_bytes();
        let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, nonce)
            .expect("create mounted MT-022 V2 diagnostic ring");
        assert!(
            install_v2(writer),
            "install the ring before either shell records"
        );
        (path, nonce)
    })
}

fn cabinet_child_read_events(ring: &DiagRingReaderV2) -> Vec<DiagEvent> {
    const CABINET_CHILD_READ: u64 = 0x4755_4900_0000_000f;
    ring.read_last_n(ring.capacity())
        .into_iter()
        .rev()
        .map(|record| record.event)
        .filter(|event| event.counter_a == CABINET_CHILD_READ)
        .collect()
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
fn folders_shell(
    base: &str,
    workspace_id: &str,
    account: Option<std::sync::Arc<handshake_native::local_account::AuthenticatedContext>>,
) -> (HandshakeApp, tokio::runtime::Runtime) {
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
    // MT-153 C3: the mounted Loom folder client carries the account session when one is supplied.
    if let Some(account) = account {
        app.bind_initial_account(account)
            .expect("bind the fixture's authenticated proof account");
    }
    app.set_folder_backend_base_url_for_test(base);
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    retype_pane_a_to_folders(&mut app);
    (app, runtime)
}

struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) {
        assert_eq!(
            self.backend.delete_workspace(&self.workspace_id),
            204,
            "delete the exact MT-022 owned workspace"
        );
        self.cleaned = true;
    }
}

impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.backend.delete_workspace(&self.workspace_id)
            }));
        }
    }
}

fn require_absent_proof_artifact(path: &Path) {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("inspect proof artifact {}: {error}", path.display()),
        Ok(_) => panic!("preserve existing proof artifact {}", path.display()),
    }
}

fn create_proof_artifact(path: &Path) -> std::fs::File {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|error| panic!("create new proof artifact {}: {error}", path.display()))
}

fn drive_until(
    harness: &mut Harness<'_, HandshakeApp>,
    condition: impl Fn(&HandshakeApp) -> bool,
    proof: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        harness.run_steps(2);
        if condition(harness.state()) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {proof}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn folder_status(tree: &serde_json::Value, author_id: &str) -> serde_json::Value {
    let value = json_node_by_author_id(tree, author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .expect("folder status node exposes machine-readable state");
    serde_json::from_str(value).expect("folder status value is valid JSON")
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

fn assert_unavailable_child_read(
    live: &interconnect_support::LiveBackend,
    workspace_id: &str,
    folder_id: &str,
    ring: &DiagRingReaderV2,
    prior_events: &[DiagEvent],
) -> serde_json::Value {
    use egui_kittest::kittest::{NodeT, Queryable};
    use handshake_native::graph::folder_tree::FolderRow;

    // Reuse the owned fixture's authenticated list GET. Only its selected root row crosses the
    // mounted delivery seam, so the runtime-absent shell needs no account or transport binding.
    let (status, body) =
        live.get_json_response(&format!("/workspaces/{workspace_id}/loom/folders"));
    assert_eq!(status, 200, "authenticated folder list");
    let row = body
        .as_array()
        .expect("folder list array")
        .iter()
        .find(|row| row["folder_id"].as_str() == Some(folder_id))
        .expect("owned Projects folder in the authenticated list");
    assert_eq!(row["workspace_id"], workspace_id);
    assert!(
        row["parent_folder_id"].is_null(),
        "Projects is a root folder"
    );
    let name = row["name"].as_str().expect("folder name");
    let color = row["color"].as_str().expect("fixture folder color");
    let row = FolderRow::new(folder_id, None, name, Some(color.to_owned()));

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.bind_active_project_for_integration_test(workspace_id);
    retype_pane_a_to_folders(&mut app);
    let mounted_tree = app.mounted_folder_tree_for_test();
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let (epoch, sequence) = harness.state().folder_request_identity_for_test();
    assert_eq!(
        mounted_tree
            .lock()
            .expect("mounted folder tree")
            .workspace_id,
        workspace_id
    );
    harness
        .state()
        .deliver_folder_list_for_test(workspace_id, epoch, sequence, Ok(vec![row]));
    harness.run_steps(2);

    assert_eq!(harness.state().folder_child_cells_in_flight_for_test(), 0);
    assert_eq!(cabinet_child_read_events(ring).as_slice(), prior_events);
    let author = node_author_id(folder_id);
    let target = harness
        .root()
        .children_recursive()
        .find_map(|node| {
            let node = node.accesskit_node();
            (node.author_id() == Some(author.as_str())).then(|| node.id())
        })
        .expect("authenticated Projects row is mounted and actionable");
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target,
            data: None,
        },
    ));
    harness.run_steps(2);

    let in_flight = harness.state().folder_child_cells_in_flight_for_test();
    assert_eq!(in_flight, 0);
    let mut tree = mounted_tree.lock().expect("mounted folder tree");
    assert_eq!(tree.selected_folder_id.as_deref(), Some(folder_id));
    let node = tree
        .find_folder_mut(folder_id)
        .expect("opened Projects folder");
    assert!(node.expanded);
    assert!(!node.loading);
    assert!(node.child_blocks.is_none());
    let request_sequence = node
        .expansion_request_sequence()
        .expect("Cabinet action sequence");
    assert_eq!(node.expansion_terminal_sequence(), Some(request_sequence));
    let error = "Folder children unavailable: runtime unavailable";
    assert_eq!(node.child_error.as_deref(), Some(error));

    let events = cabinet_child_read_events(ring);
    let suffix = events
        .strip_prefix(prior_events)
        .expect("earlier Cabinet events retained");
    let [unavailable] = suffix else {
        panic!("runtime-absent Cabinet action must add exactly one ring event: {suffix:?}");
    };
    assert_eq!(
        unavailable.counter_b,
        GuiDiagnosticPhase::Unavailable as u64
    );
    assert_eq!(unavailable.phase_marker, DiagPhase::Degraded.as_u8());
    assert_eq!(unavailable.severity, DiagSeverity::Warn.as_u8());
    assert_eq!(unavailable.event_code, DiagEventCode::Other.as_u16());
    assert_eq!(unavailable.thread_id, 0);
    assert_eq!(unavailable.metric_micros, 0);
    assert!(unavailable.sequence_id > prior_events.last().expect("prior Failed event").sequence_id);

    serde_json::json!({
        "fixture_list_http_status": status,
        "folder_id": folder_id,
        "request_sequence": request_sequence,
        "in_flight": in_flight,
        "error": error,
        "events": suffix,
    })
}

#[test]
fn mt022_mounted_folder_tree_canonical_argus_inspect_steer_reobserve() {
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-022/canonical-argus");
    let tree_path = artifact_dir.join("mt022-mounted-folder-tree-argus.json");
    let screenshot_path = artifact_dir.join("mt022-mounted-folder-tree.png");
    require_absent_proof_artifact(&tree_path);
    require_absent_proof_artifact(&screenshot_path);

    let (ring_path, nonce) = install_mounted_diagnostics_ring();
    let ring = DiagRingReaderV2::open(ring_path, *nonce)
        .expect("open the mounted app's V2 ring through the independent reader");

    let mut live = interconnect_support::require_reachable_backend();
    let nonce = uuid::Uuid::now_v7().simple().to_string();
    let workspace = live.create_workspace(&format!("mt022-argus-{nonce}"));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    let create_folder = |name: &str, color: &str| {
        let folder = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/folders"),
            &serde_json::json!({
                "name": name,
                "parent_folder_id": null,
                "color": color
            }),
        );
        folder["folder_id"]
            .as_str()
            .expect("folder create returns folder_id")
            .to_owned()
    };
    let projects_id = create_folder(&format!("MT-022 Projects {nonce}"), "#ff0000");
    let archive_id = create_folder(&format!("MT-022 Archive {nonce}"), "#3b82f6");
    let child = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({
            "content_type": "note",
            "title": format!("MT-022 child {nonce}")
        }),
    );
    let child_id = child["block_id"]
        .as_str()
        .expect("block create returns block_id")
        .to_owned();
    live.put_json(
        &format!("/workspaces/{workspace_id}/loom/folders/{projects_id}/blocks/{child_id}"),
        &serde_json::json!({ "sort_order": 0 }),
    );

    let (app, _rt) = folders_shell(&live.base, &workspace_id, live.account());
    let mounted_tree = app.mounted_folder_tree_for_test();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    drive_until(
        &mut harness,
        |_| {
            let mut tree = mounted_tree.lock().expect("mounted folder tree");
            tree.find_folder_mut(&projects_id).is_some()
                && tree.find_folder_mut(&archive_id).is_some()
                && !tree.loading
                && tree.error.is_none()
        },
        "mounted host loads the two real-SurrealDB folders",
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-022-folder-tree");
    let projects_author = node_author_id(&projects_id);
    let archive_author = node_author_id(&archive_id);
    let child_author = node_author_id(&child_id);
    let status_author = status_author_id(&projects_id);

    // (1) Canonical inspect: the mounted folder nodes are addressable by stable author_id.
    let before = argus.inspect(&mut harness);
    for author in [&projects_author, &archive_author] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted folder node '{author}' in the live tree"
        );
    }
    assert!(!json_has_author_id(&before, &child_author));
    let before_status = folder_status(&before, &status_author);
    let generation_before = before_status["generation"]
        .as_u64()
        .expect("pre-action folder generation");
    assert_eq!(before_status["expanded"], false);
    assert_eq!(before_status["child_state"], "not_requested");
    assert!(
        cabinet_child_read_events(&ring).is_empty(),
        "the mounted fixture must have no Cabinet child-read events before the click"
    );

    // (2) Click the real folder row. ActionChannel holds the receipt Pending while the production host
    // performs the authoritative child-list request and applies only the current workspace/sequence.
    let observation = argus.click_and_reinspect(&mut harness, &projects_author);
    assert_eq!(
        observation.receipt_status, "applied",
        "folder expansion must be causally acknowledged by the exact terminal generation"
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-022-folder-tree-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Bind the raw receipt to the exact selected/expanded generation and fresh child-list result.
    let predicate_status_author = status_author.clone();
    let predicate_child_author = child_author.clone();
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "folder-open-expand.exact-child-list-v1",
        serde_json::json!({
            "workspace_id": &workspace_id,
            "folder_id": &projects_id,
            "expected_generation": generation_before + 1,
            "child_block_id": &child_id,
        }),
        move |tree| {
            let state = folder_status(tree, &predicate_status_author);
            let request_sequence = state["request_sequence"].as_u64();
            state["generation"].as_u64() == Some(generation_before + 1)
                && state["selected"] == true
                && state["expanded"] == true
                && state["loading"] == false
                && request_sequence.is_some()
                && state["terminal_request_sequence"].as_u64() == request_sequence
                && state["child_state"] == "loaded"
                && state["child_count"].as_u64() == Some(1)
                && json_has_author_id(tree, &predicate_child_author)
        },
    );
    let terminal_status = folder_status(&terminal, &status_author);
    let child_read_events = cabinet_child_read_events(&ring);
    let [pending, completed] = child_read_events.as_slice() else {
        panic!("the one mounted read must produce exactly two ring events: {child_read_events:?}");
    };
    assert_eq!(pending.counter_b, GuiDiagnosticPhase::Pending as u64);
    assert_eq!(pending.phase_marker, DiagPhase::Start.as_u8());
    assert_eq!(completed.counter_b, GuiDiagnosticPhase::Completed as u64);
    assert_eq!(completed.phase_marker, DiagPhase::End.as_u8());
    assert!(pending.sequence_id < completed.sequence_id);
    for event in &child_read_events {
        assert_eq!(event.event_code, DiagEventCode::Other.as_u16());
        assert_eq!(event.severity, DiagSeverity::Info.as_u8());
        assert_eq!(event.thread_id, 0);
        assert_eq!(event.metric_micros, 0);
    }

    let (epoch, _) = harness.state().folder_request_identity_for_test();
    let settled_sequence = terminal_status["terminal_request_sequence"]
        .as_u64()
        .expect("the genuine mounted child-read sequence settled");
    // Replay an empty delivery for the retired request through the existing mounted FIFO seam.
    assert_eq!(harness.state().folder_child_cells_in_flight_for_test(), 0);
    harness.state().deliver_folder_children_for_test(
        &workspace_id,
        &projects_id,
        epoch,
        settled_sequence,
        Ok(Vec::new()),
    );
    assert_eq!(harness.state().folder_child_cells_in_flight_for_test(), 1);
    harness.run_steps(1);
    assert_eq!(harness.state().folder_child_cells_in_flight_for_test(), 0);
    assert_eq!(
        cabinet_child_read_events(&ring),
        child_read_events,
        "a late delivery for the settled request must not produce another terminal event"
    );
    {
        let mut tree = mounted_tree
            .lock()
            .expect("mounted folder tree after late delivery");
        let children = tree
            .find_folder_mut(&projects_id)
            .and_then(|node| node.child_blocks.as_ref())
            .expect("the genuine mounted child result remains loaded");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].block_id, child_id);
    }

    let parent_node = json_node_by_author_id(&terminal, &projects_author)
        .expect("terminal parent folder node remains addressable");
    assert!(json_has_author_id(&terminal, &archive_author));
    let child_node = json_node_by_author_id(&terminal, &child_author)
        .expect("terminal child leaf node is addressable");
    let child_actions = child_node["actions"]
        .as_array()
        .expect("terminal child actions array");
    assert!(
        child_actions.iter().any(|action| action == "Click")
            && !child_actions.iter().any(|action| action == "Expand")
            && !child_actions.iter().any(|action| action == "Collapse"),
        "terminal leaf must advertise Click only, never folder Expand/Collapse: {child_actions:?}"
    );
    let parent_x = parent_node["bounds"]["x"]
        .as_f64()
        .expect("terminal parent has horizontal bounds");
    let child_x = child_node["bounds"]["x"]
        .as_f64()
        .expect("terminal child has horizontal bounds");
    assert!(
        child_x >= parent_x + f64::from(handshake_native::graph::folder_tree::INDENT_PER_LEVEL),
        "terminal child must begin at least one indent step right of parent label: parent_x={parent_x}, child_x={child_x}"
    );

    // The mounted Folders pane is deliberately compact. Let the one-shot reveal requested by the exact
    // child-list completion settle through the production ScrollArea before capturing pixels; this is
    // what makes the newly loaded row visible rather than merely present in the AccessKit tree.
    harness.run_steps(2);
    let rendered = harness.render();
    let receipt_id = observation.receipt_id;
    let receipt_status = observation.receipt_status.clone();
    let agent_id = observation.agent_id.clone();
    let immediate_after = observation.after.clone();

    let archive_status_author = status_author_id(&archive_id);
    let archive_before = folder_status(&terminal, &archive_status_author);
    assert_eq!(archive_before["child_state"], "not_requested");
    let archive_generation = archive_before["generation"]
        .as_u64()
        .expect("pre-action Archive generation");
    // Remove only the unused fixture folder while its cached row is still mounted.
    let archive_route = format!("/workspaces/{workspace_id}/loom/folders/{archive_id}");
    assert_eq!(live.delete(&archive_route), 200, "delete the owned Archive fixture");
    let missing_status = live.get_status(&format!("{archive_route}/blocks"));
    assert_eq!(missing_status, 404, "the authenticated child route reports the missing folder");

    let failure_observation =
        argus.click_expect_applied_and_reinspect(&mut harness, &archive_author);
    let failed_after = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "folder-open-expand.current-child-read-failure-v1",
        serde_json::json!({
            "workspace_id": &workspace_id,
            "folder_id": &archive_id,
            "expected_generation": archive_generation + 1,
            "http_status": missing_status,
        }),
        |tree| {
            let state = folder_status(tree, &archive_status_author);
            let request_sequence = state["request_sequence"].as_u64();
            state["workspace_id"].as_str() == Some(workspace_id.as_str())
                && state["folder_id"].as_str() == Some(archive_id.as_str())
                && state["generation"].as_u64() == Some(archive_generation + 1)
                && state["selected"] == true
                && state["expanded"] == true
                && state["loading"] == false
                && request_sequence.is_some()
                && state["terminal_request_sequence"].as_u64() == request_sequence
                && state["child_state"] == "failed"
                && state["child_count"].is_null()
                && state["error"]
                    .as_str()
                    .is_some_and(|error| error.contains("GET non-success status 404"))
        },
    );
    let failure_status = folder_status(&failed_after, &archive_status_author);
    let all_child_read_events = cabinet_child_read_events(&ring);
    let failure_events = all_child_read_events
        .strip_prefix(child_read_events.as_slice())
        .expect("the existing successful read remains in the ring");
    let [failed_pending, failed] = failure_events else {
        panic!("the failing mounted read must add exactly two ring events: {failure_events:?}");
    };
    assert_eq!(failed_pending.counter_b, GuiDiagnosticPhase::Pending as u64);
    assert_eq!(failed_pending.phase_marker, DiagPhase::Start.as_u8());
    assert_eq!(failed_pending.severity, DiagSeverity::Info.as_u8());
    assert_eq!(failed.counter_b, GuiDiagnosticPhase::Failed as u64);
    assert_eq!(failed.phase_marker, DiagPhase::Degraded.as_u8());
    assert_eq!(failed.severity, DiagSeverity::Warn.as_u8());
    assert!(completed.sequence_id < failed_pending.sequence_id);
    assert!(failed_pending.sequence_id < failed.sequence_id);
    for event in failure_events {
        assert_eq!(event.event_code, DiagEventCode::Other.as_u16());
        assert_eq!(event.thread_id, 0);
        assert_eq!(event.metric_micros, 0);
    }

    let unavailable = assert_unavailable_child_read(
        &live,
        &workspace_id,
        &projects_id,
        &ring,
        &all_child_read_events,
    );

    // A proof is publishable only after the exact live fixture is reclaimed and the canonical driver
    // has accepted every action's terminal predicate. Drop the workspace cleanup borrow, then prove
    // teardown of the fixture-owned workspace and backend process before canonical finish/publication.
    cleanup.assert_cleaned();
    drop(cleanup);
    live.assert_cleanup();
    argus.finish();
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-022 Argus artifact dir");
    serde_json::to_writer_pretty(
        create_proof_artifact(&tree_path),
        &serde_json::json!({
            "workspace_id": &workspace_id,
            "folder_id": &projects_id,
            "child_block_id": &child_id,
            "before": before,
            "immediate_after": immediate_after,
            "terminal_after": terminal,
            "terminal_status": terminal_status,
            "cabinet_child_read_ring": ring_path,
            "cabinet_child_read_events": child_read_events,
            "cabinet_child_read_failure": {
                "receipt_id": failure_observation.receipt_id,
                "http_status": missing_status,
                "terminal_status": failure_status,
                "events": failure_events,
            },
            "cabinet_child_read_unavailable": unavailable,
            "receipt_id": receipt_id,
            "receipt_status": &receipt_status,
            "agent_id": &agent_id,
        }),
    )
    .expect("write canonical MT-022 folder-tree tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match rendered {
        Ok(image) => {
            use image::ImageEncoder;

            image::codecs::png::PngEncoder::new(create_proof_artifact(&screenshot_path))
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .expect("save mounted folder-tree screenshot");
            format!("CAPTURED {}", screenshot_path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-022 canonical Argus mounted folder tree: inspect(2 real-SurrealDB folders) -> click({projects_author}) \
         -> exact +1 selected/expanded child-list completion; receipt={} agent={} screenshot={} tree={}",
        receipt_status,
        agent_id,
        screenshot_marker,
        tree_path.display()
    );
    assert_no_local_artifact_dir();
}

#[test]
fn mt022_mounted_folder_tree_empty_state_canonical_argus() {
    install_mounted_diagnostics_ring();
    // AC7: with no folders, the mounted panel renders + inspects through canonical Argus with no folder
    // nodes and no panic.
    let (app, _rt) = folders_shell("http://127.0.0.1:1", "mt022-empty", None);
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
