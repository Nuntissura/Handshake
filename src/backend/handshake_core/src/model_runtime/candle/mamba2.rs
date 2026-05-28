#![cfg(feature = "candle-runtime-engine")]

// MT-115 / MT-116 (INF-9 full feature parity): Handshake-OWNED Mamba2 forward.
//
// Adapted from candle-transformers 0.10.2 `src/models/mamba2.rs`. Upstream keeps
// the Mamba2 block forward + its in_proj/out_proj Linears private, so neither a
// LoRA delta nor an activation-steering vector can be threaded through the
// opaque `Model::forward`. This module re-implements the Mamba2 forward with
// repo-OWNED `candle_nn::Linear` projections + an explicit residual-stream seam,
// exactly mirroring the InstrumentedLlama pattern (transformer.rs +
// instrumented_llama.rs):
//   - LoRA: after each owned in_proj/out_proj forward we call
//     `CandleLoraStack::apply_to_linear_output(...)` (the same PEFT delta engine
//     the transformer path uses).
//   - Steering: after each layer block we call
//     `CandleSteeringHooks::apply_record_and_capture_tensor(layer, ResidStream,
//     ...)` — the SSM "residual stream" is the per-token layer-block output
//     (see `ssm_hook_site_for(Mamba2, ResidStream) = mamba2.layer_block.output`).
//
// Numerical fidelity is pinned by `inf9_ssm_forward_parity_tests`, which builds
// BOTH this owned model and the upstream candle `Model` from the SAME VarMap and
// asserts step-by-step logit parity — no downloaded model required.
//
// The mutable SSM inference state reuses the upstream `candle_transformers`
// `State` (its fields are public), so the MT-088 snapshot/restore logic is
// preserved unchanged.

use std::{fs, path::Path};

use candle_core::{DType, Device, IndexOp, Module, Tensor, D};
use candle_nn::{Linear, RmsNorm, VarBuilder};
use candle_transformers::models::mamba2::{Config, State};
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

const D_CONV: usize = 4;

// ---------------------------------------------------------------------------
// Owned Mamba2 mixer block (re-implements candle_transformers Mamba2Block but
// with repo-owned, hook-/LoRA-instrumentable Linears).
// ---------------------------------------------------------------------------

struct OwnedMamba2Mixer {
    in_proj: Linear,
    conv1d_weight: Tensor,
    conv1d_bias: Tensor,
    a_log: Tensor,
    d: Tensor,
    dt_bias: Tensor,
    out_proj: Linear,
    norm: RmsNorm,
    in_proj_target: String,
    out_proj_target: String,
    d_inner: usize,
    d_state: usize,
    d_xbc: usize,
    headdim: usize,
    nheads: usize,
    ngroups: usize,
    layer_idx: usize,
}

impl OwnedMamba2Mixer {
    fn new(layer_idx: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let d_inner = cfg.d_model * cfg.expand;
        let nheads = d_inner / cfg.headdim;
        let ngroups = cfg.ngroups;
        let d_state = cfg.d_state;
        let d_xbc = d_inner + 2 * ngroups * d_state;

        let proj_size = d_inner + d_xbc + nheads;
        let in_proj = candle_nn::linear_no_bias(cfg.d_model, proj_size, vb.pp("in_proj"))
            .map_err(load_err)?;

        let conv1d_weight = vb
            .get((d_xbc, 1, D_CONV), "conv1d.weight")
            .map_err(load_err)?;
        let conv1d_bias = vb.get(d_xbc, "conv1d.bias").map_err(load_err)?;

        let a_log = vb.get(nheads, "A_log").map_err(load_err)?;
        let d = vb.get(nheads, "D").map_err(load_err)?;
        let dt_bias = vb.get(nheads, "dt_bias").map_err(load_err)?;

        let out_proj =
            candle_nn::linear_no_bias(d_inner, cfg.d_model, vb.pp("out_proj")).map_err(load_err)?;
        let norm = candle_nn::rms_norm(d_inner, 1e-5, vb.pp("norm")).map_err(load_err)?;

        Ok(Self {
            in_proj,
            conv1d_weight,
            conv1d_bias,
            a_log,
            d,
            dt_bias,
            out_proj,
            norm,
            in_proj_target: mamba2_target(layer_idx, "in_proj"),
            out_proj_target: mamba2_target(layer_idx, "out_proj"),
            d_inner,
            d_state,
            d_xbc,
            headdim: cfg.headdim,
            nheads,
            ngroups,
            layer_idx,
        })
    }

