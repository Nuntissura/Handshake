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

    async fn append_receipt_for_aggregate(
        pg: &KnowledgePg,
        label: &str,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> String {
        let suffix = Uuid::now_v7();
        pg.db
            .append_kernel_event(
                NewKernelEvent::builder(
                    format!("KTR-{label}-{suffix}"),
                    format!("SR-{label}-{suffix}"),
                    KernelEventType::ValidationRecorded,
                    KernelActor::ValidationRunner(label.to_string()),
                )
                .aggregate(aggregate_type.to_string(), aggregate_id.to_string())
                .idempotency_key(format!("idem-{label}-{suffix}"))
                .payload(json!({"resolution": label}))
                .build()
                .expect("event"),
            )
            .await
            .expect("append conflict resolution receipt")
            .event_id
    }

    async fn append_conflict_resolution_receipt(
        pg: &KnowledgePg,
        conflict_id: &str,
        label: &str,
    ) -> String {
        append_receipt_for_aggregate(pg, label, "knowledge_claim_conflict", conflict_id).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_lifecycle_proposed_accepted_retired_with_receipts() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!(
                "SKIP claim_lifecycle_proposed_accepted_retired_with_receipts: no PostgreSQL"
            );
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

    /// HARDENING (MT-056): the four-state lifecycle MUST hold at the DB layer,
    /// not only inside transition_knowledge_claim. A raw
    /// `UPDATE ... SET lifecycle_state='accepted', retirement_reason=NULL
    ///  WHERE lifecycle_state='retired'` is a legal SHAPE (the 0137 CHECKs pass)
    /// and previously resurrected a terminal-retired claim by bypassing the app
    /// method. The 0200 BEFORE UPDATE trigger must refuse it. This test drives
    /// raw SQL (NOT the app method) to prove the DB itself is the guard.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn db_blocks_retired_to_accepted_resurrection_via_raw_sql() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP db_blocks_retired_to_accepted_resurrection_via_raw_sql: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;

        // Create + retire a claim through the normal app path.
        let created = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "a retired claim that must stay retired",
                vec![span_id.clone()],
            ))
            .await
            .expect("create claim");
        let retired = pg
            .db
            .transition_knowledge_claim(
                &created.claim_id,
                KnowledgeClaimState::Retired,
                Some(KnowledgeClaimRetirement {
                    reason: KnowledgeClaimRetirementReason::OperatorRetired,
                    superseded_by_claim_id: None,
                }),
                None,
            )
            .await
            .expect("retire claim");
        assert_eq!(retired.lifecycle_state, KnowledgeClaimState::Retired);

        let mut conn = pg.raw_connection().await;

        // THE ATTACK: raw resurrection that satisfies every CHECK
        // (accepted + NULL reason is a valid shape). The DB must refuse it via
        // the transition-guard trigger, not just the app method.
        let err = sqlx::query(
            "UPDATE knowledge_claims
                 SET lifecycle_state = 'accepted', retirement_reason = NULL
             WHERE claim_id = $1",
        )
        .bind(&retired.claim_id)
        .execute(&mut conn)
        .await
        .expect_err("DB must block retired -> accepted (terminal retired)");
        assert!(
            err.to_string().contains("retired is terminal"),
            "unexpected error (expected transition-guard refusal): {err}"
        );

        // The row is unchanged on disk: still retired, reason intact.
        let still = pg
            .db
            .get_knowledge_claim(&retired.claim_id)
            .await
            .expect("get claim")
            .expect("claim exists");
        assert_eq!(
            still.lifecycle_state,
            KnowledgeClaimState::Retired,
            "blocked update must not have mutated the row"
        );
        assert_eq!(
            still.retirement_reason,
            Some(KnowledgeClaimRetirementReason::OperatorRetired)
        );

        // The guard blocks lifecycle MOVEMENT, not metadata updates: a raw
        // metadata-only UPDATE on the same retired row (no lifecycle change)
        // still succeeds, so the trigger is not over-broad.
        sqlx::query("UPDATE knowledge_claims SET confidence = 0.123 WHERE claim_id = $1")
            .bind(&retired.claim_id)
            .execute(&mut conn)
            .await
            .expect("metadata-only update on a retired row must still succeed");

        // And an ILLEGAL non-terminal jump (proposed -> retired is legal, but
        // e.g. a fabricated proposed -> stale is not a state at all; test a real
        // illegal pair: accepted -> proposed) is also refused by the guard.
        let live = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "a live claim for the backward-transition probe",
                vec![span_id.clone()],
            ))
            .await
            .expect("create live claim");
        let suffix = Uuid::now_v7();
        let receipt = pg
            .db
            .append_kernel_event(
                NewKernelEvent::builder(
                    format!("KTR-GUARD-{suffix}"),
                    format!("SR-GUARD-{suffix}"),
                    KernelEventType::ValidationRecorded,
                    KernelActor::ValidationRunner("guard-test".to_string()),
                )
                .aggregate("knowledge_claim", live.claim_id.clone())
                .idempotency_key(format!("idem-guard-accept-{suffix}"))
                .payload(json!({"verdict": "accepted"}))
                .build()
                .expect("event"),
            )
            .await
            .expect("append receipt");
        let accepted = pg
            .db
            .transition_knowledge_claim(
                &live.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&receipt.event_id),
            )
            .await
            .expect("accept live claim");
        assert_eq!(accepted.lifecycle_state, KnowledgeClaimState::Accepted);

        // Raw backward jump accepted -> proposed: not in the legal table.
        let err = sqlx::query(
            "UPDATE knowledge_claims SET lifecycle_state = 'proposed'
             WHERE claim_id = $1",
        )
        .bind(&accepted.claim_id)
        .execute(&mut conn)
        .await
        .expect_err("DB must block accepted -> proposed (illegal transition)");
        assert!(
            err.to_string().contains("illegal lifecycle transition"),
            "unexpected error (expected illegal-transition refusal): {err}"
        );
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
        sqlx::query("BEGIN")
            .execute(&mut conn)
            .await
            .expect("begin");
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
            err.to_string()
                .contains("chk_knowledge_claims_retirement_shape"),
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
            err.to_string()
                .contains("uq_knowledge_claim_conflicts_pair"),
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt231_contradictory_claims_stay_conflicted_until_resolution_receipt() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!(
                "SKIP mt231_contradictory_claims_stay_conflicted_until_resolution_receipt: no PostgreSQL"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;

        let api_claim_a = pg
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5544", vec![span_id.clone()]))
            .await
            .expect("api claim a");
        let api_claim_b = pg
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5432", vec![span_id.clone()]))
            .await
            .expect("api claim b");
        let api_conflict = pg
            .db
            .record_knowledge_claim_conflict(
                &api_claim_a.claim_id,
                &api_claim_b.claim_id,
                "MT-231 contradictory runtime memory claim fixture",
                None,
            )
            .await
            .expect("record api conflict");
        let reverse_err = pg
            .db
            .record_knowledge_claim_conflict(
                &api_claim_b.claim_id,
                &api_claim_a.claim_id,
                "MT-231 reverse duplicate conflict",
                None,
            )
            .await
            .expect_err("reverse duplicate conflicts must fail closed");
        assert!(
            matches!(reverse_err, StorageError::Conflict(_)),
            "expected reverse duplicate conflict, got {reverse_err:?}"
        );
        let wrong_receipt = append_receipt_for_aggregate(
            &pg,
            "mt231-wrong",
            "knowledge_claim_conflict",
            "KCC-00000000000000000000000000000000",
        )
        .await;
        let err = pg
            .db
            .resolve_knowledge_claim_conflict(&api_conflict.conflict_id, &wrong_receipt)
            .await
            .expect_err("resolution receipt must target this conflict aggregate");
        assert!(
            matches!(err, StorageError::Conflict(_)),
            "expected aggregate mismatch conflict, got {err:?}"
        );
        assert!(
            err.to_string().contains("aggregate"),
            "unexpected receipt mismatch error: {err}"
        );
        let api_receipt =
            append_conflict_resolution_receipt(&pg, &api_conflict.conflict_id, "mt231-api").await;

        let err = pg
            .db
            .transition_knowledge_claim(
                &api_claim_a.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&api_receipt),
            )
            .await
            .expect_err("unresolved conflicted claims must not become accepted via API");
        assert!(
            matches!(err, StorageError::Conflict(_)),
            "expected typed conflict, got {err:?}"
        );
        assert!(
            err.to_string().contains("unresolved"),
            "unexpected API error: {err}"
        );
        let err = pg
            .db
            .transition_knowledge_claim(
                &api_claim_b.claim_id,
                KnowledgeClaimState::Retired,
                Some(KnowledgeClaimRetirement {
                    reason: KnowledgeClaimRetirementReason::Rejected,
                    superseded_by_claim_id: None,
                }),
                None,
            )
            .await
            .expect_err("unresolved conflicted claims must not retire via API");
        assert!(
            matches!(err, StorageError::Conflict(_)),
            "expected unresolved-retirement conflict, got {err:?}"
        );
        assert!(
            err.to_string().contains("unresolved"),
            "unexpected unresolved-retirement API error: {err}"
        );
        assert_eq!(
            pg.db
                .get_knowledge_claim(&api_claim_a.claim_id)
                .await
                .expect("get api claim")
                .expect("api claim exists")
                .lifecycle_state,
            KnowledgeClaimState::Conflicted
        );

        let resolved_api_conflict = pg
            .db
            .resolve_knowledge_claim_conflict(&api_conflict.conflict_id, &api_receipt)
            .await
            .expect("resolve api conflict");
        assert!(resolved_api_conflict.resolved_at.is_some());
        let stale_api_receipt =
            append_conflict_resolution_receipt(&pg, &api_conflict.conflict_id, "mt231-api-stale")
                .await;
        let err = pg
            .db
            .transition_knowledge_claim(
                &api_claim_a.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&stale_api_receipt),
            )
            .await
            .expect_err("acceptance receipt must be the recorded conflict resolution receipt");
        assert!(
            err.to_string().contains("match a resolved conflict"),
            "unexpected stale receipt API error: {err}"
        );
        let accepted = pg
            .db
            .transition_knowledge_claim(
                &api_claim_a.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&api_receipt),
            )
            .await
            .expect("resolved conflicted claim may become accepted");
        assert_eq!(accepted.lifecycle_state, KnowledgeClaimState::Accepted);

        let sql_claim_a = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "manual notes say port is 5544",
                vec![span_id.clone()],
            ))
            .await
            .expect("sql claim a");
        let sql_claim_b = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "manual notes say port is 5432",
                vec![span_id.clone()],
            ))
            .await
            .expect("sql claim b");
        let sql_conflict = pg
            .db
            .record_knowledge_claim_conflict(
                &sql_claim_a.claim_id,
                &sql_claim_b.claim_id,
                "MT-231 contradictory raw SQL memory claim fixture",
                None,
            )
            .await
            .expect("record sql conflict");
        let sql_receipt =
            append_conflict_resolution_receipt(&pg, &sql_conflict.conflict_id, "mt231-sql").await;

        let mut conn = pg.raw_connection().await;
        let raw_reverse_err = sqlx::query(
            "INSERT INTO knowledge_claim_conflicts
                 (conflict_id, claim_id, conflicting_claim_id, conflict_reason)
             VALUES ($1, $2, $3, 'MT-231 raw reverse duplicate conflict')",
        )
        .bind(format!("KCC-{}", Uuid::now_v7().simple()))
        .bind(&sql_claim_b.claim_id)
        .bind(&sql_claim_a.claim_id)
        .execute(&mut conn)
        .await
        .expect_err("DB must reject reverse duplicate conflict rows");
        assert!(
            raw_reverse_err
                .to_string()
                .contains("uq_knowledge_claim_conflicts_unordered_pair"),
            "unexpected raw reverse duplicate error: {raw_reverse_err}"
        );

        let wrong_sql_receipt = append_receipt_for_aggregate(
            &pg,
            "mt231-sql-wrong",
            "knowledge_claim_conflict",
            &api_conflict.conflict_id,
        )
        .await;
        let wrong_resolution_err = sqlx::query(
            "UPDATE knowledge_claim_conflicts
             SET resolution_receipt_event_id = $2, resolved_at = NOW()
             WHERE conflict_id = $1",
        )
        .bind(&sql_conflict.conflict_id)
        .bind(&wrong_sql_receipt)
        .execute(&mut conn)
        .await
        .expect_err("DB trigger must reject wrong conflict receipt aggregate");
        assert!(
            wrong_resolution_err.to_string().contains("aggregate"),
            "unexpected wrong-receipt SQL error: {wrong_resolution_err}"
        );

        let legacy_claim_a = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "legacy row says mode is local",
                vec![span_id.clone()],
            ))
            .await
            .expect("legacy claim a");
        let legacy_claim_b = pg
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "legacy row says mode is cloud",
                vec![span_id.clone()],
            ))
            .await
            .expect("legacy claim b");
        let legacy_conflict = pg
            .db
            .record_knowledge_claim_conflict(
                &legacy_claim_a.claim_id,
                &legacy_claim_b.claim_id,
                "MT-231 legacy wrong aggregate resolved conflict fixture",
                None,
            )
            .await
            .expect("record legacy conflict");
        let legacy_wrong_receipt = append_receipt_for_aggregate(
            &pg,
            "mt231-legacy-wrong",
            "knowledge_memory_fixture",
            "legacy-conflict-resolution",
        )
        .await;
        sqlx::query(
            "ALTER TABLE knowledge_claim_conflicts
             DISABLE TRIGGER trg_knowledge_claim_conflict_resolution_receipt_guard",
        )
        .execute(&mut conn)
        .await
        .expect("temporarily disable receipt guard");
        sqlx::query(
            "UPDATE knowledge_claim_conflicts
             SET resolution_receipt_event_id = $2, resolved_at = NOW()
             WHERE conflict_id = $1",
        )
        .bind(&legacy_conflict.conflict_id)
        .bind(&legacy_wrong_receipt)
        .execute(&mut conn)
        .await
        .expect("simulate pre-MT-231 wrong-aggregate resolved conflict row");
        sqlx::query(
            "ALTER TABLE knowledge_claim_conflicts
             ENABLE TRIGGER trg_knowledge_claim_conflict_resolution_receipt_guard",
        )
        .execute(&mut conn)
        .await
        .expect("re-enable receipt guard");
        let err = pg
            .db
            .transition_knowledge_claim(
                &legacy_claim_a.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&legacy_wrong_receipt),
            )
            .await
            .expect_err(
                "legacy wrong-aggregate conflict receipt must not authorize API acceptance",
            );
        assert!(
            err.to_string().contains("aggregate"),
            "unexpected legacy wrong aggregate API error: {err}"
        );
        let legacy_sql_err = sqlx::query(
            "UPDATE knowledge_claims
             SET lifecycle_state = 'accepted',
                 resolution_receipt_event_id = $2,
                 updated_at = NOW()
             WHERE claim_id = $1",
        )
        .bind(&legacy_claim_a.claim_id)
        .bind(&legacy_wrong_receipt)
        .execute(&mut conn)
        .await
        .expect_err("legacy wrong-aggregate conflict receipt must not authorize SQL acceptance");
        assert!(
            legacy_sql_err.to_string().contains("aggregate"),
            "unexpected legacy wrong aggregate SQL error: {legacy_sql_err}"
        );

        let retire_err = sqlx::query(
            "UPDATE knowledge_claims
             SET lifecycle_state = 'retired',
                 retirement_reason = 'rejected',
                 updated_at = NOW()
             WHERE claim_id = $1",
        )
        .bind(&sql_claim_b.claim_id)
        .execute(&mut conn)
        .await
        .expect_err("DB trigger must reject unresolved conflicted -> retired");
        assert!(
            retire_err.to_string().contains("unresolved"),
            "unexpected unresolved-retirement SQL error: {retire_err}"
        );

        let err = sqlx::query(
            "UPDATE knowledge_claims
             SET lifecycle_state = 'accepted',
                 resolution_receipt_event_id = $2,
                 updated_at = NOW()
             WHERE claim_id = $1",
        )
        .bind(&sql_claim_a.claim_id)
        .bind(&sql_receipt)
        .execute(&mut conn)
        .await
        .expect_err("DB trigger must reject unresolved conflicted -> accepted");
        assert!(
            err.to_string().contains("unresolved"),
            "unexpected DB error: {err}"
        );
        assert_eq!(
            pg.db
                .get_knowledge_claim(&sql_claim_a.claim_id)
                .await
                .expect("get sql claim")
                .expect("sql claim exists")
                .lifecycle_state,
            KnowledgeClaimState::Conflicted
        );

        pg.db
            .resolve_knowledge_claim_conflict(&sql_conflict.conflict_id, &sql_receipt)
            .await
            .expect("resolve sql conflict");
        let stale_sql_receipt =
            append_conflict_resolution_receipt(&pg, &sql_conflict.conflict_id, "mt231-sql-stale")
                .await;
        let stale_sql_err = sqlx::query(
            "UPDATE knowledge_claims
             SET lifecycle_state = 'accepted',
                 resolution_receipt_event_id = $2,
                 updated_at = NOW()
             WHERE claim_id = $1",
        )
        .bind(&sql_claim_a.claim_id)
        .bind(&stale_sql_receipt)
        .execute(&mut conn)
        .await
        .expect_err("DB trigger must reject non-recorded conflict receipt");
        assert!(
            stale_sql_err
                .to_string()
                .contains("match a resolved conflict"),
            "unexpected stale receipt SQL error: {stale_sql_err}"
        );
        sqlx::query(
            "UPDATE knowledge_claims
             SET lifecycle_state = 'accepted',
                 resolution_receipt_event_id = $2,
                 updated_at = NOW()
             WHERE claim_id = $1",
        )
        .bind(&sql_claim_a.claim_id)
        .bind(&sql_receipt)
        .execute(&mut conn)
        .await
        .expect("resolved raw SQL conflicted -> accepted is allowed");
        assert_eq!(
            pg.db
                .get_knowledge_claim(&sql_claim_a.claim_id)
                .await
                .expect("get accepted sql claim")
                .expect("accepted sql claim exists")
                .lifecycle_state,
            KnowledgeClaimState::Accepted
        );
    }
}

