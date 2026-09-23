//! HTTP surface for the Settings & Preferences domain (Master Spec v02.201 §10.17).
//!
//! WP-KERNEL-012 MT-072: the editor settings dialog reads/writes editor preferences through this
//! canonical typed [`PreferenceRecord`](crate::preferences::PreferenceRecord) surface backed by
//! the durable store + EventLedger, replacing the opaque workspace-settings JSON document. Every route is
//! workspace-scoped and confined to the registry-defined editor preferences (SET-SCOPE-001
//! `view-defaults`). SQLite is forbidden anywhere in this domain (SET-STORE-002).
//!
//! Preference records and their EventLedger receipts are persisted by the
//! shared embedded `SurrealDatabase` implementation of the `Database` trait.
//!
//! MT-154 (Master Spec 02-system-architecture.md:2758/:2773/:2776, AC-154-3/AC-154-5): every route
//! authorizes the workspace resource through the ResourceBroker (Read + `fs.read` for reads,
//! Update + `fs.write` for writes) and runs as the account record user, so the
//! `preference_records` / `preference_change_receipts` table permissions are the data boundary. The
//! `PREFERENCE_RECORD_CHANGED` receipt carries the session principal, never a header actor.
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
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::authority::{authorize_request, constant_denial, AuthorizedResourceContext};
use crate::preferences::{
    editor_preference_registry, lookup_editor_preference, PreferenceScope, PreferenceSource,
};
use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
use crate::AppState;

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
        // MT-154 AC-154-2: deny by default before any extractor or table is touched.
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::authority::require_authenticated_session,
        ))
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

fn error(status: StatusCode, code: &str, message: &str) -> ApiError {
    (status, Json(json!({ "error": code, "message": message })))
}

/// Authorizes `action` on the workspace resource (reads `fs.read`, writes `fs.write`). Every failure
/// is the constant denial; it replaces the former root `get_workspace` existence probe.
async fn preference_workspace(
    state: &AppState,
    headers: &HeaderMap,
    workspace_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, ApiError> {
    let capability = if matches!(action, ResourceAction::Read) {
        "fs.read"
    } else {
        "fs.write"
    };
    authorize_request(
        state,
        headers,
        capability,
        ResourceKind::Workspace,
        workspace_id,
        action,
    )
    .await
    .map_err(|_| constant_denial())
}

/// The session principal every receipt written by this request carries.
fn session_actor(authority: &AuthorizedResourceContext) -> crate::kernel::KernelActor {
    match authority.actor_kind.as_str() {
        "system" => crate::kernel::KernelActor::System(authority.actor_id.clone()),
        _ => crate::kernel::KernelActor::Operator(authority.actor_id.clone()),
    }
}

/// Runs `operation` as the account record user, with receipts stamped with the session principal
/// and bound to `workspace_id` (`fn::mt154_workspace_receipt`).
async fn run_scoped<T>(
    state: &AppState,
    authority: &AuthorizedResourceContext,
    workspace_id: &str,
    operation: impl std::future::Future<Output = T>,
) -> T {
    state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            crate::storage::surreal::event_ledger::with_loom_session_receipt(
                session_actor(authority),
                workspace_id.to_owned(),
                operation,
            ),
        )
        .await
}

fn require_entry(
    preference_id: &str,
) -> Result<crate::preferences::PreferenceSchemaEntry, ApiError> {
    lookup_editor_preference(preference_id).ok_or_else(|| {
        error(
            StatusCode::NOT_FOUND,
            "unknown_preference",
            &format!("'{preference_id}' is not a defined editor preference"),
        )
    })
}

fn db_error(err: crate::storage::StorageError) -> ApiError {
    // MT-154 silent-deny ruling: a record-user write the table permissions dropped is the constant
    // 403, never 200/500.
    if matches!(err, crate::storage::StorageError::Guard(_))
        || err.to_string().contains("HSK-403-PROTECTED-RESOURCE")
    {
        return constant_denial();
    }
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "db_error",
        &err.to_string(),
    )
}

async fn list_preferences(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let authority =
        preference_workspace(&state, &headers, &workspace_id, ResourceAction::Read).await?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let entries = editor_preference_registry();
    let rows = run_scoped(
        &state,
        &authority,
        &workspace_id,
        state.storage.preference_projection(&scope, &entries),
    )
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
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let authority =
        preference_workspace(&state, &headers, &workspace_id, ResourceAction::Read).await?;
    let entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let record = run_scoped(
        &state,
        &authority,
        &workspace_id,
        state.storage.preference_get(&scope, &entry),
    )
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
    let authority =
        preference_workspace(&state, &headers, &workspace_id, ResourceAction::Update).await?;
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
    let actor = authority.actor_id.clone();
    let (record, receipt) = run_scoped(
        &state,
        &authority,
        &workspace_id,
        state.storage.preference_set(
            &scope,
            &entry,
            payload.value,
            PreferenceSource::Operator,
            &actor,
        ),
    )
    .await
    .map_err(db_error)?;
    Ok(Json(json!({ "record": record, "receipt": receipt })))
}

async fn reset_preference(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let authority =
        preference_workspace(&state, &headers, &workspace_id, ResourceAction::Update).await?;
    let entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let actor = authority.actor_id.clone();
    let (record, receipt) = run_scoped(
        &state,
        &authority,
        &workspace_id,
        state.storage.preference_reset(&scope, &entry, &actor),
    )
    .await
    .map_err(db_error)?;
    Ok(Json(json!({ "record": record, "receipt": receipt })))
}

async fn preference_history(
    State(state): State<AppState>,
    Path((workspace_id, preference_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let authority =
        preference_workspace(&state, &headers, &workspace_id, ResourceAction::Read).await?;
    let _entry = require_entry(&preference_id)?;
    let scope = PreferenceScope::workspace(&workspace_id);
    let receipts = run_scoped(
        &state,
        &authority,
        &workspace_id,
        state.storage.preference_history(&scope, &preference_id),
    )
    .await
    .map_err(db_error)?;
    Ok(Json(json!({
        "preference_id": preference_id,
        "workspace_id": workspace_id,
        "receipts": receipts,
    })))
}
