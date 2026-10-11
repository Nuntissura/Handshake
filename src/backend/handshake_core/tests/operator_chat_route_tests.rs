//! WP-1 MT-012 operator chat/launch HTTP route proof.
//!
//! Drives the real axum handlers over a loopback listener (no full AppState).
//! Proves: the picker-enumeration route serves local + cloud rows (MT-015 cloud
//! degrade), the launch route resolves through the sanctioned launch service and
//! FAILS CLOSED at the route when the coordinator has no ModelLaneStore, and the
//! selection route records ok.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use handshake_core::api::operator_chat::{
    resolve_operator_chat_lineage, routes, OperatorChatState,
};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::cloud::{
    ByokProvider, CliBridgeAuthStatus, CliBridgeAuthStatusProbe, CliBridgeProvider,
    InMemoryAccessRegistry,
};
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::storage::{ModelSession, ModelSessionState};
use handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchService;
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};

#[derive(Default)]
struct RecordingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl RecordingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("recorder lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for RecordingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

struct LoggedInCliProbe;

impl CliBridgeAuthStatusProbe for LoggedInCliProbe {
    fn auth_status(&self, _provider: CliBridgeProvider) -> CliBridgeAuthStatus {
        CliBridgeAuthStatus::LoggedIn
    }
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed("must not be reached".into()))
    }
}

async fn start_server(state: OperatorChatState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = routes(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("operator-chat server");
    });
    (format!("http://{addr}"), handle)
}

fn launch_body(owner_session_id: &str) -> serde_json::Value {
    serde_json::json!({
        "lane_kind": "cli",
        "model_id": "claude-sonnet-4",
        "cli_provider": "claude_code",
        "working_dir": "D:/work/repo",
        "prompt": "audit the repo",
        "owner_session_id": owner_session_id
    })
}

