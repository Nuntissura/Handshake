//! WP-1 MT-014: Argus + pixel proof for the Rust-native ModelRuntime panel.
//!
//! Artifact and pixel proofs read producer-attested JSON emitted by
//! `model_runtime_registry_api_tests`. The selection proof independently boots
//! the real Handshake route over an isolated managed-PostgreSQL schema and
//! drives it through the production native transport.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use egui::accesskit::Action;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use handshake_core::test_harness::model_runtime_selection::Mt014NativeCompositeProof;
use handshake_native::{
    app::{HandshakeApp, HealthDisplayState},
    backend_client::{HealthInfo, ModelRuntimeRegistryClient},
    mcp::{build_action_request, UiAction},
    model_runtime_panel::{
        refresh_author_id, row_action_author_id, row_active_purposes_author_id,
        row_active_selection_author_id, row_adapter_author_id, row_artifact_path_author_id,
        row_audit_author_id, row_author_id, row_default_ineligible_author_id,
        row_dormant_reason_author_id, row_engine_internals_author_id,
        row_engine_internals_expand_author_id, row_kv_cache_author_id,
        row_last_call_age_author_id, row_last_call_author_id,
        row_ledger_link_author_id, row_live_model_author_id, row_locator_author_id,
        row_lora_author_id, row_revision_author_id, row_role_author_id, row_sha_author_id,
        row_state_author_id, row_steering_author_id, row_switch_author_id,
        row_tokens_per_second_author_id, row_vram_author_id, status_author_id, surface_author_id,
        take_process_ledger_navigation_request, validate_projection_for_native_surface,
        ModelRuntimeControlAction, ModelRuntimePaneFactory, ModelRuntimeRegistryCell,
        ModelRuntimeRegistryProjection, ModelRuntimeRegistryRowState,
        ModelRuntimeRegistryTransport, ModelRuntimeRole, AUTHOR_ID_PREFIX, PROJECTION_SCHEMA_ID,
    },
    module_switcher::ModuleId,
    pane_registry::PaneType,
    rails::SCROLLBAR_V_NODE_IDS,
    tab_bar::{TabBarState, TabState},
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

const PRIMARY_MODEL_RUNTIME_PANE_ID: &str = "pane-a";
const SECONDARY_MODEL_RUNTIME_PANE_ID: &str = "pane-b";
const PROJECTION_FILE: &str = "mt014-model-runtime-registry-projection.json";
const SCREENSHOT_FILE: &str = "mt014-model-runtime-panel.png";
const PROJECTION_PROVENANCE_SCHEMA_ID: &str = "hsk.mt014_model_runtime_projection_provenance@1";
const MAX_PRODUCER_AGE_MS: u64 = 30 * 60 * 1_000;

#[derive(Clone, Debug, Deserialize)]
struct ModelRuntimeProjectionProvenance {
    schema_id: String,
    proof_nonce: String,
    projection_schema_id: String,
    artifact_sha256: String,
    producer_test_id: String,
    producer_status: String,
    producer_completed_at_unix_ms: u64,
}

fn canonical_artifacts_root() -> Result<PathBuf, String> {
    let configured = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .map_err(|_| "HANDSHAKE_ARTIFACTS_DIR must name the sibling Handshake_Artifacts root")?;
    let configured = std::fs::canonicalize(configured)
        .map_err(|error| format!("canonicalize HANDSHAKE_ARTIFACTS_DIR: {error}"))?;
    let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .map_err(|error| format!("canonicalize native crate manifest: {error}"))?;
    let worktree_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .ok_or_else(|| "native crate must live below the worktree src directory".to_owned())?;
    let expected = std::fs::canonicalize(
        worktree_root
            .parent()
            .ok_or_else(|| "worktree must have a parent".to_owned())?
            .join("Handshake_Artifacts"),
    )
    .map_err(|error| format!("canonicalize sibling Handshake_Artifacts: {error}"))?;
    if configured != expected {
        return Err(format!(
            "HANDSHAKE_ARTIFACTS_DIR must be the canonical sibling root: expected {}, got {}",
            expected.display(),
            configured.display()
        ));
    }
    Ok(configured)
}

fn canonical_artifact_path(file_name: &str) -> Result<PathBuf, String> {
    Ok(canonical_artifacts_root()?
        .join("handshake-test")
        .join("wp1-final-audit")
        .join(file_name))
}

fn require_configured_path(variable: &str, expected: &std::path::Path) -> Result<(), String> {
    let configured = PathBuf::from(
        std::env::var(variable).map_err(|_| format!("{variable} is required for MT-014 proof"))?,
    );
    validate_configured_artifact_path(&configured, expected, variable)
}

fn validate_configured_artifact_path(
    configured: &std::path::Path,
    expected: &std::path::Path,
    variable: &str,
) -> Result<(), String> {
    let configured = match (configured.parent(), configured.file_name()) {
        (Some(parent), Some(file_name)) => std::fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .unwrap_or_else(|_| configured.to_path_buf()),
        _ => configured.to_path_buf(),
    };
    if configured != expected {
        return Err(format!(
            "{variable} must name canonical artifact {}; got {}",
            expected.display(),
            configured.display()
        ));
    }
    Ok(())
}

fn validate_projection_provenance(
    provenance: &ModelRuntimeProjectionProvenance,
    artifact_bytes: &[u8],
    expected_nonce: &str,
    projection_schema_id: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if provenance.schema_id != PROJECTION_PROVENANCE_SCHEMA_ID {
        return Err(format!(
            "unsupported provenance schema `{}`",
            provenance.schema_id
        ));
    }
    if provenance.proof_nonce != expected_nonce {
        return Err("projection provenance nonce does not match this producer run".to_owned());
    }
    if provenance.projection_schema_id != projection_schema_id {
        return Err("projection schema does not match producer provenance".to_owned());
    }
    if provenance.producer_test_id
        != "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256"
    {
        return Err("projection provenance names an unexpected producer".to_owned());
    }
    if provenance.producer_status != "passed_all_backend_assertions" {
        return Err("projection producer did not complete every backend assertion".to_owned());
    }
    if provenance.producer_completed_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err("projection provenance timestamp is in the future".to_owned());
    }
    if now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) > MAX_PRODUCER_AGE_MS {
        return Err("projection provenance is stale".to_owned());
    }
    let actual_sha256 = format!("{:x}", Sha256::digest(artifact_bytes));
    if provenance.artifact_sha256 != actual_sha256 {
        return Err("projection bytes do not match producer provenance".to_owned());
    }
    Ok(())
}

