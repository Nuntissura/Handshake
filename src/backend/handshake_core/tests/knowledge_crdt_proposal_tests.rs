//! WP-KERNEL-009 CRDTAndConcurrencyCore proposal tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_068_graph_proposals: MT-068 GraphMutationProposalModel
//!   - mt_069_claim_promotion: MT-069 ClaimPromotionBridge
//!   - mt_074_ai_edit_proposals: MT-074 AiEditProposalReviewFlow
//!
//! All durable assertions run against the real isolated embedded SurrealDB
//! store and its migrations.

use handshake_core::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1,
};
use handshake_core::storage::knowledge::{
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind, KnowledgeSpanKind,
    KnowledgeStore, NewKnowledgeSource, NewKnowledgeSpan,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::{Database, NewWorkspace, StorageError, WriteContext};
use serde_json::json;
use uuid::Uuid;

async fn embedded_backend_or_blocked() -> EmbeddedTestBackend {
    match embedded_test_backend().await {
        Ok(backend) => backend,
        Err(err) => panic!("failed to init embedded backend: {err:?}"),
    }
}

async fn reopen_embedded_store(
    backend: &EmbeddedTestBackend,
) -> (SurrealStorage, std::sync::Arc<dyn Database>) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded CRDT proposal store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded CRDT proposal store"),
    )
    .await
    .expect("reopen embedded CRDT proposal store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened CRDT proposal schema");
    let database: std::sync::Arc<dyn Database> =
        std::sync::Arc::new(SurrealDatabase::new(reopened.clone()));
    (reopened, database)
}

async fn close_reopened_and_remove(reopened: SurrealStorage, backend: EmbeddedTestBackend) {
    reopened
        .shutdown()
        .await
        .expect("close reopened CRDT proposal store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded CRDT proposal store");
}

