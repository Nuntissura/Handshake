//! Focused embedded-Surreal proof for routing execution durability.
//!
//! These tests exercise the public ModelLane persistence facade. The higher-level
//! executor's lease/recovery methods are crate-private, so their control flow is
//! covered by unit tests in that module; this target attacks their durable CAS,
//! fencing, cancellation, scope, restart, and EventLedger boundary.

mod surreal_test_store_support;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::surreal::{RowFilter, ScalarValue, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneLocusBinding, ModelLaneRecoveryState, ModelLaneRunRecord, ModelLaneStore,
    NewModelLaneRun,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use handshake_core::swarm_orchestration::routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingDispatchTarget, ModelLaneRoutingStageLaunchPlan,
};
use handshake_core::swarm_orchestration::routing_execution::{
    ModelLaneRoutingExecutionState, ModelLaneRoutingExecutionStatus, ModelLaneRoutingStageClaim,
    ModelLaneRoutingStageState, ModelLaneRoutingStageStateKind,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surreal_test_store_support::EmbeddedSurrealTestScope;
use uuid::Uuid;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-017";
const BOARD_ID: &str = "task-board://wp-1";
const STAGE_ID: &str = "local-attempt";
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    store: ModelLaneStore,
    run: ModelLaneRunRecord,
    execution_id: String,
}

impl Fixture {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate isolated embedded Surreal scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production Surreal storage wrapper");
        let scope = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
        let run = store
            .record_run(sample_run(label))
            .await
            .expect("record exact-scope routing run authority");
        Self {
            isolated,
            storage,
            scope,
            store,
            execution_id: format!("routing-{label}-{}", Uuid::now_v7().simple()),
            run,
        }
    }

    fn initial_projection(&self) -> (ModelLaneRoutingExecutionState, ModelLaneRoutingStageState) {
        projection(
            &self.run,
            &self.execution_id,
            1,
            ModelLaneRoutingStageStateKind::Scheduled,
            None,
            None,
            None,
            ModelLaneRoutingExecutionStatus::Running,
            None,
        )
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("remove only the allocator-owned Surreal scope");
    }
}

#[tokio::test]
async fn initial_creation_retry_extra_event_and_restart_share_one_event_authority() {
    let mut fixture = Fixture::create("initial-restart").await;
    let namespace = fixture.isolated.namespace().to_owned();
    let database = fixture.isolated.database().to_owned();
    let (next, attempt) = fixture.initial_projection();
    let mut events = required_events(&next, &attempt, "initial", "pending");
    let context_handoff = extra_event(
        &next,
        "context-handoff-a",
        json!({
            "kind": "context_handoff",
            "from": "local-attempt",
            "to": "coordinator"
        }),
    );
    let context_idempotency = context_handoff.idempotency_key.clone();
    events.push(context_handoff);

    let committed = fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            next.clone(),
            attempt.clone(),
            "pending",
            events.clone(),
        )
        .await
        .expect("atomically create routing projection and canonical events");
    assert_eq!(committed.revision, 1);
    assert!(!committed.event_ledger_event_id.is_empty());
    assert!(committed.event_ledger_seq > 0);
    let committed_attempt = committed.stages.get(STAGE_ID).expect("durable stage");
    assert!(!committed_attempt.event_ledger_event_id.is_empty());
    assert!(committed_attempt.event_ledger_seq > 0);

    let inspector = fixture.storage.test_inspector();
    assert!(
        field_value_exists(
            &inspector,
            "kernel_event_ledger",
            "event_id",
            &committed.event_ledger_event_id,
        )
        .await,
        "execution projection must resolve to its canonical EventLedger row"
    );
    assert!(
        field_value_exists(
            &inspector,
            "kernel_event_ledger",
            "event_id",
            &committed_attempt.event_ledger_event_id,
        )
        .await,
        "attempt projection must resolve to its canonical EventLedger row"
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_execution").await,
        1
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_stage_attempt").await,
        1
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_outbox").await,
        1
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_extra_event_link").await,
        1
    );
    assert!(
        field_value_exists(
            &inspector,
            "kernel_event_ledger",
            "idempotency_key",
            &context_idempotency,
        )
        .await
    );
    let ledger_count = table_count(&inspector, "kernel_event_ledger").await;

    let retried = fixture
        .store
        .commit_routing_execution_atomic(0, None, next, attempt, "pending", events)
        .await
        .expect("identical initial retry returns the existing projection");
    assert_eq!(retried, committed);
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        ledger_count
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_extra_event_link").await,
        1
    );

    drop(inspector);
    drop(fixture.store);
    drop(fixture.storage);
    fixture
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close shared production storage before restart");
    fixture
        .isolated
        .reopen()
        .await
        .expect("reopen the same embedded Surreal scope");
    assert_eq!(fixture.isolated.namespace(), namespace);
    assert_eq!(fixture.isolated.database(), database);
    fixture.storage = fixture
        .isolated
        .activate_storage()
        .await
        .expect("reactivate the same namespace and database");
    fixture.store = ModelLaneStore::new_scoped(fixture.storage.clone(), fixture.scope.clone());
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read restarted routing projection"),
        Some(committed)
    );
    fixture.cleanup().await;
}