    /// Single-token mixer forward (mirrors candle Mamba2Block::forward), with
    /// the owned in_proj/out_proj outputs routed through the LoRA delta engine.
    fn forward_step(
        &self,
        xs: &Tensor,
        state: &mut State,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let (b_sz, _dim) = xs.dims2().map_err(gen_err)?;

        let proj = self.in_proj.forward(xs).map_err(gen_err)?;
        let proj =
            lora_stack.apply_to_linear_output(&self.in_proj_target, &proj, xs, lora_overrides)?;

        let z = proj.narrow(D::Minus1, 0, self.d_inner).map_err(gen_err)?;
        let xbc = proj
            .narrow(D::Minus1, self.d_inner, self.d_xbc)
            .map_err(gen_err)?;
        let dt = proj
            .narrow(D::Minus1, self.d_inner + self.d_xbc, self.nheads)
            .map_err(gen_err)?;

        let xbc_conv = self.apply_conv1d(&xbc, &mut state.conv_states[self.layer_idx])?;
        let xbc_conv = candle_nn::ops::silu(&xbc_conv).map_err(gen_err)?;

        let x_conv = xbc_conv.narrow(D::Minus1, 0, self.d_inner).map_err(gen_err)?;
        let b = xbc_conv
            .narrow(D::Minus1, self.d_inner, self.ngroups * self.d_state)
            .map_err(gen_err)?;
        let c = xbc_conv
            .narrow(
                D::Minus1,
                self.d_inner + self.ngroups * self.d_state,
                self.ngroups * self.d_state,
            )
            .map_err(gen_err)?;

        let dt_bias = self.dt_bias.broadcast_as(dt.shape()).map_err(gen_err)?;
        let dt = (&dt + &dt_bias)
            .and_then(|t| t.exp())
            .and_then(|t| t + 1.)
            .and_then(|t| t.log())
            .map_err(gen_err)?; // softplus

        let a = self.a_log.exp().and_then(|t| t.neg()).map_err(gen_err)?;

        let y = self.ssm_step(&x_conv, &a, &b, &c, &dt, state)?;

        let d = self.d.broadcast_as((b_sz, self.nheads)).map_err(gen_err)?;
        let x_skip = x_conv
            .reshape((b_sz, self.nheads, self.headdim))
            .map_err(gen_err)?;
        let y = (&y
            + x_skip
                .broadcast_mul(&d.unsqueeze(D::Minus1).map_err(gen_err)?)
                .map_err(gen_err)?)
        .map_err(gen_err)?;
        let y = y.reshape((b_sz, self.d_inner)).map_err(gen_err)?;

        let y = (y * candle_nn::ops::silu(&z).map_err(gen_err)?).map_err(gen_err)?;
        let y = self.norm.forward(&y).map_err(gen_err)?;

        let out = self.out_proj.forward(&y).map_err(gen_err)?;
        lora_stack.apply_to_linear_output(&self.out_proj_target, &out, &y, lora_overrides)
    }

    fn apply_conv1d(
        &self,
        xbc: &Tensor,
        conv_state: &mut Tensor,
    ) -> Result<Tensor, ModelRuntimeError> {
        let (b_sz, d_xbc) = xbc.dims2().map_err(gen_err)?;

        let shifted = conv_state
            .narrow(D::Minus1, 1, D_CONV - 1)
            .map_err(gen_err)?;
        let xbc_expanded = xbc.unsqueeze(D::Minus1).map_err(gen_err)?;
        *conv_state = Tensor::cat(&[shifted, xbc_expanded], D::Minus1).map_err(gen_err)?;

        let mut result = self
            .conv1d_bias
            .broadcast_as((b_sz, d_xbc))
            .map_err(gen_err)?;
        for i in 0..D_CONV {
            let w = self.conv1d_weight.i((.., 0, i)).map_err(gen_err)?;
            let xbc_i = conv_state.i((.., .., i)).map_err(gen_err)?;
            result = (result + w.broadcast_mul(&xbc_i).map_err(gen_err)?).map_err(gen_err)?;
        }
        Ok(result)
    }

