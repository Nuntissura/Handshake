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

// ---------------------------------------------------------------------------
// MT-053 KnowledgeEntityTables
// ---------------------------------------------------------------------------

mod mt_053_entities {
    use super::*;
    use handshake_core::storage::knowledge::{
        KnowledgeEntityKind, KnowledgeEntityLifecycle, NewKnowledgeEntity,
    };

    fn symbol_entity(workspace_id: &str, key: &str, spans: Vec<String>) -> NewKnowledgeEntity {
        NewKnowledgeEntity {
            workspace_id: workspace_id.to_string(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: key.to_string(),
            display_name: "KernelEventType".to_string(),
            detection_provenance: json!({
                "extractor": "rust_ast",
                "extractor_version": "v1",
                "method": "ast_walk"
            }),
            primary_source_id: None,
            detected_in_run: None,
            evidence_span_ids: spans,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn entity_upsert_keeps_stable_id_and_merges_evidence() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP entity_upsert_keeps_stable_id_and_merges_evidence: no PostgreSQL");
            return;
        };
        let (workspace_id, source_id) = source_fixture(&pg).await;
        let span_a = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 100))
            .await
            .expect("span a");
        let span_b = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 100, 200))
            .await
            .expect("span b");

        let key = "handshake_core::kernel::KernelEventType";
        let first = pg
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                key,
                vec![span_a.span_id.clone()],
            ))
            .await
            .expect("first detection");
        assert!(first.entity_id.starts_with("KEN-"));
        assert_eq!(first.lifecycle_state, KnowledgeEntityLifecycle::Active);

        // Re-detection: same identity, stable id, merged evidence.
        let second = pg
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                key,
                vec![span_a.span_id.clone(), span_b.span_id.clone()],
            ))
            .await
            .expect("re-detection");
        assert_eq!(second.entity_id, first.entity_id, "entity id must be stable");

        let evidence = pg
            .db
            .list_knowledge_entity_span_ids(&first.entity_id)
            .await
            .expect("list evidence");
        assert_eq!(evidence.len(), 2, "evidence spans must merge, not duplicate");

        let by_identity = pg
            .db
            .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::Symbol, key)
            .await
            .expect("get by identity")
            .expect("entity by identity");
        assert_eq!(by_identity.entity_id, first.entity_id);

        let listed = pg
            .db
            .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
            .await
            .expect("list by kind");
        assert_eq!(listed.len(), 1);

        let retired = pg
            .db
            .retire_knowledge_entity(&first.entity_id)
            .await
            .expect("retire entity");
        assert_eq!(retired.lifecycle_state, KnowledgeEntityLifecycle::Retired);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_spec_entity_kind_is_storable() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP every_spec_entity_kind_is_storable: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id) = source_fixture(&pg).await;

        // Rust enum and SQL CHECK list must agree for all 21 kinds.
        for kind in KnowledgeEntityKind::all() {
            let entity = pg
                .db
                .upsert_knowledge_entity(NewKnowledgeEntity {
                    workspace_id: workspace_id.clone(),
                    entity_kind: *kind,
                    entity_key: format!("key-{}", kind.as_str()),
                    display_name: format!("entity {}", kind.as_str()),
                    detection_provenance: json!({"extractor": "kind_matrix_test"}),
                    primary_source_id: None,
                    detected_in_run: None,
                    evidence_span_ids: vec![],
                })
                .await
                .unwrap_or_else(|err| panic!("kind {} must be storable: {err}", kind.as_str()));
            assert_eq!(entity.entity_kind, *kind, "kind must round-trip");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn entity_evidence_is_fk_guarded() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP entity_evidence_is_fk_guarded: no PostgreSQL");
            return;
        };
        let (workspace_id, source_id) = source_fixture(&pg).await;

        // Evidence pointing at a nonexistent span must roll the upsert back.
        let err = pg
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                "ghost::evidence",
                vec!["KSP-00000000000000000000000000000000".to_string()],
            ))
            .await
            .expect_err("ghost evidence span must violate the FK");
        assert!(err.to_string().contains("foreign key"), "got {err}");
        // Transactionality: the entity row must not exist after the rollback.
        let ghost = pg
            .db
            .get_knowledge_entity_by_identity(
                &workspace_id,
                KnowledgeEntityKind::Symbol,
                "ghost::evidence",
            )
            .await
            .expect("lookup ghost identity");
        assert!(ghost.is_none(), "failed evidence link must roll back the entity insert");

        // Spans referenced as evidence are delete-protected (RESTRICT).
        let span = pg
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 50))
            .await
            .expect("span");
        pg.db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                "anchored::symbol",
                vec![span.span_id.clone()],
            ))
            .await
            .expect("entity with evidence");
        let mut conn = pg.raw_connection().await;
        let err = sqlx::query("DELETE FROM knowledge_spans WHERE span_id = $1")
            .bind(&span.span_id)
            .execute(&mut conn)
            .await
            .expect_err("span referenced as entity evidence must be delete-protected");
        assert!(err.to_string().contains("foreign key"), "got {err}");
    }
}
