//! WP-1 MT-004: Dexterity promotion/routing runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! model-lane outputs remain advisory until Dexterity records an explicit,
//! replayable promotion decision with CRDT/schema/version guards.

mod knowledge_pg_support;
mod model_lane_cloud_support;

use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneError, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneNavigationLookup, ModelLanePromotionDenialReason,
    ModelLanePromotionOutcome, ModelLanePromotionState, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessContext, ResourceScope, WorkspaceScopeRef,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

    assert_eq!(
        ModelLaneRoutingPolicy::all()
            .iter()
            .map(ModelLaneRoutingPolicy::as_str)
            .collect::<Vec<_>>(),
        vec![
            "local_first",
            "cloud_review",
            "cloud_plan_local_execute",
            "parallel_debate",
            "validator_lane",
            "operator_lane",
        ],
        "Dexterity must expose every MT-004 routing policy as typed Rust data"
    );

    let decision = sample_decision(
        "decision-approved-001",
        "idem-promotion-approved-001",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    let stored = store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record approved promotion decision");

    assert_eq!(stored.outcome, ModelLanePromotionOutcome::Approved);
    assert_eq!(stored.final_state, ModelLanePromotionState::Executed);
    assert_eq!(
        stored.state_history,
        vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::PendingApproval,
            ModelLanePromotionState::Approved,
            ModelLanePromotionState::Executing,
            ModelLanePromotionState::Executed,
        ],
        "promotion must be a deterministic state machine, not a boolean"
    );
    assert_eq!(
        stored.canonical_input_refs,
        vec![
            "model-lane-message://msg-critique-001".to_string(),
            "model-lane-message://msg-proposal-001".to_string(),
        ]
    );
    assert_eq!(
        stored.selected_input_refs,
        vec!["model-lane-message://msg-proposal-001".to_string()]
    );
    assert_eq!(
        stored.rejected_input_refs,
        vec!["model-lane-message://msg-critique-001".to_string()]
    );
    assert_eq!(
        stored.validator_authority_ref.as_deref(),
        Some("validator://mt004/v1")
    );
    assert_eq!(
        stored.operator_authority_ref.as_deref(),
        Some("operator://mt004/approve")
    );
    assert_eq!(stored.promotion_gate_ref, "promotion-gate://mt004/approved");
    assert_eq!(
        stored.promotion_receipt_ref.as_deref(),
        Some("promotion-receipt://mt004/approved")
    );
    assert_eq!(stored.denial_reason, None);
    assert!(stored.event_ledger_event_id.starts_with("KE-"));
    assert!(
        stored.event_ledger_seq > seeded.proposal_event_seq,
        "promotion decision must append after the selected advisory message"
    );
    assert_eq!(
        stored.canonical_hash_basis["input_refs"],
        json!([
            "model-lane-message://msg-critique-001",
            "model-lane-message://msg-proposal-001"
        ])
    );
    assert_eq!(
        stored.canonical_hash_basis["selected_input_refs"],
        json!(["model-lane-message://msg-proposal-001"])
    );
    assert_eq!(
        stored.canonical_hash_basis["promoted_artifact"]["ref"],
        json!("artifact://mt004/promoted/msg-promoted-001")
    );

    let ledger_row: (String, String, String) = sqlx::query_as(
        "SELECT event_type, aggregate_type, aggregate_id \
         FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&stored.event_ledger_event_id)
    .fetch_one(&pool)
    .await
    .expect("promotion decision EventLedger row");
    assert_eq!(ledger_row.0, "PROMOTION_ACCEPTED");
    assert_eq!(ledger_row.1, "model_lane_promotion_decision");
    assert_eq!(ledger_row.2, "decision-approved-001");

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("replay promotion decisions");
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].decision_id, stored.decision_id);
    assert_eq!(
        replay[0].canonical_decision_hash,
        stored.canonical_decision_hash
    );

    let duplicate = store
        .record_promotion_decision(decision)
        .await
        .expect("same idempotency and same content replays existing decision");
    assert_eq!(
        duplicate.event_ledger_event_id,
        stored.event_ledger_event_id
    );
    assert_eq!(
        duplicate.canonical_decision_hash,
        stored.canonical_decision_hash
    );

    let promoted = store
        .record_message(promoted_message(
            "msg-promoted-001",
            "idem-message-promoted-001",
            "decision-approved-001",
            "promotion-gate://mt004/approved",
            "promotion-receipt://mt004/approved",
        ))
        .await
        .expect("accepted PromotionGate decision allows promoted authority message");
    assert_eq!(promoted.authority, ModelLaneAuthority::Promoted);

    let wrong_artifact_err = store
        .record_message(promoted_message(
            "msg-promoted-wrong-artifact",
            "idem-message-promoted-wrong-artifact",
            "decision-approved-001",
            "promotion-gate://mt004/approved",
            "promotion-receipt://mt004/approved",
        ))
        .await
        .expect_err("approved decision cannot authorize a different promoted artifact");
    assert!(
        wrong_artifact_err.to_string().contains("artifact binding"),
        "promoted message must bind to the exact approved artifact: {wrong_artifact_err}"
    );

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    assert!(
        registry_rows
            .iter()
            .any(|row| row.schema_id == "hsk.model_lane_promotion_decision@1"),
        "promotion decision schema must be registered for state recovery"
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let (_pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

    let mut stale_base = sample_decision(
        "decision-denied-stale-base",
        "idem-promotion-denied-stale-base",
        ModelLaneRoutingPolicy::CloudPlanLocalExecute,
        seeded.proposal_event_seq,
    );
    stale_base.base_snapshot_ref = "crdt-snapshot://mt004/base-stale".into();
    stale_base.current_base_snapshot_ref = "crdt-snapshot://mt004/base-stale".into();
    let stale_record = store
        .record_promotion_decision(stale_base.clone())
        .await
        .expect("stale base denial is durable");
    assert_eq!(stale_record.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(stale_record.final_state, ModelLanePromotionState::Denied);
    assert_eq!(
        stale_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
    assert_eq!(
        stale_record.state_history,
        vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::Denied,
        ]
    );
    assert_eq!(
        stale_record.current_base_snapshot_ref, "not-applicable",
        "nonshared advisory input must not manufacture a current CRDT base from caller input"
    );
    assert!(stale_record
        .recovery_hint_ref
        .as_deref()
        .expect("denial recovery hint")
        .contains("usermanual://model-lane-promotion"));

    let mut duplicate_changed = stale_base;
    duplicate_changed.schema_id = "hsk.model_lane_message@2".into();
    let duplicate_err = store
        .record_promotion_decision(duplicate_changed)
        .await
        .expect_err("same idempotency with changed content must conflict");
    assert!(
        duplicate_err.to_string().contains("idempotency"),
        "duplicate idempotency conflict must be explicit: {duplicate_err}"
    );

    let mut schema_mismatch = sample_decision(
        "decision-denied-schema",
        "idem-promotion-denied-schema",
        ModelLaneRoutingPolicy::CloudReview,
        seeded.proposal_event_seq,
    );
    schema_mismatch.schema_id = "hsk.model_lane_message@2".into();
    let schema_record = store
        .record_promotion_decision(schema_mismatch)
        .await
        .expect("schema mismatch denial is durable");
    assert_eq!(
        schema_record.denial_reason,
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    );
    assert_eq!(
        schema_record.current_schema_id.as_deref(),
        Some("hsk.model_lane_message@1")
    );

    let mut stale_state = sample_decision(
        "decision-denied-stale-state",
        "idem-promotion-denied-stale-state",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    stale_state.state_vector = "sv:mt004:stale".into();
    stale_state.current_state_vector = "sv:mt004:stale".into();
    let stale_state_record = store
        .record_promotion_decision(stale_state)
        .await
        .expect("stale state-vector denial is durable");
    assert_eq!(
        stale_state_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
    assert_eq!(
        stale_state_record.current_state_vector, "not-applicable",
        "nonshared advisory input must not manufacture a current CRDT state vector"
    );

    let mut version_mismatch = sample_decision(
        "decision-denied-version",
        "idem-promotion-denied-version",
        ModelLaneRoutingPolicy::ValidatorLane,
        seeded.proposal_event_seq - 1,
    );
    version_mismatch.expected_event_ledger_version = seeded.proposal_event_seq - 1;
    let version_record = store
        .record_promotion_decision(version_mismatch)
        .await
        .expect("EventLedger version mismatch denial is durable");
    assert_eq!(
        version_record.denial_reason,
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    );
    assert_eq!(
        version_record.current_event_ledger_version,
        Some(seeded.proposal_event_seq)
    );

    let mut direct_mutation = sample_decision(
        "decision-denied-direct-mutation",
        "idem-promotion-denied-direct-mutation",
        ModelLaneRoutingPolicy::OperatorLane,
        seeded.proposal_event_seq,
    );
    direct_mutation.direct_authority_mutation_attempt_ref =
        Some("model-lane-message://msg-direct-promoted".into());
    let direct_record = store
        .record_promotion_decision(direct_mutation)
        .await
        .expect("direct mutation denial is durable");
    assert_eq!(
        direct_record.denial_reason,
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    );

    let mut phantom_ref = sample_decision(
        "decision-denied-phantom-ref",
        "idem-promotion-denied-phantom-ref",
        ModelLaneRoutingPolicy::CloudReview,
        seeded.proposal_event_seq,
    );
    phantom_ref.input_refs = vec![
        "model-lane-message://msg-proposal-001".into(),
        "model-lane-message://msg-phantom-404".into(),
    ];
    phantom_ref.selected_input_refs = vec!["model-lane-message://msg-phantom-404".into()];
    phantom_ref.rejected_input_refs = vec!["model-lane-message://msg-proposal-001".into()];
    phantom_ref.expected_event_ledger_aggregate_id = "msg-phantom-404".into();
    let phantom_record = store
        .record_promotion_decision(phantom_ref)
        .await
        .expect("phantom advisory ref denial is durable");
    assert_eq!(
        phantom_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );

    let direct_message_err = store
        .record_message(promoted_message(
            "msg-direct-promoted",
            "idem-message-direct-promoted",
            "decision-missing",
            "promotion-gate://mt004/missing",
            "promotion-receipt://mt004/missing",
        ))
        .await
        .expect_err("promoted authority message cannot bypass PromotionGate");
    assert!(
        direct_message_err
            .to_string()
            .contains("PromotionGate resolution"),
        "direct authority mutation must fail closed with recovery wording: {direct_message_err}"
    );

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("denial replay");
    assert_eq!(replay.len(), 6);
    assert!(
        replay
            .iter()
            .all(|record| record.outcome == ModelLanePromotionOutcome::Denied),
        "all negative cases must remain durable/replayable denials"
    );
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let (_pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

    let first = store
        .record_promotion_decision(sample_decision(
            "decision-hash-001",
            "idem-promotion-hash-001",
            ModelLaneRoutingPolicy::LocalFirst,
            seeded.proposal_event_seq,
        ))
        .await
        .expect("first decision");

    let mut reordered = sample_decision(
        "decision-hash-002",
        "idem-promotion-hash-002",
        ModelLaneRoutingPolicy::LocalFirst,
        seeded.proposal_event_seq,
    );
    reordered.input_refs = vec![
        "model-lane-message://msg-proposal-001".into(),
        "model-lane-message://msg-critique-001".into(),
    ];
    reordered.selected_input_refs = vec!["model-lane-message://msg-proposal-001".into()];
    reordered.rejected_input_refs = vec!["model-lane-message://msg-critique-001".into()];
    let second = store
        .record_promotion_decision(reordered)
        .await
        .expect("reordered decision");

    assert_eq!(first.canonical_input_refs, second.canonical_input_refs);
    assert_eq!(first.canonical_hash_basis, second.canonical_hash_basis);
    assert_eq!(
        first.canonical_decision_hash, second.canonical_decision_hash,
        "input ref order, decision row id, and idempotency key must not change canonical decision hash"
    );
}

#[tokio::test]
async fn model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources() {
    let (pool, store) = model_lane_store().await;
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(
        store.write_scope().expect("promotion fixture write scope"),
    )
    .expect("promotion fixture uses all five exact scope dimensions");
    let seeded = seed_run_with_advisory_messages(&store).await;

    let stored = store
        .record_promotion_decision(sample_decision(
            "decision-scoped-001",
            "idem-promotion-scoped-001",
            ModelLaneRoutingPolicy::ParallelDebate,
            seeded.proposal_event_seq,
        ))
        .await
        .expect("exact owner records PromotionGate decision");
    assert_eq!(stored.outcome, ModelLanePromotionOutcome::Approved);
    assert_eq!(stored.canonical_hash_basis["resource_scope"], json!(exact));

    let decision_scope: (
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT owner_account_id, actor_principal_id, authenticated_session_id, \
                access_space_id, workspace_id \
         FROM model_lane_promotion_decisions WHERE decision_id = $1",
    )
    .bind(&stored.decision_id)
    .fetch_one(&pool)
    .await
    .expect("read PromotionGate decision scope without a filtering predicate");
    assert_eq!(decision_scope, scope_tuple(&exact));

    let ledger_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&stored.event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("read PromotionGate EventLedger payload");
    let mut review_failures = Vec::new();
    for (field, expected) in exact_scope_json_fields(&exact) {
        assert_eq!(
            ledger_payload.get(field),
            Some(&expected),
            "PromotionGate EventLedger payload must server-stamp {field}"
        );
    }
    for (field, expected) in [
        ("event_ledger_event_id", json!(stored.event_ledger_event_id)),
        ("event_ledger_seq", json!(stored.event_ledger_seq)),
        ("event_stream_version", json!(stored.event_stream_version)),
        ("transaction_seq", json!(stored.transaction_seq)),
    ] {
        if ledger_payload["record"][field] != expected {
            review_failures.push(format!(
                "decision EventLedger record cannot reconstruct {field}: payload={} stored={expected}",
                ledger_payload["record"][field]
            ));
        }
    }

    let promoted = store
        .record_message(promoted_message(
            "msg-promoted-001",
            "idem-message-promoted-001",
            &stored.decision_id,
            "promotion-gate://mt004/approved",
            "promotion-receipt://mt004/approved",
        ))
        .await
        .expect("exact owner consumes its PromotionGate decision");
    assert_eq!(promoted.authority, ModelLaneAuthority::Promoted);
    let promoted_ledger_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&promoted.event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("read promoted-message EventLedger payload");
    for (field, expected) in exact_scope_json_fields(&exact) {
        if promoted_ledger_payload.get(field) != Some(&expected) {
            review_failures.push(format!(
                "promoted-message EventLedger payload does not server-stamp {field}"
            ));
        }
    }
    let promoted_scope: (
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT owner_account_id, actor_principal_id, authenticated_session_id, \
                access_space_id, workspace_id \
         FROM model_lane_messages WHERE message_id = $1",
    )
    .bind(&promoted.message_id)
    .fetch_one(&pool)
    .await
    .expect("read promoted artifact scope without a filtering predicate");
    assert_eq!(promoted_scope, scope_tuple(&exact));

    let exact_reader = ModelLaneStore::new_with_access(
        pool.clone(),
        ResourceAccessContext::for_exact_reader(exact.clone()),
    );
    assert_eq!(
        exact_reader
            .replay_promotion_decisions("run-mt004")
            .await
            .expect("exact owner replays PromotionGate decisions")
            .len(),
        1
    );

    for (dimension, foreign) in foreign_exact_scopes(&exact) {
        let foreign_store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&foreign));
        assert!(
            foreign_store
                .replay_promotion_decisions("run-mt004")
                .await
                .expect("foreign replay is an empty non-disclosing result")
                .is_empty(),
            "foreign {dimension} must not discover the PromotionGate decision"
        );
        let error = foreign_store
            .record_message(promoted_message(
                &format!("msg-promoted-foreign-{dimension}"),
                &format!("idem-message-promoted-foreign-{dimension}"),
                &stored.decision_id,
                "promotion-gate://mt004/approved",
                "promotion-receipt://mt004/approved",
            ))
            .await
            .expect_err("foreign exact scope cannot consume another scope's decision");
        assert!(
            error
                .to_string()
                .contains("ModelLaneMessage authority unavailable"),
            "foreign {dimension} denial must use non-disclosing message-authority wording: {error}"
        );
    }

    let foreign = foreign_exact_scopes(&exact)
        .into_iter()
        .find(|(dimension, _)| *dimension == "principal")
        .expect("foreign Principal fixture")
        .1;
    let foreign_store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&foreign));
    let foreign_proposal_seq = seed_collision_scope(&foreign_store, "foreign-collision").await;
    let mut same_hash_collision = advisory_message(
        "msg-foreign-same-hash-collision",
        &promoted.idempotency_key,
        "lane-local-foreign-collision",
        ModelLaneMessageKind::Status,
        "foreign retry must not discover owner message",
    );
    retarget_message_to_collision_scope(
        &mut same_hash_collision,
        "foreign-collision",
        &promoted.payload_sha256,
    );
    let mut different_hash_collision = same_hash_collision.clone();
    different_hash_collision.message_id = "msg-foreign-different-hash-collision".into();
    different_hash_collision.message_span_id = "span-msg-foreign-different-hash-collision".into();
    different_hash_collision.payload_sha256 = sample_sha256('9');
    let before_message_collision_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_message'",
    )
    .fetch_one(&pool)
    .await
    .expect("count message events before collision probes");
    let same_hash_error = foreign_store
        .record_message(same_hash_collision)
        .await
        .expect_err("foreign same-hash idempotency collision must fail closed");
    let different_hash_error = foreign_store
        .record_message(different_hash_collision)
        .await
        .expect_err("foreign different-hash idempotency collision must fail closed");
    assert_eq!(
        same_hash_error.to_string(),
        different_hash_error.to_string(),
        "foreign message idempotency collisions must not disclose payload-hash equality"
    );
    assert!(
        same_hash_error
            .to_string()
            .contains("ModelLaneMessage authority unavailable"),
        "foreign message collision must use generic non-disclosing wording: {same_hash_error}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_message'",
        )
        .fetch_one(&pool)
        .await
        .expect("count message events after collision probes"),
        before_message_collision_events,
        "foreign message collision probes must not append EventLedger rows"
    );
    let before_collision_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_promotion_decision'",
    )
    .fetch_one(&pool)
    .await
    .expect("count promotion events before collision probes");
    let mut decision_collision_input = sample_decision(
        &stored.decision_id,
        "idem-promotion-foreign-decision-collision",
        ModelLaneRoutingPolicy::OperatorLane,
        foreign_proposal_seq,
    );
    retarget_decision_to_collision_scope(&mut decision_collision_input, "foreign-collision");
    let decision_collision = foreign_store
        .record_promotion_decision(decision_collision_input)
        .await
        .expect_err("foreign scope cannot probe a reused physical decision id");
    let mut idempotency_collision_input = sample_decision(
        "decision-foreign-idempotency-collision",
        &stored.idempotency_key,
        ModelLaneRoutingPolicy::OperatorLane,
        foreign_proposal_seq,
    );
    retarget_decision_to_collision_scope(&mut idempotency_collision_input, "foreign-collision");
    let idempotency_collision = foreign_store
        .record_promotion_decision(idempotency_collision_input)
        .await
        .expect_err("foreign scope cannot probe a reused physical idempotency key");
    if decision_collision.to_string() != idempotency_collision.to_string()
        || !decision_collision
            .to_string()
            .contains("PromotionGate decision authority unavailable")
    {
        review_failures.push(format!(
            "cross-scope physical-key collision remains an oracle: decision={decision_collision}; idempotency={idempotency_collision}"
        ));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_promotion_decision'",
        )
        .fetch_one(&pool)
        .await
        .expect("count promotion events after collision probes"),
        before_collision_events,
        "foreign collision probes must not append EventLedger rows"
    );

    let unscoped = ModelLaneStore::new(pool.clone());
    unscoped
        .record_promotion_decision(sample_decision(
            "decision-unscoped-denied",
            "idem-promotion-unscoped-denied",
            ModelLaneRoutingPolicy::ParallelDebate,
            seeded.proposal_event_seq,
        ))
        .await
        .expect_err("unscoped PromotionGate fixture must fail closed");

    let foreign = foreign_exact_scopes(&exact)
        .into_iter()
        .find(|(dimension, _)| *dimension == "access-space")
        .expect("foreign AccessSpace fixture")
        .1;
    let foreign_store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&foreign));
    let before_foreign_write_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_message'",
    )
    .fetch_one(&pool)
    .await
    .expect("count message events before foreign source-lane write");
    let foreign_write_error = foreign_store
        .record_message(advisory_message(
            "msg-foreign-owner-run-denied",
            "idem-message-foreign-owner-run-denied",
            "lane-local",
            ModelLaneMessageKind::Critique,
            "foreign AccessSpace must not append through owner source lane",
        ))
        .await
        .expect_err("foreign scope cannot append through owner run/lane authority");
    assert!(
        foreign_write_error
            .to_string()
            .contains("ModelLaneMessage authority unavailable"),
        "foreign source-lane denial must be non-disclosing: {foreign_write_error}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM model_lane_messages WHERE message_id = 'msg-foreign-owner-run-denied'",
        )
        .fetch_one(&pool)
        .await
        .expect("count denied foreign message rows"),
        0,
        "foreign source-lane denial must not persist a message row"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_message'",
        )
        .fetch_one(&pool)
        .await
        .expect("count message events after foreign source-lane write"),
        before_foreign_write_events,
        "foreign source-lane denial must happen before EventLedger append"
    );
    seed_collision_scope(&foreign_store, "foreign-mixed").await;
    let mut mixed = sample_decision(
        "decision-mixed-scope-denied",
        "idem-promotion-mixed-scope-denied",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    mixed
        .input_refs
        .push("model-lane-message://msg-proposal-foreign-mixed".into());
    mixed.selected_input_refs = vec![
        "model-lane-message://msg-proposal-001".into(),
        "model-lane-message://msg-proposal-foreign-mixed".into(),
    ];
    mixed.rejected_input_refs = vec!["model-lane-message://msg-critique-001".into()];
    let mixed = store
        .record_promotion_decision(mixed)
        .await
        .expect("mixed-scope denial remains durable for the requesting owner");
    assert_eq!(mixed.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        mixed.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM model_lane_messages \
             WHERE authority = 'promoted' AND record_json->>'promotion_decision_id' = $1",
        )
        .bind(&mixed.decision_id)
        .fetch_one(&pool)
        .await
        .expect("count mixed-scope authority mutations"),
        0,
        "a mixed-scope denial must not create a promoted artifact"
    );

    let mut fabricated_artifact = sample_decision(
        "decision-fabricated-artifact-denied",
        "idem-promotion-fabricated-artifact-denied",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    fabricated_artifact.promoted_artifact_ref = Some("artifact://mt004/promoted/fabricated".into());
    match store.record_promotion_decision(fabricated_artifact).await {
        Ok(record)
            if record.outcome != ModelLanePromotionOutcome::Denied
                || record.denial_reason
                    != Some(ModelLanePromotionDenialReason::MissingPromotedArtifactBinding) =>
        {
            review_failures.push(format!(
                "fabricated artifact ref was not durably denied: outcome={:?}, reason={:?}",
                record.outcome, record.denial_reason
            ));
        }
        Ok(_) => {}
        Err(error) => review_failures.push(format!(
            "fabricated artifact should produce a replayable PromotionGate denial: {error}"
        )),
    }
    assert!(
        review_failures.is_empty(),
        "MT-004 independent-review blockers remain:\n{}",
        review_failures.join("\n")
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_decision_projection_tamper_on_every_consumer() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;
    let decision = sample_decision(
        "decision-projection-tamper",
        "idem-decision-projection-tamper",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    let stored = store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record decision before projection tamper");
    sqlx::query(
        "UPDATE model_lane_promotion_decisions \
         SET record_json = jsonb_set(record_json, '{canonical_hash_basis,routing_policy}', '\"tampered\"'::jsonb) \
         WHERE decision_id = $1",
    )
    .bind(&stored.decision_id)
    .execute(&pool)
    .await
    .expect("tamper decision projection without changing EventLedger");

    let mut failures = Vec::new();
    capture_authority_failure(
        "decision replay accepted projection-only tamper",
        store.replay_promotion_decisions("run-mt004").await,
        &mut failures,
    );
    capture_authority_failure(
        "decision idempotent retry accepted projection-only tamper",
        store.record_promotion_decision(decision).await,
        &mut failures,
    );
    capture_authority_failure(
        "promoted consumption accepted projection-only decision tamper",
        store
            .record_message(promoted_message(
                "msg-promoted-001",
                "idem-promoted-decision-projection-tamper",
                &stored.decision_id,
                "promotion-gate://mt004/approved",
                "promotion-receipt://mt004/approved",
            ))
            .await,
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "decision projection authority gaps:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_decision_eventledger_scope_tamper_on_every_consumer() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;
    let decision = sample_decision(
        "decision-ledger-tamper",
        "idem-decision-ledger-tamper",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    let stored = store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record decision before EventLedger tamper");
    sqlx::query(
        "UPDATE kernel_event_ledger \
         SET payload = jsonb_set(payload, '{workspace_id}', '\"workspace-foreign-tamper\"'::jsonb) \
         WHERE event_id = $1",
    )
    .bind(&stored.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("tamper decision EventLedger exact scope");

    let mut failures = Vec::new();
    capture_authority_failure(
        "decision replay accepted EventLedger scope tamper",
        store.replay_promotion_decisions("run-mt004").await,
        &mut failures,
    );
    capture_authority_failure(
        "decision idempotent retry accepted EventLedger scope tamper",
        store.record_promotion_decision(decision).await,
        &mut failures,
    );
    capture_authority_failure(
        "promoted consumption accepted EventLedger decision scope tamper",
        store
            .record_message(promoted_message(
                "msg-promoted-001",
                "idem-promoted-decision-ledger-tamper",
                &stored.decision_id,
                "promotion-gate://mt004/approved",
                "promotion-receipt://mt004/approved",
            ))
            .await,
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "decision EventLedger authority gaps:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_message_projection_and_scope_tamper_on_replay_and_retry() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;
    let decision = store
        .record_promotion_decision(sample_decision(
            "decision-message-tamper",
            "idem-decision-message-tamper",
            ModelLaneRoutingPolicy::ParallelDebate,
            seeded.proposal_event_seq,
        ))
        .await
        .expect("record decision before message tamper");
    let promoted_input = promoted_message(
        "msg-promoted-001",
        "idem-promoted-message-tamper",
        &decision.decision_id,
        "promotion-gate://mt004/approved",
        "promotion-receipt://mt004/approved",
    );
    let promoted = store
        .record_message(promoted_input.clone())
        .await
        .expect("record promoted message before tamper");
    let original_projection: Value =
        sqlx::query_scalar("SELECT record_json FROM model_lane_messages WHERE message_id = $1")
            .bind(&promoted.message_id)
            .fetch_one(&pool)
            .await
            .expect("read original message projection");
    sqlx::query(
        "UPDATE model_lane_messages \
         SET record_json = jsonb_set(record_json, '{summary}', '\"projection tamper\"'::jsonb) \
         WHERE message_id = $1",
    )
    .bind(&promoted.message_id)
    .execute(&pool)
    .await
    .expect("tamper promoted message projection");

    let mut failures = Vec::new();
    capture_authority_failure(
        "run replay accepted promoted-message projection tamper",
        store.replay_run("run-mt004").await,
        &mut failures,
    );
    capture_authority_failure(
        "message retry accepted promoted-message projection tamper",
        store.record_message(promoted_input.clone()).await,
        &mut failures,
    );
    sqlx::query("UPDATE model_lane_messages SET record_json = $2 WHERE message_id = $1")
        .bind(&promoted.message_id)
        .bind(original_projection)
        .execute(&pool)
        .await
        .expect("restore promoted message projection");
    sqlx::query(
        "UPDATE kernel_event_ledger \
         SET payload = jsonb_set(payload, '{access_space_id}', '\"00000000-0000-4000-8000-000000000099\"'::jsonb) \
         WHERE event_id = $1",
    )
    .bind(&promoted.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("tamper promoted message EventLedger exact scope");
    capture_authority_failure(
        "run replay accepted promoted-message EventLedger scope tamper",
        store.replay_run("run-mt004").await,
        &mut failures,
    );
    capture_authority_failure(
        "message retry accepted promoted-message EventLedger scope tamper",
        store.record_message(promoted_input).await,
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "message projection/EventLedger authority gaps:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn model_lane_promotion_durably_denies_tampered_advisory_eventledger_input() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;
    let proposal_event_id: String = sqlx::query_scalar(
        "SELECT event_ledger_event_id FROM model_lane_messages WHERE message_id = 'msg-proposal-001'",
    )
    .fetch_one(&pool)
    .await
    .expect("read proposal EventLedger id");
    sqlx::query(
        "UPDATE kernel_event_ledger \
         SET payload = jsonb_set(payload, '{record,summary}', '\"tampered advisory payload\"'::jsonb) \
         WHERE event_id = $1",
    )
    .bind(proposal_event_id)
    .execute(&pool)
    .await
    .expect("tamper advisory EventLedger payload");
    let denied = store
        .record_promotion_decision(sample_decision(
            "decision-tampered-advisory-denied",
            "idem-decision-tampered-advisory-denied",
            ModelLaneRoutingPolicy::ParallelDebate,
            seeded.proposal_event_seq,
        ))
        .await
        .expect("tampered advisory input must produce a replayable denial");
    assert_eq!(denied.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        denied.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
}

#[tokio::test]
async fn model_lane_exact_scope_authority_rejects_message_run_lane_and_navigation_tamper() {
    for suffix in ["missing", "malformed", "extra-key"] {
        let (pool, store) = model_lane_store().await;
        let seeded = seed_run_with_advisory_messages(&store).await;
        let proposal_event_id: String = sqlx::query_scalar(
            "SELECT event_ledger_event_id FROM model_lane_messages \
             WHERE message_id = 'msg-proposal-001'",
        )
        .fetch_one(&pool)
        .await
        .expect("read advisory EventLedger id");
        match suffix {
            "malformed" => {
                sqlx::query(
                    "UPDATE kernel_event_ledger \
                     SET payload = jsonb_set(payload, '{owner_account_id}', \
                         '\"not-a-uuid\"'::jsonb) \
                     WHERE event_id = $1",
                )
                .bind(&proposal_event_id)
                .execute(&pool)
                .await
                .expect("malform advisory EventLedger exact scope");
            }
            "extra-key" => {
                sqlx::query(
                    "UPDATE kernel_event_ledger \
                     SET payload = jsonb_set(payload, '{unexpected_scope_dimension}', 'true'::jsonb) \
                     WHERE event_id = $1",
                )
                .bind(&proposal_event_id)
                .execute(&pool)
                .await
                .expect("add unexpected advisory EventLedger scope dimension");
            }
            _ => {
                sqlx::query(
                    "UPDATE kernel_event_ledger SET payload = payload - 'owner_account_id' \
                     WHERE event_id = $1",
                )
                .bind(&proposal_event_id)
                .execute(&pool)
                .await
                .expect("remove advisory EventLedger exact scope dimension");
            }
        }

        let replay = store.replay_run("run-mt004").await;
        assert!(
            replay.is_err(),
            "{suffix} advisory EventLedger exact scope was accepted by replay"
        );
        let denied = store
            .record_promotion_decision(sample_decision(
                &format!("decision-advisory-scope-{suffix}"),
                &format!("idem-decision-advisory-scope-{suffix}"),
                ModelLaneRoutingPolicy::ParallelDebate,
                seeded.proposal_event_seq,
            ))
            .await
            .expect("advisory scope tamper must produce a replayable denial");
        assert_eq!(denied.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(
            denied.denial_reason,
            Some(ModelLanePromotionDenialReason::InputRefMismatch),
            "{suffix} advisory EventLedger scope must fail promotion input authority"
        );
    }

    let (pool, owner_store) = model_lane_store().await;
    let owner_exact = owner_store
        .access()
        .exact_read_scope()
        .cloned()
        .expect("owner store exposes complete exact scope");
    let mut foreign_exact = owner_exact.clone();
    foreign_exact.owner_account_id = OwnerAccountId::mint();
    foreign_exact.actor_principal_id = ActorPrincipalId::mint();
    foreign_exact.authenticated_session_id = AuthenticatedSessionRef::mint();
    foreign_exact.access_space_id = AccessSpaceRef::mint();
    foreign_exact.workspace_id = WorkspaceScopeRef::new("workspace-mt004-foreign-projection")
        .expect("nonblank foreign projection workspace");
    let foreign_store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&foreign_exact));
    seed_collision_scope(&foreign_store, "projection-scope-tamper").await;
    let (owner, principal, session, access_space, workspace) = scope_tuple(&owner_exact);
    for table in ["model_lane_runs", "model_lanes"] {
        let key_column = if table == "model_lane_runs" {
            "run_id"
        } else {
            "lane_id"
        };
        let key = if table == "model_lane_runs" {
            "run-mt004-projection-scope-tamper"
        } else {
            "lane-local-projection-scope-tamper"
        };
        let sql = format!(
            "UPDATE {table} SET owner_account_id=$2, actor_principal_id=$3, \
             authenticated_session_id=$4, access_space_id=$5, workspace_id=$6 \
             WHERE {key_column}=$1"
        );
        sqlx::query(&sql)
            .bind(key)
            .bind(owner)
            .bind(principal)
            .bind(session)
            .bind(access_space)
            .bind(workspace.as_deref())
            .execute(&pool)
            .await
            .expect("forge owner scope on foreign run/lane projection");
    }
    let before_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_message'",
    )
    .fetch_one(&pool)
    .await
    .expect("count messages before forged projection append");
    let mut forged_message = advisory_message(
        "msg-forged-projection-scope-denied",
        "idem-msg-forged-projection-scope-denied",
        "lane-local-projection-scope-tamper",
        ModelLaneMessageKind::Proposal,
        "forged projection scope must not manufacture append authority",
    );
    retarget_message_to_collision_scope(
        &mut forged_message,
        "projection-scope-tamper",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let forged_error = owner_store
        .record_message(forged_message)
        .await
        .expect_err("foreign run/lane EventLedger scope must deny forged projection authority");
    assert!(
        forged_error
            .to_string()
            .contains("ModelLaneMessage authority unavailable"),
        "forged run/lane projection denial must be non-disclosing: {forged_error}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM model_lane_messages \
             WHERE message_id = 'msg-forged-projection-scope-denied'",
        )
        .fetch_one(&pool)
        .await
        .expect("count denied forged projection message"),
        0,
        "forged run/lane projection authority must not persist a message"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger \
             WHERE aggregate_type = 'model_lane_message'",
        )
        .fetch_one(&pool)
        .await
        .expect("count messages after forged projection append"),
        before_events,
        "forged run/lane projection authority must fail before EventLedger append"
    );

    let (pool, store) = model_lane_store().await;
    seed_run_with_advisory_messages(&store).await;
    seed_collision_scope(&store, "navigation-target").await;
    sqlx::query(
        r#"UPDATE model_lane_messages
           SET record_json = jsonb_set(
               record_json,
               '{run_id}',
               '"run-mt004-navigation-target"'::jsonb
           )
           WHERE message_id = 'msg-proposal-001'"#,
    )
    .execute(&pool)
    .await
    .expect("tamper message projection run_id before navigation");
    assert!(
        store
            .navigation_by_message("msg-proposal-001")
            .await
            .is_err(),
        "navigation_by_message followed a projection-only run_id redirect"
    );
}

#[test]
fn model_lane_navigation_and_promotion_validate_every_origin_before_run_redirect() {
    // This single acceptance matrix intentionally retains every typed
    // navigation origin in one named proof. Its generated async state exceeds
    // Windows' small default test-thread stack, so run it on an explicitly
    // bounded worker rather than weakening or deleting attack branches.
    std::thread::Builder::new()
        .name("mt004-navigation-origin-matrix".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build MT-004 navigation matrix runtime")
                .block_on(model_lane_navigation_and_promotion_origin_matrix())
        })
        .expect("spawn MT-004 navigation matrix worker")
        .join()
        .expect("MT-004 navigation matrix worker completes");
}

async fn model_lane_navigation_and_promotion_origin_matrix() {
    let (pool, store) = model_lane_store().await;
    seed_run_with_advisory_messages(&store).await;
    seed_collision_scope(&store, "navigation-target").await;
    let target_run = "run-mt004-navigation-target";

    sqlx::query(
        r#"UPDATE model_lanes
           SET trace_id = 'trace-mt004-origin-lane-only',
               record_json = jsonb_set(
                   jsonb_set(
                       jsonb_set(
                           jsonb_set(record_json, '{run_id}', to_jsonb($1::text)),
                           '{trace_id}', '"trace-mt004-origin-lane-only"'::jsonb
                       ),
                       '{locus_binding,locus_binding_ref}', '"locus://mt004/origin-lane-only"'::jsonb
                   ),
                   '{lane_span_id}', '"span-mt004-origin-lane-only"'::jsonb
               )
           WHERE lane_id = 'lane-local'"#,
    )
    .bind(target_run)
    .execute(&pool)
    .await
    .expect("tamper lane navigation origin projection");
    sqlx::query(
        r#"UPDATE model_lanes
           SET record_json = jsonb_set(record_json, '{failstate_code}', '"ERR-MT004-ORIGIN-LANE"'::jsonb)
           WHERE lane_id = 'lane-local'"#,
    )
    .execute(&pool)
    .await
    .expect("seed lane-only error-code navigation origin");
    for (label, result) in [
        (
            "navigation_by_lane",
            store.navigation_by_lane("lane-local").await,
        ),
        (
            "lookup lane_id",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    lane_id: Some("lane-local".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup model_session_id",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    model_session_id: Some("model-session-lane-local".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup session_id",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    session_id: Some("session-lane-local".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "navigation_by_trace lane origin",
            store
                .navigation_by_trace("trace-mt004-origin-lane-only", None)
                .await,
        ),
        (
            "lookup span_id lane origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    span_id: Some("span-mt004-origin-lane-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup locus_ref lane origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    locus_ref: Some("locus://mt004/origin-lane-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup error_code lane origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    error_code: Some("ERR-MT004-ORIGIN-LANE".into()),
                    ..Default::default()
                })
                .await,
        ),
    ] {
        assert!(result.is_err(), "{label} followed tampered lane run_id");
    }

    sqlx::query(
        r#"UPDATE model_lane_messages
           SET trace_id = 'trace-mt004-origin-message-only',
               record_json = jsonb_set(
                   jsonb_set(
                       jsonb_set(
                           jsonb_set(record_json, '{run_id}', to_jsonb($1::text)),
                           '{trace_id}', '"trace-mt004-origin-message-only"'::jsonb
                       ),
                       '{locus_binding,locus_binding_ref}', '"locus://mt004/origin-message-only"'::jsonb
                   ),
                   '{message_span_id}', '"span-mt004-origin-message-only"'::jsonb
               )
           WHERE message_id = 'msg-proposal-001'"#,
    )
    .bind(target_run)
    .execute(&pool)
    .await
    .expect("tamper message navigation origin projection");
    sqlx::query(
        r#"UPDATE model_lane_messages
           SET record_json = jsonb_set(
               jsonb_set(
                   jsonb_set(record_json, '{diagnostic_payload,loom_ref}', '"loom://mt004/origin-message-only"'::jsonb),
                   '{diagnostic_payload,fems_ref}', '"fems://mt004/origin-message-only"'::jsonb
               ),
               '{diagnostic_payload,loom_block_id}', '"loom-block-mt004-origin-message-only"'::jsonb
           )
           WHERE message_id = 'msg-proposal-001'"#,
    )
    .execute(&pool)
    .await
    .expect("seed message-only diagnostic navigation origins");
    for (label, result) in [
        (
            "lookup message_id",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    message_id: Some("msg-proposal-001".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "navigation_by_trace message origin",
            store
                .navigation_by_trace("trace-mt004-origin-message-only", None)
                .await,
        ),
        (
            "lookup span_id message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    span_id: Some("span-mt004-origin-message-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup locus_ref message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    locus_ref: Some("locus://mt004/origin-message-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup artifact_ref message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    artifact_ref: Some("artifact://mt004/msg-proposal-001".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup loom_ref message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    loom_ref: Some("loom://mt004/origin-message-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup fems_ref message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    fems_ref: Some("fems://mt004/origin-message-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup loom_block_id message origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    loom_block_id: Some("loom-block-mt004-origin-message-only".into()),
                    ..Default::default()
                })
                .await,
        ),
    ] {
        assert!(result.is_err(), "{label} followed tampered message run_id");
    }

    sqlx::query(
        r#"UPDATE model_lane_runs
           SET trace_id = 'trace-mt004-origin-run-only',
               record_json = jsonb_set(
                   jsonb_set(
                       jsonb_set(record_json, '{run_id}', to_jsonb($1::text)),
                       '{trace_id}', '"trace-mt004-origin-run-only"'::jsonb
                   ),
                   '{run_span_id}', '"span-mt004-origin-run-only"'::jsonb
               )
           WHERE run_id = 'run-mt004'"#,
    )
    .bind(target_run)
    .execute(&pool)
    .await
    .expect("tamper run navigation origin projection");
    sqlx::query(
        r#"UPDATE model_lane_runs
           SET record_json = jsonb_set(record_json, '{failstate_code}', '"ERR-MT004-ORIGIN-RUN"'::jsonb)
           WHERE run_id = 'run-mt004'"#,
    )
    .execute(&pool)
    .await
    .expect("seed run-only error-code navigation origin");
    sqlx::query(
        r#"UPDATE model_lane_runs
           SET record_json = jsonb_set(
               record_json,
               '{context_bundle_id}',
               '"context-bundle://mt004"'::jsonb
           )
           WHERE run_id = $1"#,
    )
    .bind(target_run)
    .execute(&pool)
    .await
    .expect("make redirect target satisfy context lookup post-filter");
    for (label, result) in [
        (
            "navigation_by_trace run origin",
            store
                .navigation_by_trace("trace-mt004-origin-run-only", None)
                .await,
        ),
        (
            "lookup trace_id run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    trace_id: Some("trace-mt004-origin-run-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup span_id run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    span_id: Some("span-mt004-origin-run-only".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "context_bundle run origin",
            store
                .navigation_by_artifact_or_context(None, Some("context-bundle://mt004"), None)
                .await,
        ),
        (
            "lookup work_packet_id run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    work_packet_id: Some(
                        "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
                    ),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup micro_task_id run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    micro_task_id: Some("MT-004".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup task_board_id run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    task_board_id: Some("task-board://wp-1".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup memory_pack_ref run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    memory_pack_ref: Some("memory-pack://mt004".into()),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup memory_pack_hash run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    memory_pack_hash: Some(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    ),
                    ..Default::default()
                })
                .await,
        ),
        (
            "lookup error_code run origin",
            store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    error_code: Some("ERR-MT004-ORIGIN-RUN".into()),
                    ..Default::default()
                })
                .await,
        ),
    ] {
        assert!(result.is_err(), "{label} followed tampered run run_id");
    }

    let (pool, owner_store) = model_lane_store().await;
    let owner_exact = owner_store
        .access()
        .exact_read_scope()
        .cloned()
        .expect("owner store exact scope");
    let mut foreign_exact = owner_exact.clone();
    foreign_exact.owner_account_id = OwnerAccountId::mint();
    foreign_exact.actor_principal_id = ActorPrincipalId::mint();
    foreign_exact.authenticated_session_id = AuthenticatedSessionRef::mint();
    foreign_exact.access_space_id = AccessSpaceRef::mint();
    foreign_exact.workspace_id = WorkspaceScopeRef::new("workspace-mt004-foreign-promotion-run")
        .expect("nonblank foreign promotion workspace");
    let foreign_store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&foreign_exact));
    let foreign_seq = seed_collision_scope(&foreign_store, "promotion-run-forgery").await;
    let (owner, principal, session, access_space, workspace) = scope_tuple(&owner_exact);
    sqlx::query(
        "UPDATE model_lane_runs SET owner_account_id=$2, actor_principal_id=$3, \
         authenticated_session_id=$4, access_space_id=$5, workspace_id=$6 WHERE run_id=$1",
    )
    .bind("run-mt004-promotion-run-forgery")
    .bind(owner)
    .bind(principal)
    .bind(session)
    .bind(access_space)
    .bind(workspace.as_deref())
    .execute(&pool)
    .await
    .expect("forge owner scope on foreign promotion run projection");
    let before_decisions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_promotion_decisions")
            .fetch_one(&pool)
            .await
            .expect("count decisions before forged run promotion");
    let before_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type='model_lane_promotion_decision'",
    )
    .fetch_one(&pool)
    .await
    .expect("count decision events before forged run promotion");
    let mut decision = sample_decision(
        "decision-forged-promotion-run",
        "idem-decision-forged-promotion-run",
        ModelLaneRoutingPolicy::ParallelDebate,
        foreign_seq,
    );
    retarget_decision_to_collision_scope(&mut decision, "promotion-run-forgery");
    assert!(
        owner_store
            .record_promotion_decision(decision)
            .await
            .is_err(),
        "forged promotion run projection manufactured decision authority"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM model_lane_promotion_decisions")
            .fetch_one(&pool)
            .await
            .expect("count decisions after forged run promotion"),
        before_decisions,
        "forged promotion run must not persist a decision"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type='model_lane_promotion_decision'",
        )
        .fetch_one(&pool)
        .await
        .expect("count decision events after forged run promotion"),
        before_events,
        "forged promotion run must fail before EventLedger append"
    );
}