fn real_projection_with_provenance() -> (
    ModelRuntimeRegistryProjection,
    ModelRuntimeProjectionProvenance,
) {
    let path = canonical_artifact_path(PROJECTION_FILE)
        .expect("resolve canonical MT-014 projection artifact path");
    require_configured_path("HANDSHAKE_MT014_PROJECTION_ARTIFACT", &path)
        .expect("configured MT-014 projection path is canonical");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("read real registry projection {}: {error}", path.display())
    });
    let projection: ModelRuntimeRegistryProjection =
        serde_json::from_slice(&bytes).unwrap_or_else(|error| {
            panic!(
                "deserialize real registry projection {}: {error}",
                path.display()
            )
        });
    let provenance: ModelRuntimeProjectionProvenance = serde_json::from_slice(
        &std::fs::read(path.with_extension("provenance.json"))
            .expect("read MT-014 projection provenance"),
    )
    .expect("MT-014 projection provenance is typed JSON");
    let expected_nonce = std::env::var("HANDSHAKE_MT014_PROOF_NONCE")
        .expect("HANDSHAKE_MT014_PROOF_NONCE is required by the native proof");
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    validate_projection_provenance(
        &provenance,
        &bytes,
        &expected_nonce,
        &projection.schema_id,
        now_unix_ms,
    )
    .expect("MT-014 native proof consumes a fresh backend-produced projection");
    validate_projection_for_native_surface(&projection)
        .expect("real backend projection satisfies the native surface contract");
    (projection, provenance)
}

fn real_projection() -> ModelRuntimeRegistryProjection {
    real_projection_with_provenance().0
}

#[test]
fn mt014_projection_provenance_rejects_stale_nonce_root_status_and_hash() {
    let bytes = br#"{"schema_id":"hsk.model_runtime_registry_projection@3"}"#;
    let now = 2_000_000_000_u64;
    let mut provenance = ModelRuntimeProjectionProvenance {
        schema_id: PROJECTION_PROVENANCE_SCHEMA_ID.to_owned(),
        proof_nonce: "current-run".to_owned(),
        projection_schema_id: PROJECTION_SCHEMA_ID.to_owned(),
        artifact_sha256: format!("{:x}", Sha256::digest(bytes)),
        producer_test_id:
            "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256".to_owned(),
        producer_status: "passed_all_backend_assertions".to_owned(),
        producer_completed_at_unix_ms: now - 1_000,
    };
    validate_projection_provenance(&provenance, bytes, "current-run", PROJECTION_SCHEMA_ID, now)
        .expect("fresh matching producer provenance is accepted");

    provenance.proof_nonce = "old-run".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("nonce"));
    provenance.proof_nonce = "current-run".to_owned();

    provenance.producer_completed_at_unix_ms = now - MAX_PRODUCER_AGE_MS - 1;
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("stale"));
    provenance.producer_completed_at_unix_ms = now - 1_000;

    provenance.producer_status = "failed_before_publish".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("did not complete"));
    provenance.producer_status = "passed_all_backend_assertions".to_owned();

    provenance.artifact_sha256 = "0".repeat(64);
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("bytes"));

    assert!(validate_configured_artifact_path(
        std::path::Path::new("D:/wrong-root/projection.json"),
        std::path::Path::new("D:/canonical-root/projection.json"),
        "HANDSHAKE_MT014_PROJECTION_ARTIFACT"
    )
    .unwrap_err()
    .contains("canonical artifact"));
}

fn app_with_real_projection(projection: ModelRuntimeRegistryProjection) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(348),
    }));
    app.set_pane_factory(
        PaneType::ModelRuntime,
        Box::new(ModelRuntimePaneFactory::with_projection(projection)),
    );
    assert!(app.set_module(ModuleId::Studio));
    app
}

fn app_with_two_model_runtime_panes(projection: ModelRuntimeRegistryProjection) -> HandshakeApp {
    let mut app = app_with_real_projection(projection);
    let secondary_pane_id = app
        .tab_bar_states()
        .keys()
        .find(|pane_id| pane_id.as_ref() == SECONDARY_MODEL_RUNTIME_PANE_ID)
        .cloned()
        .expect("the real split layout owns pane-b");
    app.tab_bar_states_mut().insert(
        secondary_pane_id.clone(),
        TabBarState::new(
            secondary_pane_id,
            vec![TabState::new(PaneType::ModelRuntime)],
        ),
    );
    app
}

struct OrderedRegistryTransport {
    deliveries: Mutex<VecDeque<Result<ModelRuntimeRegistryProjection, String>>>,
}

impl OrderedRegistryTransport {
    fn success_then_failure(projection: ModelRuntimeRegistryProjection, error: &str) -> Self {
        Self {
            deliveries: Mutex::new(VecDeque::from([Ok(projection), Err(error.to_owned())])),
        }
    }
}

impl ModelRuntimeRegistryTransport for OrderedRegistryTransport {
    fn fetch_registry(&self, cell: ModelRuntimeRegistryCell) {
        let delivery = self
            .deliveries
            .lock()
            .expect("registry delivery order lock is available")
            .pop_front()
            .expect("the proof declares one delivery per fetch");
        *cell.lock().expect("registry delivery cell is available") = Some(delivery);
    }
}

