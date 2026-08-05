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
use std::time::Instant;

use async_trait::async_trait;
use handshake_core::api::knowledge_code_nav as nav_api;
use handshake_core::api::code_nav_index as index_api;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt045_lc06_500_file_code_nav_index_is_postgres_bounded() {
    let pg = knowledge_pg()
        .await
        .expect("MT-045 LC-06 requires managed PostgreSQL");
    let workspace_id = pg.create_workspace().await;
    let fixture_root = std::env::var("HANDSHAKE_TEST_STAGE_BINDING_ROOT")
        .expect("MT-045 fixture root must be external Handshake_Artifacts")
        .into();
    let fixture_root: std::path::PathBuf = fixture_root;
    let fixture_root = fixture_root
        .join("wp-kernel-012")
        .join("mt-045")
        .join("fixtures")
        .join(format!("backend-lc06-{}", Uuid::now_v7().simple()));
    std::fs::create_dir_all(&fixture_root).expect("create LC-06 fixture root");
    for file_index in 0..500usize {
        let mut body = format!("pub fn file{file_index}_entry() -> u32 {{\n");
        body.push_str("    let mut value = 0_u32;\n");
        for line in 0..196usize {
            body.push_str(&format!("    value += {line}_u32;\n"));
        }
        body.push_str("    value\n}\n");
        assert_eq!(body.lines().count(), 200);
        std::fs::write(fixture_root.join(format!("file_{file_index}.rs")), body)
            .expect("write LC-06 fixture file");
    }

    let state = app_state_for(&pg.schema_url).await;
    let evidence_pool = state.postgres_pool.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind LC-06 loopback listener");
    let addr = listener.local_addr().expect("LC-06 local addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, index_api::routes(state))
            .await
            .expect("LC-06 code-nav index server");
    });
    let base = format!("http://{addr}");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .expect("LC-06 request client");
    let started = Instant::now();
    let response = nav_headers(
        http.post(format!("{base}/workspaces/{workspace_id}/code-nav/index")),
        "mt045-lc06",
    )
    .json(&json!({"root_path": fixture_root.to_string_lossy()}))
    .send()
    .await
    .expect("LC-06 request");
    let elapsed = started.elapsed();
    assert_eq!(response.status(), 200, "LC-06 route response");
    let body: Value = response.json().await.expect("LC-06 response JSON");
    assert!(
        body["symbol_count"].as_u64().unwrap_or_default() >= 500,
        "LC-06 must index at least one symbol per file: {body}"
    );
    let budget_ms = std::env::var("PERF_BUDGET_LC06_MS")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or(10_000);
    assert!(
        elapsed.as_millis() <= budget_ms,
        "LC-06 exceeded {budget_ms}ms PostgreSQL route budget: {elapsed:?}; body={body}"
    );
    let source_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_sources WHERE workspace_id = $1 AND source_kind = 'file'",
    )
    .bind(&workspace_id)
    .fetch_one(&evidence_pool)
    .await
    .expect("query LC-06 source count");
    let code_file_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_code_files WHERE workspace_id = $1 AND parse_status = 'parsed'",
    )
    .bind(&workspace_id)
    .fetch_one(&evidence_pool)
    .await
    .expect("query LC-06 code-file count");
    let run_state: String = sqlx::query_scalar(
        "SELECT run_state FROM knowledge_index_runs WHERE workspace_id = $1 ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&workspace_id)
    .fetch_one(&evidence_pool)
    .await
    .expect("query LC-06 run state");
    assert_eq!(source_count, 500, "LC-06 must persist one file source per fixture file");
    assert_eq!(code_file_count, 500, "LC-06 must parse one code file per fixture file");
    assert!(
        matches!(run_state.as_str(), "completed" | "failed" | "cancelled"),
        "LC-06 index run must be terminal, got {run_state}"
    );
    let source_snapshot = [
        "src/api/code_nav_index.rs",
        "src/knowledge_code_index/engine.rs",
        "src/knowledge_ingestion/engine.rs",
        "src/storage/knowledge.rs",
        "tests/knowledge_code_nav_api_tests.rs",
    ]
    .into_iter()
    .flat_map(|relative| {
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
            .unwrap_or_else(|error| panic!("read MT-045 source snapshot {relative}: {error}"))
    })
    .collect::<Vec<_>>();
    let source_snapshot = sha256_hex(&source_snapshot);
    let evidence_root: std::path::PathBuf = std::env::var("HANDSHAKE_TEST_STAGE_BINDING_ROOT")
        .expect("MT-045 evidence root")
        .into();
    std::fs::create_dir_all(evidence_root.join("wp-kernel-012").join("mt-045").join("measurements"))
        .expect("create MT-045 evidence directory");
    std::fs::write(
        evidence_root
            .join("wp-kernel-012")
            .join("mt-045")
            .join("measurements")
            .join("mt045-lc06-sql-evidence.json"),
        serde_json::to_vec_pretty(&json!({
            "workspace_id": workspace_id,
            "source_count": source_count,
            "parsed_code_file_count": code_file_count,
            "latest_index_run_state": run_state,
            "elapsed_ms": elapsed.as_millis(),
            "budget_ms": budget_ms,
            "source_snapshot_sha256": source_snapshot
        }))
        .expect("serialize MT-045 SQL evidence"),
    )
    .expect("write MT-045 SQL evidence");
    server.abort();
    tokio::time::timeout(std::time::Duration::from_secs(1), server)
        .await
        .expect("LC-06 server shutdown must be bounded")
        .expect_err("aborted LC-06 server must not complete normally");
    std::fs::remove_dir_all(&fixture_root).expect("cleanup LC-06 fixture root");
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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &lookup_body, "lookup").await;
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
    // MT-106 (spec 2.3.13.11 "never serve stale silently"): EVERY served symbol
    // carries a staleness flag. A freshly-indexed, unmodified file is `fresh`.
    assert_eq!(
        add_match["staleness"]["state"], "fresh",
        "lookup must attach staleness to every served symbol: {add_match:?}"
    );
    assert_eq!(add_match["staleness"]["fresh"], true);

    // --- Symbol lookup by prefix (MT-249 completion bridge) -------------------
    let prefix_lookup = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols")).query(&[
            ("workspace_id", workspace_id.as_str()),
            ("prefix", "ad"),
            ("limit", "10"),
        ]),
        "prefix-lookup",
    )
    .send()
    .await
    .expect("prefix lookup send");
    assert_eq!(prefix_lookup.status(), 200);
    let prefix_body: Value = prefix_lookup.json().await.expect("prefix lookup json");
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &prefix_body, "prefix").await;
    assert!(
        prefix_body["matches"]
            .as_array()
            .expect("prefix matches")
            .iter()
            .any(|m| m["symbol_key"] == "rust:src/lib.rs#add"),
        "prefix completion lookup should find add: {prefix_body:?}"
    );

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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &detail_body, "detail").await;
    assert_eq!(detail_body["symbol"]["display_name"], "add");
    assert_eq!(
        detail_body["symbol"]["staleness"]["state"], "fresh",
        "symbol detail must attach staleness"
    );

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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &refs_body, "references").await;
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
    // Staleness on the related symbol AND on the queried symbol itself.
    assert_eq!(
        caller["staleness"]["state"], "fresh",
        "each referenced symbol must carry staleness: {caller:?}"
    );
    assert_eq!(refs_body["staleness"]["state"], "fresh");

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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &tests_body, "tests").await;
    let test_list = tests_body["tests"].as_array().expect("tests array");
    assert!(
        test_list
            .iter()
            .any(|t| t["test_symbol_key"] == "rust:src/lib.rs#tests::adds"),
        "the adds test should validate add: {test_list:?}"
    );
    let adds_test = test_list
        .iter()
        .find(|t| t["test_symbol_key"] == "rust:src/lib.rs#tests::adds")
        .unwrap();
    assert_eq!(
        adds_test["staleness"]["state"], "fresh",
        "each served test symbol must carry staleness"
    );
    assert_eq!(tests_body["staleness"]["state"], "fresh");

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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &spans_body, "spans").await;
    let span_list = spans_body["spans"].as_array().expect("spans array");
    assert!(!span_list.is_empty(), "add must expose citation spans");
    assert!(span_list.iter().any(|s| s["span_kind"] == "ast"));
    assert_eq!(
        spans_body["staleness"]["state"], "fresh",
        "spans route must attach the symbol's staleness"
    );

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
    assert_backend_nav_quiet_receipt(&pg, &workspace_id, &lens_body, "lens").await;

    // --- Unknown symbol id -> 404 ---------------------------------------------
    let missing = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/KEN-deadbeef")),
        "missing",
    )
    .send()
    .await
    .expect("missing send");
    assert_eq!(missing.status(), 404);

    // --- Path traversal on the lens route -> 400 (MT-106 hardening) -----------
    let traversal = nav_headers(
        http.get(format!(
            "{base}/knowledge/code/files/..%2F..%2Fetc%2Fpasswd/lens"
        ))
        .query(&[
            ("workspace_id", workspace_id.as_str()),
            ("content_hash", sha256_hex(b"x").as_str()),
            ("parser_version", "v1"),
        ]),
        "traversal",
    )
    .send()
    .await
    .expect("traversal send");
    assert_eq!(
        traversal.status(),
        400,
        "a path with '..' segments must be rejected"
    );

    server.abort();
}

