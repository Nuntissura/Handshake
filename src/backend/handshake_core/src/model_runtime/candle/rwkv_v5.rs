#![cfg(feature = "candle-runtime-engine")]

// MT-115 / MT-116 (INF-9 full feature parity): Handshake-OWNED RWKV v5 forward.
//
// Adapted from candle-transformers 0.10.2 `src/models/rwkv_v5.rs`. Upstream
// keeps the RWKV `Block` / `SelfAttention` (time-mix) / `FeedForward`
// (channel-mix) forwards + their projection Linears private, so neither a LoRA
// delta nor an activation-steering vector can be threaded through the opaque
// `Model::forward`. This module re-implements the RWKV v5 forward with
// repo-OWNED `candle_nn::Linear` projections + an explicit residual-stream
// seam, exactly mirroring the owned Mamba2 implementation (mamba2.rs):
//   - LoRA: after each owned time-mix / channel-mix Linear forward we call
//     `CandleLoraStack::apply_to_linear_output(...)` (the same PEFT delta
//     engine the transformer + Mamba2 paths use).
//   - Steering: after each layer block we call
//     `CandleSteeringHooks::apply_record_and_capture_tensor(layer, ResidStream,
//     ...)` — the RWKV "residual stream" is the per-token layer-block output
//     (see `ssm_hook_site_for(RwkvV5, ResidStream) = rwkv.layer_block.output`).
//
// Numerical fidelity is pinned by the `tests` module, which builds BOTH this
// owned model and the upstream candle `Model` from the SAME VarMap and asserts
// step-by-step logit parity (no downloaded model required).
//
// RECURRENCE SUBTLETY (documented per MT contract): RWKV v5 stores TWO distinct
// token-shift states per layer (`extract_key_value` for the time-mix block,
// `feed_forward` for the channel-mix block) plus a headwise WKV matrix
// (`linear_attention`, shape `[b, n_heads, head_size, head_size]`). The
// time-mix interpolation reads `extract_key_value` (the PREVIOUS token's hidden
// state) and then OVERWRITES it with the CURRENT token's hidden state — so the
// owned forward must update state in the exact same order as upstream. The
// per-head WKV recurrence uses a per-head time-decay `exp(-exp(time_decay))`
// and a per-head bonus `time_faaaa`; both are reshaped to `[n_heads, head, 1]`.
// The owned forward reuses candle's RWKV v5 `State` verbatim, so the MT-088
// snapshot/restore logic (rwkv_v5_state_to_snapshots / rwkv_v5_restore_state)
// is preserved unchanged and is still shared with the v6 / v7 wrappers.

use std::{fs, path::Path};

