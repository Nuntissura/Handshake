//! INF-2 KV cache public technique surface.
//!
//! Lifts the per-adapter `KvCacheOps` (LlamaCpp + Candle StateVector) into a
//! ModelRuntime-level technique API for the Inference Lab Work Profile
//! knob `settings.exec_policy.kv_cache`. Mirrors `lora_hotswap` exactly:
//! every entry point preflights the capability gate, dispatches through
//! the `KvCacheHandle::with_ops`-bearing handle, and returns a typed
//! receipt carrying the canonical `FR-EVT-LLM-INFER-KV-*` event_type.
//!
//! Capability gating:
//! - `set_quantization`             → `supports_kv_quantization != None`
//! - `prefix_commit`/`restore`      → `supports_kv_prefix_cache`
//! - `evict_all`                    → `supports_kv_prefix_cache`
//! - `occupancy`                    → either prefix_cache OR quantization
//!   (telemetry is read-only; failing closed only when neither is
//!   supported keeps the UI knob hidden on truly KV-less adapters).

pub use crate::flight_recorder::events_llm_infer::{
    FR_EVT_LLM_INFER_KV_EVICT, FR_EVT_LLM_INFER_KV_PREFIX_COMMIT,
    FR_EVT_LLM_INFER_KV_PREFIX_RESTORE, FR_EVT_LLM_INFER_KV_SET_QUANTIZATION,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::model_runtime::{
    KvCacheHandle, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelId, ModelRuntime,
    ModelRuntimeError,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvQuantSetReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub previous_quantization: KvQuantSupport,
    pub current_quantization: KvQuantSupport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvPrefixCommitReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandle,
    pub occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvPrefixRestoreReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandle,
    pub occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvEvictAllReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub previous_occupancy: KvCacheStats,
    pub current_occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvOccupancyReceipt {
    pub model_id: ModelId,
    pub occupancy: KvCacheStats,
}

/// Set the active KV quantization level. Capability gate: the model's
/// declared `supports_kv_quantization` must not be `KvQuantSupport::None`;
/// the adapter is still free to reject an unsupported level (e.g. Q4
/// on a Q8-only adapter) with a typed `KvCacheError`.
pub fn set_quantization(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    level: KvQuantSupport,
) -> Result<KvQuantSetReceipt, ModelRuntimeError> {
    let stack = require_kv_quantization(runtime, model_id)?;
    let previous = stack.quantization();
    stack.set_quantization(level)?;
    let current = stack.quantization();
    Ok(KvQuantSetReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_KV_SET_QUANTIZATION.to_string(),
        previous_quantization: previous,
        current_quantization: current,
    })
}

/// Commit a prefix to the KV cache. Capability gate: declared
/// `supports_kv_prefix_cache` must be true.
pub fn prefix_commit(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    prefix_tokens: &[u32],
) -> Result<KvPrefixCommitReceipt, ModelRuntimeError> {
    let stack = require_kv_prefix_cache(runtime, model_id)?;
    // MT-095: bind a per-process replay-resistance derivation onto the handle
    // the adapter returns (adapters produce handles with no binding via
    // from_scoped_tokens). This is the public-surface enforcement point that
    // makes the prefix handle replay-resistant end-to-end; restore verifies it.
    let prefix_handle = stack
        .prefix_commit(prefix_tokens)?
        .bind_derived(model_id, Utc::now().timestamp_micros());
    let occupancy = stack.occupancy();
    Ok(KvPrefixCommitReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_KV_PREFIX_COMMIT.to_string(),
        prefix_handle,
        occupancy,
    })
}

/// Restore a previously committed prefix. Capability gate: declared
/// `supports_kv_prefix_cache` must be true. The adapter validates the
/// handle's `content_hash` against the stored entry; tampered handles
/// surface as `KvCacheError` (already enforced in MT-063 / MT-075).
pub fn prefix_restore(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    prefix_handle: &KvPrefixHandle,
) -> Result<KvPrefixRestoreReceipt, ModelRuntimeError> {
    let stack = require_kv_prefix_cache(runtime, model_id)?;
    // MT-095: enforce replay-resistance before restoring. Rejects handles that
    // were never bound at commit, had their content_hash tampered, or were
    // forged/replayed under a different process or model key.
    prefix_handle.verify_self_against(model_id)?;
    stack.prefix_restore(prefix_handle)?;
    let occupancy = stack.occupancy();
    Ok(KvPrefixRestoreReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_KV_PREFIX_RESTORE.to_string(),
        prefix_handle: prefix_handle.clone(),
        occupancy,
    })
}

/// Evict every committed prefix from the cache. Reuses the existing
/// `FR-EVT-LLM-INFER-KV-EVICT` registry entry.
pub fn evict_all(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvEvictAllReceipt, ModelRuntimeError> {
    let stack = require_kv_prefix_cache(runtime, model_id)?;
    let previous = stack.occupancy();
    stack.evict_all()?;
    let current = stack.occupancy();
    Ok(KvEvictAllReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_KV_EVICT.to_string(),
        previous_occupancy: previous,
        current_occupancy: current,
    })
}

/// Read-only telemetry. Capability gate: at least one of
/// `supports_kv_prefix_cache` or `supports_kv_quantization` must be
/// declared, otherwise the adapter genuinely has no KV cache surface
/// and the operator's Inference Lab knob should be hidden.
pub fn occupancy(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvOccupancyReceipt, ModelRuntimeError> {
    let stack = require_kv_cache(runtime, model_id)?;
    let occupancy = stack.occupancy();
    Ok(KvOccupancyReceipt {
        model_id,
        occupancy,
    })
}

// ----------------------------------------------------------------------------
// Capability gates.
// ----------------------------------------------------------------------------

fn require_kv_quantization(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvCacheHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if capabilities.supports_kv_quantization == KvQuantSupport::None {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache_quantization".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.kv_cache(model_id)
}

fn require_kv_prefix_cache(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvCacheHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if !capabilities.supports_kv_prefix_cache {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache_prefix".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.kv_cache(model_id)
}

fn require_kv_cache(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvCacheHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if !capabilities.supports_kv_prefix_cache
        && capabilities.supports_kv_quantization == KvQuantSupport::None
    {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.kv_cache(model_id)
}
