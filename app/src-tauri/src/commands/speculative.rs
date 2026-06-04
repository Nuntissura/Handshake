//! MT-110 unblock work — Tauri IPC for MT-109 self-speculative-decoding
//! technique surface.
//!
//! MT-109 deferred this layer to a follow-up; MT-110 (the React UI knob)
//! depends on the IPC channels, so the dependency is built here as
//! out-of-scope unblock work per the operator "work on dependencies
//! first" rule. Documented in MT-110.implementation_record.
//!
//! IPC surface:
//!   kernel_model_runtime_spec_set_mode    — operator commits a chosen
//!                                            SpeculativeMode (or None
//!                                            to disable) for a model
//!   kernel_model_runtime_spec_get_mode    — read the current saved mode
//!   kernel_model_runtime_spec_validate    — validate a candidate mode
//!                                            against model capabilities
//!                                            (frontend can prevent UI
//!                                            from committing rejected
//!                                            modes)
//!
//! Per-model override storage lives in `SpeculativeModeOverrides`, a
//! Mutex<HashMap<ModelId, Option<SpeculativeMode>>> wired into the
//! Tauri app via `manage`. The frontend reads/writes via the IPC
//! commands above; the generate path bridge (apply the override to
//! GenerateRequest) is a separate MT.

use std::collections::HashMap;
use std::sync::Mutex;

use handshake_core::model_runtime::techniques::speculative_decoding::{
    self, validate_mode, SpeculativeModeValidation,
};
use handshake_core::model_runtime::{ModelId, SpeculativeMode};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::model_runtime::ModelRuntimeState;

pub const KERNEL_MODEL_RUNTIME_SPEC_SET_MODE_IPC_CHANNEL: &str =
    "kernel_model_runtime_spec_set_mode";
pub const KERNEL_MODEL_RUNTIME_SPEC_GET_MODE_IPC_CHANNEL: &str =
    "kernel_model_runtime_spec_get_mode";
pub const KERNEL_MODEL_RUNTIME_SPEC_VALIDATE_IPC_CHANNEL: &str =
    "kernel_model_runtime_spec_validate";

#[derive(Default)]
pub struct SpeculativeModeOverrides {
    inner: Mutex<HashMap<ModelId, Option<SpeculativeMode>>>,
}

impl SpeculativeModeOverrides {
    pub fn get(&self, model_id: ModelId) -> Option<SpeculativeMode> {
        self.inner
            .lock()
            .expect("speculative override lock")
            .get(&model_id)
            .and_then(|stored| stored.clone())
    }