    fn ssm_step(
        &self,
        x: &Tensor,
        a: &Tensor,
        b: &Tensor,
        c: &Tensor,
        dt: &Tensor,
        state: &mut State,
    ) -> Result<Tensor, ModelRuntimeError> {
        let (b_sz, _) = x.dims2().map_err(gen_err)?;
        let h = &mut state.hs[self.layer_idx];

        let x = x
            .reshape((b_sz, self.nheads, self.headdim))
            .map_err(gen_err)?;

        let b = b
            .reshape((b_sz, self.ngroups, self.d_state))
            .map_err(gen_err)?;
        let c = c
            .reshape((b_sz, self.ngroups, self.d_state))
            .map_err(gen_err)?;
        let heads_per_group = self.nheads / self.ngroups;
        let b = b
            .unsqueeze(2)
            .and_then(|t| {
                t.broadcast_as((b_sz, self.ngroups, heads_per_group, self.d_state))
            })
            .and_then(|t| t.reshape((b_sz, self.nheads, self.d_state)))
            .map_err(gen_err)?;
        let c = c
            .unsqueeze(2)
            .and_then(|t| {
                t.broadcast_as((b_sz, self.ngroups, heads_per_group, self.d_state))
            })
            .and_then(|t| t.reshape((b_sz, self.nheads, self.d_state)))
            .map_err(gen_err)?;

        let dt_a = dt.broadcast_mul(a).map_err(gen_err)?;
        let decay = dt_a
            .exp()
            .and_then(|t| t.unsqueeze(D::Minus1))
            .and_then(|t| t.unsqueeze(D::Minus1))
            .and_then(|t| {
                t.broadcast_as((b_sz, self.nheads, self.headdim, self.d_state))
            })
            .map_err(gen_err)?;

        let x_unsq = x.unsqueeze(D::Minus1).map_err(gen_err)?;
        let b_unsq = b.unsqueeze(2).map_err(gen_err)?;
        let x_b = x_unsq.broadcast_mul(&b_unsq).map_err(gen_err)?;

        let dt_expanded = dt
            .unsqueeze(D::Minus1)
            .and_then(|t| t.unsqueeze(D::Minus1))
            .and_then(|t| {
                t.broadcast_as((b_sz, self.nheads, self.headdim, self.d_state))
            })
            .map_err(gen_err)?;

        // SSM recurrence: h = exp(A*dt) * h + dt * (x ⊗ B)
        *h = ((&*h * &decay).map_err(gen_err)?
            + (&dt_expanded * &x_b).map_err(gen_err)?)
        .map_err(gen_err)?;

        let c_unsq = c.unsqueeze(2).map_err(gen_err)?;
        let c_broadcast = c_unsq.broadcast_as(h.shape()).map_err(gen_err)?;
        (&*h * &c_broadcast)
            .and_then(|t| t.sum(D::Minus1))
            .map_err(gen_err)
    }
}

struct OwnedResidualBlock {
    mixer: OwnedMamba2Mixer,
    norm: RmsNorm,
}

impl OwnedResidualBlock {
    fn new(layer_idx: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let norm = candle_nn::rms_norm(cfg.d_model, 1e-5, vb.pp("norm")).map_err(load_err)?;
        let mixer = OwnedMamba2Mixer::new(layer_idx, cfg, vb.pp("mixer"))?;
        Ok(Self { mixer, norm })
    }

    fn forward_step(
        &self,
        xs: &Tensor,
        state: &mut State,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let normed = self.norm.forward(xs).map_err(gen_err)?;
        let mixed = self
            .mixer
            .forward_step(&normed, state, lora_stack, lora_overrides)?;
        (mixed + xs).map_err(gen_err)
    }
}

/// Repo-owned Mamba2 model: embedding -> N residual blocks -> norm_f -> lm_head.
/// Exposes a residual-stream hook seam + LoRA-instrumentable projections.
pub struct InstrumentedMamba2 {
    embedding: candle_nn::Embedding,
    layers: Vec<OwnedResidualBlock>,
    norm_f: RmsNorm,
    lm_head: Linear,
}

impl InstrumentedMamba2 {
    pub fn load(cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let padded_vocab = {
            let pad = cfg.pad_vocab_size_multiple;
            cfg.vocab_size.div_ceil(pad) * pad
        };
        let embedding = candle_nn::embedding(padded_vocab, cfg.d_model, vb.pp("embeddings"))
            .map_err(load_err)?;
        let mut layers = Vec::with_capacity(cfg.n_layer);
        let vb_l = vb.pp("layers");
        for layer_idx in 0..cfg.n_layer {
            layers.push(OwnedResidualBlock::new(layer_idx, cfg, vb_l.pp(layer_idx))?);
        }
        let norm_f = candle_nn::rms_norm(cfg.d_model, 1e-5, vb.pp("norm_f")).map_err(load_err)?;
        let lm_head = Linear::new(embedding.embeddings().clone(), None);
        Ok(Self {
            embedding,
            layers,
            norm_f,
            lm_head,
        })
    }