#[tokio::test]
async fn stale_cas_event_set_conflict_and_stale_fence_cannot_mutate_cancellation() {
    let mut fixture = Fixture::create("cas-fence-cancel").await;
    let (initial, scheduled) = fixture.initial_projection();
    fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            initial.clone(),
            scheduled,
            "pending",
            required_events(
                &initial,
                initial.stages.get(STAGE_ID).expect("scheduled stage"),
                "initial",
                "pending",
            ),
        )
        .await
        .expect("seed initial routing revision");

    let lease_expiry = now_ms() + 600_000;
    let (claimed, claimed_attempt) = projection(
        &fixture.run,
        &fixture.execution_id,
        2,
        ModelLaneRoutingStageStateKind::Claimed,
        Some("worker-a"),
        Some("fence-a"),
        Some(lease_expiry),
        ModelLaneRoutingExecutionStatus::Running,
        None,
    );
    let claim_events = required_events(&claimed, &claimed_attempt, "claim", "claimed");
    let durable_claimed = fixture
        .store
        .commit_routing_execution_atomic(
            1,
            None,
            claimed.clone(),
            claimed_attempt.clone(),
            "claimed",
            claim_events.clone(),
        )
        .await
        .expect("claim revision wins CAS");

    let mut competing = claimed.clone();
    competing
        .stages
        .get_mut(STAGE_ID)
        .expect("competing stage")
        .detail = Some("losing concurrent writer".into());
    let competing_attempt = competing
        .stages
        .get(STAGE_ID)
        .expect("competing stage")
        .clone();
    let stale_error = fixture
        .store
        .commit_routing_execution_atomic(
            1,
            None,
            competing.clone(),
            competing_attempt.clone(),
            "claimed",
            required_events(&competing, &competing_attempt, "competing", "claimed"),
        )
        .await
        .expect_err("different writer at the committed revision must lose");
    assert!(
        stale_error.to_string().contains("conflict")
            || stale_error.to_string().contains("retry")
            || stale_error.to_string().contains("revision"),
        "CAS denial must identify the conflict boundary: {stale_error}"
    );

    let mut extra_conflict_events = claim_events.clone();
    extra_conflict_events.push(extra_event(
        &claimed,
        "unexpected-retry-event",
        json!({"kind": "different_retry_set"}),
    ));
    let event_set_error = fixture
        .store
        .commit_routing_execution_atomic(
            1,
            None,
            claimed.clone(),
            claimed_attempt.clone(),
            "claimed",
            extra_conflict_events,
        )
        .await
        .expect_err("retry with a widened event set must fail");
    assert!(
        event_set_error.to_string().contains("event")
            || event_set_error.to_string().contains("retry")
    );

    let mut foreign_claim = stage_claim(
        &fixture.execution_id,
        &claimed_attempt,
        "worker-a",
        "fence-a",
        lease_expiry,
    );
    foreign_claim.expected_run_id = "same-scope-foreign-run".into();
    let (cancelled, cancelled_attempt) = projection(
        &fixture.run,
        &fixture.execution_id,
        3,
        ModelLaneRoutingStageStateKind::Cancelled,
        None,
        None,
        None,
        ModelLaneRoutingExecutionStatus::Cancelled,
        Some("operator cancellation"),
    );
    let cancel_events = required_events(&cancelled, &cancelled_attempt, "cancel", "acked");
    fixture
        .store
        .commit_routing_execution_atomic(
            2,
            Some(&foreign_claim),
            cancelled.clone(),
            cancelled_attempt.clone(),
            "acked",
            cancel_events.clone(),
        )
        .await
        .expect_err("same-scope foreign claim receipt must not authorize cancellation");

    let stale_claim = stage_claim(
        &fixture.execution_id,
        &claimed_attempt,
        "worker-a",
        "wrong-fence",
        lease_expiry,
    );
    fixture
        .store
        .commit_routing_execution_atomic(
            2,
            Some(&stale_claim),
            cancelled.clone(),
            cancelled_attempt.clone(),
            "acked",
            cancel_events.clone(),
        )
        .await
        .expect_err("stale fencing token must not cancel the execution");
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read after denied stale fence"),
        Some(durable_claimed)
    );

    let live_claim = stage_claim(
        &fixture.execution_id,
        &claimed_attempt,
        "worker-a",
        "fence-a",
        lease_expiry,
    );
    let durable_cancelled = fixture
        .store
        .commit_routing_execution_atomic(
            2,
            Some(&live_claim),
            cancelled,
            cancelled_attempt,
            "acked",
            cancel_events,
        )
        .await
        .expect("matching lease and fence atomically cancel execution");
    assert_eq!(
        durable_cancelled.status,
        ModelLaneRoutingExecutionStatus::Cancelled
    );
    assert_eq!(
        durable_cancelled.cancel_reason.as_deref(),
        Some("operator cancellation")
    );

    drop(fixture.store);
    drop(fixture.storage);
    fixture
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close cancellation store");
    fixture
        .isolated
        .reopen()
        .await
        .expect("reopen cancellation store");
    fixture.storage = fixture
        .isolated
        .activate_storage()
        .await
        .expect("reactivate store");
    fixture.store = ModelLaneStore::new_scoped(fixture.storage.clone(), fixture.scope.clone());
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("recover cancellation after restart"),
        Some(durable_cancelled)
    );
    fixture.cleanup().await;
}

