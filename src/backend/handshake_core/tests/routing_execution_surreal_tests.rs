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
use handshake_core::storage::surreal::{
    RowFilter, ScalarValue, SurrealStorage,
};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneLocusBinding, ModelLaneRecoveryState, ModelLaneRunRecord, ModelLaneStore,
    NewModelLaneRun,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
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
use surreal_test_store_support::IsolatedSurrealTestStore;
use uuid::Uuid;
use std::{sync::Arc, future::Future, pin::Pin};
use futures::FutureExt;
use handshake_core::AccountBootstrapState;
use handshake_core::storage::{Database, surreal::SurrealDatabase};
use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction};
use handshake_core::test_harness::{ac9_role_authority::Ac9RoleAuthorityFixture,
    crdt_workspace::AuthenticatedModelLaneTestOperation};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneError, ModelLaneResult, ModelLanePromotionDecisionRecord, ModelLanePromotionOutcome,
    NewModelLane, NewModelLaneMessage, NewModelLanePromotionDecision,
    NewModelLaneContextBundleArtifactBinding, LaunchAuthority, ModelLaneAuthority, ModelLaneKind,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy,
    ModelLaneStatus, ModelLaneTarget, RuntimeBinding,
};
use handshake_core::swarm_orchestration::routing::{ModelLaneRoutingGraph};
use handshake_core::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionDiagnostics;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-017";
const BOARD_ID: &str = "task-board://wp-1";
const STAGE_ID: &str = "local-attempt";
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct OpenHandle<T>(Option<T>);
impl<T> std::ops::Deref for OpenHandle<T> {
    type Target = T;
    fn deref(&self) -> &T { self.0.as_ref().expect("open owned fixture handle") }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleRetirement { Unknown, Owned, Retired }
type RoleRetirementMarkers = Arc<std::sync::Mutex<BTreeMap<String, RoleRetirement>>>;
struct AuthenticatedRoutingStore {
    inner: ModelLaneStore,
    retirement: RoleRetirementMarkers,
    scope: ResourceScope,
    bootstrap: AccountBootstrapState,
    authorization: AuthorizationRequest,
}
impl AuthenticatedRoutingStore {
    fn mark_role_retirement(&self, state: RoleRetirement) {
        self.retirement.lock().unwrap_or_else(|error| error.into_inner())
            .insert(self.authorization.external_resource_id.clone(), state);
    }
    async fn operation(&self, action: ResourceAction) -> ModelLaneResult<AuthenticatedModelLaneTestOperation> {
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&self.scope)
            .map_err(|error| ModelLaneError::AuthorityDenied(format!("exact owner scope required: {error}")))?;
        let mut request = self.authorization.clone();
        request.action = action;
        request.capability_id = if action == ResourceAction::Read { "fs.read" } else { "fs.write" }.into();
        AuthenticatedModelLaneTestOperation::authorize(self.bootstrap.surreal.clone(), request, exact).await
    }
    async fn commit_routing_execution_atomic(&self, revision: u64, claim: Option<&ModelLaneRoutingStageClaim>,
        next: ModelLaneRoutingExecutionState, attempt: ModelLaneRoutingStageState,
        status: &str, events: Vec<NewKernelEvent>) -> ModelLaneResult<ModelLaneRoutingExecutionState> {
        self.operation(if revision == 0 { ResourceAction::Create } else { ResourceAction::Update }).await?
            .run_with_configured_registry(self.bootstrap.capability_registry.clone(),
                self.inner.commit_routing_execution_atomic(revision, claim, next, attempt, status, events)).await
    }
    async fn routing_execution_snapshot(&self, id: &str) -> ModelLaneResult<Option<ModelLaneRoutingExecutionState>> {
        self.operation(ResourceAction::Read).await?.run_with_configured_registry(
            self.bootstrap.capability_registry.clone(), self.inner.routing_execution_snapshot(id)).await
    }
    async fn routing_execution_diagnostics_for_run(&self, run: &str) -> ModelLaneResult<Vec<ModelLaneRoutingExecutionDiagnostics>> {
        self.operation(ResourceAction::Read).await?.run_with_configured_registry(
            self.bootstrap.capability_registry.clone(), self.inner.routing_execution_diagnostics_for_run(run)).await
    }
}

