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

use crate::api::account_scope::RequestAccountScope;
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

/// Server-authoritative exact scope retains actor, session, AccessSpace, and
/// required workspace through both SQL and post-decode authorization.
fn model_lane_store(state: &AppState, scope: &RequestAccountScope) -> ModelLaneStore {
    ModelLaneStore::new_scoped(state.postgres_pool.clone(), scope.resource_scope())
}

async fn latest_model_lane_diagnostics(
    State(state): State<AppState>,
    scope: RequestAccountScope,
) -> Result<Json<ModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
    // The inline "globally newest run" probe that used to live here disclosed
    // another account's run id before any store-level scoping applied. Resolving
    // "latest" now happens inside the scoped store, so "latest" means "latest
    // that this account owns".
    let store = model_lane_store(&state, &scope);
    let model_catalog = state.llm_client.model_catalog();
    let projection = store
        .latest_diagnostics_projection_with_model_catalog(model_catalog.as_deref())
        .await
        .map_err(map_model_lane_diagnostics_error)?;
    Ok(Json(projection))
}

async fn get_model_lane_diagnostics(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
    // The previous inline `SELECT EXISTS(... WHERE run_id = $1)` probe was an
    // unscoped existence oracle: it told any caller whether a run id belonged to
    // somebody. Existence is now decided by the scoped store, which reports a
    // run this account may not read as absent.
    let store = model_lane_store(&state, &scope);
    let model_catalog = state.llm_client.model_catalog();
    let projection = store
        .diagnostics_projection_with_model_catalog(&run_id, model_catalog.as_deref())
        .await
        .map_err(map_model_lane_diagnostics_error)?;
    Ok(Json(projection))
}

/// Map a store error to an API error without letting a scope denial leak the
/// withheld resource's metadata (HBR-PRIV-004).
fn map_model_lane_diagnostics_error(
    error: crate::swarm_orchestration::model_lane::ModelLaneError,
) -> ModelLaneDiagnosticsApiError {
    use crate::swarm_orchestration::model_lane::ModelLaneError;
    match error {
        ModelLaneError::NotFound(_) => {
            ModelLaneDiagnosticsApiError::not_found("model lane run not found")
        }
        ModelLaneError::ScopeDenied(denied) => ModelLaneDiagnosticsApiError::scope_denied(&denied),
        // A database that is unreachable is NOT an integrity failure. Routing it
        // to 500 (as this did after the scoped-store change removed the inline
        // SQL probes) tells the caller "this server is broken" when the truthful
        // answer is "the authority store is unavailable, retry" - and it makes a
        // transient outage indistinguishable from a real corruption bug, which is
        // the one distinction an operator staring at diagnostics needs most.
        ModelLaneError::Sqlx(err) => ModelLaneDiagnosticsApiError::authority_unavailable(err),
        other => ModelLaneDiagnosticsApiError::integrity(other),
    }
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

    /// The storage-unavailable contract. The two ModelLane handlers no longer
    /// run their own inline SQL probes (those probes were the unscoped
    /// disclosure the scoped-store change removed), so this is now reached
    /// through `map_model_lane_diagnostics_error`'s `Sqlx` arm instead.
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

    /// The denial reason code only. `ScopeDenied`'s own Display carries no
    /// identifiers, and the stored owner is never echoed.
    fn scope_denied(denied: &crate::swarm_orchestration::resource_scope::ScopeDenied) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "MODEL_LANE_DIAGNOSTICS_SCOPE_DENIED",
            detail: denied.reason_code().to_owned(),
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
