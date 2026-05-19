//! MT-102 adjacent Tauri IPC surface for INF-4 refusal vector extraction.
//!
//! Wraps `handshake_core::model_runtime::techniques::refusal_vector` for the
//! Inference Lab UI. Like the steering surface in MT-096, the inner extract
//! function preflights validation and capability checks but returns
//! `live_runtime_unavailable` until MT-074 (LlamaCppRuntime streaming)
//! unblocks the live runtime path.

use handshake_core::model_runtime::{LayerIndex, RuntimeBinding};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::model_runtime::{parse_model_id, ModelRuntimeState};

pub const KERNEL_MODEL_RUNTIME_REFUSAL_EXTRACT_IPC_CHANNEL: &str =
    "kernel_model_runtime_refusal_extract";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusalExtractRequestIpc {
    pub model_id: String,
    pub harmful_prompts: Vec<String>,
    pub harmless_prompts: Vec<String>,
    pub layers: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusalDirectionIpc {
    pub layer: u32,
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusalExtractResultIpc {
    pub directions: Vec<RefusalDirectionIpc>,
    pub event_type: String,
}

#[tauri::command]
pub async fn kernel_model_runtime_refusal_extract(
    request: RefusalExtractRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<RefusalExtractResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_REFUSAL_EXTRACT_IPC_CHANNEL;
    refusal_extract(request, state.inner())
}

pub fn refusal_extract(
    request: RefusalExtractRequestIpc,
    state: &ModelRuntimeState,
) -> Result<RefusalExtractResultIpc, String> {
    if request.harmful_prompts.is_empty() {
        return Err("refusal extraction requires at least one harmful prompt".to_string());
    }
    if request
        .harmful_prompts
        .iter()
        .any(|prompt| prompt.trim().is_empty())
    {
        return Err("refusal extraction harmful prompts must not be blank".to_string());
    }
    if request.harmless_prompts.is_empty() {
        return Err("refusal extraction requires at least one harmless prompt".to_string());
    }
    if request
        .harmless_prompts
        .iter()
        .any(|prompt| prompt.trim().is_empty())
    {
        return Err("refusal extraction harmless prompts must not be blank".to_string());
    }
    if request.layers.is_empty() {
        return Err("refusal extraction requires at least one layer".to_string());
    }
    for layer in &request.layers {
        let _ = LayerIndex::new(*layer);
    }

    let binding = preflight_refusal(&request.model_id, state)?;
    Err(live_runtime_unavailable(binding))
}

fn preflight_refusal(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<RuntimeBinding, String> {
    let model_id = parse_model_id(model_id)?;
    state.activation_steering_command_binding(model_id)
}

fn live_runtime_unavailable(binding: RuntimeBinding) -> String {
    format!(
        "refusal_vector live model runtime manager is not attached for adapter {}; command surface is registered but cannot dispatch to refusal_vector::extract_refusal_direction yet",
        binding.adapter_id()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::model_runtime::ModelId;

    fn request(model_id: ModelId) -> RefusalExtractRequestIpc {
        RefusalExtractRequestIpc {
            model_id: model_id.to_string(),
            harmful_prompts: vec!["harmful prompt".to_string()],
            harmless_prompts: vec!["harmless prompt".to_string()],
            layers: vec![14],
        }
    }

    #[test]
    fn refusal_extract_dto_uses_camel_case_serde() {
        let value = serde_json::to_value(RefusalExtractRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            harmful_prompts: vec!["a".to_string()],
            harmless_prompts: vec!["b".to_string()],
            layers: vec![14],
        })
        .expect("serialize");
        assert!(value.get("modelId").is_some());
        assert!(value.get("harmfulPrompts").is_some());
        assert!(value.get("harmlessPrompts").is_some());
        assert!(value.get("model_id").is_none());
    }

    #[test]
    fn refusal_extract_rejects_empty_inputs() {
        let state = ModelRuntimeState::default();
        let model_id = ModelId::new_v7();

        let mut bad = request(model_id);
        bad.harmful_prompts.clear();
        let err = refusal_extract(bad, &state).expect_err("empty harmful");
        assert!(err.contains("harmful"), "{err}");

        let mut bad = request(model_id);
        bad.harmless_prompts.clear();
        let err = refusal_extract(bad, &state).expect_err("empty harmless");
        assert!(err.contains("harmless"), "{err}");

        let mut bad = request(model_id);
        bad.layers.clear();
        let err = refusal_extract(bad, &state).expect_err("empty layers");
        assert!(err.contains("layer"), "{err}");

        let mut bad = request(model_id);
        bad.harmful_prompts = vec!["   ".to_string()];
        let err = refusal_extract(bad, &state).expect_err("blank harmful");
        assert!(err.contains("blank"), "{err}");
    }
}