/// Authority-hardening #1 fixture: create a real workspace + source + span and
/// return `(workspace_id, span_id)` so a proposal in `workspace_id` citing
/// `span_id` has promotable (live, same-workspace, non-stale) evidence.
async fn live_span_fixture(backend: &EmbeddedTestBackend, label: &str) -> (String, String) {
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace_id = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("span-fixture-{label}"),
            },
        )
        .await
        .expect("create workspace")
        .id;
    let hash = "a".repeat(64);
    let source = db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::ExternalImport,
            relative_path: None,
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: hash.clone(),
            size_bytes: Some(16),
            provenance: json!({"fixture": "crdt_proposal", "label": label}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("create source");
    let span = db
        .create_knowledge_span(NewKnowledgeSpan {
            source_id: source.source_id,
            span_kind: KnowledgeSpanKind::Byte,
            range_start: 0,
            range_end: 16,
            line_start: None,
            line_end: None,
            section_path: None,
            content_sha256: hash,
            parser_version: "v1".to_string(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("embedded CRDT proposal span".to_string()),
        })
        .await
        .expect("create span");
    (workspace_id, span.span_id)
}

/// Claim a workspace-scope lane lease for a model actor (proposals from
/// model actors require one, MT-041 seed).
async fn model_lease(
    backend: &EmbeddedTestBackend,
    actor: &KnowledgeActorIdV1,
    workspace_id: &str,
    session_id: &str,
) -> String {
    let outcome = claim_lease(
        backend.database.as_ref(),
        &backend.storage,
        LeaseClaimRequestV1 {
            lane_id: format!("lane-{session_id}"),
            actor: actor.clone(),
            session_id: session_id.to_string(),
            correlation_id: format!("corr-{session_id}"),
            scope_kind: KnowledgeLeaseScopeKind::Workspace,
            scope_id: workspace_id.to_string(),
            ttl_seconds: 600,
        },
    )
    .await
    .expect("lease claim flow");
    match outcome {
        LeaseClaimOutcomeV1::Claimed(lease) => lease.lease_id.clone(),
        other => panic!("expected lease claim, got {other:?}"),
    }
}

mod mt_068_graph_proposals {
    use super::*;
    use handshake_core::kernel::crdt::graph_proposal::{
        decide_graph_proposal, record_graph_proposal, validate_graph_proposal_request,
        GraphMutationKind, GraphMutationProposalRequestV1, GraphProposalValidationError,
        ProposalDecisionError, ProposalReviewState, RecordGraphProposalOutcomeV1,
    };
    use handshake_core::kernel::KernelEventType;
    use serde_json::json;
    use uuid::Uuid;

    fn request(
        ws: &str,
        actor: &KnowledgeActorIdV1,
        lease_id: Option<String>,
    ) -> GraphMutationProposalRequestV1 {
        GraphMutationProposalRequestV1 {
            workspace_id: ws.to_string(),
            mutation_kind: GraphMutationKind::AddClaim,
            mutation_payload: json!({
                "claim_text": "managed_storage.rs starts the embedded cluster on port 5544",
                "claim_kind": "product_behavior"
            }),
            source_span_refs: vec![format!("KSP-{:032x}", 0xfeedu32)],
            confidence: 0.83,
            actor: actor.clone(),
            session_id: "sr-mt068".to_string(),
            correlation_id: "corr-mt068".to_string(),
            lease_id,
        }
    }

    #[test]
    fn review_state_machine_is_pinned() {
        use ProposalReviewState as S;
        assert!(S::Proposed.can_transition_to(S::Approved));
        assert!(S::Proposed.can_transition_to(S::Rejected));
        assert!(S::Approved.can_transition_to(S::Promoted));
        for (from, to) in [
            (S::Approved, S::Rejected),
            (S::Rejected, S::Approved),
            (S::Rejected, S::Promoted),
            (S::Promoted, S::Approved),
            (S::Proposed, S::Promoted),
        ] {
            assert!(
                !from.can_transition_to(to),
                "{from:?} -> {to:?} must be illegal"
            );
        }
    }

    #[test]
    fn validation_rejects_unevidenced_or_unleased_proposals() {
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "graph-lm").expect("actor");

        let mut no_spans = request("ws", &model, Some("lease".to_string()));
        no_spans.source_span_refs.clear();
        assert!(validate_graph_proposal_request(&no_spans)
            .expect_err("no spans must fail")
            .iter()
            .any(|error| matches!(error, GraphProposalValidationError::NoSourceSpanRefs)));

        let unleased = request("ws", &model, None);
        assert!(validate_graph_proposal_request(&unleased)
            .expect_err("model without lease must fail")
            .iter()
            .any(|error| matches!(
                error,
                GraphProposalValidationError::ModelActorWithoutLease { .. }
            )));

        let mut bad_confidence = request("ws", &model, Some("lease".to_string()));
        bad_confidence.confidence = 1.7;
        assert!(validate_graph_proposal_request(&bad_confidence)
            .expect_err("confidence > 1 must fail")
            .iter()
            .any(|error| matches!(
                error,
                GraphProposalValidationError::ConfidenceOutOfRange { .. }
            )));

        // Operator proposals do not need a lease.
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        assert!(validate_graph_proposal_request(&request("ws", &operator, None)).is_ok());
    }

    /// Embedded-store proof: proposal recorded with event receipt; reviewed by a
    /// validator; double decisions and model reviewers are refused by typed
    /// server validation. The direct unevidenced-insert branch is also checked
    /// through the embedded storage mutation seam below.
    #[tokio::test]
    async fn proposal_lifecycle_with_event_receipts_on_embedded_store() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt068-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "graph-lm").expect("actor");
        let validator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Validator, "wp-val").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt068-{suffix}")).await;

        let recorded = match record_graph_proposal(
            db.as_ref(),
            &pool,
            request(&ws, &model, Some(lease_id.clone())),
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        assert_eq!(recorded.review_state, "proposed");
        assert_eq!(recorded.actor_id, model.canonical());
        assert_eq!(recorded.lease_id.as_deref(), Some(lease_id.as_str()));

        // Recorded event receipt exists.
        let events = db
            .list_kernel_events_for_aggregate("knowledge_graph_proposal", &recorded.proposal_id)
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::GraphMutationProposalRecorded));

        // A model actor cannot review.
        let model_review = decide_graph_proposal(
            db.as_ref(),
            &pool,
            &recorded.proposal_id,
            true,
            &model,
            "sr-review",
            "self-approval attempt",
        )
        .await
        .expect("decide flow");
        assert!(matches!(
            model_review,
            Err(ProposalDecisionError::ReviewerNotAllowed { .. })
        ));

        // Validator approves.
        let approved = decide_graph_proposal(
            db.as_ref(),
            &pool,
            &recorded.proposal_id,
            true,
            &validator,
            "sr-review",
            "claim is span-backed and accurate",
        )
        .await
        .expect("decide flow")
        .expect("approval lands");
        assert_eq!(approved.review_state, "approved");
        assert_eq!(approved.decided_by.as_deref(), Some("validator:wp-val"));
        assert!(approved.decided_event_id.is_some());

        // Double decision is refused with the current state.
        let double = decide_graph_proposal(
            db.as_ref(),
            &pool,
            &recorded.proposal_id,
            false,
            &validator,
            "sr-review",
            "flip-flop attempt",
        )
        .await
        .expect("decide flow");
        assert!(matches!(
            double,
            Err(ProposalDecisionError::NotInProposedState { ref current_state }) if current_state == "approved"
        ));

        // The schema-negative direct-insert branch is covered by the active
        // storage-seam proof below.
    }

    #[tokio::test]
    async fn unevidenced_graph_proposal_is_rejected_by_embedded_boundary() {
        let backend = embedded_backend_or_blocked().await;
        let proposal_id = format!("KGP-{}", Uuid::now_v7().simple());

        // Exercise the real embedded mutation seam with the schema-negative
        // shape: a graph proposal without any source-span evidence.
        let err = handshake_core::storage::knowledge_crdt::insert_graph_proposal(
            &backend.storage,
            handshake_core::storage::knowledge_crdt::NewGraphMutationProposal {
                proposal_id: proposal_id.clone(),
                workspace_id: format!("ws-graph-negative-{}", Uuid::now_v7().simple()),
                mutation_kind: "add_claim".to_string(),
                mutation_payload: json!({"claim_text": "unevidenced claim"}),
                source_span_refs: Vec::new(),
                confidence: 0.5,
                actor_id: "operator:graph-negative".to_string(),
                actor_kind: "operator".to_string(),
                session_id: "sr-graph-negative".to_string(),
                correlation_id: "corr-graph-negative".to_string(),
                lease_id: None,
                recorded_event_id: "KE-graph-negative-never-written".to_string(),
            },
        )
        .await
        .expect_err("unevidenced graph proposals must be rejected");
        assert!(matches!(
            err,
            StorageError::Validation(message)
                if message.contains("at least one non-empty source span ref")
        ));

        // The rejected mutation must not leave a durable proposal row behind.
        assert!(
            handshake_core::storage::knowledge_crdt::get_graph_proposal(
                &backend.storage,
                &proposal_id,
            )
            .await
            .expect("read rejected graph proposal")
            .is_none(),
            "rejected unevidenced proposal must not persist"
        );
    }
}

mod mt_069_claim_promotion {
    use super::*;
    use handshake_core::kernel::crdt::claim_promotion::{
        promote_graph_proposal, GraphPromotionDenialReasonV1, GraphPromotionOutcomeV1,
    };
    use handshake_core::kernel::crdt::graph_proposal::{
        decide_graph_proposal, record_graph_proposal, GraphMutationKind,
        GraphMutationProposalRequestV1, RecordGraphProposalOutcomeV1,
    };
    use handshake_core::kernel::KernelEventType;
    use handshake_core::storage::knowledge_crdt::list_denial_receipts_for_scope;
    use serde_json::json;
    use uuid::Uuid;

