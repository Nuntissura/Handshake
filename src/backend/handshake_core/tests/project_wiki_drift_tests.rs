//! WP-KERNEL-009 MT-242 WikiProjectionDriftAndStaleness — REAL PostgreSQL +
//! route-level proof.
//!
//! Proves LM-PWIKI-006..009 over a wiki bootstrapped from REAL handshake_core
//! sources:
//!   * every compiled page is stamped with the EventLedger source version +
//!     the exact cited-source set (ids + content hashes);
//!   * editing a real cited source flags EXACTLY the citing pages (set
//!     equality proven against the stamps) with concrete reasons (which
//!     source, stamped vs current hash); unrelated pages stay fresh;
//!   * the staleness verdict is attached on EVERY page-serve path of the real
//!     Axum routes (list, single GET, compile, stale) — fail-closed; an
//!     unstamped legacy page serves as `unstamped`, never fresh;
//!   * negative: a no-change recompile yields ZERO stale pages.

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_wiki::compiler::{
    ProjectWikiCompiler, WikiBootstrapOptions, WikiCompileContext,
};
use handshake_core::knowledge_wiki::drift::WikiDriftChecker;
use handshake_core::knowledge_wiki::{CitedSourceKind, WikiCompileStamp, WikiStalenessVerdict};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgeProjectionKind, KnowledgeRebuildStatus,
    KnowledgeRootKind, KnowledgeStore, NewKnowledgeSourceRoot, NewKnowledgeWikiProjection,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, NewLoomBlock, WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-242 wiki drift proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

const CORE_FILES: [&str; 3] = [
    "src/knowledge_code_index/mod.rs",
    "src/knowledge_code_index/staleness.rs",
    "src/knowledge_wiki/mod.rs",
];

/// The real source this suite edits to provoke drift.
const EDIT_TARGET: &str = "src/knowledge_code_index/staleness.rs";

fn core_src(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read real handshake_core source {relative_path}: {err}"))
}

fn wiki_ctx() -> WikiCompileContext {
    WikiCompileContext {
        actor: KernelActor::System("wiki-drift-test".to_string()),
        kernel_task_run_id: "KTR-wiki-drift-test".to_string(),
        session_run_id: "SR-wiki-drift-test".to_string(),
        correlation_id: None,
    }
}

fn index_ctx() -> CodeIndexContext {
    CodeIndexContext {
        actor: KernelActor::System("wiki-drift-index".to_string()),
        kernel_task_run_id: "KTR-wiki-drift-index".to_string(),
        session_run_id: "SR-wiki-drift-index".to_string(),
        correlation_id: None,
    }
}

struct Seeded {
    workspace_id: String,
    engine: CodeIndexEngine,
    root_id: String,
    /// relative_path -> source_id
    sources: std::collections::HashMap<String, String>,
}

async fn seed_workspace(pg: &KnowledgePg) -> Seeded {
    let workspace_id = pg.create_workspace().await;
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect engine handle");
    let engine = CodeIndexEngine::new(Arc::new(db));
    let root_id = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "handshake_core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("root/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("create root")
        .root_id;
    let ctx = index_ctx();
    let mut sources = std::collections::HashMap::new();
    for relative_path in CORE_FILES {
        let text = core_src(relative_path);
        let source_id = engine
            .register_code_source(&workspace_id, Some(&root_id), relative_path, &text)
            .await
            .expect("register real source");
        engine
            .index_code_source(&ctx, &workspace_id, &source_id, relative_path, &text, None)
            .await
            .expect("index real source");
        sources.insert(relative_path.to_string(), source_id);
    }
    Seeded {
        workspace_id,
        engine,
        root_id,
        sources,
    }
}

async fn compiler_for(pg: &KnowledgePg) -> ProjectWikiCompiler {
    ProjectWikiCompiler::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("compiler handle"),
    ))
}

async fn checker_for(pg: &KnowledgePg) -> WikiDriftChecker {
    WikiDriftChecker::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("checker handle"),
    ))
}

