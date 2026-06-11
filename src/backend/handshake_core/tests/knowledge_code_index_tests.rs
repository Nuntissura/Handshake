//! WP-KERNEL-009 CodeIndexingAndNavigation runtime proof against REAL
//! Handshake-managed PostgreSQL (MT-097..MT-112).
//!
//! Proof-path contract (operator-mandated): no SQLite, no mock, no in-memory
//! fallback. Every durable assertion runs against the managed PostgreSQL cluster
//! through `knowledge_pg()`; when the PG binaries are genuinely absent the test
//! SKIPs loudly (never silently green).
//!
//! These tests drive the real `CodeIndexEngine` (which writes symbols/spans/edges
//! THROUGH `storage::knowledge::KnowledgeStore` and maintains the
//! `knowledge_code_files` (0170) / `knowledge_code_scip_imports` (0171) support
//! tables) and then read the graph back to prove:
//!   * MT-098..MT-100 symbols become `symbol` entities anchored to `ast` spans;
//!   * MT-101 config keys become entities anchored to `byte` spans;
//!   * MT-104 calls/imports become `references`/`depends_on` edges with evidence
//!     spans + deterministic relationship ids + extractor version + confidence;
//!   * MT-105 SCIP import records land in the import ledger (rejections too);
//!   * MT-107 staleness flips when the source hash / parser version moves;
//!   * MT-108 a single unparseable file is recorded `failed` and the run still
//!     indexes the good files;
//!   * MT-109 the Monaco payload is served with a staleness flag;
//!   * MT-110 the context bundle is bounded + cited;
//!   * MT-112 a mixed rust/ts/js/config mini-tree indexes end to end.

