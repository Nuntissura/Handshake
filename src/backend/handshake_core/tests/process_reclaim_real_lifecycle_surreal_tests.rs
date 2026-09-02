//! WP-1 MT-019 production-boundary proofs over one injected embedded SurrealDB.
//!
//! Every fixture writes and reads the same cloned `SurrealStorage` namespace/database.
//! Durable probes bind the exact five-field `ReclaimResourceScope`; no relational
//! compatibility store, fixture, or fallback participates in this suite.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use handshake_core::{
    process_ledger::{
        acquire_embedded_runtime_instance_lease, production_process_sandbox_registry,
        reconcile_restart_orphans_at_boot, set_dead_owner_confirmation_gap_override_for_test,
        spawn_managed_staleness_reclaim_task_after_boot, KillError, KillOutcome,
        LedgerDrainJoinOutcome, LedgerEvent, ProcessEngineKind, ProcessLedgerError,
        ProcessLedgerStore, ProcessReclaimRuntime, ProcessRuntimeOwner, ProcessStart, ProcessStop,
        ProductionSandboxKill, Reclaim, ReclaimKillOperationStatus, ReclaimProcessStore,
        ReclaimResourceScope, ReclaimStopReservation, ReclaimStopWriter, ReclaimTrigger,
        SandboxKill, StaleSessionSource, StalenessReclaimConfig,
        SurrealModelLaneStaleSessionSource, SurrealProcessLedgerStore,
    },
    sandbox::{AdapterId, SandboxAdapterRegistry},
    storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig},
};
use serde_json::{json, Value};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

struct Fixture {
    _directory: tempfile::TempDir,
    storage: SurrealStorage,
    store: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
}

struct DeadOwnerGapReset;

impl Drop for DeadOwnerGapReset {
    fn drop(&mut self) {
        set_dead_owner_confirmation_gap_override_for_test(None);
    }
}

impl Fixture {
    async fn open() -> Self {
        let directory = tempfile::tempdir().expect("create MT-019 Surreal test directory");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(directory.path().join("data"))
                .expect("configure MT-019 Surreal test store"),
        )
        .await
        .expect("open MT-019 Surreal test store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap shared Surreal schema");
        let store = Arc::new(
            SurrealProcessLedgerStore::open(storage.clone())
                .await
                .expect("open Surreal ProcessLedger provider"),
        );
        Self {
            _directory: directory,
            storage,
            store,
            scope: exact_scope(),
        }
    }

    async fn shutdown(self) {
        drop(self.store);
        self.storage
            .shutdown()
            .await
            .expect("shutdown MT-019 Surreal test store");
    }

    async fn seed(&self, start: ProcessStart) {
        self.store
            .write_batch(vec![LedgerEvent::Start(start)])
            .await
            .expect("seed exact-scope ProcessLedger START");
    }

    async fn state(&self, process_uuid: Uuid) -> Option<LifecycleState> {
        lifecycle_state(&self.storage, &self.scope, process_uuid).await
    }
}

fn exact_scope() -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: Uuid::now_v7(),
        actor_uuid: Uuid::now_v7(),
        session_uuid: Uuid::now_v7(),
        workspace_id: format!("workspace-{}", Uuid::now_v7()),
        access_space_uuid: Uuid::now_v7(),
    }
}

fn scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id.clone(),
    })
}

fn scoped_start(
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
    parent_session_id: Option<&str>,
    owner: Option<ProcessRuntimeOwner>,
    sandboxed: bool,
) -> ProcessStart {
    let mut start = ProcessStart::new(
        ProcessEngineKind::OfficialCliBridge,
        "MT-019-SURREAL-PROOF",
        Some("WP-1".to_owned()),
    )
    .with_process_uuid(process_uuid)
    .with_metadata_jsonb(scope_metadata(scope));
    if let Some(session_id) = parent_session_id {
        start = start.with_parent_session_id(session_id);
    }
    if let Some(owner) = owner {
        start = start.with_runtime_owner(owner);
    }
    if sandboxed {
        start = start
            .with_sandbox_adapter_id("mt019-test-adapter")
            .with_sandbox_internal_id(process_uuid.to_string());
    }
    start
}

#[derive(Debug, SurrealValue)]
struct LifecycleBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl LifecycleBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid) -> Self {
        Self {
            record: RecordId::new("kernel_process_lifecycle", process_uuid.to_string()),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct LifecycleState {
    process_uuid: Uuid,
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    reclaim_claimant_uuid: Option<Uuid>,
    reclaim_generation: Option<i64>,
    metadata: Value,
}

const READ_LIFECYCLE: &str = r#"
SELECT process_uuid, stopped_at, stop_reason, reclaim_claimant_uuid,
    reclaim_generation, metadata
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

async fn lifecycle_state(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) -> Option<LifecycleState> {
    let bindings = LifecycleBindings::new(scope, process_uuid);
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<LifecycleState, _>(READ_LIFECYCLE, bindings)
                    .await
            })
        })
        .await
        .expect("read exact-scope lifecycle")
}