    /// Single-token forward producing `[b_sz, padded_vocab]` logits, applying
    /// the residual-stream steering hook after each layer block (per-token
    /// semantics) and the LoRA delta after each owned projection.
    pub fn forward_step(
        &self,
        input_ids: &Tensor,
        state: &mut State,
        hooks: &CandleSteeringHooks,
        snapshot: &[crate::model_runtime::SteeringVector],
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let mut xs = self.embedding.forward(input_ids).map_err(gen_err)?;
        for (idx, layer) in self.layers.iter().enumerate() {
            xs = layer.forward_step(&xs, state, lora_stack, lora_overrides)?;
            let li = LayerIndex::new(idx as u32);
            xs = hooks.apply_record_and_capture_tensor(li, HookPoint::ResidStream, &xs, snapshot)?;
        }
        state.pos += 1;
        xs.apply(&self.norm_f)
            .and_then(|t| t.apply(&self.lm_head))
            .map_err(gen_err)
    }

    pub fn n_layers(&self) -> usize {
        self.layers.len()
    }
}

// ---------------------------------------------------------------------------
// CandleMamba2Model: TransformerModel adapter over the owned forward.
// ---------------------------------------------------------------------------

pub struct CandleMamba2Model {
    model: InstrumentedMamba2,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    dtype: DType,
    device: Device,
    lora_stack: CandleLoraStack,
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
        model_id: ModelId,
        config: Config,
        eos_token_ids: Vec<u32>,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let dtype = vb.dtype();
        let model = InstrumentedMamba2::load(&config, vb)?;
        let state = State::new(1, &config, dtype, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle Mamba2 state: {error}"
            ))
        })?;
        let lora_stack = CandleLoraStack::new_for_device(
            model_id,
            "candle-mamba2",
            CandleLoraStack::available_mamba2_targets(config.n_layer),
            device.clone(),
        );
        Ok(Self {
            model,
            state,
            config,
            eos_token_ids,
            dtype,
            device: device.clone(),
            lora_stack,
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
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        self.validate_lora_overrides(lora_overrides)?;
        let snapshot = hooks.snapshot_vectors_for_request(steering_overrides)?;
        match input_ids.dims() {
            [1, seq_len] if *seq_len > 1 => {
                // Token-by-token prefill: the owned SSM recurrence is processed
                // sequentially (numerically the chunked-SSD equivalent), so the
                // per-token steering hook fires per token per layer.
                let mut last = None;
                for pos in 0..*seq_len {
                    let token = input_ids.i((.., pos)).map_err(gen_err)?;
                    let logits = self.model.forward_step(
                        &token,
                        &mut self.state,
                        hooks,
                        &snapshot,
                        &self.lora_stack,
                        lora_overrides,
                    )?;
                    last = Some(logits);
                }
                let logits = last.expect("seq_len > 1 yields at least one step");
                let logits = self.trim_padded_vocab(logits)?;
                logits.i(0).map_err(gen_err)
            }
            [1, 1] => {
                let token = input_ids.squeeze(0).map_err(gen_err)?;
                let logits = self.model.forward_step(
                    &token,
                    &mut self.state,
                    hooks,
                    &snapshot,
                    &self.lora_stack,
                    lora_overrides,
                )?;
                self.trim_padded_vocab(logits)
            }
            [seq_len] if *seq_len == 1 => {
                let logits = self.model.forward_step(
                    input_ids,
                    &mut self.state,
                    hooks,
                    &snapshot,
                    &self.lora_stack,
                    lora_overrides,
                )?;
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

    fn lora_stack(&self) -> LoraStackHandle {
        self.lora_stack.handle()
    }

    fn validate_lora_overrides(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        self.lora_stack.ensure_overrides_mounted(ids)
    }

    // CRIT-1 / MT-088: pack conv1d state + SSM hs into the typed snapshot.
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

/// Per-layer LoRA target name for a Mamba2 projection module. candle's Mamba2
/// fuses x_proj/dt_proj/z into a single `in_proj`, so the realisable LoRA
/// targets for this implementation are `in_proj` (the fused input projection)
/// and `out_proj`.
pub fn mamba2_target(layer_idx: usize, module: &str) -> String {
    format!("backbone.layers.{layer_idx}.mixer.{module}")
}

fn load_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!("Candle instrumented Mamba2 load failed: {error}"))
}

fn gen_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::GenerateError(format!("Candle instrumented Mamba2 forward failed: {error}"))
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

#[cfg(test)]
mod tests {
    //! MT-115 / MT-116 (INF-9) load-bearing proofs for the OWNED Mamba2 forward:
    //!   - `owned_mamba2_forward_matches_candle_transformers_step_by_step`:
    //!     builds BOTH the upstream candle Mamba2 `Model` and our owned
    //!     `InstrumentedMamba2` from the SAME random-weight VarMap and asserts
    //!     step-by-step logit parity (no downloaded model). This proves the
    //!     owned forward is numerically faithful — the prerequisite for trusting
    //!     the LoRA/steering seams.
    //!   - `owned_mamba2_lora_mount_diverges_then_unmount_reverts`: a fixture
    //!     LoRA mounted on layer-0 in_proj actually changes generated logits,
    //!     and unmount restores them (MT-115 real PEFT mount/unmount).
    //!   - `owned_mamba2_steering_zero_is_identity_random_diverges`: a zero
    //!     steering vector is an identity (MT-116 identity-test correctness
    //!     gate); a non-zero vector changes the logits.
    use super::*;
    use std::collections::HashMap;

    use candle_core::Tensor;
    use candle_transformers::models::mamba2::Model as CandleMamba2;

    use crate::model_runtime::{
        BaseModelTag, HookPoint as HP, LayerIndex as LI, LicenseTag, LoraDescriptor, LoraId,
        LoraStackOps, LoraStrength, SteeringProvenance, SteeringVector, SteeringVectorValues,
    };

    fn tiny_config() -> Config {
        serde_json::from_value(serde_json::json!({
            "d_model": 16,
            "n_layer": 2,
            "vocab_size": 20,
            "d_state": 8,
            "expand": 2,
            "headdim": 4,
            "ngroups": 1,
            "pad_vocab_size_multiple": 16
        }))
        .expect("tiny Mamba2 config")
    }

    struct Dims {
        d_model: usize,
        d_inner: usize,
        nheads: usize,
        d_xbc: usize,
        proj_size: usize,
        padded_vocab: usize,
    }

    fn dims(cfg: &Config) -> Dims {
        let d_model = cfg.d_model;
        let d_inner = d_model * cfg.expand;
        let nheads = d_inner / cfg.headdim;
        let d_xbc = d_inner + 2 * cfg.ngroups * cfg.d_state;
        let proj_size = d_inner + d_xbc + nheads;
        let pad = cfg.pad_vocab_size_multiple;
        let padded_vocab = cfg.vocab_size.div_ceil(pad) * pad;
        Dims {
            d_model,
            d_inner,
            nheads,
            d_xbc,
            proj_size,
            padded_vocab,
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
        put("embeddings.weight".to_string(), &[d.padded_vocab, d.d_model]);
        put("norm_f.weight".to_string(), &[d.d_model]);
        for i in 0..cfg.n_layer {
            put(format!("layers.{i}.norm.weight"), &[d.d_model]);
            put(
                format!("layers.{i}.mixer.in_proj.weight"),
                &[d.proj_size, d.d_model],
            );
            put(
                format!("layers.{i}.mixer.conv1d.weight"),
                &[d.d_xbc, 1, D_CONV],
            );
            put(format!("layers.{i}.mixer.conv1d.bias"), &[d.d_xbc]);
            put(format!("layers.{i}.mixer.A_log"), &[d.nheads]);
            put(format!("layers.{i}.mixer.D"), &[d.nheads]);
            put(format!("layers.{i}.mixer.dt_bias"), &[d.nheads]);
            put(
                format!("layers.{i}.mixer.out_proj.weight"),
                &[d.d_model, d.d_inner],
            );
            put(format!("layers.{i}.mixer.norm.weight"), &[d.d_inner]);
        }
        m
    }

    fn token(id: u32, device: &Device) -> Tensor {
        Tensor::from_vec(vec![id], 1, device).unwrap()
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
    fn owned_mamba2_forward_matches_candle_transformers_step_by_step() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let weights = random_weights(&cfg, &device);

        let candle_model =
            CandleMamba2::new(&cfg, VarBuilder::from_tensors(weights.clone(), DType::F32, &device))
                .expect("candle mamba2 builds");
        let mut candle_state = State::new(1, &cfg, DType::F32, &device).unwrap();

        let owned =
            InstrumentedMamba2::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned mamba2 builds");
        let mut owned_state = State::new(1, &cfg, DType::F32, &device).unwrap();

        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.d_model);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-mamba2",
            CandleLoraStack::available_mamba2_targets(cfg.n_layer),
        );

        // Drive both models token-by-token through their single-token forward;
        // the owned recurrence must match candle's ssm_step at every position.
        for &tid in &[3u32, 7, 1, 5, 2, 9] {
            let tok = token(tid, &device);
            let candle_logits = candle_model.forward(&tok, &mut candle_state).unwrap();
            let owned_logits = owned
                .forward_step(&tok, &mut owned_state, &hooks, &[], &lora, &[])
                .unwrap();
            let c = logits_vec(&candle_logits);
            let o = logits_vec(&owned_logits);
            assert_eq!(c.len(), o.len(), "logit width mismatch at token {tid}");
            let diff = max_abs_diff(&c, &o);
            assert!(
                diff < 1e-4,
                "owned Mamba2 diverged from candle at token {tid}: max |Δlogit| = {diff}"
            );
        }
    }

    #[tokio::test]
    async fn owned_mamba2_lora_mount_diverges_then_unmount_reverts() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedMamba2::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned mamba2 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.d_model);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-mamba2",
            CandleLoraStack::available_mamba2_targets(cfg.n_layer),
        );

        let baseline = {
            let mut state = State::new(1, &cfg, DType::F32, &device).unwrap();
            logits_vec(
                &owned
                    .forward_step(&token(4, &device), &mut state, &hooks, &[], &lora, &[])
                    .unwrap(),
            )
        };

        // Fixture LoRA on layer-0 in_proj (rank 2): A [rank, d_model], B [proj, rank].
        let rank = 2usize;
        let target = mamba2_target(0, "in_proj");
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("ssm_lora.safetensors");
        let mut tensors = HashMap::new();
        tensors.insert(
            format!("{target}.lora_A.weight"),
            Tensor::randn(0f32, 0.5f32, vec![rank, d.d_model], &device).unwrap(),
        );
        tensors.insert(
            format!("{target}.lora_B.weight"),
            Tensor::randn(0f32, 0.5f32, vec![d.proj_size, rank], &device).unwrap(),
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
            base_model_compat: BaseModelTag::new("candle-mamba2"),
            license_tag: LicenseTag::new("test-license"),
        };
        lora
            .mount(descriptor, LoraStrength::try_new(1.0).unwrap())
            .await
            .expect("SSM LoRA mounts");

        let mounted = {
            let mut state = State::new(1, &cfg, DType::F32, &device).unwrap();
            logits_vec(
                &owned
                    .forward_step(&token(4, &device), &mut state, &hooks, &[], &lora, &[lora_id])
                    .unwrap(),
            )
        };
        assert!(
            max_abs_diff(&baseline, &mounted) > 1e-4,
            "mounted SSM LoRA must change in_proj output and therefore the logits"
        );

        lora.unmount(lora_id).await.expect("unmount");
        let reverted = {
            let mut state = State::new(1, &cfg, DType::F32, &device).unwrap();
            logits_vec(
                &owned
                    .forward_step(&token(4, &device), &mut state, &hooks, &[], &lora, &[])
                    .unwrap(),
            )
        };
        assert!(
            max_abs_diff(&baseline, &reverted) < 1e-6,
            "unmounting the SSM LoRA must revert the logits to baseline"
        );
    }

    #[tokio::test]
    async fn owned_mamba2_steering_zero_is_identity_random_diverges() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedMamba2::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned mamba2 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.d_model);
        let lora = CandleLoraStack::new(ModelId::new_v7(), "candle-mamba2", Vec::new());

        let run = |snapshot: &[SteeringVector]| {
            let mut state = State::new(1, &cfg, DType::F32, &device).unwrap();
            logits_vec(
                &owned
                    .forward_step(&token(6, &device), &mut state, &hooks, snapshot, &lora, &[])
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
            SteeringVectorValues::try_new(vec![0.0_f32; d.d_model], 1.0).unwrap(),
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
            "a zero steering vector must be an identity on the SSM residual stream"
        );

        // Non-zero vector at layer 0 -> logits must change.
        let mut direction = vec![0.0_f32; d.d_model];
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
            "a non-zero steering vector must change the SSM logits"
        );
    }
}
