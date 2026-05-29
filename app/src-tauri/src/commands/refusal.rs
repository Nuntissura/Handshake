//! MT-102 + MT-100 adjacent Tauri IPC surface for INF-4 refusal vector extraction.
//!
//! Wraps `handshake_core::model_runtime::techniques::refusal_vector::extract_refusal_direction`
//! for the Inference Lab `RefusalVectorWizard`. Mirrors the MT-098 steering
//! command pattern:
//!
//! 1. Validate the request shape.
//! 2. Preflight `ModelRuntimeState::activation_steering_command_binding` so
//!    capability gating + loaded-handle gating runs identically to the
//!    pre-MT-082 contract (callers depend on the typed string errors).
//! 3. Dispatch into the live `dyn ModelRuntime` adapter attached to the model
//!    via `ModelRuntimeState::attach_live_runtime`. The adapter is the real
//!    production `CandleRuntime` in the app binary, or the `FakeCandleRuntime`
//!    test double in unit tests. Both compose the same `SteeringHookHandle` +
//!    `SteeringHookOps` contract.
//! 4. If no live runtime is attached for the model, return the typed
//!    `capture_not_available` reason — the same prefix MT-098 surfaces, so the
//!    `RefusalVectorWizard` extract-error view shows the same wording without
//!    branching.

use std::sync::Arc;

use handshake_core::model_runtime::{
    techniques::refusal_vector::{extract_refusal_direction, RefusalDirection},
    LayerIndex, ModelId, ModelRuntime,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::model_runtime::{parse_model_id, ModelRuntimeState};
use super::steering::STEERING_CAPTURE_NOT_AVAILABLE_PREFIX;

pub const KERNEL_MODEL_RUNTIME_REFUSAL_EXTRACT_IPC_CHANNEL: &str =
    "kernel_model_runtime_refusal_extract";

/// FR event tag emitted on a successful refusal-direction extraction. Mirrors
/// the steering capture/register surface so the UI can correlate extraction
/// events on the same operator-facing event channel.
pub const FR_EVT_LLM_INFER_REFUSAL_EXTRACT: &str = "FR-EVT-LLM-INFER-REFUSAL-EXTRACT";

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
    refusal_extract(request, state.inner()).await
}

pub async fn refusal_extract(
    request: RefusalExtractRequestIpc,
    state: &ModelRuntimeState,
) -> Result<RefusalExtractResultIpc, String> {
    validate_refusal_request(&request)?;
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "refusal_extract")?;

    let layers: Vec<LayerIndex> = request
        .layers
        .iter()
        .copied()
        .map(LayerIndex::new)
        .collect();

    let directions: Vec<RefusalDirection> = extract_refusal_direction(
        runtime.as_ref(),
        model_id,
        request.harmful_prompts.clone(),
        request.harmless_prompts.clone(),
        layers,
    )
    .await
    .map_err(|error| capture_not_available_message(&error.to_string()))?;

    Ok(RefusalExtractResultIpc {
        directions: directions
            .into_iter()
            .map(|d| RefusalDirectionIpc {
                layer: d.layer.as_u32(),
                values: d.values,
            })
            .collect(),
        event_type: FR_EVT_LLM_INFER_REFUSAL_EXTRACT.to_string(),
    })
}

fn validate_refusal_request(request: &RefusalExtractRequestIpc) -> Result<(), String> {
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
    Ok(())
}