#[derive(Clone)]
struct DirectStopWriter {
    store: Arc<SurrealProcessLedgerStore>,
}

struct DirectStopReservation {
    store: Arc<SurrealProcessLedgerStore>,
}

#[async_trait]
impl ReclaimStopReservation for DirectStopReservation {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        _timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.store.write_batch(vec![LedgerEvent::Stop(stop)]).await
    }
}

impl ReclaimStopWriter for DirectStopWriter {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(DirectStopReservation {
            store: Arc::clone(&self.store),
        }))
    }
}

struct RejectingStopWriter;

impl ReclaimStopWriter for RejectingStopWriter {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Err(ProcessLedgerError::EnqueueDropped(
            "injected MT-019 STOP reservation rejection".to_owned(),
        ))
    }
}

#[derive(Default)]
struct RecordingKill {
    attempts: Mutex<Vec<(ReclaimResourceScope, Uuid, Uuid)>>,
    fail: bool,
}

impl RecordingKill {
    fn succeeding() -> Self {
        Self::default()
    }

    fn failing() -> Self {
        Self {
            attempts: Mutex::new(Vec::new()),
            fail: true,
        }
    }

    fn attempts(&self) -> Vec<(ReclaimResourceScope, Uuid, Uuid)> {
        self.attempts.lock().unwrap().clone()
    }
}

#[async_trait]
impl SandboxKill for RecordingKill {
    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError> {
        self.attempts.lock().unwrap().push((
            resource_scope.clone(),
            process_uuid,
            kill_operation_uuid,
        ));
        if self.fail {
            Err(KillError::new("injected MT-019 kill failure"))
        } else {
            Ok(())
        }
    }

    async fn kill_operation_status(
        &self,
        _resource_scope: &ReclaimResourceScope,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        Ok(if self.fail {
            ReclaimKillOperationStatus::Failed
        } else {
            ReclaimKillOperationStatus::Succeeded
        })
    }
}

#[derive(Default)]
struct UnresponsiveKill {
    attempts: Mutex<Vec<(ReclaimResourceScope, Uuid, Uuid)>>,
}

#[async_trait]
impl SandboxKill for UnresponsiveKill {
    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError> {
        self.attempts.lock().unwrap().push((
            resource_scope.clone(),
            process_uuid,
            kill_operation_uuid,
        ));
        std::future::pending().await
    }

    async fn kill_operation_status(
        &self,
        _resource_scope: &ReclaimResourceScope,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        Ok(ReclaimKillOperationStatus::Unknown)
    }
}

fn reclaim_with(store: Arc<SurrealProcessLedgerStore>, killer: Arc<RecordingKill>) -> Reclaim {
    Reclaim::new(
        Arc::clone(&store),
        killer,
        Arc::new(DirectStopWriter { store }),
    )
}

#[derive(Debug, SurrealValue)]
struct LaneBindings {
    record: RecordId,
    lane_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const CREATE_LANE_AUTHORITY: &str = r#"
CREATE $record CONTENT {
    record_kind: 'lane', aggregate_id: $lane_id, run_id: $run_id,
    idempotency_key: $idempotency_key, record_json: $record_json,
    search_terms: [], event_id: $event_id, event_seq: $event_seq,
    event_stream_version: 1, transaction_seq: $event_seq,
    owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id,
    access_space_id: $access_space_id, workspace_id: $workspace_id
};
"#;

async fn seed_lane(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: &str,
    process_uuid: Uuid,
    status: &str,
) {
    let lane_id = format!("lane-{}", Uuid::now_v7());
    let bindings = LaneBindings {
        record: RecordId::new("model_lane_authority", lane_id.clone()),
        lane_id: lane_id.clone(),
        run_id: format!("run-{}", Uuid::now_v7()),
        idempotency_key: format!("idem-{}", Uuid::now_v7()),
        record_json: json!({
            "lane_id": lane_id,
            "coordinator_session_id": session_id,
            "process_ownership_ref": format!("process-ledger://{process_uuid}"),
            "status": status,
            "heartbeat_at_utc": Utc::now().to_rfc3339(),
            "reclaim_after_utc": Value::Null,
        })
        .to_string(),
        event_id: format!("event-{}", Uuid::now_v7()),
        event_seq: Utc::now().timestamp_micros(),
        owner_account_id: scope.account_uuid.to_string(),
        actor_principal_id: scope.actor_uuid.to_string(),
        authenticated_session_id: scope.session_uuid.to_string(),
        access_space_id: scope.access_space_uuid.to_string(),
        workspace_id: scope.workspace_id.clone(),
    };
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .execute_returning(CREATE_LANE_AUTHORITY, bindings)
                    .await
            })
        })
        .await
        .expect("seed exact-scope model-lane authority");
}

