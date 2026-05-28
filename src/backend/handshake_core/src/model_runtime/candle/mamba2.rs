#![cfg(feature = "candle-runtime-engine")]

use std::{fs, path::Path};

use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::mamba2::{Config, Model, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks,
    ssm_state::{snapshot_to_tensor, tensor_to_snapshot},
    state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{LoraId, ModelId, ModelRuntimeError, SteeringVectorId};

const MAMBA2_PREFILL_CHUNK_SIZE: usize = 64;

pub struct CandleMamba2Model {
    model: Model,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    dtype: DType,
    device: Device,
}

impl CandleMamba2Model {
    pub fn load_safetensors_for_model(
        model_id: ModelId,
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let config_path = config_json_path_for_artifact(artifact_path);
        let (config, eos_token_ids) = read_mamba2_config(&config_path)?;
        let dtype = DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[artifact_path], dtype, device).map_err(
                |error| {
                    ModelRuntimeError::LoadError(format!(
                        "failed to mmap Candle Mamba2 safetensors {}: {error}",
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
        let dtype = vb.dtype();
        let model = Model::new(&config, vb).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to construct Candle Mamba2 model: {error}"
            ))
        })?;
        let state = State::new(1, &config, dtype, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle Mamba2 state: {error}"
            ))
        })?;
        Ok(Self {
            model,
            state,
            config,
            eos_token_ids,
            dtype,
            device: device.clone(),
        })
    }

    pub fn state_position(&self) -> usize {
        self.state.pos
    }

    fn reset_state(&mut self) -> Result<(), ModelRuntimeError> {
        self.state = State::new(1, &self.config, self.dtype, &self.device).map_err(|error| {
            ModelRuntimeError::GenerateError(format!(
                "failed to reset Candle Mamba2 state: {error}"
            ))
        })?;
        Ok(())
    }

    fn trim_padded_vocab(&self, logits: Tensor) -> Result<Tensor, ModelRuntimeError> {
        let dim = logits.rank().saturating_sub(1);
        logits
            .narrow(dim, 0, self.config.vocab_size)
            .map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "failed to trim Candle Mamba2 padded vocabulary logits: {error}"
                ))
            })
    }
}

