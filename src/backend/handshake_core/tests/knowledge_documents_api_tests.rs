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
use handshake_core::storage::knowledge::KnowledgeStore;
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, NewLoomBlock, NewLoomCanvasPlacement,
    WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{base_database_url, knowledge_pg, KnowledgePg, PANIC_CLEANUP_TIMEOUT};
use serde_json::{json, Value};
use sqlx::Connection;

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
struct DocServerGuard(Option<tokio::task::JoinHandle<()>>);

impl DocServerGuard {
    async fn shutdown(mut self) {
        let handle = self.0.take().expect("server handle owned");
        handle.abort();
        let error = handle
            .await
            .expect_err("aborted server must not complete normally");
        assert!(error.is_cancelled(), "server shutdown must be cancellation");
    }
}

impl Drop for DocServerGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
            if !std::thread::panicking() {
                let runtime = tokio::runtime::Handle::current();
                tokio::task::block_in_place(|| {
                    let result = runtime.block_on(handle);
                    assert!(
                        result.is_err_and(|error| error.is_cancelled()),
                        "dropped server must abort and join"
                    );
                });
            }
        }
    }
}

async fn test_state(pg: &KnowledgePg) -> AppState {
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
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("docs-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

async fn route_server(app: axum::Router) -> (String, reqwest::Client, DocServerGuard) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("docs api server");
    });
    (
        format!("http://{addr}"),
        reqwest::Client::new(),
        DocServerGuard(Some(handle)),
    )
}

async fn doc_server(pg: &KnowledgePg) -> (String, reqwest::Client, DocServerGuard) {
    route_server(docs_api::routes(test_state(pg).await)).await
}

async fn loom_server(pg: &KnowledgePg) -> (String, reqwest::Client, DocServerGuard) {
    route_server(handshake_core::api::loom::routes(test_state(pg).await)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_schema_guard_drop_cleans_during_unwind_without_explicit_teardown() {
    let base_url = base_database_url()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let schema = pg.schema.clone();
    let held_connection = pg.raw_connection().await;
    let unwind = tokio::time::timeout(
        PANIC_CLEANUP_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                // Reverse drop order intentionally drops KnowledgePg while a
                // fixture-scoped PostgreSQL connection is still outstanding.
                let _held_connection = held_connection;
                let _pg = pg;
                panic!("intentional MT-032 schema-guard unwind");
            }))
        }),
    )
    .await
    .expect("panic-path cleanup must finish inside its external bound")
    .expect("join panic-path cleanup task");
    assert!(
        unwind.is_err(),
        "fixture must have traversed the panic path"
    );

    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect after KnowledgePg drop cleanup");
    let remains: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(&schema)
            .fetch_one(&mut conn)
            .await
            .expect("verify Drop removed isolated schema");
    assert!(!remains, "KnowledgePg Drop must remove its isolated schema");
    conn.close()
        .await
        .expect("close Drop cleanup verification connection");
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explorer_document_rename_rejects_stale_token_without_overwrite() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for stale-rename proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "Original title").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("created document id");
    let original_updated_at = created["document"]["updated_at"]
        .as_str()
        .expect("explorer concurrency token");

    let listed = identity_headers(
        http.get(format!(
            "{base}/knowledge/documents?workspace_id={workspace_id}"
        )),
        "explorer-list",
    )
    .send()
    .await
    .expect("explorer authority list");
    assert_eq!(listed.status(), 200);
    let listed: Value = listed.json().await.expect("explorer list body");
    assert_eq!(listed[0]["rich_document_id"], document_id);
    assert_eq!(listed[0]["updated_at"], original_updated_at);

    let first = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{document_id}/rename")),
        "rename-first",
        "operator",
    )
    .json(&json!({
        "title": "First writer",
        "expected_updated_at": original_updated_at,
    }))
    .send()
    .await
    .expect("first rename");
    assert_eq!(first.status(), 200);

    let stale = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{document_id}/rename")),
        "rename-stale",
        "operator",
    )
    .json(&json!({
        "title": "Stale overwrite",
        "expected_updated_at": original_updated_at,
    }))
    .send()
    .await
    .expect("stale rename");
    assert_eq!(stale.status(), 409, "stale explorer write must conflict");

    let readback = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{document_id}")),
        "rename-readback",
        "operator",
    )
    .send()
    .await
    .expect("document readback");
    assert_eq!(readback.status(), 200);
    let readback: Value = readback.json().await.expect("readback body");
    assert_eq!(readback["document"]["title"], "First writer");
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explorer_bookmark_rename_returns_409_and_preserves_first_writer() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for bookmark stale-rename proof");
    let workspace_id = pg.create_workspace().await;
    let created = pg
        .db
        .create_loom_block(
            &WriteContext::human(None),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Pinned original".to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: true,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create pinned Loom block");
    let (base, http, server) = loom_server(&pg).await;
    let url = format!(
        "{base}/workspaces/{workspace_id}/loom/blocks/{}",
        created.block_id
    );
    let expected_updated_at = created.updated_at.to_rfc3339();
    let first = headers_with_kind(http.patch(&url), "bookmark-first", "operator")
        .json(&json!({
            "title": "Pinned first writer",
            "expected_updated_at": expected_updated_at,
        }))
        .send()
        .await
        .expect("first bookmark rename");
    assert_eq!(first.status(), 200);

    let stale = headers_with_kind(http.patch(&url), "bookmark-stale", "operator")
        .json(&json!({
            "title": "Pinned stale overwrite",
            "expected_updated_at": expected_updated_at,
        }))
        .send()
        .await
        .expect("stale bookmark rename");
    assert_eq!(stale.status(), 409);

    let readback = http.get(&url).send().await.expect("bookmark readback");
    assert_eq!(readback.status(), 200);
    let readback: Value = readback.json().await.expect("bookmark readback body");
    assert_eq!(readback["title"], "Pinned first writer");
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_create_if_title_absent_returns_one_canonical_document() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for concurrent-create proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let title = "Concurrent   Design Note";
    let mut body = doc_body(&workspace_id, title);
    body["create_if_title_absent"] = Value::Bool(true);

    let send = |label: &'static str| {
        headers_with_kind(
            http.post(format!("{base}/knowledge/documents")),
            label,
            "operator",
        )
        .json(&body)
        .send()
    };
    let (left, right) = tokio::join!(send("create-race-left"), send("create-race-right"));
    let left = left.expect("left concurrent create response");
    let right = right.expect("right concurrent create response");
    assert_eq!(left.status(), 200);
    assert_eq!(right.status(), 200);
    let left: Value = left.json().await.expect("left create body");
    let right: Value = right.json().await.expect("right create body");
    assert_eq!(
        left["document"]["rich_document_id"], right["document"]["rich_document_id"],
        "both clients must converge on the backend-authoritative document"
    );
    assert_ne!(
        left["created"], right["created"],
        "exactly one caller creates and one idempotently observes"
    );

    let mut conn = pg.raw_connection().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_rich_documents \
         WHERE workspace_id = $1 AND deleted_at IS NULL \
           AND regexp_replace(lower(btrim(title)), '[[:space:]]+', ' ', 'g') = $2",
    )
    .bind(&workspace_id)
    .bind("concurrent design note")
    .fetch_one(&mut conn)
    .await
    .expect("canonical title count");
    assert_eq!(
        count, 1,
        "concurrent create leaves one canonical authority row"
    );
    conn.close().await.expect("close proof connection");
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_if_title_absent_rejects_preexisting_normalized_title_ambiguity() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for ambiguity proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    create_doc(&base, &http, &workspace_id, "Ambiguous   Design Note").await;
    create_doc(&base, &http, &workspace_id, "ambiguous design note").await;

    let mut body = doc_body(&workspace_id, "AMBIGUOUS DESIGN NOTE");
    body["create_if_title_absent"] = Value::Bool(true);
    let response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "create-ambiguous",
        "operator",
    )
    .json(&body)
    .send()
    .await
    .expect("ambiguous create response");
    assert_eq!(response.status(), 409);
    let error: Value = response.json().await.expect("typed ambiguity body");
    assert_eq!(error["error"], "conflict");
    assert_eq!(error["detail"], "knowledge_rich_document_title_ambiguous");

    let mut conn = pg.raw_connection().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_rich_documents \
         WHERE workspace_id = $1 AND deleted_at IS NULL \
           AND regexp_replace(lower(btrim(title)), '[[:space:]]+', ' ', 'g') = $2",
    )
    .bind(&workspace_id)
    .bind("ambiguous design note")
    .fetch_one(&mut conn)
    .await
    .expect("ambiguous title count");
    assert_eq!(count, 2, "failed special create must not add a third row");
    conn.close().await.expect("close proof connection");
    server.shutdown().await;
}

#[derive(Debug, PartialEq)]
struct Mt032DeleteDependencySnapshot {
    document: (Option<chrono::DateTime<chrono::Utc>>, i64, String),
    block: (String, String, Option<String>, Option<String>),
    search: (String, String, String),
    source_stale: bool,
    delete_receipts: i64,
    inbound_backlinks: i64,
    canvas_placements: i64,
}

async fn mt032_delete_dependency_snapshot(
    conn: &mut sqlx::PgConnection,
    workspace_id: &str,
    document_id: &str,
    placement_id: &str,
) -> Mt032DeleteDependencySnapshot {
    let document = sqlx::query_as(
        "SELECT deleted_at, doc_version, title \
         FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(document_id)
    .fetch_one(&mut *conn)
    .await
    .expect("snapshot rich document authority");
    let block = sqlx::query_as(
        "SELECT workspace_id, content_type, title, content_hash \
         FROM loom_blocks WHERE block_id = $1",
    )
    .bind(document_id)
    .fetch_one(&mut *conn)
    .await
    .expect("snapshot LoomBlock projection");
    let search = sqlx::query_as(
        "SELECT workspace_id, content_type, search_text \
         FROM loom_block_search_index WHERE block_id = $1",
    )
    .bind(document_id)
    .fetch_one(&mut *conn)
    .await
    .expect("snapshot LoomBlock search projection");
    let source_stale = sqlx::query_scalar(
        "SELECT stale FROM knowledge_sources \
         WHERE workspace_id = $1 \
           AND source_kind = 'rich_document' \
           AND provenance->>'rich_document_id' = $2",
    )
    .bind(workspace_id)
    .bind(document_id)
    .fetch_one(&mut *conn)
    .await
    .expect("snapshot knowledge source state");
    let delete_receipts = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'knowledge_rich_document' \
           AND aggregate_id = $1 \
           AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED'",
    )
    .bind(document_id)
    .fetch_one(&mut *conn)
    .await
    .expect("snapshot delete receipt count");
    let inbound_backlinks =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_document_backlinks WHERE target = $1")
            .bind(document_id)
            .fetch_one(&mut *conn)
            .await
            .expect("snapshot inbound backlink count");
    let canvas_placements =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_canvas_placements WHERE placement_id = $1")
            .bind(placement_id)
            .fetch_one(&mut *conn)
            .await
            .expect("snapshot canvas placement count");

    Mt032DeleteDependencySnapshot {
        document,
        block,
        search,
        source_stale,
        delete_receipts,
        inbound_backlinks,
        canvas_placements,
    }
}

