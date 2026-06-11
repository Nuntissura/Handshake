//! WP-KERNEL-009 PostgresEventLedgerCore integration tests against REAL
//! Handshake-managed PostgreSQL: MT-058 (WikiProjectionTables), MT-059
//! (RichDocumentTables + EditorCodeNode), MT-060 (ContextBundleTables +
//! RetrievalTrace), MT-061 (EventLedgerEventFamilies on the real ledger).

mod knowledge_pg_support;

use handshake_core::storage::knowledge::KnowledgeStore;
use knowledge_pg_support::knowledge_pg;
use serde_json::json;

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// MT-058 WikiProjectionTables
// ---------------------------------------------------------------------------

mod mt_058_projections {
    use super::*;
    use handshake_core::storage::knowledge::{
        KnowledgeAuthorityClass, KnowledgeProjectionKind, KnowledgeRebuildStatus,
        NewKnowledgeWikiProjection,
    };

    fn projection(workspace_id: &str, title: &str) -> NewKnowledgeWikiProjection {
        NewKnowledgeWikiProjection {
            workspace_id: workspace_id.to_string(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: title.to_string(),
            source_records: json!([
                {"record_family": "KnowledgeClaim", "record_id": "KCL-demo"},
            ]),
            rendered_content: "# Generated page\n\nprojection only".to_string(),
            staleness_hash: super::sha256_hex(b"render-input-v1"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn projection_lifecycle_rebuild_stale_delete_without_authority_mutation() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP projection_lifecycle...: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        let created = pg
            .db
            .upsert_knowledge_wiki_projection(projection(&workspace_id, "Kernel Events"))
            .await
            .expect("create projection");
        assert!(created.projection_id.starts_with("KWP-"));
        assert_eq!(created.rebuild_status, KnowledgeRebuildStatus::Stale);

        // Rebuild workers can mark the projection rebuilding, then failed,
        // without touching the rendered content.
        let rebuilding = pg
            .db
            .set_knowledge_projection_rebuild_status(
                &created.projection_id,
                KnowledgeRebuildStatus::Rebuilding,
            )
            .await
            .expect("mark rebuilding");
        assert_eq!(
            rebuilding.rebuild_status,
            KnowledgeRebuildStatus::Rebuilding
        );
        let failed = pg
            .db
            .set_knowledge_projection_rebuild_status(
                &created.projection_id,
                KnowledgeRebuildStatus::Failed,
            )
            .await
            .expect("mark failed");
        assert_eq!(failed.rebuild_status, KnowledgeRebuildStatus::Failed);
        assert_eq!(failed.rendered_content, created.rendered_content);

        // Rebuild completes: fresh + rebuild timestamp.
        let fresh = pg
            .db
            .mark_knowledge_projection_rebuilt(
                &created.projection_id,
                &super::sha256_hex(b"render-input-v2"),
                "# Generated page v2",
                None,
            )
            .await
            .expect("mark rebuilt");
        assert_eq!(fresh.rebuild_status, KnowledgeRebuildStatus::Fresh);
        assert!(fresh.last_rebuilt_at.is_some());

        // Upserting the same (workspace, kind, title) updates in place.
        let again = pg
            .db
            .upsert_knowledge_wiki_projection(projection(&workspace_id, "Kernel Events"))
            .await
            .expect("re-upsert projection");
        assert_eq!(again.projection_id, created.projection_id);
        assert_eq!(
            again.rebuild_status,
            KnowledgeRebuildStatus::Stale,
            "re-upsert marks the projection stale for rebuild"
        );

        // Registry classifies the table as projection, never authority.
        let registry = pg
            .db
            .list_knowledge_schema_registry()
            .await
            .expect("registry");
        let row = registry
            .iter()
            .find(|row| row.table_name == "knowledge_wiki_projections")
            .expect("projection table registered");
        assert_eq!(row.authority_class, KnowledgeAuthorityClass::Projection);

        // Deleting the projection mutates NO authority records: count an
        // authority surface before and after the delete.
        let before: i64 = {
            let mut conn = pg.raw_connection().await;
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_schema_registry")
                .fetch_one(&mut conn)
                .await
                .expect("count registry")
        };
        pg.db
            .delete_knowledge_wiki_projection(&created.projection_id)
            .await
            .expect("delete projection");
        let gone = pg
            .db
            .get_knowledge_wiki_projection(&created.projection_id)
            .await
            .expect("get after delete");
        assert!(gone.is_none(), "projection row is gone");
        let after: i64 = {
            let mut conn = pg.raw_connection().await;
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_schema_registry")
                .fetch_one(&mut conn)
                .await
                .expect("count registry after")
        };
        assert_eq!(
            before, after,
            "deleting a projection must not touch authority rows"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_authority_table_references_the_projection_table() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP no_authority_table_references_the_projection_table: no PostgreSQL");
            return;
        };
        // Structural boundary proof straight from the catalog: zero foreign
        // keys anywhere point INTO knowledge_wiki_projections.
        let mut conn = pg.raw_connection().await;
        let inbound: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.table_constraints tc
            JOIN information_schema.constraint_column_usage ccu
              ON tc.constraint_name = ccu.constraint_name
             AND tc.constraint_schema = ccu.constraint_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
              AND tc.constraint_schema = current_schema()
              AND ccu.table_name = 'knowledge_wiki_projections'
            "#,
        )
        .fetch_one(&mut conn)
        .await
        .expect("catalog query");
        assert_eq!(
            inbound, 0,
            "spec 2.3.13.11: no FK may point from authority into projection content"
        );
    }
}

// ---------------------------------------------------------------------------
// MT-059 RichDocumentTables + EditorCodeNode
// ---------------------------------------------------------------------------

mod mt_059_rich_documents {
    use super::*;
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use handshake_core::storage::knowledge::{NewKnowledgeRichDocument, UpsertEditorCodeNode};
    use handshake_core::storage::{Database, StorageError};
    use uuid::Uuid;

    fn doc(workspace_id: &str, title: &str) -> NewKnowledgeRichDocument {
        let content = json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "v1"}]}]
        });
        NewKnowledgeRichDocument {
            workspace_id: workspace_id.to_string(),
            document_id: None,
            title: title.to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: content,
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn versioned_save_with_optimistic_concurrency_and_history() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP versioned_save_with_optimistic_concurrency_and_history: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        let created = pg
            .db
            .create_knowledge_rich_document(doc(&workspace_id, "Runbook"))
            .await
            .expect("create rich document");
        assert!(created.rich_document_id.starts_with("KRD-"));
        assert_eq!(created.doc_version, 1);

        // Promotion receipt for v2 through the real EventLedger.
        let suffix = Uuid::now_v7();
        let receipt = pg
            .db
            .append_kernel_event(
                NewKernelEvent::builder(
                    format!("KTR-RICHDOC-{suffix}"),
                    format!("SR-RICHDOC-{suffix}"),
                    KernelEventType::PromotionAccepted,
                    KernelActor::PromotionGate("richdoc-test".to_string()),
                )
                .aggregate("knowledge_rich_document", created.rich_document_id.clone())
                .idempotency_key(format!("idem-richdoc-promote-{suffix}"))
                .payload(json!({"doc_version": 2}))
                .build()
                .expect("event"),
            )
            .await
            .expect("append promotion receipt");

        let v2_content = json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "v2"}]}]
        });
        let saved = pg
            .db
            .save_knowledge_rich_document_version(
                &created.rich_document_id,
                1,
                v2_content.clone(),
                Some(&receipt.event_id),
            )
            .await
            .expect("save v2");
        assert_eq!(saved.doc_version, 2);
        assert_eq!(saved.content_json, v2_content);
        assert_eq!(
            saved.promotion_receipt_event_id.as_deref(),
            Some(receipt.event_id.as_str())
        );

        // Optimistic concurrency: stale expected_version fails closed.
        let err = pg
            .db
            .save_knowledge_rich_document_version(
                &created.rich_document_id,
                1,
                json!({"type": "doc", "content": []}),
                None,
            )
            .await
            .expect_err("stale expected_version must be a typed Conflict");
        assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");

        // Version history is append-only and complete.
        let versions = pg
            .db
            .list_knowledge_rich_document_versions(&created.rich_document_id)
            .await
            .expect("version history");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].doc_version, 1);
        assert_eq!(versions[1].doc_version, 2);
        assert_eq!(
            versions[1].promotion_receipt_event_id.as_deref(),
            Some(receipt.event_id.as_str())
        );

        // content_sha256 matches the canonical content hash discipline.
        let reread = pg
            .db
            .get_knowledge_rich_document(&created.rich_document_id)
            .await
            .expect("get doc")
            .expect("doc exists");
        assert_eq!(reread.doc_version, 2);
        assert_eq!(reread.content_sha256.len(), 64);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn editor_code_nodes_roundtrip_with_integrity_hash() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP editor_code_nodes_roundtrip_with_integrity_hash: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;
        let document = pg
            .db
            .create_knowledge_rich_document(doc(&workspace_id, "Code Doc"))
            .await
            .expect("doc");

        let code = "fn main() { println!(\"hello\"); }";
        let node = pg
            .db
            .upsert_knowledge_editor_code_node(UpsertEditorCodeNode {
                rich_document_id: document.rich_document_id.clone(),
                node_path: "body.0.code".to_string(),
                language_id: "rust".to_string(),
                code_text: code.to_string(),
                worker_requirements: json!({"worker": "editor", "bundled": true}),
                source_mapping: Some(json!({"source_path": "src/main.rs"})),
                lint_diagnostics: json!([]),
            })
            .await
            .expect("upsert code node");
        assert!(node.code_node_id.starts_with("KCN-"));
        // Round-trip hash is the sha256 of the exact code text.
        assert_eq!(node.round_trip_sha256, super::sha256_hex(code.as_bytes()));

        // Round-trip proof: read back and re-derive the hash from the stored
        // text — a Monaco mount/unmount must preserve this equality.
        let listed = pg
            .db
            .list_knowledge_editor_code_nodes(&document.rich_document_id)
            .await
            .expect("list code nodes");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].code_text, code);
        assert_eq!(
            super::sha256_hex(listed[0].code_text.as_bytes()),
            listed[0].round_trip_sha256,
            "stored round-trip hash must re-derive from stored code text"
        );

        // Same node_path upserts in place (stable node identity).
        let updated = pg
            .db
            .upsert_knowledge_editor_code_node(UpsertEditorCodeNode {
                rich_document_id: document.rich_document_id.clone(),
                node_path: "body.0.code".to_string(),
                language_id: "rust".to_string(),
                code_text: "fn main() {}".to_string(),
                worker_requirements: json!({"worker": "editor", "bundled": true}),
                source_mapping: None,
                lint_diagnostics: json!([{"severity": "warning", "message": "empty main"}]),
            })
            .await
            .expect("re-upsert code node");
        assert_eq!(updated.code_node_id, node.code_node_id);
        assert_eq!(
            updated.round_trip_sha256,
            super::sha256_hex(b"fn main() {}")
        );
        assert_eq!(
            pg.db
                .list_knowledge_editor_code_nodes(&document.rich_document_id)
                .await
                .expect("list again")
                .len(),
            1
        );

        // Ghost document: FK violation fails closed.
        let err = pg
            .db
            .upsert_knowledge_editor_code_node(UpsertEditorCodeNode {
                rich_document_id: "KRD-00000000000000000000000000000000".to_string(),
                node_path: "body.0.code".to_string(),
                language_id: "rust".to_string(),
                code_text: "x".to_string(),
                worker_requirements: json!({}),
                source_mapping: None,
                lint_diagnostics: json!([]),
            })
            .await
            .expect_err("code nodes must belong to a real rich document");
        assert!(err.to_string().contains("foreign key"), "got {err}");
    }
}

