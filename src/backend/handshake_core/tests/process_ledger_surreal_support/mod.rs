#[path = "../surreal_test_store_support/mod.rs"]
mod surreal_test_store_support;

use std::{collections::BTreeSet, sync::{Arc, Mutex}, time::Duration};

use chrono::{DateTime, Utc};
use handshake_core::{
    process_ledger::{
        ProcessLedgerError, ProcessLedgerStore, ReclaimResourceScope,
        acquire_embedded_runtime_instance_lease, EmbeddedRuntimeInstanceLease,
        RetainedLedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome, NoopOverflowSink,
        ProductionSandboxKill, ScopedSandboxKill, ScopedReclaim, ScopedKillOutcome,
        StaleSessionSource, StaleSessionProcessSet, ReclaimKillOperation,
        ReclaimKillOperationStatus, KillError, LedgerEventKind, reconcile_restart_orphans_at_boot,
    },
    storage::surreal::{SurrealStorage, SurrealProcessLedgerStore, SurrealModelLaneStaleSessionSource},
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
        WorkspaceScopeRef,
    },
};
use serde_json::{json, Value};
use surreal_test_store_support::IsolatedSurrealTestStore;
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

pub struct ProcessLedgerSurrealHarness {
    allocator: IsolatedSurrealTestStore,
    storage: SurrealStorage,
    store: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
    restart: Arc<RestartResources>,
    restart_faults: Mutex<Vec<Arc<BlockingScopedKill>>>,
}

impl ProcessLedgerSurrealHarness {
    pub async fn open() -> Self {
        Self::open_with_scope(exact_scope()).await
    }

