use serde::{Deserialize, Serialize};

pub use crate::flight_recorder::events_llm_infer::{
    FR_EVT_LLM_INFER_LORA_MOUNT, FR_EVT_LLM_INFER_LORA_SWAP, FR_EVT_LLM_INFER_LORA_UNMOUNT,
};

use crate::distillation::candidate_registry::{
    mount_with_promotion_gate, CandidateRegistry, DistilledMountError,
};
use crate::model_runtime::{
    LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackSnapshot, LoraStrength,
    ModelId, ModelRuntime, ModelRuntimeError,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStackListReceipt {
    pub model_id: ModelId,
    pub active_stack: Vec<LoraStackEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStackMutationReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub active_stack: Vec<LoraStackEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStackSwapReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub previous_stack: LoraStackSnapshot,
    pub active_stack: Vec<LoraStackEntry>,
}

pub async fn mount(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    descriptor: LoraDescriptor,
    strength: LoraStrength,
) -> Result<LoraStackMutationReceipt, ModelRuntimeError> {
    let stack = require_lora_stack(runtime, model_id)?;
    stack.mount(descriptor, strength).await?;
    Ok(LoraStackMutationReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_LORA_MOUNT.to_string(),
        active_stack: stack.list_active(),
    })
}

/// Production mount entrypoint with mount-side PromotionGate enforcement
/// (MT-123 / AC-DISTILL-LOOP-SAFEGUARDS).
///
/// This is the gate the [`CandidateRegistry`] exists for. Before any LoRA is
/// loaded into the live stack, the descriptor's `lora_id` is checked against
/// the registry:
///
/// - A distilled candidate in `Pending`/`Rejected` review status is REFUSED
///   (fail closed) — the underlying `stack.mount` future is never awaited, so
///   an unpromoted distilled LoRA cannot reach a production session with zero
///   operator review.
/// - A `Promoted` candidate, or any LoRA that is not a registered distillation
///   candidate (externally-provided adapters), mounts normally.
///
/// `allow_unpromoted` is the operator's explicit per-session opt-in
/// (`settings.exec_policy.allow_unpromoted_distill`) for experimental mounts of
/// Pending candidates; it defaults to `false` and must only be set where a
/// legitimate experimental session needs it. It has no effect on
/// non-candidate or Promoted LoRAs.
pub async fn mount_gated(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    descriptor: LoraDescriptor,
    strength: LoraStrength,
    registry: &CandidateRegistry,
    allow_unpromoted: bool,
) -> Result<LoraStackMutationReceipt, DistilledMountError> {
    // Capability/loaded preflight runs before the gate so a missing stack is a
    // ModelRuntimeError, not a misleading mount-refusal.
    let stack = require_lora_stack(runtime, model_id).map_err(DistilledMountError::Mount)?;
    let lora_id = descriptor.id.to_string();
    mount_with_promotion_gate(registry, &lora_id, allow_unpromoted, || async {
        stack.mount(descriptor, strength).await
    })
    .await?;
    Ok(LoraStackMutationReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_LORA_MOUNT.to_string(),
        active_stack: stack.list_active(),
    })
}

pub async fn unmount(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    lora_id: LoraId,
) -> Result<LoraStackMutationReceipt, ModelRuntimeError> {
    let stack = require_lora_stack(runtime, model_id)?;
    stack.unmount(lora_id).await?;
    Ok(LoraStackMutationReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_LORA_UNMOUNT.to_string(),
        active_stack: stack.list_active(),
    })
}

pub fn list(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<LoraStackListReceipt, ModelRuntimeError> {
    let stack = require_lora_stack(runtime, model_id)?;
    Ok(LoraStackListReceipt {
        model_id,
        active_stack: stack.list_active(),
    })
}

pub async fn swap(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    new_stack: Vec<(LoraDescriptor, LoraStrength)>,
) -> Result<LoraStackSwapReceipt, ModelRuntimeError> {
    let stack = require_lora_stack(runtime, model_id)?;
    let previous_stack = stack.swap(new_stack).await?;
    Ok(LoraStackSwapReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_LORA_SWAP.to_string(),
        previous_stack,
        active_stack: stack.list_active(),
    })
}

fn require_lora_stack(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
) -> Result<LoraStackHandle, ModelRuntimeError> {
    let capabilities = runtime.capabilities(model_id)?;
    if !capabilities.supports_lora {
        return Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "lora_stack".to_string(),
            adapter: runtime.adapter_name().to_string(),
        });
    }
    runtime.lora_stack(model_id)
}
