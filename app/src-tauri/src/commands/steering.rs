//! MT-096: Activation steering Tauri IPC surface.
//!
//! Each command:
//! 1. Validates the request shape.
//! 2. Preflights `ModelRuntimeState::activation_steering_command_binding` so
//!    that capability gating + loaded-handle gating runs identically to the
//!    pre-MT-082 contract (callers depend on the typed string errors).
//! 3. Dispatches into the live `dyn ModelRuntime` adapter attached to the
//!    model via `ModelRuntimeState::attach_live_runtime`. The adapter is the
//!    real production `CandleRuntime` in the app binary, or the
//!    `FakeCandleRuntime` test double in unit tests. Both compose the same
//!    `SteeringHookHandle` + `SteeringHookOps` contract.
//! 4. If no live runtime is attached for the model, returns a typed
//!    `capture_not_available` reason — NOT `live_runtime_unavailable`. The
//!    UI surfaces the reason verbatim per the MT-098 contract.
//!
//! Register-vector additionally persists the operator-authored vector through
//! the production `SteeringVectorStore` (MT-097) before reporting success.
//! When no SteeringVectorStore is wired (test mode without persistence), the
//! command still dispatches into the adapter's `register_vector` hook so the
//! adapter sees the real vector lifecycle.

use std::sync::Arc;