    /// Approved proposal -> EventLedger promotion pair -> authority fact row;
    /// re-promotion is idempotent; invalid promotions leave durable denial
    /// receipts + PROMOTION_REJECTED.
    #[tokio::test]
    async fn approved_proposals_promote_idempotently_and_invalid_ones_deny_durably() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "claims-lm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        // Authority-hardening #1: the promoted proposal must cite a LIVE span;
        // the workspace + lease are aligned to that span's workspace so the
        // promotion span-evidence gate (and the #4 lease guard) both pass.
        let (ws, live_span_id) = live_span_fixture(&backend, &format!("mt069-{suffix}")).await;
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt069-{suffix}")).await;

        let mut record_request = GraphMutationProposalRequestV1 {
            workspace_id: ws.clone(),
            mutation_kind: GraphMutationKind::AddEdge,
            mutation_payload: json!({
                "edge_kind": "documents",
                "from_entity": "module:managed_storage",
                "to_entity": "behavior:embedded-cluster-5544"
            }),
            source_span_refs: vec![live_span_id],
            confidence: 0.9,
            actor: model.clone(),
            session_id: format!("sr-mt069-{suffix}"),
            correlation_id: format!("corr-mt069-{suffix}"),
            lease_id: Some(lease_id),
        };
        let approved_proposal =
            match record_graph_proposal(db.as_ref(), &pool, record_request.clone())
                .await
                .expect("record flow")
            {
                RecordGraphProposalOutcomeV1::Recorded(row) => *row,
                other => panic!("expected recorded draft, got {other:?}"),
            };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &approved_proposal.proposal_id,
            true,
            &operator,
            "sr-review",
            "edge verified against source",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        // Promote.
        let outcome = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &approved_proposal.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        let fact = match outcome {
            GraphPromotionOutcomeV1::Promoted(fact) => fact,
            other => panic!("expected promotion, got {other:?}"),
        };
        assert_eq!(fact.proposal_id, approved_proposal.proposal_id);
        assert_eq!(fact.proposed_by, model.canonical());
        assert_eq!(fact.promoted_by, operator.canonical());

        // The EventLedger promotion pair exists and is causation-linked.
        let events = db
            .list_kernel_events_for_aggregate(
                "knowledge_graph_promotion",
                &approved_proposal.proposal_id,
            )
            .await
            .expect("events");
        let requested = events
            .iter()
            .find(|event| event.event_type == KernelEventType::PromotionRequested)
            .expect("PROMOTION_REQUESTED present");
        let accepted = events
            .iter()
            .find(|event| event.event_type == KernelEventType::PromotionAccepted)
            .expect("PROMOTION_ACCEPTED present");
        assert_eq!(fact.promotion_requested_event_id, requested.event_id);
        assert_eq!(fact.promotion_accepted_event_id, accepted.event_id);
        assert_eq!(
            accepted.causation_id.as_deref(),
            Some(requested.event_id.as_str())
        );

        // Proposal row is stamped 'promoted'.
        let stamped = handshake_core::storage::knowledge_crdt::get_graph_proposal(
            &pool,
            &approved_proposal.proposal_id,
        )
        .await
        .expect("get proposal")
        .expect("row exists");
        assert_eq!(stamped.review_state, "promoted");

        // Idempotent re-promotion returns the same fact.
        let replay = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &approved_proposal.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        match replay {
            GraphPromotionOutcomeV1::AlreadyPromoted(existing) => {
                assert_eq!(existing.fact_id, fact.fact_id);
            }
            other => panic!("expected idempotent replay, got {other:?}"),
        }

        // Invalid promotion: a still-'proposed' proposal denies durably.
        record_request.correlation_id = format!("corr-mt069-b-{suffix}");
        let pending = match record_graph_proposal(db.as_ref(), &pool, record_request)
            .await
            .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        let denied = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &pending.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        let denial = match denied {
            GraphPromotionOutcomeV1::Denied(denial) => denial,
            other => panic!("expected denial, got {other:?}"),
        };
        assert!(matches!(
            denial.reason,
            GraphPromotionDenialReasonV1::NotApproved { ref current_state } if current_state == "proposed"
        ));
        let receipts =
            list_denial_receipts_for_scope(&pool, &format!("proposal:{}", pending.proposal_id))
                .await
                .expect("receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].receipt_kind, "graph_promotion_denied");
        assert_eq!(receipts[0].receipt_id, denial.denial_receipt_id);

        // Unknown proposal also denies durably.
        let unknown = promote_graph_proposal(
            db.as_ref(),
            &pool,
            "01ARZ3NDEKTSV4RRFFQ69G5FAX",
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        assert!(matches!(
            unknown,
            GraphPromotionOutcomeV1::Denied(denial)
                if matches!(denial.reason, GraphPromotionDenialReasonV1::UnknownProposal { .. })
        ));

        let denial_receipt_id = denial.denial_receipt_id.clone();
        drop(pool);
        drop(db);
        let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
        let reopened_fact = handshake_core::storage::knowledge_crdt::get_promoted_fact_by_proposal(
            &reopened,
            &approved_proposal.proposal_id,
        )
        .await
        .expect("read promoted fact after reopen")
        .expect("promoted fact survives reopen");
        assert_eq!(reopened_fact.fact_id, fact.fact_id);
        let reopened_proposal = handshake_core::storage::knowledge_crdt::get_graph_proposal(
            &reopened,
            &approved_proposal.proposal_id,
        )
        .await
        .expect("read promoted proposal after reopen")
        .expect("promoted proposal survives reopen");
        assert_eq!(reopened_proposal.review_state, "promoted");
        let reopened_receipts =
            list_denial_receipts_for_scope(&reopened, &format!("proposal:{}", pending.proposal_id))
                .await
                .expect("read promotion denial receipts after reopen");
        assert_eq!(reopened_receipts.len(), 1);
        assert_eq!(reopened_receipts[0].receipt_id, denial_receipt_id);
        let reopened_events = reopened_db
            .list_kernel_events_for_aggregate(
                "knowledge_graph_promotion",
                &approved_proposal.proposal_id,
            )
            .await
            .expect("read promotion events after reopen");
        let reopened_requested = reopened_events
            .iter()
            .find(|event| event.event_type == KernelEventType::PromotionRequested)
            .expect("reopened PROMOTION_REQUESTED event");
        let reopened_accepted = reopened_events
            .iter()
            .find(|event| event.event_type == KernelEventType::PromotionAccepted)
            .expect("reopened PROMOTION_ACCEPTED event");
        assert_eq!(
            reopened_accepted.causation_id.as_deref(),
            Some(reopened_requested.event_id.as_str())
        );
        drop(reopened_db);
        close_reopened_and_remove(reopened, backend).await;
    }
}

mod mt_074_ai_edit_proposals {
    use super::*;
    use handshake_core::kernel::crdt::ai_edit_proposal::{
        decide_ai_edit_proposal, promote_ai_edit_proposal, record_ai_edit_proposal,
        validate_ai_edit_proposal_request, AiEditPromotionDenialReasonV1, AiEditPromotionOutcomeV1,
        AiEditProposalRequestV1, AiEditProposalValidationError, RecordAiEditProposalOutcomeV1,
    };
    use handshake_core::kernel::KernelEventType;
    use handshake_core::storage::knowledge_crdt::list_denial_receipts_for_scope;
    use serde_json::json;
    use uuid::Uuid;