struct SeededRun {
    proposal_event_seq: i64,
}

async fn model_lane_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-004 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane promotion schema");
    let store = ModelLaneStore::new_scoped(pool.clone(), scope_from_exact(&exact_scope()));
    (pool, store)
}

fn exact_scope() -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new("workspace-mt004-promotion")
            .expect("nonblank promotion workspace"),
    }
}

fn scope_from_exact(exact: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(exact.owner_account_id, exact.actor_principal_id)
        .with_session(exact.authenticated_session_id)
        .with_access_space(exact.access_space_id)
        .with_workspace(exact.workspace_id.clone())
}

fn scope_tuple(
    exact: &ExactResourceScopeAttribution,
) -> (
    Option<uuid::Uuid>,
    Option<uuid::Uuid>,
    Option<uuid::Uuid>,
    Option<uuid::Uuid>,
    Option<String>,
) {
    (
        Some(exact.owner_account_id.as_uuid()),
        Some(exact.actor_principal_id.as_uuid()),
        Some(exact.authenticated_session_id.as_uuid()),
        Some(exact.access_space_id.as_uuid()),
        Some(exact.workspace_id.as_str().to_owned()),
    )
}

fn exact_scope_json_fields(
    exact: &ExactResourceScopeAttribution,
) -> [(&'static str, serde_json::Value); 5] {
    [
        ("owner_account_id", json!(exact.owner_account_id)),
        ("actor_principal_id", json!(exact.actor_principal_id)),
        (
            "authenticated_session_id",
            json!(exact.authenticated_session_id),
        ),
        ("access_space_id", json!(exact.access_space_id)),
        ("workspace_id", json!(exact.workspace_id)),
    ]
}

fn foreign_exact_scopes(
    exact: &ExactResourceScopeAttribution,
) -> Vec<(&'static str, ExactResourceScopeAttribution)> {
    let mut owner = exact.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut principal = exact.clone();
    principal.actor_principal_id = ActorPrincipalId::mint();
    let mut session = exact.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access_space = exact.clone();
    access_space.access_space_id = AccessSpaceRef::mint();
    let mut workspace = exact.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new("workspace-mt004-foreign").expect("nonblank foreign workspace");
    vec![
        ("owner", owner),
        ("principal", principal),
        ("session", session),
        ("access-space", access_space),
        ("workspace", workspace),
    ]
}

async fn seed_run_with_advisory_messages(store: &ModelLaneStore) -> SeededRun {
    store
        .record_run(sample_run())
        .await
        .expect("record MT-004 run");
    store
        .record_context_bundle_artifact_binding(promoted_artifact_binding())
        .await
        .expect("persist promoted artifact through scoped ArtifactStore/EventLedger authority");
    store
        .record_lane(sample_lane(
            "lane-local",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
            ModelLaneProviderKind::LocalRuntime,
        ))
        .await
        .expect("record local lane");

    // Cloud lanes fail closed unless durable ProjectionPlan/ConsentReceipt
    // authority already exists (spec 4.3.9.2.5, CX-MM-007). Seed the cloud
    // lane's authority before recording it, matching the identity that
    // `sample_lane("lane-cloud", Cloud, ...)` stamps.
    model_lane_cloud_support::seed_cloud_lane_authority(
        store,
        model_lane_cloud_support::CloudLaneAuthoritySpec {
            run_id: "run-mt004",
            lane_id: "lane-cloud",
            model_session_id: "model-session-lane-cloud",
            provider_kind: ModelLaneProviderKind::OpenAi.as_str(),
            requested_model_id: "model://mt004/lane-cloud",
            projection_plan_id: "projection-plan://lane-cloud",
            consent_receipt_id: "consent://lane-cloud",
            event_ledger_stream_id: "mlane-stream-run-mt004",
            work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            micro_task_id: "MT-004",
            task_board_id: "task-board://wp-1",
            owner_session: "KERNEL_BUILDER-MT004",
        },
    )
    .await;

    store
        .record_lane(sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
            ModelLaneProviderKind::OpenAi,
        ))
        .await
        .expect("record cloud lane");

    let proposal = store
        .record_message(advisory_message(
            "msg-proposal-001",
            "idem-message-proposal-001",
            "lane-local",
            ModelLaneMessageKind::Proposal,
            "local lane proposes CRDT edit",
        ))
        .await
        .expect("record advisory proposal");
    store
        .record_message(advisory_message(
            "msg-critique-001",
            "idem-message-critique-001",
            "lane-cloud",
            ModelLaneMessageKind::Critique,
            "cloud lane critiques local plan",
        ))
        .await
        .expect("record advisory critique");

    SeededRun {
        proposal_event_seq: proposal.event_ledger_seq,
    }
}