struct Fixture {
    native: authenticated_promotion_fixture::PublicPromotionFixture,
    storage: OpenHandle<SurrealStorage>,
    scope: ResourceScope,
    store: OpenHandle<AuthenticatedRoutingStore>,
    run: ModelLaneRunRecord,
    selection: ModelLanePromotionDecisionRecord,
    role_owner: Option<Ac9RoleAuthorityFixture>,
    execution_id: String,
}
impl std::ops::Deref for Fixture {
    type Target = authenticated_promotion_fixture::PublicPromotionFixture;
    fn deref(&self) -> &Self::Target { &self.native }
}
impl Fixture {
    async fn create(label: &str) -> Self {
        let mut native = authenticated_promotion_fixture::PublicPromotionFixture::create().await;
        let prepared = std::panic::AssertUnwindSafe(async {
            native.authenticate_workspace().await;
            let scope = native.scope.as_ref().expect("native workspace").clone();
            let store = Self::scoped_store(&native, scope.clone());
            let (run, selection, role_owner) = seed_selecting_authority(&store, sample_run(label), label).await;
            (scope, store, run, selection, role_owner)
        }).catch_unwind().await;
        match prepared {
            Ok((scope, store, run, selection, role_owner)) => Self {
                storage: OpenHandle(Some(native.isolated.storage().clone())), native, scope,
                store: OpenHandle(Some(store)), run, selection, role_owner: Some(role_owner),
                execution_id: format!("routing-{label}-{}", Uuid::now_v7().simple()),
            },
            Err(primary) => {
                let cleanup = std::panic::AssertUnwindSafe(native.cleanup()).catch_unwind().await.err();
                std::panic::resume_unwind(Box::new((primary, cleanup)));
            }
        }
    }
    fn scoped_store(native: &authenticated_promotion_fixture::PublicPromotionFixture, scope: ResourceScope)
        -> AuthenticatedRoutingStore {
        let mut authorization = native.authorization();
        if let Some(workspace) = scope.workspace.as_ref() {
            authorization.external_resource_id = workspace.as_str().into();
        }
        AuthenticatedRoutingStore {
            scope: scope.clone(),
            retirement: native.retirement.clone(),
            inner: ModelLaneStore::new_scoped_with_lifecycle(native.isolated.storage().clone(), scope,
                native.isolated.storage().resource_access_lifecycle()),
            bootstrap: native.bootstrap.as_ref().expect("actual bootstrap").clone(), authorization,
        }
    }
    fn store_for(&self, scope: ResourceScope) -> AuthenticatedRoutingStore { Self::scoped_store(&self.native, scope) }
    fn incomplete_store_for(&self, scope: ResourceScope) -> AuthenticatedRoutingStore { self.store_for(scope) }
    fn initial_projection(&self) -> (ModelLaneRoutingExecutionState, ModelLaneRoutingStageState) {
        projection(&self.run, &self.selection, &self.execution_id, 1, ModelLaneRoutingStageStateKind::Scheduled,
            None, None, None, ModelLaneRoutingExecutionStatus::Running, None)
    }
    async fn reopen(&mut self) {
        let registry = self.native.bootstrap.as_ref().unwrap().capability_registry.clone();
        if let Some(role) = self.role_owner.take() {
            retire_role_owner(&self.store, role).await.expect("drain original role owner before storage close; failure retains UNKNOWN allocation");
        }
        self.native.assert_roles_retired();
        drop(self.store.0.take());
        drop(self.storage.0.take());
        drop(self.native.bootstrap.take());
        self.native.isolated.reopen().await.expect("reopen original isolated storage after all handles drain");
        let storage = self.native.isolated.storage().clone();
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
        self.native.bootstrap = Some(AccountBootstrapState::new(database, storage.clone(), registry));
        self.store.0 = Some(Self::scoped_store(&self.native, self.scope.clone()));
        self.storage.0 = Some(storage);
    }
    async fn cleanup(&mut self) {
        if let Some(role) = self.role_owner.take() {
            retire_role_owner(&self.store, role).await
                .expect("original genuine role owner drain failed; retain UNKNOWN allocation");
        }
        self.native.assert_roles_retired();
        drop(self.store.0.take());
        drop(self.storage.0.take());
        self.native.cleanup().await;
    }
}

