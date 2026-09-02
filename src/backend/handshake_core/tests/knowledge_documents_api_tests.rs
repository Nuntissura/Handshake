//! WP-KERNEL-009 RichDocumentCore route-level integration tests against the
//! real isolated embedded Handshake store — adversarial-v2 hardening proofs.
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
//!   * MT-149: committed saves remain successful under independent receipt,
//!     backlink, and embed post-commit failures.
//!   * MT-152: content_json embed blocks are validated + persisted on the save
//!     path with the same EmbedTarget law as the side table.
//!   * MT-156: history is paginated and version bodies are omitted from the
//!     list response (single-version lazy body load).
//!   * MT-157: a move with an empty body does NOT clear project/folder
//!     membership (absent != explicit null).

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
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
use handshake_core::storage::surreal::RowFilter;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    NewLoomBlock, NewLoomCanvasPlacement, NewLoomEdge, WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
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

/// Boot the real document routes over loopback against the isolated store.
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

async fn test_state(store: &EmbeddedKnowledgeStore) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("docs-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
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

async fn doc_server(store: &EmbeddedKnowledgeStore) -> (String, reqwest::Client, DocServerGuard) {
    route_server(docs_api::routes(test_state(store).await)).await
}

async fn loom_server(store: &EmbeddedKnowledgeStore) -> (String, reqwest::Client, DocServerGuard) {
    route_server(handshake_core::api::loom::routes(test_state(store).await)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_schema_guard_drop_cleans_during_unwind_without_explicit_teardown() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires an isolated embedded store");
    let data_dir = store.data_dir.clone();
    let held_storage = store.storage.clone();
    let unwind = tokio::time::timeout(
        Duration::from_secs(20),
        tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                // Reverse drop order intentionally drops the fixture while a
                // cloned store handle is still outstanding. The fixture guard
                // must close the shared embedded authority and remove its
                // isolated directory while unwinding.
                let _held_storage = held_storage;
                let _store = store;
                panic!("intentional MT-032 embedded-store unwind");
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
    assert!(
        !data_dir.exists(),
        "fixture Drop must remove the isolated embedded-store directory"
    );
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

async fn embedded_table_count(store: &EmbeddedKnowledgeStore, table_name: &str) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .expect("select embedded table");
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .expect("count embedded rows")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explorer_document_rename_rejects_stale_token_without_overwrite() {
    let store = open_embedded_store()
        .await
        .expect("isolated embedded store is required for stale-rename proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explorer_bookmark_rename_returns_409_and_preserves_first_writer() {
    let store = open_embedded_store()
        .await
        .expect("isolated embedded store is required for bookmark stale-rename proof");
    let workspace_id = store.create_workspace().await;
    let created = store
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
    let (base, http, server) = loom_server(&store).await;
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_create_if_title_absent_returns_one_canonical_document() {
    let store = open_embedded_store()
        .await
        .expect("isolated embedded store is required for concurrent-create proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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

    let documents = store
        .db
        .list_knowledge_rich_documents(&workspace_id, None, None)
        .await
        .expect("canonical title list");
    let count = documents
        .iter()
        .filter(|document| {
            document
                .title
                .trim()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .eq_ignore_ascii_case("concurrent design note")
        })
        .count();
    assert_eq!(
        count, 1,
        "concurrent create leaves one canonical authority row"
    );
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_if_title_absent_rejects_preexisting_normalized_title_ambiguity() {
    let store = open_embedded_store()
        .await
        .expect("isolated embedded store is required for ambiguity proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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

    let documents = store
        .db
        .list_knowledge_rich_documents(&workspace_id, None, None)
        .await
        .expect("ambiguous title list");
    let count = documents
        .iter()
        .filter(|document| {
            document
                .title
                .trim()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .eq_ignore_ascii_case("ambiguous design note")
        })
        .count();
    assert_eq!(count, 2, "failed special create must not add a third row");
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// WP-KERNEL-012 MT-032: RichDocument <-> LoomBlock atomic projection and
// target-inbound backlink route.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_rich_documents_are_addressable_and_target_backlinks_are_inbound() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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

    let block = store
        .db
        .get_loom_block(&workspace_id, &b_id)
        .await
        .expect("same-id LoomBlock projection");
    assert_eq!(block.block_id, b_id);
    assert_eq!(block.content_hash.as_deref(), Some(document_hash.as_str()));

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
    let projected_block = store
        .db
        .get_loom_block(&workspace_id, &b_id)
        .await
        .expect("renamed LoomBlock");
    assert_eq!(
        projected_block.title.as_deref(),
        Some("MT032 Target B Renamed")
    );

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
    let imported_block = store
        .db
        .get_loom_block(&workspace_id, imported_id)
        .await
        .expect("one same-id imported block");
    assert_eq!(imported_block.block_id, imported_id);
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
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test]
async fn mt032_loom_projection_failure_rolls_back_document_create() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let tables = [
        "knowledge_rich_documents",
        "knowledge_rich_document_versions",
        "loom_blocks",
        "loom_block_search_index",
        "knowledge_document_backlinks",
    ];
    let mut baseline = Vec::new();
    for table in tables {
        baseline.push((table, embedded_table_count(&store, table).await));
    }
    store
        .storage
        .test_set_rich_document_projection_failpoint(true)
        .await
        .expect("arm RichDocument projection failpoint");
    let failed = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "mt032-create-rollback",
        "operator",
    )
    .json(&doc_body(&workspace_id, "MT032 projection rollback"))
    .send()
    .await
    .expect("send failing create");
    assert_eq!(failed.status(), 500);
    store
        .storage
        .test_set_rich_document_projection_failpoint(false)
        .await
        .expect("reset RichDocument projection failpoint");
    for (table, expected) in baseline {
        assert_eq!(
            embedded_table_count(&store, table).await,
            expected,
            "{table} must not retain a partial create"
        );
    }
    let retried = create_doc(&base, &http, &workspace_id, "MT032 projection rollback").await;
    let document_id = retried["document"]["rich_document_id"]
        .as_str()
        .expect("retried document id");
    assert!(store
        .db
        .get_loom_block(&workspace_id, document_id)
        .await
        .is_ok());
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test]
async fn mt032_save_rejects_same_workspace_wrong_type_projection_collision() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 Loom collision").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("collision document id")
        .to_owned();
    store
        .storage
        .test_set_rich_document_loom_identity(&document_id, LoomBlockContentType::File)
        .await
        .expect("seed wrong-type same-id LoomBlock");
    let content = json!({"type": "doc", "content": [{
        "type": "paragraph", "content": [{"type": "text", "text": "must roll back"}]
    }]});
    let failed = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt032-wrong-loom-type",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": content.clone()}))
    .send()
    .await
    .expect("send wrong-type save");
    assert_eq!(failed.status(), 409);
    let retained = store
        .db
        .get_knowledge_rich_document(&document_id)
        .await
        .expect("read retained document")
        .expect("document remains live");
    assert_eq!(retained.doc_version, 1);
    assert_eq!(
        store
            .db
            .count_knowledge_rich_document_versions(&document_id)
            .await
            .expect("count retained versions"),
        1
    );
    store
        .storage
        .test_set_rich_document_loom_identity(&document_id, LoomBlockContentType::Note)
        .await
        .expect("restore LoomBlock identity");
    let retry = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt032-restored-loom-type",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": content}))
    .send()
    .await
    .expect("retry save after identity restore");
    assert_eq!(retry.status(), 200);
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test]
async fn mt032_save_and_rename_reject_search_projection_identity_collisions() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let foreign_workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 search collision").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("search collision document id")
        .to_owned();
    let content = json!({"type": "doc", "content": [{
        "type": "paragraph", "content": [{"type": "text", "text": "collision save"}]
    }]});

    for (collision_workspace, collision_type) in [
        (workspace_id.as_str(), LoomBlockContentType::File),
        (foreign_workspace_id.as_str(), LoomBlockContentType::Note),
    ] {
        store
            .storage
            .test_set_rich_document_search_identity(
                &document_id,
                collision_workspace,
                collision_type,
            )
            .await
            .expect("seed search identity collision");
        let save = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{document_id}/save")),
            "mt032-search-collision-save",
            "operator",
        )
        .json(&json!({"expected_version": 1, "content_json": content.clone()}))
        .send()
        .await
        .expect("send collision save");
        assert_eq!(save.status(), 409);
        let rename = headers_with_kind(
            http.post(format!("{base}/knowledge/documents/{document_id}/rename")),
            "mt032-search-collision-rename",
            "operator",
        )
        .json(&json!({"title": "must not persist"}))
        .send()
        .await
        .expect("send collision rename");
        assert_eq!(rename.status(), 409);
        let retained = store
            .db
            .get_knowledge_rich_document(&document_id)
            .await
            .expect("read retained collision document")
            .expect("collision document remains live");
        assert_eq!(retained.doc_version, 1);
        assert_eq!(retained.title, "MT032 search collision");
        store
            .storage
            .test_set_rich_document_search_identity(
                &document_id,
                &workspace_id,
                LoomBlockContentType::Note,
            )
            .await
            .expect("restore search identity");
    }
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test]
async fn mt032_delete_rejects_search_projection_identity_collisions_atomically() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let foreign_workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let created = create_doc(&base, &http, &workspace_id, "MT032 delete collision").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("delete collision document id")
        .to_owned();
    for (collision_workspace, collision_type) in [
        (workspace_id.as_str(), LoomBlockContentType::File),
        (foreign_workspace_id.as_str(), LoomBlockContentType::Note),
    ] {
        store
            .storage
            .test_set_rich_document_search_identity(
                &document_id,
                collision_workspace,
                collision_type,
            )
            .await
            .expect("seed delete search collision");
        let failed = headers_with_kind(
            http.delete(format!("{base}/knowledge/documents/{document_id}")),
            "mt032-delete-search-collision",
            "operator",
        )
        .send()
        .await
        .expect("send collision delete");
        assert_eq!(failed.status(), 409);
        assert!(store
            .db
            .get_knowledge_rich_document(&document_id)
            .await
            .expect("read retained delete collision document")
            .is_some());
        assert!(store
            .db
            .get_loom_block(&workspace_id, &document_id)
            .await
            .is_ok());
        let delete_receipts = store
            .db
            .list_kernel_events_for_aggregate("knowledge_rich_document", &document_id)
            .await
            .expect("read collision delete receipts")
            .into_iter()
            .filter(|event| event.event_type.as_str() == "KNOWLEDGE_RICH_DOCUMENT_DELETED")
            .count();
        assert_eq!(delete_receipts, 0);
        store
            .storage
            .test_set_rich_document_search_identity(
                &document_id,
                &workspace_id,
                LoomBlockContentType::Note,
            )
            .await
            .expect("restore delete search identity");
    }
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_idempotent_save_projects_body_once_and_replays_without_writes() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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
    let first = store
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

    let replay = store
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

    let block = store
        .db
        .get_loom_block(&workspace_id, &document_id)
        .await
        .expect("idempotent Loom projection");
    assert_eq!(
        block.content_hash.as_deref(),
        Some(first.value.content_sha256.as_str())
    );
    assert_eq!(
        block.derived.full_text_index.as_deref(),
        Some("idempotent projected body")
    );
    let version_count = store
        .db
        .count_knowledge_rich_document_versions(&document_id)
        .await
        .expect("idempotent version count");
    assert_eq!(version_count, 2);
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test]
async fn mt032_markdown_import_bridge_failure_rolls_back_and_retry_creates_one() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let tables = [
        "knowledge_rich_documents",
        "knowledge_rich_document_versions",
        "loom_blocks",
        "loom_block_search_index",
    ];
    let mut baseline = Vec::new();
    for table in tables {
        baseline.push((table, embedded_table_count(&store, table).await));
    }
    let request = json!({
        "workspace_id": workspace_id,
        "title": "MT032 import rollback",
        "format": "markdown",
        "snippet": "# Imported rollback\n\nbody"
    });
    store
        .storage
        .test_set_rich_document_projection_failpoint(true)
        .await
        .expect("arm import projection failpoint");
    let failed = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/import")),
        "mt032-import-rollback",
        "operator",
    )
    .json(&request)
    .send()
    .await
    .expect("send failing import");
    assert_eq!(failed.status(), 500);
    store
        .storage
        .test_set_rich_document_projection_failpoint(false)
        .await
        .expect("reset import projection failpoint");
    for (table, expected) in baseline {
        assert_eq!(
            embedded_table_count(&store, table).await,
            expected,
            "{table} must roll back with the failed import"
        );
    }
    let retry = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/import")),
        "mt032-import-retry",
        "operator",
    )
    .json(&request)
    .send()
    .await
    .expect("retry import");
    assert_eq!(retry.status(), 200);
    let retry: Value = retry.json().await.expect("retry import body");
    let document_id = retry["document"]["rich_document_id"]
        .as_str()
        .expect("retry import document id");
    assert!(store
        .db
        .get_loom_block(&workspace_id, document_id)
        .await
        .is_ok());
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_save_delete_and_backlink_rebuild_delete_races_do_not_resurrect() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let updated_content = json!({"type": "doc", "content": [{
        "type": "paragraph", "content": [{"type": "text", "text": "race update"}]
    }]});

    let save_first = create_doc(&base, &http, &workspace_id, "MT032 save waits").await;
    let save_first_id = save_first["document"]["rich_document_id"]
        .as_str()
        .expect("save-waits document id")
        .to_owned();
    docs_api::test_arm_document_pause(
        &save_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::SaveBeforeMutation,
    );
    let save_http = http.clone();
    let save_base = base.clone();
    let save_id = save_first_id.clone();
    let save_content = updated_content.clone();
    let waiting_save = tokio::spawn(async move {
        headers_with_kind(
            save_http.put(format!("{save_base}/knowledge/documents/{save_id}/save")),
            "mt032-waiting-save",
            "operator",
        )
        .json(&json!({"expected_version": 1, "content_json": save_content}))
        .send()
        .await
        .expect("waiting save response")
    });
    docs_api::test_wait_for_document_pause(
        &save_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::SaveBeforeMutation,
    )
    .await;
    let deleted = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{save_first_id}")),
        "mt032-delete-before-save",
        "operator",
    )
    .send()
    .await
    .expect("delete before waiting save");
    assert_eq!(deleted.status(), 200);
    docs_api::test_release_document_pause(
        &save_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::SaveBeforeMutation,
    );
    assert_eq!(
        waiting_save
            .await
            .expect("waiting save task joins")
            .status(),
        404,
        "a stale queued save must not resurrect a deleted document"
    );

    let delete_first = create_doc(&base, &http, &workspace_id, "MT032 delete waits").await;
    let delete_first_id = delete_first["document"]["rich_document_id"]
        .as_str()
        .expect("delete-waits document id")
        .to_owned();
    docs_api::test_arm_document_pause(
        &delete_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::DeleteBeforeMutation,
    );
    let delete_http = http.clone();
    let delete_base = base.clone();
    let delete_id = delete_first_id.clone();
    let waiting_delete = tokio::spawn(async move {
        headers_with_kind(
            delete_http.delete(format!("{delete_base}/knowledge/documents/{delete_id}")),
            "mt032-waiting-delete",
            "operator",
        )
        .send()
        .await
        .expect("waiting delete response")
    });
    docs_api::test_wait_for_document_pause(
        &delete_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::DeleteBeforeMutation,
    )
    .await;
    let saved = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{delete_first_id}/save")),
        "mt032-save-before-delete",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": updated_content.clone()}))
    .send()
    .await
    .expect("save before waiting delete");
    assert_eq!(saved.status(), 200);
    docs_api::test_release_document_pause(
        &delete_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::DeleteBeforeMutation,
    );
    assert_eq!(
        waiting_delete
            .await
            .expect("waiting delete task joins")
            .status(),
        409,
        "a delete with a stale predecessor snapshot must not erase the committed save"
    );
    let retained = store
        .db
        .get_knowledge_rich_document(&delete_first_id)
        .await
        .expect("read save-before-delete document")
        .expect("save-before-delete document remains live");
    assert_eq!(retained.doc_version, 2);

    let rebuild_first = create_doc(&base, &http, &workspace_id, "MT032 rebuild waits").await;
    let rebuild_first_id = rebuild_first["document"]["rich_document_id"]
        .as_str()
        .expect("rebuild-waits document id")
        .to_owned();
    docs_api::test_arm_document_pause(
        &rebuild_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::BacklinkRebuildBeforeMutation,
    );
    let rebuild_http = http.clone();
    let rebuild_base = base.clone();
    let rebuild_id = rebuild_first_id.clone();
    let waiting_rebuild = tokio::spawn(async move {
        headers_with_kind(
            rebuild_http.post(format!(
                "{rebuild_base}/knowledge/documents/{rebuild_id}/backlinks"
            )),
            "mt032-waiting-rebuild",
            "operator",
        )
        .send()
        .await
        .expect("waiting rebuild response")
    });
    docs_api::test_wait_for_document_pause(
        &rebuild_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::BacklinkRebuildBeforeMutation,
    )
    .await;
    let rebuild_deleted = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{rebuild_first_id}")),
        "mt032-delete-before-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("delete before waiting rebuild");
    assert_eq!(rebuild_deleted.status(), 200);
    docs_api::test_release_document_pause(
        &rebuild_first_id,
        docs_api::KnowledgeDocumentTestPausePoint::BacklinkRebuildBeforeMutation,
    );
    assert_eq!(
        waiting_rebuild
            .await
            .expect("waiting rebuild task joins")
            .status(),
        404,
        "a stale queued backlink rebuild must not recreate deleted projections"
    );
    assert!(
        store
            .db
            .list_knowledge_document_backlinks_from(&rebuild_first_id)
            .await
            .expect("read post-delete backlink projection")
            .is_empty(),
        "deleted document backlinks must remain absent"
    );

    let cleanup = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{delete_first_id}")),
        "mt032-delete-after-stale-delete",
        "operator",
    )
    .send()
    .await
    .expect("fresh cleanup delete");
    assert_eq!(cleanup.status(), 200);
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[allow(dead_code)]
const MT141_MIGRATION_0343_DISPOSITION: &str =
    "RETIRED migration 0343: schema.surql lines under \
     `0343_knowledge_rich_document_loom_projection` explicitly omit legacy backfill DML because \
     embedded bootstrap opens an empty latest-schema database; current projection identity is proved \
     by loom_blocks and loom_block_search_index schema plus the atomic runtime create/save transactions.";