async fn seed_collision_scope(store: &ModelLaneStore, suffix: &str) -> i64 {
    let run_id = format!("run-mt004-{suffix}");
    let lane_id = format!("lane-local-{suffix}");
    let message_id = format!("msg-proposal-{suffix}");
    let mut run = sample_run();
    run.run_id = run_id.clone();
    run.trace_id = format!("trace-mt004-{suffix}");
    run.run_span_id = format!("span-run-mt004-{suffix}");
    run.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    run.context_bundle_id = format!("context-bundle://mt004/{suffix}");
    run.lane_ids = vec![lane_id.clone()];
    run.event_ledger_stream_id = format!("mlane-stream-{run_id}");
    run.artifact_namespace = format!("artifact://model-lane/mt004/{suffix}");
    run.projection_plan_ref = None;
    run.consent_receipt_ref = None;
    run.idempotency_key = format!("idem-{run_id}");
    if let Some(locus) = run.locus_binding.as_mut() {
        locus.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    }
    store
        .record_run(run)
        .await
        .expect("record collision-scope run");

    let mut lane = sample_lane(
        &lane_id,
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
        ModelLaneProviderKind::LocalRuntime,
    );
    lane.run_id = run_id.clone();
    lane.trace_id = format!("trace-mt004-{suffix}");
    lane.event_ledger_stream_id = format!("mlane-stream-{run_id}");
    lane.session_id = format!("session-{lane_id}");
    lane.model_session_id = format!("model-session-{lane_id}");
    lane.locus_binding = Some(sample_locus(&lane.session_id, &lane.model_session_id));
    if let Some(locus) = lane.locus_binding.as_mut() {
        locus.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    }
    store
        .record_lane(lane)
        .await
        .expect("record collision-scope lane");

    let mut message = advisory_message(
        &message_id,
        &format!("idem-{message_id}"),
        &lane_id,
        ModelLaneMessageKind::Proposal,
        "foreign scope collision proposal",
    );
    message.run_id = run_id.clone();
    message.trace_id = format!("trace-mt004-{suffix}");
    message.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    message.event_ledger_stream_id = format!("mlane-stream-{run_id}");
    if let Some(locus) = message.locus_binding.as_mut() {
        locus.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    }
    store
        .record_message(message)
        .await
        .expect("record collision-scope proposal")
        .event_ledger_seq
}