    fn request(
        ws: &str,
        doc: &str,
        crdt_doc: &str,
        actor: &KnowledgeActorIdV1,
        lease_id: Option<String>,
    ) -> AiEditProposalRequestV1 {
        AiEditProposalRequestV1 {
            workspace_id: ws.to_string(),
            document_id: doc.to_string(),
            crdt_document_id: crdt_doc.to_string(),
            base_update_seq: 4,
            base_state_vector: "hsk-sv1:site-aaaa=4".to_string(),
            proposed_diff: json!({
                "diff_kind": "prosemirror_steps_v1",
                "steps": [
                    {"stepType": "replace", "from": 12, "to": 18,
                     "slice": {"content": [{"type": "text", "text": "port 5544"}]}}
                ]
            }),
            source_span_citations: vec![format!("KSP-{:032x}", 0xc1c1u32)],
            actor: actor.clone(),
            session_id: "sr-mt074".to_string(),
            correlation_id: "corr-mt074".to_string(),
            lease_id,
        }
    }

    #[test]
    fn validation_pins_model_actor_lease_and_citations() {
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "edit-lm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");

        // Operator actors cannot file AI edit proposals.
        assert!(
            validate_ai_edit_proposal_request(&request("ws", "doc", "crdt", &operator, None))
                .expect_err("operator proposer must fail")
                .iter()
                .any(|error| matches!(error, AiEditProposalValidationError::ActorNotModel { .. }))
        );

        // Model without a lease fails (MT-041 seed).
        assert!(
            validate_ai_edit_proposal_request(&request("ws", "doc", "crdt", &model, None))
                .expect_err("unleased model must fail")
                .iter()
                .any(|error| matches!(
                    error,
                    AiEditProposalValidationError::ModelActorWithoutLease { .. }
                ))
        );

        // Citations are mandatory.
        let mut uncited = request("ws", "doc", "crdt", &model, Some("lease".to_string()));
        uncited.source_span_citations.clear();
        assert!(validate_ai_edit_proposal_request(&uncited)
            .expect_err("no citations must fail")
            .iter()
            .any(|error| matches!(error, AiEditProposalValidationError::NoCitations)));
    }