use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_nn::{Embedding, GroupNorm, LayerNorm, Linear, VarBuilder};
use candle_transformers::models::rwkv_v5::{Config, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks,
    lora_impl::CandleLoraStack,
    ssm_state::{snapshot_to_tensor, tensor_to_snapshot},
    state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{
    HookPoint, LayerIndex, LoraId, LoraStackHandle, ModelId, ModelRuntimeError, SteeringVectorId,
};

// ---------------------------------------------------------------------------
// Owned RWKV v5 time-mix block (re-implements candle SelfAttention but with
// repo-owned, hook-/LoRA-instrumentable Linears).
// ---------------------------------------------------------------------------

struct OwnedTimeMix {
    key: Linear,
    receptance: Linear,
    value: Linear,
    gate: Linear,
    output: Linear,
    ln_x: GroupNorm,
    time_mix_key: Tensor,
    time_mix_value: Tensor,
    time_mix_receptance: Tensor,
    time_decay: Tensor,
    time_faaaa: Tensor,
    time_mix_gate: Tensor,
    key_target: String,
    receptance_target: String,
    value_target: String,
    gate_target: String,
    output_target: String,
    layer_id: usize,
    n_attn_heads: usize,
}

impl OwnedTimeMix {
    fn new(layer_id: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let hidden_size = cfg.hidden_size;
        let attn_hidden_size = cfg.attention_hidden_size;
        let key = candle_nn::linear_no_bias(hidden_size, attn_hidden_size, vb.pp("key"))
            .map_err(load_err)?;
        let receptance =
            candle_nn::linear_no_bias(hidden_size, attn_hidden_size, vb.pp("receptance"))
                .map_err(load_err)?;
        let value = candle_nn::linear_no_bias(hidden_size, attn_hidden_size, vb.pp("value"))
            .map_err(load_err)?;
        let gate = candle_nn::linear_no_bias(hidden_size, attn_hidden_size, vb.pp("gate"))
            .map_err(load_err)?;
        let output = candle_nn::linear_no_bias(attn_hidden_size, hidden_size, vb.pp("output"))
            .map_err(load_err)?;
        let ln_x = candle_nn::group_norm(
            hidden_size / cfg.head_size,
            hidden_size,
            1e-5,
            vb.pp("ln_x"),
        )
        .map_err(load_err)?;
        let time_mix_key = vb
            .get((1, 1, cfg.hidden_size), "time_mix_key")
            .map_err(load_err)?;
        let time_mix_value = vb
            .get((1, 1, cfg.hidden_size), "time_mix_value")
            .map_err(load_err)?;
        let time_mix_receptance = vb
            .get((1, 1, cfg.hidden_size), "time_mix_receptance")
            .map_err(load_err)?;
        let n_attn_heads = cfg.hidden_size / cfg.head_size;
        let time_decay = vb
            .get((n_attn_heads, cfg.head_size), "time_decay")
            .map_err(load_err)?;
        let time_faaaa = vb
            .get((n_attn_heads, cfg.head_size), "time_faaaa")
            .map_err(load_err)?;
        let time_mix_gate = vb
            .get((1, 1, cfg.hidden_size), "time_mix_gate")
            .map_err(load_err)?;
        Ok(Self {
            key,
            value,
            receptance,
            gate,
            output,
            ln_x,
            time_mix_key,
            time_mix_value,
            time_mix_receptance,
            time_decay,
            time_faaaa,
            time_mix_gate,
            key_target: rwkv_v5_target(layer_id, "time_mix", "key"),
            receptance_target: rwkv_v5_target(layer_id, "time_mix", "receptance"),
            value_target: rwkv_v5_target(layer_id, "time_mix", "value"),
            gate_target: rwkv_v5_target(layer_id, "time_mix", "gate"),
            output_target: rwkv_v5_target(layer_id, "time_mix", "output"),
            layer_id,
            n_attn_heads,
        })
    }

    /// Time-mix forward (mirrors candle `SelfAttention::forward`), with the
    /// owned key/receptance/value/gate/output outputs routed through the LoRA
    /// delta engine.
    fn forward(
        &self,
        xs: &Tensor,
        state: &mut State,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let h = self.time_decay.dim(0).map_err(gen_err)?;
        let (b, t, s) = xs.dims3().map_err(gen_err)?;
        let s = s / h;
        let (receptance, key, value, gate) = {
            // extract key-value (token shift): the PREVIOUS token's hidden state.
            let shifted = state.per_layer[self.layer_id].extract_key_value.clone();
            let shifted = if shifted.rank() == 2 {
                shifted.unsqueeze(1).map_err(gen_err)?
            } else {
                shifted
            };
            let key = ((xs * &self.time_mix_key).map_err(gen_err)?
                + &shifted * (1.0 - &self.time_mix_key).map_err(gen_err)?)
                .map_err(gen_err)?;
            let value = ((xs * &self.time_mix_value).map_err(gen_err)?
                + &shifted * (1.0 - &self.time_mix_value).map_err(gen_err)?)
                .map_err(gen_err)?;
            let receptance = ((xs * &self.time_mix_receptance).map_err(gen_err)?
                + &shifted * (1.0 - &self.time_mix_receptance).map_err(gen_err)?)
                .map_err(gen_err)?;
            let gate = ((xs * &self.time_mix_gate).map_err(gen_err)?
                + &shifted * (1.0 - &self.time_mix_gate).map_err(gen_err)?)
                .map_err(gen_err)?;

            let key_out = self.key.forward(&key).map_err(gen_err)?;
            let key_out =
                lora_stack.apply_to_linear_output(&self.key_target, &key_out, &key, lora_overrides)?;
            let value_out = self.value.forward(&value).map_err(gen_err)?;
            let value_out = lora_stack.apply_to_linear_output(
                &self.value_target,
                &value_out,
                &value,
                lora_overrides,
            )?;
            let receptance_out = self.receptance.forward(&receptance).map_err(gen_err)?;
            let receptance_out = lora_stack.apply_to_linear_output(
                &self.receptance_target,
                &receptance_out,
                &receptance,
                lora_overrides,
            )?;
            let gate_out = self.gate.forward(&gate).map_err(gen_err)?;
            let gate_out = lora_stack.apply_to_linear_output(
                &self.gate_target,
                &gate_out,
                &gate,
                lora_overrides,
            )?;
            let gate_out = candle_nn::ops::silu(&gate_out).map_err(gen_err)?;
            state.per_layer[self.layer_id].extract_key_value = xs.i((.., t - 1)).map_err(gen_err)?;
            (receptance_out, key_out, value_out, gate_out)
        };
        // linear attention (headwise WKV recurrence).
        let mut state_ = state.per_layer[self.layer_id].linear_attention.clone();
        let key = key
            .reshape((b, t, h, s))
            .and_then(|t| t.permute((0, 2, 3, 1)))
            .map_err(gen_err)?;
        let value = value
            .reshape((b, t, h, s))
            .and_then(|t| t.transpose(1, 2))
            .map_err(gen_err)?;
        let receptance = receptance
            .reshape((b, t, h, s))
            .and_then(|t| t.transpose(1, 2))
            .map_err(gen_err)?;

        let time_decay = self
            .time_decay
            .exp()
            .and_then(|t| t.neg())
            .and_then(|t| t.exp())
            .and_then(|t| t.reshape(((), 1, 1)))
            .and_then(|t| t.reshape((self.n_attn_heads, (), 1)))
            .map_err(gen_err)?;
        let time_faaaa = self
            .time_faaaa
            .reshape(((), 1, 1))
            .and_then(|t| t.reshape((self.n_attn_heads, (), 1)))
            .map_err(gen_err)?;

        let mut out: Vec<Tensor> = Vec::with_capacity(t);
        for t_ in 0..t {
            let rt = receptance
                .i((.., .., t_..t_ + 1))
                .and_then(|t| t.contiguous())
                .map_err(gen_err)?;
            let kt = key
                .i((.., .., .., t_..t_ + 1))
                .and_then(|t| t.contiguous())
                .map_err(gen_err)?;
            let vt = value
                .i((.., .., t_..t_ + 1))
                .and_then(|t| t.contiguous())
                .map_err(gen_err)?;
            let at = kt.matmul(&vt).map_err(gen_err)?;
            let rhs = (time_faaaa.broadcast_mul(&at).map_err(gen_err)? + &state_).map_err(gen_err)?;
            let out_ = rt.matmul(&rhs).and_then(|t| t.squeeze(2)).map_err(gen_err)?;
            state_ = (&at + time_decay.broadcast_mul(&state_).map_err(gen_err)?).map_err(gen_err)?;
            out.push(out_)
        }
        let out = Tensor::cat(&out, 1)
            .and_then(|cat| cat.reshape((b * t, h * s, 1)))
            .map_err(gen_err)?;
        let out = out
            .apply(&self.ln_x)
            .and_then(|normed| normed.reshape((b, t, h * s)))
            .map_err(gen_err)?;
        let out = (out * gate).map_err(gen_err)?;
        let out_proj = self.output.forward(&out).map_err(gen_err)?;
        let out_proj =
            lora_stack.apply_to_linear_output(&self.output_target, &out_proj, &out, lora_overrides)?;
        state.per_layer[self.layer_id].linear_attention = state_;
        Ok(out_proj)
    }
}

// ---------------------------------------------------------------------------
// Owned RWKV v5 channel-mix block (re-implements candle FeedForward).
// ---------------------------------------------------------------------------

struct OwnedChannelMix {
    time_mix_key: Tensor,
    time_mix_receptance: Tensor,
    key: Linear,
    receptance: Linear,
    value: Linear,
    key_target: String,
    receptance_target: String,
    value_target: String,
    layer_id: usize,
}

impl OwnedChannelMix {
    fn new(layer_id: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let int_size = cfg
            .intermediate_size
            .unwrap_or(((cfg.hidden_size as f64 * 3.5) as usize) / 32 * 32);
        let key = candle_nn::linear_no_bias(cfg.hidden_size, int_size, vb.pp("key"))
            .map_err(load_err)?;
        let receptance =
            candle_nn::linear_no_bias(cfg.hidden_size, cfg.hidden_size, vb.pp("receptance"))
                .map_err(load_err)?;
        let value = candle_nn::linear_no_bias(int_size, cfg.hidden_size, vb.pp("value"))
            .map_err(load_err)?;
        let time_mix_key = vb
            .get((1, 1, cfg.hidden_size), "time_mix_key")
            .map_err(load_err)?;
        let time_mix_receptance = vb
            .get((1, 1, cfg.hidden_size), "time_mix_receptance")
            .map_err(load_err)?;
        Ok(Self {
            key,
            receptance,
            value,
            time_mix_key,
            time_mix_receptance,
            key_target: rwkv_v5_target(layer_id, "channel_mix", "key"),
            receptance_target: rwkv_v5_target(layer_id, "channel_mix", "receptance"),
            value_target: rwkv_v5_target(layer_id, "channel_mix", "value"),
            layer_id,
        })
    }

    fn forward(
        &self,
        xs: &Tensor,
        state: &mut State,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let shifted = &state.per_layer[self.layer_id].feed_forward;
        let key = (xs.broadcast_mul(&self.time_mix_key).map_err(gen_err)?
            + shifted
                .broadcast_mul(&(1.0 - &self.time_mix_key).map_err(gen_err)?)
                .map_err(gen_err)?)
        .map_err(gen_err)?;
        let receptance = (xs.broadcast_mul(&self.time_mix_receptance).map_err(gen_err)?
            + shifted
                .broadcast_mul(&(1.0 - &self.time_mix_receptance).map_err(gen_err)?)
                .map_err(gen_err)?)
        .map_err(gen_err)?;
        let key_out = key.apply(&self.key).map_err(gen_err)?;
        let key_out =
            lora_stack.apply_to_linear_output(&self.key_target, &key_out, &key, lora_overrides)?;
        let key_out = key_out.relu().and_then(|t| t.sqr()).map_err(gen_err)?;
        let value = key_out.apply(&self.value).map_err(gen_err)?;
        let value =
            lora_stack.apply_to_linear_output(&self.value_target, &value, &key_out, lora_overrides)?;
        let receptance_out = receptance.apply(&self.receptance).map_err(gen_err)?;
        let receptance_out = lora_stack.apply_to_linear_output(
            &self.receptance_target,
            &receptance_out,
            &receptance,
            lora_overrides,
        )?;
        let receptance_out = candle_nn::ops::sigmoid(&receptance_out).map_err(gen_err)?;
        state.per_layer[self.layer_id].feed_forward =
            xs.i((.., xs.dim(1).map_err(gen_err)? - 1)).map_err(gen_err)?;
        (receptance_out * value).map_err(gen_err)
    }
}

// ---------------------------------------------------------------------------
// Owned RWKV v5 block: pre_ln (layer 0) -> ln1 -> time-mix (+residual) ->
// ln2 -> channel-mix (+residual).
// ---------------------------------------------------------------------------

struct OwnedBlock {
    pre_ln: Option<LayerNorm>,
    ln1: LayerNorm,
    ln2: LayerNorm,
    time_mix: OwnedTimeMix,
    channel_mix: OwnedChannelMix,
}

impl OwnedBlock {
    fn new(layer_id: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let ln1 =
            candle_nn::layer_norm(cfg.hidden_size, cfg.layer_norm_epsilon, vb.pp("ln1"))
                .map_err(load_err)?;
        let ln2 =
            candle_nn::layer_norm(cfg.hidden_size, cfg.layer_norm_epsilon, vb.pp("ln2"))
                .map_err(load_err)?;
        let pre_ln = if layer_id == 0 {
            Some(
                candle_nn::layer_norm(cfg.hidden_size, cfg.layer_norm_epsilon, vb.pp("pre_ln"))
                    .map_err(load_err)?,
            )
        } else {
            None
        };
        let time_mix = OwnedTimeMix::new(layer_id, cfg, vb.pp("attention"))?;
        let channel_mix = OwnedChannelMix::new(layer_id, cfg, vb.pp("feed_forward"))?;
        Ok(Self {
            pre_ln,
            ln1,
            ln2,
            time_mix,
            channel_mix,
        })
    }

    fn forward(
        &self,
        xs: &Tensor,
        state: &mut State,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let xs = match self.pre_ln.as_ref() {
            None => xs.clone(),
            Some(pre_ln) => xs.apply(pre_ln).map_err(gen_err)?,
        };
        let attention =
            self.time_mix
                .forward(&xs.apply(&self.ln1).map_err(gen_err)?, state, lora_stack, lora_overrides)?;
        let xs = (xs + attention).map_err(gen_err)?;
        let feed_forward = self.channel_mix.forward(
            &xs.apply(&self.ln2).map_err(gen_err)?,
            state,
            lora_stack,
            lora_overrides,
        )?;
        (xs + feed_forward).map_err(gen_err)
    }
}

/// Repo-owned RWKV v5 model: embedding -> N blocks -> ln_out -> head.
/// Exposes a residual-stream hook seam + LoRA-instrumentable projections.
pub struct InstrumentedRwkvV5 {
    embeddings: Embedding,
    blocks: Vec<OwnedBlock>,
    ln_out: LayerNorm,
    head: Linear,
    rescale_every: usize,
    layers_are_rescaled: bool,
}

impl InstrumentedRwkvV5 {
    pub fn load(cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let vb_m = vb.pp("rwkv");
        let embeddings =
            candle_nn::embedding(cfg.vocab_size, cfg.hidden_size, vb_m.pp("embeddings"))
                .map_err(load_err)?;
        let mut blocks = Vec::with_capacity(cfg.num_hidden_layers);
        let vb_b = vb_m.pp("blocks");
        for block_index in 0..cfg.num_hidden_layers {
            blocks.push(OwnedBlock::new(block_index, cfg, vb_b.pp(block_index))?);
        }
        let ln_out = candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb_m.pp("ln_out"))
            .map_err(load_err)?;
        let head = candle_nn::linear_no_bias(cfg.hidden_size, cfg.vocab_size, vb.pp("head"))
            .map_err(load_err)?;
        Ok(Self {
            embeddings,
            blocks,
            ln_out,
            head,
            rescale_every: cfg.rescale_every,
            // Mirrors upstream: only the f16/bf16 path rescales; the owned
            // forward runs in f32 so this stays false.
            layers_are_rescaled: false,
        })
    }

    /// Forward over a `[b, t]` token tensor producing `[b, t, vocab]` logits,
    /// applying the residual-stream steering hook after each layer block
    /// (per-token semantics) and the LoRA delta after each owned projection.
    pub fn forward(
        &self,
        input_ids: &Tensor,
        state: &mut State,
        hooks: &CandleSteeringHooks,
        snapshot: &[crate::model_runtime::SteeringVector],
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let (_b_size, _seq_len) = input_ids.dims2().map_err(gen_err)?;
        let mut xs = input_ids.apply(&self.embeddings).map_err(gen_err)?;
        for (block_idx, block) in self.blocks.iter().enumerate() {
            xs = block.forward(&xs, state, lora_stack, lora_overrides)?;
            let li = LayerIndex::new(block_idx as u32);
            xs = hooks.apply_record_and_capture_tensor(li, HookPoint::ResidStream, &xs, snapshot)?;
            if self.layers_are_rescaled && (block_idx + 1) % self.rescale_every == 0 {
                xs = (xs / 2.).map_err(gen_err)?;
            }
        }
        let logits = xs
            .apply(&self.ln_out)
            .and_then(|t| t.apply(&self.head))
            .map_err(gen_err)?;
        state.pos += 1;
        Ok(logits)
    }

    pub fn n_layers(&self) -> usize {
        self.blocks.len()
    }
}

