#![cfg(feature = "candle-runtime-engine")]

use std::path::Path;

use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::rwkv_v6::{Config, Model, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks, rwkv_v5, state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{LoraId, ModelId, ModelRuntimeError, SteeringVectorId};

pub struct CandleRwkvV6Model {
    model: Model,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    device: Device,
}

impl CandleRwkvV6Model {
    pub fn load_safetensors_for_model(
        model_id: ModelId,
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let config_path = rwkv_v5::config_json_path_for_artifact(artifact_path);
        let (config, eos_token_ids) = rwkv_v5::read_rwkv_config(&config_path)?;
        let dtype = candle_core::DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[artifact_path], dtype, device).map_err(
                |error| {
                    ModelRuntimeError::LoadError(format!(
                        "failed to mmap Candle RWKV v6 safetensors {}: {error}",
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
                "failed to construct Candle RWKV v6 model: {error}"
            ))
        })?;
        let state = State::new(1, &config, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle RWKV v6 state: {error}"
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
                "failed to reset Candle RWKV v6 state: {error}"
            ))
        })?;
        Ok(())
    }
}

impl TransformerModel for CandleRwkvV6Model {
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
                adapter: "candle_rwkv_v6".to_string(),
            });
        }
        self.validate_lora_overrides(lora_overrides)?;
        let seq_len = match input_ids.dims() {
            [1, seq_len] if *seq_len > 0 => *seq_len,
            dims => {
                return Err(ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v6 expected input shape [1, seq], got {dims:?}"
                )))
            }
        };
        let mut final_logits = None;
        for idx in 0..seq_len {
            let token = input_ids.i((.., idx..idx + 1)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v6 token select failed: {error}"
                ))
            })?;
            let logits = self
                .model
                .forward(&token, &mut self.state)
                .map_err(|error| {
                    ModelRuntimeError::GenerateError(format!(
                        "Candle RWKV v6 forward failed: {error}"
                    ))
                })?;
            final_logits = Some(logits.i((0, 0)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v6 final logits select failed: {error}"
                ))
            })?);
        }
        final_logits.ok_or_else(|| {
            ModelRuntimeError::GenerateError("Candle RWKV v6 produced no logits".to_string())
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
                capability: "rwkv_v6_lora".to_string(),
                adapter: "candle_rwkv_v6".to_string(),
            })
        }
    }

    // CRIT-1 / MT-088: RWKV v6 reuses v5's State + StatePerLayer types
    // (`pub use crate::models::rwkv_v5::{Config, State, Tokenizer};` in
    // candle-transformers), so the v5 packing layout applies as-is.
    fn extract_ssm_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        let (token_shift, ssm) = rwkv_v5::rwkv_v5_state_to_snapshots(&self.state)?;
        Ok(SSMStateSnapshot::RwkvV6 { token_shift, ssm })
    }

    fn restore_ssm_snapshot(
        &mut self,
        snapshot: &SSMStateSnapshot,
    ) -> Result<(), ModelRuntimeError> {
        let (token_shift, ssm) = match snapshot {
            SSMStateSnapshot::RwkvV6 { token_shift, ssm } => (token_shift, ssm),
            other => {
                return Err(ModelRuntimeError::KvCacheError(format!(
                    "CandleRwkvV6Model::restore_ssm_snapshot variant mismatch: got {}",
                    other.variant()
                )));
            }
        };
        rwkv_v5::rwkv_v5_restore_state(&mut self.state, &self.device, token_shift, ssm)
    }
}

pub fn artifact_config_declares_rwkv_v6(path: &Path) -> Result<bool, ModelRuntimeError> {
    let config_path = rwkv_v5::config_json_path_for_artifact(path);
    let config_json = std::fs::read_to_string(&config_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read Candle RWKV config {}: {error}",
            config_path.display()
        ))
    })?;
    let value = serde_json::from_str::<Value>(&config_json).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse Candle RWKV config {}: {error}",
            config_path.display()
        ))
    })?;
    Ok(config_value_declares_rwkv_v6(&value))
}

pub fn config_value_declares_rwkv_v6(value: &Value) -> bool {
    let model_type = value
        .get("model_type")
        .and_then(Value::as_str)
        .is_some_and(|model_type| {
            let model_type = model_type.to_ascii_lowercase();
            model_type == "rwkv6" || model_type == "rwkv-v6"
        });
    let architecture = value
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|arch| {
            let arch = arch.to_ascii_lowercase();
            arch.contains("rwkv6") || arch.contains("rwkv_v6") || arch.contains("rwkv-6")
        });
    model_type || architecture
}
