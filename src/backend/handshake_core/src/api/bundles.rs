use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bundles::{
    bundle_path, BundleExportError, BundleScope, DebugBundleExporter, DefaultDebugBundleExporter,
    ExportableFilter, ExportableInventory, RedactionMode,
};
use crate::jobs::create_job;
use crate::models::JobKind;
use crate::storage::{EntityRef, JobState};
use crate::workflows::start_workflow_for_job;
use crate::AppState;

#[derive(Debug, Deserialize, Clone)]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub redaction_mode: RedactionMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExportScope {
    pub kind: String,
    #[serde(default)]
    pub problem_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub time_range: Option<TimeRangeRequest>,
    #[serde(default)]
    pub wsid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeRangeRequest {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub export_job_id: String,
    pub status: String,
    pub estimated_size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BundleStatus {
    pub bundle_id: String,
    pub status: String,
    pub manifest: Option<serde_json::Value>,
    pub error: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub findings: Vec<crate::bundles::ValidationFinding>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/bundles/debug/export", post(export_bundle))
        .route("/api/bundles/debug/exportable", get(list_exportable))
        .route("/api/bundles/debug/:bundle_id", get(bundle_status))
        .route(
            "/api/bundles/debug/:bundle_id/validate",
            post(validate_bundle),
        )
        .route(
            "/api/bundles/debug/:bundle_id/download",
            get(download_bundle),
        )
        .with_state(state)
}

fn parse_scope(scope: ExportScope) -> Result<BundleScope, BundleExportError> {
    match scope.kind.as_str() {
        "problem" => scope
            .problem_id
            .map(|id| BundleScope::Problem { diagnostic_id: id })
            .ok_or_else(|| BundleExportError::InvalidScope("problem_id required".to_string())),
        "job" => scope
            .job_id
            .map(|id| BundleScope::Job { job_id: id })
            .ok_or_else(|| BundleExportError::InvalidScope("job_id required".to_string())),
        "time_window" => {
            if let Some(range) = scope.time_range {
                let start = chrono::DateTime::parse_from_rfc3339(&range.start)
                    .map_err(|e| {
                        BundleExportError::InvalidScope(format!(
                            "invalid time_range.start (expected RFC3339): {e}"
                        ))
                    })?
                    .with_timezone(&chrono::Utc);
                let end = chrono::DateTime::parse_from_rfc3339(&range.end)
                    .map_err(|e| {
                        BundleExportError::InvalidScope(format!(
                            "invalid time_range.end (expected RFC3339): {e}"
                        ))
                    })?
                    .with_timezone(&chrono::Utc);
                Ok(BundleScope::TimeWindow {
                    start,
                    end,
                    wsid: scope.wsid,
                })
            } else {
                Err(BundleExportError::InvalidScope(
                    "time_range required for time_window scope".to_string(),
                ))
            }
        }
        "workspace" => scope
            .wsid
            .map(|wsid| BundleScope::Workspace { wsid })
            .ok_or_else(|| BundleExportError::InvalidScope("wsid required".to_string())),
        _ => Err(BundleExportError::InvalidScope(format!(
            "invalid scope.kind: {}",
            &scope.kind
        ))),
    }
}

async fn export_bundle(
    State(state): State<AppState>,
    Json(request): Json<ExportRequest>,
) -> Result<(StatusCode, Json<ExportResponse>), (StatusCode, String)> {
    // Validate the request is well-formed per spec before queuing the job.
    let _parsed_scope =
        parse_scope(request.scope.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let scope_value = serde_json::to_value(&request.scope)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let job_kind = JobKind::DebugBundleExport;
    let capability_profile = state
        .capability_registry
        .profile_for_job(job_kind.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut entity_refs: Vec<EntityRef> = Vec::new();
    if let Some(wsid) = request.scope.wsid.clone() {
        entity_refs.push(EntityRef {
            entity_id: wsid,
            entity_kind: "workspace".to_string(),
        });
    }

    let job_inputs = json!({
        "scope": scope_value,
        "redaction_mode": request.redaction_mode,
        "include_artifacts": false,
    });

    let job = create_job(
        &state,
        job_kind,
        "hsk.bundle.export.v0",
        // Server-enforced capability profile to prevent client-side escalation.
        capability_profile.id.as_str(),
        Some(job_inputs),
        entity_refs,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let export_job_id = job.job_id.to_string();
    let state_clone = state.clone();
    tokio::spawn(async move {
        let _ = start_workflow_for_job(&state_clone, job).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(ExportResponse {
            export_job_id,
            status: "queued".to_string(),
            estimated_size_bytes: None,
        }),
    ))
}

async fn bundle_status(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
) -> Result<Json<BundleStatus>, (StatusCode, String)> {
    let mut status = "pending".to_string();
    let mut error: Option<String> = None;

    if let Ok(job) = state.storage.get_ai_job(&bundle_id).await {
        status = match job.state {
            JobState::Queued
            | JobState::Running
            | JobState::AwaitingUser
            | JobState::AwaitingValidation
            | JobState::Stalled => "pending".to_string(),
            JobState::Completed | JobState::CompletedWithIssues => "ready".to_string(),
            JobState::Failed | JobState::Poisoned | JobState::Cancelled => {
                error = job
                    .error_message
                    .clone()
                    .or_else(|| Some(job.state.as_str().to_string()));
                "failed".to_string()
            }
        };
    }

    let default_dir = PathBuf::from("data")
        .join("bundles")
        .join(format!("bundle-{}", bundle_id));
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);

    let manifest_path = path.join("bundle_manifest.json");
    if status == "pending" && manifest_path.exists() {
        status = "ready".to_string();
    }
    let mut manifest_value = if status == "ready" && manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some(
            serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        )
    } else {
        None
    };

    let expires_at = if let Some(manifest_value) = manifest_value.as_ref() {
        let manifest: crate::bundles::schemas::BundleManifest =
            serde_json::from_value(manifest_value.clone())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some((manifest.created_at + chrono::Duration::hours(24)).to_rfc3339())
    } else {
        None
    };

    if let Some(ref expires_at_str) = expires_at {
        if let Ok(expires_at_dt) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
            if chrono::Utc::now() > expires_at_dt.with_timezone(&chrono::Utc) {
                status = "expired".to_string();
                manifest_value = None;
            }
        }
    }

    Ok(Json(BundleStatus {
        bundle_id,
        status,
        manifest: manifest_value,
        error,
        expires_at,
    }))
}

async fn download_bundle(Path(bundle_id): Path<String>) -> Result<Response, (StatusCode, String)> {
    let default_dir = PathBuf::from("data")
        .join("bundles")
        .join(format!("bundle-{}", bundle_id));
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    }
    let zip_path = path.join(format!("{}.zip", bundle_id));
    let bytes = std::fs::read(&zip_path).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let mut response = Response::new(axum::body::Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/zip"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{}.zip\"", bundle_id))
            .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );
    Ok(response)
}

async fn list_exportable(
    State(state): State<AppState>,
    axum::extract::Query(filter): axum::extract::Query<ExportableFilter>,
) -> Result<Json<ExportableInventory>, (StatusCode, String)> {
    let exporter = DefaultDebugBundleExporter::new(state);
    let inventory = exporter
        .list_exportable(filter)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inventory))
}

async fn validate_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
) -> Result<Json<ValidationResponse>, (StatusCode, String)> {
    let default_dir = PathBuf::from("data")
        .join("bundles")
        .join(format!("bundle-{}", bundle_id));
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    }
    let exporter = DefaultDebugBundleExporter::new(state);
    let report = exporter
        .validate(&path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ValidationResponse {
        valid: report.valid,
        findings: report
            .findings
            .into_iter()
            .map(|f| crate::bundles::ValidationFinding {
                severity: f.severity,
                code: f.code,
                message: f.message,
                file: f.file,
                path: f.path,
            })
            .collect(),
    }))
}
