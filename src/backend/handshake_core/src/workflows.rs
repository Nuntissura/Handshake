use crate::{
    ace::{
        validators::{
            build_query_plan_from_blocks, build_retrieval_trace_from_blocks,
            freshness::{REGEN_SKIPPED_PREFIX, STALE_PACK_WARNING_PREFIX},
            scan_content_for_security, SecurityViolationType, StorageContentResolver,
            ValidatorPipeline,
        },
        AceError, ArtifactHandle, CandidateRef, CandidateScores, ContextPackAnchorV1,
        ContextPackBuilder, ContextPackCoverageV1, ContextPackFreshnessPolicyV1,
        ContextPackPayloadV1, ContextPackRecord, DeterminismMode, QueryKind, QueryPlan,
        RetrievalCandidate, RetrievalTrace, RouteTaken, SelectedEvidence, SourceRef,
        SpanExtraction, StoreKind,
    },
    bundles::{BundleScope, DebugBundleRequest, DefaultDebugBundleExporter, RedactionMode},
    capabilities::{RegistryError, GOVERNANCE_PACK_EXPORT_PROTOCOL_ID},
    capability_registry_workflow::{
        repo_root_from_manifest_dir, run_capability_registry_workflow,
        CapabilityRegistryWorkflowParams,
    },
    flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
        FrEvt008SecurityViolation, FrEvtWorkflowRecovery,
    },
    governance_pack::{export_governance_pack, GovernancePackExportRequest},
    llm::{
         guard::{
            CloudEscalationBundleV0_4, CloudEscalationPolicy, CloudEscalationRequestV0_4,
            ConsentReceiptV0_4, ProjectionPlanV0_4, RuntimeGovernanceMode,
         },
         openai_compat_canonical_request_bytes, CompletionRequest, LlmError,
     },
    mex::runtime::ShellEngineAdapter,
    mex::{
        BudgetGate, BudgetSpec, CapabilityGate, DetGate, DeterminismLevel, EvidencePolicy,
        GatePipeline, IntegrityGate, MexRegistry, MexRuntime, OutputSpec, PlannedOperation,
        ProvenanceGate, SchemaGate, POE_SCHEMA_VERSION,
    },
    models::{AiJob, JobKind, WorkflowRun},
    runtime_governance::RuntimeGovernancePaths,
    storage::{validate_job_contract, JobState, JobStatusUpdate, NewNodeExecution, StorageError},
    terminal::{
        config::TerminalConfig,
        guards::{DefaultTerminalGuard, TerminalGuard},
        redaction::PatternRedactor,
        JobContext, TerminalMode, TerminalRequest, TerminalService,
    },
    AppState,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    collections::HashMap,
    fs,
    path::Path,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
};
use thiserror::Error;
use tokio::sync::Notify;
use uuid::Uuid;

#[path = "locus/mod.rs"]
pub mod locus;

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Capability error: {0}")]
    Capability(#[from] RegistryError),
    /// Security violation detected by ACE validators [HSK-ACE-VAL-101]
    /// This error triggers JobState::Poisoned transition
    #[error("Security violation: {0}")]
    SecurityViolation(#[from] AceError),
}

#[derive(Error, Debug)]
pub enum StartupRecoveryError {
    #[error("startup recovery failed: {reason}")]
    Failed { reason: String },
}

struct StartupRecoveryGate {
    enabled: AtomicBool,
    status: AtomicU8,
    failure_reason: Mutex<Option<String>>,
    notify: Notify,
}

const STARTUP_GATE_DISABLED: u8 = 0;
const STARTUP_GATE_IN_PROGRESS: u8 = 1;
const STARTUP_GATE_COMPLETE: u8 = 2;
const STARTUP_GATE_FAILED: u8 = 3;

static STARTUP_RECOVERY_GATE: Lazy<StartupRecoveryGate> = Lazy::new(|| StartupRecoveryGate {
    enabled: AtomicBool::new(false),
    status: AtomicU8::new(STARTUP_GATE_DISABLED),
    failure_reason: Mutex::new(None),
    notify: Notify::new(),
});

pub fn enable_startup_recovery_gate() {
    STARTUP_RECOVERY_GATE.enabled.store(true, Ordering::Release);
    STARTUP_RECOVERY_GATE
        .status
        .store(STARTUP_GATE_IN_PROGRESS, Ordering::Release);
    if let Ok(mut guard) = STARTUP_RECOVERY_GATE.failure_reason.lock() {
        *guard = None;
    }
}

pub fn mark_startup_recovery_complete() {
    if !STARTUP_RECOVERY_GATE.enabled.load(Ordering::Acquire) {
        return;
    }
    STARTUP_RECOVERY_GATE
        .status
        .store(STARTUP_GATE_COMPLETE, Ordering::Release);
    STARTUP_RECOVERY_GATE.notify.notify_waiters();
}

pub fn mark_startup_recovery_failed(reason: String) {
    if !STARTUP_RECOVERY_GATE.enabled.load(Ordering::Acquire) {
        return;
    }
    if let Ok(mut guard) = STARTUP_RECOVERY_GATE.failure_reason.lock() {
        *guard = Some(reason);
    }
    STARTUP_RECOVERY_GATE
        .status
        .store(STARTUP_GATE_FAILED, Ordering::Release);
    STARTUP_RECOVERY_GATE.notify.notify_waiters();
}

pub async fn wait_for_startup_recovery() -> Result<(), StartupRecoveryError> {
    if !STARTUP_RECOVERY_GATE.enabled.load(Ordering::Acquire) {
        return Ok(());
    }

    loop {
        let notified = STARTUP_RECOVERY_GATE.notify.notified();
        match STARTUP_RECOVERY_GATE.status.load(Ordering::Acquire) {
            STARTUP_GATE_COMPLETE => return Ok(()),
            STARTUP_GATE_FAILED => {
                let reason = STARTUP_RECOVERY_GATE
                    .failure_reason
                    .lock()
                    .ok()
                    .and_then(|guard| guard.as_ref().cloned())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(StartupRecoveryError::Failed { reason });
            }
            _ => {}
        }
        notified.await;
    }
}

#[cfg(test)]
pub fn reset_startup_recovery_gate_for_test() {
    STARTUP_RECOVERY_GATE
        .enabled
        .store(false, Ordering::Release);
    STARTUP_RECOVERY_GATE
        .status
        .store(STARTUP_GATE_DISABLED, Ordering::Release);
    if let Ok(mut guard) = STARTUP_RECOVERY_GATE.failure_reason.lock() {
        *guard = None;
    }
    STARTUP_RECOVERY_GATE.notify.notify_waiters();
}

fn parse_inputs(raw: Option<&serde_json::Value>) -> serde_json::Value {
    raw.map_or_else(|| json!({}), serde_json::Value::clone)
}

fn derive_trace_id(job: &AiJob, workflow_id: Option<&str>) -> Uuid {
    if let Some(wf) = workflow_id {
        if let Ok(id) = Uuid::parse_str(wf) {
            return id;
        }
    }

    job.job_id
}

async fn execute_locus_sync_task_board(
    state: &AppState,
    db: &dyn crate::storage::Database,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
    params: locus::LocusSyncTaskBoardParams,
) -> Result<Value, WorkflowError> {
    fn task_board_status_db(status: locus::TaskBoardStatus) -> &'static str {
        match status {
            locus::TaskBoardStatus::Unknown => "STUB",
            locus::TaskBoardStatus::Ready => "READY",
            locus::TaskBoardStatus::InProgress => "IN_PROGRESS",
            locus::TaskBoardStatus::Blocked => "BLOCKED",
            locus::TaskBoardStatus::Gated => "GATED",
            locus::TaskBoardStatus::Done => "DONE",
            locus::TaskBoardStatus::Cancelled => "CANCELLED",
        }
    }

    fn work_packet_status_db(status: locus::TaskBoardStatus) -> &'static str {
        match status {
            locus::TaskBoardStatus::Unknown => "stub",
            locus::TaskBoardStatus::Ready => "ready",
            locus::TaskBoardStatus::InProgress => "in_progress",
            locus::TaskBoardStatus::Blocked => "blocked",
            locus::TaskBoardStatus::Gated => "gated",
            locus::TaskBoardStatus::Done => "done",
            locus::TaskBoardStatus::Cancelled => "cancelled",
        }
    }

    fn parse_task_board_status_db(value: &str) -> locus::TaskBoardStatus {
        match value.trim() {
            "READY" => locus::TaskBoardStatus::Ready,
            "IN_PROGRESS" => locus::TaskBoardStatus::InProgress,
            "BLOCKED" => locus::TaskBoardStatus::Blocked,
            "GATED" => locus::TaskBoardStatus::Gated,
            "DONE" => locus::TaskBoardStatus::Done,
            "CANCELLED" => locus::TaskBoardStatus::Cancelled,
            _ => locus::TaskBoardStatus::Unknown,
        }
    }

    fn default_task_board_token(status: locus::TaskBoardStatus) -> &'static str {
        match status {
            locus::TaskBoardStatus::Unknown => "STUB",
            locus::TaskBoardStatus::Ready => "READY_FOR_DEV",
            locus::TaskBoardStatus::InProgress => "IN_PROGRESS",
            locus::TaskBoardStatus::Blocked => "BLOCKED",
            locus::TaskBoardStatus::Gated => "GATED",
            locus::TaskBoardStatus::Done => "VALIDATED",
            locus::TaskBoardStatus::Cancelled => "SUPERSEDED",
        }
    }

    crate::storage::locus_sqlite::ensure_locus_sqlite(db)?;

    let runtime_governance_paths = RuntimeGovernancePaths::resolve().map_err(|e| {
        WorkflowError::Terminal(format!("failed to resolve runtime governance paths: {e}"))
    })?;
    let workspace_root = runtime_governance_paths.workspace_root().to_path_buf();
    let task_board_path = runtime_governance_paths.task_board_path();
    let dry_run = params.dry_run.unwrap_or(false);
    let sync_target = runtime_governance_paths.task_board_display();
    // WAIVER [CX-573E]: Instant::now() for observability (sync duration metrics).
    let sync_started_at = std::time::Instant::now();

    record_event_required(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::LocusSyncStarted,
            FlightRecorderActor::Agent,
            trace_id,
            locus_event_payload(
                job,
                workflow_run_id,
                trace_id,
                &job.protocol_id,
                "FR-EVT-SYNC-001",
                "sync_started",
                json!({
                    "sync_target": sync_target.as_str(),
                    "dry_run": dry_run,
                }),
            ),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await?;

    let result: Result<
        (
            Value,
            bool,
            Vec<locus::task_board::TaskBoardEntry>,
            u64,
            u64,
            u64,
            u32,
            Vec<(
                String,
                locus::TaskBoardStatus,
                locus::TaskBoardStatus,
                String,
            )>,
            Vec<String>,
        ),
        WorkflowError,
    > = async {
        let task_board = fs::read_to_string(&task_board_path).map_err(|e| {
            WorkflowError::Terminal(format!(
                "failed to read Task Board {}: {e}",
                task_board_path.display()
            ))
        })?;

        let parsed = locus::task_board::parse_task_board(&task_board);

        let statuses = [
            locus::TaskBoardStatus::Unknown,
            locus::TaskBoardStatus::Ready,
            locus::TaskBoardStatus::InProgress,
            locus::TaskBoardStatus::Blocked,
            locus::TaskBoardStatus::Gated,
            locus::TaskBoardStatus::Done,
            locus::TaskBoardStatus::Cancelled,
        ];

        let mut unknown_wp_ids: Vec<String> = Vec::new();
        let mut applied_updates = 0u32;
        let mut before_map: std::collections::BTreeMap<String, (locus::TaskBoardStatus, String)> =
            std::collections::BTreeMap::new();

        for status in statuses {
            for entry in parsed.entries_for_status(status) {
                before_map.insert(entry.wp_id.clone(), (entry.status, entry.token.clone()));

                let row = crate::storage::locus_sqlite::locus_task_board_get_status_and_metadata(
                    db,
                    &entry.wp_id,
                )
                .await?;

                let Some((db_task_board_status, metadata_raw)) = row else {
                    unknown_wp_ids.push(entry.wp_id.clone());
                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::Diagnostic,
                            FlightRecorderActor::Agent,
                            trace_id,
                            json!({
                                "diagnostic_id": "HSK-LOCUS-TB-UNKNOWN-WP",
                                "wp_id": entry.wp_id.as_str(),
                                "token": entry.token.as_str(),
                                "task_board_status": task_board_status_db(entry.status),
                                "protocol_id": job.protocol_id.as_str(),
                            }),
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;
                    continue;
                };

                let prev_status = parse_task_board_status_db(&db_task_board_status);
                let status_changed = prev_status != entry.status;

                let mut metadata: Value =
                    serde_json::from_str(&metadata_raw).unwrap_or_else(|_| json!({}));
                if !metadata.is_object() {
                    metadata = json!({});
                }
                let prev_token = metadata
                    .get("task_board_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let token_changed = prev_token != entry.token.as_str();
                if token_changed {
                    metadata["task_board_token"] = Value::String(entry.token.clone());
                }

                if (status_changed || token_changed) && !dry_run {
                    let now = Utc::now().to_rfc3339();
                    let metadata = serde_json::to_string(&metadata)
                        .map_err(crate::storage::StorageError::from)?;
                    crate::storage::locus_sqlite::locus_task_board_update_work_packet(
                        db,
                        work_packet_status_db(entry.status),
                        task_board_status_db(entry.status),
                        &now,
                        &metadata,
                        &entry.wp_id,
                    )
                    .await?;

                    applied_updates += 1;
                } else if status_changed && dry_run {
                    // In dry-run we still persist token into metadata for deterministic rewrite.
                    // (No DB writes happen in dry-run.)
                }
            }
        }

        let mut canonical = locus::task_board::TaskBoardSections::default();
        let mut after_map: std::collections::BTreeMap<String, (locus::TaskBoardStatus, String)> =
            std::collections::BTreeMap::new();

        let rows = crate::storage::locus_sqlite::locus_task_board_list_rows(db).await?;

        for (wp_id, task_board_status, metadata_raw) in rows {
            let status = parse_task_board_status_db(&task_board_status);
            let token = match serde_json::from_str::<Value>(&metadata_raw) {
                Ok(val) => val
                    .get("task_board_token")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| default_task_board_token(status).to_string()),
                Err(_) => default_task_board_token(status).to_string(),
            };
            after_map.insert(wp_id.clone(), (status, token.clone()));
            canonical
                .entries_for_status_mut(status)
                .push(locus::task_board::TaskBoardEntry {
                    wp_id,
                    token,
                    status,
                });
        }

        let mut entries_added: Vec<locus::task_board::TaskBoardEntry> = Vec::new();
        for (wp_id, (status, token)) in after_map.iter() {
            if !before_map.contains_key(wp_id) {
                entries_added.push(locus::task_board::TaskBoardEntry {
                    wp_id: wp_id.clone(),
                    token: token.clone(),
                    status: *status,
                });
            }
        }

        let entries_added_count = entries_added.len() as u64;
        let entries_removed_count = before_map
            .keys()
            .filter(|wp_id| !after_map.contains_key(*wp_id))
            .count() as u64;

        let mut status_change_entries: Vec<(
            String,
            locus::TaskBoardStatus,
            locus::TaskBoardStatus,
            String,
        )> = Vec::new();
        for (wp_id, (before_status, _before_token)) in before_map.iter() {
            let Some((after_status, after_token)) = after_map.get(wp_id) else {
                continue;
            };
            if before_status != after_status {
                status_change_entries.push((
                    wp_id.clone(),
                    *before_status,
                    *after_status,
                    after_token.clone(),
                ));
            }
        }
        let status_changes = status_change_entries.len() as u64;

        let rewritten = locus::task_board::rewrite_task_board(&task_board, &canonical);
        let would_write = rewritten != task_board;
        if would_write && !dry_run {
            write_bytes_atomic(&workspace_root, &task_board_path, rewritten.as_bytes())?;
        }

        unknown_wp_ids.sort();
        unknown_wp_ids.dedup();

        let task_board_written = would_write && !dry_run;
        let output = json!({
            "dry_run": dry_run,
            "applied_updates": applied_updates,
            "unknown_wp_ids": unknown_wp_ids,
            "task_board_written": task_board_written,
            "entries_added": entries_added_count,
            "entries_removed": entries_removed_count,
            "status_changes": status_changes,
        });

        Ok((
            output,
            task_board_written,
            entries_added,
            entries_added_count,
            entries_removed_count,
            status_changes,
            applied_updates,
            status_change_entries,
            unknown_wp_ids,
        ))
    }
    .await;

    match result {
        Ok((
            output,
            task_board_written,
            entries_added,
            entries_added_count,
            entries_removed_count,
            status_changes,
            applied_updates,
            status_change_entries,
            unknown_wp_ids,
        )) => {
            for (wp_id, from_status, to_status, token) in &status_change_entries {
                record_event_required(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::LocusTaskBoardStatusChanged,
                        FlightRecorderActor::Agent,
                        trace_id,
                        locus_event_payload(
                            job,
                            workflow_run_id,
                            trace_id,
                            &job.protocol_id,
                            "FR-EVT-TB-003",
                            "task_board_status_changed",
                            json!({
                                "wp_id": wp_id.as_str(),
                                "from_status": task_board_status_db(*from_status),
                                "to_status": task_board_status_db(*to_status),
                                "token": token.as_str(),
                            }),
                        ),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await?;
            }

            for entry in &entries_added {
                record_event_required(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::LocusTaskBoardEntryAdded,
                        FlightRecorderActor::Agent,
                        trace_id,
                        locus_event_payload(
                            job,
                            workflow_run_id,
                            trace_id,
                            &job.protocol_id,
                            "FR-EVT-TB-001",
                            "task_board_entry_added",
                            json!({
                                "wp_id": entry.wp_id.as_str(),
                                "task_board_status": task_board_status_db(entry.status),
                                "token": entry.token.as_str(),
                            }),
                        ),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await?;
            }

            record_event_required(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LocusTaskBoardSynced,
                    FlightRecorderActor::Agent,
                    trace_id,
                    locus_event_payload(
                        job,
                        workflow_run_id,
                        trace_id,
                        &job.protocol_id,
                        "FR-EVT-TB-002",
                        "task_board_synced",
                        json!({
                            "dry_run": dry_run,
                            "applied_updates": applied_updates,
                            "unknown_wp_ids": unknown_wp_ids,
                            "task_board_written": task_board_written,
                            "entries_added": entries_added_count,
                            "entries_removed": entries_removed_count,
                            "status_changes": status_changes,
                        }),
                    ),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await?;

            let duration_ms = sync_started_at.elapsed().as_millis() as u64;
            record_event_required(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LocusSyncCompleted,
                    FlightRecorderActor::Agent,
                    trace_id,
                    locus_event_payload(
                        job,
                        workflow_run_id,
                        trace_id,
                        &job.protocol_id,
                        "FR-EVT-SYNC-002",
                        "sync_completed",
                        json!({
                            "sync_target": sync_target.as_str(),
                            "duration_ms": duration_ms,
                        }),
                    ),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await?;

            Ok(output)
        }
        Err(err) => {
            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LocusSyncFailed,
                    FlightRecorderActor::Agent,
                    trace_id,
                    locus_event_payload(
                        job,
                        workflow_run_id,
                        trace_id,
                        &job.protocol_id,
                        "FR-EVT-SYNC-003",
                        "sync_failed",
                        json!({
                            "sync_target": sync_target.as_str(),
                            "error": err.to_string(),
                        }),
                    ),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;
            Err(err)
        }
    }
}

// ============================================================================
// Model Swap Protocol [§4.3.3.4.3-4.3.3.4.4]
// ============================================================================

const MODEL_SWAP_SCHEMA_VERSION_V0_4: &str = "hsk.model_swap@0.4";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSwapRole {
    Frontend,
    Orchestrator,
    Worker,
    Validator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSwapPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSwapStrategy {
    UnloadReload,
    KeepHotSwap,
    DiskOffload,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSwapRequesterSubsystem {
    MtExecutor,
    Governance,
    Ui,
    Orchestrator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelSwapRequesterV0_4 {
    pub subsystem: ModelSwapRequesterSubsystem,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub wp_id: Option<String>,
    #[serde(default)]
    pub mt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSwapRequestV0_4 {
    pub schema_version: String,
    pub request_id: String,

    pub current_model_id: String,
    pub target_model_id: String,

    pub role: ModelSwapRole,
    pub priority: ModelSwapPriority,
    pub reason: String,

    pub swap_strategy: ModelSwapStrategy,

    pub state_persist_refs: Vec<String>,
    pub state_hash: String,
    pub context_compile_ref: String,

    pub max_vram_mb: u64,
    pub max_ram_mb: u64,
    pub timeout_ms: u64,

    pub requester: ModelSwapRequesterV0_4,

    #[serde(default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistedModelSwapStateV0_4 {
    pub schema_version: String,
    pub request_id: String,
    pub current_model_id: String,
    pub target_model_id: String,
    pub role: ModelSwapRole,
    pub priority: ModelSwapPriority,
    pub reason: String,
    pub swap_strategy: ModelSwapStrategy,
    pub state_persist_refs: Vec<String>,
    pub context_compile_ref: String,
    pub max_vram_mb: u64,
    pub max_ram_mb: u64,
    pub timeout_ms: u64,
    pub requester: ModelSwapRequesterV0_4,
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone)]
struct PendingModelSwapV0_4 {
    request: ModelSwapRequestV0_4,
    context_compile_rel: PathBuf,
}

fn model_swap_min_budgets_mb_for_model_id(model_id: &str) -> Option<(u64, u64)> {
    match model_id {
        "qwen2.5-coder:7b" => Some((4096, 16384)),
        "qwen2.5-coder:13b" => Some((8192, 32768)),
        "qwen2.5-coder:32b" => Some((24576, 65536)),
        _ => None,
    }
}

fn model_swap_strategy_str(strategy: ModelSwapStrategy) -> &'static str {
    match strategy {
        ModelSwapStrategy::UnloadReload => "unload_reload",
        ModelSwapStrategy::KeepHotSwap => "keep_hot_swap",
        ModelSwapStrategy::DiskOffload => "disk_offload",
    }
}

async fn record_model_swap_event_v0_4(
    state: &AppState,
    event_type: FlightRecorderEventType,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: &str,
    request: &ModelSwapRequestV0_4,
    payload_type: &'static str,
    outcome: Option<&'static str>,
    error_summary: Option<&str>,
) {
    let mut payload = serde_json::Map::new();
    payload.insert("type".to_string(), json!(payload_type));
    payload.insert("request_id".to_string(), json!(request.request_id.clone()));
    payload.insert(
        "current_model_id".to_string(),
        json!(request.current_model_id.clone()),
    );
    payload.insert(
        "target_model_id".to_string(),
        json!(request.target_model_id.clone()),
    );
    payload.insert("role".to_string(), json!("worker"));
    payload.insert("reason".to_string(), json!(request.reason.clone()));
    payload.insert(
        "swap_strategy".to_string(),
        json!(model_swap_strategy_str(request.swap_strategy)),
    );
    payload.insert("max_vram_mb".to_string(), json!(request.max_vram_mb));
    payload.insert("max_ram_mb".to_string(), json!(request.max_ram_mb));
    payload.insert("timeout_ms".to_string(), json!(request.timeout_ms));
    payload.insert(
        "state_persist_refs".to_string(),
        json!(request.state_persist_refs.clone()),
    );
    payload.insert("state_hash".to_string(), json!(request.state_hash.clone()));
    payload.insert(
        "context_compile_ref".to_string(),
        json!(request.context_compile_ref.clone()),
    );
    payload.insert("wp_id".to_string(), json!(wp_id));
    payload.insert("mt_id".to_string(), json!(mt_id));
    if let Some(outcome) = outcome {
        payload.insert("outcome".to_string(), json!(outcome));
    }
    if let Some(error_summary) = error_summary {
        payload.insert("error_summary".to_string(), json!(error_summary));
    }

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            event_type,
            FlightRecorderActor::System,
            trace_id,
            Value::Object(payload),
        )
        .with_job_id(job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;
}

impl ModelSwapRequestV0_4 {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != MODEL_SWAP_SCHEMA_VERSION_V0_4 {
            let msg = format!(
                "invalid model swap schema_version: expected={}, got={}",
                MODEL_SWAP_SCHEMA_VERSION_V0_4, self.schema_version
            );
            return Err(msg);
        }

        if !is_safe_id_string(self.request_id.as_str(), 256) {
            let msg = "invalid model swap field request_id: must be a safe id".to_string();
            return Err(msg);
        }

        for (field, value, max_len) in [
            ("current_model_id", self.current_model_id.as_str(), 256usize),
            ("target_model_id", self.target_model_id.as_str(), 256usize),
            (
                "context_compile_ref",
                self.context_compile_ref.as_str(),
                512usize,
            ),
        ] {
            if !is_bounded_token(value, max_len) {
                let msg = format!(
                    "invalid model swap field {field}: must be non-empty and <= {max_len} chars"
                );
                return Err(msg);
            }
        }

        if self.reason.trim().is_empty() {
            let msg = "invalid model swap field reason: must be non-empty".to_string();
            return Err(msg);
        }

        if self.state_persist_refs.is_empty() {
            let msg = "invalid model swap field state_persist_refs: must contain at least one ref"
                .to_string();
            return Err(msg);
        }
        for (idx, value) in self.state_persist_refs.iter().enumerate() {
            if !is_bounded_token(value, 512) {
                let msg = format!(
                    "invalid model swap field state_persist_refs[{idx}]: must be non-empty and <= 512 chars"
                );
                return Err(msg);
            }
        }

        if !is_sha256_hex_lowercase(self.state_hash.as_str()) {
            let msg = "invalid model swap field state_hash: expected 64-char lowercase sha256 hex"
                .to_string();
            return Err(msg);
        }

        Ok(())
    }
}

fn is_bounded_token(value: &str, max_len: usize) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return false;
    }
    !trimmed.chars().any(|c| c.is_control())
}

fn is_safe_id_string(value: &str, max_len: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_sha256_hex_lowercase(value: &str) -> bool {
    let value = value.trim();
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}

fn model_swap_state_ref_to_abs_path(
    repo_root: &Path,
    state_ref: &str,
) -> Result<PathBuf, WorkflowError> {
    let state_ref = state_ref.trim();
    if state_ref.is_empty() {
        return Err(WorkflowError::Terminal(
            "invalid model swap state ref: empty".to_string(),
        ));
    }

    let rel_path = if let Some(rest) = state_ref.strip_prefix("artifact:") {
        let mut parts = rest.splitn(2, ':');
        let _artifact_id = parts.next().unwrap_or_default();
        let Some(path) = parts.next() else {
            return Err(WorkflowError::Terminal(format!(
                "invalid model swap state ref: expected artifact:<uuid>:<path>, got={state_ref}"
            )));
        };
        path
    } else {
        state_ref
    };

    // Accept both "/data/..." and "data/..." forms.
    let rel_path = rel_path.strip_prefix('/').unwrap_or(rel_path);
    Ok(repo_root.join(rel_path))
}

fn compute_model_swap_state_hash_v0_4(
    repo_root: &Path,
    state_persist_refs: &[String],
) -> Result<String, WorkflowError> {
    if state_persist_refs.is_empty() {
        return Err(WorkflowError::Terminal(
            "invalid model swap state_persist_refs: must contain at least one ref".to_string(),
        ));
    }

    let mut ref_hashes: Vec<(String, String)> = Vec::with_capacity(state_persist_refs.len());
    for state_ref in state_persist_refs {
        let abs_path = model_swap_state_ref_to_abs_path(repo_root, state_ref)?;
        let bytes = fs::read(&abs_path).map_err(|e| {
            WorkflowError::Terminal(format!(
                "failed to read model swap state_persist_ref {} (resolved to {}): {e}",
                state_ref,
                abs_path.display()
            ))
        })?;
        ref_hashes.push((state_ref.clone(), sha256_hex(&bytes)));
    }

    ref_hashes.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut manifest = String::new();
    for (state_ref, file_hash) in ref_hashes {
        manifest.push_str(&state_ref);
        manifest.push('\n');
        manifest.push_str(&file_hash);
        manifest.push('\n');
    }

    Ok(sha256_hex(manifest.as_bytes()))
}

fn persist_model_swap_state_v0_4(
    repo_root: &Path,
    job_id: Uuid,
    request: &ModelSwapRequestV0_4,
) -> Result<(PathBuf, String), WorkflowError> {
    request
        .validate()
        .map_err(|e| WorkflowError::Terminal(format!("invalid model swap request: {e}")))?;

    let state = PersistedModelSwapStateV0_4 {
        schema_version: request.schema_version.clone(),
        request_id: request.request_id.clone(),
        current_model_id: request.current_model_id.clone(),
        target_model_id: request.target_model_id.clone(),
        role: request.role,
        priority: request.priority,
        reason: request.reason.clone(),
        swap_strategy: request.swap_strategy,
        state_persist_refs: request.state_persist_refs.clone(),
        context_compile_ref: request.context_compile_ref.clone(),
        max_vram_mb: request.max_vram_mb,
        max_ram_mb: request.max_ram_mb,
        timeout_ms: request.timeout_ms,
        requester: request.requester.clone(),
        metadata: request.metadata.clone(),
    };

    let job_dir_rel = micro_task_job_dir_rel(job_id);
    let swap_dir_rel = job_dir_rel.join("model_swap");
    let state_rel = swap_dir_rel.join(format!("swap_state_{}.json", request.request_id));
    let state_abs = repo_root.join(&state_rel);

    write_json_atomic(repo_root, &state_abs, &state)?;
    let state_hash = compute_model_swap_state_hash_v0_4(repo_root, &state.state_persist_refs)?;
    if !is_sha256_hex_lowercase(&state_hash) {
        return Err(WorkflowError::Terminal(format!(
            "model swap state_hash was not lowercase sha256 hex: {state_hash}"
        )));
    }

    Ok((state_abs, state_hash))
}

fn persist_model_swap_request_v0_4(
    repo_root: &Path,
    job_id: Uuid,
    request: &ModelSwapRequestV0_4,
) -> Result<PathBuf, WorkflowError> {
    request
        .validate()
        .map_err(|e| WorkflowError::Terminal(format!("invalid model swap request: {e}")))?;

    let job_dir_rel = micro_task_job_dir_rel(job_id);
    let swap_dir_rel = job_dir_rel.join("model_swap");
    let request_rel = swap_dir_rel.join(format!("request_{}.json", request.request_id));
    let request_abs = repo_root.join(&request_rel);
    write_json_atomic(repo_root, &request_abs, request)?;
    Ok(request_abs)
}

fn verify_model_swap_state_hash_v0_4(
    repo_root: &Path,
    abs_state_path: &Path,
    expected_hash: &str,
) -> Result<(), WorkflowError> {
    if !is_sha256_hex_lowercase(expected_hash) {
        return Err(WorkflowError::Terminal(
            "invalid expected model swap state_hash: expected 64-char lowercase sha256 hex"
                .to_string(),
        ));
    }

    let bytes = fs::read(abs_state_path).map_err(|e| {
        WorkflowError::Terminal(format!(
            "failed to read model swap state {}: {e}",
            abs_state_path.display()
        ))
    })?;
    let state: PersistedModelSwapStateV0_4 = serde_json::from_slice(&bytes).map_err(|e| {
        WorkflowError::Terminal(format!(
            "invalid model swap state JSON {}: {e}",
            abs_state_path.display()
        ))
    })?;
    let actual = compute_model_swap_state_hash_v0_4(repo_root, &state.state_persist_refs)?;
    if actual != expected_hash {
        return Err(WorkflowError::Terminal(format!(
            "model swap state_hash mismatch: expected={expected_hash}, actual={actual}"
        )));
    }

    Ok(())
}

async fn record_event_safely(state: &AppState, event: FlightRecorderEvent) {
    if let Err(err) = state.flight_recorder.record_event(event).await {
        tracing::warn!(
            target: "handshake_core::flight_recorder",
            error = %err,
            "failed to record flight recorder event"
        );
    }
}

async fn record_event_required(
    state: &AppState,
    event: FlightRecorderEvent,
) -> Result<(), WorkflowError> {
    state
        .flight_recorder
        .record_event(event)
        .await
        .map_err(|e| WorkflowError::Terminal(format!("flight recorder rejected event: {e}")))
}

fn locus_event_payload(
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
    protocol_id: &str,
    event_id: &'static str,
    event_name: &'static str,
    inner_payload: Value,
) -> Value {
    json!({
        "event_id": event_id,
        "event_name": event_name,
        "timestamp": Utc::now().to_rfc3339(),
        "trace_id": trace_id.to_string(),
        "job_id": job.job_id.to_string(),
        "workflow_run_id": workflow_run_id.to_string(),
        "protocol_id": protocol_id,
        "payload": inner_payload,
    })
}

async fn emit_locus_operation_event(
    state: &AppState,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
    op: &locus::LocusOperation,
    result: &Value,
) -> Result<(), WorkflowError> {
    fn gate_str(gate: locus::LocusGateKind) -> &'static str {
        match gate {
            locus::LocusGateKind::PreWork => "pre_work",
            locus::LocusGateKind::PostWork => "post_work",
        }
    }

    fn gate_status_str(status: locus::GateStatusKind) -> &'static str {
        match status {
            locus::GateStatusKind::Pending => "pending",
            locus::GateStatusKind::Pass => "pass",
            locus::GateStatusKind::Fail => "fail",
            locus::GateStatusKind::Skip => "skip",
        }
    }

    fn mt_outcome_str(outcome: locus::MicroTaskIterationOutcome) -> &'static str {
        match outcome {
            locus::MicroTaskIterationOutcome::Success => "SUCCESS",
            locus::MicroTaskIterationOutcome::Retry => "RETRY",
            locus::MicroTaskIterationOutcome::Escalate => "ESCALATE",
            locus::MicroTaskIterationOutcome::Blocked => "BLOCKED",
            locus::MicroTaskIterationOutcome::Skipped => "SKIPPED",
        }
    }

    fn dep_type_str(kind: locus::DependencyType) -> &'static str {
        match kind {
            locus::DependencyType::Blocks => "blocks",
            locus::DependencyType::BlockedBy => "blocked_by",
            locus::DependencyType::Related => "related",
            locus::DependencyType::ParentChild => "parent-child",
            locus::DependencyType::DiscoveredFrom => "discovered-from",
            locus::DependencyType::DuplicateOf => "duplicate-of",
            locus::DependencyType::DependsOn => "depends-on",
            locus::DependencyType::Implements => "implements",
            locus::DependencyType::Tests => "tests",
            locus::DependencyType::Documents => "documents",
        }
    }

    let mut inner = serde_json::Map::new();
    let (event_type, event_id, event_name) = match op {
        locus::LocusOperation::CreateWp(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("version".to_string(), json!(1));
            if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                inner.insert("status".to_string(), Value::String(status.to_string()));
            }
            if let Some(task_board_status) =
                result.get("task_board_status").and_then(|v| v.as_str())
            {
                inner.insert(
                    "task_board_status".to_string(),
                    Value::String(task_board_status.to_string()),
                );
            }
            inner.insert("title".to_string(), Value::String(params.title.clone()));
            (
                FlightRecorderEventType::LocusWorkPacketCreated,
                "FR-EVT-WP-001",
                "work_packet_created",
            )
        }
        locus::LocusOperation::UpdateWp(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            let updated_fields: Vec<Value> = params
                .updates
                .keys()
                .map(|k| Value::String(k.to_string()))
                .collect();
            inner.insert("updated_fields".to_string(), Value::Array(updated_fields));
            if let Some(updated_at) = result.get("updated_at").and_then(|v| v.as_str()) {
                inner.insert(
                    "updated_at".to_string(),
                    Value::String(updated_at.to_string()),
                );
            }
            if let Some(source) = &params.source {
                inner.insert("source".to_string(), Value::String(source.clone()));
            }
            (
                FlightRecorderEventType::LocusWorkPacketUpdated,
                "FR-EVT-WP-002",
                "work_packet_updated",
            )
        }
        locus::LocusOperation::GateWp(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert(
                "gate".to_string(),
                Value::String(gate_str(params.gate).to_string()),
            );
            inner.insert(
                "gate_status".to_string(),
                Value::String(gate_status_str(params.result.status).to_string()),
            );
            if let Some(notes) = params
                .result
                .notes
                .as_ref()
                .filter(|s| !s.trim().is_empty())
            {
                inner.insert("notes".to_string(), Value::String(notes.to_string()));
            }
            (
                FlightRecorderEventType::LocusWorkPacketGated,
                "FR-EVT-WP-003",
                "work_packet_gated",
            )
        }
        locus::LocusOperation::CloseWp(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("status".to_string(), Value::String("done".to_string()));
            (
                FlightRecorderEventType::LocusWorkPacketCompleted,
                "FR-EVT-WP-004",
                "work_packet_completed",
            )
        }
        locus::LocusOperation::DeleteWp(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("status".to_string(), Value::String("cancelled".to_string()));
            (
                FlightRecorderEventType::LocusWorkPacketDeleted,
                "FR-EVT-WP-005",
                "work_packet_deleted",
            )
        }
        locus::LocusOperation::RegisterMts(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            let mt_ids: Vec<Value> = params
                .micro_tasks
                .iter()
                .map(|mt| Value::String(mt.mt_id.clone()))
                .collect();
            inner.insert("mt_ids".to_string(), Value::Array(mt_ids));
            inner.insert("count".to_string(), json!(params.micro_tasks.len() as u64));
            (
                FlightRecorderEventType::LocusMicroTasksRegistered,
                "FR-EVT-MT-001",
                "micro_tasks_registered",
            )
        }
        locus::LocusOperation::StartMt(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("mt_id".to_string(), Value::String(params.mt_id.clone()));
            (
                FlightRecorderEventType::LocusMtStarted,
                "FR-EVT-MT-003",
                "mt_started",
            )
        }
        locus::LocusOperation::RecordIteration(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("mt_id".to_string(), Value::String(params.mt_id.clone()));
            inner.insert("iteration".to_string(), json!(params.iteration.iteration));
            inner.insert(
                "model_id".to_string(),
                Value::String(params.iteration.model_id.clone()),
            );
            if let Some(lora_id) = &params.iteration.lora_id {
                inner.insert("lora_id".to_string(), Value::String(lora_id.clone()));
            }
            inner.insert(
                "escalation_level".to_string(),
                json!(params.iteration.escalation_level),
            );
            inner.insert(
                "tokens_prompt".to_string(),
                json!(params.iteration.tokens_prompt),
            );
            inner.insert(
                "tokens_completion".to_string(),
                json!(params.iteration.tokens_completion),
            );
            inner.insert(
                "outcome".to_string(),
                Value::String(mt_outcome_str(params.iteration.outcome).to_string()),
            );
            if let Some(validation_passed) = params.iteration.validation_passed {
                inner.insert(
                    "validation_passed".to_string(),
                    Value::Bool(validation_passed),
                );
            }
            inner.insert(
                "duration_ms".to_string(),
                json!(params.iteration.duration_ms),
            );
            inner.insert(
                "output_artifact_ref".to_string(),
                params.iteration.output_artifact_ref.clone(),
            );
            if let Some(ref value) = params.iteration.validation_artifact_ref {
                inner.insert("validation_artifact_ref".to_string(), value.clone());
            }
            if let Some(error_summary) = params
                .iteration
                .error_summary
                .as_ref()
                .filter(|s| !s.trim().is_empty())
            {
                inner.insert(
                    "error_summary".to_string(),
                    Value::String(error_summary.clone()),
                );
            }
            if let Some(failure_category) = params
                .iteration
                .failure_category
                .as_ref()
                .filter(|s| !s.trim().is_empty())
            {
                inner.insert(
                    "failure_category".to_string(),
                    Value::String(failure_category.clone()),
                );
            }
            (
                FlightRecorderEventType::LocusMtIterationCompleted,
                "FR-EVT-MT-002",
                "mt_iteration_completed",
            )
        }
        locus::LocusOperation::CompleteMt(params) => {
            inner.insert("wp_id".to_string(), Value::String(params.wp_id.clone()));
            inner.insert("mt_id".to_string(), Value::String(params.mt_id.clone()));
            inner.insert("final_iteration".to_string(), json!(params.final_iteration));
            (
                FlightRecorderEventType::LocusMtCompleted,
                "FR-EVT-MT-004",
                "mt_completed",
            )
        }
        locus::LocusOperation::AddDependency(params) => {
            inner.insert(
                "dependency_id".to_string(),
                Value::String(params.dependency_id.clone()),
            );
            inner.insert(
                "from_wp_id".to_string(),
                Value::String(params.from_wp_id.clone()),
            );
            inner.insert(
                "to_wp_id".to_string(),
                Value::String(params.to_wp_id.clone()),
            );
            inner.insert(
                "type".to_string(),
                Value::String(dep_type_str(params.kind).to_string()),
            );
            (
                FlightRecorderEventType::LocusDependencyAdded,
                "FR-EVT-DEP-001",
                "dependency_added",
            )
        }
        locus::LocusOperation::RemoveDependency(params) => {
            inner.insert(
                "dependency_id".to_string(),
                Value::String(params.dependency_id.clone()),
            );
            (
                FlightRecorderEventType::LocusDependencyRemoved,
                "FR-EVT-DEP-002",
                "dependency_removed",
            )
        }
        locus::LocusOperation::QueryReady(params) => {
            let result_count = result
                .get("wp_ids")
                .and_then(|v| v.as_array())
                .map(|v| v.len() as u64)
                .unwrap_or(0);
            inner.insert(
                "query_op".to_string(),
                Value::String("query_ready".to_string()),
            );
            inner.insert("result_count".to_string(), json!(result_count));
            if let Some(limit) = params.limit {
                inner.insert("limit".to_string(), json!(limit));
            }
            (
                FlightRecorderEventType::LocusWorkQueryExecuted,
                "FR-EVT-QUERY-001",
                "work_query_executed",
            )
        }
        locus::LocusOperation::GetWpStatus(params) => {
            inner.insert(
                "query_op".to_string(),
                Value::String("get_wp_status".to_string()),
            );
            inner.insert("result_count".to_string(), json!(1u64));
            inner.insert("filters".to_string(), json!({ "wp_id": params.wp_id }));
            (
                FlightRecorderEventType::LocusWorkQueryExecuted,
                "FR-EVT-QUERY-001",
                "work_query_executed",
            )
        }
        locus::LocusOperation::GetMtProgress(params) => {
            inner.insert(
                "query_op".to_string(),
                Value::String("get_mt_progress".to_string()),
            );
            inner.insert("result_count".to_string(), json!(1u64));
            inner.insert("filters".to_string(), json!({ "mt_id": params.mt_id }));
            (
                FlightRecorderEventType::LocusWorkQueryExecuted,
                "FR-EVT-QUERY-001",
                "work_query_executed",
            )
        }
        locus::LocusOperation::SyncTaskBoard(_) => return Ok(()),
    };

    let payload = locus_event_payload(
        job,
        workflow_run_id,
        trace_id,
        &job.protocol_id,
        event_id,
        event_name,
        Value::Object(inner),
    );
    record_event_required(
        state,
        FlightRecorderEvent::new(event_type, FlightRecorderActor::Agent, trace_id, payload)
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
    )
    .await?;

    if let locus::LocusOperation::RecordIteration(params) = op {
        match params.iteration.outcome {
            locus::MicroTaskIterationOutcome::Escalate => {
                let to_level = params.iteration.escalation_level;
                let from_level = to_level.saturating_sub(1);
                record_event_required(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::LocusMtEscalated,
                        FlightRecorderActor::Agent,
                        trace_id,
                        locus_event_payload(
                            job,
                            workflow_run_id,
                            trace_id,
                            &job.protocol_id,
                            "FR-EVT-MT-005",
                            "mt_escalated",
                            json!({
                                "wp_id": params.wp_id.as_str(),
                                "mt_id": params.mt_id.as_str(),
                                "from_model": params.iteration.model_id.as_str(),
                                "to_model": params.iteration.model_id.as_str(),
                                "from_level": from_level,
                                "to_level": to_level,
                                "reason": "iteration outcome=ESCALATE",
                            }),
                        ),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await?;
            }
            locus::MicroTaskIterationOutcome::Blocked => {
                let failure_category = params
                    .iteration
                    .failure_category
                    .as_ref()
                    .filter(|s| !s.trim().is_empty())
                    .map(String::as_str)
                    .unwrap_or("blocked");
                let error_summary = params
                    .iteration
                    .error_summary
                    .as_ref()
                    .filter(|s| !s.trim().is_empty())
                    .map(String::as_str)
                    .unwrap_or("mt blocked");
                record_event_required(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::LocusMtFailed,
                        FlightRecorderActor::Agent,
                        trace_id,
                        locus_event_payload(
                            job,
                            workflow_run_id,
                            trace_id,
                            &job.protocol_id,
                            "FR-EVT-MT-006",
                            "mt_failed",
                            json!({
                                "wp_id": params.wp_id.as_str(),
                                "mt_id": params.mt_id.as_str(),
                                "failure_category": failure_category,
                                "error_summary": error_summary,
                            }),
                        ),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await?;
            }
            _ => {}
        }
    }
    Ok(())
}

// ============================================================================
// Security Violation Handling [HSK-ACE-VAL-101]
// ============================================================================

/// Handle a security violation with atomic poisoning [HSK-ACE-VAL-101]
///
/// This function MUST:
/// 1. Emit FR-EVT-SEC-VIOLATION to Flight Recorder
/// 2. Transition job to JobState::Poisoned
/// 3. Terminate all workflow nodes atomically
/// 4. Update workflow run status
///
/// Per ┬º2.6.6.7.11.0, security violations trigger immediate job poisoning
/// to prevent any further processing of potentially compromised content.
#[allow(clippy::too_many_arguments)] // Explicit args keep FR payload + state transitions clear
pub async fn handle_security_violation(
    state: &AppState,
    job: &AiJob,
    workflow_run: &WorkflowRun,
    violation: &AceError,
    violation_type: SecurityViolationType,
    guard_name: &str,
    trace_id: Uuid,
    offset: Option<usize>,
    context: Option<String>,
) -> Result<(), WorkflowError> {
    let violation_type_str = match violation_type {
        SecurityViolationType::PromptInjection => "prompt_injection",
        SecurityViolationType::CloudLeakage => "cloud_leakage",
        SecurityViolationType::SensitivityViolation => "sensitivity_violation",
        SecurityViolationType::UnknownSensitivity => "unknown_sensitivity",
        SecurityViolationType::ExportViolation => "export_violation",
    };
    let trigger = match violation {
        AceError::PromptInjectionDetected { pattern, .. } => pattern.clone(),
        AceError::CloudLeakageBlocked { reason } => reason.clone(),
        _ => violation.to_string(),
    };

    // 1. Emit FR-EVT-SEC-VIOLATION to Flight Recorder
    let payload = FrEvt008SecurityViolation {
        violation_type: violation_type_str.to_string(),
        description: violation.to_string(),
        source_id: None, // Could be enhanced to include source_ref if available
        trigger: trigger.clone(),
        guard_name: guard_name.to_string(),
        offset,
        context: context.clone(),
        action_taken: "poisoned".to_string(),
        job_state_transition: Some("poisoned".to_string()),
    };

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::SecurityViolation,
            FlightRecorderActor::System,
            trace_id,
            serde_json::to_value(&payload).unwrap_or(json!({})),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run.id.to_string()),
    )
    .await;

    tracing::error!(
        target: "handshake_core::security",
        job_id = %job.job_id,
        workflow_id = %workflow_run.id,
        violation_type = %violation_type_str,
        trigger = %trigger,
        guard = %guard_name,
        offset = ?offset,
        context = ?context,
        "Security violation detected - poisoning job"
    );

    // 2. Terminate all workflow nodes atomically
    let nodes = state
        .storage
        .list_workflow_node_executions(workflow_run.id)
        .await?;

    for node in nodes {
        if matches!(node.status, JobState::Running | JobState::Queued) {
            let _ = state
                .storage
                .update_workflow_node_execution_status(
                    node.id,
                    JobState::Poisoned,
                    None,
                    Some(format!("Security violation: {}", violation)),
                )
                .await;
        }
    }

    // 3. Transition job to JobState::Poisoned
    state
        .storage
        .update_ai_job_status(JobStatusUpdate {
            job_id: job.job_id,
            state: JobState::Poisoned,
            error_message: Some(format!("Security violation: {}", violation)),
            status_reason: format!("poisoned: {}", violation_type_str),
            metrics: None,
            workflow_run_id: Some(workflow_run.id),
            trace_id: Some(trace_id),
            job_outputs: None,
        })
        .await?;

    // 4. Update workflow run status
    state
        .storage
        .update_workflow_run_status(
            workflow_run.id,
            JobState::Poisoned,
            Some(format!("Security violation: {}", violation)),
        )
        .await?;

    Ok(())
}

