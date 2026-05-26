pub mod batcher;
pub mod escalation_router;
pub mod idempotency;
pub mod mt_executor;
pub mod mt_loop_control;
pub mod mt_outcome;
pub mod overflow;
pub mod reclaim;
pub mod restart_resume;
pub mod schema;
pub mod table;
pub mod writer;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub use batcher::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
pub use idempotency::{
    ApplyOutcome, IdempotencyKey, IdempotencyLedger, IdempotencyLedgerError, IdempotentApply,
    SideEffectKind,
};
pub use overflow::{cap_metadata_jsonb, cap_metadata_value, MetadataCapOutcome};
pub use reclaim::{
    reclaim_handle, spawn_staleness_reclaim_task, KillError, KillOutcome, Reclaim,
    ReclaimProcessStore, ReclaimReport, ReclaimStopWriter, ReclaimTrigger, ReclaimableProcess,
    ReclaimedProcess, SandboxKill, StaleSessionSource, StalenessReclaimConfig,
    POSTGRES_ACTIVE_RECLAIM_QUERY_SQL,
};
pub use restart_resume::{
    OperatorDecisionRequest, OrphanReclaimInfo, RestartResumeOrchestrator, ResumableSession,
    ResumeError, ResumeReport, ResumedSessionInfo,
};
pub use table::{
    LedgerEvent, LedgerEventKind, ProcessEngineKind, ProcessStart, ProcessStop,
    PROCESS_LEDGER_BATCH_SIZE, PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
    PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS,
    PROCESS_LEDGER_FLUSH_INTERVAL_MS, PROCESS_LEDGER_METADATA_CAP_BYTES,
    PROCESS_LEDGER_MIGRATION_SQL, PROCESS_LEDGER_RING_CAPACITY, PROCESS_LEDGER_TABLE_NAME,
    PROCESS_START_INSERT_SQL, PROCESS_STOP_UPSERT_SQL,
};
pub use writer::{
    is_degraded, LedgerOverflowEvent, PostgresProcessLedgerStore, ProcessLedgerDrain,
    ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter,
    WriterConfig, FR_EVT_LEDGER_OVERFLOW,
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
    pub engine_kind: ProcessEngineKind,
    pub model_id: Option<String>,
    pub runtime_binding: Option<String>,
    pub parent_session_id: Option<String>,
    pub started_at_utc: DateTime<Utc>,
    pub sandbox_adapter: Option<String>,
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
            engine_kind,
            model_id: None,
            runtime_binding: None,
            parent_session_id: None,
            started_at_utc: Utc::now(),
            sandbox_adapter: None,
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
}

pub fn record_spawn(
    ledger: &LedgerBatcher,
    meta: SpawnMeta,
) -> Result<ProcessOwnershipRecordId, ProcessLedgerError> {
    let record_id = ProcessOwnershipRecordId::new_v7();
    let mut start = ProcessStart::new(
        meta.engine_kind,
        meta.owner_role.clone(),
        meta.owner_wp.clone(),
    )
    .with_process_uuid(record_id.as_uuid())
    .with_os_pid(meta.pid)
    .with_metadata_jsonb(meta.metadata_blob)
    .with_sandbox_capabilities_snapshot(meta.sandbox_capabilities_snapshot);
    start.started_at = meta.started_at_utc;

    if let Some(parent_session_id) = meta.parent_session_id {
        start = start.with_parent_session_id(parent_session_id);
    }
    if let Some(sandbox_adapter) = meta.sandbox_adapter {
        start = start.with_sandbox_adapter_id(sandbox_adapter);
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
