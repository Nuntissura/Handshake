use crate::{
    jobs::{create_job, JobError},
    models::{AiJob, ErrorResponse, JobKind, WorkflowRun},
    storage::{EntityRef, JobState},
    workflows::{record_cloud_escalation_consent_v0_4, start_workflow_for_job, WorkflowError},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn job_not_resumable() -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: "HSK-409-JOB-NOT-RESUMABLE",
        }),
    )
}

fn job_not_awaiting_user_consent() -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: "HSK-409-JOB-NOT-AWAITING-USER",
        }),
    )
}

fn storage_error(err: crate::storage::StorageError) -> ApiError {
    match err {
        crate::storage::StorageError::NotFound(_) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "HSK-404-JOB-NOT-FOUND",
            }),
        ),
        crate::storage::StorageError::Conflict(_) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "HSK-409-STORAGE-CONFLICT",
            }),
        ),
        crate::storage::StorageError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "HSK-400-STORAGE-VALIDATION",
            }),
        ),
        other => {
            tracing::error!(target: "handshake_core", error = %other, "jobs_api_storage_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "HSK-500-DB",
                }),
            )
        }
    }
}

fn workflow_error(err: WorkflowError) -> ApiError {
    tracing::error!(target: "handshake_core", error = %err, "jobs_api_workflow_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-WORKFLOW",
        }),
    )
}

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub job_kind: String,
    pub protocol_id: String,
    #[serde(default)]
    pub doc_id: Option<String>,
    #[serde(default)]
    pub job_inputs: Option<Value>,
}

#[derive(Deserialize)]
pub struct CloudEscalationConsentRequest {
    pub request_id: String,
    pub approved: bool,
    pub user_id: String,
    #[serde(default)]
    pub ui_surface: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
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
        .route("/jobs", get(list_jobs).post(create_new_job))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/resume", post(resume_job))
        .route(
            "/jobs/:id/cloud_escalation/consent",
            post(record_cloud_escalation_consent),
        )
        .with_state(state)
}

/// This is the API handler. It receives a request, calls the jobs and
/// workflows modules, and returns a response.
async fn create_new_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<WorkflowRun>, String> {
    let job_kind = JobKind::from_str(payload.job_kind.as_str()).map_err(|e| e.to_string())?;

    let capability_profile_id = state
        .capability_registry
        .profile_for_job(job_kind.as_str())
        .map_err(|e| e.to_string())?;

    let job_inputs = payload
        .job_inputs
        .clone()
        .or_else(|| payload.doc_id.as_ref().map(|id| json!({ "doc_id": id })));

    let mut entity_refs: Vec<EntityRef> = Vec::new();
    if let Some(doc_id) = payload.doc_id.as_deref() {
        let doc = state
            .storage
            .get_document(doc_id)
            .await
            .map_err(|e| e.to_string())?;
        entity_refs.push(EntityRef {
            entity_id: doc.workspace_id,
            entity_kind: "workspace".to_string(),
        });
        entity_refs.push(EntityRef {
            entity_id: doc.id,
            entity_kind: "document".to_string(),
        });
    }

    let job = create_job(
        &state,
        job_kind,
        &payload.protocol_id,
        // Server-enforced capability profile to prevent client-side escalation.
        capability_profile_id.id.as_str(),
        job_inputs,
        entity_refs,
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

async fn resume_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<WorkflowRun>> {
    let job = state.storage.get_ai_job(&id).await.map_err(storage_error)?;

    match job.state {
        JobState::AwaitingUser | JobState::Stalled => {}
        _ => {
            tracing::warn!(
                target: "handshake_core",
                job_id = %job.job_id,
                state = job.state.as_str(),
                "job_not_resumable"
            );
            return Err(job_not_resumable());
        }
    }

    let workflow_run = start_workflow_for_job(&state, job)
        .await
        .map_err(workflow_error)?;
    Ok(Json(workflow_run))
}

async fn record_cloud_escalation_consent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CloudEscalationConsentRequest>,
) -> ApiResult<Json<Value>> {
    let job = state.storage.get_ai_job(&id).await.map_err(storage_error)?;

    if !matches!(job.state, JobState::AwaitingUser) {
        tracing::warn!(
            target: "handshake_core",
            job_id = %job.job_id,
            state = job.state.as_str(),
            "job_not_awaiting_user_consent"
        );
        return Err(job_not_awaiting_user_consent());
    }

    record_cloud_escalation_consent_v0_4(
        &state,
        &job,
        payload.request_id,
        payload.approved,
        payload.user_id,
        payload.ui_surface,
        payload.notes,
    )
    .await
    .map_err(workflow_error)?;

    Ok(Json(json!({ "status": "recorded" })))
}

#[derive(Deserialize, Default)]
struct JobListFilters {
    status: Option<String>,
    job_kind: Option<String>,
    wsid: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(filters): Query<JobListFilters>,
) -> Result<Json<Vec<AiJob>>, String> {
    let status = filters
        .status
        .as_deref()
        .map(JobState::try_from)
        .transpose()
        .map_err(|e| e.to_string())?;
    let job_kind = filters
        .job_kind
        .as_deref()
        .map(JobKind::from_str)
        .transpose()
        .map_err(|e| e.to_string())?;

    let items = state
        .storage
        .list_ai_jobs(crate::storage::AiJobListFilter {
            status,
            job_kind,
            wsid: filters.wsid,
            from: filters.from,
            to: filters.to,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{sqlite::SqliteDatabase, Database, JobState};
    use axum::extract::State;
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
            job_kind: "terminal_exec".to_string(),
            protocol_id: "protocol-default".to_string(),
            doc_id: None,
            job_inputs: Some(json!({ "program": program, "args": args })),
        };

        let response = create_new_job(State(state.clone()), Json(request)).await?;
        let workflow_run = response.0;

        let job = state
            .storage
            .get_ai_job(&workflow_run.job_id.to_string())
            .await?;
        assert!(matches!(job.state, JobState::Completed));

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