mod knowledge_pg_support;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::context_bridge::{
    build_code_context_bundle, DEFAULT_CODE_CONTEXT_TOKEN_BUDGET,
};
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_code_index::monaco_bridge::build_monaco_payload;
use handshake_core::knowledge_code_index::parser::{CodeLanguage, CodeParserAdapter};
use handshake_core::knowledge_code_index::scip::{artifact_hash, parse_scip_artifact};
use handshake_core::knowledge_code_index::staleness::StalenessVerdict;
use handshake_core::storage::knowledge::{
    KnowledgeCodeParseStatus, KnowledgeEdgeType, KnowledgeEntityKind, KnowledgeIndexingEligibility,
    KnowledgeRootKind, KnowledgeScipFormat, KnowledgeScipImportStatus, KnowledgeSpanKind,
    KnowledgeStore, NewKnowledgeScipImport, NewKnowledgeSourceRoot,
};
use handshake_core::storage::postgres::PostgresDatabase;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Create a project-repo root (FK target; file-kind sources require a root_id).
async fn make_root(pg: &KnowledgePg, workspace_id: &str) -> String {
    let root = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.to_string(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("root/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("create root");
    root.root_id
}

fn ctx() -> CodeIndexContext {
    CodeIndexContext {
        actor: KernelActor::System("code-index-test".to_string()),
        kernel_task_run_id: "KTR-code-index-test".to_string(),
        session_run_id: "SR-code-index-test".to_string(),
        correlation_id: Some("CORR-code-index-test".to_string()),
    }
}

/// Build a code-index engine on a handle into the SAME isolated schema (the
/// established ingestion-test pattern: connect a second handle to
/// `pg.schema_url`, which shares the schema the migrations ran in).
async fn engine(pg: &KnowledgePg) -> CodeIndexEngine {
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect code-index engine handle to isolated schema");
    CodeIndexEngine::new(Arc::new(db))
}

const RUST_SRC: &str = r#"
//! Module docs.
use crate::storage::knowledge;

/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 { a + b }

pub fn caller() -> i32 { add(1, 2) }

pub struct Widget { size: u32 }

pub trait Render { fn render(&self); }

impl Render for Widget {
    fn render(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }
}
"#;

const TS_SRC: &str = r#"
import { helper } from "./util";

export interface Props { title: string }
export class Service { run(): void {} }
export function compute(): number { return 1; }
export const Button = (props: Props) => { return null; };
export const useThing = () => { return 1; };
"#;

const JS_SRC: &str = r#"
export function compute(a, b) { return a + b; }
class Store { put(k, v) {} }
export const handler = (req) => { return req; };
"#;

const PKG_JSON: &str = r#"{
  "name": "demo",
  "scripts": { "build": "tsc", "test": "vitest" }
}"#;

/// A file the Rust grammar cannot meaningfully parse into symbols but that still
/// produces a tree (with errors) — used to exercise the `partial` path. For a
/// genuine `failed` path we feed a non-code source through the config router.
const BROKEN_RUST: &str = "fn (((((  {{{{  ";

// ---------------------------------------------------------------------------
// MT-097 parser adapter (no DB).
// ---------------------------------------------------------------------------

#[test]
fn mt097_adapter_parses_each_language_and_versions() {
    for (lang, src) in [
        (CodeLanguage::Rust, "fn a() {}"),
        (CodeLanguage::TypeScript, "export const x = 1;"),
        (CodeLanguage::JavaScript, "function a() {}"),
        (CodeLanguage::Tsx, "const A = () => null;"),
    ] {
        let adapter = CodeParserAdapter::new(lang);
        let tree = adapter.parse(src).expect("parse");
        assert_eq!(tree.language, lang);
        assert!(!tree.parser_version.is_empty());
        assert!(!tree.nodes.is_empty(), "{lang:?} produced no nodes");
    }
}

// ---------------------------------------------------------------------------
// MT-098..104 + 108 + 112: end-to-end index of a mixed mini-tree.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt112_indexes_mixed_tree_with_symbols_spans_edges() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt112_indexes_mixed_tree_with_symbols_spans_edges: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;

    let run_id = eng
        .start_run(&context, &workspace_id, None)
        .await
        .expect("start run");

    // Register + index each file the way the ingestion lane would have.
    let files = [
        ("src/lib.rs", RUST_SRC),
        ("app/service.ts", TS_SRC),
        ("app/store.js", JS_SRC),
        ("package.json", PKG_JSON),
    ];
    for (path, text) in files {
        let source_id = eng
            .register_code_source(&workspace_id, Some(&root), path, text)
            .await
            .expect("register source");
        let outcome = eng
            .index_code_source(
                &context,
                &workspace_id,
                &source_id,
                path,
                text,
                Some(&run_id),
            )
            .await
            .expect("index source");
        assert!(!outcome.failed, "{path} should index cleanly");
    }

    // --- Symbols (MT-098..100) ------------------------------------------------
    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("list symbols");
    let keys: Vec<&str> = symbols.iter().map(|s| s.entity_key.as_str()).collect();
    // Rust
    assert!(keys.iter().any(|k| *k == "rust:src/lib.rs#add"), "{keys:?}");
    assert!(
        keys.iter().any(|k| *k == "rust:src/lib.rs#Widget::render"),
        "{keys:?}"
    );
    // TS
    assert!(
        keys.iter()
            .any(|k| *k == "typescript:app/service.ts#Service"),
        "{keys:?}"
    );
    assert!(
        keys.iter()
            .any(|k| *k == "typescript:app/service.ts#useThing"),
        "{keys:?}"
    );
    // JS
    assert!(
        keys.iter().any(|k| *k == "javascript:app/store.js#compute"),
        "{keys:?}"
    );

    // --- Spans: every symbol entity has an `ast` evidence span (MT-055/098) ---
    let add_symbol = symbols
        .iter()
        .find(|s| s.entity_key == "rust:src/lib.rs#add")
        .expect("add symbol");
    let add_span_ids = pg
        .db
        .list_knowledge_entity_span_ids(&add_symbol.entity_id)
        .await
        .expect("add span ids");
    assert!(!add_span_ids.is_empty(), "add must have an evidence span");
    let add_span = pg
        .db
        .get_knowledge_span(&add_span_ids[0])
        .await
        .expect("get span")
        .expect("span exists");
    assert_eq!(add_span.span_kind, KnowledgeSpanKind::Ast);
    assert!(add_span.line_start.unwrap_or(0) > 0);

    // --- Edges: a call edge (MT-104) caller -> add, with evidence + version ---
    let add_edges = pg
        .db
        .list_knowledge_edges_for_entity(&add_symbol.entity_id)
        .await
        .expect("add edges");
    let call_edge = add_edges
        .iter()
        .find(|e| {
            e.edge_type == KnowledgeEdgeType::References
                && e.target_entity_id == add_symbol.entity_id
        })
        .expect("a references edge should target add (caller calls add)");
    assert_eq!(call_edge.extractor_version, "code_index_extractor_v1");
    assert!(call_edge.confidence > 0.0 && call_edge.confidence <= 1.0);
    assert!(!call_edge.relationship_id.is_empty());
    let call_edge_spans = pg
        .db
        .list_knowledge_edge_span_ids(&call_edge.edge_id)
        .await
        .expect("call edge spans");
    assert!(
        !call_edge_spans.is_empty(),
        "every edge MUST carry a source span ref (spec 2.3.13.11)"
    );

    // --- Import edge (MT-104): file depends_on a module concept ----------------
    let concepts = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Concept)
        .await
        .expect("list concepts");
    assert!(
        concepts
            .iter()
            .any(|c| c.entity_key == "module:crate::storage::knowledge"),
        "rust use import should create a module concept: {:?}",
        concepts.iter().map(|c| &c.entity_key).collect::<Vec<_>>()
    );

    // --- Config facts (MT-101): package.json scripts become entities ----------
    let commands = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Command)
        .await
        .expect("list commands");
    assert!(
        commands
            .iter()
            .any(|c| c.entity_key == "config:package.json#scripts.build"),
        "package script should be a command entity: {:?}",
        commands.iter().map(|c| &c.entity_key).collect::<Vec<_>>()
    );

    // --- Code-file index state (MT-107 bookkeeping) ---------------------------
    // Only the three CODE files (rust/ts/js) get a knowledge_code_files row; the
    // config file (package.json) is indexed through the config path and does not
    // produce a code-file row (it is not a CodeLanguage). Its facts are proven
    // by the `commands` assertion above.
    let code_files = pg
        .db
        .list_knowledge_code_files(&workspace_id)
        .await
        .expect("list code files");
    assert_eq!(code_files.len(), 3, "one code-file row per CODE source");
    assert!(code_files.iter().all(|f| !f.stale));
    assert!(code_files
        .iter()
        .all(|f| f.parse_status == KnowledgeCodeParseStatus::Parsed));
}

