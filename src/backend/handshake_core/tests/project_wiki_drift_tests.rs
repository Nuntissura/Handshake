//! WP-KERNEL-009 MT-242 WikiProjectionDriftAndStaleness — embedded SurrealDB +
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

mod user_manual_support;

// WP-KERNEL-012 MT-109 / LM-RLS-002: the Loom wiki route proof runs as an authenticated record
// user (persisted account session + live native-MCP channel binding) in an Owner-created
// workspace.
#[path = "account_session_support/mod.rs"]
mod account_session_support;

use sha2::{Digest, Sha256};
use std::sync::Arc;

use account_session_support::AccountFixture;
use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_wiki::compiler::{
    ProjectWikiCompiler, WikiBootstrapOptions, WikiCompileContext,
};
use handshake_core::knowledge_wiki::drift::WikiDriftChecker;
use handshake_core::knowledge_wiki::{CitedSourceKind, WikiCompileStamp, WikiStalenessVerdict};
use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgeProjectionKind, KnowledgeRebuildStatus,
    KnowledgeRootKind, KnowledgeStore, NewKnowledgeSourceRoot, NewKnowledgeWikiProjection,
};
use handshake_core::storage::surreal::resource_authority::{
    ProvisionedIdentity, ResourceAction, ResourceGrantSpec, ResourceKind,
};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::Database;
use serde_json::{json, Value};
use user_manual_support::{app_state_for, manual_test_backend, start_server, ManualTestBackend};
use uuid::Uuid;

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

async fn seed_workspace(backend: &ManualTestBackend) -> Seeded {
    let workspace_id = backend.create_workspace().await;
    seed_workspace_in(backend, workspace_id).await
}

