pub mod batcher;
pub mod escalation_router;
pub mod idempotency;
pub mod mt_executor;
pub mod mt_loop_control;
pub mod mt_outcome;
pub mod overflow;
pub mod reclaim;
pub mod restart_resume;
pub mod runtime;
pub mod schema;
pub mod table;
pub mod writer;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub use batcher::{
    drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
    NoopOverflowSink, RetainedLedgerBatcher,
};
pub use idempotency::{
    ApplyOutcome, IdempotencyKey, IdempotencyLedger, IdempotencyLedgerError, IdempotentApply,
    SideEffectKind,
};
pub use overflow::{cap_metadata_jsonb, cap_metadata_value, MetadataCapOutcome};
pub use reclaim::{
    acquire_embedded_runtime_instance_lease, reclaim_handle, reclaim_pidless_embedded_orphans,
    resolve_embedded_runtime_host_scope, resolve_embedded_runtime_host_scope_with_managed_local,
    resolve_embedded_runtime_host_scope_with_override, spawn_managed_staleness_reclaim_task,
    spawn_staleness_reclaim_task, verify_proven_local_postgres_endpoint_pool,
    EmbeddedRuntimeInstanceDescriptor, EmbeddedRuntimeInstanceLease, KillError, KillOutcome,
    LegacyHostScopeOpenRowProbe, ManagedStalenessReclaimTask, PidlessEmbeddedReclaimReport,
    PostgresModelLaneStaleSessionSource, ProductionSandboxKill, Reclaim, ReclaimClaim,
    ReclaimKillOperation, ReclaimKillOperationCandidate, ReclaimKillOperationStatus,
    ReclaimKillOperationSweep, ReclaimKillOperationSweepEntry, ReclaimKillOperationSweepOutcome,
    ReclaimProcessStore, ReclaimReport, ReclaimStopReservation, ReclaimStopWriter, ReclaimTrigger,
    ReclaimableProcess, ReclaimedProcess, SandboxKill, StaleSessionSource, StalenessReclaimConfig,
    EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID, EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
    EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX, HANDSHAKE_HOST_SCOPE_ID_ENV,
    PIDLESS_RECLAIM_INSTANCE_CAP, POSTGRES_ACTIVE_RECLAIM_QUERY_SQL,
};
pub use restart_resume::{
    OperatorDecisionRequest, OrphanReclaimInfo, RestartResumeOrchestrator, ResumableSession,
    ResumeError, ResumeReport, ResumedSessionInfo,
};
pub use runtime::{
    production_process_sandbox_registry, ProcessReclaimRuntime,
    ProcessReclaimRuntimeDrainReport,
};
pub use table::{
    LedgerEvent, LedgerEventKind, ProcessEngineKind, ProcessRuntimeOwner, ProcessStart, ProcessStop,
    PROCESS_LEDGER_BATCH_SIZE, PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
    PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS,
    PROCESS_LEDGER_FLUSH_INTERVAL_MS, PROCESS_LEDGER_METADATA_CAP_BYTES,
    PROCESS_LEDGER_MIGRATION_SQL, PROCESS_LEDGER_RING_CAPACITY, PROCESS_LEDGER_TABLE_NAME,
    PROCESS_START_INSERT_SQL, PROCESS_STOP_UPSERT_SQL,
};
pub use writer::{
    flush_failed_row_count, is_degraded, ActiveProcessLifecycle, LedgerOverflowEvent,
    PostgresProcessLedgerStore, ProcessLedgerDrain, ProcessLedgerDurabilityAck, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter, ReservedProcessLifecycle,
    ReservedProcessStop, StopRecordOutcome, WriterConfig, FR_EVT_LEDGER_OVERFLOW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessOwnershipRecordId(Uuid);

impl ProcessOwnershipRecordId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnMeta {
    pub pid: u32,
    /// Whether `pid` names a real Handshake-owned operating-system process.
    /// In-process runtimes may still carry a coordinator-local numeric handle,
    /// but must set this false so the canonical ledger remains honestly
    /// pidless and recovery never targets a fabricated OS identity.
    #[serde(default = "spawn_meta_pid_authoritative_default")]
    pub pid_authoritative: bool,
    pub engine_kind: ProcessEngineKind,
    pub model_id: Option<String>,
    pub runtime_binding: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub parent_process_id: Option<Uuid>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub cancellation_id: Option<String>,
    #[serde(default)]
    pub reclaim_key: Option<String>,
    #[serde(default)]
    pub model_identity: Option<String>,
    pub started_at_utc: DateTime<Utc>,
    pub sandbox_adapter: Option<String>,
    /// The sandbox adapter's INTERNAL handle id (e.g. `hsk-ch-<uuid>`) for a
    /// boxed/microVM-routed session. Populated by the swarm factory's sandboxed
    /// path from the `ProcessHandle.sandbox_internal_id`; `None` for in-process
    /// (non-sandboxed) sessions. `record_spawn` maps it onto
    /// `ProcessStart::with_sandbox_internal_id` so the ledger START/STOP rows
    /// carry the microVM identity (WP-KERNEL-004 wave 1).
    pub sandbox_internal_id: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub sandbox_capabilities_snapshot: Value,
    pub metadata_blob: Value,
}

impl SpawnMeta {
    pub fn new(pid: u32, engine_kind: ProcessEngineKind, owner_role: impl Into<String>) -> Self {
        Self {
            pid,
            pid_authoritative: true,
            engine_kind,
            model_id: None,
            runtime_binding: None,
            session_id: None,
            parent_session_id: None,
            parent_process_id: None,
            trace_id: None,
            span_id: None,
            cancellation_id: None,
            reclaim_key: None,
            model_identity: None,
            started_at_utc: Utc::now(),
            sandbox_adapter: None,
            sandbox_internal_id: None,
            model_artifact_sha256: None,
            work_profile_id: None,
            owner_role: owner_role.into(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            sandbox_capabilities_snapshot: json!({}),
            metadata_blob: json!({}),
        }
    }

    pub fn without_os_pid(mut self) -> Self {
        self.pid_authoritative = false;
        self
    }
}

fn spawn_meta_pid_authoritative_default() -> bool {
    true
}

pub fn record_spawn(
    ledger: &LedgerBatcher,
    meta: SpawnMeta,
) -> Result<ProcessOwnershipRecordId, ProcessLedgerError> {
    let record_id = ProcessOwnershipRecordId::new_v7();
    let mut metadata = if meta.metadata_blob.is_object() {
        meta.metadata_blob.clone()
    } else {
        json!({ "legacy_metadata": meta.metadata_blob.clone() })
    };
    metadata["session_id"] = json!(meta.session_id);
    metadata["trace_id"] = json!(meta.trace_id);
    metadata["span_id"] = json!(meta.span_id);
    metadata["cancellation_id"] = json!(meta.cancellation_id);
    metadata["reclaim_key"] = json!(meta.reclaim_key);
    metadata["model_identity"] = json!(meta.model_identity);
    let mut start = ProcessStart::new(
        meta.engine_kind,
        meta.owner_role.clone(),
        meta.owner_wp.clone(),
    )
    .with_process_uuid(record_id.as_uuid())
    .with_metadata_jsonb(metadata)
    .with_sandbox_capabilities_snapshot(meta.sandbox_capabilities_snapshot);
    if meta.pid_authoritative {
        start = start.with_os_pid(meta.pid);
    }
    start.started_at = meta.started_at_utc;

    if let Some(parent_session_id) = meta.parent_session_id {
        start = start.with_parent_session_id(parent_session_id);
    }
    if let Some(parent_process_id) = meta.parent_process_id {
        start = start.with_parent_process_id(parent_process_id);
    }
    if let Some(sandbox_adapter) = meta.sandbox_adapter {
        start = start.with_sandbox_adapter_id(sandbox_adapter);
    }
    if let Some(sandbox_internal_id) = meta.sandbox_internal_id {
        start = start.with_sandbox_internal_id(sandbox_internal_id);
    }
    if let Some(model_artifact_sha256) = meta.model_artifact_sha256 {
        start = start.with_model_artifact_sha256(model_artifact_sha256);
    }
    if let Some(work_profile_id) = meta.work_profile_id {
        start = start.with_work_profile_id(work_profile_id);
    }
    if let Some(role_id) = meta.role_id {
        start = start.with_role_id(role_id);
    }
    if let Some(wp_id) = meta.wp_id {
        start = start.with_wp_id(wp_id);
    }
    if let Some(mt_id) = meta.mt_id {
        start = start.with_mt_id(mt_id);
    }

    ledger.record_start(start)?;
    Ok(record_id)
}