/// Check if an AceError represents a security violation that requires poisoning
pub fn is_poisonable_violation(error: &AceError) -> bool {
    matches!(
        error,
        AceError::PromptInjectionDetected { .. }
            | AceError::CloudLeakageBlocked { .. }
            | AceError::ValidationFailed { .. }
            | AceError::BudgetExceeded { .. }
    )
}

/// Get the SecurityViolationType for an AceError
pub fn get_violation_type(error: &AceError) -> SecurityViolationType {
    match error {
        AceError::PromptInjectionDetected { .. } => SecurityViolationType::PromptInjection,
        AceError::CloudLeakageBlocked { .. } => SecurityViolationType::CloudLeakage,
        _ => SecurityViolationType::SensitivityViolation,
    }
}

pub async fn mark_stalled_workflows(
    state: &AppState,
    threshold_secs: u64,
    is_startup_recovery: bool,
) -> Result<Vec<WorkflowRun>, WorkflowError> {
    if !is_startup_recovery {
        tracing::debug!(
            target: "handshake_core::recovery",
            "Skipping workflow recovery outside startup scan"
        );
        return Ok(Vec::new());
    }

    let stalled = state.storage.find_stalled_workflows(threshold_secs).await?;

    for run in &stalled {
        let reason = "stalled (startup recovery: heartbeat timeout)";

        state
            .storage
            .update_workflow_run_status(run.id, JobState::Stalled, Some(reason.to_string()))
            .await?;

        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: run.job_id,
                state: JobState::Stalled,
                error_message: Some(reason.to_string()),
                status_reason: "stalled".to_string(),
                metrics: None,
                workflow_run_id: Some(run.id),
                trace_id: None,
                job_outputs: None,
            })
            .await?;

        let payload = FrEvtWorkflowRecovery {
            workflow_run_id: run.id.to_string(),
            job_id: Some(run.job_id.to_string()),
            from_state: run.status.as_str().to_string(),
            to_state: "stalled".to_string(),
            reason: reason.to_string(),
            last_heartbeat_ts: run.last_heartbeat.to_rfc3339(),
            threshold_secs,
        };

        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::WorkflowRecovery,
                FlightRecorderActor::System,
                Uuid::new_v4(),
                serde_json::to_value(&payload).unwrap_or(json!({})),
            )
            .with_job_id(run.job_id.to_string())
            .with_workflow_id(run.id.to_string()),
        )
        .await;
    }

    Ok(stalled)
}

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    let trace_id = derive_trace_id(&job, None);

    if let Err(err) = enforce_capabilities(state, &job, trace_id).await {
        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.job_id,
                state: JobState::Failed,
                error_message: Some(err.to_string()),
                status_reason: err.to_string(),
                metrics: None,
                workflow_run_id: None,
                trace_id: Some(trace_id),
                job_outputs: None,
            })
            .await?;
        return Err(err);
    }

    let heartbeat_at = Utc::now();

    let workflow_run = state
        .storage
        .create_workflow_run(job.job_id, JobState::Running, Some(heartbeat_at))
        .await?;

    state
        .storage
        .heartbeat_workflow(workflow_run.id, heartbeat_at)
        .await?;

    let node_exec = state
        .storage
        .create_workflow_node_execution(NewNodeExecution {
            workflow_run_id: workflow_run.id,
            node_id: job.job_id.to_string(),
            node_type: job.job_kind.as_str().to_string(),
            status: JobState::Running,
            sequence: 1,
            input_payload: job.job_inputs.clone(),
            started_at: heartbeat_at,
        })
        .await?;

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::Agent,
            trace_id,
            json!({ "status": workflow_run.status.as_str() }),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run.id.to_string()),
    )
    .await;

    let result = run_job(state, &job, workflow_run.id, trace_id).await;

    let (final_status, error_message, status_reason, output_payload, captured_error) = match result
    {
        Ok(outcome) => (
            outcome.state,
            outcome.error_message.clone(),
            outcome.status_reason.clone(),
            outcome.output,
            None,
        ),
        Err(WorkflowError::SecurityViolation(ace_err)) if is_poisonable_violation(&ace_err) => {
            let violation_type = get_violation_type(&ace_err);
            let (offset, context) = match &ace_err {
                AceError::PromptInjectionDetected {
                    offset, context, ..
                } => (Some(*offset), Some(context.clone())),
                _ => (None, None),
            };
            handle_security_violation(
                state,
                &job,
                &workflow_run,
                &ace_err,
                violation_type,
                "PromptInjectionGuard",
                trace_id,
                offset,
                context,
            )
            .await?;
            return Err(WorkflowError::SecurityViolation(ace_err));
        }
        Err(e) => {
            let msg = e.to_string();
            (
                JobState::Failed,
                Some(msg.clone()),
                msg.clone(),
                None,
                Some(e),
            )
        }
    };

    let updated_node = state
        .storage
        .update_workflow_node_execution_status(
            node_exec.id,
            final_status.clone(),
            output_payload.clone(),
            error_message.clone(),
        )
        .await?;

    state
        .storage
        .heartbeat_workflow(workflow_run.id, Utc::now())
        .await?;

    state
        .storage
        .update_ai_job_status(JobStatusUpdate {
            job_id: job.job_id,
            state: final_status.clone(),
            error_message: error_message.clone(),
            status_reason,
            metrics: None,
            workflow_run_id: Some(workflow_run.id),
            trace_id: Some(trace_id),
            job_outputs: output_payload.clone(),
        })
        .await?;

    let completed_run = state
        .storage
        .update_workflow_run_status(workflow_run.id, final_status.clone(), error_message.clone())
        .await?;

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::Agent,
            trace_id,
            json!({
                "status": completed_run.status.as_str(),
                "error": error_message,
                "node_id": updated_node.node_id
            }),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(completed_run.id.to_string()),
    )
    .await;

    captured_error.map_or(Ok(completed_run), Err)
}

#[derive(Debug, Clone)]
struct RunJobOutcome {
    state: JobState,
    status_reason: String,
    output: Option<Value>,
    error_message: Option<String>,
}

async fn log_capability_check(
    state: &AppState,
    job: &AiJob,
    capability_id: &str,
    decision_outcome: &str,
    trace_id: Uuid,
) {
    let actor_id = "workflow_engine";
    let payload = json!({
        "capability_id": capability_id,
        "actor_id": actor_id,
        "job_id": job.job_id.to_string(),
        "decision_outcome": decision_outcome,
    });
    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::CapabilityAction,
            FlightRecorderActor::Agent,
            trace_id,
            payload,
        )
        .with_job_id(job.job_id.to_string())
        .with_capability(capability_id.to_string())
        .with_actor_id(actor_id),
    )
    .await;
}

async fn enforce_capabilities(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
) -> Result<(), WorkflowError> {
    let required = state
        .capability_registry
        .required_capabilities_for_job_request(job.job_kind.as_str(), &job.protocol_id)?;

    for capability_id in required {
        let result = state
            .capability_registry
            .profile_can(&job.capability_profile_id, &capability_id);

        match result {
            Ok(true) => {
                log_capability_check(state, job, &capability_id, "allow", trace_id).await;
            }
            Ok(false) => {
                log_capability_check(state, job, &capability_id, "deny", trace_id).await;
                return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                    capability_id,
                )));
            }
            Err(err) => {
                log_capability_check(state, job, &capability_id, "deny", trace_id).await;
                return Err(WorkflowError::Capability(err));
            }
        }
    }

    Ok(())
}

async fn enforce_work_packet_binding_if_required(
    state: &AppState,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
) -> Result<Option<RunJobOutcome>, WorkflowError> {
    let inputs = parse_inputs(job.job_inputs.as_ref());
    let binding_required = inputs
        .get("work_packet_binding_required")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || inputs.get("work_packet_binding").is_some();

    if !binding_required {
        return Ok(None);
    }

    let Some(binding) = inputs.get("work_packet_binding") else {
        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::Diagnostic,
                FlightRecorderActor::Agent,
                trace_id,
                json!({
                    "diagnostic_id": "HSK-LOCUS-WPB-MISSING",
                    "reason": "work_packet_binding required but missing",
                }),
            )
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
        )
        .await;
        return Ok(Some(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "missing_work_packet_binding".to_string(),
            output: None,
            error_message: Some("work_packet_binding required but missing".to_string()),
        }));
    };

    if !binding.is_object() {
        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::Diagnostic,
                FlightRecorderActor::Agent,
                trace_id,
                json!({
                    "diagnostic_id": "HSK-LOCUS-WPB-INVALID",
                    "reason": "work_packet_binding must be an object",
                }),
            )
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
        )
        .await;
        return Ok(Some(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_work_packet_binding".to_string(),
            output: None,
            error_message: Some("work_packet_binding must be an object".to_string()),
        }));
    }

    let wp_id = binding
        .get("work_packet_id")
        .and_then(|v| v.as_str())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| StorageError::Validation("work_packet_binding.work_packet_id missing"))?;

    if !wp_id.starts_with("WP-") || wp_id.len() > 128 {
        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::Diagnostic,
                FlightRecorderActor::Agent,
                trace_id,
                json!({
                    "diagnostic_id": "HSK-LOCUS-WPB-INVALID",
                    "reason": "work_packet_binding.work_packet_id invalid format",
                    "work_packet_id": wp_id,
                }),
            )
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
        )
        .await;
        return Ok(Some(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_work_packet_id".to_string(),
            output: None,
            error_message: Some("invalid work_packet_binding.work_packet_id".to_string()),
        }));
    }

    let exists =
        crate::storage::locus_sqlite::locus_work_packet_exists(state.storage.as_ref(), wp_id)
            .await?;

    if !exists {
        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::Diagnostic,
                FlightRecorderActor::Agent,
                trace_id,
                json!({
                    "diagnostic_id": "HSK-LOCUS-WPB-NOTFOUND",
                    "reason": "work_packet_binding.work_packet_id does not exist in Locus",
                    "work_packet_id": wp_id,
                }),
            )
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
        )
        .await;
        return Ok(Some(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_work_packet_id".to_string(),
            output: None,
            error_message: Some("work_packet_binding.work_packet_id not found".to_string()),
        }));
    }

    Ok(None)
}

pub async fn record_cloud_escalation_consent_v0_4(
    state: &AppState,
    job: &AiJob,
    request_id: String,
    approved: bool,
    user_id: String,
    ui_surface: Option<String>,
    notes: Option<String>,
) -> Result<(), WorkflowError> {
    let request_id = request_id.trim().to_string();
    if request_id.is_empty() {
        return Err(WorkflowError::Terminal(
            "cloud escalation consent requires request_id".to_string(),
        ));
    }
    let user_id = user_id.trim().to_string();
    if user_id.is_empty() {
        return Err(WorkflowError::Terminal(
            "cloud escalation consent requires user_id".to_string(),
        ));
    }

    let ui_surface = match ui_surface.as_deref().map(|v| v.trim()) {
        Some("cloud_escalation_modal") => Some("cloud_escalation_modal".to_string()),
        Some("settings") => Some("settings".to_string()),
        Some("operator_console") => Some("operator_console".to_string()),
        _ => None,
    };
    let notes = notes
        .map(|value| {
            let mut cleaned = value.trim().replace(['\r', '\n', '\t'], " ");
            if cleaned.len() > 1024 {
                cleaned.truncate(1024);
            }
            cleaned
        })
        .filter(|value| !value.trim().is_empty());

    let repo_root = repo_root_for_artifacts()?;
    let job_dir_rel = micro_task_job_dir_rel(job.job_id);
    let progress_rel = job_dir_rel.join("progress_artifact.json");
    let progress_abs = repo_root.join(&progress_rel);

    let progress_bytes = fs::read(&progress_abs).map_err(|e| {
        WorkflowError::Terminal(format!(
            "failed to read progress artifact {}: {e}",
            progress_abs.display()
        ))
    })?;
    let mut progress: ProgressArtifact = serde_json::from_slice(&progress_bytes).map_err(|e| {
        WorkflowError::Terminal(format!(
            "invalid progress_artifact.json {}: {e}",
            progress_abs.display()
        ))
    })?;

    let Some(pending) = progress.current_state.pending_cloud_escalation.as_mut() else {
        return Err(WorkflowError::Terminal(
            "no pending cloud escalation to approve/deny".to_string(),
        ));
    };
    if pending.request_id != request_id {
        return Err(WorkflowError::Terminal(format!(
            "cloud escalation request_id mismatch: expected {}, got {}",
            pending.request_id, request_id
        )));
    }
    if pending.consent_receipt.is_some() || pending.cloud_escalation_request.is_some() {
        return Err(WorkflowError::Terminal(
            "cloud escalation consent already recorded".to_string(),
        ));
    }

    let wp_id = progress.wp_id.clone();
    let mt_id = pending.mt_id.clone();
    let requested_model_id = pending.requested_model_id.clone();
    let reason = pending.reason.clone();
    let local_attempts = pending.local_attempts;
    let last_error_summary = pending.last_error_summary.clone();
    let projection_plan_id = pending.projection_plan.projection_plan_id.clone();
    let payload_sha256 = pending.projection_plan.payload_sha256.clone();

    let consent_receipt_id = Uuid::new_v4().to_string();
    let approved_at = Utc::now().to_rfc3339();
    let receipt = ConsentReceiptV0_4 {
        schema_version: "hsk.consent_receipt@0.4".to_string(),
        consent_receipt_id: consent_receipt_id.clone(),
        projection_plan_id: projection_plan_id.clone(),
        payload_sha256,
        approved,
        approved_at,
        user_id,
        ui_surface,
        notes,
    };

    let cloud_request = CloudEscalationRequestV0_4 {
        schema_version: "hsk.cloud_escalation@0.4".to_string(),
        request_id: request_id.clone(),
        wp_id: wp_id.clone(),
        mt_id: mt_id.clone(),
        reason: reason.clone(),
        local_attempts,
        last_error_summary: last_error_summary.clone(),
        requested_model_id: requested_model_id.clone(),
        projection_plan_id: projection_plan_id.clone(),
        consent_receipt_id: consent_receipt_id.clone(),
    };

    let cloud_dir_rel = job_dir_rel
        .join("cloud_escalation")
        .join(request_id.as_str());
    let cloud_dir_abs = repo_root.join(&cloud_dir_rel);
    if !cloud_dir_abs.exists() {
        return Err(WorkflowError::Terminal(format!(
            "cloud escalation dir missing: {}",
            cloud_dir_abs.display()
        )));
    }

    let receipt_rel = cloud_dir_rel.join("consent_receipt.json");
    let receipt_abs = repo_root.join(&receipt_rel);
    if receipt_abs.exists() {
        return Err(WorkflowError::Terminal(format!(
            "consent_receipt.json already exists: {}",
            receipt_abs.display()
        )));
    }
    write_json_atomic(&repo_root, &receipt_abs, &receipt)?;

    let request_rel = cloud_dir_rel.join("cloud_escalation_request.json");
    let request_abs = repo_root.join(&request_rel);
    if request_abs.exists() {
        return Err(WorkflowError::Terminal(format!(
            "cloud_escalation_request.json already exists: {}",
            request_abs.display()
        )));
    }
    write_json_atomic(&repo_root, &request_abs, &cloud_request)?;

    pending.consent_receipt = Some(receipt);
    pending.cloud_escalation_request = Some(cloud_request);
    progress.updated_at = Utc::now();
    write_json_atomic(&repo_root, &progress_abs, &progress)?;

    let (event_type, event_type_str, outcome_str) = if approved {
        (
            FlightRecorderEventType::CloudEscalationApproved,
            "cloud_escalation_approved",
            "approved",
        )
    } else {
        (
            FlightRecorderEventType::CloudEscalationDenied,
            "cloud_escalation_denied",
            "denied",
        )
    };
    let payload = json!({
        "type": event_type_str,
        "request_id": request_id,
        "reason": reason,
        "requested_model_id": requested_model_id,
        "projection_plan_id": projection_plan_id,
        "consent_receipt_id": consent_receipt_id,
        "wp_id": wp_id,
        "mt_id": mt_id,
        "local_attempts": local_attempts,
        "last_error_summary": last_error_summary,
        "outcome": outcome_str,
    });

    let mut event = FlightRecorderEvent::new(
        event_type,
        FlightRecorderActor::Human,
        job.trace_id,
        payload,
    )
    .with_job_id(job.job_id.to_string());
    if let Some(workflow_run_id) = job.workflow_run_id {
        event = event.with_workflow_id(workflow_run_id.to_string());
    }
    record_event_safely(state, event).await;

    Ok(())
}

