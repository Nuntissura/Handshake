//! MT-004 PromotionGate authority over one embedded SurrealDB namespace/database.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, SurrealStorage, SurrealTestInspector,
};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneAuthorityTestCorruption, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLanePromotionDenialReason,
    ModelLanePromotionOutcome, ModelLanePromotionState, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessLifecycleRegistry, ResourceScope, WorkspaceScopeRef,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use surreal_test_store_support::EmbeddedSurrealTestScope;

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    lifecycle: ResourceAccessLifecycleRegistry,
    store: ModelLaneStore,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate MT-004 embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical schema");
        let scope = exact_scope(label);
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        register_active_context(&lifecycle, &scope);
        let store = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            scope.clone(),
            lifecycle.clone(),
        );
        Self {
            isolated,
            storage,
            scope,
            lifecycle,
            store,
        }
    }

    /// One iteration of a sequential matrix, isolated by its own exact five-field scope inside
    /// the harness's already-bootstrapped store. A fresh canonical bootstrap costs minutes, so a
    /// per-iteration store would dominate this suite's runtime. Iterations run sequentially, so
    /// sharing one embedded store cannot hit the concurrent key-value conflict that forces
    /// per-test isolation; distinct scopes still give each iteration its own authority rows.
    fn iteration(&self, label: &str) -> ModelLaneStore {
        self.store_for(exact_scope(label))
    }

    /// Builds a store for `scope` after registering its exact five-field attribution as an
    /// ACTIVE authenticated context, so a denial proves ResourceScope isolation rather than
    /// an unregistered lifecycle.
    fn store_for(&self, scope: ResourceScope) -> ModelLaneStore {
        register_active_context(&self.lifecycle, &scope);
        ModelLaneStore::new_scoped_with_lifecycle(
            self.storage.clone(),
            scope,
            self.lifecycle.clone(),
        )
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated.cleanup().await.expect("clean MT-004 scope");
    }
}

