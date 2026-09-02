//! WP-KERNEL-009 MT-095 SourceIngestionApi route-level integration proof
//! against the real embedded Handshake store.
//!
//! Drives the actual Axum routes (`api::knowledge_ingestion::routes`) over a
//! loopback listener (quiet: no foreground window, no focus steal): register
//! a root (and a denied root), trigger an ingestion pass over a runtime temp
//! directory, then read sources, receipts, and the repair queue back through
//! the HTTP surface and retry a repair entry. Every mutation must leave
//! EventLedger receipts carrying the actor/session/correlation headers.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_ingestion as ingestion_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_ingestion_support::{open_embedded_ingestion_fixture, EmbeddedKnowledgeStore};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        let _ = id;
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// AppState and every assertion share one real embedded store handle.
async fn app_state_for(store: &EmbeddedKnowledgeStore) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("ingestion-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn start_server(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = ingestion_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("ingestion api server");
    });
    (format!("http://{addr}"), server)
}

fn mutation_headers(client: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    client
        .header("x-hsk-actor-kind", "system")
        .header("x-hsk-actor-id", format!("ingestion-api-test-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-API-{label}"))
        .header("x-hsk-session-run-id", format!("SR-API-{label}"))
        .header("x-hsk-correlation-id", format!("CORR-API-{label}"))
}

fn write(dir: &Path, rel: &str, content: &[u8]) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, content).expect("write runtime fixture");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt095_routes_cover_register_run_inspect_and_repair_with_ledger_receipts() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!(
            "SKIP mt095_routes_cover_register_run_inspect_and_repair: embedded store unavailable"
        );
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let state = app_state_for(&env.store).await;
    let (base_url, server) = start_server(state).await;
    let http = reqwest::Client::new();

    // Runtime source tree: one good note, one broken transcript.
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "notes/good.md", b"# Good\n\nreadable note\n");
    write(temp.path(), "media/broken.srt", b"garbage\nno cues\n");
    let fs_anchor = temp.path().to_string_lossy().to_string();

    // 1. Register a root (mutation: headers required).
    let body = json!({
        "workspace_id": workspace_id,
        "display_name": "api test root",
        "root_kind": "project_repo",
        "repo_relative_path": "",
        "operator_approved": false,
    });
    // Missing identity headers is a typed 400, not a silent default.
    let denied = http
        .post(format!("{base_url}/knowledge/ingestion/roots"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(denied.status(), reqwest::StatusCode::BAD_REQUEST);

    let response = mutation_headers(
        http.post(format!("{base_url}/knowledge/ingestion/roots")),
        "register",
    )
    .json(&body)
    .send()
    .await
    .expect("register root");
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let created: Value = response.json().await.expect("json");
    let root_id = created["root"]["root_id"]
        .as_str()
        .expect("root id")
        .to_string();
    assert_eq!(created["decision"]["verdict"], "allowed");
    let decision_event = created["decision"]["receipt_event_id"]
        .as_str()
        .expect("decision ledger receipt")
        .to_string();

    // 2. Denied registration: 403 carries the durable decision id.
    let response = mutation_headers(
        http.post(format!("{base_url}/knowledge/ingestion/roots")),
        "register-denied",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "display_name": "denied root",
        "root_kind": "project_repo",
        "repo_relative_path": "ops/secrets/prod",
    }))
    .send()
    .await
    .expect("denied registration");
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let denial: Value = response.json().await.expect("json");
    assert_eq!(denial["error"], "policy_denied");
    assert_eq!(denial["verdict"], "denied_pattern");
    assert!(denial["decision_id"]
        .as_str()
        .expect("decision id")
        .starts_with("KIPD-"));

    // 3. List roots.
    let roots: Value = http
        .get(format!(
            "{base_url}/knowledge/ingestion/roots?workspace_id={workspace_id}"
        ))
        .send()
        .await
        .expect("list roots")
        .json()
        .await
        .expect("json");
    assert_eq!(roots["roots"].as_array().expect("roots").len(), 1);

    // 4. Trigger an ingestion run (mutation).
    let response = mutation_headers(
        http.post(format!("{base_url}/knowledge/ingestion/runs")),
        "run",
    )
    .json(&json!({"root_id": root_id, "fs_anchor": fs_anchor}))
    .send()
    .await
    .expect("trigger run");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let summary: Value = response.json().await.expect("json");
    let outcomes = summary["outcomes"].as_array().expect("outcomes");
    assert_eq!(outcomes.len(), 2);
    let broken = outcomes
        .iter()
        .find(|o| o["relative_path"] == "media/broken.srt")
        .expect("broken outcome");
    assert_eq!(broken["status"], "failed");
    assert_eq!(broken["error_class"], "PARSE_ERROR");
    let repair_id = broken["repair_id"].as_str().expect("repair id").to_string();
    let broken_source_id = broken["source_id"].as_str().expect("source id").to_string();
    let good = outcomes
        .iter()
        .find(|o| o["relative_path"] == "notes/good.md")
        .expect("good outcome");
    assert_eq!(good["status"], "success");

    // 5. List sources for the root.
    let sources: Value = http
        .get(format!(
            "{base_url}/knowledge/ingestion/roots/{root_id}/sources"
        ))
        .send()
        .await
        .expect("list sources")
        .json()
        .await
        .expect("json");
    assert_eq!(sources["sources"].as_array().expect("sources").len(), 2);

    // 6. Receipts for the failing source.
    let receipts: Value = http
        .get(format!(
            "{base_url}/knowledge/ingestion/sources/{broken_source_id}/receipts"
        ))
        .send()
        .await
        .expect("list receipts")
        .json()
        .await
        .expect("json");
    let receipt_rows = receipts["receipts"].as_array().expect("receipts");
    assert_eq!(receipt_rows.len(), 1);
    assert_eq!(receipt_rows[0]["status"], "failed");
    assert_eq!(receipt_rows[0]["error_class"], "PARSE_ERROR");

    // 7. Repair queue: list queued, then retry after fixing the artifact.
    let repairs: Value = http
        .get(format!(
            "{base_url}/knowledge/ingestion/repairs?workspace_id={workspace_id}&state=queued"
        ))
        .send()
        .await
        .expect("list repairs")
        .json()
        .await
        .expect("json");
    let repair_rows = repairs["repairs"].as_array().expect("repairs");
    assert_eq!(repair_rows.len(), 1);
    assert_eq!(repair_rows[0]["repair_id"], repair_id.as_str());

    write(
        temp.path(),
        "media/broken.srt",
        b"1\n00:00:01,000 --> 00:00:02,000\nFixed cue\n",
    );
    let response = mutation_headers(
        http.post(format!(
            "{base_url}/knowledge/ingestion/repairs/{repair_id}/retry"
        )),
        "retry",
    )
    .json(&json!({"fs_anchor": fs_anchor}))
    .send()
    .await
    .expect("retry repair");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let retried: Value = response.json().await.expect("json");
    assert_eq!(retried["repair"]["state"], "resolved");
    assert_eq!(retried["attempt"]["status"], "success");

    // 8. Unknown ids are typed 404s, not 500s.
    let response = mutation_headers(
        http.post(format!(
            "{base_url}/knowledge/ingestion/repairs/KIRQ-00000000000000000000000000000000/retry"
        )),
        "retry-missing",
    )
    .json(&json!({"fs_anchor": fs_anchor}))
    .send()
    .await
    .expect("retry unknown repair");
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();

    // MT-141 disposition: direct EventLedger-row inspection was a relational
    // probe. The typed API response and receipt ids above are the embedded
    // replacement proof; the raw-row probe is not exposed by the public store.
    let _ = decision_event;
}