async fn run_job(
    state: &AppState,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
) -> Result<RunJobOutcome, WorkflowError> {
    if let Some(inputs) = job.job_inputs.as_ref() {
        if inputs
            .get("force_prompt_injection")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(WorkflowError::SecurityViolation(
                AceError::PromptInjectionDetected {
                    pattern: "test-trigger".to_string(),
                    offset: 0,
                    context: "test-trigger".to_string(),
                },
            ));
        }
    }

    if matches!(job.job_kind, JobKind::WorkflowRun) && job.profile_id == "micro_task_executor_v1" {
        return Ok(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_job_contract".to_string(),
            output: None,
            error_message: Some(
                "invalid job contract: legacy micro_task_executor_v1 jobs must use job_kind micro_task_execution (migration required)"
                    .to_string(),
            ),
        });
    }

    if let Err(err) = validate_job_contract(&job.job_kind, &job.profile_id, &job.protocol_id) {
        return Ok(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_job_contract".to_string(),
            output: None,
            error_message: Some(err.to_string()),
        });
    }

    if let Some(outcome) =
        enforce_work_packet_binding_if_required(state, job, workflow_run_id, trace_id).await?
    {
        return Ok(outcome);
    }

    if matches!(job.job_kind, JobKind::DocSummarize | JobKind::DocEdit) {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let doc_id = inputs.get("doc_id").and_then(|v| v.as_str());

        if let Some(doc_id) = doc_id {
            let blocks = state.storage.get_blocks(doc_id).await?;
            let model_tier = state.llm_client.profile().model_tier;

            // [┬º2.6.6.7.14.5] Build QueryPlan and RetrievalTrace
            // MUST fail on invalid UUIDs - returns Result
            let (query_label, query_kind) = if matches!(job.job_kind, JobKind::DocEdit) {
                ("edit selection", "doc_edit")
            } else {
                ("summarize document", "doc_summarization")
            };
            let plan = build_query_plan_from_blocks(&blocks, query_label, query_kind)
                .map_err(WorkflowError::SecurityViolation)?;
            let trace = build_retrieval_trace_from_blocks(&blocks, &plan)
                .map_err(WorkflowError::SecurityViolation)?;

            // WAIVER [CX-573E]: Instant::now() for observability per ┬º2.6.6.7.12
            let validation_start = std::time::Instant::now();

            // [┬º2.6.6.7.14.11] Run ValidatorPipeline
            let pipeline = ValidatorPipeline::with_default_guards();
            let plan_result = pipeline.validate_plan(&plan).await;
            let trace_result = pipeline.validate_trace(&trace).await;

            // Collect validation results
            let mut guards_passed = Vec::new();
            let mut guards_failed = Vec::new();
            let mut violation_codes = Vec::new();

            match &plan_result {
                Ok(()) => guards_passed.push("plan_validation".to_string()),
                Err(e) => {
                    guards_failed.push("plan_validation".to_string());
                    violation_codes.push(format!("{:?}", e));
                }
            }

            match &trace_result {
                Ok(()) => guards_passed.push("trace_validation".to_string()),
                Err(e) => {
                    guards_failed.push("trace_validation".to_string());
                    violation_codes.push(format!("{:?}", e));
                }
            }

            // [HSK-ACE-VAL-100] Content-aware security scan
            let resolver = StorageContentResolver::new(state.storage.clone());
            let source_refs: Vec<SourceRef> =
                trace.spans.iter().map(|s| s.source_ref.clone()).collect();

            match scan_content_for_security(&source_refs, &resolver, model_tier).await {
                Ok(()) => guards_passed.push("content_security".to_string()),
                Err(e) => {
                    guards_failed.push("content_security".to_string());
                    violation_codes.push(format!("{:?}", e));
                }
            }

            let validation_duration_ms = validation_start.elapsed().as_millis() as u64;

            // [HSK-ACE-VAL-101] Poison job on security violation
            if !violation_codes.is_empty() {
                state
                    .storage
                    .update_ai_job_status(JobStatusUpdate {
                        job_id: job.job_id,
                        state: JobState::Poisoned,
                        error_message: Some(format!("Security violation: {:?}", violation_codes)),
                        status_reason: "ACE validator triggered".into(),
                        metrics: None,
                        workflow_run_id: job.workflow_run_id,
                        trace_id: Some(job.trace_id),
                        job_outputs: None,
                    })
                    .await?;
                return Err(WorkflowError::SecurityViolation(
                    AceError::ValidationFailed {
                        message: violation_codes.join("; "),
                    },
                ));
            }

            // Build prompt and full text
            let full_text = blocks
                .iter()
                .map(|b| b.raw_content.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            let mut doc_edit_role_id: Option<String> = None;
            let mut doc_edit_selection: Option<
                crate::ace::validators::atelier_scope::SelectionRangeV1,
            > = None;

            let prompt = if matches!(job.job_kind, JobKind::DocEdit) {
                let role_id = inputs
                    .get("role_id")
                    .and_then(|v| v.as_str())
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or("unknown")
                    .to_string();

                let selection_value = inputs.get("selection").cloned();
                let selection: crate::ace::validators::atelier_scope::SelectionRangeV1 =
                    match selection_value {
                        Some(value) => match serde_json::from_value(value) {
                            Ok(parsed) => parsed,
                            Err(err) => {
                                return Ok(RunJobOutcome {
                                    state: JobState::Failed,
                                    status_reason: "invalid_job_inputs".to_string(),
                                    output: None,
                                    error_message: Some(format!(
                                        "invalid selection in job_inputs: {}",
                                        err
                                    )),
                                });
                            }
                        },
                        None => {
                            return Ok(RunJobOutcome {
                                state: JobState::Failed,
                                status_reason: "invalid_job_inputs".to_string(),
                                output: None,
                                error_message: Some(
                                    "missing selection in job_inputs for doc_edit".to_string(),
                                ),
                            });
                        }
                    };

                let selection_text =
                    match crate::ace::validators::atelier_scope::validate_selection_preimage(
                        &full_text, &selection,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            return Ok(RunJobOutcome {
                                state: JobState::Failed,
                                status_reason: "invalid_job_inputs".to_string(),
                                output: None,
                                error_message: Some(err.to_string()),
                            });
                        }
                    };

                doc_edit_role_id = Some(role_id.clone());
                doc_edit_selection = Some(selection);

                format!(
                    "ROLE: {role_id}\n\nTASK: Propose an improved replacement for SELECTED_TEXT ONLY.\n- Do not edit anything outside SELECTED_TEXT.\n- Output ONLY the replacement text.\n\nSELECTED_TEXT:\n{selection_text}\n"
                )
            } else {
                format!("Please summarize the following document:\n\n{}", full_text)
            };

            // Compute hashes for logging
            let prompt_envelope_hash = {
                let mut h = Sha256::new();
                h.update(prompt.as_bytes());
                hex::encode(h.finalize())
            };

            let scope_inputs_hash = {
                let scope_json = serde_json::to_string(&job.job_inputs).map_err(|e| {
                    WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("job_inputs serialization failed: {}", e),
                    })
                })?;
                let mut h = Sha256::new();
                h.update(scope_json.as_bytes());
                hex::encode(h.finalize())
            };

            let query_plan_hash = {
                let plan_json = serde_json::to_string(&plan).map_err(|e| {
                    WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("QueryPlan serialization failed: {}", e),
                    })
                })?;
                let mut h = Sha256::new();
                h.update(plan_json.as_bytes());
                hex::encode(h.finalize())
            };

            let retrieval_trace_hash = {
                let trace_json = serde_json::to_string(&trace).map_err(|e| {
                    WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("RetrievalTrace serialization failed: {}", e),
                    })
                })?;
                let mut h = Sha256::new();
                h.update(trace_json.as_bytes());
                hex::encode(h.finalize())
            };

            // Extract candidate/selected IDs and hashes
            let candidate_ids: Vec<String> = trace
                .candidates
                .iter()
                .map(|c| c.candidate_id.clone())
                .collect();
            let candidate_hashes: Vec<String> = trace
                .candidates
                .iter()
                .filter_map(|c| match &c.candidate_ref {
                    CandidateRef::Source(s) => Some(s.source_hash.clone()),
                    _ => None,
                })
                .collect();
            let selected_ids: Vec<String> = trace
                .selected
                .iter()
                .filter_map(|s| match &s.candidate_ref {
                    CandidateRef::Source(src) => Some(src.source_id.to_string()),
                    _ => None,
                })
                .collect();
            let selected_hashes: Vec<String> = trace
                .selected
                .iter()
                .filter_map(|s| match &s.candidate_ref {
                    CandidateRef::Source(src) => Some(src.source_hash.clone()),
                    _ => None,
                })
                .collect();

            // Cache markers from route_taken
            let cache_markers: Vec<Value> = trace
                .route_taken
                .iter()
                .map(|r| {
                    json!({
                        "stage": format!("{:?}", r.store),
                        "cache_hit": r.cache_hit
                    })
                })
                .collect();

            // Drift flags from warnings
            let drift_flags: Vec<String> = trace
                .warnings
                .iter()
                .filter(|w| w.contains("drift"))
                .cloned()
                .collect();

            let req = CompletionRequest::new(
                job.trace_id,
                prompt.clone(),
                state.llm_client.profile().model_id.clone(),
            );

            let response = state.llm_client.completion(req).await?;

            // Compute response hash
            let response_hash = {
                let mut h = Sha256::new();
                h.update(response.text.as_bytes());
                hex::encode(h.finalize())
            };

            let model_id = state.llm_client.profile().model_id.clone();
            let outcome = if !violation_codes.is_empty() {
                "poisoned"
            } else if !guards_failed.is_empty() {
                "blocked"
            } else {
                "passed"
            };

            // Build extended LlmInference payload with ┬º2.6.6.7.12 ACE validation fields
            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LlmInference,
                    FlightRecorderActor::Agent,
                    trace_id,
                    json!({
                        // Existing LlmInference required fields
                        "type": "llm_inference",
                        "trace_id": trace_id.to_string(),
                        "model_id": model_id.clone(),
                        "token_usage": {
                            "prompt_tokens": response.usage.prompt_tokens,
                            "completion_tokens": response.usage.completion_tokens,
                            "total_tokens": response.usage.total_tokens
                        },
                        "latency_ms": response.latency_ms,
                        "prompt_hash": prompt_envelope_hash,
                        "response_hash": response_hash,

                        // ┬º2.6.6.7.12 ACE validation fields
                        "ace_validation": {
                            // scope inputs + hashes
                            "scope_document_id": doc_id,
                            "scope_inputs_hash": scope_inputs_hash,

                            // determinism mode
                            "determinism_mode": format!("{:?}", plan.determinism_mode),

                            // candidate source IDs/hashes
                            "candidate_ids": candidate_ids,
                            "candidate_hashes": candidate_hashes,
                            "candidate_list_artifact_ref": Value::Null,  // Phase 2

                            // selected IDs/hashes
                            "selected_ids": selected_ids,
                            "selected_hashes": selected_hashes,

                            // truncation/compaction decisions
                            "truncation_applied": !trace.truncation_flags.is_empty(),
                            "truncation_flags": trace.truncation_flags.clone(),
                            "compaction_applied": false,

                            // QueryPlan ID + hash
                            "query_plan_id": plan.plan_id.to_string(),
                            "query_plan_hash": query_plan_hash,

                            // normalized_query_hash
                            "normalized_query_hash": trace.normalized_query_hash.clone(),

                            // RetrievalTrace ID + hash
                            "retrieval_trace_id": trace.trace_id.to_string(),
                            "retrieval_trace_hash": retrieval_trace_hash,

                            // rerank metadata
                            "rerank_method": if trace.rerank.used { Some(trace.rerank.method.clone()) } else { None::<String> },
                            "rerank_inputs_hash": if trace.rerank.used { Some(trace.rerank.inputs_hash.clone()) } else { None::<String> },
                            "rerank_outputs_hash": if trace.rerank.used { Some(trace.rerank.outputs_hash.clone()) } else { None::<String> },

                            // diversity metadata
                            "diversity_method": if trace.diversity.used { Some(trace.diversity.method.clone()) } else { None::<String> },
                            "diversity_lambda": trace.diversity.lambda,

                            // cache hit/miss markers
                            "cache_markers": cache_markers,

                            // drift flags + degraded marker
                            "drift_flags": drift_flags,
                            "degraded_mode": !trace.errors.is_empty(),

                            // ContextSnapshot (Phase 2)
                            "context_snapshot_id": Value::Null,
                            "context_snapshot_hash": Value::Null,

                            // artifact handles (Phase 2)
                            "artifact_handles": Vec::<String>::new(),

                            // validation results
                            "guards_passed": guards_passed,
                            "guards_failed": guards_failed,
                            "violation_codes": violation_codes,
                            "outcome": outcome,

                            // model tier
                            "model_tier": format!("{:?}", model_tier),

                            // timing
                            "validation_duration_ms": validation_duration_ms
                        }
                    }),
                )
                .with_job_id(job.job_id.to_string())
                .with_model_id(model_id.clone()),
            )
            .await;

            let output = if matches!(job.job_kind, JobKind::DocEdit) {
                let role_id = doc_edit_role_id.unwrap_or_else(|| "unknown".to_string());
                let selection = match doc_edit_selection {
                    Some(selection) => selection,
                    None => {
                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "invalid_job_inputs".to_string(),
                            output: None,
                            error_message: Some(
                                "missing selection in job_inputs for doc_edit".to_string(),
                            ),
                        });
                    }
                };

                let selection_len_utf8 = selection.end_utf8.saturating_sub(selection.start_utf8);
                let patchset = crate::ace::validators::atelier_scope::DocPatchsetV1 {
                    schema_version: "hsk.doc_patchset@v1".to_string(),
                    doc_id: doc_id.to_string(),
                    selection: selection.clone(),
                    boundary_normalization: "disabled".to_string(),
                    ops: vec![
                        crate::ace::validators::atelier_scope::PatchOpV1::ReplaceRange {
                            range_utf8: crate::ace::validators::atelier_scope::RangeUtf8 {
                                start: 0,
                                end: selection_len_utf8,
                            },
                            insert_text: response.text.clone(),
                        },
                    ],
                    summary: None,
                };

                let suggestion = json!({
                    "suggestion_id": Uuid::new_v4().to_string(),
                    "role_id": role_id.clone(),
                    "contract_id": format!("ROLE:{role_id}:C:1"),
                    "title": "Suggested edit".to_string(),
                    "rationale": null,
                    "patchset": patchset,
                    "protocol_id": job.protocol_id.clone(),
                    "source_job_id": job.job_id.to_string(),
                    "source_trace_id": job.trace_id.to_string(),
                    "source_model_id": model_id.clone(),
                });

                json!({
                    "schema_version": "hsk.atelier.role_suggestions@v1",
                    "doc_id": doc_id,
                    "selection": selection,
                    "by_role": [
                        {
                            "role_id": role_id,
                            "suggestions": [suggestion]
                        }
                    ]
                })
            } else {
                json!({ "summary": response.text })
            };

            state
                .storage
                .set_job_outputs(&job.job_id.to_string(), Some(output.clone()))
                .await?;

            return Ok(RunJobOutcome {
                state: JobState::Completed,
                status_reason: "completed".to_string(),
                output: Some(output),
                error_message: None,
            });
        }
    } else if matches!(job.job_kind, JobKind::TerminalExec) {
        let payload = execute_terminal_job(state, job, trace_id).await?;
        state
            .storage
            .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
            .await?;
        return Ok(RunJobOutcome {
            state: JobState::Completed,
            status_reason: "completed".to_string(),
            output: Some(payload),
            error_message: None,
        });
    } else if matches!(job.job_kind, JobKind::LocusOperation) {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let op = locus::sqlite_store::parse_locus_operation(&job.protocol_id, &inputs)?;
        let op_for_event = op.clone();

        let payload = match op {
            locus::LocusOperation::SyncTaskBoard(params) => {
                execute_locus_sync_task_board(
                    state,
                    state.storage.as_ref(),
                    job,
                    workflow_run_id,
                    trace_id,
                    params,
                )
                .await?
            }
            other => {
                crate::storage::locus_sqlite::execute_locus_operation(state.storage.as_ref(), other)
                    .await?
            }
        };

        if !matches!(op_for_event, locus::LocusOperation::SyncTaskBoard(_)) {
            emit_locus_operation_event(
                state,
                job,
                workflow_run_id,
                trace_id,
                &op_for_event,
                &payload,
            )
            .await?;
        }
        state
            .storage
            .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
            .await?;
        return Ok(RunJobOutcome {
            state: JobState::Completed,
            status_reason: "completed".to_string(),
            output: Some(payload),
            error_message: None,
        });
    } else if matches!(job.job_kind, JobKind::MicroTaskExecution) {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let wp_id = inputs
            .get("wp_id")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("unknown")
            .to_string();

        let result = run_micro_task_executor_v1(state, job, workflow_run_id, trace_id).await;
        match &result {
            Ok(outcome) if matches!(outcome.state, JobState::Failed) => {
                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskLoopFailed,
                    "FR-EVT-MT-010",
                    "micro_task_loop_failed",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "wp_id": wp_id,
                        "status_reason": outcome.status_reason,
                        "error_message": outcome.error_message,
                    }),
                )
                .await;
            }
            Err(err) => {
                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskLoopFailed,
                    "FR-EVT-MT-010",
                    "micro_task_loop_failed",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({ "wp_id": wp_id, "error": err.to_string() }),
                )
                .await;
            }
            _ => {}
        }

        return result;
    } else if matches!(job.job_kind, JobKind::WorkflowRun) {
        if job.profile_id == "micro_task_executor_v1" {
            return Ok(RunJobOutcome {
                state: JobState::Poisoned,
                status_reason: "invalid_job_contract".to_string(),
                output: None,
                error_message: Some(
                    "invalid job contract: legacy micro_task_executor_v1 jobs must use job_kind micro_task_execution (migration required)"
                        .to_string(),
                ),
            });
        }

        if job.protocol_id == GOVERNANCE_PACK_EXPORT_PROTOCOL_ID {
            let inputs = parse_inputs(job.job_inputs.as_ref());
            let request: GovernancePackExportRequest = serde_json::from_value(inputs)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

            let outcome = export_governance_pack(&request, Some(job.job_id))
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

            let export_record_value = serde_json::to_value(&outcome.export_record)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::GovernancePackExport,
                    FlightRecorderActor::Agent,
                    trace_id,
                    export_record_value.clone(),
                )
                .with_job_id(job.job_id.to_string())
                .with_capability("export.governance_pack"),
            )
            .await;

            let payload = json!({
                "export_id": outcome.export_record.export_id,
                "templates_written": outcome.templates_written,
                "materialized_paths": outcome.export_record.materialized_paths,
                "export_record": export_record_value,
            });

            state
                .storage
                .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
                .await?;
            return Ok(RunJobOutcome {
                state: JobState::Completed,
                status_reason: "completed".to_string(),
                output: Some(payload),
                error_message: None,
            });
        }

        if job.profile_id == "capability_registry_build" {
            let inputs = parse_inputs(job.job_inputs.as_ref());
            let policy_decision_id = inputs
                .get("policy_decision_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| WorkflowError::Terminal("policy_decision_id is required".into()))?;
            let model_id = inputs
                .get("model_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "llama3".to_string());
            let approve = inputs
                .get("approve")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let reviewer_id = inputs
                .get("reviewer_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let repo_root = repo_root_from_manifest_dir()
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let params = CapabilityRegistryWorkflowParams {
                trace_id,
                policy_decision_id: policy_decision_id.to_string(),
                model_id,
                reviewer_id,
                approve,
                job_id: Some(job.job_id),
                workflow_id: Some(workflow_run_id),
            };

            let artifacts = run_capability_registry_workflow(
                &repo_root,
                state.capability_registry.as_ref(),
                state.flight_recorder.as_ref(),
                params,
            )
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

            let payload = json!({
                "profile_id": job.profile_id,
                "draft_path": artifacts.draft_path.to_string_lossy(),
                "diff_path": artifacts.diff_path.to_string_lossy(),
                "review_path": artifacts.review_path.to_string_lossy(),
                "published_path": artifacts.published_path.to_string_lossy(),
                "draft_sha256": artifacts.draft_sha256,
                "diff_sha256": artifacts.diff_sha256,
                "capability_registry_version": artifacts.capability_registry_version,
            });
            state
                .storage
                .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
                .await?;
            return Ok(RunJobOutcome {
                state: JobState::Completed,
                status_reason: "completed".to_string(),
                output: Some(payload),
                error_message: None,
            });
        }
    } else if matches!(job.job_kind, JobKind::DebugBundleExport) {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let scope_value = inputs.get("scope").cloned().unwrap_or_else(|| {
            json!({
                "kind": "job",
                "job_id": job.job_id.to_string()
            })
        });

        let scope = match scope_value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("job")
        {
            "problem" => scope_value
                .get("problem_id")
                .and_then(|v| v.as_str())
                .map(|id| BundleScope::Problem {
                    diagnostic_id: id.to_string(),
                })
                .ok_or_else(|| {
                    WorkflowError::Terminal("scope.problem_id missing for bundle export".into())
                })?,
            "time_window" => {
                let start = scope_value
                    .get("time_range")
                    .and_then(|r| r.get("start"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::Terminal("time_range.start missing".into()))?;
                let end = scope_value
                    .get("time_range")
                    .and_then(|r| r.get("end"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkflowError::Terminal("time_range.end missing".into()))?;
                let start_dt = chrono::DateTime::parse_from_rfc3339(start)
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                let end_dt = chrono::DateTime::parse_from_rfc3339(end)
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                BundleScope::TimeWindow {
                    start: start_dt,
                    end: end_dt,
                    wsid: scope_value
                        .get("wsid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                }
            }
            "workspace" => scope_value
                .get("wsid")
                .and_then(|v| v.as_str())
                .map(|wsid| BundleScope::Workspace {
                    wsid: wsid.to_string(),
                })
                .ok_or_else(|| {
                    WorkflowError::Terminal("wsid missing for workspace scope".into())
                })?,
            _ => scope_value
                .get("job_id")
                .and_then(|v| v.as_str())
                .map(|jid| BundleScope::Job {
                    job_id: jid.to_string(),
                })
                .unwrap_or(BundleScope::Job {
                    job_id: job.job_id.to_string(),
                }),
        };

        let redaction_mode = inputs
            .get("redaction_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("SAFE_DEFAULT");
        let redaction_mode = match redaction_mode {
            "WORKSPACE" | "workspace" => RedactionMode::Workspace,
            "FULL_LOCAL" | "full_local" => RedactionMode::FullLocal,
            _ => RedactionMode::SafeDefault,
        };

        let exporter = DefaultDebugBundleExporter::new(state.clone());
        if redaction_mode != RedactionMode::SafeDefault {
            let capability_id = "export.include_payloads";
            let result = state
                .capability_registry
                .profile_can(&job.capability_profile_id, capability_id);
            match result {
                Ok(true) => {
                    log_capability_check(state, job, capability_id, "allow", trace_id).await;
                }
                Ok(false) => {
                    log_capability_check(state, job, capability_id, "deny", trace_id).await;
                    return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                        capability_id.to_string(),
                    )));
                }
                Err(err) => {
                    log_capability_check(state, job, capability_id, "deny", trace_id).await;
                    return Err(WorkflowError::Capability(err));
                }
            }
        }

        let manifest = exporter
            .export_for_job(
                DebugBundleRequest {
                    scope,
                    redaction_mode,
                    output_path: None,
                    include_artifacts: inputs
                        .get("include_artifacts")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                },
                job.job_id,
                workflow_run_id,
                trace_id,
            )
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

        let manifest_value =
            serde_json::to_value(&manifest).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        state
            .storage
            .set_job_outputs(&job.job_id.to_string(), Some(manifest_value.clone()))
            .await?;
        return Ok(RunJobOutcome {
            state: JobState::Completed,
            status_reason: "completed".to_string(),
            output: Some(manifest_value),
            error_message: None,
        });
    }
    Ok(RunJobOutcome {
        state: JobState::Completed,
        status_reason: "completed".to_string(),
        output: None,
        error_message: None,
    })
}

// =============================================================================
// Micro-Task Executor v1 (Spec §2.6.6.8)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MicroTaskExecutorInputs {
    pub wp_id: String,
    pub wp_scope: WorkPacketScope,
    #[serde(default)]
    pub execution_policy: Option<ExecutionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkPacketScope {
    pub in_scope_paths: Vec<String>,
    #[serde(default)]
    pub out_of_scope: Vec<String>,
    pub done_means: Vec<String>,
    pub test_plan: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MicroTaskDefinition {
    pub mt_id: String,
    pub name: String,
    pub scope: String,
    pub files: FileAccessSpec,
    pub actions: Vec<String>,
    pub verify: Vec<VerificationSpec>,
    pub done: Vec<DoneCriterion>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub token_budget: u32,
    #[serde(default)]
    pub task_tags: Vec<String>,
    pub risk_level: RiskLevel,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileAccessSpec {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub modify: Vec<String>,
    #[serde(default)]
    pub create: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VerifyExpect {
    #[serde(rename = "exit_0")]
    Exit0,
    #[serde(rename = "exit_nonzero")]
    ExitNonzero,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "not_contains")]
    NotContains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationSpec {
    pub command: String,
    pub expect: VerifyExpect,
    #[serde(default)]
    pub pattern: Option<String>,
    pub timeout_ms: u64,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DoneVerification {
    Automated,
    EvidenceRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DoneCriterion {
    pub description: String,
    pub verification: DoneVerification,
    #[serde(default)]
    pub verify_ref: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RiskLevel {
    Low,
    Medium,
    High,
}

const EXEC_POLICY_EXT_SCHEMA_VERSION_V0_4: &str = "hsk.exec_policy_ext@0.4";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExecutionPolicyExtension {
    #[serde(rename = "model_swap_policy")]
    ModelSwapPolicy(ExecutionPolicyExtensionModelSwapPolicyV0_4),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionPolicyExtensionModelSwapPolicyV0_4 {
    pub schema_version: String,
    pub model_swap_policy: ModelSwapPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelSwapPolicy {
    #[serde(default = "default_allow_swaps")]
    pub allow_swaps: bool,
    #[serde(default = "default_max_swaps_per_job")]
    pub max_swaps_per_job: u32,
    #[serde(default = "default_swap_timeout_ms")]
    pub swap_timeout_ms: u64,
    #[serde(default)]
    pub fallback_strategy: ModelSwapFallbackStrategy,
}

impl Default for ModelSwapPolicy {
    fn default() -> Self {
        Self {
            allow_swaps: default_allow_swaps(),
            max_swaps_per_job: default_max_swaps_per_job(),
            swap_timeout_ms: default_swap_timeout_ms(),
            fallback_strategy: ModelSwapFallbackStrategy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContextPackPolicy {
    #[serde(default)]
    pub regen_allowed: bool,
    #[serde(default)]
    pub regen_required: bool,
    #[serde(default)]
    pub human_consent_obtained: bool,
}

impl Default for ContextPackPolicy {
    fn default() -> Self {
        Self {
            regen_allowed: false,
            regen_required: false,
            human_consent_obtained: false,
        }
    }
}

fn default_allow_swaps() -> bool {
    true
}

fn default_max_swaps_per_job() -> u32 {
    10
}

fn default_swap_timeout_ms() -> u64 {
    300_000
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ModelSwapFallbackStrategy {
    Abort,
    ContinueWithCurrent,
    EscalateToCloud,
}

impl Default for ModelSwapFallbackStrategy {
    fn default() -> Self {
        Self::Abort
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum AutomationLevel {
    FullHuman,
    #[serde(alias = "ASSISTED", alias = "SUPERVISED")]
    Hybrid,
    Autonomous,
    Locked,
}

impl AutomationLevel {
    fn is_locked(&self) -> bool {
        matches!(self, AutomationLevel::Locked)
    }

    fn requires_human_approval(&self) -> bool {
        matches!(self, AutomationLevel::FullHuman)
    }

    fn as_str(&self) -> &'static str {
        match self {
            AutomationLevel::FullHuman => "FULL_HUMAN",
            AutomationLevel::Hybrid => "HYBRID",
            AutomationLevel::Autonomous => "AUTONOMOUS",
            AutomationLevel::Locked => "LOCKED",
        }
    }
}

fn default_automation_level() -> AutomationLevel {
    AutomationLevel::Autonomous
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionPolicy {
    #[serde(default = "default_max_iterations_per_mt")]
    pub max_iterations_per_mt: u32,
    #[serde(default = "default_max_total_iterations")]
    pub max_total_iterations: u32,
    #[serde(default = "default_max_duration_ms")]
    pub max_duration_ms: u64,
    #[serde(default = "default_automation_level")]
    pub automation_level: AutomationLevel,
    #[serde(default = "default_escalation_chain")]
    pub escalation_chain: Vec<EscalationLevel>,
    #[serde(default)]
    pub cloud_escalation_allowed: bool,
    #[serde(default)]
    pub context_pack_policy: ContextPackPolicy,
    #[serde(default)]
    pub drop_back_strategy: DropBackStrategy,
    #[serde(default)]
    pub lora_selection: LoRASelectionStrategy,
    #[serde(default)]
    pub pause_points: Vec<String>,
    #[serde(default = "default_enable_distillation")]
    pub enable_distillation: bool,
    #[serde(default)]
    pub extensions: Vec<ExecutionPolicyExtension>,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            max_iterations_per_mt: default_max_iterations_per_mt(),
            max_total_iterations: default_max_total_iterations(),
            max_duration_ms: default_max_duration_ms(),
            automation_level: default_automation_level(),
            escalation_chain: default_escalation_chain(),
            cloud_escalation_allowed: false,
            context_pack_policy: ContextPackPolicy::default(),
            drop_back_strategy: DropBackStrategy::default(),
            lora_selection: LoRASelectionStrategy::default(),
            pause_points: Vec::new(),
            enable_distillation: default_enable_distillation(),
            extensions: Vec::new(),
        }
    }
}

impl ExecutionPolicy {
    fn model_swap_policy(&self) -> ModelSwapPolicy {
        self.extensions
            .iter()
            .rev()
            .find_map(|ext| match ext {
                ExecutionPolicyExtension::ModelSwapPolicy(ext)
                    if ext.schema_version == EXEC_POLICY_EXT_SCHEMA_VERSION_V0_4 =>
                {
                    Some(ext.model_swap_policy.clone())
                }
                ExecutionPolicyExtension::ModelSwapPolicy(_) => None,
                ExecutionPolicyExtension::Unknown => None,
            })
            .unwrap_or_default()
    }
}

const GOV_DECISION_SCHEMA_VERSION: &str = "hsk.gov_decision@0.4";
const GOV_AUTO_SIGNATURE_SCHEMA_VERSION: &str = "hsk.auto_signature@0.1";

const GOV_GATE_TYPE_MICRO_TASK_VALIDATION: &str = "MicroTaskValidation";
const GOV_GATE_TYPE_CLOUD_ESCALATION: &str = "CloudEscalation";
const GOV_GATE_TYPE_POLICY_VIOLATION: &str = "PolicyViolation";
const GOV_GATE_TYPE_HUMAN_INTERVENTION: &str = "HumanIntervention";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GovernanceDecisionOutcome {
    Approve,
    Reject,
    Defer,
}

impl GovernanceDecisionOutcome {
    fn as_str(&self) -> &'static str {
        match self {
            GovernanceDecisionOutcome::Approve => "approve",
            GovernanceDecisionOutcome::Reject => "reject",
            GovernanceDecisionOutcome::Defer => "defer",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GovernanceActorKind {
    Human,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceDecisionActor {
    pub kind: GovernanceActorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceDecision {
    pub schema_version: String,
    pub decision_id: String,
    pub gate_type: String,
    pub target_ref: String,
    pub decision: GovernanceDecisionOutcome,
    pub confidence: f64,
    pub rationale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<String>>,
    pub timestamp: String,
    pub actor: GovernanceDecisionActor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AutoSignatureActorKind {
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoSignatureActor {
    pub kind: AutoSignatureActorKind,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoSignature {
    pub schema_version: String,
    pub auto_signature_id: String,
    pub decision_id: String,
    pub gate_type: String,
    pub target_ref: String,
    pub created_at: String,
    pub actor: AutoSignatureActor,
}

fn default_max_iterations_per_mt() -> u32 {
    5
}

fn default_max_total_iterations() -> u32 {
    100
}

fn default_max_duration_ms() -> u64 {
    3_600_000
}

fn default_enable_distillation() -> bool {
    true
}

fn default_escalation_chain() -> Vec<EscalationLevel> {
    vec![
        EscalationLevel {
            level: 0,
            model_id: "qwen2.5-coder:7b".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::Auto),
            is_cloud: false,
            is_hard_gate: false,
        },
        EscalationLevel {
            level: 1,
            model_id: "qwen2.5-coder:7b".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::Alternate),
            is_cloud: false,
            is_hard_gate: false,
        },
        EscalationLevel {
            level: 2,
            model_id: "qwen2.5-coder:13b".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::Auto),
            is_cloud: false,
            is_hard_gate: false,
        },
        EscalationLevel {
            level: 3,
            model_id: "qwen2.5-coder:13b".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::Alternate),
            is_cloud: false,
            is_hard_gate: false,
        },
        EscalationLevel {
            level: 4,
            model_id: "qwen2.5-coder:32b".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::None),
            is_cloud: false,
            is_hard_gate: false,
        },
        EscalationLevel {
            level: 5,
            model_id: "HARD_GATE".to_string(),
            lora_id: None,
            lora_selector: Some(LoraSelector::None),
            is_cloud: false,
            is_hard_gate: true,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EscalationLevel {
    pub level: u32,
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
    #[serde(default)]
    pub lora_selector: Option<LoraSelector>,
    pub is_cloud: bool,
    pub is_hard_gate: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DropBackStrategy {
    Always,
    Never,
    Smart,
}

impl Default for DropBackStrategy {
    fn default() -> Self {
        Self::Smart
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LoRASelectionStrategy {
    AutoByTaskTags,
    Explicit,
    None,
}

impl Default for LoRASelectionStrategy {
    fn default() -> Self {
        Self::AutoByTaskTags
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LoraSelector {
    Auto,
    Alternate,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionSignal {
    pub claimed_complete: bool,
    #[serde(default)]
    pub evidence: Option<Vec<CompletionEvidence>>,
    pub blocked: bool,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub raw_block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionEvidence {
    pub criterion: String,
    pub evidence_location: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProgressStatus {
    InProgress,
    Completed,
    CompletedWithIssues,
    Failed,
    Cancelled,
    Paused,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MTStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum IterationOutcome {
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "RETRY")]
    Retry,
    #[serde(rename = "ESCALATE")]
    Escalate,
    #[serde(rename = "BLOCKED")]
    Blocked,
    #[serde(rename = "SKIPPED")]
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgressArtifact {
    pub schema_version: String,
    pub wp_id: String,
    pub job_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ProgressStatus,
    pub policy: ExecutionPolicy,
    pub learning_context: LearningContext,
    pub current_state: CurrentExecutionState,
    pub micro_tasks: Vec<MTProgressEntry>,
    pub aggregate_stats: AggregateStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningContext {
    pub skill_bank_snapshot_at_start: DateTime<Utc>,
    #[serde(default)]
    pub loras_available: Vec<LoRAInfo>,
    pub pending_distillation_jobs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoRAInfo {
    pub lora_id: String,
    #[serde(default)]
    pub task_type_tags: Vec<String>,
    #[serde(default)]
    pub lora_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingGovGate {
    pub decision_id: String,
    pub gate_type: String,
    pub target_ref: String,
    pub mt_id: String,
    pub final_iteration: u32,
    pub final_model_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingCloudEscalation {
    pub schema_version: String, // "hsk.pending_cloud_escalation@0.1"
    pub request_id: String,
    pub mt_id: String,
    pub requested_model_id: String,
    pub reason: String,
    pub local_attempts: u32,
    pub last_error_summary: String,
    pub projection_plan: ProjectionPlanV0_4,
    #[serde(default)]
    pub consent_receipt: Option<ConsentReceiptV0_4>,
    #[serde(default)]
    pub cloud_escalation_request: Option<CloudEscalationRequestV0_4>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CurrentExecutionState {
    #[serde(default)]
    pub active_mt: Option<String>,
    pub active_model_level: u32,
    pub total_iterations: u32,
    pub total_escalations: u32,
    #[serde(default)]
    pub total_model_swaps: u32,
    pub total_drop_backs: u32,
    #[serde(default)]
    pub pending_gov_gate: Option<PendingGovGate>,
    #[serde(default)]
    pub pending_cloud_escalation: Option<PendingCloudEscalation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MTProgressEntry {
    pub mt_id: String,
    pub name: String,
    pub status: MTStatus,
    #[serde(default)]
    pub iterations: Vec<IterationRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_iteration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_model_level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_record_ref: Option<ArtifactHandle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<ArtifactHandle>,
    #[serde(default)]
    pub evidence_refs: Vec<ArtifactHandle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distillation_candidate: Option<DistillationInfo>,
    #[serde(default)]
    pub pending_distillation_candidates: Vec<PendingDistillationCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DistillationInfo {
    pub eligible: bool,
    #[serde(default)]
    pub skill_log_entry_id: Option<String>,
    #[serde(default)]
    pub candidate_ref: Option<ArtifactHandle>,
    #[serde(default)]
    pub task_type_tags: Vec<String>,
    #[serde(default)]
    pub data_trust_score: Option<f64>,
    #[serde(default)]
    pub distillation_eligible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingDistillationCandidate {
    pub skill_log_entry_id: String,
    pub student_attempt: DistillationAttempt,
    #[serde(default)]
    pub task_type_tags: Vec<String>,
    #[serde(default)]
    pub contributing_factors: Vec<String>,
    pub data_trust_score: f64,
    pub distillation_eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DistillationAttempt {
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
    #[serde(default)]
    pub lora_version: Option<String>,
    pub prompt_snapshot_ref: ArtifactHandle,
    pub output_snapshot_ref: ArtifactHandle,
    pub outcome: String,
    pub iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtLayerScope {
    pub read: Vec<String>,
    pub write: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtIdsHashCount {
    pub ids_hash: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtPromptEnvelopeHashes {
    pub stable_prefix_hash: String,
    pub variable_suffix_hash: String,
    pub full_prompt_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtContextSnapshot {
    pub context_snapshot_id: Uuid,
    pub job_id: String,
    pub step_id: String,
    pub created_at: DateTime<Utc>,
    pub determinism_mode: String,
    pub model_tier: String,
    pub model_id: String,
    pub policy_profile_id: String,
    pub layer_scope: MtLayerScope,
    pub scope_inputs_hash: String,
    pub retrieval_candidates: MtIdsHashCount,
    pub selected_sources: MtIdsHashCount,
    pub prompt_envelope_hashes: MtPromptEnvelopeHashes,
    #[serde(default)]
    pub artifact_handles: Vec<ArtifactHandle>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_only_payload_ref: Option<ArtifactHandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtContextFilesArtifact {
    pub schema_version: String,
    pub mt_id: String,
    pub iteration: u32,
    pub files: Vec<MtContextFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MtContextFile {
    pub path: String,
    pub source_id: Uuid,
    pub source_hash: String,
    pub token_estimate: u32,
    pub truncated: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IterationRecord {
    pub iteration: u32,
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
    pub escalation_level: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub tokens_prompt: u32,
    pub tokens_completion: u32,
    pub claimed_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_passed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_evidence_ref: Option<ArtifactHandle>,
    pub outcome: IterationOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_summary: Option<String>,
    pub context_snapshot_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggregateStats {
    pub total_mts: u32,
    pub completed_mts: u32,
    pub failed_mts: u32,
    pub skipped_mts: u32,
    pub total_iterations: u32,
    pub total_tokens_prompt: u32,
    pub total_tokens_completion: u32,
    pub total_duration_ms: u64,
    pub escalation_count: u32,
    pub drop_back_count: u32,
    pub distillation_candidates_generated: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LedgerStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunLedger {
    pub ledger_id: Uuid,
    pub wp_id: String,
    pub job_id: String,
    pub created_at: DateTime<Utc>,
    pub steps: Vec<LedgerStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_point: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerStep {
    pub step_id: String,
    pub idempotency_key: String,
    pub status: LedgerStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_artifact_ref: Option<ArtifactHandle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MtValidationReport {
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub infos: Vec<String>,
}

impl MtValidationReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

fn repo_root_for_artifacts() -> Result<PathBuf, WorkflowError> {
    repo_root_from_manifest_dir().map_err(|e| WorkflowError::Terminal(e.to_string()))
}

fn micro_task_job_dir_rel(job_id: Uuid) -> PathBuf {
    PathBuf::from("data")
        .join("micro_task_executor")
        .join(job_id.to_string())
}

fn rel_path_string(rel_path: &Path) -> String {
    rel_path.to_string_lossy().replace('\\', "/")
}

fn artifact_handle_for_rel(rel_path: &Path) -> ArtifactHandle {
    ArtifactHandle::new(Uuid::new_v4(), rel_path_string(rel_path))
}

fn micro_task_target_ref(wp_id: &str, mt_id: &str) -> String {
    format!("wp/{wp_id}/mt/{mt_id}")
}

fn runtime_artifact_handle_for_abs_path(
    workspace_root: &Path,
    abs_path: &Path,
    seed: &str,
) -> Result<ArtifactHandle, WorkflowError> {
    let rel_path = abs_path.strip_prefix(workspace_root).map_err(|_| {
        WorkflowError::Terminal(format!(
            "runtime governance artifact must be under workspace root: {}",
            abs_path.display()
        ))
    })?;
    Ok(ArtifactHandle::new(
        deterministic_uuid_for_str(seed),
        rel_path_string(rel_path),
    ))
}

fn write_governance_decision_artifact(
    runtime_paths: &RuntimeGovernancePaths,
    decision: &GovernanceDecision,
) -> Result<ArtifactHandle, WorkflowError> {
    fs::create_dir_all(runtime_paths.governance_decisions_dir()).map_err(|e| {
        WorkflowError::Terminal(format!("failed to create governance decisions dir: {e}"))
    })?;

    let abs_path = runtime_paths.governance_decision_path(decision.decision_id.as_str());
    write_json_atomic(runtime_paths.workspace_root(), &abs_path, decision)?;

    runtime_artifact_handle_for_abs_path(
        runtime_paths.workspace_root(),
        &abs_path,
        &format!("gov_decision:{}", decision.decision_id),
    )
}

fn load_governance_decision_artifact(
    runtime_paths: &RuntimeGovernancePaths,
    decision_id: &str,
) -> Result<GovernanceDecision, WorkflowError> {
    let abs_path = runtime_paths.governance_decision_path(decision_id);
    let bytes = fs::read(&abs_path).map_err(|e| {
        WorkflowError::Terminal(format!(
            "failed to read governance decision {}: {e}",
            abs_path.display()
        ))
    })?;
    serde_json::from_slice::<GovernanceDecision>(&bytes).map_err(|e| {
        WorkflowError::Terminal(format!(
            "invalid governance decision JSON {}: {e}",
            abs_path.display()
        ))
    })
}

fn auto_signature_allowed_for_gate_type(gate_type: &str) -> bool {
    gate_type != GOV_GATE_TYPE_CLOUD_ESCALATION && gate_type != GOV_GATE_TYPE_POLICY_VIOLATION
}

fn enforce_auto_signature_binding(
    decision: &GovernanceDecision,
    auto_signature: &AutoSignature,
) -> Result<(), WorkflowError> {
    if auto_signature.decision_id != decision.decision_id
        || auto_signature.gate_type != decision.gate_type
        || auto_signature.target_ref != decision.target_ref
    {
        return Err(WorkflowError::Terminal(format!(
            "autosignature binding mismatch: decision_id={} gate_type={} target_ref={}",
            decision.decision_id, decision.gate_type, decision.target_ref
        )));
    }
    Ok(())
}

fn write_auto_signature_artifact(
    runtime_paths: &RuntimeGovernancePaths,
    auto_signature: &AutoSignature,
) -> Result<ArtifactHandle, WorkflowError> {
    fs::create_dir_all(runtime_paths.auto_signatures_dir()).map_err(|e| {
        WorkflowError::Terminal(format!("failed to create auto signatures dir: {e}"))
    })?;

    let abs_path = runtime_paths.auto_signature_path(auto_signature.auto_signature_id.as_str());
    write_json_atomic(runtime_paths.workspace_root(), &abs_path, auto_signature)?;

    runtime_artifact_handle_for_abs_path(
        runtime_paths.workspace_root(),
        &abs_path,
        &format!("auto_signature:{}", auto_signature.auto_signature_id),
    )
}

fn build_gov_automation_event_payload(
    event_type: &str,
    decision_id: &str,
    gate_type: &str,
    target_ref: &str,
    automation_level: AutomationLevel,
    decision: Option<GovernanceDecisionOutcome>,
    confidence: Option<f64>,
    evidence_refs: Option<Vec<String>>,
    wp_id: Option<&str>,
    mt_id: Option<&str>,
    user_id: Option<&str>,
) -> Value {
    let mut payload = json!({
        "type": event_type,
        "decision_id": decision_id,
        "gate_type": gate_type,
        "target_ref": target_ref,
        "automation_level": automation_level.as_str(),
    });

    if let Some(obj) = payload.as_object_mut() {
        if let Some(decision) = decision {
            obj.insert("decision".to_string(), json!(decision.as_str()));
        }
        if let Some(confidence) = confidence {
            obj.insert("confidence".to_string(), json!(confidence));
        }
        if let Some(evidence_refs) = evidence_refs {
            obj.insert("evidence_refs".to_string(), json!(evidence_refs));
        }
        if let Some(wp_id) = wp_id {
            obj.insert("wp_id".to_string(), json!(wp_id));
        }
        if let Some(mt_id) = mt_id {
            obj.insert("mt_id".to_string(), json!(mt_id));
        }
        if let Some(user_id) = user_id {
            obj.insert("user_id".to_string(), json!(user_id));
        }
    }

    payload
}

async fn record_gov_automation_event(
    state: &AppState,
    event_variant: FlightRecorderEventType,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    payload: Value,
) -> Result<(), WorkflowError> {
    let event =
        FlightRecorderEvent::new(event_variant, FlightRecorderActor::Agent, trace_id, payload)
            .with_job_id(job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string());
    record_event_required(state, event).await
}

async fn create_gov_decision_and_emit_created(
    state: &AppState,
    runtime_paths: &RuntimeGovernancePaths,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: Option<&str>,
    automation_level: AutomationLevel,
    gate_type: &str,
    target_ref: &str,
    outcome: GovernanceDecisionOutcome,
    confidence: f64,
    rationale: &str,
    evidence_refs: Option<Vec<String>>,
    actor_model_id: Option<String>,
    actor_user_id: Option<String>,
) -> Result<(GovernanceDecision, ArtifactHandle), WorkflowError> {
    let decision_id = Uuid::new_v4().to_string();

    let decision = GovernanceDecision {
        schema_version: GOV_DECISION_SCHEMA_VERSION.to_string(),
        decision_id: decision_id.clone(),
        gate_type: gate_type.to_string(),
        target_ref: target_ref.to_string(),
        decision: outcome,
        confidence,
        rationale: rationale.to_string(),
        evidence_refs,
        timestamp: Utc::now().to_rfc3339(),
        actor: GovernanceDecisionActor {
            kind: GovernanceActorKind::Model,
            model_id: actor_model_id,
            user_id: actor_user_id,
        },
    };

    let decision_artifact = write_governance_decision_artifact(runtime_paths, &decision)?;

    let created_payload = build_gov_automation_event_payload(
        "gov_decision_created",
        &decision.decision_id,
        &decision.gate_type,
        &decision.target_ref,
        automation_level,
        Some(decision.decision),
        Some(decision.confidence),
        Some(vec![decision_artifact.canonical_id()]),
        Some(wp_id),
        mt_id,
        None,
    );
    record_gov_automation_event(
        state,
        FlightRecorderEventType::GovDecisionCreated,
        trace_id,
        job_id,
        workflow_run_id,
        created_payload,
    )
    .await?;

    Ok((decision, decision_artifact))
}

async fn create_auto_signature_and_emit_created(
    state: &AppState,
    runtime_paths: &RuntimeGovernancePaths,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: Option<&str>,
    automation_level: AutomationLevel,
    decision: &GovernanceDecision,
    actor_model_id: &str,
) -> Result<(AutoSignature, ArtifactHandle), WorkflowError> {
    if !auto_signature_allowed_for_gate_type(decision.gate_type.as_str()) {
        return Err(WorkflowError::Terminal(format!(
            "autosignature forbidden for gate_type: {}",
            decision.gate_type
        )));
    }

    let auto_signature = AutoSignature {
        schema_version: GOV_AUTO_SIGNATURE_SCHEMA_VERSION.to_string(),
        auto_signature_id: Uuid::new_v4().to_string(),
        decision_id: decision.decision_id.clone(),
        gate_type: decision.gate_type.clone(),
        target_ref: decision.target_ref.clone(),
        created_at: Utc::now().to_rfc3339(),
        actor: AutoSignatureActor {
            kind: AutoSignatureActorKind::Model,
            model_id: actor_model_id.to_string(),
        },
    };

    enforce_auto_signature_binding(decision, &auto_signature)?;
    let auto_signature_artifact = write_auto_signature_artifact(runtime_paths, &auto_signature)?;

    let payload = build_gov_automation_event_payload(
        "gov_auto_signature_created",
        &auto_signature.decision_id,
        &auto_signature.gate_type,
        &auto_signature.target_ref,
        automation_level,
        None,
        None,
        Some(vec![auto_signature_artifact.canonical_id()]),
        Some(wp_id),
        mt_id,
        None,
    );
    record_gov_automation_event(
        state,
        FlightRecorderEventType::GovAutoSignatureCreated,
        trace_id,
        job_id,
        workflow_run_id,
        payload,
    )
    .await?;

    Ok((auto_signature, auto_signature_artifact))
}

async fn emit_gov_decision_applied(
    state: &AppState,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: Option<&str>,
    automation_level: AutomationLevel,
    decision: &GovernanceDecision,
    evidence_refs: Option<Vec<String>>,
) -> Result<(), WorkflowError> {
    let payload = build_gov_automation_event_payload(
        "gov_decision_applied",
        &decision.decision_id,
        &decision.gate_type,
        &decision.target_ref,
        automation_level,
        Some(decision.decision),
        Some(decision.confidence),
        evidence_refs,
        Some(wp_id),
        mt_id,
        None,
    );
    record_gov_automation_event(
        state,
        FlightRecorderEventType::GovDecisionApplied,
        trace_id,
        job_id,
        workflow_run_id,
        payload,
    )
    .await
}

async fn emit_gov_human_intervention_requested(
    state: &AppState,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: Option<&str>,
    automation_level: AutomationLevel,
    decision: &GovernanceDecision,
    evidence_refs: Option<Vec<String>>,
) -> Result<(), WorkflowError> {
    let payload = build_gov_automation_event_payload(
        "gov_human_intervention_requested",
        &decision.decision_id,
        &decision.gate_type,
        &decision.target_ref,
        automation_level,
        Some(decision.decision),
        Some(decision.confidence),
        evidence_refs,
        Some(wp_id),
        mt_id,
        None,
    );
    record_gov_automation_event(
        state,
        FlightRecorderEventType::GovHumanInterventionRequested,
        trace_id,
        job_id,
        workflow_run_id,
        payload,
    )
    .await
}

async fn emit_gov_human_intervention_received(
    state: &AppState,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    wp_id: &str,
    mt_id: Option<&str>,
    automation_level: AutomationLevel,
    decision: &GovernanceDecision,
    evidence_refs: Option<Vec<String>>,
) -> Result<(), WorkflowError> {
    let payload = build_gov_automation_event_payload(
        "gov_human_intervention_received",
        &decision.decision_id,
        &decision.gate_type,
        &decision.target_ref,
        automation_level,
        Some(decision.decision),
        Some(decision.confidence),
        evidence_refs,
        Some(wp_id),
        mt_id,
        None,
    );
    record_gov_automation_event(
        state,
        FlightRecorderEventType::GovHumanInterventionReceived,
        trace_id,
        job_id,
        workflow_run_id,
        payload,
    )
    .await
}

fn write_bytes_atomic(root: &Path, abs_path: &Path, bytes: &[u8]) -> Result<(), WorkflowError> {
    crate::storage::artifacts::write_file_atomic(root, abs_path, bytes, true)
        .map_err(|e| WorkflowError::Terminal(format!("atomic write failed: {e}")))
}

fn write_json_atomic<T: Serialize>(
    root: &Path,
    abs_path: &Path,
    value: &T,
) -> Result<(), WorkflowError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| WorkflowError::Terminal(format!("json serialize error: {e}")))?;
    write_bytes_atomic(root, abs_path, &bytes)
}

fn write_json_atomic_with_hash<T: Serialize>(
    root: &Path,
    abs_path: &Path,
    value: &T,
) -> Result<String, WorkflowError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| WorkflowError::Terminal(format!("json serialize error: {e}")))?;
    let hash = sha256_hex(&bytes);
    write_bytes_atomic(root, abs_path, &bytes)?;
    Ok(hash)
}

fn build_context_compile_payload_v1(
    request_id: &str,
    job_id: Uuid,
    wp_id: &str,
    mt_id: &str,
    created_at: &str,
    target_model_id: &str,
    context_snapshot_ref: &ArtifactHandle,
    context_snapshot_hash: &str,
    ace_query_plan_ref: &ArtifactHandle,
    ace_retrieval_trace_ref: &ArtifactHandle,
    context_pack_records: &[ContextPackRecord],
    retrieval_warnings: &[String],
) -> Value {
    let context_pack_refs: Vec<Value> = context_pack_records
        .iter()
        .map(|r| {
            json!({
                "pack_id": r.pack_id.to_string(),
                "pack_artifact_ref": r.pack_artifact.canonical_id(),
                "payload_hash": r.payload_hash.clone(),
                "target_source_id": r.target.source_id.to_string(),
                "target_source_hash": r.target.source_hash.clone(),
            })
        })
        .collect();

    let freshness_markers: Vec<String> = retrieval_warnings
        .iter()
        .filter(|w| {
            w.starts_with(STALE_PACK_WARNING_PREFIX)
                || w.starts_with(REGEN_SKIPPED_PREFIX)
                || w.starts_with("context_pack:")
        })
        .cloned()
        .collect();

    json!({
        "schema_version": "1.0",
        "request_id": request_id.to_string(),
        "job_id": job_id.to_string(),
        "wp_id": wp_id,
        "mt_id": mt_id,
        "created_at": created_at,
        "target_model_id": target_model_id,
        "context_snapshot_ref": context_snapshot_ref,
        "context_snapshot_hash": context_snapshot_hash,
        "ace_query_plan_ref": ace_query_plan_ref,
        "ace_retrieval_trace_ref": ace_retrieval_trace_ref,
        "context_packs": context_pack_refs,
        "freshness_markers": freshness_markers,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn sha256_hex_str(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn deterministic_uuid_for_str(value: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Set UUID variant + version bits (version 4-ish) for well-formed UUID strings.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn estimate_tokens(text: &str) -> u32 {
    // ~4 chars/token heuristic (matches ACE validators usage)
    (text.len().saturating_add(3) / 4) as u32
}

fn truncate_to_token_budget(text: &str, budget_tokens: u32) -> (String, bool) {
    let max_chars = (budget_tokens as usize).saturating_mul(4);
    if text.len() <= max_chars {
        return (text.to_string(), false);
    }
    let truncated: String = text.chars().take(max_chars).collect();
    (truncated, true)
}

#[derive(Debug, Clone, Copy)]
struct ShadowWsRegion {
    start_line: usize,
    end_line: usize,
    score: u32,
    center_line: usize,
}

fn build_mt_context_query_text(mt: &MicroTaskDefinition) -> String {
    let mut query_text = String::new();
    query_text.push_str(mt.scope.trim());
    query_text.push('\n');

    for action in &mt.actions {
        let trimmed = action.trim();
        if trimmed.is_empty() {
            continue;
        }
        query_text.push_str(trimmed);
        query_text.push('\n');
    }

    for criterion in &mt.done {
        let trimmed = criterion.description.trim();
        if trimmed.is_empty() {
            continue;
        }
        query_text.push_str(trimmed);
        query_text.push('\n');
    }

    query_text
}

fn build_mt_context_query_terms(query_text: &str) -> Vec<String> {
    let normalized = crate::ace::normalize_query(query_text);
    let mut terms: Vec<String> = normalized
        .split_ascii_whitespace()
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_string())
        .collect();
    terms.sort();
    terms.dedup();
    if terms.len() > 24 {
        terms.truncate(24);
    }
    terms
}

fn shadow_ws_lexical_regions(
    lines: &[&str],
    query_terms: &[String],
    max_results: usize,
    neighbor_lines: usize,
) -> Vec<ShadowWsRegion> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut scored_lines: Vec<(u32, usize)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let normalized_line = crate::ace::normalize_query(line);
        let mut score = 0u32;
        for term in query_terms {
            if normalized_line.contains(term) {
                score = score.saturating_add(1);
            }
        }
        if score > 0 {
            scored_lines.push((score, idx));
        }
    }

    let max_results = max_results.max(1);
    scored_lines.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut regions: Vec<ShadowWsRegion> = Vec::new();
    if scored_lines.is_empty() {
        let end_line = (neighbor_lines.saturating_mul(2)).min(lines.len().saturating_sub(1));
        regions.push(ShadowWsRegion {
            start_line: 0,
            end_line,
            score: 0,
            center_line: 0,
        });
    } else {
        for (score, center_line) in scored_lines.into_iter().take(max_results) {
            let start_line = center_line.saturating_sub(neighbor_lines);
            let end_line = center_line
                .saturating_add(neighbor_lines)
                .min(lines.len() - 1);
            regions.push(ShadowWsRegion {
                start_line,
                end_line,
                score,
                center_line,
            });
        }
    }

    regions.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| a.end_line.cmp(&b.end_line))
            .then_with(|| a.center_line.cmp(&b.center_line))
    });

    let mut merged: Vec<ShadowWsRegion> = Vec::new();
    for region in regions {
        match merged.last_mut() {
            Some(last) if region.start_line <= last.end_line.saturating_add(1) => {
                last.end_line = last.end_line.max(region.end_line);
                last.score = last.score.max(region.score);
            }
            _ => merged.push(region),
        }
    }
    merged
}

const CONTEXT_PACK_BUILDER_TOOL_ID: &str = "context_pack_builder_v0.1";
const CONTEXT_PACK_BUILDER_TOOL_VERSION: &str = "0.1";

#[derive(Debug, Clone, Serialize)]
struct ContextPackBuilderConfigV1 {
    chunk_lines: usize,
    max_anchors: usize,
}

fn default_context_pack_builder_config_v1() -> ContextPackBuilderConfigV1 {
    ContextPackBuilderConfigV1 {
        chunk_lines: 60,
        max_anchors: 24,
    }
}

fn context_pack_builder_config_hash_v1(config: &ContextPackBuilderConfigV1) -> String {
    let json = serde_json::to_string(config).unwrap_or_default();
    sha256_hex_str(&json)
}

fn context_pack_store_dir_rel(source_id: Uuid, builder_config_hash: &str) -> PathBuf {
    PathBuf::from("data")
        .join("context_packs")
        .join(source_id.to_string())
        .join(builder_config_hash)
}

fn context_pack_store_record_rel(source_id: Uuid, builder_config_hash: &str) -> PathBuf {
    context_pack_store_dir_rel(source_id, builder_config_hash).join("record.json")
}

fn context_pack_store_payload_rel(source_id: Uuid, builder_config_hash: &str) -> PathBuf {
    context_pack_store_dir_rel(source_id, builder_config_hash).join("payload.json")
}

fn context_pack_anchor_id(
    source_id: Uuid,
    start_line_1: usize,
    end_line_1: usize,
    idx: usize,
) -> String {
    format!("chunk:{source_id}:{start_line_1}-{end_line_1}:{idx}")
}

fn parse_context_pack_anchor_range(anchor_id: &str) -> Option<(usize, usize)> {
    let mut parts = anchor_id.split(':');
    if parts.next()? != "chunk" {
        return None;
    }
    let _source_id = parts.next()?;
    let range = parts.next()?;
    let mut range_parts = range.split('-');
    let start_line_1 = range_parts.next()?.parse::<usize>().ok()?;
    let end_line_1 = range_parts.next()?.parse::<usize>().ok()?;
    if start_line_1 == 0 || end_line_1 == 0 || end_line_1 < start_line_1 {
        return None;
    }
    Some((start_line_1 - 1, end_line_1 - 1))
}

fn build_context_pack_payload_v1_for_source(
    path: &str,
    source_ref: &SourceRef,
    bytes: &[u8],
    config: &ContextPackBuilderConfigV1,
) -> ContextPackPayloadV1 {
    let content = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = content.lines().collect();

    let mut anchors: Vec<ContextPackAnchorV1> = Vec::new();
    if !lines.is_empty() {
        let mut idx = 1usize;
        for start_line in (0..lines.len()).step_by(config.chunk_lines.max(1)) {
            if anchors.len() >= config.max_anchors {
                break;
            }
            let end_line = (start_line + config.chunk_lines.saturating_sub(1)).min(lines.len() - 1);
            anchors.push(ContextPackAnchorV1 {
                anchor_id: context_pack_anchor_id(
                    source_ref.source_id,
                    start_line + 1,
                    end_line + 1,
                    idx,
                ),
                source_ref: source_ref.clone(),
                excerpt_hint: format!("lines {}-{}", start_line + 1, end_line + 1),
            });
            idx = idx.saturating_add(1);
        }
    }

    let scanned_selectors: Vec<String> = anchors.iter().map(|a| a.anchor_id.clone()).collect();
    let synopsis = format!(
        "ContextPack for {path} (source_id={})",
        source_ref.source_id
    );
    let synopsis = synopsis.chars().take(800).collect::<String>();

    ContextPackPayloadV1 {
        synopsis,
        facts: Vec::new(),
        constraints: Vec::new(),
        open_loops: Vec::new(),
        anchors,
        coverage: ContextPackCoverageV1 {
            scanned_selectors,
            skipped_selectors: None,
        },
    }
}

#[derive(Debug, Clone)]
struct ContextPackSelection {
    record: ContextPackRecord,
    source_ref: SourceRef,
    file: MtContextFile,
    spans: Vec<SpanExtraction>,
    match_score: u32,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ContextPackFallback {
    source_ref: SourceRef,
    pack_id: Option<Uuid>,
    warnings: Vec<String>,
}

enum ContextPackOutcome {
    Selected(ContextPackSelection),
    Fallback(ContextPackFallback),
    Skipped { warning: String },
}

fn retrieve_context_pack_for_source(
    repo_root: &Path,
    path: &str,
    source_ref: SourceRef,
    bytes: &[u8],
    query_terms: &[String],
    max_results: usize,
    allowance_tokens: u32,
    max_span_tokens: u32,
    builder_config: &ContextPackBuilderConfigV1,
    builder_config_hash: &str,
    policy: &ContextPackFreshnessPolicyV1,
) -> Result<ContextPackOutcome, WorkflowError> {
    let record_rel = context_pack_store_record_rel(source_ref.source_id, builder_config_hash);
    let record_abs = repo_root.join(&record_rel);

    let load_record = if record_abs.exists() {
        match fs::read(&record_abs) {
            Ok(raw) => match serde_json::from_slice::<ContextPackRecord>(&raw) {
                Ok(record) => Some(record),
                Err(err) => {
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: None,
                        warnings: vec![format!("context_pack:record_parse_failed:{path}:{err}")],
                    }));
                }
            },
            Err(err) => {
                return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                    source_ref,
                    pack_id: None,
                    warnings: vec![format!("context_pack:record_read_failed:{path}:{err}")],
                }));
            }
        }
    } else {
        None
    };

    let mut record = match load_record {
        Some(record) => record,
        None if policy.regen_allowed => {
            let payload_rel =
                context_pack_store_payload_rel(source_ref.source_id, builder_config_hash);
            let mut payload =
                build_context_pack_payload_v1_for_source(path, &source_ref, bytes, builder_config);
            payload.enforce_provenance_binding();

            let payload_abs = repo_root.join(&payload_rel);
            if let Some(parent) = payload_abs.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: None,
                        warnings: vec![format!("context_pack:mkdir_failed:{path}:{err}")],
                    }));
                }
            }

            let payload_hash = match write_json_atomic_with_hash(repo_root, &payload_abs, &payload)
            {
                Ok(hash) => hash,
                Err(err) => {
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: None,
                        warnings: vec![format!("context_pack:payload_write_failed:{path}:{err}")],
                    }));
                }
            };

            let pack_id = deterministic_uuid_for_str(&format!(
                "context_pack:{}:{}:{}",
                source_ref.source_id, builder_config_hash, payload_hash
            ));

            let record = ContextPackRecord {
                pack_id,
                target: source_ref.clone(),
                pack_artifact: ArtifactHandle::new(pack_id, rel_path_string(&payload_rel)),
                source_hashes: vec![source_ref.source_hash.clone()],
                source_refs: vec![source_ref.clone()],
                created_at: Utc::now(),
                builder: ContextPackBuilder {
                    tool_id: CONTEXT_PACK_BUILDER_TOOL_ID.to_string(),
                    tool_version: CONTEXT_PACK_BUILDER_TOOL_VERSION.to_string(),
                    config_hash: builder_config_hash.to_string(),
                },
                payload_hash,
                version: 1,
            };

            if let Some(parent) = record_abs.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: Some(record.pack_id),
                        warnings: vec![format!("context_pack:mkdir_failed:{path}:{err}")],
                    }));
                }
            }
            if let Err(err) = write_json_atomic(repo_root, &record_abs, &record) {
                return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                    source_ref,
                    pack_id: Some(record.pack_id),
                    warnings: vec![format!("context_pack:record_write_failed:{path}:{err}")],
                }));
            }

            record
        }
        None => {
            return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                source_ref,
                pack_id: None,
                warnings: Vec::new(),
            }));
        }
    };

    // Load payload (hash verified)
    let payload_rel = PathBuf::from(&record.pack_artifact.path);
    let payload_abs = repo_root.join(&payload_rel);
    let raw_payload = match fs::read(&payload_abs) {
        Ok(raw) => raw,
        Err(err) => {
            return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                source_ref,
                pack_id: Some(record.pack_id),
                warnings: vec![format!("context_pack:payload_read_failed:{path}:{err}")],
            }));
        }
    };

    let payload_hash = sha256_hex(&raw_payload);
    if payload_hash != record.payload_hash {
        return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
            source_ref,
            pack_id: Some(record.pack_id),
            warnings: vec![format!("context_pack:payload_hash_mismatch:{path}")],
        }));
    }

    let mut payload = match serde_json::from_slice::<ContextPackPayloadV1>(&raw_payload) {
        Ok(payload) => payload,
        Err(err) => {
            return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                source_ref,
                pack_id: Some(record.pack_id),
                warnings: vec![format!("context_pack:payload_parse_failed:{path}:{err}")],
            }));
        }
    };

    // Staleness check (regen or fallback)
    if record.is_stale(&[source_ref.clone()]) {
        if policy.regen_allowed {
            let old_pack_id = record.pack_id;
            let payload_rel =
                context_pack_store_payload_rel(source_ref.source_id, builder_config_hash);
            let mut new_payload =
                build_context_pack_payload_v1_for_source(path, &source_ref, bytes, builder_config);
            new_payload.enforce_provenance_binding();

            let payload_abs = repo_root.join(&payload_rel);
            if let Some(parent) = payload_abs.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    let mut warnings: Vec<String> =
                        vec![format!("{}{}", STALE_PACK_WARNING_PREFIX, old_pack_id)];
                    if policy.regen_required {
                        warnings.push(format!("{}{}", REGEN_SKIPPED_PREFIX, old_pack_id));
                    }
                    warnings.push(format!("context_pack:mkdir_failed:{path}:{err}"));
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: Some(old_pack_id),
                        warnings,
                    }));
                }
            }

            let new_payload_hash =
                match write_json_atomic_with_hash(repo_root, &payload_abs, &new_payload) {
                    Ok(hash) => hash,
                    Err(err) => {
                        let mut warnings: Vec<String> =
                            vec![format!("{}{}", STALE_PACK_WARNING_PREFIX, old_pack_id)];
                        if policy.regen_required {
                            warnings.push(format!("{}{}", REGEN_SKIPPED_PREFIX, old_pack_id));
                        }
                        warnings.push(format!("context_pack:payload_write_failed:{path}:{err}"));
                        return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                            source_ref,
                            pack_id: Some(old_pack_id),
                            warnings,
                        }));
                    }
                };

            let new_pack_id = deterministic_uuid_for_str(&format!(
                "context_pack:{}:{}:{}",
                source_ref.source_id, builder_config_hash, new_payload_hash
            ));

            record = ContextPackRecord {
                pack_id: new_pack_id,
                target: source_ref.clone(),
                pack_artifact: ArtifactHandle::new(new_pack_id, rel_path_string(&payload_rel)),
                source_hashes: vec![source_ref.source_hash.clone()],
                source_refs: vec![source_ref.clone()],
                created_at: Utc::now(),
                builder: ContextPackBuilder {
                    tool_id: CONTEXT_PACK_BUILDER_TOOL_ID.to_string(),
                    tool_version: CONTEXT_PACK_BUILDER_TOOL_VERSION.to_string(),
                    config_hash: builder_config_hash.to_string(),
                },
                payload_hash: new_payload_hash,
                version: 1,
            };
            if let Some(parent) = record_abs.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    let mut warnings: Vec<String> =
                        vec![format!("{}{}", STALE_PACK_WARNING_PREFIX, old_pack_id)];
                    if policy.regen_required {
                        warnings.push(format!("{}{}", REGEN_SKIPPED_PREFIX, old_pack_id));
                    }
                    warnings.push(format!("context_pack:mkdir_failed:{path}:{err}"));
                    return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                        source_ref,
                        pack_id: Some(old_pack_id),
                        warnings,
                    }));
                }
            }
            if let Err(err) = write_json_atomic(repo_root, &record_abs, &record) {
                let mut warnings: Vec<String> =
                    vec![format!("{}{}", STALE_PACK_WARNING_PREFIX, old_pack_id)];
                if policy.regen_required {
                    warnings.push(format!("{}{}", REGEN_SKIPPED_PREFIX, old_pack_id));
                }
                warnings.push(format!("context_pack:record_write_failed:{path}:{err}"));
                return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                    source_ref,
                    pack_id: Some(old_pack_id),
                    warnings,
                }));
            }

            payload = new_payload;
            // Note: we do not emit stale_pack marker when regeneration succeeds.
            // Caller may choose to record a regeneration marker if desired.
        } else {
            let mut warnings: Vec<String> =
                vec![format!("{}{}", STALE_PACK_WARNING_PREFIX, record.pack_id)];
            if policy.regen_required {
                warnings.push(format!("{}{}", REGEN_SKIPPED_PREFIX, record.pack_id));
            }
            return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
                source_ref,
                pack_id: Some(record.pack_id),
                warnings,
            }));
        }
    }

    payload.enforce_provenance_binding();

    let content = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(ContextPackOutcome::Skipped {
            warning: format!("empty_file:{path}"),
        });
    }

    let mut scored: Vec<(u32, usize, usize, String)> = Vec::new();
    for anchor in &payload.anchors {
        if let Some((start, end)) = parse_context_pack_anchor_range(&anchor.anchor_id) {
            let end = end.min(lines.len().saturating_sub(1));
            let start = start.min(end);
            let mut score = 0u32;
            for line in &lines[start..=end] {
                let normalized = crate::ace::normalize_query(line);
                for term in query_terms {
                    if normalized.contains(term) {
                        score = score.saturating_add(1);
                    }
                }
            }
            scored.push((score, start, end, anchor.anchor_id.clone()));
        }
    }

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });

    if scored.is_empty() {
        return Ok(ContextPackOutcome::Fallback(ContextPackFallback {
            source_ref,
            pack_id: Some(record.pack_id),
            warnings: vec![format!("context_pack:no_anchors:{path}")],
        }));
    }

    let mut warnings: Vec<String> = Vec::new();
    if scored[0].0 == 0 {
        warnings.push(format!("context_pack:no_match:{path}"));
    }

    let mut line_offsets: Vec<u32> = Vec::with_capacity(lines.len() + 1);
    let mut acc = 0u32;
    for line in &lines {
        line_offsets.push(acc);
        acc = acc.saturating_add(line.chars().count() as u32);
        acc = acc.saturating_add(1);
    }
    line_offsets.push(acc);

    let mut spans: Vec<SpanExtraction> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    let mut tokens_used = 0u32;
    let mut match_score = 0u32;
    let mut truncated_any = false;

    for (idx, (score, start_line, end_line, anchor_id)) in
        scored.into_iter().take(max_results).enumerate()
    {
        if tokens_used >= allowance_tokens {
            break;
        }
        let remaining = allowance_tokens.saturating_sub(tokens_used);
        if remaining == 0 {
            break;
        }

        let per_span_budget = remaining.min(max_span_tokens).max(1);
        let raw_chunk = lines[start_line..=end_line].join("\n");
        let (chunk, truncated) = truncate_to_token_budget(&raw_chunk, per_span_budget);
        let chunk_tokens = estimate_tokens(&chunk);
        if chunk_tokens == 0 {
            continue;
        }

        tokens_used = tokens_used.saturating_add(chunk_tokens);
        match_score = match_score.saturating_add(score);
        truncated_any |= truncated;
        if truncated {
            warnings.push(format!(
                "context_pack:truncated:{}:{}-{}",
                path,
                start_line + 1,
                end_line + 1
            ));
        }

        let selector = format!("context_pack:{}:{}", anchor_id, idx + 1);
        let start = line_offsets.get(start_line).copied().unwrap_or_default();
        let end = line_offsets
            .get(end_line.saturating_add(1))
            .copied()
            .unwrap_or(acc);

        spans.push(SpanExtraction {
            source_ref: source_ref.clone(),
            selector,
            start,
            end,
            token_estimate: chunk_tokens,
        });

        parts.push(format!(
            "/* pack chunk {} lines {}-{} */\n{}",
            idx + 1,
            start_line + 1,
            end_line + 1,
            chunk
        ));
    }

    let snippet = parts.join("\n\n");
    let file = MtContextFile {
        path: path.to_string(),
        source_id: source_ref.source_id,
        source_hash: source_ref.source_hash.clone(),
        token_estimate: tokens_used,
        truncated: truncated_any,
        content: snippet,
    };

    Ok(ContextPackOutcome::Selected(ContextPackSelection {
        record,
        source_ref,
        file,
        spans,
        match_score,
        warnings,
    }))
}

