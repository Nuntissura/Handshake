//! WP-KERNEL-009 MT-106 CodeNavigationApi route-level integration proof against
//! REAL Handshake-managed PostgreSQL.
//!
//! Drives the actual Axum routes (`api::knowledge_code_nav::routes`) over a
//! loopback listener (quiet: no foreground window, no focus steal). It indexes a
//! Rust file through the real `CodeIndexEngine`, then navigates the graph through
//! the HTTP surface: symbol lookup, symbol detail (definition span), references
//! (callers/callees), tests, citation spans, and the Monaco file-lens payload.
//! Every nav query MUST require the backend-navigation identity headers (400 if
//! absent) and leave a `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` receipt (the response
//! returns its event id).
//!
//! No SQLite, no mock store: the AppState pool and the engine handle both point
//! at the SAME isolated schema the migrations ran in.

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_code_nav as nav_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_code_index::parser::{CodeLanguage, CodeParserAdapter};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgeRootKind, KnowledgeStore, NewKnowledgeSourceRoot,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
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
        _id: Uuid,
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
        .expect("connect AppState storage to isolated schema")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool to isolated schema");
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("code-nav-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

async fn start_server(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = nav_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("code nav api server");
    });
    (format!("http://{addr}"), server)
}

fn nav_headers(client: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    client
        .header("x-hsk-actor-kind", "model_adapter")
        .header("x-hsk-actor-id", format!("code-nav-test-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-NAV-{label}"))
        .header("x-hsk-session-run-id", format!("SR-NAV-{label}"))
        .header("x-hsk-correlation-id", format!("CORR-NAV-{label}"))
}

const RUST_SRC: &str = r#"
/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 { a + b }

pub fn caller() -> i32 { add(1, 2) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds() { assert_eq!(add(1, 2), 3); }
}
"#;

async fn index_fixture(pg: &KnowledgePg) -> String {
    let workspace_id = pg.create_workspace().await;
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect engine handle");
    let eng = CodeIndexEngine::new(Arc::new(db));
    let context = CodeIndexContext {
        actor: KernelActor::System("code-nav-fixture".to_string()),
        kernel_task_run_id: "KTR-fixture".to_string(),
        session_run_id: "SR-fixture".to_string(),
        correlation_id: None,
    };
    let root = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("root/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("root")
        .root_id;
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/lib.rs", RUST_SRC)
        .await
        .expect("register");
    eng.index_code_source(
        &context,
        &workspace_id,
        &source_id,
        "src/lib.rs",
        RUST_SRC,
        None,
    )
    .await
    .expect("index");
    workspace_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt106_nav_api_lookup_definition_references_tests_spans_with_receipts() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt106_nav_api: no PostgreSQL");
        return;
    };
    let workspace_id = index_fixture(&pg).await;
    let state = app_state_for(&pg.schema_url).await;
    let (base, server) = start_server(state).await;
    let http = reqwest::Client::new();

    // --- Missing identity headers -> 400 (receipt law) ------------------------
    let no_hdr = http
        .get(format!("{base}/knowledge/code/symbols"))
        .query(&[("workspace_id", workspace_id.as_str()), ("name", "add")])
        .send()
        .await
        .expect("send no-header");
    assert_eq!(no_hdr.status(), 400, "nav without identity must be 400");

    // --- Symbol lookup by name ------------------------------------------------
    let lookup = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols"))
            .query(&[("workspace_id", workspace_id.as_str()), ("name", "add")]),
        "lookup",
    )
    .send()
    .await
    .expect("lookup send");
    assert_eq!(lookup.status(), 200);
    let lookup_body: Value = lookup.json().await.expect("lookup json");
    assert!(
        lookup_body["nav_receipt_event_id"].is_string(),
        "lookup must leave a retrieval receipt"
    );
    let matches = lookup_body["matches"].as_array().expect("matches array");
    let add_match = matches
        .iter()
        .find(|m| m["symbol_key"] == "rust:src/lib.rs#add")
        .expect("add in lookup");
    let add_id = add_match["symbol_entity_id"]
        .as_str()
        .expect("add id")
        .to_string();
    // Definition span present.
    assert!(add_match["definition"]["line_start"].as_i64().unwrap_or(0) > 0);

    // --- Symbol detail --------------------------------------------------------
    let detail = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/{add_id}")),
        "detail",
    )
    .send()
    .await
    .expect("detail send");
    assert_eq!(detail.status(), 200);
    let detail_body: Value = detail.json().await.expect("detail json");
    assert_eq!(detail_body["symbol"]["display_name"], "add");

    // --- References: add has a caller (incoming reference) --------------------
    let refs = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/{add_id}/references")),
        "refs",
    )
    .send()
    .await
    .expect("refs send");
    assert_eq!(refs.status(), 200);
    let refs_body: Value = refs.json().await.expect("refs json");
    let callers = refs_body["callers"].as_array().expect("callers");
    assert!(
        callers
            .iter()
            .any(|c| c["symbol_key"] == "rust:src/lib.rs#caller"),
        "caller should appear as a caller of add: {callers:?}"
    );
    // Caller evidence carries a span.
    let caller = callers
        .iter()
        .find(|c| c["symbol_key"] == "rust:src/lib.rs#caller")
        .unwrap();
    assert!(
        caller["evidence_spans"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "reference must carry evidence spans"
    );

    // --- Tests: the `adds` test validates add ---------------------------------
    let tests = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/{add_id}/tests")),
        "tests",
    )
    .send()
    .await
    .expect("tests send");
    assert_eq!(tests.status(), 200);
    let tests_body: Value = tests.json().await.expect("tests json");
    let test_list = tests_body["tests"].as_array().expect("tests array");
    assert!(
        test_list
            .iter()
            .any(|t| t["test_symbol_key"] == "rust:src/lib.rs#tests::adds"),
        "the adds test should validate add: {test_list:?}"
    );

    // --- Spans: citation spans for add ----------------------------------------
    let spans = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/{add_id}/spans")),
        "spans",
    )
    .send()
    .await
    .expect("spans send");
    assert_eq!(spans.status(), 200);
    let spans_body: Value = spans.json().await.expect("spans json");
    let span_list = spans_body["spans"].as_array().expect("spans array");
    assert!(!span_list.is_empty(), "add must expose citation spans");
    assert!(span_list.iter().any(|s| s["span_kind"] == "ast"));

    // --- File lens (MT-109 via the API) ---------------------------------------
    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let lens = nav_headers(
        http.get(format!("{base}/knowledge/code/files/src%2Flib.rs/lens"))
            .query(&[
                ("workspace_id", workspace_id.as_str()),
                ("content_hash", sha256_hex(RUST_SRC.as_bytes()).as_str()),
                ("parser_version", parser_version.as_str()),
            ]),
        "lens",
    )
    .send()
    .await
    .expect("lens send");
    assert_eq!(lens.status(), 200);
    let lens_body: Value = lens.json().await.expect("lens json");
    assert_eq!(lens_body["staleness"]["state"], "fresh");
    let entries = lens_body["entries"].as_array().expect("lens entries");
    assert!(
        entries
            .iter()
            .any(|e| e["symbol_key"] == "rust:src/lib.rs#add"),
        "lens should list add"
    );
    assert!(lens_body["nav_receipt_event_id"].is_string());

    // --- Unknown symbol id -> 404 ---------------------------------------------
    let missing = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/KEN-deadbeef")),
        "missing",
    )
    .send()
    .await
    .expect("missing send");
    assert_eq!(missing.status(), 404);

    server.abort();
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
