#![cfg(feature = "candle-runtime-engine")]

use std::{fs, path::Path};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Config, LlamaConfig, LlamaEosToks};

use super::{
    hooks::CandleSteeringHooks,
    instrumented_llama::{InstrumentedLlama, InstrumentedLlamaCache},
    lora_impl::CandleLoraStack,
    state_vector::SSMStateSnapshot,
};
use crate::model_runtime::{LoraId, LoraStackHandle, ModelId, ModelRuntimeError, SteeringVectorId};

pub trait TransformerModel: Send {
    fn forward(
        &mut self,
        input_ids: &Tensor,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError>;

    /// Teacher-forcing forward returning logits for EVERY position, shape
    /// `[batch, seq, vocab]` (F32). This is what `score()` needs to read the
    /// next-token log-probability at each position. The default impl reports
    /// capability-not-supported so a backend that genuinely cannot expose
    /// per-position logits fails honestly instead of faking a score.
    fn forward_full_logits(
        &mut self,
        _input_ids: &Tensor,
        _hooks: &CandleSteeringHooks,
        _steering_overrides: &[SteeringVectorId],
        _lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "per_position_teacher_forcing_logits".to_string(),
            adapter: "candle_transformer_model".to_string(),
        })
    }

    /// Forward returning the post-final-norm hidden states for every position,
    /// shape `[batch, seq, hidden]` (F32) — the representation `embed()` pools
    /// over. Default impl is capability-not-supported for backends that do not
    /// cleanly expose a hidden state.
    fn forward_hidden_states(
        &mut self,
        _input_ids: &Tensor,
        _hooks: &CandleSteeringHooks,
        _steering_overrides: &[SteeringVectorId],
        _lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "per_position_hidden_states".to_string(),
            adapter: "candle_transformer_model".to_string(),
        })
    }

    fn n_layers(&self) -> u32;

    fn hidden_dim(&self) -> u32;

    fn vocab_size(&self) -> u32;

    fn eos_token_ids(&self) -> &[u32];

    fn device(&self) -> Device;

    fn reset_generation_state(&mut self) -> Result<(), ModelRuntimeError>;

    fn lora_stack(&self) -> LoraStackHandle {
        LoraStackHandle::new("candle_transformer_unbound_lora_stack")
    }

    fn validate_lora_overrides(&self, _ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    /// CRIT-1 / MT-088 — serialize SSM-style mutable inference state to a
    /// portable [`SSMStateSnapshot`]. Non-SSM models (Llama, etc.) leave
    /// the default impl in place and report capability-not-supported so
    /// `StateVectorHandle::prefix_commit` never silently treats an empty
    /// payload as success.
    fn extract_ssm_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "ssm_state_snapshot_extract".to_string(),
            adapter: "non_ssm_transformer_model".to_string(),
        })
    }

    /// CRIT-1 / MT-088 — write a previously-extracted snapshot back into
    /// the model's mutable inference state. Mirror semantics of
    /// `extract_ssm_snapshot`.
    fn restore_ssm_snapshot(
        &mut self,
        _snapshot: &SSMStateSnapshot,
    ) -> Result<(), ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "ssm_state_snapshot_restore".to_string(),
            adapter: "non_ssm_transformer_model".to_string(),
        })
    }
}

pub struct CandleLlamaModel {
    model: InstrumentedLlama,
    cache: InstrumentedLlamaCache,
    config: Config,
    eos_token_ids: Vec<u32>,
    index_pos: usize,
    dtype: DType,
    device: Device,
    lora_stack: CandleLoraStack,
}

impl CandleLlamaModel {
    pub fn load_safetensors(
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        Self::load_safetensors_for_model(ModelId::new_v7(), artifact_path, device)
    }