// ---------------------------------------------------------------------------
// CandleRwkvV5Model: TransformerModel adapter over the owned forward.
// ---------------------------------------------------------------------------

pub struct CandleRwkvV5Model {
    model: InstrumentedRwkvV5,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    device: Device,
    lora_stack: CandleLoraStack,
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
        model_id: ModelId,
        config: Config,
        eos_token_ids: Vec<u32>,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let model = InstrumentedRwkvV5::load(&config, vb)?;
        let state = State::new(1, &config, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle RWKV v5 state: {error}"
            ))
        })?;
        let lora_stack = CandleLoraStack::new_for_device(
            model_id,
            "candle-rwkv-v5",
            CandleLoraStack::available_rwkv_targets(config.num_hidden_layers),
            device.clone(),
        );
        Ok(Self {
            model,
            state,
            config,
            eos_token_ids,
            device: device.clone(),
            lora_stack,
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
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        self.validate_lora_overrides(lora_overrides)?;
        let snapshot = hooks.snapshot_vectors_for_request(steering_overrides)?;
        let seq_len = match input_ids.dims() {
            [1, seq_len] if *seq_len > 0 => *seq_len,
            dims => {
                return Err(ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v5 expected input shape [1, seq], got {dims:?}"
                )))
            }
        };
        // Token-by-token prefill: the owned RWKV recurrence is processed
        // sequentially (the per-token steering hook fires per token per layer).
        let mut final_logits = None;
        for idx in 0..seq_len {
            let token = input_ids.i((.., idx..idx + 1)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v5 token select failed: {error}"
                ))
            })?;
            let logits = self.model.forward(
                &token,
                &mut self.state,
                hooks,
                &snapshot,
                &self.lora_stack,
                lora_overrides,
            )?;
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

    fn lora_stack(&self) -> LoraStackHandle {
        self.lora_stack.handle()
    }

    fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        self.lora_stack.ensure_overrides_mounted(ids)
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

