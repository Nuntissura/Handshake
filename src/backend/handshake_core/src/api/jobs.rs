use crate::{
    jobs::{create_job, JobError},
    models::{AiJob, WorkflowRun},
    workflows::{start_workflow_for_job, WorkflowError},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub job_kind: String,
    pub protocol_id: String,
    #[serde(default)]
    pub doc_id: Option<String>,
    #[serde(default)]
    pub job_inputs: Option<Value>,
}

// We will improve error handling later. For now, we combine the possible
// errors from the jobs and workflows modules.
#[derive(Debug)]
pub enum ApiJobError {
    Job(JobError),
    Workflow(WorkflowError),
}

impl From<JobError> for ApiJobError {
    fn from(err: JobError) -> Self {
        ApiJobError::Job(err)
    }
}
impl From<WorkflowError> for ApiJobError {
    fn from(err: WorkflowError) -> Self {
        ApiJobError::Workflow(err)
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/jobs", post(create_new_job))
        .route("/jobs/:id", get(get_job))
        .with_state(state)
}

/// This is the API handler. It receives a request, calls the jobs and
/// workflows modules, and returns a response.
async fn create_new_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<WorkflowRun>, String> {
    let capability_profile_id = state
        .capability_registry
        .profile_for_job_kind(&payload.job_kind)
        .map_err(|e| e.to_string())?;

    let job_inputs = payload
        .job_inputs
        .clone()
        .or_else(|| payload.doc_id.as_ref().map(|id| json!({ "doc_id": id })));

    let job = create_job(
        &state,
        &payload.job_kind,
        &payload.protocol_id,
        // Server-enforced capability profile to prevent client-side escalation.
        capability_profile_id.as_str(),
        job_inputs,
    )
    .await
    .map_err(|e| e.to_string())?;

    let workflow_run = start_workflow_for_job(&state, job)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(workflow_run))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AiJob>, String> {
    let job = state
        .storage
        .get_ai_job(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(job))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::llm::TestLLMClient;
    use crate::storage::sqlite::SqliteDatabase;
    use axum::extract::State;
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
    async fn create_job_rejects_unknown_job_kind() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let request = CreateJobRequest {
            job_kind: "unknown_job".to_string(),
            protocol_id: "protocol-default".to_string(),
            doc_id: None,
            job_inputs: None,
        };

        let result = create_new_job(State(state), Json(request)).await;
        assert!(result.is_err(), "expected unknown job_kind to be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn create_job_allows_terminal_when_authorized() -> Result<(), Box<dyn std::error::Error>>
    {
        let state = setup_state().await?;
        let (program, args) = terminal_command();

        let request = CreateJobRequest {
            job_kind: "term_exec".to_string(),
            protocol_id: "protocol-default".to_string(),
            doc_id: None,
            job_inputs: Some(json!({ "program": program, "args": args })),
        };

        let response = create_new_job(State(state.clone()), Json(request)).await?;
        let workflow_run = response.0;

        let job = state.storage.get_ai_job(&workflow_run.job_id).await?;
        assert_eq!(job.status, "completed");

        let outputs = job.job_outputs.as_ref().ok_or("missing job outputs")?;
        let stdout = outputs
            .get("stdout")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        assert!(
            stdout.contains("hello"),
            "expected terminal output to contain 'hello', got '{}'",
            stdout
        );

        Ok(())
    }
}
