//! MT-105 adjacent Tauri IPC surface for INF-5 CAA extraction.
//!
//! Wraps `handshake_core::model_runtime::techniques::caa::extract_caa_vector`
//! for the Inference Lab CaaWizard. Mirrors the MT-096 steering /
//! MT-102-adjacent refusal command shape: the inner function preflights
//! validation + capability checks and returns `live_runtime_unavailable`
//! until MT-074 (LlamaCppRuntime streaming) unblocks the runtime path.

use handshake_core::model_runtime::{LayerIndex, RuntimeBinding};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::model_runtime::{parse_model_id, ModelRuntimeState};

pub const KERNEL_MODEL_RUNTIME_CAA_EXTRACT_IPC_CHANNEL: &str =
    "kernel_model_runtime_caa_extract";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaaPromptPairIpc {
    pub context: String,
    pub positive: String,
    pub negative: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaaExtractRequestIpc {
    pub model_id: String,
    pub name: String,
    pub description: String,
    pub pairs: Vec<CaaPromptPairIpc>,
    pub layer: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaaExtractResultIpc {
    pub vector_id: String,
    pub values: Vec<f32>,
    pub layer: u32,
    pub event_type: String,
}

#[tauri::command]
pub async fn kernel_model_runtime_caa_extract(
    request: CaaExtractRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<CaaExtractResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_CAA_EXTRACT_IPC_CHANNEL;
    caa_extract(request, state.inner())
}

pub fn caa_extract(
    request: CaaExtractRequestIpc,
    state: &ModelRuntimeState,
) -> Result<CaaExtractResultIpc, String> {
    if request.name.trim().is_empty() {
        return Err("CAA extraction requires a non-empty vector name".to_string());
    }
    if request.description.trim().is_empty() {
        return Err("CAA extraction requires a non-empty description".to_string());
    }
    if request.pairs.is_empty() {
        return Err("CAA extraction requires at least one prompt pair".to_string());
    }
    for (idx, pair) in request.pairs.iter().enumerate() {
        if pair.positive.trim().is_empty() {
            return Err(format!(
                "CAA extraction pair {idx} positive completion must not be blank"
            ));
        }
        if pair.negative.trim().is_empty() {
            return Err(format!(
                "CAA extraction pair {idx} negative completion must not be blank"
            ));
        }
    }
    let _ = LayerIndex::new(request.layer);
    let binding = preflight_caa(&request.model_id, state)?;
    Err(live_runtime_unavailable(binding))
}

fn preflight_caa(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<RuntimeBinding, String> {
    let model_id = parse_model_id(model_id)?;
    state.activation_steering_command_binding(model_id)
}

fn live_runtime_unavailable(binding: RuntimeBinding) -> String {
    format!(
        "caa live model runtime manager is not attached for adapter {}; command surface is registered but cannot dispatch to caa::extract_caa_vector yet",
        binding.adapter_id()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::model_runtime::ModelId;

    fn request(model_id: ModelId) -> CaaExtractRequestIpc {
        CaaExtractRequestIpc {
            model_id: model_id.to_string(),
            name: "syco-caa".to_string(),
            description: "CAA sycophancy".to_string(),
            pairs: vec![CaaPromptPairIpc {
                context: "ctx".to_string(),
                positive: "yes".to_string(),
                negative: "no".to_string(),
            }],
            layer: 14,
        }
    }

    #[test]
    fn caa_extract_dto_uses_camel_case_serde() {
        let value = serde_json::to_value(request(ModelId::new_v7())).expect("serialize");
        assert!(value.get("modelId").is_some());
        assert!(value.get("pairs").is_some());
        assert!(value.get("model_id").is_none());
    }

    #[test]
    fn caa_extract_rejects_empty_inputs() {
        let state = ModelRuntimeState::default();
        let model_id = ModelId::new_v7();

        let mut bad = request(model_id);
        bad.name = "".to_string();
        assert!(caa_extract(bad, &state).unwrap_err().contains("name"));

        let mut bad = request(model_id);
        bad.description = "".to_string();
        assert!(caa_extract(bad, &state).unwrap_err().contains("description"));

        let mut bad = request(model_id);
        bad.pairs.clear();
        assert!(caa_extract(bad, &state).unwrap_err().contains("pair"));

        let mut bad = request(model_id);
        bad.pairs[0].positive = "  ".to_string();
        assert!(caa_extract(bad, &state).unwrap_err().contains("positive"));

        let mut bad = request(model_id);
        bad.pairs[0].negative = "".to_string();
        assert!(caa_extract(bad, &state).unwrap_err().contains("negative"));
    }
}