enum ShadowWsLexicalOutcome {
    Selected(ShadowWsLexicalSelection),
    Skipped { warning: String },
}

struct ShadowWsLexicalSelection {
    source_ref: SourceRef,
    file: MtContextFile,
    spans: Vec<SpanExtraction>,
    match_score: u32,
    warnings: Vec<String>,
}

fn retrieve_shadow_ws_lexical_for_file(
    path: &str,
    source_ref: SourceRef,
    bytes: &[u8],
    query_terms: &[String],
    max_results: usize,
    neighbor_lines: usize,
    allowance_tokens: u32,
    max_span_tokens: u32,
) -> Result<ShadowWsLexicalOutcome, WorkflowError> {
    let content = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(ShadowWsLexicalOutcome::Skipped {
            warning: format!("empty_file:{path}"),
        });
    }

    let regions = shadow_ws_lexical_regions(&lines, query_terms, max_results, neighbor_lines);
    if regions.is_empty() {
        return Ok(ShadowWsLexicalOutcome::Skipped {
            warning: format!("shadow_ws_lexical:no_regions:{path}"),
        });
    }

    let mut line_offsets: Vec<u32> = Vec::with_capacity(lines.len() + 1);
    let mut acc = 0u32;
    for line in &lines {
        line_offsets.push(acc);
        acc = acc.saturating_add(line.chars().count() as u32);
        acc = acc.saturating_add(1);
    }
    line_offsets.push(acc);

    let mut warnings: Vec<String> = Vec::new();
    if regions.len() == 1 && regions[0].score == 0 {
        warnings.push(format!("shadow_ws_lexical:no_match:{path}"));
    }

    let mut spans: Vec<SpanExtraction> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    let mut tokens_used = 0u32;
    let mut match_score = 0u32;
    let mut truncated_any = false;

    for (idx, region) in regions.iter().enumerate() {
        if tokens_used >= allowance_tokens {
            break;
        }

        let remaining = allowance_tokens.saturating_sub(tokens_used);
        if remaining == 0 {
            break;
        }

        let per_span_budget = remaining.min(max_span_tokens).max(1);
        let raw_chunk = lines[region.start_line..=region.end_line].join("\n");
        let (chunk, truncated) = truncate_to_token_budget(&raw_chunk, per_span_budget);
        let chunk_tokens = estimate_tokens(&chunk);
        if chunk_tokens == 0 {
            continue;
        }

        tokens_used = tokens_used.saturating_add(chunk_tokens);
        match_score = match_score.saturating_add(region.score);
        truncated_any |= truncated;
        if truncated {
            warnings.push(format!(
                "shadow_ws_lexical:truncated:{}:{}-{}",
                path,
                region.start_line + 1,
                region.end_line + 1
            ));
        }

        let selector = format!(
            "shadow_ws_lexical:{}:{}-{}:{}",
            path,
            region.start_line + 1,
            region.end_line + 1,
            idx + 1
        );

        let start = line_offsets
            .get(region.start_line)
            .copied()
            .unwrap_or_default();
        let end = line_offsets
            .get(region.end_line.saturating_add(1))
            .copied()
            .unwrap_or(acc);

        spans.push(SpanExtraction {
            source_ref: source_ref.clone(),
            selector,
            start,
            end,
            token_estimate: chunk_tokens,
        });

        parts.push(format!(
            "/* chunk {} lines {}-{} */\n{}",
            idx + 1,
            region.start_line + 1,
            region.end_line + 1,
            chunk
        ));
    }

    let snippet = parts.join("\n\n");
    let file = MtContextFile {
        path: path.to_string(),
        source_id: source_ref.source_id,
        source_hash: source_ref.source_hash.clone(),
        token_estimate: tokens_used,
        truncated: truncated_any,
        content: snippet,
    };

    Ok(ShadowWsLexicalOutcome::Selected(ShadowWsLexicalSelection {
        source_ref,
        file,
        spans,
        match_score,
        warnings,
    }))
}