    pub async fn open_with_scope(scope: ReclaimResourceScope) -> Self {
        let allocator = IsolatedSurrealTestStore::create()
            .await.expect("allocate isolated ProcessLedger Surreal store");
        let storage = allocator.storage().clone();
        let store = Arc::new(SurrealProcessLedgerStore::open(storage.clone()).await
            .expect("open canonical scoped ProcessLedger facade"));
        let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(),
            format!("process-ledger-test-{}", scope.workspace_id))
            .expect("acquire current test runtime lease");
        let ledger = RetainedLedgerBatcher::spawn_with_runtime_owner(
            store.clone(), Arc::new(NoopOverflowSink), LedgerBatcherConfig::default(),
            lease.descriptor().process_runtime_owner());
        let restart = Arc::new(RestartResources { ledger, lease: Some(lease) });
        Self {
            allocator,
            storage,
            store,
            scope,
            restart,
            restart_faults: Mutex::new(Vec::new()),
        }
    }

    pub fn storage(&self) -> SurrealStorage {
        self.storage.clone()
    }

    pub fn store(&self) -> Arc<SurrealProcessLedgerStore> {
        self.store.clone()
    }

    pub async fn restart_runner(
        &self,
    ) -> handshake_core::process_ledger::restart_resume::SurrealRestartResumeRunner {
        self.restart_runner_with_fault(None).await
    }

    pub fn restart_host_scope(&self) -> String {
        self.restart.lease().descriptor().host_scope_id.clone()
    }

    pub async fn restart_runner_with_fault(&self, fault: Option<Arc<BlockingScopedKill>>) ->
        handshake_core::process_ledger::restart_resume::SurrealRestartResumeRunner {
        if let Some(fault) = &fault {
            self.restart_faults.lock().expect("restart fault registry lock").push(fault.clone());
        }
        let registry =
            Arc::new(handshake_core::sandbox::build_startup_recovery_registry_async().await);
        let killer = Arc::new(FixtureScopedKill {
            production: ProductionSandboxKill::with_registry(self.storage.clone(), registry), fault,
        });
        let reclaim = ScopedReclaim::new(self.store.clone(), killer,
            Arc::new(self.restart.ledger.ledger()));
        let source = Arc::new(SurrealModelLaneStaleSessionSource::new(
            self.storage.clone(), self.restart.lease().descriptor().clone())
            .with_dead_owner_confirmation_gap(Duration::ZERO));
        let bridge = Arc::new(ScopedRestartBridge {
            storage: self.storage.clone(), store: self.store.clone(), scope: self.scope.clone(),
            reclaim, source, _resources: self.restart.clone(),
        });
        handshake_core::process_ledger::restart_resume::SurrealRestartResumeRunner::new(
            self.storage.clone(), self.scope.clone(), bridge)
    }

    pub fn resource_scope(&self) -> &ReclaimResourceScope {
        &self.scope
    }

    pub fn model_resource_scope(&self) -> ResourceScope {
        ResourceScope::new(
            OwnerAccountId::from_uuid(self.scope.account_uuid),
            ActorPrincipalId::from_uuid(self.scope.actor_uuid),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(self.scope.session_uuid))
        .with_access_space(AccessSpaceRef::from_uuid(self.scope.access_space_uuid))
        .with_workspace(
            WorkspaceScopeRef::new(self.scope.workspace_id.clone())
                .expect("nonblank ProcessLedger test workspace"),
        )
    }

    pub fn metadata(&self) -> Value {
        scope_metadata(&self.scope)
    }

    pub async fn write_batch(
        &self,
        events: Vec<handshake_core::process_ledger::LedgerEvent>,
    ) -> Result<(), ProcessLedgerError> {
        self.store.write_batch(events).await
    }

    pub async fn lifecycle(&self, process_uuid: Uuid) -> Option<LifecycleProbe> {
        let bindings = ExactRecordBindings::new(
            &self.scope,
            RecordId::new("kernel_process_lifecycle", process_uuid.to_string()),
        );
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleProbe, _>(READ_EXACT_LIFECYCLE, bindings)
                        .await.map(|rows| rows.into_iter().next())
                })
            })
            .await
            .expect("read exact-scope lifecycle")
    }

    pub async fn lifecycle_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_LIFECYCLES).await
    }

    pub async fn lifecycles(&self) -> Vec<LifecycleProbe> {
        let bindings = ExactScopeBindings::new(&self.scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleProbe, _>(READ_EXACT_LIFECYCLES, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope lifecycles")
    }

    pub async fn open_lifecycle_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_OPEN_LIFECYCLES).await
    }

    pub async fn process_event_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_PROCESS_EVENTS).await
    }

    pub async fn lifecycles_for_artifact(&self, artifact_sha256: &str) -> Vec<LifecycleProbe> {
        let bindings = ExactArtifactBindings::new(&self.scope, artifact_sha256);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleProbe, _>(READ_EXACT_ARTIFACT_LIFECYCLES, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope artifact lifecycles")
    }

    async fn exact_count(&self, statement: &'static str) -> i64 {
        let bindings = ExactScopeBindings::new(&self.scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move { database.query_values::<i64, _>(statement, bindings).await.map(|rows| rows.into_iter().next()) })
            })
            .await
            .expect("count exact-scope ProcessLedger rows")
            .expect("exact-scope count query returns one value")
    }

    pub async fn close(mut self) {
        let faults = std::mem::take(self.restart_faults.get_mut().expect("restart fault registry lock"));
        for fault in faults {
            if fault.operation.lock().expect("scoped fault operation lock").is_some() {
                fault.release.notify_one();
                tokio::time::timeout(Duration::from_secs(5), fault.finished.notified()).await
                    .expect("bounded scoped fault operation exits before database close");
            }
        }
        assert!(matches!(self.restart.ledger.drain_and_join(Duration::from_secs(5)).await,
            LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained),
            "retained scoped restart writer must drain before database close");
        let scope_path = self.allocator.scope_path().to_owned();
        let marker_path = self.allocator.owner_marker_path_for_proof().to_owned();
        drop(self.store);
        drop(self.storage);
        drop(self.restart);
        let diagnostics = self.allocator.cleanup().await
            .expect("acknowledge engine close and clean owned ProcessLedger store");
        assert!(diagnostics.database_absent);
        assert!(diagnostics.namespace_absent_after_reopen);
        assert!(diagnostics.error.is_none());
        assert!(!scope_path.exists(), "closed owned database directory is absent");
        assert!(!marker_path.exists(), "closed owned store ownership marker is absent");
    }
}

struct RestartResources {
    ledger: RetainedLedgerBatcher,
    lease: Option<EmbeddedRuntimeInstanceLease>,
}

impl RestartResources {
    fn lease(&self) -> &EmbeddedRuntimeInstanceLease {
        self.lease.as_ref().expect("retained current runtime lease")
    }
}

impl Drop for RestartResources {
    fn drop(&mut self) {
        if !self.ledger.abort_and_join_blocking(Duration::from_secs(5)) {
            // A writer whose terminal state is unknown must retain its liveness lease.
            if let Some(lease) = self.lease.take() { std::mem::forget(lease); }
        }
    }
}

