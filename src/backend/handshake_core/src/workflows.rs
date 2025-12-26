use crate::{
    capabilities::RegistryError,
    flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType},
    llm::ChatMessage,
    models::{AiJob, WorkflowRun},
    storage::{JobState, JobStatusUpdate, NewNodeExecution, StorageError},
    terminal::{TerminalError, TerminalService},
    AppState,
};
use chrono::Utc;
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Capability error: {0}")]
    Capability(#[from] RegistryError),
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

pub async fn mark_stalled_workflows(
    state: &AppState,
    threshold_secs: u64,
) -> Result<Vec<WorkflowRun>, WorkflowError> {
    let stalled = state.storage.find_stalled_workflows(threshold_secs).await?;

    for run in &stalled {
        let _ = state
            .storage
            .update_workflow_run_status(
                run.id,
                JobState::Stalled,
                Some("stalled (heartbeat timeout)".to_string()),
            )
            .await?;

        let _ = state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: run.job_id,
                state: JobState::Stalled,
                error_message: Some("workflow stalled (heartbeat timeout)".to_string()),
                status_reason: "stalled".to_string(),
                metrics: None,
                workflow_run_id: Some(run.id),
                trace_id: None,
                job_outputs: None,
            })
            .await?;
    }

    Ok(stalled)
}

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    let trace_id = derive_trace_id(&job, None);

    // Opportunistically mark any stale running workflows as stalled before starting a new one.
    let _ = mark_stalled_workflows(state, 30).await?;

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
            node_type: job.job_kind.clone(),
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
        .with_job_id(job.job_id.to_string()),
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
        .required_capability_for_job(&job.job_kind)?;

    let result = state
        .capability_registry
        .profile_can(&job.capability_profile_id, &required);

    match result {
        Ok(_) => {
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
    if job.job_kind == "doc_test" || job.job_kind == "doc_summarize" {
        let inputs = parse_inputs(job.job_inputs.as_ref());
        let doc_id = inputs.get("doc_id").and_then(|v| v.as_str());

        if let Some(doc_id) = doc_id {
            let blocks = state.storage.get_blocks(doc_id).await?;
            let full_text = blocks
                .into_iter()
                .map(|b| b.raw_content)
                .collect::<Vec<_>>()
                .join("\n");

            let messages = vec![
                ChatMessage {
                    role: "system".into(),
                    content: "You are a helpful assistant that summarizes documents.".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!("Please summarize the following document:\n\n{}", full_text),
                },
            ];

            let response = state
                .llm_client
                .chat(messages)
                .await
                .map_err(WorkflowError::Llm)?;

            record_event_safely(
                state,
                FlightRecorderEvent::new(
                    FlightRecorderEventType::LlmInference,
                    FlightRecorderActor::Agent,
                    trace_id,
                    json!({
                        "job_kind": job.job_kind,
                        "doc_id": doc_id,
                        "input_bytes": full_text.len(),
                        "output_bytes": response.len()
                    }),
                )
                .with_job_id(job.job_id.to_string()),
            )
            .await;

            state
                .storage
                .set_job_outputs(
                    &job.job_id.to_string(),
                    Some(json!({ "summary": response })),
                )
                .await?;
            return Ok(Some(json!({ "summary": response })));
        }
    } else if job.job_kind == "term_exec" || job.job_kind == "terminal_exec" {
        let payload = execute_terminal_job(state, &job, trace_id).await?;
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
    let timeout_ms = inputs
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .or(Some(30_000));

    let output = TerminalService::run(program, &args, timeout_ms)
        .await
        .map_err(|e| match e {
            TerminalError::Invalid(msg) | TerminalError::Exec(msg) => {
                WorkflowError::Terminal(msg.to_string())
            }
            TerminalError::Timeout(ms) => {
                WorkflowError::Terminal(format!("command timed out after {} ms", ms))
            }
            TerminalError::Io(ioe) => WorkflowError::Terminal(ioe.to_string()),
        })?;

    let payload = json!({
        "job_kind": job.job_kind,
        "program": program,
        "args": args,
        "status_code": output.status_code,
        "stdout": output.stdout,
        "stderr": output.stderr
    });

    record_event_safely(
        state,
        FlightRecorderEvent::new(
            FlightRecorderEventType::CapabilityAction,
            FlightRecorderActor::Agent,
            trace_id,
            payload.clone(),
        )
        .with_job_id(job.job_id.to_string()),
    )
    .await;

    state
        .storage
        .set_job_outputs(&job.job_id.to_string(), Some(payload.clone()))
        .await?;

    if output.status_code != 0 {
        return Err(WorkflowError::Terminal(format!(
            "command exited with code {}",
            output.status_code
        )));
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::llm::TestLLMClient;
    use crate::storage::{sqlite::SqliteDatabase, AccessMode, JobMetrics, SafetyMode};
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
        let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
        sqlite.run_migrations().await?;

        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        Ok(AppState {
            storage: sqlite.into_arc(),
            flight_recorder,
            llm_client: Arc::new(TestLLMClient {
                response: "ok".into(),
            }),
            capability_registry: Arc::new(CapabilityRegistry::new_default()),
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
                job_kind: "doc_summarize".to_string(),
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
                job_kind: "term_exec".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "default".to_string(),
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
                job_kind: "term_exec".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "terminal".to_string(),
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
                job_kind: "term_exec".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "terminal".to_string(),
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
}