#[cfg(test)]
mod context_pack_tests {
    use super::*;

    #[test]
    fn context_pack_stale_fallback_and_regen_policy() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let repo_root = tmp.path();

        let path = "src/example.txt";
        let bytes_v1 = b"hello world\nfn test() {}\n".to_vec();

        let source_id = deterministic_uuid_for_str(path);
        let source_ref_v1 = SourceRef::new(source_id, sha256_hex(&bytes_v1));

        let query_terms = vec!["hello".to_string()];
        let builder_config = default_context_pack_builder_config_v1();
        let builder_config_hash = context_pack_builder_config_hash_v1(&builder_config);

        // Build on miss when regen is allowed
        let allow = ContextPackFreshnessPolicyV1 {
            regen_allowed: true,
            regen_required: false,
        };
        let built = retrieve_context_pack_for_source(
            repo_root,
            path,
            source_ref_v1.clone(),
            &bytes_v1,
            &query_terms,
            3,
            200,
            100,
            &builder_config,
            &builder_config_hash,
            &allow,
        )?;
        let built_record = match built {
            ContextPackOutcome::Selected(sel) => sel.record,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "expected Selected on regen_allowed miss",
                )
                .into())
            }
        };

        // Stale fallback when regen is not allowed
        let bytes_v2 = b"hello changed\nfn test() {}\n".to_vec();
        let source_ref_v2 = SourceRef::new(source_id, sha256_hex(&bytes_v2));
        let deny = ContextPackFreshnessPolicyV1 {
            regen_allowed: false,
            regen_required: false,
        };
        let stale = retrieve_context_pack_for_source(
            repo_root,
            path,
            source_ref_v2.clone(),
            &bytes_v2,
            &query_terms,
            3,
            200,
            100,
            &builder_config,
            &builder_config_hash,
            &deny,
        )?;
        match stale {
            ContextPackOutcome::Fallback(fb) => {
                assert!(fb
                    .warnings
                    .iter()
                    .any(|w| w.starts_with(STALE_PACK_WARNING_PREFIX)));
                assert_eq!(fb.pack_id, Some(built_record.pack_id));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "expected Fallback on stale with regen_allowed=false",
                )
                .into())
            }
        }

        // Stale + regen_required should emit regen_skipped marker
        let require = ContextPackFreshnessPolicyV1 {
            regen_allowed: false,
            regen_required: true,
        };
        let stale_required = retrieve_context_pack_for_source(
            repo_root,
            path,
            source_ref_v2.clone(),
            &bytes_v2,
            &query_terms,
            3,
            200,
            100,
            &builder_config,
            &builder_config_hash,
            &require,
        )?;
        match stale_required {
            ContextPackOutcome::Fallback(fb) => {
                assert!(fb
                    .warnings
                    .iter()
                    .any(|w| w.starts_with(REGEN_SKIPPED_PREFIX)));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "expected Fallback on stale with regen_required",
                )
                .into())
            }
        }

        // Stale regen when allowed should return Selected and update target hash
        let regenerated = retrieve_context_pack_for_source(
            repo_root,
            path,
            source_ref_v2.clone(),
            &bytes_v2,
            &query_terms,
            3,
            200,
            100,
            &builder_config,
            &builder_config_hash,
            &allow,
        )?;
        match regenerated {
            ContextPackOutcome::Selected(sel) => {
                assert_eq!(sel.record.target.source_hash, source_ref_v2.source_hash);
                assert!(!sel.record.is_stale(&[source_ref_v2]));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "expected Selected on stale with regen_allowed=true",
                )
                .into())
            }
        }

        Ok(())
    }

    #[test]
    fn context_compile_payload_includes_pack_refs_and_markers() {
        let pack_id = Uuid::new_v4();
        let source_ref = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        let record = ContextPackRecord {
            pack_id,
            target: source_ref.clone(),
            pack_artifact: ArtifactHandle::new(
                pack_id,
                "data/context_packs/payload.json".to_string(),
            ),
            source_hashes: vec![source_ref.source_hash.clone()],
            source_refs: vec![source_ref.clone()],
            created_at: Utc::now(),
            builder: ContextPackBuilder {
                tool_id: CONTEXT_PACK_BUILDER_TOOL_ID.to_string(),
                tool_version: CONTEXT_PACK_BUILDER_TOOL_VERSION.to_string(),
                config_hash: "cfg".to_string(),
            },
            payload_hash: "payload_hash".to_string(),
            version: 1,
        };

        let snapshot_ref = ArtifactHandle::new(Uuid::new_v4(), "data/snap.json".to_string());
        let qp_ref = ArtifactHandle::new(Uuid::new_v4(), "data/qp.json".to_string());
        let rt_ref = ArtifactHandle::new(Uuid::new_v4(), "data/rt.json".to_string());

        let stale_marker = format!("{}{}", STALE_PACK_WARNING_PREFIX, pack_id);
        let denied_marker = "context_pack:regen_denied:capability".to_string();

        let payload = build_context_compile_payload_v1(
            "req-1",
            Uuid::nil(),
            "WP-1",
            "MT-1",
            "2026-02-14T00:00:00Z",
            "model-1",
            &snapshot_ref,
            "snap_hash",
            &qp_ref,
            &rt_ref,
            &[record],
            &[stale_marker.clone(), denied_marker.clone()],
        );

        assert_eq!(
            payload.get("request_id").and_then(Value::as_str),
            Some("req-1")
        );
        assert_eq!(
            payload.get("context_snapshot_hash").and_then(Value::as_str),
            Some("snap_hash")
        );
        assert_eq!(
            payload
                .get("context_packs")
                .and_then(Value::as_array)
                .map(|a| a.len()),
            Some(1)
        );
        let markers = payload
            .get("freshness_markers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(markers
            .iter()
            .any(|v| v.as_str() == Some(stale_marker.as_str())));
        assert!(markers
            .iter()
            .any(|v| v.as_str() == Some(denied_marker.as_str())));
    }
}

fn idempotency_key(
    mt_id: &str,
    iteration: u32,
    model_id: &str,
    lora_id: Option<&str>,
    prompt_hash: &str,
) -> String {
    let key = format!(
        "mt_id={mt_id}\niteration={iteration}\nmodel_id={model_id}\nlora_id={}\nprompt_hash={prompt_hash}\n",
        lora_id.unwrap_or("")
    );
    sha256_hex_str(&key)
}

fn extract_tag_block(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_string())
}

fn parse_completion_signal(response: &str) -> CompletionSignal {
    let mt_complete = extract_tag_block(response, "mt_complete");
    let blocked = extract_tag_block(response, "blocked");

    let evidence = mt_complete
        .as_deref()
        .and_then(|block| parse_completion_evidence(block));
    let blocked_reason = blocked.as_ref().and_then(|block| {
        block
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix("REASON:"))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    });

    CompletionSignal {
        claimed_complete: mt_complete.is_some(),
        evidence,
        blocked: blocked.is_some(),
        blocked_reason: blocked_reason.or_else(|| blocked.clone().filter(|s| !s.trim().is_empty())),
        raw_block: mt_complete.or(blocked),
    }
}

fn parse_completion_evidence(raw_block: &str) -> Option<Vec<CompletionEvidence>> {
    fn split_once_any<'a>(s: &'a str, delims: &[&str]) -> Option<(&'a str, &'a str)> {
        for delim in delims {
            if let Some((left, right)) = s.split_once(delim) {
                return Some((left, right));
            }
        }
        None
    }

    let mut evidence = Vec::new();
    for line in raw_block.lines() {
        let line = line.trim();
        if !line.starts_with('-') {
            continue;
        }
        let rest = line.trim_start_matches('-').trim();
        if rest.is_empty() {
            continue;
        }

        let (criterion, after_criterion) = if rest.starts_with('"') {
            let remainder = &rest[1..];
            let end_quote = remainder.find('"')?;
            let crit = &remainder[..end_quote];
            let after = remainder[end_quote + 1..].trim();
            (crit.trim(), after)
        } else {
            let (crit, after) = split_once_any(rest, &["→", "->", "=>"])?;
            (crit.trim(), after.trim())
        };

        let (_maybe_label, loc) =
            split_once_any(after_criterion, &["→", "->", "=>"]).unwrap_or(("", after_criterion));
        let loc = loc.trim();
        if criterion.is_empty() || loc.is_empty() {
            continue;
        }

        evidence.push(CompletionEvidence {
            criterion: criterion.to_string(),
            evidence_location: loc.to_string(),
        });
    }

    if evidence.is_empty() {
        None
    } else {
        Some(evidence)
    }
}

fn parse_first_shell_token(command: &str) -> Option<String> {
    let mut iter = command.trim_start().chars().peekable();
    let Some(_) = iter.peek() else {
        return None;
    };

    let mut token = String::new();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = iter.next() {
        if !in_single && !in_double && ch.is_whitespace() {
            break;
        }

        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if in_double => {
                if let Some(next) = iter.next() {
                    token.push(next);
                }
            }
            _ => token.push(ch),
        }
    }

    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn proc_exec_capability_for_command(command: &str) -> String {
    let Some(first) = parse_first_shell_token(command) else {
        return "proc.exec".to_string();
    };
    if first.is_empty() {
        return "proc.exec".to_string();
    }
    let tool = first
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first.as_str())
        .trim_end_matches(".exe");
    format!("proc.exec:{tool}")
}

fn previous_output_summary_for_mt(
    repo_root: &Path,
    run_ledger: &RunLedger,
    mt_id: &str,
    budget_tokens: u32,
) -> Option<String> {
    let prefix = format!("{mt_id}_");
    let step = run_ledger.steps.iter().rev().find(|s| {
        s.step_id.starts_with(&prefix)
            && matches!(s.status, LedgerStepStatus::Completed)
            && s.output_artifact_ref.is_some()
    })?;
    let output_artifact_ref = step.output_artifact_ref.as_ref()?;

    let output_abs = repo_root.join(PathBuf::from(&output_artifact_ref.path));
    let raw = fs::read_to_string(output_abs).ok()?;
    let val: Value = serde_json::from_str(&raw).ok()?;

    let output_snapshot_path = val
        .get("output_snapshot_ref")
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())?;
    let response_abs = repo_root.join(PathBuf::from(output_snapshot_path));
    let response_text = fs::read_to_string(response_abs).ok()?;

    let (summary, _truncated) = truncate_to_token_budget(&response_text, budget_tokens);
    Some(summary)
}

fn generate_mt_definitions_from_scope(scope: &WorkPacketScope) -> Vec<MicroTaskDefinition> {
    let mut read = scope.in_scope_paths.clone();
    read.truncate(10);
    let mut modify = scope.in_scope_paths.clone();
    modify.truncate(5);

    let mut verify: Vec<VerificationSpec> = scope
        .test_plan
        .iter()
        .filter(|s| !s.trim().is_empty())
        .take(3)
        .map(|cmd| VerificationSpec {
            command: cmd.trim().to_string(),
            expect: VerifyExpect::Exit0,
            pattern: None,
            timeout_ms: 180_000,
            blocking: true,
        })
        .collect();
    if verify.is_empty() {
        verify.push(VerificationSpec {
            command: "cargo test --manifest-path src/backend/handshake_core/Cargo.toml".to_string(),
            expect: VerifyExpect::Exit0,
            pattern: None,
            timeout_ms: 300_000,
            blocking: true,
        });
    }

    let mut done: Vec<DoneCriterion> = Vec::new();
    done.push(DoneCriterion {
        description: "All validation commands succeed".to_string(),
        verification: DoneVerification::Automated,
        verify_ref: Some(0),
    });
    for item in scope
        .done_means
        .iter()
        .filter(|s| !s.trim().is_empty())
        .take(7)
    {
        done.push(DoneCriterion {
            description: item.trim().chars().take(200).collect(),
            verification: DoneVerification::EvidenceRequired,
            verify_ref: None,
        });
    }
    if done.is_empty() {
        done.push(DoneCriterion {
            description: "Work Packet requirements are satisfied".to_string(),
            verification: DoneVerification::EvidenceRequired,
            verify_ref: None,
        });
    }

    let mut actions: Vec<String> = vec![
        scope.description.trim().chars().take(200).collect(),
        "Implement required changes within scope".to_string(),
        "Run validation and collect evidence".to_string(),
    ];
    actions.retain(|s| !s.trim().is_empty());

    let task_tags = vec!["rust".to_string(), "handshake".to_string()];

    vec![MicroTaskDefinition {
        mt_id: "MT-001".to_string(),
        name: "Execute Work Packet".to_string(),
        scope: scope.description.trim().chars().take(500).collect(),
        files: FileAccessSpec {
            read,
            modify,
            create: Vec::new(),
        },
        actions,
        verify,
        done,
        depends_on: Vec::new(),
        token_budget: 2048,
        task_tags,
        risk_level: RiskLevel::High,
        notes: None,
    }]
}

fn validate_mt_definitions(
    defs: &[MicroTaskDefinition],
    scope: &WorkPacketScope,
    model_max_context_tokens: u32,
) -> MtValidationReport {
    let mut report = MtValidationReport::default();

    let mut ids = std::collections::HashSet::new();
    for mt in defs {
        // MT-VAL-001/002
        if !ids.insert(mt.mt_id.clone()) {
            report
                .errors
                .push(format!("MT-VAL-001 duplicate mt_id: {}", mt.mt_id));
        }
        let ok_pattern = mt.mt_id.len() == 6
            && mt.mt_id.starts_with("MT-")
            && mt.mt_id[3..].chars().all(|c| c.is_ascii_digit());
        if !ok_pattern {
            report
                .errors
                .push(format!("MT-VAL-002 invalid mt_id format: {}", mt.mt_id));
        }

        // Schema constraints
        if mt.name.trim().is_empty() || mt.name.len() > 100 {
            report
                .errors
                .push(format!("schema violation: name length for {}", mt.mt_id));
        }
        if mt.scope.trim().is_empty() || mt.scope.len() > 500 {
            report
                .errors
                .push(format!("schema violation: scope length for {}", mt.mt_id));
        }
        if mt.actions.is_empty() || mt.actions.len() > 10 {
            report
                .errors
                .push(format!("schema violation: actions count for {}", mt.mt_id));
        }
        if mt
            .actions
            .iter()
            .any(|a| a.trim().is_empty() || a.len() > 200)
        {
            report.errors.push(format!(
                "schema violation: actions item length for {}",
                mt.mt_id
            ));
        }
        if mt.verify.is_empty() || mt.verify.len() > 5 {
            report
                .errors
                .push(format!("schema violation: verify count for {}", mt.mt_id));
        }
        if mt.done.is_empty() || mt.done.len() > 8 {
            report
                .errors
                .push(format!("schema violation: done count for {}", mt.mt_id));
        }
        if mt.files.read.len() > 10 || mt.files.modify.len() > 5 || mt.files.create.len() > 3 {
            report
                .errors
                .push(format!("schema violation: files.* limits for {}", mt.mt_id));
        }
        if mt.token_budget < 512 || mt.token_budget > 8192 {
            report.errors.push(format!(
                "schema violation: token_budget range for {}",
                mt.mt_id
            ));
        }

        // MT-VAL-003
        for dep in &mt.depends_on {
            if !defs.iter().any(|d| d.mt_id == *dep) {
                report.errors.push(format!(
                    "MT-VAL-003 depends_on unresolved: {} -> {}",
                    mt.mt_id, dep
                ));
            }
        }

        // MT-VAL-005
        for path in mt.files.modify.iter().chain(mt.files.create.iter()) {
            if !scope.in_scope_paths.iter().any(|p| p == path) {
                report.errors.push(format!(
                    "MT-VAL-005 path out of scope: {} -> {}",
                    mt.mt_id, path
                ));
            }
        }

        // MT-VAL-006
        let max_allowed = model_max_context_tokens.saturating_sub(512);
        if mt.token_budget > max_allowed {
            report.errors.push(format!(
                "MT-VAL-006 token_budget {} exceeds max_allowed {} for {}",
                mt.token_budget, max_allowed, mt.mt_id
            ));
        }

        // MT-VAL-007
        if !mt.verify.iter().any(|v| v.blocking) {
            report
                .warnings
                .push(format!("MT-VAL-007 no blocking verify for {}", mt.mt_id));
        }

        // MT-VAL-008
        for (idx, criterion) in mt.done.iter().enumerate() {
            if matches!(criterion.verification, DoneVerification::Automated) {
                let ok = criterion
                    .verify_ref
                    .and_then(|v| mt.verify.get(v))
                    .is_some();
                if !ok {
                    report.warnings.push(format!(
                        "MT-VAL-008 done[{idx}] missing verify_ref for {}",
                        mt.mt_id
                    ));
                }
            }
        }
    }

    // MT-VAL-004 (acyclic)
    let mut visiting = std::collections::HashSet::new();
    let mut visited = std::collections::HashSet::new();
    let mut stack = Vec::new();
    fn dfs(
        mt_id: &str,
        defs: &[MicroTaskDefinition],
        visiting: &mut std::collections::HashSet<String>,
        visited: &mut std::collections::HashSet<String>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if visited.contains(mt_id) {
            return None;
        }
        if visiting.contains(mt_id) {
            let start = stack.iter().position(|s| s == mt_id).unwrap_or(0);
            return Some(stack[start..].to_vec());
        }
        visiting.insert(mt_id.to_string());
        stack.push(mt_id.to_string());
        if let Some(mt) = defs.iter().find(|d| d.mt_id == mt_id) {
            for dep in &mt.depends_on {
                if let Some(cycle) = dfs(dep, defs, visiting, visited, stack) {
                    return Some(cycle);
                }
            }
        }
        stack.pop();
        visiting.remove(mt_id);
        visited.insert(mt_id.to_string());
        None
    }

    for mt in defs {
        if let Some(cycle) = dfs(&mt.mt_id, defs, &mut visiting, &mut visited, &mut stack) {
            report.errors.push(format!(
                "MT-VAL-004 dependency cycle: {}",
                cycle.join(" -> ")
            ));
            break;
        }
    }

    report
}

