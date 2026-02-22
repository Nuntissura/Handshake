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
    llm::{CompletionRequest, LlmError},
    mex::runtime::{AdapterError as MexAdapterError, EngineAdapter, ShellEngineAdapter},
    mex::{
        BudgetGate, BudgetSpec, CapabilityGate, DetGate, DeterminismLevel, EngineError,
        EngineResult, EngineStatus, EvidencePolicy, GatePipeline, IntegrityGate, MexRegistry,
        MexRuntime, OutputSpec, PlannedOperation, ProvenanceGate, ProvenanceRecord, SchemaGate,
        POE_SCHEMA_VERSION,
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
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    collections::HashMap,
    collections::{HashSet, VecDeque},
    fs,
    io::{Read, Write},
    path::Path,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU8, Ordering},
        Arc, Mutex,
    },
};
use thiserror::Error;
use tokio::sync::Notify;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
    task::JoinSet,
};
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

#[derive(Debug, Clone)]
struct MdCancelEntry {
    sender: watch::Sender<bool>,
    refs: usize,
}

static MD_CANCEL_REGISTRY: Lazy<Mutex<HashMap<String, MdCancelEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn md_request_cancel(cancel_key: &str) -> bool {
    let mut registry = match MD_CANCEL_REGISTRY.lock() {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    let sender = if let Some(entry) = registry.get(cancel_key) {
        entry.sender.clone()
    } else {
        let (sender, _receiver) = watch::channel(false);
        registry.insert(
            cancel_key.to_string(),
            MdCancelEntry {
                sender: sender.clone(),
                refs: 0,
            },
        );
        sender
    };

    sender.send(true).is_ok()
}

fn md_register_cancel_receiver(
    cancel_key: String,
) -> Result<(watch::Receiver<bool>, MdCancelGuard), WorkflowError> {
    let mut registry = MD_CANCEL_REGISTRY.lock().map_err(|_| {
        WorkflowError::Terminal("media_downloader cancel registry lock poisoned".into())
    })?;

    let receiver = if let Some(entry) = registry.get_mut(&cancel_key) {
        entry.refs = entry.refs.saturating_add(1);
        entry.sender.subscribe()
    } else {
        let (sender, receiver) = watch::channel(false);
        registry.insert(cancel_key.clone(), MdCancelEntry { sender, refs: 1 });
        receiver
    };

    Ok((receiver, MdCancelGuard { key: cancel_key }))
}

struct MdCancelGuard {
    key: String,
}

impl Drop for MdCancelGuard {
    fn drop(&mut self) {
        let Ok(mut registry) = MD_CANCEL_REGISTRY.lock() else {
            return;
        };
        let Some(entry) = registry.get_mut(&self.key) else {
            return;
        };
        entry.refs = entry.refs.saturating_sub(1);
        if entry.refs == 0 {
            registry.remove(&self.key);
        }
    }
}

#[derive(Debug, Clone)]
struct MediaDownloaderSignals {
    pause_tx: watch::Sender<bool>,
    retry_tx: watch::Sender<u64>,
}

static MEDIA_DOWNLOADER_SIGNAL_REGISTRY: Lazy<Mutex<HashMap<Uuid, MediaDownloaderSignals>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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

    state
        .storage
        .update_ai_job_status(JobStatusUpdate {
            job_id: job.job_id,
            state: JobState::Running,
            error_message: None,
            status_reason: "running".to_string(),
            metrics: None,
            workflow_run_id: Some(workflow_run.id),
            trace_id: Some(trace_id),
            job_outputs: None,
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

    let is_background_job = matches!(
        job.job_kind,
        JobKind::WorkflowRun | JobKind::MediaDownloader
    ) && (job.protocol_id == MD_BATCH_PROTOCOL_ID_V0
        || job.protocol_id == MD_CONTROL_PROTOCOL_ID_V0
        || job.protocol_id == MD_COOKIE_IMPORT_PROTOCOL_ID_V0);

    if is_background_job {
        let state_clone = state.clone();
        let job_clone = job.clone();
        let workflow_run_clone = workflow_run.clone();
        let node_exec_clone = node_exec.clone();

        tokio::spawn(async move {
            let _ = run_and_finalize_workflow_job(
                state_clone,
                job_clone,
                workflow_run_clone,
                node_exec_clone,
                trace_id,
            )
            .await;
        });

        return Ok(workflow_run);
    }

    run_and_finalize_workflow_job(state.clone(), job, workflow_run, node_exec, trace_id).await
}

async fn run_and_finalize_workflow_job(
    state: AppState,
    job: AiJob,
    workflow_run: WorkflowRun,
    node_exec: crate::storage::WorkflowNodeExecution,
    trace_id: Uuid,
) -> Result<WorkflowRun, WorkflowError> {
    let result = run_job(&state, &job, workflow_run.id, trace_id).await;

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
                &state,
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
        &state,
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
    } else if matches!(job.job_kind, JobKind::MediaDownloader) {
        if job.protocol_id == MD_BATCH_PROTOCOL_ID_V0 {
            return run_media_downloader_job(state, job, workflow_run_id, trace_id).await;
        }
        if job.protocol_id == MD_CONTROL_PROTOCOL_ID_V0 {
            return run_media_downloader_control_job(state, job, workflow_run_id, trace_id).await;
        }
        if job.protocol_id == MD_COOKIE_IMPORT_PROTOCOL_ID_V0 {
            return run_media_downloader_cookie_import_job(state, job, workflow_run_id, trace_id)
                .await;
        }

        return Ok(RunJobOutcome {
            state: JobState::Poisoned,
            status_reason: "invalid_protocol_id".to_string(),
            output: None,
            error_message: Some("invalid protocol_id for media_downloader".to_string()),
        });
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

        if job.protocol_id == MD_BATCH_PROTOCOL_ID_V0 {
            return run_media_downloader_job(state, job, workflow_run_id, trace_id).await;
        }
        if job.protocol_id == MD_CONTROL_PROTOCOL_ID_V0 {
            return run_media_downloader_control_job(state, job, workflow_run_id, trace_id).await;
        }
        if job.protocol_id == MD_COOKIE_IMPORT_PROTOCOL_ID_V0 {
            return run_media_downloader_cookie_import_job(state, job, workflow_run_id, trace_id)
                .await;
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

struct MediaDownloaderEngineAdapter {
    artifact_root: PathBuf,
}

impl MediaDownloaderEngineAdapter {
    fn new(artifact_root: PathBuf) -> Self {
        Self { artifact_root }
    }

    fn rel_path_string(rel_path: &std::path::Path) -> String {
        rel_path.to_string_lossy().replace('\\', "/")
    }

    fn artifact_handle_for_rel(&self, rel_path: &std::path::Path) -> ArtifactHandle {
        ArtifactHandle::new(Uuid::new_v4(), Self::rel_path_string(rel_path))
    }

    fn artifact_dir_rel(op: &PlannedOperation) -> PathBuf {
        PathBuf::from("data")
            .join("mex_media_downloader")
            .join("ops")
            .join(op.op_id.to_string())
    }

    async fn collect_output_to_file<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
        mut reader: Option<R>,
        mut out: tokio::fs::File,
        max_output_bytes: u64,
    ) -> Result<u64, std::io::Error> {
        let mut reader = match reader.take() {
            Some(value) => value,
            None => return Ok(0),
        };

        let mut written: u64 = 0;
        let mut truncated: u64 = 0;
        let mut buf = [0u8; 4096];
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let available = max_output_bytes.saturating_sub(written);
            if available > 0 {
                let to_take = std::cmp::min(n as u64, available) as usize;
                out.write_all(&buf[..to_take]).await?;
                written = written.saturating_add(to_take as u64);
                if n as u64 > available {
                    truncated = truncated.saturating_add(n as u64 - available);
                }
            } else {
                truncated = truncated.saturating_add(n as u64);
            }
        }

        out.flush().await?;
        Ok(truncated)
    }

    async fn wait_for_cancel(mut receivers: Vec<watch::Receiver<bool>>) {
        if receivers.is_empty() {
            std::future::pending::<()>().await;
            return;
        }

        loop {
            if receivers.iter().any(|rx| *rx.borrow()) {
                return;
            }

            if receivers.len() == 1 {
                let _ = receivers[0].changed().await;
                continue;
            }

            let (first, rest) = receivers.split_at_mut(1);
            tokio::select! {
                _ = first[0].changed() => {},
                _ = rest[0].changed() => {},
            }
        }
    }

    #[cfg(windows)]
    async fn kill_process_tree(pid: u32) {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
    }

    async fn kill_child_best_effort(child: &mut tokio::process::Child) {
        if let Some(pid) = child.id() {
            #[cfg(windows)]
            {
                Self::kill_process_tree(pid).await;
                return;
            }
        }

        let _ = child.start_kill();
        let _ = child.kill().await;
    }
}

#[async_trait]
impl EngineAdapter for MediaDownloaderEngineAdapter {
    async fn invoke(&self, op: &PlannedOperation) -> Result<EngineResult, MexAdapterError> {
        if op.engine_id != MD_TOOL_ENGINE_ID {
            return Err(MexAdapterError::Engine(format!(
                "unsupported engine_id for adapter: {}",
                op.engine_id
            )));
        }

        let params = op
            .params
            .as_object()
            .ok_or_else(|| MexAdapterError::Engine("params must be an object".to_string()))?;
        let tool_path = params
            .get("tool_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MexAdapterError::Engine("missing params.tool_path".to_string()))?
            .to_string();
        let cwd = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .or(op.budget.wall_time_ms)
            .unwrap_or(600_000);
        let max_output_bytes = op
            .output_spec
            .max_bytes
            .or(op.budget.output_bytes)
            .unwrap_or(1_500_000);

        let args_value = params
            .get("args")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MexAdapterError::Engine("missing params.args".to_string()))?;
        let mut args: Vec<String> = Vec::with_capacity(args_value.len());
        for v in args_value {
            let s = v.as_str().ok_or_else(|| {
                MexAdapterError::Engine("params.args must be strings".to_string())
            })?;
            args.push(s.to_string());
        }

        let mut env_overrides = std::collections::HashMap::new();
        if let Some(env) = params.get("env").and_then(|v| v.as_object()) {
            for (k, v) in env {
                if let Some(value) = v.as_str() {
                    env_overrides.insert(k.to_string(), Some(value.to_string()));
                }
            }
        }

        let cancel_keys: Vec<String> = params
            .get("cancel_keys")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .filter(|s| !s.trim().is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let mut cancel_receivers: Vec<watch::Receiver<bool>> = Vec::new();
        let mut _cancel_guards: Vec<MdCancelGuard> = Vec::new();
        for key in cancel_keys {
            if let Ok((rx, guard)) = md_register_cancel_receiver(key) {
                cancel_receivers.push(rx);
                _cancel_guards.push(guard);
            }
        }

        let started_at = Utc::now();
        // WAIVER [CX-573E]: Instant::now() for observability (engine exec timing metrics).
        let start = std::time::Instant::now();

        if std::path::Path::new(&cwd).is_absolute() {
            return Err(MexAdapterError::Engine(
                "cwd must be workspace-relative".to_string(),
            ));
        }

        let artifact_dir_rel = Self::artifact_dir_rel(op);
        let artifact_dir_abs = self.artifact_root.join(&artifact_dir_rel);
        std::fs::create_dir_all(&artifact_dir_abs).map_err(|e| {
            MexAdapterError::Engine(format!(
                "failed to create {}: {e}",
                artifact_dir_abs.display()
            ))
        })?;

        let stdout_rel = artifact_dir_rel.join("stdout.txt");
        let stderr_rel = artifact_dir_rel.join("stderr.txt");
        let output_rel = artifact_dir_rel.join("terminal_output.json");

        let stdout_abs = self.artifact_root.join(&stdout_rel);
        let stderr_abs = self.artifact_root.join(&stderr_rel);
        let output_abs = self.artifact_root.join(&output_rel);

        let stdout_file = tokio::fs::File::create(&stdout_abs).await.map_err(|e| {
            MexAdapterError::Engine(format!("failed to create {}: {e}", stdout_abs.display()))
        })?;
        let stderr_file = tokio::fs::File::create(&stderr_abs).await.map_err(|e| {
            MexAdapterError::Engine(format!("failed to create {}: {e}", stderr_abs.display()))
        })?;

        let mut command = tokio::process::Command::new(&tool_path);
        command.args(&args);
        command.current_dir(self.artifact_root.join(&cwd));
        command.kill_on_drop(true);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        for (k, v) in env_overrides.iter() {
            if let Some(value) = v {
                command.env(k, value);
            } else {
                command.env_remove(k);
            }
        }

        let mut child = command.spawn().map_err(|e| {
            MexAdapterError::Engine(format!("failed to spawn tool {tool_path}: {e}"))
        })?;

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();
        let stdout_task = tokio::spawn(Self::collect_output_to_file(
            stdout_handle,
            stdout_file,
            max_output_bytes,
        ));
        let stderr_task = tokio::spawn(Self::collect_output_to_file(
            stderr_handle,
            stderr_file,
            max_output_bytes,
        ));

        let timeout = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms));
        tokio::pin!(timeout);
        let cancel_wait = Self::wait_for_cancel(cancel_receivers);
        tokio::pin!(cancel_wait);

        let (exit_status, timed_out, cancelled) = tokio::select! {
            status = child.wait() => {
                let status = status.map_err(|e| MexAdapterError::Engine(e.to_string()))?;
                (status, false, false)
            }
            _ = &mut timeout => {
                Self::kill_child_best_effort(&mut child).await;
                let status = child.wait().await.map_err(|e| MexAdapterError::Engine(e.to_string()))?;
                (status, true, false)
            }
            _ = &mut cancel_wait => {
                Self::kill_child_best_effort(&mut child).await;
                let status = child.wait().await.map_err(|e| MexAdapterError::Engine(e.to_string()))?;
                (status, false, true)
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout_trunc = stdout_task
            .await
            .map_err(|e| MexAdapterError::Engine(e.to_string()))?
            .map_err(|e| MexAdapterError::Engine(e.to_string()))?;
        let stderr_trunc = stderr_task
            .await
            .map_err(|e| MexAdapterError::Engine(e.to_string()))?
            .map_err(|e| MexAdapterError::Engine(e.to_string()))?;
        let truncated_bytes = stdout_trunc.saturating_add(stderr_trunc);

        let exit_code = exit_status.code().unwrap_or(-1);
        let ended_at = Utc::now();

        let terminal_output_payload = json!({
            "command": tool_path,
            "cwd": cwd,
            "exit_code": exit_code,
            "timed_out": timed_out,
            "cancelled": cancelled,
            "truncated_bytes": truncated_bytes,
            "duration_ms": duration_ms,
            "stdout_ref": Self::rel_path_string(&stdout_rel),
            "stderr_ref": Self::rel_path_string(&stderr_rel),
        });
        let output_bytes = serde_json::to_vec_pretty(&terminal_output_payload)
            .map_err(|e| MexAdapterError::Engine(e.to_string()))?;
        std::fs::write(&output_abs, output_bytes).map_err(|e| {
            MexAdapterError::Engine(format!("failed to write {}: {e}", output_abs.display()))
        })?;

        let stdout_handle = self.artifact_handle_for_rel(&stdout_rel);
        let stderr_handle = self.artifact_handle_for_rel(&stderr_rel);
        let output_handle = self.artifact_handle_for_rel(&output_rel);

        let status = if exit_code == 0 && !timed_out && !cancelled {
            EngineStatus::Succeeded
        } else {
            EngineStatus::Failed
        };

        let mut errors = Vec::new();
        if status != EngineStatus::Succeeded {
            errors.push(EngineError {
                code: "ENGINE_MEDIA_DOWNLOADER_NONZERO_EXIT".to_string(),
                message: format!("tool exited with code {}", exit_code),
                details_ref: Some(output_handle.clone()),
            });
        }

        let provenance = ProvenanceRecord {
            engine_id: op.engine_id.clone(),
            engine_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            implementation: Some("media_downloader_adapter".to_string()),
            determinism: op.determinism,
            config_hash: None,
            inputs: op.inputs.clone(),
            outputs: vec![
                output_handle.clone(),
                stdout_handle.clone(),
                stderr_handle.clone(),
            ],
            capabilities_granted: op.capabilities_requested.clone(),
            environment: None,
        };

        Ok(EngineResult {
            op_id: op.op_id,
            status,
            started_at,
            ended_at,
            outputs: vec![output_handle.clone()],
            evidence: vec![stdout_handle, stderr_handle, output_handle.clone()],
            provenance,
            errors,
            logs_ref: None,
        })
    }
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
    )
    .with_adapter(
        MD_TOOL_ENGINE_ID,
        Arc::new(MediaDownloaderEngineAdapter::new(repo_root.to_path_buf())),
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

                write_bytes_atomic(&repo_root, &repo_root.join(&prompt_rel), prompt.as_bytes())?;

                let response = state
                    .llm_client
                    .completion(CompletionRequest::new(
                        trace_id,
                        prompt.clone(),
                        model_id.clone(),
                    ))
                    .await?;
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

// =============================================================================
// Media Downloader (Spec §10.14)
// =============================================================================

const MD_BATCH_PROTOCOL_ID_V0: &str = "hsk.media_downloader.batch.v0";
const MD_CONTROL_PROTOCOL_ID_V0: &str = "hsk.media_downloader.control.v0";
const MD_COOKIE_IMPORT_PROTOCOL_ID_V0: &str = "hsk.media_downloader.cookie_import.v0";
const MD_BATCH_SCHEMA_V0: &str = "hsk.media_downloader.batch@v0";
const MD_CONTROL_SCHEMA_V0: &str = "hsk.media_downloader.control@v0";
const MD_COOKIE_IMPORT_SCHEMA_V0: &str = "hsk.media_downloader.cookie_import@v0";
const MD_RESULT_SCHEMA_V0: &str = "hsk.media_downloader.result@v0";
const MD_COOKIE_IMPORT_RESULT_SCHEMA_V0: &str = "hsk.media_downloader.cookie_import.result@v0";

const MD_SESSIONS_REGISTRY_SCHEMA_V0: &str = "hsk.media_downloader.sessions@v0";
const MD_SESSIONS_REGISTRY_FILENAME: &str = "media_downloader_sessions.json";

const MD_TOOL_ENGINE_ID: &str = "engine.media_downloader";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum MdSourceKindV0 {
    Youtube,
    Instagram,
    Forumcrawler,
    Videodownloader,
}

impl MdSourceKindV0 {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Youtube => "youtube",
            Self::Instagram => "instagram",
            Self::Forumcrawler => "forumcrawler",
            Self::Videodownloader => "videodownloader",
        }
    }

    fn output_subdir(&self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdControlsV0 {
    #[serde(default)]
    concurrency: Option<u8>,
    #[serde(default)]
    max_pages: Option<u32>,
    #[serde(default)]
    allowlist_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdAuthV0 {
    mode: String,
    #[serde(default)]
    stage_session_id: Option<String>,
    #[serde(default)]
    cookie_jar_artifact_ref: Option<ArtifactHandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdBatchRequestV0 {
    schema_version: String,
    source_kind: MdSourceKindV0,
    sources: Vec<String>,
    #[serde(default)]
    auth: Option<MdAuthV0>,
    #[serde(default)]
    controls: Option<MdControlsV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdControlRequestV0 {
    schema_version: String,
    #[serde(default)]
    target_job_id: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdCookieImportRequestV0 {
    schema_version: String,
    /// Absolute path to a user-provided cookie export (JSON) or Netscape cookies.txt file.
    /// IMPORTANT: raw cookie values MUST NOT be in job_inputs (secrets).
    source_path: String,
    /// If set, updates `.handshake/gov/media_downloader_sessions.json` to bind the resulting
    /// cookie jar artifact to this stage session.
    #[serde(default)]
    stage_session_id: Option<String>,
    /// If true, best-effort removes `source_path` after ingest (use for temp files).
    #[serde(default)]
    cleanup_source: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdCookieImportResultV0 {
    schema_version: String,
    cookie_jar_artifact_ref: ArtifactHandle,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_session_id: Option<String>,
    updated_session: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdPlanItemV0 {
    item_id: String,
    source_kind: MdSourceKindV0,
    url_canonical: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdPlanV0 {
    stable_item_total: usize,
    items: Vec<MdPlanItemV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdProgressV0 {
    state: String,
    item_done: usize,
    item_total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes_downloaded: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes_total: Option<u64>,
    concurrency: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdItemResultV0 {
    item_id: String,
    status: String,
    #[serde(default)]
    artifact_handles: Vec<ArtifactHandle>,
    #[serde(default)]
    materialized_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdJobOutputV0 {
    schema_version: String,
    plan: MdPlanV0,
    progress: MdProgressV0,
    items: Vec<MdItemResultV0>,
    #[serde(default)]
    export_records: Vec<crate::governance_pack::ExportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdSessionsRegistryV0 {
    schema_version: String,
    #[serde(default)]
    sessions: Vec<MdSessionRecordV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdSessionRecordV0 {
    session_id: String,
    kind: String,
    label: String,
    created_at: String,
    #[serde(default)]
    last_used_at: Option<String>,
    allow_private_network: bool,
    #[serde(default)]
    cookie_jar_artifact_ref: Option<ArtifactHandle>,
}

static MD_FR_EVENT_ID_LAST: Lazy<AtomicI64> = Lazy::new(|| AtomicI64::new(0));

fn md_next_fr_event_id() -> i64 {
    let now = Utc::now().timestamp_micros();
    loop {
        let prev = MD_FR_EVENT_ID_LAST.load(Ordering::SeqCst);
        let next = now.max(prev.saturating_add(1));
        if MD_FR_EVENT_ID_LAST
            .compare_exchange(prev, next, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return next;
        }
    }
}

async fn md_insert_fr_event_best_effort(
    state: &AppState,
    job_id: Uuid,
    workflow_run_id: Uuid,
    event_kind: &str,
    payload: &Value,
) {
    if !event_kind.starts_with("media_downloader.") {
        return;
    }

    let Some(conn) = state.flight_recorder.duckdb_connection() else {
        return;
    };
    let Ok(conn) = conn.lock() else {
        return;
    };

    let event_id = md_next_fr_event_id();
    let ts_utc = Utc::now().to_rfc3339();
    let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());

    let _ = conn.execute(
        r#"
        INSERT INTO fr_events (
            event_id,
            ts_utc,
            session_id,
            task_id,
            job_id,
            workflow_run_id,
            event_kind,
            source,
            level,
            message,
            payload
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        duckdb::params![
            event_id,
            ts_utc,
            Option::<String>::None,
            Option::<String>::None,
            job_id.to_string(),
            workflow_run_id.to_string(),
            event_kind.to_string(),
            "media_downloader".to_string(),
            Option::<String>::None,
            Option::<String>::None,
            payload_str,
        ],
    );
}

async fn md_record_md_system_event(
    state: &AppState,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_run_id: Uuid,
    payload: Value,
) {
    let event_kind = payload
        .get("event_kind")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    md_insert_fr_event_best_effort(state, job_id, workflow_run_id, event_kind, &payload).await;

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            payload,
        )
        .with_job_id(job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;
}

fn md_job_cancel_key(job_id: Uuid) -> String {
    format!("media_downloader:{job_id}:job")
}

fn md_item_cancel_key(job_id: Uuid, item_id: &str) -> String {
    format!("media_downloader:{job_id}:item:{}", item_id.trim())
}

fn md_sanitize_url_string_for_telemetry(raw: &str) -> String {
    let mut clean = raw.trim();
    if let Some((head, _)) = clean.split_once('#') {
        clean = head;
    }
    if let Some((head, _)) = clean.split_once('?') {
        clean = head;
    }
    clean.to_string()
}

fn md_parse_url(raw: &str) -> Result<reqwest::Url, WorkflowError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(WorkflowError::Terminal("empty url".into()));
    }
    let url = reqwest::Url::parse(trimmed).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(WorkflowError::Terminal("url must be http(s)".into())),
    }
    Ok(url)
}

fn md_is_private_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if host.eq_ignore_ascii_case("0.0.0.0") {
        return true;
    }
    if host.eq_ignore_ascii_case("[::1]") || host.eq_ignore_ascii_case("::1") {
        return true;
    }
    // IPv4 literal checks.
    if let Ok(addr) = host.parse::<std::net::Ipv4Addr>() {
        let octets = addr.octets();
        if octets[0] == 0 {
            return true;
        }
        if octets[0] == 10 {
            return true;
        }
        if octets[0] == 127 {
            return true;
        }
        if octets[0] == 192 && octets[1] == 168 {
            return true;
        }
        if octets[0] == 172 && (16..=31).contains(&octets[1]) {
            return true;
        }
        if octets[0] == 169 && octets[1] == 254 {
            return true;
        }
        if octets[0] == 100 && (64..=127).contains(&octets[1]) {
            return true;
        }
    }
    false
}

fn md_validate_url_target(url: &reqwest::Url) -> Result<(), WorkflowError> {
    let host = url
        .host_str()
        .ok_or_else(|| WorkflowError::Terminal("url missing host".into()))?;
    if md_is_private_host(host) {
        return Err(WorkflowError::Terminal(
            "blocked url target (localhost/private network)".into(),
        ));
    }
    Ok(())
}

fn md_item_id_from_url(url: &reqwest::Url) -> String {
    let mut h = Sha256::new();
    h.update(url.as_str().as_bytes());
    let hex = hex::encode(h.finalize());
    hex.chars().take(16).collect()
}

fn md_normalize_allowlist_domains(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for entry in raw {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }

        let host = if trimmed.contains("://") {
            reqwest::Url::parse(trimmed)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_default()
        } else {
            trimmed.split('/').next().unwrap_or_default().to_string()
        };

        let mut host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        if host.is_empty() {
            continue;
        }
        if let Some((h, port)) = host.rsplit_once(':') {
            if !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
                host = h.to_string();
            }
        }
        if host.is_empty() {
            continue;
        }

        out.push(host);
    }

    out.sort();
    out.dedup();
    out
}

fn md_default_concurrency() -> u8 {
    4
}

fn md_concurrency_from_request(req: &MdBatchRequestV0) -> u8 {
    let raw = req
        .controls
        .as_ref()
        .and_then(|c| c.concurrency)
        .unwrap_or(md_default_concurrency());
    raw.clamp(1, 16)
}

fn md_forumcrawler_max_pages(req: &MdBatchRequestV0) -> usize {
    let raw = req
        .controls
        .as_ref()
        .and_then(|c| c.max_pages)
        .unwrap_or(1500);
    let capped = raw.clamp(1, 5000);
    capped as usize
}

#[derive(Debug, Clone)]
struct MdToolsV0 {
    yt_dlp_path: PathBuf,
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

const MD_YTDLP_VERSION: &str = "2026.02.04";
const MD_YTDLP_SHA256_WINDOWS_X64: &str =
    "78a3ac4cd1eeb2bdbf08b17dca64a1c06224971dc8af147e5f01652e9f6f940e";
const MD_YTDLP_SHA256_MACOS_UNIVERSAL: &str =
    "ae42ac3e7612c1d878f9f8384df6a7c54fc63361210709bd77682095653f0cf1";
const MD_YTDLP_SHA256_LINUX_X64: &str =
    "ccdbb1fcd90c51f7848d0f17a1d741fc557c37818a70c4886027aa43bcaa91af";

const MD_FFMPEG_VERSION: &str = "8.0.1";
const MD_FFMPEG_ZIP_SHA256_WINDOWS_X64: &str =
    "e2aaeaa0fdbc4935916bbf14d4bbc25b13f0171cd65487bcdaf5fc555f89dbfa";

fn md_tools_root(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("tools")
        .join("media_downloader")
}

fn sha256_file(path: &Path) -> Result<String, WorkflowError> {
    let mut file = fs::File::open(path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

async fn download_to_path(url: &str, dest_path: &Path) -> Result<String, WorkflowError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
        .send()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(WorkflowError::Terminal(format!(
            "download failed: {url} status={status}"
        )));
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }

    let tmp_path = dest_path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut hasher = Sha256::new();

    let mut resp = resp;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?
    {
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }
    file.flush()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    drop(file);

    if dest_path.exists() {
        fs::remove_file(dest_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }
    fs::rename(&tmp_path, dest_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(not(windows))]
fn ensure_executable(path: &Path) -> Result<(), WorkflowError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?
        .permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms).map_err(|e| WorkflowError::Terminal(e.to_string()))
}

#[cfg(windows)]
async fn extract_ffmpeg_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), WorkflowError> {
    let zip_path = zip_path.to_path_buf();
    let dest_dir = dest_dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
        let file =
            fs::File::open(&zip_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

        let mut extracted = 0usize;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let name = entry.name().to_string();
            let lower = name.to_ascii_lowercase();
            let target_name = if lower.ends_with("/bin/ffmpeg.exe") {
                Some("ffmpeg.exe")
            } else if lower.ends_with("/bin/ffprobe.exe") {
                Some("ffprobe.exe")
            } else {
                None
            };

            let Some(target_name) = target_name else {
                continue;
            };

            fs::create_dir_all(&dest_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let target_path = dest_dir.join(target_name);
            let tmp_path = target_path.with_extension("tmp");
            let mut out =
                fs::File::create(&tmp_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            out.flush()
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            drop(out);
            if target_path.exists() {
                fs::remove_file(&target_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            }
            fs::rename(&tmp_path, &target_path)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            extracted += 1;
        }

        if extracted < 2 {
            return Err(WorkflowError::Terminal(
                "ffmpeg zip did not contain expected binaries".into(),
            ));
        }
        Ok(())
    })
    .await
    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
}

async fn ensure_media_downloader_tools(workspace_root: &Path) -> Result<MdToolsV0, WorkflowError> {
    let tools_root = md_tools_root(workspace_root);
    fs::create_dir_all(&tools_root).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    // yt-dlp
    let ytdlp_dir = tools_root.join("yt-dlp").join(MD_YTDLP_VERSION);
    #[cfg(windows)]
    let ytdlp_path = ytdlp_dir.join("yt-dlp.exe");
    #[cfg(not(windows))]
    let ytdlp_path = ytdlp_dir.join("yt-dlp");

    let (ytdlp_url, ytdlp_sha256_expected) = if cfg!(windows) {
        (
            format!(
                "https://github.com/yt-dlp/yt-dlp/releases/download/{}/yt-dlp.exe",
                MD_YTDLP_VERSION
            ),
            MD_YTDLP_SHA256_WINDOWS_X64,
        )
    } else if cfg!(target_os = "macos") {
        (
            format!(
                "https://github.com/yt-dlp/yt-dlp/releases/download/{}/yt-dlp_macos",
                MD_YTDLP_VERSION
            ),
            MD_YTDLP_SHA256_MACOS_UNIVERSAL,
        )
    } else {
        (
            format!(
                "https://github.com/yt-dlp/yt-dlp/releases/download/{}/yt-dlp_linux",
                MD_YTDLP_VERSION
            ),
            MD_YTDLP_SHA256_LINUX_X64,
        )
    };

    let needs_ytdlp = !ytdlp_path.exists()
        || sha256_file(&ytdlp_path).unwrap_or_default() != ytdlp_sha256_expected;
    if needs_ytdlp {
        let sha = download_to_path(&ytdlp_url, &ytdlp_path).await?;
        if sha != ytdlp_sha256_expected {
            let _ = fs::remove_file(&ytdlp_path);
            return Err(WorkflowError::Terminal(
                "yt-dlp sha256 mismatch after download".into(),
            ));
        }
    }

    #[cfg(not(windows))]
    ensure_executable(&ytdlp_path)?;

    // ffmpeg + ffprobe (managed provisioning for Windows; other platforms rely on packaging).
    let ffmpeg_dir = tools_root.join("ffmpeg").join(MD_FFMPEG_VERSION);
    #[cfg(windows)]
    let ffmpeg_path = ffmpeg_dir.join("ffmpeg.exe");
    #[cfg(windows)]
    let ffprobe_path = ffmpeg_dir.join("ffprobe.exe");

    #[cfg(windows)]
    {
        let zip_path =
            ffmpeg_dir.join(format!("ffmpeg-{}-essentials_build.zip", MD_FFMPEG_VERSION));
        let needs_zip = !zip_path.exists()
            || sha256_file(&zip_path).unwrap_or_default() != MD_FFMPEG_ZIP_SHA256_WINDOWS_X64;
        if needs_zip {
            let url = format!(
                "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-{}-essentials_build.zip",
                MD_FFMPEG_VERSION
            );
            let sha = download_to_path(&url, &zip_path).await?;
            if sha != MD_FFMPEG_ZIP_SHA256_WINDOWS_X64 {
                let _ = fs::remove_file(&zip_path);
                return Err(WorkflowError::Terminal(
                    "ffmpeg zip sha256 mismatch after download".into(),
                ));
            }
        }

        if !ffmpeg_path.exists() || !ffprobe_path.exists() {
            extract_ffmpeg_zip(&zip_path, &ffmpeg_dir).await?;
        }
    }

    #[cfg(not(windows))]
    let ffmpeg_path = PathBuf::from("ffmpeg");
    #[cfg(not(windows))]
    let ffprobe_path = PathBuf::from("ffprobe");

    Ok(MdToolsV0 {
        yt_dlp_path: ytdlp_path,
        ffmpeg_path,
        ffprobe_path,
    })
}

fn md_sessions_registry_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("gov")
        .join(MD_SESSIONS_REGISTRY_FILENAME)
}

fn md_load_sessions_registry(workspace_root: &Path) -> Result<MdSessionsRegistryV0, WorkflowError> {
    let path = md_sessions_registry_path(workspace_root);
    if !path.exists() {
        return Ok(MdSessionsRegistryV0 {
            schema_version: MD_SESSIONS_REGISTRY_SCHEMA_V0.to_string(),
            sessions: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let parsed: MdSessionsRegistryV0 =
        serde_json::from_str(&raw).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    Ok(parsed)
}

fn md_save_sessions_registry(
    workspace_root: &Path,
    registry: &MdSessionsRegistryV0,
) -> Result<(), WorkflowError> {
    let path = md_sessions_registry_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }
    write_json_atomic(workspace_root, &path, registry)
}

fn md_find_cookie_jar_for_stage_session(
    registry: &MdSessionsRegistryV0,
    stage_session_id: &str,
) -> Option<ArtifactHandle> {
    registry
        .sessions
        .iter()
        .find(|s| s.session_id == stage_session_id && s.kind == "stage_session")
        .and_then(|s| s.cookie_jar_artifact_ref.clone())
}

async fn run_media_downloader_control_job(
    _state: &AppState,
    job: &AiJob,
    _workflow_run_id: Uuid,
    _trace_id: Uuid,
) -> Result<RunJobOutcome, WorkflowError> {
    if job.protocol_id != MD_CONTROL_PROTOCOL_ID_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_contract".to_string(),
            output: None,
            error_message: Some("invalid protocol_id for media_downloader_control".to_string()),
        });
    }

    let inputs = parse_inputs(job.job_inputs.as_ref());
    let request: MdControlRequestV0 = match serde_json::from_value(inputs) {
        Ok(req) => req,
        Err(err) => {
            return Ok(RunJobOutcome {
                state: JobState::Failed,
                status_reason: "invalid_job_inputs".to_string(),
                output: None,
                error_message: Some(err.to_string()),
            })
        }
    };

    if request.schema_version != MD_CONTROL_SCHEMA_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("schema_version mismatch".to_string()),
        });
    }

    let target_job_id = request
        .target_job_id
        .or_else(|| {
            job.job_inputs
                .as_ref()
                .and_then(|v| v.get("job_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            job.job_inputs
                .as_ref()
                .and_then(|v| v.get("jobId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    let target_job_id = target_job_id.trim().to_string();
    if target_job_id.is_empty() {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("target_job_id is required".to_string()),
        });
    }
    let target_job_uuid = match Uuid::parse_str(&target_job_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(RunJobOutcome {
                state: JobState::Failed,
                status_reason: "invalid_job_inputs".to_string(),
                output: None,
                error_message: Some("target_job_id must be a UUID".to_string()),
            })
        }
    };

    let action = request.action.unwrap_or_default();
    let action = action.trim().to_string();

    match action.as_str() {
        "pause" => {
            let registry = MEDIA_DOWNLOADER_SIGNAL_REGISTRY.lock().map_err(|_| {
                WorkflowError::Terminal("media_downloader control registry lock poisoned".into())
            })?;
            if let Some(signals) = registry.get(&target_job_uuid) {
                let _ = signals.pause_tx.send(true);
            } else {
                return Ok(RunJobOutcome {
                    state: JobState::Failed,
                    status_reason: "not_found".to_string(),
                    output: None,
                    error_message: Some("target job is not running".to_string()),
                });
            }
        }
        "resume" => {
            let registry = MEDIA_DOWNLOADER_SIGNAL_REGISTRY.lock().map_err(|_| {
                WorkflowError::Terminal("media_downloader control registry lock poisoned".into())
            })?;
            if let Some(signals) = registry.get(&target_job_uuid) {
                let _ = signals.pause_tx.send(false);
            } else {
                return Ok(RunJobOutcome {
                    state: JobState::Failed,
                    status_reason: "not_found".to_string(),
                    output: None,
                    error_message: Some("target job is not running".to_string()),
                });
            }
        }
        "cancel_all" => {
            md_request_cancel(&md_job_cancel_key(target_job_uuid));
        }
        "cancel_one" => {
            let item_id = request.item_id.unwrap_or_default();
            let item_id = item_id.trim();
            if item_id.is_empty() {
                return Ok(RunJobOutcome {
                    state: JobState::Failed,
                    status_reason: "invalid_job_inputs".to_string(),
                    output: None,
                    error_message: Some("item_id is required for cancel_one".to_string()),
                });
            }
            md_request_cancel(&md_item_cancel_key(target_job_uuid, item_id));
        }
        "retry_failed" => {
            let mut registry = MEDIA_DOWNLOADER_SIGNAL_REGISTRY.lock().map_err(|_| {
                WorkflowError::Terminal("media_downloader control registry lock poisoned".into())
            })?;
            if let Some(signals) = registry.get_mut(&target_job_uuid) {
                let current = *signals.retry_tx.borrow();
                let next = current.saturating_add(1);
                let _ = signals.retry_tx.send(next);
            } else {
                return Ok(RunJobOutcome {
                    state: JobState::Failed,
                    status_reason: "not_found".to_string(),
                    output: None,
                    error_message: Some("target job is not running".to_string()),
                });
            }
        }
        _ => {
            return Ok(RunJobOutcome {
                state: JobState::Failed,
                status_reason: "invalid_job_inputs".to_string(),
                output: None,
                error_message: Some("unsupported action".to_string()),
            })
        }
    }

    let output = json!({
        "schema_version": MD_CONTROL_SCHEMA_V0,
        "target_job_id": target_job_id,
        "action": action,
        "ok": true,
    });

    Ok(RunJobOutcome {
        state: JobState::Completed,
        status_reason: "completed".to_string(),
        output: Some(output),
        error_message: None,
    })
}

fn md_trim_utf8_bom(input: &str) -> &str {
    input.strip_prefix('\u{FEFF}').unwrap_or(input)
}

fn md_netscape_cookie_jar_text_or_empty(raw: &str) -> Option<String> {
    let trimmed = md_trim_utf8_bom(raw).trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("netscape http cookie file") {
        return Some(trimmed.to_string());
    }

    // Heuristic: Netscape cookies.txt is tab-delimited with 7 columns.
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 7 {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn md_cookie_field_sanitize(input: &str) -> String {
    input
        .replace('\t', " ")
        .replace('\r', "")
        .replace('\n', "")
        .trim()
        .to_string()
}

fn md_cookie_json_to_netscape(raw_json: &str) -> Result<String, WorkflowError> {
    #[derive(Debug, Clone, Deserialize)]
    struct MdJsonCookieV0 {
        #[serde(default)]
        domain: String,
        #[serde(rename = "hostOnly", default)]
        host_only: bool,
        #[serde(default)]
        path: String,
        #[serde(default)]
        secure: bool,
        #[serde(rename = "httpOnly", default)]
        http_only: bool,
        #[serde(rename = "expirationDate", default)]
        expiration_date: Option<f64>,
        #[serde(default)]
        session: bool,
        #[serde(default)]
        name: String,
        #[serde(default)]
        value: String,
    }

    let parsed: Value =
        serde_json::from_str(raw_json).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let cookies_value: Value = match parsed {
        Value::Array(_) => parsed,
        Value::Object(ref obj) if obj.get("cookies").is_some() => obj
            .get("cookies")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        Value::Object(_) => Value::Array(Vec::new()),
        _ => Value::Array(Vec::new()),
    };

    let cookies: Vec<MdJsonCookieV0> = serde_json::from_value(cookies_value)
        .map_err(|e| WorkflowError::Terminal(format!("invalid cookie json: {e}")))?;

    let mut out = String::new();
    out.push_str("# Netscape HTTP Cookie File\n");
    out.push_str("# This file is generated by Handshake (Media Downloader).\n");
    out.push_str("# Do not share. Contains sensitive session cookies.\n\n");

    for cookie in cookies {
        let mut domain = cookie.domain.trim().trim_end_matches('.').to_string();
        if domain.is_empty() {
            continue;
        }

        // Normalize domain + includeSubdomains field.
        let include_subdomains = if cookie.host_only { "FALSE" } else { "TRUE" };
        if !cookie.host_only && !domain.starts_with('.') {
            domain = format!(".{domain}");
        }
        if cookie.host_only && domain.starts_with('.') {
            domain = domain.trim_start_matches('.').to_string();
        }

        if md_is_private_host(&domain) {
            continue;
        }

        let secure = if cookie.secure { "TRUE" } else { "FALSE" };
        let path = if cookie.path.trim().is_empty() {
            "/".to_string()
        } else {
            cookie.path.trim().to_string()
        };

        let expires = if cookie.session {
            0i64
        } else {
            cookie
                .expiration_date
                .map(|v| v.floor() as i64)
                .unwrap_or(0i64)
        };

        let name = md_cookie_field_sanitize(&cookie.name);
        let value = md_cookie_field_sanitize(&cookie.value);
        if name.is_empty() {
            continue;
        }

        // HttpOnly cookies use a #HttpOnly_ prefix convention.
        let domain_field = if cookie.http_only {
            format!("#HttpOnly_{domain}")
        } else {
            domain
        };

        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            domain_field, include_subdomains, path, secure, expires, name, value
        ));
    }

    Ok(out)
}

async fn run_media_downloader_cookie_import_job(
    state: &AppState,
    job: &AiJob,
    _workflow_run_id: Uuid,
    trace_id: Uuid,
) -> Result<RunJobOutcome, WorkflowError> {
    if job.protocol_id != MD_COOKIE_IMPORT_PROTOCOL_ID_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_contract".to_string(),
            output: None,
            error_message: Some(
                "invalid protocol_id for media_downloader.cookie_import".to_string(),
            ),
        });
    }

    let inputs = parse_inputs(job.job_inputs.as_ref());
    let request: MdCookieImportRequestV0 = match serde_json::from_value(inputs) {
        Ok(req) => req,
        Err(err) => {
            return Ok(RunJobOutcome {
                state: JobState::Failed,
                status_reason: "invalid_job_inputs".to_string(),
                output: None,
                error_message: Some(err.to_string()),
            })
        }
    };

    if request.schema_version != MD_COOKIE_IMPORT_SCHEMA_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("schema_version mismatch".to_string()),
        });
    }

    // Cookie jar artifacts are high sensitivity; enforce secrets.use for import/export handling.
    {
        let cap = "secrets.use";
        let cap_result = state
            .capability_registry
            .profile_can(&job.capability_profile_id, cap);
        match cap_result {
            Ok(true) => log_capability_check(state, job, cap, "allow", trace_id).await,
            Ok(false) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                    cap.to_string(),
                )));
            }
            Err(err) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(err));
            }
        }
    }

    let source_path = PathBuf::from(request.source_path.trim());
    if !source_path.is_absolute() {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("source_path must be absolute".to_string()),
        });
    }
    let meta = fs::metadata(&source_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    if !meta.is_file() {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("source_path must be a file".to_string()),
        });
    }

    let raw_bytes = fs::read(&source_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let raw_text = String::from_utf8_lossy(&raw_bytes).to_string();

    let netscape_text = if let Some(existing) = md_netscape_cookie_jar_text_or_empty(&raw_text) {
        existing
    } else {
        // JSON cookie exports (Chrome/Firefox extensions) are supported.
        md_cookie_json_to_netscape(md_trim_utf8_bom(&raw_text))?
    };

    let workspace_root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let tmp_dir = workspace_root
        .join(".handshake")
        .join("tmp")
        .join("media_downloader")
        .join(job.job_id.to_string())
        .join("cookie_import");
    fs::create_dir_all(&tmp_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let tmp_cookie_path = tmp_dir.join("cookies.txt");
    tokio::fs::write(&tmp_cookie_path, netscape_text.as_bytes())
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let (_manifest, handle) = md_write_file_artifact_from_path(
        &workspace_root,
        crate::storage::artifacts::ArtifactLayer::L3,
        crate::storage::artifacts::ArtifactPayloadKind::File,
        "text/plain".to_string(),
        Some("cookies.txt".to_string()),
        &tmp_cookie_path,
        crate::storage::artifacts::ArtifactClassification::High,
        false,
        Some(30),
        Some(job.job_id),
        Vec::new(),
        Vec::new(),
    )?;

    let _ = fs::remove_file(&tmp_cookie_path);

    let mut updated_session = false;
    let stage_session_id = request.stage_session_id.clone().and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    if let Some(stage_session_id) = stage_session_id.as_deref() {
        let mut registry = md_load_sessions_registry(&workspace_root)?;
        let now = Utc::now().to_rfc3339();
        for session in &mut registry.sessions {
            if session.session_id == stage_session_id && session.kind == "stage_session" {
                session.cookie_jar_artifact_ref = Some(handle.clone());
                session.last_used_at = Some(now.clone());
                updated_session = true;
                break;
            }
        }
        if updated_session {
            md_save_sessions_registry(&workspace_root, &registry)?;
        }
    }

    // Optional cleanup: only delete sources under workspace tmp to avoid deleting user files.
    if request.cleanup_source.unwrap_or(false) {
        let tmp_root = workspace_root.join(".handshake").join("tmp");
        if source_path.starts_with(&tmp_root) {
            let _ = fs::remove_file(&source_path);
        }
    }

    let output = MdCookieImportResultV0 {
        schema_version: MD_COOKIE_IMPORT_RESULT_SCHEMA_V0.to_string(),
        cookie_jar_artifact_ref: handle,
        stage_session_id,
        updated_session,
    };
    let payload = serde_json::to_value(&output).unwrap_or(json!({}));

    state
        .storage
        .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
        .await?;

    Ok(RunJobOutcome {
        state: JobState::Completed,
        status_reason: "completed".to_string(),
        output: Some(payload),
        error_message: None,
    })
}

const MD_OUTPUT_ROOT_DIR_SCHEMA_V0: &str = "hsk.output_root_dir@v0";
const MD_OUTPUT_ROOT_DIR_FILENAME: &str = "output_root_dir.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdOutputRootDirConfigV0 {
    schema_version: String,
    output_root_dir: String,
}

