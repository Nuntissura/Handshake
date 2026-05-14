use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::kernel::{KernelError, KernelTraceInspector, TraceProjection};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TraceProjectionQuery {
    pub kernel_task_run_id: String,
    pub session_run_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: &'static str,
    pub message: String,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

fn api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            code,
            message: message.into(),
        }),
    )
}

fn map_kernel_error(err: KernelError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        KernelError::InvalidEvent(_)
        | KernelError::InvalidEventType(_)
        | KernelError::InvalidSessionTransition { .. } => api_error(
            StatusCode::BAD_REQUEST,
            "kernel_trace_invalid",
            err.to_string(),
        ),
        _ => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_trace_inspection_failed",
            err.to_string(),
        ),
    }
}

pub async fn inspect_trace_projection(
    State(state): State<AppState>,
    Query(query): Query<TraceProjectionQuery>,
) -> ApiResult<TraceProjection> {
    if query.kernel_task_run_id.trim().is_empty() || query.session_run_id.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "kernel_trace_missing_ids",
            "kernel_task_run_id and session_run_id are required",
        ));
    }

    let projection = KernelTraceInspector::new(state.storage.clone())
        .inspect_session(&query.kernel_task_run_id, &query.session_run_id)
        .await
        .map_err(map_kernel_error)?;
    Ok(Json(projection))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/kernel/trace_projection", get(inspect_trace_projection))
        .with_state(state)
}