async fn with_fixture<F>(label: &str, case: F)
where F: for<'a> FnOnce(&'a mut Fixture) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
    let mut fixture = Fixture::create(label).await;
    let primary = std::panic::AssertUnwindSafe(case(&mut fixture)).catch_unwind().await.err();
    let cleanup = std::panic::AssertUnwindSafe(fixture.cleanup()).catch_unwind().await.err();
    if primary.is_some() || cleanup.is_some() { std::panic::resume_unwind(Box::new((primary, cleanup))); }
}

fn assert_scope_denial(error: ModelLaneError, dimension: &str) {
    assert!(matches!(&error, ModelLaneError::AuthorityDenied(_)), "{dimension}: expected native authority/scope denial, got {error}");
}
#[tokio::test]
async fn initial_creation_retry_extra_event_and_restart_share_one_event_authority() {
    with_fixture("initial-restart", |fixture| Box::pin(async move {


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
    fixture.reopen().await;
    assert_eq!(fixture.isolated.namespace(), namespace);
    assert_eq!(fixture.isolated.database(), database);
    
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("read restarted routing projection"),
        Some(committed)
    );


    })).await;
}

#[tokio::test]
async fn stale_cas_event_set_conflict_and_stale_fence_cannot_mutate_cancellation() {
    with_fixture("cas-fence-cancel", |fixture| Box::pin(async move {


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
        &fixture.selection,
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
        &fixture.selection,
        &fixture.execution_id,
        3,
        ModelLaneRoutingStageStateKind::Cancelled,
        None,
        None,
        None,
        ModelLaneRoutingExecutionStatus::Cancelled,
        Some("operator cancellation"),
    );
    let cancel_events = required_events(&cancelled, &cancelled_attempt, "cancel", "cancelled");
    fixture
        .store
        .commit_routing_execution_atomic(
            2,
            Some(&foreign_claim),
            cancelled.clone(),
            cancelled_attempt.clone(),
            "cancelled",
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
            "cancelled",
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
            "cancelled",
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

    fixture.reopen().await;
    
    assert_eq!(
        fixture
            .store
            .routing_execution_snapshot(&fixture.execution_id)
            .await
            .expect("recover cancellation after restart"),
        Some(durable_cancelled)
    );


    })).await;
}

#[tokio::test]
async fn five_scope_mismatches_and_incomplete_scopes_fail_closed_without_rows_or_events() {
    with_fixture("scope-denial", |fixture| Box::pin(async move {


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
        let denied = fixture.store_for(scope);
        assert_scope_denial(denied.routing_execution_snapshot(&fixture.execution_id).await
            .expect_err("one-field scope mismatch must not expose an execution"), dimension);
        assert_scope_denial(denied.routing_execution_diagnostics_for_run(&fixture.run.run_id).await
            .expect_err("one-field scope mismatch must not expose identifiers"), dimension);
        let denied_write = denied
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
        assert_scope_denial(denied_write, dimension);
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
        let denied = fixture.incomplete_store_for(scope);
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


    })).await;
}

#[tokio::test]
async fn projection_and_required_event_counterfactuals_fail_without_mutation() {
    with_fixture("event-counterfactuals", |fixture| Box::pin(async move {


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


    })).await;
    with_fixture("false-event-pointer", |false_pointer| Box::pin(async move {

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


    })).await;
}

