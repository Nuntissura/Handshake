use crate::{
    jobs::{create_job, JobError},
    models::WorkflowRun,
    workflows::{start_workflow_for_job, WorkflowError},
    AppState,
};
use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub job_kind: String,
    pub protocol_id: String,
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
        .with_state(state)
}

/// This is the API handler. It receives a request, calls the jobs and
/// workflows modules, and returns a response.
async fn create_new_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<WorkflowRun>, String> {
    let job = create_job(&state, &payload.job_kind, &payload.protocol_id)
        .await
        .map_err(|e| format!("Failed to create job: {:?}", e))?;

    let workflow_run = start_workflow_for_job(&state, job)
        .await
        .map_err(|e| format!("Failed to start workflow: {:?}", e))?;

    Ok(Json(workflow_run))
}