/// Seed the real sources into an existing workspace (an Owner-created one for route proofs).
async fn seed_workspace_in(backend: &ManualTestBackend, workspace_id: String) -> Seeded {
    let engine = CodeIndexEngine::new(Arc::new(backend.db.clone()));
    let root_id = backend
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

fn compiler_for(backend: &ManualTestBackend) -> ProjectWikiCompiler {
    ProjectWikiCompiler::new(Arc::new(backend.db.clone()))
}

fn checker_for(backend: &ManualTestBackend) -> WikiDriftChecker {
    WikiDriftChecker::new(Arc::new(backend.db.clone()))
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
    let pg = manual_test_backend().await.expect("embedded test backend");
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg);
    let outcome = compiler
        .bootstrap(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiBootstrapOptions::default(),
        )
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
    let pg = manual_test_backend().await.expect("embedded test backend");
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg);
    let outcome = compiler
        .bootstrap(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiBootstrapOptions::default(),
        )
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
    let checker = checker_for(&pg);
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
    assert!(
        stamp_ledger_version < current_ledger_version,
        "version delta visible"
    );
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
    let kind = pg
        .db
        .list_kernel_events_for_aggregate("knowledge_wiki", &seeded.workspace_id)
        .await
        .expect("ledger query")
        .into_iter()
        .find(|event| event.event_id == receipt_id)
        .and_then(|event| {
            event
                .payload
                .get("kind")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    assert_eq!(kind.as_deref(), Some("wiki_drift_check"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_no_change_recompile_yields_zero_stale() {
    let pg = manual_test_backend().await.expect("embedded test backend");
    let seeded = seed_workspace(&pg).await;
    let compiler = compiler_for(&pg);
    compiler
        .bootstrap(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiBootstrapOptions::default(),
        )
        .await
        .expect("bootstrap");

    let checker = checker_for(&pg);
    let first = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, true)
        .await
        .expect("drift check 1");
    assert_eq!(
        first.stale_pages, 0,
        "freshly compiled wiki has zero stale pages"
    );

    // Recompile with NO source change…
    compiler
        .bootstrap(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiBootstrapOptions::default(),
        )
        .await
        .expect("recompile");
    // …and the negative gate holds: zero stale, zero unstamped, all fresh.
    let second = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, true)
        .await
        .expect("drift check 2");
    assert_eq!(
        second.stale_pages, 0,
        "no-change recompile yields zero stale pages"
    );
    assert_eq!(second.unstamped_pages, 0);
    assert_eq!(second.fresh_pages, second.pages.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt242_verdict_attached_on_every_serve_path_fail_closed() {
    eprintln!("MT242 serve backend begin");
    let pg = manual_test_backend().await.expect("embedded test backend");
    eprintln!("MT242 serve backend ready");
    let account = AccountFixture::install(pg.db.storage()).await;
    eprintln!("MT242 serve account ready");
    let owned_workspace_id = account.create_workspace(&app_state_for(&pg.db).await).await;
    eprintln!("MT242 serve workspace ready");
    let seeded = seed_workspace_in(&pg, owned_workspace_id).await;
    eprintln!("MT242 serve sources ready");
    let compiler = compiler_for(&pg);
    let outcome = compiler
        .bootstrap(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiBootstrapOptions::default(),
        )
        .await
        .expect("bootstrap");
    eprintln!("MT242 serve bootstrap ready");
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
    eprintln!("MT242 serve legacy ready");

    let state = app_state_for(&pg.db).await;
    let (base, _server) = start_server(handshake_core::api::loom::routes(state)).await;
    eprintln!("MT242 serve server ready");
    let http = account.client();

    // Root-indexed fixtures have no source grants. A workspace grant alone must
    // not make their unavailable hashes read as fresh on the record-user route.
    let module_page = outcome
        .pages
        .iter()
        .find(|p| p.title == "module: src/knowledge_code_index")
        .expect("module page");
    eprintln!("MT242 serve ungranted GET begin");
    let ungranted_response = http
        .get(format!(
            "{base}/workspaces/{ws}/loom/wiki/{}",
            module_page.projection_id
        ))
        .send()
        .await
        .expect("ungranted page send");
    eprintln!("MT242 serve ungranted GET end");
    assert_eq!(ungranted_response.status(), 200);
    let ungranted: Value = ungranted_response
        .json()
        .await
        .expect("ungranted page json");
    assert_eq!(ungranted["staleness_verdict"]["state"], "stale");
    assert!(
        ungranted["staleness_verdict"]["reasons"]
            .as_array()
            .expect("unavailable-source reasons")
            .iter()
            .any(|reason| reason["kind"] == "source"
                && reason["id"] == seeded.sources[EDIT_TARGET]
                && reason["change"] == "source_deleted"
                && reason["current_content_hash"].is_null()),
        "ungranted source must remain unavailable: {ungranted}"
    );

    // Provision only exact read authority for the three seeded sources, using
    // the product registry/grant API; keep record-user evaluation unchanged.
    let identity = ProvisionedIdentity {
        account_id: account.account_id.clone(),
        principal_id: account.principal_id.clone(),
        access_space_id: account.access_space_id.clone(),
    };
    let storage = pg.db.storage();
    eprintln!("MT242 serve workspace resource begin");
    let workspace_resource = storage
        .register_workspace_resource(&identity, &ws)
        .await
        .expect("existing owner workspace resource");
    eprintln!("MT242 serve workspace resource end");
    for source_id in seeded.sources.values() {
        eprintln!("MT242 serve source register begin");
        let resource = storage
            .register_protected_resource(
                &identity,
                ResourceKind::KnowledgeSource,
                source_id,
                Some(&workspace_resource.resource_id),
                "private",
            )
            .await
            .expect("register exact seeded source resource");
        eprintln!("MT242 serve source register end");
        eprintln!("MT242 serve source grant begin");
        storage
            .grant_resource(
                &identity.account_id,
                &identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: identity.principal_id.clone(),
                    resource_id: resource.resource_id,
                    actions: vec![ResourceAction::Read],
                    capability_ids: vec!["memory.read".to_string()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await
            .expect("grant owner exact source read");
        eprintln!("MT242 serve source grant end");
    }

    // ---- list serve path: EVERY page carries a verdict ----------------------
    eprintln!("MT242 serve list GET begin");
    let list: Value = http
        .get(format!("{base}/workspaces/{ws}/loom/wiki"))
        .send()
        .await
        .expect("list send")
        .json()
        .await
        .expect("list json");
    eprintln!("MT242 serve list GET end");
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
    eprintln!("MT242 serve single GET begin");
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
    eprintln!("MT242 serve single GET end");
    assert_eq!(single["staleness_verdict"]["state"], "fresh", "{single}");
    assert_eq!(single["page_type"], "module");

    // ---- stale endpoint (verdict + derived bool) ------------------------------
    eprintln!("MT242 serve stale GET begin");
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
    eprintln!("MT242 serve stale GET end");
    assert_eq!(stale["stale"], false);
    assert_eq!(stale["verdict"]["state"], "fresh");

    // ---- edit the real source -> the SERVED verdict flips to stale -----------
    eprintln!("MT242 serve reindex begin");
    let (source_id, _new_hash) = edit_and_reindex(&seeded).await;
    eprintln!("MT242 serve reindex end");
    eprintln!("MT242 serve edited GET begin");
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
    eprintln!("MT242 serve edited GET end");
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
    eprintln!("MT242 serve legacy GET begin");
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
    eprintln!("MT242 serve legacy GET end");
    assert_eq!(legacy_single["staleness_verdict"]["state"], "unstamped");
    eprintln!("MT242 serve legacy stale GET begin");
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
    eprintln!("MT242 serve legacy stale GET end");
    assert_eq!(
        legacy_stale["stale"], true,
        "unstamped pages are fail-closed stale, never fresh"
    );

    // ---- compile serve path (POST returns the page WITH its verdict) ----------
    // MT-109 C3: the compile route runs as the account record user, so its cited block is created
    // by the account through the product route (a root-created block has no account grant).
    eprintln!("MT242 serve block POST begin");
    let block_json: Value = http
        .post(format!("{base}/workspaces/{ws}/loom/blocks"))
        .json(&json!({"content_type": "note", "title": "Drift API note wiki drift api test block"}))
        .send()
        .await
        .expect("account block send")
        .json()
        .await
        .expect("account block json");
    eprintln!("MT242 serve block POST end");
    let block_id = block_json["block_id"]
        .as_str()
        .expect("account block id")
        .to_owned();
    eprintln!("MT242 serve compile POST begin");
    let compiled: Value = http
        .post(format!("{base}/workspaces/{ws}/loom/wiki"))
        .json(&json!({"title": "Drift API topic", "block_ids": [block_id]}))
        .send()
        .await
        .expect("compile send")
        .json()
        .await
        .expect("compile json");
    eprintln!("MT242 serve compile POST end");
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
