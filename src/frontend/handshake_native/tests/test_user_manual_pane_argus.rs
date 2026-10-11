#![cfg(target_os = "windows")]

#[path = "argus_socket_support/live_socket.rs"]
mod live_socket;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::backend_client::{HttpMethod, UserManualClient};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneRecord, PaneRenderContext, PaneType,
};
use handshake_native::user_manual_pane::{
    page_author_id, search_hit_author_id, UserManualNavigation, UserManualPageContent,
    UserManualPageSummary, UserManualPaneController, UserManualPaneFactory, UserManualResultCell,
    UserManualSearchHit, UserManualSearchResponse, UserManualTransport, ERROR_AUTHOR_ID,
    LOADING_AUTHOR_ID, NAVIGATION_AUTHOR_ID, PAGE_AUTHOR_ID, READ_RECEIPT_AUTHOR_ID,
    RETRY_AUTHOR_ID, SEARCH_ACTION_AUTHOR_ID, SEARCH_INPUT_AUTHOR_ID, SEARCH_RESULTS_AUTHOR_ID,
    SEARCH_STATUS_AUTHOR_ID, SURFACE_AUTHOR_ID, UNAVAILABLE_AUTHOR_ID,
};
use live_socket::{
    collect_author_ids, contains_author_id, decode_verified_capture, node_text, require_node,
    wait_for_author_id, wait_for_author_id_between, LiveApp, LoopbackHttpFaultProxy,
};
use serde_json::json;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

const REQUIRED_ARTIFACT_ROOT: &str =
    r"D:\Projects\LLM projects\Handshake\Handshake Worktrees\Handshake\_Artifacts";
const LIVE_PROOF_DIR: &str = "mt022/user_manual_argus";
const LIVE_TIMEOUT: Duration = Duration::from_secs(20);

struct FixedManualTransport {
    fail: bool,
}

struct PendingManualTransport;

impl UserManualTransport for PendingManualTransport {
    fn fetch_navigation(&self, _cell: UserManualResultCell<UserManualNavigation>) {}

    fn fetch_page(&self, _slug: &str, _cell: UserManualResultCell<UserManualPageContent>) {}

    fn fetch_search(&self, _query: &str, _cell: UserManualResultCell<UserManualSearchResponse>) {}
}

impl UserManualTransport for FixedManualTransport {
    fn fetch_navigation(&self, cell: UserManualResultCell<UserManualNavigation>) {
        let result = if self.fail {
            Err("backend unavailable".to_string())
        } else {
            Ok(UserManualNavigation {
                manual_version: "2.0.11".to_string(),
                route_namespace: "/usermanual".to_string(),
                pages: vec![UserManualPageSummary {
                    slug: "model-runtime-registry-and-loom-degrade".to_string(),
                    title: "Model Runtime Registry and Loom Semantic Degrade".to_string(),
                    page_kind: "workflow".to_string(),
                    audience: "operator,model".to_string(),
                    manual_version: "2.0.11".to_string(),
                    content_hash: "hash-1".to_string(),
                    status: "current".to_string(),
                }],
            })
        };
        *cell.lock().expect("navigation cell") = Some(result);
    }

    fn fetch_page(&self, _slug: &str, cell: UserManualResultCell<UserManualPageContent>) {
        *cell.lock().expect("page cell") = Some(Ok(UserManualPageContent {
            page: json!({
                "slug": "model-runtime-registry-and-loom-degrade",
                "title": "Model Runtime Registry and Loom Semantic Degrade",
                "manual_version": "2.0.11"
            }),
            sections: vec![json!({
                "heading": "Loom dimension mismatch and recovery",
                "body_markdown": "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}."
            })],
            anchors: Vec::new(),
            bootstrap_receipt_event_id: "event-user-manual-open-1".to_string(),
            bootstrap_identity_used: true,
        }));
    }

    fn fetch_search(&self, query: &str, cell: UserManualResultCell<UserManualSearchResponse>) {
        let result = if self.fail {
            Err("backend unavailable".to_string())
        } else {
            Ok(UserManualSearchResponse {
                query: query.to_string(),
                count: 1,
                results: vec![UserManualSearchHit {
                    result_kind: "section".to_string(),
                    result_ref: "model-runtime-registry-and-loom-degrade:recovery".to_string(),
                    page_slug: Some("model-runtime-registry-and-loom-degrade".to_string()),
                    title: "Loom dimension mismatch and recovery".to_string(),
                    excerpt: "Loom degrades to keyword/trigram".to_string(),
                }],
            })
        };
        *cell.lock().expect("search cell") = Some(result);
    }
}

struct ManualHarnessState {
    factory: UserManualPaneFactory,
    record: PaneRecord,
}