#[allow(dead_code)]
const MT141_MIGRATION_0347_DISPOSITION: &str =
    "RETIRED migration 0347: current schema.surql defines knowledge_document_backlinks.workspace_id \
     as record<workspaces>, source_document_id as record<knowledge_rich_documents>, and target as the \
     stable string target; current same-workspace Loom resolution is runtime behavior in \
     knowledge::resolve_backlink_rows, while cross-workspace Loom ids are dropped before persistence.";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt032_delete_is_atomic_and_removes_canvas_references() {
    let store = open_embedded_store()
        .await
        .expect("MT-032 requires an isolated embedded store");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let target = create_doc(&base, &http, &workspace_id, "MT032 Delete Target").await;
    let document_id = target["document"]["rich_document_id"]
        .as_str()
        .expect("delete target id")
        .to_owned();

    let referrer_response = headers_with_kind(
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
                "content": [{
                    "type": "hsLink",
                    "attrs": {
                        "refKind": "note",
                        "refValue": document_id,
                        "label": "MT032 Delete Target"
                    }
                }]
            }]
        }
    }))
    .send()
    .await
    .expect("create delete referrer");
    assert_eq!(referrer_response.status(), 200);
    let referrer: Value = referrer_response
        .json()
        .await
        .expect("delete referrer body");
    let referrer_id = referrer["document"]["rich_document_id"]
        .as_str()
        .expect("delete referrer id")
        .to_owned();

    let rebuilt = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referrer_id}/backlinks"
        )),
        "mt032-delete-referrer-rebuild",
        "operator",
    )
    .send()
    .await
    .expect("rebuild delete referrer backlinks");
    assert_eq!(rebuilt.status(), 200);
    let projected = store
        .db
        .list_knowledge_document_backlinks_from(&referrer_id)
        .await
        .expect("projected delete-target backlink");
    let relationship_id = projected
        .iter()
        .find(|row| row.target == document_id)
        .expect("stable delete-target relationship")
        .relationship_id
        .clone();

    // Recreate the former independently-owned edge collision through public
    // typed APIs only. Rebuild must fail atomically without deleting or
    // overwriting the independent edge.
    let write_ctx = WriteContext::human(Some("mt032-delete-proof".to_owned()));
    store
        .db
        .delete_loom_edge(&write_ctx, &workspace_id, &relationship_id)
        .await
        .expect("remove projector-owned edge before collision setup");
    store
        .db
        .replace_knowledge_document_backlinks(&referrer_id, Vec::new())
        .await
        .expect("remove rebuildable backlink before collision setup");
    store
        .db
        .create_loom_edge(
            &write_ctx,
            NewLoomEdge {
                edge_id: Some(relationship_id.clone()),
                workspace_id: workspace_id.clone(),
                source_block_id: referrer_id.clone(),
                target_block_id: document_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("create independently-owned collision edge");
    let collision_rebuild = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referrer_id}/backlinks"
        )),
        "mt032-independent-edge-collision",
        "operator",
    )
    .send()
    .await
    .expect("rebuild against independently-owned edge collision");
    assert_eq!(collision_rebuild.status(), 409);
    assert!(
        store
            .db
            .list_knowledge_document_backlinks_from(&referrer_id)
            .await
            .expect("collision rollback backlink state")
            .is_empty(),
        "failed rebuild must roll its backlink insert back"
    );
    let independent_edges = store
        .db
        .list_loom_edges_for_block(&workspace_id, &referrer_id)
        .await
        .expect("independent collision edge readback");
    assert!(
        independent_edges.iter().any(|edge| {
            edge.edge_id == relationship_id
                && edge.source_block_id == referrer_id
                && edge.target_block_id == document_id
                && edge.created_by == LoomEdgeCreatedBy::User
        }),
        "failed rebuild must preserve the independently-owned edge"
    );
    store
        .db
        .delete_loom_edge(&write_ctx, &workspace_id, &relationship_id)
        .await
        .expect("remove independent collision fixture");
    let restored = headers_with_kind(
        http.post(format!(
            "{base}/knowledge/documents/{referrer_id}/backlinks"
        )),
        "mt032-restore-owned-edge",
        "operator",
    )
    .send()
    .await
    .expect("restore projector-owned edge");
    assert_eq!(restored.status(), 200);

    let canvas_block = store
        .db
        .create_loom_block(
            &write_ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some("MT032 Delete Canvas".to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create delete canvas block");
    store
        .db
        .bridge_loom_block_to_knowledge(&write_ctx, &workspace_id, &canvas_block.block_id)
        .await
        .expect("bridge delete canvas block");
    store
        .db
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
        .expect("create delete canvas board");
    let placement = store
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
        .expect("place delete target on canvas");
    let source_before = store
        .db
        .get_knowledge_source_by_document_id(&workspace_id, &document_id)
        .await
        .expect("delete target source lookup")
        .expect("delete target is indexed");
    assert!(!source_before.stale);

    let deleted = headers_with_kind(
        http.delete(format!("{base}/knowledge/documents/{document_id}")),
        "mt032-delete-success",
        "operator",
    )
    .send()
    .await
    .expect("delete with dependency cleanup");
    assert_eq!(deleted.status(), 200);
    let deleted: Value = deleted.json().await.expect("delete response");
    assert_eq!(deleted["loom_block_deleted"], true);
    assert_eq!(deleted["source_marked_stale"], true);
    let receipt_id = deleted["deleted_receipt_event_id"]
        .as_str()
        .expect("delete receipt id");

    let inspector = store.storage.test_inspector();
    let documents = inspector
        .table_selector("knowledge_rich_documents")
        .await
        .expect("rich-document table selector");
    let tombstone = inspector
        .project(
            &documents,
            &[
                documents.field("deleted_at").expect("deleted_at field"),
                documents
                    .field("deleted_receipt_event_id")
                    .expect("delete receipt field"),
            ],
            RowFilter::IdEquals(document_id.clone()),
        )
        .await
        .expect("tombstone projection");
    assert_eq!(tombstone.len(), 1, "authority row remains as one tombstone");
    assert!(!tombstone[0].values["deleted_at"].is_null());
    assert!(!tombstone[0].values["deleted_receipt_event_id"].is_null());

    let loom_blocks = inspector
        .table_selector("loom_blocks")
        .await
        .expect("LoomBlock table selector");
    assert_eq!(
        inspector
            .row_count(&loom_blocks, RowFilter::IdEquals(document_id.clone()))
            .await
            .expect("deleted LoomBlock count"),
        0
    );
    let placements = inspector
        .table_selector("loom_canvas_placements")
        .await
        .expect("canvas placement table selector");
    assert_eq!(
        inspector
            .row_count(
                &placements,
                RowFilter::IdEquals(placement.placement_id.clone()),
            )
            .await
            .expect("deleted placement count"),
        0
    );
    assert!(
        store
            .db
            .get_knowledge_rich_document(&document_id)
            .await
            .expect("deleted document readback")
            .is_none(),
        "tombstone must not remain live"
    );
    let source_after = store
        .db
        .get_knowledge_source_by_document_id(&workspace_id, &document_id)
        .await
        .expect("deleted source lookup")
        .expect("deleted source remains auditable");
    assert!(source_after.stale);
    assert!(store
        .db
        .list_knowledge_document_backlinks_from(&referrer_id)
        .await
        .expect("deleted-target backlink cleanup")
        .is_empty());
    assert!(store
        .db
        .list_loom_edges_for_block(&workspace_id, &referrer_id)
        .await
        .expect("deleted-target Loom edge cleanup")
        .iter()
        .all(|edge| edge.target_block_id != document_id));
    let board = store
        .db
        .get_canvas_board(&workspace_id, &canvas_block.block_id)
        .await
        .expect("canvas readback after target deletion");
    assert!(
        board
            .placements
            .iter()
            .all(|row| row.placement_id != placement.placement_id),
        "atomic delete must remove every canvas reference to the target"
    );
    let events = store
        .db
        .list_kernel_events_for_aggregate("knowledge_rich_document", &document_id)
        .await
        .expect("delete EventLedger readback");
    assert!(events.iter().any(|event| event.event_id == receipt_id));

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// MT-158 adversarial-v2: actor-kind fail-closed boundary.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_missing_actor_kind_is_least_privileged_never_system() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt158_missing_actor_kind...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_cloud_model_cannot_write_and_bogus_kind_is_rejected() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt158_cloud_model...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
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
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt151_imported_html...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
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
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt152_save_path...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt246_save_rejects_cross_document_crdt_id() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt246_save_rejects_cross_document_crdt_id: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// MT-156 adversarial-v2: history is paginated and omits version bodies.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_history_is_paginated_and_omits_version_bodies() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt156_history...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// MT-154 adversarial-v2: documents are indexed into the Project Knowledge
// Index (source row + title entity + staleness on change).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_save_indexes_document_into_project_knowledge_index() {
    use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};

    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt154_save_indexes...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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

    let source = store
        .db
        .get_knowledge_source_by_document_id(&workspace_id, &doc_id)
        .await
        .expect("source lookup")
        .expect("document source row exists in the Project Knowledge Index");
    assert_eq!(source.content_hash, doc_sha);
    assert!(!source.stale, "freshly indexed source is not stale");
    let entity = store
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
    let source = store
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
    let entity = store
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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// MT-157 adversarial-v2: move absent != null; batch with per-item reporting.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_move_empty_body_preserves_membership_and_batch_reports_per_item() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt157_move...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// MT-149 adversarial-v2: a committed save never returns an error.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt149_committed_save_never_errors_when_post_commit_steps_fail() {
    let store = open_embedded_store()
        .await
        .expect("MT-149 requires isolated embedded-store proof");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;
    let target = create_doc(&base, &http, &workspace_id, "MT149 link target").await;
    let target_id = target["document"]["rich_document_id"]
        .as_str()
        .expect("MT149 target id")
        .to_owned();
    let created = create_doc(&base, &http, &workspace_id, "MT149 post-commit").await;
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("MT149 document id")
        .to_owned();
    for point in [
        docs_api::KnowledgeDocumentPostCommitFailpoint::Receipt,
        docs_api::KnowledgeDocumentPostCommitFailpoint::Backlinks,
        docs_api::KnowledgeDocumentPostCommitFailpoint::Embeds,
    ] {
        docs_api::test_arm_document_post_commit_failpoint(&document_id, point);
    }
    let content = json!({
        "type": "doc",
        "content": [
            {
                "type": "paragraph",
                "content": [{
                    "type": "hsLink",
                    "attrs": {
                        "refKind": "note",
                        "refValue": target_id,
                        "label": "MT149 link target"
                    }
                }]
            },
            {
                "type": "image",
                "attrs": {"target": "KMED-mt149"},
                "content": [{"type": "text", "text": "MT149 embed"}]
            }
        ]
    });
    let response = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt149-post-commit-failures",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": content.clone()}))
    .send()
    .await
    .expect("send failpoint save");
    assert_eq!(
        response.status(),
        200,
        "post-commit failures must not turn a committed save into an error"
    );
    let body: Value = response.json().await.expect("post-commit failure body");
    assert_eq!(body["document"]["doc_version"], 2);
    assert!(body["save_receipt_event_id"].is_null());
    assert!(body["receipt_error"].is_string());
    assert_eq!(body["backlinks_persisted"], 0);
    assert!(body["backlinks_error"].is_string());
    assert_eq!(body["embeds_persisted"], 0);
    assert!(body["embeds_error"].is_string());
    let committed = store
        .db
        .get_knowledge_rich_document(&document_id)
        .await
        .expect("read committed save")
        .expect("saved document remains live");
    assert_eq!(committed.doc_version, 2);
    assert_eq!(committed.content_json, content);

    let retry = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{document_id}/save")),
        "mt149-failpoints-reset",
        "operator",
    )
    .json(&json!({"expected_version": 2, "content_json": content}))
    .send()
    .await
    .expect("send save after failpoint reset");
    assert_eq!(retry.status(), 200);
    let retry: Value = retry.json().await.expect("reset save body");
    assert!(retry["save_receipt_event_id"].is_string());
    assert!(retry["receipt_error"].is_null());
    assert_eq!(retry["backlinks_persisted"], 1);
    assert!(retry["backlinks_error"].is_null());
    assert_eq!(retry["embeds_persisted"], 1);
    assert!(retry["embeds_error"].is_null());
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_imported_markdown_table_document_roundtrips_load_save_export() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt151_imported_markdown_table...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

    let md = "# Title\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\ntail paragraph";
    let (_doc_id, loaded) =
        import_roundtrip(&base, &http, &workspace_id, "mdtable", "markdown", md).await;

    let blocks = loaded["tree"]["blocks"].as_array().expect("blocks");
    assert!(blocks.iter().any(|b| b["kind"] == "imported_raw"));
    assert!(blocks.iter().any(|b| b["kind"] == "heading"));
    assert!(blocks.iter().any(|b| b["kind"] == "paragraph"));
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt255_backend_draft_recovery_roundtrips_and_clears_on_save_or_discard() {
    let Some(store) = open_embedded_store().await else {
        eprintln!("SKIP mt255_backend_draft_recovery...: embedded store unavailable");
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

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
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}