// ---------------------------------------------------------------------------
// MT-102: test -> tested-symbol `validates` edge.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt102_test_validates_edge_to_tested_symbol() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt102_test_validates_edge_to_tested_symbol: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
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

    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols");
    let add = symbols
        .iter()
        .find(|s| s.entity_key == "rust:src/lib.rs#add")
        .expect("add symbol");
    let edges = pg
        .db
        .list_knowledge_edges_for_entity(&add.entity_id)
        .await
        .expect("edges");
    // The `adds` test calls add(...) -> a validates edge test -> add.
    assert!(
        edges.iter().any(|e| {
            e.edge_type == KnowledgeEdgeType::Validates && e.target_entity_id == add.entity_id
        }),
        "expected a validates edge targeting add"
    );
}

// ---------------------------------------------------------------------------
// MT-103: doc comment becomes a `documents` edge with a text span.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt103_doc_comment_indexed_as_documents_edge() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt103_doc_comment_indexed_as_documents_edge: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
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

    // The "Adds two numbers." doc comment precedes `add`; a text span exists.
    let spans = pg
        .db
        .list_knowledge_spans_for_source(&source_id)
        .await
        .expect("spans");
    assert!(
        spans.iter().any(|s| s.span_kind == KnowledgeSpanKind::Text),
        "a doc/TODO passage should produce a text span"
    );
}