fn render(ctx: &egui::Context, state: &mut ManualHarnessState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        state.factory.render(
            ui,
            &PaneRenderContext {
                record: &state.record,
                egui_id: egui::Id::new("user-manual-test-pane"),
            },
        );
    });
}

fn state(factory: UserManualPaneFactory) -> ManualHarnessState {
    ManualHarnessState {
        factory,
        record: PaneRecord::new(
            Arc::from("pane-a"),
            PaneType::UserManual,
            "default-project",
            Some("model-runtime-registry-and-loom-degrade".to_string()),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ),
    }
}

fn author_ids(harness: &Harness<'_, ManualHarnessState>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn node_by_author<'a>(
    harness: &'a Harness<'_, ManualHarnessState>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("missing {author_id}: {:?}", author_ids(harness)))
}

fn canonical_existing(path: impl AsRef<Path>, label: &str) -> PathBuf {
    std::fs::canonicalize(path.as_ref())
        .unwrap_or_else(|error| panic!("canonicalize {label} {}: {error}", path.as_ref().display()))
}

fn configure_live_proof_roots() -> (PathBuf, PathBuf, String) {
    let configured_artifacts = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .expect("HANDSHAKE_ARTIFACTS_DIR is required for MT-022 live proof");
    let configured_artifacts = canonical_existing(configured_artifacts, "artifact root");
    let required_artifacts = canonical_existing(REQUIRED_ARTIFACT_ROOT, "required artifact root");
    assert_eq!(
        configured_artifacts, required_artifacts,
        "MT-022 live proof must use the operator-authorized artifact root"
    );

    let data_dir = std::env::var("HANDSHAKE_DATA_DIR")
        .expect("HANDSHAKE_DATA_DIR must identify the running core's embedded data directory");
    let data_dir = canonical_existing(data_dir, "embedded data directory");
    assert!(
        data_dir.starts_with(&configured_artifacts),
        "embedded proof data must stay beneath the authorized artifact root: {}",
        data_dir.display()
    );

    let nonce = std::env::var("HANDSHAKE_MT022_ARGUS_PROOF_NONCE")
        .expect("HANDSHAKE_MT022_ARGUS_PROOF_NONCE is required for fresh evidence");
    assert!(
        !nonce.trim().is_empty()
            && nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        "MT-022 proof nonce must be non-empty and artifact-safe"
    );

    let proof_dir = configured_artifacts.join(LIVE_PROOF_DIR);
    std::fs::create_dir_all(&proof_dir).expect("create MT-022 proof directory");
    std::env::set_var("HANDSHAKE_PROOF_ARTIFACT_DIR", &proof_dir);
    (proof_dir, data_dir, nonce)
}

fn scoped_author_id(prefix: &str, author_id: &str) -> String {
    format!("{prefix}{author_id}")
}

fn foreground_process_id() -> u32 {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return 0;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    pid
}

#[test]
fn user_manual_route_content_is_reachable_and_argus_inspectable() {
    let factory =
        UserManualPaneFactory::with_transport(Arc::new(FixedManualTransport { fail: false }));
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run_steps(3);

    let ids = author_ids(&harness);
    for expected in [
        SURFACE_AUTHOR_ID.to_string(),
        NAVIGATION_AUTHOR_ID.to_string(),
        PAGE_AUTHOR_ID.to_string(),
        READ_RECEIPT_AUTHOR_ID.to_string(),
        page_author_id("model-runtime-registry-and-loom-degrade"),
    ] {
        assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
    }
    harness.get_by_label(
        "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}.",
    );
    harness.get_by_label("Read receipt: event-user-manual-open-1");
}

#[test]
fn user_manual_uses_the_production_routes_and_navigable_search_targets() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = UserManualClient::new("http://127.0.0.1:37501/", runtime.handle().clone());
    let page = client.page_request("model-runtime-registry-and-loom-degrade");
    assert_eq!(page.method, HttpMethod::Get);
    assert_eq!(
        page.url,
        "http://127.0.0.1:37501/usermanual/pages/model-runtime-registry-and-loom-degrade"
    );
    assert!(page.body.is_none());
    let search = client.search_request("loom recovery & retry");
    assert_eq!(search.method, HttpMethod::Get);
    assert_eq!(
        search.url,
        "http://127.0.0.1:37501/usermanual/search?q=loom+recovery+%26+retry&limit=50"
    );
    assert!(search.body.is_none());

    let controller = UserManualPaneController::default();
    let factory = UserManualPaneFactory::with_transport_and_controller(
        Arc::new(FixedManualTransport { fail: false }),
        controller.clone(),
    );
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run_steps(3);
    controller.request_search_focus();
    harness.run();
    node_by_author(&harness, SEARCH_INPUT_AUTHOR_ID).type_text("loom recovery");
    harness.run();
    node_by_author(&harness, SEARCH_ACTION_AUTHOR_ID).click();
    harness.run_steps(2);

    let result_id = search_hit_author_id(
        "section",
        "model-runtime-registry-and-loom-degrade:recovery",
    );
    let ids = author_ids(&harness);
    for expected in [
        SEARCH_INPUT_AUTHOR_ID.to_string(),
        SEARCH_ACTION_AUTHOR_ID.to_string(),
        SEARCH_STATUS_AUTHOR_ID.to_string(),
        SEARCH_RESULTS_AUTHOR_ID.to_string(),
        result_id.clone(),
    ] {
        assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
    }
    node_by_author(&harness, &result_id).click();
    harness.run_steps(2);
    harness.get_by_label(
        "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}.",
    );
}

