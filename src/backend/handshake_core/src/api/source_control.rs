//! WP-KERNEL-009 MT-253 source-control REST surface.
//!
//! This is the product-callable wrapper over `crate::source_control`: local git
//! only, typed JSON results, and explicit confirmation for destructive discard.

use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::source_control::{
    normalize_paths, validate_branch_name, DiffScope, SourceControlCommit, SourceControlError,
    SourceControlReceipt, SourceControlRepository, SourceControlStatus,
};
use crate::storage::Database;
use crate::AppState;

type ApiError = (StatusCode, Json<Value>);
type ApiResult<T> = Result<Json<T>, ApiError>;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

#[derive(Clone)]
struct SourceControlApiState {
    event_recorder: Arc<dyn SourceControlEventRecorder>,
}

#[derive(Debug, Clone)]
pub struct SourceControlEventRecord {
    pub operation: String,
    pub repo_root: String,
    pub paths: Vec<String>,
    pub commit_message: Option<String>,
    pub actor: KernelActor,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum SourceControlReceiptError {
    #[error("source-control receipt build failed: {0}")]
    Build(String),
    #[error("source-control receipt storage failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait SourceControlEventRecorder: Send + Sync {
    async fn record(
        &self,
        record: SourceControlEventRecord,
    ) -> Result<String, SourceControlReceiptError>;
}

struct KernelSourceControlEventRecorder {
    storage: Arc<dyn Database>,
}

#[async_trait]
impl SourceControlEventRecorder for KernelSourceControlEventRecorder {
    async fn record(
        &self,
        record: SourceControlEventRecord,
    ) -> Result<String, SourceControlReceiptError> {
        let mut builder = NewKernelEvent::builder(
            record.kernel_task_run_id,
            record.session_run_id,
            KernelEventType::SourceControlOperationRecorded,
            record.actor,
        )
        .aggregate("source_control_repo", record.repo_root.clone())
        .source_component("source_control_api")
        .payload(json!({
            "kind": "source_control_operation",
            "operation": record.operation,
            "repo_root": record.repo_root,
            "paths": record.paths,
            "commit_message": record.commit_message,
            "phase": "pre_git_write",
            "authority_source": "postgres_event_ledger",
        }));

        if let Some(correlation_id) = record.correlation_id {
            builder = builder.correlation_id(correlation_id);
        }

        let event = builder
            .build()
            .map_err(|err| SourceControlReceiptError::Build(err.to_string()))?;
        let stored = self
            .storage
            .append_kernel_event(event)
            .await
            .map_err(|err| SourceControlReceiptError::Storage(err.to_string()))?;
        Ok(stored.event_id)
    }
}

pub fn routes(state: AppState) -> Router {
    routes_with_event_recorder(Arc::new(KernelSourceControlEventRecorder {
        storage: state.storage.clone(),
    }))
}

pub fn routes_with_event_recorder(event_recorder: Arc<dyn SourceControlEventRecorder>) -> Router {
    Router::new()
        .route("/source-control/status", get(status))
        .route("/source-control/diff", get(diff))
        .route("/source-control/stage", post(stage))
        .route("/source-control/unstage", post(unstage))
        .route("/source-control/discard", post(discard))
        .route("/source-control/commit", post(commit))
        .route(
            "/source-control/branches",
            get(branches).post(create_branch),
        )
        .route("/source-control/switch", post(switch_branch))
        .route("/source-control/log", get(log))
        .route("/source-control/blame", get(blame))
        .with_state(SourceControlApiState { event_recorder })
}

#[derive(Debug, Deserialize)]
struct RepoQuery {
    repo_path: String,
}

#[derive(Debug, Deserialize)]
struct DiffQuery {
    repo_path: String,
    path: String,
    scope: DiffScope,
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    repo_path: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BlameQuery {
    repo_path: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct PathsRequest {
    repo_path: String,
    paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DiscardRequest {
    repo_path: String,
    paths: Vec<String>,
    confirmed: bool,
}

#[derive(Debug, Deserialize)]
struct CommitRequest {
    repo_path: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct BranchRequest {
    repo_path: String,
    name: String,
}

async fn status(Query(query): Query<RepoQuery>) -> ApiResult<SourceControlStatus> {
    let repo = repo(&query.repo_path)?;
    repo.status().map(Json).map_err(map_error)
}

async fn diff(
    Query(query): Query<DiffQuery>,
) -> ApiResult<crate::source_control::SourceControlDiff> {
    let repo = repo(&query.repo_path)?;
    repo.diff(&query.path, query.scope)
        .map(Json)
        .map_err(map_error)
}

async fn stage(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<PathsRequest>,
) -> ApiResult<SourceControlReceipt> {
    let repo = repo(&payload.repo_path)?;
    let paths = path_refs(&payload.paths);
    let normalized_paths = normalize_paths(&paths).map_err(map_error)?;
    let event_ledger_event_id =
        record_write_receipt(&state, &headers, &repo, "stage", normalized_paths, None).await?;
    let mut receipt = repo.stage(&paths).map_err(map_error)?;
    receipt.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(receipt))
}

async fn unstage(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<PathsRequest>,
) -> ApiResult<SourceControlReceipt> {
    let repo = repo(&payload.repo_path)?;
    let paths = path_refs(&payload.paths);
    let normalized_paths = normalize_paths(&paths).map_err(map_error)?;
    let event_ledger_event_id =
        record_write_receipt(&state, &headers, &repo, "unstage", normalized_paths, None).await?;
    let mut receipt = repo.unstage(&paths).map_err(map_error)?;
    receipt.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(receipt))
}

async fn discard(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<DiscardRequest>,
) -> ApiResult<SourceControlReceipt> {
    let repo = repo(&payload.repo_path)?;
    let paths = path_refs(&payload.paths);
    if !payload.confirmed {
        return repo
            .discard(&paths, payload.confirmed)
            .map(Json)
            .map_err(map_error);
    }
    let normalized_paths = normalize_paths(&paths).map_err(map_error)?;
    let event_ledger_event_id =
        record_write_receipt(&state, &headers, &repo, "discard", normalized_paths, None).await?;
    let mut receipt = repo.discard(&paths, payload.confirmed).map_err(map_error)?;
    receipt.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(receipt))
}

async fn commit(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<CommitRequest>,
) -> ApiResult<SourceControlCommit> {
    let repo = repo(&payload.repo_path)?;
    let message = payload.message.trim().to_string();
    if message.is_empty() {
        return Err(map_error(SourceControlError::EmptyCommitMessage));
    }
    let event_ledger_event_id = record_write_receipt(
        &state,
        &headers,
        &repo,
        "commit",
        Vec::new(),
        Some(message.clone()),
    )
    .await?;
    let mut commit = repo.commit(&message).map_err(map_error)?;
    commit.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(commit))
}

async fn branches(
    Query(query): Query<RepoQuery>,
) -> ApiResult<Vec<crate::source_control::SourceControlBranch>> {
    let repo = repo(&query.repo_path)?;
    repo.branches().map(Json).map_err(map_error)
}

async fn create_branch(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<BranchRequest>,
) -> ApiResult<SourceControlReceipt> {
    let repo = repo(&payload.repo_path)?;
    let branch_name = validate_branch_name(&payload.name).map_err(map_error)?;
    let event_ledger_event_id = record_write_receipt(
        &state,
        &headers,
        &repo,
        "create_branch",
        vec![branch_name.clone()],
        None,
    )
    .await?;
    let mut receipt = repo.create_branch(&branch_name).map_err(map_error)?;
    receipt.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(receipt))
}

