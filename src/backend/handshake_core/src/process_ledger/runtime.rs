use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use sqlx::PgPool;
use tokio::time;
use uuid::Uuid;

use crate::sandbox::{
    AdapterId, HandshakeNativeSandboxAdapter, SandboxAdapterRegistry,
    HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};

use super::{
    acquire_embedded_runtime_instance_lease,
    reclaim::spawn_managed_staleness_reclaim_task_after_boot, EmbeddedRuntimeInstanceDescriptor,
    EmbeddedRuntimeInstanceLease, LedgerBatcherConfig, LedgerDrainJoinOutcome,
    ManagedStalenessReclaimTask, NoopOverflowSink, PostgresModelLaneStaleSessionSource,
    PostgresProcessLedgerStore, ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
    ProductionSandboxKill, Reclaim, ReclaimTrigger, RetainedLedgerBatcher, StaleSessionSource,
    StalenessReclaimConfig,
};

#[derive(Debug)]
pub struct ProcessReclaimRuntimeDrainReport {
    pub reclaim_task_quiesced: bool,
    pub ledger: LedgerDrainJoinOutcome,
    pub lease_released: bool,
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
    ledger: RetainedLedgerBatcher,
    reclaim: Arc<Reclaim>,
    registry: Arc<SandboxAdapterRegistry>,
    runtime_instance: EmbeddedRuntimeInstanceDescriptor,
    runtime_lease: Mutex<Option<EmbeddedRuntimeInstanceLease>>,
    reclaim_task: ManagedStalenessReclaimTask,
}

impl ProcessReclaimRuntime {
    pub async fn production(
        pool: PgPool,
        ledger_store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Option<Arc<dyn ProcessLedgerOverflowSink>>,
        registry: Arc<SandboxAdapterRegistry>,
        host_scope_id: impl Into<String>,
        startup_timeout: Duration,
    ) -> Result<Self, ProcessLedgerError> {
        let runtime_lease =
            acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope_id.into())?;
        Self::production_with_lease(
            pool,
            ledger_store,
            overflow_sink,
            registry,
            runtime_lease,
            startup_timeout,
        )
        .await
    }

    pub async fn production_with_lease(
        pool: PgPool,
        ledger_store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Option<Arc<dyn ProcessLedgerOverflowSink>>,
        registry: Arc<SandboxAdapterRegistry>,
        runtime_lease: EmbeddedRuntimeInstanceLease,
        startup_timeout: Duration,
    ) -> Result<Self, ProcessLedgerError> {
        let runtime_instance = runtime_lease.descriptor().clone();
        let ledger = RetainedLedgerBatcher::spawn_with_runtime_owner(
            ledger_store,
            overflow_sink.unwrap_or_else(|| Arc::new(NoopOverflowSink)),
            LedgerBatcherConfig::default(),
            runtime_instance.process_runtime_owner(),
        );
        let reclaim_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
        let killer = Arc::new(ProductionSandboxKill::with_registry(
            pool.clone(),
            Arc::clone(&registry),
        ));
        let reclaim = Arc::new(Reclaim::new(
            reclaim_store,
            killer,
            Arc::new(ledger.ledger()),
        ));
        let stale_source: Arc<dyn StaleSessionSource> = Arc::new(
            PostgresModelLaneStaleSessionSource::new(pool, runtime_instance.clone()),
        );

        let boot_reconcile = async {
            for session_id in stale_source.restart_sessions().await? {
                reclaim
                    .reconcile_in_progress_for_session(&session_id)
                    .await?;
                reclaim.run(&session_id, ReclaimTrigger::Restart).await?;
            }
            Ok::<(), ProcessLedgerError>(())
        };
        let boot_error = match time::timeout(startup_timeout, boot_reconcile).await {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error),
            Err(_) => Some(ProcessLedgerError::Store(format!(
                "process reclaim boot reconciliation exceeded {} ms",
                startup_timeout.as_millis()
            ))),
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

        let reclaim_task = spawn_managed_staleness_reclaim_task_after_boot(
            Arc::clone(&reclaim),
            stale_source,
            StalenessReclaimConfig::default(),
        );
        Ok(Self {
            inner: Arc::new(ProcessReclaimRuntimeInner {
                ledger,
                reclaim,
                registry,
                runtime_instance,
                runtime_lease: Mutex::new(Some(runtime_lease)),
                reclaim_task,
            }),
        })
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

    pub async fn shutdown_and_drain(&self, timeout: Duration) -> ProcessReclaimRuntimeDrainReport {
        let started = std::time::Instant::now();
        let reclaim_task_quiesced = self.inner.reclaim_task.shutdown_and_join(timeout).await;
        let remaining = timeout.saturating_sub(started.elapsed());
        let ledger = self.inner.ledger.drain_and_join(remaining).await;
        let lease_released = if reclaim_task_quiesced
            && matches!(
                &ledger,
                LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained
            ) {
            self.inner
                .runtime_lease
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
                .is_some()
        } else {
            false
        };
        ProcessReclaimRuntimeDrainReport {
            reclaim_task_quiesced,
            ledger,
            lease_released,
        }
    }
}

impl Drop for ProcessReclaimRuntimeInner {
    fn drop(&mut self) {
        const DROP_JOIN_TIMEOUT: Duration = Duration::from_secs(2);
        let reclaim_stopped = self.reclaim_task.abort_and_join_blocking(DROP_JOIN_TIMEOUT);
        let writer_stopped = self.ledger.abort_and_join_blocking(DROP_JOIN_TIMEOUT);
        let runtime_lease = self
            .runtime_lease
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(runtime_lease) = runtime_lease {
            if reclaim_stopped && writer_stopped {
                drop(runtime_lease);
            } else {
                // Do not advertise this runtime as dead while either detached
                // owner can still access PostgreSQL. Leaking the UDP socket is
                // intentional fail-closed behavior; the OS releases it at
                // process termination.
                std::mem::forget(runtime_lease);
            }
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