    /// Full review flow on the embedded store: proposed -> approved -> promoted with
    /// the EventLedger pair; rejected proposals deny promotion durably.
    #[tokio::test]
    async fn review_flow_promotes_approved_and_denies_rejected_durably() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt074-{suffix}");
        let doc = format!("doc-mt074-{suffix}");
        let crdt_doc = format!("crdt-mt074-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "edit-cm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt074-{suffix}")).await;

        // Record.
        let proposal = match record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            request(&ws, &doc, &crdt_doc, &model, Some(lease_id.clone())),
        )
        .await
        .expect("record flow")
        {
            RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        assert_eq!(proposal.review_state, "proposed");
        assert_eq!(proposal.actor_id, model.canonical());
        assert_eq!(proposal.diff_sha256.len(), 64);

        let events = db
            .list_kernel_events_for_aggregate("knowledge_ai_edit_proposal", &proposal.proposal_id)
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::AiEditProposalRecorded));

        // Promoting a pending proposal is denied durably.
        let premature = promote_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        let premature_denial = match premature {
            AiEditPromotionOutcomeV1::Denied(denial) => denial,
            other => panic!("expected denial, got {other:?}"),
        };
        assert!(matches!(
            premature_denial.reason,
            AiEditPromotionDenialReasonV1::NotApproved { ref current_state } if current_state == "proposed"
        ));

        // Operator approves; decision event lands.
        let approved = decide_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            "sr-review",
            "diff verified against citations",
        )
        .await
        .expect("decide flow")
        .expect("approved");
        assert_eq!(approved.review_state, "approved");
        let events = db
            .list_kernel_events_for_aggregate("knowledge_ai_edit_proposal", &proposal.proposal_id)
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::AiEditProposalDecided));

        // Promotion lands the pair and stamps the row.
        let promoted = promote_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        let promoted_row = match promoted {
            AiEditPromotionOutcomeV1::Promoted(row) => row,
            other => panic!("expected promotion, got {other:?}"),
        };
        assert_eq!(promoted_row.review_state, "promoted");
        assert!(promoted_row.promotion_requested_event_id.is_some());
        assert!(promoted_row.promotion_accepted_event_id.is_some());

        // Idempotent replay.
        let replay = promote_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        assert!(matches!(
            replay,
            AiEditPromotionOutcomeV1::AlreadyPromoted(_)
        ));

        // Rejection path: a second proposal is rejected; its promotion is
        // denied with a durable receipt.
        let rejected = match record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            request(&ws, &doc, &crdt_doc, &model, Some(lease_id)),
        )
        .await
        .expect("record flow")
        {
            RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &rejected.proposal_id,
            false,
            &operator,
            "sr-review",
            "diff contradicts cited spans",
        )
        .await
        .expect("decide flow")
        .expect("rejected");

        let denied = promote_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &rejected.proposal_id,
            &operator,
            "sr-gate",
            "corr-gate",
        )
        .await
        .expect("promotion flow");
        let denial = match denied {
            AiEditPromotionOutcomeV1::Denied(denial) => denial,
            other => panic!("expected denial, got {other:?}"),
        };
        let receipts =
            list_denial_receipts_for_scope(&pool, &format!("proposal:{}", rejected.proposal_id))
                .await
                .expect("receipts");
        assert!(receipts
            .iter()
            .any(|receipt| receipt.receipt_id == denial.denial_receipt_id
                && receipt.receipt_kind == "ai_edit_promotion_denied"));

        // The schema-negative reviewer mutation is covered by the active
        // typed reviewer proof below.
    }

    #[tokio::test]
    async fn model_reviewer_is_rejected_by_embedded_boundary() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-model-review-negative-{suffix}");
        let doc = format!("doc-model-review-negative-{suffix}");
        let crdt_doc = format!("crdt-model-review-negative-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "review-negative-model")
                .expect("model actor");
        let lease_id = model_lease(
            &backend,
            &model,
            &ws,
            &format!("sr-model-review-negative-{suffix}"),
        )
        .await;

        let proposal = match record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            request(&ws, &doc, &crdt_doc, &model, Some(lease_id)),
        )
        .await
        .expect("record model proposal")
        {
            RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded proposal, got {other:?}"),
        };

        let decision = decide_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &model,
            "sr-model-review-negative",
            "model self-review attempt",
        )
        .await
        .expect("model reviewer decision flow");
        assert!(matches!(
            decision,
            Err(
                handshake_core::kernel::crdt::ai_edit_proposal::AiEditDecisionError::ReviewerNotAllowed {
                    ..
                }
            )
        ));

        // Reviewer rejection is a pre-mutation guard: the proposal remains
        // proposed and no decision event is appended.
        let stored = handshake_core::storage::knowledge_crdt::get_ai_edit_proposal(
            &pool,
            &proposal.proposal_id,
        )
        .await
        .expect("read model-review proposal")
        .expect("proposal remains durable");
        assert_eq!(stored.review_state, "proposed");
        let events = db
            .list_kernel_events_for_aggregate("knowledge_ai_edit_proposal", &proposal.proposal_id)
            .await
            .expect("read model-review events");
        assert_eq!(
            events.len(),
            1,
            "rejected reviewer must not append a decision event"
        );
        assert_eq!(
            events[0].event_type,
            KernelEventType::AiEditProposalRecorded
        );
    }
}

/// Authority-hardening #4: every draft/proposal write that presents a lease
/// is routed through the server-side lease write-guard; an expired / foreign /
/// wrong-scope lease is DENIED with a durable receipt and writes no draft.
mod hardening_lease_chokepoint {
    use super::*;
    use base64::Engine as _;
    use handshake_core::kernel::crdt::actor_site::derive_knowledge_site_id;
    use handshake_core::kernel::crdt::agent_lease::{release_lease, LeaseWriteDenialReasonV1};
    use handshake_core::kernel::crdt::ai_edit_proposal::{
        record_ai_edit_proposal, AiEditProposalRequestV1, RecordAiEditProposalOutcomeV1,
    };
    use handshake_core::kernel::crdt::graph_proposal::{
        record_graph_proposal, GraphMutationKind, GraphMutationProposalRequestV1,
        RecordGraphProposalOutcomeV1,
    };
    use handshake_core::kernel::crdt::save_semantics::{
        save_rich_document_draft_under_lease, KnowledgeDraftSaveOutcomeV1,
    };
    use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
    use handshake_core::kernel::crdt::yjs_bridge::{
        YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
    };
    use handshake_core::storage::knowledge_crdt::{get_lease, list_denial_receipts_for_document};
    use serde_json::json;
    use uuid::Uuid;

    async fn wait_for_db_expiry(
        pool: &handshake_core::storage::surreal::SurrealStorage,
        lease_id: &str,
    ) {
        for _ in 0..40 {
            if get_lease(pool, lease_id)
                .await
                .expect("get lease")
                .expect("lease")
                .is_expired
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        panic!("lease {lease_id} did not expire within 10s");
    }

    async fn claim_ws_lease(
        backend: &EmbeddedTestBackend,
        actor: &KnowledgeActorIdV1,
        ws: &str,
        session: &str,
        ttl: i64,
    ) -> String {
        match claim_lease(
            backend.database.as_ref(),
            &backend.storage,
            LeaseClaimRequestV1 {
                lane_id: format!("lane-{session}"),
                actor: actor.clone(),
                session_id: session.to_string(),
                correlation_id: format!("corr-{session}"),
                scope_kind: KnowledgeLeaseScopeKind::Workspace,
                scope_id: ws.to_string(),
                ttl_seconds: ttl,
            },
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease.lease_id.clone(),
            other => panic!("expected claim, got {other:?}"),
        }
    }

    /// An EXPIRED lease on a graph proposal write is denied (not presence-only).
    #[tokio::test]
    async fn expired_lease_denies_graph_proposal_write() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-choke-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "choke-lm").expect("actor");
        let lease_id = claim_ws_lease(&backend, &model, &ws, &format!("sr-{suffix}"), 1).await;
        wait_for_db_expiry(&pool, &lease_id).await;

