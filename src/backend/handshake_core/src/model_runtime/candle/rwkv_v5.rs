#![cfg(feature = "candle-runtime-engine")]

use std::{fs, path::Path};

use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::rwkv_v5::{Config, Model, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks,
    ssm_state::{snapshot_to_tensor, tensor_to_snapshot},
    state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{LoraId, ModelId, ModelRuntimeError, SteeringVectorId};

pub struct CandleRwkvV5Model {
    model: Model,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    device: Device,
}

impl CandleRwkvV5Model {
    pub fn load_safetensors_for_model(
        model_id: ModelId,
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let config_path = config_json_path_for_artifact(artifact_path);
        let (config, eos_token_ids) = read_rwkv_config(&config_path)?;
        let dtype = DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[artifact_path], dtype, device).map_err(
                |error| {
                    ModelRuntimeError::LoadError(format!(
                        "failed to mmap Candle RWKV v5 safetensors {}: {error}",
                        artifact_path.display()
                    ))
                },
            )?
        };
        Self::from_varbuilder_for_model(model_id, config, eos_token_ids, vb, device)
    }

    pub fn from_varbuilder_for_model(
        _model_id: ModelId,
        config: Config,
        eos_token_ids: Vec<u32>,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let model = Model::new(&config, vb).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to construct Candle RWKV v5 model: {error}"
            ))
        })?;
        let state = State::new(1, &config, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle RWKV v5 state: {error}"
            ))
        })?;
        Ok(Self {
            model,
            state,
            config,
            eos_token_ids,
            device: device.clone(),
        })
    }

    pub fn state_position(&self) -> usize {
        self.state.pos
    }

    pub fn state_tensor_count(&self) -> usize {
        self.state.per_layer.len() * 3
    }

    fn reset_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.state = State::new(1, &self.config, &self.device).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "failed to reset Candle RWKV v5 state: {error}"
            ))
        })?;
        Ok(())
    }
}

impl TransformerModel for CandleRwkvV5Model {
    fn forward(
        &mut self,
        input_ids: &Tensor,
        _hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        if !steering_overrides.is_empty() {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "activation_steering".to_string(),
                adapter: "candle_rwkv_v5".to_string(),
            });
        }
        self.validate_lora_overrides(lora_overrides)?;
        let seq_len = match input_ids.dims() {
            [1, seq_len] if *seq_len > 0 => *seq_len,
            dims => {
                return Err(ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v5 expected input shape [1, seq], got {dims:?}"
                )))
            }
        };
        let mut final_logits = None;
        for idx in 0..seq_len {
            let token = input_ids.i((.., idx..idx + 1)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v5 token select failed: {error}"
                ))
            })?;
            let logits = self
                .model
                .forward(&token, &mut self.state)
                .map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "Candle RWKV v5 forward failed: {error}"
                    ))
                })?;
            final_logits = Some(logits.i((0, 0)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v5 final logits select failed: {error}"
                ))
            })?);
        }
        final_logits.ok_or_else(|| {
            ModelRuntimeError::GenerateError("Candle RWKV v5 produced no logits".to_string())
        })
    }

    fn n_layers(&self) -> u32 {
        self.config.num_hidden_layers as u32
    }

    fn hidden_dim(&self) -> u32 {
        self.config.hidden_size as u32
    }

    fn vocab_size(&self) -> u32 {
        self.config.vocab_size as u32
    }

    fn eos_token_ids(&self) -> &[u32] {
        &self.eos_token_ids
    }

    fn device(&self) -> Device {
        self.device.clone()
    }

    fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.reset_state()
    }

    fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        if ids.is_empty() {
            Ok(())
        } else {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "rwkv_v5_lora".to_string(),
                adapter: "candle_rwkv_v5".to_string(),
            })
        }
    }

    // CRIT-1 / MT-088: pack per-layer extract_key_value + feed_forward
    // into the token_shift bucket and linear_attention into the ssm
    // bucket; layout is the inverse of `rwkv_v5_restore_state`.
    fn extract_ssm_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        let (token_shift, ssm) = rwkv_v5_state_to_snapshots(&self.state)?;
        Ok(SSMStateSnapshot::RwkvV5 { token_shift, ssm })
    }

    fn restore_ssm_snapshot(
        &mut self,
        snapshot: &SSMStateSnapshot,
    ) -> Result<(), ModelRuntimeError> {
        let (token_shift, ssm) = match snapshot {
            SSMStateSnapshot::RwkvV5 { token_shift, ssm } => (token_shift, ssm),
            other => {
                return Err(ModelRuntimeError::KvCacheError(format!(
                    "CandleRwkvV5Model::restore_ssm_snapshot variant mismatch: got {}",
                    other.variant()
                )));
            }
        };
        rwkv_v5_restore_state(&mut self.state, &self.device, token_shift, ssm)
    }
}

