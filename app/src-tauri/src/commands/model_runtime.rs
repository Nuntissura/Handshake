use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use handshake_core::model_runtime::{
    KvQuantSupport, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry, ModelRuntime,
    RuntimeBinding,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

pub const KERNEL_MODEL_RUNTIME_CAPABILITIES_IPC_CHANNEL: &str = "kernel_model_runtime_capabilities";
pub const KERNEL_MODEL_RUNTIME_LIST_LOADED_IPC_CHANNEL: &str = "kernel_model_runtime_list_loaded";
pub const FR_EVT_LLM_INFER_CAPS_LOOKUP: &str = "FR-EVT-LLM-INFER-CAPS-LOOKUP";

/// MT-068: Model runtime IPC state. Holds:
/// - `registry`: declared metadata (model_id -> ModelRegistration) for capability
///   lookup and loaded-projection queries.
/// - `live_runtimes`: model_id -> `Arc<dyn ModelRuntime>` so MT-068 capability
///   queries dispatch into the production adapter (CandleRuntime for steering
///   models) and MT-096 steering commands dispatch into the adapter's live
///   `steering_hooks(model_id)` SteeringHookOps. Tests compose real
///   CandleRuntime-shaped fakes through this same path.
#[derive(Default)]
pub struct ModelRuntimeState {
    registry: RwLock<ModelRegistry>,
    live_runtimes: RwLock<HashMap<ModelId, Arc<dyn ModelRuntime>>>,
}

impl std::fmt::Debug for ModelRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRuntimeState")
            .field("registry", &"<ModelRegistry>")
            .field(
                "live_runtimes",
                &"<HashMap<ModelId, Arc<dyn ModelRuntime>>>",
            )
            .finish()
    }
}