/// Per-layer LoRA target name for an owned RWKV v5 projection. `module` is the
/// block (`time_mix` or `channel_mix`); `proj` is the Linear inside it
/// (`receptance`/`key`/`value`/`gate`/`output` for time-mix,
/// `receptance`/`key`/`value` for channel-mix). Naming mirrors the
/// `CandleLoraStack::available_rwkv_targets()` scheme.
pub fn rwkv_v5_target(layer_idx: usize, module: &str, proj: &str) -> String {
    format!("backbone.layers.{layer_idx}.{module}.{proj}")
}

fn load_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!("Candle instrumented RWKV v5 load failed: {error}"))
}

fn gen_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::GenerateError(format!("Candle instrumented RWKV v5 forward failed: {error}"))
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

#[cfg(test)]
mod tests {
    //! MT-115 / MT-116 (INF-9) load-bearing proofs for the OWNED RWKV v5 forward:
    //!   - `owned_rwkv_v5_forward_matches_candle_transformers_step_by_step`:
    //!     builds BOTH the upstream candle RWKV v5 `Model` and our owned
    //!     `InstrumentedRwkvV5` from the SAME random-weight VarMap and asserts
    //!     step-by-step logit parity (no downloaded model). This proves the
    //!     owned forward is numerically faithful — the prerequisite for trusting
    //!     the LoRA/steering seams.
    //!   - `owned_rwkv_v5_lora_mount_diverges_then_unmount_reverts`: a fixture
    //!     LoRA mounted on layer-0 time-mix key actually changes generated
    //!     logits, and unmount restores them (MT-115 real PEFT mount/unmount).
    //!   - `owned_rwkv_v5_steering_zero_is_identity_random_diverges`: a zero
    //!     steering vector is an identity (MT-116 identity-test correctness
    //!     gate); a non-zero vector changes the logits.
    use super::*;
    use std::collections::HashMap;