    pub fn set(&self, model_id: ModelId, mode: Option<SpeculativeMode>) -> Option<SpeculativeMode> {
        self.inner
            .lock()
            .expect("speculative override lock")
            .insert(model_id, mode.clone())
            .and_then(|prev| prev)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSetModeRequestIpc {
    pub model_id: String,
    /// `None` disables speculative decoding for the model (conservative
    /// default). `Some(_)` validates against the model's capabilities
    /// before committing.
    pub mode: Option<SpeculativeMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSetModeResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub previous_mode: Option<SpeculativeMode>,
    pub current_mode: Option<SpeculativeMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecGetModeRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecGetModeResultIpc {
    pub model_id: String,
    pub current_mode: Option<SpeculativeMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecValidateRequestIpc {
    pub model_id: String,
    pub mode: Option<SpeculativeMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecValidateResultIpc {
    pub model_id: String,
    pub validation: SpeculativeModeValidation,
}

#[tauri::command]
pub async fn kernel_model_runtime_spec_set_mode(
    request: SpecSetModeRequestIpc,
    runtime: State<'_, ModelRuntimeState>,
    overrides: State<'_, SpeculativeModeOverrides>,
) -> Result<SpecSetModeResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SPEC_SET_MODE_IPC_CHANNEL;
    spec_set_mode(request, runtime.inner(), overrides.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_spec_get_mode(
    request: SpecGetModeRequestIpc,
    overrides: State<'_, SpeculativeModeOverrides>,
) -> Result<SpecGetModeResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SPEC_GET_MODE_IPC_CHANNEL;
    spec_get_mode(request, overrides.inner())
}

#[tauri::command]
pub async fn kernel_model_runtime_spec_validate(
    request: SpecValidateRequestIpc,
    runtime: State<'_, ModelRuntimeState>,
) -> Result<SpecValidateResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_SPEC_VALIDATE_IPC_CHANNEL;
    spec_validate(request, runtime.inner())
}

pub fn spec_set_mode(
    request: SpecSetModeRequestIpc,
    runtime: &ModelRuntimeState,
    overrides: &SpeculativeModeOverrides,
) -> Result<SpecSetModeResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    // Validate before mutating the override store so a rejected mode
    // can never silently land. The validation surface is the MT-109
    // technique-surface primitive — single source of truth for what
    // "acceptable" means per (adapter, capabilities) pair.
    let live = runtime
        .live_runtime(model_id)?
        .ok_or_else(|| format!("speculative: model_id {model_id} has no live runtime attached"))?;
    let validation = validate_mode(live.as_ref(), model_id, request.mode.as_ref())
        .map_err(|error| format!("speculative validate_mode rejected: {error}"))?;
    let _ = validation; // technique surface accepted; carry on
    let previous = overrides.set(model_id, request.mode.clone());
    let receipt = speculative_decoding::build_mode_change_receipt(
        model_id,
        previous.clone(),
        request.mode.clone(),
    );
    Ok(SpecSetModeResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        previous_mode: receipt.previous_mode,
        current_mode: receipt.current_mode,
    })
}

pub fn spec_get_mode(
    request: SpecGetModeRequestIpc,
    overrides: &SpeculativeModeOverrides,
) -> Result<SpecGetModeResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    Ok(SpecGetModeResultIpc {
        model_id: model_id.to_string(),
        current_mode: overrides.get(model_id),
    })
}

pub fn spec_validate(
    request: SpecValidateRequestIpc,
    runtime: &ModelRuntimeState,
) -> Result<SpecValidateResultIpc, String> {
    let model_id = parse_model_id(&request.model_id)?;
    let live = runtime
        .live_runtime(model_id)?
        .ok_or_else(|| format!("speculative: model_id {model_id} has no live runtime attached"))?;
    let validation = validate_mode(live.as_ref(), model_id, request.mode.as_ref())
        .map_err(|error| format!("speculative validate_mode error: {error}"))?;
    Ok(SpecValidateResultIpc {
        model_id: model_id.to_string(),
        validation,
    })
}

fn parse_model_id(value: &str) -> Result<ModelId, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("model_id must not be empty".to_string());
    }
    let uuid = Uuid::parse_str(trimmed).map_err(|error| format!("invalid model_id: {error}"))?;
    if uuid.get_version_num() != 7 {
        return Err(format!("model_id must be UUID v7: {trimmed}"));
    }
    Ok(ModelId::from(uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speculative_overrides_round_trip_none_and_some() {
        let store = SpeculativeModeOverrides::default();
        let model_id = ModelId::new_v7();
        assert!(store.get(model_id).is_none());
        let prev = store.set(
            model_id,
            Some(SpeculativeMode::Ngram {
                lookback: 32,
                max_draft: 8,
            }),
        );
        assert!(prev.is_none(), "initial set returns no previous value");
        let current = store.get(model_id);
        assert!(matches!(current, Some(SpeculativeMode::Ngram { .. })));
        let prev = store.set(model_id, None);
        assert!(matches!(prev, Some(SpeculativeMode::Ngram { .. })));
        assert!(store.get(model_id).is_none());
    }

    #[test]
    fn spec_set_mode_request_camel_case_serialization() {
        let value = serde_json::to_value(SpecSetModeRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            mode: Some(SpeculativeMode::Ngram {
                lookback: 16,
                max_draft: 4,
            }),
        })
        .expect("serialize spec set_mode request");
        assert!(value.get("modelId").is_some());
        assert!(value.get("model_id").is_none());
        assert_eq!(value["mode"]["mode"], "ngram");
    }
}