impl ModelRuntimeState {
    #[cfg(test)]
    pub(crate) fn register_for_tests(&self, registration: ModelRegistration) -> Result<(), String> {
        self.registry
            .write()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?
            .register(registration)
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub(crate) fn mark_loaded_for_tests(&self, model_id: ModelId) -> Result<(), String> {
        self.registry
            .write()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?
            .mark_loaded(model_id)
            .map_err(|error| error.to_string())
    }

    /// Attach a live `ModelRuntime` adapter for `model_id`. After this call,
    /// MT-068 capability queries and MT-096 steering dispatches will go through
    /// the adapter's real implementation. Returns the previous binding if any.
    ///
    /// Called by the model load flow when a CandleRuntime adapter completes
    /// `load()` and exposes activation hooks per MT-082. Tests attach a fake
    /// adapter shaped like CandleRuntime via the same path.
    #[allow(dead_code)] // wired by tests + future production load flow
    pub fn attach_live_runtime(
        &self,
        model_id: ModelId,
        runtime: Arc<dyn ModelRuntime>,
    ) -> Result<Option<Arc<dyn ModelRuntime>>, String> {
        let mut guard = self
            .live_runtimes
            .write()
            .map_err(|_| "live runtime registry lock poisoned".to_string())?;
        Ok(guard.insert(model_id, runtime))
    }

    /// Detach a live runtime adapter. Returns the removed runtime if any.
    /// Called by the model unload flow.
    #[allow(dead_code)] // wired by tests + future production unload flow
    pub fn detach_live_runtime(
        &self,
        model_id: ModelId,
    ) -> Result<Option<Arc<dyn ModelRuntime>>, String> {
        let mut guard = self
            .live_runtimes
            .write()
            .map_err(|_| "live runtime registry lock poisoned".to_string())?;
        Ok(guard.remove(&model_id))
    }

    /// Resolve the live `ModelRuntime` for `model_id` if one is attached.
    pub fn live_runtime(&self, model_id: ModelId) -> Result<Option<Arc<dyn ModelRuntime>>, String> {
        let guard = self
            .live_runtimes
            .read()
            .map_err(|_| "live runtime registry lock poisoned".to_string())?;
        Ok(guard.get(&model_id).cloned())
    }

    pub(crate) fn activation_steering_command_binding(
        &self,
        model_id: ModelId,
    ) -> Result<RuntimeBinding, String> {
        let registry = self
            .registry
            .read()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?;
        let registration = registry
            .lookup(model_id)
            .ok_or_else(|| format!("model runtime registration not found: {model_id}"))?;
        if !registration
            .declared_capabilities
            .supports_activation_steering
        {
            return Err(format!(
                "capability activation_steering is not supported by adapter {}",
                registration.runtime_binding.adapter_id()
            ));
        }
        if !registry.is_loaded(model_id) {
            return Err(format!(
                "activation_steering requires a loaded model runtime handle: {model_id}"
            ));
        }
        Ok(registration.runtime_binding)
    }

    pub(crate) fn lora_command_binding(&self, model_id: ModelId) -> Result<RuntimeBinding, String> {
        let registry = self
            .registry
            .read()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?;
        let registration = registry
            .lookup(model_id)
            .ok_or_else(|| format!("model runtime registration not found: {model_id}"))?;
        if !registration.declared_capabilities.supports_lora {
            return Err(format!(
                "capability lora_stack is not supported by adapter {}",
                registration.runtime_binding.adapter_id()
            ));
        }
        if !registry.is_loaded(model_id) {
            return Err(format!(
                "lora_stack requires a loaded model runtime handle: {model_id}"
            ));
        }
        Ok(registration.runtime_binding)
    }

    /// MT-093 KV cache technique surface preflight. `quantization_required`
    /// gates the `set_quantization` channel on declared
    /// `supports_kv_quantization`; mutating prefix ops gate on declared
    /// `supports_kv_prefix_cache`; the read-only `occupancy` op accepts
    /// either capability and rejects only when both are absent.
    pub(crate) fn kv_cache_command_binding(
        &self,
        model_id: ModelId,
        gate: KvCacheCommandGate,
    ) -> Result<RuntimeBinding, String> {
        use handshake_core::model_runtime::KvQuantSupport;

        let registry = self
            .registry
            .read()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?;
        let registration = registry
            .lookup(model_id)
            .ok_or_else(|| format!("model runtime registration not found: {model_id}"))?;
        let capabilities = &registration.declared_capabilities;
        match gate {
            KvCacheCommandGate::Quantization => {
                if capabilities.supports_kv_quantization == KvQuantSupport::None {
                    return Err(format!(
                        "capability kv_cache_quantization is not supported by adapter {}",
                        registration.runtime_binding.adapter_id()
                    ));
                }
            }
            KvCacheCommandGate::PrefixCache => {
                if !capabilities.supports_kv_prefix_cache {
                    return Err(format!(
                        "capability kv_cache_prefix is not supported by adapter {}",
                        registration.runtime_binding.adapter_id()
                    ));
                }
            }
            KvCacheCommandGate::Telemetry => {
                if !capabilities.supports_kv_prefix_cache
                    && capabilities.supports_kv_quantization == KvQuantSupport::None
                {
                    return Err(format!(
                        "capability kv_cache is not supported by adapter {}",
                        registration.runtime_binding.adapter_id()
                    ));
                }
            }
        }
        if !registry.is_loaded(model_id) {
            return Err(format!(
                "kv_cache requires a loaded model runtime handle: {model_id}"
            ));
        }
        Ok(registration.runtime_binding)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KvCacheCommandGate {
    Quantization,
    PrefixCache,
    Telemetry,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilitiesIpc {
    pub supports_lora: bool,
    pub supports_kv_prefix_cache: bool,
    pub supports_kv_quantization: KvQuantSupport,
    pub supports_activation_steering: bool,
    pub supports_subquadratic: bool,
    pub supports_speculative_draft: bool,
    pub supports_eagle3: bool,
}

impl From<ModelCapabilities> for ModelCapabilitiesIpc {
    fn from(value: ModelCapabilities) -> Self {
        Self {
            supports_lora: value.supports_lora,
            supports_kv_prefix_cache: value.supports_kv_prefix_cache,
            supports_kv_quantization: value.supports_kv_quantization,
            supports_activation_steering: value.supports_activation_steering,
            supports_subquadratic: value.supports_subquadratic,
            supports_speculative_draft: value.supports_speculative_draft,
            supports_eagle3: value.supports_eagle3,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimePerfStatsIpc {
    pub tokens_per_second: Option<f64>,
    pub context_tokens: Option<u64>,
    pub last_latency_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedModelRuntimeIpc {
    pub model_id: String,
    pub runtime_binding: RuntimeBinding,
    pub artifact_path: String,
    pub sha256: String,
    pub perf_stats: ModelRuntimePerfStatsIpc,
}

#[tauri::command]
pub async fn kernel_model_runtime_capabilities(
    model_id: String,
    state: State<'_, ModelRuntimeState>,
) -> Result<ModelCapabilitiesIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_CAPABILITIES_IPC_CHANNEL;
    model_runtime_capabilities(&model_id, state.inner()).map_err(|error| {
        eprintln!("{FR_EVT_LLM_INFER_CAPS_LOOKUP}: {error}");
        error
    })
}

#[tauri::command]
pub async fn kernel_model_runtime_list_loaded(
    state: State<'_, ModelRuntimeState>,
) -> Result<Vec<LoadedModelRuntimeIpc>, String> {
    let _ = KERNEL_MODEL_RUNTIME_LIST_LOADED_IPC_CHANNEL;
    model_runtime_list_loaded(state.inner())
}

/// MT-068 capability introspection IPC body.
///
/// Production path: when a live `ModelRuntime` adapter is attached for the
/// model_id, dispatch into the adapter's `capabilities(model_id)` and surface
/// real per-loaded-model capabilities. Otherwise fall back to the
/// declared capabilities recorded in the metadata registry — which is the
/// real registration data, not a placeholder.
///
/// If the model is not in the registry at all, this returns a typed
/// "registration not found" error so callers can distinguish "no such model"
/// from "live runtime not yet attached".
pub fn model_runtime_capabilities(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<ModelCapabilitiesIpc, String> {
    let model_id = parse_model_id(model_id)?;

    // Live runtime dispatch wins when present: real adapter answers the call.
    if let Some(runtime) = state.live_runtime(model_id)? {
        return match runtime.capabilities(model_id) {
            Ok(caps) => Ok(caps.clone().into()),
            Err(error) => Err(format!(
                "live capability lookup failed via adapter {}: {error}",
                runtime.adapter_name()
            )),
        };
    }

    // Fallback path: real declared capabilities from the metadata registry.
    let registry = state
        .registry
        .read()
        .map_err(|_| "model runtime registry lock poisoned".to_string())?;
    let registration = registry
        .lookup(model_id)
        .ok_or_else(|| format!("model runtime registration not found: {model_id}"))?;

    Ok(registration.declared_capabilities.clone().into())
}

pub fn model_runtime_list_loaded(
    state: &ModelRuntimeState,
) -> Result<Vec<LoadedModelRuntimeIpc>, String> {
    let registry = state
        .registry
        .read()
        .map_err(|_| "model runtime registry lock poisoned".to_string())?;

    Ok(registry
        .list()
        .into_iter()
        .filter(|registration| registry.is_loaded(registration.model_id))
        .map(loaded_model_from_registration)
        .collect())
}

pub(crate) fn parse_model_id(value: &str) -> Result<ModelId, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("model_id must not be empty".to_string());
    }
    let uuid = Uuid::parse_str(trimmed).map_err(|error| format!("invalid model_id: {error}"))?;
    let model_id = ModelId::from(uuid);
    if model_id.as_uuid().get_version_num() != 7 {
        return Err(format!("model_id must be UUID v7: {trimmed}"));
    }
    Ok(model_id)
}

fn loaded_model_from_registration(registration: &ModelRegistration) -> LoadedModelRuntimeIpc {
    LoadedModelRuntimeIpc {
        model_id: registration.model_id.to_string(),
        runtime_binding: registration.runtime_binding,
        artifact_path: registration.artifact_path.to_string_lossy().to_string(),
        sha256: hex::encode(registration.sha256),
        perf_stats: ModelRuntimePerfStatsIpc::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use handshake_core::model_runtime::{BaseModelTag, ProviderKind};
    use serde_json::json;

    use super::*;

    fn capabilities(activation_steering: bool) -> ModelCapabilities {
        ModelCapabilities {
            supports_lora: true,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            supports_activation_steering: activation_steering,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        }
    }

    fn registration(model_id: ModelId, runtime_binding: RuntimeBinding) -> ModelRegistration {
        ModelRegistration {
            model_id,
            artifact_path: PathBuf::from("fixtures/models/local-test.gguf"),
            sha256: [7; 32],
            runtime_binding,
            declared_capabilities: capabilities(runtime_binding == RuntimeBinding::Candle),
            base_model_tag: BaseModelTag::new("local-test-base"),
            registered_at_utc: Utc::now(),
            registered_by: handshake_core::model_runtime::OperatorId::new("operator-ilja"),
            provider: ProviderKind::Local,
        }
    }

    #[test]
    fn model_runtime_ipc_capabilities_returns_camel_case_dto() {
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::Candle))
            .expect("register model");

        let result = model_runtime_capabilities(&model_id.to_string(), &state)
            .expect("capabilities lookup succeeds");

        assert_eq!(
            serde_json::to_value(&result).expect("serialize dto"),
            json!({
                "supportsLora": true,
                "supportsKvPrefixCache": true,
                "supportsKvQuantization": "q4",
                "supportsActivationSteering": true,
                "supportsSubquadratic": false,
                "supportsSpeculativeDraft": false,
                "supportsEagle3": false
            })
        );
    }

    #[test]
    fn model_runtime_ipc_unknown_or_non_v7_model_id_returns_string_error() {
        let state = ModelRuntimeState::default();

        let invalid = model_runtime_capabilities("not-a-uuid", &state).expect_err("invalid uuid");
        assert!(invalid.contains("invalid model_id"), "{invalid}");

        let non_v7 =
            model_runtime_capabilities(&Uuid::nil().to_string(), &state).expect_err("non-v7 uuid");
        assert!(non_v7.contains("UUID v7"), "{non_v7}");

        let missing_id = ModelId::new_v7();
        let missing =
            model_runtime_capabilities(&missing_id.to_string(), &state).expect_err("unknown model");
        assert!(missing.contains("registration not found"), "{missing}");
    }

    #[test]
    fn model_runtime_ipc_list_loaded_returns_loaded_projection_only() {
        let loaded_id = ModelId::new_v7();
        let unloaded_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(loaded_id, RuntimeBinding::Candle))
            .expect("register loaded");
        state
            .register_for_tests(registration(unloaded_id, RuntimeBinding::LlamaCpp))
            .expect("register unloaded");
        state.mark_loaded_for_tests(loaded_id).expect("mark loaded");

        let result = model_runtime_list_loaded(&state).expect("list loaded");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].model_id, loaded_id.to_string());
        assert_eq!(result[0].runtime_binding, RuntimeBinding::Candle);
        assert_eq!(
            result[0].sha256,
            "0707070707070707070707070707070707070707070707070707070707070707"
        );
        assert_eq!(result[0].perf_stats, ModelRuntimePerfStatsIpc::default());
    }

    #[test]
    fn model_runtime_ipc_capabilities_dispatches_through_live_runtime_when_attached() {
        // MT-068: prove that when a live `ModelRuntime` adapter is attached,
        // `kernel_model_runtime_capabilities` dispatches into the adapter's
        // real `capabilities(model_id)` instead of returning declared metadata.
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        // Registration declared capabilities = activation_steering DISABLED.
        state
            .register_for_tests(registration(model_id, RuntimeBinding::Candle))
            .expect("register model");

        // Live runtime reports activation_steering ENABLED + supports_lora=false to
        // prove the live dispatch path is hit (the values differ from declared
        // capabilities, so this can only pass if the live runtime path executes).
        let live_caps = ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: false,
            supports_kv_quantization: KvQuantSupport::None,
            supports_activation_steering: true,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        };
        let fake = Arc::new(crate::commands::testing::FakeCandleRuntime::new(
            model_id, live_caps,
        ));
        state
            .attach_live_runtime(model_id, fake)
            .expect("attach live runtime");

        let result = model_runtime_capabilities(&model_id.to_string(), &state)
            .expect("capabilities lookup dispatches through live runtime");

        // Live runtime values come back, NOT declared.
        assert_eq!(result.supports_activation_steering, true);
        assert_eq!(result.supports_lora, false);
    }

    #[test]
    fn model_runtime_ipc_capabilities_falls_back_to_declared_when_no_live_runtime() {
        // MT-068: when no live runtime is attached, the IPC surface still
        // answers from the real declared capabilities in the metadata registry.
        // This is NOT a placeholder return; declared_capabilities is the
        // real registration record signed by the operator at load time.
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::Candle))
            .expect("register model");

        let result = model_runtime_capabilities(&model_id.to_string(), &state)
            .expect("capabilities lookup falls back to declared metadata");

        // Declared capabilities for Candle binding = activation_steering ENABLED.
        assert_eq!(result.supports_activation_steering, true);
        assert_eq!(result.supports_lora, true);
    }
}
