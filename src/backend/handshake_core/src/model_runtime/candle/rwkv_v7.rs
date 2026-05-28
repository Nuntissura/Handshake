#![cfg(feature = "candle-runtime-engine")]

// MT-115 / MT-116 (INF-9 full feature parity): Handshake-OWNED RWKV v7 "Goose"
// forward.
//
// Adapted from candle-transformers 0.10.2 `src/models/rwkv_v7.rs`. Upstream
// keeps the RWKV v7 `Block` / `TimeMix` / `ChannelMix` forwards + their
// projection weights private (and `Model::forward` operates on opaque pre-
// transposed raw tensors), so neither a LoRA delta nor an activation-steering
// vector can be threaded through `Model::forward`. This module re-implements
// the RWKV v7 forward with repo-OWNED `candle_nn::Linear` projections + an
// explicit residual-stream seam, exactly mirroring the owned RWKV v5 / v6
// implementations (rwkv_v5.rs, rwkv_v6.rs) and the owned Mamba2 implementation
// (mamba2.rs):
//   - LoRA: after each owned time-mix / channel-mix Linear forward we call
//     `CandleLoraStack::apply_to_linear_output(...)` (the same PEFT delta
//     engine the transformer + Mamba2 + RWKV v5/v6 paths use). v7's GENUINE
//     full Linear projections are time-mix receptance/key/value/output (each
//     `[C, C]`) and channel-mix key/value (`[dim_ffn, C]` / `[C, dim_ffn]`).
//     The decay / ICL-rate / value-residual / gate parameters are LOW-RANK
//     LoRA-STYLE interpolation tensors (`w0/w1/w2`, `a0/a1/a2`, `v0/v1/v2`,
//     `g1/g2`) and the per-head key/bonus tensors (`k_k`, `k_a`, `r_k`) — these
//     are loaded as RAW tensors for parity and are NOT PEFT LoRA targets (same
//     treatment as Mamba2's A/D/conv and the RWKV v5/v6 time_mix_* / decay).
//   - Steering: after each layer block we call
//     `CandleSteeringHooks::apply_record_and_capture_tensor(layer, ResidStream,
//     ...)` — the RWKV "residual stream" is the per-token layer-block output
//     (see `ssm_hook_site_for(RwkvV7, ResidStream) = rwkv.layer_block.output`).
//
// Numerical fidelity is pinned by the `tests` module, which builds BOTH this
// owned model and the upstream candle `Model` from the SAME VarMap and asserts
// step-by-step logit parity (no downloaded model required).
//
// RECURRENCE SUBTLETY (documented per MT contract): RWKV v7 replaces the v5/v6
// headwise WKV recurrence with the GENERALIZED DELTA RULE. Per token, per head:
//
//   vk    = v ⊗ k                      (outer product, [N,N])
//   ab    = (-kk) ⊗ (kk * a)           (ICL in-context-learning correction)
//   state = state * w + state @ ab + vk
//   out   = state @ r
//
// where:
//   * `w = exp(-0.606531 * sigmoid(w0 + tanh(xw @ w1) @ w2))` is a per-channel
//     data-dependent decay (note: the decay is applied as a per-COLUMN scale
//     `w.view(H,1,N)`, NOT a per-row scale like v5/v6's `time_decay`).
//   * `kk = L2_normalize_per_head(k * k_k)` and `a = sigmoid(a0 + (xa@a1)@a2)`
//     drive the delta-rule "remove-then-add" correction `state @ ab`. This is
//     the v7 in-context-learning term that has no v5/v6 analogue — it lets the
//     state literally subtract a rank-1 component of itself before writing the
//     new key/value, which is why the WKV state is a full `[N,N]` matrix
//     accumulated in F32.
//   * a per-layer VALUE-RESIDUAL stream: layer 0 emits `v_first = v`; layers
//     >0 blend their own value toward `v_first` through a sigmoid gate
//     (`v0/v1/v2`). `v_first` is threaded across blocks exactly like upstream.
//   * the bonus term `(r * k * r_k).sum_per_head * v` is added AFTER the per-
//     head GroupNorm (manual, H groups, eps=64e-5 — NOT 1e-5 like v5/v6).
//
// The owned forward updates `att_x_prev` / `ffn_x_prev` to the CURRENT token's
// hidden state in the exact same order as upstream (after computing the token-
// shift mixes, before the recurrence), so the recurrence is bit-faithful. The
// owned forward reuses candle's RWKV v7 `State` verbatim, so the MT-088
// snapshot/restore logic (DEA-less two-bucket layout) is preserved unchanged.
//
// SCOPE: the owned forward implements the V7 BASE variant. V7a (DeepEmbed FFN
// gating) and V7b (Deep Embedding Attention) carry token-dependent merged-
// embedding machinery (and, for V7b, a non-tensor growing KV cache the MT-088
// snapshot format cannot represent). The owned model therefore loads only V7
// base and returns a typed `CapabilityNotSupported` for V7a/V7b instead of a
// partial / unfaithful replication; the upstream opaque path remains available
// for those variants if a later MT needs them.

use std::{fs, path::Path};

use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_nn::{Embedding, LayerNorm, Linear, VarBuilder};
use candle_transformers::models::rwkv_v7::{Config, ModelVersion, State};
use serde_json::Value;

use super::{
    hooks::CandleSteeringHooks,
    lora_impl::CandleLoraStack,
    rwkv_v5,
    ssm_state::{snapshot_to_tensor, tensor_to_snapshot},
    state_vector::SSMStateSnapshot,
    transformer::TransformerModel,
};
use crate::model_runtime::{
    HookPoint, LayerIndex, LoraId, LoraStackHandle, ModelId, ModelRuntimeError, SteeringVectorId,
};