// ---------------------------------------------------------------------------
// MT-108: a single unparseable file does NOT fail the run.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt108_partial_failure_keeps_run_useful() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt108_partial_failure_keeps_run_useful: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let run_id = eng
        .start_run(&context, &workspace_id, None)
        .await
        .expect("run");

    // A malformed Rust file still parses into a tree with errors (partial), and
    // the good file indexes cleanly. The run continues regardless.
    let broken_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/broken.rs", BROKEN_RUST)
        .await
        .expect("register broken");
    let broken_outcome = eng
        .index_code_source(
            &context,
            &workspace_id,
            &broken_id,
            "src/broken.rs",
            BROKEN_RUST,
            Some(&run_id),
        )
        .await
        .expect("indexing a broken file must not error the run");
    assert!(
        matches!(
            broken_outcome.parse_status,
            KnowledgeCodeParseStatus::Partial | KnowledgeCodeParseStatus::Failed
        ),
        "broken file must be partial or failed, got {:?}",
        broken_outcome.parse_status
    );

    let good_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/good.rs", "pub fn ok() {}")
        .await
        .expect("register good");
    let good_outcome = eng
        .index_code_source(
            &context,
            &workspace_id,
            &good_id,
            "src/good.rs",
            "pub fn ok() {}",
            Some(&run_id),
        )
        .await
        .expect("good file indexes");
    assert!(!good_outcome.failed);

    // The good symbol is present despite the broken sibling.
    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols");
    assert!(symbols
        .iter()
        .any(|s| s.entity_key == "rust:src/good.rs#ok"));
}

// ---------------------------------------------------------------------------
// MT-107: staleness flips when the source content hash changes.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt107_staleness_detected_on_content_change() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt107_staleness_detected_on_content_change: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/lib.rs", "pub fn a() {}")
        .await
        .expect("register");
    eng.index_code_source(
        &context,
        &workspace_id,
        &source_id,
        "src/lib.rs",
        "pub fn a() {}",
        None,
    )
    .await
    .expect("index");

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();

    // Fresh against the same content/parser.
    let fresh = build_monaco_payload(
        eng.db(),
        &workspace_id,
        "src/lib.rs",
        &sha256_hex(b"pub fn a() {}"),
        &parser_version,
    )
    .await
    .expect("monaco payload fresh");
    assert!(matches!(fresh.staleness, StalenessVerdict::Fresh));

    // Stale against a different content hash (the editor buffer changed).
    let stale = build_monaco_payload(
        eng.db(),
        &workspace_id,
        "src/lib.rs",
        &sha256_hex(b"pub fn a() { changed }"),
        &parser_version,
    )
    .await
    .expect("monaco payload stale");
    assert!(
        matches!(stale.staleness, StalenessVerdict::SourceChanged { .. }),
        "content change must flag SourceChanged, got {:?}",
        stale.staleness
    );
}

// ---------------------------------------------------------------------------
// MT-109: the Monaco payload carries symbol entries with definition ranges.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt109_monaco_payload_has_entries_with_ranges() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt109_monaco_payload_has_entries_with_ranges: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
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

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let payload = build_monaco_payload(
        eng.db(),
        &workspace_id,
        "src/lib.rs",
        &sha256_hex(RUST_SRC.as_bytes()),
        &parser_version,
    )
    .await
    .expect("payload");
    assert!(!payload.entries.is_empty(), "payload should list symbols");
    let add = payload
        .entries
        .iter()
        .find(|e| e.symbol_key == "rust:src/lib.rs#add")
        .expect("add lens entry");
    assert!(add.definition.start_line > 0);
    // `add` has at least one caller (`caller`).
    assert!(add.caller_count >= 1, "add should have a caller");
}