    pub fn load_safetensors_for_model(
        model_id: ModelId,
        artifact_path: &Path,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let config_path = config_json_path_for_artifact(artifact_path);
        let config_json = fs::read_to_string(&config_path).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to read Candle Llama config {}: {error}",
                config_path.display()
            ))
        })?;
        let llama_config = serde_json::from_str::<LlamaConfig>(&config_json).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to parse Candle Llama config {}: {error}",
                config_path.display()
            ))
        })?;
        let eos_token_ids = eos_token_ids(&llama_config);
        let config = llama_config.into_config(false);
        let dtype = DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[artifact_path], dtype, device).map_err(
                |error| {
                    ModelRuntimeError::LoadError(format!(
                        "failed to mmap Candle safetensors {}: {error}",
                        artifact_path.display()
                    ))
                },
            )?
        };
        Self::from_varbuilder_with_dtype(model_id, config, eos_token_ids, vb, dtype, device)
    }

    pub fn from_varbuilder(
        config: Config,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        Self::from_varbuilder_for_model(ModelId::new_v7(), config, vb, device)
    }

    pub fn from_varbuilder_for_model(
        model_id: ModelId,
        config: Config,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        Self::from_varbuilder_with_dtype(model_id, config, Vec::new(), vb, DType::F32, device)
    }

    fn from_varbuilder_with_dtype(
        model_id: ModelId,
        config: Config,
        eos_token_ids: Vec<u32>,
        vb: VarBuilder,
        dtype: DType,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let model = InstrumentedLlama::load(vb, &config)?;
        let cache = InstrumentedLlamaCache::new(true, dtype, &config, device)?;
        let lora_stack = CandleLoraStack::new_for_device(
            model_id,
            "candle-llama",
            CandleLoraStack::available_llama_targets(config.num_hidden_layers),
            device.clone(),
        );
        Ok(Self {
            model,
            cache,
            config,
            eos_token_ids,
            index_pos: 0,
            dtype,
            device: device.clone(),
            lora_stack,
        })
    }
}

impl TransformerModel for CandleLlamaModel {
    fn forward(
        &mut self,
        input_ids: &Tensor,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let seq_len = input_ids.dims().last().copied().ok_or_else(|| {
            ModelRuntimeError::GenerateError("empty Candle input tensor".to_string())
        })?;
        let logits = self.model.forward(
            input_ids,
            self.index_pos,
            &mut self.cache,
            hooks,
            steering_overrides,
            &self.lora_stack,
            lora_overrides,
        )?;
        self.index_pos = self.index_pos.saturating_add(seq_len);
        Ok(logits)
    }

    fn forward_full_logits(
        &mut self,
        input_ids: &Tensor,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        // Teacher forcing is a single pass over the whole sequence from a clean
        // KV cache: reset, run from position 0 over all positions, then reset
        // again so the model is left in the same clean state a generate path
        // expects (no leftover index_pos / KV from the scoring pass).
        self.reset_generation_state()?;
        let result = self.model.forward_full_logits(
            input_ids,
            0,
            &mut self.cache,
            hooks,
            steering_overrides,
            &self.lora_stack,
            lora_overrides,
        );
        self.reset_generation_state()?;
        result
    }

    fn forward_hidden_states(
        &mut self,
        input_ids: &Tensor,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        self.reset_generation_state()?;
        let result = self.model.forward_hidden_states(
            input_ids,
            0,
            &mut self.cache,
            hooks,
            steering_overrides,
            &self.lora_stack,
            lora_overrides,
        );
        self.reset_generation_state()?;
        result
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
        self.cache = InstrumentedLlamaCache::new(true, self.dtype, &self.config, &self.device)?;
        self.index_pos = 0;
        Ok(())
    }

    fn lora_stack(&self) -> LoraStackHandle {
        self.lora_stack.handle()
    }

    fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        self.lora_stack.ensure_overrides_mounted(ids)
    }
}

pub fn config_json_path_for_artifact(artifact_path: &Path) -> std::path::PathBuf {
    artifact_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("config.json")
}

fn eos_token_ids(config: &LlamaConfig) -> Vec<u32> {
    match &config.eos_token_id {
        Some(LlamaEosToks::Single(value)) => vec![*value],
        Some(LlamaEosToks::Multiple(values)) => values.clone(),
        None => Vec::new(),
    }
}