impl TransformerModel for CandleMamba2Model {
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
                adapter: "candle_mamba2".to_string(),
            });
        }
        self.validate_lora_overrides(lora_overrides)?;
        match input_ids.dims() {
            [1, seq_len] if *seq_len > 1 => {
                let logits = self
                    .model
                    .forward_prefill(input_ids, &mut self.state, MAMBA2_PREFILL_CHUNK_SIZE)
                    .map_err(|error| {
                        ModelRuntimeError::GenerateError(format!(
                            "Candle Mamba2 prefill failed: {error}"
                        ))
                    })?;
                let logits = self.trim_padded_vocab(logits)?;
                logits.i((0, seq_len - 1)).map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "Candle Mamba2 final prefill logits select failed: {error}"
                    ))
                })
            }
            [1, 1] => {
                let token = input_ids.squeeze(0).map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "Candle Mamba2 token squeeze failed: {error}"
                    ))
                })?;
                let logits = self
                    .model
                    .forward(&token, &mut self.state)
                    .map_err(|error| {
                        ModelRuntimeError::GenerateError(format!(
                            "Candle Mamba2 step failed: {error}"
                        ))
                    })?;
                self.trim_padded_vocab(logits)
            }
            [seq_len] if *seq_len == 1 => {
                let logits = self
                    .model
                    .forward(input_ids, &mut self.state)
                    .map_err(|error| {
                        ModelRuntimeError::GenerateError(format!(
                            "Candle Mamba2 step failed: {error}"
                        ))
                    })?;
                self.trim_padded_vocab(logits)
            }
            dims => Err(ModelRuntimeError::GenerateError(format!(
                "Candle Mamba2 expected input shape [1, seq] or [1], got {dims:?}"
            ))),
        }
    }

    fn n_layers(&self) -> u32 {
        self.config.n_layer as u32
    }

    fn hidden_dim(&self) -> u32 {
        self.config.d_model as u32
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
                capability: "mamba2_lora".to_string(),
                adapter: "candle_mamba2".to_string(),
            })
        }
    }

    // CRIT-1 / MT-088: pack conv1d state + SSM hs into the typed snapshot
    // expected by StateVectorHandle.
    fn extract_ssm_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        let conv_states = self
            .state
            .conv_states
            .iter()
            .map(tensor_to_snapshot)
            .collect::<Result<Vec<_>, _>>()?;
        let ssm_states = self
            .state
            .hs
            .iter()
            .map(tensor_to_snapshot)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SSMStateSnapshot::Mamba2 {
            conv_states,
            ssm_states,
        })
    }

    fn restore_ssm_snapshot(
        &mut self,
        snapshot: &SSMStateSnapshot,
    ) -> Result<(), ModelRuntimeError> {
        let (conv_states, ssm_states) = match snapshot {
            SSMStateSnapshot::Mamba2 {
                conv_states,
                ssm_states,
            } => (conv_states, ssm_states),
            other => {
                return Err(ModelRuntimeError::KvCacheError(format!(
                    "CandleMamba2Model::restore_ssm_snapshot variant mismatch: got {}",
                    other.variant()
                )));
            }
        };
        if conv_states.len() != self.state.conv_states.len() {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "CandleMamba2Model::restore_ssm_snapshot conv_states length mismatch: expected {} got {}",
                self.state.conv_states.len(),
                conv_states.len()
            )));
        }
        if ssm_states.len() != self.state.hs.len() {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "CandleMamba2Model::restore_ssm_snapshot ssm_states length mismatch: expected {} got {}",
                self.state.hs.len(),
                ssm_states.len()
            )));
        }
        for (index, snap) in conv_states.iter().enumerate() {
            self.state.conv_states[index] = snapshot_to_tensor(snap, &self.device)?;
        }
        for (index, snap) in ssm_states.iter().enumerate() {
            self.state.hs[index] = snapshot_to_tensor(snap, &self.device)?;
        }
        Ok(())
    }
}

pub fn artifact_config_declares_mamba2(artifact_path: &Path) -> Result<bool, ModelRuntimeError> {
    let config_path = config_json_path_for_artifact(artifact_path);
    let config_json = fs::read_to_string(&config_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read Candle config {}: {error}",
            config_path.display()
        ))
    })?;
    let value = serde_json::from_str::<Value>(&config_json).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse Candle config {}: {error}",
            config_path.display()
        ))
    })?;
    Ok(config_value_declares_mamba2(&value))
}

pub fn config_value_declares_mamba2(value: &Value) -> bool {
    let model_type = value
        .get("model_type")
        .and_then(Value::as_str)
        .is_some_and(|model_type| model_type.eq_ignore_ascii_case("mamba2"));
    let architecture = value
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|arch| arch.to_ascii_lowercase().contains("mamba2"));
    let ssm_layer = value
        .get("ssm_cfg")
        .and_then(|ssm| ssm.get("layer"))
        .and_then(Value::as_str)
        .is_some_and(|layer| layer.eq_ignore_ascii_case("mamba2"));
    model_type || architecture || ssm_layer
}

fn read_mamba2_config(path: &Path) -> Result<(Config, Vec<u32>), ModelRuntimeError> {
    let config_json = fs::read_to_string(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read Candle Mamba2 config {}: {error}",
            path.display()
        ))
    })?;
    let value = serde_json::from_str::<Value>(&config_json).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse Candle Mamba2 config {}: {error}",
            path.display()
        ))
    })?;
    let config = serde_json::from_value::<Config>(value.clone()).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to decode Candle Mamba2 config {}: {error}",
            path.display()
        ))
    })?;
    Ok((config, eos_token_ids(&value)))
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