fn retarget_decision_to_collision_scope(
    decision: &mut NewModelLanePromotionDecision,
    suffix: &str,
) {
    let run_id = format!("run-mt004-{suffix}");
    let message_id = format!("msg-proposal-{suffix}");
    let message_ref = format!("model-lane-message://{message_id}");
    decision.run_id = run_id.clone();
    decision.trace_id = format!("trace-mt004-{suffix}");
    decision.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    decision.input_refs = vec![message_ref.clone()];
    decision.selected_input_refs = vec![message_ref];
    decision.rejected_input_refs.clear();
    decision.expected_event_ledger_aggregate_id = message_id;
    decision.event_ledger_stream_id = format!("mlane-stream-{run_id}");
    decision.diagnostic_payload = json!({
        "flight_recorder": "cross-scope physical-key collision probe"
    });
}

fn retarget_message_to_collision_scope(
    message: &mut NewModelLaneMessage,
    suffix: &str,
    payload_sha256: &str,
) {
    let run_id = format!("run-mt004-{suffix}");
    let lane_id = format!("lane-local-{suffix}");
    message.run_id = run_id.clone();
    message.trace_id = format!("trace-mt004-{suffix}");
    message.from_lane_id = lane_id.clone();
    message.parent_span_id = Some(format!("span-{lane_id}"));
    message.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    message.event_ledger_stream_id = format!("mlane-stream-{run_id}");
    message.payload_sha256 = payload_sha256.into();
    message.locus_binding = Some(sample_locus(
        &format!("session-{lane_id}"),
        &format!("model-session-{lane_id}"),
    ));
    if let Some(locus) = message.locus_binding.as_mut() {
        locus.coordinator_session_id = format!("coordinator-session-mt004-{suffix}");
    }
}