fn app_with_registry_transport(transport: Arc<dyn ModelRuntimeRegistryTransport>) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(348),
    }));
    app.set_pane_factory(
        PaneType::ModelRuntime,
        Box::new(ModelRuntimePaneFactory::with_transport(transport)),
    );
    assert!(app.set_module(ModuleId::Studio));
    app
}

fn projection_http_server(
    projection: &ModelRuntimeRegistryProjection,
) -> (std::thread::JoinHandle<String>, String) {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind loopback ModelRuntime projection server");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("read loopback ModelRuntime server address")
    );
    let body = serde_json::to_vec(projection).expect("serialize real PostgreSQL projection");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("accept production ModelRuntime client request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let read = stream
                .read(&mut chunk)
                .expect("read production ModelRuntime client request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request_line = String::from_utf8_lossy(&request)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .expect("write ModelRuntime response headers");
        stream
            .write_all(&body)
            .expect("write real PostgreSQL projection response");
        stream.flush().expect("flush ModelRuntime response");
        request_line
    });
    (server, base_url)
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn live_labels(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

fn assert_author_id(harness: &Harness<'_, HandshakeApp>, author_id: &str) {
    assert!(
        live_author_ids(harness)
            .iter()
            .any(|candidate| candidate == author_id),
        "{author_id} missing from live AccessKit tree: {:?}",
        live_author_ids(harness)
    );
}

fn assert_unique_model_runtime_author_ids(harness: &Harness<'_, HandshakeApp>) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for author_id in live_author_ids(harness)
        .into_iter()
        .filter(|author_id| author_id.starts_with(&format!("{AUTHOR_ID_PREFIX}.")))
    {
        *counts.entry(author_id).or_insert(0) += 1;
    }
    let duplicates = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .collect::<Vec<_>>();
    assert!(
        duplicates.is_empty(),
        "ModelRuntime panel author IDs must be unique: {duplicates:?}"
    );
}

fn assert_positive_author_bounds(harness: &Harness<'_, HandshakeApp>, author_id: &str) {
    let bounds = author_bounds(harness, author_id);
    assert!(
        bounds.width() > 0.0 && bounds.height() > 0.0,
        "{author_id} has non-positive bounds {bounds:?}"
    );
}

fn author_bounds(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> egui::accesskit::Rect {
    let bounds = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .and_then(|node| {
            node.accesskit_node()
                .bounding_box()
                .or_else(|| node.accesskit_node().raw_bounds())
        })
        .unwrap_or_else(|| panic!("{author_id} has no consumer-visible bounds"));
    bounds
}

fn click_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("{author_id} missing from live AccessKit tree"));
    assert!(
        node.accesskit_node().data().supports_action(Action::Click),
        "{author_id} must expose the real button Click action"
    );
    let outcome = build_action_request(node.accesskit_node().id(), &UiAction::Click);
    harness.event(egui::Event::AccessKitActionRequest(outcome.request));
}

fn assert_action_disabled(harness: &Harness<'_, HandshakeApp>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("{author_id} missing from live AccessKit tree"));
    assert!(
        !node.accesskit_node().data().supports_action(Action::Click),
        "{author_id} must not expose Click while the backend reports the action unavailable"
    );
}

fn assert_action_enabled(harness: &Harness<'_, HandshakeApp>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("{author_id} missing from live AccessKit tree"));
    assert!(
        node.accesskit_node().data().supports_action(Action::Click),
        "{author_id} must expose Click when the backend reports quiesce available"
    );
}

fn wait_for_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    for _ in 0..500 {
        // The production ModelRuntime pane requests repaint while its async
        // transport is in flight. Advance exactly one frame per poll so the
        // bounded waiter, rather than `Harness::run`'s four-frame settle cap,
        // owns the timeout.
        harness.step();
        if live_author_ids(harness)
            .iter()
            .any(|candidate| candidate == author_id)
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!(
        "timed out waiting for {author_id}; live author IDs: {:?}",
        live_author_ids(harness)
    );
}

fn await_registry_delivery(
    cell: &ModelRuntimeRegistryCell,
) -> Result<ModelRuntimeRegistryProjection, String> {
    for _ in 0..1_500 {
        if let Some(result) = cell
            .lock()
            .expect("production registry delivery cell is available")
            .take()
        {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err("timed out waiting for production ModelRuntime transport delivery".to_owned())
}

fn fetch_through_production_transport(
    client: &ModelRuntimeRegistryClient,
) -> Result<ModelRuntimeRegistryProjection, String> {
    let cell = Arc::new(Mutex::new(None));
    client.fetch_registry(cell.clone());
    await_registry_delivery(&cell)
}

fn select_through_production_transport(
    client: &ModelRuntimeRegistryClient,
    target_model_id: &str,
) -> Result<ModelRuntimeRegistryProjection, String> {
    let cell = Arc::new(Mutex::new(None));
    client.select_model(target_model_id.to_owned(), cell.clone());
    await_registry_delivery(&cell)
}

#[test]
fn mt014_argus_operator_menu_fetches_real_pg_projection_through_production_transport() {
    let projection = real_projection();
    let (server, base_url) = projection_http_server(&projection);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build production-client proof runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(354),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|context, app: &mut HandshakeApp| app.ui(context), app);

    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    assert_author_id(&harness, "menu.models.model-runtime");
    harness.get_by_label("Open Model Runtime").click();
    for _ in 0..100 {
        harness.step();
        if live_labels(&harness)
            .iter()
            .any(|label| label.contains("live |") && label.contains("dormant |"))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(
        harness.state().active_module() == ModuleId::Studio,
        "RUN > Open Model Runtime must activate the STUDIO module"
    );
    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::ModelRuntime)),
        "RUN > Open Model Runtime must navigate to the production pane"
    );
    assert_author_id(&harness, &surface_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID));
    for row in &projection.rows {
        assert_author_id(
            &harness,
            &row_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &row.artifact_sha256),
        );
    }
    assert_eq!(
        server.join().expect("projection server joins"),
        "GET /model-runtime/registry HTTP/1.1",
        "the operator click path must use the production registry route"
    );
}