fn md_output_root_dir_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("gov")
        .join(MD_OUTPUT_ROOT_DIR_FILENAME)
}

fn md_default_output_root_dir() -> PathBuf {
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    };

    let mut base = home
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let docs = base.join("Documents");
    if docs.is_dir() {
        base = docs;
    }

    base.join("Handshake_Output")
}

fn md_read_or_init_output_root_dir(workspace_root: &Path) -> Result<PathBuf, WorkflowError> {
    let gov_dir = workspace_root.join(".handshake").join("gov");
    fs::create_dir_all(&gov_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let config_path = md_output_root_dir_config_path(workspace_root);
    if config_path.exists() {
        let raw =
            fs::read_to_string(&config_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        let config: MdOutputRootDirConfigV0 =
            serde_json::from_str(&raw).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        if config.schema_version != MD_OUTPUT_ROOT_DIR_SCHEMA_V0 {
            return Err(WorkflowError::Terminal(
                "output_root_dir.json schema_version mismatch".to_string(),
            ));
        }
        let trimmed = config.output_root_dir.trim();
        if trimmed.is_empty() {
            return Err(WorkflowError::Terminal(
                "output_root_dir is empty".to_string(),
            ));
        }
        let dir = PathBuf::from(trimmed);
        if !dir.is_absolute() {
            return Err(WorkflowError::Terminal(format!(
                "output_root_dir is not absolute: {trimmed}"
            )));
        }
        fs::create_dir_all(&dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        return Ok(dir);
    }

    let default_dir = md_default_output_root_dir();
    if !default_dir.is_absolute() {
        return Err(WorkflowError::Terminal(format!(
            "default output root dir is not absolute: {}",
            default_dir.display()
        )));
    }
    fs::create_dir_all(&default_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let config = MdOutputRootDirConfigV0 {
        schema_version: MD_OUTPUT_ROOT_DIR_SCHEMA_V0.to_string(),
        output_root_dir: default_dir.to_string_lossy().to_string(),
    };
    write_json_atomic(workspace_root, &config_path, &config)?;

    Ok(default_dir)
}

async fn run_media_downloader_job(
    state: &AppState,
    job: &AiJob,
    workflow_run_id: Uuid,
    trace_id: Uuid,
) -> Result<RunJobOutcome, WorkflowError> {
    if job.protocol_id != MD_BATCH_PROTOCOL_ID_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_contract".to_string(),
            output: None,
            error_message: Some("invalid protocol_id for media_downloader".to_string()),
        });
    }

    let inputs = parse_inputs(job.job_inputs.as_ref());
    let req: MdBatchRequestV0 = match serde_json::from_value(inputs) {
        Ok(req) => req,
        Err(err) => {
            return Ok(RunJobOutcome {
                state: JobState::Failed,
                status_reason: "invalid_job_inputs".to_string(),
                output: None,
                error_message: Some(err.to_string()),
            })
        }
    };

    if req.schema_version != MD_BATCH_SCHEMA_V0 {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("schema_version mismatch".to_string()),
        });
    }

    let workspace_root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let concurrency = md_concurrency_from_request(&req);
    let forum_max_pages = md_forumcrawler_max_pages(&req);
    let forum_allowlist_domains = md_normalize_allowlist_domains(
        req.controls
            .as_ref()
            .and_then(|c| c.allowlist_domains.as_deref())
            .unwrap_or(&[]),
    );

    let (pause_tx, mut pause_rx) = watch::channel(false);
    let (retry_tx, mut retry_rx) = watch::channel(0u64);

    {
        let mut registry = MEDIA_DOWNLOADER_SIGNAL_REGISTRY.lock().map_err(|_| {
            WorkflowError::Terminal("media_downloader signal registry lock poisoned".into())
        })?;
        registry.insert(
            job.job_id,
            MediaDownloaderSignals {
                pause_tx: pause_tx.clone(),
                retry_tx: retry_tx.clone(),
            },
        );
    }

    struct RegistryGuard(Uuid);
    impl Drop for RegistryGuard {
        fn drop(&mut self) {
            if let Ok(mut reg) = MEDIA_DOWNLOADER_SIGNAL_REGISTRY.lock() {
                reg.remove(&self.0);
            }
        }
    }
    let _guard = RegistryGuard(job.job_id);

    let job_cancel_key = md_job_cancel_key(job.job_id);
    let (mut job_cancel_rx, _job_cancel_guard) =
        md_register_cancel_receiver(job_cancel_key.clone())?;

    // Validate and normalize sources.
    let mut sources: Vec<reqwest::Url> = Vec::new();
    for raw in &req.sources {
        let url = md_parse_url(raw)?;
        md_validate_url_target(&url)?;
        sources.push(url);
    }
    // Dedupe by canonical string.
    let mut seen = HashSet::new();
    sources.retain(|u| seen.insert(u.as_str().to_string()));

    if sources.is_empty() {
        return Ok(RunJobOutcome {
            state: JobState::Failed,
            status_reason: "invalid_job_inputs".to_string(),
            output: None,
            error_message: Some("sources must be non-empty".to_string()),
        });
    }

    // Resolve auth cookie jar (if any).
    let mut cookie_jar_payload: Option<PathBuf> = None;
    if let Some(auth) = &req.auth {
        match auth.mode.as_str() {
            "none" => {}
            "cookie_jar" => {
                let handle = auth.cookie_jar_artifact_ref.as_ref().ok_or_else(|| {
                    WorkflowError::Terminal("cookie_jar_artifact_ref missing".into())
                })?;
                cookie_jar_payload = Some(workspace_root.join(PathBuf::from(&handle.path)));
            }
            "stage_session" => {
                let stage_session_id = auth
                    .stage_session_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| WorkflowError::Terminal("stage_session_id missing".into()))?;
                let registry = md_load_sessions_registry(&workspace_root)?;
                let handle = md_find_cookie_jar_for_stage_session(&registry, stage_session_id)
                    .ok_or_else(|| {
                        WorkflowError::Terminal("stage_session has no exported cookie jar".into())
                    })?;
                cookie_jar_payload = Some(workspace_root.join(PathBuf::from(&handle.path)));
            }
            _ => {
                return Ok(RunJobOutcome {
                    state: JobState::Failed,
                    status_reason: "invalid_job_inputs".to_string(),
                    output: None,
                    error_message: Some("unsupported auth.mode".to_string()),
                })
            }
        }
    }

    // Enforce net.http for all media_downloader jobs (external fetches + tool provisioning).
    {
        let cap = "net.http";
        let cap_result = state
            .capability_registry
            .profile_can(&job.capability_profile_id, cap);
        match cap_result {
            Ok(true) => log_capability_check(state, job, cap, "allow", trace_id).await,
            Ok(false) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                    cap.to_string(),
                )));
            }
            Err(err) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(err));
            }
        }
    }

    // Enforce secrets.use only when cookie/session auth is used.
    if cookie_jar_payload.is_some() {
        let cap = "secrets.use";
        let cap_result = state
            .capability_registry
            .profile_can(&job.capability_profile_id, cap);
        match cap_result {
            Ok(true) => log_capability_check(state, job, cap, "allow", trace_id).await,
            Ok(false) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                    cap.to_string(),
                )));
            }
            Err(err) => {
                log_capability_check(state, job, cap, "deny", trace_id).await;
                return Err(WorkflowError::Capability(err));
            }
        }
    }

    let output_root_dir = md_read_or_init_output_root_dir(&workspace_root)?;

    let materialize_root = output_root_dir.join("media_downloader");
    fs::create_dir_all(&materialize_root).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let mex_runtime = {
        let repo_root =
            repo_root_from_manifest_dir().map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        Arc::new(build_mex_runtime(state, &repo_root)?)
    };

    let tools = ensure_media_downloader_tools(&workspace_root).await?;

    // Expand plan where required (YouTube + Instagram use flat-playlist expansion).
    let mut plan_items: Vec<MdPlanItemV0> = Vec::new();
    match req.source_kind {
        MdSourceKindV0::Youtube | MdSourceKindV0::Instagram => {
            for src in &sources {
                let expanded = md_expand_ytdlp_sources(
                    &workspace_root,
                    mex_runtime.as_ref(),
                    job,
                    &tools,
                    cookie_jar_payload.as_deref(),
                    &req.source_kind,
                    src,
                    &job_cancel_key,
                )
                .await?;
                plan_items.extend(expanded);
            }
        }
        _ => {
            for src in &sources {
                plan_items.push(MdPlanItemV0 {
                    item_id: md_item_id_from_url(src),
                    source_kind: req.source_kind,
                    url_canonical: src.as_str().to_string(),
                });
            }
        }
    }

    // Dedupe plan items by item_id.
    let mut seen_items = HashSet::new();
    plan_items.retain(|item| seen_items.insert(item.item_id.clone()));

    let item_total = plan_items.len();
    let items: Vec<MdItemResultV0> = plan_items
        .iter()
        .map(|p| MdItemResultV0 {
            item_id: p.item_id.clone(),
            status: "pending".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: None,
            error_message: None,
        })
        .collect();

    let mut output = MdJobOutputV0 {
        schema_version: MD_RESULT_SCHEMA_V0.to_string(),
        plan: MdPlanV0 {
            stable_item_total: item_total,
            items: plan_items.clone(),
        },
        progress: MdProgressV0 {
            state: "running".to_string(),
            item_done: 0,
            item_total,
            bytes_downloaded: None,
            bytes_total: None,
            concurrency,
        },
        items,
        export_records: Vec::new(),
    };

    state
        .storage
        .set_job_outputs(
            &job.job_id.to_string(),
            Some(serde_json::to_value(&output).unwrap_or(json!({}))),
        )
        .await?;

    md_record_md_system_event(
        state,
        trace_id,
        job.job_id,
        workflow_run_id,
        json!({
            "event_kind": "media_downloader.job_state",
            "job_id": job.job_id.to_string(),
            "source_kind": req.source_kind.as_str(),
            "url": null,
            "bytes_downloaded": null,
            "bytes_total": null,
            "item_index": 0,
            "status": "running",
            "error_code": null,
            "item_total": item_total,
            "concurrency": concurrency,
        }),
    )
    .await;

    // Main run loop with bounded concurrency.
    let mut pending: VecDeque<MdPlanItemV0> = plan_items.into_iter().collect();
    let mut join_set: JoinSet<(String, Result<MdItemResultV0, WorkflowError>)> = JoinSet::new();

    let mut completed: usize = 0;
    let mut any_failed = false;

    loop {
        let paused = *pause_rx.borrow();
        let cancelled = *job_cancel_rx.borrow();

        if cancelled && output.progress.state != "cancelled" {
            output.progress.state = "cancelled".to_string();
            md_record_md_system_event(
                state,
                trace_id,
                job.job_id,
                workflow_run_id,
                json!({
                    "event_kind": "media_downloader.job_state",
                    "job_id": job.job_id.to_string(),
                    "source_kind": req.source_kind.as_str(),
                    "url": null,
                    "bytes_downloaded": output.progress.bytes_downloaded,
                    "bytes_total": output.progress.bytes_total,
                    "item_index": completed,
                    "status": "cancelled",
                    "error_code": "cancelled",
                    "item_done": completed,
                    "item_total": item_total,
                }),
            )
            .await;
        }

        // Spawn new tasks up to concurrency.
        while !paused && !cancelled && join_set.len() < concurrency as usize && !pending.is_empty()
        {
            let Some(item) = pending.pop_front() else {
                break;
            };
            let item_id = item.item_id.clone();
            let item_url = item.url_canonical.clone();
            let state_clone = state.clone();
            let job_clone = job.clone();
            let mex_clone = mex_runtime.clone();
            let tools_clone = tools.clone();
            let output_root_dir_clone = output_root_dir.clone();
            let cookie_clone = cookie_jar_payload.clone();
            let job_cancel_key_clone = job_cancel_key.clone();
            let source_kind = req.source_kind;
            let trace_id_clone = trace_id;
            let workflow_run_id_clone = workflow_run_id;
            let forum_max_pages_clone = forum_max_pages;
            let forum_allowlist_domains_clone = forum_allowlist_domains.clone();

            let item_id_for_task = item_id.clone();
            join_set.spawn(async move {
                let res = md_process_item(
                    &state_clone,
                    &job_clone,
                    trace_id_clone,
                    workflow_run_id_clone,
                    &mex_clone,
                    &tools_clone,
                    &output_root_dir_clone,
                    source_kind,
                    &item,
                    cookie_clone.as_deref(),
                    forum_max_pages_clone,
                    &forum_allowlist_domains_clone,
                    &job_cancel_key_clone,
                )
                .await;
                (item_id_for_task, res)
            });

            // Mark running in local output.
            if let Some(entry) = output.items.iter_mut().find(|e| e.item_id == item_id) {
                entry.status = "running".to_string();
            }

            md_record_md_system_event(
                state,
                trace_id,
                job.job_id,
                workflow_run_id,
                json!({
                    "event_kind": "media_downloader.progress",
                    "job_id": job.job_id.to_string(),
                    "source_kind": req.source_kind.as_str(),
                    "url": md_sanitize_url_string_for_telemetry(&item_url),
                    "item_id": item_id,
                    "item_index": completed,
                    "item_total": item_total,
                    "status": "running",
                    "error_code": null,
                    "bytes_downloaded": null,
                    "bytes_total": null,
                }),
            )
            .await;

            state
                .storage
                .set_job_outputs(
                    &job.job_id.to_string(),
                    Some(serde_json::to_value(&output).unwrap_or(json!({}))),
                )
                .await?;
        }

        // Persist pause state.
        if paused && output.progress.state != "paused" && output.progress.state != "cancelled" {
            output.progress.state = "paused".to_string();
            md_record_md_system_event(
                state,
                trace_id,
                job.job_id,
                workflow_run_id,
                json!({
                    "event_kind": "media_downloader.job_state",
                    "job_id": job.job_id.to_string(),
                    "source_kind": req.source_kind.as_str(),
                    "url": null,
                    "bytes_downloaded": output.progress.bytes_downloaded,
                    "bytes_total": output.progress.bytes_total,
                    "item_index": completed,
                    "status": "paused",
                    "error_code": null,
                    "item_done": completed,
                    "item_total": item_total,
                }),
            )
            .await;
        }
        if !paused && output.progress.state == "paused" {
            output.progress.state = "running".to_string();
            md_record_md_system_event(
                state,
                trace_id,
                job.job_id,
                workflow_run_id,
                json!({
                    "event_kind": "media_downloader.job_state",
                    "job_id": job.job_id.to_string(),
                    "source_kind": req.source_kind.as_str(),
                    "url": null,
                    "bytes_downloaded": output.progress.bytes_downloaded,
                    "bytes_total": output.progress.bytes_total,
                    "item_index": completed,
                    "status": "running",
                    "error_code": null,
                    "item_done": completed,
                    "item_total": item_total,
                }),
            )
            .await;
        }

        // Exit if nothing left.
        if join_set.is_empty() && pending.is_empty() {
            break;
        }
        if cancelled && join_set.is_empty() {
            // Drop remaining work and mark pending items as cancelled.
            for item in output.items.iter_mut() {
                if item.status == "pending" {
                    item.status = "cancelled".to_string();
                }
            }
            break;
        }

        tokio::select! {
            joined = join_set.join_next(), if !join_set.is_empty() => {
                if let Some(joined) = joined {
                    let (item_id, result) = joined.map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                    let item_out = match result {
                        Ok(item_out) => item_out,
                        Err(err) => {
                            any_failed = true;
                            MdItemResultV0 {
                                item_id: item_id.clone(),
                                status: "failed".to_string(),
                                artifact_handles: Vec::new(),
                                materialized_paths: Vec::new(),
                                error_code: Some("job_error".to_string()),
                                error_message: Some(err.to_string()),
                            }
                        }
                    };
                    completed = completed.saturating_add(1);
                    output.progress.item_done = completed;
                    if let Some(entry) = output.items.iter_mut().find(|e| e.item_id == item_id) {
                        *entry = item_out.clone();
                    }

                    let item_url = output
                        .plan
                        .items
                        .iter()
                        .find(|p| p.item_id == item_id)
                        .map(|p| p.url_canonical.clone())
                        .unwrap_or_default();
                    let bytes_downloaded =
                        md_sum_artifact_sizes(&workspace_root, &item_out.artifact_handles);
                    let item_id_for_events = item_id.clone();
                    let item_status_for_events = item_out.status.clone();
                    let item_error_code_for_events = item_out.error_code.clone();

                    if let Some(bytes) = bytes_downloaded {
                        let prev = output.progress.bytes_downloaded.unwrap_or(0);
                        output.progress.bytes_downloaded = Some(prev.saturating_add(bytes));
                    }

                    md_record_md_system_event(
                        state,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "event_kind": "media_downloader.item_result",
                            "job_id": job.job_id.to_string(),
                            "source_kind": req.source_kind.as_str(),
                            "url": md_sanitize_url_string_for_telemetry(&item_url),
                            "bytes_downloaded": bytes_downloaded,
                            "bytes_total": null,
                            "item_id": item_id_for_events.clone(),
                            "item_index": completed,
                            "item_total": item_total,
                            "status": item_status_for_events.clone(),
                            "error_code": item_error_code_for_events.clone(),
                        }),
                    )
                    .await;

                    md_record_md_system_event(
                        state,
                        trace_id,
                        job.job_id,
                        workflow_run_id,
                        json!({
                            "event_kind": "media_downloader.progress",
                            "job_id": job.job_id.to_string(),
                            "source_kind": req.source_kind.as_str(),
                            "url": md_sanitize_url_string_for_telemetry(&item_url),
                            "bytes_downloaded": bytes_downloaded,
                            "bytes_total": null,
                            "item_id": item_id_for_events,
                            "item_index": completed,
                            "item_total": item_total,
                            "status": item_status_for_events,
                            "error_code": item_error_code_for_events,
                        }),
                    )
                    .await;

                    state
                        .storage
                        .set_job_outputs(&job.job_id.to_string(), Some(serde_json::to_value(&output).unwrap_or(json!({}))))
                        .await?;
                }
            }
            _ = pause_rx.changed() => {}
            _ = retry_rx.changed() => {
                // Retry failed: requeue failed items (best-effort).
                let gen = *retry_rx.borrow();
                if gen > 0 {
                    for item in output.items.iter_mut() {
                        if item.status == "failed" {
                            item.status = "pending".to_string();
                            item.error_code = None;
                            item.error_message = None;
                            pending.push_back(MdPlanItemV0 {
                                item_id: item.item_id.clone(),
                                source_kind: req.source_kind,
                                url_canonical: output.plan.items.iter().find(|p| p.item_id == item.item_id).map(|p| p.url_canonical.clone()).unwrap_or_default(),
                            });
                        }
                    }
                    // Reset counter so repeated retry requests still trigger.
                    let _ = retry_tx.send(0);
                    any_failed = false;
                }
            }
            cancel_res = job_cancel_rx.changed() => {
                if cancel_res.is_err() {
                    // Sender dropped; ignore and continue.
                }
            }
        }
    }

    // Final status.
    let cancelled = *job_cancel_rx.borrow();

    let final_state = if cancelled {
        JobState::Cancelled
    } else if any_failed {
        JobState::CompletedWithIssues
    } else {
        JobState::Completed
    };

    output.progress.state = match final_state {
        JobState::Cancelled => "cancelled".to_string(),
        JobState::CompletedWithIssues => "completed_with_issues".to_string(),
        _ => "completed".to_string(),
    };

    md_emit_materialization_export_record(
        state,
        job,
        trace_id,
        workflow_run_id,
        &output_root_dir,
        &mut output,
    )
    .await;

    state
        .storage
        .set_job_outputs(
            &job.job_id.to_string(),
            Some(serde_json::to_value(&output).unwrap_or(json!({}))),
        )
        .await?;

    Ok(RunJobOutcome {
        state: final_state,
        status_reason: output.progress.state.clone(),
        output: Some(serde_json::to_value(&output).unwrap_or(json!({}))),
        error_message: None,
    })
}

