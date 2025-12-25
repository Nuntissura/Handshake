use crate::{
    capabilities::RegistryError,
    flight_recorder::log_event,
    llm::ChatMessage,
    models::{AiJob, WorkflowRun},
    storage::{JobStatusUpdate, StorageError},
    terminal::{TerminalError, TerminalService},
    AppState,
};
use serde_json::json;
use thiserror::Error;

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

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    if let Err(err) = enforce_capabilities(state, &job) {
        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.id.clone(),
                status: "failed".to_string(),
                error_message: Some(err.to_string()),
                job_outputs: None,
            })
            .await?;
        return Err(err);
    }

    let workflow_run = state
        .storage
        .create_workflow_run(&job.id, "running")
        .await?;

    let _ = log_event(
        state,
        "workflow_started",
        Some(&job.id),
        Some(&workflow_run.id),
        json!({ "status": workflow_run.status }),
    );

    let result = run_job(state, &job).await;

    let (final_status, error_message, captured_error) = match result {
        Ok(_) => ("completed".to_string(), None, None),
        Err(e) => {
            let msg = e.to_string();
            ("failed".to_string(), Some(msg.clone()), Some(e))
        }
    };

    state
        .storage
        .update_ai_job_status(JobStatusUpdate {
            job_id: job.id.clone(),
            status: final_status.clone(),
            error_message: error_message.clone(),
            job_outputs: None,
        })
        .await?;

    let completed_run = state
        .storage
        .update_workflow_run_status(&workflow_run.id, &final_status, error_message.clone())
        .await?;

    let event_type = if final_status == "completed" {
        "workflow_completed"
    } else {
        "workflow_failed"
    };
    let _ = log_event(
        state,
        event_type,
        Some(&job.id),
        Some(&completed_run.id),
        json!({ "status": completed_run.status, "error": error_message }),
    );

    captured_error.map_or(Ok(completed_run), Err)
}

fn log_capability_check(
    state: &AppState,
    job: &AiJob,
    capability_id: &str,
    profile_id: &str,
    outcome: &str,
) {
    let payload = json!({
        "capability_id": capability_id,
        "profile_id": profile_id,
        "job_id": job.id,
        "outcome": outcome,
    });
    let _ = log_event(state, "capability_check", Some(&job.id), None, payload);
}

fn enforce_capabilities(state: &AppState, job: &AiJob) -> Result<(), WorkflowError> {
    let required = state
        .capability_registry
        .required_capability_for_job(&job.job_kind)?;

    let result = state
        .capability_registry
        .profile_can(&job.capability_profile_id, &required);

    match result {
        Ok(_) => {
            log_capability_check(state, job, &required, &job.capability_profile_id, "allowed");
            Ok(())
        }
        Err(err) => {
            log_capability_check(state, job, &required, &job.capability_profile_id, "denied");
            Err(WorkflowError::Capability(err))
        }
    }
}

async fn run_job(state: &AppState, job: &AiJob) -> Result<(), WorkflowError> {
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

            state
                .storage
                .set_job_outputs(&job.id, Some(json!({ "summary": response })))
                .await?;
        }
    } else if job.job_kind == "term_exec" || job.job_kind == "terminal_exec" {
        execute_terminal_job(state, &job).await?;
    }
    Ok(())
}

async fn execute_terminal_job(state: &AppState, job: &AiJob) -> Result<(), WorkflowError> {
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

    let _ = log_event(state, "terminal_exec", Some(&job.id), None, payload.clone());

    state
        .storage
        .set_job_outputs(&job.id, Some(payload.clone()))
        .await?;

    if output.status_code != 0 {
        return Err(WorkflowError::Terminal(format!(
            "command exited with code {}",
            output.status_code
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::llm::TestLLMClient;
    use crate::storage::sqlite::SqliteDatabase;
    use duckdb::Connection as DuckDbConnection;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
        let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
        sqlite.run_migrations().await?;

        let fr_conn = DuckDbConnection::open_in_memory()?;
        fr_conn.execute_batch(
            r#"
                CREATE TABLE events (
                    timestamp DATETIME DEFAULT current_timestamp,
                    event_type TEXT NOT NULL,
                    job_id TEXT,
                    workflow_id TEXT,
                    payload JSON
                );
                "#,
        )?;

        Ok(AppState {
            storage: sqlite.into_arc(),
            fr_pool: Arc::new(Mutex::new(fr_conn)),
            llm_client: Arc::new(TestLLMClient {
                response: "mock".to_string(),
            }),
            capability_registry: Arc::new(CapabilityRegistry::new_default()),
        })
    }

    #[tokio::test]
    async fn job_fails_when_missing_required_capability() -> Result<(), Box<dyn std::error::Error>>
    {
        let state = setup_state().await?;
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                job_kind: "doc_summarize".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "missing_profile".to_string(),
                access_mode: "default".to_string(),
                safety_mode: "default".to_string(),
                job_inputs: Some(json!({ "doc_id": "doc-1" })),
            })
            .await?;
        let job_id = job.id.clone();

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = state.storage.get_ai_job(&job_id).await?;

        assert_eq!(updated_job.status, "failed");
        let message = updated_job
            .error_message
            .clone()
            .map_or_else(String::new, |m| m);
        assert!(
            message.contains("Capability profile not found"),
            "unexpected error message: {}",
            message
        );
        Ok(())
    }

    #[tokio::test]
    async fn terminal_job_enforces_capability() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let job = state
            .storage
            .create_ai_job(crate::storage::NewAiJob {
                job_kind: "term_exec".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "default".to_string(),
                access_mode: "default".to_string(),
                safety_mode: "default".to_string(),
                job_inputs: Some(json!({ "program": "printf", "args": ["hello"] })),
            })
            .await?;
        let job_id = job.id.clone();

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = state.storage.get_ai_job(&job_id).await?;

        assert_eq!(updated_job.status, "failed");
        let message = updated_job.error_message.clone().unwrap_or_default();
        assert!(
            message.contains("missing capability term.exec"),
            "unexpected error message: {}",
            message
        );
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
                job_kind: "term_exec".to_string(),
                protocol_id: "protocol-default".to_string(),
                profile_id: "default".to_string(),
                capability_profile_id: "terminal".to_string(),
                access_mode: "default".to_string(),
                safety_mode: "default".to_string(),
                job_inputs: Some(json!({ "program": program, "args": args })),
            })
            .await?;
        let job_id = job.id.clone();

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_ok(), "terminal job failed: {:?}", result.err());

        let updated_job = state.storage.get_ai_job(&job_id).await?;

        assert_eq!(updated_job.status, "completed");

        let outputs = updated_job
            .job_outputs
            .as_ref()
            .ok_or("missing job outputs")?;
        let stdout = outputs.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        assert!(stdout.trim().contains("hello"));

        Ok(())
    }
}
