//! MT-105 + MT-103 adjacent Tauri IPC surface for INF-5 CAA extraction.
//!
//! Wraps `handshake_core::model_runtime::techniques::caa::extract_caa_vector`
//! and `register_steering_vector` for the Inference Lab `CaaWizard`. Mirrors
//! the MT-098 / MT-102 dispatch pattern:
//!
//! 1. Validate the request shape.
//! 2. Preflight `ModelRuntimeState::activation_steering_command_binding` so
//!    capability gating + loaded-handle gating runs identically to the
//!    pre-MT-082 contract.
//! 3. Dispatch into the live `dyn ModelRuntime` adapter attached to the model.
//!    The adapter is `CandleRuntime` in the app binary or `FakeCandleRuntime`
//!    in unit tests; both compose the same `SteeringHookHandle` +
//!    `SteeringHookOps` contract.
//! 4. Extract the CAA vector AND register it through the shared steering
//!    hook ops so the resulting vector is available to `set_active` and
//!    `list_vectors` exactly like a manually-authored steering vector.
//! 5. If no live runtime is attached, return the typed
//!    `capture_not_available` prefix the wizard already surfaces.

use std::sync::Arc;

use handshake_core::model_runtime::{
    techniques::{
        activation_steering::register_steering_vector,
        caa::{extract_caa_vector, CaaPromptPair},
    },
    LayerIndex, ModelId, ModelRuntime,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::model_runtime::{parse_model_id, ModelRuntimeState};
use super::steering::STEERING_CAPTURE_NOT_AVAILABLE_PREFIX;

pub const KERNEL_MODEL_RUNTIME_CAA_EXTRACT_IPC_CHANNEL: &str = "kernel_model_runtime_caa_extract";

/// FR event tag emitted on a successful CAA vector extract + register. Mirrors
/// the steering register surface so the UI can correlate CAA extract events
/// on the same event channel.
pub const FR_EVT_LLM_INFER_CAA_EXTRACT: &str = "FR-EVT-LLM-INFER-CAA-EXTRACT";

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
    caa_extract(request, state.inner()).await
}

pub async fn caa_extract(
    request: CaaExtractRequestIpc,
    state: &ModelRuntimeState,
) -> Result<CaaExtractResultIpc, String> {
    validate_caa_request(&request)?;
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "caa_extract")?;
    let layer = LayerIndex::new(request.layer);

    // CAA semantics per Rimsky 2024: the kernel-side helper treats the
    // operator-authored prompts as full prefixes-up-to-and-including the
    // completion token. The kernel CaaPromptPair only carries positive +
    // negative; the IPC `context` field is supplied by operators as a UI
    // affordance and is concatenated by joining context + positive /
    // context + negative below so the kernel sees full prompts.
    let pairs: Vec<CaaPromptPair> = request
        .pairs
        .iter()
        .map(|pair| {
            let positive = compose_prompt(&pair.context, &pair.positive);
            let negative = compose_prompt(&pair.context, &pair.negative);
            CaaPromptPair { positive, negative }
        })
        .collect();

    let vector = extract_caa_vector(
        runtime.as_ref(),
        model_id,
        pairs,
        layer,
        request.name.clone(),
        request.description.clone(),
    )
    .await
    .map_err(|error| capture_not_available_message(&error.to_string()))?;

    let values_clone = vector.values.values().to_vec();

    let vector_id = register_steering_vector(runtime.as_ref(), model_id, vector)
        .await
        .map_err(|error| format!("CAA register dispatch failed: {error}"))?;

    Ok(CaaExtractResultIpc {
        vector_id: vector_id.to_string(),
        values: values_clone,
        layer: request.layer,
        event_type: FR_EVT_LLM_INFER_CAA_EXTRACT.to_string(),
    })
}

/// If the operator supplied a non-empty context, concatenate it as a prefix.
/// Otherwise pass the completion through unchanged. Trimming the joiner keeps
/// existing newline / punctuation choices in either field intact (no
/// sanitisation per GLOBAL-PRODUCTION-005..009).
fn compose_prompt(context: &str, completion: &str) -> String {
    if context.is_empty() {
        completion.to_string()
    } else {
        format!("{context}{completion}")
    }
}

