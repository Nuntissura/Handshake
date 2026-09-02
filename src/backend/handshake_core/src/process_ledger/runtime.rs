use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use surrealdb::types::SurrealValue;
use tokio::time;
use uuid::Uuid;

use crate::sandbox::{
    AdapterId, HandshakeNativeSandboxAdapter, SandboxAdapterRegistry,
    HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};
use crate::storage::surreal::SurrealStorage;

use super::{
    acquire_embedded_runtime_instance_lease,
    reclaim::spawn_managed_staleness_reclaim_task_after_boot, reconcile_restart_orphans_at_boot,
    EmbeddedRuntimeInstanceDescriptor, EmbeddedRuntimeInstanceLease, LedgerBatcherConfig,
    LedgerDrainJoinOutcome, ManagedStalenessReclaimTask, NoopOverflowSink, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessLedgerStore, ProductionSandboxKill, Reclaim,
    ReclaimResourceScope, RestartOrphanBootReconcileReport, RetainedLedgerBatcher,
    StaleSessionSource, StalenessReclaimConfig, SurrealModelLaneStaleSessionSource,
    SurrealProcessLedgerStore,
};

#[derive(Debug)]
pub struct ProcessReclaimRuntimeDrainReport {
    pub reclaim_task_quiesced: bool,
    pub ledger: LedgerDrainJoinOutcome,
    pub lease_released: bool,
    /// MT-019 P-4(a): why the OS liveness lease was retained past drain. `None`
    /// means it was released because this instance provably owns zero open
    /// lifecycle rows.
    pub lease_retained_reason: Option<String>,
}

/// One host-process composition root for process lifecycle writes, sandbox
/// reclaim, boot reconciliation, and the OS liveness lease. Backend and Tauri
/// construct this same type and pass its ledger/reclaimer/registry downward;
/// no product lane owns a second process-ledger writer or reclaimer.
#[derive(Clone)]
pub struct ProcessReclaimRuntime {
    inner: Arc<ProcessReclaimRuntimeInner>,
}

struct ProcessReclaimRuntimeInner {
    storage: SurrealStorage,
    resource_scope: ReclaimResourceScope,
    ledger: RetainedLedgerBatcher,
    reclaim: Arc<Reclaim>,
    registry: Arc<SandboxAdapterRegistry>,
    runtime_instance: EmbeddedRuntimeInstanceDescriptor,
    runtime_lease: Mutex<Option<EmbeddedRuntimeInstanceLease>>,
    reclaim_task: ManagedStalenessReclaimTask,
    boot_reconcile: RestartOrphanBootReconcileReport,
}

#[derive(Debug, SurrealValue)]
struct OpenRowCountBindings {
    runtime_instance_id: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const OPEN_ROW_COUNT_FOR_RUNTIME: &str = r#"
RETURN array::len(
    SELECT VALUE id FROM kernel_process_lifecycle
    WHERE stopped_at = NONE
        AND owner_runtime_instance_id = $runtime_instance_id
        AND owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id
);
"#;

impl ProcessReclaimRuntime {
    pub async fn production(
        storage: SurrealStorage,
        resource_scope: ReclaimResourceScope,
        overflow_sink: Option<Arc<dyn ProcessLedgerOverflowSink>>,
        registry: Arc<SandboxAdapterRegistry>,
        host_scope_id: impl Into<String>,
        startup_timeout: Duration,
    ) -> Result<Self, ProcessLedgerError> {
        let runtime_lease =
            acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope_id.into())?;
        Self::production_with_lease(
            storage,
            resource_scope,
            overflow_sink,
            registry,
            runtime_lease,
            startup_timeout,
        )
        .await
    }

