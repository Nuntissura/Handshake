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
use uuid::Uuid;

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

/// Authority-hardening #1 fixture: create a real workspace + source + span and
/// return `(workspace_id, span_id)` so a proposal in `workspace_id` citing
/// `span_id` has promotable (live, same-workspace, non-stale) evidence.
async fn live_span_fixture(pool: &sqlx::PgPool, label: &str) -> (String, String) {
    let workspace_id = format!("ws-span-{label}");
    let source_id = format!("KSRC-{}", Uuid::now_v7().simple());
    let span_id = format!("KSP-{}", Uuid::now_v7().simple());
    let hash = "a".repeat(64);
    sqlx::query("INSERT INTO workspaces (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
        .bind(&workspace_id)
        .bind(format!("span-fixture-{label}"))
        .execute(pool)
        .await
        .expect("insert workspace");
    sqlx::query(
        r#"INSERT INTO knowledge_sources
           (source_id, workspace_id, source_kind, content_hash)
           VALUES ($1, $2, 'external_import', $3)"#,
    )
    .bind(&source_id)
    .bind(&workspace_id)
    .bind(&hash)
    .execute(pool)
    .await
    .expect("insert source");
    sqlx::query(
        r#"INSERT INTO knowledge_spans
           (span_id, source_id, span_kind, range_start, range_end,
            content_sha256, parser_version)
           VALUES ($1, $2, 'byte', 0, 16, $3, 'v1')"#,
    )
    .bind(&span_id)
    .bind(&source_id)
    .bind(&hash)
    .execute(pool)
    .await
    .expect("insert span");
    (workspace_id, span_id)
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
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "claims-lm").expect("actor");
        let operator = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op").expect("actor");
        // Authority-hardening #1: the promoted proposal must cite a LIVE span;
        // the workspace + lease are aligned to that span's workspace so the
        // promotion span-evidence gate (and the #4 lease guard) both pass.
        let (ws, live_span_id) = live_span_fixture(&pool, &format!("mt069-{suffix}")).await;
        let lease_id = model_lease(&backend, &model, &ws, &format!("sr-mt069-{suffix}")).await;

        let mut record_request = GraphMutationProposalRequestV1 {
            workspace_id: ws.clone(),
            mutation_kind: GraphMutationKind::AddEdge,
            mutation_payload: json!({
                "edge_kind": "documents",
                "from_entity": "module:managed_postgres",
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

    /// Full review flow on PostgreSQL: proposed -> approved -> promoted with
    /// the EventLedger pair; rejected proposals deny promotion durably.
    #[tokio::test]
    async fn review_flow_promotes_approved_and_denies_rejected_durably() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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

        // The DB CHECK keeps models out of the reviewer column.
        let rogue_review = sqlx::query(
            r#"
            UPDATE knowledge_crdt_ai_edit_proposals
            SET review_state = 'approved', decided_by = 'local_model:rogue',
                decided_at_utc = NOW(), decision_reason = 'self-approve',
                decided_event_id = $2
            WHERE proposal_id = $1
            "#,
        )
        .bind(&rejected.proposal_id)
        .bind(&rejected.recorded_event_id)
        .execute(&pool)
        .await;
        assert!(
            rogue_review.is_err(),
            "model reviewer must be refused by CHECK"
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

    async fn wait_for_db_expiry(pool: &sqlx::PgPool, lease_id: &str) {
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
        backend: &PostgresTestBackend,
        actor: &KnowledgeActorIdV1,
        ws: &str,
        session: &str,
        ttl: i64,
    ) -> String {
        match claim_lease(
            backend.database.as_ref(),
            &backend.postgres_pool,
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
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_crdt_graph_proposals WHERE workspace_id = $1",
        )
        .bind(&ws)
        .fetch_one(&pool)
        .await
        .expect("count");
        assert_eq!(count, 0, "no draft may be written under a dead lease");
    }

    /// A FOREIGN lease (held by another actor) on an AI edit write is denied.
    #[tokio::test]
    async fn foreign_lease_denies_ai_edit_write() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_crdt_updates WHERE crdt_document_id = $1",
        )
        .bind(&crdt_doc)
        .fetch_one(&pool)
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
    use handshake_core::kernel::crdt::ai_edit_proposal::{
        apply_approved_ai_edit, decide_ai_edit_proposal, record_ai_edit_proposal,
        AiEditApplyOutcomeV1, AiEditProposalRequestV1, RecordAiEditProposalOutcomeV1,
    };
    use handshake_core::storage::knowledge_crdt::{
        get_ai_edit_proposal, list_denial_receipts_for_scope,
    };
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn applied_update_must_hash_to_approved_diff() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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
        // mismatch receipt; the binding is refused.
        let tampered_diff = json!({"steps": [{"insert": "TAMPERED text"}]});
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

        // The matching applied update binds successfully.
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
}
