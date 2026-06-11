//! WP-KERNEL-009 PostgresEventLedgerCore integration tests against REAL
//! Handshake-managed PostgreSQL: MT-055 (KnowledgeSpanTables), MT-053
//! (KnowledgeEntityTables), MT-054 (KnowledgeEdgeTables).
//!
//! Spans are tested first: they are the minimum citeable evidence unit and
//! both entities and edges carry REQUIRED span refs.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
    KnowledgeRootKind, KnowledgeSourceKind, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeSource,
    NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_SPAN: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

/// workspace -> root -> file source fixture; returns (workspace_id, source_id).
async fn source_fixture(pg: &KnowledgePg) -> (String, String) {
    let workspace_id = pg.create_workspace().await;
    let root = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("src/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*.rs"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("create root");
    let source = pg
        .db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: Some(root.root_id),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("kernel/mod.rs".to_string()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: HASH_SRC.to_string(),
            size_bytes: Some(4096),
            provenance: json!({"discovered_by": "graph_test"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("create source");
    (workspace_id, source.source_id)
}

fn byte_span(source_id: &str, start: i64, end: i64) -> NewKnowledgeSpan {
    NewKnowledgeSpan {
        source_id: source_id.to_string(),
        span_kind: KnowledgeSpanKind::Byte,
        range_start: start,
        range_end: end,
        line_start: Some(10),
        line_end: Some(24),
        section_path: Some("impl KernelEventType".to_string()),
        content_sha256: HASH_SPAN.to_string(),
        parser_version: "rust_ast_v1".to_string(),
        extraction_receipt_event_id: None,
        index_run_id: None,
        display_snippet: Some("pub enum KernelEventType {".to_string()),
    }
}

// ---------------------------------------------------------------------------
// MT-055 KnowledgeSpanTables
// ---------------------------------------------------------------------------

mod mt_055_spans {
    use super::*;
    use handshake_core::storage::StorageError;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn span_roundtrip_and_source_anchoring() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP span_roundtrip_and_source_anchoring: no PostgreSQL");
            return;
        };
        let (_ws, source_id) = source_fixture(&pg).await;

        let span = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 2048, 4096))
            .await
            .expect("create span");
        assert!(span.span_id.starts_with("KSP-"));
        assert_eq!(span.span_kind, KnowledgeSpanKind::Byte);
        assert_eq!(span.parser_version, "rust_ast_v1");

        let fetched = pg
            .db
            .get_knowledge_span(&span.span_id)
            .await
            .expect("get span")
            .expect("span exists");
        assert_eq!(fetched, span);

        let second = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 1024))
            .await
            .expect("create second span");
        let listed = pg
            .db
            .list_knowledge_spans_for_source(&source_id)
            .await
            .expect("list spans");
        assert_eq!(listed.len(), 2);
        // Ordered by range_start.
        assert_eq!(listed[0].span_id, second.span_id);
        assert_eq!(listed[1].span_id, span.span_id);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn span_constraints_fail_closed() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP span_constraints_fail_closed: no PostgreSQL");
            return;
        };
        let (_ws, source_id) = source_fixture(&pg).await;

        // Inverted range: Rust-level typed validation.
        let mut inverted = byte_span(&source_id, 100, 50);
        inverted.line_start = None;
        inverted.line_end = None;
        let err = pg
            .db
            .create_knowledge_span(inverted)
            .await
            .expect_err("inverted range must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

        // Span anchored to a nonexistent source: FK violation.
        let err = pg
            .db
            .create_knowledge_span(byte_span(
                "KSRC-00000000000000000000000000000000",
                0,
                10,
            ))
            .await
            .expect_err("span must be anchored to a real KnowledgeSource");
        assert!(err.to_string().contains("foreign key"), "got {err}");

        // DB-level range CHECK when the app layer is bypassed.
        let mut conn = pg.raw_connection().await;
        let err = sqlx::query(
            "INSERT INTO knowledge_spans
                 (span_id, source_id, span_kind, range_start, range_end, content_sha256, parser_version)
             VALUES ('KSP-00000000000000000000000000000001', $1, 'byte', 500, 100, $2, 'v1')",
        )
        .bind(&source_id)
        .bind(HASH_SPAN)
        .execute(&mut conn)
        .await
        .expect_err("DB must reject inverted ranges");
        assert!(
            err.to_string().contains("chk_knowledge_spans_range"),
            "unexpected: {err}"
        );

        // Deleting the source cascades its spans (anchored evidence cannot
        // outlive the source registration).
        let span = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 10))
            .await
            .expect("create span");
        sqlx::query("DELETE FROM knowledge_sources WHERE source_id = $1")
            .bind(&source_id)
            .execute(&mut conn)
            .await
            .expect("delete source");
        let gone = pg
            .db
            .get_knowledge_span(&span.span_id)
            .await
            .expect("get span after source delete");
        assert!(gone.is_none(), "spans must cascade with their source");
    }
}