fn validate_caa_request(request: &CaaExtractRequestIpc) -> Result<(), String> {
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
        BaseModelTag, ContrastiveTechnique, ModelCapabilities, ModelId, ModelRegistration,
        OperatorId, ProviderKind, RuntimeBinding, SteeringProvenance,
    };

    use crate::commands::testing::{
        fake_runtime_with_unloaded_hooks, FakeCandleRuntime, FakeSteeringHookOps,
    };

    use super::*;

    fn request(model_id: ModelId) -> CaaExtractRequestIpc {
        CaaExtractRequestIpc {
            model_id: model_id.to_string(),
            name: "syco-caa".to_string(),
            description: "CAA sycophancy".to_string(),
            pairs: vec![
                CaaPromptPairIpc {
                    context: "Q: Are you a robot? A: ".to_string(),
                    positive: "Yes".to_string(),
                    negative: "No".to_string(),
                },
                CaaPromptPairIpc {
                    context: "Q: Like dogs? A: ".to_string(),
                    positive: "Yes I love dogs".to_string(),
                    negative: "No I prefer cats".to_string(),
                },
            ],
            layer: 14,
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
    fn caa_extract_dto_uses_camel_case_serde_and_event_constant_is_wired() {
        let value = serde_json::to_value(request(ModelId::new_v7())).expect("serialize");
        assert!(value.get("modelId").is_some());
        assert!(value.get("pairs").is_some());
        assert!(value.get("model_id").is_none());
        assert_eq!(FR_EVT_LLM_INFER_CAA_EXTRACT, "FR-EVT-LLM-INFER-CAA-EXTRACT");
    }

    #[tokio::test]
    async fn caa_extract_rejects_empty_inputs() {
        let state = ModelRuntimeState::default();
        let model_id = ModelId::new_v7();

        let mut bad = request(model_id);
        bad.name = "".to_string();
        assert!(caa_extract(bad, &state).await.unwrap_err().contains("name"));

        let mut bad = request(model_id);
        bad.description = "".to_string();
        assert!(caa_extract(bad, &state)
            .await
            .unwrap_err()
            .contains("description"));

        let mut bad = request(model_id);
        bad.pairs.clear();
        assert!(caa_extract(bad, &state).await.unwrap_err().contains("pair"));

        let mut bad = request(model_id);
        bad.pairs[0].positive = "  ".to_string();
        assert!(caa_extract(bad, &state)
            .await
            .unwrap_err()
            .contains("positive"));

        let mut bad = request(model_id);
        bad.pairs[0].negative = "".to_string();
        assert!(caa_extract(bad, &state)
            .await
            .unwrap_err()
            .contains("negative"));
    }

    #[tokio::test]
    async fn caa_extract_fails_closed_when_adapter_lacks_activation_steering() {
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::LlamaCpp, false))
            .expect("register llama.cpp");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");

        let err = caa_extract(request(model_id), &state)
            .await
            .expect_err("llama.cpp cannot run CAA");

        assert!(
            err.contains("activation_steering") && err.contains("llama_cpp"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn caa_extract_dispatches_to_live_candle_runtime_and_persists_through_hooks() {
        // MT-105 production path: extract goes through the live ModelRuntime
        // adapter and registers the resulting vector via the shared
        // SteeringHookOps so set_active / list_vectors / unregister see the
        // CAA vector exactly like any other steering vector. No
        // live_runtime_unavailable.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let result = caa_extract(request(model_id), &state)
            .await
            .expect("live CAA extract succeeds");

        assert_eq!(result.event_type, FR_EVT_LLM_INFER_CAA_EXTRACT);
        assert_eq!(result.layer, 14);
        assert!(
            !result.values.is_empty(),
            "expected non-empty CAA direction vector"
        );

        // The vector landed in the shared steering hook ops with
        // ContrastiveTechnique::CAA provenance.
        let registered = hooks.vectors.lock().unwrap();
        assert_eq!(registered.len(), 1);
        let stored = registered.values().next().unwrap();
        assert_eq!(stored.layer.as_u32(), 14);
        match &stored.derivation_provenance {
            SteeringProvenance::Contrastive { technique, .. } => {
                assert_eq!(*technique, ContrastiveTechnique::CAA);
            }
            other => panic!("expected Contrastive provenance, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn caa_extract_returns_typed_capture_not_available_when_no_live_runtime() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);

        let err = caa_extract(request(model_id), &state)
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
    async fn caa_extract_propagates_adapter_unloaded_as_capture_not_available() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let runtime = fake_runtime_with_unloaded_hooks(model_id);
        state
            .attach_live_runtime(model_id, runtime)
            .expect("attach unloaded runtime");

        let err = caa_extract(request(model_id), &state)
            .await
            .expect_err("adapter reports unloaded model");

        assert!(err.contains(STEERING_CAPTURE_NOT_AVAILABLE_PREFIX), "{err}");
        assert!(err.contains("not loaded"), "{err}");
    }

    #[test]
    fn compose_prompt_preserves_operator_text_verbatim() {
        // GLOBAL-PRODUCTION-005..009: no sanitisation; context + completion
        // are concatenated verbatim. An empty context passes the completion
        // through unchanged.
        assert_eq!(compose_prompt("", "completion"), "completion");
        assert_eq!(compose_prompt("ctx ", "x"), "ctx x");
        // Explicit / adult wording survives the joiner without modification.
        assert_eq!(
            compose_prompt("Q: explicit verbatim ", "raw pornographic answer"),
            "Q: explicit verbatim raw pornographic answer"
        );
    }
}