#[tokio::test]
async fn five_scope_mismatches_and_incomplete_scopes_fail_closed_without_rows_or_events() {
    let fixture = Fixture::create("scope-denial").await;
    let (initial, attempt) = fixture.initial_projection();
    let events = required_events(&initial, &attempt, "initial", "pending");
    fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            initial.clone(),
            attempt.clone(),
            "pending",
            events.clone(),
        )
        .await
        .expect("seed exact-scope routing authority");
    let inspector = fixture.storage.test_inspector();
    let routing_rows = table_count(&inspector, "model_lane_routing_execution").await;
    let event_rows = table_count(&inspector, "kernel_event_ledger").await;

    let mut mismatches = Vec::new();
    let mut owner = fixture.scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    mismatches.push(("owner_account_id", owner));
    let mut actor = fixture.scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    mismatches.push(("actor_principal_id", actor));
    let mut session = fixture.scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    mismatches.push(("authenticated_session_id", session));
    let mut access = fixture.scope.clone();
    access.access_space = Some(AccessSpaceRef::mint());
    mismatches.push(("access_space_id", access));
    let mut workspace = fixture.scope.clone();
    workspace.workspace = Some(WorkspaceScopeRef::new("workspace-other").expect("workspace"));
    mismatches.push(("workspace_id", workspace));

    for (dimension, scope) in mismatches {
        let denied = ModelLaneStore::new_scoped(fixture.storage.clone(), scope);
        assert_eq!(
            denied
                .routing_execution_snapshot(&fixture.execution_id)
                .await
                .unwrap_or_else(|error| panic!("{dimension} mismatch read: {error}")),
            None,
            "{dimension} mismatch leaked an execution"
        );
        assert!(
            denied
                .routing_execution_diagnostics_for_run(&fixture.run.run_id)
                .await
                .unwrap_or_else(|error| panic!("{dimension} mismatch diagnostics: {error}"))
                .is_empty(),
            "{dimension} mismatch leaked diagnostic identifiers"
        );
        let _ = denied
            .commit_routing_execution_atomic(
                0,
                None,
                initial.clone(),
                attempt.clone(),
                "pending",
                events.clone(),
            )
            .await
            .expect_err("one-field scope mismatch unexpectedly mutated routing");
    }

    let incomplete = [
        ResourceScope::new(
            fixture.scope.owner_account_id,
            fixture.scope.actor_principal_id,
        )
        .with_access_space(fixture.scope.access_space.expect("access"))
        .with_workspace(fixture.scope.workspace.clone().expect("workspace")),
        ResourceScope::new(
            fixture.scope.owner_account_id,
            fixture.scope.actor_principal_id,
        )
        .with_session(fixture.scope.authenticated_session.expect("session"))
        .with_workspace(fixture.scope.workspace.clone().expect("workspace")),
        ResourceScope::new(
            fixture.scope.owner_account_id,
            fixture.scope.actor_principal_id,
        )
        .with_session(fixture.scope.authenticated_session.expect("session"))
        .with_access_space(fixture.scope.access_space.expect("access")),
    ];
    for scope in incomplete {
        let denied = ModelLaneStore::new_scoped(fixture.storage.clone(), scope);
        let error = denied
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect_err("incomplete read scope must fail before storage access");
        assert!(error.to_string().contains("exact owner"));
        denied
            .commit_routing_execution_atomic(
                0,
                None,
                initial.clone(),
                attempt.clone(),
                "pending",
                events.clone(),
            )
            .await
            .expect_err("incomplete write scope must fail before storage access");
    }

    assert_eq!(
        table_count(&inspector, "model_lane_routing_execution").await,
        routing_rows
    );
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        event_rows
    );
    drop(inspector);
    fixture.cleanup().await;
}