// ---------------------------------------------------------------------------
// WP-KERNEL-012 MT-032: RichDocument <-> LoomBlock atomic projection and
// target-inbound backlink route.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_rich_documents_are_addressable_and_target_backlinks_are_inbound() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    let b = create_doc(&base, &http, &workspace_id, "MT032 Target B").await;
    let b_id = b["document"]["rich_document_id"]
        .as_str()
        .expect("B document id")
        .to_string();
    assert_eq!(b["document"]["block_id"].as_str(), Some(b_id.as_str()));

    let loaded_b = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{b_id}")),
        "mt032-load-b",
        "operator",
    )
    .send()
    .await
    .expect("load B");
    assert_eq!(loaded_b.status(), 200);
    let loaded_b: Value = loaded_b.json().await.expect("load B body");
    assert_eq!(loaded_b["document"]["block_id"], b_id);

    let a_body = json!({
        "workspace_id": workspace_id,
        "title": "MT032 Source A",
        "content_json": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "see [[MT032 Target B]]"}]
            }]
        }
    });
    let a_response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-a",
        "operator",
    )
    .json(&a_body)
    .send()
    .await
    .expect("create A");
    assert_eq!(a_response.status(), 200);
    let a: Value = a_response.json().await.expect("A body");
    let a_id = a["document"]["rich_document_id"]
        .as_str()
        .expect("A document id")
        .to_string();

    let direct_inbound = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{b_id}/backlinks")),
        "mt032-direct-create-inbound",
        "operator",
    )
    .send()
    .await
    .expect("list direct-create inbound backlinks");
    assert_eq!(direct_inbound.status(), 200);
    let direct_inbound: Value = direct_inbound.json().await.expect("direct inbound body");
    assert!(direct_inbound["backlinks"]
        .as_array()
        .expect("direct inbound backlinks")
        .iter()
        .any(|row| row["source_document_id"] == a_id && row["target"] == b_id));

    let rebuild = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{a_id}/backlinks")),
        "mt032-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("rebuild A backlinks");
    assert_eq!(rebuild.status(), 200);
    let rebuilt: Value = rebuild.json().await.expect("rebuild body");
    assert!(rebuilt["backlinks"]
        .as_array()
        .expect("rebuilt backlinks")
        .iter()
        .any(|row| row["target"] == b_id));

    let inbound = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{b_id}/backlinks")),
        "mt032-inbound",
        "operator",
    )
    .send()
    .await
    .expect("list B inbound backlinks");
    assert_eq!(inbound.status(), 200);
    let inbound: Value = inbound.json().await.expect("inbound body");
    assert_eq!(inbound["source_document_id"], b_id);
    assert!(inbound["backlinks"]
        .as_array()
        .expect("backlinks array")
        .iter()
        .any(|row| row["source_document_id"] == a_id && row["target"] == b_id));

    // An ordinary wikilink title is promoted to a stable id only when the
    // target title is unambiguous in the workspace. Duplicate titles must
    // remain unresolved instead of silently corrupting either target's
    // inbound graph.
    let duplicate_one = create_doc(&base, &http, &workspace_id, "MT032 Duplicate").await;
    let duplicate_one_id = duplicate_one["document"]["rich_document_id"]
        .as_str()
        .expect("first duplicate id")
        .to_string();
    let duplicate_two = create_doc(&base, &http, &workspace_id, "MT032 Duplicate").await;
    let duplicate_two_id = duplicate_two["document"]["rich_document_id"]
        .as_str()
        .expect("second duplicate id")
        .to_string();
    let ambiguous_source_response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-ambiguous-source",
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "MT032 Ambiguous Source",
        "content_json": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "see [[MT032 Duplicate]]"}]
            }]
        }
    }))
    .send()
    .await
    .expect("create ambiguous source");
    assert_eq!(ambiguous_source_response.status(), 200);
    let ambiguous_source: Value = ambiguous_source_response
        .json()
        .await
        .expect("ambiguous source body");
    let ambiguous_source_id = ambiguous_source["document"]["rich_document_id"]
        .as_str()
        .expect("ambiguous source id")
        .to_string();
    let ambiguous_rebuild = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{ambiguous_source_id}/backlinks"
        )),
        "mt032-ambiguous-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("rebuild ambiguous backlinks");
    assert_eq!(ambiguous_rebuild.status(), 200);
    let ambiguous_rebuild: Value = ambiguous_rebuild
        .json()
        .await
        .expect("ambiguous rebuild body");
    assert!(ambiguous_rebuild["backlinks"]
        .as_array()
        .expect("ambiguous backlinks")
        .iter()
        .any(|row| row["target"] == "MT032 Duplicate"));
    for duplicate_id in [&duplicate_one_id, &duplicate_two_id] {
        let duplicate_inbound = headers_with_kind(
            http.get(format!(
                "{base}/knowledge/documents/{duplicate_id}/backlinks"
            )),
            "mt032-ambiguous-inbound",
            "operator",
        )
        .send()
        .await
        .expect("list duplicate inbound backlinks");
        assert_eq!(duplicate_inbound.status(), 200);
        let duplicate_inbound: Value = duplicate_inbound
            .json()
            .await
            .expect("duplicate inbound body");
        assert!(!duplicate_inbound["backlinks"]
            .as_array()
            .expect("duplicate inbound backlinks")
            .iter()
            .any(|row| row["source_document_id"] == ambiguous_source_id));
    }
    let delete_duplicate = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{duplicate_one_id}")),
        "mt032-delete-one-duplicate",
        "operator",
    )
    .send()
    .await
    .expect("delete one duplicate");
    assert_eq!(delete_duplicate.status(), 200);
    let remaining_duplicate_inbound = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{duplicate_two_id}/backlinks"
        )),
        "mt032-remaining-duplicate-inbound",
        "operator",
    )
    .send()
    .await
    .expect("list remaining duplicate inbound backlinks");
    assert_eq!(remaining_duplicate_inbound.status(), 200);
    let remaining_duplicate_inbound: Value = remaining_duplicate_inbound
        .json()
        .await
        .expect("remaining duplicate inbound body");
    assert!(remaining_duplicate_inbound["backlinks"]
        .as_array()
        .expect("remaining duplicate backlinks")
        .iter()
        .any(|row| {
            row["source_document_id"] == ambiguous_source_id && row["target"] == "MT032 Duplicate"
        }));

    let saved_content = json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [
                {"type": "text", "text": "saved B "},
                {"type": "hsLink", "attrs": {
                    "refKind": "locus",
                    "refValue": "mt/MT-032-SEARCH-PROJECTION",
                    "label": "MT"
                }}
            ]
        }]
    });
    let save = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{b_id}/save")),
        "mt032-save",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": saved_content.clone()}))
    .send()
    .await
    .expect("save B");
    assert_eq!(save.status(), 200);
    let saved: Value = save.json().await.expect("save body");
    assert_eq!(saved["document"]["block_id"], b_id);
    let document_hash = saved["document"]["content_sha256"]
        .as_str()
        .expect("document hash")
        .to_string();

    let mut conn = pg.raw_connection().await;
    let block: (String, String) = sqlx::query_as(
        "SELECT block_id, content_hash FROM loom_blocks WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&workspace_id)
    .bind(&b_id)
    .fetch_one(&mut conn)
    .await
    .expect("same-id LoomBlock projection");
    assert_eq!(block.0, b_id);
    assert_eq!(block.1, document_hash);

    let indexed_text: String =
        sqlx::query_scalar("SELECT search_text FROM loom_block_search_index WHERE block_id = $1")
            .bind(&b_id)
            .fetch_one(&mut conn)
            .await
            .expect("saved document search projection");
    assert!(indexed_text.contains("saved B"));
    assert!(
        indexed_text.contains("mt/MT-032-SEARCH-PROJECTION")
            && indexed_text.contains("locus://mt/mt-032-search-projection"),
        "prefix-stripped hsLink refValue and its canonical normalized Locus URI must be searchable even when the compact label omits the identity"
    );

    let rename = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{b_id}/rename")),
        "mt032-rename-b",
        "operator",
    )
    .json(&json!({"title": "MT032 Target B Renamed"}))
    .send()
    .await
    .expect("rename B");
    assert_eq!(rename.status(), 200);
    let renamed: Value = rename.json().await.expect("rename body");
    assert_eq!(renamed["document"]["block_id"], b_id);
    let projected_title: String =
        sqlx::query_scalar("SELECT title FROM loom_blocks WHERE block_id = $1")
            .bind(&b_id)
            .fetch_one(&mut conn)
            .await
            .expect("renamed LoomBlock title");
    assert_eq!(projected_title, "MT032 Target B Renamed");

    let resave_a = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{a_id}/save")),
        "mt032-resave-after-target-rename",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": a_body["content_json"].clone()
    }))
    .send()
    .await
    .expect("resave source after target rename");
    assert_eq!(resave_a.status(), 200);
    let renamed_target_inbound = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{b_id}/backlinks")),
        "mt032-renamed-target-inbound",
        "operator",
    )
    .send()
    .await
    .expect("list renamed target inbound backlinks");
    assert_eq!(renamed_target_inbound.status(), 200);
    let renamed_target_inbound: Value = renamed_target_inbound
        .json()
        .await
        .expect("renamed target inbound body");
    assert!(renamed_target_inbound["backlinks"]
        .as_array()
        .expect("renamed target backlinks")
        .iter()
        .any(|row| row["source_document_id"] == a_id && row["target"] == b_id));

    let batch = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/batch")),
        "mt032-batch-block-id",
        "operator",
    )
    .json(&json!({
        "operations": [{
            "op": "set_authority_label",
            "document_id": b_id,
            "authority_label": "promoted"
        }]
    }))
    .send()
    .await
    .expect("batch stable block id");
    assert_eq!(batch.status(), 200);
    let batch: Value = batch.json().await.expect("batch body");
    assert_eq!(batch["results"][0]["block_id"], b_id);

    let local_model_create = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-local-model",
        "local_model",
    )
    .json(&doc_body(&workspace_id, "MT032 Local Model"))
    .send()
    .await
    .expect("local-model create");
    assert_eq!(local_model_create.status(), 200);

    let imported = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/import")),
        "mt032-import",
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "MT032 Imported",
        "format": "markdown",
        "snippet": "# Imported\n\nbody [[MT032 Target B Renamed]]"
    }))
    .send()
    .await
    .expect("import document");
    assert_eq!(imported.status(), 200);
    let imported: Value = imported.json().await.expect("import body");
    let imported_id = imported["document"]["rich_document_id"]
        .as_str()
        .expect("imported document id");
    assert_eq!(imported["document"]["block_id"], imported_id);
    let imported_blocks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&workspace_id)
    .bind(imported_id)
    .fetch_one(&mut conn)
    .await
    .expect("one same-id imported block");
    assert_eq!(imported_blocks, 1);
    let imported_inbound = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{b_id}/backlinks")),
        "mt032-direct-import-inbound",
        "operator",
    )
    .send()
    .await
    .expect("list direct-import inbound backlinks");
    assert_eq!(imported_inbound.status(), 200);
    let imported_inbound: Value = imported_inbound.json().await.expect("import inbound body");
    assert!(imported_inbound["backlinks"]
        .as_array()
        .expect("import inbound backlinks")
        .iter()
        .any(|row| row["source_document_id"] == imported_id && row["target"] == b_id));
    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_loom_projection_failure_rolls_back_document_create() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let mut conn = pg.raw_connection().await;
    sqlx::query("DROP TABLE loom_block_search_index")
        .execute(&mut conn)
        .await
        .expect("inject projection failure in isolated schema");

    let response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-rollback",
        "operator",
    )
    .json(&doc_body(&workspace_id, "MT032 Must Roll Back"))
    .send()
    .await
    .expect("create with injected projection failure");
    assert_eq!(response.status(), 500);

    let document_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_rich_documents WHERE workspace_id = $1 AND title = $2",
    )
    .bind(&workspace_id)
    .bind("MT032 Must Roll Back")
    .fetch_one(&mut conn)
    .await
    .expect("count rolled-back documents");
    let block_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_blocks WHERE workspace_id = $1 AND title = $2",
    )
    .bind(&workspace_id)
    .bind("MT032 Must Roll Back")
    .fetch_one(&mut conn)
    .await
    .expect("count rolled-back blocks");
    assert_eq!(
        document_count, 0,
        "document insert rolls back with projection failure"
    );
    assert_eq!(block_count, 0, "LoomBlock insert rolls back with document");
    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_save_rejects_same_workspace_wrong_type_projection_collision() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 Wrong Type").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("wrong-type document id")
        .to_string();
    let mut conn = pg.raw_connection().await;
    sqlx::query("UPDATE loom_blocks SET content_type = 'file' WHERE block_id = $1")
        .bind(&document_id)
        .execute(&mut conn)
        .await
        .expect("inject same-workspace incompatible projection type");

    let response = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt032-wrong-type-collision",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": "must not project"}]
        }]}
    }))
    .send()
    .await
    .expect("save against wrong-type projection");
    assert_eq!(response.status(), 409);

    let delete = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-wrong-type-delete",
        "operator",
    )
    .send()
    .await
    .expect("delete against wrong-type projection");
    assert_eq!(delete.status(), 409);

    let (content_type, title): (String, Option<String>) =
        sqlx::query_as("SELECT content_type, title FROM loom_blocks WHERE block_id = $1")
            .bind(&document_id)
            .fetch_one(&mut conn)
            .await
            .expect("wrong-type block survives conflict unchanged");
    let doc_version: i64 = sqlx::query_scalar(
        "SELECT doc_version FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("document save rolled back on projection conflict");
    let (deleted_at, deleted_receipt_event_id): (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT deleted_at, deleted_receipt_event_id \
         FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("wrong-type delete document state");
    let delete_receipts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'knowledge_rich_document' \
           AND aggregate_id = $1 \
           AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED'",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("wrong-type delete receipt count");
    let search_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_block_search_index WHERE block_id = $1")
            .bind(&document_id)
            .fetch_one(&mut conn)
            .await
            .expect("wrong-type delete keeps search dependency");
    let source_stale: bool = sqlx::query_scalar(
        "SELECT stale FROM knowledge_sources \
         WHERE workspace_id = $1 \
           AND source_kind = 'rich_document' \
           AND provenance->>'rich_document_id' = $2",
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("wrong-type delete keeps source dependency live");
    assert_eq!(content_type, "file");
    assert_eq!(title.as_deref(), Some("MT032 Wrong Type"));
    assert_eq!(doc_version, 1);
    assert!(deleted_at.is_none());
    assert!(deleted_receipt_event_id.is_none());
    assert_eq!(delete_receipts, 0);
    assert_eq!(search_rows, 1);
    assert!(!source_stale);

    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_save_and_rename_reject_search_projection_identity_collisions() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let foreign_workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 Search Identity").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("search-identity document id")
        .to_string();
    let mut conn = pg.raw_connection().await;

    sqlx::query("UPDATE loom_block_search_index SET content_type = 'file' WHERE block_id = $1")
        .bind(&document_id)
        .execute(&mut conn)
        .await
        .expect("inject wrong-type search projection");
    let save = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt032-search-wrong-type-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": "must roll back"}]
        }]}
    }))
    .send()
    .await
    .expect("save against wrong-type search projection");
    assert_eq!(save.status(), 409);
    let (doc_version, search_type): (i64, String) = sqlx::query_as(
        "SELECT d.doc_version, s.content_type \
         FROM knowledge_rich_documents d \
         JOIN loom_block_search_index s ON s.block_id = d.rich_document_id \
         WHERE d.rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("wrong-type search save rollback state");
    assert_eq!(doc_version, 1);
    assert_eq!(search_type, "file");

    sqlx::query(
        "UPDATE loom_block_search_index \
         SET content_type = 'note', workspace_id = $2 \
         WHERE block_id = $1",
    )
    .bind(&document_id)
    .bind(&foreign_workspace_id)
    .execute(&mut conn)
    .await
    .expect("inject cross-workspace search projection");
    let rename = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{document_id}/rename")),
        "mt032-search-cross-workspace-rename",
        "operator",
    )
    .json(&json!({"title": "Must Not Rename"}))
    .send()
    .await
    .expect("rename against cross-workspace search projection");
    assert_eq!(rename.status(), 409);
    let (document_title, block_title, search_workspace): (String, Option<String>, String) =
        sqlx::query_as(
            "SELECT d.title, b.title, s.workspace_id \
             FROM knowledge_rich_documents d \
             JOIN loom_blocks b ON b.block_id = d.rich_document_id \
             JOIN loom_block_search_index s ON s.block_id = d.rich_document_id \
             WHERE d.rich_document_id = $1",
        )
        .bind(&document_id)
        .fetch_one(&mut conn)
        .await
        .expect("cross-workspace rename rollback state");
    assert_eq!(document_title, "MT032 Search Identity");
    assert_eq!(block_title.as_deref(), Some("MT032 Search Identity"));
    assert_eq!(search_workspace, foreign_workspace_id);

    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_delete_rejects_search_projection_identity_collisions_atomically() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let foreign_workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let target = create_doc(&base, &http, &workspace_id, "MT032 Delete Search Identity").await;
    let document_id = target["document"]["rich_document_id"]
        .as_str()
        .expect("delete search-identity document id")
        .to_string();

    let referrer_response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-delete-search-referrer",
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "MT032 Delete Search Referrer",
        "content_json": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{
                    "type": "hsLink",
                    "attrs": {
                        "refKind": "note",
                        "refValue": document_id,
                        "label": "MT032 Delete Search Identity"
                    }
                }]
            }]
        }
    }))
    .send()
    .await
    .expect("create delete search-identity referrer");
    assert_eq!(referrer_response.status(), 200);
    let referrer: Value = referrer_response
        .json()
        .await
        .expect("delete search-identity referrer body");
    let referrer_id = referrer["document"]["rich_document_id"]
        .as_str()
        .expect("delete search-identity referrer id");
    let rebuilt = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referrer_id}/backlinks"
        )),
        "mt032-delete-search-referrer-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("rebuild delete search-identity backlinks");
    assert_eq!(rebuilt.status(), 200);

    let write_ctx = WriteContext::human(Some("mt032-delete-search-identity".to_string()));
    let canvas_block = pg
        .db
        .create_loom_block(
            &write_ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some("MT032 Delete Search Canvas".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create delete search-identity canvas block");
    pg.db
        .bridge_loom_block_to_knowledge(&write_ctx, &workspace_id, &canvas_block.block_id)
        .await
        .expect("bridge delete search-identity canvas block");
    pg.db
        .create_canvas_board(
            &write_ctx,
            &workspace_id,
            &canvas_block.block_id,
            json!({
                "schema_id": "hsk.loom_canvas_board@1",
                "pan_x": 0.0,
                "pan_y": 0.0,
                "zoom": 1.0
            }),
        )
        .await
        .expect("create delete search-identity canvas board");
    let placement = pg
        .db
        .place_block_on_canvas(
            &write_ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_block.block_id,
                workspace_id: workspace_id.clone(),
                placed_block_id: document_id.clone(),
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place delete search-identity document on canvas");

    let mut conn = pg.raw_connection().await;
    sqlx::query("UPDATE loom_block_search_index SET content_type = 'file' WHERE block_id = $1")
        .bind(&document_id)
        .execute(&mut conn)
        .await
        .expect("inject wrong-type search projection for delete");
    let wrong_type_before = mt032_delete_dependency_snapshot(
        &mut conn,
        &workspace_id,
        &document_id,
        &placement.placement_id,
    )
    .await;
    assert!(!wrong_type_before.source_stale);
    assert_eq!(wrong_type_before.delete_receipts, 0);
    assert_eq!(wrong_type_before.inbound_backlinks, 1);
    assert_eq!(wrong_type_before.canvas_placements, 1);

    let wrong_type_delete = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-delete-search-wrong-type",
        "operator",
    )
    .send()
    .await
    .expect("delete against wrong-type search projection");
    assert_eq!(wrong_type_delete.status(), 409);
    let wrong_type_after = mt032_delete_dependency_snapshot(
        &mut conn,
        &workspace_id,
        &document_id,
        &placement.placement_id,
    )
    .await;
    assert_eq!(
        wrong_type_after, wrong_type_before,
        "wrong-type search identity conflict must not mutate delete dependencies"
    );

    sqlx::query(
        "UPDATE loom_block_search_index \
         SET content_type = 'note', workspace_id = $2 \
         WHERE block_id = $1",
    )
    .bind(&document_id)
    .bind(&foreign_workspace_id)
    .execute(&mut conn)
    .await
    .expect("inject cross-workspace search projection for delete");
    let cross_workspace_before = mt032_delete_dependency_snapshot(
        &mut conn,
        &workspace_id,
        &document_id,
        &placement.placement_id,
    )
    .await;

    let cross_workspace_delete = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-delete-search-cross-workspace",
        "operator",
    )
    .send()
    .await
    .expect("delete against cross-workspace search projection");
    assert_eq!(cross_workspace_delete.status(), 409);
    let cross_workspace_after = mt032_delete_dependency_snapshot(
        &mut conn,
        &workspace_id,
        &document_id,
        &placement.placement_id,
    )
    .await;
    assert_eq!(
        cross_workspace_after, cross_workspace_before,
        "cross-workspace search identity conflict must not mutate delete dependencies"
    );

    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_idempotent_save_projects_body_once_and_replays_without_writes() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 Idempotent").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("idempotent document id")
        .to_string();
    let content = json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": "idempotent projected body"}]
        }]
    });
    let first = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            "mt032-idempotent-save-key",
            &document_id,
            1,
            content.clone(),
            None,
            None,
            None,
        )
        .await
        .expect("first idempotent save");
    assert!(!first.replayed);
    assert_eq!(first.value.doc_version, 2);
    assert_eq!(first.value.block_id, document_id);

    let replay = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            "mt032-idempotent-save-key",
            &document_id,
            1,
            content,
            None,
            None,
            None,
        )
        .await
        .expect("replayed idempotent save");
    assert!(replay.replayed);
    assert_eq!(replay.value.doc_version, 2);
    assert_eq!(replay.value.content_sha256, first.value.content_sha256);

    let mut conn = pg.raw_connection().await;
    let (content_hash, derived_json, search_text): (String, serde_json::Value, String) =
        sqlx::query_as(
            "SELECT b.content_hash, b.derived_json::jsonb, s.search_text \
             FROM loom_blocks b \
             JOIN loom_block_search_index s ON s.block_id = b.block_id \
             WHERE b.workspace_id = $1 AND b.block_id = $2",
        )
        .bind(&workspace_id)
        .bind(&document_id)
        .fetch_one(&mut conn)
        .await
        .expect("idempotent Loom projection");
    assert_eq!(content_hash, first.value.content_sha256);
    assert_eq!(derived_json["full_text_index"], "idempotent projected body");
    assert!(search_text.contains("idempotent projected body"));
    let version_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_rich_document_versions WHERE rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("idempotent version count");
    assert_eq!(version_count, 2);
    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_markdown_import_bridge_failure_rolls_back_and_retry_creates_one() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let mut conn = pg.raw_connection().await;
    sqlx::query(
        r#"
        CREATE FUNCTION mt032_reject_import_bridge()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'MT032 injected import bridge failure';
        END
        $$
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("create import bridge failure function");
    sqlx::query(
        "CREATE TRIGGER mt032_reject_import_bridge \
         BEFORE INSERT ON loom_block_knowledge_bridge FOR EACH ROW \
         EXECUTE FUNCTION mt032_reject_import_bridge()",
    )
    .execute(&mut conn)
    .await
    .expect("create import bridge failure trigger");
    let ctx = WriteContext::human(Some("mt032-import-atomicity".to_string()));
    let failed = pg
        .db
        .import_markdown_to_loom(
            &ctx,
            &workspace_id,
            "MT032 Atomic Import",
            "# Atomic\n\nbridge body",
        )
        .await;
    assert!(failed.is_err());
    for (table, predicate) in [
        (
            "knowledge_rich_documents",
            "workspace_id = $1 AND title = 'MT032 Atomic Import'",
        ),
        (
            "loom_blocks",
            "workspace_id = $1 AND title = 'MT032 Atomic Import'",
        ),
        (
            "knowledge_entities",
            "workspace_id = $1 AND display_name = 'MT032 Atomic Import'",
        ),
    ] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"))
                .bind(&workspace_id)
                .fetch_one(&mut conn)
                .await
                .expect("failed import partial-row count");
        assert_eq!(count, 0, "failed import leaves no row in {table}");
    }
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE source_component = 'loom_block_knowledge_bridge'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("failed import receipt count");
    assert_eq!(receipt_count, 0);
    sqlx::query("DROP TRIGGER mt032_reject_import_bridge ON loom_block_knowledge_bridge")
        .execute(&mut conn)
        .await
        .expect("drop import bridge failure trigger");
    sqlx::query("DROP FUNCTION mt032_reject_import_bridge()")
        .execute(&mut conn)
        .await
        .expect("drop import bridge failure function");

    let imported = pg
        .db
        .import_markdown_to_loom(
            &ctx,
            &workspace_id,
            "MT032 Atomic Import",
            "# Atomic\n\nbridge body",
        )
        .await
        .expect("retry atomic import");
    assert_eq!(imported.block.block_id, imported.rich_document_id);
    let closure_counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM knowledge_rich_documents WHERE rich_document_id = $1),
            (SELECT COUNT(*) FROM loom_blocks WHERE block_id = $1),
            (SELECT COUNT(*) FROM loom_block_knowledge_bridge WHERE block_id = $1),
            (SELECT COUNT(*) FROM kernel_event_ledger
             WHERE source_component = 'loom_block_knowledge_bridge'
               AND payload->>'block_id' = $1)
        "#,
    )
    .bind(&imported.rich_document_id)
    .fetch_one(&mut conn)
    .await
    .expect("successful import closure counts");
    assert_eq!(closure_counts, (1, 1, 1, 1));
    drop(conn);
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt032_save_delete_and_backlink_rebuild_delete_races_do_not_resurrect() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let mut conn = pg.raw_connection().await;
    sqlx::query(
        r#"
        CREATE FUNCTION mt032_hold_mutation()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            PERFORM pg_sleep(0.4);
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("create mutation hold function");

    // Save obtains the live document lock first; delete waits, then removes
    // the freshly advanced projection. Final state is tombstoned/no Loom.
    let save_first = create_doc(&base, &http, &workspace_id, "MT032 Save First").await;
    let save_first_id = save_first["document"]["rich_document_id"]
        .as_str()
        .expect("save-first id")
        .to_string();
    sqlx::query(&format!(
        "CREATE TRIGGER mt032_hold_save_first BEFORE UPDATE ON knowledge_rich_documents \
         FOR EACH ROW WHEN (OLD.rich_document_id = '{}' AND NEW.doc_version > OLD.doc_version) \
         EXECUTE FUNCTION mt032_hold_mutation()",
        save_first_id.replace('\'', "''")
    ))
    .execute(&mut conn)
    .await
    .expect("create save-first trigger");
    let save_task = {
        let http = http.clone();
        let base = base.clone();
        let id = save_first_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.put(format!("{base}/knowledge/documents/{id}/save")),
                "mt032-race-save-first",
                "operator",
            )
            .json(&json!({
                "expected_version": 1,
                "content_json": {"type": "doc", "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": "save wins lock first"}]
                }]}
            }))
            .send()
            .await
            .expect("save-first request")
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let delete_task = {
        let http = http.clone();
        let base = base.clone();
        let id = save_first_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.delete(format!("{base}/knowledge/documents/{id}")),
                "mt032-race-delete-second",
                "operator",
            )
            .send()
            .await
            .expect("delete-second request")
        })
    };
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), save_task)
            .await
            .expect("save-first must complete without deadlock")
            .expect("join save-first")
            .status(),
        200
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), delete_task)
            .await
            .expect("delete-second must complete without deadlock")
            .expect("join delete-second")
            .status(),
        200
    );
    let deletion_payload: Value = sqlx::query_scalar(
        "SELECT kel.payload FROM knowledge_rich_documents d \
         JOIN kernel_event_ledger kel ON kel.event_id = d.deleted_receipt_event_id \
         WHERE d.rich_document_id = $1",
    )
    .bind(&save_first_id)
    .fetch_one(&mut conn)
    .await
    .expect("load save-then-delete receipt payload");
    assert_eq!(deletion_payload["doc_version"], json!(2));
    assert_eq!(deletion_payload["title"], json!("MT032 Save First"));
    sqlx::query("DROP TRIGGER mt032_hold_save_first ON knowledge_rich_documents")
        .execute(&mut conn)
        .await
        .expect("drop save-first trigger");

    // Delete obtains the document lock first; save waits, observes tombstone,
    // and returns NotFound without recreating the projection.
    let delete_first = create_doc(&base, &http, &workspace_id, "MT032 Delete First").await;
    let delete_first_id = delete_first["document"]["rich_document_id"]
        .as_str()
        .expect("delete-first id")
        .to_string();
    sqlx::query(&format!(
        "CREATE TRIGGER mt032_hold_delete_first BEFORE UPDATE ON knowledge_rich_documents \
         FOR EACH ROW WHEN (OLD.rich_document_id = '{}' AND NEW.deleted_at IS NOT NULL) \
         EXECUTE FUNCTION mt032_hold_mutation()",
        delete_first_id.replace('\'', "''")
    ))
    .execute(&mut conn)
    .await
    .expect("create delete-first trigger");
    let delete_task = {
        let http = http.clone();
        let base = base.clone();
        let id = delete_first_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.delete(format!("{base}/knowledge/documents/{id}")),
                "mt032-race-delete-first",
                "operator",
            )
            .send()
            .await
            .expect("delete-first request")
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let save_task = {
        let http = http.clone();
        let base = base.clone();
        let id = delete_first_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.put(format!("{base}/knowledge/documents/{id}/save")),
                "mt032-race-save-second",
                "operator",
            )
            .json(&json!({
                "expected_version": 1,
                "content_json": {"type": "doc", "content": []}
            }))
            .send()
            .await
            .expect("save-second request")
        })
    };
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), delete_task)
            .await
            .expect("delete-first must complete without deadlock")
            .expect("join delete-first")
            .status(),
        200
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), save_task)
            .await
            .expect("save-second must complete without deadlock")
            .expect("join save-second")
            .status(),
        404
    );
    sqlx::query("DROP TRIGGER mt032_hold_delete_first ON knowledge_rich_documents")
        .execute(&mut conn)
        .await
        .expect("drop delete-first trigger");
    assert!(matches!(
        pg.db
            .move_knowledge_rich_document(&delete_first_id, Some("P-DELETED"), None)
            .await,
        Err(handshake_core::storage::StorageError::NotFound(_))
    ));
    assert!(matches!(
        pg.db
            .set_knowledge_rich_document_authority_label(&delete_first_id, "archived")
            .await,
        Err(handshake_core::storage::StorageError::NotFound(_))
    ));

    for id in [&save_first_id, &delete_first_id] {
        let block_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM loom_blocks WHERE block_id = $1")
                .bind(id)
                .fetch_one(&mut conn)
                .await
                .expect("race final Loom count");
        assert_eq!(block_count, 0);
    }

    // Rebuild first: target lock holds delete; delete then removes the row.
    let target = create_doc(&base, &http, &workspace_id, "MT032 Rebuild Target").await;
    let target_id = target["document"]["rich_document_id"]
        .as_str()
        .expect("rebuild target id")
        .to_string();
    let source = create_doc(&base, &http, &workspace_id, "MT032 Rebuild Source").await;
    let source_id = source["document"]["rich_document_id"]
        .as_str()
        .expect("rebuild source id")
        .to_string();
    let source_content = json!({"type": "doc", "content": [{
        "type": "paragraph",
        "content": [{"type": "text", "text": "[[MT032 Rebuild Target]]"}]
    }]});
    let save_source = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{source_id}/save")),
        "mt032-seed-rebuild-source",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": source_content}))
    .send()
    .await
    .expect("seed rebuild source");
    assert_eq!(save_source.status(), 200);
    sqlx::query(
        "CREATE TRIGGER mt032_hold_backlink_insert BEFORE INSERT ON knowledge_document_backlinks \
         FOR EACH ROW EXECUTE FUNCTION mt032_hold_mutation()",
    )
    .execute(&mut conn)
    .await
    .expect("create backlink insert hold trigger");
    let rebuild_task = {
        let http = http.clone();
        let base = base.clone();
        let id = source_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.post(format!("{base}/knowledge/documents/{id}/backlinks")),
                "mt032-race-rebuild-first",
                "operator",
            )
            .send()
            .await
            .expect("rebuild-first request")
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let delete_task = {
        let http = http.clone();
        let base = base.clone();
        let id = target_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.delete(format!("{base}/knowledge/documents/{id}")),
                "mt032-race-target-delete-second",
                "operator",
            )
            .send()
            .await
            .expect("target delete-second request")
        })
    };
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), rebuild_task)
            .await
            .expect("rebuild-first must complete without deadlock")
            .expect("join rebuild-first")
            .status(),
        200
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), delete_task)
            .await
            .expect("target delete must complete without deadlock")
            .expect("join target delete")
            .status(),
        200
    );
    sqlx::query("DROP TRIGGER mt032_hold_backlink_insert ON knowledge_document_backlinks")
        .execute(&mut conn)
        .await
        .expect("drop backlink insert hold trigger");
    let stale_target_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_document_backlinks WHERE target = $1")
            .bind(&target_id)
            .fetch_one(&mut conn)
            .await
            .expect("rebuild-first stale target count");
    assert_eq!(stale_target_count, 0);

    // Delete first: rebuild waits on the target row, then observes its
    // tombstone and does not recreate either a stable-id or stale-title row.
    let target_two = create_doc(
        &base,
        &http,
        &workspace_id,
        "MT032 Delete-First Backlink Target",
    )
    .await;
    let target_two_id = target_two["document"]["rich_document_id"]
        .as_str()
        .expect("delete-first backlink target id")
        .to_string();
    let source_two = create_doc(
        &base,
        &http,
        &workspace_id,
        "MT032 Delete-First Backlink Source",
    )
    .await;
    let source_two_id = source_two["document"]["rich_document_id"]
        .as_str()
        .expect("delete-first backlink source id")
        .to_string();
    let save_source_two = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{source_two_id}/save")),
        "mt032-seed-delete-first-backlink",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": "[[MT032 Delete-First Backlink Target]]"}]
        }]}
    }))
    .send()
    .await
    .expect("seed delete-first backlink");
    assert_eq!(save_source_two.status(), 200);
    sqlx::query(&format!(
        "CREATE TRIGGER mt032_hold_target_delete_first BEFORE UPDATE ON knowledge_rich_documents \
         FOR EACH ROW WHEN (OLD.rich_document_id = '{}' AND NEW.deleted_at IS NOT NULL) \
         EXECUTE FUNCTION mt032_hold_mutation()",
        target_two_id.replace('\'', "''")
    ))
    .execute(&mut conn)
    .await
    .expect("create target delete-first hold trigger");
    let delete_task = {
        let http = http.clone();
        let base = base.clone();
        let id = target_two_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.delete(format!("{base}/knowledge/documents/{id}")),
                "mt032-target-delete-first",
                "operator",
            )
            .send()
            .await
            .expect("target delete-first request")
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let rebuild_task = {
        let http = http.clone();
        let base = base.clone();
        let id = source_two_id.clone();
        tokio::spawn(async move {
            headers_with_kind(
                http.post(format!("{base}/knowledge/documents/{id}/backlinks")),
                "mt032-rebuild-after-target-delete",
                "operator",
            )
            .send()
            .await
            .expect("rebuild after target delete request")
        })
    };
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), delete_task)
            .await
            .expect("target delete-first must complete without deadlock")
            .expect("join target delete-first")
            .status(),
        200
    );
    let rebuild_response = tokio::time::timeout(std::time::Duration::from_secs(10), rebuild_task)
        .await
        .expect("rebuild-second must complete without deadlock")
        .expect("join rebuild second");
    let rebuild_status = rebuild_response.status();
    let rebuild_body = rebuild_response
        .text()
        .await
        .expect("delete-first rebuild body text");
    if rebuild_status != 200 {
        let document = pg
            .db
            .get_knowledge_rich_document(&source_two_id)
            .await
            .expect("load delete-first source for direct diagnostic")
            .expect("delete-first source remains live");
        let tree = handshake_core::knowledge_document::block_tree::BlockTree::from_document_json(
            &document.rich_document_id,
            &document.schema_version,
            &document.content_json,
        )
        .expect("parse delete-first source for direct diagnostic");
        let refs =
            handshake_core::knowledge_document::backlink::DocumentLinkReferences::extract(&tree);
        let upserts = refs
            .references
            .iter()
            .map(
                |reference| handshake_core::storage::knowledge::UpsertKnowledgeDocumentBacklink {
                    workspace_id: document.workspace_id.clone(),
                    relationship_id: reference.relationship_id.clone(),
                    source_document_id: document.rich_document_id.clone(),
                    link_kind: reference.kind.as_str().to_string(),
                    target: reference.target.clone(),
                    block_id: reference.block_id.clone(),
                },
            )
            .collect();
        let direct_error = pg
            .db
            .replace_knowledge_document_backlinks(&source_two_id, upserts)
            .await
            .expect_err("failed HTTP rebuild must reproduce through storage");
        panic!(
            "delete-first rebuild failed: {rebuild_body}; direct storage error: {direct_error:?}"
        );
    }
    assert_eq!(
        rebuild_status, 200,
        "delete-first rebuild failed: {rebuild_body}"
    );
    let rebuild_response: Value =
        serde_json::from_str(&rebuild_body).expect("delete-first rebuild JSON body");
    assert!(rebuild_response["backlinks"]
        .as_array()
        .expect("delete-first rebuilt backlinks")
        .is_empty());
    sqlx::query("DROP TRIGGER mt032_hold_target_delete_first ON knowledge_rich_documents")
        .execute(&mut conn)
        .await
        .expect("drop target delete-first hold trigger");
    let stale_target_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_document_backlinks \
         WHERE target = $1 OR target = 'MT032 Delete-First Backlink Target'",
    )
    .bind(&target_two_id)
    .fetch_one(&mut conn)
    .await
    .expect("delete-first stale target count");
    assert_eq!(stale_target_count, 0);

    // Symmetric rebuilds must acquire the same advisory-lock set in the same
    // order. Release A->B and B->A together and bound both completions so a
    // lock-order regression fails as a deadlock instead of hanging the suite.
    let cycle_a = create_doc(&base, &http, &workspace_id, "MT032 Cycle A").await;
    let cycle_a_id = cycle_a["document"]["rich_document_id"]
        .as_str()
        .expect("cycle A id")
        .to_string();
    let cycle_b = create_doc(&base, &http, &workspace_id, "MT032 Cycle B").await;
    let cycle_b_id = cycle_b["document"]["rich_document_id"]
        .as_str()
        .expect("cycle B id")
        .to_string();
    for (id, target_title, task_id) in [
        (&cycle_a_id, "MT032 Cycle B", "mt032-seed-cycle-a"),
        (&cycle_b_id, "MT032 Cycle A", "mt032-seed-cycle-b"),
    ] {
        let saved = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{id}/save")),
            task_id,
            "operator",
        )
        .json(&json!({
            "expected_version": 1,
            "content_json": {"type": "doc", "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": format!("[[{target_title}]]")}]
            }]}
        }))
        .send()
        .await
        .expect("seed symmetric backlink document");
        assert_eq!(saved.status(), 200);
    }
    let cycle_barrier = Arc::new(tokio::sync::Barrier::new(3));
    let cycle_a_task = {
        let http = http.clone();
        let base = base.clone();
        let id = cycle_a_id.clone();
        let barrier = cycle_barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            headers_with_kind(
                http.post(format!("{base}/knowledge/documents/{id}/backlinks")),
                "mt032-cycle-a-to-b",
                "operator",
            )
            .send()
            .await
            .expect("cycle A rebuild request")
        })
    };
    let cycle_b_task = {
        let http = http.clone();
        let base = base.clone();
        let id = cycle_b_id.clone();
        let barrier = cycle_barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            headers_with_kind(
                http.post(format!("{base}/knowledge/documents/{id}/backlinks")),
                "mt032-cycle-b-to-a",
                "operator",
            )
            .send()
            .await
            .expect("cycle B rebuild request")
        })
    };
    cycle_barrier.wait().await;
    let cycle_a_response = tokio::time::timeout(std::time::Duration::from_secs(10), cycle_a_task)
        .await
        .expect("A-to-B rebuild must complete without deadlock")
        .expect("join A-to-B rebuild");
    let cycle_b_response = tokio::time::timeout(std::time::Duration::from_secs(10), cycle_b_task)
        .await
        .expect("B-to-A rebuild must complete without deadlock")
        .expect("join B-to-A rebuild");
    assert_eq!(cycle_a_response.status(), 200);
    assert_eq!(cycle_b_response.status(), 200);
    let (a_to_b, b_to_a): (i64, i64) = sqlx::query_as(
        "SELECT \
         COUNT(*) FILTER (WHERE source_document_id = $1 AND target = $2), \
         COUNT(*) FILTER (WHERE source_document_id = $2 AND target = $1) \
         FROM knowledge_document_backlinks",
    )
    .bind(&cycle_a_id)
    .bind(&cycle_b_id)
    .fetch_one(&mut conn)
    .await
    .expect("load symmetric backlink rows");
    assert_eq!((a_to_b, b_to_a), (1, 1));

    sqlx::query("DROP FUNCTION mt032_hold_mutation()")
        .execute(&mut conn)
        .await
        .expect("drop mutation hold function");
    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_migration_0343_upgrade_collision_idempotence_and_safe_down() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let mut conn = pg.raw_connection().await;
    sqlx::raw_sql(include_str!(
        "../migrations/0343_knowledge_rich_document_loom_projection.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("return isolated schema to pre-0343 state");

    let legacy_id = format!("KRD-{}", uuid::Uuid::now_v7().simple());
    let changed_id = format!("KRD-{}", uuid::Uuid::now_v7().simple());
    let preexisting_block_id = format!("KRD-{}", uuid::Uuid::now_v7().simple());
    for (id, title, body, hash_char) in [
        (
            &legacy_id,
            "MT032 Legacy Upgrade",
            "legacy searchable body",
            'a',
        ),
        (
            &changed_id,
            "MT032 Changed After Upgrade",
            "changed searchable body",
            'b',
        ),
        (
            &preexisting_block_id,
            "MT032 Preexisting Block",
            "preexisting block searchable body",
            'd',
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO knowledge_rich_documents
                (rich_document_id, workspace_id, title, schema_version,
                 content_json, content_sha256)
            VALUES ($1, $2, $3, 'hsk_richdoc_v1', $4, $5)
            "#,
        )
        .bind(id)
        .bind(&workspace_id)
        .bind(title)
        .bind(json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": body}]
            }]
        }))
        .bind(hash_char.to_string().repeat(64))
        .execute(&mut conn)
        .await
        .expect("insert pre-0343 RichDocument");
    }

    pg.db
        .create_loom_block(
            &WriteContext::system(Some("mt032-preexisting-block".to_string())),
            NewLoomBlock {
                block_id: Some(preexisting_block_id.clone()),
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("MT032 Prior Block Title".to_string()),
                original_filename: None,
                content_hash: Some("e".repeat(64)),
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("insert pre-0343 LoomBlock");
    sqlx::query("DELETE FROM loom_block_search_index WHERE block_id = $1")
        .bind(&preexisting_block_id)
        .execute(&mut conn)
        .await
        .expect("remove preexisting block search row before upgrade");

    let forward = include_str!("../migrations/0343_knowledge_rich_document_loom_projection.sql");
    sqlx::raw_sql(forward)
        .execute(&mut conn)
        .await
        .expect("run 0343 upgrade");
    let (derived, search, block_updated_at, indexed_at): (
        Value,
        String,
        chrono::NaiveDateTime,
        chrono::DateTime<chrono::Utc>,
    ) = sqlx::query_as(
        r#"
        SELECT b.derived_json::jsonb, s.search_text, b.updated_at, s.indexed_at
        FROM loom_blocks b
        JOIN loom_block_search_index s ON s.block_id = b.block_id
        WHERE b.block_id = $1 AND b.workspace_id = $2
        "#,
    )
    .bind(&legacy_id)
    .bind(&workspace_id)
    .fetch_one(&mut conn)
    .await
    .expect("upgraded body-aware projection");
    assert_eq!(derived["full_text_index"], "legacy searchable body");
    assert!(search.contains("legacy searchable body"));

    sqlx::raw_sql(forward)
        .execute(&mut conn)
        .await
        .expect("repeat 0343 forward idempotently");
    let repeated: (
        i64,
        i64,
        chrono::NaiveDateTime,
        chrono::DateTime<chrono::Utc>,
    ) = sqlx::query_as(
        r#"
            SELECT
                (SELECT COUNT(*) FROM loom_blocks WHERE block_id = $1),
                (SELECT COUNT(*) FROM loom_block_search_index WHERE block_id = $1),
                (SELECT updated_at FROM loom_blocks WHERE block_id = $1),
                (SELECT indexed_at FROM loom_block_search_index WHERE block_id = $1)
            "#,
    )
    .bind(&legacy_id)
    .fetch_one(&mut conn)
    .await
    .expect("repeated forward state");
    assert_eq!(repeated.0, 1);
    assert_eq!(repeated.1, 1);
    assert_eq!(repeated.2, block_updated_at);
    assert_eq!(repeated.3, indexed_at);

    let post_upgrade_edge_id = format!("LE-{}", uuid::Uuid::now_v7().simple());
    sqlx::query(
        r#"
        INSERT INTO loom_edges
            (edge_id, workspace_id, source_block_id, target_block_id,
             edge_type, created_by, last_actor_kind)
        VALUES ($1, $2, $3, $4, 'mention', 'user', 'HUMAN')
        "#,
    )
    .bind(&post_upgrade_edge_id)
    .bind(&workspace_id)
    .bind(&legacy_id)
    .bind(&changed_id)
    .execute(&mut conn)
    .await
    .expect("attach post-upgrade dependency to migration-created block");

    let collision_workspace = pg.create_workspace().await;
    let collision_id = format!("KRD-{}", uuid::Uuid::now_v7().simple());
    sqlx::query(
        r#"
        INSERT INTO knowledge_rich_documents
            (rich_document_id, workspace_id, title, schema_version,
             content_json, content_sha256)
        VALUES ($1, $2, 'MT032 Collision', 'hsk_richdoc_v1',
                '{"type":"doc","content":[]}', $3)
        "#,
    )
    .bind(&collision_id)
    .bind(&workspace_id)
    .bind("c".repeat(64))
    .execute(&mut conn)
    .await
    .expect("insert collision RichDocument");
    pg.db
        .create_loom_block(
            &WriteContext::system(Some("mt032-collision".to_string())),
            NewLoomBlock {
                block_id: Some(collision_id.clone()),
                workspace_id: collision_workspace,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Foreign Collision".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("insert cross-workspace collision block");
    let collision = sqlx::raw_sql(forward).execute(&mut conn).await;
    assert!(
        collision.is_err(),
        "cross-workspace identity collision must fail"
    );
    sqlx::query("DELETE FROM loom_blocks WHERE block_id = $1")
        .bind(&collision_id)
        .execute(&mut conn)
        .await
        .expect("remove collision block");
    sqlx::query("DELETE FROM knowledge_rich_documents WHERE rich_document_id = $1")
        .bind(&collision_id)
        .execute(&mut conn)
        .await
        .expect("remove collision document");

    sqlx::query("UPDATE loom_blocks SET title = 'Operator Changed' WHERE block_id = $1")
        .bind(&changed_id)
        .execute(&mut conn)
        .await
        .expect("change upgraded block before down");
    let rollback =
        include_str!("../migrations/0343_knowledge_rich_document_loom_projection.down.sql");
    let fk_consumers: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT referencing_table.relname::text, referencing_column.attname::text
        FROM pg_constraint constraint_row
        JOIN pg_class referencing_table
          ON referencing_table.oid = constraint_row.conrelid
        JOIN LATERAL unnest(constraint_row.conkey) WITH ORDINALITY
          AS source_key(attnum, ordinal) ON TRUE
        JOIN LATERAL unnest(constraint_row.confkey) WITH ORDINALITY
          AS target_key(attnum, ordinal) ON target_key.ordinal = source_key.ordinal
        JOIN pg_attribute referencing_column
          ON referencing_column.attrelid = constraint_row.conrelid
         AND referencing_column.attnum = source_key.attnum
        JOIN pg_attribute referenced_column
          ON referenced_column.attrelid = constraint_row.confrelid
         AND referenced_column.attnum = target_key.attnum
        WHERE constraint_row.contype = 'f'
          AND constraint_row.confrelid = 'loom_blocks'::regclass
          AND referenced_column.attname = 'block_id'
        ORDER BY referencing_table.relname, referencing_column.attname
        "#,
    )
    .fetch_all(&mut conn)
    .await
    .expect("catalog LoomBlock FK consumers");
    assert!(
        fk_consumers.iter().any(|(table, column)| {
            table == "atelier_intake_item_loom_projection" && column == "loom_block_id"
        }),
        "0344 Atelier projection must be present in the rollback dependency catalog proof"
    );
    for (table, column) in &fk_consumers {
        assert!(
            rollback.contains(table) && rollback.contains(column),
            "0343 rollback must explicitly account for current LoomBlock FK consumer {table}.{column}"
        );
    }
    sqlx::raw_sql(rollback)
        .execute(&mut conn)
        .await
        .expect("run safe 0343 down");
    let legacy_block_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_blocks WHERE block_id = $1")
            .bind(&legacy_id)
            .fetch_one(&mut conn)
            .await
            .expect("dependent legacy block survives down");
    let post_upgrade_edge_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_edges WHERE edge_id = $1")
            .bind(&post_upgrade_edge_id)
            .fetch_one(&mut conn)
            .await
            .expect("post-upgrade dependency survives down");
    let preexisting_search_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_block_search_index WHERE block_id = $1")
            .bind(&preexisting_block_id)
            .fetch_one(&mut conn)
            .await
            .expect("preexisting block search count after down");
    let changed_title: String =
        sqlx::query_scalar("SELECT title FROM loom_blocks WHERE block_id = $1")
            .bind(&changed_id)
            .fetch_one(&mut conn)
            .await
            .expect("operator-changed block survives down");
    assert_eq!(legacy_block_count, 1);
    assert_eq!(post_upgrade_edge_count, 1);
    assert_eq!(preexisting_search_count, 0);
    assert_eq!(changed_title, "Operator Changed");
    drop(conn);
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt046_migration_0347_backfills_links_to_any_same_workspace_loom_block() {
    let pg = knowledge_pg()
        .await
        .expect("MT-046 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let foreign_workspace_id = pg.create_workspace().await;
    let mut conn = pg.raw_connection().await;
    sqlx::raw_sql(include_str!(
        "../migrations/0347_knowledge_rich_document_loom_edges.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("return isolated schema to pre-0347 edge-projection state");

    let source_id = format!("KRD-{}", uuid::Uuid::now_v7().simple());
    sqlx::query(
        r#"
        INSERT INTO knowledge_rich_documents
            (rich_document_id, workspace_id, title, schema_version,
             content_json, content_sha256)
        VALUES ($1, $2, 'MT046 0347 legacy source', 'hsk_richdoc_v1',
                '{"type":"doc","content":[]}', $3)
        "#,
    )
    .bind(&source_id)
    .bind(&workspace_id)
    .bind("a".repeat(64))
    .execute(&mut conn)
    .await
    .expect("insert pre-0347 RichDocument source");

    let file_target = format!("BLK-{}", uuid::Uuid::now_v7().simple());
    let ckc_target = format!("BLK-{}", uuid::Uuid::now_v7().simple());
    let foreign_source = format!("BLK-{}", uuid::Uuid::now_v7().simple());
    let foreign_target = format!("BLK-{}", uuid::Uuid::now_v7().simple());
    for (block_id, workspace, content_type, title) in [
        (
            source_id.as_str(),
            workspace_id.as_str(),
            LoomBlockContentType::Note,
            "MT046 source note",
        ),
        (
            file_target.as_str(),
            workspace_id.as_str(),
            LoomBlockContentType::File,
            "MT046 code file target",
        ),
        (
            ckc_target.as_str(),
            workspace_id.as_str(),
            LoomBlockContentType::CkcCharacter,
            "MT046 CKC character target",
        ),
        (
            foreign_source.as_str(),
            foreign_workspace_id.as_str(),
            LoomBlockContentType::Note,
            "MT046 foreign edge source",
        ),
        (
            foreign_target.as_str(),
            foreign_workspace_id.as_str(),
            LoomBlockContentType::File,
            "MT046 foreign target",
        ),
    ] {
        pg.db
            .create_loom_block(
                &WriteContext::system(Some("mt046-0347-upgrade".to_string())),
                NewLoomBlock {
                    block_id: Some(block_id.to_string()),
                    workspace_id: workspace.to_string(),
                    content_type,
                    document_id: None,
                    asset_id: None,
                    title: Some(title.to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived::default(),
                },
            )
            .await
            .expect("insert migration LoomBlock fixture");
    }

    let file_relationship = format!("KDLNK-{}", "b".repeat(64));
    let ckc_relationship = format!("KDLNK-{}", "c".repeat(64));
    let foreign_relationship = format!("KDLNK-{}", "d".repeat(64));
    for (relationship_id, target, block_id) in [
        (&file_relationship, &file_target, "paragraph-file"),
        (&ckc_relationship, &ckc_target, "paragraph-ckc"),
        (&foreign_relationship, &foreign_target, "paragraph-foreign"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO knowledge_document_backlinks
                (backlink_id, workspace_id, relationship_id, source_document_id,
                 link_kind, target, block_id)
            VALUES ($1, $2, $3, $4, 'wikilink', $5, $6)
            "#,
        )
        .bind(format!("KDBL-{}", uuid::Uuid::now_v7().simple()))
        .bind(&workspace_id)
        .bind(relationship_id)
        .bind(&source_id)
        .bind(target)
        .bind(block_id)
        .execute(&mut conn)
        .await
        .expect("insert pre-0347 backlink");
    }

    // Model an independently-authored KDLNK-shaped identity in another workspace. It is not a
    // projectable backlink for the source workspace and therefore must neither block nor be overwritten.
    sqlx::query(
        r#"
        INSERT INTO loom_edges
            (edge_id, workspace_id, source_block_id, target_block_id,
             edge_type, created_by, last_actor_kind, last_actor_id)
        VALUES ($1, $2, $3, $4, 'mention', 'user', 'HUMAN', 'foreign-writer')
        "#,
    )
    .bind(&foreign_relationship)
    .bind(&foreign_workspace_id)
    .bind(&foreign_source)
    .bind(&foreign_target)
    .execute(&mut conn)
    .await
    .expect("insert unrelated cross-workspace KDLNK-shaped edge");

    let forward = include_str!("../migrations/0347_knowledge_rich_document_loom_edges.sql");
    sqlx::raw_sql(forward)
        .execute(&mut conn)
        .await
        .expect("0347 upgrades KRD links to non-KRD Loom targets");
    sqlx::raw_sql(forward)
        .execute(&mut conn)
        .await
        .expect("0347 upgrade is idempotent");

    let projected: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT edge_id, source_block_id, target_block_id, last_actor_id
        FROM loom_edges
        WHERE workspace_id = $1 AND edge_id IN ($2, $3)
        ORDER BY edge_id
        "#,
    )
    .bind(&workspace_id)
    .bind(&file_relationship)
    .bind(&ckc_relationship)
    .fetch_all(&mut conn)
    .await
    .expect("read 0347 projected edges");
    assert_eq!(projected.len(), 2, "file and CKC targets both backfill");
    for (edge_id, source, target, actor) in &projected {
        assert!(edge_id == &file_relationship || edge_id == &ckc_relationship);
        assert_eq!(source, &source_id);
        assert!(target == &file_target || target == &ckc_target);
        assert_eq!(actor, "knowledge_rich_document_backlink_projection");
    }

    let (source_mentions, file_backlinks, ckc_backlinks): (i32, i32, i32) = sqlx::query_as(
        r#"
        SELECT
            (SELECT mention_count FROM loom_blocks WHERE block_id = $1),
            (SELECT backlink_count FROM loom_blocks WHERE block_id = $2),
            (SELECT backlink_count FROM loom_blocks WHERE block_id = $3)
        "#,
    )
    .bind(&source_id)
    .bind(&file_target)
    .bind(&ckc_target)
    .fetch_one(&mut conn)
    .await
    .expect("read 0347 derived counters");
    assert_eq!((source_mentions, file_backlinks, ckc_backlinks), (2, 1, 1));

    let foreign_edge: (String, String) =
        sqlx::query_as("SELECT workspace_id, last_actor_id FROM loom_edges WHERE edge_id = $1")
            .bind(&foreign_relationship)
            .fetch_one(&mut conn)
            .await
            .expect("unrelated foreign edge survives upgrade");
    assert_eq!(foreign_edge.0, foreign_workspace_id);
    assert_eq!(foreign_edge.1, "foreign-writer");

    drop(conn);
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_delete_is_atomic_and_removes_canvas_references() {
    let pg = knowledge_pg()
        .await
        .expect("MT-032 requires Handshake-managed PostgreSQL proof");
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let document = create_doc(&base, &http, &workspace_id, "MT032 Delete Target").await;
    let document_id = document["document"]["rich_document_id"]
        .as_str()
        .expect("delete target id")
        .to_string();

    let referring_response = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-delete-referrer",
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "MT032 Delete Referrer",
        "content_json": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "[[MT032 Delete Target]]"}]
            }]
        }
    }))
    .send()
    .await
    .expect("create delete referrer");
    assert_eq!(referring_response.status(), 200);
    let referring: Value = referring_response
        .json()
        .await
        .expect("delete referrer body");
    let referring_id = referring["document"]["rich_document_id"]
        .as_str()
        .expect("delete referrer id")
        .to_string();
    let rebuilt = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referring_id}/backlinks"
        )),
        "mt032-delete-referrer-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("rebuild delete referrer backlinks");
    assert_eq!(rebuilt.status(), 200);

    let write_ctx = WriteContext::human(Some("mt032-delete-proof".to_string()));
    let canvas_block = pg
        .db
        .create_loom_block(
            &write_ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some("MT032 Delete Canvas".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create canvas LoomBlock");
    pg.db
        .bridge_loom_block_to_knowledge(&write_ctx, &workspace_id, &canvas_block.block_id)
        .await
        .expect("bridge canvas block");
    pg.db
        .create_canvas_board(
            &write_ctx,
            &workspace_id,
            &canvas_block.block_id,
            json!({
                "schema_id": "hsk.loom_canvas_board@1",
                "pan_x": 0.0,
                "pan_y": 0.0,
                "zoom": 1.0
            }),
        )
        .await
        .expect("create canvas board");
    let placement = pg
        .db
        .place_block_on_canvas(
            &write_ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_block.block_id.clone(),
                workspace_id: workspace_id.clone(),
                placed_block_id: document_id.clone(),
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place document block on canvas");

    let mut conn = pg.raw_connection().await;
    let loom_backlink_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_edges \
         WHERE workspace_id = $1 AND source_block_id = $2 \
           AND target_block_id = $3 AND edge_type = 'mention' \
           AND edge_id LIKE 'KDLNK-%'",
    )
    .bind(&workspace_id)
    .bind(&referring_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("same-id RichDocument Loom backlink projection");
    assert_eq!(
        loom_backlink_count, 1,
        "one durable knowledge wikilink projects to one same-id Loom edge"
    );
    let relationship_id: String = sqlx::query_scalar(
        "SELECT relationship_id FROM knowledge_document_backlinks \
         WHERE workspace_id = $1 AND source_document_id = $2 AND target = $3",
    )
    .bind(&workspace_id)
    .bind(&referring_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("projected backlink relationship id");
    sqlx::query(
        "UPDATE loom_edges SET last_actor_kind = 'HUMAN', last_actor_id = 'independent-edge' \
         WHERE edge_id = $1",
    )
    .bind(&relationship_id)
    .execute(&mut conn)
    .await
    .expect("convert projected edge into independently-authored collision fixture");
    sqlx::query("DELETE FROM knowledge_document_backlinks WHERE relationship_id = $1")
        .bind(&relationship_id)
        .execute(&mut conn)
        .await
        .expect("remove rebuildable backlink while retaining independent edge");
    let collision_rebuild = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referring_id}/backlinks"
        )),
        "mt032-independent-edge-collision",
        "operator",
    )
    .send()
    .await
    .expect("rebuild against independently-authored edge collision");
    assert_eq!(collision_rebuild.status(), 409);
    let independent_edge: (String, String, String) = sqlx::query_as(
        "SELECT last_actor_kind, last_actor_id, target_block_id FROM loom_edges WHERE edge_id = $1",
    )
    .bind(&relationship_id)
    .fetch_one(&mut conn)
    .await
    .expect("independently-authored edge survives collision");
    assert_eq!(
        independent_edge,
        (
            "HUMAN".to_owned(),
            "independent-edge".to_owned(),
            document_id.clone()
        ),
        "a KDLNK-shaped identity owned by another writer is never deleted or overwritten"
    );
    let rolled_back_backlinks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_document_backlinks WHERE relationship_id = $1",
    )
    .bind(&relationship_id)
    .fetch_one(&mut conn)
    .await
    .expect("collision rebuild rollback count");
    assert_eq!(rolled_back_backlinks, 0);
    sqlx::query("DELETE FROM loom_edges WHERE edge_id = $1")
        .bind(&relationship_id)
        .execute(&mut conn)
        .await
        .expect("remove independent collision fixture");
    let restored_rebuild = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referring_id}/backlinks"
        )),
        "mt032-restore-owned-edge",
        "operator",
    )
    .send()
    .await
    .expect("restore owned backlink projection");
    assert_eq!(restored_rebuild.status(), 200);
    let source_stale_before: bool = sqlx::query_scalar(
        "SELECT stale FROM knowledge_sources \
         WHERE workspace_id = $1 \
           AND source_kind = 'rich_document' \
           AND provenance->>'rich_document_id' = $2",
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("delete target knowledge source");
    assert!(!source_stale_before);
    let failure_function_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION mt032_reject_target_block_delete()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF OLD.block_id = '{}' THEN
                RAISE EXCEPTION 'MT032 injected projection delete failure';
            END IF;
            RETURN OLD;
        END
        $$
        "#,
        document_id.replace('\'', "''")
    );
    sqlx::query(&failure_function_sql)
        .execute(&mut conn)
        .await
        .expect("create delete failure function");
    sqlx::query(
        "CREATE TRIGGER mt032_reject_target_block_delete \
         BEFORE DELETE ON loom_blocks FOR EACH ROW \
         EXECUTE FUNCTION mt032_reject_target_block_delete()",
    )
    .execute(&mut conn)
    .await
    .expect("create delete failure trigger");
    let failed_delete = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-delete-fail",
        "operator",
    )
    .send()
    .await
    .expect("forced failed delete");
    assert_eq!(failed_delete.status(), 500);
    let deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("tombstone rollback state");
    assert!(
        deleted_at.is_none(),
        "failed projection delete rolls tombstone back"
    );
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'knowledge_rich_document' \
           AND aggregate_id = $1 \
           AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED'",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("rolled-back delete receipt count");
    assert_eq!(receipt_count, 0, "failed delete rolls receipt back");
    let placement_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_canvas_placements WHERE placement_id = $1")
            .bind(&placement.placement_id)
            .fetch_one(&mut conn)
            .await
            .expect("placement rollback count");
    assert_eq!(placement_count, 1, "failed delete keeps canvas placement");
    let backlink_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_document_backlinks WHERE target = $1")
            .bind(&document_id)
            .fetch_one(&mut conn)
            .await
            .expect("backlink rollback count");
    assert_eq!(backlink_count, 1, "failed delete keeps backlinks");
    let loom_backlink_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_edges \
         WHERE workspace_id = $1 AND source_block_id = $2 AND target_block_id = $3",
    )
    .bind(&workspace_id)
    .bind(&referring_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("loom backlink rollback count");
    assert_eq!(
        loom_backlink_count, 1,
        "failed delete keeps the same-id Loom backlink projection"
    );
    let source_stale_after_failure: bool = sqlx::query_scalar(
        "SELECT stale FROM knowledge_sources \
         WHERE workspace_id = $1 \
           AND source_kind = 'rich_document' \
           AND provenance->>'rich_document_id' = $2",
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("source stale rollback state");
    assert!(
        !source_stale_after_failure,
        "failed delete keeps source live"
    );

    sqlx::query("DROP TRIGGER mt032_reject_target_block_delete ON loom_blocks")
        .execute(&mut conn)
        .await
        .expect("drop delete failure trigger");
    sqlx::query("DROP FUNCTION mt032_reject_target_block_delete()")
        .execute(&mut conn)
        .await
        .expect("drop delete failure function");

    let deleted = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-delete-success",
        "operator",
    )
    .send()
    .await
    .expect("delete with placement cleanup");
    assert_eq!(deleted.status(), 200);
    let deleted: Value = deleted.json().await.expect("delete response");
    assert_eq!(deleted["loom_block_deleted"], true);
    assert_eq!(deleted["source_marked_stale"], true);
    let deleted_receipt_event_id = deleted["deleted_receipt_event_id"]
        .as_str()
        .expect("delete receipt id")
        .to_string();

    let load_deleted = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-load-deleted",
        "operator",
    )
    .send()
    .await
    .expect("load deleted document");
    assert_eq!(load_deleted.status(), 404);
    let block_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_blocks WHERE block_id = $1")
            .bind(&document_id)
            .fetch_one(&mut conn)
            .await
            .expect("deleted block count");
    assert_eq!(block_count, 0);
    let placement_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM loom_canvas_placements WHERE placement_id = $1")
            .bind(&placement.placement_id)
            .fetch_one(&mut conn)
            .await
            .expect("deleted placement count");
    assert_eq!(placement_count, 0);
    let (deleted_at, persisted_receipt): (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT deleted_at, deleted_receipt_event_id FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("successful tombstone state");
    assert!(deleted_at.is_some());
    assert_eq!(
        persisted_receipt.as_deref(),
        Some(deleted_receipt_event_id.as_str())
    );
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'knowledge_rich_document' \
           AND aggregate_id = $1 \
           AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED'",
    )
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("successful delete receipt count");
    assert_eq!(receipt_count, 1);
    let backlink_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_document_backlinks \
         WHERE source_document_id = $1 OR target = $1 OR target = $2",
    )
    .bind(&document_id)
    .bind("MT032 Delete Target")
    .fetch_one(&mut conn)
    .await
    .expect("successful backlink cleanup count");
    assert_eq!(backlink_count, 0);
    let loom_backlink_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_edges \
         WHERE workspace_id = $1 AND (source_block_id = $2 OR target_block_id = $2)",
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("successful Loom backlink cleanup count");
    assert_eq!(loom_backlink_count, 0);
    let source_stale_after_success: bool = sqlx::query_scalar(
        "SELECT stale FROM knowledge_sources \
         WHERE workspace_id = $1 \
           AND source_kind = 'rich_document' \
           AND provenance->>'rich_document_id' = $2",
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .fetch_one(&mut conn)
    .await
    .expect("source stale success state");
    assert!(source_stale_after_success);
    drop(conn);
    server.shutdown().await;
    pg.teardown().await;
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
    let (base, http, server) = doc_server(&pg).await;

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
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_cloud_model_cannot_write_and_bogus_kind_is_rejected() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt158_cloud_model...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
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
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-151 adversarial-v2: import -> load -> save -> export round-trips.
// ---------------------------------------------------------------------------

/// Drive one full import -> load -> save -> export cycle through the real
/// routes and return (document_id, loaded body). Before the ImportedRaw
/// hardening, the LOAD step 400'd for any imported HTML/table document.
async fn import_roundtrip(
    base: &str,
    http: &reqwest::Client,
    workspace_id: &str,
    label: &str,
    format: &str,
    snippet: &str,
) -> (String, Value) {
    // IMPORT.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/import")),
        label,
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": format!("Imported {label}"),
        "format": format,
        "snippet": snippet,
    }))
    .send()
    .await
    .expect("import send");
    assert_eq!(resp.status(), 200, "import must succeed");
    let imported: Value = resp.json().await.expect("import json");
    let doc_id = imported["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // LOAD (typed block tree) — the adversarial-v2 finding: this was a 400.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        label,
        "operator",
    )
    .send()
    .await
    .expect("load send");
    assert_eq!(
        resp.status(),
        200,
        "imported {format} document must LOAD through the typed API"
    );
    let loaded: Value = resp.json().await.expect("load json");
    assert_eq!(loaded["tree"]["schema_matches"], true);

    // BLOCKS endpoint loads too.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/blocks")),
        label,
        "operator",
    )
    .send()
    .await
    .expect("blocks send");
    assert_eq!(resp.status(), 200, "blocks endpoint must load");

    // SAVE the loaded content back (v1 -> v2): the round-trip must validate.
    let content = loaded["document"]["content_json"].clone();
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        label,
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": content}))
    .send()
    .await
    .expect("save send");
    assert_eq!(resp.status(), 200, "imported document must SAVE");
    let saved: Value = resp.json().await.expect("save json");
    assert_eq!(saved["document"]["doc_version"], 2);
    assert_eq!(
        saved["document"]["content_json"], loaded["document"]["content_json"],
        "save round-trip is lossless"
    );

    // EXPORT projections (markdown + html) — render, never 400.
    for proj in ["markdown", "html"] {
        let resp = headers_with_kind(
            http.get(format!(
                "{base}/knowledge/documents/{doc_id}/projection?format={proj}"
            )),
            label,
            "operator",
        )
        .send()
        .await
        .expect("projection send");
        assert_eq!(
            resp.status(),
            200,
            "imported {format} document must EXPORT as {proj}"
        );
    }
    (doc_id, loaded)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_imported_html_document_roundtrips_load_save_export() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt151_imported_html...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    let html = "<h1>Doc</h1><table><tr><td>cell</td></tr></table>";
    let (doc_id, loaded) =
        import_roundtrip(&base, &http, &workspace_id, "html", "html", html).await;

    // The importedRaw block is present in the typed tree with its source.
    let blocks = loaded["tree"]["blocks"].as_array().expect("blocks");
    assert!(
        blocks.iter().any(|b| b["kind"] == "imported_raw"),
        "typed tree exposes the imported_raw block: {blocks:?}"
    );

    // The markdown export carries the captured source INERT (fenced).
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/projection?format=markdown"
        )),
        "html-md",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    let content = body["projection"]["content"].as_str().expect("content");
    assert!(
        content.contains("```html") && content.contains("<table>"),
        "markdown export fences the imported source: {content}"
    );
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-152 adversarial-v2: the save path validates + persists content embeds.
// ---------------------------------------------------------------------------