fn sample_run() -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        run_span_id: "span-run-mt004".into(),
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy: ModelLaneRoutingPolicy::ParallelDebate.as_str().into(),
        context_bundle_id: "context-bundle://mt004".into(),
        lane_ids: vec!["lane-local".into(), "lane-cloud".into()],
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        artifact_namespace: "artifact://model-lane/mt004".into(),
        projection_plan_ref: Some("projection-plan://mt004/cloud-review".into()),
        consent_receipt_ref: Some("consent://mt004/cloud-review".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: "idem-run-mt004".into(),
        replay_order_key: "00000000/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#run".into()),
        locus_binding: Some(sample_locus("session-run", "model-session-run")),
        memory_pack_ref: "memory-pack://mt004".into(),
        memory_pack_hash: sample_sha256('a'),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt004".into(),
        selected_model_id: Some("model://mt004/local".into()),
        candidate_model_ids: vec!["model://mt004/local".into(), "model://mt004/cloud".into()],
        procedural_review_status: "runtime_promotion_preflight".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec!["rejection://mt004/no-direct-authority".into()],
    }
}

fn sample_lane(
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
) -> NewModelLane {
    let process_backed = !matches!(
        runtime_binding,
        RuntimeBinding::Human | RuntimeBinding::Subagent | RuntimeBinding::Validator
    );
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("backend-{lane_id}"),
        model_id: Some(format!("model://mt004/{lane_id}")),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: runtime_binding.clone(),
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt004/read-context".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://{lane_id}")),
        provider_feature_profile_ref: Some(format!("provider-feature-profile://{lane_id}")),
        requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane_id}")),
        effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane_id}")),
        projection_plan_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("projection-plan://{lane_id}")),
        consent_receipt_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("consent://{lane_id}")),
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-29T08:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T08:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T08:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt004".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt004".into()),
        process_ownership_ref: process_backed.then_some(format!("process-ledger://{lane_id}")),
        no_os_process_reason_ref: (!process_backed).then_some(format!("no-os://{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt004".into()),
        last_runtime_status_ref: Some("runtime-status://ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
    }
}

