//! MT-112 — INF-9 Subquadratic Tauri IPC bridge.
//!
//! Wraps `handshake_core::model_runtime::techniques::subquadratic::*`
//! with camelCase DTOs that the React `SubquadraticPanel` (MT-113) will
//! consume. Follows the MT-110 speculative IPC pattern (no per-command
//! gate enum added to `ModelRuntimeState`): the technique surface's
//! `require_subquadratic()` enforces the capability gate at dispatch
//! time, so this layer stays narrow.
//!
//! IPC channels (kernel_model_runtime_subquad_*):
//! - `state_commit`  — commit current SSM state under a prefix handle
//! - `state_restore` — restore a previously committed state
//! - `state_list`    — read in-memory cache occupancy
//! - `state_evict_all` — clear all committed states
//! - `persist`       — DEFERRED to MT-117 (cross-session restore);
//!                     fails closed with the deferred marker
//! - `rehydrate`     — DEFERRED to MT-117; fails closed
//!
//! Persist/rehydrate are present in the public surface so the
//! UI knob (MT-113) can render disabled controls with a deferral
//! tooltip rather than requiring a follow-up IPC migration when
//! MT-117 lands.

use std::sync::Arc;

use handshake_core::model_runtime::{
    techniques::subquadratic, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelId, ModelRuntime,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::model_runtime::ModelRuntimeState;

pub const KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_COMMIT_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_state_commit";
pub const KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_RESTORE_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_state_restore";
pub const KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_LIST_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_state_list";
pub const KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_EVICT_ALL_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_state_evict_all";
pub const KERNEL_MODEL_RUNTIME_SUBQUAD_PERSIST_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_persist";
pub const KERNEL_MODEL_RUNTIME_SUBQUAD_REHYDRATE_IPC_CHANNEL: &str =
    "kernel_model_runtime_subquad_rehydrate";
pub const SUBQUAD_NOT_AVAILABLE_PREFIX: &str = "subquad_not_available";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadCacheStatsIpc {
    pub bytes_used: u64,
    pub bytes_capacity: u64,
    pub prefix_cache_entries: u32,
    pub prefix_cache_hit_count: u64,
    pub prefix_cache_miss_count: u64,
    pub quant_level_current: KvQuantSupport,
}

impl From<KvCacheStats> for SubquadCacheStatsIpc {
    fn from(stats: KvCacheStats) -> Self {
        Self {
            bytes_used: stats.bytes_used,
            bytes_capacity: stats.bytes_capacity,
            prefix_cache_entries: stats.prefix_cache_entries,
            prefix_cache_hit_count: stats.prefix_cache_hit_count,
            prefix_cache_miss_count: stats.prefix_cache_miss_count,
            quant_level_current: stats.quant_level_current,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadPrefixHandleIpc {
    pub prefix_id: String,
    pub content_hash_hex: String,
    pub token_count: u32,
}

impl From<KvPrefixHandle> for SubquadPrefixHandleIpc {
    fn from(handle: KvPrefixHandle) -> Self {
        Self {
            prefix_id: handle.prefix_id().to_string(),
            content_hash_hex: hex::encode(handle.content_hash()),
            token_count: handle.token_count(),
        }
    }
}

impl SubquadPrefixHandleIpc {
    pub fn try_into_handle(self) -> Result<KvPrefixHandle, String> {
        let prefix_id = Uuid::parse_str(self.prefix_id.trim())
            .map_err(|error| format!("invalid prefix_id: {error}"))?;
        let bytes = hex::decode(self.content_hash_hex.trim())
            .map_err(|error| format!("invalid content_hash_hex: {error}"))?;
        let content_hash: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            format!("content_hash must decode to 32 bytes, got {}", bytes.len())
        })?;
        KvPrefixHandle::from_parts(prefix_id, content_hash, self.token_count)
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateCommitRequestIpc {
    pub model_id: String,
    pub prefix_tokens: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateCommitResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub prefix_handle: SubquadPrefixHandleIpc,
    pub occupancy: SubquadCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateRestoreRequestIpc {
    pub model_id: String,
    pub prefix_handle: SubquadPrefixHandleIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateRestoreResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub prefix_handle: SubquadPrefixHandleIpc,
    pub hit: bool,
    pub occupancy: SubquadCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateListRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadStateListResultIpc {
    pub model_id: String,
    pub occupancy: SubquadCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadEvictAllRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadEvictAllResultIpc {
    pub model_id: String,
    pub previous_occupancy: SubquadCacheStatsIpc,
    pub current_occupancy: SubquadCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadPersistRequestIpc {
    pub model_id: String,
    pub prefix_handle: SubquadPrefixHandleIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubquadRehydrateRequestIpc {
    pub model_id: String,
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_state_commit(
    request: SubquadStateCommitRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<SubquadStateCommitResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_COMMIT_IPC_CHANNEL;
    subquad_state_commit(request, state.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_state_restore(
    request: SubquadStateRestoreRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<SubquadStateRestoreResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_RESTORE_IPC_CHANNEL;
    subquad_state_restore(request, state.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_state_list(
    request: SubquadStateListRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<SubquadStateListResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_LIST_IPC_CHANNEL;
    subquad_state_list(request, state.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_state_evict_all(
    request: SubquadEvictAllRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<SubquadEvictAllResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_STATE_EVICT_ALL_IPC_CHANNEL;
    subquad_state_evict_all(request, state.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_persist(
    request: SubquadPersistRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_PERSIST_IPC_CHANNEL;
    subquad_persist(request, state.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_subquad_rehydrate(
    request: SubquadRehydrateRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_MODEL_RUNTIME_SUBQUAD_REHYDRATE_IPC_CHANNEL;
    subquad_rehydrate(request, state.inner())
}

pub fn subquad_state_commit(
    request: SubquadStateCommitRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SubquadStateCommitResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = subquadratic::state_commit(runtime.as_ref(), model_id, &request.prefix_tokens)
        .map_err(|error| format!("subquad state_commit dispatch failed: {error}"))?;
    Ok(SubquadStateCommitResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        prefix_handle: receipt.prefix_handle.into(),
        occupancy: receipt.occupancy.into(),
    })
}

pub fn subquad_state_restore(
    request: SubquadStateRestoreRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SubquadStateRestoreResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    let handle = request.prefix_handle.try_into_handle()?;
    let receipt = subquadratic::state_restore(runtime.as_ref(), model_id, &handle)
        .map_err(|error| format!("subquad state_restore dispatch failed: {error}"))?;
    Ok(SubquadStateRestoreResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        prefix_handle: receipt.prefix_handle.into(),
        hit: receipt.hit,
        occupancy: receipt.occupancy.into(),
    })
}

pub fn subquad_state_list(
    request: SubquadStateListRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SubquadStateListResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = subquadratic::state_list(runtime.as_ref(), model_id)
        .map_err(|error| format!("subquad state_list dispatch failed: {error}"))?;
    Ok(SubquadStateListResultIpc {
        model_id: receipt.model_id.to_string(),
        occupancy: receipt.occupancy.into(),
    })
}

pub fn subquad_state_evict_all(
    request: SubquadEvictAllRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SubquadEvictAllResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = subquadratic::evict_all(runtime.as_ref(), model_id)
        .map_err(|error| format!("subquad evict_all dispatch failed: {error}"))?;
    Ok(SubquadEvictAllResultIpc {
        model_id: receipt.model_id.to_string(),
        previous_occupancy: receipt.previous_occupancy.into(),
        current_occupancy: receipt.current_occupancy.into(),
    })
}

pub fn subquad_persist(
    request: SubquadPersistRequestIpc,
    state: &ModelRuntimeState,
) -> Result<(), String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    let handle = request.prefix_handle.try_into_handle()?;
    subquadratic::persist(runtime.as_ref(), model_id, &handle)
        .map(|_| ())
        .map_err(|error| format!("subquad persist deferred: {error}"))
}

pub fn subquad_rehydrate(
    request: SubquadRehydrateRequestIpc,
    state: &ModelRuntimeState,
) -> Result<(), String> {
    let model_id = parse_model_id(&request.model_id)?;
    let runtime = require_live_runtime(model_id, state)?;
    subquadratic::rehydrate(runtime.as_ref(), model_id)
        .map(|_| ())
        .map_err(|error| format!("subquad rehydrate deferred: {error}"))
}

fn require_live_runtime(
    model_id: ModelId,
    state: &ModelRuntimeState,
) -> Result<Arc<dyn ModelRuntime>, String> {
    state.live_runtime(model_id)?.ok_or_else(|| {
        format!(
            "{SUBQUAD_NOT_AVAILABLE_PREFIX}: subquadratic technique requires a live ModelRuntime adapter attached for model {model_id}; the adapter is not yet bound to this model in this app session"
        )
    })
}

fn parse_model_id(value: &str) -> Result<ModelId, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("model_id must not be empty".to_string());
    }
    let uuid = Uuid::parse_str(trimmed).map_err(|error| format!("invalid model_id: {error}"))?;
    if uuid.get_version_num() != 7 {
        return Err(format!("model_id must be UUID v7: {trimmed}"));
    }
    Ok(ModelId::from(uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subquad_state_commit_request_camel_case_serialization() {
        let value = serde_json::to_value(SubquadStateCommitRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            prefix_tokens: vec![1, 2, 3],
        })
        .expect("serialize state_commit request");
        assert!(value.get("modelId").is_some());
        assert!(value.get("model_id").is_none());
        assert!(value.get("prefixTokens").is_some());
    }

    #[test]
    fn subquad_prefix_handle_round_trips_via_dto() {
        let handle = KvPrefixHandle::from_tokens(&[5u32, 10, 15]).expect("v7 handle");
        let dto: SubquadPrefixHandleIpc = handle.clone().into();
        let restored = dto.try_into_handle().expect("dto round-trips");
        assert_eq!(restored, handle);
    }

    #[test]
    fn subquad_prefix_handle_rejects_invalid_hex() {
        let dto = SubquadPrefixHandleIpc {
            prefix_id: Uuid::now_v7().to_string(),
            content_hash_hex: "not-hex".to_string(),
            token_count: 1,
        };
        let err = dto.try_into_handle().expect_err("invalid hex must reject");
        assert!(err.contains("invalid content_hash_hex"), "{err}");
    }
}
