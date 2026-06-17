//! WP-KERNEL-009 MT-241 ProjectWikiBootstrapCompiler — REAL PostgreSQL proof.
//!
//! Bootstraps the project wiki over REAL handshake_core sources (the files are
//! read from this crate's own `src/` tree at test time — "bootstrap over
//! handshake_core itself") indexed through the real `CodeIndexEngine`, then
//! proves the LM-PWIKI-001..005 acceptance surface:
//!   * typed module/concept/entity/decision pages with REAL citations
//!     (entity/span/source ids + content hashes) — no placeholder pages;
//!   * the index/catalog page lists every generated page;
//!   * EventLedger carries the compile receipts;
//!   * deleting ALL generated pages leaves authority BYTE-IDENTICAL (the
//!     operator hard requirement / negative test);
//!   * recompile is idempotent (stable ids, identical rendered content, zero
//!     stale);
//!   * token-aware clustering splits oversized clusters LOUDLY.
//!
//! Proof-path contract: real Handshake-managed PostgreSQL only (no SQLite, no
//! mocks); SKIPs loudly when PG binaries are absent.

mod knowledge_pg_support;

use std::sync::Arc;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_wiki::compiler::{
    ProjectWikiCompiler, WikiBootstrapOptions, WikiCompileContext, WIKI_INDEX_PAGE_TITLE,
};
use handshake_core::knowledge_wiki::{CitedSourceKind, WikiCompileStamp};
use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeIndexingEligibility, KnowledgeRootKind, KnowledgeStore,
    NewKnowledgeEntity, NewKnowledgeRichDocument, NewKnowledgeSourceRoot,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use uuid::Uuid;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-241 project wiki bootstrap proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

/// REAL handshake_core sources indexed for the bootstrap (read from this
/// crate's own src tree — never inline fixtures).
const CORE_FILES: [&str; 4] = [
    "src/knowledge_code_index/mod.rs",
    "src/knowledge_code_index/staleness.rs",
    "src/knowledge_wiki/mod.rs",
    "src/knowledge_wiki/drift.rs",
];

fn core_src(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read real handshake_core source {relative_path}: {err}"))
}

fn wiki_ctx() -> WikiCompileContext {
    WikiCompileContext {
        actor: KernelActor::System("project-wiki-test".to_string()),
        kernel_task_run_id: "KTR-project-wiki-test".to_string(),
        session_run_id: "SR-project-wiki-test".to_string(),
        correlation_id: Some("CORR-project-wiki-test".to_string()),
    }
}

fn index_ctx() -> CodeIndexContext {
    CodeIndexContext {
        actor: KernelActor::System("project-wiki-index".to_string()),
        kernel_task_run_id: "KTR-project-wiki-index".to_string(),
        session_run_id: "SR-project-wiki-index".to_string(),
        correlation_id: None,
    }
}

/// Index the real handshake_core files + create one rich document (entity
/// page input) and one work-packet entity (decision page input). Returns
/// (workspace_id, engine, root_id).
async fn seed_workspace(pg: &KnowledgePg) -> (String, CodeIndexEngine, String) {
    let workspace_id = pg.create_workspace().await;
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect engine handle");
    let engine = CodeIndexEngine::new(Arc::new(db));
    let root = pg
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
    for relative_path in CORE_FILES {
        let text = core_src(relative_path);
        let source_id = engine
            .register_code_source(&workspace_id, Some(&root), relative_path, &text)
            .await
            .expect("register real source");
        engine
            .index_code_source(&ctx, &workspace_id, &source_id, relative_path, &text, None)
            .await
            .expect("index real source");
    }

    // Entity-page input: a real rich document.
    pg.db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: workspace_id.clone(),
            document_id: None,
            title: "Wiki compile design note".to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: json!({
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": "The compiled wiki is a regenerable projection over authority."
                    }]
                }]
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
        .expect("create rich document");

    // Decision-page input: a real work-packet entity.
    pg.db
        .upsert_knowledge_entity(NewKnowledgeEntity {
            workspace_id: workspace_id.clone(),
            entity_kind: KnowledgeEntityKind::WorkPacket,
            entity_key: "wp:WP-KERNEL-009".to_string(),
            display_name: "WP-KERNEL-009 Unified Work Surface".to_string(),
            detection_provenance: json!({"extractor": "project_wiki_test"}),
            primary_source_id: None,
            detected_in_run: None,
            evidence_span_ids: Vec::new(),
        })
        .await
        .expect("create work packet entity");

    (workspace_id, engine, root)
}