fn doc_with_embed(workspace_id: &str, title: &str, target: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "intro" }] },
                { "type": "image", "attrs": { "target": target },
                  "content": [{ "type": "text", "text": "diagram" }] }
            ]
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt152_save_path_validates_and_persists_content_embeds() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt152_save_path...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    // CREATE with a valid typed embed target -> the side table is synced.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "embed-create",
        "operator",
    )
    .json(&doc_with_embed(&workspace_id, "Embeds", "KMED-ok"))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let created: Value = resp.json().await.expect("json");
    assert_eq!(
        created["embeds_persisted"], 1,
        "create syncs the embed table"
    );
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    let list_embeds = |label: &'static str| {
        headers_with_kind(
            http.get(format!("{base}/knowledge/documents/{doc_id}/embeds")),
            label,
            "operator",
        )
        .send()
    };
    let body: Value = list_embeds("e1")
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    let embeds = body["embeds"].as_array().expect("embeds");
    assert_eq!(embeds.len(), 1);
    assert_eq!(embeds[0]["ref_value"], "KMED-ok");
    assert_eq!(embeds[0]["ref_kind"], "media");

    // SAVE v2 with two embeds (media id + https url) -> table resyncs to 2.
    let v2 = json!({
        "type": "doc",
        "content": [
            { "type": "image", "attrs": { "target": "KMED-ok" },
              "content": [{ "type": "text", "text": "diagram" }] },
            { "type": "video", "attrs": { "src": "https://cdn.example/clip.mp4" },
              "content": [{ "type": "text", "text": "clip" }] }
        ]
    });
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "embed-save2",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": v2}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let saved: Value = resp.json().await.expect("json");
    assert_eq!(saved["embeds_persisted"], 2);
    let body: Value = list_embeds("e2")
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    let embeds = body["embeds"].as_array().expect("embeds");
    assert_eq!(embeds.len(), 2);
    assert!(embeds
        .iter()
        .any(|e| e["ref_kind"] == "url" && e["ref_value"] == "https://cdn.example/clip.mp4"));

    // SAVE with a dangerous embed target -> 400 BEFORE commit; version stays 2.
    for bad in [
        "javascript:alert(1)",
        "JaVa\tScRiPt:alert(1)",
        "data:text/html,<script>",
        "C:\\secrets\\x.png",
        "/etc/passwd",
        "file:///etc/passwd",
    ] {
        let v3 = json!({
            "type": "doc",
            "content": [
                { "type": "image", "attrs": { "target": bad },
                  "content": [{ "type": "text", "text": "evil" }] }
            ]
        });
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "embed-bad",
            "operator",
        )
        .json(&json!({"expected_version": 2, "content_json": v3}))
        .send()
        .await
        .expect("send");
        assert_eq!(
            resp.status(),
            400,
            "embed target `{bad}` must reject the save fail-closed"
        );
    }
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "embed-check",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(
        body["document"]["doc_version"], 2,
        "rejected saves never committed"
    );

    // SAVE v3 with NO embeds -> the side table empties (true sync, no drift).
    let v3 = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "no embeds left" }] }
        ]
    });
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "embed-save3",
        "operator",
    )
    .json(&json!({"expected_version": 2, "content_json": v3}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = list_embeds("e3")
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["embeds"].as_array().expect("embeds").len(), 0);
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt246_save_rejects_cross_document_crdt_id() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt246_save_rejects_cross_document_crdt_id: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "CRDT Boundary").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();
    let expected_crdt_id = doc_id.replacen("KRD-", "KCRDT-", 1);

    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "crdt-bad-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": []},
        "crdt_document_id": "KCRDT-ffffffffffffffffffffffffffffffff"
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        400,
        "save must reject a CRDT id that does not belong to this rich document"
    );

    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "crdt-good-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": []},
        "crdt_document_id": expected_crdt_id
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200, "canonical CRDT id should save");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["crdt_document_id"], expected_crdt_id);
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-156 adversarial-v2: history is paginated and omits version bodies.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_history_is_paginated_and_omits_version_bodies() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt156_history...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "History").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // Build 5 versions (v1 from create + 4 saves with distinct bodies).
    for v in 1..=4i64 {
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "hist-save",
            "operator",
        )
        .json(&json!({
            "expected_version": v,
            "content_json": {"type": "doc", "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": format!("body v{}", v + 1) }] }
            ]}
        }))
        .send()
        .await
        .expect("send");
        assert_eq!(resp.status(), 200, "save v{} must succeed", v + 1);
    }

    // Paginated page: limit=2 offset=1 -> versions 2 and 3, metadata ONLY.
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/history?limit=2&offset=1"
        )),
        "hist-page",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["total_versions"], 5);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 1);
    let versions = body["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["doc_version"], 2);
    assert_eq!(versions[1]["doc_version"], 3);
    for version in versions {
        assert!(
            version.get("content_json").is_none(),
            "history list must omit version bodies: {version}"
        );
        assert!(version["content_sha256"].is_string());
    }

    // The limit is capped server-side: a huge requested limit clamps to 200.
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/history?limit=100000"
        )),
        "hist-cap",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["limit"], 200, "requested limit must clamp to the cap");
    assert_eq!(body["versions"].as_array().expect("versions").len(), 5);

    // Lazy single-version body load: GET history/3 returns the v3 content.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/history/3")),
        "hist-one",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["version"]["doc_version"], 3);
    assert_eq!(
        body["version"]["content_json"]["content"][0]["content"][0]["text"],
        "body v3"
    );

    // A missing version is a 404, not an empty 200.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/history/99")),
        "hist-missing",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 404);
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-154 adversarial-v2: documents are indexed into the Project Knowledge
// Index (source row + title entity + staleness on change).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_save_indexes_document_into_project_knowledge_index() {
    use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};

    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt154_save_indexes...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    // CREATE indexes the document: a rich_document SOURCE row + title ENTITY.
    let created = create_doc(&base, &http, &workspace_id, "Indexed Doc").await;
    assert_eq!(created["knowledge_indexed"], true, "{created}");
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();
    let doc_sha = created["document"]["content_sha256"]
        .as_str()
        .expect("sha")
        .to_string();

    let source = pg
        .db
        .get_knowledge_source_by_document_id(&workspace_id, &doc_id)
        .await
        .expect("source lookup")
        .expect("document source row exists in the Project Knowledge Index");
    assert_eq!(source.content_hash, doc_sha);
    assert!(!source.stale, "freshly indexed source is not stale");
    let entity = pg
        .db
        .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::RichDocument, &doc_id)
        .await
        .expect("entity lookup")
        .expect("document title entity exists in the Project Knowledge Index");
    assert_eq!(entity.display_name, "Indexed Doc");

    // SAVE with changed content marks the source STALE (the truthful index
    // state: the indexed bytes no longer match the document).
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "index-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "changed body" }] }
        ]}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["knowledge_indexed"], true);
    let source = pg
        .db
        .get_knowledge_source_by_document_id(&workspace_id, &doc_id)
        .await
        .expect("source lookup")
        .expect("source row persists");
    assert!(
        source.stale,
        "a content change marks the document source stale for re-indexing"
    );

    // RENAME refreshes the indexed title entity.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/{doc_id}/rename")),
        "index-rename",
        "operator",
    )
    .json(&json!({"title": "Indexed Doc Renamed"}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let entity = pg
        .db
        .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::RichDocument, &doc_id)
        .await
        .expect("entity lookup")
        .expect("entity persists");
    assert_eq!(
        entity.display_name, "Indexed Doc Renamed",
        "rename refreshes the indexed title"
    );

    // The indexed document is now a CONFIRMABLE authoritative handle for the
    // retrieval planner (ties MT-154 into the MT-130 existence checks).
    assert_eq!(entity.entity_kind, KnowledgeEntityKind::RichDocument);
    assert_eq!(entity.entity_key, doc_id);
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-157 adversarial-v2: move absent != null; batch with per-item reporting.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_move_empty_body_preserves_membership_and_batch_reports_per_item() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt157_move...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    // A document WITH project + folder membership.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "move-setup",
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "Membership",
        "project_ref": "PRJ-alpha",
        "folder_ref": "runbooks",
        "content_json": {"type": "doc", "content": []}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let created: Value = resp.json().await.expect("json");
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();
    assert_eq!(created["document"]["project_ref"], "PRJ-alpha");

    let do_move = |label: &'static str, body: Value| {
        headers_with_kind(
            http.post(format!("{base}/knowledge/documents/{doc_id}/move")),
            label,
            "operator",
        )
        .json(&body)
        .send()
    };

    // EMPTY body: a no-op move — membership is PRESERVED (the review found it
    // silently cleared both refs).
    let resp = do_move("empty", json!({})).await.expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(
        body["document"]["project_ref"], "PRJ-alpha",
        "empty move body must not clear project membership"
    );
    assert_eq!(body["document"]["folder_ref"], "runbooks");

    // Explicit null clears ONLY the named field.
    let resp = do_move("clear-folder", json!({"folder_ref": null}))
        .await
        .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["project_ref"], "PRJ-alpha");
    assert!(body["document"]["folder_ref"].is_null());

    // A value sets only the named field; the absent one stays.
    let resp = do_move("set-project", json!({"project_ref": "PRJ-beta"}))
        .await
        .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["project_ref"], "PRJ-beta");
    assert!(body["document"]["folder_ref"].is_null());

    // BATCH: rename (ok) + move on a ghost doc (not_found) + bad label
    // (validation) -> 200 with per-item outcomes + per-item receipt on the
    // success; one failure never aborts the batch.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/batch")),
        "batch",
        "operator",
    )
    .json(&json!({"operations": [
        {"op": "rename", "document_id": doc_id, "title": "Membership v2"},
        {"op": "move", "document_id": "KRD-00000000000000000000000000000000", "project_ref": "PRJ-x"},
        {"op": "set_authority_label", "document_id": doc_id, "authority_label": "published"}
    ]}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200, "partial failure is per-item, not a 4xx");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["succeeded"], 1);
    assert_eq!(body["failed"], 2);
    let results = body["results"].as_array().expect("results");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["ok"], true);
    assert!(results[0]["save_receipt_event_id"].is_string());
    assert_eq!(results[1]["ok"], false);
    assert_eq!(results[1]["error"], "not_found");
    assert_eq!(results[2]["ok"], false);
    assert_eq!(results[2]["error"], "validation");

    // The successful rename landed; the failed label change did not.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "batch-check",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["title"], "Membership v2");
    assert_eq!(body["document"]["authority_label"], "promoted");

    // A cloud model cannot batch-write (the MT-158 boundary covers batch too).
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/batch")),
        "batch-cloud",
        "cloud_model",
    )
    .json(&json!({"operations": [
        {"op": "rename", "document_id": doc_id, "title": "Hijack"}
    ]}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403);
    server.shutdown().await;
    pg.teardown().await;
}