/// Edit the real target source (append a probe symbol) and re-index it.
async fn edit_and_reindex(seeded: &Seeded) -> (String, String) {
    let mut text = core_src(EDIT_TARGET);
    text.push_str("\n/// MT-242 drift probe.\npub fn wiki_drift_probe_symbol() -> u32 { 42 }\n");
    let source_id = seeded
        .engine
        .register_code_source(
            &seeded.workspace_id,
            Some(&seeded.root_id),
            EDIT_TARGET,
            &text,
        )
        .await
        .expect("re-register edited source");
    assert_eq!(
        &source_id, &seeded.sources[EDIT_TARGET],
        "re-registering the same path keeps the stable source id"
    );
    seeded
        .engine
        .index_code_source(
            &index_ctx(),
            &seeded.workspace_id,
            &source_id,
            EDIT_TARGET,
            &text,
            None,
        )
        .await
        .expect("re-index edited source");
    let new_hash = format!("{:x}", Sha256::digest(text.as_bytes()));
    (source_id, new_hash)
}

/// Pages whose stamps cite the source directly OR through one of its
/// entities — the EXPECTED stale set, derived independently from the stamps.
fn expected_affected_titles(
    pages: &[handshake_core::storage::knowledge::KnowledgeWikiProjection],
    source_id: &str,
) -> std::collections::BTreeSet<String> {
    pages
        .iter()
        .filter(|page| {
            WikiCompileStamp::from_value(page.compile_stamp.as_ref())
                .map(|stamp| {
                    stamp.cited_sources.iter().any(|c| {
                        (c.kind == CitedSourceKind::Source && c.id == source_id)
                            || (c.kind == CitedSourceKind::Entity
                                && c.source_id.as_deref() == Some(source_id))
                    })
                })
                .unwrap_or(false)
        })
        .map(|page| page.title.clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_stamps_record_ledger_version_and_cited_source_hashes() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg).await;
    let outcome = compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");

    let current_ledger = compiler
        .db()
        .current_event_ledger_version()
        .await
        .expect("ledger version");
    for page in &outcome.pages {
        let stamp = WikiCompileStamp::from_value(page.compile_stamp.as_ref())
            .unwrap_or_else(|| panic!("page '{}' must be stamped", page.title));
        assert_eq!(stamp.stamp_version, "wiki_stamp_v1");
        assert_eq!(stamp.compiler_version, "project_wiki_compiler_v1");
        assert!(
            stamp.ledger_version > 0 && stamp.ledger_version <= current_ledger,
            "stamp ledger_version {} within (0, {current_ledger}]",
            stamp.ledger_version
        );
        if page.page_type.as_deref() == Some("index") {
            assert!(
                stamp.cited_sources.is_empty(),
                "the catalog page derives from the page set, not authority sources"
            );
        } else {
            assert!(
                !stamp.cited_sources.is_empty(),
                "page '{}' carries its exact cited-source set",
                page.title
            );
        }
        for cited in &stamp.cited_sources {
            assert_eq!(cited.content_hash.len(), 64);
            assert!(cited.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_source_edit_flags_exactly_the_citing_pages_with_reasons() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg).await;
    let outcome = compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");

    // The EXPECTED stale set, derived independently from the stamps.
    let expected = expected_affected_titles(&outcome.pages, &seeded.sources[EDIT_TARGET]);
    assert!(
        expected.contains("module: src/knowledge_code_index"),
        "the module page cites the edit target"
    );
    assert!(
        expected.contains("concepts: src/knowledge_code_index"),
        "the concept page cites the edit target"
    );
    assert!(
        !expected.contains("module: src/knowledge_wiki"),
        "the other module does not cite the edit target"
    );

    // Edit + re-index the REAL source.
    let (source_id, new_source_hash) = edit_and_reindex(&seeded).await;

    // Drift check: exactly the citing pages flag stale, with concrete reasons.
    let checker = checker_for(&pg).await;
    let report = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, true)
        .await
        .expect("drift check");
    let stale_titles: std::collections::BTreeSet<String> = report
        .pages
        .iter()
        .filter(|d| matches!(d.verdict, WikiStalenessVerdict::Stale { .. }))
        .map(|d| d.title.clone())
        .collect();
    assert_eq!(
        stale_titles, expected,
        "drift flags EXACTLY the pages citing the edited source (set equality)"
    );
    assert_eq!(report.stale_pages, expected.len());
    assert_eq!(report.unstamped_pages, 0);

    // Concrete reasons: the source citation names stamped vs CURRENT hash.
    let module_drift = report
        .pages
        .iter()
        .find(|d| d.title == "module: src/knowledge_code_index")
        .expect("module page drift entry");
    let WikiStalenessVerdict::Stale {
        reasons,
        stamp_ledger_version,
        current_ledger_version,
    } = &module_drift.verdict
    else {
        panic!("module page must be stale");
    };
    assert!(stamp_ledger_version < current_ledger_version, "version delta visible");
    let source_reason = reasons
        .iter()
        .find(|r| r.kind == CitedSourceKind::Source && r.id == source_id)
        .expect("reason names the changed source");
    assert_eq!(
        source_reason.current_content_hash.as_deref(),
        Some(new_source_hash.as_str()),
        "reason carries the source's CURRENT content hash"
    );
    assert_ne!(
        source_reason.stamped_content_hash, new_source_hash,
        "stamped hash differs from current (that is WHY the page is stale)"
    );
    // Entity-level reasons too: the symbols of the edited file moved.
    assert!(
        reasons.iter().any(|r| r.kind == CitedSourceKind::Entity),
        "entity citations of the edited file also flag"
    );

    // Persisted marks: the stale pages are durably marked.
    let pages_after = compiler
        .db()
        .list_knowledge_wiki_pages(&seeded.workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages");
    for page in &pages_after {
        if expected.contains(&page.title) {
            assert_eq!(
                page.rebuild_status,
                KnowledgeRebuildStatus::Stale,
                "drifted page '{}' is durably marked stale",
                page.title
            );
        }
    }

    // The drift run left its staleness-verdict receipt (LM-PWIKI-012).
    let receipt_id = report.receipt_event_id.expect("drift receipt id");
    let mut conn = pg.raw_connection().await;
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT payload ->> 'kind' FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&receipt_id)
    .fetch_optional(&mut conn)
    .await
    .expect("ledger query");
    assert_eq!(kind.as_deref(), Some("wiki_drift_check"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_no_change_recompile_yields_zero_stale() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg).await;
    compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");

    let checker = checker_for(&pg).await;
    let first = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, true)
        .await
        .expect("drift check 1");
    assert_eq!(first.stale_pages, 0, "freshly compiled wiki has zero stale pages");

    // Recompile with NO source change…
    compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("recompile");
    // …and the negative gate holds: zero stale, zero unstamped, all fresh.
    let second = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, true)
        .await
        .expect("drift check 2");
    assert_eq!(second.stale_pages, 0, "no-change recompile yields zero stale pages");
    assert_eq!(second.unstamped_pages, 0);
    assert_eq!(second.fresh_pages, second.pages.len());
}