// ---------------------------------------------------------------------------
// LoRA dimension inference (mirrors upstream `infer_lora_dims`): the low-rank
// decay / ICL / value-residual / gate tensors have model-specific inner
// dimensions read from the actual weight shapes in block 0.
// ---------------------------------------------------------------------------

fn infer_lora_dims(vb: &VarBuilder) -> Result<(usize, usize, usize, usize), ModelRuntimeError> {
    let att = vb.pp("blocks").pp(0).pp("att");
    let d_decay = att
        .get_unchecked("w1")
        .and_then(|t| t.dim(1))
        .map_err(load_err)?;
    let d_aaa = att
        .get_unchecked("a1")
        .and_then(|t| t.dim(1))
        .map_err(load_err)?;
    let d_mv = att
        .get_unchecked("v1")
        .and_then(|t| t.dim(1))
        .map_err(load_err)?;
    let d_gate = att
        .get_unchecked("g1")
        .and_then(|t| t.dim(1))
        .map_err(load_err)?;
    Ok((d_decay, d_aaa, d_mv, d_gate))
}

// ---------------------------------------------------------------------------
// Owned RWKV v7 time-mix block (re-implements candle v7 `TimeMix` but with
// repo-owned, LoRA-instrumentable `candle_nn::Linear` projections).
//
// Operates on 1D `[C]` per-token vectors exactly like upstream. The four
// GENUINE Linears (receptance/key/value/output) are owned; all other tensors
// are loaded raw to preserve the delta-rule math bit-for-bit.
// ---------------------------------------------------------------------------

struct OwnedTimeMix {
    // Genuine full Linear projections (the real LoRA targets).
    receptance: Linear,
    key: Linear,
    value: Linear,
    output: Linear,
    // Token-shift lerp mixes (raw, not LoRA targets).
    x_r: Tensor,
    x_w: Tensor,
    x_k: Tensor,
    x_v: Tensor,
    x_a: Tensor,
    x_g: Tensor,
    // Decay LoRA-style low-rank (raw).
    w0: Tensor,
    w1: Tensor,
    w2: Tensor,
    // ICL rate LoRA-style low-rank (raw).
    a0: Tensor,
    a1: Tensor,
    a2: Tensor,
    // Value residual LoRA-style low-rank (raw, None for layer 0).
    v0: Option<Tensor>,
    v1: Option<Tensor>,
    v2: Option<Tensor>,
    // Gate LoRA-style low-rank (raw).
    g1: Tensor,
    g2: Tensor,
    // Per-head key processing + bonus (raw).
    k_k: Tensor,
    k_a: Tensor,
    r_k: Tensor,
    // Manual GroupNorm weights (raw; eps differs from v5/v6).
    ln_x_weight: Tensor,
    ln_x_bias: Tensor,
    // LoRA target names for the four owned projections.
    receptance_target: String,
    key_target: String,
    value_target: String,
    output_target: String,
    layer_id: usize,
    n_heads: usize,
    head_size: usize,
}

impl OwnedTimeMix {
    fn new(
        layer_id: usize,
        cfg: &Config,
        lora: (usize, usize, usize, usize),
        vb: VarBuilder,
    ) -> Result<Self, ModelRuntimeError> {
        let c = cfg.hidden_size;
        let (d_decay, d_aaa, d_mv, d_gate) = lora;
        let n_heads = c / cfg.head_size;
        let head_size = cfg.head_size;

        // Genuine full [C, C] Linear projections — these own the PEFT LoRA seam.
        let receptance = candle_nn::linear_no_bias(c, c, vb.pp("receptance")).map_err(load_err)?;
        let key = candle_nn::linear_no_bias(c, c, vb.pp("key")).map_err(load_err)?;
        let value = candle_nn::linear_no_bias(c, c, vb.pp("value")).map_err(load_err)?;
        let output = candle_nn::linear_no_bias(c, c, vb.pp("output")).map_err(load_err)?;

        // Token-shift lerp mixes; pre-squeeze (1,1,C) -> (C,) like upstream.
        let x_r = squeeze_mix(&vb, "x_r", c)?;
        let x_w = squeeze_mix(&vb, "x_w", c)?;
        let x_k = squeeze_mix(&vb, "x_k", c)?;
        let x_v = squeeze_mix(&vb, "x_v", c)?;
        let x_a = squeeze_mix(&vb, "x_a", c)?;
        let x_g = squeeze_mix(&vb, "x_g", c)?;

        let w0 = squeeze_mix(&vb, "w0", c)?;
        let w1 = vb.get((c, d_decay), "w1").map_err(load_err)?;
        let w2 = vb.get((d_decay, c), "w2").map_err(load_err)?;

        let a0 = squeeze_mix(&vb, "a0", c)?;
        let a1 = vb.get((c, d_aaa), "a1").map_err(load_err)?;
        let a2 = vb.get((d_aaa, c), "a2").map_err(load_err)?;

        // v0/v1/v2 exist in the weights file for every layer but are only used
        // for layers > 0 (layer 0 emits v_first instead of blending toward it).
        let (v0, v1, v2) = if layer_id > 0 {
            (
                Some(squeeze_mix(&vb, "v0", c)?),
                Some(vb.get((c, d_mv), "v1").map_err(load_err)?),
                Some(vb.get((d_mv, c), "v2").map_err(load_err)?),
            )
        } else {
            // Load-and-discard to keep VarBuilder consumption aligned with
            // upstream (the tensors exist but are ignored at layer 0).
            let _ = vb.get((1, 1, c), "v0");
            let _ = vb.get((c, d_mv), "v1");
            let _ = vb.get((d_mv, c), "v2");
            (None, None, None)
        };

        let g1 = vb.get((c, d_gate), "g1").map_err(load_err)?;
        let g2 = vb.get((d_gate, c), "g2").map_err(load_err)?;

        let k_k = squeeze_mix(&vb, "k_k", c)?;
        let k_a = squeeze_mix(&vb, "k_a", c)?;
        // Pre-flatten r_k to (H*N,) like upstream.
        let r_k = vb
            .get((n_heads, head_size), "r_k")
            .and_then(|t| t.reshape(n_heads * head_size))
            .map_err(load_err)?;

        let ln_x_weight = vb.get(c, "ln_x.weight").map_err(load_err)?;
        let ln_x_bias = vb.get(c, "ln_x.bias").map_err(load_err)?;

        Ok(Self {
            receptance,
            key,
            value,
            output,
            x_r,
            x_w,
            x_k,
            x_v,
            x_a,
            x_g,
            w0,
            w1,
            w2,
            a0,
            a1,
            a2,
            v0,
            v1,
            v2,
            g1,
            g2,
            k_k,
            k_a,
            r_k,
            ln_x_weight,
            ln_x_bias,
            receptance_target: rwkv_v7_target(layer_id, "time_mix", "receptance"),
            key_target: rwkv_v7_target(layer_id, "time_mix", "key"),
            value_target: rwkv_v7_target(layer_id, "time_mix", "value"),
            output_target: rwkv_v7_target(layer_id, "time_mix", "output"),
            layer_id,
            n_heads,
            head_size,
        })
    }