// ---------------------------------------------------------------------------
// MT-149 adversarial-v2: a committed save never returns an error.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_committed_save_never_errors_when_post_commit_steps_fail() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt149_committed_save...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "Atomicity").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // Break EVERY post-commit step for real: drop the backlink + embed side
    // tables and the EventLedger table in the isolated schema. The save's own
    // tables stay intact, so the save itself can still commit.
    {
        let mut conn = pg.raw_connection().await;
        for table in [
            "knowledge_document_backlinks",
            "knowledge_document_embeds",
            "kernel_event_ledger",
        ] {
            sqlx::query(&format!("DROP TABLE {table} CASCADE"))
                .execute(&mut conn)
                .await
                .unwrap_or_else(|err| panic!("drop {table}: {err}"));
        }
    }

    // The save must COMMIT and return 200 with every failure RECORDED.
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "atomic-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "v2 body" }] },
            { "type": "image", "attrs": { "target": "KMED-1" } }
        ]}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        200,
        "a committed save must NEVER surface a post-commit step failure as an error"
    );
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["doc_version"], 2, "the save committed");
    assert!(
        body["save_receipt_event_id"].is_null(),
        "no receipt could be written"
    );
    assert!(
        body["receipt_error"].is_string(),
        "the receipt failure is recorded: {body}"
    );
    assert!(
        body["backlinks_error"].is_string(),
        "the backlink index failure is recorded: {body}"
    );
    assert!(
        body["embeds_error"].is_string(),
        "the embed sync failure is recorded: {body}"
    );

    // The committed write is durable and loadable.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "atomic-load",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let loaded: Value = resp.json().await.expect("json");
    assert_eq!(loaded["document"]["doc_version"], 2);
    assert_eq!(
        loaded["document"]["content_json"]["content"][0]["content"][0]["text"],
        "v2 body"
    );
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_imported_markdown_table_document_roundtrips_load_save_export() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt151_imported_markdown_table...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    let md = "# Title\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\ntail paragraph";
    let (_doc_id, loaded) =
        import_roundtrip(&base, &http, &workspace_id, "mdtable", "markdown", md).await;

    let blocks = loaded["tree"]["blocks"].as_array().expect("blocks");
    assert!(blocks.iter().any(|b| b["kind"] == "imported_raw"));
    assert!(blocks.iter().any(|b| b["kind"] == "heading"));
    assert!(blocks.iter().any(|b| b["kind"] == "paragraph"));
    server.shutdown().await;
    pg.teardown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt255_backend_draft_recovery_roundtrips_and_clears_on_save_or_discard() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt255_backend_draft_recovery...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http, server) = doc_server(&pg).await;

    let created = create_doc(&base, &http, &workspace_id, "Draft Recovery").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();
    let base_hash = created["document"]["content_sha256"]
        .as_str()
        .expect("base hash")
        .to_string();
    let recovered_content = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "crash sentinel draft" }] }
        ]
    });

    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-save",
        "operator",
    )
    .json(&json!({
        "base_doc_version": 1,
        "base_content_sha256": base_hash,
        "content_json": recovered_content,
    }))
    .send()
    .await
    .expect("draft upsert send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("draft upsert json");
    assert_eq!(body["cleared"], false);
    assert!(
        body["draft_receipt_event_id"].is_string(),
        "draft write must leave an EventLedger receipt: {body}"
    );

    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-load",
        "operator",
    )
    .send()
    .await
    .expect("draft load send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("draft load json");
    assert_eq!(
        body["draft"]["draft_content_json"]["content"][0]["content"][0]["text"],
        "crash sentinel draft"
    );

    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "draft-clean-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": recovered_content,
    }))
    .send()
    .await
    .expect("clean save send");
    assert_eq!(resp.status(), 200);
    let saved: Value = resp.json().await.expect("clean save json");
    assert_eq!(saved["document"]["doc_version"], 2);

    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-load-after-save",
        "operator",
    )
    .send()
    .await
    .expect("draft load after save send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("draft load after save json");
    assert!(
        body["draft"].is_null(),
        "clean save must clear draft: {body}"
    );

    let discard_content = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "discard me" }] }
        ]
    });
    let saved_hash = saved["document"]["content_sha256"]
        .as_str()
        .expect("saved hash")
        .to_string();
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-discard-save",
        "operator",
    )
    .json(&json!({
        "base_doc_version": 2,
        "base_content_sha256": saved_hash,
        "content_json": discard_content,
    }))
    .send()
    .await
    .expect("discard draft upsert send");
    assert_eq!(resp.status(), 200);

    let resp = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-discard",
        "operator",
    )
    .send()
    .await
    .expect("draft discard send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("draft discard json");
    assert_eq!(body["cleared"], true);
    assert!(
        body["clear_receipt_event_id"].is_string(),
        "explicit discard must leave an EventLedger receipt: {body}"
    );

    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "load-after-discard",
        "operator",
    )
    .send()
    .await
    .expect("load after discard send");
    assert_eq!(resp.status(), 200);
    let loaded: Value = resp.json().await.expect("load after discard json");
    assert_eq!(
        loaded["document"]["content_json"]["content"][0]["content"][0]["text"],
        "crash sentinel draft",
        "discarding a recovery draft must leave the saved head untouched"
    );

    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/draft")),
        "draft-load-after-discard",
        "operator",
    )
    .send()
    .await
    .expect("draft load after discard send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("draft load after discard json");
    assert!(body["draft"].is_null(), "discard must remove draft: {body}");
    server.shutdown().await;
    pg.teardown().await;
}
