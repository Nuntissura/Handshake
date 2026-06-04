//! MT-112 — INF-9 Subquadratic technique public surface.
//!
//! Lifts the per-adapter SSM/RWKV state-vector implementations (MT-085
//! Mamba2 + MT-086 RWKV v5/v6 + MT-087 RWKV v7 + MT-088 state-vector)
//! into a ModelRuntime-level technique API for the Inference Lab
//! Work Profile knob `settings.exec_policy.subquadratic`. Mirrors the
//! `kv_cache_technique` / `speculative_decoding` shape:
//! every entry point preflights the `supports_subquadratic` capability
//! gate, dispatches through the per-adapter `KvCacheHandle` that the
//! Candle adapter wires from `StateVectorHandle::as_kv_cache_handle`
//! (see candle/state_vector.rs:369-374), and returns a typed receipt
//! carrying the canonical `FR-EVT-LLM-INFER-SUBQUAD-*` event_type.
//!
//! Cross-session persist/rehydrate (`persist_to_disk` / `load_from_disk`
//! in the MT-112 contract narrative) land in MT-117. This MT exposes
//! the public method names but fails closed with
//! `CapabilityNotSupported{ capability: "subquadratic_persist_disk_deferred_mt117" }`
//! so the UI knob (MT-113) can render them disabled with a deferral
//! tooltip — mirrors MT-110's Eagle-3 deferred-mode UX pattern.
//!
//! Reference models (from MT-085/086/087):
//! - `state-spaces/mamba2-2.7b` — Mamba2 2.7B production reference.
//! - `BlinkDL/rwkv-5-world` — RWKV v5 World series.
//! - `BlinkDL/rwkv-6-world-7b` — RWKV v6 reference.
//! - `BlinkDL/rwkv-7-world` (or `rwkv-7-g1-2.9b`) — RWKV v7 Goose/G1.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::model_runtime::{
    KvCacheHandle, KvCacheStats, KvPrefixHandle, ModelId, ModelRuntime, ModelRuntimeError,
};

pub const FR_EVT_LLM_INFER_SUBQUAD_LOAD: &str = "FR-EVT-LLM-INFER-SUBQUAD-LOAD";
pub const FR_EVT_LLM_INFER_SUBQUAD_STATE_COMMIT: &str = "FR-EVT-LLM-INFER-SUBQUAD-STATE-COMMIT";
pub const FR_EVT_LLM_INFER_SUBQUAD_STATE_RESTORE: &str = "FR-EVT-LLM-INFER-SUBQUAD-STATE-RESTORE";
pub const FR_EVT_LLM_INFER_SUBQUAD_PERSIST: &str = "FR-EVT-LLM-INFER-SUBQUAD-PERSIST";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubquadStateCommitReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandle,
    pub occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubquadStateRestoreReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandle,
    pub hit: bool,
    pub occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubquadStateListReceipt {
    pub model_id: ModelId,
    pub occupancy: KvCacheStats,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubquadEvictReceipt {
    pub model_id: ModelId,
    pub previous_occupancy: KvCacheStats,
    pub current_occupancy: KvCacheStats,
}

/// Commit the current SSM state under a prefix handle so it can be
/// restored later in the session. Capability gate:
/// `supports_subquadratic == true`. The Candle adapter wires
/// `kv_cache(model_id)` to the `StateVectorHandle` underlying the
/// loaded Mamba2/RWKV model (candle/adapter.rs:404-405); non-Candle
/// adapters declare `supports_subquadratic=false` and fail the gate.
pub fn state_commit(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    prefix_tokens: &[u32],
) -> Result<SubquadStateCommitReceipt, ModelRuntimeError> {
    let stack = require_subquadratic(runtime, model_id)?;
    // MT-095: bind the replay-resistance derivation at commit (the adapter
    // returns an unbound handle via from_scoped_tokens) so the centralized
    // KvCacheHandle restore chokepoint can verify it. Mirrors
    // kv_cache_technique::prefix_commit — previously subquadratic restore was
    // ungated, which the centralized chokepoint now closes.
    let prefix_handle = stack
        .prefix_commit(prefix_tokens)?
        .bind_derived(model_id, Utc::now().timestamp_micros());
    let occupancy = stack.occupancy();
    Ok(SubquadStateCommitReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_SUBQUAD_STATE_COMMIT.to_string(),
        prefix_handle,
        occupancy,
    })
}