    use candle_core::Tensor;
    use candle_transformers::models::rwkv_v5::Model as CandleRwkvV5;

    use crate::model_runtime::{
        BaseModelTag, HookPoint as HP, LayerIndex as LI, LicenseTag, LoraDescriptor, LoraId,
        LoraStackOps, LoraStrength, SteeringProvenance, SteeringVector, SteeringVectorValues,
    };

    fn tiny_config() -> Config {
        // head_size 4, hidden 16 -> 4 attention heads; attention_hidden_size
        // == hidden_size (the standard RWKV v5 convention).
        //
        // NOTE the dual "head" notion in upstream RWKV v5: `SelfAttention`
        // derives `n_attn_heads = hidden_size / head_size` (= 4 here), while
        // `State::new` derives its WKV-matrix head count as
        // `hidden_size / cfg.num_attention_heads`. For the headwise recurrence
        // matmuls to line up, both must agree, so `num_attention_heads` is set
        // to 4 (= hidden_size / head_size). Omitting it would default to 64 and
        // panic with divide-by-zero in State::new for a tiny model.
        serde_json::from_value(serde_json::json!({
            "vocab_size": 24,
            "hidden_size": 16,
            "num_hidden_layers": 2,
            "attention_hidden_size": 16,
            "num_attention_heads": 4,
            "head_size": 4,
            "intermediate_size": 32,
            "layer_norm_epsilon": 1e-5,
            "rescale_every": 6
        }))
        .expect("tiny RWKV v5 config")
    }

