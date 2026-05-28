#![cfg(feature = "candle-runtime-engine")]

use std::{fs, path::Path};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::rwkv_v7::{Config, Model, ModelVersion, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks,
    rwkv_v5,
    ssm_state::{snapshot_to_tensor, tensor_to_snapshot},
    state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{LoraId, ModelId, ModelRuntimeError, SteeringVectorId};

pub struct CandleRwkvV7Model {
    model: Model,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    device: Device,
}

impl CandleRwkvV7Model {
    pub fn load_safetensors_for_model(
        model_id: ModelId,
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let config_path = rwkv_v5::config_json_path_for_artifact(artifact_path);
        let (config, eos_token_ids) = read_rwkv_v7_config(&config_path)?;
        let dtype = DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[artifact_path], dtype, device).map_err(
                |error| {
                    ModelRuntimeError::LoadError(format!(
                        "failed to mmap Candle RWKV v7 safetensors {}: {error}",
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
                "failed to construct Candle RWKV v7 model: {error}"
            ))
        })?;
        let state = State::new(&config, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle RWKV v7 state: {error}"
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
        let dea_tensors = self
            .state
            .dea
            .as_ref()
            .map(|dea| dea.k_cache.len() + dea.v_cache.len() + dea.q_prev.len())
            .unwrap_or(0);
        self.state.per_layer.len() * 3 + dea_tensors
    }

    pub fn state_has_dea(&self) -> bool {
        self.state.dea.is_some()
    }

    fn reset_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.state = State::new(&self.config, &self.device).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "failed to reset Candle RWKV v7 state: {error}"
            ))
        })?;
        Ok(())
    }
}

impl TransformerModel for CandleRwkvV7Model {
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
                adapter: "candle_rwkv_v7".to_string(),
            });
        }
        self.validate_lora_overrides(lora_overrides)?;
        let tokens = input_tokens(input_ids)?;
        self.model
            .forward_seq(&tokens, &mut self.state)
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!("Candle RWKV v7 forward failed: {error}"))
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
                capability: "rwkv_v7_lora".to_string(),
                adapter: "candle_rwkv_v7".to_string(),
            })
        }
    }

    // CRIT-1 / MT-088: DEA (delta-attention) state carries non-tensor
    // Vec<u32> token ids and per-layer K/V/Q caches that the current
    // SSMStateSnapshot wire format does not represent. Surface a typed
    // CapabilityNotSupported so the operator/validator sees the gap
    // instead of a corrupted round-trip. The DEA-less RWKV v7 path
    // round-trips through the same two-bucket layout as v5/v6 but
    // with v7's per-layer field names.
    fn extract_ssm_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        if self.state.dea.is_some() {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "rwkv_v7_dea_state_snapshot".to_string(),
                adapter: "candle_rwkv_v7".to_string(),
            });
        }
        let mut token_shift = Vec::with_capacity(self.state.per_layer.len() * 2);
        let mut ssm = Vec::with_capacity(self.state.per_layer.len());
        for layer in &self.state.per_layer {
            token_shift.push(tensor_to_snapshot(&layer.att_x_prev)?);
            token_shift.push(tensor_to_snapshot(&layer.ffn_x_prev)?);
            ssm.push(tensor_to_snapshot(&layer.att_kv)?);
        }
        Ok(SSMStateSnapshot::RwkvV7 { token_shift, ssm })
    }

    fn restore_ssm_snapshot(
        &mut self,
        snapshot: &SSMStateSnapshot,
    ) -> Result<(), ModelRuntimeError> {
        let (token_shift, ssm) = match snapshot {
            SSMStateSnapshot::RwkvV7 { token_shift, ssm } => (token_shift, ssm),
            other => {
                return Err(ModelRuntimeError::KvCacheError(format!(
                    "CandleRwkvV7Model::restore_ssm_snapshot variant mismatch: got {}",
                    other.variant()
                )));
            }
        };
        if self.state.dea.is_some() {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "rwkv_v7_dea_state_restore".to_string(),
                adapter: "candle_rwkv_v7".to_string(),
            });
        }
        let layer_count = self.state.per_layer.len();
        if token_shift.len() != layer_count * 2 {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "CandleRwkvV7Model::restore_ssm_snapshot token_shift length mismatch: expected {} got {}",
                layer_count * 2,
                token_shift.len()
            )));
        }
        if ssm.len() != layer_count {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "CandleRwkvV7Model::restore_ssm_snapshot ssm length mismatch: expected {} got {}",
                layer_count,
                ssm.len()
            )));
        }
        for (index, layer) in self.state.per_layer.iter_mut().enumerate() {
            layer.att_x_prev = snapshot_to_tensor(&token_shift[index * 2], &self.device)?;
            layer.ffn_x_prev = snapshot_to_tensor(&token_shift[index * 2 + 1], &self.device)?;
            layer.att_kv = snapshot_to_tensor(&ssm[index], &self.device)?;
        }
        Ok(())
    }
}

