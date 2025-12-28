use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

use crate::diagnostics::{
    DiagFilter, Diagnostic, DiagnosticInput, DiagnosticSeverity, DiagnosticSurface, ProblemGroup,
};
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

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/diagnostics",
            get(list_diagnostics).post(create_diagnostic),
        )
        .route("/diagnostics/problems", get(list_problems))
        .route("/diagnostics/:id", get(get_diagnostic))
        .with_state(state)
}
