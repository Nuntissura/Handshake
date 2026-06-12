//! WP-KERNEL-009 MT-243 WikiIncrementalIngestFanOut — REAL PostgreSQL proof.
//!
//! Proves LM-PWIKI-010..012 over a wiki bootstrapped from REAL handshake_core
//! sources:
//!   * one changed source regenerates EXACTLY the pages whose stamps cite it —
//!     set equality proven against an INDEPENDENTLY computed MT-242 drift
//!     stale set, both directions;
//!   * regenerated pages are rebuilt from CURRENT authority (the new probe
//!     symbol appears on the page) and their wikilinks refresh in the same
//!     pass; the index/catalog page refreshes too; EventLedger carries a
//!     receipt per regenerated page;
//!   * fan-out is bounded by the explicit budget — truncation leaves a LOUD
//!     ledger receipt, marks skipped pages stale, and a re-run RESUMES the
//!     remainder;
//!   * re-running a completed fan-out is idempotent: no duplicate pages, no
//!     duplicate links, no duplicate per-page receipts;
//!   * a changed Loom block fans out to the MT-184 topic pages citing it.

mod knowledge_pg_support;

use std::collections::BTreeSet;
use std::sync::Arc;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_wiki::compiler::{
    ProjectWikiCompiler, WikiBootstrapOptions, WikiCompileContext,
};
use handshake_core::knowledge_wiki::drift::WikiDriftChecker;
use handshake_core::knowledge_wiki::fanout::{WikiFanOutEngine, WikiFanOutRequest};
use handshake_core::knowledge_wiki::{CitedSourceKind, WikiStalenessVerdict};
use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgeRebuildStatus, KnowledgeRootKind, KnowledgeStore,
    NewKnowledgeSourceRoot,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, NewLoomBlock, WriteContext,
};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use uuid::Uuid;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-243 wiki fan-out proof: PostgreSQL unavailable");
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

const EDIT_TARGET: &str = "src/knowledge_code_index/staleness.rs";

fn core_src(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read real handshake_core source {relative_path}: {err}"))
}

fn wiki_ctx() -> WikiCompileContext {
    WikiCompileContext {
        actor: KernelActor::System("wiki-fanout-test".to_string()),
        kernel_task_run_id: "KTR-wiki-fanout-test".to_string(),
        session_run_id: "SR-wiki-fanout-test".to_string(),
        correlation_id: None,
    }
}

fn index_ctx() -> CodeIndexContext {
    CodeIndexContext {
        actor: KernelActor::System("wiki-fanout-index".to_string()),
        kernel_task_run_id: "KTR-wiki-fanout-index".to_string(),
        session_run_id: "SR-wiki-fanout-index".to_string(),
        correlation_id: None,
    }
}

struct Seeded {
    workspace_id: String,
    engine: CodeIndexEngine,
    root_id: String,
    edit_target_source_id: String,
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
    let mut edit_target_source_id = String::new();
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
        if relative_path == EDIT_TARGET {
            edit_target_source_id = source_id;
        }
    }
    Seeded {
        workspace_id,
        engine,
        root_id,
        edit_target_source_id,
    }
}

async fn pg_handle(pg: &KnowledgePg) -> Arc<PostgresDatabase> {
    Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("pg handle"),
    )
}

async fn edit_and_reindex(seeded: &Seeded) {
    let mut text = core_src(EDIT_TARGET);
    text.push_str("\n/// MT-243 fan-out probe.\npub fn wiki_fanout_probe_symbol() -> u32 { 43 }\n");
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
    assert_eq!(source_id, seeded.edit_target_source_id);
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
}

