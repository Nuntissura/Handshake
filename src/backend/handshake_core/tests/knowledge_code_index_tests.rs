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

use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::knowledge_code_index::context_bridge::{
    build_code_context_bundle, DEFAULT_CODE_CONTEXT_TOKEN_BUDGET,
};
use handshake_core::knowledge_code_index::engine::{
    read_and_index, CodeIndexContext, CodeIndexEngine,
};
use handshake_core::knowledge_code_index::monaco_bridge::build_monaco_payload;
use handshake_core::knowledge_code_index::parser::{CodeLanguage, CodeParserAdapter};
use handshake_core::knowledge_code_index::scip::{artifact_hash, parse_scip_artifact};
use handshake_core::knowledge_code_index::staleness::StalenessVerdict;
use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::engine::{
    IngestionContext, IngestionEngine, RootRegistrationRequest,
};
use handshake_core::storage::knowledge::{
    KnowledgeCodeLanguage, KnowledgeCodeParseStatus, KnowledgeCodeRepairReason, KnowledgeEdgeType,
    KnowledgeEntityKind, KnowledgeIndexingEligibility, KnowledgeRootKind, KnowledgeScipFormat,
    KnowledgeScipImportStatus, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeScipImport,
    NewKnowledgeSourceRoot,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
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

fn ingestion_ctx() -> IngestionContext {
    IngestionContext {
        actor: KernelActor::System("knowledge-ingestion-test".to_string()),
        kernel_task_run_id: "KTR-knowledge-ingestion-test".to_string(),
        session_run_id: "SR-knowledge-ingestion-test".to_string(),
        correlation_id: Some("CORR-knowledge-ingestion-test".to_string()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ledger_idempotency_uses_payload_hash_not_jsonb_shape() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP event_ledger_idempotency_uses_payload_hash_not_jsonb_shape: no PostgreSQL");
        return;
    };
    let suffix = Uuid::now_v7();
    let event = NewKernelEvent::builder(
        format!("KTR-jsonb-normalized-{suffix}"),
        format!("SR-jsonb-normalized-{suffix}"),
        KernelEventType::KnowledgeValidationRecorded,
        KernelActor::System("code-index-test".to_string()),
    )
    .aggregate(
        "knowledge_code_index_file",
        format!("KSRC-jsonb-normalized-{suffix}"),
    )
    .idempotency_key(format!("idem-jsonb-normalized-{suffix}"))
    .source_component("knowledge_code_index")
    .payload(json!({
        "kind": "jsonb_normalized_payload",
        "elapsed_ms": 1e-17,
    }))
    .build()
    .expect("build event");

    let first = pg
        .db
        .append_kernel_event(event.clone())
        .await
        .expect("append first event");
    let second = pg
        .db
        .append_kernel_event(event)
        .await
        .expect("idempotent duplicate append");

    assert_eq!(first.event_id, second.event_id);
    assert_eq!(first.payload_hash, second.payload_hash);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ledger_idempotency_conflict_reports_key_and_hashes() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP event_ledger_idempotency_conflict_reports_key_and_hashes: no PostgreSQL");
        return;
    };
    let suffix = Uuid::now_v7();
    let idempotency_key = format!("idem-conflict-detail-{suffix}");
    let first = NewKernelEvent::builder(
        format!("KTR-conflict-detail-{suffix}"),
        format!("SR-conflict-detail-{suffix}"),
        KernelEventType::KnowledgeValidationRecorded,
        KernelActor::System("code-index-test".to_string()),
    )
    .aggregate(
        "knowledge_code_index_file",
        format!("KSRC-conflict-detail-{suffix}"),
    )
    .idempotency_key(idempotency_key.clone())
    .source_component("knowledge_code_index")
    .payload(json!({"kind": "conflict_detail", "ordinal": 1}))
    .build()
    .expect("build first event");
    let second = NewKernelEvent::builder(
        format!("KTR-conflict-detail-{suffix}"),
        format!("SR-conflict-detail-{suffix}"),
        KernelEventType::KnowledgeValidationRecorded,
        KernelActor::System("code-index-test".to_string()),
    )
    .aggregate(
        "knowledge_code_index_file",
        format!("KSRC-conflict-detail-{suffix}"),
    )
    .idempotency_key(idempotency_key.clone())
    .source_component("knowledge_code_index")
    .payload(json!({"kind": "conflict_detail", "ordinal": 2}))
    .build()
    .expect("build second event");

    pg.db
        .append_kernel_event(first)
        .await
        .expect("append first event");
    let err = pg
        .db
        .append_kernel_event(second)
        .await
        .expect_err("same idempotency key with divergent payload must fail");
    let message = err.to_string();

    assert!(message.contains(&idempotency_key), "{message}");
    assert!(message.contains("existing_payload_hash"), "{message}");
    assert!(message.contains("new_payload_hash"), "{message}");
    assert!(message.contains("knowledge_code_index_file"), "{message}");
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

async fn ingestion_engine(pg: &KnowledgePg) -> IngestionEngine {
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect ingestion engine handle to isolated schema");
    IngestionEngine::from_database(Arc::new(db))
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

const TSX_SRC: &str = r#"
import React from "react";

export interface WidgetProps { title: string }
export function Widget(props: WidgetProps) {
    return <section>{props.title}</section>;
}
export const useWidget = () => "widget";
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

const JSON_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "title": { "type": "string" },
    "count": { "type": "number" }
  }
}"#;

const MIGRATION_SQL: &str = r#"
-- CREATE TABLE ignored_comment (id bigint);
CREATE TABLE public.users (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL
);

CREATE INDEX idx_users_email ON public.users (email);
"#;

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
        ("app/widget.tsx", TSX_SRC),
        ("app/store.js", JS_SRC),
        ("package.json", PKG_JSON),
        ("schema/user.schema.json", JSON_SCHEMA),
        ("migrations/20240614_create_users.sql", MIGRATION_SQL),
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

    let broken_source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/broken.rs", BROKEN_RUST)
        .await
        .expect("register malformed rust source");
    let broken_outcome = eng
        .index_code_source(
            &context,
            &workspace_id,
            &broken_source_id,
            "src/broken.rs",
            BROKEN_RUST,
            Some(&run_id),
        )
        .await
        .expect("malformed source should degrade to partial, not abort");
    assert!(!broken_outcome.failed);
    assert_eq!(
        broken_outcome.parse_status,
        KnowledgeCodeParseStatus::Partial
    );

    // --- Symbols (MT-098..100) ------------------------------------------------
    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("list symbols");
    let keys: Vec<&str> = symbols.iter().map(|s| s.entity_key.as_str()).collect();
    // Rust: free fns keep a clean key; a TRAIT-impl method carries the `as:Trait`
    // collision discriminator (MT-098) so it never merges with an inherent one.
    assert!(keys.iter().any(|k| *k == "rust:src/lib.rs#add"), "{keys:?}");
    assert!(
        keys.iter()
            .any(|k| *k == "rust:src/lib.rs#Widget::render~as:Render"),
        "{keys:?}"
    );
    // TS: top-level decls carry a kind tag (MT-099 declaration-merging safety).
    assert!(
        keys.iter()
            .any(|k| *k == "typescript:app/service.ts#Service~class"),
        "{keys:?}"
    );
    assert!(
        keys.iter()
            .any(|k| *k == "typescript:app/service.ts#useThing~hook"),
        "{keys:?}"
    );
    // TSX fixture: component and hook symbols are indexed from TSX, not only
    // plain TypeScript.
    assert!(
        keys.iter()
            .any(|k| *k == "tsx:app/widget.tsx#Widget~component"),
        "{keys:?}"
    );
    assert!(
        keys.iter()
            .any(|k| *k == "tsx:app/widget.tsx#useWidget~hook"),
        "{keys:?}"
    );
    // JS
    assert!(
        keys.iter()
            .any(|k| *k == "javascript:app/store.js#compute~value"),
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
    let schemas = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Schema)
        .await
        .expect("list schema entities");
    assert!(
        schemas.iter().any(|s| {
            s.entity_key == "config:migrations/20240614_create_users.sql#table.public.users"
        }),
        "migration table should be a schema entity: {:?}",
        schemas.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );
    assert!(
        schemas.iter().any(|s| {
            s.entity_key == "config:migrations/20240614_create_users.sql#index.idx_users_email"
        }),
        "migration index should be a schema entity: {:?}",
        schemas.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );
    assert!(
        schemas
            .iter()
            .any(|s| s.entity_key == "config:schema/user.schema.json#properties.title"),
        "JSON schema property should be a schema entity: {:?}",
        schemas.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );
    assert!(
        schemas
            .iter()
            .all(|s| !s.entity_key.contains("ignored_comment")),
        "commented-out migration DDL must not be indexed: {:?}",
        schemas.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );

    // --- Code-file index state (MT-107 bookkeeping) ---------------------------
    // MT-101 hardening: every indexed source — the three CODE files
    // (rust/ts/js) AND the config files (package.json + migration SQL) — now
    // produces a knowledge_code_files
    // row, so staleness (MT-107) and the lens cover config sources too. The
    // config row carries language 'config'.
    let code_files = pg
        .db
        .list_knowledge_code_files(&workspace_id)
        .await
        .expect("list code files");
    assert_eq!(
        code_files.len(),
        8,
        "one code-file row per source incl. config: {:?}",
        code_files
            .iter()
            .map(|f| (&f.source_id, f.language))
            .collect::<Vec<_>>()
    );
    assert!(code_files.iter().all(|f| !f.stale));
    assert!(code_files.iter().all(|f| {
        f.parse_status == KnowledgeCodeParseStatus::Parsed
            || (f.source_id == broken_source_id
                && f.parse_status == KnowledgeCodeParseStatus::Partial)
    }));
    let config_rows: Vec<_> = code_files
        .iter()
        .filter(|f| f.language == KnowledgeCodeLanguage::Config)
        .collect();
    assert_eq!(
        config_rows.len(),
        3,
        "the config files must each produce a language='config' code-file row"
    );
    assert!(config_rows.iter().all(|row| row.symbols_indexed == 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt112_ingestion_allowlist_excludes_generated_files_before_indexing() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP mt112_ingestion_allowlist_excludes_generated_files_before_indexing: no PostgreSQL"
        );
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let ing = ingestion_engine(&pg).await;
    let context = ingestion_ctx();
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("src dir");
    std::fs::create_dir_all(dir.path().join("generated")).expect("generated dir");
    std::fs::write(dir.path().join("src/main.rs"), "pub fn allowed() {}\n").expect("write source");
    std::fs::write(
        dir.path().join("generated/out.rs"),
        "pub fn generated_should_not_index() {}\n",
    )
    .expect("write generated");

    let (root, _decision) = ing
        .register_root(
            &context,
            RootRegistrationRequest {
                workspace_id: workspace_id.clone(),
                display_name: "fixture-root".to_string(),
                root_kind: KnowledgeRootKind::ProjectRepo,
                repo_relative_path: "".to_string(),
                file_allowlist_policy: json!({
                    "include": ["**/*"],
                    "exclude": ["generated/**"],
                }),
                operator_approved: false,
            },
        )
        .await
        .expect("register root");

    let summary = ing
        .run_ingestion_pass(
            &context,
            &root.root_id,
            dir.path(),
            &IngestionLimits::default(),
        )
        .await
        .expect("ingestion pass");
    assert_eq!(
        summary.skipped_by_allowlist, 1,
        "generated file should be skipped before source/index registration"
    );
    assert_eq!(summary.outcomes.len(), 1);

    let sources = pg
        .db
        .list_knowledge_sources_for_root(&root.root_id)
        .await
        .expect("list sources");
    let paths: Vec<_> = sources
        .iter()
        .filter_map(|source| source.relative_path.as_deref())
        .collect();
    assert!(paths.contains(&"src/main.rs"), "{paths:?}");
    assert!(
        !paths.contains(&"generated/out.rs"),
        "generated file must not become an indexable knowledge source: {paths:?}"
    );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt102_typescript_test_runner_blocks_validate_called_symbols() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP mt102_typescript_test_runner_blocks_validate_called_symbols: no PostgreSQL"
        );
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let src = r#"
import { describe, it, test, expect } from "vitest";

export function compute(): number { return 1; }
export function helper(): number { return 2; }

describe("math", () => {
    it("computes value", () => {
        expect(compute()).toBe(1);
    });
    test("uses helper", () => {
        helper();
    });
});
"#;
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "app/service.test.ts", src)
        .await
        .expect("register");
    eng.index_code_source(
        &context,
        &workspace_id,
        &source_id,
        "app/service.test.ts",
        src,
        None,
    )
    .await
    .expect("index");

    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols");
    let compute = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/service.test.ts#compute~value")
        .expect("compute symbol");
    let helper = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/service.test.ts#helper~value")
        .expect("helper symbol");
    assert!(
        symbols.iter().any(|s| {
            s.entity_key == "typescript:app/service.test.ts#test.math.computes value~test"
        }),
        "it() block should be persisted as a test symbol: {:?}",
        symbols.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );
    assert!(
        symbols.iter().any(|s| {
            s.entity_key == "typescript:app/service.test.ts#test.math.uses helper~test"
        }),
        "test() block should be persisted as a test symbol: {:?}",
        symbols.iter().map(|s| &s.entity_key).collect::<Vec<_>>()
    );

    for target in [compute, helper] {
        let edges = pg
            .db
            .list_knowledge_edges_for_entity(&target.entity_id)
            .await
            .expect("edges");
        assert!(
            edges.iter().any(|e| {
                e.edge_type == KnowledgeEdgeType::Validates
                    && e.target_entity_id == target.entity_id
            }),
            "expected a validates edge targeting {}: {edges:?}",
            target.entity_key
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt102_duplicate_typescript_test_titles_keep_distinct_validates_edges() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP mt102_duplicate_typescript_test_titles_keep_distinct_validates_edges: no PostgreSQL"
        );
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let src = r#"
import { describe, it } from "vitest";

export function first(): number { return 1; }
export function second(): number { return 2; }

describe("math", () => {
    it("duplicates", () => {
        first();
    });
    it("duplicates", () => {
        second();
    });
});
"#;
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "app/duplicate.test.ts", src)
        .await
        .expect("register");
    eng.index_code_source(
        &context,
        &workspace_id,
        &source_id,
        "app/duplicate.test.ts",
        src,
        None,
    )
    .await
    .expect("index");

    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols");
    let first = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/duplicate.test.ts#first~value")
        .expect("first symbol");
    let second = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/duplicate.test.ts#second~value")
        .expect("second symbol");
    let first_test = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/duplicate.test.ts#test.math.duplicates~test.dup0")
        .expect("first duplicate-title test symbol");
    let second_test = symbols
        .iter()
        .find(|s| s.entity_key == "typescript:app/duplicate.test.ts#test.math.duplicates~test.dup1")
        .expect("second duplicate-title test symbol");

    let first_test_edges = pg
        .db
        .list_knowledge_edges_for_entity(&first_test.entity_id)
        .await
        .expect("first test edges");
    let second_test_edges = pg
        .db
        .list_knowledge_edges_for_entity(&second_test.entity_id)
        .await
        .expect("second test edges");

    assert!(
        first_test_edges.iter().any(|e| {
            e.edge_type == KnowledgeEdgeType::Validates
                && e.source_entity_id == first_test.entity_id
                && e.target_entity_id == first.entity_id
        }),
        "first duplicate test must validate first(), not be collapsed onto the later test: {first_test_edges:?}"
    );
    assert!(
        !first_test_edges.iter().any(|e| {
            e.edge_type == KnowledgeEdgeType::Validates
                && e.source_entity_id == first_test.entity_id
                && e.target_entity_id == second.entity_id
        }),
        "first duplicate test must not inherit second() coverage: {first_test_edges:?}"
    );
    assert!(
        second_test_edges.iter().any(|e| {
            e.edge_type == KnowledgeEdgeType::Validates
                && e.source_entity_id == second_test.entity_id
                && e.target_entity_id == second.entity_id
        }),
        "second duplicate test must validate second(): {second_test_edges:?}"
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

/// MT-103 V1 remediation: operator-facing string literals are indexed as their
/// OWN concept entities (passage_kind `operator_string`), end-to-end in real
/// PostgreSQL, SEPARATE from doc-comment and TODO concept entities. This is the
/// "missing operator-facing string extraction coverage" the WP validator FAILed
/// MT-103 on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt103_operator_strings_indexed_separately_from_markers() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt103_operator_strings_indexed_separately_from_markers: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;

    // One file carrying all three: a doc comment, a TODO marker, and a
    // user-visible println! string.
    let src = r#"
/// Runs the export.
pub fn run() {
    // TODO: add a retry
    println!("Export complete, files written");
}
"#;
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/exp.rs", src)
        .await
        .expect("register");
    eng.index_code_source(&context, &workspace_id, &source_id, "src/exp.rs", src, None)
        .await
        .expect("index");

    // Concept entities of this workspace, partitioned by passage kind.
    let concepts = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Concept)
        .await
        .expect("concepts");
    let kind_of = |c: &handshake_core::storage::knowledge::KnowledgeEntity| -> Option<String> {
        c.detection_provenance
            .get("passage_kind")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    // The operator string is its OWN concept entity, keyed `...:operator_string`.
    let op_string = concepts
        .iter()
        .find(|c| kind_of(c).as_deref() == Some("operator_string"))
        .unwrap_or_else(|| {
            panic!(
                "an operator_string concept must be indexed; concepts: {:?}",
                concepts
                    .iter()
                    .map(|c| (&c.entity_key, kind_of(c)))
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        op_string.entity_key.contains("operator_string"),
        "operator-string concept key must carry its kind: {}",
        op_string.entity_key
    );
    assert!(
        op_string.display_name.contains("Export complete"),
        "operator-string display must be the message: {}",
        op_string.display_name
    );

    // It is DISTINCT from the doc-comment and TODO concept entities (separate
    // rows, separate keys) — never merged into a marker/comment passage.
    assert!(
        concepts
            .iter()
            .any(|c| kind_of(c).as_deref() == Some("doc_comment")),
        "the doc comment must still be its own concept"
    );
    assert!(
        concepts
            .iter()
            .any(|c| kind_of(c).as_deref() == Some("todo")),
        "the TODO must still be its own concept"
    );
    let op_keys: Vec<&String> = concepts
        .iter()
        .filter(|c| kind_of(c).as_deref() == Some("operator_string"))
        .map(|c| &c.entity_key)
        .collect();
    let marker_keys: Vec<&String> = concepts
        .iter()
        .filter(|c| matches!(kind_of(c).as_deref(), Some("doc_comment") | Some("todo")))
        .map(|c| &c.entity_key)
        .collect();
    for ok in &op_keys {
        assert!(
            !marker_keys.contains(ok),
            "operator-string key {ok} must not collide with any marker/comment key"
        );
    }

    // The operator string is anchored to a `text` span (citeable evidence).
    let op_span_ids = pg
        .db
        .list_knowledge_entity_span_ids(&op_string.entity_id)
        .await
        .expect("op span ids");
    assert!(!op_span_ids.is_empty(), "operator string must cite a span");
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
// MT-108 (hardening): a BINARY / non-UTF-8 file on disk does NOT abort the
// directory run, is recorded `failed`, and is ENQUEUED on the durable
// code-index repair queue with reason READ_ERROR (real recovery surface, not a
// flag). A good sibling in the same directory still indexes. This is the exact
// gap the adversarial review flagged: the read path used to propagate
// CodeIndexError::Io and abort a `?`-propagating batch caller.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt108_binary_read_failure_enqueues_repair_and_keeps_run_useful() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP mt108_binary_read_failure_enqueues_repair_and_keeps_run_useful: no PostgreSQL"
        );
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

    // A real on-disk directory anchor (machine-local runtime config, never
    // stored) with one binary file (invalid UTF-8 with a .rs extension) and one
    // good Rust file.
    let dir = tempfile::tempdir().expect("tempdir");
    // 0xFF 0xFE is not valid UTF-8; the .rs extension forces the code path.
    std::fs::write(dir.path().join("blob.rs"), [0xFFu8, 0xFE, 0x00, 0x80, 0x81])
        .expect("write binary file");
    std::fs::write(dir.path().join("ok.rs"), "pub fn ok() {}").expect("write good file");

    let binary_id = eng
        .register_code_source(&workspace_id, Some(&root), "blob.rs", "binary-placeholder")
        .await
        .expect("register binary source");
    // The whole point: reading a binary file returns an outcome, never an Err
    // that would abort the directory run.
    let binary_outcome = read_and_index(
        &eng,
        &context,
        &workspace_id,
        &binary_id,
        "blob.rs",
        dir.path(),
        Some(&run_id),
    )
    .await
    .expect("a binary file must NOT abort the run (returns a failed outcome)");
    assert!(
        binary_outcome.failed,
        "binary file must be recorded failed, got {:?}",
        binary_outcome.parse_status
    );
    assert_eq!(
        binary_outcome.parse_status,
        KnowledgeCodeParseStatus::Failed
    );
    // No misleading code language is asserted for a binary file.
    assert!(
        binary_outcome.language.is_none(),
        "a binary/unreadable file must not claim a code language"
    );

    // The good sibling still indexes after the binary failure.
    let good_id = eng
        .register_code_source(&workspace_id, Some(&root), "ok.rs", "ok-placeholder")
        .await
        .expect("register good source");
    let good_outcome = read_and_index(
        &eng,
        &context,
        &workspace_id,
        &good_id,
        "ok.rs",
        dir.path(),
        Some(&run_id),
    )
    .await
    .expect("good file indexes");
    assert!(!good_outcome.failed);

    // The durable repair surface holds the binary file for re-processing with a
    // typed READ_ERROR class (this is what makes MT-108 a recovery surface).
    let open = pg
        .db
        .get_open_knowledge_code_repair(&binary_id)
        .await
        .expect("query repair queue")
        .expect("a READ_ERROR repair entry must be enqueued for the binary file");
    assert_eq!(open.reason_class, KnowledgeCodeRepairReason::ReadError);
    assert_eq!(open.source_id, binary_id);
    assert_eq!(open.relative_path, "blob.rs");
    assert_eq!(open.attempts, 0);

    // Re-indexing the same binary file REFRESHES the open entry (does not
    // multiply rows): exactly one repair entry exists for the workspace.
    read_and_index(
        &eng,
        &context,
        &workspace_id,
        &binary_id,
        "blob.rs",
        dir.path(),
        Some(&run_id),
    )
    .await
    .expect("re-read binary");
    let all = pg
        .db
        .list_knowledge_code_repairs(&workspace_id)
        .await
        .expect("list repairs");
    assert_eq!(
        all.len(),
        1,
        "re-failing must refresh the open entry, not multiply rows: {all:?}"
    );

    // The good symbol is present despite the binary sibling.
    let symbols = pg
        .db
        .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
        .await
        .expect("symbols");
    assert!(symbols.iter().any(|s| s.entity_key == "rust:ok.rs#ok"));

    // Resolving the entry after a (hypothetical) fixed re-index is terminal.
    let resolved = pg
        .db
        .resolve_knowledge_code_repair(&open.code_repair_id, &good_outcome.receipt_event_id)
        .await
        .expect("resolve repair");
    assert_eq!(resolved.state, "resolved");
    assert!(pg
        .db
        .get_open_knowledge_code_repair(&binary_id)
        .await
        .expect("query")
        .is_none());
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt107_reindex_replaces_moved_symbol_definition_span() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt107_reindex_replaces_moved_symbol_definition_span: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let v1 = "pub fn target() -> i32 { 1 }\n";
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/lib.rs", v1)
        .await
        .expect("register v1");
    eng.index_code_source(&context, &workspace_id, &source_id, "src/lib.rs", v1, None)
        .await
        .expect("index v1");

    let symbol_key = "rust:src/lib.rs#target";
    let symbol = pg
        .db
        .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::Symbol, symbol_key)
        .await
        .expect("lookup symbol")
        .expect("target symbol");
    let v1_span_ids = pg
        .db
        .list_knowledge_entity_span_ids(&symbol.entity_id)
        .await
        .expect("v1 symbol spans");
    assert_eq!(
        v1_span_ids.len(),
        1,
        "initial symbol has one definition span"
    );
    let old_span_id = v1_span_ids[0].clone();

    let v2 = "// moved down\n\npub fn target() -> i32 { 2 }\n";
    let refreshed_source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/lib.rs", v2)
        .await
        .expect("register v2");
    assert_eq!(
        source_id, refreshed_source_id,
        "same path refreshes same source"
    );
    eng.index_code_source(&context, &workspace_id, &source_id, "src/lib.rs", v2, None)
        .await
        .expect("index v2");

    let span_ids = pg
        .db
        .list_knowledge_entity_span_ids(&symbol.entity_id)
        .await
        .expect("reindexed symbol spans");
    assert!(
        !span_ids.contains(&old_span_id),
        "reindex must unlink the old definition span {old_span_id}, not keep it as current evidence"
    );
    let mut ast_spans = Vec::new();
    for span_id in span_ids {
        let span = pg
            .db
            .get_knowledge_span(&span_id)
            .await
            .expect("span lookup")
            .expect("span exists");
        if span.source_id == source_id && matches!(span.span_kind, KnowledgeSpanKind::Ast) {
            ast_spans.push(span);
        }
    }

    let expected_start = v2.find("pub fn target").expect("expected target byte") as i64;
    assert_eq!(
        ast_spans.len(),
        1,
        "reindex must replace stale AST definition evidence, not accumulate old spans: {ast_spans:?}"
    );
    assert_eq!(ast_spans[0].range_start, expected_start);
    assert_eq!(ast_spans[0].line_start, Some(3));

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let payload = build_monaco_payload(
        eng.db(),
        &workspace_id,
        "src/lib.rs",
        &sha256_hex(v2.as_bytes()),
        &parser_version,
    )
    .await
    .expect("monaco payload after reindex");
    let lens = payload
        .entries
        .iter()
        .find(|entry| entry.symbol_key == symbol_key)
        .expect("target lens entry");
    assert_eq!(
        lens.definition.start_line, 3,
        "Monaco/nav projection must consume the refreshed definition span"
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

    let add_symbol = pg
        .db
        .get_knowledge_entity_by_identity(
            &workspace_id,
            KnowledgeEntityKind::Symbol,
            "rust:src/lib.rs#add",
        )
        .await
        .expect("lookup add symbol")
        .expect("add symbol");
    let validates_edge = pg
        .db
        .list_knowledge_edges_for_entity(&add_symbol.entity_id)
        .await
        .expect("add edges")
        .into_iter()
        .find(|edge| {
            edge.edge_type == KnowledgeEdgeType::Validates
                && edge.target_entity_id == add_symbol.entity_id
        })
        .expect("validates edge targeting add");
    let validates_span_id = pg
        .db
        .list_knowledge_edge_span_ids(&validates_edge.edge_id)
        .await
        .expect("validates edge spans")
        .into_iter()
        .next()
        .expect("validates edge carries span");
    let validates_span = pg
        .db
        .get_knowledge_span(&validates_span_id)
        .await
        .expect("get validates span")
        .expect("validates span exists");
    let validates_range = (
        validates_span.line_start.unwrap_or(0),
        validates_span.line_end.unwrap_or(0),
    );
    assert!(
        add.references
            .iter()
            .any(|range| (range.start_line, range.end_line) == validates_range),
        "Monaco references must include validates/test evidence range {validates_range:?}, got {:?}",
        add.references
    );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt110_context_bundle_rejects_symbol_path_mismatch() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt110_context_bundle_rejects_symbol_path_mismatch: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let a_src = "pub fn focus() {}\n";
    let b_src = "pub fn other() {}\n";
    for (path, text) in [("src/a.rs", a_src), ("src/b.rs", b_src)] {
        let source_id = eng
            .register_code_source(&workspace_id, Some(&root), path, text)
            .await
            .expect("register source");
        eng.index_code_source(&context, &workspace_id, &source_id, path, text, None)
            .await
            .expect("index source");
    }

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let err = build_code_context_bundle(
        eng.db(),
        "KTR-bundle-path-test",
        "SR-bundle-path-test",
        &workspace_id,
        "src/b.rs",
        "rust:src/a.rs#focus",
        &sha256_hex(b_src.as_bytes()),
        &parser_version,
        DEFAULT_CODE_CONTEXT_TOKEN_BUDGET,
    )
    .await
    .expect_err("symbol from src/a.rs must not be bundled under src/b.rs staleness");
    assert!(
        err.to_string().contains("does not belong to relative_path"),
        "unexpected error: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt110_context_bundle_reports_over_budget_focus_doc() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt110_context_bundle_reports_over_budget_focus_doc: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let eng = engine(&pg).await;
    let context = ctx();
    let root = make_root(&pg, &workspace_id).await;
    let long_doc_line = "/// ".to_string() + &"long documented behavior ".repeat(80);
    let src = format!("{long_doc_line}\npub fn documented() {{}}\n");
    let source_id = eng
        .register_code_source(&workspace_id, Some(&root), "src/doc.rs", &src)
        .await
        .expect("register");
    eng.index_code_source(
        &context,
        &workspace_id,
        &source_id,
        "src/doc.rs",
        &src,
        None,
    )
    .await
    .expect("index");

    let parser_version = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
    let bundle = build_code_context_bundle(
        eng.db(),
        "KTR-bundle-budget-test",
        "SR-bundle-budget-test",
        &workspace_id,
        "src/doc.rs",
        "rust:src/doc.rs#documented",
        &sha256_hex(src.as_bytes()),
        &parser_version,
        1,
    )
    .await
    .expect("bundle");
    let accounting = &bundle.allowed_context["token_accounting"];
    assert!(
        accounting["estimated_tokens_used"].as_u64().unwrap_or(0) > 1,
        "focus doc is emitted in full, so accounting must report true over-budget usage: {accounting}"
    );
    assert_eq!(
        accounting["truncated"], true,
        "over-budget focus content must be surfaced as truncated/over-budget"
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt111_runtime_index_receipt_records_perf_budget_verdict() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt111_runtime_index_receipt_records_perf_budget_verdict: no PostgreSQL");
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
    let outcome = eng
        .index_code_source(
            &context,
            &workspace_id,
            &source_id,
            "src/lib.rs",
            RUST_SRC,
            None,
        )
        .await
        .expect("index");

    let code_file = pg
        .db
        .get_knowledge_code_file_by_source(&source_id)
        .await
        .expect("code file")
        .expect("code file row");
    assert_eq!(
        code_file.last_index_receipt_event_id.as_deref(),
        Some(outcome.receipt_event_id.as_str())
    );
    let events = pg
        .db
        .list_kernel_events_for_aggregate("knowledge_code_index_file", &source_id)
        .await
        .expect("list index events");
    let receipt = events
        .iter()
        .find(|event| event.event_id == outcome.receipt_event_id)
        .expect("index receipt event");
    let perf = &receipt.payload["perf_budget"];
    assert_eq!(perf["relative_path"], "src/lib.rs");
    assert_eq!(perf["within_budget"], true);
    assert!(
        perf["elapsed_ms"].as_f64().unwrap_or(0.0) >= 0.0,
        "receipt must record measured elapsed_ms: {perf}"
    );
    assert!(
        perf["allowed_ms"].as_f64().unwrap_or(0.0) > 0.0,
        "receipt must record allowed_ms: {perf}"
    );
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
