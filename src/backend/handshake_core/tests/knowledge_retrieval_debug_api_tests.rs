//! WP-KERNEL-009 MT-143 RetrievalDebugApi — adversarial-v2 hardening proof for
//! the stale-reason surface and the repair action, driven
//! over the REAL Axum routes against the embedded store.
//!
//! MT-141 ports the missing-evidence probe through the closed, feature-gated
//! embedded-store test mutator, so the API still proves both source staleness
//! and evidence deletion before repairing the durable bundle.

#[path = "knowledge_memory_fixtures/mod.rs"]
mod knowledge_memory_fixtures;

// WP-KERNEL-012 MT-109 / LM-RLS-002: the retrieval routes run as an authenticated record user
// (persisted account session + live native-MCP channel binding).
#[path = "account_session_support/mod.rs"]
mod account_session_support;

use std::collections::BTreeSet;
use std::sync::Arc;

use account_session_support::{AccountFixture, OwnerSession};
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
    KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef, KnowledgeRetrievalMode, KnowledgeStore,
    NewKnowledgeMemoryPassage,
};
use handshake_core::storage::surreal::{SurrealDatabase, SurrealStorage};
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

async fn app_state_for(storage: &SurrealStorage) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(SurrealDatabase::new(storage.clone())),
        surreal: storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("retrieval-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// The client carries `owner`'s account-session credentials on every request.
async fn retrieval_server(
    storage: SurrealStorage,
    owner: &OwnerSession,
) -> (String, reqwest::Client) {
    let state = app_state_for(&storage).await;
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
    (format!("http://{addr}"), owner.client())
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
/// WP-KERNEL-012 MT-154: the [`MemoryFixture`] seed (root -> source -> span), but inside a
/// workspace the Owner created through the real `POST /workspaces` route, so the account's
/// workspace grant covers the seeded evidence. The seed rows are test setup written through root
/// storage; every assertion below goes through the account-scoped routes as the record user.
async fn account_memory_fixture() -> Option<(MemoryFixture, AccountFixture)> {
    use handshake_core::storage::knowledge::{
        KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
        KnowledgeRootKind, KnowledgeSourceKind, KnowledgeSpanKind, NewKnowledgeSource,
        NewKnowledgeSourceRoot, NewKnowledgeSpan,
    };
    let store = knowledge_memory_fixtures::open_embedded_store().await?;
    let account = AccountFixture::install(&store.storage).await;
    let workspace_id = account
        .create_workspace(&app_state_for(&store.storage).await)
        .await;
    let root = store
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("src/{}", uuid::Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("root");
    let source = store
        .db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: Some(root.root_id),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("memory/graph.rs".to_string()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: "a".repeat(64),
            size_bytes: Some(2048),
            provenance: json!({"discovered_by": "memory_fixture"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("source");
    let span = store
        .db
        .create_knowledge_span(NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Text,
            range_start: 0,
            range_end: 200,
            line_start: Some(1),
            line_end: Some(5),
            section_path: None,
            content_sha256: "b".repeat(64),
            parser_version: "text_v1".to_string(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("memory graph fixture span".to_string()),
        })
        .await
        .expect("span");
    Some((
        MemoryFixture {
            workspace_id,
            source_id: source.source_id,
            span_id: span.span_id,
            store,
        },
        account,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt143_staleness_surface_and_repair_action() {
    // MT-154: the evidence is seeded (root test setup) inside a workspace the Owner created through
    // POST /workspaces; every route below authorizes that workspace as the record user.
    let Some((fx, account)) = account_memory_fixture().await else {
        eprintln!("SKIP mt143_staleness_surface_and_repair_action: embedded storage unavailable");
        return;
    };
    let ws = [("workspace_id", fx.workspace_id.clone())];
    let pool = pool_for(&fx.store).await;

    // A span-backed passage; the executed pipeline (no edges) falls back to it
    // and compiles a REAL bundle with a passage item.
    let passage = fx
        .store
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
        &fx.store.db,
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

    let (base, http) = retrieval_server(fx.store.storage.clone(), &account).await;

    // FRESH bundle: every item ok, stale=false, receipt present.
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/staleness"
        ))
        .query(&ws),
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

    // Delete the projected passage through the closed embedded test seam. The
    // bundle remains append-only while its live evidence becomes missing.
    let passages = fx
        .store
        .storage
        .test_inspector()
        .table_selector("knowledge_memory_passages")
        .await
        .expect("select passage table");
    fx.store
        .storage
        .test_mutator()
        .delete_row(&passages, passage.passage_id.as_str())
        .await
        .expect("delete passage behind durable bundle");
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/{bundle_id}/staleness"
        ))
        .query(&ws),
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
        ))
        .query(&ws),
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
        .store
        .db
        .get_knowledge_context_bundle(repaired_id)
        .await
        .expect("get repaired")
        .expect("repaired bundle persisted");
    assert_eq!(repaired.bundle_id, repaired_id);
    assert!(fx
        .store
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
        .query(&ws)
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 400);
    let resp = nav_headers(
        http.get(format!(
            "{base}/knowledge/retrieval/bundles/CTX-ffffffffffffffff/staleness"
        ))
        .query(&ws),
        "ghost",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 404);
}