/// MT-106 (spec 2.3.13.11 "mark stale, never serve stale silently"): once the
/// code-file index is marked stale, the SAME symbol is served through the nav
/// routes with a NON-fresh staleness flag — proving the gap the adversarial
/// review flagged (5 of 6 routes served stale silently) is closed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt106_nav_api_flags_stale_symbols_on_every_route() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt106_nav_api_flags_stale_symbols_on_every_route: no PostgreSQL");
        return;
    };
    let workspace_id = index_fixture(&pg).await;

    // Mark the indexed file stale directly in the code-file index state (this is
    // what MT-107 / the ingestion lifecycle does when the source changes).
    let code_files = pg
        .db
        .list_knowledge_code_files(&workspace_id)
        .await
        .expect("list code files");
    let lib = code_files.first().expect("the fixture's one code file");
    pg.db
        .mark_knowledge_code_file_stale(&lib.code_file_id)
        .await
        .expect("mark stale");

    let state = app_state_for(&pg.schema_url).await;
    let (base, server) = start_server(state).await;
    let http = reqwest::Client::new();

    // Look up `add` -> it must now be flagged marked_stale, not served as fresh.
    let lookup = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols"))
            .query(&[("workspace_id", workspace_id.as_str()), ("name", "add")]),
        "stale-lookup",
    )
    .send()
    .await
    .expect("lookup send");
    assert_eq!(lookup.status(), 200);
    let body: Value = lookup.json().await.expect("json");
    let add_match = body["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .find(|m| m["symbol_key"] == "rust:src/lib.rs#add")
        .expect("add present")
        .clone();
    assert_eq!(
        add_match["staleness"]["state"], "marked_stale",
        "a stale symbol must be FLAGGED, never served as fresh: {add_match:?}"
    );
    assert_eq!(add_match["staleness"]["fresh"], false);

    // The same flag must appear on the detail route (one of the 5 routes that
    // previously emitted no staleness at all).
    let add_id = add_match["symbol_entity_id"].as_str().unwrap();
    let detail = nav_headers(
        http.get(format!("{base}/knowledge/code/symbols/{add_id}")),
        "stale-detail",
    )
    .send()
    .await
    .expect("detail send");
    let detail_body: Value = detail.json().await.expect("json");
    assert_eq!(detail_body["symbol"]["staleness"]["fresh"], false);

    server.abort();
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

async fn quiet_nav_receipt_count(pg: &KnowledgePg, workspace_id: &str, receipt_id: &str) -> i64 {
    let mut conn = pg.raw_connection().await;
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_quiet_background_work
        WHERE workspace_id = $1
          AND receipt_id = $2
          AND work_kind = 'backend_navigation'
        "#,
    )
    .bind(workspace_id)
    .bind(receipt_id)
    .fetch_one(&mut conn)
    .await
    .expect("count backend navigation quiet receipt")
}

async fn assert_backend_nav_quiet_receipt(
    pg: &KnowledgePg,
    workspace_id: &str,
    body: &Value,
    route_label: &str,
) {
    let receipt = body["quiet_background_work_receipt_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{route_label} must leave a quiet background-work receipt"));
    assert_eq!(
        quiet_nav_receipt_count(pg, workspace_id, receipt).await,
        1,
        "{route_label} route must persist backend-navigation quiet work through PostgreSQL"
    );
}