fn compute_execution_waves(
    defs: &[MicroTaskDefinition],
) -> Result<Vec<Vec<String>>, WorkflowError> {
    let mut indegree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut edges: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for mt in defs {
        indegree.entry(mt.mt_id.clone()).or_insert(0);
        for dep in &mt.depends_on {
            edges.entry(dep.clone()).or_default().push(mt.mt_id.clone());
            *indegree.entry(mt.mt_id.clone()).or_insert(0) += 1;
        }
    }

    let mut waves: Vec<Vec<String>> = Vec::new();
    let mut remaining = indegree.clone();
    loop {
        let mut ready: Vec<String> = remaining
            .iter()
            .filter_map(|(k, v)| if *v == 0 { Some(k.clone()) } else { None })
            .collect();
        if ready.is_empty() {
            break;
        }
        ready.sort();
        for mt_id in &ready {
            remaining.remove(mt_id);
            if let Some(nexts) = edges.get(mt_id) {
                for nxt in nexts {
                    if let Some(v) = remaining.get_mut(nxt) {
                        *v = v.saturating_sub(1);
                    }
                }
            }
        }
        waves.push(ready);
    }

    if !remaining.is_empty() {
        return Err(WorkflowError::Terminal(
            "MT-VAL-004 dependency graph has a cycle".to_string(),
        ));
    }

    Ok(waves)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifySpecResult {
    pub spec_index: u32,
    pub command: String,
    pub expected: String,
    pub actual_exit_code: i64,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_ref: Option<ArtifactHandle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_ref: Option<ArtifactHandle>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationResult {
    pub passed: bool,
    pub spec_results: Vec<VerifySpecResult>,
    pub evidence_artifact_ref: ArtifactHandle,
    pub started_at: String,
    pub completed_at: String,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationEvidenceArtifact {
    pub schema_version: String,
    pub evidence: Vec<ArtifactHandle>,
}

fn build_mex_runtime(state: &AppState, repo_root: &Path) -> Result<MexRuntime, WorkflowError> {
    let registry_path = repo_root.join("src/backend/handshake_core/mechanical_engines.json");
    let registry = MexRegistry::load_from_path(&registry_path)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new((*state.capability_registry).clone())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    Ok(MexRuntime::new(
        registry,
        state.flight_recorder.clone(),
        state.diagnostics.clone(),
        gates,
    )
    .with_adapter(
        "engine.shell",
        Arc::new(ShellEngineAdapter::new(
            repo_root.to_path_buf(),
            state.capability_registry.clone(),
            state.flight_recorder.clone(),
        )),
    ))
}

async fn run_validation_via_mex(
    mex_runtime: &MexRuntime,
    repo_root: &Path,
    verify: &[VerificationSpec],
    capability_profile_id: &str,
    evidence_artifact_rel: &Path,
    evidence_artifact_ref: ArtifactHandle,
) -> Result<ValidationResult, WorkflowError> {
    let started_at = Utc::now().to_rfc3339();
    // WAIVER [CX-573E]: Instant::now() for observability (validation duration metrics).
    let start = std::time::Instant::now();

    let mut all_evidence: Vec<ArtifactHandle> = Vec::new();
    let mut spec_results: Vec<VerifySpecResult> = Vec::new();
    let mut overall_passed = true;

    for (spec_index, spec) in verify.iter().enumerate() {
        let capability = proc_exec_capability_for_command(&spec.command);
        let max_bytes = 10_485_760u64;
        let wall_time_ms = spec.timeout_ms;

        let op = PlannedOperation {
            schema_version: POE_SCHEMA_VERSION.to_string(),
            op_id: Uuid::new_v4(),
            engine_id: "engine.shell".to_string(),
            engine_version_req: None,
            operation: "exec".to_string(),
            inputs: Vec::new(),
            params: json!({
                "command": spec.command.clone(),
                "cwd": ".",
                "timeout_ms": wall_time_ms,
                "env": {},
            }),
            capabilities_requested: vec![capability],
            capability_profile_id: Some(capability_profile_id.to_string()),
            human_consent_obtained: false,
            budget: BudgetSpec {
                cpu_time_ms: None,
                wall_time_ms: Some(wall_time_ms),
                memory_bytes: None,
                output_bytes: Some(max_bytes),
            },
            determinism: DeterminismLevel::D1,
            evidence_policy: Some(EvidencePolicy {
                required: true,
                notes: Some("capture_stdout_stderr".to_string()),
            }),
            output_spec: OutputSpec {
                expected_types: vec!["artifact.terminal_output".to_string()],
                max_bytes: Some(max_bytes),
            },
        };

        let result = mex_runtime
            .execute(op)
            .await
            .map_err(|e| WorkflowError::Terminal(format!("MEX execute failed: {e}")))?;

        let expected = match spec.expect {
            VerifyExpect::Exit0 => "exit_0".to_string(),
            VerifyExpect::ExitNonzero => "exit_nonzero".to_string(),
            VerifyExpect::Contains => match spec.pattern.as_deref() {
                Some(pattern) => format!("contains:{pattern}"),
                None => "contains:<missing_pattern>".to_string(),
            },
            VerifyExpect::NotContains => match spec.pattern.as_deref() {
                Some(pattern) => format!("not_contains:{pattern}"),
                None => "not_contains:<missing_pattern>".to_string(),
            },
        };

        let mut passed = false;
        let mut actual_exit_code = -1i64;
        let mut duration_ms = 0u64;
        let mut stdout_ref: Option<ArtifactHandle> = None;
        let mut stderr_ref: Option<ArtifactHandle> = None;
        let mut error: Option<String> = None;

        if let Some(output_handle) = result.outputs.first() {
            let output_abs = repo_root.join(PathBuf::from(&output_handle.path));
            match fs::read_to_string(&output_abs) {
                Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                    Ok(val) => {
                        actual_exit_code =
                            val.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
                        let timed_out = val
                            .get("timed_out")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        duration_ms = val.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

                        let stdout_rel = val
                            .get("stdout_ref")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let stderr_rel = val
                            .get("stderr_ref")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if let Some(stdout_rel) = stdout_rel.as_deref() {
                            stdout_ref = result
                                .evidence
                                .iter()
                                .find(|h| h.path == stdout_rel)
                                .cloned()
                                .or_else(|| Some(artifact_handle_for_rel(Path::new(stdout_rel))));
                        }
                        if let Some(stderr_rel) = stderr_rel.as_deref() {
                            stderr_ref = result
                                .evidence
                                .iter()
                                .find(|h| h.path == stderr_rel)
                                .cloned()
                                .or_else(|| Some(artifact_handle_for_rel(Path::new(stderr_rel))));
                        }

                        match spec.expect {
                            VerifyExpect::Exit0 => {
                                passed = actual_exit_code == 0 && !timed_out;
                            }
                            VerifyExpect::ExitNonzero => {
                                passed = actual_exit_code != 0 || timed_out;
                            }
                            VerifyExpect::Contains => {
                                if let Some(pattern) = spec.pattern.as_deref() {
                                    if let Some(stdout_rel) = stdout_rel.as_deref() {
                                        let stdout_abs = repo_root.join(PathBuf::from(stdout_rel));
                                        match fs::read_to_string(stdout_abs) {
                                            Ok(stdout) => {
                                                passed = stdout.contains(pattern);
                                            }
                                            Err(e) => {
                                                error.get_or_insert_with(|| {
                                                    format!(
                                                        "failed to read stdout ({stdout_rel}): {e}"
                                                    )
                                                });
                                            }
                                        }
                                    } else {
                                        error.get_or_insert_with(|| {
                                            "missing stdout_ref for contains verification"
                                                .to_string()
                                        });
                                    }
                                } else {
                                    error.get_or_insert_with(|| {
                                        "missing pattern for contains verification".to_string()
                                    });
                                }
                            }
                            VerifyExpect::NotContains => {
                                if let Some(pattern) = spec.pattern.as_deref() {
                                    if let Some(stdout_rel) = stdout_rel.as_deref() {
                                        let stdout_abs = repo_root.join(PathBuf::from(stdout_rel));
                                        match fs::read_to_string(stdout_abs) {
                                            Ok(stdout) => {
                                                passed = !stdout.contains(pattern);
                                            }
                                            Err(e) => {
                                                error.get_or_insert_with(|| {
                                                    format!(
                                                        "failed to read stdout ({stdout_rel}): {e}"
                                                    )
                                                });
                                            }
                                        }
                                    } else {
                                        error.get_or_insert_with(|| {
                                            "missing stdout_ref for not_contains verification"
                                                .to_string()
                                        });
                                    }
                                } else {
                                    error.get_or_insert_with(|| {
                                        "missing pattern for not_contains verification".to_string()
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error = Some(format!("invalid terminal output JSON: {e}"));
                    }
                },
                Err(e) => {
                    error = Some(format!("failed to read terminal output artifact: {e}"));
                }
            }
        } else {
            error = Some("missing terminal output artifact".to_string());
        }

        if !passed && spec.blocking {
            overall_passed = false;
        }

        all_evidence.extend(result.evidence.iter().cloned());

        spec_results.push(VerifySpecResult {
            spec_index: spec_index as u32,
            command: spec.command.clone(),
            expected,
            actual_exit_code,
            passed,
            stdout_ref,
            stderr_ref,
            duration_ms,
            error,
        });
    }

    all_evidence.sort_by(|a, b| a.canonical_id().cmp(&b.canonical_id()));
    all_evidence.dedup_by(|a, b| a.canonical_id() == b.canonical_id());

    let evidence_artifact = ValidationEvidenceArtifact {
        schema_version: "1.0".to_string(),
        evidence: all_evidence,
    };
    write_json_atomic(
        repo_root,
        &repo_root.join(evidence_artifact_rel),
        &evidence_artifact,
    )?;

    Ok(ValidationResult {
        passed: overall_passed,
        spec_results,
        evidence_artifact_ref,
        started_at,
        completed_at: Utc::now().to_rfc3339(),
        total_duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn record_micro_task_event(
    state: &AppState,
    event_variant: FlightRecorderEventType,
    event_type: &str,
    event_name: &str,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    payload: Value,
) {
    let full_payload = json!({
        "event_type": event_type,
        "event_name": event_name,
        "timestamp": Utc::now().to_rfc3339(),
        "trace_id": trace_id.to_string(),
        "job_id": job_id.to_string(),
        "workflow_run_id": workflow_run_id.to_string(),
        "payload": payload,
    });

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            event_variant,
            FlightRecorderActor::Agent,
            trace_id,
            full_payload,
        )
        .with_job_id(job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;
}

fn init_progress_artifact(
    wp_id: &str,
    job_id: Uuid,
    policy: ExecutionPolicy,
    defs: &[MicroTaskDefinition],
) -> ProgressArtifact {
    let now = Utc::now();
    let micro_tasks: Vec<MTProgressEntry> = defs
        .iter()
        .map(|mt| MTProgressEntry {
            mt_id: mt.mt_id.clone(),
            name: mt.name.clone(),
            status: MTStatus::Pending,
            iterations: Vec::new(),
            final_iteration: None,
            final_model_level: None,
            escalation_record_ref: None,
            summary_ref: None,
            evidence_refs: Vec::new(),
            distillation_candidate: None,
            pending_distillation_candidates: Vec::new(),
        })
        .collect();

    let mut progress = ProgressArtifact {
        schema_version: "1.0".to_string(),
        wp_id: wp_id.to_string(),
        job_id: job_id.to_string(),
        created_at: now,
        updated_at: now,
        completed_at: None,
        status: ProgressStatus::InProgress,
        policy,
        learning_context: LearningContext {
            skill_bank_snapshot_at_start: now,
            loras_available: Vec::new(),
            pending_distillation_jobs: 0,
        },
        current_state: CurrentExecutionState {
            active_mt: None,
            active_model_level: 0,
            total_iterations: 0,
            total_escalations: 0,
            total_model_swaps: 0,
            total_drop_backs: 0,
            pending_gov_gate: None,
            pending_cloud_escalation: None,
        },
        micro_tasks,
        aggregate_stats: AggregateStats {
            total_mts: defs.len() as u32,
            completed_mts: 0,
            failed_mts: 0,
            skipped_mts: 0,
            total_iterations: 0,
            total_tokens_prompt: 0,
            total_tokens_completion: 0,
            total_duration_ms: 0,
            escalation_count: 0,
            drop_back_count: 0,
            distillation_candidates_generated: 0,
        },
    };

    refresh_aggregate_stats(&mut progress);
    progress
}

fn init_run_ledger(wp_id: &str, job_id: Uuid) -> RunLedger {
    RunLedger {
        ledger_id: Uuid::new_v4(),
        wp_id: wp_id.to_string(),
        job_id: job_id.to_string(),
        created_at: Utc::now(),
        steps: Vec::new(),
        resume_point: None,
        resume_reason: None,
    }
}

fn refresh_aggregate_stats(progress: &mut ProgressArtifact) {
    let mut completed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut duration_ms = 0u64;
    let mut distillation_candidates = 0u32;

    for mt in &progress.micro_tasks {
        match mt.status {
            MTStatus::Completed => completed += 1,
            MTStatus::Failed | MTStatus::Blocked => failed += 1,
            MTStatus::Skipped => skipped += 1,
            _ => {}
        }
        if mt
            .distillation_candidate
            .as_ref()
            .map(|d| d.eligible)
            .unwrap_or(false)
        {
            distillation_candidates = distillation_candidates.saturating_add(1);
        }
        for it in &mt.iterations {
            prompt_tokens = prompt_tokens.saturating_add(it.tokens_prompt);
            completion_tokens = completion_tokens.saturating_add(it.tokens_completion);
            duration_ms = duration_ms.saturating_add(it.duration_ms);
        }
    }

    progress.aggregate_stats.total_mts = progress.micro_tasks.len() as u32;
    progress.aggregate_stats.completed_mts = completed;
    progress.aggregate_stats.failed_mts = failed;
    progress.aggregate_stats.skipped_mts = skipped;
    progress.aggregate_stats.total_iterations = progress.current_state.total_iterations;
    progress.aggregate_stats.total_tokens_prompt = prompt_tokens;
    progress.aggregate_stats.total_tokens_completion = completion_tokens;
    progress.aggregate_stats.total_duration_ms = duration_ms;
    progress.aggregate_stats.escalation_count = progress.current_state.total_escalations;
    progress.aggregate_stats.drop_back_count = progress.current_state.total_drop_backs;
    progress.aggregate_stats.distillation_candidates_generated = distillation_candidates;
}

async fn run_micro_task_executor_v1(
    state: &AppState,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
) -> Result<RunJobOutcome, WorkflowError> {
    let raw_inputs = parse_inputs(job.job_inputs.as_ref());
    if raw_inputs.get("mt_definitions").is_some() || raw_inputs.get("mt_definitions_ref").is_some()
    {
        return Err(WorkflowError::Terminal(
            "mt_definitions must not be provided; MT definitions are auto-generated from WP scope"
                .to_string(),
        ));
    }

    let inputs: MicroTaskExecutorInputs = serde_json::from_value(raw_inputs).map_err(|e| {
        WorkflowError::Terminal(format!("invalid micro_task_executor_v1 inputs: {e}"))
    })?;
    let mut policy = inputs.execution_policy.unwrap_or_default();
    let automation_level = policy.automation_level;
    if automation_level.is_locked() {
        // Spec Â§11.1.7.3 + Â§2.6.8.12.6.1: cloud escalation must be denied in LOCKED.
        policy.cloud_escalation_allowed = false;
    }
    let cloud_policy = CloudEscalationPolicy::from_env();
    let cloud_governance_locked = cloud_policy.governance_mode == RuntimeGovernanceMode::Locked;

    if job.state == JobState::Cancelled {
        record_micro_task_event(
            state,
            FlightRecorderEventType::MicroTaskLoopCancelled,
            "FR-EVT-MT-011",
            "micro_task_loop_cancelled",
            trace_id,
            job.job_id,
            workflow_run_id,
            json!({ "wp_id": inputs.wp_id }),
        )
        .await;

        return Ok(RunJobOutcome {
            state: JobState::Cancelled,
            status_reason: "cancelled".to_string(),
            output: Some(json!({ "wp_id": inputs.wp_id })),
            error_message: None,
        });
    }

    let mt_definitions = generate_mt_definitions_from_scope(&inputs.wp_scope);
    let mt_validation = validate_mt_definitions(
        &mt_definitions,
        &inputs.wp_scope,
        state.llm_client.profile().max_context_tokens,
    );
    if mt_validation.has_errors() {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "mt_definitions_invalid".to_string(),
            output: Some(json!({
                "wp_id": inputs.wp_id,
                "validation": mt_validation,
            })),
            error_message: Some("MicroTaskDefinition validation failed".to_string()),
        });
    }

    let execution_waves = compute_execution_waves(&mt_definitions)?;

    let repo_root = repo_root_for_artifacts()?;
    let runtime_paths = RuntimeGovernancePaths::resolve().map_err(|e| {
        WorkflowError::Terminal(format!("failed to resolve runtime governance paths: {e}"))
    })?;
    let job_dir_rel = micro_task_job_dir_rel(job.job_id);
    let job_dir_abs = repo_root.join(&job_dir_rel);
    fs::create_dir_all(&job_dir_abs).map_err(|e| {
        WorkflowError::Terminal(format!("failed to create {}: {e}", job_dir_abs.display()))
    })?;

    let mt_defs_rel = job_dir_rel.join("mt_definitions.json");
    let mt_defs_abs = repo_root.join(&mt_defs_rel);
    write_json_atomic(&repo_root, &mt_defs_abs, &mt_definitions)?;
    let mt_definitions_ref = artifact_handle_for_rel(&mt_defs_rel);

    let progress_rel = job_dir_rel.join("progress_artifact.json");
    let progress_abs = repo_root.join(&progress_rel);
    let run_ledger_rel = job_dir_rel.join("run_ledger.json");
    let run_ledger_abs = repo_root.join(&run_ledger_rel);

    let mut resumed_from_pause_mt: Option<String> = None;
    let (mut progress, mut run_ledger, loaded_existing_state) = if progress_abs.exists()
        && run_ledger_abs.exists()
    {
        let progress_bytes = fs::read(&progress_abs).map_err(|e| {
            WorkflowError::Terminal(format!("failed to read {}: {e}", progress_abs.display()))
        })?;
        let progress: ProgressArtifact = serde_json::from_slice(&progress_bytes)
            .map_err(|e| WorkflowError::Terminal(format!("invalid progress_artifact.json: {e}")))?;

        let run_ledger_bytes = fs::read(&run_ledger_abs).map_err(|e| {
            WorkflowError::Terminal(format!("failed to read {}: {e}", run_ledger_abs.display()))
        })?;
        let run_ledger: RunLedger = serde_json::from_slice(&run_ledger_bytes)
            .map_err(|e| WorkflowError::Terminal(format!("invalid run_ledger.json: {e}")))?;

        (progress, run_ledger, true)
    } else {
        let progress =
            init_progress_artifact(&inputs.wp_id, job.job_id, policy.clone(), &mt_definitions);
        let run_ledger = init_run_ledger(&inputs.wp_id, job.job_id);
        write_json_atomic(&repo_root, &progress_abs, &progress)?;
        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;
        (progress, run_ledger, false)
    };

    if loaded_existing_state && progress.status == ProgressStatus::Completed {
        return Ok(RunJobOutcome {
            state: JobState::Completed,
            status_reason: "completed".to_string(),
            output: Some(json!({
                "wp_id": inputs.wp_id,
                "mt_definitions_ref": mt_definitions_ref,
                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
            })),
            error_message: None,
        });
    }

    if loaded_existing_state && progress.status == ProgressStatus::Paused {
        resumed_from_pause_mt = progress.current_state.active_mt.clone();
        progress.status = ProgressStatus::InProgress;
        progress.updated_at = Utc::now();
        write_json_atomic(&repo_root, &progress_abs, &progress)?;
        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

        record_micro_task_event(
            state,
            FlightRecorderEventType::MicroTaskResumed,
            "FR-EVT-MT-008",
            "micro_task_resumed",
            trace_id,
            job.job_id,
            workflow_run_id,
            json!({ "wp_id": inputs.wp_id }),
        )
        .await;
    }

    if loaded_existing_state && progress.status == ProgressStatus::InProgress {
        let mut steps_recovered = 0u32;
        let mut steps_to_retry = 0u32;
        for step in run_ledger.steps.iter_mut() {
            if step.status != LedgerStepStatus::InProgress {
                continue;
            }

            let output_rel = job_dir_rel
                .join("step_outputs")
                .join(&step.idempotency_key)
                .join("output.json");
            let output_abs = repo_root.join(&output_rel);
            if output_abs.exists() {
                step.status = LedgerStepStatus::Completed;
                step.output_artifact_ref = Some(artifact_handle_for_rel(&output_rel));
                steps_recovered = steps_recovered.saturating_add(1);
            } else {
                step.status = LedgerStepStatus::Pending;
                step.error = Some("crash_recovery: reset in_progress -> pending".to_string());
                steps_to_retry = steps_to_retry.saturating_add(1);
            }
        }

        if steps_recovered > 0 || steps_to_retry > 0 {
            run_ledger.resume_point = run_ledger
                .steps
                .iter()
                .find(|s| {
                    matches!(
                        s.status,
                        LedgerStepStatus::Pending | LedgerStepStatus::Failed
                    )
                })
                .map(|s| s.step_id.clone());
            run_ledger.resume_reason = Some("crash_recovery".to_string());
            progress.updated_at = Utc::now();
            write_json_atomic(&repo_root, &progress_abs, &progress)?;
            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

            let payload = json!({
                "workflow_run_id": workflow_run_id.to_string(),
                "job_id": job.job_id.to_string(),
                "from_state": "stalled",
                "to_state": "running",
                "reason": format!(
                    "micro_task_executor crash recovery: wp_id={} resume_point={:?} steps_recovered={} steps_to_retry={}",
                    inputs.wp_id.as_str(),
                    run_ledger.resume_point.as_ref(),
                    steps_recovered,
                    steps_to_retry
                ),
                "last_heartbeat_ts": progress.updated_at.to_rfc3339(),
                "threshold_secs": 0,
                "resume_point": run_ledger.resume_point,
                "steps_recovered": steps_recovered,
                "steps_to_retry": steps_to_retry,
            });

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::WorkflowRecovery,
                    FlightRecorderActor::System,
                    Uuid::new_v4(),
                    payload,
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;
        }
    }

    if !loaded_existing_state {
        record_micro_task_event(
            state,
            FlightRecorderEventType::MicroTaskLoopStarted,
            "FR-EVT-MT-001",
            "micro_task_loop_started",
            trace_id,
            job.job_id,
            workflow_run_id,
            json!({
                "wp_id": inputs.wp_id,
                "total_mts": mt_definitions.len(),
                "execution_policy": serde_json::to_value(&policy).unwrap_or(json!({})),
                "mt_ids": mt_definitions.iter().map(|m| m.mt_id.clone()).collect::<Vec<_>>(),
                "execution_waves": execution_waves,
            }),
        )
        .await;
    }

    let mex_runtime = build_mex_runtime(state, &repo_root)?;

    for wave in &execution_waves {
        for mt_id in wave {
            let mt = mt_definitions
                .iter()
                .find(|m| m.mt_id == *mt_id)
                .ok_or_else(|| {
                    WorkflowError::Terminal(format!("missing mt_definition for {mt_id}"))
                })?;
            let mt_progress_index = progress
                .micro_tasks
                .iter()
                .position(|m| m.mt_id == *mt_id)
                .ok_or_else(|| {
                    WorkflowError::Terminal(format!("missing progress entry for {mt_id}"))
                })?;
            let continuing_active_mt =
                progress.current_state.active_mt.as_deref() == Some(mt_id.as_str());

            let current_mt_status = progress.micro_tasks[mt_progress_index].status;
            if matches!(current_mt_status, MTStatus::Completed | MTStatus::Skipped) {
                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskSkipped,
                    "FR-EVT-MT-016",
                    "micro_task_skipped",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({ "wp_id": inputs.wp_id, "mt_id": mt.mt_id }),
                )
                .await;
                continue;
            }

            let resuming_this_mt = resumed_from_pause_mt.as_deref() == Some(mt_id.as_str());
            if resuming_this_mt {
                if let Some(pending_gate) = progress.current_state.pending_gov_gate.clone() {
                    if pending_gate.mt_id == mt.mt_id {
                        let decision = load_governance_decision_artifact(
                            &runtime_paths,
                            &pending_gate.decision_id,
                        )?;
                        let decision_abs = runtime_paths
                            .governance_decision_path(pending_gate.decision_id.as_str());
                        let decision_artifact = runtime_artifact_handle_for_abs_path(
                            runtime_paths.workspace_root(),
                            &decision_abs,
                            &format!("gov_decision:{}", pending_gate.decision_id),
                        )?;

                        emit_gov_human_intervention_received(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![decision_artifact.canonical_id()]),
                        )
                        .await?;

                        if pending_gate.final_model_level > 0
                            && !matches!(policy.drop_back_strategy, DropBackStrategy::Never)
                        {
                            record_micro_task_event(
                                state,
                                FlightRecorderEventType::MicroTaskDropBack,
                                "FR-EVT-MT-014",
                                "micro_task_drop_back",
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                json!({
                                    "wp_id": inputs.wp_id,
                                    "from_level": pending_gate.final_model_level,
                                    "to_level": 0
                                }),
                            )
                            .await;

                            progress.current_state.total_drop_backs =
                                progress.current_state.total_drop_backs.saturating_add(1);
                            progress.current_state.active_model_level = 0;
                        }

                        progress.micro_tasks[mt_progress_index].status = MTStatus::Completed;
                        progress.micro_tasks[mt_progress_index].final_iteration =
                            Some(pending_gate.final_iteration);
                        progress.micro_tasks[mt_progress_index].final_model_level =
                            Some(pending_gate.final_model_level);
                        progress.current_state.active_mt = None;
                        progress.current_state.pending_gov_gate = None;
                        progress.updated_at = Utc::now();
                        refresh_aggregate_stats(&mut progress);
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        record_micro_task_event(
                            state,
                            FlightRecorderEventType::MicroTaskComplete,
                            "FR-EVT-MT-004",
                            "micro_task_complete",
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            json!({ "wp_id": inputs.wp_id, "mt_id": mt.mt_id }),
                        )
                        .await;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![decision_artifact.canonical_id()]),
                        )
                        .await?;

                        resumed_from_pause_mt = None;
                        continue;
                    }
                }
            }
            if policy.pause_points.iter().any(|p| p == mt_id) && !resuming_this_mt {
                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskPauseRequested,
                    "FR-EVT-MT-007",
                    "micro_task_pause_requested",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({ "wp_id": inputs.wp_id, "mt_id": mt.mt_id }),
                )
                .await;

                if automation_level.is_locked() {
                    let target_ref =
                        micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                    let (decision, artifact) = create_gov_decision_and_emit_created(
                        state,
                        &runtime_paths,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        inputs.wp_id.as_str(),
                        Some(mt.mt_id.as_str()),
                        automation_level,
                        GOV_GATE_TYPE_HUMAN_INTERVENTION,
                        target_ref.as_str(),
                        GovernanceDecisionOutcome::Defer,
                        1.0,
                        "locked_fail_closed:pause_point",
                        None,
                        None,
                        None,
                    )
                    .await?;

                    emit_gov_decision_applied(
                        state,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        inputs.wp_id.as_str(),
                        Some(mt.mt_id.as_str()),
                        automation_level,
                        &decision,
                        Some(vec![artifact.canonical_id()]),
                    )
                    .await?;

                    progress.status = ProgressStatus::Failed;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::Failed,
                        status_reason: "locked_fail_closed".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "pause_point",
                            "mt_id": mt.mt_id,
                            "decision_id": decision.decision_id,
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: Some("LOCKED fail-closed".to_string()),
                    });
                }

                progress.status = ProgressStatus::Paused;
                progress.current_state.active_mt = Some(mt_id.clone());
                progress.updated_at = Utc::now();
                run_ledger.resume_reason = Some(format!("pause_point:{}", mt.mt_id));
                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                return Ok(RunJobOutcome {
                    state: JobState::AwaitingUser,
                    status_reason: "paused_user_gate".to_string(),
                    output: Some(json!({
                        "wp_id": inputs.wp_id,
                        "reason": "pause_point",
                        "mt_id": mt.mt_id,
                        "mt_definitions_ref": mt_definitions_ref,
                        "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                        "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                    })),
                    error_message: None,
                });
            }
            if resuming_this_mt {
                resumed_from_pause_mt = None;
            }

            if !continuing_active_mt {
                progress.current_state.active_model_level = 0;
            }

            progress.micro_tasks[mt_progress_index].status = MTStatus::InProgress;
            progress.current_state.active_mt = Some(mt_id.clone());
            progress.updated_at = Utc::now();
            write_json_atomic(&repo_root, &progress_abs, &progress)?;

            let mut escalation_level: u32 = progress.current_state.active_model_level;
            let mut false_completion_streak: u32 = 0;
            let mut iteration: u32 = progress.micro_tasks[mt_progress_index]
                .iterations
                .iter()
                .filter(|r| r.escalation_level == escalation_level)
                .map(|r| r.iteration)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            if iteration == 0 {
                iteration = 1;
            }

            let mut pending_model_swap: Option<PendingModelSwapV0_4> = None;

            loop {
                if progress.current_state.total_iterations >= policy.max_total_iterations {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({ "wp_id": inputs.wp_id, "reason": "max_total_iterations", "mt_id": mt.mt_id }),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_HUMAN_INTERVENTION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Defer,
                            1.0,
                            "locked_fail_closed:max_total_iterations",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "max_total_iterations",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "max_total_iterations",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                let elapsed_ms = Utc::now()
                    .signed_duration_since(progress.created_at)
                    .num_milliseconds()
                    .max(0) as u64;
                if elapsed_ms >= policy.max_duration_ms {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({ "wp_id": inputs.wp_id, "reason": "max_duration", "mt_id": mt.mt_id }),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_HUMAN_INTERVENTION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Defer,
                            1.0,
                            "locked_fail_closed:max_duration",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "max_duration",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "max_duration",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                let Some(level_cfg) = policy.escalation_chain.get(escalation_level as usize) else {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_id": mt.mt_id,
                            "from_level": escalation_level,
                        }),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_HUMAN_INTERVENTION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Defer,
                            1.0,
                            "locked_fail_closed:escalation_exhausted",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "escalation_exhausted",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                };

                if level_cfg.is_cloud && !policy.cloud_escalation_allowed {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "reason": "cloud_escalation_disallowed",
                            "mt_id": mt.mt_id,
                            "from_level": escalation_level,
                        }),
                    )
                    .await;

                    let denied_request_id = Uuid::new_v4().to_string();
                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::CloudEscalationDenied,
                            FlightRecorderActor::System,
                            trace_id,
                            json!({
                                "type": "cloud_escalation_denied",
                                "request_id": denied_request_id,
                                "reason": "cloud_escalation_disallowed",
                                "requested_model_id": level_cfg.model_id.clone(),
                                "wp_id": inputs.wp_id,
                                "mt_id": mt.mt_id,
                                "outcome": "denied",
                            }),
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_CLOUD_ESCALATION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Reject,
                            1.0,
                            "locked_cloud_escalation_denied",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "cloud_escalation_disallowed",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "cloud_escalation_disallowed",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                if level_cfg.is_hard_gate || level_cfg.model_id == "HARD_GATE" {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_id": mt.mt_id,
                            "from_level": escalation_level,
                        }),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_HUMAN_INTERVENTION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Defer,
                            1.0,
                            "locked_fail_closed:hard_gate",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "escalation_exhausted",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                let model_id = level_cfg.model_id.clone();
                let lora_id = level_cfg.lora_id.clone();

                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskLoraSelection,
                    "FR-EVT-MT-013",
                    "micro_task_lora_selection",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "wp_id": inputs.wp_id,
                        "mt_id": mt.mt_id,
                        "iteration": iteration,
                        "model_id": model_id.clone(),
                    }),
                )
                .await;

                let context_snapshot_id = Uuid::new_v4();

                // -------------------------------------------------------------------------
                // Â§2.6.6.8.8 MT Context Compilation (ACE-integrated; PromptEnvelope + Snapshot)
                // -------------------------------------------------------------------------

                let mut mt_files: Vec<String> = mt
                    .files
                    .read
                    .iter()
                    .chain(mt.files.modify.iter())
                    .cloned()
                    .collect();
                mt_files.sort();
                mt_files.dedup();

                let system_rules_budget = 300u32;
                let iteration_context_budget = 200u32;
                let mt_definition_budget = 500u32;
                let previous_output_budget = if iteration > 1 { 200u32 } else { 0u32 };

                let mut file_contents_budget = mt
                    .token_budget
                    .saturating_sub(system_rules_budget)
                    .saturating_sub(iteration_context_budget)
                    .saturating_sub(mt_definition_budget)
                    .saturating_sub(previous_output_budget);

                let completed_mts = progress
                    .micro_tasks
                    .iter()
                    .filter(|m| matches!(m.status, MTStatus::Completed))
                    .count();
                let total_mts = progress.micro_tasks.len();

                let previous_iter = progress.micro_tasks[mt_progress_index].iterations.last();
                let previous_outcome = previous_iter
                    .map(|r| match r.outcome {
                        IterationOutcome::Success => "SUCCESS",
                        IterationOutcome::Retry => "RETRY",
                        IterationOutcome::Escalate => "ESCALATE",
                        IterationOutcome::Blocked => "BLOCKED",
                        IterationOutcome::Skipped => "SKIPPED",
                    })
                    .unwrap_or("FIRST_ATTEMPT");
                let previous_error = previous_iter
                    .and_then(|r| r.error_summary.clone())
                    .unwrap_or_default();
                let previous_output_summary = if iteration > 1 {
                    previous_output_summary_for_mt(
                        &repo_root,
                        &run_ledger,
                        &mt.mt_id,
                        previous_output_budget,
                    )
                    .unwrap_or_default()
                } else {
                    String::new()
                };

                let system_rules_raw = r#"Follow the Work Packet scope and only modify allowed files.