fn md_is_cancelled(cancel_key: &str) -> bool {
    let Ok(registry) = MD_CANCEL_REGISTRY.lock() else {
        return false;
    };
    registry
        .get(cancel_key)
        .map(|entry| *entry.sender.borrow())
        .unwrap_or(false)
}

fn md_artifact_layer_from_handle_path(
    handle_path: &str,
) -> Option<crate::storage::artifacts::ArtifactLayer> {
    let normalized = handle_path.replace('\\', "/");
    let mut parts = normalized.split('/');
    while let Some(part) = parts.next() {
        if part == "artifacts" {
            return match parts.next()? {
                "L1" => Some(crate::storage::artifacts::ArtifactLayer::L1),
                "L2" => Some(crate::storage::artifacts::ArtifactLayer::L2),
                "L3" => Some(crate::storage::artifacts::ArtifactLayer::L3),
                "L4" => Some(crate::storage::artifacts::ArtifactLayer::L4),
                _ => None,
            };
        }
    }
    None
}

fn md_sum_artifact_sizes(workspace_root: &Path, handles: &[ArtifactHandle]) -> Option<u64> {
    let mut sum: u64 = 0;
    let mut any = false;

    for handle in handles {
        let Some(layer) = md_artifact_layer_from_handle_path(&handle.path) else {
            continue;
        };
        let Ok(manifest) = crate::storage::artifacts::read_artifact_manifest(
            workspace_root,
            layer,
            handle.artifact_id,
        ) else {
            continue;
        };
        sum = sum.saturating_add(manifest.size_bytes);
        any = true;
    }

    any.then_some(sum)
}