#[test]
fn user_manual_loading_state_has_stable_argus_target() {
    let factory = UserManualPaneFactory::with_transport(Arc::new(PendingManualTransport));
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run();

    let ids = author_ids(&harness);
    assert!(ids.contains(&LOADING_AUTHOR_ID.to_string()));
    harness.get_by_label("Loading UserManual content...");
}

#[test]
fn user_manual_unavailable_error_and_retry_states_have_stable_targets() {
    let mut offline =
        Harness::builder().build_state(render, state(UserManualPaneFactory::offline()));
    offline.run();
    assert!(author_ids(&offline).contains(&UNAVAILABLE_AUTHOR_ID.to_string()));

    let factory =
        UserManualPaneFactory::with_transport(Arc::new(FixedManualTransport { fail: true }));
    let mut failed = Harness::builder().build_state(render, state(factory));
    failed.run_steps(2);
    let ids = author_ids(&failed);
    assert!(ids.contains(&ERROR_AUTHOR_ID.to_string()));
    assert!(ids.contains(&RETRY_AUTHOR_ID.to_string()));
}

#[test]
#[ignore = "LIVE production socket proof: requires the current core and an interactive desktop"]
fn production_socket_user_manual_navigation_search_and_pixels() {
    let (proof_dir, data_dir, nonce) = configure_live_proof_roots();
    let foreground_before = foreground_process_id();
    let fault_proxy = LoopbackHttpFaultProxy::start();
    let mut app = LiveApp::start_with_http_proxy("mt022_user_manual", &fault_proxy.url());
    fault_proxy.hold();

    app.client
        .mutation_on_live_surface("argus.click", "main", "menu-run", None);
    wait_for_author_id(
        &mut app.client,
        "main",
        "menu.run.user-manual",
        Duration::from_secs(10),
    );
    app.client
        .mutation_on_live_surface("argus.click", "main", "menu.run.user-manual", None);

    let discovered_surface =
        wait_for_author_id_between(&mut app.client, "main", "", SURFACE_AUTHOR_ID, LIVE_TIMEOUT);
    let pane_prefix = discovered_surface
        .strip_suffix(SURFACE_AUTHOR_ID)
        .expect("surface uses the stable UserManual author id");
    assert!(pane_prefix.is_empty() || pane_prefix.ends_with('.'));

    let loading_id = scoped_author_id(pane_prefix, LOADING_AUTHOR_ID);
    let error_id = scoped_author_id(pane_prefix, ERROR_AUTHOR_ID);
    let retry_id = scoped_author_id(pane_prefix, RETRY_AUTHOR_ID);
    let loading = wait_for_author_id(&mut app.client, "main", &loading_id, LIVE_TIMEOUT);
    assert!(
        node_text(require_node(&loading["snapshot"]["root"], &loading_id))
            .contains("Loading UserManual content"),
        "held production HTTP request did not render the live loading state"
    );
    assert!(
        fault_proxy.held_request_count() >= 1,
        "loading state appeared without an owned held /usermanual socket request"
    );

    fault_proxy.fail();
    let failed = wait_for_author_id(&mut app.client, "main", &error_id, LIVE_TIMEOUT);
    let failed_root = failed["snapshot"]["root"].clone();
    let failure_text = node_text(require_node(&failed_root, &error_id));
    assert!(
        failure_text.contains("503") || failure_text.contains("Service Unavailable"),
        "owned socket 503 did not reach the production UserManual error state: {failure_text}"
    );
    assert!(contains_author_id(&failed_root, &retry_id));
    assert!(
        fault_proxy.failed_request_count() >= 1,
        "error state appeared without an owned failed /usermanual socket request"
    );

    fault_proxy.forward();
    let retry_receipt = app
        .client
        .mutation_on_live_surface("argus.click", "main", &retry_id, None);
    let navigation_id = scoped_author_id(pane_prefix, NAVIGATION_AUTHOR_ID);
    let page_id = scoped_author_id(pane_prefix, PAGE_AUTHOR_ID);
    let receipt_id = scoped_author_id(pane_prefix, READ_RECEIPT_AUTHOR_ID);
    wait_for_author_id(&mut app.client, "main", &navigation_id, LIVE_TIMEOUT);
    wait_for_author_id(&mut app.client, "main", &page_id, LIVE_TIMEOUT);
    let settled = wait_for_author_id(&mut app.client, "main", &receipt_id, LIVE_TIMEOUT);
    assert!(
        fault_proxy.forwarded_request_count() >= 1,
        "retry recovery appeared without a forwarded /usermanual socket request"
    );
    let settled_root = settled["snapshot"]["root"].clone();
    assert!(node_text(require_node(&settled_root, &navigation_id)).contains("navigation"));
    assert!(node_text(require_node(&settled_root, &page_id)).contains("page"));
    assert!(node_text(require_node(&settled_root, &receipt_id)).contains("Read receipt:"));

    let ids = collect_author_ids(&settled_root);
    let unique = ids.iter().collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), unique.len(), "live author ids must be unique");

    let search_input_id = scoped_author_id(pane_prefix, SEARCH_INPUT_AUTHOR_ID);
    let search_action_id = scoped_author_id(pane_prefix, SEARCH_ACTION_AUTHOR_ID);
    let search_status_id = scoped_author_id(pane_prefix, SEARCH_STATUS_AUTHOR_ID);
    let search_results_id = scoped_author_id(pane_prefix, SEARCH_RESULTS_AUTHOR_ID);
    let set_value_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &search_input_id,
        Some(("value", json!("backlinks"))),
    );
    let search_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", &search_action_id, None);
    let searched = wait_for_author_id(&mut app.client, "main", &search_status_id, LIVE_TIMEOUT);
    let searched_root = searched["snapshot"]["root"].clone();
    assert!(contains_author_id(&searched_root, &search_results_id));
    assert!(node_text(require_node(&searched_root, &search_status_id)).contains("backlinks"));
    let result_prefix = format!("{pane_prefix}user-manual.search.result.");
    let result_id =
        wait_for_author_id_between(&mut app.client, "main", &result_prefix, "", LIVE_TIMEOUT);
    app.client
        .mutation_on_live_surface("argus.click", "main", &result_id, None);
    wait_for_author_id(&mut app.client, "main", &receipt_id, LIVE_TIMEOUT);

    let foreground_after = foreground_process_id();
    assert_ne!(
        foreground_before, 0,
        "MT-022 focus proof requires an identifiable foreground owner"
    );
    assert_eq!(
        foreground_after, foreground_before,
        "socket-driven UserManual proof changed the operator's foreground owner"
    );
    assert_ne!(
        foreground_after, app.child_pid,
        "socket-driven UserManual proof must not activate the production child"
    );

    let capture = app.client.screenshot("main");
    let png = decode_verified_capture(
        &capture,
        "main",
        app.child_pid,
        "MT-022 UserManual production socket capture",
    );
    let transcript = app.client.assert_transcript_is_secret_free(&[]);
    let screenshot_path = app.write_proof_artifact("user_manual_production_socket.png", &png);
    let transcript_path =
        app.write_proof_artifact("user_manual_production_socket_transcript.json", &transcript);
    let provenance = serde_json::to_vec_pretty(&json!({
        "schema_id": "handshake.mt022.user_manual_production_socket@1",
        "proof_nonce": nonce,
        "artifact_root": proof_dir,
        "embedded_data_dir": data_dir,
        "child_pid": app.child_pid,
        "authenticated_agent_id": app.authenticated_agent_id,
        "foreground_pid_before": foreground_before,
        "foreground_pid_after": foreground_after,
        "navigation": ["menu-run", "menu.run.user-manual"],
        "socket_state_sequence": ["held_loading", "http_503_error", "forwarded_retry_recovery"],
        "held_socket_requests": fault_proxy.held_request_count(),
        "failed_socket_requests": fault_proxy.failed_request_count(),
        "forwarded_socket_requests": fault_proxy.forwarded_request_count(),
        "surface_author_id": discovered_surface,
        "loading_author_id": loading_id,
        "error_author_id": error_id,
        "retry_author_id": retry_id,
        "navigation_author_id": navigation_id,
        "page_author_id": page_id,
        "read_receipt_author_id": receipt_id,
        "search_result_author_id": result_id,
        "retry_evidence_ref": retry_receipt["result"]["evidence_ref"],
        "set_value_evidence_ref": set_value_receipt["result"]["evidence_ref"],
        "search_evidence_ref": search_receipt["result"]["evidence_ref"],
        "screenshot": screenshot_path,
        "transcript": transcript_path,
    }))
    .expect("serialize fresh MT-022 provenance");
    app.write_proof_artifact("user_manual_production_socket_provenance.json", &provenance);
    app.shutdown();
}