        let outcome = record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({"claim_text": "written under a dead lease"}),
                source_span_refs: vec![format!("KSP-{}", "0".repeat(32))],
                confidence: 0.5,
                actor: model.clone(),
                session_id: format!("sr-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                lease_id: Some(lease_id.clone()),
            },
        )
        .await
        .expect("record flow");
        match outcome {
            RecordGraphProposalOutcomeV1::LeaseDenied(denial) => {
                assert!(matches!(
                    denial.reason,
                    LeaseWriteDenialReasonV1::LeaseExpired { .. }
                ));
            }
            other => panic!("expired lease must deny the write, got {other:?}"),
        }
        // No draft row landed for this workspace.
        let inspector = backend.storage.test_inspector();
        let proposals = inspector
            .table_selector("knowledge_crdt_graph_proposals")
            .await
            .expect("select graph proposals table");
        let count = inspector
            .row_count(
                &proposals,
                handshake_core::storage::surreal::RowFilter::FieldEquals {
                    field: proposals
                        .field("workspace_id")
                        .expect("select workspace field"),
                    value: ws.as_str().into(),
                },
            )
            .await
            .expect("count");
        assert_eq!(count, 0, "no draft may be written under a dead lease");
    }

    /// A FOREIGN lease (held by another actor) on an AI edit write is denied.
    #[tokio::test]
    async fn foreign_lease_denies_ai_edit_write() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-choke-ai-{suffix}");
        let owner =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "owner-lm").expect("actor");
        let other =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "other-cm").expect("actor");
        // `owner` holds the workspace lease; `other` tries to write under it.
        let lease_id =
            claim_ws_lease(&backend, &owner, &ws, &format!("sr-own-{suffix}"), 600).await;

        let outcome = record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            AiEditProposalRequestV1 {
                workspace_id: ws.clone(),
                document_id: format!("doc-{suffix}"),
                crdt_document_id: format!("crdt-{suffix}"),
                base_update_seq: 0,
                base_state_vector: "hsk-sv1:".to_string(),
                proposed_diff: json!({"steps": []}),
                source_span_citations: vec![format!("KSP-{}", "0".repeat(32))],
                actor: other.clone(),
                session_id: format!("sr-other-{suffix}"),
                correlation_id: format!("corr-other-{suffix}"),
                lease_id: Some(lease_id.clone()),
            },
        )
        .await
        .expect("record flow");
        match outcome {
            RecordAiEditProposalOutcomeV1::LeaseDenied(denial) => {
                assert!(matches!(
                    denial.reason,
                    LeaseWriteDenialReasonV1::ForeignLease { .. }
                ));
            }
            other => panic!("foreign lease must deny the write, got {other:?}"),
        }
    }

    /// A WRONG-SCOPE lease on a guarded save is denied with a durable receipt.
    #[tokio::test]
    async fn wrong_scope_lease_denies_guarded_save() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-choke-save-{suffix}");
        let doc = format!("doc-{suffix}");
        let crdt_doc = format!("crdt-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "save-op").expect("actor");
        // A workspace lease on a DIFFERENT workspace -> wrong scope for this
        // document save (the save guard checks document scope).
        let lease_id = claim_ws_lease(
            &backend,
            &operator,
            "ws-unrelated",
            &format!("sr-{suffix}"),
            600,
        )
        .await;

        let site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.clone();
        sv.increment(&site.site_id);
        let bytes = b"guarded-save";
        let env = YjsUpdateEnvelopeV1 {
            schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
            workspace_id: ws.clone(),
            document_id: doc.clone(),
            crdt_document_id: crdt_doc.clone(),
            update_id: format!("u-{suffix}"),
            actor_id: operator.canonical(),
            site_id: site.site_id.clone(),
            session_id: format!("sr-{suffix}"),
            trace_id: format!("trace-{suffix}"),
            document_schema_id: "hsk.doc.rich_document@1".to_string(),
            update_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
            update_sha256: handshake_core::kernel::crdt::persistence::sha256_hex(bytes),
            state_vector_before: before.encode(),
            state_vector_after: sv.encode(),
            encoding: YJS_UPDATE_ENCODING_V1.to_string(),
        };
        let outcome = save_rich_document_draft_under_lease(db.as_ref(), &pool, &env, &lease_id)
            .await
            .expect("save flow");
        match outcome {
            KnowledgeDraftSaveOutcomeV1::LeaseDenied { denial } => {
                assert!(matches!(
                    denial.reason,
                    LeaseWriteDenialReasonV1::ScopeMismatch { .. }
                ));
            }
            other => panic!("wrong-scope lease must deny the save, got {other:?}"),
        }
        // The denial is durable and the draft did not land.
        let receipts = list_denial_receipts_for_document(&pool, &crdt_doc)
            .await
            .expect("receipts");
        assert!(receipts
            .iter()
            .any(|r| r.receipt_kind == "lease_write_denied"));
        let inspector = backend.storage.test_inspector();
        let updates = inspector
            .table_selector("kernel_crdt_updates")
            .await
            .expect("select CRDT updates table");
        let count = inspector
            .row_count(
                &updates,
                handshake_core::storage::surreal::RowFilter::FieldEquals {
                    field: updates
                        .field("crdt_document_id")
                        .expect("select CRDT document field"),
                    value: crdt_doc.as_str().into(),
                },
            )
            .await
            .expect("count");
        assert_eq!(count, 0, "no update may land under a wrong-scope lease");

        // Cleanup the unrelated lease so the scope is reusable.
        release_lease(db.as_ref(), &pool, &lease_id, &operator)
            .await
            .expect("release");
    }
}

