//! WP-KERNEL-009 MT-258 NoteTransclusion — REAL PostgreSQL route-level proof.
//!
//! Obsidian-parity note transclusion on LoomBlock authority. Drives the actual
//! Axum routes (`api::loom::routes` + `api::knowledge_documents::routes`) over a
//! loopback listener against the Handshake-managed PostgreSQL cluster (no
//! Docker, no SQLite, no mock). Proves the three transclusion invariants the
//! reviewers hunt for:
//!
//!   1. READ-THROUGH: GET /loom/blocks/:id/transclusion resolves a block to its
//!      SOURCE rich document (`loom_blocks.document_id` -> the rich document) and
//!      returns the SOURCE content_json + version (`resolved=true`).
//!   2. EDIT-ROUTES-TO-SOURCE: editing the transcluded content via
//!      `PUT /knowledge/documents/:source/save` mutates the SOURCE document, and
//!      a subsequent transclusion read-through reflects the new SOURCE content
//!      and version — one authority document, not a copy.
//!   3. NO-COPY: a HOST document whose content is a single `loomTransclusion`
//!      atom node (`attrs.refValue` = source block id) persists ONLY that atom
//!      node on save/reload — the host never absorbs the source body.

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_documents as docs_api;
use handshake_core::api::loom as loom_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::{KnowledgeStore, NewKnowledgeRichDocument};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{Database, NewDocument, NewLoomBlock, WriteContext};
use handshake_core::storage::{LoomBlockContentType, LoomBlockDerived};
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
        Err(handshake_core::storage::StorageError::NotFound("diagnostic"))
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

/// Boot the real loom + document routes over loopback against the isolated
/// schema. Both route groups share one AppState so a loom block's
/// `document_id` resolves to a rich document created through the docs API.
async fn server(pg: &KnowledgePg) -> (String, reqwest::Client) {
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
            profile: ModelProfile::new("transclusion-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = loom_api::routes(state.clone()).merge(docs_api::routes(state));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("transclusion api server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

fn doc_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("transclusion-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-TRANS-{label}"))
        .header("x-hsk-session-run-id", format!("SR-TRANS-{label}"))
        .header("x-hsk-actor-kind", "operator")
}

/// Build a storage handle on the isolated schema for setup that the HTTP
/// surface cannot express (the legacy `documents` anchor + a rich document
/// anchored to it). This is REAL Postgres, no mock.
async fn storage_for(pg: &KnowledgePg) -> PostgresDatabase {
    PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect test storage")
}

/// Set up a SOURCE: a legacy `documents` row, a rich document anchored to it
/// (the transclusion authority), and a LoomBlock whose `document_id` is that
/// same legacy anchor. Returns (block_id, source_rich_document_id, version).
async fn setup_source(
    db: &PostgresDatabase,
    workspace_id: &str,
    text: &str,
) -> (String, String, i64) {
    let ctx = WriteContext::human(None);
    // 1. Legacy documents anchor (the FK target of loom_blocks.document_id).
    let document = db
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace_id.to_string(),
                title: "Transclusion source anchor".to_string(),
            },
        )
        .await
        .expect("create documents anchor");

    // 2. The authority rich document anchored to that documents row.
    let rich = db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: workspace_id.to_string(),
            document_id: Some(document.id.clone()),
            title: "Transclusion source note".to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: json!({
                "type": "doc",
                "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": text }] }
                ]
            }),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            project_ref: None,
            folder_ref: None,
            authority_label: None,
            owner_actor_kind: None,
            owner_actor_id: None,
        })
        .await
        .expect("create source rich document");

    // 3. The LoomBlock backed by that anchor (note resolves through it).
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Transclusion source block".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create loom block backed by source doc");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge loom block");

    (block.block_id, rich.rich_document_id, rich.doc_version)
}

async fn get_transclusion(
    base: &str,
    http: &reqwest::Client,
    workspace_id: &str,
    block_id: &str,
) -> Value {
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/loom/blocks/{block_id}/transclusion"
        ))
        .send()
        .await
        .expect("get transclusion send");
    assert_eq!(resp.status(), 200, "transclusion read-through must succeed");
    resp.json().await.expect("transclusion json")
}

/// Recursively collect every plain-text fragment in a ProseMirror JSON node.
fn collect_text(node: &Value, out: &mut Vec<String>) {
    if let Some(text) = node.get("text").and_then(Value::as_str) {
        out.push(text.to_string());
    }
    if let Some(content) = node.get("content").and_then(Value::as_array) {
        for child in content {
            collect_text(child, out);
        }
    }
}