    struct Dims {
        hidden: usize,
        attn_hidden: usize,
        int_size: usize,
        vocab: usize,
        n_heads: usize,
        head_size: usize,
    }

    fn dims(cfg: &Config) -> Dims {
        let int_size = cfg
            .intermediate_size
            .unwrap_or(((cfg.hidden_size as f64 * 3.5) as usize) / 32 * 32);
        Dims {
            hidden: cfg.hidden_size,
            attn_hidden: cfg.attention_hidden_size,
            int_size,
            vocab: cfg.vocab_size,
            n_heads: cfg.hidden_size / cfg.head_size,
            head_size: cfg.head_size,
        }
    }

    fn random_weights(cfg: &Config, device: &Device) -> HashMap<String, Tensor> {
        let d = dims(cfg);
        let mut m = HashMap::new();
        let mut put = |name: String, shape: &[usize]| {
            m.insert(
                name,
                Tensor::randn(0f32, 0.2f32, shape.to_vec(), device).unwrap(),
            );
        };
        // Top-level.
        put("rwkv.embeddings.weight".to_string(), &[d.vocab, d.hidden]);
        put("rwkv.ln_out.weight".to_string(), &[d.hidden]);
        put("rwkv.ln_out.bias".to_string(), &[d.hidden]);
        put("head.weight".to_string(), &[d.vocab, d.hidden]);
        for i in 0..cfg.num_hidden_layers {
            let b = format!("rwkv.blocks.{i}");
            if i == 0 {
                put(format!("{b}.pre_ln.weight"), &[d.hidden]);
                put(format!("{b}.pre_ln.bias"), &[d.hidden]);
            }
            put(format!("{b}.ln1.weight"), &[d.hidden]);
            put(format!("{b}.ln1.bias"), &[d.hidden]);
            put(format!("{b}.ln2.weight"), &[d.hidden]);
            put(format!("{b}.ln2.bias"), &[d.hidden]);
            // time-mix (attention).
            let a = format!("{b}.attention");
            put(format!("{a}.key.weight"), &[d.attn_hidden, d.hidden]);
            put(format!("{a}.receptance.weight"), &[d.attn_hidden, d.hidden]);
            put(format!("{a}.value.weight"), &[d.attn_hidden, d.hidden]);
            put(format!("{a}.gate.weight"), &[d.attn_hidden, d.hidden]);
            put(format!("{a}.output.weight"), &[d.hidden, d.attn_hidden]);
            put(format!("{a}.ln_x.weight"), &[d.hidden]);
            put(format!("{a}.ln_x.bias"), &[d.hidden]);
            put(format!("{a}.time_mix_key"), &[1, 1, d.hidden]);
            put(format!("{a}.time_mix_value"), &[1, 1, d.hidden]);
            put(format!("{a}.time_mix_receptance"), &[1, 1, d.hidden]);
            put(format!("{a}.time_mix_gate"), &[1, 1, d.hidden]);
            put(format!("{a}.time_decay"), &[d.n_heads, d.head_size]);
            put(format!("{a}.time_faaaa"), &[d.n_heads, d.head_size]);
            // channel-mix (feed_forward).
            let f = format!("{b}.feed_forward");
            put(format!("{f}.key.weight"), &[d.int_size, d.hidden]);
            put(format!("{f}.receptance.weight"), &[d.hidden, d.hidden]);
            put(format!("{f}.value.weight"), &[d.hidden, d.int_size]);
            put(format!("{f}.time_mix_key"), &[1, 1, d.hidden]);
            put(format!("{f}.time_mix_receptance"), &[1, 1, d.hidden]);
        }
        m
    }