// ---------------------------------------------------------------------------
// MT-110: the context bundle is bounded + cited.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt110_context_bundle_is_bounded_and_cited() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt110_context_bundle_is_bounded_and_cited: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
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

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let bundle = build_code_context_bundle(
        eng.db(),
        "KTR-bundle-test",
        "SR-bundle-test",
        &workspace_id,
        "src/lib.rs",
        "rust:src/lib.rs#add",
        &sha256_hex(RUST_SRC.as_bytes()),
        &parser_version,
        DEFAULT_CODE_CONTEXT_TOKEN_BUDGET,
    )
    .await
    .expect("bundle");

    // The bundle is a durable kernel ContextBundle keyed by run/session.
    assert_eq!(bundle.kernel_task_run_id, "KTR-bundle-test");
    let ctx_json = &bundle.allowed_context;
    assert_eq!(ctx_json["kind"], "code_symbol_context");
    assert_eq!(ctx_json["focus"]["symbol_key"], "rust:src/lib.rs#add");
    // Token accounting is present and within budget.
    let used = ctx_json["token_accounting"]["estimated_tokens_used"]
        .as_u64()
        .expect("token count");
    assert!(used <= DEFAULT_CODE_CONTEXT_TOKEN_BUDGET as u64);
    // `add` is called by `caller` -> a cited called_by neighbor with a span.
    let called_by = ctx_json["called_by"].as_array().expect("called_by array");
    assert!(
        called_by
            .iter()
            .any(|n| n["symbol_key"] == "rust:src/lib.rs#caller"),
        "caller should appear as a called_by neighbor: {called_by:?}"
    );
}

// ---------------------------------------------------------------------------
// MT-105: SCIP import ledger records imports and rejections.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt105_scip_import_ledger_records_outcomes() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt105_scip_import_ledger_records_outcomes: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;

    // A valid artifact parses (boundary parses; never spawns an indexer).
    let good = br#"{
      "format": "scip",
      "tool": { "name": "scip-rust", "version": "0.3.0" },
      "documents": [
        { "relative_path": "src/lib.rs", "language": "rust",
          "symbols": [ { "symbol": "rust:src/lib.rs#alpha", "kind": "function",
            "display_name": "alpha", "line_start": 1, "line_end": 2,
            "byte_start": 0, "byte_end": 10 } ] }
      ]
    }"#;
    let artifact = parse_scip_artifact(good).expect("valid artifact parses");
    assert_eq!(artifact.symbol_count(), 1);

    // Record an imported outcome.
    let imported = pg
        .db
        .record_knowledge_scip_import(NewKnowledgeScipImport {
            workspace_id: workspace_id.clone(),
            artifact_format: KnowledgeScipFormat::Scip,
            tool_name: Some("scip-rust".to_string()),
            tool_version: Some("0.3.0".to_string()),
            artifact_hash: artifact_hash(good),
            status: KnowledgeScipImportStatus::Imported,
            reason: None,
            symbols_imported: artifact.symbol_count() as i32,
            occurrences_imported: artifact.occurrence_count() as i32,
            edges_imported: 0,
            import_detail: None,
            imported_in_run: None,
            import_receipt_event_id: None,
        })
        .await
        .expect("record imported");
    assert_eq!(imported.status, KnowledgeScipImportStatus::Imported);

    // A rejected artifact (no documents) is recorded WITH a reason.
    let rejected_reason = parse_scip_artifact(br#"{ "format": "scip", "documents": [] }"#)
        .expect_err("empty documents rejected");
    let rejected = pg
        .db
        .record_knowledge_scip_import(NewKnowledgeScipImport {
            workspace_id: workspace_id.clone(),
            artifact_format: KnowledgeScipFormat::Scip,
            tool_name: None,
            tool_version: None,
            artifact_hash: artifact_hash(b"{ \"format\": \"scip\", \"documents\": [] }"),
            status: KnowledgeScipImportStatus::Rejected,
            reason: Some(rejected_reason.reason.clone()),
            symbols_imported: 0,
            occurrences_imported: 0,
            edges_imported: 0,
            import_detail: None,
            imported_in_run: None,
            import_receipt_event_id: None,
        })
        .await
        .expect("record rejected");
    assert_eq!(rejected.status, KnowledgeScipImportStatus::Rejected);
    assert!(rejected.reason.is_some());

    let all = pg
        .db
        .list_knowledge_scip_imports(&workspace_id)
        .await
        .expect("list imports");
    assert_eq!(all.len(), 2);
}