fn registry_session(
    session_id: &str,
    parent_session_id: Option<&str>,
    spawn_depth: i32,
    state: ModelSessionState,
) -> ModelSession {
    ModelSession {
        session_id: session_id.to_string(),
        parent_session_id: parent_session_id.map(str::to_string),
        spawn_depth,
        state,
        model_id: "gpt-test".to_string(),
        backend: "codex".to_string(),
        parameter_class: "standard".to_string(),
        role: "CODER".to_string(),
        wp_id: Some("WP-1".to_string()),
        mt_id: Some("MT-017".to_string()),
        work_profile_id: None,
        execution_mode: "delegated".to_string(),
        memory_policy: "SESSION_SCOPED".to_string(),
        consent_receipt_id: None,
        capability_grants: Vec::new(),
        capability_token_ids: None,
        job_id: None,
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        merge_back_artifact: None,
        agent: None,
        purpose: None,
        close_reason: None,
        closed_by_actor: None,
        closed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

async fn governed_registry() -> Arc<SessionRegistry> {
    let registry = Arc::new(SessionRegistry::new(SessionSchedulerConfig::default()));
    for session in [
        registry_session("root-a", None, 0, ModelSessionState::Active),
        registry_session("child-a", Some("root-a"), 1, ModelSessionState::Active),
        registry_session("root-b", None, 0, ModelSessionState::Active),
        registry_session("child-b", Some("root-b"), 1, ModelSessionState::Active),
        registry_session("paused-child", Some("root-a"), 1, ModelSessionState::Paused),
        registry_session("invalid-depth", Some("root-a"), 3, ModelSessionState::Active),
    ] {
        registry.upsert_session(session).await;
    }
    registry
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_models_route_lists_local_and_cloud_with_degrade() {
    let registry = InMemoryAccessRegistry::new();
    registry.set_configured(ByokProvider::Anthropic, true);
    let session_registry = governed_registry().await;
    let state = OperatorChatState::production()
        .with_catalog(ModelCatalog::empty())
        .with_cloud_registry(Arc::new(registry))
        .with_session_registry(session_registry);
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/operator-chat/models"))
        .send()
        .await
        .expect("models request");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("models json");

    assert_eq!(body["inventory_source"], "operator_chat_backend");
    let sessions = body["sessions"].as_array().expect("sessions array");
    assert_eq!(
        sessions
            .iter()
            .find(|row| row["session_id"] == "child-a")
            .expect("available governed child")["status"],
        "available"
    );
    assert_eq!(
        sessions
            .iter()
            .find(|row| row["session_id"] == "paused-child")
            .expect("paused child row")["status"],
        "unavailable"
    );
    assert!(body.get("local").and_then(|v| v.as_array()).is_some());
    let byok = body["cloud_byok"].as_array().expect("cloud_byok array");
    let anthropic = byok
        .iter()
        .find(|r| r["provider"] == "anthropic")
        .expect("anthropic row");
    assert_eq!(anthropic["status"], "configured");
    assert_eq!(anthropic["model_id"], "claude-sonnet-4");
    let openai = byok
        .iter()
        .find(|r| r["provider"] == "openai")
        .expect("openai row");
    assert_eq!(openai["model_id"], "gpt-4o");
    assert_eq!(
        openai["status"], "unavailable",
        "an unconfigured cloud provider degrades to unavailable, never mocked"
    );
    let cli = body["cloud_cli_bridge"]
        .as_array()
        .expect("cloud_cli_bridge array");
    let codex = cli
        .iter()
        .find(|r| r["provider"] == "codex")
        .expect("codex CLI row");
    assert_eq!(codex["model_id"], "gpt-5-codex");
    assert_eq!(
        codex["status"], "unavailable",
        "CLI picker rows must reflect launch-service PATH wiring; absent wiring degrades to unavailable"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logged_in_cli_requires_matching_registered_launch_builder() {
    let state = OperatorChatState::production()
        .with_cli_bridge_auth_probe(Arc::new(LoggedInCliProbe))
        .with_cli_bridge_launchable_providers(["codex".to_string()]);
    let (base, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base}/operator-chat/models"))
        .send()
        .await
        .expect("models request");
    assert_eq!(response.status().as_u16(), 200);
    let body: serde_json::Value = response.json().await.expect("models json");
    let rows = body["cloud_cli_bridge"]
        .as_array()
        .expect("CLI rows");
    let claude = rows
        .iter()
        .find(|row| row["provider"] == "claude_code")
        .expect("Claude row");
    let codex = rows
        .iter()
        .find(|row| row["provider"] == "codex")
        .expect("Codex row");
    assert_eq!(
        claude["status"], "unavailable",
        "auth alone must not advertise a provider whose launch builder is absent"
    );
    assert_eq!(
        codex["status"], "logged_in",
        "typed auth may become selectable only when the same provider is launchable"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_route_returns_503_when_not_wired() {
    let state = OperatorChatState::production();
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/launch"))
        .json(&launch_body("child-a"))
        .send()
        .await
        .expect("launch request");
    assert_eq!(
        resp.status().as_u16(),
        503,
        "an unwired launch route reports launch_not_wired"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_single_run_cloud_routes_are_shipped_rust_handlers() {
    let state = OperatorChatState::production();
    let (base, server) = start_server(state).await;
    let client = reqwest::Client::new();

    let malformed_grant = client
        .post(format!(
            "{base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("SingleRun grant-launch route request");
    assert_eq!(
        malformed_grant.status().as_u16(),
        422,
        "registered Rust handler rejects malformed JSON instead of returning 404"
    );

    let revoke = client
        .post(format!("{base}/operator-chat/cloud/single-run/revoke"))
        .json(&serde_json::json!({
            "consent_receipt_id": "cloud-consent-receipt://route-proof/broadcast",
            "revoked_by_ref": "operator://route-proof",
            "reason": "route proof"
        }))
        .send()
        .await
        .expect("SingleRun revoke route request");
    assert_eq!(
        revoke.status().as_u16(),
        503,
        "registered Rust revoke handler fails closed when coordinator is unwired"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_route_fails_closed_without_model_lane_store() {
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let calls = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let service = Arc::new(OperatorChatLaunchService::new(
        Arc::new(coordinator),
        ModelCatalog::empty(),
        Arc::new(RecordingRecorder::default()),
    ));
    let state = OperatorChatState::production()
        .with_launch_service(service)
        .with_session_registry(governed_registry().await);
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/launch"))
        .json(&launch_body("child-a"))
        .send()
        .await
        .expect("launch request");
    assert_eq!(
        resp.status().as_u16(),
        500,
        "a launch with no ModelLaneStore fails closed at the route"
    );
    let body: serde_json::Value = resp.json().await.expect("error json");
    assert_eq!(body["error"], "launch_failed_closed");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "factory must not be reached when the launch fails closed"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_lineage_resolves_across_roots_and_rejects_invalid_owners() {
    let registry = governed_registry().await;

    let child_a = resolve_operator_chat_lineage(registry.as_ref(), "child-a")
        .await
        .expect("child-a lineage");
    assert_eq!(child_a.owner_session_id, "child-a");
    assert_eq!(child_a.parent_session_id, "root-a");

    let child_b = resolve_operator_chat_lineage(registry.as_ref(), "child-b")
        .await
        .expect("child-b lineage from independent root");
    assert_eq!(child_b.owner_session_id, "child-b");
    assert_eq!(child_b.parent_session_id, "root-b");

    for (owner, expected) in [
        ("", "owner_session_id_required"),
        ("missing", "owner_session_not_registered"),
        ("root-a", "owner_session_lineage_missing"),
        ("paused-child", "owner_session_not_active"),
        ("invalid-depth", "owner_session_lineage_invalid"),
    ] {
        assert_eq!(
            resolve_operator_chat_lineage(registry.as_ref(), owner)
                .await
                .expect_err("invalid owner must fail closed"),
            expected
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_launch_api_requires_registered_active_governed_owner() {
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: Arc::new(AtomicUsize::new(0)),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let service = Arc::new(OperatorChatLaunchService::new(
        Arc::new(coordinator),
        ModelCatalog::empty(),
        Arc::new(RecordingRecorder::default()),
    ));
    let state = OperatorChatState::production()
        .with_launch_service(service)
        .with_session_registry(governed_registry().await);
    let (base, server) = start_server(state).await;
    let client = reqwest::Client::new();

    let mut missing_owner = launch_body("child-a");
    missing_owner
        .as_object_mut()
        .expect("launch object")
        .remove("owner_session_id");
    let response = client
        .post(format!("{base}/operator-chat/launch"))
        .json(&missing_owner)
        .send()
        .await
        .expect("missing-owner request");
    assert_eq!(response.status().as_u16(), 422);

    for (owner, expected_code) in [
        ("missing", "owner_session_not_registered"),
        ("root-a", "owner_session_lineage_missing"),
        ("paused-child", "owner_session_not_active"),
        ("invalid-depth", "owner_session_lineage_invalid"),
    ] {
        let response = client
            .post(format!("{base}/operator-chat/launch"))
            .json(&launch_body(owner))
            .send()
            .await
            .expect("invalid-owner request");
        assert_eq!(response.status().as_u16(), 400);
        let body: serde_json::Value = response.json().await.expect("invalid-owner json");
        assert_eq!(body["error"], "invalid_owner_session");
        assert_eq!(body["code"], expected_code);
    }

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_selection_route_records_ok() {
    let recorder = Arc::new(RecordingRecorder::default());
    let state = OperatorChatState::production()
        .with_catalog(ModelCatalog::empty())
        .with_recorder(recorder.clone());
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/selection"))
        .json(&serde_json::json!({
            "selected_model_id": "claude-sonnet-4",
            "lane_kind": "cli",
            "model_id": "claude-sonnet-4",
            "cli_provider": "claude_code",
            "working_dir": "D:/work/repo",
            "actor": "operator",
            "reason": "operator picked the CLI lane"
        }))
        .send()
        .await
        .expect("selection request");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("selection json");
    assert_eq!(body["status"], "recorded");
    let events = recorder.events();
    assert_eq!(events.len(), 1, "selection route emits one audit event");
    let payload = &events[0].payload;
    assert_eq!(payload["fr_event"], "FR-EVT-MODEL-SELECTION-RECORDED");
    assert_eq!(payload["selection_context"]["lane_kind"], "cli");
    assert_eq!(payload["selection_context"]["model_id"], "claude-sonnet-4");
    assert_eq!(payload["selection_context"]["cli_provider"], "claude_code");
    assert_eq!(payload["selection_context"]["working_dir"], "D:/work/repo");
    server.abort();
}
