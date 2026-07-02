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

const FEMS_PROTOCOL_MEMORY_EXTRACT_V0_1: &str = "memory_extract_v0.1";
const FEMS_PROTOCOL_MEMORY_CONSOLIDATE_V0_1: &str = "memory_consolidate_v0.1";
const FEMS_PROTOCOL_MEMORY_FORGET_V0_1: &str = "memory_forget_v0.1";
const FEMS_PROTOCOL_MEMORY_HYGIENE_V0_1: &str = "memory_hygiene_v0.1";

fn is_fems_protocol(protocol_id: &str) -> bool {
    matches!(
        protocol_id,
        FEMS_PROTOCOL_MEMORY_EXTRACT_V0_1
            | FEMS_PROTOCOL_MEMORY_CONSOLIDATE_V0_1
            | FEMS_PROTOCOL_MEMORY_FORGET_V0_1
            | FEMS_PROTOCOL_MEMORY_HYGIENE_V0_1
    )
}

#[derive(Debug)]
enum ParseJobKindRequestError {
    ContractMismatch {
        job_kind: String,
        protocol_id: String,
    },
    InvalidJobKind(crate::storage::StorageError),
}

impl std::fmt::Display for ParseJobKindRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseJobKindRequestError::ContractMismatch {
                job_kind,
                protocol_id: _,
            } => {
                write!(
                    f,
                    "invalid job contract: job_kind {job_kind} requires matching protocol_id"
                )
            }
            ParseJobKindRequestError::InvalidJobKind(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ParseJobKindRequestError {}

fn parse_job_kind_request(
    job_kind: &str,
    protocol_id: &str,
) -> Result<JobKind, ParseJobKindRequestError> {
    let trimmed = job_kind.trim();
    if is_fems_protocol(trimmed) {
        if trimmed != protocol_id {
            return Err(ParseJobKindRequestError::ContractMismatch {
                job_kind: trimmed.to_string(),
                protocol_id: protocol_id.to_string(),
            });
        }
        return Ok(JobKind::WorkflowRun);
    }

    JobKind::from_str(trimmed).map_err(ParseJobKindRequestError::InvalidJobKind)
}

fn parse_job_kind_filter(job_kind: &str) -> Result<JobKind, String> {
    let trimmed = job_kind.trim();
    if is_fems_protocol(trimmed) {
        return Ok(JobKind::WorkflowRun);
    }
    JobKind::from_str(trimmed).map_err(|e| e.to_string())
}

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
    let job_kind = parse_job_kind_request(payload.job_kind.as_str(), payload.protocol_id.as_str())
        .map_err(|e| e.to_string())?;

    let capability_job_kind = if matches!(job_kind, JobKind::ModelRun) {
        JobKind::WorkflowRun
    } else {
        job_kind.clone()
    };

    let capability_profile_id = state
        .capability_registry
        .profile_for_job_request(capability_job_kind.as_str(), payload.protocol_id.as_str())
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
        .map(parse_job_kind_filter)
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

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::jobs::create_job;
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{
        tests::optional_postgres_backend_with_pool_from_env, AiJob, JobKind, JobState,
        ModelSessionState, SessionMessageRole,
    };
    use axum::extract::State;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::time::{sleep, timeout, Duration};

    async fn setup_state() -> Result<Option<AppState>, Box<dyn std::error::Error>> {
        let Some(backend) = optional_postgres_backend_with_pool_from_env().await? else {
            return Ok(None);
        };

        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        Ok(Some(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        }))
    }

    async fn require_mt101_runtime_state(
        proof_name: &str,
    ) -> Result<AppState, Box<dyn std::error::Error>> {
        let setup = timeout(Duration::from_secs(300), setup_state())
            .await
            .map_err(|_| format!("ENVIRONMENT_BLOCKED: {proof_name} setup_state timed out"))?
            .map_err(|err| {
                format!("ENVIRONMENT_BLOCKED: {proof_name} setup_state failed: {err}")
            })?;
        eprintln!("MT-101 proof {proof_name}: setup_state ready");
        setup.ok_or_else(|| {
            format!(
                "ENVIRONMENT_BLOCKED: {proof_name} requires real PostgreSQL; tests auto-resolve POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL"
            )
            .into()
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

    async fn wait_for_job_terminal(
        state: &AppState,
        job_id: &str,
    ) -> Result<AiJob, Box<dyn std::error::Error>> {
        let mut last_job = None;
        for _ in 0..1_200 {
            let job = state.storage.get_ai_job(job_id).await?;
            if matches!(
                job.state,
                JobState::Stalled
                    | JobState::AwaitingValidation
                    | JobState::AwaitingUser
                    | JobState::Completed
                    | JobState::CompletedWithIssues
                    | JobState::Failed
                    | JobState::Cancelled
                    | JobState::Poisoned
            ) {
                return Ok(job);
            }
            last_job = Some(job);
            sleep(Duration::from_millis(50)).await;
        }
        let detail = last_job
            .map(|job| {
                format!(
                    "last observed state={:?}, status_reason={}, error={:?}",
                    job.state, job.status_reason, job.error_message
                )
            })
            .unwrap_or_else(|| "job was never observed".to_string());
        Err(format!("timed out waiting for job {job_id} to leave queued/running; {detail}").into())
    }

    #[tokio::test]
    async fn create_job_rejects_unknown_job_kind() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            eprintln!(
                "ENVIRONMENT_BLOCKED: real PostgreSQL unavailable; skipped unknown-job route proof"
            );
            return Ok(());
        };
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
        let Some(state) = setup_state().await? else {
            eprintln!(
                "ENVIRONMENT_BLOCKED: real PostgreSQL unavailable; skipped terminal route proof"
            );
            return Ok(());
        };
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

    #[tokio::test]
    async fn create_model_run_job_binds_workflow_before_workflow_start(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            eprintln!(
                "ENVIRONMENT_BLOCKED: real PostgreSQL unavailable; skipped MT-101 model_run pre-start workflow binding proof"
            );
            return Ok(());
        };
        let session_id = format!("native-mt101-prestart-{}", uuid::Uuid::now_v7());
        let workspace_folder = "D:/Projects/Handshake/repo";

        let job = create_job(
            &state,
            JobKind::ModelRun,
            "protocol-default",
            "workflow_run",
            Some(json!({
                "launch_surface": "handshake_native",
                "launch_mode": "workspace_model_session",
                "wp_id": "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
                "mt_id": "MT-101",
                "session_id": session_id.as_str(),
                "workspace_id": "default-project",
                "workspace_folder": workspace_folder,
                "working_dir": workspace_folder,
                "model_provider": "local",
                "model_id": "qwen2.5-coder:7b",
                "backend": "local",
                "wrapper": "repo-folder-wrapper-v1",
                "prompt": "MT-101 pre-start workflow binding proof",
                "role": "assistant",
                "lane": "PRIMARY",
                "priority": 50,
                "retry_backoff": "exponential",
                "timeout_ms": 120000,
                "max_tokens_budget": 4096,
                "max_retries": 3,
                "parameter_class": "default",
                "execution_mode": "STANDARD",
                "memory_policy": "EPHEMERAL",
                "capability_grants": [],
                "capability_token_ids": [],
                "session_messages": [],
                "simulate_duration_ms": 0
            })),
            Vec::new(),
        )
        .await?;
        let workflow_run_id = job
            .workflow_run_id
            .ok_or("model_run job must be born with workflow_run_id")?;
        assert!(
            matches!(job.state, JobState::Queued),
            "new model_run job should remain queued until dispatcher runs"
        );

        let persisted = state.storage.get_ai_job(&job.job_id.to_string()).await?;
        assert_eq!(persisted.workflow_run_id, Some(workflow_run_id));
        let run = state
            .storage
            .update_workflow_run_status(workflow_run_id, JobState::Queued, None)
            .await?;
        assert_eq!(run.job_id, job.job_id);
        assert!(matches!(run.status, JobState::Queued));

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL"]
    async fn create_model_run_job_launches_runtime_session_and_preserves_native_binding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = require_mt101_runtime_state(
            "create_model_run_job_launches_runtime_session_and_preserves_native_binding",
        )
        .await?;
        let session_id = format!("native-mt101-{}", uuid::Uuid::now_v7());
        let workspace_folder = "D:/Projects/Handshake/repo";
        let wrapper = "repo-folder-wrapper-v1";

        let request = CreateJobRequest {
            job_kind: "model_run".to_string(),
            protocol_id: "protocol-default".to_string(),
            doc_id: None,
            job_inputs: Some(json!({
                "launch_surface": "handshake_native",
                "launch_mode": "workspace_model_session",
                "wp_id": "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
                "mt_id": "MT-101",
                "session_id": session_id.as_str(),
                "workspace_id": "default-project",
                "workspace_folder": workspace_folder,
                "working_dir": workspace_folder,
                "model_provider": "local",
                "model_id": "qwen2.5-coder:7b",
                "backend": "local",
                "wrapper": wrapper,
                "prompt": "MT-101 runtime proof",
                "role": "assistant",
                "lane": "PRIMARY",
                "priority": 50,
                "retry_backoff": "exponential",
                "timeout_ms": 120000,
                "max_tokens_budget": 4096,
                "max_retries": 3,
                "parameter_class": "default",
                "execution_mode": "STANDARD",
                "memory_policy": "EPHEMERAL",
                "capability_grants": [],
                "capability_token_ids": [],
                "session_messages": [],
                "simulate_duration_ms": 0
            })),
        };

        eprintln!("MT-101 local runtime proof: create_new_job begin");
        let response = timeout(
            Duration::from_secs(30),
            create_new_job(State(state.clone()), Json(request)),
        )
        .await
        .map_err(|_| "MT-101 local model_run proof timed out before create_new_job returned")??;
        let workflow_run = response.0;
        eprintln!(
            "MT-101 local runtime proof: create_new_job returned job_id={}",
            workflow_run.job_id
        );
        let job = wait_for_job_terminal(&state, &workflow_run.job_id.to_string()).await?;

        assert!(
            matches!(job.state, JobState::Completed),
            "local model_run should complete through the runtime path, got {:?}: {:?}",
            job.state,
            job.error_message
        );
        assert_eq!(job.workflow_run_id, Some(workflow_run.id));
        let inputs = job.job_inputs.as_ref().ok_or("missing job_inputs")?;
        assert_eq!(inputs["session_id"], json!(session_id.as_str()));
        assert_eq!(inputs["workspace_folder"], json!(workspace_folder));
        assert_eq!(inputs["working_dir"], json!(workspace_folder));
        assert_eq!(inputs["wrapper"], json!(wrapper));

        let outputs = job.job_outputs.as_ref().ok_or("missing job_outputs")?;
        assert_eq!(outputs["session_id"], json!(session_id.as_str()));

        let session = state.storage.get_model_session(&session_id).await?;
        assert!(matches!(session.state, ModelSessionState::Completed));
        assert_eq!(session.job_id, Some(workflow_run.job_id));
        assert_eq!(session.model_id, "qwen2.5-coder:7b");
        assert_eq!(session.backend, "local");
        assert_eq!(
            session.wp_id.as_deref(),
            Some("WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1")
        );
        assert_eq!(session.mt_id.as_deref(), Some("MT-101"));

        let messages = state.storage.list_session_messages(&session_id).await?;
        assert!(
            messages.iter().any(|message| {
                message.role == SessionMessageRole::Assistant
                    && message.content_artifact_id.contains(session_id.as_str())
                    && message.token_count.unwrap_or_default() > 0
            }),
            "local runtime path must append an assistant message for {session_id}"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL"]
    async fn create_cloud_model_run_without_consent_blocks_runtime_session(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = require_mt101_runtime_state(
            "create_cloud_model_run_without_consent_blocks_runtime_session",
        )
        .await?;
        let session_id = format!("native-mt101-cloud-{}", uuid::Uuid::now_v7());

        let request = CreateJobRequest {
            job_kind: "model_run".to_string(),
            protocol_id: "protocol-default".to_string(),
            doc_id: None,
            job_inputs: Some(json!({
                "launch_surface": "handshake_native",
                "launch_mode": "workspace_model_session",
                "wp_id": "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
                "mt_id": "MT-101",
                "session_id": session_id.as_str(),
                "workspace_id": "default-project",
                "workspace_folder": "D:/Projects/Handshake/repo",
                "working_dir": "D:/Projects/Handshake/repo",
                "model_provider": "cloud",
                "model_id": "gpt-5.4",
                "backend": "cloud",
                "wrapper": "repo-folder-wrapper-v1",
                "prompt": "MT-101 cloud consent blocker proof",
                "role": "assistant",
                "lane": "PRIMARY",
                "priority": 50,
                "retry_backoff": "exponential",
                "timeout_ms": 120000,
                "max_tokens_budget": 4096,
                "max_retries": 3,
                "parameter_class": "default",
                "execution_mode": "STANDARD",
                "memory_policy": "EPHEMERAL",
                "session_messages": []
            })),
        };

        eprintln!("MT-101 cloud runtime proof: create_new_job begin");
        let response = timeout(
            Duration::from_secs(30),
            create_new_job(State(state.clone()), Json(request)),
        )
        .await
        .map_err(|_| "MT-101 cloud model_run proof timed out before create_new_job returned")??;
        let workflow_run = response.0;
        eprintln!(
            "MT-101 cloud runtime proof: create_new_job returned job_id={}",
            workflow_run.job_id
        );
        let job = wait_for_job_terminal(&state, &workflow_run.job_id.to_string()).await?;

        assert!(
            matches!(job.state, JobState::AwaitingUser),
            "cloud launch without consent must block, not claim running/completed: {:?}",
            job.state
        );
        assert_eq!(job.status_reason, "paused_cloud_consent");
        assert_eq!(job.error_message.as_deref(), Some("cloud_consent_required"));

        let session = state.storage.get_model_session(&session_id).await?;
        assert!(matches!(session.state, ModelSessionState::Blocked));
        assert_eq!(session.job_id, Some(workflow_run.job_id));
        assert_eq!(session.backend, "cloud");
        assert!(
            state
                .session_registry
                .get_session_worktree_path(&session_id)
                .await
                .is_none(),
            "cloud launch blocked before consent must not allocate a session worktree"
        );

        Ok(())
    }

    #[tokio::test]
    async fn create_job_accepts_fems_job_kind_alias() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let request = CreateJobRequest {
            job_kind: "memory_extract_v0.1".to_string(),
            protocol_id: "memory_extract_v0.1".to_string(),
            doc_id: None,
            job_inputs: Some(json!({
                "memory_policy": "EPHEMERAL",
                "memory_ids": ["mem_001"]
            })),
        };

        let response = create_new_job(State(state.clone()), Json(request)).await?;
        let job = state
            .storage
            .get_ai_job(&response.0.job_id.to_string())
            .await?;
        assert_eq!(job.job_kind.as_str(), "workflow_run");
        assert_eq!(job.protocol_id, "memory_extract_v0.1");
        Ok(())
    }

    #[tokio::test]
    async fn create_job_rejects_mismatched_fems_alias_protocol(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let request = CreateJobRequest {
            job_kind: "memory_extract_v0.1".to_string(),
            protocol_id: "memory_forget_v0.1".to_string(),
            doc_id: None,
            job_inputs: Some(json!({ "memory_ids": ["mem_001"] })),
        };

        let result = create_new_job(State(state), Json(request)).await;
        assert!(
            result.is_err(),
            "expected mismatched FEMS alias/protocol to be rejected"
        );
        Ok(())
    }
}