async fn ledger_kind_count(pg: &KnowledgePg, kind: &str) -> i64 {
    let mut conn = pg.raw_connection().await;
    sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE payload ->> 'kind' = $1")
        .bind(kind)
        .fetch_one(&mut conn)
        .await
        .expect("ledger count")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt243_single_source_edit_regenerates_exactly_the_stale_set() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let handle = pg_handle(&pg).await;
    let compiler = ProjectWikiCompiler::new(handle.clone());
    compiler
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");

    edit_and_reindex(&seeded).await;

    // INDEPENDENT MT-242 stale set (drift checker, no persistence).
    let checker = WikiDriftChecker::new(handle.clone());
    let drift_before = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, false)
        .await
        .expect("drift before fan-out");
    let drift_stale: BTreeSet<String> = drift_before
        .pages
        .iter()
        .filter(|d| matches!(d.verdict, WikiStalenessVerdict::Stale { .. }))
        .map(|d| d.title.clone())
        .collect();
    assert!(
        drift_stale.len() >= 2,
        "module + concept pages of the edited dir must be stale, got {drift_stale:?}"
    );

    // Fan-out for THE one changed source.
    let engine = WikiFanOutEngine::new(handle.clone());
    let outcome = engine
        .run(
            &wiki_ctx(),
            &seeded.workspace_id,
            &WikiFanOutRequest::new(CitedSourceKind::Source, seeded.edit_target_source_id.clone()),
        )
        .await
        .expect("fan-out");

    // LM-PWIKI-010 SET EQUALITY, proven against the independent drift result.
    let fanout_stale: BTreeSet<String> =
        outcome.stale_set.iter().map(|p| p.title.clone()).collect();
    let regenerated: BTreeSet<String> =
        outcome.regenerated.iter().map(|p| p.title.clone()).collect();
    assert_eq!(
        fanout_stale, drift_stale,
        "fan-out stale set == MT-242 drift stale set (set equality)"
    );
    assert_eq!(
        regenerated, drift_stale,
        "fan-out regenerated EXACTLY the stale set (set equality)"
    );
    assert!(outcome.truncated.is_empty(), "no truncation within budget");
    assert!(outcome.orphaned.is_empty());
    assert!(outcome.index_refreshed, "index/catalog refreshed in the same pass");

    // Regeneration used CURRENT authority: the probe symbol is ON the page.
    let pages = handle
        .list_knowledge_wiki_pages(&seeded.workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages");
    let module_page = pages
        .iter()
        .find(|p| p.title == "module: src/knowledge_code_index")
        .expect("module page");
    assert!(
        module_page
            .rendered_content
            .contains("wiki_fanout_probe_symbol"),
        "regenerated page reflects the NEW symbol from current authority"
    );
    // Same-pass link refresh: every wikilink resolved to a projection id.
    for title in &regenerated {
        let page = pages.iter().find(|p| &p.title == title).expect("regen page");
        let links = page.page_links.as_array().expect("links array");
        assert!(!links.is_empty(), "regenerated page '{title}' carries wikilinks");
        for link in links {
            assert!(
                link.get("projection_id").and_then(|v| v.as_str()).is_some(),
                "link resolved on regenerated page '{title}': {link}"
            );
        }
    }

    // Per-page EventLedger receipts (LM-PWIKI-012) + page rows reference them.
    let mut conn = pg.raw_connection().await;
    for entry in &outcome.regenerated {
        let kind: Option<String> = sqlx::query_scalar(
            "SELECT payload ->> 'kind' FROM kernel_event_ledger WHERE event_id = $1",
        )
        .bind(&entry.receipt_event_id)
        .fetch_optional(&mut conn)
        .await
        .expect("receipt lookup");
        assert_eq!(
            kind.as_deref(),
            Some("wiki_page_fanout_regenerated"),
            "per-page regeneration receipt exists for '{}'",
            entry.title
        );
        let page = pages
            .iter()
            .find(|p| p.projection_id == entry.projection_id)
            .expect("regen page row");
        assert_eq!(
            page.rebuild_receipt_event_id.as_deref(),
            Some(entry.receipt_event_id.as_str()),
            "page row references its per-page receipt"
        );
    }

    // After the pass: drift reports ZERO stale (the loop is closed).
    let drift_after = checker
        .check_workspace(&wiki_ctx(), &seeded.workspace_id, false)
        .await
        .expect("drift after fan-out");
    assert_eq!(drift_after.stale_pages, 0, "fan-out closed the stale set");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt243_budget_truncation_is_loud_and_resumable() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let handle = pg_handle(&pg).await;
    ProjectWikiCompiler::new(handle.clone())
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");
    edit_and_reindex(&seeded).await;

    let engine = WikiFanOutEngine::new(handle.clone());
    let mut request = WikiFanOutRequest::new(
        CitedSourceKind::Source,
        seeded.edit_target_source_id.clone(),
    );
    request.budget = 1; // force truncation

    let outcome = engine
        .run(&wiki_ctx(), &seeded.workspace_id, &request)
        .await
        .expect("budgeted fan-out");
    assert_eq!(outcome.budget, 1);
    assert_eq!(outcome.regenerated.len(), 1, "budget bounds the pass");
    assert!(
        !outcome.truncated.is_empty(),
        "stale set larger than the budget must truncate"
    );

    // LOUD truncation: the ledger receipt exists and names the skipped pages.
    let truncation_receipt = outcome
        .truncation_receipt_event_id
        .clone()
        .expect("truncation receipt id");
    let mut conn = pg.raw_connection().await;
    let payload: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT payload FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&truncation_receipt)
    .fetch_optional(&mut conn)
    .await
    .expect("truncation receipt lookup");
    let payload = payload.expect("truncation receipt payload");
    assert_eq!(payload["kind"], "wiki_fanout_truncated");
    assert_eq!(
        payload["skipped_total"].as_u64().unwrap() as usize,
        outcome.truncated.len()
    );
    assert!(
        payload["skipped"]
            .as_array()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "skipped pages are named in the receipt"
    );

    // Skipped pages are durably marked stale (visible, never silent).
    let pages = handle
        .list_knowledge_wiki_pages(&seeded.workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages");
    for skipped in &outcome.truncated {
        let page = pages
            .iter()
            .find(|p| p.projection_id == skipped.projection_id)
            .expect("skipped page row");
        assert_eq!(
            page.rebuild_status,
            KnowledgeRebuildStatus::Stale,
            "truncated page '{}' stays visibly stale",
            skipped.title
        );
    }

    // RESUME: a second run picks up exactly the remainder…
    let mut resume = WikiFanOutRequest::new(
        CitedSourceKind::Source,
        seeded.edit_target_source_id.clone(),
    );
    resume.budget = 50;
    let second = engine
        .run(&wiki_ctx(), &seeded.workspace_id, &resume)
        .await
        .expect("resume fan-out");
    let resumed: BTreeSet<String> = second.regenerated.iter().map(|p| p.title.clone()).collect();
    let expected_remainder: BTreeSet<String> =
        outcome.truncated.iter().map(|p| p.title.clone()).collect();
    assert_eq!(resumed, expected_remainder, "re-run resumes exactly the truncated remainder");
    assert!(second.truncated.is_empty());

    // …and a third run finds nothing left.
    let third = engine
        .run(&wiki_ctx(), &seeded.workspace_id, &resume)
        .await
        .expect("third fan-out");
    assert!(third.stale_set.is_empty());
    assert!(third.regenerated.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt243_rerun_is_idempotent_no_duplicates() {
    let pg = pg_or_skip!();
    let seeded = seed_workspace(&pg).await;
    let handle = pg_handle(&pg).await;
    ProjectWikiCompiler::new(handle.clone())
        .bootstrap(&wiki_ctx(), &seeded.workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap");
    edit_and_reindex(&seeded).await;

    let engine = WikiFanOutEngine::new(handle.clone());
    let request = WikiFanOutRequest::new(
        CitedSourceKind::Source,
        seeded.edit_target_source_id.clone(),
    );
    let first = engine
        .run(&wiki_ctx(), &seeded.workspace_id, &request)
        .await
        .expect("first fan-out");
    assert!(first.regenerated.len() >= 2);

    // Snapshot pages + receipt counts after the completed pass.
    let pages_before = handle
        .list_knowledge_wiki_pages(&seeded.workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages");
    let regen_receipts_before = ledger_kind_count(&pg, "wiki_page_fanout_regenerated").await;
    let truncation_receipts_before = ledger_kind_count(&pg, "wiki_fanout_truncated").await;

    // RE-RUN the same fan-out.
    let second = engine
        .run(&wiki_ctx(), &seeded.workspace_id, &request)
        .await
        .expect("second fan-out");
    assert!(second.stale_set.is_empty(), "stamps now match authority — nothing stale");
    assert!(second.regenerated.is_empty(), "no duplicate regeneration");
    assert!(second.truncated.is_empty());
    assert!(!second.index_refreshed, "nothing regenerated -> index untouched");

    // No duplicate pages / links / receipts.
    let pages_after = handle
        .list_knowledge_wiki_pages(&seeded.workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages after");
    assert_eq!(pages_before.len(), pages_after.len(), "no duplicate pages");
    for (before, after) in pages_before.iter().zip(pages_after.iter()) {
        assert_eq!(before.projection_id, after.projection_id);
        assert_eq!(before.rendered_content, after.rendered_content);
        assert_eq!(before.compile_stamp, after.compile_stamp, "stamps untouched by the no-op re-run");
        // Link sets stay duplicate-free.
        let titles: Vec<&str> = after
            .page_links
            .as_array()
            .map(|links| {
                links
                    .iter()
                    .filter_map(|l| l.get("title").and_then(|t| t.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        let unique: BTreeSet<&str> = titles.iter().copied().collect();
        assert_eq!(titles.len(), unique.len(), "no duplicate links on '{}'", after.title);
    }
    assert_eq!(
        ledger_kind_count(&pg, "wiki_page_fanout_regenerated").await,
        regen_receipts_before,
        "no duplicate per-page regeneration receipts"
    );
    assert_eq!(
        ledger_kind_count(&pg, "wiki_fanout_truncated").await,
        truncation_receipts_before,
        "no spurious truncation receipts"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt243_loom_block_change_fans_out_to_topic_pages() {
    let pg = pg_or_skip!();
    let workspace_id = pg.create_workspace().await;
    let handle = pg_handle(&pg).await;
    let ctx = WriteContext::human(None);

    // Two blocks; a loom topic page citing both (the MT-184 path, stamped).
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some("fan-out block body".to_string());
    let block = pg
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("FanOut Alpha".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: derived.clone(),
            },
        )
        .await
        .expect("block a")
        .block_id;
    let other = pg
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("FanOut Beta".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived,
            },
        )
        .await
        .expect("block b")
        .block_id;
    let topic = pg
        .db
        .compile_loom_wiki_projection(&workspace_id, "FanOut Topic", &[block.clone(), other])
        .await
        .expect("compile topic page");

    // Edit ONE cited block.
    pg.db
        .update_loom_block(
            &ctx,
            &workspace_id,
            &block,
            LoomBlockUpdate {
                title: Some("FanOut Alpha Renamed".into()),
                ..Default::default()
            },
        )
        .await
        .expect("rename block");

    // Fan-out for the changed loom block regenerates the topic page.
    let engine = WikiFanOutEngine::new(handle.clone());
    let outcome = engine
        .run(
            &wiki_ctx(),
            &workspace_id,
            &WikiFanOutRequest::new(CitedSourceKind::LoomBlock, block.clone()),
        )
        .await
        .expect("loom block fan-out");
    assert_eq!(outcome.regenerated.len(), 1);
    assert_eq!(outcome.regenerated[0].projection_id, topic.projection_id);

    // The regenerated topic page reflects the rename and is fresh again.
    let refreshed = pg
        .db
        .get_loom_wiki_projection(&workspace_id, &topic.projection_id)
        .await
        .expect("get refreshed topic");
    assert!(refreshed.rendered_content.contains("FanOut Alpha Renamed"));

    // Idempotent re-run: nothing left.
    let second = engine
        .run(
            &wiki_ctx(),
            &workspace_id,
            &WikiFanOutRequest::new(CitedSourceKind::LoomBlock, block),
        )
        .await
        .expect("re-run");
    assert!(second.regenerated.is_empty());
    assert!(second.stale_set.is_empty());
}
