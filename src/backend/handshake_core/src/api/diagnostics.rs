use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::diagnostics::{
    DiagFilter, Diagnostic, DiagnosticInput, DiagnosticSeverity, DiagnosticSurface, ProblemGroup,
};
use crate::swarm_orchestration::model_lane::{ModelLaneDiagnosticsProjection, ModelLaneStore};
use crate::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct DiagnosticsQuery {
    pub severity: Option<String>,
    pub source: Option<String>,
    pub surface: Option<String>,
    pub wsid: Option<String>,
    pub job_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub fingerprint: Option<String>,
    pub limit: Option<u32>,
}

fn parse_severity(raw: Option<String>) -> Result<Option<DiagnosticSeverity>, String> {
    raw.map(|s| DiagnosticSeverity::from_str(s.as_str()).map_err(|e| e.to_string()))
        .transpose()
}

fn parse_surface(raw: Option<String>) -> Result<Option<DiagnosticSurface>, String> {
    raw.map(|s| DiagnosticSurface::from_str(s.as_str()).map_err(|e| e.to_string()))
        .transpose()
}

fn parse_job_id(raw: Option<String>) -> Result<Option<Uuid>, String> {
    raw.map(|s| Uuid::parse_str(&s).map_err(|e| e.to_string()))
        .transpose()
}

fn into_filter(query: DiagnosticsQuery) -> Result<DiagFilter, String> {
    let severity = parse_severity(query.severity)?;
    let surface = parse_surface(query.surface)?;
    let job_id = parse_job_id(query.job_id)?;

    Ok(DiagFilter {
        severity,
        source: query.source,
        surface,
        wsid: query.wsid,
        job_id,
        from: query.from,
        to: query.to,
        fingerprint: query.fingerprint,
        limit: query.limit,
    })
}

async fn list_diagnostics(
    State(state): State<AppState>,
    Query(query): Query<DiagnosticsQuery>,
) -> Result<Json<Vec<Diagnostic>>, String> {
    let filter = into_filter(query)?;
    let diagnostics = state
        .diagnostics
        .list_diagnostics(filter)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(diagnostics))
}

async fn list_problems(
    State(state): State<AppState>,
    Query(query): Query<DiagnosticsQuery>,
) -> Result<Json<Vec<ProblemGroup>>, String> {
    let filter = into_filter(query)?;
    let problems = state
        .diagnostics
        .list_problems(filter)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(problems))
}

async fn get_diagnostic(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Diagnostic>, String> {
    let diagnostic = state
        .diagnostics
        .get_diagnostic(id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(diagnostic))
}

async fn create_diagnostic(
    State(state): State<AppState>,
    Json(payload): Json<DiagnosticInput>,
) -> Result<Json<Diagnostic>, String> {
    let mut diagnostic = payload.into_diagnostic().map_err(|e| e.to_string())?;
    if diagnostic.first_seen.is_none() {
        diagnostic.first_seen = Some(diagnostic.timestamp);
    }
    if diagnostic.last_seen.is_none() {
        diagnostic.last_seen = Some(diagnostic.timestamp);
    }

    state
        .diagnostics
        .record_diagnostic(diagnostic.clone())
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(diagnostic))
}

async fn latest_model_lane_diagnostics(
    State(state): State<AppState>,
) -> Result<Json<ModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
    let run_id = sqlx::query_scalar::<_, String>(
        "SELECT run_id FROM model_lane_runs ORDER BY event_ledger_seq DESC LIMIT 1",
    )
    .fetch_optional(&state.postgres_pool)
    .await
    .map_err(ModelLaneDiagnosticsApiError::authority_unavailable)?
    .ok_or_else(|| ModelLaneDiagnosticsApiError::not_found("no model lane runs recorded"))?;
    let store = ModelLaneStore::new(state.postgres_pool.clone());
    let model_catalog = state.llm_client.model_catalog();
    let projection = store
        .diagnostics_projection_with_model_catalog(&run_id, model_catalog.as_deref())
        .await
        .map_err(ModelLaneDiagnosticsApiError::integrity)?;
    Ok(Json(projection))
}

async fn get_model_lane_diagnostics(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM model_lane_runs WHERE run_id = $1)",
    )
    .bind(&run_id)
    .fetch_one(&state.postgres_pool)
    .await
    .map_err(ModelLaneDiagnosticsApiError::authority_unavailable)?;
    if !exists {
        return Err(ModelLaneDiagnosticsApiError::not_found(format!(
            "model lane run {run_id} not found"
        )));
    }
    let store = ModelLaneStore::new(state.postgres_pool.clone());
    let model_catalog = state.llm_client.model_catalog();
    let projection = store
        .diagnostics_projection_with_model_catalog(&run_id, model_catalog.as_deref())
        .await
        .map_err(ModelLaneDiagnosticsApiError::integrity)?;
    Ok(Json(projection))
}

#[derive(Debug, Serialize)]
struct ModelLaneDiagnosticsErrorBody {
    error: &'static str,
    detail: String,
}

#[derive(Debug)]
struct ModelLaneDiagnosticsApiError {
    status: StatusCode,
    code: &'static str,
    detail: String,
}

impl ModelLaneDiagnosticsApiError {
    fn not_found(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "MODEL_LANE_DIAGNOSTICS_NOT_FOUND",
            detail: detail.into(),
        }
    }

    fn authority_unavailable(error: sqlx::Error) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "MODEL_LANE_DIAGNOSTICS_AUTHORITY_UNAVAILABLE",
            detail: error.to_string(),
        }
    }

    fn integrity(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "MODEL_LANE_DIAGNOSTICS_INTEGRITY_FAILURE",
            detail: error.to_string(),
        }
    }
}

impl IntoResponse for ModelLaneDiagnosticsApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ModelLaneDiagnosticsErrorBody {
                error: self.code,
                detail: self.detail,
            }),
        )
            .into_response()
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/diagnostics",
            get(list_diagnostics).post(create_diagnostic),
        )
        .route("/diagnostics/problems", get(list_problems))
        .route("/diagnostics/:id", get(get_diagnostic))
        .route(
            "/swarm/model-lanes/diagnostics/latest",
            get(latest_model_lane_diagnostics),
        )
        .route(
            "/swarm/model-lanes/diagnostics/:run_id",
            get(get_model_lane_diagnostics),
        )
        .with_state(state)
}
