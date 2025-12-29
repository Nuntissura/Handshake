use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::bundles::{
    bundle_path, BundleScope, DebugBundleExporter, DebugBundleRequest, DefaultDebugBundleExporter,
    ExportableFilter, ExportableInventory, RedactionMode,
};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub redaction_mode: RedactionMode,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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
        .route(
            "/api/bundles/debug/:bundle_id",
            get(bundle_status).post(validate_bundle),
        )
        .route(
            "/api/bundles/debug/:bundle_id/download",
            get(download_bundle),
        )
        .with_state(state)
}

fn parse_scope(scope: ExportScope) -> Result<BundleScope, String> {
    match scope.kind.as_str() {
        "problem" => scope
            .problem_id
            .map(|id| BundleScope::Problem { diagnostic_id: id })
            .ok_or_else(|| "problem_id required".to_string()),
        "job" => scope
            .job_id
            .map(|id| BundleScope::Job { job_id: id })
            .ok_or_else(|| "job_id required".to_string()),
        "time_window" => {
            if let Some(range) = scope.time_range {
                let start = chrono::DateTime::parse_from_rfc3339(&range.start)
                    .map_err(|e| e.to_string())?
                    .with_timezone(&chrono::Utc);
                let end = chrono::DateTime::parse_from_rfc3339(&range.end)
                    .map_err(|e| e.to_string())?
                    .with_timezone(&chrono::Utc);
                Ok(BundleScope::TimeWindow {
                    start,
                    end,
                    wsid: scope.wsid,
                })
            } else {
                Err("time_range required".to_string())
            }
        }
        "workspace" => scope
            .wsid
            .map(|wsid| BundleScope::Workspace { wsid })
            .ok_or_else(|| "wsid required".to_string()),
        _ => Err("invalid scope.kind".to_string()),
    }
}

async fn export_bundle(
    State(state): State<AppState>,
    Json(request): Json<ExportRequest>,
) -> Result<Json<ExportResponse>, (StatusCode, String)> {
    let scope = parse_scope(request.scope).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let exporter = DefaultDebugBundleExporter::new(state);

    let manifest = exporter
        .export(DebugBundleRequest {
            scope,
            redaction_mode: request.redaction_mode,
            output_path: None,
            include_artifacts: false,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ExportResponse {
        export_job_id: manifest.bundle_id,
        status: "running".to_string(),
        estimated_size_bytes: None,
    }))
}

async fn bundle_status(
    Path(bundle_id): Path<String>,
) -> Result<Json<BundleStatus>, (StatusCode, String)> {
    let Some(path) = bundle_path(&bundle_id) else {
        return Ok(Json(BundleStatus {
            bundle_id,
            status: "pending".to_string(),
            manifest: None,
            error: None,
            expires_at: None,
        }));
    };

    let manifest_path = path.join("bundle_manifest.json");
    let manifest_json = match std::fs::read_to_string(&manifest_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(serde_json::Value::Null),
        Err(_) => serde_json::Value::Null,
    };

    Ok(Json(BundleStatus {
        bundle_id,
        status: "ready".to_string(),
        manifest: Some(manifest_json),
        error: None,
        expires_at: None,
    }))
}

async fn download_bundle(Path(bundle_id): Path<String>) -> Result<Response, (StatusCode, String)> {
    let Some(path) = bundle_path(&bundle_id) else {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    };
    let zip_path = path.join(format!("{}.zip", bundle_id));
    let bytes =
        std::fs::read(&zip_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
) -> Result<Json<ExportableInventory>, (StatusCode, String)> {
    let exporter = DefaultDebugBundleExporter::new(state);
    let inventory = exporter
        .list_exportable(ExportableFilter::default())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inventory))
}

async fn validate_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
) -> Result<Json<ValidationResponse>, (StatusCode, String)> {
    let Some(path) = bundle_path(&bundle_id) else {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    };
    let exporter = DefaultDebugBundleExporter::new(state);
    let report = exporter
        .validate(&PathBuf::from(path))
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