async fn md_emit_materialization_export_record(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    output_root_dir: &Path,
    output: &mut MdJobOutputV0,
) {
    let mut materialized_paths: Vec<String> = output
        .items
        .iter()
        .flat_map(|i| i.materialized_paths.iter().cloned())
        .collect();
    materialized_paths.sort();
    materialized_paths.dedup();
    if materialized_paths.is_empty() {
        return;
    }

    let mut output_artifact_handles: Vec<ArtifactHandle> = output
        .items
        .iter()
        .flat_map(|i| i.artifact_handles.iter().cloned())
        .collect();
    let mut seen = HashSet::new();
    output_artifact_handles.retain(|h| seen.insert(h.canonical_id()));
    output_artifact_handles.sort_by(|a, b| a.canonical_id().cmp(&b.canonical_id()));
    if output_artifact_handles.is_empty() {
        return;
    }

    let source_entity_refs: Vec<crate::storage::EntityRef> = if !job.entity_refs.is_empty() {
        job.entity_refs.clone()
    } else {
        vec![crate::storage::EntityRef {
            entity_id: job.job_id.to_string(),
            entity_kind: "job".to_string(),
        }]
    };

    let mut source_inputs = String::new();
    source_inputs.push_str("handshake.media_downloader.export_record.v1\n");
    source_inputs.push_str(&format!("job_id={}\n", job.job_id));
    source_inputs.push_str(&format!("output_root_dir={}\n", output_root_dir.display()));
    source_inputs.push_str("artifacts:\n");
    for h in &output_artifact_handles {
        source_inputs.push_str(&h.canonical_id());
        source_inputs.push('\n');
    }
    source_inputs.push_str("materialized_paths:\n");
    for p in &materialized_paths {
        source_inputs.push_str(p);
        source_inputs.push('\n');
    }

    let source_hash = sha256_hex_str(&source_inputs);
    let config_hash = sha256_hex_str(&format!(
        "handshake.media_downloader.materialize.v1\noutput_root_dir={}\n",
        output_root_dir.display()
    ));

    let record = crate::governance_pack::ExportRecord {
        export_id: Uuid::new_v4(),
        created_at: Utc::now(),
        actor: crate::governance_pack::ExportActor::AiJob,
        job_id: Some(job.job_id),
        source_entity_refs,
        source_hashes: vec![source_hash],
        display_projection_ref: None,
        export_format: "media_downloader_materialization".to_string(),
        exporter: crate::governance_pack::ExporterInfo {
            engine_id: "handshake.media_downloader".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash,
        },
        determinism_level: crate::governance_pack::DeterminismLevel::BestEffort,
        export_target: crate::governance_pack::ExportTarget::LocalFile {
            path: output_root_dir.to_path_buf(),
        },
        policy_id: "SAFE_DEFAULT".to_string(),
        redactions_applied: false,
        output_artifact_handles: output_artifact_handles.clone(),
        materialized_paths: materialized_paths.clone(),
        warnings: vec![
            "materialization: local_file (atomic writes, traversal-safe paths)".to_string(),
        ],
        errors: Vec::new(),
    };

    output.export_records.push(record.clone());

    let payload = serde_json::to_value(&record).unwrap_or(json!({}));
    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::GovernancePackExport,
            FlightRecorderActor::System,
            trace_id,
            payload,
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;
}