    /// Single-token time-mix forward (mirrors candle v7 `TimeMix::forward`).
    /// `x` is the post-ln1 hidden vector `[C]`. Returns `(out [C], v_first [C])`.
    /// The four owned projections are routed through the LoRA delta engine.
    fn forward(
        &self,
        x: &Tensor,
        state: &mut candle_transformers::models::rwkv_v7::StatePerLayer,
        v_first: Option<Tensor>,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<(Tensor, Tensor), ModelRuntimeError> {
        let h = self.n_heads;
        let n = self.head_size;

        // 1. Token shift: lerp between current and previous token (xx = prev - x).
        let xx = (&state.att_x_prev - x).map_err(gen_err)?;
        let xr = (x + xx.broadcast_mul(&self.x_r).map_err(gen_err)?).map_err(gen_err)?;
        let xw = (x + xx.broadcast_mul(&self.x_w).map_err(gen_err)?).map_err(gen_err)?;
        let xk = (x + xx.broadcast_mul(&self.x_k).map_err(gen_err)?).map_err(gen_err)?;
        let xv = (x + xx.broadcast_mul(&self.x_v).map_err(gen_err)?).map_err(gen_err)?;
        let xa = (x + xx.broadcast_mul(&self.x_a).map_err(gen_err)?).map_err(gen_err)?;
        let xg = (x + xx.broadcast_mul(&self.x_g).map_err(gen_err)?).map_err(gen_err)?;
        state.att_x_prev = x.clone();

        // 2. Genuine Linear projections + LoRA delta (owned r/k/v).
        let r = self.linear_lora(&self.receptance, &self.receptance_target, &xr, lora_stack, lora_overrides)?;
        let k = self.linear_lora(&self.key, &self.key_target, &xk, lora_stack, lora_overrides)?;
        let v = self.linear_lora(&self.value, &self.value_target, &xv, lora_stack, lora_overrides)?;

        // 3. Decay: w = exp(-0.606531 * sigmoid(w0 + tanh(xw @ w1) @ w2)).
        let w = mm1d(&mm1d(&xw, &self.w1)?.tanh().map_err(gen_err)?, &self.w2)?;
        let w = (&self.w0 + &w)
            .and_then(|t| t.to_dtype(DType::F32))
            .map_err(gen_err)?;
        let w = (w.neg().and_then(|t| t.exp()).map_err(gen_err)? + 1.0)
            .and_then(|t| t.recip())
            .map_err(gen_err)?; // sigmoid
        let w = (w * (-0.606531)).and_then(|t| t.exp()).map_err(gen_err)?;

        // 4. Value residual stream (layer 0 emits v_first; layers > 0 blend).
        let (v, v_first) = if self.layer_id == 0 {
            let v_first = v.clone();
            (v, v_first)
        } else {
            let v_first = v_first.ok_or_else(|| {
                gen_err(candle_core::Error::Msg(
                    "RWKV v7 layer > 0 requires v_first from layer 0".to_string(),
                ))
            })?;
            if let (Some(v0), Some(v1), Some(v2)) = (&self.v0, &self.v1, &self.v2) {
                let gate = candle_nn::ops::sigmoid(
                    &(v0 + mm1d(&mm1d(&xv, v1)?, v2)?).map_err(gen_err)?,
                )
                .map_err(gen_err)?;
                let v = (&v
                    + (&v_first - &v)
                        .and_then(|d| d.broadcast_mul(&gate))
                        .map_err(gen_err)?)
                .map_err(gen_err)?;
                (v, v_first)
            } else {
                (v, v_first)
            }
        };

        // 5. ICL rate: a = sigmoid(a0 + (xa @ a1) @ a2).
        let a = candle_nn::ops::sigmoid(
            &(&self.a0 + mm1d(&mm1d(&xa, &self.a1)?, &self.a2)?).map_err(gen_err)?,
        )
        .map_err(gen_err)?;

        // 6. Gate: g = sigmoid(xg @ g1) @ g2.
        let g = mm1d(
            &candle_nn::ops::sigmoid(&mm1d(&xg, &self.g1)?).map_err(gen_err)?,
            &self.g2,
        )?;

        // 7. Key processing: kk = L2_normalize_per_head(k * k_k);
        //    k = k * (1 + (a - 1) * k_a).
        let kk = (&k * &self.k_k).map_err(gen_err)?;
        let kk = kk.reshape((h, n)).map_err(gen_err)?;
        let kk_norm = (kk
            .sqr()
            .and_then(|t| t.sum_keepdim(1))
            .and_then(|t| t.sqrt())
            .map_err(gen_err)?
            + 1e-12)
            .map_err(gen_err)?;
        let kk = kk.broadcast_div(&kk_norm).map_err(gen_err)?;
        let kk = kk.reshape(h * n).map_err(gen_err)?;

        let k = (&k
            * (1.0
                + (&a - 1.0)
                    .and_then(|d| d.broadcast_mul(&self.k_a))
                    .map_err(gen_err)?)
            .map_err(gen_err)?)
        .map_err(gen_err)?;

        // 8. State update (generalized delta rule).
        // vk = v.view(H,N,1) @ k.view(H,1,N)  — outer product.
        let v_hn = v.reshape((h, n, 1)).map_err(gen_err)?;
        let k_hn = k.reshape((h, 1, n)).map_err(gen_err)?;
        let vk = v_hn.matmul(&k_hn).map_err(gen_err)?;

        // ab = (-kk).view(H,N,1) @ (kk * a).view(H,1,N)  — ICL correction.
        let kk_h = kk.reshape((h, n)).map_err(gen_err)?;
        let a_h = a.reshape((h, n)).map_err(gen_err)?;
        let neg_kk = kk_h.neg().and_then(|t| t.reshape((h, n, 1))).map_err(gen_err)?;
        let kk_a = (&kk_h * &a_h)
            .and_then(|t| t.reshape((h, 1, n)))
            .map_err(gen_err)?;
        let ab = neg_kk.matmul(&kk_a).map_err(gen_err)?;

        // state = state * w.view(H,1,N) + state @ ab + vk   (F32 accumulation).
        let w_h = w.reshape((h, 1, n)).map_err(gen_err)?;
        let att_kv = &state.att_kv;
        let new_state = (att_kv.broadcast_mul(&w_h).map_err(gen_err)?
            + att_kv
                .to_dtype(DType::F32)
                .and_then(|s| s.matmul(&ab.to_dtype(DType::F32)?))
                .map_err(gen_err)?
            + vk.to_dtype(DType::F32).map_err(gen_err)?)
        .map_err(gen_err)?;
        state.att_kv = new_state;

        // out = state @ r.view(H,N,1).
        let r_hn = r.reshape((h, n, 1)).map_err(gen_err)?;
        let out = state
            .att_kv
            .to_dtype(r.dtype())
            .and_then(|s| s.matmul(&r_hn))
            .map_err(gen_err)?;

        // 9. Manual per-head GroupNorm (H groups, eps = 64e-5, NOT 1e-5).
        let out = {
            let reshaped = out.reshape((h, n)).map_err(gen_err)?;
            let mean = reshaped.mean_keepdim(1).map_err(gen_err)?;
            let centered = reshaped.broadcast_sub(&mean).map_err(gen_err)?;
            let var = centered
                .sqr()
                .and_then(|t| t.mean_keepdim(1))
                .map_err(gen_err)?;
            let normed = centered
                .broadcast_div(&(var + 64e-5).and_then(|t| t.sqrt()).map_err(gen_err)?)
                .map_err(gen_err)?;
            normed.reshape(h * n).map_err(gen_err)?
        };
        let out = (out.broadcast_mul(&self.ln_x_weight).map_err(gen_err)? + &self.ln_x_bias)
            .map_err(gen_err)?;

        // 10. Bonus term: (r * k * r_k).sum_per_head * v.
        let bonus = (&r * &k)
            .and_then(|t| &t * &self.r_k)
            .and_then(|t| t.reshape((h, n)))
            .and_then(|t| t.sum_keepdim(1))
            .and_then(|t| t.broadcast_mul(&v.reshape((h, n))?))
            .and_then(|t| t.reshape(h * n))
            .map_err(gen_err)?;
        let out = (out + bonus).map_err(gen_err)?;

        // 11. Output projection (owned Linear + LoRA delta) on (out * g).
        let gated = (out * g).map_err(gen_err)?;
        let out =
            self.linear_lora(&self.output, &self.output_target, &gated, lora_stack, lora_overrides)?;

        Ok((out, v_first))
    }

    /// Owned `candle_nn::Linear` forward on a 1D `[in]` vector followed by the
    /// PEFT LoRA delta, returning a 1D `[out]` vector. The Linear is applied at
    /// `[1, in]` (== upstream's `unsqueeze(0).matmul(w_t).squeeze(0)`), the LoRA
    /// engine requires rank-2 input, and the result is squeezed back to 1D.
    fn linear_lora(
        &self,
        linear: &Linear,
        target: &str,
        input_1d: &Tensor,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let input_2d = input_1d.unsqueeze(0).map_err(gen_err)?;
        let base = linear.forward(&input_2d).map_err(gen_err)?;
        let out = lora_stack.apply_to_linear_output(target, &base, &input_2d, lora_overrides)?;
        out.squeeze(0).map_err(gen_err)
    }
}

// ---------------------------------------------------------------------------
// Owned RWKV v7 channel-mix block (re-implements candle v7 `ChannelMix`,
// V7-base path: squared-ReLU FFN, no DeepEmbed gating).
// ---------------------------------------------------------------------------

struct OwnedChannelMix {
    x_k: Tensor,
    key: Linear,
    value: Linear,
    key_target: String,
    value_target: String,
}

impl OwnedChannelMix {
    fn new(layer_id: usize, cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        let c = cfg.hidden_size;
        let dim_ffn = cfg.intermediate_size.unwrap_or(c * 4);
        let x_k = squeeze_mix(&vb, "x_k", c)?;
        let key = candle_nn::linear_no_bias(c, dim_ffn, vb.pp("key")).map_err(load_err)?;
        let value = candle_nn::linear_no_bias(dim_ffn, c, vb.pp("value")).map_err(load_err)?;
        Ok(Self {
            x_k,
            key,
            value,
            key_target: rwkv_v7_target(layer_id, "channel_mix", "key"),
            value_target: rwkv_v7_target(layer_id, "channel_mix", "value"),
        })
    }