#[tokio::test]
async fn identical_logical_routing_ids_are_isolated_between_two_exact_scopes() {
    with_fixture("scope-a", |fixture| Box::pin(async move {


    let scope_b = fixture.native.additional_workspace("scope-b").await;
    let store_b = fixture.store_for(scope_b.clone());
    let (run_b, selection_b, role_b) = seed_selecting_authority(&store_b, fixture.run.inner.clone(), "scope-b").await;
    let secondary_result = std::panic::AssertUnwindSafe(async {
    let (execution_a, attempt_a) = fixture.initial_projection();
    let (execution_b, attempt_b) = projection(
        &run_b,
        &selection_b,
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
    }).catch_unwind().await.err();
    let secondary_cleanup = retire_role_owner(&store_b, role_b).await;
    assert!(secondary_cleanup.is_ok(), "second scoped owner drain (UNKNOWN retained on failure): {secondary_cleanup:?}");
    drop(store_b);
    if let Some(primary) = secondary_result { std::panic::resume_unwind(primary); }


    })).await;
}

#[tokio::test]
async fn preexisting_canonical_receipt_without_routing_projection_is_rejected_as_orphan() {
    with_fixture("orphan-receipt", |fixture| Box::pin(async move {


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


    })).await;
}

fn sample_run(label: &str) -> NewModelLaneRun {
    let suffix = Uuid::now_v7().simple().to_string();
    let run_id = format!("run-routing-{label}-{suffix}");
    let coordinator_session_id = format!("coordinator-{suffix}");
    let owner_session = format!("owner-session-{suffix}");
    NewModelLaneRun {
        engine_profile: None,
        run_id: run_id.clone(),
        trace_id: format!("trace-{suffix}"),
        run_span_id: format!("run-span-{suffix}"),
        coordinator_session_id: coordinator_session_id.clone(),
        routing_policy: ModelLaneRoutingPolicy::LocalFirst.as_str().into(),
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
    selection: &ModelLanePromotionDecisionRecord,
    execution_id: &str,
    revision: u64,
    stage_kind: ModelLaneRoutingStageStateKind,
    lease_owner: Option<&str>,
    fencing_token: Option<&str>,
    lease_expires_at_unix_ms: Option<u64>,
    status: ModelLaneRoutingExecutionStatus,
    cancel_reason: Option<&str>,
) -> (ModelLaneRoutingExecutionState, ModelLaneRoutingStageState) {
    let canonical_graph = serde_json::to_value(ModelLaneRoutingGraph::for_policy(selection.routing_policy)).expect("canonical policy graph");
    let canonical_launch_plan = selection.routing_launch_plan.clone();
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
        input_refs: selection.selected_input_refs.clone(),
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
            selecting_decision_id: selection.decision_id.clone(),
            selecting_decision_event_id: selection.event_ledger_event_id.clone(),
            selecting_decision_event_seq: selection.event_ledger_seq,
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
            initial_input_ref: selection.selected_input_refs.first().cloned(),
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

mod authenticated_promotion_fixture {
    use super::*;
    use std::path::Path;
    use std::sync::{Arc, Mutex, MutexGuard};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use futures::FutureExt;
    use handshake_core::api::{authority, stage, workspaces};
    use handshake_core::capabilities::CapabilityRegistry;
    use handshake_core::storage::Database;
    use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
    use handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation;
    use handshake_core::AccountBootstrapState;
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    static CHANNEL_LOCK: Mutex<()> = Mutex::new(());

    pub(super) struct NativeChannel {
        _lock: MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        pub(super) token: String,
    }

    impl NativeChannel {
        pub(super) fn create(root: &Path) -> Self {
            let lock = CHANNEL_LOCK.lock().unwrap_or_else(|error| error.into_inner());
            let directory = tempfile::tempdir_in(root).expect("own promotion native binding");
            let token = hex::encode(Sha256::digest(Uuid::now_v7().as_bytes()));
            let path = directory.path().join("native-binding.json");
            std::fs::write(&path, serde_json::to_vec(
                &stage::current_process_native_session_binding(&token),
            ).expect("serialize actual promotion process binding")).expect("write owned native binding");
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Self { _lock: lock, _directory: directory, previous, token }
        }
    }

    impl Drop for NativeChannel {
        fn drop(&mut self) {
            match &self.previous {
                Some(previous) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous),
                None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
            }
        }
    }

    struct OwnedSession { token: String, identity: Value }

    pub(super) struct PublicPromotionFixture {
        pub(super) retirement: RoleRetirementMarkers,
        pub(super) isolated: IsolatedSurrealTestStore,
        pub(super) bootstrap: Option<AccountBootstrapState>,
        channel: NativeChannel,
        session: Option<OwnedSession>,
        pub(super) scope: Option<ResourceScope>,
    }

    impl PublicPromotionFixture {
        pub(super) async fn create() -> Self {
            let root = surreal_test_store_support::canonical_artifacts_root_for_proof()
                .expect("verified promotion artifact root");
            let owner = root.join("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
                .join("MT-017").join("authenticated-routing-fixture");
            assert!(owner.starts_with(&root));
            std::fs::create_dir_all(&owner).expect("create assigned promotion fixture descendant");
            let isolated = IsolatedSurrealTestStore::create_in(&owner).await.expect("isolated promotion database");
            let channel = NativeChannel::create(&owner);
            let registry = Arc::new(CapabilityRegistry::new());
            registry.profile_by_id("Operator").expect("actual loaded Operator profile");
            let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(isolated.storage().clone()));
            let bootstrap = AccountBootstrapState::new(database, isolated.storage().clone(), registry);
            Self { retirement: Arc::new(Mutex::new(BTreeMap::new())), isolated, bootstrap: Some(bootstrap), channel, session: None, scope: None }
        }

        fn router(&self) -> Router {
            let state = self.bootstrap.as_ref().expect("owned bootstrap").clone();
            let (activation, _receiver) = tokio::sync::mpsc::channel(1);
            authority::bootstrap_routes(state.clone(), activation).merge(workspaces::bootstrap_routes(state))
        }

        pub(super) async fn authenticate_workspace(&mut self) {
            assert!(self.session.is_none(), "one original session per fixture");
            let router = self.router();
            let (status, setup) = dispatch(&router, "GET", "/authority/setup", &self.channel.token, None, Value::Null).await;
            assert_eq!(status, StatusCode::OK, "read canonical setup state");
            let credentials = json!({"account_name": "Synthetic isolated promotion operator",
                "password": "synthetic isolated promotion fixture password without egress"});
            if setup["setup_required"].as_bool().expect("typed setup state") {
                let (status, _) = dispatch(&router, "POST", "/authority/setup", &self.channel.token, None, credentials.clone()).await;
                assert_eq!(status, StatusCode::OK, "actual one-time account setup");
            }
            let (status, credential) = dispatch(&router, "POST", "/authority/login", &self.channel.token, None, credentials).await;
            assert_eq!(status, StatusCode::OK, "actual password authentication");
            let (status, identity) = dispatch(&router, "POST", "/authority/session", &self.channel.token, None,
                json!({"account_id": field(&credential, "account_id"), "principal_id": field(&credential, "principal_id"),
                    "access_space_id": field(&credential, "access_space_id"), "authentication_token": field(&credential, "token")})).await;
            assert_eq!(status, StatusCode::OK, "actual session and vault exchange");
            let token = field(&identity, "session_token").to_owned();
            self.session = Some(OwnedSession { token: token.clone(), identity });
            let session = self.session.as_ref().expect("session cleanup custody installed");
            for key in ["account_id", "principal_id", "access_space_id"] {
                assert_eq!(field(&credential, key), field(&session.identity, key));
            }
            let (status, workspace) = dispatch(&router, "POST", "/workspaces", &self.channel.token, Some(&token),
                json!({"name": "Authenticated MT-017 promotion fixture"})).await;
            assert_eq!(status, StatusCode::CREATED, "real workspace/resource/Create grant producer");
            self.scope = Some(ResourceScope::new(
                OwnerAccountId::from_uuid(id(&session.identity, "account_id")),
                ActorPrincipalId::from_uuid(id(&session.identity, "principal_id")),
            ).with_session(AuthenticatedSessionRef::from_uuid(id(&session.identity, "session_id")))
                .with_access_space(AccessSpaceRef::from_uuid(id(&session.identity, "access_space_id")))
                .with_workspace(WorkspaceScopeRef::new(field(&workspace, "id")).expect("returned workspace id")));
        }

        pub(super) async fn additional_workspace(&self, label: &str) -> ResourceScope {
            let session = self.session.as_ref().expect("original authenticated session");
            let (status, workspace) = dispatch(&self.router(), "POST", "/workspaces", &self.channel.token,
                Some(&session.token), json!({"name":format!("Authenticated MT-017 {label}")})).await;
            assert_eq!(status, StatusCode::CREATED, "canonical additional workspace and owning grant");
            let mut scope = self.scope.as_ref().expect("original canonical account/session tuple").clone();
            scope.workspace = Some(WorkspaceScopeRef::new(field(&workspace, "id")).expect("real returned workspace"));
            scope
        }

        pub(super) fn authorization(&self) -> AuthorizationRequest {
            let session = self.session.as_ref().expect("actual authenticated session");
            let scope = self.scope.as_ref().expect("actual workspace scope");
            AuthorizationRequest { session_token: session.token.clone(),
                channel_binding_hash: Some(hex::encode(Sha256::digest(self.channel.token.as_bytes()))),
                capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
                external_resource_id: scope.workspace.as_ref().expect("actual workspace").as_str().to_owned(),
                action: ResourceAction::Create }
        }

        pub(super) fn scoped_store(&self) -> ModelLaneStore {
            ModelLaneStore::new_scoped_with_lifecycle(self.isolated.storage().clone(),
                self.scope.as_ref().expect("actual complete scope").clone(),
                self.isolated.storage().resource_access_lifecycle())
        }

        pub(super) fn assert_roles_retired(&self) {
            let markers = self.retirement.lock().unwrap_or_else(|error| error.into_inner());
            assert!(markers.values().all(|state| *state == RoleRetirement::Retired),
                "retain isolated allocation {}: role retirement not proven; markers={markers:?}",
                self.isolated.store_path().display());
        }
        pub(super) async fn cleanup(&mut self) {
            self.assert_roles_retired();
            let logout_failure = std::panic::AssertUnwindSafe(async {
                if let Some(session) = self.session.as_ref() {
                    let router = self.router();
                    let (status, body) = dispatch(&router, "POST", "/authority/logout", &self.channel.token,
                        Some(&session.token), Value::Null).await;
                    assert_eq!(status, StatusCode::OK, "revoke actual session and remove its vault secret");
                    assert!(body.is_object(), "typed native logout response");
                }
            }).catch_unwind().await.err();
            self.session.take();
            drop(self.bootstrap.take());
            let database_failure = std::panic::AssertUnwindSafe(async {
                self.isolated.cleanup().await.expect("close actual promotion fixture database");
            }).catch_unwind().await.err();
            if logout_failure.is_some() || database_failure.is_some() {
                std::panic::resume_unwind(Box::new((logout_failure, database_failure)));
            }
        }
    }

    async fn dispatch(router: &Router, method: &str, path: &str, channel: &str, token: Option<&str>, payload: Value)
        -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path)
            .header("content-type", "application/json").header("x-hsk-channel-binding-token", channel);
        if let Some(token) = token { request = request.header("x-hsk-session-token", token); }
        let response = router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&payload)
            .expect("serialize actual public fixture request"))).expect("public fixture request")).await
            .expect("dispatch actual bootstrap route");
        let status = response.status();
        let body = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded public fixture response");
        (status, serde_json::from_slice(&body).expect("actual public response JSON"))
    }

    fn field<'a>(value: &'a Value, key: &str) -> &'a str {
        value.get(key).and_then(Value::as_str).filter(|value| !value.is_empty()).expect("canonical response field")
    }
    fn id(value: &Value, key: &str) -> Uuid { Uuid::parse_str(field(value, key)).expect("canonical response UUID") }

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