fn advisory_message(
    message_id: &str,
    idempotency_key: &str,
    lane_id: &str,
    kind: ModelLaneMessageKind,
    summary: &str,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec!["span-coordinator-mt004".into()],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(sample_routing(
            &format!("corr-{message_id}"),
            "coordinator",
            "coordinator-session-mt004",
        )),
        kind,
        payload_ref: format!("artifact://mt004/{message_id}"),
        payload_sha256: sample_sha256('b'),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        summary: summary.into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000010/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#advisory".into()),
        created_at_utc: "2026-06-29T08:01:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger-backed",
            "locus": "locus://wp1/mt004/coordinator-session-mt004",
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn promoted_message(
    message_id: &str,
    idempotency_key: &str,
    promotion_decision_id: &str,
    promotion_gate_ref: &str,
    promotion_receipt_ref: &str,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some("span-coordinator-mt004".into()),
        linked_span_contexts: vec!["span-msg-proposal-001".into()],
        from_lane_id: "lane-local".into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(sample_routing(
            &format!("corr-{message_id}"),
            "coordinator",
            "coordinator-session-mt004",
        )),
        kind: ModelLaneMessageKind::PromotionRequest,
        payload_ref: format!("artifact://mt004/{message_id}"),
        payload_sha256: sample_sha256('c'),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        summary: "promoted model-lane output after explicit PromotionGate decision".into(),
        authority: ModelLaneAuthority::Promoted,
        promotion_decision_id: Some(promotion_decision_id.into()),
        promotion_gate_ref: Some(promotion_gate_ref.into()),
        promotion_receipt_ref: Some(promotion_receipt_ref.into()),
        validator_verdict_ref: Some("validator://mt004/v1".into()),
        operator_decision_ref: Some("operator://mt004/approve".into()),
        promoted_artifact_ref: Some(format!("artifact://mt004/promoted/{message_id}")),
        promoted_artifact_sha256: Some(promoted_artifact_hash()),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            "session-lane-local",
            "model-session-lane-local",
        )),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000030/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#promoted".into()),
        created_at_utc: "2026-06-29T08:03:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "PromotionGate EventLedger row required",
            "direct_authority_mutation": "denied_without_decision"
        }),
    }
}