fn preflight_capability_and_loaded(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<ModelId, String> {
    let model_id = parse_model_id(model_id)?;
    state.activation_steering_command_binding(model_id)?;
    Ok(model_id)
}

fn require_live_runtime(
    model_id: ModelId,
    state: &ModelRuntimeState,
    operation: &str,
) -> Result<Arc<dyn ModelRuntime>, String> {
    state.live_runtime(model_id)?.ok_or_else(|| {
        format!(
            "{STEERING_CAPTURE_NOT_AVAILABLE_PREFIX}: \
             {operation} requires a live ModelRuntime adapter attached for model {model_id}; \
             the adapter is not yet bound to this model in this app session"
        )
    })
}

fn capture_not_available_message(reason: &str) -> String {
    format!("{STEERING_CAPTURE_NOT_AVAILABLE_PREFIX}: {reason}")
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use chrono::Utc;
    use handshake_core::model_runtime::{
        BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, OperatorId, ProviderKind,
        RuntimeBinding,
    };

    use crate::commands::testing::{
        fake_runtime_with_unloaded_hooks, FakeCandleRuntime, FakeSteeringHookOps,
    };

    use super::*;

    fn request(model_id: ModelId) -> RefusalExtractRequestIpc {
        RefusalExtractRequestIpc {
            model_id: model_id.to_string(),
            harmful_prompts: vec!["harmful prompt one".to_string()],
            harmless_prompts: vec!["harmless prompt one".to_string()],
            layers: vec![14],
        }
    }

    fn registration(
        model_id: ModelId,
        runtime_binding: RuntimeBinding,
        supports_activation_steering: bool,
    ) -> ModelRegistration {
        ModelRegistration {
            model_id,
            artifact_path: PathBuf::from("fixtures/models/local-test.gguf"),
            sha256: [9; 32],
            runtime_binding,
            declared_capabilities: ModelCapabilities {
                supports_activation_steering,
                ..Default::default()
            },
            base_model_tag: BaseModelTag::new("local-test-base"),
            registered_at_utc: Utc::now(),
            registered_by: OperatorId::new("operator-ilja"),
            provider: ProviderKind::Local,
        }
    }

    fn state_with_candle_model(model_id: ModelId) -> ModelRuntimeState {
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::Candle, true))
            .expect("register candle");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");
        state
    }

    fn attach_fake_runtime(
        state: &ModelRuntimeState,
        model_id: ModelId,
    ) -> Arc<FakeSteeringHookOps> {
        let hooks = Arc::new(FakeSteeringHookOps::new(4));
        let fake = Arc::new(FakeCandleRuntime::new_with_hooks(
            model_id,
            ModelCapabilities {
                supports_activation_steering: true,
                ..Default::default()
            },
            hooks.clone(),
        ));
        state
            .attach_live_runtime(model_id, fake)
            .expect("attach live runtime");
        hooks
    }

    #[test]
    fn refusal_extract_dto_uses_camel_case_serde_and_event_constant_is_wired() {
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
        assert_eq!(
            FR_EVT_LLM_INFER_REFUSAL_EXTRACT,
            "FR-EVT-LLM-INFER-REFUSAL-EXTRACT"
        );
    }

    #[tokio::test]
    async fn refusal_extract_rejects_empty_inputs() {
        let state = ModelRuntimeState::default();
        let model_id = ModelId::new_v7();

        let mut bad = request(model_id);
        bad.harmful_prompts.clear();
        let err = refusal_extract(bad, &state)
            .await
            .expect_err("empty harmful");
        assert!(err.contains("harmful"), "{err}");

        let mut bad = request(model_id);
        bad.harmless_prompts.clear();
        let err = refusal_extract(bad, &state)
            .await
            .expect_err("empty harmless");
        assert!(err.contains("harmless"), "{err}");

        let mut bad = request(model_id);
        bad.layers.clear();
        let err = refusal_extract(bad, &state)
            .await
            .expect_err("empty layers");
        assert!(err.contains("layer"), "{err}");

        let mut bad = request(model_id);
        bad.harmful_prompts = vec!["   ".to_string()];
        let err = refusal_extract(bad, &state)
            .await
            .expect_err("blank harmful");
        assert!(err.contains("blank"), "{err}");
    }

    #[tokio::test]
    async fn refusal_extract_fails_closed_when_adapter_lacks_activation_steering() {
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::LlamaCpp, false))
            .expect("register llama.cpp");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");

        let err = refusal_extract(request(model_id), &state)
            .await
            .expect_err("llama.cpp cannot run activation steering");

        assert!(
            err.contains("activation_steering") && err.contains("llama_cpp"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn refusal_extract_dispatches_to_live_candle_runtime_and_returns_unit_directions() {
        // MT-102 production path: refusal extraction flows through the live
        // ModelRuntime adapter and returns real refusal-direction vectors
        // derived from the adapter's SteeringHookOps capture output. No
        // live_runtime_unavailable.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let req = RefusalExtractRequestIpc {
            model_id: model_id.to_string(),
            harmful_prompts: vec!["harmful one".to_string(), "harmful two".to_string()],
            harmless_prompts: vec!["harmless one".to_string(), "harmless two".to_string()],
            layers: vec![10, 14, 18],
        };

        let result = refusal_extract(req, &state)
            .await
            .expect("live refusal extract succeeds");

        assert_eq!(result.event_type, FR_EVT_LLM_INFER_REFUSAL_EXTRACT);
        assert_eq!(result.directions.len(), 3);
        // Per-layer ordering preserved.
        assert_eq!(result.directions[0].layer, 10);
        assert_eq!(result.directions[1].layer, 14);
        assert_eq!(result.directions[2].layer, 18);
        // Each direction must be unit-length (refusal_vector::extract_refusal_direction
        // normalises before returning).
        for direction in &result.directions {
            let norm: f32 = direction.values.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-5,
                "direction at layer {} not unit length; norm={norm}, values={:?}",
                direction.layer,
                direction.values,
            );
        }
        // Two capture calls were made (harmful + harmless), each through the
        // shared activation_steering plumbing.
        assert_eq!(hooks.capture_calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn refusal_extract_returns_typed_capture_not_available_when_no_live_runtime() {
        // Model is registered + capability-supported + marked loaded but no
        // live ModelRuntime is attached. The error must use the typed
        // `capture_not_available` prefix shared with the steering surface so
        // the RefusalVectorWizard surfaces the same wording.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);

        let err = refusal_extract(request(model_id), &state)
            .await
            .expect_err("no live runtime attached");

        assert!(
            err.contains(STEERING_CAPTURE_NOT_AVAILABLE_PREFIX),
            "expected capture_not_available, got: {err}"
        );
        assert!(
            !err.contains("live model runtime manager"),
            "must not return legacy live_runtime_unavailable wording: {err}"
        );
    }

    #[tokio::test]
    async fn refusal_extract_propagates_adapter_unloaded_as_capture_not_available() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let runtime = fake_runtime_with_unloaded_hooks(model_id);
        state
            .attach_live_runtime(model_id, runtime)
            .expect("attach unloaded runtime");

        let err = refusal_extract(request(model_id), &state)
            .await
            .expect_err("adapter reports unloaded model");

        assert!(err.contains(STEERING_CAPTURE_NOT_AVAILABLE_PREFIX), "{err}");
        assert!(err.contains("not loaded"), "{err}");
    }
}