#[test]
fn mt014_stable_switch_author_id_posts_then_reobserves_backend_projection() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build MT-014 production-composite runtime");
    let proof = runtime
        .block_on(Mt014NativeCompositeProof::start())
        .expect("start real MT-014 managed-PostgreSQL composite proof");
    assert!(proof.managed_postgres_is_proven());
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(356),
    }));
    app.set_backend_base_url_for_test(proof.base_url(), runtime.handle().clone());
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|context, app: &mut HandshakeApp| app.ui(context), app);

    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Model Runtime").click();
    let switch_id =
        row_switch_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &proof.target_artifact_sha256);
    wait_for_author_id(&mut harness, &switch_id);
    assert_author_id(
        &harness,
        &row_role_author_id(
            PRIMARY_MODEL_RUNTIME_PANE_ID,
            &proof.embedding_artifact_sha256,
        ),
    );
    assert_author_id(
        &harness,
        &row_default_ineligible_author_id(
            PRIMARY_MODEL_RUNTIME_PANE_ID,
            &proof.embedding_artifact_sha256,
        ),
    );
    assert!(
        !live_author_ids(&harness).iter().any(|author_id| author_id
            == &row_switch_author_id(
                PRIMARY_MODEL_RUNTIME_PANE_ID,
                &proof.embedding_artifact_sha256,
            )),
        "the real embedding-role row must not expose an AccessKit Switch action"
    );

    click_author_id(&mut harness, &switch_id);
    let active_id = row_active_selection_author_id(
        PRIMARY_MODEL_RUNTIME_PANE_ID,
        &proof.target_artifact_sha256,
    );
    wait_for_author_id(&mut harness, &active_id);
    assert_author_id(&harness, &active_id);
    assert_eq!(proof.selected_model_id(), proof.target_model_id);
    assert_eq!(proof.selection_event_count(), 1);
    assert!(proof.has_native_selection_event(&proof.target_model_id));

    let client = ModelRuntimeRegistryClient::new(proof.base_url(), runtime.handle().clone());
    let reobserved = fetch_through_production_transport(&client)
        .expect("GET active durable projection through the production native transport");
    assert!(reobserved.rows.iter().any(|row| {
        row.artifact_sha256 == proof.target_artifact_sha256
            && row.live_model_id.as_deref() == Some(proof.target_model_id.as_str())
            && row.selected
            && row.default_selectable
    }));
    assert!(reobserved.rows.iter().any(|row| {
        row.artifact_sha256 == proof.current_artifact_sha256
            && row.live_model_id.as_deref() == Some(proof.current_model_id.as_str())
            && !row.selected
    }));

    let stale_error = select_through_production_transport(&client, &proof.stale_model_id)
        .expect_err("stale target must fail through the production POST route");
    assert!(stale_error.contains("not a current READY registry row"));
    assert_eq!(proof.selected_model_id(), proof.target_model_id);

    let embedding_error = select_through_production_transport(&client, &proof.embedding_model_id)
        .expect_err("embedding-role target must fail through the production POST route");
    assert!(
        embedding_error.contains("Embedding") && embedding_error.contains("not eligible"),
        "unexpected embedding-role rejection: {embedding_error}"
    );
    assert_eq!(proof.selected_model_id(), proof.target_model_id);

    proof.drift_current_catalog_role_to_embedding();
    let integrity_error = select_through_production_transport(&client, &proof.current_model_id)
        .expect_err("role-integrity drift must fail before the production runtime swap");
    assert!(
        integrity_error.contains("runtime role") && integrity_error.contains("disagrees"),
        "unexpected role-integrity rejection: {integrity_error}"
    );
    assert_eq!(proof.selected_model_id(), proof.target_model_id);
    proof.restore_current_catalog_role_to_completion();

    proof.set_audit_failure(true);
    let audit_error = select_through_production_transport(&client, &proof.current_model_id)
        .expect_err("Flight Recorder failure must fail before the production runtime swap");
    assert!(
        audit_error.contains("model selection audit failed before switch"),
        "unexpected audit-failure rejection: {audit_error}"
    );
    assert_eq!(proof.selected_model_id(), proof.target_model_id);
    assert_eq!(proof.selection_event_count(), 1);
    proof.set_audit_failure(false);

    let final_projection = fetch_through_production_transport(&client)
        .expect("re-observe active projection after every fail-closed boundary");
    assert_eq!(
        final_projection
            .rows
            .iter()
            .find(|row| row.selected)
            .and_then(|row| row.live_model_id.as_deref()),
        Some(proof.target_model_id.as_str()),
        "stale, embedding-role, integrity, and audit failures must preserve the active model"
    );
}