struct Seeded {
    proposal_version: i64,
}

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let mut harness = Harness::create("promotion-positive").await;
    let seeded = seed_authority(&harness.store, "promotion-positive", true).await;
    let decision = sample_decision(
        "promotion-positive",
        "decision-approved",
        "idem-approved",
        seeded.proposal_version,
    );
    let stored = harness
        .store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record canonical PromotionGate decision");
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
        ]
    );
    assert!(stored.event_ledger_seq > 0);
    assert_eq!(
        harness
            .store
            .record_promotion_decision(decision)
            .await
            .expect("identical decision retry"),
        stored
    );
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-positive")
            .await
            .expect("replay decisions"),
        vec![stored.clone()]
    );

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close before restart");
    harness.isolated.reopen().await.expect("reopen same scope");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace/database");
    let reopened = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        harness.scope.clone(),
        harness.lifecycle.clone(),
    );
    assert_eq!(
        reopened
            .replay_promotion_decisions("run-mt004-promotion-positive")
            .await
            .expect("decision survives restart"),
        vec![stored]
    );
    harness.store = reopened;
    harness.storage = storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let harness = Harness::create("promotion-denials").await;
    let seeded = seed_authority(&harness.store, "promotion-denials", true).await;

    let mut stale = sample_decision(
        "promotion-denials",
        "decision-stale",
        "idem-stale",
        seeded.proposal_version,
    );
    stale.base_snapshot_ref = "snapshot://stale".into();
    let stale = harness
        .store
        .record_promotion_decision(stale)
        .await
        .expect("stale base is a durable denial");
    assert_eq!(stale.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        stale.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );

    let mut schema = sample_decision(
        "promotion-denials",
        "decision-schema",
        "idem-schema",
        seeded.proposal_version,
    );
    schema.schema_id = "hsk.model_lane_message@999".into();
    let schema = harness
        .store
        .record_promotion_decision(schema)
        .await
        .expect("schema mismatch is a durable denial");
    assert_eq!(
        schema.denial_reason,
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    );

    let mut direct = sample_decision(
        "promotion-denials",
        "decision-direct",
        "idem-direct",
        seeded.proposal_version,
    );
    direct.direct_authority_mutation_attempt_ref = Some("mutation://forbidden".into());
    let direct = harness
        .store
        .record_promotion_decision(direct)
        .await
        .expect("direct mutation is a durable denial");
    assert_eq!(
        direct.denial_reason,
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    );
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-denials")
            .await
            .expect("replay durable denials")
            .len(),
        3
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let harness = Harness::create("promotion-order").await;
    let seeded = seed_authority(&harness.store, "promotion-order", true).await;
    let first = harness
        .store
        .record_promotion_decision(sample_decision(
            "promotion-order",
            "decision-order-a",
            "idem-order-a",
            seeded.proposal_version,
        ))
        .await
        .expect("first canonical decision");
    let mut reordered = sample_decision(
        "promotion-order",
        "decision-order-b",
        "idem-order-b",
        seeded.proposal_version,
    );
    reordered.input_refs.reverse();
    let second = harness
        .store
        .record_promotion_decision(reordered)
        .await
        .expect("reordered canonical decision");
    assert_eq!(first.canonical_input_refs, second.canonical_input_refs);
    assert_eq!(first.canonical_hash_basis, second.canonical_hash_basis);
    assert_eq!(
        first.canonical_decision_hash,
        second.canonical_decision_hash
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources() {
    let harness = Harness::create("promotion-scope").await;
    let seeded = seed_authority(&harness.store, "promotion-scope", true).await;
    let owner = harness
        .store
        .record_promotion_decision(sample_decision(
            "promotion-scope",
            "decision-owner",
            "idem-owner",
            seeded.proposal_version,
        ))
        .await
        .expect("owner decision");
    assert_eq!(owner.outcome, ModelLanePromotionOutcome::Approved);

    for (index, foreign_scope) in one_field_mismatches(&harness.scope).into_iter().enumerate() {
        let foreign = harness.store_for(foreign_scope);
        seed_authority(&foreign, "promotion-scope", false).await;
        let foreign_decision = foreign
            .record_promotion_decision(sample_decision(
                "promotion-scope",
                &format!("decision-foreign-{index}"),
                &format!("idem-foreign-{index}"),
                seeded.proposal_version,
            ))
            .await
            .expect("foreign mixed-source decision is durably denied");
        assert_eq!(foreign_decision.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(
            foreign_decision.denial_reason,
            Some(ModelLanePromotionDenialReason::InputRefMismatch)
        );
    }
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-scope")
            .await
            .expect("owner decision unchanged"),
        vec![owner]
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn promotion_projection_and_event_receipt_tamper_fail_every_consumer_closed() {
    let harness = Harness::create("promotion-tamper").await;
    for (index, corruption) in [
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
        ModelLaneAuthorityTestCorruption::ProjectionScope,
        ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        ModelLaneAuthorityTestCorruption::ReceiptScope,
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("promotion-tamper-{index}");
        let store = harness.iteration(&label);
        let seeded = seed_authority(&store, &label, true).await;
        let decision_id = format!("decision-tamper-{index}");
        let decision = sample_decision(
            &label,
            &decision_id,
            &format!("idem-tamper-{index}"),
            seeded.proposal_version,
        );
        store
            .record_promotion_decision(decision.clone())
            .await
            .expect("seed canonical promotion decision");
        store
            .test_corrupt_scoped_authority("promotion_decision", &decision_id, corruption)
            .await
            .expect("apply enumerated exact-scope corruption");

        let run_id = format!("run-mt004-{label}");
        assert!(store.replay_promotion_decisions(&run_id).await.is_err());
        assert!(store.replay_run(&run_id).await.is_err());
        assert!(store.navigation_by_run(&run_id).await.is_err());
        assert!(store.record_promotion_decision(decision).await.is_err());
        assert!(store
            .record_message(sample_promoted_message(&label, &decision_id, index))
            .await
            .is_err());
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn navigation_rejects_tampered_run_lane_and_message_origins_before_redirect() {
    let harness = Harness::create("origin-matrix").await;
    for (index, origin) in ["run", "lane", "message"].into_iter().enumerate() {
        let label = format!("origin-{origin}-{index}");
        let store = harness.iteration(&label);
        seed_authority(&store, &label, true).await;
        let run_id = format!("run-mt004-{label}");
        let lane_id = format!("lane-mt004-{label}");
        let message_id = format!("message-mt004-{label}-proposal");
        let aggregate_id = match origin {
            "run" => run_id.as_str(),
            "lane" => lane_id.as_str(),
            "message" => message_id.as_str(),
            _ => unreachable!(),
        };
        store
            .test_corrupt_scoped_authority(
                origin,
                aggregate_id,
                ModelLaneAuthorityTestCorruption::ProjectionScope,
            )
            .await
            .expect("retarget typed origin projection outside its receipt scope");

        let denied = match origin {
            "run" => store.navigation_by_run(&run_id).await.is_err(),
            "lane" => store.navigation_by_lane(&lane_id).await.is_err(),
            "message" => store.navigation_by_message(&message_id).await.is_err(),
            _ => unreachable!(),
        };
        assert!(denied, "{origin} origin must validate before run redirect");
    }
    harness.cleanup().await;
}

/// Acceptance row 1 (all six coordinator routing policies reach a durable, EventLedger-backed
/// promotion decision) plus the two negative-proof classes the acceptance rows name that no other
/// case covers: mismatched expected aggregate version, and a duplicate idempotency key that
/// carries a different canonical decision.
#[tokio::test]
async fn promotion_covers_every_routing_policy_and_denies_version_and_idempotency_conflicts() {
    let harness = Harness::create("policy-matrix").await;
    let seeded = seed_authority(&harness.store, "policy-matrix", true).await;

    for (index, policy) in ModelLaneRoutingPolicy::all().iter().copied().enumerate() {
        let mut decision = sample_decision(
            "policy-matrix",
            &format!("decision-policy-{index}"),
            &format!("idem-policy-{index}"),
            seeded.proposal_version,
        );
        decision.routing_policy = policy;
        // Each policy graph gates its stages on a different authority: the cloud stages need a
        // consent receipt, the validator lane needs a validator ref, the operator lane needs an
        // operator ref. Supply all three so the proof exercises the graph, not a missing field.
        decision.validator_authority_ref = Some("validator://mt004/policy-matrix".into());
        decision.diagnostic_payload = json!({
            "flight_recorder": "kernel_event_ledger",
            "operator_authority_ref": "operator://mt004/policy-matrix",
            "cloud_consent_receipt_ref": "consent://mt004/policy-matrix"
        });
        let stored = harness
            .store
            .record_promotion_decision(decision)
            .await
            .unwrap_or_else(|error| panic!("{policy:?} promotion decision: {error}"));
        assert_eq!(
            stored.outcome,
            ModelLanePromotionOutcome::Approved,
            "{policy:?} must reach an approved promotion"
        );
        assert_eq!(stored.inner.routing_policy, policy);
        assert_eq!(stored.final_state, ModelLanePromotionState::Executed);
        assert!(
            stored.event_ledger_seq > 0,
            "{policy:?} decision must be EventLedger-backed"
        );
    }
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-policy-matrix")
            .await
            .expect("replay every policy decision")
            .len(),
        ModelLaneRoutingPolicy::all().len()
    );

    let version = harness
        .store
        .record_promotion_decision(sample_decision(
            "policy-matrix",
            "decision-version",
            "idem-version",
            seeded.proposal_version + 41,
        ))
        .await
        .expect("aggregate version mismatch is a durable denial");
    assert_eq!(version.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        version.denial_reason,
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    );

    // `idem-policy-0` already belongs to the LocalFirst decision recorded above. This reuse
    // carries a different canonical decision under a policy whose authority gate is satisfied,
    // so the conflict is proven on idempotency, not on input validation.
    let conflict = sample_decision(
        "policy-matrix",
        "decision-conflict",
        "idem-policy-0",
        seeded.proposal_version,
    );
    assert!(
        harness
            .store
            .record_promotion_decision(conflict)
            .await
            .is_err(),
        "a reused idempotency key carrying a different canonical decision must conflict"
    );
    harness.cleanup().await;
}

/// Ports the fenced `model_lane_promotion_pg_tests` advisory-input authority matrix
/// (`model_lane_promotion_durably_denies_tampered_advisory_eventledger_input` and the advisory
/// scope arm of `model_lane_exact_scope_authority_rejects_message_run_lane_and_navigation_tamper`)
/// onto the embedded substrate. A promotion decision may never be Approved from an advisory input
/// whose durable projection or canonical EventLedger receipt has been tampered with.
#[tokio::test]
async fn promotion_denies_tampered_advisory_input_on_projection_and_receipt_authority() {
    let harness = Harness::create("advisory-tamper").await;
    for (index, corruption) in [
        ModelLaneAuthorityTestCorruption::ProjectionScope,
        ModelLaneAuthorityTestCorruption::IncompleteAttribution,
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
        ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        ModelLaneAuthorityTestCorruption::ReceiptScope,
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("advisory-tamper-{index}");
        let store = harness.iteration(&label);
        let seeded = seed_authority(&store, &label, true).await;
        let proposal_id = format!("message-mt004-{label}-proposal");
        store
            .test_corrupt_scoped_authority("message", &proposal_id, corruption)
            .await
            .expect("tamper the advisory input this decision will name");

        let run_id = format!("run-mt004-{label}");
        // Two distinct authority failures need two distinct expectations. A corruption that
        // rewrites the row's own scope removes it from this reader's exact scope, so replay
        // legitimately succeeds while no longer returning the message. A corruption that breaks
        // the projection/receipt link leaves the row in scope, so replay MUST fail closed.
        let replay = store.replay_run(&run_id).await;
        match corruption {
            ModelLaneAuthorityTestCorruption::ProjectionScope
            | ModelLaneAuthorityTestCorruption::IncompleteAttribution => {
                let replay = replay.unwrap_or_else(|error| {
                    panic!("{corruption:?}: scope-removed message must not break replay: {error}")
                });
                assert!(
                    !replay
                        .messages
                        .iter()
                        .any(|message| message.message_id == proposal_id),
                    "{corruption:?}: replay returned a message outside this exact scope"
                );
            }
            _ => assert!(
                replay.is_err(),
                "{corruption:?}: run replay accepted a broken advisory receipt link"
            ),
        }

        let decision = store
            .record_promotion_decision(sample_decision(
                &label,
                &format!("decision-advisory-{index}"),
                &format!("idem-advisory-{index}"),
                seeded.proposal_version,
            ))
            .await;
        match decision {
            Ok(record) => {
                assert_eq!(
                    record.outcome,
                    ModelLanePromotionOutcome::Denied,
                    "{corruption:?}: tampered advisory input produced an approved promotion"
                );
                assert_eq!(
                    record.denial_reason,
                    Some(ModelLanePromotionDenialReason::InputRefMismatch),
                    "{corruption:?}: tampered advisory input must deny on input-ref authority"
                );
            }
            Err(_) => {
                // A fail-closed error is also an acceptable denial of authority, provided no
                // approved decision became durable.
                assert!(store
                    .replay_promotion_decisions(&run_id)
                    .await
                    .map(|records| records
                        .iter()
                        .all(|record| record.outcome != ModelLanePromotionOutcome::Approved))
                    .unwrap_or(true));
            }
        }
    }
    harness.cleanup().await;
}

/// Ports the fenced pg
/// `model_lane_promotion_rejects_message_projection_and_scope_tamper_on_replay_and_retry`:
/// once a promoted message is durable, tampering its projection or canonical receipt must fail
/// both run replay and the idempotent message retry.
#[tokio::test]
async fn promoted_message_projection_and_receipt_tamper_deny_replay_and_retry() {
    let harness = Harness::create("promoted-tamper").await;
    for (index, corruption) in [
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
        ModelLaneAuthorityTestCorruption::ProjectionScope,
        ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        ModelLaneAuthorityTestCorruption::ReceiptScope,
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("promoted-tamper-{index}");
        let store = harness.iteration(&label);
        let seeded = seed_authority(&store, &label, true).await;
        let decision_id = format!("decision-promoted-{index}");
        store
            .record_promotion_decision(sample_decision(
                &label,
                &decision_id,
                &format!("idem-promoted-{index}"),
                seeded.proposal_version,
            ))
            .await
            .expect("seed canonical promotion decision");
        let promoted_input = sample_promoted_message(&label, &decision_id, index);
        let promoted = store
            .record_message(promoted_input.clone())
            .await
            .expect("record promoted message before tamper");
        store
            .test_corrupt_scoped_authority("message", &promoted.message_id, corruption)
            .await
            .expect("tamper the durable promoted message");

        let run_id = format!("run-mt004-{label}");
        // Same split as the advisory matrix: a rewritten row scope removes the promoted message
        // from this reader entirely, while a broken projection/receipt link must fail replay closed.
        let replay = store.replay_run(&run_id).await;
        if corruption == ModelLaneAuthorityTestCorruption::ProjectionScope {
            let replay = replay.unwrap_or_else(|error| {
                panic!("{corruption:?}: scope-removed promoted message broke replay: {error}")
            });
            assert!(
                !replay
                    .messages
                    .iter()
                    .any(|message| message.message_id == promoted.message_id),
                "{corruption:?}: replay returned a promoted message outside this exact scope"
            );
        } else {
            assert!(
                replay.is_err(),
                "{corruption:?}: run replay accepted a tampered promoted message"
            );
        }
        // In every class the idempotent retry must refuse: it may neither silently re-create the
        // tampered promoted message nor return the corrupted row as a settled success.
        assert!(
            store.record_message(promoted_input).await.is_err(),
            "{corruption:?}: idempotent retry accepted a tampered promoted message"
        );
    }
    harness.cleanup().await;
}

/// Ports the substantive invariant of the fenced pg forged-projection arm of
/// `model_lane_exact_scope_authority_rejects_message_run_lane_and_navigation_tamper`: run/lane
/// authority held only in a foreign exact scope can never manufacture message-append authority
/// for the owner scope, and the denial must be non-disclosing and leave both the projection and
/// the EventLedger row counts unchanged.
#[tokio::test]
async fn foreign_scope_run_and_lane_cannot_manufacture_message_append_authority() {
    let harness = Harness::create("foreign-append").await;
    let foreign_scope = one_field_mismatches(&harness.scope)
        .into_iter()
        .next()
        .expect("one foreign owner scope");
    let foreign_owner = foreign_scope.owner_account_id.to_string();
    let foreign = harness.store_for(foreign_scope);
    seed_authority(&foreign, "foreign-append", false).await;

    let inspector = harness.storage.test_inspector();
    let authority_rows = table_count(&inspector, "model_lane_authority").await;
    let ledger_rows = table_count(&inspector, "kernel_event_ledger").await;

    // The owner store never seeded run/lane authority for this label; only the foreign scope did.
    let error = harness
        .store
        .record_message(sample_message("foreign-append", "proposal"))
        .await
        .expect_err("foreign-scope run/lane must not authorize an owner-scope message append");
    let rendered = error.to_string();
    assert!(
        !rendered.contains(&foreign_owner),
        "denial must not disclose the foreign owner account: {rendered}"
    );
    assert_eq!(
        table_count(&inspector, "model_lane_authority").await,
        authority_rows,
        "denied append mutated model-lane authority"
    );
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        ledger_rows,
        "denied append reached the canonical EventLedger"
    );
    drop(inspector);
    harness.cleanup().await;
}

async fn table_count(inspector: &SurrealTestInspector, table: &str) -> u64 {
    let table = inspector
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("select {table}: {error}"));
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("count {}: {error}", table.name()))
}

async fn seed_authority(store: &ModelLaneStore, label: &str, include_messages: bool) -> Seeded {
    store
        .record_run(sample_run(label))
        .await
        .expect("seed promotion run");
    store
        .record_lane(sample_lane(label))
        .await
        .expect("seed promotion lane");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding(label))
        .await
        .expect("seed promoted artifact authority");
    if !include_messages {
        return Seeded {
            proposal_version: 1,
        };
    }
    let proposal = store
        .record_message(sample_message(label, "proposal"))
        .await
        .expect("seed proposal advisory");
    store
        .record_message(sample_message(label, "critique"))
        .await
        .expect("seed critique advisory");
    Seeded {
        proposal_version: proposal.event_stream_version,
    }
}