#[tokio::test]
async fn projection_and_required_event_counterfactuals_fail_without_mutation() {
    let fixture = Fixture::create("event-counterfactuals").await;
    let (initial, attempt) = fixture.initial_projection();
    let valid_events = required_events(&initial, &attempt, "initial", "pending");
    let inspector = fixture.storage.test_inspector();
    let run_event_rows = table_count(&inspector, "kernel_event_ledger").await;

    let mut mismatched_payload = valid_events.clone();
    mismatched_payload[0].payload = json!({"valid_looking_hash_but": "different payload"});
    fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            initial.clone(),
            attempt.clone(),
            "pending",
            mismatched_payload,
        )
        .await
        .expect_err("payload bytes that do not match payload_hash must fail");
    assert_eq!(
        table_count(&inspector, "model_lane_routing_execution").await,
        0
    );
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        run_event_rows
    );

    let committed = fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            initial.clone(),
            attempt.clone(),
            "pending",
            valid_events.clone(),
        )
        .await
        .expect("valid initial commit remains available after rejected payload");
    let committed_event_rows = table_count(&inspector, "kernel_event_ledger").await;

    let mut changed_projection = initial.clone();
    changed_projection.owner_session = "changed-owner-session".into();
    changed_projection.initial_input_sha256 = Some(HASH_A.into());
    let changed_attempt = changed_projection
        .stages
        .get(STAGE_ID)
        .expect("changed retry stage")
        .clone();
    fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            changed_projection.clone(),
            changed_attempt.clone(),
            "pending",
            required_events(&changed_projection, &changed_attempt, "initial", "pending"),
        )
        .await
        .expect_err("initial retry cannot retarget owner session or input digest");

    let mut changed_envelope = valid_events;
    changed_envelope[0].source_component = "tampered-routing-source".into();
    fixture
        .store
        .commit_routing_execution_atomic(0, None, initial, attempt, "pending", changed_envelope)
        .await
        .expect_err("same idempotency and payload hash cannot hide a changed event envelope");
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read after counterfactual retries"),
        Some(committed)
    );
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        committed_event_rows
    );
    drop(inspector);
    fixture.cleanup().await;

    let false_pointer = Fixture::create("false-event-pointer").await;
    let (mut initial, mut attempt) = false_pointer.initial_projection();
    initial.event_ledger_event_id = "fabricated-execution-event".into();
    initial.event_ledger_seq = 777;
    attempt.event_ledger_event_id = "fabricated-attempt-event".into();
    attempt.event_ledger_seq = 778;
    initial.stages.insert(STAGE_ID.into(), attempt.clone());
    let false_inspector = false_pointer.storage.test_inspector();
    let false_event_rows = table_count(&false_inspector, "kernel_event_ledger").await;
    false_pointer
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            initial.clone(),
            attempt.clone(),
            "pending",
            required_events(&initial, &attempt, "false-pointer", "pending"),
        )
        .await
        .expect_err("positive but false projection EventLedger pointers must fail");
    assert_eq!(
        table_count(&false_inspector, "model_lane_routing_execution").await,
        0
    );
    assert_eq!(
        table_count(&false_inspector, "kernel_event_ledger").await,
        false_event_rows
    );
    drop(false_inspector);
    false_pointer.cleanup().await;
}