#[test]
fn mt014_argus_renders_real_pg_live_and_dormant_registry_rows() {
    let projection = real_projection();
    assert!(
        projection
            .rows
            .iter()
            .any(|row| row.runtime_state == ModelRuntimeRegistryRowState::Live),
        "real PostgreSQL projection must contain a current READY row"
    );
    assert!(
        projection
            .rows
            .iter()
            .any(|row| row.runtime_state == ModelRuntimeRegistryRowState::Dormant),
        "real PostgreSQL projection must contain a dormant row"
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_real_projection(projection.clone()),
        );
    harness.run();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::ModelRuntime)),
        "STUDIO opens the real Rust-native ModelRuntime pane"
    );
    for author_id in [
        surface_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
        refresh_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
        status_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
    ] {
        assert_author_id(&harness, &author_id);
    }

    let labels = live_labels(&harness);
    for row in &projection.rows {
        let artifact = row.artifact_sha256.as_str();
        for author_id in [
            row_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_state_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_adapter_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_revision_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_sha_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_locator_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_artifact_path_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_kv_cache_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_lora_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_steering_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_tokens_per_second_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_vram_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_last_call_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_last_call_age_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_engine_internals_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_ledger_link_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "quiesce"),
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "unload"),
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "adapter-swap"),
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "inspect-internals"),
            row_audit_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
        ] {
            assert_author_id(&harness, &author_id);
        }
        if matches!(
            &row.engine_internals,
            handshake_native::model_runtime_panel::ModelRuntimeValue::Available { .. }
        ) {
            assert_author_id(
                &harness,
                &row_engine_internals_expand_author_id(
                    PRIMARY_MODEL_RUNTIME_PANE_ID,
                    artifact,
                ),
            );
        }
        for (action, availability) in [
            ("unload", &row.unload_action),
            ("adapter-swap", &row.compatible_adapter_swap_action),
        ] {
            let author_id =
                row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, action);
            if availability.enabled {
                assert_action_enabled(&harness, &author_id);
            } else {
                assert_action_disabled(&harness, &author_id);
            }
        }
        let quiesce_author_id =
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "quiesce");
        if row.quiesce_action.enabled {
            assert_action_enabled(&harness, &quiesce_author_id);
        } else {
            assert_action_disabled(&harness, &quiesce_author_id);
        }
        let inspect_author_id =
            row_action_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact, "inspect-internals");
        if matches!(
            &row.engine_internals,
            handshake_native::model_runtime_panel::ModelRuntimeValue::Available { .. }
        ) {
            assert_action_enabled(&harness, &inspect_author_id);
        } else {
            assert_action_disabled(&harness, &inspect_author_id);
        }
        if !row.active_purposes.is_empty() {
            assert_author_id(
                &harness,
                &row_active_purposes_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            );
        }
        assert!(
            labels
                .iter()
                .any(|label| label.contains(&row.display_label)),
            "display label is operator-readable: {:?}",
            labels
        );
        assert!(
            labels
                .iter()
                .any(|label| label
                    .contains(&format!("Selection revision: {}", row.selection_revision))),
            "selection revision is visible"
        );
        assert!(
            labels
                .iter()
                .any(|label| label.contains(&row.selection_audit_event_ref)),
            "selection audit EventLedger reference is visible"
        );
        let adapter_label = match row.selected_adapter.as_str() {
            "candle" => "CandleRuntime",
            "llama_cpp" => "LlamaCppRuntime",
            other => panic!("backend emitted unsupported adapter {other}"),
        };
        assert!(
            labels.iter().any(|label| label.contains(adapter_label)),
            "selected adapter is visible as {adapter_label}"
        );
        match row.runtime_state {
            ModelRuntimeRegistryRowState::Live => {
                assert_author_id(
                    &harness,
                    &row_live_model_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
                );
                assert!(row.live_model_id.is_some());
            }
            ModelRuntimeRegistryRowState::Dormant => {
                assert_author_id(
                    &harness,
                    &row_dormant_reason_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
                );
                assert!(row.live_model_id.is_none());
                assert!(
                    !live_author_ids(&harness).iter().any(|id| id
                        == &row_live_model_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact)),
                    "dormant row must not expose a stale loaded-model target"
                );
            }
        }
        if row.selected {
            assert_author_id(
                &harness,
                &row_active_selection_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, artifact),
            );
            assert!(
                labels.iter().any(|label| label == "ACTIVE DEFAULT MODEL"),
                "the active READY model is explicit in the native pane"
            );
        }
    }

    let ledger_row = projection
        .rows
        .iter()
        .find_map(|row| match &row.process_ownership_ledger_link {
            handshake_native::model_runtime_panel::ModelRuntimeValue::Available { value } => {
                Some((&row.artifact_sha256, value))
            }
            handshake_native::model_runtime_panel::ModelRuntimeValue::Unavailable { .. } => None,
        })
        .expect("real projection has a live ProcessOwnershipLedger link");
    assert!(take_process_ledger_navigation_request().is_none());
    click_author_id(
        &mut harness,
        &row_ledger_link_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, ledger_row.0),
    );
    assert_eq!(
        take_process_ledger_navigation_request().as_deref(),
        Some(ledger_row.1.as_str()),
        "ModelRuntime ledger control must request in-app Flight Recorder navigation"
    );

    assert_unique_model_runtime_author_ids(&harness);

    // Constrained viewport: the same real PostgreSQL projection remains
    // navigable without a desktop-only layout assumption. The panel uses a
    // vertical scroll surface, while its refresh/status and first row's
    // state/adapter targets retain concrete consumer-visible bounds.
    let mut constrained = Harness::builder()
        .with_size(egui::Vec2::new(760.0, 520.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_real_projection(projection.clone()),
        );
    constrained.run();
    constrained.run();
    for author_id in [
        surface_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
        refresh_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
        status_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
    ] {
        assert_author_id(&constrained, &author_id);
    }
    for row in &projection.rows {
        assert_author_id(
            &constrained,
            &row_state_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &row.artifact_sha256),
        );
        assert_author_id(
            &constrained,
            &row_adapter_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &row.artifact_sha256),
        );
    }
    assert_positive_author_bounds(
        &constrained,
        &refresh_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
    );
    assert_positive_author_bounds(
        &constrained,
        &status_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
    );
    let first_artifact = &projection.rows[0].artifact_sha256;
    assert_positive_author_bounds(
        &constrained,
        &row_state_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, first_artifact),
    );
    assert_positive_author_bounds(
        &constrained,
        &row_adapter_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, first_artifact),
    );
    assert_unique_model_runtime_author_ids(&constrained);

    let (scrollbar_author_id, _) = SCROLLBAR_V_NODE_IDS[0];
    let initial_scrollbar = constrained
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(scrollbar_author_id))
        .unwrap_or_else(|| panic!("{scrollbar_author_id} missing from constrained live tree"));
    assert_eq!(
        initial_scrollbar.accesskit_node().role(),
        egui::accesskit::Role::ScrollBar
    );
    assert!(initial_scrollbar
        .accesskit_node()
        .data()
        .supports_action(Action::SetValue));
    assert!(initial_scrollbar
        .accesskit_node()
        .data()
        .supports_action(Action::ScrollDown));
    assert!(
        initial_scrollbar
            .accesskit_node()
            .numeric_value()
            .is_some_and(|value| value.abs() < 1e-6),
        "constrained ModelRuntime rail must begin at the top"
    );

    let final_artifact = &projection
        .rows
        .last()
        .expect("real projection has a final durable row")
        .artifact_sha256;
    let final_state_author_id = row_state_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, final_artifact);
    let final_state_node_id = {
        let node = constrained
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(final_state_author_id.as_str()))
            .unwrap_or_else(|| panic!("{final_state_author_id} missing before model scroll"));
        assert!(node
            .accesskit_node()
            .data()
            .supports_action(Action::ScrollIntoView));
        node.accesskit_node().id()
    };
    let scroll_outcome = build_action_request(final_state_node_id, &UiAction::Scroll);
    assert_eq!(scroll_outcome.request.action, Action::ScrollIntoView);
    assert_eq!(scroll_outcome.request.target, final_state_node_id);
    assert_eq!(scroll_outcome.text_payload, None);
    constrained.event(egui::Event::AccessKitActionRequest(scroll_outcome.request));
    constrained.run();
    constrained.run();

    let scrolled_value = constrained
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(scrollbar_author_id))
        .and_then(|node| node.accesskit_node().numeric_value())
        .expect("scrolled ModelRuntime rail exposes its numeric offset");
    assert!(
        scrolled_value > 0.0,
        "the product ScrollIntoView action must move the constrained panel; got {scrolled_value}"
    );
    let pane_bounds = author_bounds(&constrained, PRIMARY_MODEL_RUNTIME_PANE_ID);
    let status_bounds = author_bounds(
        &constrained,
        &status_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID),
    );
    for author_id in [
        final_state_author_id,
        row_adapter_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, final_artifact),
    ] {
        let bounds = author_bounds(&constrained, &author_id);
        assert!(
            bounds.x1 > pane_bounds.x0
                && bounds.x0 < pane_bounds.x1
                && bounds.y0 >= status_bounds.y1
                && bounds.y1 <= pane_bounds.y1,
            "{author_id} must be visible below status chrome after product model-scroll dispatch: row={bounds:?}, status={status_bounds:?}, pane={pane_bounds:?}"
        );
    }

    // Keep the production client route coupled to the proven backend contract.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build request-shape runtime");
    let client =
        ModelRuntimeRegistryClient::new("http://127.0.0.1:37501/", runtime.handle().clone());
    assert_eq!(
        client.registry_request().url,
        "http://127.0.0.1:37501/model-runtime/registry"
    );
    let selection = client.selection_request("0195f783-8ce0-7000-8000-000000000001");
    assert_eq!(
        selection.url,
        "http://127.0.0.1:37501/model-runtime/selection"
    );
    assert_eq!(
        selection.method,
        handshake_native::backend_client::HttpMethod::Post
    );
    assert_eq!(
        selection
            .body
            .as_ref()
            .and_then(|body| body.get("target_model_id"))
            .and_then(serde_json::Value::as_str),
        Some("0195f783-8ce0-7000-8000-000000000001")
    );
    let control_request_id = uuid::Uuid::parse_str("0195f783-8ce0-7000-8000-000000000002")
        .expect("fixed control request id");
    let quiesce = client.quiesce_request(
        "0195f783-8ce0-7000-8000-000000000001",
        control_request_id,
    );
    assert_eq!(
        quiesce.url,
        "http://127.0.0.1:37501/model-runtime/control"
    );
    assert_eq!(
        quiesce.method,
        handshake_native::backend_client::HttpMethod::Post
    );
    assert_eq!(
        quiesce.body,
        Some(json!({
            "schema_version": 1,
            "request_id": control_request_id,
            "model_id": "0195f783-8ce0-7000-8000-000000000001",
            "action": { "action": "quiesce" },
            "timeout_ms": 5_000,
            "expected_catalog_revision": null,
            "expected_selection_revision": null
        }))
    );
    let unload = client.control_request(
        "0195f783-8ce0-7000-8000-000000000001",
        control_request_id,
        &ModelRuntimeControlAction::Unload,
        Some(17),
        None,
    );
    assert_eq!(
        unload.body.as_ref().and_then(|body| body.get("action")),
        Some(&json!({ "action": "unload" }))
    );
    assert_eq!(
        unload
            .body
            .as_ref()
            .and_then(|body| body.get("expected_catalog_revision"))
            .and_then(serde_json::Value::as_u64),
        Some(17)
    );
    let swap = client.control_request(
        "0195f783-8ce0-7000-8000-000000000001",
        control_request_id,
        &ModelRuntimeControlAction::SwapCompatibleAdapter {
            target_adapter: "candle".to_owned(),
        },
        Some(17),
        Some(9),
    );
    assert_eq!(
        swap.body.as_ref().and_then(|body| body.get("action")),
        Some(&json!({
            "action": "swap_compatible_adapter",
            "target_adapter": "candle"
        }))
    );
    assert_eq!(
        swap.body
            .as_ref()
            .and_then(|body| body.get("expected_selection_revision"))
            .and_then(serde_json::Value::as_u64),
        Some(9)
    );
}

