//! WP-KERNEL-009 knowledge graph integration tests against the real
//! Handshake-managed embedded SurrealDB store: MT-055 (KnowledgeSpanTables),
//! MT-053 (KnowledgeEntityTables), MT-054 (KnowledgeEdgeTables).
//!
//! Spans are tested first: they are the minimum citeable evidence unit and
//! both entities and edges carry REQUIRED span refs.

#[path = "knowledge_ingestion_support.rs"]
mod knowledge_surreal_support;

use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
    KnowledgeRootKind, KnowledgeSourceKind, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeSource,
    NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use handshake_core::storage::surreal::{
    RowFilter, SurrealTestInspectorError, TestFieldMutation, TestMutationValue,
};
use knowledge_surreal_support::{
    open_embedded_store as embedded_knowledge, EmbeddedKnowledgeStore as KnowledgeSurreal,
};
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_SPAN: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

/// workspace -> root -> file source fixture; returns (workspace_id, source_id).
async fn source_fixture(store: &KnowledgeSurreal) -> (String, String) {
    let workspace_id = store.create_workspace().await;
    let root = store
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
    let source = store
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
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP span_roundtrip_and_source_anchoring: no embedded store");
            return;
        };
        let (_ws, source_id) = source_fixture(&store).await;

        let span = store
            .db
            .create_knowledge_span(byte_span(&source_id, 2048, 4096))
            .await
            .expect("create span");
        assert!(span.span_id.starts_with("KSP-"));
        assert_eq!(span.span_kind, KnowledgeSpanKind::Byte);
        assert_eq!(span.parser_version, "rust_ast_v1");

        let fetched = store
            .db
            .get_knowledge_span(&span.span_id)
            .await
            .expect("get span")
            .expect("span exists");
        assert_eq!(fetched, span);

        let second = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 1024))
            .await
            .expect("create second span");
        let listed = store
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
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP span_constraints_fail_closed: no embedded store");
            return;
        };
        let (_ws, source_id) = source_fixture(&store).await;

        // Inverted range: Rust-level typed validation.
        let mut inverted = byte_span(&source_id, 100, 50);
        inverted.line_start = None;
        inverted.line_end = None;
        let err = store
            .db
            .create_knowledge_span(inverted)
            .await
            .expect_err("inverted range must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

        // Span anchored to a nonexistent source: the embedded reference guard.
        let err = store
            .db
            .create_knowledge_span(byte_span("KSRC-00000000000000000000000000000000", 0, 10))
            .await
            .expect_err("span must be anchored to a real KnowledgeSource");
        assert!(matches!(err, StorageError::Database(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn span_embedded_schema_and_cascade_probes() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP span_embedded_schema_and_cascade_probes: no embedded store");
            return;
        };
        let (_workspace_id, source_id) = source_fixture(&store).await;
        let inspector = store.storage.test_inspector();
        let spans = inspector
            .table_selector("knowledge_spans")
            .await
            .expect("select knowledge spans table");
        let sources = inspector
            .table_selector("knowledge_sources")
            .await
            .expect("select knowledge sources table");
        let source_reference = inspector
            .references_to(&sources)
            .await
            .expect("inspect span source reference")
            .into_iter()
            .find(|reference| {
                reference.source_table() == "knowledge_spans"
                    && reference.source_field() == "source_id"
            })
            .expect("knowledge_spans.source_id reference");
        assert_eq!(source_reference.target_table(), "knowledge_sources");
        assert_eq!(source_reference.on_delete(), "CASCADE");

        // Closed mutator + live schema prove the direct range guard without
        // reconstructing a caller-authored SurrealQL statement.
        let span = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 10))
            .await
            .expect("create valid span");
        let invalid_span_id = "KSP-00000000000000000000000000000001";
        let range_end = spans.field("range_end").expect("select span range_end");
        let span_id = spans.field("span_id").expect("select span id");
        let err = store
            .storage
            .test_mutator()
            .duplicate_row(
                &spans,
                &span.span_id,
                invalid_span_id,
                &[
                    TestFieldMutation::new(span_id, TestMutationValue::string(invalid_span_id)),
                    TestFieldMutation::new(range_end, TestMutationValue::i64(-1)),
                ],
            )
            .await
            .expect_err("live span range schema must reject an inverted direct insert");
        assert!(
            matches!(err, SurrealTestInspectorError::Storage(_)),
            "got {err:?}"
        );
        assert!(
            store
                .db
                .get_knowledge_span(invalid_span_id)
                .await
                .expect("inspect rejected span")
                .is_none(),
            "rejected direct span must not persist"
        );

        // The catalog-declared cascade is exercised against a real source row.
        store
            .storage
            .test_mutator()
            .delete_row(&sources, source_id.as_str())
            .await
            .expect("delete source through embedded mutator");
        assert!(
            store
                .db
                .get_knowledge_span(&span.span_id)
                .await
                .expect("get span after source delete")
                .is_none(),
            "spans must cascade with their source"
        );
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
    use handshake_core::storage::StorageError;

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
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP entity_upsert_keeps_stable_id_and_merges_evidence: no embedded store");
            return;
        };
        let (workspace_id, source_id) = source_fixture(&store).await;
        let span_a = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 100))
            .await
            .expect("span a");
        let span_b = store
            .db
            .create_knowledge_span(byte_span(&source_id, 100, 200))
            .await
            .expect("span b");

        let key = "handshake_core::kernel::KernelEventType";
        let first = store
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
        let second = store
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                key,
                vec![span_a.span_id.clone(), span_b.span_id.clone()],
            ))
            .await
            .expect("re-detection");
        assert_eq!(
            second.entity_id, first.entity_id,
            "entity id must be stable"
        );

        let evidence = store
            .db
            .list_knowledge_entity_span_ids(&first.entity_id)
            .await
            .expect("list evidence");
        assert_eq!(
            evidence.len(),
            2,
            "evidence spans must merge, not duplicate"
        );

        let by_identity = store
            .db
            .get_knowledge_entity_by_identity(&workspace_id, KnowledgeEntityKind::Symbol, key)
            .await
            .expect("get by identity")
            .expect("entity by identity");
        assert_eq!(by_identity.entity_id, first.entity_id);

        let listed = store
            .db
            .list_knowledge_entities_by_kind(&workspace_id, KnowledgeEntityKind::Symbol)
            .await
            .expect("list by kind");
        assert_eq!(listed.len(), 1);

        let retired = store
            .db
            .retire_knowledge_entity(&first.entity_id)
            .await
            .expect("retire entity");
        assert_eq!(retired.lifecycle_state, KnowledgeEntityLifecycle::Retired);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_spec_entity_kind_is_storable() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP every_spec_entity_kind_is_storable: no embedded store");
            return;
        };
        let (workspace_id, _source_id) = source_fixture(&store).await;

        // Rust enum and SQL CHECK list must agree for all 21 kinds.
        for kind in KnowledgeEntityKind::all() {
            let entity = store
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
    async fn entity_evidence_is_guarded() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP entity_evidence_is_fk_guarded: no embedded store");
            return;
        };
        let (workspace_id, source_id) = source_fixture(&store).await;

        // Evidence pointing at a nonexistent span must roll the upsert back.
        let err = store
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                "ghost::evidence",
                vec!["KSP-00000000000000000000000000000000".to_string()],
            ))
            .await
            .expect_err("ghost evidence span must violate the embedded reference guard");
        assert!(matches!(err, StorageError::Database(_)), "got {err:?}");
        // Transactionality: the entity row must not exist after the rollback.
        let ghost = store
            .db
            .get_knowledge_entity_by_identity(
                &workspace_id,
                KnowledgeEntityKind::Symbol,
                "ghost::evidence",
            )
            .await
            .expect("lookup ghost identity");
        assert!(
            ghost.is_none(),
            "failed evidence link must roll back the entity insert"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn entity_evidence_delete_guard_is_catalog_backed() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP entity_evidence_delete_guard_is_catalog_backed: no embedded store");
            return;
        };
        let (workspace_id, source_id) = source_fixture(&store).await;
        let span = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 50))
            .await
            .expect("create evidence span");
        let entity = store
            .db
            .upsert_knowledge_entity(symbol_entity(
                &workspace_id,
                "anchored::symbol",
                vec![span.span_id.clone()],
            ))
            .await
            .expect("create entity with evidence");

        let inspector = store.storage.test_inspector();
        let spans = inspector
            .table_selector("knowledge_spans")
            .await
            .expect("select knowledge spans table");
        let evidence_reference = inspector
            .references_to(&spans)
            .await
            .expect("inspect entity evidence reference")
            .into_iter()
            .find(|reference| {
                reference.source_table() == "knowledge_entity_spans"
                    && reference.source_field() == "span_id"
            })
            .expect("knowledge_entity_spans.span_id reference");
        assert_eq!(evidence_reference.target_table(), "knowledge_spans");
        assert_eq!(evidence_reference.on_delete(), "REJECT");

        let err = store
            .storage
            .test_mutator()
            .delete_row(&spans, span.span_id.as_str())
            .await
            .expect_err("entity evidence span must be delete-protected");
        assert!(
            matches!(err, SurrealTestInspectorError::Storage(_)),
            "got {err:?}"
        );
        assert!(
            store
                .db
                .get_knowledge_span(&span.span_id)
                .await
                .expect("get protected evidence span")
                .is_some(),
            "delete rejection must preserve the evidence span"
        );
        assert_eq!(
            store
                .db
                .list_knowledge_entity_span_ids(&entity.entity_id)
                .await
                .expect("list retained entity evidence"),
            vec![span.span_id]
        );
    }
}