pub fn artifact_config_declares_rwkv_v7(path: &Path) -> Result<bool, ModelRuntimeError> {
    let value = read_config_value(&rwkv_v5::config_json_path_for_artifact(path))?;
    Ok(config_value_declares_rwkv_v7(&value))
}

pub fn config_value_declares_rwkv_v7(value: &Value) -> bool {
    let model_type = value
        .get("model_type")
        .and_then(Value::as_str)
        .is_some_and(|model_type| {
            let model_type = model_type.to_ascii_lowercase();
            model_type == "rwkv7" || model_type == "rwkv-v7"
        });
    let architecture = value
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|arch| {
            let arch = arch.to_ascii_lowercase();
            arch.contains("rwkv7") || arch.contains("rwkv_v7") || arch.contains("rwkv-7")
        });
    model_type || architecture
}

pub fn read_rwkv_v7_config(path: &Path) -> Result<(Config, Vec<u32>), ModelRuntimeError> {
    let value = read_config_value(path)?;
    let config = config_from_value(&value).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to decode Candle RWKV v7 config {}: {error}",
            path.display()
        ))
    })?;
    Ok((config, eos_token_ids(&value)))
}

pub fn config_from_value(value: &Value) -> Result<Config, String> {
    let hidden_size = required_usize(value, "hidden_size")?;
    let head_size = optional_usize(value, "head_size")
        .or_else(|| optional_usize(value, "head_dim"))
        .or_else(|| optional_usize(value, "head_size_a"))
        .unwrap_or(64);
    if head_size == 0 {
        return Err("head_size must be greater than zero".to_string());
    }
    if hidden_size % head_size != 0 {
        return Err(format!(
            "hidden_size {hidden_size} must be divisible by head_size {head_size}"
        ));
    }
    Ok(Config {
        version: version_from_value(value),
        vocab_size: required_usize(value, "vocab_size")?,
        hidden_size,
        num_hidden_layers: required_usize(value, "num_hidden_layers")?,
        head_size,
        intermediate_size: optional_usize(value, "intermediate_size")
            .or_else(|| hidden_ratio_intermediate_size(value)),
        rescale_every: optional_usize(value, "rescale_every").unwrap_or(0),
    })
}

fn input_tokens(input_ids: &Tensor) -> Result<Vec<u32>, ModelRuntimeError> {
    match input_ids.dims() {
        [1, seq_len] if *seq_len > 0 => {}
        dims => {
            return Err(ModelRuntimeError::GenerateError(format!(
                "Candle RWKV v7 expected input shape [1, seq], got {dims:?}"
            )))
        }
    }
    let cpu = input_ids.to_device(&Device::Cpu).map_err(|error| {
        ModelRuntimeError::GenerateError(format!("Candle RWKV v7 input transfer failed: {error}"))
    })?;
    let rows = cpu.to_vec2::<u32>().map_err(|error| {
        ModelRuntimeError::GenerateError(format!("Candle RWKV v7 input decode failed: {error}"))
    })?;
    rows.into_iter().next().ok_or_else(|| {
        ModelRuntimeError::GenerateError("Candle RWKV v7 input contained no rows".to_string())
    })
}

fn version_from_value(value: &Value) -> ModelVersion {
    let marker = value
        .get("version")
        .or_else(|| value.get("model_version"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("architectures")
                .and_then(Value::as_array)
                .and_then(|items| {
                    items.iter().filter_map(Value::as_str).find(|arch| {
                        let arch = arch.to_ascii_lowercase();
                        arch.contains("v7a") || arch.contains("v7b")
                    })
                })
        })
        .unwrap_or("v7")
        .to_ascii_lowercase();
    if marker.contains("v7b") {
        ModelVersion::V7b
    } else if marker.contains("v7a") {
        ModelVersion::V7a
    } else {
        ModelVersion::V7
    }
}

fn required_usize(value: &Value, key: &str) -> Result<usize, String> {
    optional_usize(value, key).ok_or_else(|| format!("missing required {key}"))
}

fn optional_usize(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

fn hidden_ratio_intermediate_size(value: &Value) -> Option<usize> {
    let hidden_size = optional_usize(value, "hidden_size")?;
    let ratio = value.get("hidden_ratio").and_then(Value::as_f64)?;
    Some((hidden_size as f64 * ratio).round() as usize)
}

fn read_config_value(path: &Path) -> Result<Value, ModelRuntimeError> {
    let config_json = fs::read_to_string(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read Candle RWKV v7 config {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str::<Value>(&config_json).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse Candle RWKV v7 config {}: {error}",
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
