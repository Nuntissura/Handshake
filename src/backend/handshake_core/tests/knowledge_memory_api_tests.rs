//! WP-KERNEL-009 MT-126 MemoryGraphBackendApi route-level integration proof
//! against REAL Handshake-managed PostgreSQL.
//!
//! Drives the actual Axum routes (`api::knowledge_memory::routes`) over a
//! loopback listener (quiet: no foreground window, no focus steal). Builds a
//! memory fact backed by a claim, records a conflict, then navigates the memory
//! graph through the HTTP surface: claim+evidence, conflict review / repair
//! queue, fact trace, entity neighborhood, and the visual-debug payload. Every
//! nav query MUST require the backend-navigation identity headers (400 if
//! absent) and leave a `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` receipt (the
//! response returns its event id).

mod knowledge_memory_fixtures;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_memory as mem_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::KnowledgeStore;
use handshake_core::storage::knowledge_memory::{
    create_memory_fact, MemoryClaimAuthorityLabel, MemoryFactObject, NewMemoryFact,
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

async fn app_state_for(schema_url: &str) -> AppState {
    let storage = PostgresDatabase::connect(schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = pool_for_url(schema_url).await;
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("memory-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

async fn pool_for_url(schema_url: &str) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool")
}

async fn start_server(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = mem_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("memory api server");
    });
    (format!("http://{addr}"), server)
}

fn nav_headers(client: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    client
        .header("x-hsk-actor-kind", "model_adapter")
        .header("x-hsk-actor-id", format!("mem-api-test-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-MEM-{label}"))
        .header("x-hsk-session-run-id", format!("SR-MEM-{label}"))
        .header("x-hsk-correlation-id", format!("CORR-MEM-{label}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt126_memory_api_claim_conflict_fact_neighborhood_visualdebug_with_receipts() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP mt126_memory_api: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    // Build two facts (subject+predicate, different object) and a conflict.
    let subject = fx
        .entity("api", "managed_postgres", "ManagedPostgres")
        .await;
    let object = fx.entity("symbol", "crate::pg::Port", "Port").await;
    let claim_a = fx.claim("managed PG port is 5544").await;
    let claim_b = fx.claim("managed PG port is 5432").await;
    let fact_a = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_a.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Entity {
                entity_id: object.clone(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("fact a");
    let conflict = db
        .record_knowledge_claim_conflict(
            &claim_a.claim_id,
            &claim_b.claim_id,
            "port disagreement",
            None,
        )
        .await
        .expect("conflict");

    let state = app_state_for(&fx.pg.schema_url).await;
    let (base, server) = start_server(state).await;
    let http = reqwest::Client::new();

    // --- Missing identity headers -> 400 (receipt law) -----------------------
    let no_hdr = http
        .get(format!(
            "{base}/knowledge/memory/claims/{}",
            claim_a.claim_id
        ))
        .send()
        .await
        .expect("send no-header");
    assert_eq!(no_hdr.status(), 400, "nav without identity must be 400");

    // --- Claim + evidence ----------------------------------------------------
    let claim_resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/memory/claims/{}",
            claim_a.claim_id
        )),
        "claim",
    )
    .send()
    .await
    .expect("claim send");
    assert_eq!(claim_resp.status(), 200);
    let claim_body: Value = claim_resp.json().await.expect("claim json");
    assert!(claim_body["retrieval_receipt_event_id"].is_string());
    assert_eq!(claim_body["claim"]["claim_id"], claim_a.claim_id);
    assert_eq!(
        claim_body["evidence_span_ids"]
            .as_array()
            .expect("spans")
            .len(),
        1
    );
    assert_eq!(claim_body["backing_fact"]["fact_id"], fact_a.fact_id);
    // The claim is now conflicted, and its conflict is listed.
    assert_eq!(claim_body["claim"]["lifecycle_state"], "conflicted");
    assert_eq!(
        claim_body["conflicts"].as_array().expect("conflicts").len(),
        1
    );

    // --- Conflict review / repair queue --------------------------------------
    let conflicts_resp = nav_headers(
        http.get(format!("{base}/knowledge/memory/conflicts"))
            .query(&[("workspace_id", fx.workspace_id.as_str())]),
        "conflicts",
    )
    .send()
    .await
    .expect("conflicts send");
    assert_eq!(conflicts_resp.status(), 200);
    let conflicts_body: Value = conflicts_resp.json().await.expect("conflicts json");
    assert_eq!(conflicts_body["count"], 1);
    assert_eq!(
        conflicts_body["conflicts"][0]["conflict_id"],
        conflict.conflict_id
    );
    assert!(conflicts_body["retrieval_receipt_event_id"].is_string());

    // --- Fact trace ----------------------------------------------------------
    let fact_resp = nav_headers(
        http.get(format!("{base}/knowledge/memory/facts/{}", fact_a.fact_id)),
        "fact",
    )
    .send()
    .await
    .expect("fact send");
    assert_eq!(fact_resp.status(), 200);
    let fact_body: Value = fact_resp.json().await.expect("fact json");
    assert_eq!(fact_body["fact"]["fact_id"], fact_a.fact_id);
    assert_eq!(fact_body["backing_claim_id"], claim_a.claim_id);

    // --- Entity neighborhood (the fact-a relationship has no graph edge, but
    //     the API returns the edge list for the entity; here it is empty since
    //     facts are not knowledge_edges. Assert the shape + receipt). ----------
    let nbhd_resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/memory/entities/{subject}/neighborhood"
        )),
        "nbhd",
    )
    .send()
    .await
    .expect("nbhd send");
    assert_eq!(nbhd_resp.status(), 200);
    let nbhd_body: Value = nbhd_resp.json().await.expect("nbhd json");
    assert_eq!(nbhd_body["entity_id"], subject);
    assert!(nbhd_body["edges"].is_array());
    assert!(nbhd_body["retrieval_receipt_event_id"].is_string());

    // --- Visual-debug payload ------------------------------------------------
    let vd_resp = nav_headers(
        http.get(format!("{base}/knowledge/memory/visual-debug"))
            .query(&[("workspace_id", fx.workspace_id.as_str())]),
        "vd",
    )
    .send()
    .await
    .expect("vd send");
    assert_eq!(vd_resp.status(), 200);
    let vd_body: Value = vd_resp.json().await.expect("vd json");
    let payload = &vd_body["payload"];
    assert_eq!(payload["authority_class"], "projection");
    assert_eq!(payload["schema_id"], "hsk.memory_graph_visual_debug@1");
    // Two claims, both conflicted; one open conflict.
    assert_eq!(payload["claim_state_counts"]["conflicted"], 2);
    assert_eq!(payload["open_conflict_count"], 1);
    // One source-labelled fact.
    assert_eq!(payload["fact_label_counts"]["source"], 1);

    server.abort();
}