pub struct BlockingScopedKill {
    scope: ReclaimResourceScope,
    process_uuid: Uuid,
    entered: Arc<std::sync::atomic::AtomicBool>,
    operation: Mutex<Option<ReclaimKillOperation>>,
    release: tokio::sync::Notify,
    finished: tokio::sync::Notify,
}

impl BlockingScopedKill {
    pub fn new(scope: ReclaimResourceScope, process_uuid: Uuid,
        entered: Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self { scope, process_uuid, entered, operation: Mutex::new(None),
            release: tokio::sync::Notify::new(), finished: tokio::sync::Notify::new() }
    }
}

struct FixtureScopedKill {
    production: ProductionSandboxKill,
    fault: Option<Arc<BlockingScopedKill>>,
}

#[async_trait::async_trait]
impl ScopedSandboxKill for FixtureScopedKill {
    async fn kill(&self, scope: &ReclaimResourceScope, process_uuid: Uuid,
        kill_operation_uuid: Uuid) -> Result<(), KillError> {
        if let Some(fault) = &self.fault {
            if fault.scope == *scope && fault.process_uuid == process_uuid {
                *fault.operation.lock().expect("scoped fault operation lock") = Some(
                    ReclaimKillOperation { resource_scope: scope.clone(), process_uuid, kill_operation_uuid });
                fault.entered.store(true, std::sync::atomic::Ordering::Release);
                fault.release.notified().await;
                fault.finished.notify_one();
                return Err(KillError::new("bounded scoped fixture teardown released pending operation"));
            }
        }
        self.production.kill(scope, process_uuid, kill_operation_uuid).await
    }

    async fn kill_operation_status(&self, scope: &ReclaimResourceScope, process_uuid: Uuid,
        kill_operation_uuid: Uuid) -> Result<ReclaimKillOperationStatus, KillError> {
        if let Some(fault) = &self.fault {
            let operation = ReclaimKillOperation { resource_scope: scope.clone(), process_uuid, kill_operation_uuid };
            if fault.operation.lock().expect("scoped fault operation lock").as_ref() == Some(&operation) {
                return Ok(ReclaimKillOperationStatus::InProgress);
            }
        }
        self.production.kill_operation_status(scope, process_uuid, kill_operation_uuid).await
    }
}

struct FilteredRestartSource {
    source: Arc<SurrealModelLaneStaleSessionSource>,
    session: String,
    scope: ReclaimResourceScope,
    authorized: BTreeSet<Uuid>,
}

impl FilteredRestartSource {
    fn filter(&self, candidates: Vec<StaleSessionProcessSet>) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        let selected: Vec<_> = candidates.into_iter()
            .filter(|candidate| candidate.session_id == self.session && candidate.resource_scope == self.scope)
            .collect();
        if selected.len() != 1 || selected[0].authorized_process_uuids.len() != self.authorized.len()
            || selected[0].authorized_process_uuids.iter().copied().collect::<BTreeSet<_>>() != self.authorized {
            return Err(ProcessLedgerError::Store("restart canonical open UUID coverage is not authorized by real stale source".into()));
        }
        Ok(selected)
    }
}

#[async_trait::async_trait]
impl StaleSessionSource for FilteredRestartSource {
    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(self.source.stale_sessions(ttl).await?.into_iter().filter(|id| id == &self.session).collect())
    }
    async fn stale_session_process_sets(&self, ttl: Duration) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        self.filter(self.source.stale_session_process_sets(ttl).await?)
    }
    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(self.source.restart_sessions().await?.into_iter().filter(|id| id == &self.session).collect())
    }
    async fn restart_session_process_sets(&self) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        self.filter(self.source.restart_session_process_sets().await?)
    }
    fn self_runtime_instance_id(&self) -> Option<Uuid> { self.source.self_runtime_instance_id() }
    fn self_runtime_owner_scope(&self) -> Option<(Uuid, String)> { self.source.self_runtime_owner_scope() }
}

struct ScopedRestartBridge {
    storage: SurrealStorage,
    store: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
    reclaim: ScopedReclaim,
    source: Arc<SurrealModelLaneStaleSessionSource>,
    _resources: Arc<RestartResources>,
}

