//! WP-KERNEL-009 CRDTAndConcurrencyCore proposal tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_068_graph_proposals: MT-068 GraphMutationProposalModel
//!   - mt_069_claim_promotion: MT-069 ClaimPromotionBridge
//!   - mt_074_ai_edit_proposals: MT-074 AiEditProposalReviewFlow
//!
//! All durable assertions run against real PostgreSQL (POSTGRES_TEST_URL,
//! isolated schema, migrations 0150-0155).

use handshake_core::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1,
};
use handshake_core::storage::tests::{postgres_backend_with_pool_from_env, PostgresTestBackend};
use handshake_core::storage::StorageError;

async fn backend_or_blocked() -> PostgresTestBackend {
    match postgres_backend_with_pool_from_env().await {
        Ok(backend) => backend,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: WP-009 CRDT proposal tests require POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

/// Claim a workspace-scope lane lease for a model actor (proposals from
/// model actors require one, MT-041 seed).
async fn model_lease(
    backend: &PostgresTestBackend,
    actor: &KnowledgeActorIdV1,
    workspace_id: &str,
    session_id: &str,
) -> String {
    let outcome = claim_lease(
        backend.database.as_ref(),
        &backend.postgres_pool,
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
        ProposalDecisionError, ProposalReviewState,
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
                "claim_text": "managed_postgres.rs starts the embedded cluster on port 5544",
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

    /// PostgreSQL proof: proposal recorded with event receipt; reviewed by a
    /// validator; double decisions and model reviewers are refused; DB CHECK
    /// constraints fail closed on direct unevidenced inserts.
    #[tokio::test]
    async fn proposal_lifecycle_with_event_receipts_on_postgres() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt068-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "graph-lm").expect("actor");
        let validator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Validator, "wp-val").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt068-{suffix}")).await;

        let recorded = record_graph_proposal(
            db.as_ref(),
            &pool,
            request(&ws, &model, Some(lease_id.clone())),
        )
        .await
        .expect("record flow")
        .expect("valid proposal accepted");
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

        // DB CHECK fails closed: direct insert with EMPTY span evidence.
        let direct = sqlx::query(
            r#"
            INSERT INTO knowledge_crdt_graph_proposals (
                proposal_id, workspace_id, mutation_kind, mutation_payload,
                source_span_refs, confidence, actor_id, actor_kind,
                session_id, correlation_id, recorded_event_id
            )
            VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAV', $1, 'add_claim', '{}'::jsonb,
                    '[]'::jsonb, 0.5, 'local_model:rogue', 'local_model',
                    'sr-rogue', 'corr-rogue', $2)
            "#,
        )
        .bind(&ws)
        .bind(&recorded.recorded_event_id)
        .execute(&pool)
        .await;
        assert!(
            direct.is_err(),
            "unevidenced direct insert must be refused by CHECK"
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
        GraphMutationProposalRequestV1,
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
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt069-{suffix}");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "claims-lm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt069-{suffix}")).await;

        let mut record_request = GraphMutationProposalRequestV1 {
            workspace_id: ws.clone(),
            mutation_kind: GraphMutationKind::AddEdge,
            mutation_payload: json!({
                "edge_kind": "documents",
                "from_entity": "module:managed_postgres",
                "to_entity": "behavior:embedded-cluster-5544"
            }),
            source_span_refs: vec![format!("KSP-{:032x}", 0xabcdu32)],
            confidence: 0.9,
            actor: model.clone(),
            session_id: format!("sr-mt069-{suffix}"),
            correlation_id: format!("corr-mt069-{suffix}"),
            lease_id: Some(lease_id),
        };
        let approved_proposal = record_graph_proposal(db.as_ref(), &pool, record_request.clone())
            .await
            .expect("record flow")
            .expect("valid proposal");
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
        let pending = record_graph_proposal(db.as_ref(), &pool, record_request)
            .await
            .expect("record flow")
            .expect("valid proposal");
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
    }
}
