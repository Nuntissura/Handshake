use crate::{
    ace::{
        validators::{
            build_query_plan_from_blocks, build_retrieval_trace_from_blocks,
            scan_content_for_security, SecurityViolationType, StorageContentResolver,
            ValidatorPipeline,
        },
        AceError, ArtifactHandle, CandidateRef, CandidateScores, DeterminismMode, QueryKind,
        QueryPlan, RetrievalCandidate, RetrievalTrace, RouteTaken, SelectedEvidence, SourceRef,
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
    mex::runtime::ShellEngineAdapter,
    mex::{
        BudgetGate, BudgetSpec, CapabilityGate, DetGate, DeterminismLevel, EvidencePolicy,
        GatePipeline, IntegrityGate, MexRegistry, MexRuntime, OutputSpec, PlannedOperation,
        ProvenanceGate, SchemaGate, POE_SCHEMA_VERSION,
    },
    models::{AiJob, JobKind, WorkflowRun},
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

// ============================================================================
// Model Swap Protocol [§4.3.3.4.3-4.3.3.4.4]
// ============================================================================

const MODEL_SWAP_SCHEMA_VERSION_V0_4: &str = "hsk.model_swap@0.4";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ModelSwapRole {
    Frontend,
    Orchestrator,
    Worker,
    Validator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ModelSwapPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ModelSwapStrategy {
    UnloadReload,
    KeepHotSwap,
    DiskOffload,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ModelSwapRequesterSubsystem {
    MtExecutor,
    Governance,
    Ui,
    Orchestrator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ModelSwapRequesterV0_4 {
    pub subsystem: ModelSwapRequesterSubsystem,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub wp_id: Option<String>,
    #[serde(default)]
    pub mt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ModelSwapRequestV0_4 {
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

    let state_hash = write_json_atomic_with_hash(&state_abs, &state)?;
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
    write_json_atomic(&request_abs, request)?;
    Ok(request_abs)
}

fn verify_model_swap_state_hash_v0_4(
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
    let actual = sha256_hex(&bytes);
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
                                })
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
                            })
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
                            })
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
                        })
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionPolicy {
    #[serde(default = "default_max_iterations_per_mt")]
    pub max_iterations_per_mt: u32,
    #[serde(default = "default_max_total_iterations")]
    pub max_total_iterations: u32,
    #[serde(default = "default_max_duration_ms")]
    pub max_duration_ms: u64,
    #[serde(default = "default_escalation_chain")]
    pub escalation_chain: Vec<EscalationLevel>,
    #[serde(default)]
    pub cloud_escalation_allowed: bool,
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
            escalation_chain: default_escalation_chain(),
            cloud_escalation_allowed: false,
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
struct CurrentExecutionState {
    #[serde(default)]
    pub active_mt: Option<String>,
    pub active_model_level: u32,
    pub total_iterations: u32,
    pub total_escalations: u32,
    #[serde(default)]
    pub total_model_swaps: u32,
    pub total_drop_backs: u32,
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

fn write_bytes_atomic(abs_path: &Path, bytes: &[u8]) -> Result<(), WorkflowError> {
    let Some(parent) = abs_path.parent() else {
        return Err(WorkflowError::Terminal(format!(
            "invalid path for atomic write: {}",
            abs_path.display()
        )));
    };
    fs::create_dir_all(parent).map_err(|e| {
        WorkflowError::Terminal(format!("failed to create {}: {e}", parent.display()))
    })?;

    let tmp = abs_path.with_extension("tmp");
    fs::write(&tmp, bytes)
        .map_err(|e| WorkflowError::Terminal(format!("failed to write {}: {e}", tmp.display())))?;

    if abs_path.exists() {
        fs::remove_file(abs_path).map_err(|e| {
            WorkflowError::Terminal(format!("failed to remove {}: {e}", abs_path.display()))
        })?;
    }
    fs::rename(&tmp, abs_path).map_err(|e| {
        WorkflowError::Terminal(format!(
            "failed to rename {} -> {}: {e}",
            tmp.display(),
            abs_path.display()
        ))
    })?;
    Ok(())
}

fn write_json_atomic<T: Serialize>(abs_path: &Path, value: &T) -> Result<(), WorkflowError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| WorkflowError::Terminal(format!("json serialize error: {e}")))?;
    write_bytes_atomic(abs_path, &bytes)
}

