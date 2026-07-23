//! ModelRuntime registry projection and deterministic READY-model selector.
//!
//! PostgreSQL owns durable artifact-to-adapter selection. The process-local
//! [`ModelCatalog`](crate::model_runtime::ModelCatalog) owns only current boot
//! readiness. This endpoint joins those authorities by artifact SHA-256; a
//! boot-scoped UUID is never used as restart identity.

use std::{collections::BTreeMap, path::Path};

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    llm::{
        ModelRuntimeControlAction, ModelRuntimeControlReceipt, ModelRuntimeControlRequest,
        ModelRuntimeInspection, ModelRuntimeKvInspection, ModelRuntimeLoraInspection,
        ModelRuntimeSteeringInspection, ModelRuntimeValue, MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
    },
    model_runtime::{
        ModelCatalogEntry, ModelRegistryPersistenceError, ModelRegistryStore, ModelRuntimeRole,
        ModelRuntimeSelectionPurpose, PersistedActiveModelSelection,
    },
    workflows::{
        ModelSwapPriority, ModelSwapRequestV0_4, ModelSwapRequesterSubsystem,
        ModelSwapRequesterV0_4, ModelSwapRole, ModelSwapStrategy,
    },
    AppState,
};

pub const MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID: &str =
    "hsk.model_runtime_registry_projection@3";
pub const MODEL_RUNTIME_REGISTRY_ROUTE: &str = "/model-runtime/registry";
pub const MODEL_RUNTIME_SELECTION_ROUTE: &str = "/model-runtime/selection";
pub const MODEL_RUNTIME_CONTROL_ROUTE: &str = "/model-runtime/control";
pub const MODEL_RUNTIME_PROCESS_OWNERSHIP_ROUTE: &str =
    "/model-runtime/process-ownership/:process_uuid";
pub const MODEL_RUNTIME_SELECTION_INVALID_CODE: &str = "MODEL_RUNTIME_SELECTION_INVALID";
pub const MODEL_RUNTIME_SELECTION_REJECTED_CODE: &str = "MODEL_RUNTIME_SELECTION_REJECTED";
pub const MODEL_RUNTIME_CONTROL_INVALID_CODE: &str = "MODEL_RUNTIME_CONTROL_INVALID";
pub const MODEL_RUNTIME_CONTROL_REJECTED_CODE: &str = "MODEL_RUNTIME_CONTROL_REJECTED";
pub const MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE: &str =
    "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR";
pub const MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE: &str = "MODEL_RUNTIME_REGISTRY_UNAVAILABLE";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeRegistryRowState {
    Live,
    Dormant,
}