// ---------------------------------------------------------------------------
// MT-057 PassageEvidenceTables
// ---------------------------------------------------------------------------

mod mt_057_passages {
    use super::*;
    use handshake_core::storage::knowledge::{
        KnowledgeClaimKind, KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef,
        KnowledgeRetrievalMode, NewKnowledgeClaim, NewKnowledgeMemoryPassage,
    };
    use handshake_core::storage::StorageError;

    fn passage(
        workspace_id: &str,
        evidence: Vec<KnowledgePassageEvidenceRef>,
    ) -> NewKnowledgeMemoryPassage {
        NewKnowledgeMemoryPassage {
            workspace_id: workspace_id.to_string(),
            passage_text: "The managed PostgreSQL cluster listens on port 5544 by default."
                .to_string(),
            token_count: Some(14),
            ocr_transcript_metadata: None,
            extraction_confidence: 0.92,
            ranking_features: json!({"recency_score": 0.8, "pin_weight": 0.0}),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            compaction_policy: KnowledgeCompactionPolicy::Keep,
            failure_receipt_event_id: None,
            derived_in_run: None,
            evidence,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn passage_roundtrip_with_mixed_evidence_lineage() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP passage_roundtrip_with_mixed_evidence_lineage: no PostgreSQL");
            return;
        };
        let (workspace_id, source_id, span_id) = span_fixture(&pg).await;
        let claim = pg
            .db
            .create_knowledge_claim(NewKnowledgeClaim {
                workspace_id: workspace_id.clone(),
                claim_kind: KnowledgeClaimKind::ProductBehavior,
                claim_text: "managed PG default port is 5544".to_string(),
                subject_entity_id: None,
                temporal_qualifier: None,
                granularity_qualifier: None,
                confidence: 0.9,
                proposed_in_run: None,
                evidence_span_ids: vec![span_id.clone()],
            })
            .await
            .expect("claim");

        let evidence = vec![
            KnowledgePassageEvidenceRef::Source {
                source_id: source_id.clone(),
            },
            KnowledgePassageEvidenceRef::Claim {
                claim_id: claim.claim_id.clone(),
            },
            KnowledgePassageEvidenceRef::Span {
                span_id: span_id.clone(),
            },
        ];
        let created = pg
            .db
            .create_knowledge_memory_passage(passage(&workspace_id, evidence.clone()))
            .await
            .expect("create passage");
        assert!(created.passage_id.starts_with("KMP-"));
        assert_eq!(created.retrieval_mode, KnowledgeRetrievalMode::HybridRag);
        assert_eq!(created.compaction_policy, KnowledgeCompactionPolicy::Keep);
        assert!((created.extraction_confidence - 0.92).abs() < f64::EPSILON);

        let fetched = pg
            .db
            .get_knowledge_memory_passage(&created.passage_id)
            .await
            .expect("get passage")
            .expect("passage exists");
        assert_eq!(fetched, created);

        let lineage = pg
            .db
            .list_knowledge_passage_evidence(&created.passage_id)
            .await
            .expect("lineage");
        assert_eq!(lineage, evidence, "lineage must round-trip in order");

        // Compaction lifecycle: keep -> compactable refreshes policy.
        let compactable = pg
            .db
            .set_knowledge_passage_compaction(
                &created.passage_id,
                KnowledgeCompactionPolicy::Compactable,
                true,
            )
            .await
            .expect("set compaction");
        assert_eq!(
            compactable.compaction_policy,
            KnowledgeCompactionPolicy::Compactable
        );
        assert!(compactable.freshness_at >= created.freshness_at);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn passages_require_lineage_at_every_layer() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP passages_require_lineage_at_every_layer: no PostgreSQL");
            return;
        };
        let (workspace_id, _source_id, _span_id) = span_fixture(&pg).await;

