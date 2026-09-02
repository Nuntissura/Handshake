//! WP-KERNEL-009 knowledge-claim and passage integration tests against the real
//! embedded Handshake storage authority: MT-056 (KnowledgeClaimTables) and
//! MT-057 (PassageEvidenceTables).

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::storage::knowledge::{
    KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
    KnowledgeRootKind, KnowledgeSourceKind, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeSource,
    NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use handshake_core::storage::surreal::RowFilter;
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const HASH_SPAN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// workspace -> root -> source -> span fixture.
async fn span_fixture(store: &EmbeddedKnowledgeStore) -> (String, String, String) {
    let workspace_id = store.create_workspace().await;
    let root = store
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
    let source = store
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
    let span = store
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

async fn inspected_row_count(store: &EmbeddedKnowledgeStore, table_name: &str) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .unwrap_or_else(|error| panic!("select inspector table {table_name}: {error}"));
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("count inspector table {table_name}: {error}"))
}

async fn inspected_row_field(
    store: &EmbeddedKnowledgeStore,
    table_name: &str,
    record_id: &str,
    field_name: &str,
) -> serde_json::Value {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .unwrap_or_else(|error| panic!("select inspector table {table_name}: {error}"));
    let field = table.field(field_name).unwrap_or_else(|error| {
        panic!("select inspector field {table_name}.{field_name}: {error}")
    });
    let mut rows = inspector
        .project(&table, &[field], RowFilter::IdEquals(record_id.to_owned()))
        .await
        .unwrap_or_else(|error| panic!("inspect {table_name}:{record_id}: {error}"));
    assert_eq!(
        rows.len(),
        1,
        "expected exactly one {table_name}:{record_id} row"
    );
    rows.pop()
        .expect("inspected row")
        .values
        .remove(field_name)
        .unwrap_or_else(|| panic!("inspected row omitted {table_name}.{field_name}"))
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
        store: &EmbeddedKnowledgeStore,
        label: &str,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> String {
        let suffix = Uuid::now_v7();
        store
            .db
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
        store: &EmbeddedKnowledgeStore,
        conflict_id: &str,
        label: &str,
    ) -> String {
        append_receipt_for_aggregate(store, label, "knowledge_claim_conflict", conflict_id).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_lifecycle_proposed_accepted_retired_with_receipts() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP claim_lifecycle_proposed_accepted_retired_with_receipts: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&store).await;

        let created = store
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "knowledge storage uses the embedded authority store",
                vec![span_id.clone()],
            ))
            .await
            .expect("create claim");
        assert!(created.claim_id.starts_with("KCL-"));
        assert_eq!(created.lifecycle_state, KnowledgeClaimState::Proposed);
        assert_eq!(
            store
                .db
                .list_knowledge_claim_span_ids(&created.claim_id)
                .await
                .expect("claim evidence"),
            vec![span_id.clone()]
        );

        // Acceptance backed by a real EventLedger receipt.
        let suffix = Uuid::now_v7();
        let receipt = store
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
        let accepted = store
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
        let successor = store
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "knowledge storage fails closed with typed StorageError",
                vec![span_id.clone()],
            ))
            .await
            .expect("successor claim");
        let retired = store
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
        let err = store
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
    async fn claim_lifecycle_rejects_backward_and_terminal_transitions_without_mutation() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP claim_lifecycle_rejects_backward_and_terminal_transitions_without_mutation: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&store).await;
        let created = store
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "claim lifecycle remains monotonic",
                vec![span_id],
            ))
            .await
            .expect("create lifecycle rejection claim");
        let acceptance_receipt = append_receipt_for_aggregate(
            &store,
            "lifecycle-accept",
            "knowledge_claim",
            &created.claim_id,
        )
        .await;
        let accepted = store
            .db
            .transition_knowledge_claim(
                &created.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&acceptance_receipt),
            )
            .await
            .expect("accept lifecycle rejection claim");

        let backward_error = store
            .db
            .transition_knowledge_claim(
                &accepted.claim_id,
                KnowledgeClaimState::Proposed,
                None,
                None,
            )
            .await
            .expect_err("accepted claims must not return to proposed");
        assert!(
            matches!(backward_error, StorageError::Conflict(_)),
            "unexpected backward-transition error: {backward_error:?}"
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &accepted.claim_id,
                "lifecycle_state",
            )
            .await,
            json!("accepted"),
            "a rejected backward transition must not alter persisted state"
        );

        let retired = store
            .db
            .transition_knowledge_claim(
                &accepted.claim_id,
                KnowledgeClaimState::Retired,
                Some(KnowledgeClaimRetirement {
                    reason: KnowledgeClaimRetirementReason::OperatorRetired,
                    superseded_by_claim_id: None,
                }),
                None,
            )
            .await
            .expect("retire lifecycle rejection claim");
        let terminal_error = store
            .db
            .transition_knowledge_claim(
                &retired.claim_id,
                KnowledgeClaimState::Accepted,
                None,
                Some(&acceptance_receipt),
            )
            .await
            .expect_err("retired claims must remain terminal");
        assert!(
            matches!(terminal_error, StorageError::Conflict(_)),
            "unexpected terminal-transition error: {terminal_error:?}"
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &retired.claim_id,
                "lifecycle_state",
            )
            .await,
            json!("retired")
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &retired.claim_id,
                "retirement_reason",
            )
            .await,
            json!("operator_retired")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_creation_and_retirement_reject_invalid_shapes_without_persisting() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP claim_creation_and_retirement_reject_invalid_shapes_without_persisting: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&store).await;

        assert_eq!(inspected_row_count(&store, "knowledge_claims").await, 0);
        let err = store
            .db
            .create_knowledge_claim(claim(&workspace_id, "evidence-free claim", vec![]))
            .await
            .expect_err("claims without evidence spans must be rejected");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
        assert_eq!(
            inspected_row_count(&store, "knowledge_claims").await,
            0,
            "rejected evidence-free claim must not create an authority row"
        );
        assert_eq!(
            inspected_row_count(&store, "knowledge_claim_spans").await,
            0,
            "rejected evidence-free claim must not create an evidence row"
        );

        let valid = store
            .db
            .create_knowledge_claim(claim(
                &workspace_id,
                "retirement shape is validated by the public API",
                vec![span_id],
            ))
            .await
            .expect("create claim for retirement-shape proof");
        let err = store
            .db
            .transition_knowledge_claim(&valid.claim_id, KnowledgeClaimState::Retired, None, None)
            .await
            .expect_err("retired claims must carry a retirement reason");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
        assert!(
            err.to_string().contains("retirement reason"),
            "unexpected retirement-shape error: {err}"
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &valid.claim_id,
                "lifecycle_state",
            )
            .await,
            json!("proposed"),
            "invalid retirement shape must leave the claim proposed"
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &valid.claim_id,
                "retirement_reason",
            )
            .await,
            serde_json::Value::Null
        );
        assert_eq!(inspected_row_count(&store, "knowledge_claims").await, 1);
        assert_eq!(
            inspected_row_count(&store, "knowledge_claim_spans").await,
            1
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conflict_detection_and_receipt_backed_resolution() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP conflict_detection_and_receipt_backed_resolution: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&store).await;

        let claim_a = store
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5544", vec![span_id.clone()]))
            .await
            .expect("claim a");
        let claim_b = store
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5432", vec![span_id.clone()]))
            .await
            .expect("claim b");

        let conflict = store
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
            let state = store
                .db
                .get_knowledge_claim(id)
                .await
                .expect("get claim")
                .expect("claim exists")
                .lifecycle_state;
            assert_eq!(state, KnowledgeClaimState::Conflicted);
        }

        // Self-conflict and duplicate pair fail closed.
        let err = store
            .db
            .record_knowledge_claim_conflict(&claim_a.claim_id, &claim_a.claim_id, "self", None)
            .await
            .expect_err("self-conflict must be rejected");
        assert!(matches!(err, StorageError::Validation(_)));
        let err = store
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
        let err = store
            .db
            .resolve_knowledge_claim_conflict(&conflict.conflict_id, "KE-GHOST")
            .await
            .expect_err("resolution receipt must reference a real ledger event");
        assert!(err.to_string().contains("foreign key"), "got {err}");

        let suffix = Uuid::now_v7();
        let receipt = store
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
        let resolved = store
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
        let err = store
            .db
            .resolve_knowledge_claim_conflict(&conflict.conflict_id, &receipt.event_id)
            .await
            .expect_err("conflicts resolve exactly once");
        assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");

        // Winning claim returns to accepted through the guarded transition.
        let accepted = store
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

        let conflicts = store
            .db
            .list_knowledge_claim_conflicts(&claim_a.claim_id)
            .await
            .expect("list conflicts");
        assert_eq!(conflicts.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt231_contradictory_claims_stay_conflicted_until_resolution_receipt() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP mt231_contradictory_claims_stay_conflicted_until_resolution_receipt: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, span_id) = span_fixture(&store).await;

        let api_claim_a = store
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5544", vec![span_id.clone()]))
            .await
            .expect("api claim a");
        let api_claim_b = store
            .db
            .create_knowledge_claim(claim(&workspace_id, "port is 5432", vec![span_id.clone()]))
            .await
            .expect("api claim b");
        let api_conflict = store
            .db
            .record_knowledge_claim_conflict(
                &api_claim_a.claim_id,
                &api_claim_b.claim_id,
                "MT-231 contradictory runtime memory claim fixture",
                None,
            )
            .await
            .expect("record api conflict");
        let reverse_err = store
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
        assert_eq!(
            inspected_row_count(&store, "knowledge_claim_conflicts").await,
            1,
            "reverse duplicate rejection must not create a second conflict row"
        );
        let wrong_receipt = append_receipt_for_aggregate(
            &store,
            "mt231-wrong",
            "knowledge_claim_conflict",
            "KCC-00000000000000000000000000000000",
        )
        .await;
        let err = store
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
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claim_conflicts",
                &api_conflict.conflict_id,
                "resolved_at",
            )
            .await,
            serde_json::Value::Null,
            "wrong aggregate receipt must leave the conflict unresolved"
        );
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claim_conflicts",
                &api_conflict.conflict_id,
                "resolution_receipt_event_id",
            )
            .await,
            serde_json::Value::Null
        );
        let api_receipt =
            append_conflict_resolution_receipt(&store, &api_conflict.conflict_id, "mt231-api")
                .await;

        let err = store
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
        let err = store
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
            store
                .db
                .get_knowledge_claim(&api_claim_a.claim_id)
                .await
                .expect("get api claim")
                .expect("api claim exists")
                .lifecycle_state,
            KnowledgeClaimState::Conflicted
        );
        for claim_id in [&api_claim_a.claim_id, &api_claim_b.claim_id] {
            assert_eq!(
                inspected_row_field(&store, "knowledge_claims", claim_id, "lifecycle_state").await,
                json!("conflicted"),
                "unresolved conflict exit must not alter persisted claim state"
            );
        }

        let resolved_api_conflict = store
            .db
            .resolve_knowledge_claim_conflict(&api_conflict.conflict_id, &api_receipt)
            .await
            .expect("resolve api conflict");
        assert!(resolved_api_conflict.resolved_at.is_some());
        let stale_api_receipt = append_conflict_resolution_receipt(
            &store,
            &api_conflict.conflict_id,
            "mt231-api-stale",
        )
        .await;
        let err = store
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
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &api_claim_a.claim_id,
                "lifecycle_state",
            )
            .await,
            json!("conflicted"),
            "stale receipt rejection must leave the claim conflicted"
        );
        let accepted = store
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
        assert_eq!(
            inspected_row_field(
                &store,
                "knowledge_claims",
                &api_claim_a.claim_id,
                "lifecycle_state",
            )
            .await,
            json!("accepted")
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
            passage_text: "The knowledge index uses hybrid retrieval by default.".to_string(),
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
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP passage_roundtrip_with_mixed_evidence_lineage: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, source_id, span_id) = span_fixture(&store).await;
        let claim = store
            .db
            .create_knowledge_claim(NewKnowledgeClaim {
                workspace_id: workspace_id.clone(),
                claim_kind: KnowledgeClaimKind::ProductBehavior,
                claim_text: "knowledge retrieval defaults to hybrid mode".to_string(),
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
        let created = store
            .db
            .create_knowledge_memory_passage(passage(&workspace_id, evidence.clone()))
            .await
            .expect("create passage");
        assert!(created.passage_id.starts_with("KMP-"));
        assert_eq!(created.retrieval_mode, KnowledgeRetrievalMode::HybridRag);
        assert_eq!(created.compaction_policy, KnowledgeCompactionPolicy::Keep);
        assert!((created.extraction_confidence - 0.92).abs() < f64::EPSILON);

        let fetched = store
            .db
            .get_knowledge_memory_passage(&created.passage_id)
            .await
            .expect("get passage")
            .expect("passage exists");
        assert_eq!(fetched, created);

        let lineage = store
            .db
            .list_knowledge_passage_evidence(&created.passage_id)
            .await
            .expect("lineage");
        assert_eq!(lineage, evidence, "lineage must round-trip in order");

        // Compaction lifecycle: keep -> compactable refreshes policy.
        let compactable = store
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
    async fn passage_creation_rejects_missing_or_ghost_lineage_without_persisting() {
        let Some(store) = open_embedded_store().await else {
            eprintln!(
                "SKIP passage_creation_rejects_missing_or_ghost_lineage_without_persisting: embedded store unavailable"
            );
            return;
        };
        let (workspace_id, _source_id, _span_id) = span_fixture(&store).await;

        assert_eq!(
            inspected_row_count(&store, "knowledge_memory_passages").await,
            0
        );
        let err = store
            .db
            .create_knowledge_memory_passage(passage(&workspace_id, vec![]))
            .await
            .expect_err("passages must carry derivation lineage");
        assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
        assert_eq!(
            inspected_row_count(&store, "knowledge_memory_passages").await,
            0
        );
        assert_eq!(
            inspected_row_count(&store, "knowledge_passage_evidence").await,
            0
        );

        for ghost_lineage in [
            KnowledgePassageEvidenceRef::Source {
                source_id: "KSRC-00000000000000000000000000000000".to_string(),
            },
            KnowledgePassageEvidenceRef::Claim {
                claim_id: "KCL-00000000000000000000000000000000".to_string(),
            },
            KnowledgePassageEvidenceRef::Span {
                span_id: "KSP-00000000000000000000000000000000".to_string(),
            },
        ] {
            let err = store
                .db
                .create_knowledge_memory_passage(passage(&workspace_id, vec![ghost_lineage]))
                .await
                .expect_err("ghost lineage must be rejected");
            assert!(err.to_string().contains("foreign key"), "got {err}");
            assert_eq!(
                inspected_row_count(&store, "knowledge_memory_passages").await,
                0,
                "ghost lineage failure must roll back the passage row"
            );
            assert_eq!(
                inspected_row_count(&store, "knowledge_passage_evidence").await,
                0,
                "ghost lineage failure must roll back the evidence row"
            );
        }
    }
}
