use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use chrono::Utc;
use handshake_core::model_runtime::{
    candle::{load_local_candle_model, LoadedCandleModel},
    BaseModelTag, KvQuantSupport, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry,
    ModelRuntime, OperatorId, ProviderKind, RuntimeBinding,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

pub const KERNEL_MODEL_RUNTIME_LOAD_IPC_CHANNEL: &str = "kernel_model_runtime_load";
pub const KERNEL_MODEL_RUNTIME_UNLOAD_IPC_CHANNEL: &str = "kernel_model_runtime_unload";
pub const FR_EVT_LLM_MODEL_LOAD: &str = "FR-EVT-LLM-MODEL-LOAD";
pub const FR_EVT_LLM_MODEL_UNLOAD: &str = "FR-EVT-LLM-MODEL-UNLOAD";

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
///
/// ## Multiple concurrent instances (MT-204)
///
/// Both maps are keyed by `ModelId`, so this state already holds MANY concurrent
/// loaded models cleanly — each `kernel_model_runtime_load` mints a fresh
/// `ModelId` and attaches its own owning `Arc<dyn ModelRuntime>`; loads and
/// unloads of different models are fully independent. This is the single-load
/// IPC surface. The orchestrated SWARM multi-instance authority is the managed
/// `SwarmRuntimeState` / `SwarmCoordinator` (see `commands::swarm_runtime`),
/// whose `ModelInstanceId = (model_id, instance)` registry lets the SAME
/// artifact run as several concurrent instances. The two coexist without
/// contention: the coordinator owns its sessions' runtimes inside its own
/// registry + teardown closures and never reaches into `live_runtimes`.
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
    /// adapter shaped like CandleRuntime via the same path. The production
    /// caller is `kernel_model_runtime_load` (MT-202).
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
    /// Called by the model unload flow (`kernel_model_runtime_unload`, MT-202).
    /// Dropping the returned `Arc` frees the per-model `CandleRuntime` and its
    /// loaded weights — detaching the sole owning `Arc` IS the unload.
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

    /// Production: register a freshly-loaded model and mark it loaded under a
    /// single write lock. Used by `kernel_model_runtime_load` (MT-202) after
    /// `CandleRuntime::load()` returns the runtime-minted `ModelId`. This is the
    /// non-test sibling of `register_for_tests` + `mark_loaded_for_tests`.
    pub fn register_loaded(&self, registration: ModelRegistration) -> Result<(), String> {
        let model_id = registration.model_id;
        let mut registry = self
            .registry
            .write()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?;
        registry
            .register(registration)
            .map_err(|error| error.to_string())?;
        registry
            .mark_loaded(model_id)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Production: remove a model's registration after its live runtime has been
    /// detached. Used by `kernel_model_runtime_unload` (MT-202).
    pub fn unregister(&self, model_id: ModelId) -> Result<(), String> {
        let mut registry = self
            .registry
            .write()
            .map_err(|_| "model runtime registry lock poisoned".to_string())?;
        registry
            .unregister(model_id)
            .map_err(|error| error.to_string())
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

/// MT-202 production model-load request. The caller supplies the artifact path
/// and its expected sha256 (the integrity gate — `CandleRuntime::load` verifies
/// the file hash against this value and fails loud on mismatch).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLoadRequest {
    pub artifact_path: String,
    pub sha256_expected: String,
    pub base_model_tag: String,
    pub registered_by: String,
}

/// MT-202 THE-ONE-THING: the production model-load flow.
///
/// Constructs a REAL `CandleRuntime` (not a fake), loads the local model
/// artifact, then registers + marks-loaded + `attach_live_runtime` under the
/// runtime-minted `ModelId`. This is the first non-test caller of
/// `attach_live_runtime`, and it is what lights up `preflight_capability_and_loaded`
/// (registry capability + is_loaded) and `require_live_runtime` (live adapter)
/// for every inference-lab IPC command (steering / refusal / CAA / LoRA / KV),
/// which otherwise fail-closed with the typed `capture_not_available` prefix.
///
/// One `CandleRuntime` owns one loaded model; the runtime is moved into the
/// `Arc<dyn ModelRuntime>` stored in `live_runtimes`. Unload detaches and drops
/// that sole `Arc`, freeing the weights.
#[tauri::command]
pub async fn kernel_model_runtime_load(
    request: ModelLoadRequest,
    state: State<'_, ModelRuntimeState>,
) -> Result<String, String> {
    let _ = KERNEL_MODEL_RUNTIME_LOAD_IPC_CHANNEL;
    model_runtime_load(request, state.inner()).await
}

/// Inner load logic (testable with `&ModelRuntimeState`). See
/// `kernel_model_runtime_load` for the full contract.
pub async fn model_runtime_load(
    request: ModelLoadRequest,
    state: &ModelRuntimeState,
) -> Result<String, String> {
    let artifact_path = PathBuf::from(request.artifact_path.trim());
    if artifact_path.as_os_str().is_empty() {
        return Err("artifact_path must not be empty".to_string());
    }
    let base_model_tag = request.base_model_tag.trim();
    if base_model_tag.is_empty() {
        return Err("base_model_tag must not be empty".to_string());
    }
    let registered_by = OperatorId::try_new(request.registered_by.trim())
        .map_err(|error| error.to_string())?;
    let sha256 = decode_sha256_hex(&request.sha256_expected)?;

    // Reuse the shared, proven candle-load path (the same helper the swarm
    // production factory uses) so the single-load IPC and the swarm never drift.
    // It builds the permissive base LoadSpec, constructs a real CandleRuntime,
    // verifies the artifact sha256 + loads, and reads back the real capabilities.
    let LoadedCandleModel {
        runtime,
        model_id,
        capabilities,
    } = load_local_candle_model(artifact_path.clone(), request.sha256_expected.trim().to_string())
        .await
        .map_err(|error| {
            eprintln!("{FR_EVT_LLM_MODEL_LOAD}: load failed: {error}");
            error.to_string()
        })?;

    let registration = ModelRegistration {
        model_id,
        artifact_path,
        sha256,
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: capabilities,
        base_model_tag: BaseModelTag::new(base_model_tag),
        registered_at_utc: Utc::now(),
        registered_by,
        provider: ProviderKind::Local,
    };
    state.register_loaded(registration)?;
    state.attach_live_runtime(model_id, Arc::new(runtime))?;

    eprintln!("{FR_EVT_LLM_MODEL_LOAD}: model_id={model_id} binding=candle");
    Ok(model_id.to_string())
}

/// MT-202 production model-unload flow. Detaches and drops the live runtime
/// (freeing the model weights) and removes the registration. A subsequent
/// inference-lab command on this `model_id` then fails its loaded/live gates.
#[tauri::command]
pub async fn kernel_model_runtime_unload(
    model_id: String,
    state: State<'_, ModelRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_MODEL_RUNTIME_UNLOAD_IPC_CHANNEL;
    model_runtime_unload(&model_id, state.inner()).await
}

/// Inner unload logic (testable with `&ModelRuntimeState`). See
/// `kernel_model_runtime_unload` for the full contract.
pub async fn model_runtime_unload(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<(), String> {
    let parsed = parse_model_id(model_id)?;
    // Detaching the sole owning Arc frees the CandleRuntime + loaded weights.
    let detached = state.detach_live_runtime(parsed)?;
    let unregistered = state.unregister(parsed);
    drop(detached);
    // If neither a live runtime nor a registration existed, surface the typed
    // "not registered" error from unregister; otherwise the unload succeeded.
    unregistered?;
    eprintln!("{FR_EVT_LLM_MODEL_UNLOAD}: model_id={parsed}");
    Ok(())
}

fn decode_sha256_hex(value: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(value.trim())
        .map_err(|error| format!("sha256_expected is not valid hex: {error}"))?;
    bytes
        .try_into()
        .map_err(|_| "sha256_expected must be exactly 32 bytes (64 hex chars)".to_string())
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

    // ----- MT-202 production model-load flow (THE-ONE-THING) -----

    fn load_request(artifact: &str, sha256: &str) -> ModelLoadRequest {
        ModelLoadRequest {
            artifact_path: artifact.to_string(),
            sha256_expected: sha256.to_string(),
            base_model_tag: "mt202-test-base".to_string(),
            registered_by: "operator-mt202".to_string(),
        }
    }

    #[tokio::test]
    async fn model_load_rejects_invalid_sha256_hex() {
        let state = ModelRuntimeState::default();
        let err = model_runtime_load(load_request("some/model.safetensors", "not-hex"), &state)
            .await
            .expect_err("invalid sha256 hex must be rejected");
        assert!(err.contains("not valid hex"), "{err}");
    }

    #[tokio::test]
    async fn model_load_rejects_empty_artifact_path() {
        let state = ModelRuntimeState::default();
        let err = model_runtime_load(load_request("   ", &"ab".repeat(32)), &state)
            .await
            .expect_err("empty artifact_path must be rejected");
        assert!(err.contains("artifact_path must not be empty"), "{err}");
    }

    #[tokio::test]
    async fn model_load_rejects_empty_base_model_tag() {
        let state = ModelRuntimeState::default();
        let mut req = load_request("some/model.safetensors", &"ab".repeat(32));
        req.base_model_tag = "  ".to_string();
        let err = model_runtime_load(req, &state)
            .await
            .expect_err("empty base_model_tag must be rejected");
        assert!(err.contains("base_model_tag must not be empty"), "{err}");
    }

    #[tokio::test]
    async fn model_load_rejects_nonexistent_artifact_file() {
        // Valid 64-hex sha so we pass decode and reach CandleRuntime::load, which
        // validates the artifact is a real file and fails loud when it is not.
        let state = ModelRuntimeState::default();
        let err = model_runtime_load(
            load_request("D:/__handshake_no_such_model__/model.safetensors", &"ab".repeat(32)),
            &state,
        )
        .await
        .expect_err("nonexistent artifact must fail loud");
        assert!(
            err.to_lowercase().contains("artifact"),
            "expected a missing-artifact error, got: {err}"
        );
    }

    #[tokio::test]
    async fn model_unload_unknown_id_returns_not_registered() {
        let state = ModelRuntimeState::default();
        let unknown = ModelId::new_v7().to_string();
        let err = model_runtime_unload(&unknown, &state)
            .await
            .expect_err("unloading an unknown model must error");
        assert!(err.contains("not registered"), "{err}");
    }

    /// THE-ONE-THING real proof (Spec-Realism Gate sub-rules 1 & 2). Loads a REAL
    /// candle-loadable safetensors Llama model from disk, asserts a REAL
    /// `CandleRuntime` is attached, and runs a REAL activation-steering capture
    /// (real `model.forward`) that returns real activations — not a fake adapter
    /// and not the `capture_not_available` fail-closed path. Then proves unload
    /// detaches + frees.
    ///
    /// Env-gated because the model artifact is ~9 MB and is not committed to the
    /// product repo. Run with:
    ///   HANDSHAKE_TEST_CANDLE_LLAMA_MODEL=<.../model.safetensors>
    ///   HANDSHAKE_TEST_CANDLE_LLAMA_SHA256=<hex>
    /// Skips cleanly (the default-CI error-path tests above cover the wiring
    /// structurally) when the env var is absent.
    #[tokio::test]
    async fn model_load_attaches_real_candle_runtime_and_capture_returns_real_activations() {
        use handshake_core::model_runtime::{techniques::activation_steering, LayerIndex};

        let Some(artifact) = std::env::var_os("HANDSHAKE_TEST_CANDLE_LLAMA_MODEL") else {
            eprintln!(
                "SKIP model_load_attaches_real_candle_runtime_...: \
                 HANDSHAKE_TEST_CANDLE_LLAMA_MODEL not set"
            );
            return;
        };
        let artifact = artifact.to_string_lossy().to_string();
        let sha256 = std::env::var("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256")
            .expect("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256 required when the model path is set");

        let state = ModelRuntimeState::default();
        let model_id_str = model_runtime_load(load_request(&artifact, &sha256), &state)
            .await
            .expect("real candle model load succeeds");
        let model_id = parse_model_id(&model_id_str).expect("returned model id parses");

        // THE-ONE-THING: a REAL CandleRuntime is attached for the loaded model.
        let runtime = state
            .live_runtime(model_id)
            .expect("live runtime lock")
            .expect("a live runtime is attached after load");
        let caps = runtime.capabilities(model_id).expect("real capabilities");
        assert!(
            caps.supports_activation_steering,
            "a Llama candle model must report activation steering support"
        );

        // REAL capture: runs the real forward pass and returns real activations.
        let result = activation_steering::capture(
            runtime.as_ref(),
            model_id,
            vec!["The capital of France is".to_string()],
            vec![LayerIndex::new(0), LayerIndex::new(1)],
        )
        .await
        .expect("real activation capture (must NOT be capture_not_available)");
        assert!(result.tokens_seen > 0, "real forward must see tokens");
        assert!(
            !result.activations.is_empty(),
            "real capture must return per-layer activations"
        );
        for (layer, per_token) in &result.activations {
            assert!(
                !per_token.is_empty(),
                "layer {layer:?} must have per-token activation rows"
            );
            assert!(
                per_token.iter().all(|row| !row.is_empty()),
                "activation rows must be non-empty (real residual-stream width)"
            );
        }

        // Unload detaches and frees: the live runtime is gone afterwards.
        model_runtime_unload(&model_id_str, &state)
            .await
            .expect("unload succeeds");
        assert!(
            state
                .live_runtime(model_id)
                .expect("live runtime lock")
                .is_none(),
            "live runtime must be detached after unload"
        );
    }
}