/// One operator-readable durable registry row joined to current boot state.
///
/// `last_observed_runtime_model_id` is intentionally absent. A dormant row has
/// no current live identity, even though PostgreSQL retains the last
/// observation for audit/recovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeActionAvailability {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ModelRuntimeActionAvailability {
    fn disabled(reason: impl Into<String>) -> Self {
        Self {
            enabled: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeRegistryRow {
    pub artifact_sha256: String,
    pub artifact_locator: String,
    pub display_label: String,
    pub selected_adapter: String,
    pub selection_revision: u64,
    pub selection_audit_event_ref: String,
    pub runtime_role: ModelRuntimeRole,
    pub default_selectable: bool,
    pub runtime_state: ModelRuntimeRegistryRowState,
    pub active_purposes: Vec<ModelRuntimeSelectionPurpose>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_selection_revision: Option<u64>,
    pub selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_model_id: Option<String>,
    pub canonical_artifact_path: ModelRuntimeValue<String>,
    pub kv_cache: ModelRuntimeValue<ModelRuntimeKvInspection>,
    pub lora_stack: ModelRuntimeValue<Vec<ModelRuntimeLoraInspection>>,
    pub active_steering: ModelRuntimeValue<Vec<ModelRuntimeSteeringInspection>>,
    pub process_ownership_ledger_link: ModelRuntimeValue<String>,
    pub tokens_per_second: ModelRuntimeValue<f64>,
    pub vram_resident_bytes: ModelRuntimeValue<u64>,
    pub last_call_at_utc: ModelRuntimeValue<String>,
    pub last_call_age_seconds: ModelRuntimeValue<u64>,
    pub engine_internals: ModelRuntimeValue<Value>,
    pub quiesce_action: ModelRuntimeActionAvailability,
    pub unload_action: ModelRuntimeActionAvailability,
    pub compatible_adapter_swap_action: ModelRuntimeActionAvailability,
    pub inspect_engine_internals_action: ModelRuntimeActionAvailability,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeRegistryProjection {
    pub schema_id: String,
    pub generated_at_utc: DateTime<Utc>,
    pub catalog_revision: u64,
    pub rows: Vec<ModelRuntimeRegistryRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_receipt_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeProcessOwnershipRecord {
    pub schema_id: String,
    pub process_uuid: Uuid,
    pub os_pid: Option<i64>,
    pub engine_kind: String,
    pub started_at_utc: DateTime<Utc>,
    pub stopped_at_utc: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stop_reason: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub sandbox_adapter_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelRuntimeRegistryErrorBody {
    error: &'static str,
    detail: String,
}

#[derive(Debug, Deserialize)]
struct SelectReadyModelRequest {
    target_model_id: String,
    actor: String,
    reason: String,
}

#[derive(Debug)]
struct ModelRuntimeRegistryApiError {
    status: StatusCode,
    code: &'static str,
    detail: String,
}

impl ModelRuntimeRegistryApiError {
    fn integrity(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE,
            detail: detail.into(),
        }
    }

    fn bad_request(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            detail: detail.into(),
        }
    }

    fn conflict(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: MODEL_RUNTIME_SELECTION_REJECTED_CODE,
            detail: detail.into(),
        }
    }

    fn control_rejected(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: MODEL_RUNTIME_CONTROL_REJECTED_CODE,
            detail: detail.into(),
        }
    }
}

impl From<ModelRegistryPersistenceError> for ModelRuntimeRegistryApiError {
    fn from(error: ModelRegistryPersistenceError) -> Self {
        let status = match &error {
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
            | ModelRegistryPersistenceError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            code: MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE,
            detail: error.to_string(),
        }
    }
}

impl IntoResponse for ModelRuntimeRegistryApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ModelRuntimeRegistryErrorBody {
                error: self.code,
                detail: self.detail,
            }),
        )
            .into_response()
    }
}

async fn list_registry(
    State(state): State<AppState>,
) -> Result<Json<ModelRuntimeRegistryProjection>, ModelRuntimeRegistryApiError> {
    Ok(Json(build_registry_projection(&state).await?))
}

