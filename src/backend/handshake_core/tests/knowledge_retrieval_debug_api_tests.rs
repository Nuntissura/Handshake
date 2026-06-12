//! WP-KERNEL-009 MT-143 RetrievalDebugApi — adversarial-v2 hardening proof for
//! the stale-reason / missing-evidence surface and the repair action, driven
//! over the REAL Axum routes against real PostgreSQL.

#[path = "knowledge_memory_fixtures.rs"]
mod knowledge_memory_fixtures;

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_retrieval as retrieval_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::knowledge_retrieval::compiler::BundleTargetKind;
use handshake_core::knowledge_retrieval::executor::execute_retrieval;
use handshake_core::knowledge_retrieval::graph_planner::GraphTraversalPolicy;
use handshake_core::knowledge_retrieval::planner::RetrievalRequest;
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::{
    KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef, KnowledgeRetrievalMode,
    KnowledgeStore, NewKnowledgeMemoryPassage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_memory_fixtures::{pool_for, MemoryFixture};
use serde_json::{json, Value};

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
        _id: uuid::Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
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

async fn retrieval_server(schema_url: &str) -> (String, reqwest::Client) {
    let storage = PostgresDatabase::connect(schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool");
    let recorder = Arc::new(NoopRecorder);
    let state = AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("retrieval-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = retrieval_api::routes(state);
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("retrieval api server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

fn nav_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-kind", "model_adapter")
        .header("x-hsk-actor-id", format!("retr-api-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-RETR-{label}"))
        .header("x-hsk-session-run-id", format!("SR-RETR-{label}"))
}

/// Adversarial-v2 MT-143: the staleness surface reports per-item evidence
/// state explicitly, and the repair action re-executes the recorded query
/// into a FRESH bundle bound to current index state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt143_staleness_surface_and_repair_action() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP mt143_staleness_surface_and_repair_action: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;

    // A span-backed passage; the executed pipeline (no edges) falls back to it
    // and compiles a REAL bundle with a passage item.
    fx.pg
        .db
        .create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
            workspace_id: fx.workspace_id.clone(),
            passage_text: "evidence passage for staleness checks".to_string(),
            token_count: Some(10),
            ocr_transcript_metadata: None,
            extraction_confidence: 0.9,
            ranking_features: json!({}),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            compaction_policy: KnowledgeCompactionPolicy::Keep,
            failure_receipt_event_id: None,
            derived_in_run: None,
            evidence: vec![KnowledgePassageEvidenceRef::Span {
                span_id: fx.span_id.clone(),
            }],
        })
        .await
        .expect("passage");
    let mut request = RetrievalRequest::discovery(&fx.workspace_id, "staleness scenario");
    request.graph_neighborhood_expected = true;
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-stale-api",
        "sr-stale-api",
        BundleTargetKind::Task,
        "staleness-target",
        &request,
        &BTreeSet::new(),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute");
    let bundle_id = executed.compiled.bundle_id.clone();

    let (base, http) = retrieval_server(&fx.pg.schema_url).await;

    // FRESH bundle: every item ok, stale=false, receipt present.
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/staleness"
        )),
        "fresh",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["stale"], false, "{body}");
    assert!(body["retrieval_receipt_event_id"].is_string());
    let items = body["items"].as_array().expect("items");
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["status"] == "ok"));

    // Make the evidence go MISSING for real: drop the passage row out from
    // under the bundle item (raw SQL — passages have no protecting FK from
    // bundle items by design: bundles are projections of a past retrieval).
    {
        let mut conn = fx.pg.raw_connection().await;
        sqlx::query("DELETE FROM knowledge_passage_evidence WHERE passage_id = $1")
            .bind(&executed.ranked[0].candidate_id)
            .execute(&mut conn)
            .await
            .expect("drop passage evidence");
        sqlx::query("DELETE FROM knowledge_memory_passages WHERE passage_id = $1")
            .bind(&executed.ranked[0].candidate_id)
            .execute(&mut conn)
            .await
            .expect("drop passage");
    }
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/staleness"
        )),
        "missing",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["stale"], true, "missing evidence flips stale: {body}");
    let items = body["items"].as_array().expect("items");
    assert!(
        items.iter().any(|i| i["status"] == "missing_evidence"),
        "the missing passage is reported explicitly: {items:?}"
    );

    // REPAIR: re-execute the recorded query -> a FRESH bundle, linked.
    let resp = nav_headers(
        http.post(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/repair"
        )),
        "repair",
    )
    .send()
    .await
    .expect("send");
    let status = resp.status();
    let body: Value = resp.json().await.expect("json");
    assert_eq!(status, 200, "repair response: {body}");
    assert_eq!(body["bundle_id"], bundle_id);
    assert_eq!(body["action"], "reexecute");
    let repaired_id = body["repaired_bundle_id"].as_str().expect("repaired id");
    assert_ne!(repaired_id, bundle_id, "repair produces a FRESH bundle");
    assert!(body["retrieval_receipt_event_id"].is_string());

    // The repaired bundle is persisted and bound to a trace of its own; the
    // stale bundle is retained (append-only evidence).
    let (repaired, _items) = fx
        .pg
        .db
        .get_knowledge_context_bundle(repaired_id)
        .await
        .expect("get repaired")
        .expect("repaired bundle persisted");
    assert_eq!(repaired.bundle_id, repaired_id);
    assert!(fx
        .pg
        .db
        .get_knowledge_context_bundle(&bundle_id)
        .await
        .expect("get original")
        .is_some());

    // Identity law: no headers -> 400; ghost bundle -> 404.
    let resp = http
        .get(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/staleness"
        ))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 400);
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/CTX-ffffffffffffffff/staleness"
        )),
        "ghost",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 404);
}