async fn switch_branch(
    State(state): State<SourceControlApiState>,
    headers: HeaderMap,
    Json(payload): Json<BranchRequest>,
) -> ApiResult<SourceControlReceipt> {
    let repo = repo(&payload.repo_path)?;
    let branch_name = validate_branch_name(&payload.name).map_err(map_error)?;
    let event_ledger_event_id = record_write_receipt(
        &state,
        &headers,
        &repo,
        "switch_branch",
        vec![branch_name.clone()],
        None,
    )
    .await?;
    let mut receipt = repo.switch_branch(&branch_name).map_err(map_error)?;
    receipt.event_ledger_event_id = Some(event_ledger_event_id);
    Ok(Json(receipt))
}

async fn log(Query(query): Query<LogQuery>) -> ApiResult<crate::source_control::SourceControlLog> {
    let repo = repo(&query.repo_path)?;
    repo.log(query.limit.unwrap_or(50))
        .map(Json)
        .map_err(map_error)
}

async fn blame(
    Query(query): Query<BlameQuery>,
) -> ApiResult<crate::source_control::SourceControlBlame> {
    let repo = repo(&query.repo_path)?;
    repo.blame(&query.path).map(Json).map_err(map_error)
}

fn repo(path: &str) -> Result<SourceControlRepository, (StatusCode, Json<Value>)> {
    SourceControlRepository::open(path).map_err(map_error)
}