fn write_json_atomic_with_hash<T: Serialize>(
    abs_path: &Path,
    value: &T,
) -> Result<String, WorkflowError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| WorkflowError::Terminal(format!("json serialize error: {e}")))?;
    let hash = sha256_hex(&bytes);
    write_bytes_atomic(abs_path, &bytes)?;
    Ok(hash)
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
    repo_root: &Path,
    path: &str,
    query_terms: &[String],
    max_results: usize,
    neighbor_lines: usize,
    allowance_tokens: u32,
    max_span_tokens: u32,
) -> Result<ShadowWsLexicalOutcome, WorkflowError> {
    let abs = repo_root.join(PathBuf::from(path));
    let bytes = match fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(ShadowWsLexicalOutcome::Skipped {
                warning: format!("missing_or_unreadable_file:{path}"),
            });
        }
    };

    let source_hash = sha256_hex(&bytes);
    let source_id = deterministic_uuid_for_str(path);
    let source_ref = SourceRef::new(source_id, source_hash.clone());

    let content = String::from_utf8_lossy(&bytes);
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
        source_id,
        source_hash,
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
        let mut timed_out = false;
        let mut duration_ms = 0u64;
        let mut stdout_ref: Option<ArtifactHandle> = None;
        let mut stderr_ref: Option<ArtifactHandle> = None;
        let mut error: Option<String> = None;

        let mut stdout_rel: Option<String> = None;
        let mut stderr_rel: Option<String> = None;

        if let Some(output_handle) = result.outputs.first() {
            let output_abs = repo_root.join(PathBuf::from(&output_handle.path));
            match fs::read_to_string(&output_abs) {
                Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                    Ok(val) => {
                        actual_exit_code =
                            val.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
                        timed_out = val
                            .get("timed_out")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        duration_ms = val.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

                        stdout_rel = val
                            .get("stdout_ref")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        stderr_rel = val
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
    write_json_atomic(&repo_root.join(evidence_artifact_rel), &evidence_artifact)?;

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
    let policy = inputs.execution_policy.unwrap_or_default();

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
    let job_dir_rel = micro_task_job_dir_rel(job.job_id);
    let job_dir_abs = repo_root.join(&job_dir_rel);
    fs::create_dir_all(&job_dir_abs).map_err(|e| {
        WorkflowError::Terminal(format!("failed to create {}: {e}", job_dir_abs.display()))
    })?;

    let mt_defs_rel = job_dir_rel.join("mt_definitions.json");
    let mt_defs_abs = repo_root.join(&mt_defs_rel);
    write_json_atomic(&mt_defs_abs, &mt_definitions)?;
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
        write_json_atomic(&progress_abs, &progress)?;
        write_json_atomic(&run_ledger_abs, &run_ledger)?;
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
        write_json_atomic(&progress_abs, &progress)?;
        write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
            write_json_atomic(&progress_abs, &progress)?;
            write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                progress.status = ProgressStatus::Paused;
                progress.current_state.active_mt = Some(mt_id.clone());
                progress.updated_at = Utc::now();
                run_ledger.resume_reason = Some(format!("pause_point:{}", mt.mt_id));
                write_json_atomic(&progress_abs, &progress)?;
                write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
            write_json_atomic(&progress_abs, &progress)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
                    reason: "prefer_context_packs=true; unavailable".to_string(),
                    cache_hit: Some(false),
                });
                retrieval_trace.route_taken.push(RouteTaken {
                    store: StoreKind::ShadowWsLexical,
                    reason: "shadow_ws_lexical:deterministic_chunks".to_string(),
                    cache_hit: Some(false),
                });

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
                    let outcome = retrieve_shadow_ws_lexical_for_file(
                        &repo_root,
                        path,
                        &query_terms,
                        max_shadow_results,
                        neighbor_lines,
                        allowance,
                        query_plan.budgets.max_read_tokens,
                    )?;

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

                write_json_atomic(&repo_root.join(&ace_query_plan_rel), &query_plan)?;
                write_json_atomic(&repo_root.join(&ace_retrieval_trace_rel), &retrieval_trace)?;

                let context_files_artifact = MtContextFilesArtifact {
                    schema_version: "1.0".to_string(),
                    mt_id: mt.mt_id.clone(),
                    iteration,
                    files: selected_files.clone(),
                };
                write_json_atomic(&repo_root.join(&context_files_rel), &context_files_artifact)?;

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
                        ace_query_plan_ref,
                        ace_retrieval_trace_ref,
                    ],
                    warnings,
                    local_only_payload_ref: Some(prompt_snapshot_ref.clone()),
                };
                let context_snapshot_hash =
                    write_json_atomic_with_hash(&repo_root.join(&context_snapshot_rel), &snapshot)?;

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
                write_json_atomic(&run_ledger_abs, &run_ledger)?;

                write_bytes_atomic(&repo_root.join(&prompt_rel), prompt.as_bytes())?;

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

                write_bytes_atomic(&repo_root.join(&response_rel), response.text.as_bytes())?;
                if let Some(out) = &validation_outcome {
                    write_json_atomic(&repo_root.join(&validation_rel), out)?;
                }

                let output_artifact_ref = artifact_handle_for_rel(&output_rel);
                write_json_atomic(
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
                write_json_atomic(&progress_abs, &progress)?;
                write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
                write_json_atomic(&progress_abs, &progress)?;
                write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    progress.status = ProgressStatus::Paused;
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
                    let request_rel = swap_dir_rel.join(format!("request_{}.json", &request_id));
                    let state_rel = swap_dir_rel.join(format!("swap_state_{}.json", &request_id));
                    let request_ref = rel_path_string(&request_rel);
                    let state_ref = rel_path_string(&state_rel);

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
                            rel_path_string(&progress_rel),
                            rel_path_string(&run_ledger_rel),
                            request_ref,
                            state_ref,
                            output_artifact_ref.canonical_id(),
                            context_snapshot_ref.canonical_id(),
                        ],
                        state_hash: "0".repeat(64),
                        context_compile_ref: context_snapshot_ref.canonical_id(),
                        max_vram_mb: 4096,
                        max_ram_mb: 32768,
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
                    verify_model_swap_state_hash_v0_4(&state_path, &swap_request.state_hash)?;
                    persist_model_swap_request_v0_4(&repo_root, job.job_id, &swap_request)?;

                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::ModelSwapRequested,
                            FlightRecorderActor::System,
                            trace_id,
                            json!({
                                "type": "model_swap_requested",
                                "request_id": swap_request.request_id.clone(),
                                "current_model_id": swap_request.current_model_id.clone(),
                                "target_model_id": swap_request.target_model_id.clone(),
                                "role": "worker",
                                "reason": swap_request.reason.clone(),
                                "swap_strategy": "unload_reload",
                                "max_vram_mb": swap_request.max_vram_mb,
                                "max_ram_mb": swap_request.max_ram_mb,
                                "timeout_ms": swap_request.timeout_ms,
                                "state_persist_refs": swap_request.state_persist_refs.clone(),
                                "state_hash": swap_request.state_hash.clone(),
                                "context_compile_ref": swap_request.context_compile_ref.clone(),
                                "wp_id": inputs.wp_id.clone(),
                                "mt_id": mt.mt_id.clone(),
                            }),
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;

                    progress.current_state.total_model_swaps =
                        progress.current_state.total_model_swaps.saturating_add(1);
                    progress.updated_at = Utc::now();
                    write_json_atomic(&progress_abs, &progress)?;
                    write_json_atomic(&run_ledger_abs, &run_ledger)?;

                    let swap_limit_exceeded = swap_policy.max_swaps_per_job == 0
                        || progress.current_state.total_model_swaps > swap_policy.max_swaps_per_job;
                    if !swap_policy.allow_swaps || swap_limit_exceeded {
                        let error_summary = if !swap_policy.allow_swaps {
                            "swap_disallowed_by_policy"
                        } else {
                            "swap_limit_exceeded"
                        };

                        record_event_safely(
                            state,
                            FlightRecorderEvent::new(
                                FlightRecorderEventType::ModelSwapFailed,
                                FlightRecorderActor::System,
                                trace_id,
                                json!({
                                    "type": "model_swap_failed",
                                    "request_id": swap_request.request_id.clone(),
                                    "current_model_id": swap_request.current_model_id.clone(),
                                    "target_model_id": swap_request.target_model_id.clone(),
                                    "role": "worker",
                                    "reason": swap_request.reason.clone(),
                                    "swap_strategy": "unload_reload",
                                    "max_vram_mb": swap_request.max_vram_mb,
                                    "max_ram_mb": swap_request.max_ram_mb,
                                    "timeout_ms": swap_request.timeout_ms,
                                    "state_persist_refs": swap_request.state_persist_refs.clone(),
                                    "state_hash": swap_request.state_hash.clone(),
                                    "context_compile_ref": swap_request.context_compile_ref.clone(),
                                    "wp_id": inputs.wp_id.clone(),
                                    "mt_id": mt.mt_id.clone(),
                                    "outcome": "failure",
                                    "error_summary": error_summary,
                                }),
                            )
                            .with_job_id(job.job_id.to_string())
                            .with_workflow_id(workflow_run_id.to_string()),
                        )
                        .await;

                        if matches!(
                            swap_policy.fallback_strategy,
                            ModelSwapFallbackStrategy::ContinueWithCurrent
                        ) {
                            record_event_safely(
                                state,
                                FlightRecorderEvent::new(
                                    FlightRecorderEventType::ModelSwapRollback,
                                    FlightRecorderActor::System,
                                    trace_id,
                                    json!({
                                        "type": "model_swap_rollback",
                                        "request_id": swap_request.request_id.clone(),
                                        "current_model_id": swap_request.current_model_id.clone(),
                                        "target_model_id": swap_request.target_model_id.clone(),
                                        "role": "worker",
                                        "reason": swap_request.reason.clone(),
                                        "swap_strategy": "unload_reload",
                                        "max_vram_mb": swap_request.max_vram_mb,
                                        "max_ram_mb": swap_request.max_ram_mb,
                                        "timeout_ms": swap_request.timeout_ms,
                                        "state_persist_refs": swap_request.state_persist_refs.clone(),
                                        "state_hash": swap_request.state_hash.clone(),
                                        "context_compile_ref": swap_request.context_compile_ref.clone(),
                                        "wp_id": inputs.wp_id.clone(),
                                        "mt_id": mt.mt_id.clone(),
                                        "outcome": "rollback",
                                        "error_summary": error_summary,
                                    }),
                                )
                                .with_job_id(job.job_id.to_string())
                                .with_workflow_id(workflow_run_id.to_string()),
                            )
                            .await;

                            progress.current_state.active_model_level = from_level;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&progress_abs, &progress)?;
                            write_json_atomic(&run_ledger_abs, &run_ledger)?;

                            false_completion_streak = 0;
                            iteration = 1;
                            continue;
                        }

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&progress_abs, &progress)?;
                        write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                        record_event_safely(
                            state,
                            FlightRecorderEvent::new(
                                FlightRecorderEventType::ModelSwapTimeout,
                                FlightRecorderActor::System,
                                trace_id,
                                json!({
                                    "type": "model_swap_timeout",
                                    "request_id": swap_request.request_id.clone(),
                                    "current_model_id": swap_request.current_model_id.clone(),
                                    "target_model_id": swap_request.target_model_id.clone(),
                                    "role": "worker",
                                    "reason": swap_request.reason.clone(),
                                    "swap_strategy": "unload_reload",
                                    "max_vram_mb": swap_request.max_vram_mb,
                                    "max_ram_mb": swap_request.max_ram_mb,
                                    "timeout_ms": swap_request.timeout_ms,
                                    "state_persist_refs": swap_request.state_persist_refs.clone(),
                                    "state_hash": swap_request.state_hash.clone(),
                                    "context_compile_ref": swap_request.context_compile_ref.clone(),
                                    "wp_id": inputs.wp_id.clone(),
                                    "mt_id": mt.mt_id.clone(),
                                    "outcome": "timeout",
                                    "error_summary": error_summary,
                                }),
                            )
                            .with_job_id(job.job_id.to_string())
                            .with_workflow_id(workflow_run_id.to_string()),
                        )
                        .await;

                        if matches!(
                            swap_policy.fallback_strategy,
                            ModelSwapFallbackStrategy::ContinueWithCurrent
                        ) {
                            record_event_safely(
                                state,
                                FlightRecorderEvent::new(
                                    FlightRecorderEventType::ModelSwapRollback,
                                    FlightRecorderActor::System,
                                    trace_id,
                                    json!({
                                        "type": "model_swap_rollback",
                                        "request_id": swap_request.request_id.clone(),
                                        "current_model_id": swap_request.current_model_id.clone(),
                                        "target_model_id": swap_request.target_model_id.clone(),
                                        "role": "worker",
                                        "reason": swap_request.reason.clone(),
                                        "swap_strategy": "unload_reload",
                                        "max_vram_mb": swap_request.max_vram_mb,
                                        "max_ram_mb": swap_request.max_ram_mb,
                                        "timeout_ms": swap_request.timeout_ms,
                                        "state_persist_refs": swap_request.state_persist_refs.clone(),
                                        "state_hash": swap_request.state_hash.clone(),
                                        "context_compile_ref": swap_request.context_compile_ref.clone(),
                                        "wp_id": inputs.wp_id.clone(),
                                        "mt_id": mt.mt_id.clone(),
                                        "outcome": "rollback",
                                        "error_summary": error_summary,
                                    }),
                                )
                                .with_job_id(job.job_id.to_string())
                                .with_workflow_id(workflow_run_id.to_string()),
                            )
                            .await;

                            progress.current_state.active_model_level = from_level;
                            progress.updated_at = Utc::now();
                            write_json_atomic(&progress_abs, &progress)?;
                            write_json_atomic(&run_ledger_abs, &run_ledger)?;

                            false_completion_streak = 0;
                            iteration = 1;
                            continue;
                        }

                        progress.status = ProgressStatus::Failed;
                        progress.updated_at = Utc::now();
                        write_json_atomic(&progress_abs, &progress)?;
                        write_json_atomic(&run_ledger_abs, &run_ledger)?;

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

                    record_event_safely(
                        state,
                        FlightRecorderEvent::new(
                            FlightRecorderEventType::ModelSwapCompleted,
                            FlightRecorderActor::System,
                            trace_id,
                            json!({
                                "type": "model_swap_completed",
                                "request_id": swap_request.request_id.clone(),
                                "current_model_id": swap_request.current_model_id.clone(),
                                "target_model_id": swap_request.target_model_id.clone(),
                                "role": "worker",
                                "reason": swap_request.reason.clone(),
                                "swap_strategy": "unload_reload",
                                "max_vram_mb": swap_request.max_vram_mb,
                                "max_ram_mb": swap_request.max_ram_mb,
                                "timeout_ms": swap_request.timeout_ms,
                                "state_persist_refs": swap_request.state_persist_refs.clone(),
                                "state_hash": swap_request.state_hash.clone(),
                                "context_compile_ref": swap_request.context_compile_ref.clone(),
                                "wp_id": inputs.wp_id.clone(),
                                "mt_id": mt.mt_id.clone(),
                                "outcome": "success",
                            }),
                        )
                        .with_job_id(job.job_id.to_string())
                        .with_workflow_id(workflow_run_id.to_string()),
                    )
                    .await;
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
    write_json_atomic(&progress_abs, &progress)?;
    write_json_atomic(&run_ledger_abs, &run_ledger)?;

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
    fn model_swap_state_persist_and_verify_v0_4_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let job_id = Uuid::new_v4();

        let request = ModelSwapRequestV0_4 {
            schema_version: MODEL_SWAP_SCHEMA_VERSION_V0_4.to_string(),
            request_id: "req-2".to_string(),
            current_model_id: "qwen2.5-coder:7b".to_string(),
            target_model_id: "qwen2.5-coder:13b".to_string(),
            role: ModelSwapRole::Worker,
            priority: ModelSwapPriority::High,
            reason: "context_overflow".to_string(),
            swap_strategy: ModelSwapStrategy::UnloadReload,
            state_persist_refs: vec!["artifact:locus_checkpoint.json".to_string()],
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

        let (state_path, state_hash) =
            persist_model_swap_state_v0_4(tmp.path(), job_id, &request).expect("persist");
        assert!(state_path.exists(), "state_path should exist");
        assert!(
            is_sha256_hex_lowercase(&state_hash),
            "hash must be lowercase sha256"
        );

        let raw = fs::read(&state_path).expect("read persisted state");
        let json: Value = serde_json::from_slice(&raw).expect("valid json");
        let map = json.as_object().expect("expected object");
        assert!(
            !map.contains_key("state_hash"),
            "state must not embed state_hash"
        );

        verify_model_swap_state_hash_v0_4(&state_path, &state_hash).expect("verify");
        let bad_hash = "1".repeat(64);
        assert!(
            verify_model_swap_state_hash_v0_4(&state_path, &bad_hash).is_err(),
            "expected hash mismatch to fail"
        );
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