async fn get_process_ownership_record(
    State(state): State<AppState>,
    AxumPath(process_uuid): AxumPath<Uuid>,
) -> Result<Json<ModelRuntimeProcessOwnershipRecord>, ModelRuntimeRegistryApiError> {
    let row = sqlx::query(
        r#"
        SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at,
               exit_code, stop_reason, model_artifact_sha256, owner_role,
               owner_wp, sandbox_adapter_id
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(process_uuid)
    .fetch_optional(&state.postgres_pool)
    .await
    .map_err(ModelRegistryPersistenceError::Database)?
    .ok_or_else(|| {
        ModelRuntimeRegistryApiError::bad_request(
            MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE,
            format!("ProcessOwnershipLedger record {process_uuid} was not found"),
        )
    })?;
    Ok(Json(ModelRuntimeProcessOwnershipRecord {
        schema_id: "hsk.model_runtime_process_ownership@1".to_owned(),
        process_uuid: row
            .try_get("process_uuid")
            .map_err(ModelRegistryPersistenceError::Database)?,
        os_pid: row
            .try_get("os_pid")
            .map_err(ModelRegistryPersistenceError::Database)?,
        engine_kind: row
            .try_get("engine_kind")
            .map_err(ModelRegistryPersistenceError::Database)?,
        started_at_utc: row
            .try_get("started_at")
            .map_err(ModelRegistryPersistenceError::Database)?,
        stopped_at_utc: row
            .try_get("stopped_at")
            .map_err(ModelRegistryPersistenceError::Database)?,
        exit_code: row
            .try_get("exit_code")
            .map_err(ModelRegistryPersistenceError::Database)?,
        stop_reason: row
            .try_get("stop_reason")
            .map_err(ModelRegistryPersistenceError::Database)?,
        model_artifact_sha256: row
            .try_get("model_artifact_sha256")
            .map_err(ModelRegistryPersistenceError::Database)?,
        owner_role: row
            .try_get("owner_role")
            .map_err(ModelRegistryPersistenceError::Database)?,
        owner_wp: row
            .try_get("owner_wp")
            .map_err(ModelRegistryPersistenceError::Database)?,
        sandbox_adapter_id: row
            .try_get("sandbox_adapter_id")
            .map_err(ModelRegistryPersistenceError::Database)?,
    }))
}

async fn build_registry_projection(
    state: &AppState,
) -> Result<ModelRuntimeRegistryProjection, ModelRuntimeRegistryApiError> {
    let generated_at_utc = Utc::now();
    let store = ModelRegistryStore::new(state.postgres_pool.clone());
    let durable_rows = store.list_recoverable().await?;
    let active_selections = store.list_active_selections().await?;
    let ownership_rows = sqlx::query(
        r#"
        SELECT DISTINCT ON (model_artifact_sha256)
               model_artifact_sha256, process_uuid
        FROM kernel_process_lifecycle
        WHERE model_artifact_sha256 IS NOT NULL
        ORDER BY model_artifact_sha256, started_at DESC, process_uuid DESC
        "#,
    )
    .fetch_all(&state.postgres_pool)
    .await
    .map_err(ModelRegistryPersistenceError::Database)?;
    let mut process_by_artifact = BTreeMap::<String, Uuid>::new();
    for row in ownership_rows {
        process_by_artifact.insert(
            row.try_get("model_artifact_sha256")
                .map_err(ModelRegistryPersistenceError::Database)?,
            row.try_get("process_uuid")
                .map_err(ModelRegistryPersistenceError::Database)?,
        );
    }
    let selection_receipt_ref = active_selections
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .map(|selection| {
            format!(
                "eventledger://kernel/{}",
                selection.selection_updated_event_id
            )
        })
        .ok_or_else(|| {
            ModelRuntimeRegistryApiError::integrity(
                "PostgreSQL application/default selection receipt is absent",
            )
        })?;
    let mut active_by_artifact = BTreeMap::<String, Vec<PersistedActiveModelSelection>>::new();
    let mut active_by_purpose = BTreeMap::new();
    for active in active_selections {
        if active.runtime_role != active.purpose.runtime_role() {
            return Err(ModelRuntimeRegistryApiError::integrity(format!(
                "active purpose {} carries runtime role {}",
                active.purpose.as_str(),
                active.runtime_role.as_str()
            )));
        }
        if active_by_purpose
            .insert(active.purpose, active.artifact_sha256)
            .is_some()
        {
            return Err(ModelRuntimeRegistryApiError::integrity(format!(
                "active purpose {} appears more than once",
                active.purpose.as_str()
            )));
        }
        active_by_artifact
            .entry(hex::encode(active.artifact_sha256))
            .or_default()
            .push(active);
    }
    let catalog = state.model_catalog();
    let catalog_revision = catalog
        .as_ref()
        .map(|catalog| catalog.runtime_availability_revision())
        .unwrap_or(0);
    let live_entries = catalog
        .as_ref()
        .map(|catalog| catalog.list())
        .unwrap_or_default();

    let mut ready_adapter_counts = BTreeMap::<String, usize>::new();
    for entry in live_entries.iter().filter(|entry| entry.ready) {
        *ready_adapter_counts
            .entry(entry.runtime_binding.clone())
            .or_default() += 1;
    }

    let mut catalog_by_artifact = BTreeMap::<String, ModelCatalogEntry>::new();
    for entry in live_entries {
        validate_catalog_sha256(&entry.artifact_sha256)?;
        if catalog_by_artifact
            .insert(entry.artifact_sha256.clone(), entry)
            .is_some()
        {
            return Err(ModelRuntimeRegistryApiError::integrity(
                "multiple catalog entries claim the same artifact SHA-256",
            ));
        }
    }

    let mut rows = Vec::with_capacity(durable_rows.len());
    for durable in durable_rows {
        let artifact_sha256 = hex::encode(durable.artifact_sha256);
        let catalog_entry = catalog_by_artifact.remove(&artifact_sha256);
        if let Some(entry) = catalog_entry.as_ref() {
            let expected_adapter = durable.runtime_binding.adapter_id();
            if entry.runtime_binding != expected_adapter {
                return Err(ModelRuntimeRegistryApiError::integrity(format!(
                    "catalog adapter `{}` disagrees with durable adapter `{expected_adapter}` for artifact {artifact_sha256}",
                    entry.runtime_binding
                )));
            }
            if entry.supports_embedding != durable.declared_capabilities.supports_embedding
                || entry.embedding_dimension != durable.declared_capabilities.embedding_dimension
            {
                return Err(ModelRuntimeRegistryApiError::integrity(format!(
                    "catalog embedding capability ({}/{:?}) disagrees with durable embedding capability ({}/{:?}) for artifact {artifact_sha256}",
                    entry.supports_embedding,
                    entry.embedding_dimension,
                    durable.declared_capabilities.supports_embedding,
                    durable.declared_capabilities.embedding_dimension
                )));
            }
            if entry.runtime_role != durable.runtime_role
                || entry.default_selectable != durable.runtime_role.default_selectable()
            {
                return Err(ModelRuntimeRegistryApiError::integrity(format!(
                    "catalog runtime role {:?} / default_selectable {} disagrees with durable runtime role {:?} for artifact {artifact_sha256}",
                    entry.runtime_role, entry.default_selectable, durable.runtime_role
                )));
            }
            if entry.ready {
                let durable_model_id = durable.last_observed_runtime_model_id.to_string();
                if entry.model_id != durable_model_id {
                    return Err(ModelRuntimeRegistryApiError::integrity(format!(
                        "READY catalog model id `{}` was not committed as the durable last-observed model id `{durable_model_id}` for artifact {artifact_sha256}",
                        entry.model_id
                    )));
                }
                let durable_label = durable.base_model_tag.as_str();
                if entry.base_model_tag != durable_label || entry.display_name != durable_label {
                    return Err(ModelRuntimeRegistryApiError::integrity(format!(
                        "READY catalog label `{}` / display `{}` disagrees with durable last-observed label `{durable_label}` for artifact {artifact_sha256}",
                        entry.base_model_tag, entry.display_name
                    )));
                }
            }
        }

        let ready = catalog_entry.as_ref().is_some_and(|entry| entry.ready);
        let runtime_state = if ready {
            ModelRuntimeRegistryRowState::Live
        } else {
            ModelRuntimeRegistryRowState::Dormant
        };
        let active_for_artifact = active_by_artifact
            .remove(&artifact_sha256)
            .unwrap_or_default();
        if active_for_artifact
            .iter()
            .any(|active| active.runtime_role != durable.runtime_role)
        {
            return Err(ModelRuntimeRegistryApiError::integrity(format!(
                "active purpose role disagrees with durable registry role for artifact {artifact_sha256}"
            )));
        }
        let active_purposes = active_for_artifact
            .iter()
            .map(|active| active.purpose)
            .collect::<Vec<_>>();
        let selected = active_purposes.contains(&ModelRuntimeSelectionPurpose::ApplicationDefault);
        let active_selection_revision = active_for_artifact
            .first()
            .map(|active| active.selection_revision);
        let live_model_id = catalog_entry
            .as_ref()
            .filter(|entry| entry.ready)
            .map(|entry| entry.model_id.clone());
        let control_capabilities = live_model_id
            .as_deref()
            .map(|model_id| {
                state
                    .llm_client
                    .model_runtime_control_capabilities(model_id)
            })
            .unwrap_or_default();
        let ready_adapter_siblings = ready_adapter_counts
            .get(durable.runtime_binding.adapter_id())
            .copied()
            .unwrap_or_default()
            .saturating_sub(usize::from(ready));
        let unavailable_reason = if ready {
            "live runtime inspection did not return this field".to_owned()
        } else {
            "artifact is not READY in the current boot".to_owned()
        };
        let inspection = live_model_id
            .as_deref()
            .map(|model_id| state.llm_client.inspect_model_runtime(model_id))
            .unwrap_or_else(|| ModelRuntimeInspection::unavailable(unavailable_reason.clone()));
        let canonical_artifact_path = catalog_entry
            .as_ref()
            .filter(|entry| entry.ready)
            .map(|entry| canonical_artifact_path(&entry.artifact_path))
            .unwrap_or_else(|| ModelRuntimeValue::unavailable(unavailable_reason.clone()));
        let process_ownership_ledger_link = process_by_artifact
            .get(&artifact_sha256)
            .map(|process_uuid| {
                ModelRuntimeValue::available(format!(
                    "process-ownership-ledger://process/{process_uuid}"
                ))
            })
            .unwrap_or_else(|| {
                ModelRuntimeValue::unavailable(
                    "no ProcessOwnershipLedger record exists for this artifact",
                )
            });
        let inspect_engine_internals_action = match &inspection.engine_internals {
            ModelRuntimeValue::Available { .. } => ModelRuntimeActionAvailability {
                enabled: true,
                reason: None,
            },
            ModelRuntimeValue::Unavailable { reason } => {
                ModelRuntimeActionAvailability::disabled(reason.clone())
            }
        };
        let last_call_age_seconds = last_call_age(&inspection.last_call_at_utc, &generated_at_utc);
        rows.push(ModelRuntimeRegistryRow {
            artifact_sha256,
            artifact_locator: durable.artifact_locator,
            display_label: durable.base_model_tag.as_str().to_owned(),
            selected_adapter: durable.runtime_binding.adapter_id().to_owned(),
            selection_revision: durable.selection_revision,
            selection_audit_event_ref: format!(
                "eventledger://kernel/{}",
                durable.selection_updated_event_id
            ),
            runtime_role: durable.runtime_role,
            default_selectable: durable.runtime_role.default_selectable(),
            runtime_state,
            active_purposes,
            active_selection_revision,
            selected,
            live_model_id,
            canonical_artifact_path,
            kv_cache: inspection.kv_cache,
            lora_stack: inspection.lora_stack,
            active_steering: inspection.active_steering,
            process_ownership_ledger_link,
            tokens_per_second: inspection.tokens_per_second,
            vram_resident_bytes: inspection.vram_resident_bytes,
            last_call_at_utc: inspection.last_call_at_utc,
            last_call_age_seconds,
            engine_internals: inspection.engine_internals,
            quiesce_action: if ready && control_capabilities.quiesce {
                ModelRuntimeActionAvailability {
                    enabled: true,
                    reason: None,
                }
            } else {
                ModelRuntimeActionAvailability::disabled(if ready {
                    "the current LLM client cannot receipt runtime quiesce".to_owned()
                } else {
                    unavailable_reason.clone()
                })
            },
            unload_action: if !ready {
                ModelRuntimeActionAvailability::disabled(unavailable_reason.clone())
            } else if !control_capabilities.unload {
                ModelRuntimeActionAvailability::disabled(
                    "the current runtime has no matching embedded lifecycle authority",
                )
            } else if selected {
                ModelRuntimeActionAvailability::disabled(
                    "the active application/default model must be rebound before unload",
                )
            } else if ready_adapter_siblings != 0 {
                ModelRuntimeActionAvailability::disabled(format!(
                    "shared {} runtime still owns {ready_adapter_siblings} other READY model(s)",
                    durable.runtime_binding.adapter_id()
                ))
            } else {
                ModelRuntimeActionAvailability {
                    enabled: true,
                    reason: None,
                }
            },
            compatible_adapter_swap_action: if !ready {
                ModelRuntimeActionAvailability::disabled(unavailable_reason.clone())
            } else if !control_capabilities.swap_compatible_adapter {
                ModelRuntimeActionAvailability::disabled(
                    "the current runtime lacks lifecycle, durable ledger, or selection-rebind authority",
                )
            } else if ready_adapter_siblings != 0 {
                ModelRuntimeActionAvailability::disabled(format!(
                    "shared {} runtime still owns {ready_adapter_siblings} other READY model(s)",
                    durable.runtime_binding.adapter_id()
                ))
            } else {
                ModelRuntimeActionAvailability {
                    enabled: true,
                    reason: None,
                }
            },
            inspect_engine_internals_action,
        });
    }

    if let Some((artifact_sha256, _)) = catalog_by_artifact.first_key_value() {
        return Err(ModelRuntimeRegistryApiError::integrity(format!(
            "catalog artifact {artifact_sha256} has no durable model registry row"
        )));
    }
    if let Some((artifact_sha256, active)) = active_by_artifact.first_key_value() {
        return Err(ModelRuntimeRegistryApiError::integrity(format!(
            "active purpose {} references absent registry artifact {artifact_sha256}",
            active
                .first()
                .map(|selection| selection.purpose.as_str())
                .unwrap_or("unknown")
        )));
    }

    Ok(ModelRuntimeRegistryProjection {
        schema_id: MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID.to_owned(),
        generated_at_utc,
        catalog_revision,
        rows,
        selection_receipt_ref: Some(selection_receipt_ref),
    })
}

async fn control_model_runtime(
    State(state): State<AppState>,
    Json(request): Json<ModelRuntimeControlRequest>,
) -> Result<Json<ModelRuntimeControlReceipt>, ModelRuntimeRegistryApiError> {
    if request.schema_version != MODEL_RUNTIME_CONTROL_SCHEMA_VERSION {
        return Err(ModelRuntimeRegistryApiError::bad_request(
            MODEL_RUNTIME_CONTROL_INVALID_CODE,
            format!(
                "unsupported model runtime control schema {}; expected {}",
                request.schema_version, MODEL_RUNTIME_CONTROL_SCHEMA_VERSION
            ),
        ));
    }
    bounded_token_with_code(
        MODEL_RUNTIME_CONTROL_INVALID_CODE,
        "model_id",
        &request.model_id,
        128,
    )?;
    if request.timeout_ms == 0 || request.timeout_ms > 30_000 {
        return Err(ModelRuntimeRegistryApiError::bad_request(
            MODEL_RUNTIME_CONTROL_INVALID_CODE,
            "timeout_ms must be in 1..=30000",
        ));
    }
    if let ModelRuntimeControlAction::SwapCompatibleAdapter { target_adapter } = &request.action {
        bounded_token_with_code(
            MODEL_RUNTIME_CONTROL_INVALID_CODE,
            "target_adapter",
            target_adapter,
            64,
        )?;
    }
    state
        .llm_client
        .control_model_runtime(request)
        .await
        .map(Json)
        .map_err(|error| ModelRuntimeRegistryApiError::control_rejected(error.to_string()))
}

fn last_call_age(value: &ModelRuntimeValue<String>, now: &DateTime<Utc>) -> ModelRuntimeValue<u64> {
    let ModelRuntimeValue::Available { value } = value else {
        return ModelRuntimeValue::unavailable(
            "last-call time is unavailable, so elapsed time cannot be computed",
        );
    };
    let observed = match DateTime::parse_from_rfc3339(value) {
        Ok(value) => value.with_timezone(&Utc),
        Err(error) => {
            return ModelRuntimeValue::unavailable(format!(
                "last-call time is not valid RFC3339: {error}"
            ))
        }
    };
    let elapsed = now.signed_duration_since(observed).num_seconds();
    if elapsed < 0 {
        ModelRuntimeValue::unavailable("last-call time is in the future")
    } else {
        ModelRuntimeValue::available(elapsed as u64)
    }
}

async fn select_ready_model(
    State(state): State<AppState>,
    Json(request): Json<SelectReadyModelRequest>,
) -> Result<Json<ModelRuntimeRegistryProjection>, ModelRuntimeRegistryApiError> {
    let target_model_id = bounded_token("target_model_id", &request.target_model_id, 128)?;
    let actor = bounded_token("actor", &request.actor, 128)?;
    let reason = bounded_token("reason", &request.reason, 512)?;
    // Complete every fallible authority/projection/target check before the
    // durable swap commits and publishes the current-boot router projection.
    let mut projection = build_registry_projection(&state).await?;
    let current_row = projection
        .rows
        .iter()
        .find(|row| row.selected)
        .ok_or_else(|| {
            ModelRuntimeRegistryApiError::integrity(
                "PostgreSQL application/default selection is absent",
            )
        })?;
    let current_model_id = current_row.live_model_id.clone().ok_or_else(|| {
        ModelRuntimeRegistryApiError::conflict(
            "PostgreSQL application/default does not resolve to a READY current-boot model",
        )
    })?;
    let current_selection_revision = current_row.active_selection_revision.unwrap_or(0);
    if state.llm_client.selected_model_id() != current_model_id {
        return Err(ModelRuntimeRegistryApiError::integrity(format!(
            "current router projection disagrees with PostgreSQL application/default `{current_model_id}`"
        )));
    }
    let target_row = projection
        .rows
        .iter()
        .find(|row| row.live_model_id.as_deref() == Some(target_model_id))
        .ok_or_else(|| {
            ModelRuntimeRegistryApiError::conflict(format!(
                "CX-MM-001: target model {target_model_id} is not a current READY registry row"
            ))
        })?;
    if target_row.runtime_state != ModelRuntimeRegistryRowState::Live
        || !target_row.default_selectable
    {
        return Err(ModelRuntimeRegistryApiError::conflict(format!(
            "CX-MM-001: target model {target_model_id} has runtime role {:?} and is not eligible as the default completion model",
            target_row.runtime_role
        )));
    }
    let request_id = Uuid::now_v7().simple().to_string();
    let selection_receipt_ref = format!("model-runtime-selection://receipt/{request_id}");
    let state_ref =
        format!("model-runtime-selection://current/{current_model_id}/target/{target_model_id}");
    let state_hash = format!(
        "{:x}",
        Sha256::digest(format!("{current_model_id}\n{target_model_id}\n{request_id}\n").as_bytes())
    );
    let mut metadata = BTreeMap::<String, Value>::new();
    metadata.insert("actor".to_owned(), json!(actor));
    metadata.insert("surface".to_owned(), json!("native_model_runtime_panel"));
    metadata.insert(
        "selection_receipt_ref".to_owned(),
        json!(selection_receipt_ref),
    );
    let swap = ModelSwapRequestV0_4 {
        schema_version: "hsk.model_swap@0.4".to_owned(),
        request_id: request_id.clone(),
        current_model_id,
        target_model_id: target_model_id.to_owned(),
        role: ModelSwapRole::Orchestrator,
        priority: ModelSwapPriority::Normal,
        reason: reason.to_owned(),
        swap_strategy: ModelSwapStrategy::KeepHotSwap,
        state_persist_refs: vec![state_ref],
        state_hash,
        context_compile_ref: format!("model-runtime-panel://selection/{request_id}"),
        max_vram_mb: 0,
        max_ram_mb: 0,
        timeout_ms: 10_000,
        requester: ModelSwapRequesterV0_4 {
            subsystem: ModelSwapRequesterSubsystem::Ui,
            job_id: None,
            wp_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".to_owned()),
            mt_id: Some("MT-014".to_owned()),
        },
        metadata: Some(metadata),
    };
    // Do not place a cancellable wall-clock timeout around the authority
    // transaction. PostgreSQL lock/statement deadlines inside
    // ModelRegistryStore bound each operation and roll the transaction back;
    // cancelling this future could otherwise race after COMMIT but before the
    // process-local projection is advanced.
    state
        .llm_client
        .swap_model(swap)
        .await
        .map_err(|error| ModelRuntimeRegistryApiError::conflict(error.to_string()))?;
    // Do not perform a fallible database/projection read after mutation. The
    // validated pre-mutation projection is deterministically advanced in memory.
    for row in &mut projection.rows {
        let becomes_selected = row.live_model_id.as_deref() == Some(target_model_id);
        row.selected = becomes_selected;
        row.active_purposes
            .retain(|purpose| *purpose != ModelRuntimeSelectionPurpose::ApplicationDefault);
        if becomes_selected {
            row.active_purposes
                .push(ModelRuntimeSelectionPurpose::ApplicationDefault);
            row.active_selection_revision = Some(current_selection_revision.saturating_add(1));
        } else if row.active_purposes.is_empty() {
            row.active_selection_revision = None;
        }
    }
    projection.generated_at_utc = Utc::now();
    projection.selection_receipt_ref = Some(selection_receipt_ref);
    Ok(Json(projection))
}

fn bounded_token<'a>(
    field: &'static str,
    value: &'a str,
    max_len: usize,
) -> Result<&'a str, ModelRuntimeRegistryApiError> {
    bounded_token_with_code(MODEL_RUNTIME_SELECTION_INVALID_CODE, field, value, max_len)
}

