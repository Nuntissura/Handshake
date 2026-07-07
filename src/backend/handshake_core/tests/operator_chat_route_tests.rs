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

use handshake_core::api::operator_chat::{routes, OperatorChatState};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::model_runtime::cloud::{ByokProvider, InMemoryAccessRegistry};
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchService;
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError,
};

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

fn launch_body() -> serde_json::Value {
    serde_json::json!({
        "lane_kind": "cli",
        "model_id": "claude-sonnet-4",
        "cli_provider": "claude_code",
        "working_dir": "D:/work/repo",
        "prompt": "audit the repo",
        "owner_session": "operator-1",
        "parent_session_id": "parent-1"
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_chat_models_route_lists_local_and_cloud_with_degrade() {
    let registry = InMemoryAccessRegistry::new();
    registry.set_configured(ByokProvider::Anthropic, true);
    let state = OperatorChatState::production()
        .with_catalog(ModelCatalog::empty())
        .with_cloud_registry(Arc::new(registry));
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/operator-chat/models"))
        .send()
        .await
        .expect("models request");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("models json");

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
async fn operator_chat_launch_route_returns_503_when_not_wired() {
    let state = OperatorChatState::production();
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/launch"))
        .json(&launch_body())
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
    let state = OperatorChatState::production().with_launch_service(service);
    let (base, server) = start_server(state).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/operator-chat/launch"))
        .json(&launch_body())
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