#[tokio::test]
async fn identical_logical_routing_ids_are_isolated_between_two_exact_scopes() {
    let fixture = Fixture::create("scope-a").await;
    let scope_b = exact_scope("scope-b");
    let store_b = ModelLaneStore::new_scoped(fixture.storage.clone(), scope_b.clone());
    let run_b = store_b
        .record_run(fixture.run.inner.clone())
        .await
        .expect("record identical logical run under second exact scope");

    let (execution_a, attempt_a) = fixture.initial_projection();
    let (execution_b, attempt_b) = projection(
        &run_b,
        &fixture.execution_id,
        1,
        ModelLaneRoutingStageStateKind::Scheduled,
        None,
        None,
        None,
        ModelLaneRoutingExecutionStatus::Running,
        None,
    );
    let stored_a = fixture
        .store
        .commit_routing_execution_atomic(
            0,
            None,
            execution_a.clone(),
            attempt_a.clone(),
            "pending",
            required_events(&execution_a, &attempt_a, "scope-a", "pending"),
        )
        .await
        .expect("commit logical routing identity in first exact scope");
    let stored_b = store_b
        .commit_routing_execution_atomic(
            0,
            None,
            execution_b.clone(),
            attempt_b.clone(),
            "pending",
            required_events(&execution_b, &attempt_b, "scope-b", "pending"),
        )
        .await
        .expect("commit same logical routing identity in second exact scope");

    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read first exact scope"),
        Some(stored_a.clone())
    );
    assert_eq!(
        store_b
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read second exact scope"),
        Some(stored_b.clone())
    );
    assert_ne!(
        stored_a.event_ledger_event_id,
        stored_b.event_ledger_event_id
    );
    let inspector = fixture.storage.test_inspector();
    assert_eq!(
        table_count(&inspector, "model_lane_routing_execution").await,
        2
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_stage_attempt").await,
        2
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_outbox").await,
        2
    );
    drop(inspector);
    drop(store_b);
    fixture.cleanup().await;
}