fn md_terminal_output_paths(
    workspace_root: &Path,
    result: &EngineResult,
) -> Result<(PathBuf, PathBuf, PathBuf), WorkflowError> {
    let output_handle = result
        .outputs
        .first()
        .ok_or_else(|| WorkflowError::Terminal("engine result missing outputs".into()))?;
    let output_abs = workspace_root.join(PathBuf::from(&output_handle.path));
    let raw =
        fs::read_to_string(&output_abs).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let val: Value =
        serde_json::from_str(&raw).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let stdout_ref = val
        .get("stdout_ref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkflowError::Terminal("terminal_output missing stdout_ref".into()))?;
    let stderr_ref = val
        .get("stderr_ref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkflowError::Terminal("terminal_output missing stderr_ref".into()))?;
    Ok((
        output_abs,
        workspace_root.join(stdout_ref),
        workspace_root.join(stderr_ref),
    ))
}

fn md_terminal_output_cancelled(output_abs: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(output_abs) else {
        return false;
    };
    let Ok(val) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    val.get("cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn md_safe_item_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn md_guess_mime(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        "vtt" => "text/vtt",
        "srt" => "text/plain",
        "json" => "application/json",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn md_normalize_rel_path(input: &str) -> String {
    input
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn md_ensure_safe_rel_path(rel_path: &str) -> Result<(), WorkflowError> {
    let rel = rel_path.trim();
    if rel.is_empty() {
        return Err(WorkflowError::Terminal("empty relative path".to_string()));
    }
    if rel.contains(':') {
        return Err(WorkflowError::Terminal(
            "invalid relative path (contains ':')".to_string(),
        ));
    }

    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(WorkflowError::Terminal(
            "invalid relative path (absolute)".to_string(),
        ));
    }

    for component in path.components() {
        match component {
            std::path::Component::ParentDir
            | std::path::Component::Prefix(_)
            | std::path::Component::RootDir => {
                return Err(WorkflowError::Terminal(
                    "invalid relative path (traversal)".to_string(),
                ))
            }
            _ => {}
        }
    }
    Ok(())
}

fn md_materialize_local_file_from_path(
    export_root: &Path,
    rel_path: &str,
    source_path: &Path,
    overwrite: bool,
) -> Result<String, WorkflowError> {
    if !export_root.is_absolute() {
        return Err(WorkflowError::Terminal(format!(
            "export_root is not absolute: {}",
            export_root.display()
        )));
    }
    fs::create_dir_all(export_root).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    if !export_root.is_dir() {
        return Err(WorkflowError::Terminal(
            "export_root is not a directory".to_string(),
        ));
    }
    let export_root_canon =
        fs::canonicalize(export_root).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let rel = md_normalize_rel_path(rel_path);
    md_ensure_safe_rel_path(&rel)?;

    let target_path = export_root_canon.join(Path::new(&rel));
    if !target_path.starts_with(&export_root_canon) {
        return Err(WorkflowError::Terminal(
            "materialize target escapes export_root".to_string(),
        ));
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }

    if target_path.exists() {
        if !overwrite {
            return Err(WorkflowError::Terminal(
                "materialize target exists (overwrite=false)".to_string(),
            ));
        }
        if target_path.is_dir() {
            return Err(WorkflowError::Terminal(
                "materialize target exists and is a directory".to_string(),
            ));
        }
        fs::remove_file(&target_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }

    let tmp_name = format!(".hsk_tmp_{}", Uuid::new_v4());
    let tmp_path = target_path
        .parent()
        .unwrap_or(&export_root_canon)
        .join(tmp_name);

    let mut src =
        fs::File::open(source_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut dst =
        fs::File::create(&tmp_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    std::io::copy(&mut src, &mut dst).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    dst.flush()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    dst.sync_all()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    drop(dst);

    fs::rename(&tmp_path, &target_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    Ok(rel)
}

fn md_write_file_artifact_from_path(
    workspace_root: &Path,
    layer: crate::storage::artifacts::ArtifactLayer,
    kind: crate::storage::artifacts::ArtifactPayloadKind,
    mime: String,
    filename_hint: Option<String>,
    source_path: &Path,
    classification: crate::storage::artifacts::ArtifactClassification,
    exportable: bool,
    retention_ttl_days: Option<u32>,
    created_by_job_id: Option<Uuid>,
    source_entity_refs: Vec<crate::storage::EntityRef>,
    source_artifact_refs: Vec<ArtifactHandle>,
) -> Result<(crate::storage::artifacts::ArtifactManifest, ArtifactHandle), WorkflowError> {
    let source_meta =
        fs::metadata(source_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    if !source_meta.is_file() {
        return Err(WorkflowError::Terminal(format!(
            "source_path is not a file: {}",
            source_path.display()
        )));
    }

    if matches!(
        kind,
        crate::storage::artifacts::ArtifactPayloadKind::PromptPayload
    ) || matches!(
        classification,
        crate::storage::artifacts::ArtifactClassification::High
    ) {
        if retention_ttl_days.is_none() {
            return Err(WorkflowError::Terminal(
                "retention_ttl_days is required for high-sensitivity artifacts".to_string(),
            ));
        }
    }

    let artifact_id = Uuid::new_v4();
    let created_at = Utc::now();
    let artifact_root =
        crate::storage::artifacts::artifact_root_dir(workspace_root, layer, artifact_id);
    fs::create_dir_all(&artifact_root).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let payload_path = artifact_root.join("payload");
    let payload_tmp = artifact_root.join("payload.tmp");

    let mut src =
        fs::File::open(source_path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut dst =
        fs::File::create(&payload_tmp).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = src
            .read(&mut buf)
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n])
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        total = total.saturating_add(n as u64);
    }
    dst.flush()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    dst.sync_all()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    drop(dst);

    if let Err(err) = fs::rename(&payload_tmp, &payload_path) {
        let _ = fs::remove_file(&payload_tmp);
        return Err(WorkflowError::Terminal(err.to_string()));
    }

    let content_hash = hex::encode(hasher.finalize());
    let manifest = crate::storage::artifacts::ArtifactManifest {
        artifact_id,
        layer,
        kind,
        mime,
        filename_hint,
        created_at,
        created_by_job_id,
        source_entity_refs,
        source_artifact_refs,
        content_hash: content_hash.clone(),
        size_bytes: total,
        classification,
        exportable,
        retention_ttl_days,
        pinned: None,
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };

    crate::storage::artifacts::write_artifact_manifest_atomic(&artifact_root, &manifest)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let handle = ArtifactHandle::new(
        artifact_id,
        format!(
            "{}/payload",
            crate::storage::artifacts::artifact_root_rel(layer, artifact_id)
        ),
    );
    Ok((manifest, handle))
}

const MD_CAPTIONS_METADATA_SCHEMA_V0: &str = "hsk.media_downloader.captions_metadata@v0";
const MD_ITEM_RECORD_SCHEMA_V0: &str = "hsk.media_downloader.item_record@v0";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdCaptionTrackV0 {
    lang: String,
    filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdCaptionsMetadataV0 {
    schema_version: String,
    item_id: String,
    source_kind: MdSourceKindV0,
    url_canonical: String,
    tracks: Vec<MdCaptionTrackV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdItemRecordV0 {
    schema_version: String,
    source_kind: MdSourceKindV0,
    item_id: String,
    url_canonical: String,
    created_at: String,
    entries: Vec<MdItemRecordEntryV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdItemRecordEntryV0 {
    role: String,
    #[serde(default)]
    lang: Option<String>,
    artifact_ref: ArtifactHandle,
    content_hash: String,
    mime: String,
    filename: String,
    materialized_rel_path: String,
}

fn md_item_record_path(
    workspace_root: &Path,
    source_kind: MdSourceKindV0,
    item_id: &str,
) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("gov")
        .join("media_downloader")
        .join("item_records")
        .join(source_kind.as_str())
        .join(format!("{item_id}.json"))
}

fn md_caption_lang_from_filename(filename: &str) -> Option<String> {
    let filename = filename.trim();
    if filename.is_empty() {
        return None;
    }
    let lower = filename.to_ascii_lowercase();
    if !lower.ends_with(".vtt") {
        return None;
    }
    let stem = lower.trim_end_matches(".vtt");
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let last = parts[parts.len().saturating_sub(1)];
    let second_last = parts[parts.len().saturating_sub(2)];
    if matches!(last, "auto" | "asr" | "automatic") && !second_last.trim().is_empty() {
        return Some(second_last.to_string());
    }
    if last.trim().is_empty() {
        None
    } else {
        Some(last.to_string())
    }
}

async fn md_mex_exec(
    mex_runtime: &MexRuntime,
    capability_profile_id: &str,
    operation: &str,
    tool_path: &Path,
    cwd: &str,
    args: Vec<String>,
    cancel_keys: Vec<String>,
    timeout_ms: u64,
    max_output_bytes: u64,
) -> Result<EngineResult, WorkflowError> {
    let capability = match operation {
        "yt_dlp.exec" => "proc.exec:yt-dlp",
        "ffmpeg.exec" => "proc.exec:ffmpeg",
        "ffprobe.exec" => "proc.exec:ffprobe",
        _ => "proc.exec",
    };

    let wants_secrets_use = operation == "yt_dlp.exec" && args.iter().any(|a| a == "--cookies");

    let mut capabilities_requested: Vec<String> =
        vec![capability.to_string(), "fs.write:artifacts".to_string()];
    if operation == "yt_dlp.exec" {
        capabilities_requested.push("net.http".to_string());
        // yt-dlp may invoke ffmpeg/ffprobe internally (merge A/V, probe).
        capabilities_requested.push("proc.exec:ffmpeg".to_string());
        capabilities_requested.push("proc.exec:ffprobe".to_string());
        if wants_secrets_use {
            capabilities_requested.push("secrets.use".to_string());
        }
    }
    let mut seen = HashSet::new();
    capabilities_requested.retain(|c| seen.insert(c.clone()));

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id: Uuid::new_v4(),
        engine_id: MD_TOOL_ENGINE_ID.to_string(),
        engine_version_req: None,
        operation: operation.to_string(),
        inputs: Vec::new(),
        params: json!({
            "tool_path": tool_path.to_string_lossy().to_string(),
            "cwd": cwd,
            "timeout_ms": timeout_ms,
            "args": args,
            "env": {},
            "cancel_keys": cancel_keys,
        }),
        capabilities_requested,
        capability_profile_id: Some(capability_profile_id.to_string()),
        human_consent_obtained: false,
        budget: BudgetSpec {
            cpu_time_ms: None,
            wall_time_ms: Some(timeout_ms),
            memory_bytes: None,
            output_bytes: Some(max_output_bytes),
        },
        determinism: DeterminismLevel::D3,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: Some("capture_stdout_stderr".to_string()),
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.terminal_output".to_string()],
            max_bytes: Some(max_output_bytes),
        },
    };

    mex_runtime
        .execute(op)
        .await
        .map_err(|e| WorkflowError::Terminal(format!("MEX execute failed: {e}")))
}

async fn md_expand_ytdlp_sources(
    workspace_root: &Path,
    mex_runtime: &MexRuntime,
    job: &AiJob,
    tools: &MdToolsV0,
    cookie_jar_payload: Option<&Path>,
    source_kind: &MdSourceKindV0,
    src: &reqwest::Url,
    job_cancel_key: &str,
) -> Result<Vec<MdPlanItemV0>, WorkflowError> {
    let mut args: Vec<String> = vec![
        "--flat-playlist".to_string(),
        "--dump-json".to_string(),
        "--skip-download".to_string(),
        "--no-warnings".to_string(),
        "--no-call-home".to_string(),
        "--no-color".to_string(),
        "--ignore-errors".to_string(),
    ];
    if let Some(cookie) = cookie_jar_payload {
        args.push("--cookies".to_string());
        args.push(cookie.to_string_lossy().to_string());
    }
    args.push("--".to_string());
    args.push(src.as_str().to_string());

    let result = md_mex_exec(
        mex_runtime,
        &job.capability_profile_id,
        "yt_dlp.exec",
        &tools.yt_dlp_path,
        ".",
        args,
        vec![job_cancel_key.to_string()],
        600_000,
        50_000_000,
    )
    .await?;

    let (output_abs, stdout_abs, _stderr_abs) = md_terminal_output_paths(workspace_root, &result)?;
    if md_terminal_output_cancelled(&output_abs) {
        return Ok(Vec::new());
    }

    let stdout =
        fs::read_to_string(&stdout_abs).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut out: Vec<MdPlanItemV0> = Vec::new();
    for line in stdout.lines() {
        let Ok(val) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let id = val.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let webpage_url = val
            .get("webpage_url")
            .and_then(|v| v.as_str())
            .or_else(|| val.get("original_url").and_then(|v| v.as_str()))
            .or_else(|| val.get("url").and_then(|v| v.as_str()))
            .unwrap_or_default();

        let canonical = match source_kind {
            MdSourceKindV0::Youtube if !id.trim().is_empty() => {
                format!("https://www.youtube.com/watch?v={}", id.trim())
            }
            _ if !webpage_url.trim().is_empty() => webpage_url.trim().to_string(),
            _ => src.as_str().to_string(),
        };

        let Ok(url) = reqwest::Url::parse(&canonical) else {
            continue;
        };
        md_validate_url_target(&url)?;

        let item_id = md_safe_item_id(id).unwrap_or_else(|| md_item_id_from_url(&url));
        out.push(MdPlanItemV0 {
            item_id,
            source_kind: *source_kind,
            url_canonical: url.as_str().to_string(),
        });
    }
    out.sort_by(|a, b| a.item_id.cmp(&b.item_id));
    Ok(out)
}

fn md_sanitize_ytdlp_info_json(path: &Path) -> Result<(), WorkflowError> {
    let raw = fs::read_to_string(path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut val: Value =
        serde_json::from_str(&raw).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    if let Some(obj) = val.as_object_mut() {
        obj.remove("http_headers");
        obj.remove("cookies");
        obj.remove("cookie");
        obj.remove("headers");
    }
    let bytes =
        serde_json::to_vec_pretty(&val).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    }
    fs::rename(&tmp, path).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    Ok(())
}

fn md_list_downloaded_files(dir: &Path) -> Result<Vec<PathBuf>, WorkflowError> {
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| WorkflowError::Terminal(e.to_string()))? {
        let entry = entry.map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.ends_with(".part") || name.ends_with(".tmp") {
            continue;
        }
        out.push(path);
    }
    out.sort();
    Ok(out)
}

async fn md_ffprobe_validate(
    workspace_root: &Path,
    mex_runtime: &MexRuntime,
    job: &AiJob,
    tools: &MdToolsV0,
    target_path: &Path,
    cancel_keys: Vec<String>,
) -> Result<bool, WorkflowError> {
    let args: Vec<String> = vec![
        "-v".to_string(),
        "error".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_streams".to_string(),
        "-show_format".to_string(),
        target_path.to_string_lossy().to_string(),
    ];

    let result = md_mex_exec(
        mex_runtime,
        &job.capability_profile_id,
        "ffprobe.exec",
        &tools.ffprobe_path,
        ".",
        args,
        cancel_keys,
        120_000,
        5_000_000,
    )
    .await?;

    let (output_abs, stdout_abs, _stderr_abs) = md_terminal_output_paths(workspace_root, &result)?;
    if md_terminal_output_cancelled(&output_abs) {
        return Ok(false);
    }

    let stdout =
        fs::read_to_string(&stdout_abs).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let parsed: Value =
        serde_json::from_str(&stdout).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let streams = parsed
        .get("streams")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if streams.is_empty() {
        return Ok(false);
    }
    for stream in &streams {
        if let Some(codec_type) = stream.get("codec_type").and_then(|v| v.as_str()) {
            if codec_type == "video" || codec_type == "audio" {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn md_is_obvious_non_media_content_type(content_type: &str) -> bool {
    let ct = content_type.trim().to_ascii_lowercase();
    ct.starts_with("text/")
        || ct.contains("html")
        || ct.contains("json")
        || ct.contains("xml")
        || ct.contains("javascript")
}

fn md_sniff_non_media_prefix(prefix: &[u8]) -> Option<&'static str> {
    let trimmed = prefix
        .iter()
        .copied()
        .skip_while(|b| *b == b' ' || *b == b'\t' || *b == b'\r' || *b == b'\n')
        .take(64)
        .collect::<Vec<u8>>();
    let Ok(text) = std::str::from_utf8(&trimmed) else {
        return None;
    };
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("<!doctype") || lower.starts_with("<html") || lower.starts_with("<script")
    {
        return Some("html");
    }
    if lower.starts_with("<?xml") {
        return Some("xml");
    }
    if lower.starts_with('{') || lower.starts_with('[') {
        return Some("json");
    }
    None
}

fn md_video_ext_from_headers_and_url(url: &reqwest::Url, content_type: &str) -> String {
    let ct = content_type.trim().to_ascii_lowercase();
    if ct.contains("mp4") {
        return "mp4".to_string();
    }
    if ct.contains("webm") {
        return "webm".to_string();
    }
    if ct.contains("matroska") || ct.contains("mkv") {
        return "mkv".to_string();
    }
    if ct.contains("quicktime") {
        return "mov".to_string();
    }
    if let Some(seg) = url.path_segments().and_then(|s| s.last()) {
        if let Some((_, ext)) = seg.rsplit_once('.') {
            let ext = ext.to_ascii_lowercase();
            if ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                return ext;
            }
        }
    }
    "mp4".to_string()
}

async fn md_download_generic_video(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    mex_runtime: &MexRuntime,
    tools: &MdToolsV0,
    output_root_dir: &Path,
    item: &MdPlanItemV0,
    cookie_jar_payload: Option<&Path>,
    job_cancel_key: &str,
) -> Result<MdItemResultV0, WorkflowError> {
    let item_cancel_key = md_item_cancel_key(job.job_id, &item.item_id);
    let workspace_root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let url = reqwest::Url::parse(&item.url_canonical)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    md_validate_url_target(&url)?;

    let tmp_dir = workspace_root
        .join(".handshake")
        .join("tmp")
        .join("media_downloader")
        .join(job.job_id.to_string())
        .join("videodownloader")
        .join(&item.item_id);
    fs::create_dir_all(&tmp_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let client = reqwest::Client::new();
    let resp = client
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
        .send()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "failed".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: Some("http_error".to_string()),
            error_message: Some(format!("download failed: status={status}")),
        });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let _bytes_total = resp.content_length();

    let mut resp = resp;
    let first = resp
        .chunk()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?
        .unwrap_or_default();

    let sniff = md_sniff_non_media_prefix(&first);
    let is_embed = md_is_obvious_non_media_content_type(&content_type)
        || matches!(sniff, Some("html") | Some("xml") | Some("json"));

    if is_embed {
        // Fallback: treat as embed/page and attempt yt-dlp extraction.
        let cwd_rel = tmp_dir
            .strip_prefix(&workspace_root)
            .map_err(|_| WorkflowError::Terminal("temp dir escapes workspace root".into()))?
            .to_string_lossy()
            .replace('\\', "/");

        let mut args: Vec<String> = vec![
            "--no-warnings".to_string(),
            "--no-call-home".to_string(),
            "--no-playlist".to_string(),
            "--continue".to_string(),
            "--ffmpeg-location".to_string(),
            tools
                .ffmpeg_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_string_lossy()
                .to_string(),
            "--merge-output-format".to_string(),
            "mp4".to_string(),
            "--format".to_string(),
            "bv*+ba/b".to_string(),
            "-o".to_string(),
            "%(id)s.%(ext)s".to_string(),
        ];
        if let Some(cookie) = cookie_jar_payload {
            args.push("--cookies".to_string());
            args.push(cookie.to_string_lossy().to_string());
        }
        args.push(item.url_canonical.clone());

        let result = md_mex_exec(
            mex_runtime,
            &job.capability_profile_id,
            "yt_dlp.exec",
            &tools.yt_dlp_path,
            &cwd_rel,
            args,
            vec![job_cancel_key.to_string(), item_cancel_key.clone()],
            3_600_000,
            50_000_000,
        )
        .await?;

        let (output_abs, _stdout_abs, _stderr_abs) =
            md_terminal_output_paths(&workspace_root, &result)?;
        if md_terminal_output_cancelled(&output_abs)
            || md_is_cancelled(job_cancel_key)
            || md_is_cancelled(&item_cancel_key)
        {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Ok(MdItemResultV0 {
                item_id: item.item_id.clone(),
                status: "cancelled".to_string(),
                artifact_handles: Vec::new(),
                materialized_paths: Vec::new(),
                error_code: Some("cancelled".to_string()),
                error_message: None,
            });
        }

        let files = md_list_downloaded_files(&tmp_dir)?;
        let video_file = files.iter().find(|p| {
            let ext = p
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mov" | "m4v")
        });
        let Some(video_file) = video_file else {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Ok(MdItemResultV0 {
                item_id: item.item_id.clone(),
                status: "failed".to_string(),
                artifact_handles: Vec::new(),
                materialized_paths: Vec::new(),
                error_code: Some("no_media_found".to_string()),
                error_message: Some("yt-dlp did not produce a video file".to_string()),
            });
        };

        let valid = md_ffprobe_validate(
            &workspace_root,
            mex_runtime,
            job,
            tools,
            video_file,
            vec![job_cancel_key.to_string(), item_cancel_key.clone()],
        )
        .await?;
        if !valid {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Ok(MdItemResultV0 {
                item_id: item.item_id.clone(),
                status: "failed".to_string(),
                artifact_handles: Vec::new(),
                materialized_paths: Vec::new(),
                error_code: Some("ffprobe_invalid".to_string()),
                error_message: Some("ffprobe validation failed".to_string()),
            });
        }

        let filename = video_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4");
        let mime = md_guess_mime(video_file);
        let (manifest, handle) = md_write_file_artifact_from_path(
            &workspace_root,
            crate::storage::artifacts::ArtifactLayer::L3,
            crate::storage::artifacts::ArtifactPayloadKind::File,
            mime,
            Some(filename.to_string()),
            video_file,
            crate::storage::artifacts::ArtifactClassification::Low,
            true,
            None,
            Some(job.job_id),
            Vec::new(),
            Vec::new(),
        )?;

        record_event_safely(
            state,
            FlightRecorderEvent::new(
                FlightRecorderEventType::DataBronzeCreated,
                FlightRecorderActor::System,
                trace_id,
                json!({
                    "type": "data_bronze_created",
                    "bronze_id": manifest.artifact_id.to_string(),
                    "content_type": manifest.mime,
                    "content_hash": manifest.content_hash,
                    "size_bytes": manifest.size_bytes,
                    "ingestion_source": { "type": "system", "process": "media_downloader" },
                    "ingestion_method": "api_ingest",
                    "external_source": { "url": md_sanitize_url_string_for_telemetry(&item.url_canonical) }
                }),
            )
            .with_job_id(job.job_id.to_string())
            .with_workflow_id(workflow_run_id.to_string()),
        )
        .await;

        let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
        let rel = format!(
            "media_downloader/videodownloader/{}/{}",
            item.item_id, filename
        );
        let mat = md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

        let _ = fs::remove_dir_all(&tmp_dir);
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "completed".to_string(),
            artifact_handles: vec![handle],
            materialized_paths: vec![mat],
            error_code: None,
            error_message: None,
        });
    }

    // Direct media download path (required).
    let ext = md_video_ext_from_headers_and_url(&url, &content_type);
    let part_path = tmp_dir.join(format!("{}.part", item.item_id));
    let final_path = tmp_dir.join(format!("{}.{}", item.item_id, ext));

    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let mut downloaded: u64 = 0;

    file.write_all(&first)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    downloaded = downloaded.saturating_add(first.len() as u64);

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?
    {
        if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
            let _ = tokio::fs::remove_file(&part_path).await;
            let _ = fs::remove_dir_all(&tmp_dir);
            return Ok(MdItemResultV0 {
                item_id: item.item_id.clone(),
                status: "cancelled".to_string(),
                artifact_handles: Vec::new(),
                materialized_paths: Vec::new(),
                error_code: Some("cancelled".to_string()),
                error_message: None,
            });
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
    }
    file.flush()
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    drop(file);

    // Validate with ffprobe before finalizing.
    let valid = md_ffprobe_validate(
        &workspace_root,
        mex_runtime,
        job,
        tools,
        &part_path,
        vec![job_cancel_key.to_string(), item_cancel_key.clone()],
    )
    .await?;
    if !valid {
        let _ = tokio::fs::remove_file(&part_path).await;
        let _ = fs::remove_dir_all(&tmp_dir);
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "failed".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: Some("ffprobe_invalid".to_string()),
            error_message: Some("ffprobe validation failed".to_string()),
        });
    }

    if final_path.exists() {
        let _ = tokio::fs::remove_file(&final_path).await;
    }
    tokio::fs::rename(&part_path, &final_path)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let filename = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4");
    let mime = md_guess_mime(&final_path);
    let (manifest, handle) = md_write_file_artifact_from_path(
        &workspace_root,
        crate::storage::artifacts::ArtifactLayer::L3,
        crate::storage::artifacts::ArtifactPayloadKind::File,
        mime,
        Some(filename.to_string()),
        &final_path,
        crate::storage::artifacts::ArtifactClassification::Low,
        true,
        None,
        Some(job.job_id),
        Vec::new(),
        Vec::new(),
    )?;

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::DataBronzeCreated,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "type": "data_bronze_created",
                "bronze_id": manifest.artifact_id.to_string(),
                "content_type": manifest.mime,
                "content_hash": manifest.content_hash,
                "size_bytes": manifest.size_bytes,
                "ingestion_source": { "type": "system", "process": "media_downloader" },
                "ingestion_method": "api_ingest",
                "external_source": { "url": md_sanitize_url_string_for_telemetry(&item.url_canonical) }
            }),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;

    let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
    let rel = format!(
        "media_downloader/videodownloader/{}/{}",
        item.item_id, filename
    );
    let mat = md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(MdItemResultV0 {
        item_id: item.item_id.clone(),
        status: "completed".to_string(),
        artifact_handles: vec![handle],
        materialized_paths: vec![mat],
        error_code: None,
        error_message: None,
    })
}

async fn md_crawl_forum_images(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    output_root_dir: &Path,
    item: &MdPlanItemV0,
    forum_max_pages: usize,
    forum_allowlist_domains: &[String],
    job_cancel_key: &str,
) -> Result<MdItemResultV0, WorkflowError> {
    md_crawl_forum_images_impl(
        state,
        job,
        trace_id,
        workflow_run_id,
        output_root_dir,
        item,
        forum_max_pages,
        forum_allowlist_domains,
        job_cancel_key,
    )
    .await
}

async fn md_crawl_forum_images_impl(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    output_root_dir: &Path,
    item: &MdPlanItemV0,
    forum_max_pages: usize,
    forum_allowlist_domains: &[String],
    job_cancel_key: &str,
) -> Result<MdItemResultV0, WorkflowError> {
    let item_cancel_key = md_item_cancel_key(job.job_id, &item.item_id);
    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "cancelled".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: Some("cancelled".to_string()),
            error_message: None,
        });
    }

    use regex::Regex;

    #[derive(Debug, Clone, Serialize)]
    struct MdForumManifestEntryV0 {
        page_url: String,
        discovered_url: String,
        chosen_url: String,
        sha256: Option<String>,
        bytes: Option<u64>,
        status: String,
        reason_skipped: Option<String>,
    }

    fn unescape_urlish(input: &str) -> String {
        input
            .replace("&amp;", "&")
            .replace("&#38;", "&")
            .trim()
            .to_string()
    }

    fn url_key(url: &reqwest::Url) -> String {
        let mut clean = url.clone();
        clean.set_fragment(None);
        clean.to_string()
    }

    fn strip_query_and_fragment(mut url: reqwest::Url) -> reqwest::Url {
        url.set_query(None);
        url.set_fragment(None);
        url
    }

    fn domain_allowed(host: &str, allowlist: &HashSet<String>) -> bool {
        let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        if allowlist.contains(&host) {
            return true;
        }
        for base in allowlist {
            if host.ends_with(&format!(".{base}")) {
                return true;
            }
        }
        false
    }

    fn looks_like_noise(url: &reqwest::Url, class: &str, alt: &str) -> Option<String> {
        let hay = format!("{} {} {}", url.path(), class, alt).to_ascii_lowercase();
        for kw in [
            "avatar", "profile", "emoji", "emoticon", "smiley", "icon", "sprite", "logo", "badge",
            "reaction",
        ] {
            if hay.contains(kw) {
                return Some(format!("noise:{kw}"));
            }
        }
        None
    }

    fn parse_srcset_best(srcset: &str) -> Option<String> {
        let mut best: Option<(u32, String)> = None;
        for part in srcset.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let mut it = part.split_whitespace();
            let url = it.next().unwrap_or("").trim();
            if url.is_empty() {
                continue;
            }
            let desc = it.next().unwrap_or("").trim();
            let w = desc
                .strip_suffix('w')
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            match &best {
                Some((bw, _)) if *bw >= w => {}
                _ => best = Some((w, url.to_string())),
            }
        }
        best.map(|(_, u)| u)
    }

    fn infer_fullsize_variants(url: &reqwest::Url) -> Vec<reqwest::Url> {
        let mut out: Vec<reqwest::Url> = Vec::new();

        // Always include a query/fragment stripped variant.
        let stripped = strip_query_and_fragment(url.clone());
        if stripped.as_str() != url.as_str() {
            out.push(stripped);
        }

        // Common thumbnail directory patterns.
        let path = url.path();
        if path.contains("/thumb/") || path.contains("/thumbs/") {
            let new_path = path.replace("/thumbs/", "/").replace("/thumb/", "/");
            let mut u = url.clone();
            u.set_path(&new_path);
            out.push(strip_query_and_fragment(u));
        }

        // Common thumbnail suffix patterns like -150x150 before extension.
        if let Some((head, ext)) = path.rsplit_once('.') {
            if let Some((base, tail)) = head.rsplit_once('-') {
                let tail = tail.to_ascii_lowercase();
                let parts: Vec<&str> = tail.split('x').collect();
                if parts.len() == 2
                    && parts[0].chars().all(|c| c.is_ascii_digit())
                    && parts[1].chars().all(|c| c.is_ascii_digit())
                {
                    let mut u = url.clone();
                    u.set_path(&format!("{base}.{ext}"));
                    out.push(strip_query_and_fragment(u));
                }
            }
        }

        // Dedup preserving order.
        let mut seen: HashSet<String> = HashSet::new();
        out.retain(|u| seen.insert(url_key(u)));
        out
    }

    fn parse_attrs(re: &Regex, tag: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for caps in re.captures_iter(tag) {
            let key = caps
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            let value = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
            if !key.is_empty() {
                out.insert(key, value);
            }
        }
        out
    }

    fn attr<'a>(attrs: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
        attrs.get(key).map(|s| s.as_str())
    }

    fn parse_u32_attr(attrs: &HashMap<String, String>, key: &str) -> Option<u32> {
        attr(attrs, key)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .and_then(|v| v.parse::<u32>().ok())
    }

    let workspace_root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let topic_url = reqwest::Url::parse(&item.url_canonical)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    md_validate_url_target(&topic_url)?;

    let topic_host = topic_url
        .host_str()
        .ok_or_else(|| WorkflowError::Terminal("topic url missing host".into()))?
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();

    let mut allowlist: HashSet<String> = HashSet::new();
    allowlist.insert(topic_host);
    for dom in forum_allowlist_domains {
        let d = dom.trim();
        if d.is_empty() {
            continue;
        }
        if md_is_private_host(d) {
            continue;
        }
        allowlist.insert(d.to_string());
    }

    let max_pages = forum_max_pages.clamp(1, 5000);

    let tmp_dir = workspace_root
        .join(".handshake")
        .join("tmp")
        .join("media_downloader")
        .join(job.job_id.to_string())
        .join("forumcrawler")
        .join(&item.item_id);
    fs::create_dir_all(&tmp_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let re_attr = Regex::new(r#"(?is)([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(['"])(.*?)['"]"#)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let re_link_tag =
        Regex::new(r"(?is)<link\b[^>]*>").map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let re_a_tag =
        Regex::new(r"(?is)<a\b[^>]*>").map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let re_a_img = Regex::new(
        r#"(?is)<a\b[^>]*\bhref\s*=\s*['"](?P<href>[^'"]+)['"][^>]*>(?P<inner>.{0,2000}?)<img\b(?P<imgattrs>[^>]*)>"#,
    )
    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let re_img_tag = Regex::new(r"(?is)<img\b(?P<imgattrs>[^>]*)>")
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let topic_path = topic_url.path().trim_end_matches('/').to_string();

    let mut pages: VecDeque<reqwest::Url> = VecDeque::new();
    let mut visited_pages: HashSet<String> = HashSet::new();
    pages.push_back(topic_url.clone());
    visited_pages.insert(url_key(&topic_url));

    let mut seen_discovered: HashSet<String> = HashSet::new();
    let mut seen_sha256: HashSet<String> = HashSet::new();
    let mut manifest_entries: Vec<MdForumManifestEntryV0> = Vec::new();
    let mut artifacts: Vec<ArtifactHandle> = Vec::new();
    let mut materialized_paths: Vec<String> = Vec::new();

    let mut bytes_downloaded_total: u64 = 0;
    let mut pages_crawled: usize = 0;
    let mut images_done: usize = 0;

    while let Some(page_url) = pages.pop_front() {
        if pages_crawled >= max_pages {
            break;
        }
        if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
            break;
        }

        pages_crawled = pages_crawled.saturating_add(1);

        // Polite defaults (global rate-limit).
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let resp = client
            .get(page_url.clone())
            .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
            .send()
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        if !resp.status().is_success() {
            continue;
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Only parse HTML-ish responses.
        let ct_lower = content_type.to_ascii_lowercase();
        if !(ct_lower.contains("html") || ct_lower.starts_with("text/")) {
            continue;
        }

        let mut body: Vec<u8> = Vec::new();
        let mut resp = resp;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?
        {
            if body.len().saturating_add(chunk.len()) > 5_000_000 {
                break;
            }
            body.extend_from_slice(&chunk);
        }

        let html_text = String::from_utf8_lossy(&body);

        // Pagination discovery: rel=next links.
        for m in re_link_tag.find_iter(&html_text) {
            let tag = m.as_str();
            let attrs = parse_attrs(&re_attr, tag);
            let rel = attr(&attrs, "rel").unwrap_or("").to_ascii_lowercase();
            if !rel.split_whitespace().any(|v| v == "next") {
                continue;
            }
            let Some(href) = attr(&attrs, "href") else {
                continue;
            };
            let href = unescape_urlish(href);
            if href.is_empty() {
                continue;
            }
            let Ok(next_url) = page_url.join(&href) else {
                continue;
            };
            if next_url.scheme() != "http" && next_url.scheme() != "https" {
                continue;
            }
            if let Some(host) = next_url.host_str() {
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
            } else {
                continue;
            }
            let path = next_url.path().trim_end_matches('/');
            if !path.starts_with(&topic_path) {
                continue;
            }
            let key = url_key(&next_url);
            if visited_pages.insert(key) {
                pages.push_back(next_url);
            }
        }

        // Pagination discovery: rel=next anchors.
        for m in re_a_tag.find_iter(&html_text) {
            let tag = m.as_str();
            let attrs = parse_attrs(&re_attr, tag);
            let rel = attr(&attrs, "rel").unwrap_or("").to_ascii_lowercase();
            if !rel.split_whitespace().any(|v| v == "next") {
                continue;
            }
            let Some(href) = attr(&attrs, "href") else {
                continue;
            };
            let href = unescape_urlish(href);
            if href.is_empty() {
                continue;
            }
            let Ok(next_url) = page_url.join(&href) else {
                continue;
            };
            if next_url.scheme() != "http" && next_url.scheme() != "https" {
                continue;
            }
            let Some(host) = next_url.host_str() else {
                continue;
            };
            if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                continue;
            }
            let path = next_url.path().trim_end_matches('/');
            if !path.starts_with(&topic_path) {
                continue;
            }
            let key = url_key(&next_url);
            if visited_pages.insert(key) {
                pages.push_back(next_url);
            }
        }

        // Pagination discovery: heuristic page-ish anchors.
        for m in re_a_tag.find_iter(&html_text) {
            let tag = m.as_str();
            let attrs = parse_attrs(&re_attr, tag);
            let Some(href) = attr(&attrs, "href") else {
                continue;
            };
            let href = unescape_urlish(href);
            if href.is_empty() {
                continue;
            }
            let Ok(u) = page_url.join(&href) else {
                continue;
            };
            if u.scheme() != "http" && u.scheme() != "https" {
                continue;
            }
            let Some(host) = u.host_str() else {
                continue;
            };
            if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                continue;
            }
            let path = u.path().trim_end_matches('/');
            if !path.starts_with(&topic_path) {
                continue;
            }
            let lower = u.as_str().to_ascii_lowercase();
            let is_pageish =
                lower.contains("page=") || lower.contains("/page/") || lower.contains("start=");
            if !is_pageish {
                continue;
            }
            let key = url_key(&u);
            if visited_pages.insert(key) {
                pages.push_back(u);
            }
        }

        // Image discovery: anchors that wrap images (full-res preference).
        for caps in re_a_img.captures_iter(&html_text) {
            if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                break;
            }

            let anchor_href = caps.name("href").map(|m| m.as_str()).unwrap_or("").trim();
            if anchor_href.is_empty() {
                continue;
            }

            let imgattrs_raw = caps.name("imgattrs").map(|m| m.as_str()).unwrap_or("");
            let imgattrs = parse_attrs(&re_attr, imgattrs_raw);
            let class = attr(&imgattrs, "class").unwrap_or("");
            let alt = attr(&imgattrs, "alt").unwrap_or("");
            let width = parse_u32_attr(&imgattrs, "width").unwrap_or(0);
            let height = parse_u32_attr(&imgattrs, "height").unwrap_or(0);
            if (width > 0 && width < 24) || (height > 0 && height < 24) {
                continue;
            }

            let mut candidates_raw: Vec<reqwest::Url> = Vec::new();
            if let Ok(u) = page_url.join(&unescape_urlish(anchor_href)) {
                candidates_raw.push(u);
            }
            for key in [
                "data-fullsize",
                "data-full",
                "data-original",
                "data-src",
                "data-lazy-src",
            ] {
                if let Some(v) = attr(&imgattrs, key) {
                    let v = unescape_urlish(v);
                    if !v.is_empty() {
                        if let Ok(u) = page_url.join(&v) {
                            candidates_raw.push(u);
                        }
                    }
                }
            }
            if let Some(srcset) = attr(&imgattrs, "srcset") {
                if let Some(best) = parse_srcset_best(srcset) {
                    let best = unescape_urlish(best.trim());
                    if !best.is_empty() {
                        if let Ok(u) = page_url.join(&best) {
                            candidates_raw.push(u);
                        }
                    }
                }
            }
            if let Some(v) = attr(&imgattrs, "src") {
                let v = unescape_urlish(v);
                if !v.is_empty() {
                    if let Ok(u) = page_url.join(&v) {
                        candidates_raw.push(u);
                    }
                }
            }

            let Some(discovered_raw) = candidates_raw.first().cloned() else {
                continue;
            };

            let mut unique: Vec<reqwest::Url> = Vec::new();
            let mut seen = HashSet::new();
            for c in candidates_raw {
                if c.scheme() != "http" && c.scheme() != "https" {
                    continue;
                }
                let Some(host) = c.host_str() else {
                    continue;
                };
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
                for cand in
                    std::iter::once(c.clone()).chain(infer_fullsize_variants(&c).into_iter())
                {
                    if cand.scheme() != "http" && cand.scheme() != "https" {
                        continue;
                    }
                    let Some(host) = cand.host_str() else {
                        continue;
                    };
                    if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                        continue;
                    }
                    if !seen.insert(url_key(&cand)) {
                        continue;
                    }
                    unique.push(cand);
                }
            }
            if unique.is_empty() {
                let host = discovered_raw.host_str().unwrap_or("");
                let reason = if md_is_private_host(host) {
                    "private_host_blocked"
                } else {
                    "domain_not_allowlisted"
                };
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_raw.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_raw.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason.to_string()),
                });
                continue;
            }

            let discovered_url = unique[0].clone();
            let discovered_key = url_key(&discovered_url);
            if !seen_discovered.insert(discovered_key.clone()) {
                continue;
            }
            for extra in unique.iter().skip(1) {
                let _ = seen_discovered.insert(url_key(extra));
            }

            if let Some(reason) = looks_like_noise(&discovered_url, class, alt) {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason),
                });
                continue;
            }

            // Try candidates in order until one downloads as an image.
            let mut downloaded: Option<(reqwest::Url, String, u64, PathBuf, String)> = None;
            let mut deduped_skipped = false;
            for candidate in &unique {
                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    break;
                }

                let resp = client
                    .get(candidate.clone())
                    .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
                    .send()
                    .await;
                let Ok(mut resp) = resp else {
                    continue;
                };
                if !resp.status().is_success() {
                    continue;
                }

                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if md_is_obvious_non_media_content_type(&content_type) {
                    continue;
                }

                let Some(first) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                else {
                    continue;
                };
                if md_sniff_non_media_prefix(&first).is_some() {
                    continue;
                }

                let ext = if first.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "jpg"
                } else if first.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
                    "png"
                } else if first.starts_with(b"GIF87a") || first.starts_with(b"GIF89a") {
                    "gif"
                } else if first.starts_with(b"RIFF")
                    && first.len() >= 12
                    && &first[8..12] == b"WEBP"
                {
                    "webp"
                } else {
                    continue;
                };

                let part_path = tmp_dir.join(format!("{}.part", Uuid::new_v4()));
                let mut file = tokio::fs::File::create(&part_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                let mut hasher = Sha256::new();
                let mut bytes: u64 = 0;

                file.write_all(&first)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                hasher.update(&first);
                bytes = bytes.saturating_add(first.len() as u64);

                while let Some(chunk) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                {
                    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                        let _ = tokio::fs::remove_file(&part_path).await;
                        break;
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                    hasher.update(&chunk);
                    bytes = bytes.saturating_add(chunk.len() as u64);
                }
                file.flush()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                drop(file);

                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    continue;
                }

                let sha = hex::encode(hasher.finalize());
                if !seen_sha256.insert(sha.clone()) {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    manifest_entries.push(MdForumManifestEntryV0 {
                        page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                        discovered_url: md_sanitize_url_string_for_telemetry(
                            discovered_url.as_str(),
                        ),
                        chosen_url: md_sanitize_url_string_for_telemetry(candidate.as_str()),
                        sha256: Some(sha),
                        bytes: Some(bytes),
                        status: "skipped".to_string(),
                        reason_skipped: Some("sha256_duplicate".to_string()),
                    });
                    deduped_skipped = true;
                    break;
                }

                let final_path = tmp_dir.join(format!("{sha}.{ext}"));
                tokio::fs::rename(&part_path, &final_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

                downloaded = Some((candidate.clone(), sha, bytes, final_path, ext.to_string()));
                break;
            }

            if deduped_skipped {
                continue;
            }

            let Some((chosen_url, sha, bytes, file_path, ext)) = downloaded else {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "failed".to_string(),
                    reason_skipped: Some("no_image_candidate_succeeded".to_string()),
                });
                continue;
            };

            let mime = match ext.as_str() {
                "jpg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            }
            .to_string();
            let filename = format!("{sha}.{ext}");

            let (artifact_manifest, handle) = md_write_file_artifact_from_path(
                &workspace_root,
                crate::storage::artifacts::ArtifactLayer::L3,
                crate::storage::artifacts::ArtifactPayloadKind::File,
                mime,
                Some(filename.clone()),
                &file_path,
                crate::storage::artifacts::ArtifactClassification::Low,
                true,
                None,
                Some(job.job_id),
                Vec::new(),
                Vec::new(),
            )?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::DataBronzeCreated,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "data_bronze_created",
                        "bronze_id": artifact_manifest.artifact_id.to_string(),
                        "content_type": artifact_manifest.mime,
                        "content_hash": artifact_manifest.content_hash,
                        "size_bytes": artifact_manifest.size_bytes,
                        "ingestion_source": { "type": "system", "process": "media_downloader" },
                        "ingestion_method": "api_ingest",
                        "external_source": { "url": md_sanitize_url_string_for_telemetry(chosen_url.as_str()) }
                    }),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;

            let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
            let rel = format!(
                "media_downloader/forumcrawler/{}/{}",
                &item.item_id, filename
            );
            let mat =
                md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

            artifacts.push(handle);
            materialized_paths.push(mat.clone());
            bytes_downloaded_total = bytes_downloaded_total.saturating_add(bytes);
            images_done = images_done.saturating_add(1);

            if images_done % 25 == 0 {
                md_record_md_system_event(
                    state,
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "event_kind": "media_downloader.progress",
                        "job_id": job.job_id.to_string(),
                        "source_kind": MdSourceKindV0::Forumcrawler.as_str(),
                        "url": md_sanitize_url_string_for_telemetry(&item.url_canonical),
                        "bytes_downloaded": bytes_downloaded_total,
                        "bytes_total": null,
                        "item_id": item.item_id.clone(),
                        "item_index": pages_crawled,
                        "item_total": max_pages,
                        "status": "running",
                        "error_code": null,
                        "pages_crawled": pages_crawled,
                        "images_downloaded": images_done,
                    }),
                )
                .await;
            }

            manifest_entries.push(MdForumManifestEntryV0 {
                page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                chosen_url: md_sanitize_url_string_for_telemetry(chosen_url.as_str()),
                sha256: Some(sha),
                bytes: Some(bytes),
                status: "downloaded".to_string(),
                reason_skipped: None,
            });
        }

        // Standalone images not wrapped in anchors.
        for caps in re_img_tag.captures_iter(&html_text) {
            if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                break;
            }

            let imgattrs_raw = caps.name("imgattrs").map(|m| m.as_str()).unwrap_or("");
            let imgattrs = parse_attrs(&re_attr, imgattrs_raw);
            let class = attr(&imgattrs, "class").unwrap_or("");
            let alt = attr(&imgattrs, "alt").unwrap_or("");
            let width = parse_u32_attr(&imgattrs, "width").unwrap_or(0);
            let height = parse_u32_attr(&imgattrs, "height").unwrap_or(0);
            if (width > 0 && width < 24) || (height > 0 && height < 24) {
                continue;
            }

            let mut candidates_raw: Vec<reqwest::Url> = Vec::new();
            for key in [
                "data-fullsize",
                "data-full",
                "data-original",
                "data-src",
                "data-lazy-src",
            ] {
                if let Some(v) = attr(&imgattrs, key) {
                    let v = unescape_urlish(v);
                    if !v.is_empty() {
                        if let Ok(u) = page_url.join(&v) {
                            candidates_raw.push(u);
                        }
                    }
                }
            }
            if let Some(srcset) = attr(&imgattrs, "srcset") {
                if let Some(best) = parse_srcset_best(srcset) {
                    let best = unescape_urlish(best.trim());
                    if !best.is_empty() {
                        if let Ok(u) = page_url.join(&best) {
                            candidates_raw.push(u);
                        }
                    }
                }
            }
            if let Some(v) = attr(&imgattrs, "src") {
                let v = unescape_urlish(v);
                if !v.is_empty() {
                    if let Ok(u) = page_url.join(&v) {
                        candidates_raw.push(u);
                    }
                }
            }

            let Some(discovered_raw) = candidates_raw.first().cloned() else {
                continue;
            };

            let mut unique: Vec<reqwest::Url> = Vec::new();
            let mut seen = HashSet::new();
            for c in candidates_raw {
                if c.scheme() != "http" && c.scheme() != "https" {
                    continue;
                }
                let Some(host) = c.host_str() else {
                    continue;
                };
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
                for cand in
                    std::iter::once(c.clone()).chain(infer_fullsize_variants(&c).into_iter())
                {
                    if cand.scheme() != "http" && cand.scheme() != "https" {
                        continue;
                    }
                    let Some(host) = cand.host_str() else {
                        continue;
                    };
                    if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                        continue;
                    }
                    if !seen.insert(url_key(&cand)) {
                        continue;
                    }
                    unique.push(cand);
                }
            }
            if unique.is_empty() {
                let host = discovered_raw.host_str().unwrap_or("");
                let reason = if md_is_private_host(host) {
                    "private_host_blocked"
                } else {
                    "domain_not_allowlisted"
                };
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_raw.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_raw.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason.to_string()),
                });
                continue;
            }

            let discovered_url = unique[0].clone();
            let discovered_key = url_key(&discovered_url);
            if !seen_discovered.insert(discovered_key.clone()) {
                continue;
            }
            for extra in unique.iter().skip(1) {
                let _ = seen_discovered.insert(url_key(extra));
            }

            if let Some(reason) = looks_like_noise(&discovered_url, class, alt) {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason),
                });
                continue;
            }

            let mut downloaded: Option<(reqwest::Url, String, u64, PathBuf, String)> = None;
            let mut deduped_skipped = false;
            for candidate in &unique {
                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    break;
                }

                let resp = client
                    .get(candidate.clone())
                    .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
                    .send()
                    .await;
                let Ok(mut resp) = resp else {
                    continue;
                };
                if !resp.status().is_success() {
                    continue;
                }

                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if md_is_obvious_non_media_content_type(&content_type) {
                    continue;
                }

                let Some(first) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                else {
                    continue;
                };
                if md_sniff_non_media_prefix(&first).is_some() {
                    continue;
                }

                let ext = if first.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "jpg"
                } else if first.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
                    "png"
                } else if first.starts_with(b"GIF87a") || first.starts_with(b"GIF89a") {
                    "gif"
                } else if first.starts_with(b"RIFF")
                    && first.len() >= 12
                    && &first[8..12] == b"WEBP"
                {
                    "webp"
                } else {
                    continue;
                };

                let part_path = tmp_dir.join(format!("{}.part", Uuid::new_v4()));
                let mut file = tokio::fs::File::create(&part_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                let mut hasher = Sha256::new();
                let mut bytes: u64 = 0;

                file.write_all(&first)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                hasher.update(&first);
                bytes = bytes.saturating_add(first.len() as u64);

                while let Some(chunk) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                {
                    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                        let _ = tokio::fs::remove_file(&part_path).await;
                        break;
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                    hasher.update(&chunk);
                    bytes = bytes.saturating_add(chunk.len() as u64);
                }
                file.flush()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                drop(file);

                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    continue;
                }

                let sha = hex::encode(hasher.finalize());
                if !seen_sha256.insert(sha.clone()) {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    manifest_entries.push(MdForumManifestEntryV0 {
                        page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                        discovered_url: md_sanitize_url_string_for_telemetry(
                            discovered_url.as_str(),
                        ),
                        chosen_url: md_sanitize_url_string_for_telemetry(candidate.as_str()),
                        sha256: Some(sha),
                        bytes: Some(bytes),
                        status: "skipped".to_string(),
                        reason_skipped: Some("sha256_duplicate".to_string()),
                    });
                    deduped_skipped = true;
                    break;
                }

                let final_path = tmp_dir.join(format!("{sha}.{ext}"));
                tokio::fs::rename(&part_path, &final_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

                downloaded = Some((candidate.clone(), sha, bytes, final_path, ext.to_string()));
                break;
            }

            if deduped_skipped {
                continue;
            }

            let Some((chosen_url, sha, bytes, file_path, ext)) = downloaded else {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "failed".to_string(),
                    reason_skipped: Some("no_image_candidate_succeeded".to_string()),
                });
                continue;
            };

            let mime = match ext.as_str() {
                "jpg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            }
            .to_string();
            let filename = format!("{sha}.{ext}");

            let (artifact_manifest, handle) = md_write_file_artifact_from_path(
                &workspace_root,
                crate::storage::artifacts::ArtifactLayer::L3,
                crate::storage::artifacts::ArtifactPayloadKind::File,
                mime,
                Some(filename.clone()),
                &file_path,
                crate::storage::artifacts::ArtifactClassification::Low,
                true,
                None,
                Some(job.job_id),
                Vec::new(),
                Vec::new(),
            )?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::DataBronzeCreated,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "data_bronze_created",
                        "bronze_id": artifact_manifest.artifact_id.to_string(),
                        "content_type": artifact_manifest.mime,
                        "content_hash": artifact_manifest.content_hash,
                        "size_bytes": artifact_manifest.size_bytes,
                        "ingestion_source": { "type": "system", "process": "media_downloader" },
                        "ingestion_method": "api_ingest",
                        "external_source": { "url": md_sanitize_url_string_for_telemetry(chosen_url.as_str()) }
                    }),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;

            let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
            let rel = format!(
                "media_downloader/forumcrawler/{}/{}",
                &item.item_id, filename
            );
            let mat =
                md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

            artifacts.push(handle);
            materialized_paths.push(mat.clone());
            bytes_downloaded_total = bytes_downloaded_total.saturating_add(bytes);
            images_done = images_done.saturating_add(1);

            if images_done % 25 == 0 {
                md_record_md_system_event(
                    state,
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "event_kind": "media_downloader.progress",
                        "job_id": job.job_id.to_string(),
                        "source_kind": MdSourceKindV0::Forumcrawler.as_str(),
                        "url": md_sanitize_url_string_for_telemetry(&item.url_canonical),
                        "bytes_downloaded": bytes_downloaded_total,
                        "bytes_total": null,
                        "item_id": item.item_id.clone(),
                        "item_index": pages_crawled,
                        "item_total": max_pages,
                        "status": "running",
                        "error_code": null,
                        "pages_crawled": pages_crawled,
                        "images_downloaded": images_done,
                    }),
                )
                .await;
            }

            manifest_entries.push(MdForumManifestEntryV0 {
                page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                chosen_url: md_sanitize_url_string_for_telemetry(chosen_url.as_str()),
                sha256: Some(sha),
                bytes: Some(bytes),
                status: "downloaded".to_string(),
                reason_skipped: None,
            });
        }
    }

    // Write manifest artifact (required).
    let manifest_path = tmp_dir.join("manifest.json");
    let manifest_json = serde_json::to_vec_pretty(&manifest_entries)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    tokio::fs::write(&manifest_path, &manifest_json)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let (manifest_art, manifest_handle) = md_write_file_artifact_from_path(
        &workspace_root,
        crate::storage::artifacts::ArtifactLayer::L3,
        crate::storage::artifacts::ArtifactPayloadKind::DatasetSlice,
        "application/json".to_string(),
        Some("manifest.json".to_string()),
        &manifest_path,
        crate::storage::artifacts::ArtifactClassification::Low,
        true,
        None,
        Some(job.job_id),
        Vec::new(),
        Vec::new(),
    )?;

    let payload_abs = workspace_root.join(PathBuf::from(&manifest_handle.path));
    let rel = format!(
        "media_downloader/forumcrawler/{}/manifest.json",
        &item.item_id
    );
    let mat = md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

    artifacts.push(manifest_handle);
    materialized_paths.push(mat);

    // Also emit DataBronzeCreated for the manifest artifact.
    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::DataBronzeCreated,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "type": "data_bronze_created",
                "bronze_id": manifest_art.artifact_id.to_string(),
                "content_type": manifest_art.mime,
                "content_hash": manifest_art.content_hash,
                "size_bytes": manifest_art.size_bytes,
                "ingestion_source": { "type": "system", "process": "media_downloader" },
                "ingestion_method": "api_ingest",
                "external_source": { "url": md_sanitize_url_string_for_telemetry(&item.url_canonical) }
            }),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;

    let mut seen = HashSet::new();
    artifacts.retain(|h| seen.insert(h.canonical_id()));
    materialized_paths.sort();
    materialized_paths.dedup();

    let cancelled = md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key);
    let status = if cancelled {
        "cancelled"
    } else if images_done == 0 {
        "failed"
    } else {
        "completed"
    };

    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(MdItemResultV0 {
        item_id: item.item_id.clone(),
        status: status.to_string(),
        artifact_handles: artifacts,
        materialized_paths,
        error_code: if cancelled {
            Some("cancelled".to_string())
        } else if images_done == 0 {
            Some("no_images_found".to_string())
        } else {
            None
        },
        error_message: None,
    })
}

