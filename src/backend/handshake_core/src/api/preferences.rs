//! HTTP surface for the Settings & Preferences domain (Master Spec v02.201 §10.17).
//!
//! WP-KERNEL-012 MT-072: the editor settings dialog reads/writes editor preferences through this
//! canonical typed [`PreferenceRecord`](crate::preferences::PreferenceRecord) surface backed by
//! the durable store + EventLedger, replacing the opaque workspace-settings JSON document. Every route is
//! workspace-scoped and confined to the registry-defined editor preferences (SET-SCOPE-001
//! `view-defaults`). SQLite is forbidden anywhere in this domain (SET-STORE-002).
//!
//! PENDING SURREALDB PORT (WP-KERNEL-012 MT-136): the preference store methods
//! on the `Database` trait have no implementor — `SurrealDatabase` does not
//! provide them, so the default bodies fail closed with `NotImplemented` and no
//! preference is persisted today.
//!
//! Routes (SET-UI-001/002/003):
//! * `GET    /workspaces/:workspace_id/preferences`                        redacted projection (SET-PROJ)
//! * `GET    /workspaces/:workspace_id/preferences/:preference_id`         resolved record (SET-REC-003)
//! * `PUT    /workspaces/:workspace_id/preferences/:preference_id`         typed set (SET-REC-002)
//! * `POST   /workspaces/:workspace_id/preferences/:preference_id/reset`   reset-to-default (SET-UI-002)
//! * `GET    /workspaces/:workspace_id/preferences/:preference_id/history` change history (SET-UI-003)

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::preferences::{
    editor_preference_registry, lookup_editor_preference, PreferenceScope, PreferenceSource,
};
use crate::AppState;

const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";

/// The preference routes, merged into the product router by `crate::api`.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/preferences",
            get(list_preferences),
        )
        .route(
            "/workspaces/:workspace_id/preferences/:preference_id",
            get(get_preference).put(set_preference),
        )
        .route(
            "/workspaces/:workspace_id/preferences/:preference_id/reset",
            post(reset_preference),
        )
        .route(
            "/workspaces/:workspace_id/preferences/:preference_id/history",
            get(preference_history),
        )
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

fn error(status: StatusCode, code: &str, message: &str) -> ApiError {
    (
        status,
        Json(json!({ "error": code, "message": message })),
    )
}

fn actor_of(headers: &HeaderMap) -> String {
    headers
        .get(HSK_HEADER_ACTOR_ID)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("operator")
        .to_owned()
}

async fn ensure_workspace(state: &AppState, workspace_id: &str) -> Result<(), ApiError> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(error(
            StatusCode::NOT_FOUND,
            "workspace_not_found",
            "workspace does not exist",
        )),
        Err(err) => Err(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            &err.to_string(),
        )),
    }
}

fn require_entry(preference_id: &str) -> Result<crate::preferences::PreferenceSchemaEntry, ApiError> {
    lookup_editor_preference(preference_id).ok_or_else(|| {
        error(
            StatusCode::NOT_FOUND,
            "unknown_preference",
            &format!("'{preference_id}' is not a defined editor preference"),
        )
    })
}

fn db_error(err: crate::storage::StorageError) -> ApiError {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "db_error",
        &err.to_string(),
    )
}

async fn list_preferences(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_workspace(&state, &workspace_id).await?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let entries = editor_preference_registry();
    let rows = state
        .storage
        .preference_projection(&scope, &entries)
        .await
        .map_err(db_error)?;
    Ok(Json(json!({
        "schema_id": "hsk.preference_projection@1",
        "workspace_id": workspace_id,
        "scope": scope.kind.as_str(),
        "preferences": rows,
    })))
}

async fn get_preference(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    ensure_workspace(&state, &workspace_id).await?;
    let entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let record = state
        .storage
        .preference_get(&scope, &entry)
        .await
        .map_err(db_error)?;
    Ok(Json(json!({ "record": record })))
}

#[derive(Debug, Deserialize)]
struct SetPreferenceRequest {
    value: Value,
}

async fn set_preference(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<SetPreferenceRequest>,
) -> Result<Json<Value>, ApiError> {
    ensure_workspace(&state, &workspace_id).await?;
    let entry = require_entry(&preference_id)?;
    // SET-REC-002: typed validation before commit; failures are explicit structured 400s.
    if let Err(validation) = entry.validate(&payload.value) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "preference_validation_failed",
                "validation": validation,
            })),
        ));
    }
    let scope = PreferenceScope::workspace(&workspace_id);
    let actor = actor_of(&headers);
    let (record, receipt) = state
        .storage
        .preference_set(&scope, &entry, payload.value, PreferenceSource::Operator, &actor)
        .await
        .map_err(db_error)?;
    Ok(Json(json!({ "record": record, "receipt": receipt })))
}

async fn reset_preference(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    ensure_workspace(&state, &workspace_id).await?;
    let entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let actor = actor_of(&headers);
    let (record, receipt) = state
        .storage
        .preference_reset(&scope, &entry, &actor)
        .await
        .map_err(db_error)?;
    Ok(Json(json!({ "record": record, "receipt": receipt })))
}

async fn preference_history(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    ensure_workspace(&state, &workspace_id).await?;
    let _entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let receipts = state
        .storage
        .preference_history(&scope, &preference_id)
        .await
        .map_err(db_error)?;
    Ok(Json(json!({
        "preference_id": preference_id,
        "workspace_id": workspace_id,
        "receipts": receipts,
    })))
}