#[test]
fn mt014_embedding_role_row_has_no_default_switch_action() {
    let mut projection = real_projection();
    let target = projection
        .rows
        .iter_mut()
        .find(|row| row.runtime_state == ModelRuntimeRegistryRowState::Live && !row.selected)
        .expect("real projection has a non-selected READY row");
    target.runtime_role = ModelRuntimeRole::Embedding;
    target.default_selectable = false;
    let artifact = target.artifact_sha256.clone();
    validate_projection_for_native_surface(&projection)
        .expect("typed embedding-role projection remains valid");
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_real_projection(projection),
        );

    harness.run();
    harness.run();
    assert_author_id(
        &harness,
        &row_role_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &artifact),
    );
    assert_author_id(
        &harness,
        &row_default_ineligible_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &artifact),
    );
    assert!(
        !live_author_ids(&harness).iter().any(|author_id| author_id
            == &row_switch_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &artifact)),
        "embedding-role rows must not expose a switch action"
    );
}

#[test]
fn inspect_engine_internals_action_opens_native_drilldown() {
    let mut projection = real_projection();
    let row = projection
        .rows
        .iter_mut()
        .find(|row| row.runtime_state == ModelRuntimeRegistryRowState::Live)
        .expect("projection contains a LIVE row");
    row.engine_internals = handshake_native::model_runtime_panel::ModelRuntimeValue::Available {
        value: json!({ "engine_probe": "visible" }),
    };
    row.inspect_engine_internals_action.enabled = true;
    row.inspect_engine_internals_action.reason = None;
    let action_id = row_action_author_id(
        PRIMARY_MODEL_RUNTIME_PANE_ID,
        &row.artifact_sha256,
        "inspect-internals",
    );
    validate_projection_for_native_surface(&projection)
        .expect("available engine internals enable the read-only drilldown");

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_real_projection(projection),
        );
    harness.run();
    click_author_id(&mut harness, &action_id);
    harness.run();
    assert!(
        live_labels(&harness)
            .iter()
            .any(|label| label.contains("engine_probe") && label.contains("visible")),
        "Inspect Engine Internals must open the existing native JSON drilldown"
    );
}

