use serde::{Deserialize, Serialize};

pub use crate::flight_recorder::events_llm_infer::{
    FR_EVT_LLM_INFER_LORA_MOUNT, FR_EVT_LLM_INFER_LORA_SWAP, FR_EVT_LLM_INFER_LORA_UNMOUNT,
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