    pub async fn production_with_lease(
        storage: SurrealStorage,
        resource_scope: ReclaimResourceScope,
        overflow_sink: Option<Arc<dyn ProcessLedgerOverflowSink>>,
        registry: Arc<SandboxAdapterRegistry>,
        runtime_lease: EmbeddedRuntimeInstanceLease,
        startup_timeout: Duration,
    ) -> Result<Self, ProcessLedgerError> {
        let runtime_instance = runtime_lease.descriptor().clone();
        let process_store = Arc::new(SurrealProcessLedgerStore::open(storage.clone()).await?);
        let ledger_store: Arc<dyn ProcessLedgerStore> = process_store.clone();
        let ledger = RetainedLedgerBatcher::spawn_with_runtime_owner(
            ledger_store,
            overflow_sink.unwrap_or_else(|| Arc::new(NoopOverflowSink)),
            LedgerBatcherConfig::default(),
            runtime_instance.process_runtime_owner(),
        );
        let killer = Arc::new(ProductionSandboxKill::with_registry(
            storage.clone(),
            Arc::clone(&registry),
        ));
        let reclaim = Arc::new(Reclaim::new(
            process_store,
            killer,
            Arc::new(ledger.ledger()),
        ));
        let stale_source: Arc<dyn StaleSessionSource> = Arc::new(
            SurrealModelLaneStaleSessionSource::new(storage.clone(), runtime_instance.clone()),
        );

        // The composed boot restart-reconcile pass is a named, callable function
        // (`reconcile_restart_orphans_at_boot`) so the exact production path —
        // `restart_sessions` -> in-progress reconcile -> restart reclaim — is
        // reachable from an integration test instead of being an unreachable
        // inline block. It surfaces generic spawned-process orphans (including
        // Official-CLI bridge children) whose owning runtime instance is provably
        // dead.
        //
        // MT-019 F3 + F5 (recorded operator decision: RESILIENT BOOT): coverage
        // is NOT total, and this comment previously claimed it was. Boot is
        // fail-open on kill failure — the row stays truthfully open, its claim is
        // released, its fence cleared, and NO false STOP is written — because the
        // native reclaim adapter is Windows-only and would otherwise brick boot
        // on every other platform, and because fail-closed cannot distinguish a
        // correct refusal-to-kill (pid-reuse guard) from a genuine reap failure.
        // Kill failures are therefore counted separately, surfaced out of boot
        // via `boot_reconcile_report()` plus a warn-level log, and retried by the
        // periodic restart tick. Store/surfacing errors and a boot TIMEOUT remain
        // fail-closed and abort startup.
        let boot_reconcile =
            reconcile_restart_orphans_at_boot(reclaim.as_ref(), stale_source.as_ref());
        let (boot_report, boot_error) = match time::timeout(startup_timeout, boot_reconcile).await {
            Ok(Ok(report)) => (report, None),
            Ok(Err(error)) => (RestartOrphanBootReconcileReport::default(), Some(error)),
            Err(_) => (
                RestartOrphanBootReconcileReport::default(),
                Some(ProcessLedgerError::Store(format!(
                    "process reclaim boot reconciliation exceeded {} ms",
                    startup_timeout.as_millis()
                ))),
            ),
        };
        if let Some(error) = boot_error {
            let writer_stopped = ledger.abort_and_join_blocking(startup_timeout);
            if writer_stopped {
                drop(runtime_lease);
            } else {
                // Fail closed: a writer whose terminal state was not observed
                // must remain protected by the exact OS lease until process exit.
                std::mem::forget(runtime_lease);
            }
            return Err(error);
        }

        if boot_report.processes_kill_failed > 0
            || !boot_report.sweep_reclaim_errors.is_empty()
            || !boot_report.session_errors.is_empty()
        {
            tracing::warn!(
                target: "handshake_core::process_ledger",
                sessions_reconciled = boot_report.sessions_reconciled,
                processes_reclaimed = boot_report.processes_reclaimed,
                processes_kill_failed = boot_report.processes_kill_failed,
                sweep_reclaim_errors = ?boot_report.sweep_reclaim_errors,
                session_errors = ?boot_report.session_errors,
                "boot restart-orphan reconcile completed fail-open with unreclaimed processes; their START rows remain truthfully open and the periodic restart tick retries them"
            );
        }
        let reclaim_task = spawn_managed_staleness_reclaim_task_after_boot(
            Arc::clone(&reclaim),
            stale_source,
            StalenessReclaimConfig::default(),
        );
        Ok(Self {
            inner: Arc::new(ProcessReclaimRuntimeInner {
                storage,
                resource_scope,
                ledger,
                reclaim,
                registry,
                runtime_instance,
                runtime_lease: Mutex::new(Some(runtime_lease)),
                reclaim_task,
                boot_reconcile: boot_report,
            }),
        })
    }