#[test]
fn mt014_failed_refresh_marks_real_projection_and_runtime_rows_stale() {
    let projection = real_projection();
    let generated_at_utc = projection.generated_at_utc.clone();
    let live_model_ids = projection
        .rows
        .iter()
        .filter_map(|row| row.live_model_id.clone())
        .collect::<Vec<_>>();
    assert!(
        !live_model_ids.is_empty(),
        "real PostgreSQL projection must contain a current READY row"
    );
    let transport: Arc<dyn ModelRuntimeRegistryTransport> =
        Arc::new(OrderedRegistryTransport::success_then_failure(
            projection,
            "injected registry authority refresh failure",
        ));
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_registry_transport(transport),
        );

    harness.run();
    harness.run();
    let successful_labels = live_labels(&harness);
    assert!(
        successful_labels
            .iter()
            .any(|label| label == "Runtime state: LIVE / READY"),
        "successful refresh must expose the real current READY join"
    );

    harness
        .get_by_label("Refresh ModelRuntime registry")
        .click_accesskit();
    harness.run();
    harness.run();

    let stale_labels = live_labels(&harness);
    assert!(
        stale_labels.iter().any(|label| {
            label.contains("STALE registry snapshot")
                && label.contains(&generated_at_utc)
                && label.contains("current runtime state unknown")
        }),
        "failed refresh must label the retained projection as stale: {stale_labels:?}"
    );
    assert!(
        stale_labels
            .iter()
            .any(|label| label == "Runtime state: STALE / LAST SEEN READY"),
        "a failed refresh must not continue presenting a prior READY join as live"
    );
    assert!(
        !stale_labels
            .iter()
            .any(|label| label == "Runtime state: LIVE / READY"),
        "stale rows must not retain the current-state LIVE / READY label"
    );
    for live_model_id in live_model_ids {
        assert!(
            stale_labels
                .iter()
                .any(|label| label == &format!("Last seen live model id: {live_model_id}")),
            "stale projection must qualify the formerly live runtime UUID"
        );
    }
    assert!(
        stale_labels.iter().any(|label| {
            label.contains("ModelRuntime registry error:")
                && label.contains("injected registry authority refresh failure")
        }),
        "the authority refresh failure remains operator-visible"
    );
}

#[test]
fn mt014_argus_two_visible_model_runtime_panes_have_globally_unique_author_ids() {
    let projection = real_projection();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(
            |context, app: &mut HandshakeApp| app.ui(context),
            app_with_two_model_runtime_panes(projection),
        );
    harness.run();
    harness.run();

    let active_model_runtime_panes = harness
        .state()
        .tab_bar_states()
        .iter()
        .filter_map(|(pane_id, bar)| {
            bar.active()
                .is_some_and(|tab| tab.pane_type == PaneType::ModelRuntime)
                .then_some(pane_id.as_ref())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        active_model_runtime_panes,
        std::collections::BTreeSet::from([
            PRIMARY_MODEL_RUNTIME_PANE_ID,
            SECONDARY_MODEL_RUNTIME_PANE_ID,
        ]),
        "the real split layout must render ModelRuntime in both requested panes"
    );

    let model_runtime_author_ids = live_author_ids(&harness)
        .into_iter()
        .filter(|author_id| author_id.starts_with(&format!("{AUTHOR_ID_PREFIX}.")))
        .collect::<Vec<_>>();
    assert!(
        !model_runtime_author_ids.is_empty(),
        "the live tree must contain ModelRuntime author IDs"
    );
    let unique_author_ids = model_runtime_author_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        model_runtime_author_ids.len(),
        unique_author_ids.len(),
        "all ModelRuntime author IDs in the complete two-pane live tree must be globally unique"
    );
    let scrollbar_author_ids = harness
        .root()
        .children_recursive()
        .filter(|node| node.accesskit_node().role() == egui::accesskit::Role::ScrollBar)
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .filter(|author_id| author_id.starts_with("scrollbar-v-pane-"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        scrollbar_author_ids,
        std::collections::BTreeSet::from([
            SCROLLBAR_V_NODE_IDS[0].0.to_owned(),
            SCROLLBAR_V_NODE_IDS[1].0.to_owned(),
        ]),
        "both visible ModelRuntime panes require distinct stable scrollbar rails"
    );

    for pane_id in [
        PRIMARY_MODEL_RUNTIME_PANE_ID,
        SECONDARY_MODEL_RUNTIME_PANE_ID,
    ] {
        let pane_prefix = format!("{AUTHOR_ID_PREFIX}.{pane_id}.");
        assert!(
            model_runtime_author_ids
                .iter()
                .any(|author_id| author_id.starts_with(&pane_prefix)),
            "{pane_id} must own pane-scoped ModelRuntime targets"
        );
        assert_author_id(&harness, &surface_author_id(pane_id));
        assert_author_id(&harness, &status_author_id(pane_id));

        let refresh = refresh_author_id(pane_id);
        let refresh_node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(refresh.as_str()))
            .unwrap_or_else(|| panic!("{refresh} missing from the complete live tree"));
        assert_eq!(
            format!("{:?}", refresh_node.accesskit_node().role()),
            "Button",
            "{refresh} must remain an actionable pane-specific refresh target"
        );
        assert_positive_author_bounds(&harness, &refresh);
    }

    assert!(
        model_runtime_author_ids.iter().all(|author_id| {
            author_id.starts_with(&format!(
                "{AUTHOR_ID_PREFIX}.{PRIMARY_MODEL_RUNTIME_PANE_ID}."
            )) || author_id.starts_with(&format!(
                "{AUTHOR_ID_PREFIX}.{SECONDARY_MODEL_RUNTIME_PANE_ID}."
            ))
        }),
        "every emitted ModelRuntime author ID must include its stable pane id: \
         {model_runtime_author_ids:?}"
    );
    assert_unique_model_runtime_author_ids(&harness);
}