/// Count nodes of a given `type` anywhere in a ProseMirror JSON tree.
fn count_node_type(node: &Value, node_type: &str, count: &mut usize) {
    if node.get("type").and_then(Value::as_str) == Some(node_type) {
        *count += 1;
    }
    if let Some(content) = node.get("content").and_then(Value::as_array) {
        for child in content {
            count_node_type(child, node_type, count);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt258_transclusion_read_through_edit_routes_to_source_and_host_stays_copy_free() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt258 transclusion proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let db = storage_for(&pg).await;
    let (base, http) = server(&pg).await;

    // --- Source authority document + a LoomBlock that resolves to it ---------
    let (block_id, source_document_id, source_v1) =
        setup_source(&db, &workspace_id, "ORIGINAL source body").await;

    // --- 1. READ-THROUGH: resolves to the SOURCE document content ------------
    let resolved = get_transclusion(&base, &http, &workspace_id, &block_id).await;
    assert_eq!(resolved["resolved"], json!(true), "read-through must resolve");
    assert_eq!(
        resolved["source_document_id"].as_str(),
        Some(source_document_id.as_str()),
        "transclusion resolves to the SOURCE rich document id"
    );
    assert_eq!(
        resolved["source_doc_version"].as_i64(),
        Some(source_v1),
        "read-through returns the SOURCE current version"
    );
    let mut read_text = Vec::new();
    collect_text(&resolved["content_json"], &mut read_text);
    assert!(
        read_text.join(" ").contains("ORIGINAL source body"),
        "read-through returns the live SOURCE content, got {read_text:?}"
    );

    // --- 2. EDIT-ROUTES-TO-SOURCE: save the source, re-read transclusion -----
    let save_resp = doc_headers(
        http.put(format!("{base}/knowledge/documents/{source_document_id}/save")),
        "edit",
    )
    .json(&json!({
        "expected_version": source_v1,
        "content_json": {
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "EDITED via transclusion source" }] }
            ]
        }
    }))
    .send()
    .await
    .expect("save source send");
    assert_eq!(save_resp.status(), 200, "save to SOURCE must succeed");
    let saved: Value = save_resp.json().await.expect("save json");
    let source_v2 = saved["document"]["doc_version"]
        .as_i64()
        .expect("post-edit doc_version");
    assert!(source_v2 > source_v1, "source version must advance on edit");

    let resolved_after = get_transclusion(&base, &http, &workspace_id, &block_id).await;
    assert_eq!(
        resolved_after["source_doc_version"].as_i64(),
        Some(source_v2),
        "transclusion re-read reflects the NEW source version (single authority)"
    );
    let mut read_after = Vec::new();
    collect_text(&resolved_after["content_json"], &mut read_after);
    let joined_after = read_after.join(" ");
    assert!(
        joined_after.contains("EDITED via transclusion source"),
        "transclusion shows the edited SOURCE content, got {read_after:?}"
    );
    assert!(
        !joined_after.contains("ORIGINAL source body"),
        "edit routed to the source (old content gone), not a divergent copy"
    );

    // --- 3. NO-COPY: a HOST doc embedding the block persists ONLY the atom ----
    // The host content is a single loomTransclusion node carrying the reference.
    let host_content = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "Host preamble." }] },
            { "type": "loomTransclusion", "attrs": { "refValue": block_id } }
        ]
    });
    let host_created = doc_headers(http.post(format!("{base}/knowledge/documents")), "host")
        .json(&json!({
            "workspace_id": workspace_id,
            "title": "Host document with transclusion",
            "content_json": host_content
        }))
        .send()
        .await
        .expect("create host doc send");
    assert_eq!(host_created.status(), 200, "host doc create must succeed");
    let host_created: Value = host_created.json().await.expect("host create json");
    let host_document_id = host_created["document"]["rich_document_id"]
        .as_str()
        .expect("host rich_document_id")
        .to_string();

    // Reload the host doc straight from the authority store and assert the
    // persisted content_json carries ONLY the atom node — never the source body.
    let host_loaded = doc_headers(
        http.get(format!("{base}/knowledge/documents/{host_document_id}")),
        "host-load",
    )
    .send()
    .await
    .expect("load host doc send");
    assert_eq!(host_loaded.status(), 200, "host doc load must succeed");
    let host_loaded: Value = host_loaded.json().await.expect("host load json");
    let host_content_json = &host_loaded["document"]["content_json"];

    let mut transclusion_nodes = 0usize;
    count_node_type(host_content_json, "loomTransclusion", &mut transclusion_nodes);
    assert_eq!(
        transclusion_nodes, 1,
        "host persists exactly one loomTransclusion atom node"
    );

    let mut host_text = Vec::new();
    collect_text(host_content_json, &mut host_text);
    let host_joined = host_text.join(" ");
    assert!(
        host_joined.contains("Host preamble."),
        "host keeps its own body"
    );
    assert!(
        !host_joined.contains("EDITED via transclusion source")
            && !host_joined.contains("ORIGINAL source body"),
        "NO-COPY: host persisted JSON must NOT contain the transcluded source body, got {host_text:?}"
    );

    // The atom node carries only the reference (refValue), no copied content.
    let mut atom_ref: Option<String> = None;
    fn find_atom(node: &Value, out: &mut Option<String>) {
        if node.get("type").and_then(Value::as_str) == Some("loomTransclusion") {
            *out = node
                .get("attrs")
                .and_then(|a| a.get("refValue"))
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            assert!(
                node.get("content").is_none(),
                "the persisted transclusion node must be content-free (atom)"
            );
        }
        if let Some(content) = node.get("content").and_then(Value::as_array) {
            for child in content {
                find_atom(child, out);
            }
        }
    }
    find_atom(host_content_json, &mut atom_ref);
    assert_eq!(
        atom_ref.as_deref(),
        Some(block_id.as_str()),
        "the persisted atom node references the source block id"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt258_transclusion_unresolved_block_without_source_is_typed_not_blank() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt258 transclusion unresolved proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = server(&pg).await;

    // A loom block with NO document_id (e.g. an asset/tag block) cannot resolve
    // to a source document; the read-through is a typed unresolved state.
    let resp = http
        .post(format!("{base}/workspaces/{workspace_id}/loom/blocks"))
        .json(&json!({ "content_type": "note", "title": "No source" }))
        .send()
        .await
        .expect("create block send");
    assert_eq!(resp.status(), 200);
    let block: Value = resp.json().await.expect("block json");
    let block_id = block["block_id"].as_str().expect("block_id").to_string();

    let resolved = get_transclusion(&base, &http, &workspace_id, &block_id).await;
    assert_eq!(resolved["resolved"], json!(false));
    assert_eq!(
        resolved["unresolved_reason"].as_str(),
        Some("loom_block_has_no_source_document")
    );
    assert!(resolved["content_json"].is_null());
}
