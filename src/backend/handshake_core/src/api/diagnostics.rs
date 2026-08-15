use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::OnceLock};
use uuid::Uuid;

use crate::api::account_scope::RequestAccountScope;
use crate::diagnostics::{
    DiagFilter, Diagnostic, DiagnosticInput, DiagnosticSeverity, DiagnosticSurface, ProblemGroup,
};
use crate::swarm_orchestration::model_lane::{ModelLaneDiagnosticsProjection, ModelLaneStore};
use crate::AppState;

#[derive(Debug, Serialize)]
struct ScopedModelLaneDiagnosticsProjection {
    #[serde(flatten)]
    projection: ModelLaneDiagnosticsProjection,
    resource_scope: PrivacySafeResourceScope,
}

#[derive(Debug, Serialize)]
struct PrivacySafeResourceScope {
    owner_account_fingerprint: String,
    actor_principal_fingerprint: String,
    authenticated_session_fingerprint: String,
    access_space_fingerprint: String,
    workspace_fingerprint: String,
    visibility: &'static str,
    denial_posture: &'static str,
}

#[derive(Debug)]
struct ScopeFingerprintError;

impl std::fmt::Display for ScopeFingerprintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("scope fingerprint key unavailable")
    }
}

fn initialize_scope_fingerprint_key(
    fill: impl FnOnce(&mut [u8; 32]) -> Result<(), ()>,
) -> Result<[u8; 32], ScopeFingerprintError> {
    let mut key = [0_u8; 32];
    fill(&mut key).map_err(|()| ScopeFingerprintError)?;
    Ok(key)
}

fn scope_process_key() -> Result<&'static [u8; 32], ScopeFingerprintError> {
    static PROCESS_KEY: OnceLock<Result<[u8; 32], ScopeFingerprintError>> = OnceLock::new();
    PROCESS_KEY
        .get_or_init(|| {
            initialize_scope_fingerprint_key(|key| getrandom::getrandom(key).map_err(|_| ()))
        })
        .as_ref()
        .map_err(|_| ScopeFingerprintError)
}

fn scope_fingerprint(value: &str) -> Result<String, ScopeFingerprintError> {
    Ok(blake3::keyed_hash(scope_process_key()?, value.as_bytes())
        .to_hex()
        .to_string())
}

fn scoped_model_lane_diagnostics(
    projection: ModelLaneDiagnosticsProjection,
    scope: &RequestAccountScope,
) -> Result<ScopedModelLaneDiagnosticsProjection, ModelLaneDiagnosticsApiError> {
    let exact = scope.exact();
    let raw_values = [
        exact.owner_account_id.to_string(),
        exact.actor_principal_id.to_string(),
        exact.authenticated_session_id.to_string(),
        exact.access_space_id.to_string(),
        exact.workspace_id.to_string(),
    ];
    let fingerprints = raw_values
        .iter()
        .map(|raw| scope_fingerprint(raw))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ModelLaneDiagnosticsApiError::integrity)?;
    let mut projection_value =
        serde_json::to_value(projection).map_err(ModelLaneDiagnosticsApiError::integrity)?;
    fn redact_text(text: &mut String, replacements: &[(&str, &str)]) {
        for (raw, fingerprint) in replacements {
            *text = text.replace(raw, &format!("[scope-fingerprint:{fingerprint}]"));
        }
    }
    fn redact(value: &mut serde_json::Value, replacements: &[(&str, &str)]) {
        match value {
            serde_json::Value::String(text) => redact_text(text, replacements),
            serde_json::Value::Array(values) => {
                for value in values {
                    redact(value, replacements);
                }
            }
            serde_json::Value::Object(values) => {
                let previous = std::mem::take(values);
                for (mut key, mut value) in previous {
                    redact_text(&mut key, replacements);
                    redact(&mut value, replacements);
                    values.insert(key, value);
                }
            }
            _ => {}
        }
    }
    let replacements = raw_values
        .iter()
        .zip(fingerprints.iter())
        .map(|(raw, fingerprint)| (raw.as_str(), fingerprint.as_str()))
        .collect::<Vec<_>>();
    redact(&mut projection_value, &replacements);
    let projection = serde_json::from_value(projection_value)
        .map_err(ModelLaneDiagnosticsApiError::integrity)?;
    Ok(ScopedModelLaneDiagnosticsProjection {
        projection,
        resource_scope: PrivacySafeResourceScope {
            owner_account_fingerprint: fingerprints[0].clone(),
            actor_principal_fingerprint: fingerprints[1].clone(),
            authenticated_session_fingerprint: fingerprints[2].clone(),
            access_space_fingerprint: fingerprints[3].clone(),
            workspace_fingerprint: fingerprints[4].clone(),
            visibility: "private_exact_scope_only",
            denial_posture: "foreign_scope_is_absent_restricted_metadata_withheld",
        },
    })
}

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
) -> Result<Json<ScopedModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
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
    Ok(Json(scoped_model_lane_diagnostics(projection, &scope)?))
}

async fn get_model_lane_diagnostics(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(run_id): Path<String>,
) -> Result<Json<ScopedModelLaneDiagnosticsProjection>, ModelLaneDiagnosticsApiError> {
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
    Ok(Json(scoped_model_lane_diagnostics(projection, &scope)?))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_fingerprint_key_failure_maps_to_fail_closed_internal_error() {
        let error = initialize_scope_fingerprint_key(|_| Err(()))
            .expect_err("CSPRNG failure must not fabricate a fingerprint key");
        let api_error = ModelLaneDiagnosticsApiError::integrity(error);

        assert_eq!(api_error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(api_error.code, "MODEL_LANE_DIAGNOSTICS_INTEGRITY_FAILURE");
        assert_eq!(api_error.detail, "scope fingerprint key unavailable");
    }
}
