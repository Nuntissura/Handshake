use crate::{
    ace::{
        validators::{
            build_query_plan_from_blocks, build_retrieval_trace_from_blocks,
            scan_content_for_security, SecurityViolationType, StorageContentResolver,
            ValidatorPipeline,
        },
        AceError, CandidateRef, SourceRef,
    },
    bundles::{BundleScope, DebugBundleRequest, DefaultDebugBundleExporter, RedactionMode},
    capabilities::RegistryError,
    flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
        FrEvt008SecurityViolation, FrEvtWorkflowRecovery,
    },
    llm::{CompletionRequest, LlmError},
    models::{AiJob, JobKind, WorkflowRun},
    storage::{JobState, JobStatusUpdate, NewNodeExecution, StorageError},
    terminal::{
        config::TerminalConfig,
        guards::{DefaultTerminalGuard, TerminalGuard},
        redaction::PatternRedactor,
        JobContext, TerminalMode, TerminalRequest, TerminalService,
    },
    AppState,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Mutex,
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
/// Per §2.6.6.7.11.0, security violations trigger immediate job poisoning
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

    let (final_status, error_message, output_payload, captured_error) = match result {
        Ok(output) => (JobState::Completed, None, output, None),
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
            (JobState::Failed, Some(msg.clone()), None, Some(e))
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
            status_reason: error_message
                .clone()
                .unwrap_or_else(|| "completed".to_string()),
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

async fn log_capability_check(
    state: &AppState,
    job: &AiJob,
    capability_id: &str,
    profile_id: &str,
    outcome: &str,
    trace_id: Uuid,
) {
    let payload = json!({
        "capability_id": capability_id,
        "profile_id": profile_id,
        "job_id": job.job_id,
        "outcome": outcome,
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
        .with_actor_id(job.capability_profile_id.clone()),
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
        .required_capabilities_for_job(job.job_kind.as_str())?;

    for capability_id in required {
        let result = state
            .capability_registry
            .profile_can(&job.capability_profile_id, &capability_id);

        match result {
            Ok(true) => {
                log_capability_check(
                    state,
                    job,
                    &capability_id,
                    &job.capability_profile_id,
                    "allowed",
                    trace_id,
                )
                .await;
            }
            Ok(false) => {
                log_capability_check(
                    state,
                    job,
                    &capability_id,
                    &job.capability_profile_id,
                    "denied",
                    trace_id,
                )
                .await;
                return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                    capability_id,
                )));
            }
            Err(err) => {
                log_capability_check(
                    state,
                    job,
                    &capability_id,
                    &job.capability_profile_id,
                    "denied",
                    trace_id,
                )
                .await;
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
) -> Result<Option<Value>, WorkflowError> {
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

    if matches!(job.job_kind, JobKind::DocSummarize | JobKind::DocEdit) {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let doc_id = inputs.get("doc_id").and_then(|v| v.as_str());

        if let Some(doc_id) = doc_id {
            let blocks = state.storage.get_blocks(doc_id).await?;
            let model_tier = state.llm_client.profile().model_tier;

            // [§2.6.6.7.14.5] Build QueryPlan and RetrievalTrace
            // MUST fail on invalid UUIDs - returns Result
            let plan = build_query_plan_from_blocks(&blocks, "summarize document", "doc_summarization")
                .map_err(WorkflowError::SecurityViolation)?;
            let trace = build_retrieval_trace_from_blocks(&blocks, &plan)
                .map_err(WorkflowError::SecurityViolation)?;

            // WAIVER [CX-573F]: Instant::now() for observability per §2.6.6.7.12
            let validation_start = std::time::Instant::now();

            // [§2.6.6.7.14.11] Run ValidatorPipeline
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
            let source_refs: Vec<SourceRef> = trace
                .spans
                .iter()
                .map(|s| s.source_ref.clone())
                .collect();

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
                return Err(WorkflowError::SecurityViolation(AceError::ValidationFailed {
                    message: violation_codes.join("; "),
                }));
            }

            // Build prompt and full text
            let full_text = blocks
                .iter()
                .map(|b| b.raw_content.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!("Please summarize the following document:\n\n{}", full_text);

            // Compute hashes for logging
            let prompt_envelope_hash = {
                let mut h = Sha256::new();
                h.update(prompt.as_bytes());
                hex::encode(h.finalize())
            };

            let scope_inputs_hash = {
                let scope_json = serde_json::to_string(&job.job_inputs)
                    .map_err(|e| WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("job_inputs serialization failed: {}", e),
                    }))?;
                let mut h = Sha256::new();
                h.update(scope_json.as_bytes());
                hex::encode(h.finalize())
            };

            let query_plan_hash = {
                let plan_json = serde_json::to_string(&plan)
                    .map_err(|e| WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("QueryPlan serialization failed: {}", e),
                    }))?;
                let mut h = Sha256::new();
                h.update(plan_json.as_bytes());
                hex::encode(h.finalize())
            };

            let retrieval_trace_hash = {
                let trace_json = serde_json::to_string(&trace)
                    .map_err(|e| WorkflowError::SecurityViolation(AceError::ValidationFailed {
                        message: format!("RetrievalTrace serialization failed: {}", e),
                    }))?;
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

            // Build extended LlmInference payload with §2.6.6.7.12 ACE validation fields
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

                        // §2.6.6.7.12 ACE validation fields
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
                .with_model_id(model_id),
            )
            .await;

            state
                .storage
                .set_job_outputs(
                    &job.job_id.to_string(),
                    Some(json!({ "summary": response.text })),
                )
                .await?;
            return Ok(Some(json!({ "summary": response.text })));
        }
    } else if matches!(job.job_kind, JobKind::TerminalExec) {
        let payload = execute_terminal_job(state, job, trace_id).await?;
        state
            .storage
            .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
            .await?;
        return Ok(Some(payload));
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
                    log_capability_check(
                        state,
                        job,
                        capability_id,
                        &job.capability_profile_id,
                        "allowed",
                        trace_id,
                    )
                    .await;
                }
                Ok(false) => {
                    log_capability_check(
                        state,
                        job,
                        capability_id,
                        &job.capability_profile_id,
                        "denied",
                        trace_id,
                    )
                    .await;
                    return Err(WorkflowError::Capability(RegistryError::AccessDenied(
                        capability_id.to_string(),
                    )));
                }
                Err(err) => {
                    log_capability_check(
                        state,
                        job,
                        capability_id,
                        &job.capability_profile_id,
                        "denied",
                        trace_id,
                    )
                    .await;
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
        return Ok(Some(manifest_value));
    }
    Ok(None)
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
        let recovery_event = events
            .iter()
            .find(|e| e.event_type == FlightRecorderEventType::WorkflowRecovery);

        assert!(
            recovery_event.is_some(),
            "Recovery event not found in Flight Recorder"
        );
        let event = recovery_event.unwrap();
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
            .create_workspace(&ctx, NewWorkspace { name: "test-ws".into() })
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