/// Restore a previously committed SSM state. Capability gate:
/// `supports_subquadratic == true`. The Candle adapter validates the
/// handle's `content_hash` against the stored entry; tampered handles
/// surface as `KvCacheError` (enforced in MT-088 state_vector::InMemoryStateVectorOps).
pub fn state_restore(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    prefix_handle: &KvPrefixHandle,
) -> Result<SubquadStateRestoreReceipt, ModelRuntimeError> {
    let stack = require_subquadratic(runtime, model_id)?;
    // MT-095: restore is gated in the KvCacheHandle chokepoint (binding + TTL).
    stack.prefix_restore(model_id, prefix_handle)?;
    let occupancy = stack.occupancy();
    Ok(SubquadStateRestoreReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_SUBQUAD_STATE_RESTORE.to_string(),
        prefix_handle: prefix_handle.clone(),
        hit: true,
        occupancy,
    })
}

/// Read the in-memory state-vector cache occupancy. Capability gate:
/// `supports_subquadratic == true`. Returns the same `KvCacheStats`
/// shape the KV technique surface uses so the Inference Lab UI can
/// share the cache-occupancy widget across both techniques.
pub fn state_list(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<SubquadStateListReceipt, ModelRuntimeError> {
    let stack = require_subquadratic(runtime, model_id)?;
    Ok(SubquadStateListReceipt {
        model_id,
        occupancy: stack.occupancy(),
    })
}

/// Evict every committed state-vector entry from the cache. Capability
/// gate: `supports_subquadratic == true`. Returns the before/after
/// occupancy so the UI can show the operator how much memory was
/// reclaimed.
pub fn evict_all(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<SubquadEvictReceipt, ModelRuntimeError> {
    let stack = require_subquadratic(runtime, model_id)?;
    let previous = stack.occupancy();
    stack.evict_all()?;
    let current = stack.occupancy();
    Ok(SubquadEvictReceipt {
        model_id,
        previous_occupancy: previous,
        current_occupancy: current,
    })
}

/// Persist the current SSM state to disk for cross-session restore.
///
/// Deferred to MT-117 per the MT-112 contract narrative
/// ("cross-session state restore from MT-117"). This MT lands the
/// public method name + canonical error path so the IPC layer
/// (commands/subquadratic.rs) and UI knob (MT-113) can render the
/// deferred entry without a separate add-method-to-trait migration
/// when MT-117 lands. Mirrors MT-110's Eagle-3 deferred-mode UX
/// pattern: capability gate passes if the adapter supports
/// subquadratic, then the persist path itself fails closed with
/// the deferred-to-MT-117 marker.
pub fn persist(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    _prefix_handle: &KvPrefixHandle,
) -> Result<SubquadEvictReceipt, ModelRuntimeError> {
    let _ = require_subquadratic(runtime, model_id)?;
    Err(ModelRuntimeError::CapabilityNotSupported {
        capability: "subquadratic_persist_disk_deferred_mt117".to_string(),
        adapter: runtime.adapter_name().to_string(),
    })
}

/// Rehydrate a previously persisted SSM state from disk. Deferred to
/// MT-117 — see [`persist`] for the deferral rationale.
pub fn rehydrate(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<SubquadStateRestoreReceipt, ModelRuntimeError> {
    let _ = require_subquadratic(runtime, model_id)?;
    Err(ModelRuntimeError::CapabilityNotSupported {
        capability: "subquadratic_rehydrate_disk_deferred_mt117".to_string(),
        adapter: runtime.adapter_name().to_string(),
    })
}

// ----------------------------------------------------------------------------
// Capability gate.
// ----------------------------------------------------------------------------

fn require_subquadratic(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<KvCacheHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if !capabilities.supports_subquadratic {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "subquadratic".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.kv_cache(model_id)
}
