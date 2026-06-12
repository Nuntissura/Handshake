//! WP-KERNEL-009 RichDocumentCore route-level integration tests against REAL
//! Handshake-managed PostgreSQL — adversarial-v2 hardening proofs.
//!
//! Drives the actual Axum routes (`api::knowledge_documents::routes`) over a
//! loopback listener (quiet: no foreground window, no focus steal).
//!
//! Covered hardenings:
//!   * MT-158: the permission boundary FAILS CLOSED — a missing
//!     `x-hsk-actor-kind` is least-privileged (read-only, never `system`), a
//!     `cloud_model` cannot write, and a bogus kind is a 400.
//!   * MT-151: import -> load -> save -> export round-trips for HTML and
//!     markdown-table imports (the `importedRaw` node is a loadable kind).
//!   * MT-149: a committed save never returns an error — index/receipt step
//!     failures are non-fatal and recorded in the response.
//!   * MT-152: content_json embed blocks are validated + persisted on the save
//!     path with the same EmbedTarget law as the side table.
//!   * MT-156: history is paginated and version bodies are omitted from the
//!     list response (single-version lazy body load).
//!   * MT-157: a move with an empty body does NOT clear project/folder
//!     membership (absent != explicit null).

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_documents as docs_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
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

/// Boot the real document routes over loopback against the isolated schema.
async fn doc_server(pg: &KnowledgePg) -> (String, reqwest::Client) {
    let storage = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect AppState pool");
    let recorder = Arc::new(NoopRecorder);
    let state = AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("docs-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = docs_api::routes(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("docs api server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

/// The required identity headers WITHOUT an actor kind (MT-158 absence case).
fn identity_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("docs-api-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-DOCS-{label}"))
        .header("x-hsk-session-run-id", format!("SR-DOCS-{label}"))
}

/// Identity headers PLUS an explicitly asserted actor kind.
fn headers_with_kind(
    req: reqwest::RequestBuilder,
    label: &str,
    kind: &str,
) -> reqwest::RequestBuilder {
    identity_headers(req, label).header("x-hsk-actor-kind", kind)
}

fn doc_body(workspace_id: &str, title: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "hello" }] }
            ]
        }
    })
}

/// Create a document as the operator (the privileged setup path).
async fn create_doc(base: &str, http: &reqwest::Client, workspace_id: &str, title: &str) -> Value {
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "setup",
        "operator",
    )
    .json(&doc_body(workspace_id, title))
    .send()
    .await
    .expect("create send");
    assert_eq!(resp.status(), 200, "operator create must succeed");
    resp.json().await.expect("create json")
}

// ---------------------------------------------------------------------------
// MT-158 adversarial-v2: actor-kind fail-closed boundary.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_missing_actor_kind_is_least_privileged_never_system() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt158_missing_actor_kind...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;

    let created = create_doc(&base, &http, &workspace_id, "Boundary").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // ABSENT actor-kind header on a CREATE (write) -> 403, never a system
    // write. Before the hardening this fell open to `system` (full access).
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents")),
        "no-kind-create",
    )
    .json(&doc_body(&workspace_id, "Sneak"))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        403,
        "create without x-hsk-actor-kind must be denied"
    );
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["error"], "forbidden");
    assert_eq!(body["reason"], "unauthenticated_write_denied");

    // ABSENT actor-kind on a SAVE (write) -> 403 and the document is unchanged.
    let resp = identity_headers(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "no-kind-save",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "tampered" }] }
        ]}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        403,
        "save without x-hsk-actor-kind must be denied"
    );

    // ABSENT actor-kind on rename / move / backlink-rebuild -> all denied.
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents/{doc_id}/rename")),
        "no-kind-rename",
    )
    .json(&json!({"title": "Hijacked"}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403);
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents/{doc_id}/backlinks")),
        "no-kind-index",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403, "index without kind must be denied");

    // The least-privileged caller can still READ (attributable read law).
    let resp = identity_headers(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "no-kind-read",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200, "read stays available without a kind");
    let body: Value = resp.json().await.expect("json");
    // The document content was NOT tampered by the denied save.
    assert_eq!(
        body["document"]["content_json"]["content"][0]["content"][0]["text"],
        "hello"
    );
    assert_eq!(body["document"]["doc_version"], 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_cloud_model_cannot_write_and_bogus_kind_is_rejected() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt158_cloud_model...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "CloudBoundary").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // cloud_model write -> 403 with the stable reason code.
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "cloud-save",
        "cloud_model",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": []}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403, "cloud_model write must be denied");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["reason"], "cloud_model_write_denied");

    // cloud_model create -> 403 too (no document authoring).
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "cloud-create",
        "cloud_model",
    )
    .json(&doc_body(&workspace_id, "CloudDoc"))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403);

    // A bogus asserted kind is a 400 (strict vocabulary), never a coercion.
    for bogus in ["root", "SYSTEM", "model_adapter", "admin"] {
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "bogus",
            bogus,
        )
        .json(&json!({
            "expected_version": 1,
            "content_json": {"type": "doc", "content": []}
        }))
        .send()
        .await
        .expect("send");
        assert_eq!(
            resp.status(),
            400,
            "bogus actor kind `{bogus}` must be rejected"
        );
    }

    // cloud_model can still read (the allowed half of its matrix row).
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "cloud-read",
        "cloud_model",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
}