        // Rust layer: lineage-free passages are rejected.
        let err = pg
            .db
            .create_knowledge_memory_passage(passage(&workspace_id, vec![]))
            .await
            .expect_err("passages must carry derivation lineage");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

        // Ghost lineage rolls the whole insert back (FK violation).
        let err = pg
            .db
            .create_knowledge_memory_passage(passage(
                &workspace_id,
                vec![KnowledgePassageEvidenceRef::Span {
                    span_id: "KSP-00000000000000000000000000000000".to_string(),
                }],
            ))
            .await
            .expect_err("ghost span lineage must violate the FK");
        assert!(err.to_string().contains("foreign key"), "got {err}");

        // DB layer: direct INSERT without lineage fails at COMMIT.
        let mut conn = pg.raw_connection().await;
        sqlx::query("BEGIN")
            .execute(&mut conn)
            .await
            .expect("begin");
        sqlx::query(
            "INSERT INTO knowledge_memory_passages
                 (passage_id, workspace_id, passage_text)
             VALUES ('KMP-00000000000000000000000000000001', $1, 'rogue passage')",
        )
        .bind(&workspace_id)
        .execute(&mut conn)
        .await
        .expect("insert inside transaction");
        let err = sqlx::query("COMMIT")
            .execute(&mut conn)
            .await
            .expect_err("commit must fail without lineage");
        assert!(
            err.to_string().contains("derived from sources and claims"),
            "unexpected commit error: {err}"
        );

        // Evidence shape CHECK: ref_kind/ref column mismatch is rejected.
        let real = pg
            .db
            .create_knowledge_memory_passage(passage(
                &workspace_id,
                vec![KnowledgePassageEvidenceRef::Source {
                    source_id: span_fixture(&pg).await.1,
                }],
            ))
            .await
            .expect("real passage");
        let err = sqlx::query(
            "INSERT INTO knowledge_passage_evidence
                 (passage_id, ref_kind, claim_id, ordinal)
             VALUES ($1, 'span', NULL, 99)",
        )
        .bind(&real.passage_id)
        .execute(&mut conn)
        .await
        .expect_err("ref_kind without matching ref column must violate CHECK");
        assert!(
            err.to_string()
                .contains("chk_knowledge_passage_evidence_shape"),
            "unexpected: {err}"
        );
    }
}