// ---------------------------------------------------------------------------
// Route-level proof: the verdict is attached on EVERY page-serve path.
// ---------------------------------------------------------------------------

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
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool");
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("wiki-drift-api-test".to_string(), 4096),
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
    let app = handshake_core::api::loom::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("loom api server");
    });
    (format!("http://{addr}"), server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_verdict_attached_on_every_serve_path_fail_closed() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg).await;
    let outcome = compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");
    let ws = seeded.workspace_id.clone();

    // A legacy UNSTAMPED row (pre-0300 shape, written through the old upsert).
    let legacy = pg
        .db
        .upsert_knowledge_wiki_projection(NewKnowledgeWikiProjection {
            workspace_id: ws.clone(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: "legacy unstamped topic".to_string(),
            source_records: json!([]),
            rendered_content: "# legacy\n".to_string(),
            staleness_hash: format!("{:x}", Sha256::digest(b"legacy")),
        })
        .await
        .expect("legacy unstamped row");

    let state = app_state_for(&pg.schema_url).await;
    let (base, _server) = start_server(state).await;
    let http = reqwest::Client::new();

    // ---- list serve path: EVERY page carries a verdict ----------------------
    let list: Value = http
        .get(format!("{base}/workspaces/{ws}/loom/wiki"))
        .send()
        .await
        .expect("list send")
        .json()
        .await
        .expect("list json");
    let pages = list["pages"].as_array().expect("pages array");
    assert!(
        pages.len() >= outcome.pages.len(),
        "list serves the compiled wiki ({} >= {})",
        pages.len(),
        outcome.pages.len()
    );
    for page in pages {
        let state_label = page["staleness_verdict"]["state"]
            .as_str()
            .unwrap_or_else(|| panic!("page served WITHOUT a verdict: {page}"));
        assert!(
            ["fresh", "stale", "unstamped"].contains(&state_label),
            "machine-readable verdict state, got {state_label}"
        );
    }
    // The unstamped legacy row is NEVER fresh.
    let legacy_row = pages
        .iter()
        .find(|p| p["title"] == "legacy unstamped topic")
        .expect("legacy row served in list");
    assert_eq!(
        legacy_row["staleness_verdict"]["state"], "unstamped",
        "unstamped page must not read as fresh (LM-PWIKI-008)"
    );

    // ---- single-page serve path ---------------------------------------------
    let module_page = outcome
        .pages
        .iter()
        .find(|p| p.title == "module: src/knowledge_code_index")
        .expect("module page");
    let single: Value = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}",
            module_page.projection_id
        ))
        .send()
        .await
        .expect("get send")
        .json()
        .await
        .expect("get json");
    assert_eq!(single["staleness_verdict"]["state"], "fresh");
    assert_eq!(single["page_type"], "module");

    // ---- stale endpoint (verdict + derived bool) ------------------------------
    let stale: Value = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}/stale",
            module_page.projection_id
        ))
        .send()
        .await
        .expect("stale send")
        .json()
        .await
        .expect("stale json");
    assert_eq!(stale["stale"], false);
    assert_eq!(stale["verdict"]["state"], "fresh");

    // ---- edit the real source -> the SERVED verdict flips to stale -----------
    let (source_id, _new_hash) = edit_and_reindex(&seeded).await;
    let single_after: Value = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}",
            module_page.projection_id
        ))
        .send()
        .await
        .expect("get-after send")
        .json()
        .await
        .expect("get-after json");
    assert_eq!(single_after["staleness_verdict"]["state"], "stale");
    let reasons = single_after["staleness_verdict"]["reasons"]
        .as_array()
        .expect("stale reasons attached");
    assert!(
        reasons
            .iter()
            .any(|r| r["kind"] == "source" && r["id"] == source_id.as_str()),
        "served stale reason names the changed source"
    );

    // ---- legacy unstamped single serve + stale endpoint ------------------------
    let legacy_single: Value = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}",
            legacy.projection_id
        ))
        .send()
        .await
        .expect("legacy get send")
        .json()
        .await
        .expect("legacy get json");
    assert_eq!(legacy_single["staleness_verdict"]["state"], "unstamped");
    let legacy_stale: Value = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}/stale",
            legacy.projection_id
        ))
        .send()
        .await
        .expect("legacy stale send")
        .json()
        .await
        .expect("legacy stale json");
    assert_eq!(
        legacy_stale["stale"], true,
        "unstamped pages are fail-closed stale, never fresh"
    );

    // ---- compile serve path (POST returns the page WITH its verdict) ----------
    let ctx = WriteContext::human(None);
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some("wiki drift api test block".to_string());
    let block = pg
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Drift API note".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived,
            },
        )
        .await
        .expect("create block");
    let compiled: Value = http
        .post(format!("{base}/workspaces/{ws}/loom/wiki"))
        .json(&json!({"title": "Drift API topic", "block_ids": [block.block_id]}))
        .send()
        .await
        .expect("compile send")
        .json()
        .await
        .expect("compile json");
    assert_eq!(
        compiled["staleness_verdict"]["state"], "fresh",
        "the compile serve path attaches the verdict too"
    );
    assert!(
        compiled["compile_stamp"]["cited_sources"]
            .as_array()
            .map(|c| !c.is_empty())
            .unwrap_or(false),
        "the MT-184 loom compile path stamps its cited blocks (ship-together upgrade)"
    );
}