#[cfg(any())]
async fn md_crawl_forum_images(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    output_root_dir: &Path,
    item: &MdPlanItemV0,
    forum_max_pages: usize,
    forum_allowlist_domains: &[String],
    job_cancel_key: &str,
) -> Result<MdItemResultV0, WorkflowError> {
    let item_cancel_key = md_item_cancel_key(job.job_id, &item.item_id);
    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "cancelled".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: Some("cancelled".to_string()),
            error_message: None,
        });
    }

    #[derive(Debug, Clone, Serialize)]
    struct MdForumManifestEntryV0 {
        page_url: String,
        discovered_url: String,
        chosen_url: String,
        sha256: Option<String>,
        bytes: Option<u64>,
        status: String,
        reason_skipped: Option<String>,
    }

    fn url_key(url: &reqwest::Url) -> String {
        let mut clean = url.clone();
        clean.set_fragment(None);
        clean.to_string()
    }

    fn domain_allowed(host: &str, allowlist: &HashSet<String>) -> bool {
        let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        if allowlist.contains(&host) {
            return true;
        }
        for base in allowlist {
            if host.ends_with(&format!(".{base}")) {
                return true;
            }
        }
        false
    }

    fn looks_like_noise(url: &reqwest::Url, class: &str, alt: &str) -> Option<String> {
        let hay = format!("{} {} {}", url.path(), class, alt).to_ascii_lowercase();
        for kw in [
            "avatar", "profile", "emoji", "emoticon", "smiley", "icon", "sprite", "logo", "badge",
            "reaction",
        ] {
            if hay.contains(kw) {
                return Some(format!("noise:{kw}"));
            }
        }
        None
    }

    fn parse_srcset_best(srcset: &str) -> Option<String> {
        let mut best: Option<(u32, String)> = None;
        for part in srcset.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let mut it = part.split_whitespace();
            let url = it.next().unwrap_or("").trim();
            if url.is_empty() {
                continue;
            }
            let desc = it.next().unwrap_or("").trim();
            let w = desc
                .strip_suffix('w')
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            match &best {
                Some((bw, _)) if *bw >= w => {}
                _ => best = Some((w, url.to_string())),
            }
        }
        best.map(|(_, u)| u)
    }

    fn strip_query_and_fragment(mut url: reqwest::Url) -> reqwest::Url {
        url.set_query(None);
        url.set_fragment(None);
        url
    }

    let workspace_root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let topic_url = reqwest::Url::parse(&item.url_canonical)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    md_validate_url_target(&topic_url)?;

    let topic_host = topic_url
        .host_str()
        .ok_or_else(|| WorkflowError::Terminal("topic url missing host".into()))?
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();

    let mut allowlist: HashSet<String> = HashSet::new();
    allowlist.insert(topic_host);
    for dom in forum_allowlist_domains {
        let d = dom.trim();
        if d.is_empty() {
            continue;
        }
        if md_is_private_host(d) {
            continue;
        }
        allowlist.insert(d.to_string());
    }

    let max_pages = forum_max_pages.clamp(1, 5000);

    let tmp_dir = workspace_root
        .join(".handshake")
        .join("tmp")
        .join("media_downloader")
        .join(job.job_id.to_string())
        .join("forumcrawler")
        .join(&item.item_id);
    fs::create_dir_all(&tmp_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let client = reqwest::Client::new();

    let sel_a = Selector::parse("a[href]").map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let sel_img = Selector::parse("img").map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    let sel_link_next = Selector::parse("link[rel=next][href]")
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let topic_path = topic_url.path().trim_end_matches('/').to_string();

    let mut pages: VecDeque<reqwest::Url> = VecDeque::new();
    pages.push_back(topic_url.clone());
    let mut visited_pages: HashSet<String> = HashSet::new();
    visited_pages.insert(url_key(&topic_url));

    let mut seen_discovered: HashSet<String> = HashSet::new();
    let mut seen_sha256: HashSet<String> = HashSet::new();

    let mut manifest_entries: Vec<MdForumManifestEntryV0> = Vec::new();
    let mut artifacts: Vec<ArtifactHandle> = Vec::new();
    let mut materialized_paths: Vec<String> = Vec::new();

    let mut bytes_downloaded_total: u64 = 0;
    let mut pages_crawled: usize = 0;
    let mut images_done: usize = 0;

    while let Some(page_url) = pages.pop_front() {
        if pages_crawled >= max_pages {
            break;
        }
        if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
            break;
        }

        pages_crawled = pages_crawled.saturating_add(1);

        // Polite defaults (global rate-limit).
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let resp = client
            .get(page_url.clone())
            .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
            .send()
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
        if !resp.status().is_success() {
            continue;
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Only parse HTML-ish responses.
        let ct_lower = content_type.to_ascii_lowercase();
        if !(ct_lower.contains("html") || ct_lower.starts_with("text/")) {
            continue;
        }

        let mut body: Vec<u8> = Vec::new();
        let mut resp = resp;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| WorkflowError::Terminal(e.to_string()))?
        {
            if body.len().saturating_add(chunk.len()) > 5_000_000 {
                break;
            }
            body.extend_from_slice(&chunk);
        }

        let html_text = String::from_utf8_lossy(&body);
        let doc = Html::parse_document(&html_text);

        // Pagination discovery: rel=next links.
        for link in doc.select(&sel_link_next) {
            let Some(href) = link.value().attr("href") else {
                continue;
            };
            let href = href.trim();
            if href.is_empty() {
                continue;
            }
            let Ok(next_url) = page_url.join(href) else {
                continue;
            };
            if next_url.scheme() != "http" && next_url.scheme() != "https" {
                continue;
            }
            if let Some(host) = next_url.host_str() {
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
            } else {
                continue;
            }
            let path = next_url.path().trim_end_matches('/');
            if !path.starts_with(&topic_path) {
                continue;
            }
            let key = url_key(&next_url);
            if visited_pages.insert(key) {
                pages.push_back(next_url);
            }
        }

        // Pagination discovery: heuristic page-ish anchors.
        for a in doc.select(&sel_a) {
            let Some(href) = a.value().attr("href") else {
                continue;
            };
            let href = href.trim();
            if href.is_empty() {
                continue;
            }
            let Ok(u) = page_url.join(href) else {
                continue;
            };
            if u.scheme() != "http" && u.scheme() != "https" {
                continue;
            }
            let Some(host) = u.host_str() else {
                continue;
            };
            if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                continue;
            }
            let path = u.path().trim_end_matches('/');
            if !path.starts_with(&topic_path) {
                continue;
            }
            let lower = u.as_str().to_ascii_lowercase();
            let is_pageish =
                lower.contains("page=") || lower.contains("/page/") || lower.contains("start=");
            if !is_pageish {
                continue;
            }
            let key = url_key(&u);
            if visited_pages.insert(key) {
                pages.push_back(u);
            }
        }

        // Image discovery: anchors that wrap images (full-res preference).
        for a in doc.select(&sel_a) {
            if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                break;
            }

            let Some(anchor_href) = a.value().attr("href") else {
                continue;
            };
            let anchor_href = anchor_href.trim();
            if anchor_href.is_empty() {
                continue;
            }

            let Some(img) = a.select(&sel_img).next() else {
                continue;
            };

            let class = img.value().attr("class").unwrap_or("");
            let alt = img.value().attr("alt").unwrap_or("");

            let mut candidates: Vec<reqwest::Url> = Vec::new();
            if let Ok(u) = page_url.join(anchor_href) {
                candidates.push(u);
            }

            for key in [
                "data-fullsize",
                "data-full",
                "data-original",
                "data-src",
                "data-lazy-src",
                "src",
            ] {
                if let Some(v) = img.value().attr(key) {
                    let v = v.trim();
                    if !v.is_empty() {
                        if let Ok(u) = page_url.join(v) {
                            candidates.push(u);
                        }
                    }
                }
            }
            if let Some(srcset) = img.value().attr("srcset") {
                if let Some(best) = parse_srcset_best(srcset) {
                    if let Ok(u) = page_url.join(best.trim()) {
                        candidates.push(u);
                    }
                }
            }

            let mut unique: Vec<reqwest::Url> = Vec::new();
            let mut seen = HashSet::new();
            for c in candidates {
                if c.scheme() != "http" && c.scheme() != "https" {
                    continue;
                }
                let Some(host) = c.host_str() else {
                    continue;
                };
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
                if !seen.insert(url_key(&c)) {
                    continue;
                }
                unique.push(c);
                let stripped = strip_query_and_fragment(c.clone());
                if seen.insert(url_key(&stripped)) {
                    unique.push(stripped);
                }
            }

            let Some(discovered_url) = unique.first().cloned() else {
                continue;
            };
            let discovered_key = url_key(&discovered_url);
            if !seen_discovered.insert(discovered_key.clone()) {
                continue;
            }
            for extra in unique.iter().skip(1) {
                let _ = seen_discovered.insert(url_key(extra));
            }

            if let Some(reason) = looks_like_noise(&discovered_url, class, alt) {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason),
                });
                continue;
            }

            // Try candidates in order until one downloads as an image.
            let mut downloaded: Option<(reqwest::Url, String, u64, PathBuf, String)> = None;
            for candidate in &unique {
                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    break;
                }

                let resp = client
                    .get(candidate.clone())
                    .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
                    .send()
                    .await;
                let Ok(mut resp) = resp else {
                    continue;
                };
                if !resp.status().is_success() {
                    continue;
                }

                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if md_is_obvious_non_media_content_type(&content_type) {
                    continue;
                }

                let Some(first) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                else {
                    continue;
                };
                if md_sniff_non_media_prefix(&first).is_some() {
                    continue;
                }

                // Simple image sniff.
                let ext = if first.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "jpg"
                } else if first.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
                    "png"
                } else if first.starts_with(b"GIF87a") || first.starts_with(b"GIF89a") {
                    "gif"
                } else if first.starts_with(b"RIFF")
                    && first.len() >= 12
                    && &first[8..12] == b"WEBP"
                {
                    "webp"
                } else {
                    continue;
                };

                let part_path = tmp_dir.join(format!("{}.part", Uuid::new_v4()));
                let mut file = tokio::fs::File::create(&part_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                let mut hasher = Sha256::new();
                let mut bytes: u64 = 0;

                file.write_all(&first)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                hasher.update(&first);
                bytes = bytes.saturating_add(first.len() as u64);

                while let Some(chunk) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                {
                    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                        let _ = tokio::fs::remove_file(&part_path).await;
                        break;
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                    hasher.update(&chunk);
                    bytes = bytes.saturating_add(chunk.len() as u64);
                }
                file.flush()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                drop(file);

                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    continue;
                }

                let sha = hex::encode(hasher.finalize());
                if !seen_sha256.insert(sha.clone()) {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    manifest_entries.push(MdForumManifestEntryV0 {
                        page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                        discovered_url: md_sanitize_url_string_for_telemetry(
                            discovered_url.as_str(),
                        ),
                        chosen_url: md_sanitize_url_string_for_telemetry(candidate.as_str()),
                        sha256: Some(sha),
                        bytes: Some(bytes),
                        status: "skipped".to_string(),
                        reason_skipped: Some("sha256_duplicate".to_string()),
                    });
                    continue;
                }

                let final_path = tmp_dir.join(format!("{sha}.{ext}"));
                tokio::fs::rename(&part_path, &final_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

                downloaded = Some((candidate.clone(), sha, bytes, final_path, ext.to_string()));
                break;
            }

            let Some((chosen_url, sha, bytes, file_path, ext)) = downloaded else {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "failed".to_string(),
                    reason_skipped: Some("no_image_candidate_succeeded".to_string()),
                });
                continue;
            };

            let mime = match ext.as_str() {
                "jpg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            }
            .to_string();
            let filename = format!("{sha}.{ext}");

            let (artifact_manifest, handle) = md_write_file_artifact_from_path(
                &workspace_root,
                crate::storage::artifacts::ArtifactLayer::L3,
                crate::storage::artifacts::ArtifactPayloadKind::File,
                mime,
                Some(filename.clone()),
                &file_path,
                crate::storage::artifacts::ArtifactClassification::Low,
                true,
                None,
                Some(job.job_id),
                Vec::new(),
                Vec::new(),
            )?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::DataBronzeCreated,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "data_bronze_created",
                        "bronze_id": artifact_manifest.artifact_id.to_string(),
                        "content_type": artifact_manifest.mime,
                        "content_hash": artifact_manifest.content_hash,
                        "size_bytes": artifact_manifest.size_bytes,
                        "ingestion_source": { "type": "system", "process": "media_downloader" },
                        "ingestion_method": "api_ingest",
                        "external_source": { "url": md_sanitize_url_string_for_telemetry(chosen_url.as_str()) }
                    }),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;

            let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
            let rel = format!(
                "media_downloader/forumcrawler/{}/{}",
                &item.item_id, filename
            );
            let mat =
                md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

            artifacts.push(handle);
            materialized_paths.push(mat.clone());
            bytes_downloaded_total = bytes_downloaded_total.saturating_add(bytes);
            images_done = images_done.saturating_add(1);

            if images_done % 25 == 0 {
                md_record_md_system_event(
                    state,
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "event_kind": "media_downloader.progress",
                        "job_id": job.job_id.to_string(),
                        "source_kind": MdSourceKindV0::Forumcrawler.as_str(),
                        "url": md_sanitize_url_string_for_telemetry(&item.url_canonical),
                        "bytes_downloaded": bytes_downloaded_total,
                        "bytes_total": null,
                        "item_id": item.item_id.clone(),
                        "item_index": pages_crawled,
                        "item_total": max_pages,
                        "status": "running",
                        "error_code": null,
                        "pages_crawled": pages_crawled,
                        "images_downloaded": images_done,
                    }),
                )
                .await;
            }

            manifest_entries.push(MdForumManifestEntryV0 {
                page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                chosen_url: md_sanitize_url_string_for_telemetry(chosen_url.as_str()),
                sha256: Some(sha),
                bytes: Some(bytes),
                status: "downloaded".to_string(),
                reason_skipped: None,
            });
        }

        // Also include standalone images not wrapped in anchors.
        for img in doc.select(&sel_img) {
            if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                break;
            }

            let class = img.value().attr("class").unwrap_or("");
            let alt = img.value().attr("alt").unwrap_or("");

            let mut candidates: Vec<reqwest::Url> = Vec::new();
            for key in [
                "data-fullsize",
                "data-full",
                "data-original",
                "data-src",
                "data-lazy-src",
                "src",
            ] {
                if let Some(v) = img.value().attr(key) {
                    let v = v.trim();
                    if !v.is_empty() {
                        if let Ok(u) = page_url.join(v) {
                            candidates.push(u);
                        }
                    }
                }
            }
            if let Some(srcset) = img.value().attr("srcset") {
                if let Some(best) = parse_srcset_best(srcset) {
                    if let Ok(u) = page_url.join(best.trim()) {
                        candidates.push(u);
                    }
                }
            }

            let mut unique: Vec<reqwest::Url> = Vec::new();
            let mut seen = HashSet::new();
            for c in candidates {
                if c.scheme() != "http" && c.scheme() != "https" {
                    continue;
                }
                let Some(host) = c.host_str() else {
                    continue;
                };
                if md_is_private_host(host) || !domain_allowed(host, &allowlist) {
                    continue;
                }
                if !seen.insert(url_key(&c)) {
                    continue;
                }
                unique.push(c);
                let stripped = strip_query_and_fragment(c.clone());
                if seen.insert(url_key(&stripped)) {
                    unique.push(stripped);
                }
            }

            let Some(discovered_url) = unique.first().cloned() else {
                continue;
            };
            let discovered_key = url_key(&discovered_url);
            if !seen_discovered.insert(discovered_key.clone()) {
                continue;
            }
            for extra in unique.iter().skip(1) {
                let _ = seen_discovered.insert(url_key(extra));
            }

            if let Some(reason) = looks_like_noise(&discovered_url, class, alt) {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "skipped".to_string(),
                    reason_skipped: Some(reason),
                });
                continue;
            }

            let mut downloaded: Option<(reqwest::Url, String, u64, PathBuf, String)> = None;
            for candidate in &unique {
                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    break;
                }

                let resp = client
                    .get(candidate.clone())
                    .header(reqwest::header::USER_AGENT, "Handshake-MediaDownloader/1.0")
                    .send()
                    .await;
                let Ok(mut resp) = resp else {
                    continue;
                };
                if !resp.status().is_success() {
                    continue;
                }

                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if md_is_obvious_non_media_content_type(&content_type) {
                    continue;
                }

                let Some(first) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                else {
                    continue;
                };
                if md_sniff_non_media_prefix(&first).is_some() {
                    continue;
                }

                let ext = if first.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "jpg"
                } else if first.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
                    "png"
                } else if first.starts_with(b"GIF87a") || first.starts_with(b"GIF89a") {
                    "gif"
                } else if first.starts_with(b"RIFF")
                    && first.len() >= 12
                    && &first[8..12] == b"WEBP"
                {
                    "webp"
                } else {
                    continue;
                };

                let part_path = tmp_dir.join(format!("{}.part", Uuid::new_v4()));
                let mut file = tokio::fs::File::create(&part_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                let mut hasher = Sha256::new();
                let mut bytes: u64 = 0;

                file.write_all(&first)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                hasher.update(&first);
                bytes = bytes.saturating_add(first.len() as u64);

                while let Some(chunk) = resp
                    .chunk()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?
                {
                    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                        let _ = tokio::fs::remove_file(&part_path).await;
                        break;
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                    hasher.update(&chunk);
                    bytes = bytes.saturating_add(chunk.len() as u64);
                }
                file.flush()
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
                drop(file);

                if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                    continue;
                }

                let sha = hex::encode(hasher.finalize());
                if !seen_sha256.insert(sha.clone()) {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    manifest_entries.push(MdForumManifestEntryV0 {
                        page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                        discovered_url: md_sanitize_url_string_for_telemetry(
                            discovered_url.as_str(),
                        ),
                        chosen_url: md_sanitize_url_string_for_telemetry(candidate.as_str()),
                        sha256: Some(sha),
                        bytes: Some(bytes),
                        status: "skipped".to_string(),
                        reason_skipped: Some("sha256_duplicate".to_string()),
                    });
                    continue;
                }

                let final_path = tmp_dir.join(format!("{sha}.{ext}"));
                tokio::fs::rename(&part_path, &final_path)
                    .await
                    .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

                downloaded = Some((candidate.clone(), sha, bytes, final_path, ext.to_string()));
                break;
            }

            let Some((chosen_url, sha, bytes, file_path, ext)) = downloaded else {
                manifest_entries.push(MdForumManifestEntryV0 {
                    page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                    discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    chosen_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                    sha256: None,
                    bytes: None,
                    status: "failed".to_string(),
                    reason_skipped: Some("no_image_candidate_succeeded".to_string()),
                });
                continue;
            };

            let mime = match ext.as_str() {
                "jpg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            }
            .to_string();
            let filename = format!("{sha}.{ext}");

            let (artifact_manifest, handle) = md_write_file_artifact_from_path(
                &workspace_root,
                crate::storage::artifacts::ArtifactLayer::L3,
                crate::storage::artifacts::ArtifactPayloadKind::File,
                mime,
                Some(filename.clone()),
                &file_path,
                crate::storage::artifacts::ArtifactClassification::Low,
                true,
                None,
                Some(job.job_id),
                Vec::new(),
                Vec::new(),
            )?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::DataBronzeCreated,
                    FlightRecorderActor::System,
                    trace_id,
                    json!({
                        "type": "data_bronze_created",
                        "bronze_id": artifact_manifest.artifact_id.to_string(),
                        "content_type": artifact_manifest.mime,
                        "content_hash": artifact_manifest.content_hash,
                        "size_bytes": artifact_manifest.size_bytes,
                        "ingestion_source": { "type": "system", "process": "media_downloader" },
                        "ingestion_method": "api_ingest",
                        "external_source": { "url": md_sanitize_url_string_for_telemetry(chosen_url.as_str()) }
                    }),
                )
                .with_job_id(job.job_id.to_string())
                .with_workflow_id(workflow_run_id.to_string()),
            )
            .await;

            let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
            let rel = format!(
                "media_downloader/forumcrawler/{}/{}",
                &item.item_id, filename
            );
            let mat =
                md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

            artifacts.push(handle);
            materialized_paths.push(mat.clone());
            bytes_downloaded_total = bytes_downloaded_total.saturating_add(bytes);
            images_done = images_done.saturating_add(1);

            if images_done % 25 == 0 {
                md_record_md_system_event(
                    state,
                    trace_id,
                    job.job_id,
                    workflow_run_id,
                    json!({
                        "event_kind": "media_downloader.progress",
                        "job_id": job.job_id.to_string(),
                        "source_kind": MdSourceKindV0::Forumcrawler.as_str(),
                        "url": md_sanitize_url_string_for_telemetry(&item.url_canonical),
                        "bytes_downloaded": bytes_downloaded_total,
                        "bytes_total": null,
                        "item_id": item.item_id.clone(),
                        "item_index": pages_crawled,
                        "item_total": max_pages,
                        "status": "running",
                        "error_code": null,
                        "pages_crawled": pages_crawled,
                        "images_downloaded": images_done,
                    }),
                )
                .await;
            }

            manifest_entries.push(MdForumManifestEntryV0 {
                page_url: md_sanitize_url_string_for_telemetry(page_url.as_str()),
                discovered_url: md_sanitize_url_string_for_telemetry(discovered_url.as_str()),
                chosen_url: md_sanitize_url_string_for_telemetry(chosen_url.as_str()),
                sha256: Some(sha),
                bytes: Some(bytes),
                status: "downloaded".to_string(),
                reason_skipped: None,
            });
        }
    }

    // Write manifest artifact (required).
    let manifest_path = tmp_dir.join("manifest.json");
    let manifest_json = serde_json::to_vec_pretty(&manifest_entries)
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
    tokio::fs::write(&manifest_path, &manifest_json)
        .await
        .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

    let (manifest_art, manifest_handle) = md_write_file_artifact_from_path(
        &workspace_root,
        crate::storage::artifacts::ArtifactLayer::L3,
        crate::storage::artifacts::ArtifactPayloadKind::DatasetSlice,
        "application/json".to_string(),
        Some("manifest.json".to_string()),
        &manifest_path,
        crate::storage::artifacts::ArtifactClassification::Low,
        true,
        None,
        Some(job.job_id),
        Vec::new(),
        Vec::new(),
    )?;

    let payload_abs = workspace_root.join(PathBuf::from(&manifest_handle.path));
    let rel = format!(
        "media_downloader/forumcrawler/{}/manifest.json",
        &item.item_id
    );
    let mat = md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

    artifacts.push(manifest_handle);
    materialized_paths.push(mat);

    // Also emit DataBronzeCreated for the manifest artifact.
    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::DataBronzeCreated,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "type": "data_bronze_created",
                "bronze_id": manifest_art.artifact_id.to_string(),
                "content_type": manifest_art.mime,
                "content_hash": manifest_art.content_hash,
                "size_bytes": manifest_art.size_bytes,
                "ingestion_source": { "type": "system", "process": "media_downloader" },
                "ingestion_method": "api_ingest",
                "external_source": { "url": md_sanitize_url_string_for_telemetry(&item.url_canonical) }
            }),
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string()),
    )
    .await;

    let mut seen = HashSet::new();
    artifacts.retain(|h| seen.insert(h.canonical_id()));
    materialized_paths.sort();
    materialized_paths.dedup();

    let cancelled = md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key);
    let status = if cancelled {
        "cancelled"
    } else if images_done == 0 {
        "failed"
    } else {
        "completed"
    };

    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(MdItemResultV0 {
        item_id: item.item_id.clone(),
        status: status.to_string(),
        artifact_handles: artifacts,
        materialized_paths,
        error_code: if cancelled {
            Some("cancelled".to_string())
        } else if images_done == 0 {
            Some("no_images_found".to_string())
        } else {
            None
        },
        error_message: None,
    })
}