    fn token(id: u32, device: &Device) -> Tensor {
        // `[1, 1]` token tensor (the adapter feeds tokens token-by-token).
        Tensor::from_vec(vec![id], (1, 1), device).unwrap()
    }

    fn logits_vec(t: &Tensor) -> Vec<f32> {
        t.flatten_all().unwrap().to_vec1::<f32>().unwrap()
    }

    fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0f32, f32::max)
    }

    #[test]
    fn owned_rwkv_v5_forward_matches_candle_transformers_step_by_step() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let weights = random_weights(&cfg, &device);

        let candle_model = CandleRwkvV5::new(
            &cfg,
            VarBuilder::from_tensors(weights.clone(), DType::F32, &device),
        )
        .expect("candle rwkv_v5 builds");
        let mut candle_state = State::new(1, &cfg, &device).unwrap();

        let owned =
            InstrumentedRwkvV5::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v5 builds");
        let mut owned_state = State::new(1, &cfg, &device).unwrap();

        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-rwkv-v5",
            CandleLoraStack::available_rwkv_targets(cfg.num_hidden_layers),
        );

        // Drive both models token-by-token through their single-token forward;
        // the owned recurrence must match candle at every position.
        for &tid in &[3u32, 7, 1, 5, 2, 9] {
            let tok = token(tid, &device);
            let candle_logits = candle_model.forward(&tok, &mut candle_state).unwrap();
            let owned_logits = owned
                .forward(&tok, &mut owned_state, &hooks, &[], &lora, &[])
                .unwrap();
            let c = logits_vec(&candle_logits);
            let o = logits_vec(&owned_logits);
            assert_eq!(c.len(), o.len(), "logit width mismatch at token {tid}");
            let diff = max_abs_diff(&c, &o);
            assert!(
                diff < 1e-4,
                "owned RWKV v5 diverged from candle at token {tid}: max |Δlogit| = {diff}"
            );
        }
    }

    #[tokio::test]
    async fn owned_rwkv_v5_lora_mount_diverges_then_unmount_reverts() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedRwkvV5::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v5 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-rwkv-v5",
            CandleLoraStack::available_rwkv_targets(cfg.num_hidden_layers),
        );

        // Drive a short fixed sequence token-by-token through ONE state, then
        // read the final-token logits. Priming the recurrence (WKV matrix +
        // token-shift) before measuring avoids the fresh-state first-token
        // attenuation that shrinks a time-mix LoRA delta — see the RECURRENCE
        // SUBTLETY note at the top of this file.
        let seq = [3u32, 7, 1, 4];
        let run_seq = |overrides: &[LoraId]| {
            let mut state = State::new(1, &cfg, &device).unwrap();
            let mut last = None;
            for &tid in &seq {
                last = Some(
                    owned
                        .forward(&token(tid, &device), &mut state, &hooks, &[], &lora, overrides)
                        .unwrap(),
                );
            }
            logits_vec(&last.unwrap())
        };

        let baseline = run_seq(&[]);

        // Fixture LoRA on layer-0 time-mix receptance (rank 2): A [rank, hidden],
        // B [attn_hidden, rank]. Receptance gates the WKV output directly
        // (out_ = rt @ rhs). The tensors are DETERMINISTIC (constant-filled, no
        // RNG) so the test is reproducible, and large enough that the GENUINE
        // PEFT delta clears the 1e-4 assertion without weakening the threshold.
        let rank = 2usize;
        let target = rwkv_v5_target(0, "time_mix", "receptance");
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("rwkv_lora.safetensors");
        let mut tensors = HashMap::new();
        tensors.insert(
            format!("{target}.lora_A.weight"),
            (Tensor::ones((rank, d.hidden), DType::F32, &device).unwrap() * 0.5).unwrap(),
        );
        tensors.insert(
            format!("{target}.lora_B.weight"),
            (Tensor::ones((d.attn_hidden, rank), DType::F32, &device).unwrap() * 0.5).unwrap(),
        );
        candle_core::safetensors::save(&tensors, &path).unwrap();
        let sha = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(std::fs::read(&path).unwrap());
            let out: [u8; 32] = h.finalize().into();
            out
        };
        let lora_id = LoraId::new_v7();
        let descriptor = LoraDescriptor {
            id: lora_id,
            artifact_path: path.clone(),
            sha256: sha,
            rank: rank as u32,
            target_modules: vec![target.clone()],
            base_model_compat: BaseModelTag::new("candle-rwkv-v5"),
            license_tag: LicenseTag::new("test-license"),
        };
        lora
            .mount(descriptor, LoraStrength::try_new(1.0).unwrap())
            .await
            .expect("RWKV LoRA mounts");

        let mounted = run_seq(&[lora_id]);
        assert!(
            max_abs_diff(&baseline, &mounted) > 1e-4,
            "mounted RWKV LoRA must change time-mix receptance output and therefore the logits"
        );

        lora.unmount(lora_id).await.expect("unmount");
        let reverted = run_seq(&[]);
        assert!(
            max_abs_diff(&baseline, &reverted) < 1e-6,
            "unmounting the RWKV LoRA must revert the logits to baseline"
        );
    }

    #[tokio::test]
    async fn owned_rwkv_v5_steering_zero_is_identity_random_diverges() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedRwkvV5::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v5 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(ModelId::new_v7(), "candle-rwkv-v5", Vec::new());

        let run = |snapshot: &[SteeringVector]| {
            let mut state = State::new(1, &cfg, &device).unwrap();
            logits_vec(
                &owned
                    .forward(&token(6, &device), &mut state, &hooks, snapshot, &lora, &[])
                    .unwrap(),
            )
        };

        let baseline = run(&[]);

        // Zero vector at layer 0 residual stream -> additive identity.
        let zero = SteeringVector::try_new(
            None,
            "zero",
            LI::new(0),
            HP::ResidStream,
            SteeringVectorValues::try_new(vec![0.0_f32; d.hidden], 1.0).unwrap(),
            "zero steering vector",
            Some(SteeringProvenance::Manual {
                author: "test".to_string(),
                notes: "identity".to_string(),
            }),
        )
        .unwrap();
        hooks.register_vector(zero.clone()).await.unwrap();
        hooks.set_active(vec![zero.id]).await.unwrap();
        let zero_snapshot = hooks.snapshot_vectors_for_request(&[]).unwrap();
        let zeroed = run(&zero_snapshot);
        assert!(
            max_abs_diff(&baseline, &zeroed) < 1e-6,
            "a zero steering vector must be an identity on the RWKV residual stream"
        );

        // Non-zero vector at layer 0 -> logits must change.
        let mut direction = vec![0.0_f32; d.hidden];
        direction[0] = 1.0;
        direction[1] = -1.0;
        let nonzero = SteeringVector::try_new(
            None,
            "nonzero",
            LI::new(0),
            HP::ResidStream,
            SteeringVectorValues::try_new(direction, 3.0).unwrap(),
            "nonzero steering vector",
            Some(SteeringProvenance::Manual {
                author: "test".to_string(),
                notes: "diverge".to_string(),
            }),
        )
        .unwrap();
        hooks.register_vector(nonzero.clone()).await.unwrap();
        hooks.set_active(vec![nonzero.id]).await.unwrap();
        let nonzero_snapshot = hooks.snapshot_vectors_for_request(&[]).unwrap();
        let steered = run(&nonzero_snapshot);
        assert!(
            max_abs_diff(&baseline, &steered) > 1e-4,
            "a non-zero steering vector must change the RWKV logits"
        );
    }
}