fn sample_decision(
    label: &str,
    decision_id: &str,
    idempotency_key: &str,
    expected_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![
            format!("span-message-mt004-{label}-proposal"),
            format!("span-message-mt004-{label}-critique"),
        ],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            format!("model-lane-message://message-mt004-{label}-critique"),
            format!("model-lane-message://message-mt004-{label}-proposal"),
        ],
        selected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-proposal"
        )],
        rejected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-critique"
        )],
        validator_authority_ref: None,
        operator_authority_ref: Some(format!("operator://mt004/{label}")),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: format!("message-mt004-{label}-proposal"),
        expected_event_ledger_version: expected_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: format!("promotion-gate://mt004/{label}"),
        promotion_receipt_ref: Some(format!("promotion-receipt://mt004/{label}")),
        promoted_artifact_ref: Some(format!("artifact://mt004/{label}/promoted")),
        promoted_artifact_sha256: Some(promoted_artifact_hash(label)),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("0020-{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:02:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "kernel_event_ledger",
            "operator_authority_ref": format!("operator://mt004/{label}")
        }),
    }
}

fn sample_artifact_binding(label: &str) -> NewModelLaneContextBundleArtifactBinding {
    let payload = promoted_artifact_payload(label);
    let hash = promoted_artifact_hash(label);
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        artifact_ref: artifact_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_promoted_artifact".into(),
        artifact_manifest_ref: format!("artifact-manifest://mt004/{label}"),
        artifact_payload_ref: artifact_ref,
        payload_json: payload,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("artifact-binding-mt004-{label}"),
        created_at_utc: "2026-09-02T00:01:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn promoted_artifact_payload(label: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_promoted_artifact@1",
        "artifact_version": "1",
        "body": format!("deterministic promoted artifact {label}")
    })
}

