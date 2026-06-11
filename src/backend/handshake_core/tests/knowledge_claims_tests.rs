//! WP-KERNEL-009 PostgresEventLedgerCore integration tests against REAL
//! Handshake-managed PostgreSQL: MT-056 (KnowledgeClaimTables) and MT-057
//! (PassageEvidenceTables).

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
    KnowledgeRootKind, KnowledgeSourceKind, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeSource,
    NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const HASH_SPAN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// workspace -> root -> source -> span fixture.
async fn span_fixture(pg: &KnowledgePg) -> (String, String, String) {
    let workspace_id = pg.create_workspace().await;
    let root = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("src/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("root");
    let source = pg
        .db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: Some(root.root_id),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("storage/knowledge.rs".to_string()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: HASH_SRC.to_string(),
            size_bytes: Some(1024),
            provenance: json!({"discovered_by": "claims_test"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("source");
    let span = pg
        .db
        .create_knowledge_span(NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Text,
            range_start: 0,
            range_end: 240,
            line_start: Some(1),
            line_end: Some(6),
            section_path: None,
            content_sha256: HASH_SPAN.to_string(),
            parser_version: "text_v1".to_string(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("module docs".to_string()),
        })
        .await
        .expect("span");
    (workspace_id, source.source_id, span.span_id)
}

// ---------------------------------------------------------------------------
// MT-056 KnowledgeClaimTables
// ---------------------------------------------------------------------------

mod mt_056_claims {
    use super::*;
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use handshake_core::storage::knowledge::{
        KnowledgeClaimKind, KnowledgeClaimRetirement, KnowledgeClaimRetirementReason,
        KnowledgeClaimState, NewKnowledgeClaim,
    };
    use handshake_core::storage::{Database, StorageError};

    fn claim(workspace_id: &str, text: &str, spans: Vec<String>) -> NewKnowledgeClaim {
        NewKnowledgeClaim {
            workspace_id: workspace_id.to_string(),
            claim_kind: KnowledgeClaimKind::ProductBehavior,
            claim_text: text.to_string(),
            subject_entity_id: None,
            temporal_qualifier: Some(json!({"valid_from": "2026-06-11T00:00:00Z"})),
            granularity_qualifier: Some("file".to_string()),
            confidence: 0.6,
            proposed_in_run: None,
            evidence_span_ids: spans,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_lifecycle_proposed_accepted_retired_with_receipts() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP claim_lifecycle_proposed_accepted_retired_with_receipts: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;

        let created = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "knowledge storage fails closed without PostgreSQL",
                vec![span_id.clone()],
            ))
            .await
            .expect("create claim");
        assert!(created.claim_id.starts_with("KCL-"));
        assert_eq!(created.lifecycle_state, KnowledgeClaimState::Proposed);
        assert_eq!(
            pg.db
                .list_knowledge_claim_span_ids(&created.claim_id)
                .await
                .expect("claim evidence"),
            vec![span_id.clone()]
        );

        // Acceptance backed by a real EventLedger receipt.
        let suffix = Uuid::now_v7();
        let receipt = pg
            .db
            .append_kernel_event(
                NewKernelEvent::builder(
                    format!("KTR-CLAIM-{suffix}"),
                    format!("SR-CLAIM-{suffix}"),
                    KernelEventType::ValidationRecorded,
                    KernelActor::ValidationRunner("claims-test".to_string()),
                )
                .aggregate("knowledge_claim", created.claim_id.clone())
                .idempotency_key(format!("idem-claim-accept-{suffix}"))
                .payload(json!({"verdict": "accepted"}))
                .build()
                .expect("event"),
            )
            .await
            .expect("append receipt");
        let accepted = pg
            .db
            .transition_knowledge_claim(
                &created.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&receipt.event_id),
            )
            .await
            .expect("accept claim");
        assert_eq!(accepted.lifecycle_state, KnowledgeClaimState::Accepted);
        assert_eq!(
            accepted.resolution_receipt_event_id.as_deref(),
            Some(receipt.event_id.as_str())
        );

        // Supersede: a new claim retires the old one with lineage.
        let successor = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "knowledge storage fails closed with typed StorageError",
                vec![span_id.clone()],
            ))
            .await
            .expect("successor claim");
        let retired = pg
            .db
            .transition_knowledge_claim(
                &accepted.claim_id,
                KnowledgeClaimState::Retired,
                Some(KnowledgeClaimRetirement {
                    reason: KnowledgeClaimRetirementReason::Superseded,
                    superseded_by_claim_id: Some(successor.claim_id.clone()),
                }),
                None,
            )
            .await
            .expect("supersede claim");
        assert_eq!(retired.lifecycle_state, KnowledgeClaimState::Retired);
        assert_eq!(
            retired.retirement_reason,
            Some(KnowledgeClaimRetirementReason::Superseded)
        );
        assert_eq!(
            retired.superseded_by_claim_id.as_deref(),
            Some(successor.claim_id.as_str())
        );

        // Retired is terminal: any further transition is a typed Conflict.
        let err = pg
            .db
            .transition_knowledge_claim(
                &retired.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                None,
            )
            .await
            .expect_err("retired claims are terminal");
        assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claims_require_evidence_at_every_layer() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP claims_require_evidence_at_every_layer: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id, _span_id) = span_fixture(&pg).await;

        // Rust layer.
        let err = pg
            .db
            .create_knowledge_claim(claim(&workspace_id, "evidence-free claim", vec![]))
            .await
            .expect_err("claims without evidence spans must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

        // DB layer: direct INSERT without evidence fails at COMMIT.
        let mut conn = pg.raw_connection().await;
        sqlx::query("BEGIN").execute(&mut conn).await.expect("begin");
        sqlx::query(
            "INSERT INTO knowledge_claims
                 (claim_id, workspace_id, claim_kind, claim_text)
             VALUES ('KCL-00000000000000000000000000000001', $1, 'source_fact', 'rogue claim')",
        )
        .bind(&workspace_id)
        .execute(&mut conn)
        .await
        .expect("insert inside transaction");
        let err = sqlx::query("COMMIT")
            .execute(&mut conn)
            .await
            .expect_err("commit must fail without evidence spans");
        assert!(
            err.to_string().contains("MUST carry evidence spans"),
            "unexpected commit error: {err}"
        );

        // Retirement-shape CHECK: retired without reason is rejected in SQL.
        let err = sqlx::query(
            "INSERT INTO knowledge_claims
                 (claim_id, workspace_id, claim_kind, claim_text, lifecycle_state)
             VALUES ('KCL-00000000000000000000000000000002', $1, 'source_fact', 'x', 'retired')",
        )
        .bind(&workspace_id)
        .execute(&mut conn)
        .await
        .expect_err("retired claims must carry a retirement reason");
        assert!(
            err.to_string().contains("chk_knowledge_claims_retirement_shape"),
            "unexpected: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conflict_detection_and_receipt_backed_resolution() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP conflict_detection_and_receipt_backed_resolution: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;

        let claim_a = pg
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5544", vec![span_id.clone()]))
            .await
            .expect("claim a");
        let claim_b = pg
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5432", vec![span_id.clone()]))
            .await
            .expect("claim b");

        let conflict = pg
            .db
            .record_knowledge_claim_conflict(
                &claim_a.claim_id,
                &claim_b.claim_id,
                "contradictory port assertions for the managed cluster",
                None,
            )
            .await
            .expect("record conflict");
        assert!(conflict.conflict_id.starts_with("KCC-"));
        assert!(conflict.resolved_at.is_none());

        // Both claims moved to conflicted.
        for id in [&claim_a.claim_id, &claim_b.claim_id] {
            let state = pg
                .db
                .get_knowledge_claim(id)
                .await
                .expect("get claim")
                .expect("claim exists")
                .lifecycle_state;
            assert_eq!(state, KnowledgeClaimState::Conflicted);
        }

        // Self-conflict and duplicate pair fail closed.
        let err = pg
            .db
            .record_knowledge_claim_conflict(&claim_a.claim_id, &claim_a.claim_id, "self", None)
            .await
            .expect_err("self-conflict must be rejected");
        assert!(matches!(err, StorageError::Validation(_)));
        let err = pg
            .db
            .record_knowledge_claim_conflict(
                &claim_a.claim_id,
                &claim_b.claim_id,
                "duplicate pair",
                None,
            )
            .await
            .expect_err("duplicate conflict pair must violate unique constraint");
        assert!(
            err.to_string().contains("uq_knowledge_claim_conflicts_pair"),
            "unexpected: {err}"
        );

        // Resolution requires a real EventLedger receipt (FK).
        let err = pg
            .db
            .resolve_knowledge_claim_conflict(&conflict.conflict_id, "KE-GHOST")
            .await
            .expect_err("resolution receipt must reference a real ledger event");
        assert!(err.to_string().contains("foreign key"), "got {err}");

        let suffix = Uuid::now_v7();
        let receipt = pg
            .db
            .append_kernel_event(
                NewKernelEvent::builder(
                    format!("KTR-CONFLICT-{suffix}"),
                    format!("SR-CONFLICT-{suffix}"),
                    KernelEventType::ValidationRecorded,
                    KernelActor::ValidationRunner("conflict-test".to_string()),
                )
                .aggregate("knowledge_claim_conflict", conflict.conflict_id.clone())
                .idempotency_key(format!("idem-conflict-resolve-{suffix}"))
                .payload(json!({"resolution": "claim_a wins"}))
                .build()
                .expect("event"),
            )
            .await
            .expect("append resolution receipt");
        let resolved = pg
            .db
            .resolve_knowledge_claim_conflict(&conflict.conflict_id, &receipt.event_id)
            .await
            .expect("resolve conflict");
        assert!(resolved.resolved_at.is_some());
        assert_eq!(
            resolved.resolution_receipt_event_id.as_deref(),
            Some(receipt.event_id.as_str())
        );

        // Double-resolution is a typed Conflict.
        let err = pg
            .db
            .resolve_knowledge_claim_conflict(&conflict.conflict_id, &receipt.event_id)
            .await
            .expect_err("conflicts resolve exactly once");
        assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");

        // Winning claim returns to accepted through the guarded transition.
        let accepted = pg
            .db
            .transition_knowledge_claim(
                &claim_a.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&receipt.event_id),
            )
            .await
            .expect("conflicted -> accepted");
        assert_eq!(accepted.lifecycle_state, KnowledgeClaimState::Accepted);

        let conflicts = pg
            .db
            .list_knowledge_claim_conflicts(&claim_a.claim_id)
            .await
            .expect("list conflicts");
        assert_eq!(conflicts.len(), 1);
    }
}