#[tokio::test]
async fn preexisting_canonical_receipt_without_routing_projection_is_rejected_as_orphan() {
    let fixture = Fixture::create("orphan-receipt").await;
    let (initial, attempt) = fixture.initial_projection();
    let mut events = required_events(&initial, &attempt, "initial", "pending");
    events[0].idempotency_key = fixture.run.event_ledger_event_id.clone();
    let inspector = fixture.storage.test_inspector();
    let event_rows = table_count(&inspector, "kernel_event_ledger").await;

    let error = fixture
        .store
        .commit_routing_execution_atomic(0, None, initial, attempt, "pending", events)
        .await
        .expect_err("orphan canonical receipt must block initial routing creation");
    assert!(
        error.to_string().contains("complete projection")
            || error.to_string().contains("without its complete"),
        "orphan denial must identify incomplete projection authority: {error}"
    );
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read after orphan denial"),
        None
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_execution").await,
        0
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_stage_attempt").await,
        0
    );
    assert_eq!(
        table_count(&inspector, "model_lane_routing_outbox").await,
        0
    );
    assert_eq!(
        table_count(&inspector, "kernel_event_ledger").await,
        event_rows
    );
    drop(inspector);
    fixture.cleanup().await;
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-routing-{label}"))
                .expect("nonblank workspace"),
        )
}

