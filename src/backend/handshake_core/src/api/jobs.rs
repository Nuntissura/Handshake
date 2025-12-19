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
    pub capability_profile_id: Option<String>,
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
    let job_inputs = payload
        .job_inputs
        .clone()
        .or_else(|| payload.doc_id.as_ref().map(|id| json!({ "doc_id": id })));

    let job = create_job(
        &state,
        &payload.job_kind,
        &payload.protocol_id,
        payload
            .capability_profile_id
            .as_deref()
            .unwrap_or("default"),
        job_inputs,
    )
    .await
    .map_err(|e| format!("Failed to create job: {:?}", e))?;

    let workflow_run = start_workflow_for_job(&state, job)
        .await
        .map_err(|e| format!("Failed to start workflow: {:?}", e))?;

    Ok(Json(workflow_run))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AiJob>, String> {
    let job = sqlx::query_as::<_, AiJob>(
        r#"
        SELECT
            id,
            job_kind,
            status,
            error_message,
            protocol_id,
            profile_id,
            capability_profile_id,
            access_mode,
            safety_mode,
            job_inputs,
            job_outputs,
            created_at,
            updated_at
        FROM ai_jobs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| format!("Database error: {:?}", e))?
    .ok_or_else(|| "Job not found".to_string())?;

    Ok(Json(job))
}