Do NOT output <mt_complete> unless ALL done criteria are satisfied with concrete file:line evidence.
The completion claim is untrusted; validations will be run after completion is claimed."#;
                let (system_rules, _) =
                    truncate_to_token_budget(system_rules_raw, system_rules_budget);

                let iteration_context_raw = format!(
                    r#"**Loop:** Iteration {iteration} of {max_iterations}
**MT:** {mt_id} - {mt_name}
**Model:** {model_id} {lora_info}
**Escalation Level:** {level} of {max_level}

**Previous Outcome:** {previous_outcome}
{previous_error_block}

**Overall Progress:** {completed_mts} of {total_mts} MTs complete"#,
                    iteration = iteration,
                    max_iterations = policy.max_iterations_per_mt,
                    mt_id = mt.mt_id,
                    mt_name = mt.name,
                    model_id = model_id,
                    lora_info = lora_id
                        .as_deref()
                        .map(|l| format!("+ LoRA: {l}"))
                        .unwrap_or_default(),
                    level = escalation_level,
                    max_level = policy.escalation_chain.len().saturating_sub(1),
                    previous_outcome = previous_outcome,
                    previous_error_block = if previous_error.trim().is_empty() {
                        String::new()
                    } else {
                        format!("**Previous Error:**\n```\n{previous_error}\n```")
                    },
                    completed_mts = completed_mts,
                    total_mts = total_mts
                );
                let (iteration_context, _) =
                    truncate_to_token_budget(&iteration_context_raw, iteration_context_budget);

                let mut mt_definition_raw = String::new();
                mt_definition_raw.push_str("### Scope\n");
                mt_definition_raw.push_str(mt.scope.trim());
                mt_definition_raw.push_str("\n\n### Actions (in order)\n");
                for (idx, action) in mt.actions.iter().enumerate() {
                    mt_definition_raw.push_str(&format!("{}. {}\n", idx + 1, action));
                }
                mt_definition_raw.push_str("\n### Done Criteria\n");
                for criterion in &mt.done {
                    mt_definition_raw.push_str(&format!("- [ ] {}\n", criterion.description));
                }
                mt_definition_raw.push_str("\n### Verification Commands\n");
                for spec in &mt.verify {
                    mt_definition_raw.push_str(&format!("- `{}`\n", spec.command));
                }
                if let Some(notes) = mt.notes.as_deref() {
                    if !notes.trim().is_empty() {
                        mt_definition_raw.push_str("\n### Notes\n");
                        mt_definition_raw.push_str(notes.trim());
                        mt_definition_raw.push('\n');
                    }
                }
                let (mt_definition, _) =
                    truncate_to_token_budget(&mt_definition_raw, mt_definition_budget);

                let completion_protocol = format!(
                    r#"<mt_complete>
MT_ID: {mt_id}
EVIDENCE:
- "{{done_criterion}}" -> {{file}}:{{line_start}}-{{line_end}}
[one line per done criterion]
</mt_complete>

<blocked>
REASON: {{specific reason}}
NEED: {{what you need to unblock}}
</blocked>"#,
                    mt_id = mt.mt_id
                );

                let mut warnings: Vec<String> = Vec::new();
                let mut selected_files: Vec<MtContextFile> = Vec::new();
                let mut candidate_source_refs: Vec<SourceRef> = Vec::new();

                let query_text = build_mt_context_query_text(mt);
                let query_terms = build_mt_context_query_terms(&query_text);
                let max_shadow_results = 10usize;
                let neighbor_lines = 20usize;

                let mut query_plan = QueryPlan::new(
                    query_text,
                    QueryKind::Transform,
                    "mt_context_compilation".to_string(),
                )
                .with_default_route();
                query_plan.determinism_mode = DeterminismMode::Replay;
                query_plan.budgets.max_total_evidence_tokens = file_contents_budget.max(1);
                query_plan.budgets.max_snippets_per_source = max_shadow_results as u32;
                query_plan.budgets.max_snippets_total = (mt_files.len() as u32)
                    .saturating_mul(max_shadow_results as u32)
                    .max(1);
                query_plan.budgets.max_candidates_total = (mt_files.len() as u32).max(1);
                query_plan.budgets.max_read_tokens = 500;
                let request_id = query_plan.plan_id.to_string();

                let mut retrieval_trace = RetrievalTrace::new(&query_plan);
                retrieval_trace.route_taken.push(RouteTaken {
                    store: StoreKind::ContextPacks,
                    reason: "context_packs:attempt".to_string(),
                    cache_hit: Some(false),
                });
                retrieval_trace.route_taken.push(RouteTaken {
                    store: StoreKind::ShadowWsLexical,
                    reason: "shadow_ws_lexical:deterministic_chunks".to_string(),
                    cache_hit: Some(false),
                });

                let context_pack_builder_config = default_context_pack_builder_config_v1();
                let context_pack_builder_config_hash =
                    context_pack_builder_config_hash_v1(&context_pack_builder_config);

                let regen_capability_id = "context_packs.regen";
                let regen_capability_allowed = match state
                    .capability_registry
                    .profile_can(&job.capability_profile_id, regen_capability_id)
                {
                    Ok(true) => {
                        log_capability_check(state, job, regen_capability_id, "allow", trace_id)
                            .await;
                        true
                    }
                    Ok(false) => {
                        log_capability_check(state, job, regen_capability_id, "deny", trace_id)
                            .await;
                        false
                    }
                    Err(err) => {
                        log_capability_check(state, job, regen_capability_id, "deny", trace_id)
                            .await;
                        warnings.push(format!("context_pack:regen_capability_error:{err}"));
                        false
                    }
                };

                let context_pack_policy = ContextPackFreshnessPolicyV1 {
                    regen_allowed: policy.context_pack_policy.regen_allowed
                        && policy.context_pack_policy.human_consent_obtained
                        && regen_capability_allowed,
                    regen_required: policy.context_pack_policy.regen_required,
                };

                if policy.context_pack_policy.regen_allowed
                    && !policy.context_pack_policy.human_consent_obtained
                {
                    let w = "context_pack:regen_denied:consent_missing".to_string();
                    warnings.push(w.clone());
                    retrieval_trace.warnings.push(w);
                }

                if policy.context_pack_policy.regen_allowed
                    && policy.context_pack_policy.human_consent_obtained
                    && !regen_capability_allowed
                {
                    let w = "context_pack:regen_denied:capability".to_string();
                    warnings.push(w.clone());
                    retrieval_trace.warnings.push(w);
                }

                let mut context_pack_used = 0u32;
                let mut context_pack_fallback = 0u32;
                let mut context_pack_records: Vec<ContextPackRecord> = Vec::new();

                let per_file_budget = if mt_files.is_empty() {
                    0u32
                } else {
                    (file_contents_budget / (mt_files.len() as u32)).max(64)
                };
                // WAIVER [CX-573E]: Instant::now() for observability (retrieval duration metrics).
                let retrieval_start = std::time::Instant::now();
                for path in &mt_files {
                    if file_contents_budget == 0 {
                        warnings.push("file_contents_budget exhausted".to_string());
                        break;
                    }

                    let allowance = file_contents_budget.min(per_file_budget);

                    let abs = repo_root.join(PathBuf::from(path));
                    let bytes = match fs::read(&abs) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            let warning = format!("missing_or_unreadable_file:{path}");
                            warnings.push(warning.clone());
                            retrieval_trace.warnings.push(warning);
                            continue;
                        }
                    };

                    let source_hash = sha256_hex(&bytes);
                    let source_id = deterministic_uuid_for_str(path);
                    let source_ref = SourceRef::new(source_id, source_hash.clone());

                    let pack_outcome = retrieve_context_pack_for_source(
                        &repo_root,
                        path,
                        source_ref.clone(),
                        &bytes,
                        &query_terms,
                        max_shadow_results,
                        allowance,
                        query_plan.budgets.max_read_tokens,
                        &context_pack_builder_config,
                        &context_pack_builder_config_hash,
                        &context_pack_policy,
                    )?;

                    let outcome = match pack_outcome {
                        ContextPackOutcome::Selected(selection) => {
                            if selection.file.token_estimate == 0 {
                                let warning = format!("context_pack:empty_snippet:{path}");
                                warnings.push(warning.clone());
                                retrieval_trace.warnings.push(warning);
                            } else {
                                context_pack_used = context_pack_used.saturating_add(1);
                                context_pack_records.push(selection.record.clone());

                                let mut scores = CandidateScores::default();
                                scores.pack = Some(1.0);
                                scores.lexical = Some(selection.match_score as f64);

                                retrieval_trace
                                    .candidates
                                    .push(RetrievalCandidate::from_source(
                                        selection.source_ref.clone(),
                                        StoreKind::ContextPacks,
                                        scores.clone(),
                                    ));

                                retrieval_trace.selected.push(SelectedEvidence {
                                    candidate_ref: CandidateRef::Source(
                                        selection.source_ref.clone(),
                                    ),
                                    final_rank: retrieval_trace.selected.len() as u32,
                                    final_score: selection.match_score as f64,
                                    why: "mt_context_compilation_context_pack".to_string(),
                                });

                                if selection.file.truncated {
                                    retrieval_trace.truncation_flags.push(format!(
                                        "truncated:{}",
                                        selection.source_ref.source_id
                                    ));
                                }

                                retrieval_trace.spans.extend(selection.spans.clone());
                                for w in selection.warnings {
                                    warnings.push(w.clone());
                                    retrieval_trace.warnings.push(w);
                                }

                                candidate_source_refs.push(selection.source_ref);
                                file_contents_budget = file_contents_budget
                                    .saturating_sub(selection.file.token_estimate);
                                selected_files.push(selection.file);
                                continue;
                            }
                            retrieve_shadow_ws_lexical_for_file(
                                path,
                                source_ref.clone(),
                                &bytes,
                                &query_terms,
                                max_shadow_results,
                                neighbor_lines,
                                allowance,
                                query_plan.budgets.max_read_tokens,
                            )?
                        }
                        ContextPackOutcome::Fallback(fallback) => {
                            context_pack_fallback = context_pack_fallback.saturating_add(1);
                            for w in &fallback.warnings {
                                warnings.push(w.clone());
                                retrieval_trace.warnings.push(w.clone());
                            }

                            if fallback.pack_id.is_some() {
                                let mut scores = CandidateScores::default();
                                scores.pack = Some(0.0);
                                retrieval_trace
                                    .candidates
                                    .push(RetrievalCandidate::from_source(
                                        fallback.source_ref.clone(),
                                        StoreKind::ContextPacks,
                                        scores,
                                    ));
                            }

                            retrieve_shadow_ws_lexical_for_file(
                                path,
                                source_ref.clone(),
                                &bytes,
                                &query_terms,
                                max_shadow_results,
                                neighbor_lines,
                                allowance,
                                query_plan.budgets.max_read_tokens,
                            )?
                        }
                        ContextPackOutcome::Skipped { warning } => {
                            warnings.push(warning.clone());
                            retrieval_trace.warnings.push(warning);
                            continue;
                        }
                    };

                    match outcome {
                        ShadowWsLexicalOutcome::Skipped { warning } => {
                            warnings.push(warning.clone());
                            retrieval_trace.warnings.push(warning);
                            continue;
                        }
                        ShadowWsLexicalOutcome::Selected(selection) => {
                            if selection.file.token_estimate == 0 {
                                warnings.push(format!("shadow_ws_lexical:empty_snippet:{path}"));
                                retrieval_trace
                                    .warnings
                                    .push(format!("shadow_ws_lexical:empty_snippet:{path}"));
                                continue;
                            }

                            let mut scores = CandidateScores::default();
                            scores.lexical = Some(selection.match_score as f64);

                            retrieval_trace
                                .candidates
                                .push(RetrievalCandidate::from_source(
                                    selection.source_ref.clone(),
                                    StoreKind::ShadowWsLexical,
                                    scores.clone(),
                                ));

                            retrieval_trace.selected.push(SelectedEvidence {
                                candidate_ref: CandidateRef::Source(selection.source_ref.clone()),
                                final_rank: retrieval_trace.selected.len() as u32,
                                final_score: selection.match_score as f64,
                                why: "mt_context_compilation_shadow_ws_lexical".to_string(),
                            });

                            if selection.file.truncated {
                                retrieval_trace
                                    .truncation_flags
                                    .push(format!("truncated:{}", selection.source_ref.source_id));
                            }

                            retrieval_trace.spans.extend(selection.spans.clone());
                            for w in selection.warnings {
                                warnings.push(w.clone());
                                retrieval_trace.warnings.push(w);
                            }

                            candidate_source_refs.push(selection.source_ref);
                            file_contents_budget =
                                file_contents_budget.saturating_sub(selection.file.token_estimate);
                            selected_files.push(selection.file);
                        }
                    }
                }

                if let Some(route) = retrieval_trace
                    .route_taken
                    .iter_mut()
                    .find(|r| r.store == StoreKind::ContextPacks)
                {
                    route.reason = format!(
                        "context_packs:used={} fallback={} regen_allowed={} regen_required={}",
                        context_pack_used,
                        context_pack_fallback,
                        context_pack_policy.regen_allowed,
                        context_pack_policy.regen_required
                    );
                    route.cache_hit = Some(context_pack_used > 0);
                }
                let retrieval_elapsed_ms = retrieval_start.elapsed().as_millis() as u64;

                // [§2.6.6.8.8.1] Validate plan + trace via ACE Runtime validators.
                // Treat failures as blocking for this iteration (do not proceed with unvalidated context).
                let pipeline = ValidatorPipeline::with_default_guards();
                pipeline
                    .validate_plan(&query_plan)
                    .await
                    .map_err(WorkflowError::SecurityViolation)?;
                pipeline
                    .validate_trace(&retrieval_trace)
                    .await
                    .map_err(WorkflowError::SecurityViolation)?;

                record_event_safely(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::DataRetrievalExecuted,
                        FlightRecorderActor::System,
                        trace_id,
                        json!({
                            "type": "data_retrieval_executed",
                            "request_id": request_id.clone(),
                            "query_hash": retrieval_trace.normalized_query_hash.clone(),
                            "query_intent": "code_search",
                            "weights": {
                                "vector": 0.0,
                                "keyword": 1.0,
                                "graph": 0.0,
                            },
                            "results": {
                                "vector_candidates": 0,
                                "keyword_candidates": retrieval_trace.candidates.len(),
                                "after_fusion": retrieval_trace.selected.len(),
                                "final_count": retrieval_trace.selected.len(),
                            },
                            "latency": {
                                "embedding_ms": 0,
                                "vector_search_ms": 0,
                                "keyword_search_ms": retrieval_elapsed_ms,
                                "total_ms": retrieval_elapsed_ms,
                            },
                            "reranking_used": false,
                        }),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await;

                let mut files_section = String::new();
                for file in &selected_files {
                    files_section.push_str(&format!(
                        "### {}\n```\n{}\n```\n\n",
                        file.path, file.content
                    ));
                }

                let context_size_tokens: u64 = selected_files
                    .iter()
                    .map(|file| file.token_estimate as u64)
                    .sum();
                record_event_safely(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::DataContextAssembled,
                        FlightRecorderActor::System,
                        trace_id,
                        json!({
                            "type": "data_context_assembled",
                            "request_id": request_id.clone(),
                            "selected_chunks": retrieval_trace.spans.len(),
                            "context_size_tokens": context_size_tokens,
                        }),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await;

                let previous_output_section = if iteration > 1 {
                    let (prev, _) =
                        truncate_to_token_budget(&previous_output_summary, previous_output_budget);
                    format!("## PREVIOUS ITERATION\n```\n{prev}\n```\n")
                } else {
                    String::new()
                };

                let stable_prefix = format!(
                    "## SYSTEM RULES\n{system_rules}\n\n## YOUR TASK\n{mt_definition}\n\n## COMPLETION PROTOCOL\n{completion_protocol}\n",
                );
                let variable_suffix = format!(
                    "## MICRO-TASK CONTEXT\n{iteration_context}\n\n## FILES\n{files_section}\n\n{previous_output_section}\nBEGIN WORK:\n"
                );
                let prompt = format!("{stable_prefix}\n\n{variable_suffix}");

                let stable_prefix_hash = sha256_hex_str(&stable_prefix);
                let variable_suffix_hash = sha256_hex_str(&variable_suffix);
                let prompt_hash = sha256_hex_str(&prompt);
                let idempo = idempotency_key(
                    &mt.mt_id,
                    iteration,
                    &model_id,
                    lora_id.as_deref(),
                    &prompt_hash,
                );
                let step_id = format!("{}_iter-{:03}", mt.mt_id, iteration);
                let step_dir_rel = job_dir_rel.join("step_outputs").join(&idempo);
                let prompt_rel = step_dir_rel.join("prompt.txt");
                let context_files_rel = step_dir_rel.join("context_files.json");
                let context_snapshot_rel = step_dir_rel.join("context_snapshot.json");
                let response_rel = step_dir_rel.join("response.txt");
                let output_rel = step_dir_rel.join("output.json");
                let validation_rel = step_dir_rel.join("validation.json");
                let validation_evidence_rel = step_dir_rel.join("validation_evidence.json");

                let prompt_snapshot_ref = artifact_handle_for_rel(&prompt_rel);
                let context_files_ref = artifact_handle_for_rel(&context_files_rel);
                let context_snapshot_ref = artifact_handle_for_rel(&context_snapshot_rel);
                let output_snapshot_ref = artifact_handle_for_rel(&response_rel);
                let validation_ref = artifact_handle_for_rel(&validation_rel);
                let validation_evidence_ref = artifact_handle_for_rel(&validation_evidence_rel);

                let ace_query_plan_rel = step_dir_rel.join("ace_query_plan.json");
                let ace_retrieval_trace_rel = step_dir_rel.join("ace_retrieval_trace.json");
                let ace_query_plan_ref = artifact_handle_for_rel(&ace_query_plan_rel);
                let ace_retrieval_trace_ref = artifact_handle_for_rel(&ace_retrieval_trace_rel);

                write_json_atomic(
                    &repo_root,
                    &repo_root.join(&ace_query_plan_rel),
                    &query_plan,
                )?;
                write_json_atomic(
                    &repo_root,
                    &repo_root.join(&ace_retrieval_trace_rel),
                    &retrieval_trace,
                )?;

                let context_files_artifact = MtContextFilesArtifact {
                    schema_version: "1.0".to_string(),
                    mt_id: mt.mt_id.clone(),
                    iteration,
                    files: selected_files.clone(),
                };
                write_json_atomic(
                    &repo_root,
                    &repo_root.join(&context_files_rel),
                    &context_files_artifact,
                )?;

                let mut candidate_ids: Vec<String> = candidate_source_refs
                    .iter()
                    .map(SourceRef::canonical_id)
                    .collect();
                candidate_ids.sort();
                let candidate_ids_hash = sha256_hex_str(&candidate_ids.join("\n"));

                let mut selected_ids: Vec<String> = selected_files
                    .iter()
                    .map(|f| SourceRef::new(f.source_id, f.source_hash.clone()).canonical_id())
                    .collect();
                selected_ids.sort();
                let selected_ids_hash = sha256_hex_str(&selected_ids.join("\n"));

                let scope_inputs_hash = {
                    let scope_inputs = json!({
                        "entity_refs": mt_files,
                        "task_context": {
                            "wp_id": inputs.wp_id,
                            "mt_id": mt.mt_id,
                            "scope": mt.scope,
                            "actions": mt.actions,
                        },
                        "iteration_context": {
                            "iteration": iteration,
                            "escalation_level": escalation_level,
                            "previous_outcome": previous_outcome,
                        }
                    });
                    let json = serde_json::to_string(&scope_inputs).unwrap_or_default();
                    sha256_hex_str(&json)
                };

                let model_tier = match state.llm_client.profile().model_tier {
                    crate::llm::ModelTier::Cloud => "cloud",
                    crate::llm::ModelTier::Local => "local",
                }
                .to_string();

                let snapshot = MtContextSnapshot {
                    context_snapshot_id,
                    job_id: job.job_id.to_string(),
                    step_id: step_id.clone(),
                    created_at: Utc::now(),
                    determinism_mode: "replay".to_string(),
                    model_tier,
                    model_id: model_id.clone(),
                    policy_profile_id: job.capability_profile_id.clone(),
                    layer_scope: MtLayerScope {
                        read: vec![
                            "raw".to_string(),
                            "derived".to_string(),
                            "display".to_string(),
                        ],
                        write: vec!["derived".to_string()],
                    },
                    scope_inputs_hash,
                    retrieval_candidates: MtIdsHashCount {
                        ids_hash: candidate_ids_hash.clone(),
                        count: candidate_ids.len() as u32,
                    },
                    selected_sources: MtIdsHashCount {
                        ids_hash: selected_ids_hash,
                        count: selected_ids.len() as u32,
                    },
                    prompt_envelope_hashes: MtPromptEnvelopeHashes {
                        stable_prefix_hash: stable_prefix_hash.clone(),
                        variable_suffix_hash: variable_suffix_hash.clone(),
                        full_prompt_hash: prompt_hash.clone(),
                    },
                    artifact_handles: vec![
                        context_files_ref.clone(),
                        ace_query_plan_ref.clone(),
                        ace_retrieval_trace_ref.clone(),
                    ],
                    warnings,
                    local_only_payload_ref: Some(prompt_snapshot_ref.clone()),
                };
                let context_snapshot_hash = write_json_atomic_with_hash(
                    &repo_root,
                    &repo_root.join(&context_snapshot_rel),
                    &snapshot,
                )?;

                if let Some(pending) = pending_model_swap.take() {
                    if pending.request.target_model_id == model_id {
                        let context_compile_abs = repo_root.join(&pending.context_compile_rel);
                        let created_at = Utc::now().to_rfc3339();
                        let context_compile_payload = build_context_compile_payload_v1(
                            &pending.request.request_id,
                            job.job_id,
                            &inputs.wp_id,
                            &mt.mt_id,
                            created_at.as_str(),
                            &model_id,
                            &context_snapshot_ref,
                            &context_snapshot_hash,
                            &ace_query_plan_ref,
                            &ace_retrieval_trace_ref,
                            &context_pack_records,
                            &retrieval_trace.warnings,
                        );
                        write_json_atomic(
                            &repo_root,
                            &context_compile_abs,
                            &context_compile_payload,
                        )?;

                        record_model_swap_event_v0_4(
                            state,
                            FlightRecorderEventType::ModelSwapCompleted,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            mt.mt_id.as_str(),
                            &pending.request,
                            "model_swap_completed",
                            Some("success"),
                            None,
                        )
                        .await;
                    } else {
                        pending_model_swap = Some(pending);
                    }
                }

                // Write the prompt snapshot before any outbound call / consent decision so the
                // ProjectionPlan can reference local artifacts deterministically.
                write_bytes_atomic(&repo_root, &repo_root.join(&prompt_rel), prompt.as_bytes())?;

                let mut cloud_bundle_for_call: Option<CloudEscalationBundleV0_4> = None;
                let mut cloud_executed_event_payload: Option<Value> = None;

                if level_cfg.is_cloud {
                    if cloud_governance_locked {
                        // Spec 11.1.7.3: GovernanceMode LOCKED => deny cloud escalation without
                        // prompting for consent (fail-closed).
                        let pending = progress.current_state.pending_cloud_escalation.as_ref();
                        let (request_id, projection_plan_id, local_attempts, last_error_summary) =
                            match pending {
                                Some(p)
                                    if p.mt_id == mt.mt_id && p.requested_model_id == model_id =>
                                {
                                    (
                                        p.request_id.clone(),
                                        Some(p.projection_plan.projection_plan_id.clone()),
                                        Some(p.local_attempts),
                                        Some(p.last_error_summary.clone()),
                                    )
                                }
                                _ => (Uuid::new_v4().to_string(), None, None, None),
                            };

                        let mut denied_payload = json!({
                            "type": "cloud_escalation_denied",
                            "request_id": request_id,
                            "reason": "governance_locked",
                            "requested_model_id": model_id.clone(),
                            "wp_id": inputs.wp_id,
                            "mt_id": mt.mt_id,
                            "outcome": "denied",
                        });
                        if let Some(plan_id) = projection_plan_id {
                            denied_payload["projection_plan_id"] = Value::String(plan_id);
                        }
                        if let Some(attempts) = local_attempts {
                            denied_payload["local_attempts"] = json!(attempts);
                        }
                        if let Some(summary) = last_error_summary {
                            denied_payload["last_error_summary"] = Value::String(summary);
                        }

                        record_event_safely(
                            state,
                            FlightRecorderEvent::new(
                                FlightRecorderEventType::CloudEscalationDenied,
                                FlightRecorderActor::System,
                                trace_id,
                                denied_payload,
                            )
                            .with_job_id(job.job_id.to_string())
                            .with_workflow_id(workflow_run_id.to_string()),
                        )
                        .await;

                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_CLOUD_ESCALATION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Reject,
                            1.0,
                            "locked_cloud_escalation_denied",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "governance_locked",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some(
                                "GovernanceMode LOCKED; cloud escalation denied".to_string(),
                            ),
                        });
                    }
                    // Cloud escalation consent is always human-gated (Spec §11.1.7).
                    // Compute payload_sha256 from the canonical OpenAI-compatible request bytes.
                    let preview_req =
                        CompletionRequest::new(trace_id, prompt.clone(), model_id.clone());
                    let canonical_bytes =
                        openai_compat_canonical_request_bytes(&preview_req, model_id.as_str());
                    let payload_sha256 = sha256_hex(&canonical_bytes);

                    let prev_level = escalation_level.saturating_sub(1);
                    let local_attempts = progress.micro_tasks[mt_progress_index]
                        .iterations
                        .iter()
                        .filter(|r| r.escalation_level == prev_level)
                        .count()
                        .min(u32::MAX as usize) as u32;

                    let last_error_summary = progress.micro_tasks[mt_progress_index]
                        .iterations
                        .iter()
                        .rev()
                        .find(|r| r.escalation_level == prev_level)
                        .and_then(|r| r.error_summary.clone())
                        .unwrap_or_else(|| "escalation".to_string());
                    let mut last_error_summary =
                        last_error_summary.trim().replace(['\r', '\n', '\t'], " ");
                    if last_error_summary.len() > 256 {
                        last_error_summary.truncate(256);
                    }

                    let reason = if last_error_summary.trim().is_empty() {
                        "mt_escalation".to_string()
                    } else {
                        format!("mt_escalation:{last_error_summary}")
                    };

                    let pending = progress.current_state.pending_cloud_escalation.clone();

                    if let Some(pending) = pending {
                        let pending_matches = pending.mt_id == mt.mt_id
                            && pending.requested_model_id == model_id
                            && pending.projection_plan.payload_sha256 == payload_sha256;

                        if pending_matches {
                            if let (Some(receipt), Some(request)) = (
                                pending.consent_receipt.as_ref(),
                                pending.cloud_escalation_request.as_ref(),
                            ) {
                                if receipt.approved {
                                    cloud_bundle_for_call = Some(CloudEscalationBundleV0_4 {
                                        request: request.clone(),
                                        projection_plan: pending.projection_plan.clone(),
                                        consent_receipt: receipt.clone(),
                                    });
                                    cloud_executed_event_payload = Some(json!({
                                        "type": "cloud_escalation_executed",
                                        "request_id": pending.request_id,
                                        "reason": pending.reason,
                                        "requested_model_id": pending.requested_model_id,
                                        "projection_plan_id": pending.projection_plan.projection_plan_id,
                                        "consent_receipt_id": receipt.consent_receipt_id.clone(),
                                        "wp_id": inputs.wp_id,
                                        "mt_id": mt.mt_id,
                                        "local_attempts": pending.local_attempts,
                                        "last_error_summary": pending.last_error_summary,
                                        "outcome": "executed",
                                    }));
                                } else {
                                    // Explicit denial: keep cloud blocked; require human intervention.
                                    progress.status = ProgressStatus::Paused;
                                    progress.updated_at = Utc::now();
                                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                                    return Ok(RunJobOutcome {
                                        state: JobState::AwaitingUser,
                                        status_reason: "paused_hard_gate".to_string(),
                                        output: Some(json!({
                                            "wp_id": inputs.wp_id,
                                            "reason": "cloud_escalation_denied",
                                            "mt_id": mt.mt_id,
                                            "request_id": pending.request_id,
                                            "requested_model_id": pending.requested_model_id,
                                            "projection_plan_id": pending.projection_plan.projection_plan_id,
                                            "mt_definitions_ref": mt_definitions_ref,
                                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                        })),
                                        error_message: None,
                                    });
                                }
                            } else {
                                // Pending request exists but consent has not been recorded yet; keep paused.
                                progress.status = ProgressStatus::Paused;
                                progress.updated_at = Utc::now();
                                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                                return Ok(RunJobOutcome {
                                    state: JobState::AwaitingUser,
                                    status_reason: "paused_cloud_consent".to_string(),
                                    output: Some(json!({
                                        "wp_id": inputs.wp_id,
                                        "reason": "cloud_escalation_consent_required",
                                        "mt_id": mt.mt_id,
                                        "request_id": pending.request_id,
                                        "requested_model_id": pending.requested_model_id,
                                        "payload_sha256": pending.projection_plan.payload_sha256.clone(),
                                        "projection_plan_id": pending.projection_plan.projection_plan_id.clone(),
                                        "projection_plan": pending.projection_plan.clone(),
                                        "mt_definitions_ref": mt_definitions_ref,
                                        "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                        "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                    })),
                                    error_message: None,
                                });
                            }
                        } else {
                            // Stale/mismatched pending consent; clear and re-request deterministically below.
                            progress.current_state.pending_cloud_escalation = None;
                        }
                    }

                    if cloud_bundle_for_call.is_none() {
                        // Create new request + ProjectionPlan, then pause for explicit consent.
                        let request_id = Uuid::new_v4().to_string();
                        let projection_plan_id = Uuid::new_v4().to_string();
                        let created_at = Utc::now().to_rfc3339();

                        let cloud_dir_rel = job_dir_rel
                            .join("cloud_escalation")
                            .join(request_id.as_str());
                        let cloud_dir_abs = repo_root.join(&cloud_dir_rel);
                        fs::create_dir_all(&cloud_dir_abs).map_err(|e| {
                            WorkflowError::Terminal(format!(
                                "failed to create cloud escalation dir {}: {e}",
                                cloud_dir_abs.display()
                            ))
                        })?;

                        let projection_plan = ProjectionPlanV0_4 {
                            schema_version: "hsk.projection_plan@0.4".to_string(),
                            projection_plan_id: projection_plan_id.clone(),
                            include_artifact_refs: vec![
                                prompt_snapshot_ref.path.clone(),
                                context_files_ref.path.clone(),
                                context_snapshot_ref.path.clone(),
                            ],
                            include_fields: None,
                            redactions_applied: vec!["none".to_string()],
                            max_bytes: canonical_bytes.len().min(u32::MAX as usize) as u32,
                            payload_sha256: payload_sha256.clone(),
                            created_at: created_at.clone(),
                            job_id: Some(job.job_id.to_string()),
                            wp_id: Some(inputs.wp_id.clone()),
                            mt_id: Some(mt.mt_id.clone()),
                        };

                        let projection_plan_rel = cloud_dir_rel.join("projection_plan.json");
                        write_json_atomic(
                            &repo_root,
                            &repo_root.join(&projection_plan_rel),
                            &projection_plan,
                        )?;
                        let projection_plan_ref = artifact_handle_for_rel(&projection_plan_rel);

                        progress.current_state.pending_cloud_escalation =
                            Some(PendingCloudEscalation {
                                schema_version: "hsk.pending_cloud_escalation@0.1".to_string(),
                                request_id: request_id.clone(),
                                mt_id: mt.mt_id.clone(),
                                requested_model_id: model_id.clone(),
                                reason: reason.clone(),
                                local_attempts,
                                last_error_summary: last_error_summary.clone(),
                                projection_plan: projection_plan.clone(),
                                consent_receipt: None,
                                cloud_escalation_request: None,
                            });

                        progress.status = ProgressStatus::Paused;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        record_event_safely(
                            state,
                            FlightRecorderEvent::new(
                                FlightRecorderEventType::CloudEscalationRequested,
                                FlightRecorderActor::System,
                                trace_id,
                                json!({
                                    "type": "cloud_escalation_requested",
                                    "request_id": request_id.clone(),
                                    "reason": reason,
                                    "requested_model_id": model_id.clone(),
                                    "projection_plan_id": projection_plan_id.clone(),
                                    "wp_id": inputs.wp_id,
                                    "mt_id": mt.mt_id,
                                    "local_attempts": local_attempts,
                                    "last_error_summary": last_error_summary,
                                }),
                            )
                            .with_job_id(job.job_id.to_string())
                            .with_workflow_id(workflow_run_id.to_string()),
                        )
                        .await;

                        return Ok(RunJobOutcome {
                            state: JobState::AwaitingUser,
                            status_reason: "paused_cloud_consent".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "cloud_escalation_consent_required",
                                "mt_id": mt.mt_id,
                                "request_id": request_id,
                                "requested_model_id": model_id,
                                "payload_sha256": payload_sha256,
                                "projection_plan_id": projection_plan_id,
                                "projection_plan": projection_plan,
                                "projection_plan_ref": projection_plan_ref,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: None,
                        });
                    }
                }

                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskIterationStarted,
                    "FR-EVT-MT-002",
                    "micro_task_iteration_started",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({ "wp_id": inputs.wp_id, "mt_id": mt.mt_id, "iteration": iteration }),
                )
                .await;

                let started_ts = Utc::now();

                run_ledger.steps.push(LedgerStep {
                    step_id: step_id.clone(),
                    idempotency_key: idempo.clone(),
                    status: LedgerStepStatus::InProgress,
                    started_at: Some(started_ts),
                    completed_at: None,
                    output_artifact_ref: None,
                    error: None,
                    recoverable: true,
                });
                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                let mut req = CompletionRequest::new(trace_id, prompt.clone(), model_id.clone());

                if let Some(bundle) = cloud_bundle_for_call {
                    req.cloud_escalation = Some(bundle);
                }

                if let Some(payload) = cloud_executed_event_payload {
                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::CloudEscalationExecuted,
                            FlightRecorderActor::System,
                            trace_id,
                            payload,
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;
                }

                let response = state.llm_client.completion(req).await?;
                let completed_ts = Utc::now();
                let completion_signal = parse_completion_signal(&response.text);

                let validation_outcome = if completion_signal.claimed_complete {
                    Some(
                        run_validation_via_mex(
                            &mex_runtime,
                            &repo_root,
                            &mt.verify,
                            &job.capability_profile_id,
                            &validation_evidence_rel,
                            validation_evidence_ref.clone(),
                        )
                        .await?,
                    )
                } else {
                    None
                };
                if let Some(out) = &validation_outcome {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskValidation,
                        "FR-EVT-MT-012",
                        "micro_task_validation",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "mt_id": mt.mt_id,
                            "iteration": iteration,
                            "passed": out.passed,
                        }),
                    )
                    .await;
                }
                let validation_passed = validation_outcome
                    .as_ref()
                    .map(|v| v.passed)
                    .unwrap_or(false);

                let mut status_str = "RETRY";
                let mut failure_category: Option<&str> = None;
                let mut error_summary: Option<String> = None;

                if completion_signal.blocked {
                    status_str = "BLOCKED";
                    failure_category = Some("blocked");
                    error_summary = completion_signal.blocked_reason.clone();
                } else if completion_signal.claimed_complete && validation_passed {
                    status_str = "SUCCESS";
                } else if completion_signal.claimed_complete && !validation_passed {
                    false_completion_streak = false_completion_streak.saturating_add(1);
                    failure_category = Some("validation_failed");
                    error_summary = Some("validation_failed".to_string());
                    if false_completion_streak >= 2 || iteration >= policy.max_iterations_per_mt {
                        status_str = "ESCALATE";
                    } else {
                        status_str = "RETRY";
                    }
                } else if iteration >= policy.max_iterations_per_mt {
                    status_str = "ESCALATE";
                    failure_category = Some("max_iterations");
                }

                if !completion_signal.claimed_complete {
                    false_completion_streak = 0;
                }

                let outcome_enum = match status_str {
                    "SUCCESS" => IterationOutcome::Success,
                    "ESCALATE" => IterationOutcome::Escalate,
                    "BLOCKED" => IterationOutcome::Blocked,
                    _ => IterationOutcome::Retry,
                };

                progress.current_state.total_iterations += 1;
                progress.updated_at = Utc::now();

                write_bytes_atomic(
                    &repo_root,
                    &repo_root.join(&response_rel),
                    response.text.as_bytes(),
                )?;
                if let Some(out) = &validation_outcome {
                    write_json_atomic(&repo_root, &repo_root.join(&validation_rel), out)?;
                }

                let output_artifact_ref = artifact_handle_for_rel(&output_rel);
                write_json_atomic(
                    &repo_root,
                    &repo_root.join(&output_rel),
                    &json!({
                        "step_id": step_id,
                        "idempotency_key": idempo,
                        "prompt_hash": prompt_hash.clone(),
                        "context_snapshot_id": context_snapshot_id.to_string(),
                        "context_snapshot_hash": context_snapshot_hash.clone(),
                        "context_snapshot_ref": context_snapshot_ref.clone(),
                        "context_files_ref": context_files_ref.clone(),
                        "prompt_envelope_hashes": {
                            "stable_prefix_hash": stable_prefix_hash.clone(),
                            "variable_suffix_hash": variable_suffix_hash.clone(),
                            "full_prompt_hash": prompt_hash.clone(),
                        },
                        "prompt_snapshot_ref": prompt_snapshot_ref.clone(),
                        "output_snapshot_ref": output_snapshot_ref.clone(),
                        "validation_ref": if validation_outcome.is_some() { Some(validation_ref.clone()) } else { None },
                    }),
                )?;

                if let Some(step) = run_ledger.steps.last_mut() {
                    step.status = LedgerStepStatus::Completed;
                    step.completed_at = Some(completed_ts);
                    step.output_artifact_ref = Some(output_artifact_ref.clone());
                    step.error = None;
                }

                progress.micro_tasks[mt_progress_index]
                    .iterations
                    .push(IterationRecord {
                        iteration,
                        model_id: model_id.clone(),
                        lora_id: lora_id.clone(),
                        escalation_level,
                        started_at: started_ts,
                        completed_at: completed_ts,
                        duration_ms: response.latency_ms,
                        tokens_prompt: response.usage.prompt_tokens,
                        tokens_completion: response.usage.completion_tokens,
                        claimed_complete: completion_signal.claimed_complete,
                        validation_passed: validation_outcome.as_ref().map(|v| v.passed),
                        validation_evidence_ref: validation_outcome
                            .as_ref()
                            .map(|v| v.evidence_artifact_ref.clone()),
                        outcome: outcome_enum,
                        error_summary: error_summary.clone(),
                        context_snapshot_id,
                    });
                if let Some(out) = &validation_outcome {
                    progress.micro_tasks[mt_progress_index]
                        .evidence_refs
                        .push(out.evidence_artifact_ref.clone());
                    progress.micro_tasks[mt_progress_index]
                        .evidence_refs
                        .push(validation_ref.clone());
                }
                progress.micro_tasks[mt_progress_index]
                    .evidence_refs
                    .push(output_artifact_ref.clone());

                refresh_aggregate_stats(&mut progress);
                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskIterationComplete,
                    "FR-EVT-MT-003",
                    "micro_task_iteration_complete",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "wp_id": inputs.wp_id,
                        "mt_id": mt.mt_id,
                        "iteration": iteration,
                        "model": { "base": model_id.clone(), "lora": lora_id.clone(), "lora_version": null, "quantization": null, "context_window": state.llm_client.profile().max_context_tokens },
                        "execution": { "tokens_prompt": response.usage.prompt_tokens, "tokens_completion": response.usage.completion_tokens, "duration_ms": response.latency_ms, "escalation_level": escalation_level },
                        "outcome": { "claimed_complete": completion_signal.claimed_complete, "validation_passed": validation_outcome.as_ref().map(|v| v.passed), "status": status_str, "failure_category": failure_category, "error_summary": error_summary.clone() },
                        "context_snapshot_id": context_snapshot_id.to_string(),
                        "context_snapshot_hash": context_snapshot_hash.clone(),
                        "context_snapshot_ref": serde_json::to_value(&context_snapshot_ref).unwrap_or(Value::Null),
                        "context_files_ref": serde_json::to_value(&context_files_ref).unwrap_or(Value::Null),
                        "prompt_envelope_hashes": {
                            "stable_prefix_hash": stable_prefix_hash.clone(),
                            "variable_suffix_hash": variable_suffix_hash.clone(),
                            "full_prompt_hash": prompt_hash.clone(),
                        },
                        "evidence_artifact_ref": serde_json::to_value(&output_artifact_ref).unwrap_or(Value::Null),
                    }),
                )
                .await;

                if status_str == "SUCCESS" {
                    let gate_type = GOV_GATE_TYPE_MICRO_TASK_VALIDATION;
                    let target_ref =
                        micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                    let decision_evidence_refs = Some(vec![
                        validation_ref.canonical_id(),
                        validation_evidence_ref.canonical_id(),
                    ]);

                    let (gov_decision, gov_decision_artifact) =
                        create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            gate_type,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Approve,
                            1.0,
                            "mt_validation_passed",
                            decision_evidence_refs,
                            Some(model_id.clone()),
                            None,
                        )
                        .await?;

                    if automation_level.requires_human_approval() {
                        emit_gov_human_intervention_requested(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &gov_decision,
                            Some(vec![gov_decision_artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Paused;
                        progress.current_state.active_mt = Some(mt_id.clone());
                        progress.current_state.pending_gov_gate = Some(PendingGovGate {
                            decision_id: gov_decision.decision_id.clone(),
                            gate_type: gov_decision.gate_type.clone(),
                            target_ref: gov_decision.target_ref.clone(),
                            mt_id: mt.mt_id.clone(),
                            final_iteration: iteration,
                            final_model_level: escalation_level,
                        });
                        progress.updated_at = Utc::now();
                        run_ledger.resume_reason =
                            Some(format!("human_gate:{}", gov_decision.decision_id));
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::AwaitingUser,
                            status_reason: "paused_human_gate".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "human_approval_required",
                                "mt_id": mt.mt_id,
                                "decision_id": gov_decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: None,
                        });
                    }

                    let (_auto_signature, auto_signature_artifact) =
                        create_auto_signature_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &gov_decision,
                            model_id.as_str(),
                        )
                        .await?;

                    if policy.enable_distillation {
                        let pending = std::mem::take(
                            &mut progress.micro_tasks[mt_progress_index]
                                .pending_distillation_candidates,
                        );
                        if !pending.is_empty() {
                            let teacher_iterations = progress.micro_tasks[mt_progress_index]
                                .iterations
                                .iter()
                                .filter(|r| r.escalation_level == escalation_level)
                                .count()
                                as u32;
                            let teacher_success = DistillationAttempt {
                                model_id: model_id.clone(),
                                lora_id: lora_id.clone(),
                                lora_version: None,
                                prompt_snapshot_ref: prompt_snapshot_ref.clone(),
                                output_snapshot_ref: output_snapshot_ref.clone(),
                                outcome: "VALIDATION_PASSED".to_string(),
                                iterations: teacher_iterations,
                            };

                            for candidate in pending {
                                let skill_log_entry_id = candidate.skill_log_entry_id.clone();
                                let data_trust_score = candidate.data_trust_score;
                                let distillation_eligible = candidate.distillation_eligible;
                                let task_type_tags = candidate.task_type_tags.clone();

                                let candidate_rel = job_dir_rel
                                    .join("distillation_candidates")
                                    .join(format!("{}_{}.json", mt.mt_id, skill_log_entry_id));
                                let candidate_abs = repo_root.join(&candidate_rel);

                                write_json_atomic(
                                    &repo_root,
                                    &candidate_abs,
                                    &json!({
                                        "schema_version": "1.0",
                                        "skill_log_entry_id": skill_log_entry_id,
                                        "mt_id": mt.mt_id,
                                        "wp_id": inputs.wp_id,
                                        "student_attempt": candidate.student_attempt,
                                        "teacher_success": teacher_success.clone(),
                                        "task_type_tags": task_type_tags,
                                        "contributing_factors": candidate.contributing_factors,
                                        "data_trust_score": data_trust_score,
                                        "distillation_eligible": distillation_eligible,
                                    }),
                                )?;

                                let candidate_ref = artifact_handle_for_rel(&candidate_rel);
                                progress.micro_tasks[mt_progress_index]
                                    .evidence_refs
                                    .push(candidate_ref.clone());
                                progress.micro_tasks[mt_progress_index].distillation_candidate =
                                    Some(DistillationInfo {
                                        eligible: true,
                                        skill_log_entry_id: Some(skill_log_entry_id),
                                        candidate_ref: Some(candidate_ref.clone()),
                                        task_type_tags: mt.task_tags.clone(),
                                        data_trust_score: Some(data_trust_score),
                                        distillation_eligible: Some(distillation_eligible),
                                    });

                                record_micro_task_event(
                                    state,
                                    FlightRecorderEventType::MicroTaskDistillationCandidate,
                                    "FR-EVT-MT-015",
                                    "micro_task_distillation_candidate",
                                    trace_id,
                                    job.job_id,
                                    workflow_run_id,
                                    json!({
                                        "wp_id": inputs.wp_id,
                                        "mt_id": mt.mt_id,
                                        "candidate_ref": serde_json::to_value(&candidate_ref).unwrap_or(Value::Null),
                                    }),
                                )
                                .await;
                            }
                        }
                    }

                    if escalation_level > 0
                        && !matches!(policy.drop_back_strategy, DropBackStrategy::Never)
                    {
                        record_micro_task_event(
                            state,
                            FlightRecorderEventType::MicroTaskDropBack,
                            "FR-EVT-MT-014",
                            "micro_task_drop_back",
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            json!({ "wp_id": inputs.wp_id, "from_level": escalation_level, "to_level": 0 }),
                        )
                        .await;

                        progress.current_state.total_drop_backs =
                            progress.current_state.total_drop_backs.saturating_add(1);
                        progress.current_state.active_model_level = 0;
                    }

                    progress.micro_tasks[mt_progress_index].status = MTStatus::Completed;
                    progress.micro_tasks[mt_progress_index].final_iteration = Some(iteration);
                    progress.micro_tasks[mt_progress_index].final_model_level =
                        Some(escalation_level);
                    progress.current_state.active_mt = None;
                    progress.updated_at = Utc::now();
                    refresh_aggregate_stats(&mut progress);
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskComplete,
                        "FR-EVT-MT-004",
                        "micro_task_complete",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({ "wp_id": inputs.wp_id, "mt_id": mt.mt_id }),
                    )
                    .await;

                    emit_gov_decision_applied(
                        state,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        inputs.wp_id.as_str(),
                        Some(mt.mt_id.as_str()),
                        automation_level,
                        &gov_decision,
                        Some(vec![
                            gov_decision_artifact.canonical_id(),
                            auto_signature_artifact.canonical_id(),
                        ]),
                    )
                    .await?;
                    break;
                }

                if status_str == "RETRY" {
                    iteration = iteration.saturating_add(1);
                    continue;
                }

                if completion_signal.blocked {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskBlocked,
                        "FR-EVT-MT-017",
                        "micro_task_blocked",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "mt_id": mt.mt_id,
                            "reason": completion_signal.blocked_reason.clone().unwrap_or_else(|| "blocked".to_string()),
                        }),
                    )
                    .await;
                }

                let from_level = escalation_level;
                let to_level = escalation_level.saturating_add(1);
                let (to_model, to_lora, to_is_cloud, to_is_hard_gate) =
                    match policy.escalation_chain.get(to_level as usize) {
                        Some(next) => (
                            next.model_id.clone(),
                            next.lora_id.clone(),
                            next.is_cloud,
                            next.is_hard_gate || next.model_id == "HARD_GATE",
                        ),
                        None => ("HARD_GATE".to_string(), None, false, true),
                    };

                let escalation_reason = if completion_signal.blocked {
                    completion_signal
                        .blocked_reason
                        .clone()
                        .unwrap_or_else(|| "blocked".to_string())
                } else if completion_signal.claimed_complete && !validation_passed {
                    "validation_failed".to_string()
                } else {
                    "max_iterations_per_mt".to_string()
                };
                let escalation_failure_category = if completion_signal.blocked {
                    "blocked"
                } else if completion_signal.claimed_complete && !validation_passed {
                    "validation_failed"
                } else {
                    "max_iterations"
                };

                let escalation_record_rel = job_dir_rel.join("escalations").join(format!(
                    "{}_from-{}_to-{}_{}.json",
                    mt.mt_id,
                    from_level,
                    to_level,
                    Uuid::new_v4()
                ));
                let escalation_record_abs = repo_root.join(&escalation_record_rel);
                write_json_atomic(
                    &repo_root,
                    &escalation_record_abs,
                    &json!({
                        "schema_version": "1.0",
                        "wp_id": inputs.wp_id,
                        "job_id": job.job_id.to_string(),
                        "mt_id": mt.mt_id,
                        "from_level": from_level,
                        "to_level": to_level,
                        "from_model": model_id.clone(),
                        "from_lora": lora_id.clone(),
                        "to_model": to_model.clone(),
                        "to_lora": to_lora.clone(),
                        "reason": escalation_reason,
                        "failure_category": escalation_failure_category,
                        "last_step_id": step_id,
                        "last_idempotency_key": idempo,
                        "last_output_artifact_ref": output_artifact_ref.clone(),
                    }),
                )?;
                let escalation_record_ref = artifact_handle_for_rel(&escalation_record_rel);
                progress.micro_tasks[mt_progress_index].escalation_record_ref =
                    Some(escalation_record_ref.clone());

                let iterations_at_previous_level = progress.micro_tasks[mt_progress_index]
                    .iterations
                    .iter()
                    .filter(|r| r.escalation_level == from_level)
                    .count() as u32;

                record_micro_task_event(
                    state,
                    FlightRecorderEventType::MicroTaskEscalated,
                    "FR-EVT-MT-005",
                    "micro_task_escalated",
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "wp_id": inputs.wp_id,
                        "mt_id": mt.mt_id,
                        "from_model": model_id.clone(),
                        "from_lora": lora_id.clone(),
                        "from_level": from_level,
                        "to_model": to_model.clone(),
                        "to_lora": to_lora.clone(),
                        "to_level": to_level,
                        "reason": escalation_reason,
                        "failure_category": escalation_failure_category,
                        "iterations_at_previous_level": iterations_at_previous_level,
                        "escalation_record_ref": serde_json::to_value(&escalation_record_ref).unwrap_or(Value::Null),
                    }),
                )
                .await;

                if policy.enable_distillation {
                    let student_outcome = if completion_signal.blocked {
                        "ERROR"
                    } else if completion_signal.claimed_complete && !validation_passed {
                        "VALIDATION_FAILED"
                    } else {
                        "INCOMPLETE"
                    };

                    let pending = PendingDistillationCandidate {
                        skill_log_entry_id: Uuid::new_v4().to_string(),
                        student_attempt: DistillationAttempt {
                            model_id: model_id.clone(),
                            lora_id: lora_id.clone(),
                            lora_version: None,
                            prompt_snapshot_ref: prompt_snapshot_ref.clone(),
                            output_snapshot_ref: output_snapshot_ref.clone(),
                            outcome: student_outcome.to_string(),
                            iterations: iterations_at_previous_level,
                        },
                        task_type_tags: mt.task_tags.clone(),
                        contributing_factors: vec![escalation_failure_category.to_string()],
                        data_trust_score: 0.8,
                        distillation_eligible: true,
                    };

                    progress.micro_tasks[mt_progress_index]
                        .pending_distillation_candidates
                        .push(pending.clone());
                }

                progress.current_state.total_escalations =
                    progress.current_state.total_escalations.saturating_add(1);
                progress.current_state.active_model_level = to_level;
                progress.updated_at = Utc::now();
                refresh_aggregate_stats(&mut progress);
                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                if to_is_cloud && !policy.cloud_escalation_allowed {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "reason": "cloud_escalation_disallowed",
                            "mt_id": mt.mt_id,
                            "from_level": from_level,
                            "to_level": to_level,
                        }),
                    )
                    .await;

                    let denied_request_id = Uuid::new_v4().to_string();
                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::CloudEscalationDenied,
                            FlightRecorderActor::System,
                            trace_id,
                            json!({
                                "type": "cloud_escalation_denied",
                                "request_id": denied_request_id,
                                "reason": "cloud_escalation_disallowed",
                                "requested_model_id": to_model.clone(),
                                "wp_id": inputs.wp_id,
                                "mt_id": mt.mt_id,
                                "outcome": "denied",
                            }),
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_CLOUD_ESCALATION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Reject,
                            1.0,
                            "locked_cloud_escalation_denied",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.current_state.active_model_level = from_level;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "cloud_escalation_disallowed",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "cloud_escalation_disallowed",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                if to_is_hard_gate {
                    record_micro_task_event(
                        state,
                        FlightRecorderEventType::MicroTaskHardGate,
                        "FR-EVT-MT-006",
                        "micro_task_hard_gate",
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_id": mt.mt_id,
                            "from_level": from_level,
                            "to_level": to_level,
                        }),
                    )
                    .await;

                    if automation_level.is_locked() {
                        let target_ref =
                            micro_task_target_ref(inputs.wp_id.as_str(), mt.mt_id.as_str());
                        let (decision, artifact) = create_gov_decision_and_emit_created(
                            state,
                            &runtime_paths,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            GOV_GATE_TYPE_HUMAN_INTERVENTION,
                            target_ref.as_str(),
                            GovernanceDecisionOutcome::Defer,
                            1.0,
                            "locked_fail_closed:hard_gate",
                            None,
                            None,
                            None,
                        )
                        .await?;

                        emit_gov_decision_applied(
                            state,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            Some(mt.mt_id.as_str()),
                            automation_level,
                            &decision,
                            Some(vec![artifact.canonical_id()]),
                        )
                        .await?;

                        progress.status = ProgressStatus::Failed;
                        progress.current_state.active_model_level = from_level;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "locked_fail_closed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": "escalation_exhausted",
                                "mt_id": mt.mt_id,
                                "decision_id": decision.decision_id,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                            })),
                            error_message: Some("LOCKED fail-closed".to_string()),
                        });
                    }

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    return Ok(RunJobOutcome {
                        state: JobState::AwaitingUser,
                        status_reason: "paused_hard_gate".to_string(),
                        output: Some(json!({
                            "wp_id": inputs.wp_id,
                            "reason": "escalation_exhausted",
                            "mt_definitions_ref": mt_definitions_ref,
                            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                        })),
                        error_message: None,
                    });
                }

                if to_model != model_id {
                    let swap_policy = policy.model_swap_policy();
                    let request_id = deterministic_uuid_for_str(&format!(
                        "{}:{}:{}->{}",
                        job.job_id, mt.mt_id, from_level, to_level
                    ))
                    .to_string();

                    let swap_dir_rel = job_dir_rel.join("model_swap");
                    let context_compile_rel =
                        swap_dir_rel.join(format!("context_compile_{}.json", &request_id));
                    let context_compile_ref = artifact_handle_for_rel(&context_compile_rel);

                    let progress_snapshot_rel =
                        swap_dir_rel.join(format!("progress_snapshot_{}.json", &request_id));
                    let run_ledger_snapshot_rel =
                        swap_dir_rel.join(format!("run_ledger_snapshot_{}.json", &request_id));
                    write_json_atomic(
                        &repo_root,
                        &repo_root.join(&progress_snapshot_rel),
                        &progress,
                    )?;
                    write_json_atomic(
                        &repo_root,
                        &repo_root.join(&run_ledger_snapshot_rel),
                        &run_ledger,
                    )?;

                    let (max_vram_mb, max_ram_mb) =
                        model_swap_min_budgets_mb_for_model_id(to_model.as_str()).unwrap_or((0, 0));

                    let mut swap_request = ModelSwapRequestV0_4 {
                        schema_version: MODEL_SWAP_SCHEMA_VERSION_V0_4.to_string(),
                        request_id,
                        current_model_id: model_id.clone(),
                        target_model_id: to_model.clone(),
                        role: ModelSwapRole::Worker,
                        priority: ModelSwapPriority::Normal,
                        reason: "escalation".to_string(),
                        swap_strategy: ModelSwapStrategy::UnloadReload,
                        state_persist_refs: vec![
                            rel_path_string(&progress_snapshot_rel),
                            rel_path_string(&run_ledger_snapshot_rel),
                            output_artifact_ref.canonical_id(),
                            context_snapshot_ref.canonical_id(),
                        ],
                        state_hash: "0".repeat(64),
                        context_compile_ref: context_compile_ref.canonical_id(),
                        max_vram_mb,
                        max_ram_mb,
                        timeout_ms: swap_policy.swap_timeout_ms,
                        requester: ModelSwapRequesterV0_4 {
                            subsystem: ModelSwapRequesterSubsystem::MtExecutor,
                            job_id: Some(job.job_id.to_string()),
                            wp_id: Some(inputs.wp_id.clone()),
                            mt_id: Some(mt.mt_id.clone()),
                        },
                        metadata: None,
                    };

                    let (state_path, state_hash) =
                        persist_model_swap_state_v0_4(&repo_root, job.job_id, &swap_request)?;
                    swap_request.state_hash = state_hash;
                    verify_model_swap_state_hash_v0_4(
                        &repo_root,
                        &state_path,
                        &swap_request.state_hash,
                    )?;
                    persist_model_swap_request_v0_4(&repo_root, job.job_id, &swap_request)?;

                    record_model_swap_event_v0_4(
                        state,
                        FlightRecorderEventType::ModelSwapRequested,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        inputs.wp_id.as_str(),
                        mt.mt_id.as_str(),
                        &swap_request,
                        "model_swap_requested",
                        None,
                        None,
                    )
                    .await;

                    progress.current_state.total_model_swaps =
                        progress.current_state.total_model_swaps.saturating_add(1);
                    progress.updated_at = Utc::now();
                    write_json_atomic(&repo_root, &progress_abs, &progress)?;
                    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                    let swap_limit_exceeded = swap_policy.max_swaps_per_job == 0
                        || progress.current_state.total_model_swaps > swap_policy.max_swaps_per_job;
                    if !swap_policy.allow_swaps || swap_limit_exceeded {
                        let error_summary = if !swap_policy.allow_swaps {
                            "swap_disallowed_by_policy"
                        } else {
                            "swap_limit_exceeded"
                        };

                        record_model_swap_event_v0_4(
                            state,
                            FlightRecorderEventType::ModelSwapFailed,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            mt.mt_id.as_str(),
                            &swap_request,
                            "model_swap_failed",
                            Some("failure"),
                            Some(error_summary),
                        )
                        .await;

                        if matches!(
                            swap_policy.fallback_strategy,
                            ModelSwapFallbackStrategy::ContinueWithCurrent
                        ) {
                            record_model_swap_event_v0_4(
                                state,
                                FlightRecorderEventType::ModelSwapRollback,
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                inputs.wp_id.as_str(),
                                mt.mt_id.as_str(),
                                &swap_request,
                                "model_swap_rollback",
                                Some("rollback"),
                                Some(error_summary),
                            )
                            .await;

                            progress.current_state.active_model_level = from_level;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&repo_root, &progress_abs, &progress)?;
                            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                            false_completion_streak = 0;
                            iteration = 1;
                            continue;
                        }

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        let msg = format!("model swap failed: {error_summary}");
                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "model_swap_failed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": error_summary,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                "model_swap_request_id": swap_request.request_id,
                            })),
                            error_message: Some(msg),
                        });
                    }

                    let budgets_ok = match model_swap_min_budgets_mb_for_model_id(
                        swap_request.target_model_id.as_str(),
                    ) {
                        Some((min_vram_mb, min_ram_mb)) => {
                            swap_request.max_vram_mb > 0
                                && swap_request.max_ram_mb > 0
                                && swap_request.max_vram_mb >= min_vram_mb
                                && swap_request.max_ram_mb >= min_ram_mb
                        }
                        None => false,
                    };

                    if !budgets_ok {
                        let error_summary = "budget_exceeded";

                        record_model_swap_event_v0_4(
                            state,
                            FlightRecorderEventType::ModelSwapFailed,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            mt.mt_id.as_str(),
                            &swap_request,
                            "model_swap_failed",
                            Some("failure"),
                            Some(error_summary),
                        )
                        .await;

                        if matches!(
                            swap_policy.fallback_strategy,
                            ModelSwapFallbackStrategy::ContinueWithCurrent
                        ) {
                            record_model_swap_event_v0_4(
                                state,
                                FlightRecorderEventType::ModelSwapRollback,
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                inputs.wp_id.as_str(),
                                mt.mt_id.as_str(),
                                &swap_request,
                                "model_swap_rollback",
                                Some("rollback"),
                                Some(error_summary),
                            )
                            .await;

                            progress.current_state.active_model_level = from_level;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&repo_root, &progress_abs, &progress)?;
                            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                            false_completion_streak = 0;
                            iteration = 1;
                            continue;
                        }

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        let msg = format!("model swap failed: {error_summary}");
                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "model_swap_failed".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": error_summary,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                "model_swap_request_id": swap_request.request_id,
                            })),
                            error_message: Some(msg),
                        });
                    }

                    if swap_request.timeout_ms == 0 {
                        let error_summary = "swap_timeout";

                        record_model_swap_event_v0_4(
                            state,
                            FlightRecorderEventType::ModelSwapTimeout,
                            trace_id,
                            job.job_id,
                            workflow_run_id,
                            inputs.wp_id.as_str(),
                            mt.mt_id.as_str(),
                            &swap_request,
                            "model_swap_timeout",
                            Some("timeout"),
                            Some(error_summary),
                        )
                        .await;

                        if matches!(
                            swap_policy.fallback_strategy,
                            ModelSwapFallbackStrategy::ContinueWithCurrent
                        ) {
                            record_model_swap_event_v0_4(
                                state,
                                FlightRecorderEventType::ModelSwapRollback,
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                inputs.wp_id.as_str(),
                                mt.mt_id.as_str(),
                                &swap_request,
                                "model_swap_rollback",
                                Some("rollback"),
                                Some(error_summary),
                            )
                            .await;

                            progress.current_state.active_model_level = from_level;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&repo_root, &progress_abs, &progress)?;
                            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                            false_completion_streak = 0;
                            iteration = 1;
                            continue;
                        }

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&repo_root, &progress_abs, &progress)?;
                        write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                        let msg = "model swap timeout".to_string();
                        return Ok(RunJobOutcome {
                            state: JobState::Failed,
                            status_reason: "model_swap_timeout".to_string(),
                            output: Some(json!({
                                "wp_id": inputs.wp_id,
                                "reason": error_summary,
                                "mt_definitions_ref": mt_definitions_ref,
                                "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                "model_swap_request_id": swap_request.request_id,
                            })),
                            error_message: Some(msg),
                        });
                    }

                    match tokio::time::timeout(
                        std::time::Duration::from_millis(swap_request.timeout_ms),
                        state.llm_client.swap_model(swap_request.clone()),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            pending_model_swap = Some(PendingModelSwapV0_4 {
                                request: swap_request.clone(),
                                context_compile_rel: context_compile_rel.clone(),
                            });
                        }
                        Ok(Err(err)) => {
                            let error_summary = format!("runtime_failure: {err}");

                            record_model_swap_event_v0_4(
                                state,
                                FlightRecorderEventType::ModelSwapFailed,
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                inputs.wp_id.as_str(),
                                mt.mt_id.as_str(),
                                &swap_request,
                                "model_swap_failed",
                                Some("failure"),
                                Some(error_summary.as_str()),
                            )
                            .await;

                            if matches!(
                                swap_policy.fallback_strategy,
                                ModelSwapFallbackStrategy::ContinueWithCurrent
                            ) {
                                record_model_swap_event_v0_4(
                                    state,
                                    FlightRecorderEventType::ModelSwapRollback,
                                    trace_id,
                                    job.job_id,
                                    workflow_run_id,
                                    inputs.wp_id.as_str(),
                                    mt.mt_id.as_str(),
                                    &swap_request,
                                    "model_swap_rollback",
                                    Some("rollback"),
                                    Some(error_summary.as_str()),
                                )
                                .await;

                                progress.current_state.active_model_level = from_level;
                                progress.updated_at = Utc::now();
                                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                                false_completion_streak = 0;
                                iteration = 1;
                                continue;
                            }

                            progress.status = ProgressStatus::Failed;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&repo_root, &progress_abs, &progress)?;
                            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                            let msg = format!("model swap failed: {error_summary}");
                            return Ok(RunJobOutcome {
                                state: JobState::Failed,
                                status_reason: "model_swap_failed".to_string(),
                                output: Some(json!({
                                    "wp_id": inputs.wp_id,
                                    "reason": error_summary,
                                    "mt_definitions_ref": mt_definitions_ref,
                                    "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                    "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                    "model_swap_request_id": swap_request.request_id,
                                })),
                                error_message: Some(msg),
                            });
                        }
                        Err(_) => {
                            let error_summary = "swap_timeout";

                            record_model_swap_event_v0_4(
                                state,
                                FlightRecorderEventType::ModelSwapTimeout,
                                trace_id,
                                job.job_id,
                                workflow_run_id,
                                inputs.wp_id.as_str(),
                                mt.mt_id.as_str(),
                                &swap_request,
                                "model_swap_timeout",
                                Some("timeout"),
                                Some(error_summary),
                            )
                            .await;

                            if matches!(
                                swap_policy.fallback_strategy,
                                ModelSwapFallbackStrategy::ContinueWithCurrent
                            ) {
                                record_model_swap_event_v0_4(
                                    state,
                                    FlightRecorderEventType::ModelSwapRollback,
                                    trace_id,
                                    job.job_id,
                                    workflow_run_id,
                                    inputs.wp_id.as_str(),
                                    mt.mt_id.as_str(),
                                    &swap_request,
                                    "model_swap_rollback",
                                    Some("rollback"),
                                    Some(error_summary),
                                )
                                .await;

                                progress.current_state.active_model_level = from_level;
                                progress.updated_at = Utc::now();
                                write_json_atomic(&repo_root, &progress_abs, &progress)?;
                                write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                                false_completion_streak = 0;
                                iteration = 1;
                                continue;
                            }

                            progress.status = ProgressStatus::Failed;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&repo_root, &progress_abs, &progress)?;
                            write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

                            let msg = "model swap timeout".to_string();
                            return Ok(RunJobOutcome {
                                state: JobState::Failed,
                                status_reason: "model_swap_timeout".to_string(),
                                output: Some(json!({
                                    "wp_id": inputs.wp_id,
                                    "reason": error_summary,
                                    "mt_definitions_ref": mt_definitions_ref,
                                    "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
                                    "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
                                    "model_swap_request_id": swap_request.request_id,
                                })),
                                error_message: Some(msg),
                            });
                        }
                    }

                    // NOTE: ModelSwapCompleted is emitted after a fresh post-swap context compile
                    // (right before the first inference on the target model).
                }

                false_completion_streak = 0;
                escalation_level = to_level;
                progress.current_state.active_model_level = escalation_level;
                iteration = 1;
                continue;
            }
        }
    }

    progress.status = ProgressStatus::Completed;
    progress.completed_at = Some(Utc::now());
    progress.updated_at = Utc::now();
    refresh_aggregate_stats(&mut progress);
    write_json_atomic(&repo_root, &progress_abs, &progress)?;
    write_json_atomic(&repo_root, &run_ledger_abs, &run_ledger)?;

    record_micro_task_event(
        state,
        FlightRecorderEventType::MicroTaskLoopCompleted,
        "FR-EVT-MT-009",
        "micro_task_loop_completed",
        trace_id,
        job.job_id,
        workflow_run_id,
        json!({ "wp_id": inputs.wp_id }),
    )
    .await;

    Ok(RunJobOutcome {
        state: JobState::Completed,
        status_reason: "completed".to_string(),
        output: Some(json!({
            "wp_id": inputs.wp_id,
            "mt_definitions_ref": mt_definitions_ref,
            "progress_artifact_ref": artifact_handle_for_rel(&progress_rel),
            "run_ledger_ref": artifact_handle_for_rel(&run_ledger_rel),
        })),
        error_message: None,
    })
}