#[derive(SurrealValue)]
struct RestartProbeBindings {
    session: String, owner: String, actor: String, authenticated: String, space: String, workspace: String,
}

#[derive(SurrealValue)]
struct RestartProbe {
    process_uuid: Uuid, started_at: DateTime<Utc>, stopped_at: Option<DateTime<Utc>>,
    reclaim_generation: Option<i64>, parent_session_id: String,
    owner_account_id: Option<String>, actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>, access_space_id: Option<String>, workspace_id: Option<String>,
}

impl ScopedRestartBridge {
    async fn rows(&self, session: &str, statement: &'static str) -> Result<Vec<RestartProbe>,
        handshake_core::process_ledger::restart_resume::RestartResumeRuntimeError> {
        let bindings = RestartProbeBindings { session: session.into(), owner: self.scope.account_uuid.to_string(),
            actor: self.scope.actor_uuid.to_string(), authenticated: self.scope.session_uuid.to_string(),
            space: self.scope.access_space_uuid.to_string(), workspace: self.scope.workspace_id.clone() };
        Ok(self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<RestartProbe, _>(statement, bindings).await
        })).await?)
    }
    fn failure(error: impl ToString) -> handshake_core::process_ledger::restart_resume::RestartResumeRuntimeError {
        handshake_core::process_ledger::restart_resume::RestartResumeRuntimeError::SideEffectFailed {
            table: "kernel_process_lifecycle".into(), error: error.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl handshake_core::process_ledger::restart_resume::RestartOrphanReclaimer for ScopedRestartBridge {
    async fn reclaim_session(&self, session: &str) -> Result<u32,
        handshake_core::process_ledger::restart_resume::RestartResumeRuntimeError> {
        let before = self.rows(session, READ_RESTART_SESSION).await?;
        let open: Vec<_> = before.iter().filter(|row| row.stopped_at.is_none()).collect();
        if open.is_empty() { return Ok(0); }
        for row in &open {
            if row.owner_account_id.as_deref() != Some(self.scope.account_uuid.to_string().as_str())
                || row.actor_principal_id.as_deref() != Some(self.scope.actor_uuid.to_string().as_str())
                || row.authenticated_session_id.as_deref() != Some(self.scope.session_uuid.to_string().as_str())
                || row.access_space_id.as_deref() != Some(self.scope.access_space_uuid.to_string().as_str())
                || row.workspace_id.as_deref() != Some(self.scope.workspace_id.as_str()) {
                return Err(Self::failure("restart session contains an open row outside exact fixture scope"));
            }
            let inspected = self.store.inspect_ownership_by_process_uuid(&self.scope, row.process_uuid)
                .await.map_err(Self::failure)?.ok_or_else(|| Self::failure("canonical open ownership absent"))?;
            if inspected.lifecycle_state != LedgerEventKind::Start || inspected.started_at != row.started_at {
                return Err(Self::failure("canonical pre-read lifecycle generation mismatch"));
            }
        }
        let authorized: BTreeSet<_> = open.iter().map(|row| row.process_uuid).collect();
        if authorized.len() != open.len() { return Err(Self::failure("duplicate canonical open UUID")); }
        let filtered = FilteredRestartSource { source: self.source.clone(), session: session.into(),
            scope: self.scope.clone(), authorized: authorized.clone() };
        // Validate complete authorization before the production sweep can mutate a row.
        filtered.restart_session_process_sets().await.map_err(Self::failure)?;
        let report = reconcile_restart_orphans_at_boot(&self.reclaim, &filtered).await.map_err(Self::failure)?;
        if report.cancelled || report.processes_kill_failed != 0 || !report.session_errors.is_empty()
            || !report.sweep_reclaim_errors.is_empty() || report.reclaim_reports.iter().any(|report|
                report.processes_reclaimed.iter().any(|process| process.kill_result != ScopedKillOutcome::Killed)) {
            return Err(Self::failure("restart reclaim failed or STOP acknowledgement remains pending"));
        }
        let after = self.rows(session, READ_EXACT_RESTART_SESSION).await?;
        let mut terminal = BTreeSet::new();
        for original in open {
            let rows: Vec<_> = after.iter().filter(|row| row.process_uuid == original.process_uuid).collect();
            if rows.len() != 1 || rows[0].parent_session_id != session || rows[0].started_at != original.started_at
                || rows[0].stopped_at.is_none() || rows[0].reclaim_generation <= original.reclaim_generation {
                return Err(Self::failure("restart exact session/generation STOP readback incomplete"));
            }
            let inspected = self.store.inspect_ownership_by_process_uuid(&self.scope, original.process_uuid)
                .await.map_err(Self::failure)?.ok_or_else(|| Self::failure("canonical STOP ownership absent"))?;
            if inspected.lifecycle_state != LedgerEventKind::Stop || inspected.started_at != original.started_at
                || inspected.stopped_at != rows[0].stopped_at || inspected.stop_reason.as_deref() != Some("reclaim") {
                return Err(Self::failure("restart linked canonical STOP evidence incomplete"));
            }
            terminal.insert(original.process_uuid);
        }
        if terminal != authorized { return Err(Self::failure("restart STOP UUID coverage incomplete")); }
        u32::try_from(terminal.len()).map_err(Self::failure)
    }
}

const READ_RESTART_SESSION: &str = "SELECT process_uuid, started_at, stopped_at, reclaim_generation, parent_session_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM kernel_process_lifecycle WHERE parent_session_id = $session;";
const READ_EXACT_RESTART_SESSION: &str = "SELECT process_uuid, started_at, stopped_at, reclaim_generation, parent_session_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM kernel_process_lifecycle WHERE parent_session_id = $session AND owner_account_id = $owner AND actor_principal_id = $actor AND authenticated_session_id = $authenticated AND access_space_id = $space AND workspace_id = $workspace;";

pub fn exact_scope() -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: Uuid::now_v7(),
        actor_uuid: Uuid::now_v7(),
        session_uuid: Uuid::now_v7(),
        workspace_id: format!("workspace-{}", Uuid::now_v7()),
        access_space_uuid: Uuid::now_v7(),
    }
}