/// Authority-hardening #5: an approved AI edit's applied update is bound to the
/// approved diff_sha256; a push whose content does not hash to the approved
/// diff is rejected.
mod hardening_applied_binding {
    use super::*;
    use handshake_core::kernel::crdt::actor_site::{
        derive_knowledge_site_id, knowledge_crdt_identity,
    };
    use handshake_core::kernel::crdt::ai_edit_proposal::{
        apply_approved_ai_edit, decide_ai_edit_proposal, record_ai_edit_proposal,
        AiEditApplyOutcomeV1, AiEditProposalRequestV1, RecordAiEditProposalOutcomeV1,
    };
    use handshake_core::kernel::crdt::persistence::{
        new_crdt_update_record, CrdtReplayMetadataV1, CrdtUpdateRecordInputV1,
    };
    use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
    use handshake_core::kernel::{KernelEventType, NewKernelEvent};
    use handshake_core::storage::knowledge_crdt::{
        get_ai_edit_proposal, list_denial_receipts_for_scope,
    };
    use serde_json::json;
    use uuid::Uuid;

    /// MT-074 V1 FAIL remediation helper: push a REAL `kernel_crdt_updates` row
    /// whose persisted `update_sha256` is the canonical hash of `applied_diff`,
    /// so an applied-binding for `update_id` can anchor to a genuine document
    /// update. Mirrors the MT-067 push path (event ledger row + update row).
    async fn insert_real_crdt_update(
        backend: &EmbeddedTestBackend,
        ws: &str,
        doc: &str,
        crdt_doc: &str,
        actor: &KnowledgeActorIdV1,
        update_id: &str,
        update_seq: u64,
        applied_diff: &serde_json::Value,
        suffix: &str,
    ) {
        let db = backend.database.clone();
        let identity = knowledge_crdt_identity(
            ws,
            doc,
            crdt_doc,
            "hsk.doc.rich_document@1",
            actor,
            &format!("trace-applied-{suffix}"),
        );
        let site = derive_knowledge_site_id(ws, crdt_doc, actor);
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.encode();
        for _ in 0..update_seq {
            sv.increment(&site.site_id);
        }
        let after = sv.encode();
        let event = NewKernelEvent::builder(
            format!("KTR-APPLIED-{suffix}"),
            format!("SR-APPLIED-{suffix}"),
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            actor.to_kernel_actor(),
        )
        .aggregate("knowledge_crdt_document", crdt_doc.to_string())
        .idempotency_key(format!("applied:{suffix}:{update_id}"))
        .source_component("knowledge_crdt_proposal_tests")
        .payload(json!({ "update_id": update_id, "after": after }))
        .build()
        .expect("valid event");
        let stored_event = db.append_kernel_event(event).await.expect("append event");
        // The persisted update_sha256 MUST be the canonical hash of the diff
        // the binder will present, so use the serde_json bytes of applied_diff.
        let bytes = serde_json::to_vec(applied_diff).expect("serialize applied diff");
        let record = new_crdt_update_record(CrdtUpdateRecordInputV1 {
            identity: &identity,
            update_id,
            update_seq,
            update_bytes: &bytes,
            update_bytes_ref: &format!(
                "surreal://kernel_crdt_updates/{crdt_doc}/{update_id}/update_bytes"
            ),
            session_id: &format!("SR-APPLIED-{suffix}"),
            trace_id: &format!("trace-applied-{suffix}"),
            state_vector_before: &before,
            state_vector_after: &after,
            replay_metadata: CrdtReplayMetadataV1 {
                replay_order_key: format!("{ws}/{doc}/{update_seq:020}"),
                dependency_update_ids: Vec::new(),
                encoding: "yjs-update-v1".to_string(),
                schema_version: "kernel-crdt-update-v1".to_string(),
            },
            event_ledger_event_id: &stored_event.event_id,
        });
        db.append_kernel_crdt_update(record, bytes)
            .await
            .expect("append real crdt update");
    }