/// md5 fingerprint over every AUTHORITY table's full row set (byte-identity
/// proof surface). The wiki projection table is deliberately NOT included —
/// it is the projection under test.
async fn authority_fingerprint(pg: &KnowledgePg) -> String {
    let mut conn = pg.raw_connection().await;
    let tables = [
        "workspaces",
        "knowledge_sources",
        "knowledge_entities",
        "knowledge_spans",
        "knowledge_entity_spans",
        "knowledge_edges",
        "knowledge_code_files",
        "knowledge_rich_documents",
        "loom_blocks",
        // Operator annotations are AUTHORITY rows (MT-185) and must survive
        // projection deletion (soft reference since migration 0301).
        "loom_wiki_overlays",
        "kernel_event_ledger",
    ];
    let mut out = String::new();
    for table in tables {
        let sql = format!(
            "SELECT COALESCE(md5(string_agg(t::text, '|' ORDER BY t::text)), 'empty') FROM {table} t"
        );
        let hash: String = sqlx::query_scalar(&sql)
            .fetch_one(&mut conn)
            .await
            .unwrap_or_else(|err| panic!("fingerprint {table}: {err}"));
        out.push_str(table);
        out.push('=');
        out.push_str(&hash);
        out.push(';');
    }
    out
}

async fn ledger_event_kind(pg: &KnowledgePg, event_id: &str) -> Option<String> {
    let mut conn = pg.raw_connection().await;
    sqlx::query_scalar::<_, String>(
        "SELECT payload ->> 'kind' FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_optional(&mut conn)
    .await
    .expect("query ledger event")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt241_bootstrap_over_handshake_core_produces_cited_typed_pages() {
    let pg = pg_or_skip!();
    let (workspace_id, _engine, _root) = seed_workspace(&pg).await;
    let compiler = ProjectWikiCompiler::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("compiler handle"),
    ));

    let outcome = compiler
        .bootstrap(&wiki_ctx(), &workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap compile");

    // Typed pages over BOTH real source directories + concept + entity +
    // decision + index.
    assert!(
        outcome.module_pages >= 2,
        "expected module pages for src/knowledge_code_index and src/knowledge_wiki, got {}",
        outcome.module_pages
    );
    assert!(
        outcome.concept_pages >= 1,
        "doc passages must produce concept pages"
    );
    assert_eq!(
        outcome.entity_pages, 1,
        "one rich document -> one entity page"
    );
    assert_eq!(
        outcome.decision_pages, 1,
        "one WP entity -> one decision page"
    );

    let titles: Vec<&str> = outcome.pages.iter().map(|p| p.title.as_str()).collect();
    assert!(
        titles.contains(&"module: src/knowledge_code_index"),
        "module page for src/knowledge_code_index missing: {titles:?}"
    );
    assert!(
        titles.contains(&"module: src/knowledge_wiki"),
        "module page for src/knowledge_wiki missing: {titles:?}"
    );
    assert!(
        titles.contains(&WIKI_INDEX_PAGE_TITLE),
        "index page missing"
    );

    // ---- citation proof on a real module page -----------------------------
    let module_page = outcome
        .pages
        .iter()
        .find(|p| p.title == "module: src/knowledge_code_index")
        .expect("module page");
    assert_eq!(module_page.page_type.as_deref(), Some("module"));
    let stamp = WikiCompileStamp::from_value(module_page.compile_stamp.as_ref())
        .expect("module page MUST be stamped (ship-together guard)");
    assert!(
        stamp.ledger_version > 0,
        "stamp carries the EventLedger version"
    );
    assert!(
        !stamp.cited_sources.is_empty(),
        "no placeholder pages: citations required"
    );
    for cited in &stamp.cited_sources {
        assert_eq!(
            cited.content_hash.len(),
            64,
            "citation hash must be sha256 hex"
        );
        assert!(
            cited.content_hash.chars().all(|c| c.is_ascii_hexdigit()),
            "citation hash must be hex"
        );
    }

    // The acceptance spot-check: the page cites a symbol that exists in
    // knowledge_code_nav for that module (`evaluate_staleness` of
    // staleness.rs).
    let symbols = pg
        .db
        .lookup_code_symbols(
            &workspace_id,
            Some("evaluate_staleness"),
            Some("src/knowledge_code_index/staleness.rs"),
            None,
            10,
        )
        .await
        .expect("nav lookup");
    let nav_symbol = symbols
        .iter()
        .find(|s| s.display_name == "evaluate_staleness")
        .expect("evaluate_staleness exists in knowledge_code_nav");
    assert!(
        stamp
            .cited_sources
            .iter()
            .any(|c| c.kind == CitedSourceKind::Entity && c.id == nav_symbol.entity_id),
        "module page must cite the evaluate_staleness symbol entity ({})",
        nav_symbol.entity_id
    );
    assert!(
        module_page.rendered_content.contains("evaluate_staleness"),
        "rendered module page lists the symbol"
    );
    assert!(
        module_page.rendered_content.contains("cite: entity:KEN-"),
        "rendered module page carries precise entity citations"
    );

    // Every cited entity resolves in authority; spans exist (id + hash —
    // never loose file paths).
    let mut conn = pg.raw_connection().await;
    for cited in &stamp.cited_sources {
        match cited.kind {
            CitedSourceKind::Entity => {
                let entity = pg
                    .db
                    .get_knowledge_entity(&cited.id)
                    .await
                    .expect("entity query")
                    .expect("cited entity exists in authority");
                assert_eq!(entity.workspace_id, workspace_id);
                if let Some(span_id) = &cited.span_id {
                    let span_count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM knowledge_spans WHERE span_id = $1",
                    )
                    .bind(span_id)
                    .fetch_one(&mut conn)
                    .await
                    .expect("span query");
                    assert_eq!(span_count, 1, "cited span {span_id} exists");
                }
            }
            CitedSourceKind::Source => {
                let source_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM knowledge_sources WHERE source_id = $1 AND content_hash = $2",
                )
                .bind(&cited.id)
                .bind(&cited.content_hash)
                .fetch_one(&mut conn)
                .await
                .expect("source query");
                assert_eq!(
                    source_count, 1,
                    "cited source {} resolves with its hash",
                    cited.id
                );
            }
            _ => {}
        }
    }

    // ---- index/catalog page lists every generated page ---------------------
    let index_page = outcome
        .pages
        .iter()
        .find(|p| p.title == WIKI_INDEX_PAGE_TITLE)
        .expect("index page");
    assert_eq!(index_page.page_type.as_deref(), Some("index"));
    for page in &outcome.pages {
        if page.title == WIKI_INDEX_PAGE_TITLE {
            continue;
        }
        assert!(
            index_page
                .rendered_content
                .contains(&format!("[[{}]]", page.title)),
            "index page lists '{}'",
            page.title
        );
    }
    let index_links = index_page.page_links.as_array().expect("index links");
    assert_eq!(
        index_links.len(),
        outcome.pages.len() - 1,
        "index links every page"
    );
    // Links resolve to projection ids (navigable wiki).
    for link in index_links {
        assert!(
            link.get("projection_id").and_then(|v| v.as_str()).is_some(),
            "index link resolved to a projection id: {link}"
        );
    }

    // ---- EventLedger compile receipts (LM-PWIKI-012) ------------------------
    assert_eq!(
        ledger_event_kind(&pg, &outcome.started_receipt_event_id)
            .await
            .as_deref(),
        Some("wiki_bootstrap_compile_started"),
        "started receipt exists in the EventLedger"
    );
    assert_eq!(
        ledger_event_kind(&pg, &outcome.completed_receipt_event_id)
            .await
            .as_deref(),
        Some("wiki_bootstrap_compile_completed"),
        "completed receipt exists in the EventLedger"
    );
    // Every page row references the compile receipt.
    for page in &outcome.pages {
        assert_eq!(
            page.rebuild_receipt_event_id.as_deref(),
            Some(outcome.started_receipt_event_id.as_str()),
            "page '{}' references the compile receipt",
            page.title
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt241_delete_all_generated_pages_leaves_authority_byte_identical() {
    let pg = pg_or_skip!();
    let (workspace_id, _engine, _root) = seed_workspace(&pg).await;
    let compiler = ProjectWikiCompiler::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("compiler handle"),
    ));
    let outcome = compiler
        .bootstrap(&wiki_ctx(), &workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("bootstrap compile");
    assert!(!outcome.pages.is_empty());

    // An operator overlay on a generated page is AUTHORITY (MT-185) and must
    // survive the deletion of the page it annotates.
    let annotated = &outcome.pages[0];
    let overlay = pg
        .db
        .add_loom_wiki_overlay(
            &workspace_id,
            &annotated.projection_id,
            "operator note: byte-identity proof anchor",
            None,
        )
        .await
        .expect("add overlay");

    // Snapshot EVERY authority table AFTER the compile (incl. the overlay).
    let before = authority_fingerprint(&pg).await;

    // Delete every generated page (the projection-delete path).
    for page in &outcome.pages {
        pg.db
            .delete_loom_wiki_projection(&workspace_id, &page.projection_id)
            .await
            .expect("delete generated page");
    }

    // The wiki is gone…
    let remaining = compiler
        .db()
        .list_knowledge_wiki_pages(&workspace_id, None, true, 2_000, 0)
        .await
        .expect("list pages");
    assert!(remaining.is_empty(), "all generated pages deleted");

    // …and authority is BYTE-IDENTICAL (including the EventLedger: deleting
    // projections appends nothing and mutates nothing; including the operator
    // overlay: annotations survive projection churn).
    let after = authority_fingerprint(&pg).await;
    assert_eq!(
        before, after,
        "authority byte-identical after deleting the whole wiki"
    );
    let overlays = pg
        .db
        .list_loom_wiki_overlays(&workspace_id, &annotated.projection_id)
        .await
        .expect("list overlays after page delete");
    assert!(
        overlays.iter().any(|o| o.overlay_id == overlay.overlay_id),
        "operator overlay (authority) survives deleting the page it annotated"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt241_recompile_is_idempotent() {
    let pg = pg_or_skip!();
    let (workspace_id, _engine, _root) = seed_workspace(&pg).await;
    let compiler = ProjectWikiCompiler::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("compiler handle"),
    ));

    let first = compiler
        .bootstrap(&wiki_ctx(), &workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("first bootstrap");
    let second = compiler
        .bootstrap(&wiki_ctx(), &workspace_id, &WikiBootstrapOptions::default())
        .await
        .expect("second bootstrap");

    // Same page set: stable identity (no duplicates), identical content.
    assert_eq!(
        first.pages.len(),
        second.pages.len(),
        "no duplicate pages on recompile"
    );
    let mut first_by_title: std::collections::BTreeMap<&str, _> =
        first.pages.iter().map(|p| (p.title.as_str(), p)).collect();
    for page in &second.pages {
        let original = first_by_title
            .remove(page.title.as_str())
            .unwrap_or_else(|| panic!("recompiled page '{}' existed before", page.title));
        assert_eq!(
            original.projection_id, page.projection_id,
            "projection id stable across recompile ('{}')",
            page.title
        );
        assert_eq!(
            original.rendered_content, page.rendered_content,
            "rendered content deterministic across recompile ('{}')",
            page.title
        );
        assert_eq!(original.page_type, page.page_type);
    }
    assert!(first_by_title.is_empty(), "no pages vanished on recompile");

    // No-change recompile -> zero stale (the MT-242 negative gate holds from
    // the compiler side too).
    let checker = handshake_core::knowledge_wiki::drift::WikiDriftChecker::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("checker handle"),
    ));
    let report = checker
        .check_workspace(&wiki_ctx(), &workspace_id, false)
        .await
        .expect("drift check");
    assert_eq!(
        report.stale_pages, 0,
        "no-change recompile yields zero stale pages"
    );
    assert_eq!(report.unstamped_pages, 0, "every compiled page is stamped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt241_token_budget_splits_clusters_loudly() {
    let pg = pg_or_skip!();
    let (workspace_id, _engine, _root) = seed_workspace(&pg).await;
    let compiler = ProjectWikiCompiler::new(Arc::new(
        PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("compiler handle"),
    ));

    // A deliberately tiny budget forces token-aware clustering to split the
    // real clusters (and to report single files that exceed the budget alone
    // — LOUDLY, never silently truncated).
    let outcome = compiler
        .bootstrap(
            &wiki_ctx(),
            &workspace_id,
            &WikiBootstrapOptions {
                page_token_budget: 300,
            },
        )
        .await
        .expect("budgeted bootstrap");

    assert!(
        outcome.split_clusters >= 1 || !outcome.oversize_files.is_empty(),
        "tiny budget must split clusters or flag oversize files (got splits={} oversize={:?})",
        outcome.split_clusters,
        outcome.oversize_files
    );
    // Split parts are titled deterministically and cross-link their siblings.
    let part_pages: Vec<_> = outcome
        .pages
        .iter()
        .filter(|p| p.title.contains("(part "))
        .collect();
    if outcome.split_clusters >= 1 {
        assert!(
            !part_pages.is_empty(),
            "split clusters emit '(part N)' pages"
        );
        let part_one = part_pages
            .iter()
            .find(|p| p.title.ends_with("(part 1)"))
            .expect("part 1 exists");
        assert!(
            part_one.rendered_content.contains("(part 2)"),
            "part pages cross-link their siblings"
        );
    }
    // The completed receipt records the split/oversize facts (LOUD).
    let kind = ledger_event_kind(&pg, &outcome.completed_receipt_event_id).await;
    assert_eq!(kind.as_deref(), Some("wiki_bootstrap_compile_completed"));
}