fn sample_decision(
    decision_id: &str,
    idempotency_key: &str,
    routing_policy: ModelLaneRoutingPolicy,
    expected_event_ledger_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some("span-coordinator-mt004".into()),
        linked_span_contexts: vec![
            "span-msg-proposal-001".into(),
            "span-msg-critique-001".into(),
        ],
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            "model-lane-message://msg-critique-001".into(),
            "model-lane-message://msg-proposal-001".into(),
        ],
        selected_input_refs: vec!["model-lane-message://msg-proposal-001".into()],
        rejected_input_refs: vec!["model-lane-message://msg-critique-001".into()],
        validator_authority_ref: Some("validator://mt004/v1".into()),
        operator_authority_ref: Some("operator://mt004/approve".into()),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: "msg-proposal-001".into(),
        expected_event_ledger_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: "promotion-gate://mt004/approved".into(),
        promotion_receipt_ref: Some("promotion-receipt://mt004/approved".into()),
        promoted_artifact_ref: Some("artifact://mt004/promoted/msg-promoted-001".into()),
        promoted_artifact_sha256: Some(promoted_artifact_hash()),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000020/{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-promotion#decision".into()),
        created_at_utc: "2026-06-29T08:02:00Z".into(),
        // Every MT-004 routing policy that reaches a cloud dispatch stage
        // (LocalFirst cloud-escalation, CloudReview cloud-review,
        // CloudPlanLocalExecute cloud-plan, ParallelDebate debate-cloud) is
        // gated by `ModelLaneRoutingAuthorityGate::CloudConsent`.
        // `record_promotion_decision` runs `require_authority_contract` over the
        // full canonical graph, so the decision must present a cloud-consent
        // authority reference. This is the durable ConsentReceipt id that
        // `seed_run_with_advisory_messages` persists via
        // `model_lane_cloud_support::seed_cloud_lane_authority` for `lane-cloud`
        // (spec 4.3.9.2.5 / CX-MM-007), not a stub. It matches the canonical
        // production seeding pattern used by the MT-006/MT-009 cloud suites,
        // which set `diagnostic_payload["cloud_consent_receipt_ref"]` to the
        // seeded receipt id before recording the promotion decision.
        diagnostic_payload: json!({
            "cloud_consent_receipt_ref": "consent://lane-cloud",
            "flight_recorder": "EventLedger append required before authority mutation",
            "loom": "promotion is deterministic host-side state",
            "locus": "locus://wp1/mt004/coordinator-session-mt004",
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn sample_locus(session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: "coordinator-session-mt004".into(),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding_ref: "locus://wp1/mt004/coordinator-session-mt004".into(),
    }
}

fn sample_routing(
    correlation_id: &str,
    target_role: &str,
    target_session: &str,
) -> ModelLaneRoutingMetadata {
    ModelLaneRoutingMetadata {
        target_role: target_role.into(),
        target_session: target_session.into(),
        correlation_id: correlation_id.into(),
        requires_ack: true,
        ack_for: None,
    }
}

fn sample_sha256(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}

fn capture_authority_failure<T: std::fmt::Debug>(
    label: &str,
    result: Result<T, ModelLaneError>,
    failures: &mut Vec<String>,
) {
    match result {
        Ok(value) => failures.push(format!("{label}: accepted {value:?}")),
        Err(error)
            if !error.to_string().contains("EventLedger")
                && !error.to_string().contains("canonical authority")
                && !error
                    .to_string()
                    .contains("ModelLane navigation authority unavailable") =>
        {
            failures.push(format!("{label}: non-authoritative error {error}"));
        }
        Err(_) => {}
    }
}

fn promoted_artifact_payload() -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_promoted_artifact@1",
        "artifact_version": "1",
        "body": "deterministic MT-004 promoted artifact"
    })
}

fn promoted_artifact_hash() -> String {
    let bytes = serde_json::to_vec(&promoted_artifact_payload())
        .expect("serialize deterministic promoted artifact");
    format!("{:x}", Sha256::digest(bytes))
}

fn promoted_artifact_binding() -> NewModelLaneContextBundleArtifactBinding {
    let artifact_ref = "artifact://mt004/promoted/msg-promoted-001".to_string();
    let hash = promoted_artifact_hash();
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: "artifact-binding-mt004-promoted-001".into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        artifact_ref: artifact_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_promoted_artifact".into(),
        artifact_manifest_ref: "artifact-store://model-lane/mt004/promoted/artifact.json".into(),
        artifact_payload_ref: artifact_ref,
        payload_json: promoted_artifact_payload(),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: "idem-artifact-binding-mt004-promoted-001".into(),
        created_at_utc: "2026-06-29T08:00:30Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger authority for promoted artifact"
        }),
    }
}