    /// MT-019 F3: the boot restart-reconcile evidence, retained instead of
    /// discarded, so a fail-open boot is observable to a backend surface rather
    /// than only to a log line.
    pub fn boot_reconcile_report(&self) -> &RestartOrphanBootReconcileReport {
        &self.inner.boot_reconcile
    }

    pub fn ledger(&self) -> RetainedLedgerBatcher {
        self.inner.ledger.clone()
    }

    pub fn reclaim(&self) -> Arc<Reclaim> {
        Arc::clone(&self.inner.reclaim)
    }

    pub fn sandbox_registry(&self) -> Arc<SandboxAdapterRegistry> {
        Arc::clone(&self.inner.registry)
    }

    pub fn runtime_instance(&self) -> &EmbeddedRuntimeInstanceDescriptor {
        &self.inner.runtime_instance
    }

    /// Count this instance's still-open lifecycle rows.
    ///
    /// MT-019 P-4(a): releasing the loopback lease is what advertises "this
    /// runtime instance is dead" to every other Handshake instance's restart
    /// sweep. It is therefore only safe when this instance owns nothing that is
    /// still running.
    async fn open_row_count_for_this_instance(&self) -> Result<i64, ProcessLedgerError> {
        let bindings = OpenRowCountBindings {
            runtime_instance_id: self.inner.runtime_instance.instance_id,
            owner_account_id: self.inner.resource_scope.account_uuid.to_string(),
            actor_principal_id: self.inner.resource_scope.actor_uuid.to_string(),
            authenticated_session_id: self.inner.resource_scope.session_uuid.to_string(),
            access_space_id: self.inner.resource_scope.access_space_uuid.to_string(),
            workspace_id: self.inner.resource_scope.workspace_id.clone(),
        };
        self.inner
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<i64, _>(OPEN_ROW_COUNT_FOR_RUNTIME, bindings)
                        .await
                })
            })
            .await
            .map_err(|error| ProcessLedgerError::Store(error.to_string()))?
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "Surreal open-row count returned no exact-scope result".to_owned(),
                )
            })
    }

    pub async fn shutdown_and_drain(&self, timeout: Duration) -> ProcessReclaimRuntimeDrainReport {
        // The caller supplies a per-infrastructure-phase budget. Reclaim-task
        // quiescence and ledger flush are independent obligations; allowing the
        // first to consume the second's budget can abort a healthy writer even
        // when both phases individually finish within the declared bound.
        let reclaim_task_quiesced = self.inner.reclaim_task.shutdown_and_join(timeout).await;
        let ledger = self.inner.ledger.drain_and_join(timeout).await;
        let drained = reclaim_task_quiesced
            && matches!(
                &ledger,
                LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained
            );
        // MT-019 P-4(a) — THE WRONG-KILL ROOT CAUSE. This used to drop the lease
        // as soon as the writer and reclaim task were quiesced, WHILE THE HOST
        // PROCESS KEPT RUNNING. The Tauri shell builds its own
        // `ProcessReclaimRuntime` and drains it mid-life with live official-CLI
        // children and open rows; a second Handshake instance then probed the
        // freed loopback port, concluded the owner was dead, and killed those
        // live healthy children. The pid + creation-time + exe-sha256 identity
        // fence offers no protection because it proves process generation, not
        // liveness or ownership, so it matches perfectly and the kill proceeds.
        //
        // The lease is now released only after the injected Surreal database
        // proves this instance owns zero open lifecycle rows in the exact
        // ResourceScope. Otherwise it is retained (the OS frees the socket at
        // real process exit), which is the only statement that is always true:
        // this process is still alive.
        let (lease_released, lease_retained_reason) = if !drained {
            (
                false,
                Some(
                    "reclaim task or ledger writer did not reach a proven terminal state"
                        .to_string(),
                ),
            )
        } else {
            match self.open_row_count_for_this_instance().await {
                Ok(0) => {
                    let released = self
                        .inner
                        .runtime_lease
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .take()
                        .is_some();
                    if released {
                        (true, None)
                    } else {
                        (false, Some("lease was already released".to_string()))
                    }
                }
                Ok(open_rows) => (
                    false,
                    Some(format!(
                        "{open_rows} lifecycle rows owned by this runtime instance are still open; \
                         releasing the liveness lease would advertise this live process as dead"
                    )),
                ),
                Err(error) => (
                    false,
                    Some(format!(
                        "could not prove this runtime instance owns zero open lifecycle rows: {error}"
                    )),
                ),
            }
        };
        if let Some(reason) = lease_retained_reason.as_deref() {
            tracing::info!(
                target: "handshake_core::process_ledger",
                runtime_instance_id = %self.inner.runtime_instance.instance_id,
                reason,
                "retaining the embedded-runtime loopback liveness lease until process exit"
            );
        }
        ProcessReclaimRuntimeDrainReport {
            reclaim_task_quiesced,
            ledger,
            lease_released,
            lease_retained_reason,
        }
    }
}