fn bounded_token_with_code<'a>(
    code: &'static str,
    field: &'static str,
    value: &'a str,
    max_len: usize,
) -> Result<&'a str, ModelRuntimeRegistryApiError> {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(ModelRuntimeRegistryApiError::bad_request(
            code,
            format!("{field} must be non-empty, control-free, and at most {max_len} bytes"),
        ));
    }
    Ok(value)
}

fn validate_catalog_sha256(artifact_sha256: &str) -> Result<(), ModelRuntimeRegistryApiError> {
    if artifact_sha256.len() != 64
        || !artifact_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ModelRuntimeRegistryApiError::integrity(format!(
            "catalog artifact SHA-256 is not 64 lowercase hexadecimal characters: `{artifact_sha256}`"
        )));
    }
    Ok(())
}

fn canonical_artifact_path(path: &str) -> ModelRuntimeValue<String> {
    let path = Path::new(path);
    match std::fs::canonicalize(path) {
        Ok(canonical) => ModelRuntimeValue::available(canonical.to_string_lossy().into_owned()),
        Err(error) => ModelRuntimeValue::unavailable(format!(
            "catalog artifact path could not be canonicalized: {error}"
        )),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(MODEL_RUNTIME_REGISTRY_ROUTE, get(list_registry))
        .route(
            MODEL_RUNTIME_PROCESS_OWNERSHIP_ROUTE,
            get(get_process_ownership_record),
        )
        .route(MODEL_RUNTIME_SELECTION_ROUTE, post(select_ready_model))
        .route(MODEL_RUNTIME_CONTROL_ROUTE, post(control_model_runtime))
        .with_state(state)
}