// ---------------------------------------------------------------------------
// MT-054 KnowledgeEdgeTables
// ---------------------------------------------------------------------------

mod mt_054_edges {
    use super::*;
    use handshake_core::storage::knowledge::{
        derive_knowledge_relationship_id, KnowledgeEdgeLifecycle, KnowledgeEdgeType,
        KnowledgeEntityKind, NewKnowledgeEdge, NewKnowledgeEntity,
    };
    use handshake_core::storage::StorageError;
    use knowledge_surreal_support::EmbeddedKnowledgeStore as KnowledgeSurreal;

    struct GraphFixture {
        workspace_id: String,
        source_entity_id: String,
        target_entity_id: String,
        span_id: String,
        span2_id: String,
    }

    async fn graph_fixture(store: &KnowledgeSurreal) -> GraphFixture {
        let (workspace_id, source_id) = source_fixture(store).await;
        let span = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 100))
            .await
            .expect("span");
        let span2 = store
            .db
            .create_knowledge_span(byte_span(&source_id, 100, 200))
            .await
            .expect("span2");
        let mk_entity = |kind: KnowledgeEntityKind, key: &str, span: &str| NewKnowledgeEntity {
            workspace_id: workspace_id.clone(),
            entity_kind: kind,
            entity_key: key.to_string(),
            display_name: key.to_string(),
            detection_provenance: json!({"extractor": "edge_test"}),
            primary_source_id: None,
            detected_in_run: None,
            evidence_span_ids: vec![span.to_string()],
        };
        let source_entity = store
            .db
            .upsert_knowledge_entity(mk_entity(
                KnowledgeEntityKind::Symbol,
                "kernel::KernelEventType",
                &span.span_id,
            ))
            .await
            .expect("source entity");
        let target_entity = store
            .db
            .upsert_knowledge_entity(mk_entity(
                KnowledgeEntityKind::File,
                "src/kernel/mod.rs",
                &span2.span_id,
            ))
            .await
            .expect("target entity");
        GraphFixture {
            workspace_id,
            source_entity_id: source_entity.entity_id,
            target_entity_id: target_entity.entity_id,
            span_id: span.span_id,
            span2_id: span2.span_id,
        }
    }

    fn defined_in_edge(fx: &GraphFixture, confidence: f64, spans: Vec<String>) -> NewKnowledgeEdge {
        NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::Defines,
            source_entity_id: fx.target_entity_id.clone(),
            target_entity_id: fx.source_entity_id.clone(),
            extractor_version: "rust_ast_v1".to_string(),
            confidence,
            detected_in_run: None,
            evidence_span_ids: spans,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn relationship_id_is_deterministic_across_reindex_runs() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!(
                "SKIP relationship_id_is_deterministic_across_reindex_runs: no embedded store"
            );
            return;
        };
        let fx = graph_fixture(&store).await;

        // Pure derivation is stable and matches the documented format.
        let expected = derive_knowledge_relationship_id(
            KnowledgeEdgeType::Defines,
            KnowledgeEntityKind::File,
            "src/kernel/mod.rs",
            KnowledgeEntityKind::Symbol,
            "kernel::KernelEventType",
        );
        assert_eq!(
            expected,
            derive_knowledge_relationship_id(
                KnowledgeEdgeType::Defines,
                KnowledgeEntityKind::File,
                "src/kernel/mod.rs",
                KnowledgeEntityKind::Symbol,
                "kernel::KernelEventType",
            ),
            "derivation must be deterministic"
        );
        assert!(expected.starts_with("KREL-") && expected.len() == 5 + 64);

        // First extraction run.
        let first = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.8, vec![fx.span_id.clone()]))
            .await
            .expect("first extraction");
        assert_eq!(first.relationship_id, expected);

        // Re-extraction (simulated second index run, higher confidence, new
        // evidence): same relationship_id, same row, no duplicate.
        let second = store
            .db
            .upsert_knowledge_edge(defined_in_edge(
                &fx,
                0.95,
                vec![fx.span_id.clone(), fx.span2_id.clone()],
            ))
            .await
            .expect("re-extraction");
        assert_eq!(second.edge_id, first.edge_id, "edge row must be stable");
        assert_eq!(second.relationship_id, expected);
        assert!((second.confidence - 0.95).abs() < f64::EPSILON);

        let by_rel = store
            .db
            .get_knowledge_edge_by_relationship_id(&fx.workspace_id, &expected)
            .await
            .expect("get by relationship id")
            .expect("edge by relationship id");
        assert_eq!(by_rel.edge_id, first.edge_id);

        let evidence = store
            .db
            .list_knowledge_edge_span_ids(&first.edge_id)
            .await
            .expect("list edge evidence");
        assert_eq!(evidence.len(), 2, "evidence must merge across runs");

        let touching = store
            .db
            .list_knowledge_edges_for_entity(&fx.source_entity_id)
            .await
            .expect("edges for entity");
        assert_eq!(touching.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edges_require_span_evidence() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP edges_require_span_evidence_at_every_layer: no embedded store");
            return;
        };
        let fx = graph_fixture(&store).await;

        // Rust layer: empty evidence is a typed Validation error.
        let err = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.9, vec![]))
            .await
            .expect_err("edge without span refs must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edge_embedded_insert_without_evidence_is_rejected() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP edge_embedded_insert_without_evidence_is_rejected: no embedded store");
            return;
        };
        let fx = graph_fixture(&store).await;
        let inspector = store.storage.test_inspector();
        let edges = inspector
            .table_selector("knowledge_edges")
            .await
            .expect("select knowledge edges table");
        let spans = inspector
            .table_selector("knowledge_spans")
            .await
            .expect("select knowledge spans table");
        let edge_fields = inspector
            .table_catalog("knowledge_edges")
            .await
            .expect("inspect knowledge edge catalog")
            .fields;
        assert!(
            edge_fields
                .iter()
                .all(|field| field.name != "evidence_span_ids"),
            "edge evidence must remain in the dedicated knowledge_edge_spans relation"
        );
        let edge_span_reference = inspector
            .references_to(&spans)
            .await
            .expect("inspect edge evidence reference")
            .into_iter()
            .find(|reference| {
                reference.source_table() == "knowledge_edge_spans"
                    && reference.source_field() == "span_id"
            })
            .expect("knowledge_edge_spans.span_id reference");
        assert_eq!(edge_span_reference.on_delete(), "REJECT");

        let err = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.9, vec![]))
            .await
            .expect_err("typed edge insert without evidence must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
        assert_eq!(
            inspector
                .row_count(&edges, RowFilter::All)
                .await
                .expect("count edge rows after rejected insert"),
            0,
            "rejected edge insert must not persist an edge row"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edge_last_evidence_delete_guard_is_catalog_backed() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP edge_last_evidence_delete_guard_is_catalog_backed: no embedded store");
            return;
        };
        let fx = graph_fixture(&store).await;
        let edge = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.9, vec![fx.span_id.clone()]))
            .await
            .expect("edge with evidence");

        let inspector = store.storage.test_inspector();
        let spans = inspector
            .table_selector("knowledge_spans")
            .await
            .expect("select knowledge spans table");
        let references = inspector
            .references_to(&spans)
            .await
            .expect("inspect evidence references");
        let edge_span_reference = references
            .iter()
            .find(|reference| {
                reference.source_table() == "knowledge_edge_spans"
                    && reference.source_field() == "span_id"
            })
            .expect("knowledge_edge_spans.span_id reference");
        assert_eq!(edge_span_reference.on_delete(), "REJECT");

        // The embedded equivalent protects the referenced evidence record at
        // the live schema boundary; no unsupported join-row trigger is faked.
        let err = store
            .storage
            .test_mutator()
            .delete_row(&spans, fx.span_id.as_str())
            .await
            .expect_err("last edge evidence span must be delete-protected");
        assert!(
            matches!(err, SurrealTestInspectorError::Storage(_)),
            "got {err:?}"
        );
        assert_eq!(
            store
                .db
                .list_knowledge_edge_span_ids(&edge.edge_id)
                .await
                .expect("list retained edge evidence"),
            vec![fx.span_id]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edge_lifecycle_and_conflict_markers() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP edge_lifecycle_and_conflict_markers: no embedded store");
            return;
        };
        let fx = graph_fixture(&store).await;
        let edge = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.7, vec![fx.span_id.clone()]))
            .await
            .expect("edge");

        // Conflicted without a marker: typed Validation (Rust) ...
        let err = store
            .db
            .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Conflicted, None)
            .await
            .expect_err("conflicted edge requires a conflict marker");
        assert!(matches!(err, StorageError::Validation(_)));

        let conflicted = store
            .db
            .set_knowledge_edge_lifecycle(
                &edge.edge_id,
                KnowledgeEdgeLifecycle::Conflicted,
                Some(json!({"reason": "duplicate definition", "with": ["KREL-x"]})),
            )
            .await
            .expect("mark conflicted with marker");
        assert_eq!(
            conflicted.lifecycle_state,
            KnowledgeEdgeLifecycle::Conflicted
        );

        let retired = store
            .db
            .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Retired, None)
            .await
            .expect("retire edge");
        assert_eq!(retired.lifecycle_state, KnowledgeEdgeLifecycle::Retired);
        assert!(
            retired.conflict_marker.is_none(),
            "leaving conflicted clears the marker"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edge_conflict_shape_is_typed_and_catalog_backed() {
        let Some(store) = embedded_knowledge().await else {
            eprintln!("SKIP edge_conflict_shape_is_typed_and_catalog_backed: no embedded store");
            return;
        };
        let fx = graph_fixture(&store).await;
        let edge = store
            .db
            .upsert_knowledge_edge(defined_in_edge(&fx, 0.7, vec![fx.span_id.clone()]))
            .await
            .expect("edge");

        let inspector = store.storage.test_inspector();
        let catalog = inspector
            .table_catalog("knowledge_edges")
            .await
            .expect("inspect knowledge edge catalog");
        let lifecycle = catalog
            .fields
            .iter()
            .find(|field| field.name == "lifecycle_state")
            .expect("edge lifecycle field");
        assert!(lifecycle.kind.contains("conflicted"));
        let marker = catalog
            .fields
            .iter()
            .find(|field| field.name == "conflict_marker")
            .expect("edge conflict marker field");
        assert!(marker.kind.contains("option"));
        assert!(marker.kind.contains("object"));

        let err = store
            .db
            .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Conflicted, None)
            .await
            .expect_err("conflicted edge without a marker must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
        let unchanged = store
            .db
            .get_knowledge_edge(&edge.edge_id)
            .await
            .expect("read edge after rejected conflict shape")
            .expect("edge remains after rejected conflict shape");
        assert_eq!(unchanged.lifecycle_state, KnowledgeEdgeLifecycle::Active);
        assert!(unchanged.conflict_marker.is_none());
    }

    /// HARDENING (MT-054): the relationship_id derivation must be
    /// separator-injective. entity_key is free text and legitimately contains
    /// `|` and `:` (file paths, FQNs, spec anchors). A plain delimiter-joined
    /// preimage aliased two distinct edges onto one relationship_id:
    ///
    ///   edge A: (file, "p")           -> (folder, "x|folder:y")
    ///   edge B: (file, "p|folder:x")  -> (folder, "y")
    ///
    /// both flatten to `...|file:p|folder:x|folder:y` under the old v1 scheme.
    /// Length-prefixing makes the framing injective, so the two derive DISTINCT
    /// ids. This test proves it at the pure-derivation and embedded persistence
    /// layers, where the old behavior silently merged the second edge into the
    /// first under the relationship uniqueness constraint.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn relationship_id_resists_separator_injection_collision() {
        // Pure-derivation layer: the classic separator-injection collision pair
        // must derive DISTINCT relationship_ids.
        let rel_a = derive_knowledge_relationship_id(
            KnowledgeEdgeType::Contains,
            KnowledgeEntityKind::File,
            "p",
            KnowledgeEntityKind::Folder,
            "x|folder:y",
        );
        let rel_b = derive_knowledge_relationship_id(
            KnowledgeEdgeType::Contains,
            KnowledgeEntityKind::File,
            "p|folder:x",
            KnowledgeEntityKind::Folder,
            "y",
        );
        assert_ne!(
            rel_a, rel_b,
            "separator-injection pair must derive distinct relationship_ids \
             (length-prefixed framing must be injective)"
        );
        assert!(rel_a.starts_with("KREL-") && rel_a.len() == 5 + 64);
        assert!(rel_b.starts_with("KREL-") && rel_b.len() == 5 + 64);

        // Determinism is preserved under the new framing.
        assert_eq!(
            rel_a,
            derive_knowledge_relationship_id(
                KnowledgeEdgeType::Contains,
                KnowledgeEntityKind::File,
                "p",
                KnowledgeEntityKind::Folder,
                "x|folder:y",
            ),
            "v2 derivation must stay deterministic"
        );

        // A second classic alias pair where the `:` between kind and key is the
        // injected byte: (symbol, "a") vs (symbol, "a") with a moved boundary.
        let rel_c = derive_knowledge_relationship_id(
            KnowledgeEdgeType::References,
            KnowledgeEntityKind::Symbol,
            "mod::a",
            KnowledgeEntityKind::Symbol,
            "b",
        );
        let rel_d = derive_knowledge_relationship_id(
            KnowledgeEdgeType::References,
            KnowledgeEntityKind::Symbol,
            "mod",
            KnowledgeEntityKind::Symbol,
            ":a|symbol:b",
        );
        assert_ne!(rel_c, rel_d, "`:`-injection pair must derive distinct ids");

        // Embedded persistence layer: the two edges must COEXIST as separate
        // rows. Under the old scheme the second upsert collided onto the first's
        // relationship_id and silently overwrote it (UNIQUE merge).
        let Some(store) = embedded_knowledge().await else {
            eprintln!(
                "SKIP relationship_id_resists_separator_injection_collision: no embedded store"
            );
            return;
        };
        let (workspace_id, source_id) = source_fixture(&store).await;
        let span = store
            .db
            .create_knowledge_span(byte_span(&source_id, 0, 10))
            .await
            .expect("span");

        let mk_entity = |kind: KnowledgeEntityKind, key: &str| NewKnowledgeEntity {
            workspace_id: workspace_id.clone(),
            entity_kind: kind,
            entity_key: key.to_string(),
            display_name: format!("{kind:?}:{key}"),
            detection_provenance: json!({"extractor": "collision_test"}),
            primary_source_id: None,
            detected_in_run: None,
            evidence_span_ids: vec![span.span_id.clone()],
        };

        // Four entities backing the A/B collision pair. The keys carry the
        // adversarial `|` and `:` bytes.
        let file_p = store
            .db
            .upsert_knowledge_entity(mk_entity(KnowledgeEntityKind::File, "p"))
            .await
            .expect("entity file:p");
        let folder_xfy = store
            .db
            .upsert_knowledge_entity(mk_entity(KnowledgeEntityKind::Folder, "x|folder:y"))
            .await
            .expect("entity folder:x|folder:y");
        let file_pfx = store
            .db
            .upsert_knowledge_entity(mk_entity(KnowledgeEntityKind::File, "p|folder:x"))
            .await
            .expect("entity file:p|folder:x");
        let folder_y = store
            .db
            .upsert_knowledge_entity(mk_entity(KnowledgeEntityKind::Folder, "y"))
            .await
            .expect("entity folder:y");

        let edge_a = store
            .db
            .upsert_knowledge_edge(NewKnowledgeEdge {
                workspace_id: workspace_id.clone(),
                edge_type: KnowledgeEdgeType::Contains,
                source_entity_id: file_p.entity_id.clone(),
                target_entity_id: folder_xfy.entity_id.clone(),
                extractor_version: "collision_v1".to_string(),
                confidence: 0.5,
                detected_in_run: None,
                evidence_span_ids: vec![span.span_id.clone()],
            })
            .await
            .expect("edge A");
        let edge_b = store
            .db
            .upsert_knowledge_edge(NewKnowledgeEdge {
                workspace_id: workspace_id.clone(),
                edge_type: KnowledgeEdgeType::Contains,
                source_entity_id: file_pfx.entity_id.clone(),
                target_entity_id: folder_y.entity_id.clone(),
                extractor_version: "collision_v1".to_string(),
                confidence: 0.9,
                detected_in_run: None,
                evidence_span_ids: vec![span.span_id.clone()],
            })
            .await
            .expect("edge B");

        // The two edges must be DISTINCT rows with DISTINCT relationship_ids.
        assert_eq!(edge_a.relationship_id, rel_a);
        assert_eq!(edge_b.relationship_id, rel_b);
        assert_ne!(
            edge_a.relationship_id, edge_b.relationship_id,
            "two distinct edges must not share a relationship_id"
        );
        assert_ne!(
            edge_a.edge_id, edge_b.edge_id,
            "two distinct edges must not collapse into one row"
        );

        // Both rows physically present under the workspace's UNIQUE namespace.
        let by_rel_a = store
            .db
            .get_knowledge_edge_by_relationship_id(&workspace_id, &rel_a)
            .await
            .expect("get A by rel")
            .expect("edge A present");
        let by_rel_b = store
            .db
            .get_knowledge_edge_by_relationship_id(&workspace_id, &rel_b)
            .await
            .expect("get B by rel")
            .expect("edge B present");
        assert_eq!(by_rel_a.edge_id, edge_a.edge_id);
        assert_eq!(by_rel_b.edge_id, edge_b.edge_id);
        // Edge A's confidence was NOT clobbered by edge B's upsert.
        assert!((by_rel_a.confidence - 0.5).abs() < f64::EPSILON);
    }
}