// ---------------------------------------------------------------------------
// MT-060 ContextBundleTables + RetrievalTrace
// ---------------------------------------------------------------------------

mod mt_060_context_bundles {
    use super::*;
    use handshake_core::kernel::context_bundle::ContextBundle;
    use handshake_core::storage::knowledge::{
        KnowledgeBundleItemDecision, KnowledgeBundleItemRefKind, KnowledgeRetrievalMode,
        NewKnowledgeContextBundle, NewKnowledgeContextBundleItem, NewKnowledgeRetrievalTrace,
    };
    use handshake_core::storage::StorageError;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn v1_compatible_bundle_persists_with_items_and_trace() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP v1_compatible_bundle_persists_with_items_and_trace: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        // Build through the REAL kernel V1 ContextBundle constructor so the
        // persisted shape is exactly the runtime shape.
        let allowed_context = json!({
            "files": ["src/storage/knowledge.rs"],
            "claims": ["KCL-demo"],
        });
        let v1 = ContextBundle::new("KTR-BUNDLE-1", "SR-BUNDLE-1", allowed_context.clone())
            .expect("kernel V1 context bundle");

        let stored = pg
            .db
            .record_knowledge_context_bundle(NewKnowledgeContextBundle {
                workspace_id: workspace_id.clone(),
                bundle: v1.clone(),
                query_text: Some("how does knowledge storage fail closed?".to_string()),
                token_budget: Some(8192),
                tokens_used: Some(2048),
                build_receipt_event_id: None,
                items: vec![
                    NewKnowledgeContextBundleItem {
                        ref_kind: KnowledgeBundleItemRefKind::Source,
                        ref_id: "KSRC-demo".to_string(),
                        retrieval_decision: KnowledgeBundleItemDecision::Included,
                        relevance_score: Some(0.91),
                        token_count: Some(1024),
                        citation: Some("src/storage/knowledge.rs#L1-L40".to_string()),
                    },
                    NewKnowledgeContextBundleItem {
                        ref_kind: KnowledgeBundleItemRefKind::Passage,
                        ref_id: "KMP-demo".to_string(),
                        retrieval_decision: KnowledgeBundleItemDecision::ExcludedBudget,
                        relevance_score: Some(0.4),
                        token_count: Some(4096),
                        citation: None,
                    },
                ],
            })
            .await
            .expect("record bundle");
        // V1 invariants hold in storage.
        assert_eq!(stored.bundle_id, v1.context_bundle_id);
        assert_eq!(stored.context_hash, v1.context_hash);
        assert_eq!(stored.allowed_context, allowed_context);

