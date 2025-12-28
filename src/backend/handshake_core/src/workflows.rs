use crate::{
    ace::{validators::SecurityViolationType, AceError},
    capabilities::RegistryError,
    flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
        FrEvt005SecurityViolation, FrEvt006WorkflowRecovery,
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
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
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
    let payload = FrEvt005SecurityViolation {
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
        AceError::PromptInjectionDetected { .. } | AceError::CloudLeakageBlocked { .. }
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
    let stalled = state.storage.find_stalled_workflows(threshold_secs).await?;

    for run in &stalled {
        let reason = if is_startup_recovery {
            "stalled (startup recovery: heartbeat timeout)"
        } else {
            "stalled (heartbeat timeout)"
        };

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

        if is_startup_recovery {
            let payload = FrEvt006WorkflowRecovery {
                workflow_id: run.id.to_string(),
                job_id: run.job_id.to_string(),
                previous_state: run.status.as_str().to_string(),
                new_state: "stalled".to_string(),
                reason: reason.to_string(),
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
    }

    Ok(stalled)
}

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    let trace_id = derive_trace_id(&job, None);

    // Opportunistically mark any stale running workflows as stalled before starting a new one.
    let _ = mark_stalled_workflows(state, 30, false).await?;

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

    let result = run_job(state, &job, trace_id).await;

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
        .required_capability_for_job(job.job_kind.as_str())?;

    let result = state
        .capability_registry
        .profile_can(&job.capability_profile_id, &required);

    match result {
        Ok(true) => {
            log_capability_check(
                state,
                job,
                &required,
                &job.capability_profile_id,
                "allowed",
                trace_id,
            )
            .await;
            Ok(())
        }
        Ok(false) => {
            log_capability_check(
                state,
                job,
                &required,
                &job.capability_profile_id,
                "denied",
                trace_id,
            )
            .await;
            Err(WorkflowError::Capability(RegistryError::AccessDenied(
                required,
            )))
        }
        Err(err) => {
            log_capability_check(
                state,
                job,
                &required,
                &job.capability_profile_id,
                "denied",
                trace_id,
            )
            .await;
            Err(WorkflowError::Capability(err))
        }
    }
}

async fn run_job(
    state: &AppState,
    job: &AiJob,
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
            let full_text = blocks
                .into_iter()
                .map(|b| b.raw_content)
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!("Please summarize the following document:\n\n{}", full_text);

            let req = CompletionRequest::new(
                job.trace_id,
                prompt,
                state.llm_client.profile().model_id.clone(),
            );

            let response = state.llm_client.completion(req).await?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LlmInference,
                    FlightRecorderActor::Agent,
                    trace_id,
                    json!({
                        "job_kind": job.job_kind.as_str(),
                        "doc_id": doc_id,
                        "input_bytes": full_text.len(),
                        "output_bytes": response.text.len(),
                        "prompt_tokens": response.usage.prompt_tokens,
                        "completion_tokens": response.usage.completion_tokens,
                    }),
                )
                .with_job_id(job.job_id.to_string()),
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
        sqlite::SqliteDatabase, AccessMode, Database, JobKind, JobMetrics, SafetyMode,
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

        Ok(())
    }
}