/// Pack a v5/v6 RWKV `State` (per_layer Vec of {extract_key_value,
/// feed_forward, linear_attention}) into the two-bucket SSMTensorSnapshot
/// shape `SSMStateSnapshot::RwkvV5/V6` expects:
///   token_shift = [layer0.extract_key_value, layer0.feed_forward,
///                  layer1.extract_key_value, layer1.feed_forward, …]
///   ssm         = [layer0.linear_attention, layer1.linear_attention, …]
/// This layout is internal to the round-trip; restore inverts it via the
/// same stride-2 walk.
pub(super) fn rwkv_v5_state_to_snapshots(
    state: &State,
) -> Result<
    (
        Vec<super::state_vector::SSMTensorSnapshot>,
        Vec<super::state_vector::SSMTensorSnapshot>,
    ),
    ModelRuntimeError,
> {
    let mut token_shift = Vec::with_capacity(state.per_layer.len() * 2);
    let mut ssm = Vec::with_capacity(state.per_layer.len());
    for layer in &state.per_layer {
        token_shift.push(tensor_to_snapshot(&layer.extract_key_value)?);
        token_shift.push(tensor_to_snapshot(&layer.feed_forward)?);
        ssm.push(tensor_to_snapshot(&layer.linear_attention)?);
    }
    Ok((token_shift, ssm))
}

pub(super) fn rwkv_v5_restore_state(
    state: &mut State,
    device: &Device,
    token_shift: &[super::state_vector::SSMTensorSnapshot],
    ssm: &[super::state_vector::SSMTensorSnapshot],
) -> Result<(), ModelRuntimeError> {
    let layer_count = state.per_layer.len();
    if token_shift.len() != layer_count * 2 {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "rwkv state restore token_shift length mismatch: expected {} got {}",
            layer_count * 2,
            token_shift.len()
        )));
    }
    if ssm.len() != layer_count {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "rwkv state restore ssm length mismatch: expected {} got {}",
            layer_count,
            ssm.len()
        )));
    }
    for (index, layer) in state.per_layer.iter_mut().enumerate() {
        layer.extract_key_value = snapshot_to_tensor(&token_shift[index * 2], device)?;
        layer.feed_forward = snapshot_to_tensor(&token_shift[index * 2 + 1], device)?;
        layer.linear_attention = snapshot_to_tensor(&ssm[index], device)?;
    }
    Ok(())
}

pub fn artifact_config_declares_rwkv_v5(path: &Path) -> Result<bool, ModelRuntimeError> {
    let value = read_config_value(&config_json_path_for_artifact(path))?;
    Ok(config_value_declares_rwkv_v5(&value))
}

pub fn artifact_config_declares_unversioned_rwkv(path: &Path) -> Result<bool, ModelRuntimeError> {
    let value = read_config_value(&config_json_path_for_artifact(path))?;
    Ok(config_value_declares_unversioned_rwkv(&value))
}

pub fn config_value_declares_rwkv_v5(value: &Value) -> bool {
    let model_type = value
        .get("model_type")
        .and_then(Value::as_str)
        .is_some_and(|model_type| {
            let model_type = model_type.to_ascii_lowercase();
            model_type == "rwkv5" || model_type == "rwkv-v5"
        });
    let architecture = value
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|arch| {
            let arch = arch.to_ascii_lowercase();
            arch.contains("rwkv5") || arch.contains("rwkv_v5") || arch.contains("rwkv-5")
        });
    model_type || architecture
}

pub fn config_value_declares_unversioned_rwkv(value: &Value) -> bool {
    let model_type_declares_generic_rwkv = value
        .get("model_type")
        .and_then(Value::as_str)
        .is_some_and(|model_type| model_type.eq_ignore_ascii_case("rwkv"));
    let architecture_declares_generic_rwkv = value
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|arch| {
            let arch = arch.to_ascii_lowercase();
            arch.contains("rwkv")
                && !arch.contains("rwkv5")
                && !arch.contains("rwkv_v5")
                && !arch.contains("rwkv-5")
                && !arch.contains("rwkv6")
                && !arch.contains("rwkv_v6")
                && !arch.contains("rwkv-6")
        });
    (model_type_declares_generic_rwkv || architecture_declares_generic_rwkv)
        && !config_value_declares_rwkv_v5(value)
}

pub fn read_rwkv_config(path: &Path) -> Result<(Config, Vec<u32>), ModelRuntimeError> {
    let value = read_config_value(path)?;
    let config = serde_json::from_value::<Config>(value.clone()).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to decode Candle RWKV config {}: {error}",
            path.display()
        ))
    })?;
    Ok((config, eos_token_ids(&value)))
}

fn read_config_value(path: &Path) -> Result<Value, ModelRuntimeError> {
    let config_json = fs::read_to_string(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read Candle RWKV config {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str::<Value>(&config_json).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse Candle RWKV config {}: {error}",
            path.display()
        ))
    })
}

fn eos_token_ids(value: &Value) -> Vec<u32> {
    match value.get("eos_token_id") {
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|id| vec![id as u32])
            .unwrap_or_default(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_u64)
            .map(|id| id as u32)
            .collect(),
        _ => Vec::new(),
    }
}

pub fn config_json_path_for_artifact(artifact_path: &Path) -> std::path::PathBuf {
    artifact_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("config.json")
}