fn sample_lane(label: &str) -> NewModelLane {
    let session = format!("session-mt004-{label}");
    let model_session = format!("model-session-mt004-{label}");
    NewModelLane {
        engine_profile: None,
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


async fn seed_selecting_authority(store: &AuthenticatedRoutingStore, input: NewModelLaneRun, label: &str)
    -> (ModelLaneRunRecord, ModelLanePromotionDecisionRecord, Ac9RoleAuthorityFixture) {
    let operation = store.operation(ResourceAction::Create).await.expect("native Create for canonical inputs");
    let (run, lane, artifact, proposal) = operation.run_with_configured_registry(
        store.bootstrap.capability_registry.clone(), async {
            let run = store.inner.record_run(input).await?;
            let mut lane = sample_lane(label);
            lane.run_id = run.run_id.clone(); lane.trace_id = run.trace_id.clone();
            lane.lane_id = run.lane_ids[0].clone(); lane.model_id = run.selected_model_id.clone();
            lane.event_ledger_stream_id = run.event_ledger_stream_id.clone();
            lane.work_packet_id = run.work_packet_id.clone(); lane.micro_task_id = run.micro_task_id.clone();
            lane.task_board_id = run.task_board_id.clone(); lane.owner_session = run.owner_session.clone();
            let locus = run.locus_binding.as_ref().expect("original run locus");
            lane.session_id = locus.session_id.clone(); lane.model_session_id = locus.model_session_id.clone();
            lane.locus_binding = Some(locus.clone());
            let lane = store.inner.record_lane(lane).await?;
            let mut artifact = sample_artifact_binding(label);
            artifact.run_id = run.run_id.clone(); artifact.trace_id = run.trace_id.clone();
            artifact.event_ledger_stream_id = run.event_ledger_stream_id.clone();
            artifact.work_packet_id = WP_ID.into(); artifact.micro_task_id = MT_ID.into();
            artifact.task_board_id = BOARD_ID.into(); artifact.owner_session = run.owner_session.clone();
            let artifact = store.inner.record_context_bundle_artifact_binding(artifact).await?;
            let mut proposal = sample_message(label, "proposal");
            proposal.run_id = run.run_id.clone(); proposal.trace_id = run.trace_id.clone();
            proposal.from_lane_id = lane.lane_id.clone(); proposal.coordinator_session_id = run.coordinator_session_id.clone();
            proposal.event_ledger_stream_id = run.event_ledger_stream_id.clone();
            proposal.work_packet_id = run.work_packet_id.clone(); proposal.micro_task_id = run.micro_task_id.clone();
            proposal.task_board_id = run.task_board_id.clone(); proposal.owner_session = run.owner_session.clone();
            proposal.locus_binding = run.locus_binding.clone();
            if let Some(routing) = proposal.routing.as_mut() { routing.target_session = run.coordinator_session_id.clone(); }
            let proposal = store.inner.record_message(proposal).await?;
            Ok((run, lane, artifact, proposal))
        }).await.expect("canonical persisted inputs under original Create");
    drop(operation);
    let mut candidate = sample_decision(label, &format!("select-routing-{}", run.run_id),
        &format!("select-routing-{}", run.run_id), proposal.event_stream_version);
    candidate.run_id = run.run_id.clone(); candidate.trace_id = run.trace_id.clone();
    candidate.coordinator_session_id = run.coordinator_session_id.clone();
    candidate.owner_session = run.owner_session.clone(); candidate.work_packet_id = run.work_packet_id.clone();
    candidate.micro_task_id = run.micro_task_id.clone(); candidate.task_board_id = run.task_board_id.clone();
    candidate.event_ledger_stream_id = run.event_ledger_stream_id.clone();
    candidate.routing_policy = ModelLaneRoutingPolicy::LocalFirst;
    candidate.routing_launch_plan = vec![
        ModelLaneRoutingStageLaunchPlan { stage_id: STAGE_ID.into(), dispatch_target: ModelLaneRoutingDispatchTarget::LocalModel,
            lane_id: Some(lane.lane_id.clone()), model_id: run.selected_model_id.clone(),
            provider: Some(handshake_core::model_runtime::ProviderKind::Local) },
        // Complete persisted optional plan; this persistence target never dispatches it.
        ModelLaneRoutingStageLaunchPlan { stage_id: "cloud-escalation".into(), dispatch_target: ModelLaneRoutingDispatchTarget::CloudModel,
            lane_id: Some(format!("cloud-planned-{}", run.run_id)), model_id: Some("routing-persistence-cloud-model".into()),
            provider: Some(handshake_core::model_runtime::ProviderKind::OfficialCli) },
    ];
    let proposal_ref = format!("model-lane-message://{}", proposal.message_id);
    candidate.input_refs = vec![proposal_ref.clone()]; candidate.selected_input_refs = vec![proposal_ref];
    candidate.rejected_input_refs.clear();
    candidate.expected_event_ledger_aggregate_id = proposal.message_id.clone();
    candidate.promoted_artifact_ref = Some(artifact.artifact_ref.clone());
    candidate.promoted_artifact_sha256 = Some(artifact.artifact_sha256.clone());
    let policy = ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract";
    let retention = handshake_role_authority::domain::RetentionBinding {
        policy_ref: policy.into(), authority_class: "Owner".into(), replay_policy_ref: policy.into(),
        artifact_retention_ttl_days: None,
    };
    let task = json!({"format_version":1,"task_id":format!("routing-persistence-{label}"),"claimed_revision":1,
        "operations":[
            {"tool":"delegate_workspace_read","resource_kind":"workspace",
             "external_resource_id":store.authorization.external_resource_id,"action":"read","capability_id":"fs.read"},
            {"tool":"delegate_workspace_create","resource_kind":"workspace",
             "external_resource_id":store.authorization.external_resource_id,"action":"create","capability_id":"fs.write"}
        ]});
    store.mark_role_retirement(RoleRetirement::Unknown);
    let role = Ac9RoleAuthorityFixture::produce(&store.bootstrap, store.inner.clone(), store.authorization.clone(),
        &run, &lane, &artifact, &candidate, retention.clone(), retention, task).await
        .expect("MissingPrerequisite or genuine existing configured CLI role production failure");
    store.mark_role_retirement(RoleRetirement::Owned);
    let selectors = role.selectors();
    candidate.operator_authority_ref = Some(selectors.operator.evidence_ref.as_str().to_owned());
    candidate.validator_authority_ref = Some(selectors.validator.evidence_ref.as_str().to_owned());
    let selected = std::panic::AssertUnwindSafe(async { match store.operation(ResourceAction::Create).await {
        Ok(operation) => operation.run_with_configured_registry(store.bootstrap.capability_registry.clone(),
            role.record_promotion_decision(&store.bootstrap, store.authorization.clone(), Uuid::now_v7(), candidate)).await,
        Err(error) => Err(error),
    } }).catch_unwind().await;
    match selected {
        Ok(Ok(selected)) => {
            if selected.outcome != ModelLanePromotionOutcome::Approved {
                let cleanup = retire_role_owner(store, role).await;
                panic!("genuine selecting authority was {:?}; role cleanup: {cleanup:?}", selected.outcome);
            }
            (run, selected, role)
        }
        Ok(Err(error)) => {
            let cleanup = retire_role_owner(store, role).await;
            panic!("genuine selecting decision failed: {error}; role cleanup: {cleanup:?}");
        }
        Err(primary) => {
            let cleanup = retire_role_owner(store, role).await;
            std::panic::resume_unwind(Box::new((primary, cleanup.err())));
        }
    }
}
async fn retire_role_owner(store: &AuthenticatedRoutingStore, role: Ac9RoleAuthorityFixture) -> ModelLaneResult<()> {
    store.mark_role_retirement(RoleRetirement::Unknown);
    let result = role.shutdown().await;
    if result.is_ok() { store.mark_role_retirement(RoleRetirement::Retired); }
    result
}