#[test]
#[ignore = "GPU-gated real-data frame capture: first run the real PostgreSQL backend proof with HANDSHAKE_MT014_PROJECTION_ARTIFACT, then run this exact ignored test with HANDSHAKE_MT014_SCREENSHOT_PATH on a GPU-capable host"]
fn mt014_model_runtime_real_pg_frame_png() {
    use image::ImageEncoder;

    let (projection, projection_provenance) = real_projection_with_provenance();
    let (server, base_url) = projection_http_server(&projection);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build screenshot production-client runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(354),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.set_left_rail_open(false);
    // Give the active top-left pane the largest legal split share so both the
    // live and dormant durable rows are readable in one evidence frame.
    app.split_weights_mut().vertical = 0.8;
    app.split_weights_mut().horizontal = 0.8;
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|context, app: &mut HandshakeApp| app.ui(context), app);
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Model Runtime").click();
    for _ in 0..100 {
        harness.step();
        if live_labels(&harness)
            .iter()
            .any(|label| label.contains("live |") && label.contains("dormant |"))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        harness.state().active_module(),
        ModuleId::Studio,
        "the screenshot operator path must activate the STUDIO module"
    );
    assert_eq!(
        server.join().expect("screenshot projection server joins"),
        "GET /model-runtime/registry HTTP/1.1",
        "the screenshot must be reached through the production registry route"
    );
    for _ in 0..3 {
        harness.run();
    }
    assert_author_id(&harness, &surface_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID));
    for row in &projection.rows {
        assert_author_id(
            &harness,
            &row_author_id(PRIMARY_MODEL_RUNTIME_PANE_ID, &row.artifact_sha256),
        );
        assert!(
            live_labels(&harness)
                .iter()
                .any(|label| label.contains(&row.display_label)),
            "the screenshot frame must still expose {} after the HTTP server closes",
            row.display_label
        );
    }
    let image = harness
        .render()
        .expect("GPU host renders the real ModelRuntime registry frame");
    let (width, height) = (image.width(), image.height());
    let pixels = image.as_raw();
    let opaque_pixels = pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] == u8::MAX)
        .count();
    let distinct_rgb = pixels
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<std::collections::BTreeSet<_>>();
    let pixel_count = usize::try_from(width * height).expect("frame pixel count fits usize");
    assert!(
        opaque_pixels * 100 >= pixel_count * 99,
        "the governed frame must be at least 99% opaque after normal UI anti-aliasing: \
         opaque={opaque_pixels}, total={pixel_count}"
    );
    assert!(
        distinct_rgb.len() > 32,
        "the governed frame must contain rendered UI detail, got only {} colors",
        distinct_rgb.len()
    );
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .expect("encode ModelRuntime registry PNG");
    assert!(png.len() > 1024, "rendered PNG must not be blank");

    let path =
        canonical_artifact_path(SCREENSHOT_FILE).expect("resolve canonical MT-014 screenshot path");
    require_configured_path("HANDSHAKE_MT014_SCREENSHOT_PATH", &path)
        .expect("configured MT-014 screenshot path is canonical");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create screenshot artifact parent");
    }
    let screenshot_sha256 = format!("{:x}", Sha256::digest(&png));
    std::fs::write(&path, &png).expect("write ModelRuntime registry screenshot");
    let producer_completed_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    std::fs::write(
        path.with_extension("provenance.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_id": "hsk.mt014_model_runtime_screenshot_provenance@1",
            "proof_nonce": projection_provenance.proof_nonce,
            "source_projection_schema_id": projection.schema_id,
            "source_projection_sha256": projection_provenance.artifact_sha256,
            "screenshot_sha256": screenshot_sha256,
            "producer_test_id": "mt014_model_runtime_real_pg_frame_png",
            "producer_status": "passed_all_native_and_pixel_assertions",
            "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
        }))
        .expect("serialize MT-014 screenshot provenance"),
    )
    .expect("write MT-014 screenshot provenance");
    println!(
        "REAL_MODEL_RUNTIME_SCREENSHOT={} ({width}x{height})",
        path.display()
    );
}