        let (reread, items) = pg
            .db
            .get_knowledge_context_bundle(&stored.bundle_id)
            .await
            .expect("get bundle")
            .expect("bundle exists");
        assert_eq!(reread, stored);
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].retrieval_decision,
            KnowledgeBundleItemDecision::Included
        );
        assert_eq!(
            items[1].retrieval_decision,
            KnowledgeBundleItemDecision::ExcludedBudget
        );

        // Retrieval trace with mode_reason (spec MUST: why broader retrieval
        // was used or skipped), linked to the bundle.
        let trace = pg
            .db
            .record_knowledge_retrieval_trace(NewKnowledgeRetrievalTrace {
                workspace_id: workspace_id.clone(),
                retrieval_mode: KnowledgeRetrievalMode::ExactLookup,
                mode_reason: "stable source id was already known; hybrid retrieval skipped"
                    .to_string(),
                query_text: Some("KSRC-demo".to_string()),
                bundle_id: Some(stored.bundle_id.clone()),
                decisions: json!([{"step": 1, "action": "exact_lookup", "hit": true}]),
                trace_receipt_event_id: None,
            })
            .await
            .expect("record trace");
        assert!(trace.trace_id.starts_with("KRT-"));

        let traces = pg
            .db
            .list_knowledge_retrieval_traces_for_bundle(&stored.bundle_id)
            .await
            .expect("traces for bundle");
        assert_eq!(traces.len(), 1);
        assert_eq!(
            traces[0].retrieval_mode,
            KnowledgeRetrievalMode::ExactLookup
        );

        // Empty mode_reason fails closed (typed Validation).
        let err = pg
            .db
            .record_knowledge_retrieval_trace(NewKnowledgeRetrievalTrace {
                workspace_id: workspace_id.clone(),
                retrieval_mode: KnowledgeRetrievalMode::HybridRag,
                mode_reason: "".to_string(),
                query_text: None,
                bundle_id: None,
                decisions: json!([]),
                trace_receipt_event_id: None,
            })
            .await
            .expect_err("mode_reason is a spec MUST");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bundle_id_hash_binding_is_db_enforced() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP bundle_id_hash_binding_is_db_enforced: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;
        let mut conn = pg.raw_connection().await;

        // A bundle id that does not match the content hash prefix violates
        // the V1 derivation CHECK even when the app layer is bypassed.
        let err = sqlx::query(
            "INSERT INTO knowledge_context_bundles
                 (bundle_id, workspace_id, kernel_task_run_id, session_run_id,
                  allowed_context, context_hash)
             VALUES ('CTX-deadbeefdeadbeef', $1, 'KTR-X', 'SR-X', '{}'::jsonb, $2)",
        )
        .bind(&workspace_id)
        .bind(super::sha256_hex(b"different-content"))
        .execute(&mut conn)
        .await
        .expect_err("bundle_id must be derived from context_hash");
        assert!(
            err.to_string()
                .contains("chk_knowledge_context_bundles_id_matches_hash"),
            "unexpected: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// MT-061 EventLedgerEventFamilies: knowledge events on the REAL ledger
// ---------------------------------------------------------------------------

mod mt_061_event_families {
    use super::*;
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use handshake_core::storage::Database;
    use uuid::Uuid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn knowledge_event_families_append_and_replay_on_real_ledger() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!(
                "SKIP knowledge_event_families_append_and_replay_on_real_ledger: no PostgreSQL"
            );
            return;
        };
        let suffix = Uuid::now_v7();
        let aggregate_id = format!("KIR-EVENTS-{suffix}");

        // One representative event per MT-061 family axis: write through the
        // real ledger, then replay by aggregate and parse the types back.
        let families = [
            KernelEventType::KnowledgeIndexRunStarted,
            KernelEventType::KnowledgeClaimProposed,
            KernelEventType::KnowledgeClaimConflictDetected,
            KernelEventType::KnowledgeRetrievalTraceRecorded,
            KernelEventType::KnowledgeRichDocumentSaved,
            KernelEventType::KnowledgeLoomBlockIndexed,
            KernelEventType::KnowledgeUserManualEntryRecorded,
            KernelEventType::KnowledgeValidationRecorded,
        ];
        for (ordinal, event_type) in families.iter().enumerate() {
            pg.db
                .append_kernel_event(
                    NewKernelEvent::builder(
                        format!("KTR-KNOWLEDGE-{suffix}"),
                        format!("SR-KNOWLEDGE-{suffix}"),
                        event_type.clone(),
                        KernelActor::System("knowledge-index".to_string()),
                    )
                    .aggregate("knowledge_index_run", aggregate_id.clone())
                    .idempotency_key(format!("idem-knowledge-family-{ordinal}-{suffix}"))
                    .payload(json!({"ordinal": ordinal}))
                    .build()
                    .expect("event"),
                )
                .await
                .unwrap_or_else(|err| panic!("append {} failed: {err:?}", event_type.as_str()));
        }

        let replayed = pg
            .db
            .list_kernel_events_for_aggregate("knowledge_index_run", &aggregate_id)
            .await
            .expect("replay by aggregate");
        assert_eq!(replayed.len(), families.len());
        for (event, expected) in replayed.iter().zip(families.iter()) {
            assert_eq!(
                &event.event_type, expected,
                "ledger replay must parse the knowledge event family back losslessly"
            );
        }
    }
}