async fn execute_terminal_job(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
) -> Result<Value, WorkflowError> {
    let inputs = parse_inputs(job.job_inputs.as_ref());

    let program = inputs
        .get("program")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkflowError::Terminal("program is required".into()))?;
    let args: Vec<String> = match inputs.get("args").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };
    let timeout_ms = inputs.get("timeout_ms").and_then(|v| v.as_u64());
    let max_output_bytes = inputs.get("max_output_bytes").and_then(|v| v.as_u64());
    let cwd = inputs
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let mut env_overrides = HashMap::new();
    if let Some(env) = inputs.get("env_overrides").and_then(|v| v.as_object()) {
        for (k, v) in env.iter() {
            let value = if v.is_null() {
                None
            } else {
                v.as_str().map(|s| s.to_string())
            };
            env_overrides.insert(k.to_string(), value);
        }
    }

    let job_id = Some(job.job_id.to_string());
    let session_type =
        crate::terminal::session::TerminalSessionType::derive(None, job_id.as_ref(), None);
    let request = TerminalRequest {
        command: program.to_string(),
        args: args.clone(),
        cwd,
        mode: TerminalMode::NonInteractive,
        timeout_ms,
        max_output_bytes,
        env_overrides,
        capture_stdout: true,
        capture_stderr: true,
        stdin_chunks: Vec::new(),
        idempotency_key: None,
        job_context: JobContext {
            job_id: job_id.clone(),
            model_id: None,
            session_id: None,
            capability_profile_id: Some(job.capability_profile_id.clone()),
            capability_id: Some("terminal.exec".to_string()),
            wsids: Vec::new(),
        },
        granted_capabilities: Vec::new(),
        requested_capability: Some("terminal.exec".to_string()),
        session_type,
        human_consent_obtained: false,
    };

    let cfg = TerminalConfig::with_defaults();
    let guards: Vec<Box<dyn TerminalGuard>> = vec![Box::new(DefaultTerminalGuard)];
    let redactor = PatternRedactor;

    let output = TerminalService::run_command(
        request,
        &cfg,
        state.capability_registry.as_ref(),
        state.flight_recorder.as_ref(),
        trace_id,
        &redactor,
        &guards,
    )
    .await
    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let payload = json!({
        "job_kind": job.job_kind.as_str(),
        "program": program,
        "args": args,
        "status_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
        "timed_out": output.timed_out,
        "truncated_bytes": output.truncated_bytes,
        "duration_ms": output.duration_ms
    });

    if output.exit_code != 0 || output.timed_out {
        return Err(WorkflowError::Terminal(format!(
            "command exited with code {}{}",
            output.exit_code,
            if output.timed_out { " (timed out)" } else { "" }
        )));
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{
        sqlite::SqliteDatabase, AccessMode, Database, JobKind, JobMetrics, JobState, SafetyMode,
    };
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
        let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
        sqlite.run_migrations().await?;

        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        Ok(AppState {
            storage: sqlite.into_arc(),
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(CapabilityRegistry::new()),
        })
    }

    fn terminal_command() -> (String, Vec<String>) {
        if cfg!(target_os = "windows") {
            (
                "cmd".to_string(),
                vec!["/C".into(), "echo".into(), "hello".into()],
            )
        } else {
            ("echo".to_string(), vec!["hello".into()])
        }
    }

    #[test]
    fn model_swap_request_v0_4_rejects_wrong_schema_version() {
        let req = ModelSwapRequestV0_4 {
            schema_version: MODEL_SWAP_SCHEMA_VERSION_V0_4.to_string(),
            request_id: "req-1".to_string(),
            current_model_id: "qwen2.5-coder:7b".to_string(),
            target_model_id: "qwen2.5-coder:13b".to_string(),
            role: ModelSwapRole::Orchestrator,
            priority: ModelSwapPriority::Normal,
            reason: "escalation".to_string(),
            swap_strategy: ModelSwapStrategy::UnloadReload,
            state_persist_refs: vec!["artifact:state.json".to_string()],
            state_hash: "0".repeat(64),
            context_compile_ref: "artifact:ace_context_compile.json".to_string(),
            max_vram_mb: 4096,
            max_ram_mb: 32768,
            timeout_ms: 120_000,
            requester: ModelSwapRequesterV0_4 {
                subsystem: ModelSwapRequesterSubsystem::MtExecutor,
                job_id: None,
                wp_id: Some("WP-1-Model-Swap-Protocol-v1".to_string()),
                mt_id: Some("MT-002".to_string()),
            },
            metadata: None,
        };
        assert!(req.validate().is_ok());

        let mut bad = req;
        bad.schema_version = "hsk.model_swap@0.3".to_string();
        assert!(bad.validate().is_err());
    }

    #[test]
    fn model_swap_state_persist_and_verify_v0_4_roundtrip() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = tempfile::tempdir()?;
        let job_id = Uuid::new_v4();

        let state_ref_rel = PathBuf::from("data").join("test_state.json");
        let state_ref_abs = tmp.path().join(&state_ref_rel);
        if let Some(parent) = state_ref_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&state_ref_abs, b"hello")?;

        let request = ModelSwapRequestV0_4 {
            schema_version: MODEL_SWAP_SCHEMA_VERSION_V0_4.to_string(),
            request_id: "req-2".to_string(),
            current_model_id: "qwen2.5-coder:7b".to_string(),
            target_model_id: "qwen2.5-coder:13b".to_string(),
            role: ModelSwapRole::Worker,
            priority: ModelSwapPriority::High,
            reason: "context_overflow".to_string(),
            swap_strategy: ModelSwapStrategy::UnloadReload,
            state_persist_refs: vec![rel_path_string(&state_ref_rel)],
            state_hash: "0".repeat(64),
            context_compile_ref: "artifact:ace_compile_job.json".to_string(),
            max_vram_mb: 4096,
            max_ram_mb: 32768,
            timeout_ms: 120_000,
            requester: ModelSwapRequesterV0_4 {
                subsystem: ModelSwapRequesterSubsystem::MtExecutor,
                job_id: Some(job_id.to_string()),
                wp_id: Some("WP-1-Model-Swap-Protocol-v1".to_string()),
                mt_id: Some("MT-002".to_string()),
            },
            metadata: None,
        };

        let (state_path, state_hash) = persist_model_swap_state_v0_4(tmp.path(), job_id, &request)?;
        assert!(state_path.exists(), "state_path should exist");
        assert!(
            is_sha256_hex_lowercase(&state_hash),
            "hash must be lowercase sha256"
        );

        let raw = fs::read(&state_path)?;
        let json: Value = serde_json::from_slice(&raw)?;
        let map = json.as_object().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "expected payload to be a JSON object",
            )
        })?;
        assert!(
            !map.contains_key("state_hash"),
            "state must not embed state_hash"
        );

        verify_model_swap_state_hash_v0_4(tmp.path(), &state_path, &state_hash)?;
        let bad_hash = "1".repeat(64);
        assert!(
            verify_model_swap_state_hash_v0_4(tmp.path(), &state_path, &bad_hash).is_err(),
            "expected hash mismatch to fail"
        );

        Ok(())
    }

    #[tokio::test]
    async fn job_fails_when_missing_required_capability() -> Result<(), Box<dyn std::error::Error>>
    {
        let state = setup_state().await?;
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocSummarize,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "missing_profile".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "doc_id": "doc-1" })),
            })
            .await?;
        let job_id = job.job_id;

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;

        assert!(matches!(updated_job.state, JobState::Failed));
        Ok(())
    }

    #[tokio::test]
    async fn terminal_job_enforces_capability() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::TerminalExec,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "Analyst".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "program": "printf", "args": ["hello"] })),
            })
            .await?;
        let job_id = job.job_id;

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;

        assert!(matches!(updated_job.state, JobState::Failed));
        Ok(())
    }

    #[tokio::test]
    async fn terminal_job_runs_when_authorized() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let (program, args) = if cfg!(target_os = "windows") {
            ("cmd", vec!["/C", "echo", "hello"])
        } else {
            ("echo", vec!["hello"])
        };

        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::TerminalExec,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "Coder".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "program": program, "args": args })),
            })
            .await?;
        let job_id = job.job_id;

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_ok(), "terminal job failed: {:?}", result.err());

        let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;

        assert!(matches!(updated_job.state, JobState::Completed));

        let outputs = updated_job
            .job_outputs
            .as_ref()
            .ok_or("missing job outputs")?;
        let stdout = outputs.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        assert!(stdout.trim().contains("hello"));

        Ok(())
    }

    #[tokio::test]
    async fn workflow_persists_node_history_and_outputs() -> Result<(), Box<dyn std::error::Error>>
    {
        let state = setup_state().await?;
        let (program, args) = terminal_command();

        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::TerminalExec,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "Coder".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "program": program, "args": args })),
            })
            .await?;

        let workflow_run = start_workflow_for_job(&state, job).await?;
        let nodes = state
            .storage
            .list_workflow_node_executions(workflow_run.id)
            .await?;

        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert!(matches!(node.status, JobState::Completed));
        assert!(node.output_payload.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_poisoning_trap() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocSummarize,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "Analyst".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "force_prompt_injection": true })),
            })
            .await?;

        let result = start_workflow_for_job(&state, job.clone()).await;
        assert!(result.is_err(), "expected poisoning trap to trigger");

        let updated_job = state.storage.get_ai_job(&job.job_id.to_string()).await?;
        assert!(matches!(updated_job.state, JobState::Poisoned));
        assert!(updated_job.status_reason.contains("poisoned"));
        assert!(updated_job.job_outputs.is_none());

        if let Some(workflow_run_id) = updated_job.workflow_run_id {
            let nodes = state
                .storage
                .list_workflow_node_executions(workflow_run_id)
                .await?;
            assert!(
                nodes.iter().all(|n| matches!(n.status, JobState::Poisoned)),
                "expected all nodes poisoned"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_mark_stalled_workflows() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        // 1. Create a job and a "Running" workflow run with an old heartbeat
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocSummarize,
                protocol_id: "p1".into(),
                profile_id: "default".into(),
                capability_profile_id: "Analyst".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "running".into(),
                metrics: JobMetrics::zero(),
                job_inputs: None,
            })
            .await?;

        let old_heartbeat = Utc::now() - chrono::Duration::seconds(60);
        let run = state
            .storage
            .create_workflow_run(job.job_id, JobState::Running, Some(old_heartbeat))
            .await?;

        // 2. Run recovery
        let recovered = mark_stalled_workflows(&state, 30, true).await?;

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].id, run.id);

        // 3. Verify states
        let updated_job = state.storage.get_ai_job(&job.job_id.to_string()).await?;
        assert!(matches!(updated_job.state, JobState::Stalled));

        // 4. Verify Flight Recorder event
        let events = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter::default())
            .await?;
        let event = events
            .iter()
            .find(|e| e.event_type == FlightRecorderEventType::WorkflowRecovery)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Recovery event not found in Flight Recorder",
                )
            })?;
        assert_eq!(event.actor, FlightRecorderActor::System);
        assert_eq!(event.workflow_id, Some(run.id.to_string()));

        let payload: FrEvtWorkflowRecovery =
            serde_json::from_value(event.payload.clone()).map_err(|e| e.to_string())?;
        assert_eq!(payload.workflow_run_id, run.id.to_string());
        assert_eq!(payload.job_id, Some(job.job_id.to_string()));
        assert_eq!(payload.from_state, JobState::Running.as_str());
        assert_eq!(payload.to_state, "stalled");
        assert_eq!(payload.threshold_secs, 30);
        assert_eq!(payload.last_heartbeat_ts, run.last_heartbeat.to_rfc3339());

        Ok(())
    }

    #[tokio::test]
    async fn test_startup_recovery_blocks_job_acceptance() -> Result<(), Box<dyn std::error::Error>>
    {
        reset_startup_recovery_gate_for_test();
        let state = setup_state().await?;
        enable_startup_recovery_gate();

        let create_future = crate::jobs::create_job(
            &state,
            JobKind::DocSummarize,
            "protocol-default",
            "Analyst",
            None,
            Vec::new(),
        );
        tokio::pin!(create_future);

        let timeout_result =
            tokio::time::timeout(std::time::Duration::from_millis(50), &mut create_future).await;
        assert!(
            timeout_result.is_err(),
            "create_job completed before startup recovery gate released"
        );

        mark_startup_recovery_complete();
        let job_result = create_future.await;
        reset_startup_recovery_gate_for_test();
        let job = job_result?;
        assert!(matches!(job.state, JobState::Queued));

        Ok(())
    }

    /// Integration test: run_job MUST invoke validate_trace and reject budget violations.
    /// This test creates a trace that EXCEEDS max_total_evidence_tokens budget.
    /// RetrievalBudgetGuard.validate_trace will detect this and return AceError::BudgetExceeded.
    #[tokio::test]
    async fn run_job_rejects_budget_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::{NewBlock, NewDocument, NewWorkspace, WriteContext};

        let state = setup_state().await?;
        let ctx = WriteContext::human(None);

        // Create workspace
        let workspace = state
            .storage
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: "test-ws".into(),
                },
            )
            .await?;

        // Create document
        let document = state
            .storage
            .create_document(
                &ctx,
                NewDocument {
                    workspace_id: workspace.id.clone(),
                    title: "Large Doc".into(),
                },
            )
            .await?;

        // Create block with large content: 20000 chars = ~5000 tokens (exceeds 4000 budget)
        let large_content = "X".repeat(20000);
        let _block = state
            .storage
            .create_block(
                &ctx,
                NewBlock {
                    id: None,
                    document_id: document.id.clone(),
                    kind: "paragraph".into(),
                    sequence: 1,
                    raw_content: large_content,
                    display_content: None,
                    derived_content: None,
                    sensitivity: None,
                    exportable: None,
                },
            )
            .await?;

        // Create DocSummarize job targeting the large document
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocSummarize,
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "Analyst".to_string(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({ "doc_id": document.id })),
            })
            .await?;
        let job_id = job.job_id;

        // Run workflow - should fail with budget exceeded
        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected budget exceeded error");

        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.contains("BudgetExceeded") || err_str.contains("max_total_evidence_tokens"),
            "expected BudgetExceeded error, got: {}",
            err_str
        );

        // Verify job is poisoned
        let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
        assert!(
            matches!(updated_job.state, JobState::Poisoned),
            "expected job state Poisoned, got {:?}",
            updated_job.state
        );

        Ok(())
    }
}