impl Drop for ProcessReclaimRuntimeInner {
    fn drop(&mut self) {
        const DROP_JOIN_TIMEOUT: Duration = Duration::from_secs(2);
        let _reclaim_stopped = self.reclaim_task.abort_and_join_blocking(DROP_JOIN_TIMEOUT);
        let _writer_stopped = self.ledger.abort_and_join_blocking(DROP_JOIN_TIMEOUT);
        let runtime_lease = self
            .runtime_lease
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(runtime_lease) = runtime_lease {
            // MT-019 P-4(a): Drop is a synchronous context, so it can never prove
            // this instance owns zero open lifecycle rows. It also does not mean
            // the host process is exiting — the Tauri shell drops its runtime and
            // keeps running with live official-CLI children. Releasing the lease
            // here therefore advertised a LIVE instance as dead and authorised
            // another instance to kill its healthy children.
            //
            // The lease is now always retained. Leaking the UDP socket is
            // deliberate: the OS releases it at real process termination, which
            // is the only moment "this owner is dead" is unconditionally true.
            // `ProcessReclaimRuntime::shutdown_and_drain` is the one path that
            // may release it early, and only against a proven-zero open-row count.
            std::mem::forget(runtime_lease);
        }
    }
}

pub fn production_process_sandbox_registry() -> Arc<SandboxAdapterRegistry> {
    let default_adapter_id = AdapterId::new(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
    let mut registry = SandboxAdapterRegistry::new(default_adapter_id);
    registry.register(Arc::new(HandshakeNativeSandboxAdapter::new()));
    registry.register(Arc::new(
        crate::sandbox::palmistry_watcher::PalmistryWatcherAdapter,
    ));
    Arc::new(registry)
}

/// Discover the full shipped sandbox set for the async application composition
/// root. Unlike the synchronous fixture/compatibility registry above, this
/// executes the adapters' real availability probes, so the same registry can
/// route Tier-3 model lanes and reclaim their durable handles after restart.
/// Palmistry remains an explicit host-process owner even though it is not part
/// of the generic sandbox bootstrap.
pub async fn production_process_sandbox_registry_async(
) -> Result<Arc<SandboxAdapterRegistry>, crate::sandbox::SandboxAdapterError> {
    let mut registry = crate::sandbox::build_default_registry_async().await?;
    registry.register(Arc::new(
        crate::sandbox::palmistry_watcher::PalmistryWatcherAdapter,
    ));
    Ok(Arc::new(registry))
}