fn sample_run(label: &str) -> NewModelLaneRun {
    let suffix = Uuid::now_v7().simple().to_string();
    let run_id = format!("run-routing-{label}-{suffix}");
    let coordinator_session_id = format!("coordinator-{suffix}");
    let owner_session = format!("owner-session-{suffix}");
    NewModelLaneRun {
        run_id: run_id.clone(),
        trace_id: format!("trace-{suffix}"),
        run_span_id: format!("run-span-{suffix}"),
        coordinator_session_id: coordinator_session_id.clone(),
        routing_policy: "balanced".into(),
        context_bundle_id: format!("context-bundle-{suffix}"),
        lane_ids: vec![format!("lane-{suffix}")],
        event_ledger_stream_id: format!("stream-{suffix}"),
        artifact_namespace: format!("artifact-routing-{suffix}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(BOARD_ID.into()),
        owner_session: owner_session.clone(),
        idempotency_key: format!("run-routing-idem-{suffix}"),
        replay_order_key: format!("routing-replay-{suffix}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("recovery://routing/restart".into()),
        locus_binding: Some(ModelLaneLocusBinding {
            work_packet_id: WP_ID.into(),
            micro_task_id: MT_ID.into(),
            task_board_id: Some(BOARD_ID.into()),
            coordinator_session_id,
            session_id: format!("session-{suffix}"),
            model_session_id: format!("model-session-{suffix}"),
            owner_session,
            locus_binding_ref: format!("locus://routing/{suffix}"),
        }),
        memory_pack_ref: format!("memory-pack://routing/{suffix}"),
        memory_pack_hash: HASH_A.into(),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://routing/{suffix}"),
        selected_model_id: Some("local-routing-model".into()),
        candidate_model_ids: vec!["local-routing-model".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn projection(
    run: &ModelLaneRunRecord,
    execution_id: &str,
    revision: u64,
    stage_kind: ModelLaneRoutingStageStateKind,
    lease_owner: Option<&str>,
    fencing_token: Option<&str>,
    lease_expires_at_unix_ms: Option<u64>,
    status: ModelLaneRoutingExecutionStatus,
    cancel_reason: Option<&str>,
) -> (ModelLaneRoutingExecutionState, ModelLaneRoutingStageState) {
    let canonical_graph = json!({
        "policy": "balanced",
        "stages": [{"stage_id": STAGE_ID, "target": "local_model"}]
    });
    let canonical_launch_plan = vec![ModelLaneRoutingStageLaunchPlan {
        stage_id: STAGE_ID.into(),
        dispatch_target: ModelLaneRoutingDispatchTarget::LocalModel,
        lane_id: Some(run.lane_ids[0].clone()),
        model_id: Some("local-routing-model".into()),
        provider: Some(handshake_core::model_runtime::ProviderKind::Local),
    }];
    let canonical_graph_sha256 = canonical_sha256(&canonical_graph);
    let canonical_launch_plan_sha256 = canonical_sha256(
        &serde_json::to_value(&canonical_launch_plan).expect("serialize canonical launch plan"),
    );
    let attempt = ModelLaneRoutingStageState {
        stage_id: STAGE_ID.into(),
        state: stage_kind,
        attempt: 1,
        dispatch_target: ModelLaneRoutingDispatchTarget::LocalModel,
        expected_run_id: run.run_id.clone(),
        expected_lane_id: run.lane_ids[0].clone(),
        expected_model_id: "local-routing-model".into(),
        expected_provider: Some(handshake_core::model_runtime::ProviderKind::Local),
        instance_id: None,
        lane_id: Some(run.lane_ids[0].clone()),
        input_refs: vec![run.context_bundle_id.clone()],
        output_ref: None,
        output_message_ref: None,
        authority_request_message_ref: None,
        output_sha256: None,
        output_payload: None,
        authority_ref: None,
        lease_owner: lease_owner.map(str::to_owned),
        fencing_token: fencing_token.map(str::to_owned),
        lease_expires_at_unix_ms,
        detail: Some(format!("routing revision {revision}")),
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        updated_at_unix_ms: now_ms(),
    };
    let stages = BTreeMap::from([(STAGE_ID.to_owned(), attempt.clone())]);
    (
        ModelLaneRoutingExecutionState {
            schema_id: "hsk.model_lane_routing_execution@5".into(),
            execution_id: execution_id.into(),
            run_id: run.run_id.clone(),
            selecting_decision_id: format!("decision-{execution_id}"),
            selecting_decision_event_id: run.event_ledger_event_id.clone(),
            selecting_decision_event_seq: run.event_ledger_seq,
            trace_id: run.trace_id.clone(),
            run_span_id: run.run_span_id.clone(),
            coordinator_session_id: run.coordinator_session_id.clone(),
            locus_ref: run
                .locus_binding
                .as_ref()
                .expect("run locus")
                .locus_binding_ref
                .clone(),
            work_packet_id: WP_ID.into(),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: BOARD_ID.into(),
            owner_session: run.owner_session.clone(),
            canonical_graph,
            canonical_graph_sha256,
            canonical_launch_plan,
            canonical_launch_plan_sha256,
            authority: ModelLaneRoutingAuthority::default(),
            initial_input_ref: Some(run.context_bundle_id.clone()),
            initial_input_sha256: Some(HASH_B.into()),
            status,
            failure_reason: None,
            cancel_reason: cancel_reason.map(str::to_owned),
            revision,
            stages,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
        },
        attempt,
    )
}

fn required_events(
    execution: &ModelLaneRoutingExecutionState,
    attempt: &ModelLaneRoutingStageState,
    action: &str,
    outbox_status: &str,
) -> Vec<NewKernelEvent> {
    let event_type = match attempt.state {
        ModelLaneRoutingStageStateKind::Succeeded | ModelLaneRoutingStageStateKind::Joined => {
            KernelEventType::ModelResponseRecorded
        }
        ModelLaneRoutingStageStateKind::Failed
        | ModelLaneRoutingStageStateKind::Cancelled
        | ModelLaneRoutingStageStateKind::Compensated => KernelEventType::SessionFailed,
        _ => KernelEventType::ModelAdapterInvoked,
    };
    let attempt_id = format!(
        "{}:{}:{}",
        execution.execution_id, attempt.stage_id, attempt.attempt
    );
    let command_id = format!(
        "routing-command:{}:{}:{}",
        execution.execution_id, attempt.stage_id, attempt.attempt
    );
    vec![
        routing_event(
            execution,
            "model_lane_routing_execution",
            &execution.execution_id,
            format!(
                "routing:{action}:execution:{}:{}",
                execution.execution_id, execution.revision
            ),
            json!({"schema_id": execution.schema_id, "record": execution}),
            event_type.clone(),
        ),
        routing_event(
            execution,
            "model_lane_routing_stage_attempt",
            &attempt_id,
            format!(
                "routing:{action}:attempt:{attempt_id}:{}",
                execution.revision
            ),
            json!({"schema_id": "hsk.model_lane_routing_stage_attempt@4", "record": attempt}),
            event_type.clone(),
        ),
        routing_event(
            execution,
            "model_lane_routing_outbox",
            &command_id,
            format!(
                "routing:{action}:outbox:{command_id}:{}",
                execution.revision
            ),
            json!({"schema_id": "hsk.model_lane_routing_outbox@4", "status": outbox_status}),
            event_type,
        ),
    ]
}

fn extra_event(
    execution: &ModelLaneRoutingExecutionState,
    tag: &str,
    payload: Value,
) -> NewKernelEvent {
    routing_event(
        execution,
        "model_lane_context_handoff",
        &format!("{}:{tag}", execution.execution_id),
        format!(
            "routing:extra:{}:{}:{tag}",
            execution.execution_id, execution.revision
        ),
        payload,
        KernelEventType::ModelAdapterInvoked,
    )
}

fn routing_event(
    execution: &ModelLaneRoutingExecutionState,
    aggregate_type: &str,
    aggregate_id: &str,
    idempotency_key: String,
    payload: Value,
    event_type: KernelEventType,
) -> NewKernelEvent {
    NewKernelEvent::builder(
        execution.run_id.clone(),
        execution.execution_id.clone(),
        event_type,
        KernelActor::ModelAdapter("DexterityRoutingExecutor".into()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(idempotency_key)
    .correlation_id(format!("dexterity-routing:{}", execution.execution_id))
    .source_component("model_lane_routing_executor")
    .payload(payload)
    .build()
    .expect("build valid canonical routing event")
}

fn stage_claim(
    execution_id: &str,
    attempt: &ModelLaneRoutingStageState,
    lease_owner: &str,
    fencing_token: &str,
    lease_expires_at_unix_ms: u64,
) -> ModelLaneRoutingStageClaim {
    ModelLaneRoutingStageClaim {
        execution_id: execution_id.into(),
        stage_id: attempt.stage_id.clone(),
        attempt: attempt.attempt,
        fencing_token: fencing_token.into(),
        lease_owner: lease_owner.into(),
        lease_expires_at_unix_ms,
        dispatch_target: attempt.dispatch_target,
        expected_run_id: attempt.expected_run_id.clone(),
        expected_lane_id: attempt.expected_lane_id.clone(),
        expected_model_id: attempt.expected_model_id.clone(),
        expected_provider: attempt.expected_provider.clone(),
    }
}

async fn table_count(
    inspector: &handshake_core::storage::surreal::SurrealTestInspector,
    table: &str,
) -> u64 {
    let table = inspector
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("select {table}: {error}"));
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("count {}: {error}", table.name()))
}

async fn field_value_exists(
    inspector: &handshake_core::storage::surreal::SurrealTestInspector,
    table: &str,
    field: &str,
    value: &str,
) -> bool {
    let table = inspector
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("select {table}: {error}"));
    let field = table
        .field(field)
        .unwrap_or_else(|error| panic!("select {} field: {error}", table.name()));
    inspector
        .exists(
            &table,
            RowFilter::FieldEquals {
                field,
                value: ScalarValue::from(value),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("inspect {} value: {error}", table.name()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_millis() as u64
}

fn canonical_sha256(value: &Value) -> String {
    format!("{:x}", Sha256::digest(canonical_json_bytes(value)))
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    fn write(output: &mut String, value: &Value) {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => output.push_str(&value.to_string()),
            Value::String(value) => {
                output.push('"');
                for ch in value.chars() {
                    match ch {
                        '"' => output.push_str("\\\""),
                        '\\' => output.push_str("\\\\"),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        ch => output.push(ch),
                    }
                }
                output.push('"');
            }
            Value::Array(items) => {
                output.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    write(output, item);
                }
                output.push(']');
            }
            Value::Object(map) => {
                output.push('{');
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort();
                for (index, key) in keys.into_iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    write(output, &Value::String(key.clone()));
                    output.push(':');
                    write(output, &map[key]);
                }
                output.push('}');
            }
        }
    }

    let mut output = String::new();
    write(&mut output, value);
    output.into_bytes()
}