    /// Single-token channel-mix forward (mirrors candle v7 `ChannelMix::forward`
    /// V7-base path). `x` is the post-ln2 hidden vector `[C]`.
    fn forward(
        &self,
        x: &Tensor,
        state: &mut candle_transformers::models::rwkv_v7::StatePerLayer,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        // Token shift (x_k pre-squeezed): k = x + (ffn_x_prev - x) * x_k.
        let xx = (&state.ffn_x_prev - x).map_err(gen_err)?;
        let k = (x + xx.broadcast_mul(&self.x_k).map_err(gen_err)?).map_err(gen_err)?;
        state.ffn_x_prev = x.clone();

        // Squared ReLU: relu(key(k))^2 (owned Linear + LoRA delta).
        let k2 = k.unsqueeze(0).map_err(gen_err)?;
        let key_out = self.key.forward(&k2).map_err(gen_err)?;
        let key_out =
            lora_stack.apply_to_linear_output(&self.key_target, &key_out, &k2, lora_overrides)?;
        let key_out = key_out
            .squeeze(0)
            .and_then(|t| t.relu())
            .and_then(|t| t.sqr())
            .map_err(gen_err)?;

        // Down-projection (owned Linear + LoRA delta).
        let key_out_2d = key_out.unsqueeze(0).map_err(gen_err)?;
        let value_out = self.value.forward(&key_out_2d).map_err(gen_err)?;
        let value_out =
            lora_stack.apply_to_linear_output(&self.value_target, &value_out, &key_out_2d, lora_overrides)?;
        value_out.squeeze(0).map_err(gen_err)
    }
}

// ---------------------------------------------------------------------------
// Owned RWKV v7 block: ln0 (layer 0 pre-norm) -> ln1 -> time-mix (+residual,
// +v_first) -> ln2 -> channel-mix (+residual).
// ---------------------------------------------------------------------------

struct OwnedBlock {
    ln0: Option<LayerNorm>,
    ln1: LayerNorm,
    ln2: LayerNorm,
    time_mix: OwnedTimeMix,
    channel_mix: OwnedChannelMix,
    layer_id: usize,
}

impl OwnedBlock {
    fn new(
        layer_id: usize,
        cfg: &Config,
        lora: (usize, usize, usize, usize),
        vb: VarBuilder,
    ) -> Result<Self, ModelRuntimeError> {
        let c = cfg.hidden_size;
        let ln0 = if layer_id == 0 {
            Some(candle_nn::layer_norm(c, 1e-5, vb.pp("ln0")).map_err(load_err)?)
        } else {
            None
        };
        let ln1 = candle_nn::layer_norm(c, 1e-5, vb.pp("ln1")).map_err(load_err)?;
        let ln2 = candle_nn::layer_norm(c, 1e-5, vb.pp("ln2")).map_err(load_err)?;
        let time_mix = OwnedTimeMix::new(layer_id, cfg, lora, vb.pp("att"))?;
        let channel_mix = OwnedChannelMix::new(layer_id, cfg, vb.pp("ffn"))?;
        Ok(Self {
            ln0,
            ln1,
            ln2,
            time_mix,
            channel_mix,
            layer_id,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        state: &mut State,
        v_first: Option<Tensor>,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<(Tensor, Tensor), ModelRuntimeError> {
        // Pre-norm (block 0 only).
        let x = match self.ln0.as_ref() {
            Some(ln0) => x.apply(ln0).map_err(gen_err)?,
            None => x.clone(),
        };

        // Time mixing (RWKV linear attention / delta rule).
        let x_ln1 = x.apply(&self.ln1).map_err(gen_err)?;
        let (att_out, v_first) = self.time_mix.forward(
            &x_ln1,
            &mut state.per_layer[self.layer_id],
            v_first,
            lora_stack,
            lora_overrides,
        )?;
        let x = (&x + att_out).map_err(gen_err)?;

        // Channel mixing (FFN).
        let x_ln2 = x.apply(&self.ln2).map_err(gen_err)?;
        let ffn_out = self.channel_mix.forward(
            &x_ln2,
            &mut state.per_layer[self.layer_id],
            lora_stack,
            lora_overrides,
        )?;
        let x = (x + ffn_out).map_err(gen_err)?;

        Ok((x, v_first))
    }
}

/// Repo-owned RWKV v7 model (V7-base): embedding -> N blocks -> ln_out -> head.
/// Exposes a residual-stream hook seam + LoRA-instrumentable projections.
pub struct InstrumentedRwkvV7 {
    embeddings: Embedding,
    blocks: Vec<OwnedBlock>,
    ln_out: LayerNorm,
    head: Linear,
}

impl InstrumentedRwkvV7 {
    pub fn load(cfg: &Config, vb: VarBuilder) -> Result<Self, ModelRuntimeError> {
        if cfg.version != ModelVersion::V7 {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "rwkv_v7_variant_owned_forward".to_string(),
                adapter: "candle_rwkv_v7".to_string(),
            });
        }
        let c = cfg.hidden_size;
        let lora = infer_lora_dims(&vb)?;
        let embeddings = candle_nn::embedding(cfg.vocab_size, c, vb.pp("emb")).map_err(load_err)?;
        let mut blocks = Vec::with_capacity(cfg.num_hidden_layers);
        let vb_b = vb.pp("blocks");
        for layer_id in 0..cfg.num_hidden_layers {
            blocks.push(OwnedBlock::new(layer_id, cfg, lora, vb_b.pp(layer_id))?);
        }
        let ln_out = candle_nn::layer_norm(c, 1e-5, vb.pp("ln_out")).map_err(load_err)?;
        let head = candle_nn::linear_no_bias(c, cfg.vocab_size, vb.pp("head")).map_err(load_err)?;
        Ok(Self {
            embeddings,
            blocks,
            ln_out,
            head,
        })
    }

    /// Forward over a single `[1, 1]` token tensor producing `[vocab]` logits
    /// (1D, matching upstream `Model::forward`), applying the residual-stream
    /// steering hook after each layer block and the LoRA delta after each owned
    /// projection. `v_first` is threaded across blocks (value-residual stream).
    pub fn forward(
        &self,
        input_ids: &Tensor,
        state: &mut State,
        hooks: &CandleSteeringHooks,
        snapshot: &[crate::model_runtime::SteeringVector],
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        // Embed + squeeze (1,1,C) -> (C,) like upstream.
        let mut xs = input_ids
            .apply(&self.embeddings)
            .and_then(|t| t.squeeze(0))
            .and_then(|t| t.squeeze(0))
            .map_err(gen_err)?;

        let mut v_first: Option<Tensor> = None;
        for (block_idx, block) in self.blocks.iter().enumerate() {
            let (new_xs, new_v_first) =
                block.forward(&xs, state, v_first, lora_stack, lora_overrides)?;
            xs = new_xs;
            v_first = Some(new_v_first);
            // Residual-stream steering seam: the per-token layer-block output.
            let li = LayerIndex::new(block_idx as u32);
            xs = hooks.apply_record_and_capture_tensor(li, HookPoint::ResidStream, &xs, snapshot)?;
        }

        let logits = xs
            .apply(&self.ln_out)
            .and_then(|t| t.unsqueeze(0))
            .and_then(|t| t.apply(&self.head))
            .and_then(|t| t.squeeze(0))
            .map_err(gen_err)?;
        state.pos += 1;
        Ok(logits)
    }

    pub fn n_layers(&self) -> usize {
        self.blocks.len()
    }
}

// ---------------------------------------------------------------------------
// CandleRwkvV7Model: TransformerModel adapter over the owned forward.
// ---------------------------------------------------------------------------

pub struct CandleRwkvV7Model {
    model: InstrumentedRwkvV7,
    state: State,
    config: Config,
    eos_token_ids: Vec<u32>,
    device: Device,
    lora_stack: CandleLoraStack,
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
        model_id: ModelId,
        config: Config,
        eos_token_ids: Vec<u32>,
        vb: VarBuilder,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let model = InstrumentedRwkvV7::load(&config, vb)?;
        let state = State::new(&config, device).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to initialize Candle RWKV v7 state: {error}"
            ))
        })?;
        let lora_stack = CandleLoraStack::new_for_device(
            model_id,
            "candle-rwkv-v7",
            CandleLoraStack::available_rwkv_v7_targets(config.num_hidden_layers),
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
                    "Candle RWKV v7 expected input shape [1, seq], got {dims:?}"
                )))
            }
        };
        // Token-by-token prefill: the owned RWKV recurrence is processed
        // sequentially (the per-token steering hook fires per token per layer).
        let mut final_logits = None;
        for idx in 0..seq_len {
            let token = input_ids.i((.., idx..idx + 1)).map_err(|error| {
                ModelRuntimeError::GenerateError(format!(
                    "Candle RWKV v7 token select failed: {error}"
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
            final_logits = Some(logits);
        }
        final_logits.ok_or_else(|| {
            ModelRuntimeError::GenerateError("Candle RWKV v7 produced no logits".to_string())
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

/// Per-layer LoRA target name for an owned RWKV v7 projection. `module` is the
/// block (`time_mix` or `channel_mix`); `proj` is the GENUINE full Linear inside
/// it (`receptance`/`key`/`value`/`output` for time-mix, `key`/`value` for
/// channel-mix). The v7 decay / ICL / value-residual / gate parameters are
/// low-rank interpolation tensors, NOT full Linears, so they are deliberately
/// excluded from the LoRA target set (mirrors v5/v6's `rwkv_v5_target`
/// exclusion of time_mix_* / time_decay / time_faaaa).
pub fn rwkv_v7_target(layer_idx: usize, module: &str, proj: &str) -> String {
    format!("backbone.layers.{layer_idx}.{module}.{proj}")
}

/// 1D vec `[in]` @ 2D weight `[in, out]` -> 1D `[out]` (upstream's
/// `unsqueeze(0).matmul(w).squeeze(0)` raw-tensor matmul, used for the low-rank
/// decay / ICL / gate / value-residual projections that are NOT owned Linears).
fn mm1d(x: &Tensor, w: &Tensor) -> Result<Tensor, ModelRuntimeError> {
    x.unsqueeze(0)
        .and_then(|t| t.matmul(w))
        .and_then(|t| t.squeeze(0))
        .map_err(gen_err)
}

/// Load a `(1, 1, C)` interpolation tensor and pre-squeeze it to `(C,)` like
/// upstream (`vb.get(...).squeeze(0).squeeze(0)`).
fn squeeze_mix(vb: &VarBuilder, name: &str, c: usize) -> Result<Tensor, ModelRuntimeError> {
    vb.get((1, 1, c), name)
        .and_then(|t| t.squeeze(0))
        .and_then(|t| t.squeeze(0))
        .map_err(load_err)
}

fn load_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!("Candle instrumented RWKV v7 load failed: {error}"))
}

fn gen_err(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::GenerateError(format!("Candle instrumented RWKV v7 forward failed: {error}"))
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

#[cfg(test)]
mod tests {
    //! MT-115 / MT-116 (INF-9) load-bearing proofs for the OWNED RWKV v7 forward:
    //!   - `owned_rwkv_v7_forward_matches_candle_transformers_step_by_step`:
    //!     builds BOTH the upstream candle RWKV v7 `Model` and our owned
    //!     `InstrumentedRwkvV7` from the SAME random-weight VarMap and asserts
    //!     step-by-step logit parity (no downloaded model). This proves the
    //!     owned forward — including the v7 generalized delta-rule recurrence,
    //!     the value-residual stream, the data-dependent decay/ICL terms, and
    //!     the manual eps=64e-5 GroupNorm — is numerically faithful, the
    //!     prerequisite for trusting the LoRA/steering seams.
    //!   - `owned_rwkv_v7_lora_mount_diverges_then_unmount_reverts`: a fixture
    //!     LoRA mounted on layer-0 time-mix receptance actually changes generated
    //!     logits, and unmount restores them (MT-115 real PEFT mount/unmount).
    //!   - `owned_rwkv_v7_steering_zero_is_identity_random_diverges`: a zero
    //!     steering vector is an identity (MT-116 identity-test correctness
    //!     gate); a non-zero vector changes the logits.
    use super::*;
    use std::collections::HashMap;

    use candle_core::Tensor;
    use candle_transformers::models::rwkv_v7::Model as CandleRwkvV7;

    use crate::model_runtime::{
        BaseModelTag, HookPoint as HP, LayerIndex as LI, LicenseTag, LoraDescriptor, LoraId,
        LoraStackOps, LoraStrength, SteeringProvenance, SteeringVector, SteeringVectorValues,
    };

    struct Dims {
        hidden: usize,
        dim_ffn: usize,
        vocab: usize,
        n_heads: usize,
        head_size: usize,
        // Low-rank inner dims for the decay / ICL / value-residual / gate LoRAs.
        d_decay: usize,
        d_aaa: usize,
        d_mv: usize,
        d_gate: usize,
    }

    fn tiny_config() -> Config {
        // head_size 4, hidden 16 -> 4 heads. intermediate_size 32. The owned
        // forward only supports the V7 base variant.
        Config {
            version: ModelVersion::V7,
            vocab_size: 24,
            hidden_size: 16,
            num_hidden_layers: 2,
            head_size: 4,
            intermediate_size: Some(32),
            rescale_every: 0,
        }
    }

    fn dims(cfg: &Config) -> Dims {
        Dims {
            hidden: cfg.hidden_size,
            dim_ffn: cfg.intermediate_size.unwrap_or(cfg.hidden_size * 4),
            vocab: cfg.vocab_size,
            n_heads: cfg.hidden_size / cfg.head_size,
            head_size: cfg.head_size,
            d_decay: 8,
            d_aaa: 8,
            d_mv: 8,
            d_gate: 8,
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
        put("emb.weight".to_string(), &[d.vocab, d.hidden]);
        put("ln_out.weight".to_string(), &[d.hidden]);
        put("ln_out.bias".to_string(), &[d.hidden]);
        put("head.weight".to_string(), &[d.vocab, d.hidden]);
        for i in 0..cfg.num_hidden_layers {
            let b = format!("blocks.{i}");
            if i == 0 {
                put(format!("{b}.ln0.weight"), &[d.hidden]);
                put(format!("{b}.ln0.bias"), &[d.hidden]);
            }
            put(format!("{b}.ln1.weight"), &[d.hidden]);
            put(format!("{b}.ln1.bias"), &[d.hidden]);
            put(format!("{b}.ln2.weight"), &[d.hidden]);
            put(format!("{b}.ln2.bias"), &[d.hidden]);
            // time-mix (att).
            let a = format!("{b}.att");
            put(format!("{a}.x_r"), &[1, 1, d.hidden]);
            put(format!("{a}.x_w"), &[1, 1, d.hidden]);
            put(format!("{a}.x_k"), &[1, 1, d.hidden]);
            put(format!("{a}.x_v"), &[1, 1, d.hidden]);
            put(format!("{a}.x_a"), &[1, 1, d.hidden]);
            put(format!("{a}.x_g"), &[1, 1, d.hidden]);
            put(format!("{a}.w0"), &[1, 1, d.hidden]);
            put(format!("{a}.w1"), &[d.hidden, d.d_decay]);
            put(format!("{a}.w2"), &[d.d_decay, d.hidden]);
            put(format!("{a}.a0"), &[1, 1, d.hidden]);
            put(format!("{a}.a1"), &[d.hidden, d.d_aaa]);
            put(format!("{a}.a2"), &[d.d_aaa, d.hidden]);
            // v0/v1/v2 exist for ALL layers in the weights file.
            put(format!("{a}.v0"), &[1, 1, d.hidden]);
            put(format!("{a}.v1"), &[d.hidden, d.d_mv]);
            put(format!("{a}.v2"), &[d.d_mv, d.hidden]);
            put(format!("{a}.g1"), &[d.hidden, d.d_gate]);
            put(format!("{a}.g2"), &[d.d_gate, d.hidden]);
            put(format!("{a}.k_k"), &[1, 1, d.hidden]);
            put(format!("{a}.k_a"), &[1, 1, d.hidden]);
            put(format!("{a}.r_k"), &[d.n_heads, d.head_size]);
            put(format!("{a}.receptance.weight"), &[d.hidden, d.hidden]);
            put(format!("{a}.key.weight"), &[d.hidden, d.hidden]);
            put(format!("{a}.value.weight"), &[d.hidden, d.hidden]);
            put(format!("{a}.output.weight"), &[d.hidden, d.hidden]);
            put(format!("{a}.ln_x.weight"), &[d.hidden]);
            put(format!("{a}.ln_x.bias"), &[d.hidden]);
            // channel-mix (ffn).
            let f = format!("{b}.ffn");
            put(format!("{f}.x_k"), &[1, 1, d.hidden]);
            put(format!("{f}.key.weight"), &[d.dim_ffn, d.hidden]);
            put(format!("{f}.value.weight"), &[d.hidden, d.dim_ffn]);
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
    fn owned_rwkv_v7_forward_matches_candle_transformers_step_by_step() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let weights = random_weights(&cfg, &device);

        let candle_model = CandleRwkvV7::new(
            &cfg,
            VarBuilder::from_tensors(weights.clone(), DType::F32, &device),
        )
        .expect("candle rwkv_v7 builds");
        let mut candle_state = State::new(&cfg, &device).unwrap();

        let owned =
            InstrumentedRwkvV7::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v7 builds");
        let mut owned_state = State::new(&cfg, &device).unwrap();

        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-rwkv-v7",
            CandleLoraStack::available_rwkv_v7_targets(cfg.num_hidden_layers),
        );

        // Drive both models token-by-token through their single-token forward;
        // the owned delta-rule recurrence must match candle at every position.
        for &tid in &[3u32, 7, 1, 5, 2, 9] {
            // Upstream forward takes a [1,1] token tensor + the token id slice.
            let candle_logits = candle_model
                .forward(&token(tid, &device), &mut candle_state, &[tid])
                .unwrap();
            let owned_logits = owned
                .forward(&token(tid, &device), &mut owned_state, &hooks, &[], &lora, &[])
                .unwrap();
            let c = logits_vec(&candle_logits);
            let o = logits_vec(&owned_logits);
            assert_eq!(c.len(), o.len(), "logit width mismatch at token {tid}");
            let diff = max_abs_diff(&c, &o);
            assert!(
                diff < 1e-4,
                "owned RWKV v7 diverged from candle at token {tid}: max |Δlogit| = {diff}"
            );
        }
    }

    #[tokio::test]
    async fn owned_rwkv_v7_lora_mount_diverges_then_unmount_reverts() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedRwkvV7::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v7 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(
            ModelId::new_v7(),
            "candle-rwkv-v7",
            CandleLoraStack::available_rwkv_v7_targets(cfg.num_hidden_layers),
        );

        // Drive a short fixed sequence token-by-token through ONE state, then
        // read the final-token logits. Priming the recurrence (WKV state matrix
        // + token-shift) before measuring avoids the fresh-state first-token
        // attenuation that shrinks a time-mix LoRA delta — see the RECURRENCE
        // SUBTLETY note at the top of this file (same trick as v5/v6).
        let seq = [3u32, 7, 1, 4];
        let run_seq = |overrides: &[LoraId]| {
            let mut state = State::new(&cfg, &device).unwrap();
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
        // B [hidden, rank]. Receptance gates the WKV output directly
        // (out = state @ r). The tensors are DETERMINISTIC (constant-filled, no
        // RNG) so the test is reproducible, and large enough that the GENUINE
        // PEFT delta clears the 1e-4 assertion without weakening the threshold.
        let rank = 2usize;
        let target = rwkv_v7_target(0, "time_mix", "receptance");
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("rwkv_v7_lora.safetensors");
        let mut tensors = HashMap::new();
        tensors.insert(
            format!("{target}.lora_A.weight"),
            (Tensor::ones((rank, d.hidden), DType::F32, &device).unwrap() * 0.5).unwrap(),
        );
        tensors.insert(
            format!("{target}.lora_B.weight"),
            (Tensor::ones((d.hidden, rank), DType::F32, &device).unwrap() * 0.5).unwrap(),
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
            base_model_compat: BaseModelTag::new("candle-rwkv-v7"),
            license_tag: LicenseTag::new("test-license"),
        };
        lora
            .mount(descriptor, LoraStrength::try_new(1.0).unwrap())
            .await
            .expect("RWKV v7 LoRA mounts");

        let mounted = run_seq(&[lora_id]);
        assert!(
            max_abs_diff(&baseline, &mounted) > 1e-4,
            "mounted RWKV v7 LoRA must change time-mix receptance output and therefore the logits"
        );

        lora.unmount(lora_id).await.expect("unmount");
        let reverted = run_seq(&[]);
        assert!(
            max_abs_diff(&baseline, &reverted) < 1e-6,
            "unmounting the RWKV v7 LoRA must revert the logits to baseline"
        );
    }

    #[tokio::test]
    async fn owned_rwkv_v7_steering_zero_is_identity_random_diverges() {
        let device = Device::Cpu;
        let cfg = tiny_config();
        let d = dims(&cfg);
        let weights = random_weights(&cfg, &device);
        let owned =
            InstrumentedRwkvV7::load(&cfg, VarBuilder::from_tensors(weights, DType::F32, &device))
                .expect("owned rwkv_v7 builds");
        let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), cfg.hidden_size);
        let lora = CandleLoraStack::new(ModelId::new_v7(), "candle-rwkv-v7", Vec::new());

        let run = |snapshot: &[SteeringVector]| {
            let mut state = State::new(&cfg, &device).unwrap();
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
            "a zero steering vector must be an identity on the RWKV v7 residual stream"
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
            "a non-zero steering vector must change the RWKV v7 logits"
        );
    }
}