fn path_refs(paths: &[String]) -> Vec<&str> {
    paths.iter().map(String::as_str).collect()
}

async fn record_write_receipt(
    state: &SourceControlApiState,
    headers: &HeaderMap,
    repo: &SourceControlRepository,
    operation: &str,
    paths: Vec<String>,
    commit_message: Option<String>,
) -> Result<String, ApiError> {
    let context = receipt_context(headers)?;
    state
        .event_recorder
        .record(SourceControlEventRecord {
            operation: operation.to_string(),
            repo_root: repo_root_id(repo.root()),
            paths,
            commit_message,
            actor: context.actor,
            kernel_task_run_id: context.kernel_task_run_id,
            session_run_id: context.session_run_id,
            correlation_id: context.correlation_id,
        })
        .await
        .map_err(map_receipt_error)
}

struct ReceiptContext {
    actor: KernelActor,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: Option<String>,
}

fn receipt_context(headers: &HeaderMap) -> Result<ReceiptContext, ApiError> {
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID)
        .unwrap_or("source-control-api")
        .to_string();
    let actor = match header_str(headers, HSK_HEADER_ACTOR_KIND).unwrap_or("system") {
        "operator" => KernelActor::Operator(actor_id),
        "model_adapter" => KernelActor::ModelAdapter(actor_id),
        "toolgate" => KernelActor::ToolGate(actor_id),
        "validation_runner" => KernelActor::ValidationRunner(actor_id),
        "promotion_gate" => KernelActor::PromotionGate(actor_id),
        "session_broker" => KernelActor::SessionBroker(actor_id),
        "system" => KernelActor::System(actor_id),
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_source_control_actor_kind",
                    "detail": format!("unsupported x-hsk-actor-kind {other}"),
                })),
            ));
        }
    };

    Ok(ReceiptContext {
        actor,
        kernel_task_run_id: header_str(headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
            .unwrap_or("KTR-MT-253-SOURCE-CONTROL")
            .to_string(),
        session_run_id: header_str(headers, HSK_HEADER_SESSION_RUN_ID)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("SR-SOURCE-CONTROL-{}", Uuid::now_v7())),
        correlation_id: header_str(headers, HSK_HEADER_CORRELATION_ID).map(ToOwned::to_owned),
    })
}

fn header_str<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers.get(key).and_then(|value| value.to_str().ok())
}

fn repo_root_id(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn map_receipt_error(err: SourceControlReceiptError) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "error": "source_control_receipt_error",
            "detail": err.to_string(),
        })),
    )
}

fn map_error(err: SourceControlError) -> (StatusCode, Json<Value>) {
    let status = match err {
        SourceControlError::DiscardRequiresConfirmation => StatusCode::CONFLICT,
        SourceControlError::InvalidPath { .. }
        | SourceControlError::InvalidRepository { .. }
        | SourceControlError::InvalidBranchName { .. }
        | SourceControlError::EmptyBranchName
        | SourceControlError::EmptyCommitMessage
        | SourceControlError::GitCommandFailed { .. } => StatusCode::BAD_REQUEST,
        SourceControlError::GitIo(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "error": "source_control_error",
            "detail": err.to_string(),
        })),
    )
}