#[allow(clippy::too_many_arguments)]
async fn md_process_item(
    state: &AppState,
    job: &AiJob,
    trace_id: Uuid,
    workflow_run_id: Uuid,
    mex_runtime: &MexRuntime,
    tools: &MdToolsV0,
    output_root_dir: &Path,
    source_kind: MdSourceKindV0,
    item: &MdPlanItemV0,
    cookie_jar_payload: Option<&Path>,
    forum_max_pages: usize,
    forum_allowlist_domains: &[String],
    job_cancel_key: &str,
) -> Result<MdItemResultV0, WorkflowError> {
    let item_cancel_key = md_item_cancel_key(job.job_id, &item.item_id);
    if md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
        return Ok(MdItemResultV0 {
            item_id: item.item_id.clone(),
            status: "cancelled".to_string(),
            artifact_handles: Vec::new(),
            materialized_paths: Vec::new(),
            error_code: Some("cancelled".to_string()),
            error_message: None,
        });
    }

    match source_kind {
        MdSourceKindV0::Youtube | MdSourceKindV0::Instagram => {
            let workspace_root = crate::storage::artifacts::resolve_workspace_root()
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;

            // Resumable + dedupe across runs: if this item was already archived into canonical
            // workspace artifacts, re-materialize as needed and skip re-downloading.
            let record_path = md_item_record_path(&workspace_root, source_kind, &item.item_id);
            if let Ok(raw) = fs::read_to_string(&record_path) {
                if let Ok(record) = serde_json::from_str::<MdItemRecordV0>(&raw) {
                    if record.schema_version == MD_ITEM_RECORD_SCHEMA_V0
                        && record.source_kind == source_kind
                        && record.item_id == item.item_id
                    {
                        let mut artifacts: Vec<ArtifactHandle> = Vec::new();
                        let mut materialized_paths: Vec<String> = Vec::new();
                        let mut can_resume = true;

                        for entry in &record.entries {
                            let payload_abs =
                                workspace_root.join(PathBuf::from(&entry.artifact_ref.path));
                            if !payload_abs.exists() {
                                can_resume = false;
                                break;
                            }

                            let target_abs =
                                output_root_dir.join(PathBuf::from(&entry.materialized_rel_path));
                            if !target_abs.exists() {
                                let mat = md_materialize_local_file_from_path(
                                    output_root_dir,
                                    &entry.materialized_rel_path,
                                    &payload_abs,
                                    true,
                                )?;
                                materialized_paths.push(mat);
                            } else {
                                materialized_paths.push(entry.materialized_rel_path.clone());
                            }

                            artifacts.push(entry.artifact_ref.clone());
                        }

                        if can_resume {
                            let mut seen = HashSet::new();
                            artifacts.retain(|h| seen.insert(h.canonical_id()));
                            materialized_paths.sort();
                            materialized_paths.dedup();

                            return Ok(MdItemResultV0 {
                                item_id: item.item_id.clone(),
                                status: "completed".to_string(),
                                artifact_handles: artifacts,
                                materialized_paths,
                                error_code: None,
                                error_message: None,
                            });
                        }
                    }
                }
            }

            let tmp_dir = workspace_root
                .join(".handshake")
                .join("tmp")
                .join("media_downloader")
                .join(job.job_id.to_string())
                .join(source_kind.as_str())
                .join(&item.item_id);
            fs::create_dir_all(&tmp_dir).map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let cwd_rel = tmp_dir
                .strip_prefix(&workspace_root)
                .map_err(|_| WorkflowError::Terminal("temp dir escapes workspace root".into()))?
                .to_string_lossy()
                .replace('\\', "/");

            let mut args: Vec<String> = vec![
                "--no-warnings".to_string(),
                "--no-call-home".to_string(),
                "--no-color".to_string(),
                "--restrict-filenames".to_string(),
                "--no-playlist".to_string(),
                "--continue".to_string(),
                "--ffmpeg-location".to_string(),
                tools
                    .ffmpeg_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_string_lossy()
                    .to_string(),
                "--merge-output-format".to_string(),
                "mp4".to_string(),
                "--format".to_string(),
                "bv*+ba/b".to_string(),
                "--write-subs".to_string(),
                "--write-auto-subs".to_string(),
                "--sub-format".to_string(),
                "vtt".to_string(),
                "--convert-subs".to_string(),
                "vtt".to_string(),
                "--sub-langs".to_string(),
                "all".to_string(),
                "--write-info-json".to_string(),
                "-o".to_string(),
                "%(id)s.%(ext)s".to_string(),
            ];
            if let Some(cookie) = cookie_jar_payload {
                args.push("--cookies".to_string());
                args.push(cookie.to_string_lossy().to_string());
            }
            args.push("--".to_string());
            args.push(item.url_canonical.clone());

            let result = md_mex_exec(
                mex_runtime,
                &job.capability_profile_id,
                "yt_dlp.exec",
                &tools.yt_dlp_path,
                &cwd_rel,
                args,
                vec![job_cancel_key.to_string(), item_cancel_key.clone()],
                3_600_000,
                50_000_000,
            )
            .await?;

            let (output_abs, _stdout_abs, stderr_abs) =
                md_terminal_output_paths(&workspace_root, &result)?;

            let terminal_raw = fs::read_to_string(&output_abs)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let terminal_val: Value = serde_json::from_str(&terminal_raw)
                .map_err(|e| WorkflowError::Terminal(e.to_string()))?;
            let exit_code = terminal_val
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1);
            let timed_out = terminal_val
                .get("timed_out")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let cancelled = terminal_val
                .get("cancelled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if cancelled || md_is_cancelled(job_cancel_key) || md_is_cancelled(&item_cancel_key) {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Ok(MdItemResultV0 {
                    item_id: item.item_id.clone(),
                    status: "cancelled".to_string(),
                    artifact_handles: Vec::new(),
                    materialized_paths: Vec::new(),
                    error_code: Some("cancelled".to_string()),
                    error_message: None,
                });
            }

            if timed_out {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Ok(MdItemResultV0 {
                    item_id: item.item_id.clone(),
                    status: "failed".to_string(),
                    artifact_handles: Vec::new(),
                    materialized_paths: Vec::new(),
                    error_code: Some("timed_out".to_string()),
                    error_message: Some("yt-dlp timed out".to_string()),
                });
            }

            if exit_code != 0 {
                let stderr = fs::read_to_string(&stderr_abs).unwrap_or_default();
                let excerpt = stderr.lines().take(20).collect::<Vec<_>>().join("\n");
                let msg = if excerpt.trim().is_empty() {
                    format!("yt-dlp exited with code {exit_code}")
                } else {
                    format!("yt-dlp exited with code {exit_code}: {excerpt}")
                };
                let _ = fs::remove_dir_all(&tmp_dir);
                return Ok(MdItemResultV0 {
                    item_id: item.item_id.clone(),
                    status: "failed".to_string(),
                    artifact_handles: Vec::new(),
                    materialized_paths: Vec::new(),
                    error_code: Some("tool_failed".to_string()),
                    error_message: Some(msg),
                });
            }

            let mut files = md_list_downloaded_files(&tmp_dir)?;

            // Captions language metadata sidecar (required when captions are present).
            let mut tracks: Vec<MdCaptionTrackV0> = Vec::new();
            for path in files.iter() {
                let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if path
                    .extension()
                    .and_then(|v| v.to_str())
                    .map(|v| v.eq_ignore_ascii_case("vtt"))
                    .unwrap_or(false)
                {
                    let lang = md_caption_lang_from_filename(filename)
                        .unwrap_or_else(|| "unknown".to_string());
                    tracks.push(MdCaptionTrackV0 {
                        lang,
                        filename: filename.to_string(),
                    });
                }
            }
            if !tracks.is_empty() {
                tracks.sort_by(|a, b| a.lang.cmp(&b.lang).then(a.filename.cmp(&b.filename)));
                let captions = MdCaptionsMetadataV0 {
                    schema_version: MD_CAPTIONS_METADATA_SCHEMA_V0.to_string(),
                    item_id: item.item_id.clone(),
                    source_kind,
                    url_canonical: item.url_canonical.clone(),
                    tracks,
                };
                let captions_path = tmp_dir.join("captions.metadata.json");
                if let Err(err) = write_json_atomic(&workspace_root, &captions_path, &captions) {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    return Ok(MdItemResultV0 {
                        item_id: item.item_id.clone(),
                        status: "failed".to_string(),
                        artifact_handles: Vec::new(),
                        materialized_paths: Vec::new(),
                        error_code: Some("state_write_failed".to_string()),
                        error_message: Some(format!("failed to write captions metadata: {err}")),
                    });
                }
                files.push(captions_path);
                files.sort();
            }

            if files.is_empty() {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Ok(MdItemResultV0 {
                    item_id: item.item_id.clone(),
                    status: "failed".to_string(),
                    artifact_handles: Vec::new(),
                    materialized_paths: Vec::new(),
                    error_code: Some("no_outputs".to_string()),
                    error_message: Some("yt-dlp produced no output files".to_string()),
                });
            }

            let source_entity_refs = vec![crate::storage::EntityRef {
                entity_id: md_sanitize_url_string_for_telemetry(&item.url_canonical),
                entity_kind: "external_url".to_string(),
            }];

            let mut artifacts: Vec<ArtifactHandle> = Vec::new();
            let mut materialized_paths: Vec<String> = Vec::new();
            let mut record_entries: Vec<MdItemRecordEntryV0> = Vec::new();

            for file_path in files {
                let filename = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("download.bin")
                    .to_string();
                let filename_lower = filename.to_ascii_lowercase();

                if filename_lower.ends_with(".info.json") {
                    let _ = md_sanitize_ytdlp_info_json(&file_path);
                }

                let mime = md_guess_mime(&file_path);
                let (role, lang, kind) = if filename_lower == "captions.metadata.json" {
                    (
                        "captions_metadata".to_string(),
                        None,
                        crate::storage::artifacts::ArtifactPayloadKind::ToolOutput,
                    )
                } else if file_path
                    .extension()
                    .and_then(|v| v.to_str())
                    .map(|v| v.eq_ignore_ascii_case("vtt"))
                    .unwrap_or(false)
                {
                    (
                        "caption_vtt".to_string(),
                        md_caption_lang_from_filename(&filename),
                        crate::storage::artifacts::ArtifactPayloadKind::Transcript,
                    )
                } else if filename_lower.ends_with(".info.json") {
                    (
                        "info_json".to_string(),
                        None,
                        crate::storage::artifacts::ArtifactPayloadKind::ToolOutput,
                    )
                } else if mime.starts_with("video/") || mime.starts_with("audio/") {
                    (
                        "media".to_string(),
                        None,
                        crate::storage::artifacts::ArtifactPayloadKind::File,
                    )
                } else if mime.starts_with("image/") {
                    (
                        "image".to_string(),
                        None,
                        crate::storage::artifacts::ArtifactPayloadKind::File,
                    )
                } else {
                    (
                        "file".to_string(),
                        None,
                        crate::storage::artifacts::ArtifactPayloadKind::File,
                    )
                };

                let (manifest, handle) = md_write_file_artifact_from_path(
                    &workspace_root,
                    crate::storage::artifacts::ArtifactLayer::L3,
                    kind,
                    mime,
                    Some(filename.clone()),
                    &file_path,
                    crate::storage::artifacts::ArtifactClassification::Low,
                    true,
                    None,
                    Some(job.job_id),
                    source_entity_refs.clone(),
                    Vec::new(),
                )?;

                record_event_safely(
                    state,
                    FlightRecorderEvent::new(
                        FlightRecorderEventType::DataBronzeCreated,
                        FlightRecorderActor::System,
                        trace_id,
                        json!({
                            "type": "data_bronze_created",
                            "bronze_id": manifest.artifact_id.to_string(),
                            "content_type": manifest.mime,
                            "content_hash": manifest.content_hash,
                            "size_bytes": manifest.size_bytes,
                            "ingestion_source": { "type": "system", "process": "media_downloader" },
                            "ingestion_method": "api_ingest",
                            "external_source": { "url": md_sanitize_url_string_for_telemetry(&item.url_canonical) }
                        }),
                    )
                    .with_job_id(job.job_id.to_string())
                    .with_workflow_id(workflow_run_id.to_string()),
                )
                .await;

                let payload_abs = workspace_root.join(PathBuf::from(&handle.path));
                let rel = format!(
                    "media_downloader/{}/{}/{}",
                    source_kind.output_subdir(),
                    item.item_id,
                    filename
                );
                let mat =
                    md_materialize_local_file_from_path(output_root_dir, &rel, &payload_abs, true)?;

                artifacts.push(handle.clone());
                materialized_paths.push(mat.clone());
                record_entries.push(MdItemRecordEntryV0 {
                    role,
                    lang,
                    artifact_ref: handle,
                    content_hash: manifest.content_hash,
                    mime: manifest.mime,
                    filename,
                    materialized_rel_path: mat,
                });
            }

            let mut seen = HashSet::new();
            artifacts.retain(|h| seen.insert(h.canonical_id()));
            materialized_paths.sort();
            materialized_paths.dedup();

            let record = MdItemRecordV0 {
                schema_version: MD_ITEM_RECORD_SCHEMA_V0.to_string(),
                source_kind,
                item_id: item.item_id.clone(),
                url_canonical: item.url_canonical.clone(),
                created_at: Utc::now().to_rfc3339(),
                entries: record_entries,
            };
            let record_path = md_item_record_path(&workspace_root, source_kind, &item.item_id);
            if let Err(err) = write_json_atomic(&workspace_root, &record_path, &record) {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Ok(MdItemResultV0 {
                    item_id: item.item_id.clone(),
                    status: "failed".to_string(),
                    artifact_handles: artifacts,
                    materialized_paths,
                    error_code: Some("state_write_failed".to_string()),
                    error_message: Some(format!("failed to write item record: {err}")),
                });
            }

            let _ = fs::remove_dir_all(&tmp_dir);

            Ok(MdItemResultV0 {
                item_id: item.item_id.clone(),
                status: "completed".to_string(),
                artifact_handles: artifacts,
                materialized_paths,
                error_code: None,
                error_message: None,
            })
        }
        MdSourceKindV0::Videodownloader => {
            md_download_generic_video(
                state,
                job,
                trace_id,
                workflow_run_id,
                mex_runtime,
                tools,
                output_root_dir,
                item,
                cookie_jar_payload,
                job_cancel_key,
            )
            .await
        }
        MdSourceKindV0::Forumcrawler => {
            md_crawl_forum_images(
                state,
                job,
                trace_id,
                workflow_run_id,
                output_root_dir,
                item,
                forum_max_pages,
                forum_allowlist_domains,
                job_cancel_key,
            )
            .await
        }
    }
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