fn promoted_artifact_hash(label: &str) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&promoted_artifact_payload(label)).expect("serialize artifact"),
    ))
}

fn sample_run(label: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        run_span_id: format!("span-run-mt004-{label}"),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane.as_str().into(),
        context_bundle_id: format!("context-mt004-{label}"),
        lane_ids: vec![format!("lane-mt004-{label}")],
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        artifact_namespace: format!("artifact://mt004/{label}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("run-mt004-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        locus_binding: Some(sample_locus(label)),
        memory_pack_ref: format!("memory-pack://mt004/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://mt004/{label}"),
        selected_model_id: Some("model://local/mt004".into()),
        candidate_model_ids: vec!["model://local/mt004".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane(label: &str) -> NewModelLane {
    let session = format!("session-mt004-{label}");
    let model_session = format!("model-session-mt004-{label}");
    NewModelLane {
        lane_id: format!("lane-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        lane_span_id: format!("span-lane-mt004-{label}"),
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        kind: ModelLaneKind::LocalModel,
        role: "proposal-author".into(),
        backend: "embedded-model-runtime".into(),
        model_id: Some("model://local/mt004".into()),
        session_id: session.clone(),
        model_session_id: model_session.clone(),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt004/context".into()],
        effective_capability_snapshot_ref: Some("capability://mt004/snapshot".into()),
        capability_negotiation_ref: Some("capability://mt004/negotiation".into()),
        provider_feature_profile_ref: Some("provider://mt004/local".into()),
        requested_execution_policy_ref: Some("execution://mt004/requested".into()),
        effective_execution_policy_ref: Some("execution://mt004/effective".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://mt004/{label}")),
        reclaim_policy_ref: Some("reclaim://mt004".into()),
        terminal_status_mapping_ref: Some("terminal://mt004".into()),
        process_ownership_ref: Some(format!("process://mt004/{label}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop://mt004".into()),
        last_runtime_status_ref: Some("runtime://mt004/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus_for(label, &session, &model_session)),
    }
}

fn sample_message(label: &str, kind: &str) -> NewModelLaneMessage {
    let message_id = format!("message-mt004-{label}-{kind}");
    NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![format!("trace-mt004-{label}")],
        from_lane_id: format!("lane-mt004-{label}"),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-mt004-{label}"),
            correlation_id: format!("correlation-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://mt004/{label}/{kind}"),
        payload_sha256: if kind == "proposal" {
            "b".repeat(64)
        } else {
            "c".repeat(64)
        },
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        summary: format!("{kind} advisory"),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus(label)),
        idempotency_key: format!("message-mt004-{label}-{kind}"),
        replay_order_key: format!("0010-{kind}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:01:30Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn sample_promoted_message(label: &str, decision_id: &str, index: usize) -> NewModelLaneMessage {
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    let mut message = sample_message(label, &format!("promoted-{index}"));
    message.kind = ModelLaneMessageKind::PromotionRequest;
    message.authority = ModelLaneAuthority::Promoted;
    message.payload_ref = artifact_ref.clone();
    message.payload_sha256 = promoted_artifact_hash(label);
    message.promotion_decision_id = Some(decision_id.into());
    message.promotion_gate_ref = Some(format!("promotion-gate://mt004/{label}"));
    message.promotion_receipt_ref = Some(format!("promotion-receipt://mt004/{label}"));
    message.validator_verdict_ref = Some(format!("validator://mt004/{label}"));
    message.operator_decision_ref = Some(format!("operator://mt004/{label}"));
    message.promoted_artifact_ref = Some(artifact_ref);
    message.promoted_artifact_sha256 = Some(promoted_artifact_hash(label));
    message.promoted_artifact_version = Some("1".into());
    message
}

fn sample_locus(label: &str) -> ModelLaneLocusBinding {
    sample_locus_for(
        label,
        &format!("session-mt004-{label}"),
        &format!("model-session-mt004-{label}"),
    )
}

fn sample_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding_ref: format!("locus://wp1/mt004/{label}"),
    }
}

/// Registers the exact five-field attribution of `scope` as an ACTIVE authenticated
/// resource-access context. Production composes this registry from the authentication/session
/// authority; `ModelLaneStore::new_scoped` is intentionally fail-closed without it.
fn register_active_context(lifecycle: &ResourceAccessLifecycleRegistry, scope: &ResourceScope) {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .expect("proof scope must carry all five exact attribution fields");
    lifecycle
        .register_active(exact)
        .expect("register active authenticated resource-access context");
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt004-{label}")).expect("nonblank workspace"),
        )
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let workspace = scope.workspace.clone().expect("exact workspace");
    let session = scope.authenticated_session.expect("exact session");
    let access_space = scope.access_space.expect("exact access space");
    vec![
        ResourceScope::new(OwnerAccountId::mint(), scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(
                WorkspaceScopeRef::new("workspace-mt004-foreign").expect("nonblank workspace"),
            ),
    ]
}