#[derive(Debug, SurrealValue)]
struct TamperScopeBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const REMOVE_ACCESS_SPACE: &str = r#"
UPDATE $record SET access_space_id = NONE
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

async fn remove_one_scope_field(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) {
    let exact = LifecycleBindings::new(scope, process_uuid);
    let bindings = TamperScopeBindings {
        record: exact.record,
        owner_account_id: exact.owner_account_id,
        actor_principal_id: exact.actor_principal_id,
        authenticated_session_id: exact.authenticated_session_id,
        access_space_id: exact.access_space_id,
        workspace_id: exact.workspace_id,
    };
    let changed = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .execute_returning(REMOVE_ACCESS_SPACE, bindings)
                    .await
            })
        })
        .await
        .expect("tamper one scope field for negative proof");
    assert_eq!(changed, 1, "negative fixture must alter exactly one row");
}

#[tokio::test]
async fn mt019_running_app_reclaims_exact_owned_process_without_reboot() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current runtime lease");
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            None,
            Some(lease.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));

    let report = reclaim
        .run_owned_process(
            &fixture.scope,
            process_uuid,
            lease.instance_id(),
            ReclaimTrigger::Failure,
        )
        .await
        .expect("running-app exact-process reclaim");
    assert_eq!(report.processes_reclaimed.len(), 1);
    assert_eq!(report.processes_reclaimed[0].process_uuid, process_uuid);
    assert!(matches!(
        &report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    ));
    assert_eq!(killer.attempts()[0].0, fixture.scope);
    assert!(fixture
        .state(process_uuid)
        .await
        .unwrap()
        .stopped_at
        .is_some());
    drop(lease);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_one_field_scope_mismatch_and_missing_scope_fail_closed() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            Some(lease.descriptor().process_runtime_owner()),
            true,
        ))
        .await;

    let mut wrong_scope = fixture.scope.clone();
    wrong_scope.access_space_uuid = Uuid::now_v7();
    assert!(fixture
        .store
        .active_processes_for_session(&wrong_scope, &session_id)
        .await
        .expect("mismatched exact-scope claim must be a safe empty result")
        .is_empty());

    remove_one_scope_field(&fixture.storage, &fixture.scope, process_uuid).await;
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        lease.descriptor().clone(),
    )
    .with_dead_owner_confirmation_gap(Duration::ZERO);
    assert!(source
        .restart_session_process_sets()
        .await
        .expect("legacy incomplete attribution must veto without mutation")
        .is_empty());
    assert!(
        lifecycle_state(&fixture.storage, &fixture.scope, process_uuid)
            .await
            .is_none()
    );
    drop(lease);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_single_row_claim_leaves_sibling_metadata_untouched() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let target = Uuid::now_v7();
    let sibling = Uuid::now_v7();
    for process_uuid in [target, sibling] {
        fixture
            .seed(scoped_start(
                &fixture.scope,
                process_uuid,
                Some(&session_id),
                None,
                true,
            ))
            .await;
    }

    let claimed = fixture
        .store
        .active_process_for_session(&fixture.scope, &session_id, target)
        .await
        .expect("claim exact target")
        .expect("target row exists");
    let sibling_state = fixture
        .state(sibling)
        .await
        .expect("sibling remains visible");
    assert_eq!(sibling_state.reclaim_claimant_uuid, None);
    assert_eq!(sibling_state.reclaim_generation, None);
    assert_eq!(sibling_state.stop_reason, None);
    fixture
        .store
        .release_reclaim_claim(target, &claimed.reclaim_claim)
        .await
        .expect("release exact target claim");
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stale_source_requires_every_open_process_to_be_exactly_owned_and_terminal() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire source owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let failed = Uuid::now_v7();
    let healthy = Uuid::now_v7();
    for process_uuid in [failed, healthy] {
        fixture
            .seed(scoped_start(
                &fixture.scope,
                process_uuid,
                Some(&session_id),
                Some(lease.descriptor().process_runtime_owner()),
                true,
            ))
            .await;
    }
    seed_lane(
        &fixture.storage,
        &fixture.scope,
        &session_id,
        failed,
        "failed",
    )
    .await;
    seed_lane(
        &fixture.storage,
        &fixture.scope,
        &session_id,
        healthy,
        "running",
    )
    .await;
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        lease.descriptor().clone(),
    );
    assert!(source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("scan mixed-health exact owner set")
        .is_empty());
    drop(lease);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stale_source_skips_null_session_without_aborting_valid_sibling() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire source owner lease");
    let owner = lease.descriptor().process_runtime_owner();
    let null_process = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            null_process,
            None,
            Some(owner.clone()),
            true,
        ))
        .await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let valid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            valid,
            Some(&session_id),
            Some(owner),
            true,
        ))
        .await;
    seed_lane(
        &fixture.storage,
        &fixture.scope,
        &session_id,
        valid,
        "failed",
    )
    .await;
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        lease.descriptor().clone(),
    );
    let candidates = source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("null session row must not abort stale scan");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].session_id, session_id);
    assert_eq!(candidates[0].authorized_process_uuids, vec![valid]);
    drop(lease);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_dead_owner_probe_requires_two_corroborating_samples() {
    let fixture = Fixture::open().await;
    let prior = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire prior owner lease");
    let prior_descriptor = prior.descriptor().clone();
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            Some(prior_descriptor.process_runtime_owner()),
            true,
        ))
        .await;
    drop(prior);
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current owner lease");
    let gap = Duration::from_millis(30);
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        current.descriptor().clone(),
    )
    .with_dead_owner_confirmation_gap(gap);

    assert!(source
        .restart_session_process_sets()
        .await
        .expect("first dead-owner sample")
        .is_empty());
    tokio::time::sleep(gap + Duration::from_millis(10)).await;
    let second = source
        .restart_session_process_sets()
        .await
        .expect("second dead-owner sample");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].resource_scope, fixture.scope);
    assert_eq!(second[0].authorized_process_uuids, vec![process_uuid]);
    drop(current);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_same_scope_process_set_drift_rejects_the_entire_claim() {
    let fixture = Fixture::open().await;
    let prior = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire prior owner lease");
    let owner = prior.descriptor().process_runtime_owner();
    let session_id = format!("session-{}", Uuid::now_v7());
    let first = scoped_start(
        &fixture.scope,
        Uuid::now_v7(),
        Some(&session_id),
        Some(owner.clone()),
        true,
    );
    let second = scoped_start(
        &fixture.scope,
        Uuid::now_v7(),
        Some(&session_id),
        Some(owner),
        true,
    );
    fixture.seed(first.clone()).await;
    fixture.seed(second.clone()).await;
    drop(prior);
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current owner lease");
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        current.descriptor().clone(),
    )
    .with_dead_owner_confirmation_gap(Duration::ZERO);
    let surfaced = source
        .restart_session_process_sets()
        .await
        .expect("surface original complete process set");
    assert_eq!(surfaced.len(), 1);
    fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(
            ProcessStop::from_start(&second, Some(0)).with_stop_reason("fixture-drift"),
        )])
        .await
        .expect("create post-scan process-set drift");

    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));
    let report = reclaim
        .run_restart_orphan_session(
            &fixture.scope,
            &session_id,
            current.instance_id(),
            &surfaced[0].authorized_process_uuids,
        )
        .await
        .expect("set drift must fail closed as an empty atomic claim");
    assert!(report.processes_reclaimed.is_empty());
    assert!(killer.attempts().is_empty());
    assert!(fixture
        .state(first.process_uuid)
        .await
        .unwrap()
        .stopped_at
        .is_none());
    drop(current);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_boot_reconcile_is_fail_open_on_kill_failure_and_reports_it() {
    let fixture = Fixture::open().await;
    let prior = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire prior owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            Some(prior.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    drop(prior);
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current owner lease");
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        current.descriptor().clone(),
    )
    .with_dead_owner_confirmation_gap(Duration::ZERO);
    let killer = Arc::new(RecordingKill::failing());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));

    let report = reconcile_restart_orphans_at_boot(&reclaim, &source)
        .await
        .expect("kill failure is reported but does not abort resilient boot");
    assert_eq!(report.processes_reclaimed, 0);
    assert_eq!(report.processes_kill_failed, 1);
    let state = fixture.state(process_uuid).await.unwrap();
    assert_eq!(state.stopped_at, None);
    assert_eq!(state.stop_reason, None);
    drop(current);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_post_boot_periodic_task_resurfaces_restart_orphan() {
    let fixture = Fixture::open().await;
    let prior = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire prior owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            Some(prior.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    drop(prior);
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current owner lease");
    let source: Arc<dyn StaleSessionSource> = Arc::new(
        SurrealModelLaneStaleSessionSource::new(
            fixture.storage.clone(),
            current.descriptor().clone(),
        )
        .with_dead_owner_confirmation_gap(Duration::ZERO),
    );
    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = Arc::new(reclaim_with(
        Arc::clone(&fixture.store),
        Arc::clone(&killer),
    ));
    let task = spawn_managed_staleness_reclaim_task_after_boot(
        reclaim,
        source,
        StalenessReclaimConfig {
            ttl: Duration::from_secs(300),
            scan_interval: Duration::from_millis(20),
        },
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while killer.attempts().is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        killer
            .attempts()
            .into_iter()
            .map(|(_, process_uuid, _)| process_uuid)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([process_uuid])
    );
    assert!(task.shutdown_and_join(Duration::from_secs(1)).await);
    assert!(fixture
        .state(process_uuid)
        .await
        .unwrap()
        .stopped_at
        .is_some());
    drop(current);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_concurrent_reclaimers_claim_each_process_exactly_once() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let expected = BTreeSet::from([Uuid::now_v7(), Uuid::now_v7()]);
    for process_uuid in &expected {
        fixture
            .seed(scoped_start(
                &fixture.scope,
                *process_uuid,
                Some(&session_id),
                None,
                true,
            ))
            .await;
    }
    let first_store = Arc::clone(&fixture.store);
    let second_store = Arc::clone(&fixture.store);
    let first_scope = fixture.scope.clone();
    let second_scope = fixture.scope.clone();
    let first_session = session_id.clone();
    let second_session = session_id.clone();
    let first = tokio::spawn(async move {
        first_store
            .active_processes_for_session(&first_scope, &first_session)
            .await
    });
    let second = tokio::spawn(async move {
        second_store
            .active_processes_for_session(&second_scope, &second_session)
            .await
    });
    let first = first
        .await
        .expect("join first claimant")
        .expect("first claim");
    let second = second
        .await
        .expect("join second claimant")
        .expect("second claim");
    let mut observed = BTreeSet::new();
    for process in first.iter().chain(second.iter()) {
        assert!(
            observed.insert(process.process_uuid),
            "one process was returned to both concurrent claimants"
        );
    }
    assert_eq!(observed, expected);
    for process in first.iter().chain(second.iter()) {
        fixture
            .store
            .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
            .await
            .expect("release concurrency proof claim");
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_concurrent_reclaimers_produce_one_kill_and_one_durable_stop() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            None,
            true,
        ))
        .await;
    let killer = Arc::new(RecordingKill::succeeding());
    let first = Arc::new(reclaim_with(
        Arc::clone(&fixture.store),
        Arc::clone(&killer),
    ));
    let second = Arc::new(reclaim_with(
        Arc::clone(&fixture.store),
        Arc::clone(&killer),
    ));
    let first_scope = fixture.scope.clone();
    let second_scope = fixture.scope.clone();
    let first_session = session_id.clone();
    let second_session = session_id.clone();
    let first_run = tokio::spawn(async move {
        first
            .run(&first_scope, &first_session, ReclaimTrigger::Failure)
            .await
    });
    let second_run = tokio::spawn(async move {
        second
            .run(&second_scope, &second_session, ReclaimTrigger::Failure)
            .await
    });
    let first_report = first_run
        .await
        .expect("join first reclaimer")
        .expect("first reclaimer completes");
    let second_report = second_run
        .await
        .expect("join second reclaimer")
        .expect("second reclaimer completes");
    assert_eq!(
        first_report.processes_reclaimed.len() + second_report.processes_reclaimed.len(),
        1
    );
    assert_eq!(killer.attempts().len(), 1);
    let state = fixture.state(process_uuid).await.unwrap();
    assert!(state.stopped_at.is_some());
    assert_eq!(state.stop_reason.as_deref(), Some("reclaim"));
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stale_claimant_cannot_mutate_a_newer_exact_scope_claim() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            None,
            true,
        ))
        .await;
    let current = fixture
        .store
        .active_process_for_session(&fixture.scope, &session_id, process_uuid)
        .await
        .expect("claim exact row")
        .expect("row is claimable");
    let mut stale = current.reclaim_claim.clone();
    stale.claimant_uuid = Uuid::now_v7();
    stale.generation = stale.generation.saturating_sub(1);
    fixture
        .store
        .release_reclaim_claim(process_uuid, &stale)
        .await
        .expect_err("stale claimant must not release the current claim");
    fixture
        .store
        .mark_reclaim_kill_started(process_uuid, &stale)
        .await
        .expect_err("stale claimant must not advance the current claim");
    let state = fixture
        .state(process_uuid)
        .await
        .expect("read current claim");
    assert_eq!(
        state.reclaim_claimant_uuid,
        Some(current.reclaim_claim.claimant_uuid)
    );
    assert_eq!(
        state.reclaim_generation,
        i64::try_from(current.reclaim_claim.generation).ok()
    );
    fixture
        .store
        .release_reclaim_claim(process_uuid, &current.reclaim_claim)
        .await
        .expect("current claimant releases its own claim");
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stale_reclaim_preserves_same_owner_non_sandbox_sibling() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire source owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let sandboxed = Uuid::now_v7();
    let non_sandbox = Uuid::now_v7();
    let owner = lease.descriptor().process_runtime_owner();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            sandboxed,
            Some(&session_id),
            Some(owner.clone()),
            true,
        ))
        .await;
    fixture
        .seed(scoped_start(
            &fixture.scope,
            non_sandbox,
            Some(&session_id),
            Some(owner),
            false,
        ))
        .await;
    seed_lane(
        &fixture.storage,
        &fixture.scope,
        &session_id,
        sandboxed,
        "failed",
    )
    .await;
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        lease.descriptor().clone(),
    );
    let candidates = source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("surface exact sandbox process set");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].authorized_process_uuids, vec![sandboxed]);
    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));
    let (owner_instance, owner_host) = source
        .self_runtime_owner_scope()
        .expect("source exposes exact owner scope");
    reclaim
        .run_stale_owned_session(
            &fixture.scope,
            &session_id,
            owner_instance,
            &owner_host,
            &candidates[0].authorized_process_uuids,
        )
        .await
        .expect("reclaim only the authorized sandbox process");
    assert_eq!(
        killer
            .attempts()
            .into_iter()
            .map(|(_, process_uuid, _)| process_uuid)
            .collect::<Vec<_>>(),
        vec![sandboxed]
    );
    assert!(fixture
        .state(non_sandbox)
        .await
        .expect("non-sandbox sibling remains visible")
        .stopped_at
        .is_none());
    drop(lease);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stale_reclaim_preserves_foreign_owner_in_same_session() {
    let fixture = Fixture::open().await;
    let selected_owner = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire selected owner lease");
    let foreign_owner = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire foreign owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let selected = Uuid::now_v7();
    let foreign = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            selected,
            Some(&session_id),
            Some(selected_owner.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    fixture
        .seed(scoped_start(
            &fixture.scope,
            foreign,
            Some(&session_id),
            Some(foreign_owner.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    seed_lane(
        &fixture.storage,
        &fixture.scope,
        &session_id,
        selected,
        "failed",
    )
    .await;

    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));
    reclaim
        .run_stale_owned_session(
            &fixture.scope,
            &session_id,
            selected_owner.instance_id(),
            &selected_owner.descriptor().host_scope_id,
            &[selected],
        )
        .await
        .expect("reclaim only the explicitly owned exact process set");
    assert_eq!(
        killer
            .attempts()
            .into_iter()
            .map(|(_, process_uuid, _)| process_uuid)
            .collect::<Vec<_>>(),
        vec![selected]
    );
    assert!(fixture.state(selected).await.unwrap().stopped_at.is_some());
    assert!(fixture.state(foreign).await.unwrap().stopped_at.is_none());
    drop(foreign_owner);
    drop(selected_owner);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_stop_reservation_rejection_prevents_kill_and_releases_claim() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            None,
            true,
        ))
        .await;
    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = Reclaim::new(
        Arc::clone(&fixture.store),
        Arc::clone(&killer),
        Arc::new(RejectingStopWriter),
    );

    reclaim
        .run(&fixture.scope, &session_id, ReclaimTrigger::Failure)
        .await
        .expect_err("STOP reservation rejection must abort before kill");
    assert!(killer.attempts().is_empty());
    let state = fixture.state(process_uuid).await.unwrap();
    assert_eq!(state.stopped_at, None);
    assert_eq!(state.stop_reason, None);
    assert_eq!(state.reclaim_claimant_uuid, None);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_unresponsive_kill_is_bounded_and_writes_no_false_stop() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            None,
            true,
        ))
        .await;
    let killer = Arc::new(UnresponsiveKill::default());
    let reclaim = Reclaim::new(
        Arc::clone(&fixture.store),
        Arc::clone(&killer),
        Arc::new(DirectStopWriter {
            store: Arc::clone(&fixture.store),
        }),
    )
    .with_kill_timeout_for_test(Duration::from_millis(20));

    let started = Instant::now();
    let report = reclaim
        .run(&fixture.scope, &session_id, ReclaimTrigger::OperatorCancel)
        .await
        .expect("bounded timeout is a reported kill failure");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(matches!(
        &report.processes_reclaimed[0].kill_result,
        KillOutcome::Failed { .. }
    ));
    assert_eq!(killer.attempts.lock().unwrap().len(), 1);
    assert_eq!(killer.attempts.lock().unwrap()[0].0, fixture.scope);
    let state = fixture.state(process_uuid).await.unwrap();
    assert_eq!(state.stopped_at, None);
    assert_eq!(state.stop_reason, None);
    assert_eq!(state.reclaim_claimant_uuid, None);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_crash_after_kill_recovery_persists_stop_without_second_kill() {
    let fixture = Fixture::open().await;
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            None,
            true,
        ))
        .await;
    let claimed = fixture
        .store
        .active_process_for_session(&fixture.scope, &session_id, process_uuid)
        .await
        .expect("claim crash-recovery row")
        .expect("crash-recovery row exists");
    fixture
        .store
        .mark_reclaim_kill_started(process_uuid, &claimed.reclaim_claim)
        .await
        .expect("persist pre-crash kill-start fence");

    let killer = Arc::new(RecordingKill::succeeding());
    let reclaim = reclaim_with(Arc::clone(&fixture.store), Arc::clone(&killer));
    let sweep = reclaim
        .reconcile_in_progress_for_session(
            &fixture.scope,
            &session_id,
            Uuid::now_v7(),
            &[process_uuid],
        )
        .await
        .expect("reconcile authoritative succeeded operation");
    assert_eq!(sweep.operations.len(), 1);
    assert!(sweep.reclaim_error.is_none());
    assert!(sweep.reclaim_report.is_some());
    assert!(killer.attempts().is_empty(), "recovery must not re-kill");
    let state = fixture.state(process_uuid).await.unwrap();
    assert!(state.stopped_at.is_some());
    assert_eq!(state.stop_reason.as_deref(), Some("reclaim"));
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_restart_scan_never_surfaces_the_live_current_instance() {
    let fixture = Fixture::open().await;
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current owner lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    fixture
        .seed(scoped_start(
            &fixture.scope,
            Uuid::now_v7(),
            Some(&session_id),
            Some(current.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    let source = SurrealModelLaneStaleSessionSource::new(
        fixture.storage.clone(),
        current.descriptor().clone(),
    )
    .with_dead_owner_confirmation_gap(Duration::ZERO);
    assert!(source
        .restart_session_process_sets()
        .await
        .expect("scan live current owner")
        .is_empty());
    drop(current);
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_production_runtime_boot_timeout_fails_closed_on_injected_surreal_store() {
    let fixture = Fixture::open().await;
    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire timeout-proof runtime lease");
    let result = ProcessReclaimRuntime::production_with_lease(
        fixture.storage.clone(),
        fixture.scope.clone(),
        None,
        production_process_sandbox_registry(),
        lease,
        Duration::from_nanos(1),
    )
    .await;
    match result {
        Ok(runtime) => {
            let _ = runtime.shutdown_and_drain(Duration::from_secs(1)).await;
            panic!("a production boot reconcile exceeding its bound must fail closed");
        }
        Err(error) => assert!(
            error
                .to_string()
                .contains("process reclaim boot reconciliation exceeded"),
            "timeout failure must preserve its typed production diagnostic: {error}"
        ),
    }
    fixture.shutdown().await;
}

#[tokio::test]
async fn mt019_production_runtime_boot_surfaces_kill_failure_and_leaves_row_open() {
    let fixture = Fixture::open().await;
    let prior = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire prior runtime lease");
    let session_id = format!("session-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    fixture
        .seed(scoped_start(
            &fixture.scope,
            process_uuid,
            Some(&session_id),
            Some(prior.descriptor().process_runtime_owner()),
            true,
        ))
        .await;
    drop(prior);
    let current = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), "mt019-host")
        .expect("acquire current runtime lease");
    set_dead_owner_confirmation_gap_override_for_test(Some(Duration::ZERO));
    let _gap_reset = DeadOwnerGapReset;
    let empty_registry = Arc::new(SandboxAdapterRegistry::new(AdapterId::new(
        "mt019-test-adapter",
    )));
    let runtime = ProcessReclaimRuntime::production_with_lease(
        fixture.storage.clone(),
        fixture.scope.clone(),
        None,
        empty_registry,
        current,
        Duration::from_secs(30),
    )
    .await
    .expect("production boot remains available after a truthful kill failure");
    let report = runtime.boot_reconcile_report();
    assert_eq!(report.sessions_reconciled, 1);
    assert_eq!(report.processes_reclaimed, 0);
    assert_eq!(report.processes_kill_failed, 1);
    let state = fixture.state(process_uuid).await.unwrap();
    assert_eq!(state.stopped_at, None);
    assert_eq!(state.stop_reason, None);
    let drained = runtime.shutdown_and_drain(Duration::from_secs(10)).await;
    assert!(drained.reclaim_task_quiesced);
    assert!(matches!(drained.ledger, LedgerDrainJoinOutcome::Flushed));
    fixture.shutdown().await;
}

#[cfg(windows)]
mod windows_real_process {
    use std::{
        os::windows::process::CommandExt,
        path::PathBuf,
        process::{Child, Command, Stdio},
    };

    use handshake_core::sandbox::{
        process_creation_time_100ns, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
    };
    use sha2::{Digest, Sha256};

    use super::*;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    struct ChildGuard(Option<Child>);

    impl ChildGuard {
        fn pid(&self) -> u32 {
            self.0.as_ref().expect("child present").id()
        }

        fn is_running(&mut self) -> bool {
            self.0
                .as_mut()
                .expect("child present")
                .try_wait()
                .expect("query child state")
                .is_none()
        }

        fn wait_exited(&mut self, timeout: Duration) -> bool {
            let deadline = Instant::now() + timeout;
            while Instant::now() < deadline {
                if !self.is_running() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            false
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn powershell_path() -> PathBuf {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:/Windows".to_owned());
        let path =
            PathBuf::from(system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        assert!(path.is_file(), "required real-process executable is absent");
        path
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt019_production_registry_kills_real_child_and_durably_closes_surreal_lifecycle() {
        let fixture = Fixture::open().await;
        let executable = powershell_path();
        let child = Command::new(&executable)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 600",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn MT-019 real child");
        let mut child = ChildGuard(Some(child));
        let process_uuid = Uuid::now_v7();
        let session_id = format!("session-{}", Uuid::now_v7());
        let mut metadata = scope_metadata(&fixture.scope);
        metadata["effective_executable_sha256"] = Value::String(hex::encode(Sha256::digest(
            std::fs::read(&executable).expect("read real executable"),
        )));
        metadata["os_creation_time_100ns"] = Value::from(
            process_creation_time_100ns(child.pid()).expect("capture child generation identity"),
        );
        metadata["sandbox_handle_id"] = Value::String(process_uuid.to_string());
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "MT-019-REAL-PROCESS",
            Some("WP-1".to_owned()),
        )
        .with_process_uuid(process_uuid)
        .with_os_pid(child.pid())
        .with_parent_session_id(&session_id)
        .with_sandbox_adapter_id(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
        .with_sandbox_internal_id(process_uuid.to_string())
        .with_metadata_jsonb(metadata);
        fixture.seed(start).await;
        assert!(child.is_running());

        let killer = Arc::new(ProductionSandboxKill::with_registry(
            fixture.storage.clone(),
            production_process_sandbox_registry(),
        ));
        let reclaim = Reclaim::new(
            Arc::clone(&fixture.store),
            killer,
            Arc::new(DirectStopWriter {
                store: Arc::clone(&fixture.store),
            }),
        )
        .with_kill_timeout_for_test(Duration::from_secs(30));
        let report = reclaim
            .run(&fixture.scope, &session_id, ReclaimTrigger::Restart)
            .await
            .expect("production registry real-process reclaim");
        assert_eq!(report.processes_reclaimed.len(), 1);
        assert!(matches!(
            &report.processes_reclaimed[0].kill_result,
            KillOutcome::Killed
        ));
        assert!(child.wait_exited(Duration::from_secs(10)));
        let state = fixture.state(process_uuid).await.unwrap();
        assert!(state.stopped_at.is_some());
        assert_eq!(state.stop_reason.as_deref(), Some("reclaim"));
        assert_eq!(state.process_uuid, process_uuid);
        assert_eq!(
            state.metadata["owner_account_id"],
            fixture.scope.account_uuid.to_string()
        );
        fixture.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt019_pid_generation_mismatch_refuses_kill_and_writes_no_false_stop() {
        let fixture = Fixture::open().await;
        let executable = powershell_path();
        let child = Command::new(&executable)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 600",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn MT-019 mismatch child");
        let mut child = ChildGuard(Some(child));
        let process_uuid = Uuid::now_v7();
        let session_id = format!("session-{}", Uuid::now_v7());
        let actual_creation =
            process_creation_time_100ns(child.pid()).expect("capture child generation identity");
        let mut metadata = scope_metadata(&fixture.scope);
        metadata["effective_executable_sha256"] = Value::String(hex::encode(Sha256::digest(
            std::fs::read(&executable).expect("read real executable"),
        )));
        metadata["os_creation_time_100ns"] = Value::from(actual_creation.saturating_add(1));
        metadata["sandbox_handle_id"] = Value::String(process_uuid.to_string());
        fixture
            .seed(
                ProcessStart::new(
                    ProcessEngineKind::OfficialCliBridge,
                    "MT-019-PID-FENCE",
                    Some("WP-1".to_owned()),
                )
                .with_process_uuid(process_uuid)
                .with_os_pid(child.pid())
                .with_parent_session_id(&session_id)
                .with_sandbox_adapter_id(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
                .with_sandbox_internal_id(process_uuid.to_string())
                .with_metadata_jsonb(metadata),
            )
            .await;
        let reclaim = Reclaim::new(
            Arc::clone(&fixture.store),
            Arc::new(ProductionSandboxKill::with_registry(
                fixture.storage.clone(),
                production_process_sandbox_registry(),
            )),
            Arc::new(DirectStopWriter {
                store: Arc::clone(&fixture.store),
            }),
        );
        let report = reclaim
            .run(&fixture.scope, &session_id, ReclaimTrigger::Restart)
            .await
            .expect("identity mismatch is a fail-open kill outcome");
        assert!(matches!(
            &report.processes_reclaimed[0].kill_result,
            KillOutcome::Failed { .. }
        ));
        assert!(
            child.is_running(),
            "mismatched process generation must survive"
        );
        let state = fixture.state(process_uuid).await.unwrap();
        assert_eq!(state.stopped_at, None);
        assert_eq!(state.stop_reason, None);
        fixture.shutdown().await;
    }
}