    #[tokio::test]
    async fn applied_update_must_hash_to_approved_diff() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-applied-{suffix}");
        let doc = format!("doc-{suffix}");
        let crdt_doc = format!("crdt-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "apply-cm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-{suffix}")).await;

        let approved_diff = json!({"steps": [{"insert": "approved text"}]});
        let proposal = match record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            AiEditProposalRequestV1 {
                workspace_id: ws.clone(),
                document_id: doc.clone(),
                crdt_document_id: crdt_doc.clone(),
                base_update_seq: 0,
                base_state_vector: "hsk-sv1:".to_string(),
                proposed_diff: approved_diff.clone(),
                source_span_citations: vec![format!("KSP-{}", "0".repeat(32))],
                actor: model.clone(),
                session_id: format!("sr-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                lease_id: Some(lease_id),
            },
        )
        .await
        .expect("record flow")
        {
            RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            &format!("sr-rev-{suffix}"),
            "approved",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        // A push NOT matching the approved diff is rejected with a durable
        // mismatch receipt; the binding is refused. A REAL kernel_crdt_updates
        // row is pushed carrying the tampered content, so the flow gets past the
        // update-row existence gate and is caught by the approved-diff hash
        // check (proving the two denial paths are distinct, not collapsed).
        let tampered_diff = json!({"steps": [{"insert": "TAMPERED text"}]});
        insert_real_crdt_update(
            &backend,
            &ws,
            &doc,
            &crdt_doc,
            &model,
            &format!("update-bad-{suffix}"),
            1,
            &tampered_diff,
            &suffix,
        )
        .await;
        let mismatch = apply_approved_ai_edit(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &format!("update-bad-{suffix}"),
            &tampered_diff,
            &model,
            &format!("sr-apply-{suffix}"),
            &format!("corr-apply-{suffix}"),
        )
        .await
        .expect("apply flow");
        match mismatch {
            AiEditApplyOutcomeV1::HashMismatch { .. } => {}
            other => panic!("non-matching applied update must be rejected, got {other:?}"),
        }
        let receipts =
            list_denial_receipts_for_scope(&pool, &format!("proposal:{}", proposal.proposal_id))
                .await
                .expect("receipts");
        assert!(
            receipts
                .iter()
                .any(|r| r.receipt_kind == "ai_edit_applied_mismatch"),
            "a durable ai_edit_applied_mismatch receipt must exist"
        );
        // The row carries NO applied binding after a mismatch.
        let row = get_ai_edit_proposal(&pool, &proposal.proposal_id)
            .await
            .expect("get")
            .expect("row");
        assert!(row.applied_update_id.is_none());
        assert!(row.applied_update_sha256.is_none());

        // The matching applied update binds successfully — but ONLY after a
        // real kernel_crdt_updates row carrying the approved content exists.
        insert_real_crdt_update(
            &backend,
            &ws,
            &doc,
            &crdt_doc,
            &model,
            &format!("update-good-{suffix}"),
            2,
            &approved_diff,
            &suffix,
        )
        .await;
        let bound = apply_approved_ai_edit(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &format!("update-good-{suffix}"),
            &approved_diff,
            &model,
            &format!("sr-apply-{suffix}"),
            &format!("corr-apply2-{suffix}"),
        )
        .await
        .expect("apply flow");
        match bound {
            AiEditApplyOutcomeV1::Bound(row) => {
                assert_eq!(
                    row.applied_update_id.as_deref(),
                    Some(format!("update-good-{suffix}").as_str())
                );
                assert_eq!(
                    row.applied_update_sha256.as_deref(),
                    Some(row.diff_sha256.as_str())
                );
            }
            other => panic!("matching applied update must bind, got {other:?}"),
        }
    }

    /// MT-074 V1 FAIL remediation (the exact negative test the validator asked
    /// for): the approved diff hash matches the content presented for binding,
    /// but NO kernel_crdt_updates row exists for the cited update id. The
    /// binding MUST be refused and a durable denial receipt MUST be emitted —
    /// the hash match alone may not bind an approved edit to a phantom update.
    #[tokio::test]
    async fn approved_hash_matches_but_update_id_absent_refuses_binding() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-absent-{suffix}");
        let doc = format!("doc-{suffix}");
        let crdt_doc = format!("crdt-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "absent-cm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-{suffix}")).await;

        let approved_diff = json!({"steps": [{"insert": "approved text"}]});
        let proposal = match record_ai_edit_proposal(
            db.as_ref(),
            &pool,
            AiEditProposalRequestV1 {
                workspace_id: ws.clone(),
                document_id: doc.clone(),
                crdt_document_id: crdt_doc.clone(),
                base_update_seq: 0,
                base_state_vector: "hsk-sv1:".to_string(),
                proposed_diff: approved_diff.clone(),
                source_span_citations: vec![format!("KSP-{}", "0".repeat(32))],
                actor: model.clone(),
                session_id: format!("sr-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                lease_id: Some(lease_id),
            },
        )
        .await
        .expect("record flow")
        {
            RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_ai_edit_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            &format!("sr-rev-{suffix}"),
            "approved",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        // Present the EXACT approved diff (hash matches diff_sha256), but cite
        // an update id that was never pushed — there is no kernel_crdt_updates
        // row for it. The binding must be refused with a durable receipt.
        let absent_update_id = format!("update-never-pushed-{suffix}");
        let outcome = apply_approved_ai_edit(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &absent_update_id,
            &approved_diff,
            &model,
            &format!("sr-apply-{suffix}"),
            &format!("corr-apply-{suffix}"),
        )
        .await
        .expect("apply flow");
        match outcome {
            AiEditApplyOutcomeV1::UpdateRowMissing {
                applied_update_id,
                stored_update_sha256,
                denial_receipt_id,
                ..
            } => {
                assert_eq!(applied_update_id, absent_update_id);
                assert!(
                    stored_update_sha256.is_none(),
                    "no kernel_crdt_updates row should exist for the absent update id"
                );
                assert!(!denial_receipt_id.is_empty());
            }
            other => {
                panic!("approved-hash-but-absent-update must refuse the binding, got {other:?}")
            }
        }

        // The durable ai_edit_applied_update_missing receipt is present.
        let receipts =
            list_denial_receipts_for_scope(&pool, &format!("proposal:{}", proposal.proposal_id))
                .await
                .expect("receipts");
        assert!(
            receipts
                .iter()
                .any(|r| r.receipt_kind == "ai_edit_applied_update_missing"),
            "a durable ai_edit_applied_update_missing receipt must exist"
        );

        // The proposal row carries NO applied binding after the refusal.
        let row = get_ai_edit_proposal(&pool, &proposal.proposal_id)
            .await
            .expect("get")
            .expect("row");
        assert!(row.applied_update_id.is_none());
        assert!(row.applied_update_sha256.is_none());

        // Control: once the real update is pushed, the same diff binds. This
        // proves the refusal was about the missing row, not the content.
        insert_real_crdt_update(
            &backend,
            &ws,
            &doc,
            &crdt_doc,
            &model,
            &absent_update_id,
            1,
            &approved_diff,
            &suffix,
        )
        .await;
        let bound = apply_approved_ai_edit(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &absent_update_id,
            &approved_diff,
            &model,
            &format!("sr-apply-{suffix}"),
            &format!("corr-apply2-{suffix}"),
        )
        .await
        .expect("apply flow");
        assert!(
            matches!(bound, AiEditApplyOutcomeV1::Bound(_)),
            "with a real update row present the same diff must bind, got {bound:?}"
        );
    }
}