pub fn scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id.clone(),
    })
}

#[derive(Clone, Debug, SurrealValue)]
pub struct LifecycleProbe {
    pub process_uuid: Uuid,
    pub os_pid: Option<i64>,
    pub engine_kind: String,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i64>,
    pub stop_reason: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub owner_role: String,
    pub owner_runtime_instance_id: Option<Uuid>,
    pub owner_host_scope_id: Option<String>,
    pub owner_lease_schema_id: Option<String>,
    pub owner_lease_protocol: Option<String>,
    pub owner_lease_address: Option<String>,
    pub owner_lease_port: Option<i64>,
    pub event_ledger_event_id: Option<RecordId>,
    pub metadata: Value,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactScopeBindings {
    fn new(scope: &ReclaimResourceScope) -> Self {
        Self {
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ExactRecordBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactRecordBindings {
    fn new(scope: &ReclaimResourceScope, record: RecordId) -> Self {
        let scope = ExactScopeBindings::new(scope);
        Self {
            record,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ExactArtifactBindings {
    artifact_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactArtifactBindings {
    fn new(scope: &ReclaimResourceScope, artifact_sha256: &str) -> Self {
        let scope = ExactScopeBindings::new(scope);
        Self {
            artifact_sha256: artifact_sha256.to_string(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }
}

const READ_EXACT_LIFECYCLE: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
-- `FROM $record`, not `FROM ONLY $record`: an exact-scope probe must observe
-- ABSENCE (rejected write, foreign scope) as `None`. `ONLY` yields SurrealDB
-- `NONE`, which the typed row decoder rejects as "Expected object, got none".
FROM $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const READ_EXACT_ARTIFACT_LIFECYCLES: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
FROM kernel_process_lifecycle
WHERE model_artifact_sha256 = $artifact_sha256
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const READ_EXACT_LIFECYCLES: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
FROM kernel_process_lifecycle
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const COUNT_EXACT_LIFECYCLES: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_process_lifecycle
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id);
"#;

const COUNT_EXACT_OPEN_LIFECYCLES: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_process_lifecycle
WHERE stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id);
"#;

const COUNT_EXACT_PROCESS_EVENTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_event_ledger
WHERE source_component = 'process_ledger'
    AND payload.metadata_jsonb.owner_account_id = $owner_account_id
    AND payload.metadata_jsonb.actor_principal_id = $actor_principal_id
    AND payload.metadata_jsonb.authenticated_session_id = $authenticated_session_id
    AND payload.metadata_jsonb.access_space_id = $access_space_id
    AND payload.metadata_jsonb.workspace_id = $workspace_id);
"#;