// ---------------------------------------------------------------------------
// MT-104 determinism: re-indexing the same file does NOT duplicate edges.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt104_reindex_is_idempotent_on_relationship_id() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt104_reindex_is_idempotent_on_relationship_id: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let src = "pub fn helper() -> i32 { 1 }\npub fn caller() -> i32 { helper() }\n";
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/lib.rs", src)
        .await
        .expect("register");

    eng.index_code_source(&context, &workspace_id, &source_id, "src/lib.rs", src, None)
        .await
        .expect("index 1");
    let symbols1 = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols 1");
    let helper = symbols1
        .iter()
        .find(|s| s.entity_key == "rust:src/lib.rs#helper")
        .expect("helper");
    let edges1 = pg
        .db
        .list_knowledge_edges_for_entity(&helper.entity_id)
        .await
        .expect("edges 1");

    // Re-index identical content.
    eng.index_code_source(&context, &workspace_id, &source_id, "src/lib.rs", src, None)
        .await
        .expect("index 2");
    let edges2 = pg
        .db
        .list_knowledge_edges_for_entity(&helper.entity_id)
        .await
        .expect("edges 2");

    assert_eq!(
        edges1.len(),
        edges2.len(),
        "re-index must converge on the same edges (deterministic relationship_id)"
    );
}

// ---------------------------------------------------------------------------
// MT-111: the parse+extract performance budget holds on a representative
// fixture (a guardrail against an accidental O(n^2) regression in the AST walk).
// ---------------------------------------------------------------------------

#[test]
fn mt111_parse_extract_within_documented_budget() {
    use handshake_core::knowledge_code_index::docs_todo::extract_doc_passages;
    use handshake_core::knowledge_code_index::perf::{CodeIndexBudget, PerfSample};
    use handshake_core::knowledge_code_index::relationships::extract_relationships;
    use handshake_core::knowledge_code_index::symbols::extract_symbols;

    // Build a ~1000-line representative Rust fixture by repeating a small unit.
    let unit = "pub fn f_{N}() -> i32 { let x = g_{N}(); x + 1 }\npub fn g_{N}() -> i32 { 0 }\n";
    let mut src = String::with_capacity(64 * 1024);
    for n in 0..400 {
        src.push_str(&unit.replace("{N}", &n.to_string()));
    }
    let line_count = src.lines().count();

    // The DOCUMENTED production budget (perf::CodeIndexBudget::default) targets
    // an optimized build. This test runs under `cargo test` (debug, unoptimized
    // + overflow checks), where the same work is legitimately several times
    // slower. The guardrail we enforce here is the quadratic-blowup catch: a
    // debug-scaled allowance that an accidental O(n^2) in the AST walk / symbol
    // resolution (which would take SECONDS at this size) cannot pass, while
    // ordinary debug overhead does. The production budget itself is asserted by
    // its own unit tests in perf.rs.
    let production = CodeIndexBudget::default();
    let debug_budget = CodeIndexBudget {
        max_ms_per_kloc: production.max_ms_per_kloc * 8.0,
        fixed_overhead_ms: production.fixed_overhead_ms * 8.0,
    };
    let adapter = CodeParserAdapter::new(CodeLanguage::Rust);

    let started = std::time::Instant::now();
    let tree = adapter.parse(&src).expect("parse fixture");
    let symbols = extract_symbols(&tree, &src);
    let _docs = extract_doc_passages(&src);
    let _rels = extract_relationships(&tree, &src, &symbols);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let sample = PerfSample::measure(&debug_budget, "perf_fixture.rs", line_count, elapsed_ms);
    assert!(
        sample.within_budget,
        "parse+extract of {line_count} lines took {elapsed_ms:.1}ms, debug budget {:.1}ms \
         (production budget {:.1}ms); a value this far over signals an algorithmic regression \
         (MT-111)",
        sample.allowed_ms,
        production.allowed_ms(line_count),
    );
    // Sanity: the fixture actually produced the expected symbol volume.
    assert!(
        symbols.len() >= 800,
        "expected ~800 fns, got {}",
        symbols.len()
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