use handshake_core::model_runtime::{
    techniques::{
        activation_steering::{
            self, FR_EVT_LLM_INFER_STEER_ACTIVE, FR_EVT_LLM_INFER_STEER_APPLY,
            FR_EVT_LLM_INFER_STEER_CAPTURE, FR_EVT_LLM_INFER_STEER_REGISTER,
        },
        steering_vector_store::{PersistSteeringVectorRequest, SteeringVectorStore},
    },
    ContrastiveTechnique, HookPoint, LayerIndex, ModelId, ModelRuntime, OperatorId,
    SteeringHookHandle, SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::model_runtime::{parse_model_id, ModelRuntimeState};

pub const KERNEL_MODEL_RUNTIME_STEERING_CAPTURE_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_capture";
pub const KERNEL_MODEL_RUNTIME_STEERING_REGISTER_VECTOR_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_register_vector";
pub const KERNEL_MODEL_RUNTIME_STEERING_SET_ACTIVE_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_set_active";
pub const KERNEL_MODEL_RUNTIME_STEERING_UNREGISTER_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_unregister";
pub const KERNEL_MODEL_RUNTIME_STEERING_LIST_VECTORS_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_list_vectors";
pub const KERNEL_MODEL_RUNTIME_STEERING_APPROVE_IPC_CHANNEL: &str =
    "kernel_model_runtime_steering_approve";
/// App-layer lifecycle event id for an operator review approval of a steering
/// vector (`Pending -> Approved`). The review gate (MT-097) makes activation
/// depend on this transition having happened.
pub const FR_EVT_LLM_INFER_STEER_APPROVE: &str = "FR-EVT-LLM-INFER-STEER-APPROVE";

/// Operator-facing prefix indicating the model is not actually available for
/// steering capture (e.g. not loaded, hook config mismatch). The MT-098 UI
/// surfaces this verbatim. Distinct from capability gating which keeps the
/// pre-existing "activation_steering is not supported by adapter" message
/// (callers and the UI test fixture depend on that text).
pub const STEERING_CAPTURE_NOT_AVAILABLE_PREFIX: &str = "capture_not_available";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringCaptureRequestIpc {
    pub model_id: String,
    pub prompts: Vec<String>,
    pub layers: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringCaptureResultIpc {
    pub tokens_seen: u32,
    pub activations_by_layer: Vec<LayerActivationsIpc>,
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerActivationsIpc {
    pub layer: u32,
    pub activations: Vec<Vec<f32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringRegisterVectorRequestIpc {
    pub model_id: String,
    pub name: String,
    pub layer: u32,
    pub hook_point: String,
    pub values: Vec<f32>,
    pub intensity: f32,
    pub description: String,
    pub provenance: SteeringProvenanceIpc,
    #[serde(default)]
    pub license_tag: Option<String>,
    #[serde(default)]
    pub model_compat_tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringProvenanceIpc {
    pub technique: String,
    #[serde(default)]
    pub positive_prompts: Vec<String>,
    #[serde(default)]
    pub negative_prompts: Vec<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringVectorIdIpc {
    pub vector_id: String,
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringSetActiveRequestIpc {
    pub model_id: String,
    pub vector_ids: Vec<String>,
    /// MT-100/102: explicit operator acknowledgement that activating a
    /// refusal-disabling vector disables the model's safety refusal. Required
    /// server-side (not just in the UI) before any refusal-ablation vector can
    /// be activated. Ignored for non-refusal vectors. Defaults to `false`.
    #[serde(default)]
    pub disables_refusal_acknowledged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringApproveRequestIpc {
    pub vector_id: String,
    pub approver: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringSetActiveResultIpc {
    pub active_ids: Vec<String>,
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringUnregisterRequestIpc {
    pub model_id: String,
    pub vector_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringMutationResultIpc {
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringListVectorsRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringVectorMetaIpc {
    pub vector_id: String,
    pub name: String,
    pub layer: u32,
    pub hook_point: String,
    pub intensity: f32,
    pub description: String,
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_capture(
    request: SteeringCaptureRequestIpc,
    state: State<'_, ModelRuntimeState>,
    store: State<'_, SteeringVectorStoreState>,
) -> Result<SteeringCaptureResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_CAPTURE_IPC_CHANNEL;
    let _ = store; // capture does not persist; reserved for future enrichment.
    steering_capture(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_register_vector(
    request: SteeringRegisterVectorRequestIpc,
    state: State<'_, ModelRuntimeState>,
    store: State<'_, SteeringVectorStoreState>,
) -> Result<SteeringVectorIdIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_REGISTER_VECTOR_IPC_CHANNEL;
    steering_register_vector(request, state.inner(), store.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_set_active(
    request: SteeringSetActiveRequestIpc,
    state: State<'_, ModelRuntimeState>,
    store: State<'_, SteeringVectorStoreState>,
) -> Result<SteeringSetActiveResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_SET_ACTIVE_IPC_CHANNEL;
    steering_set_active(request, state.inner(), store.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_approve(
    request: SteeringApproveRequestIpc,
    store: State<'_, SteeringVectorStoreState>,
) -> Result<SteeringMutationResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_APPROVE_IPC_CHANNEL;
    steering_approve(request, store.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_unregister(
    request: SteeringUnregisterRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<SteeringMutationResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_UNREGISTER_IPC_CHANNEL;
    steering_unregister(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_steering_list_vectors(
    request: SteeringListVectorsRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<Vec<SteeringVectorMetaIpc>, String> {
    let _ = KERNEL_MODEL_RUNTIME_STEERING_LIST_VECTORS_IPC_CHANNEL;
    steering_list_vectors(request, state.inner()).await
}

/// Tauri-managed handle for the production `SteeringVectorStore`. Optional so
/// that test composition can leave persistence detached and exercise the
/// adapter dispatch independently.
#[derive(Default)]
pub struct SteeringVectorStoreState {
    inner: Option<Arc<SteeringVectorStore>>,
}

impl SteeringVectorStoreState {
    pub fn new(store: Arc<SteeringVectorStore>) -> Self {
        Self { inner: Some(store) }
    }

    pub fn detached() -> Self {
        Self { inner: None }
    }

    pub fn store(&self) -> Option<Arc<SteeringVectorStore>> {
        self.inner.clone()
    }
}

pub async fn steering_capture(
    request: SteeringCaptureRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SteeringCaptureResultIpc, String> {
    validate_prompts_and_layers(&request.prompts, &request.layers)?;
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "capture")?;
    let layers: Vec<LayerIndex> = request
        .layers
        .iter()
        .copied()
        .map(LayerIndex::new)
        .collect();

    let capture_result =
        activation_steering::capture(runtime.as_ref(), model_id, request.prompts.clone(), layers)
            .await
            .map_err(|error| capture_not_available_message(&error.to_string()))?;

    let activations_by_layer: Vec<LayerActivationsIpc> = capture_result
        .activations
        .into_iter()
        .map(|(layer, activations)| LayerActivationsIpc {
            layer: layer.as_u32(),
            activations,
        })
        .collect();

    Ok(SteeringCaptureResultIpc {
        tokens_seen: capture_result.tokens_seen,
        activations_by_layer,
        event_type: FR_EVT_LLM_INFER_STEER_CAPTURE.to_string(),
    })
}

pub async fn steering_register_vector(
    request: SteeringRegisterVectorRequestIpc,
    state: &ModelRuntimeState,
    store: &SteeringVectorStoreState,
) -> Result<SteeringVectorIdIpc, String> {
    validate_register_request(&request)?;
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "register_vector")?;

    let vector = build_steering_vector(&request)?;

    // MT-097 persistence: when a SteeringVectorStore is attached, persist the
    // operator-authored vector as a pre-promotion ArtifactBox before mutating
    // live runtime hook state. Without a store (test mode), dispatch directly
    // so test-only adapters can still exercise live steering hooks.
    if let Some(store) = store.store() {
        let license_tag = request
            .license_tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .unwrap_or("SourceModelLicenseOnly")
            .to_string();
        let model_compat_tag = request
            .model_compat_tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .unwrap_or("unspecified-compat")
            .to_string();
        let created_by = OperatorId::try_new(
            request
                .provenance
                .author
                .as_deref()
                .unwrap_or("operator-inference-lab"),
        )
        .map_err(|error| format!("operator id invalid: {error}"))?;
        let persist_request = PersistSteeringVectorRequest {
            vector: vector.clone(),
            license_tag,
            model_compat_tag,
            created_by,
            session_id: "inference-lab-mt098".to_string(),
            role_id: "operator".to_string(),
        };
        store
            .persist(persist_request)
            .map_err(|error| format!("steering vector persist failed: {error}"))?;
    }

    let vector_id =
        activation_steering::register_steering_vector(runtime.as_ref(), model_id, vector.clone())
            .await
            .map_err(|error| format!("steering register dispatch failed: {error}"))?;
    if vector_id != vector.id {
        let _ = activation_steering::unregister(runtime.as_ref(), model_id, vector_id).await;
        return Err(format!(
            "steering register dispatch returned unexpected vector id {}; expected {}",
            vector_id, vector.id
        ));
    }

    Ok(SteeringVectorIdIpc {
        vector_id: vector_id.to_string(),
        event_type: FR_EVT_LLM_INFER_STEER_REGISTER.to_string(),
    })
}

pub async fn steering_set_active(
    request: SteeringSetActiveRequestIpc,
    state: &ModelRuntimeState,
    store: &SteeringVectorStoreState,
) -> Result<SteeringSetActiveResultIpc, String> {
    let mut parsed_ids = Vec::with_capacity(request.vector_ids.len());
    for vector_id in &request.vector_ids {
        parsed_ids.push(parse_vector_id(vector_id)?);
    }
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "set_active")?;

    // MT-097/100/102: when a SteeringVectorStore is attached (production), the
    // activation path is gated SERVER-SIDE, not just in the React UI:
    //  - MT-097: every vector must be review-Approved (`ensure_activatable`),
    //    so a `Pending`/`Denied` vector cannot be activated.
    //  - MT-100/102: activating a refusal-disabling (ablation) vector additionally
    //    requires the explicit operator acknowledgement, so a non-UI IPC caller
    //    cannot silently disable safety refusal.
    if let Some(store) = store.store() {
        store
            .ensure_activatable(&parsed_ids)
            .map_err(|error| format!("steering set_active blocked by review gate: {error}"))?;
        let mut requires_refusal_ack = false;
        for id in &parsed_ids {
            if store
                .is_refusal_ablation(*id)
                .map_err(|error| format!("steering set_active refusal check failed: {error}"))?
            {
                requires_refusal_ack = true;
                break;
            }
        }
        if requires_refusal_ack && !request.disables_refusal_acknowledged {
            return Err(
                "steering set_active blocked: activating a refusal-disabling vector requires \
                 explicit operator acknowledgement (disablesRefusalAcknowledged=true)"
                    .to_string(),
            );
        }
    }

    activation_steering::set_active_steering_vectors(
        runtime.as_ref(),
        model_id,
        parsed_ids.clone(),
    )
    .await
    .map_err(|error| format!("steering set_active dispatch failed: {error}"))?;

    Ok(SteeringSetActiveResultIpc {
        active_ids: parsed_ids.into_iter().map(|id| id.to_string()).collect(),
        event_type: FR_EVT_LLM_INFER_STEER_ACTIVE.to_string(),
    })
}

/// MT-097 operator review approval (`Pending -> Approved`). Activation
/// (`steering_set_active`) rejects any vector that has not been approved here,
/// which is what makes the persisted `ReviewStatus` load-bearing on the
/// production path. Requires an attached `SteeringVectorStore`.
pub async fn steering_approve(
    request: SteeringApproveRequestIpc,
    store: &SteeringVectorStoreState,
) -> Result<SteeringMutationResultIpc, String> {
    let vector_id = parse_vector_id(&request.vector_id)?;
    let approver = OperatorId::try_new(request.approver.trim())
        .map_err(|error| format!("approver operator id invalid: {error}"))?;
    let store = store
        .store()
        .ok_or_else(|| "steering approve requires an attached SteeringVectorStore".to_string())?;
    store
        .approve(vector_id, &approver)
        .map_err(|error| format!("steering approve failed: {error}"))?;
    Ok(SteeringMutationResultIpc {
        event_type: FR_EVT_LLM_INFER_STEER_APPROVE.to_string(),
    })
}

pub async fn steering_unregister(
    request: SteeringUnregisterRequestIpc,
    state: &ModelRuntimeState,
) -> Result<SteeringMutationResultIpc, String> {
    let vector_id = parse_vector_id(&request.vector_id)?;
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "unregister")?;

    activation_steering::unregister(runtime.as_ref(), model_id, vector_id)
        .await
        .map_err(|error| format!("steering unregister dispatch failed: {error}"))?;

    Ok(SteeringMutationResultIpc {
        event_type: FR_EVT_LLM_INFER_STEER_APPLY.to_string(),
    })
}

pub async fn steering_list_vectors(
    request: SteeringListVectorsRequestIpc,
    state: &ModelRuntimeState,
) -> Result<Vec<SteeringVectorMetaIpc>, String> {
    let model_id = preflight_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state, "list_vectors")?;

    let metas = activation_steering::list_vectors(runtime.as_ref(), model_id)
        .map_err(|error| format!("steering list_vectors dispatch failed: {error}"))?;

    Ok(metas
        .into_iter()
        .map(|meta| SteeringVectorMetaIpc {
            vector_id: meta.id.to_string(),
            name: meta.name,
            layer: meta.layer.as_u32(),
            hook_point: hook_point_to_string(meta.hook_point),
            intensity: meta.intensity,
            description: meta.description,
        })
        .collect())
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

fn hook_point_to_string(hook_point: HookPoint) -> String {
    match hook_point {
        HookPoint::ResidStream => "resid_stream".to_string(),
        HookPoint::MlpOut => "mlp_out".to_string(),
        HookPoint::AttnOut => "attn_out".to_string(),
    }
}

fn build_steering_vector(
    request: &SteeringRegisterVectorRequestIpc,
) -> Result<SteeringVector, String> {
    let values = SteeringVectorValues::try_new(request.values.clone(), request.intensity)
        .map_err(|error| error.to_string())?;
    let provenance = build_provenance(&request.provenance)?;
    SteeringVector::try_new(
        Some(SteeringVectorId::from(Uuid::now_v7())),
        request.name.clone(),
        LayerIndex::new(request.layer),
        HookPoint::ResidStream,
        values,
        request.description.clone(),
        Some(provenance),
    )
    .map_err(|error| error.to_string())
}

fn build_provenance(provenance: &SteeringProvenanceIpc) -> Result<SteeringProvenance, String> {
    match provenance.technique.trim() {
        "manual" => Ok(SteeringProvenance::Manual {
            author: provenance
                .author
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string(),
            notes: provenance.notes.clone().unwrap_or_default(),
        }),
        "repe" => Ok(SteeringProvenance::Contrastive {
            positive_prompts: provenance.positive_prompts.clone(),
            negative_prompts: provenance.negative_prompts.clone(),
            technique: ContrastiveTechnique::RepE,
        }),
        "caa" => Ok(SteeringProvenance::Contrastive {
            positive_prompts: provenance.positive_prompts.clone(),
            negative_prompts: provenance.negative_prompts.clone(),
            technique: ContrastiveTechnique::CAA,
        }),
        "refusal_vector" => Ok(SteeringProvenance::Contrastive {
            positive_prompts: provenance.positive_prompts.clone(),
            negative_prompts: provenance.negative_prompts.clone(),
            technique: ContrastiveTechnique::RefusalVector,
        }),
        other => Err(format!(
            "unsupported steering provenance technique: {other}"
        )),
    }
}

fn validate_prompts_and_layers(prompts: &[String], layers: &[u32]) -> Result<(), String> {
    if prompts.is_empty() {
        return Err("activation steering capture requires at least one prompt".to_string());
    }
    if prompts.iter().any(|prompt| prompt.trim().is_empty()) {
        return Err("activation steering capture prompts must not be blank".to_string());
    }
    if layers.is_empty() {
        return Err("activation steering capture requires at least one layer".to_string());
    }
    for layer in layers {
        let _ = LayerIndex::new(*layer);
    }
    Ok(())
}

fn validate_register_request(request: &SteeringRegisterVectorRequestIpc) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("steering vector name must not be empty".to_string());
    }
    if request.description.trim().is_empty() {
        return Err("steering vector description must not be empty".to_string());
    }
    if request.hook_point.trim() != "resid_stream" {
        return Err(format!(
            "activation steering command supports hookPoint=resid_stream, got {}",
            request.hook_point
        ));
    }
    SteeringVectorValues::try_new(request.values.clone(), request.intensity)
        .map_err(|error| error.to_string())?;
    match request.provenance.technique.trim() {
        "manual" => {
            if request
                .provenance
                .author
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err("manual steering provenance author is required".to_string());
            }
        }
        "repe" | "caa" | "refusal_vector" => {
            if request.provenance.positive_prompts.is_empty() {
                return Err(
                    "contrastive steering provenance positivePrompts must be non-empty".to_string(),
                );
            }
            if request.provenance.negative_prompts.is_empty() {
                return Err(
                    "contrastive steering provenance negativePrompts must be non-empty".to_string(),
                );
            }
        }
        other => {
            return Err(format!(
                "unsupported steering provenance technique: {other}"
            ));
        }
    }
    Ok(())
}

fn parse_vector_id(value: &str) -> Result<SteeringVectorId, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("steering vector id must not be empty".to_string());
    }
    let uuid =
        Uuid::parse_str(trimmed).map_err(|error| format!("invalid steering vector id: {error}"))?;
    if uuid.get_version_num() != 7 {
        return Err(format!("steering vector id must be UUID v7: {trimmed}"));
    }
    Ok(SteeringVectorId::from(uuid))
}

/// Public accessor: lets `SteeringHookHandle` consumers expose the underlying
/// handle id when needed for instrumentation. Currently unused but kept so
/// the production wiring layer can reach into the handle without exposing
/// internals from this module.
#[allow(dead_code)]
fn steering_handle_id(handle: &SteeringHookHandle) -> String {
    handle.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, sync::Arc};

    use chrono::Utc;
    use handshake_core::model_runtime::{
        BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, OperatorId, ProviderKind,
        RuntimeBinding,
    };

    use crate::commands::testing::{
        fake_runtime_with_unloaded_hooks, FakeCandleRuntime, FakeSteeringHookOps,
    };

    use super::*;

    fn capture_request(model_id: ModelId) -> SteeringCaptureRequestIpc {
        SteeringCaptureRequestIpc {
            model_id: model_id.to_string(),
            prompts: vec!["I want to be honest".to_string()],
            layers: vec![3],
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
    fn steering_command_dtos_are_camel_case_and_event_constants_are_wired() {
        let value = serde_json::to_value(SteeringCaptureRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            prompts: vec!["prompt".to_string()],
            layers: vec![3],
        })
        .expect("serialize request");

        assert!(value.get("modelId").is_some());
        assert!(value.get("model_id").is_none());
        assert_eq!(
            FR_EVT_LLM_INFER_STEER_CAPTURE,
            "FR-EVT-LLM-INFER-STEER-CAPTURE"
        );
        assert_eq!(
            FR_EVT_LLM_INFER_STEER_REGISTER,
            "FR-EVT-LLM-INFER-STEER-REGISTER"
        );
        assert_eq!(
            FR_EVT_LLM_INFER_STEER_ACTIVE,
            "FR-EVT-LLM-INFER-STEER-ACTIVE"
        );
        assert_eq!(FR_EVT_LLM_INFER_STEER_APPLY, "FR-EVT-LLM-INFER-STEER-APPLY");
    }

    #[tokio::test]
    async fn steering_commands_fail_closed_when_adapter_lacks_activation_steering() {
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(registration(model_id, RuntimeBinding::LlamaCpp, false))
            .expect("register llama.cpp");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");

        let error = steering_capture(capture_request(model_id), &state)
            .await
            .expect_err("llama.cpp cannot run activation steering");

        assert!(
            error.contains("activation_steering") && error.contains("llama_cpp"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn steering_capture_dispatches_to_live_candle_runtime_and_returns_real_activations() {
        // MT-096 production path: capture flows through the live
        // ModelRuntime adapter and returns real activation data from the
        // adapter's SteeringHookOps. No live_runtime_unavailable.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let request = SteeringCaptureRequestIpc {
            model_id: model_id.to_string(),
            prompts: vec!["positive prompt".to_string(), "another".to_string()],
            layers: vec![3, 7],
        };
        let result = steering_capture(request, &state)
            .await
            .expect("live capture succeeds");

        assert_eq!(result.tokens_seen, 2);
        assert_eq!(result.event_type, FR_EVT_LLM_INFER_STEER_CAPTURE);
        assert_eq!(result.activations_by_layer.len(), 2);
        // The fake's first row at layer 3, prompt 0 should be [3.0, 0.0, ...].
        let layer_3 = result
            .activations_by_layer
            .iter()
            .find(|entry| entry.layer == 3)
            .expect("layer 3 in response");
        assert_eq!(layer_3.activations.len(), 2);
        assert_eq!(layer_3.activations[0][0], 3.0);

        // The adapter saw both prompts in a single capture call.
        let calls = hooks.capture_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompts.len(), 2);
    }

    #[tokio::test]
    async fn steering_capture_returns_typed_capture_not_available_when_no_live_runtime() {
        // The model is registered + capability-supported + marked loaded,
        // but no live ModelRuntime is attached. The error must use the typed
        // capture_not_available prefix, NOT live_runtime_unavailable.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);

        let error = steering_capture(capture_request(model_id), &state)
            .await
            .expect_err("no live runtime attached");

        assert!(
            error.contains(STEERING_CAPTURE_NOT_AVAILABLE_PREFIX),
            "expected capture_not_available, got: {error}"
        );
        assert!(
            !error.contains("live model runtime manager"),
            "must not return legacy live_runtime_unavailable wording: {error}"
        );
        assert!(
            !error.contains("live_runtime_unavailable"),
            "must not return legacy live_runtime_unavailable wording: {error}"
        );
    }

    #[tokio::test]
    async fn steering_capture_propagates_adapter_capture_failure_as_capture_not_available() {
        // The adapter's hook ops report the model is not loaded; that bubbles
        // up as capture_not_available with the real adapter reason so the UI
        // surfaces it verbatim.
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let runtime = fake_runtime_with_unloaded_hooks(model_id);
        state
            .attach_live_runtime(model_id, runtime)
            .expect("attach unloaded runtime");

        let error = steering_capture(capture_request(model_id), &state)
            .await
            .expect_err("adapter reports unloaded model");

        assert!(
            error.contains(STEERING_CAPTURE_NOT_AVAILABLE_PREFIX),
            "{error}"
        );
        assert!(error.contains("not loaded"), "{error}");
    }

    #[tokio::test]
    async fn steering_register_vector_dispatches_through_live_runtime() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);
        let store = SteeringVectorStoreState::detached();

        let request = SteeringRegisterVectorRequestIpc {
            model_id: model_id.to_string(),
            name: "calm".to_string(),
            layer: 12,
            hook_point: "resid_stream".to_string(),
            values: vec![0.1, 0.2, 0.3, 0.4],
            intensity: 1.0,
            description: "wizard-saved".to_string(),
            provenance: SteeringProvenanceIpc {
                technique: "repe".to_string(),
                positive_prompts: vec!["pos".to_string()],
                negative_prompts: vec!["neg".to_string()],
                author: Some("operator-ilja".to_string()),
                notes: None,
            },
            license_tag: Some("SourceModelLicenseOnly".to_string()),
            model_compat_tag: Some("local-test-base".to_string()),
        };

        let result = steering_register_vector(request, &state, &store)
            .await
            .expect("register dispatches");

        assert_eq!(result.event_type, FR_EVT_LLM_INFER_STEER_REGISTER);
        let registered = hooks.vectors.lock().unwrap();
        assert_eq!(registered.len(), 1);
        let stored = registered.values().next().unwrap();
        assert_eq!(stored.name, "calm");
        assert_eq!(stored.layer.as_u32(), 12);
    }

    #[tokio::test]
    async fn steering_register_vector_persists_via_store_when_attached() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(SteeringVectorStore::new(tempdir.path()));
        let store_state = SteeringVectorStoreState::new(store.clone());

        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        attach_fake_runtime(&state, model_id);

        let request = SteeringRegisterVectorRequestIpc {
            model_id: model_id.to_string(),
            name: "calm".to_string(),
            layer: 12,
            hook_point: "resid_stream".to_string(),
            values: vec![0.1, 0.2, 0.3, 0.4],
            intensity: 1.0,
            description: "persisted via store".to_string(),
            provenance: SteeringProvenanceIpc {
                technique: "repe".to_string(),
                positive_prompts: vec!["pos".to_string()],
                negative_prompts: vec!["neg".to_string()],
                author: Some("operator-ilja".to_string()),
                notes: None,
            },
            license_tag: Some("Permissive".to_string()),
            model_compat_tag: Some("local-test-base".to_string()),
        };

        let _ = steering_register_vector(request, &state, &store_state)
            .await
            .expect("register persists");

        let listed = store
            .list_for_model("local-test-base")
            .expect("list returns artifacts");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].license_tag, "Permissive");
    }

    #[tokio::test]
    async fn steering_register_vector_does_not_mutate_runtime_when_persistence_fails() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root_file = tempdir.path().join("not-a-directory");
        fs::write(&root_file, b"blocks artifact directory creation").expect("root file");
        let store = Arc::new(SteeringVectorStore::new(&root_file));
        let store_state = SteeringVectorStoreState::new(store);

        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let request = SteeringRegisterVectorRequestIpc {
            model_id: model_id.to_string(),
            name: "calm".to_string(),
            layer: 12,
            hook_point: "resid_stream".to_string(),
            values: vec![0.1, 0.2, 0.3, 0.4],
            intensity: 1.0,
            description: "must not reach runtime when persist fails".to_string(),
            provenance: SteeringProvenanceIpc {
                technique: "repe".to_string(),
                positive_prompts: vec!["pos".to_string()],
                negative_prompts: vec!["neg".to_string()],
                author: Some("operator-ilja".to_string()),
                notes: None,
            },
            license_tag: Some("Permissive".to_string()),
            model_compat_tag: Some("local-test-base".to_string()),
        };

        let error = steering_register_vector(request, &state, &store_state)
            .await
            .expect_err("persistence failure must reject command");

        assert!(error.contains("steering vector persist failed"), "{error}");
        assert!(
            hooks.vectors.lock().unwrap().is_empty(),
            "runtime must not accept a vector when persistence fails"
        );
    }

    #[tokio::test]
    async fn steering_list_vectors_dispatches_through_live_runtime() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        // Seed a vector through the adapter so list_vectors returns it.
        let vector = SteeringVector::try_new(
            None,
            "seeded",
            LayerIndex::new(2),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3, 0.4], 1.0).unwrap(),
            "seeded",
            Some(SteeringProvenance::Manual {
                author: "operator-ilja".to_string(),
                notes: String::new(),
            }),
        )
        .expect("vector");
        let id = vector.id;
        hooks.vectors.lock().unwrap().insert(id, vector);

        let result = steering_list_vectors(
            SteeringListVectorsRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect("list dispatches");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "seeded");
        assert_eq!(result[0].hook_point, "resid_stream");
        assert_eq!(result[0].layer, 2);
    }

    #[tokio::test]
    async fn steering_set_active_dispatches_through_live_runtime() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let vector = SteeringVector::try_new(
            None,
            "calm",
            LayerIndex::new(2),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3, 0.4], 1.0).unwrap(),
            "vec",
            Some(SteeringProvenance::Manual {
                author: "operator-ilja".to_string(),
                notes: String::new(),
            }),
        )
        .expect("vector");
        let id = vector.id;
        hooks.vectors.lock().unwrap().insert(id, vector);

        let result = steering_set_active(
            SteeringSetActiveRequestIpc {
                model_id: model_id.to_string(),
                vector_ids: vec![id.to_string()],
                disables_refusal_acknowledged: false,
            },
            &state,
            &SteeringVectorStoreState::detached(),
        )
        .await
        .expect("set_active dispatches");

        assert_eq!(result.event_type, FR_EVT_LLM_INFER_STEER_ACTIVE);
        assert_eq!(result.active_ids.len(), 1);
        assert_eq!(result.active_ids[0], id.to_string());
        let active = hooks.active.lock().unwrap();
        assert!(active.contains(&id));
    }

    // ----- MT-097/100/102 server-side review + refusal-disable gate -----

    fn manual_vector(name: &str) -> SteeringVector {
        SteeringVector::try_new(
            None,
            name,
            LayerIndex::new(2),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3, 0.4], 1.0).unwrap(),
            "manual steering vector",
            Some(SteeringProvenance::Manual {
                author: "operator-ilja".to_string(),
                notes: String::new(),
            }),
        )
        .expect("manual vector")
    }

    fn refusal_ablation_vector(name: &str) -> SteeringVector {
        SteeringVector::try_new(
            None,
            name,
            LayerIndex::new(2),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3, 0.4], -1.0).unwrap(),
            "refusal-direction ablation vector",
            Some(SteeringProvenance::Contrastive {
                positive_prompts: vec!["harmful".to_string()],
                negative_prompts: vec!["harmless".to_string()],
                technique: ContrastiveTechnique::RefusalVector,
            }),
        )
        .expect("refusal vector")
    }

    fn persist_pending(vector: &SteeringVector) -> (tempfile::TempDir, Arc<SteeringVectorStore>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(SteeringVectorStore::new(dir.path()));
        store
            .persist(PersistSteeringVectorRequest {
                vector: vector.clone(),
                license_tag: "SourceModelLicenseOnly".to_string(),
                model_compat_tag: "test-compat".to_string(),
                created_by: OperatorId::new("operator-ilja"),
                session_id: "test-session".to_string(),
                role_id: "operator".to_string(),
            })
            .expect("persist vector");
        (dir, store)
    }

    fn set_active_request(model_id: ModelId, id: SteeringVectorId, ack: bool) -> SteeringSetActiveRequestIpc {
        SteeringSetActiveRequestIpc {
            model_id: model_id.to_string(),
            vector_ids: vec![id.to_string()],
            disables_refusal_acknowledged: ack,
        }
    }

    #[tokio::test]
    async fn steering_set_active_review_gate_blocks_pending_and_allows_after_approve() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let vector = manual_vector("calm");
        let id = vector.id;
        hooks.vectors.lock().unwrap().insert(id, vector.clone());

        let (_dir, store_arc) = persist_pending(&vector);
        let store = SteeringVectorStoreState::new(store_arc.clone());

        // Pending -> blocked by the MT-097 review gate; never reaches the adapter.
        let err = steering_set_active(set_active_request(model_id, id, false), &state, &store)
            .await
            .expect_err("a Pending vector must be blocked from activation");
        assert!(err.contains("review gate"), "{err}");
        assert!(!hooks.active.lock().unwrap().contains(&id));

        // Operator approval -> activation now succeeds through the same path.
        store
            .store()
            .unwrap()
            .approve(id, &OperatorId::new("operator-ilja"))
            .expect("approve");
        let ok = steering_set_active(set_active_request(model_id, id, false), &state, &store)
            .await
            .expect("approved vector activates");
        assert_eq!(ok.active_ids, vec![id.to_string()]);
        assert!(hooks.active.lock().unwrap().contains(&id));
    }

    #[tokio::test]
    async fn steering_set_active_refusal_ablation_requires_server_side_acknowledgement() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let vector = refusal_ablation_vector("disable-refusal");
        let id = vector.id;
        hooks.vectors.lock().unwrap().insert(id, vector.clone());

        let (_dir, store_arc) = persist_pending(&vector);
        store_arc
            .approve(id, &OperatorId::new("operator-ilja"))
            .expect("approve");
        let store = SteeringVectorStoreState::new(store_arc);

        // Approved, but no acknowledgement -> blocked server-side (MT-100/102),
        // independent of any UI. A non-UI IPC caller cannot silently disable refusal.
        let err = steering_set_active(set_active_request(model_id, id, false), &state, &store)
            .await
            .expect_err("refusal-ablation activation without acknowledgement must be blocked");
        assert!(err.contains("acknowledgement"), "{err}");
        assert!(!hooks.active.lock().unwrap().contains(&id));

        // Explicit acknowledgement -> activates.
        let ok = steering_set_active(set_active_request(model_id, id, true), &state, &store)
            .await
            .expect("acknowledged refusal-ablation activates");
        assert_eq!(ok.active_ids, vec![id.to_string()]);
        assert!(hooks.active.lock().unwrap().contains(&id));
    }

    #[tokio::test]
    async fn steering_approve_transitions_pending_to_approved() {
        let vector = manual_vector("calm");
        let id = vector.id;
        let (_dir, store_arc) = persist_pending(&vector);
        let store = SteeringVectorStoreState::new(store_arc.clone());

        let result = steering_approve(
            SteeringApproveRequestIpc {
                vector_id: id.to_string(),
                approver: "operator-ilja".to_string(),
            },
            &store,
        )
        .await
        .expect("approve succeeds");
        assert_eq!(result.event_type, FR_EVT_LLM_INFER_STEER_APPROVE);
        store_arc.ensure_activatable(&[id]).expect("approved vector is now activatable");
    }

    #[tokio::test]
    async fn steering_unregister_dispatches_through_live_runtime() {
        let model_id = ModelId::new_v7();
        let state = state_with_candle_model(model_id);
        let hooks = attach_fake_runtime(&state, model_id);

        let vector = SteeringVector::try_new(
            None,
            "calm",
            LayerIndex::new(2),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3, 0.4], 1.0).unwrap(),
            "vec",
            Some(SteeringProvenance::Manual {
                author: "operator-ilja".to_string(),
                notes: String::new(),
            }),
        )
        .expect("vector");
        let id = vector.id;
        hooks.vectors.lock().unwrap().insert(id, vector);

        let result = steering_unregister(
            SteeringUnregisterRequestIpc {
                model_id: model_id.to_string(),
                vector_id: id.to_string(),
            },
            &state,
        )
        .await
        .expect("unregister dispatches");

        assert_eq!(result.event_type, FR_EVT_LLM_INFER_STEER_APPLY);
        assert!(hooks.vectors.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn steering_register_vector_validates_provenance_before_runtime_dispatch() {
        let model_id = ModelId::new_v7();
        let state = ModelRuntimeState::default();
        let store = SteeringVectorStoreState::detached();
        let request = SteeringRegisterVectorRequestIpc {
            model_id: model_id.to_string(),
            name: "bad".to_string(),
            layer: 3,
            hook_point: "resid_stream".to_string(),
            values: vec![0.1, 0.2],
            intensity: 1.0,
            description: "bad vector".to_string(),
            provenance: SteeringProvenanceIpc {
                technique: "repe".to_string(),
                positive_prompts: Vec::new(),
                negative_prompts: vec!["negative".to_string()],
                author: None,
                notes: None,
            },
            license_tag: None,
            model_compat_tag: None,
        };

        let error = steering_register_vector(request, &state, &store)
            .await
            .expect_err("empty positive prompts fail before registry lookup");
        assert!(error.contains("positivePrompts"), "{error}");
    }

    #[test]
    fn steering_vector_ids_must_be_uuid_v7() {
        let error = parse_vector_id(&Uuid::nil().to_string()).expect_err("nil is not v7");
        assert!(error.contains("UUID v7"), "{error}");
    }
}
