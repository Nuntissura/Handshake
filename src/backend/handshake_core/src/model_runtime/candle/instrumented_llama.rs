#![cfg(feature = "candle-runtime-engine")]

// Adapted from candle-transformers 0.10.2 `src/models/llama.rs`.
// Upstream keeps the Llama block loop private; MT-083 needs a repo-local hook
// seam at residual-stream layer boundaries while preserving Candle tensor ops.

use std::{collections::HashMap, f32::consts::PI};

use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::{embedding, Embedding, Module, VarBuilder};
use candle_transformers::{
    models::{
        llama::{Config, Llama3RopeConfig, Llama3RopeType},
        with_tracing::{linear_no_bias as linear, Linear, RmsNorm},
    },
    utils::{build_causal_mask, repeat_kv},
};

use super::{hooks::CandleSteeringHooks, lora_impl::CandleLoraStack};
use crate::model_runtime::{HookPoint, LayerIndex, LoraId, ModelRuntimeError, SteeringVectorId};

#[derive(Debug, Clone)]
pub struct InstrumentedLlamaCache {
    masks: HashMap<(usize, usize), Tensor>,
    use_kv_cache: bool,
    kvs: Vec<Option<(Tensor, Tensor)>>,
    cos: Tensor,
    sin: Tensor,
    device: Device,
}

impl InstrumentedLlamaCache {
    pub fn new(
        use_kv_cache: bool,
        dtype: DType,
        config: &Config,
        device: &Device,
    ) -> Result<Self, ModelRuntimeError> {
        let theta = match &config.rope_scaling {
            None
            | Some(Llama3RopeConfig {
                rope_type: Llama3RopeType::Default,
                ..
            }) => calculate_default_inv_freq(config),
            Some(rope_scaling) => {
                let low_freq_wavelen = rope_scaling.original_max_position_embeddings as f32
                    / rope_scaling.low_freq_factor;
                let high_freq_wavelen = rope_scaling.original_max_position_embeddings as f32
                    / rope_scaling.high_freq_factor;

                calculate_default_inv_freq(config)
                    .into_iter()
                    .map(|freq| {
                        let wavelen = 2. * PI / freq;
                        if wavelen < high_freq_wavelen {
                            freq
                        } else if wavelen > low_freq_wavelen {
                            freq / rope_scaling.factor
                        } else {
                            let smooth = (rope_scaling.original_max_position_embeddings as f32
                                / wavelen
                                - rope_scaling.low_freq_factor)
                                / (rope_scaling.high_freq_factor - rope_scaling.low_freq_factor);
                            (1. - smooth) * freq / rope_scaling.factor + smooth * freq
                        }
                    })
                    .collect::<Vec<_>>()
            }
        };

        let theta = Tensor::new(theta, device).map_err(candle_load_error)?;
        let idx_theta = Tensor::arange(0, config.max_position_embeddings as u32, device)
            .and_then(|tensor| tensor.to_dtype(DType::F32))
            .and_then(|tensor| tensor.reshape((config.max_position_embeddings, 1)))
            .and_then(|tensor| tensor.matmul(&theta.reshape((1, theta.elem_count()))?))
            .map_err(candle_load_error)?;
        let cos = idx_theta
            .cos()
            .and_then(|tensor| tensor.to_dtype(dtype))
            .map_err(candle_load_error)?;
        let sin = idx_theta
            .sin()
            .and_then(|tensor| tensor.to_dtype(dtype))
            .map_err(candle_load_error)?;
        Ok(Self {
            masks: HashMap::new(),
            use_kv_cache,
            kvs: vec![None; config.num_hidden_layers],
            cos,
            sin,
            device: device.clone(),
        })
    }

    fn mask(&mut self, seq_len: usize, index_pos: usize) -> candle_core::Result<Tensor> {
        let kv_len = index_pos + seq_len;
        if let Some(mask) = self.masks.get(&(seq_len, kv_len)) {
            Ok(mask.clone())
        } else {
            let mask = build_causal_mask(seq_len, index_pos, &self.device)?;
            self.masks.insert((seq_len, kv_len), mask.clone());
            Ok(mask)
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstrumentedLlama {
    wte: Embedding,
    blocks: Vec<Block>,
    ln_f: RmsNorm,
    lm_head: Linear,
}

impl InstrumentedLlama {
    pub fn load(vb: VarBuilder, config: &Config) -> Result<Self, ModelRuntimeError> {
        let wte = embedding(
            config.vocab_size,
            config.hidden_size,
            vb.pp("model.embed_tokens"),
        )
        .map_err(candle_load_error)?;
        let lm_head = if config.tie_word_embeddings {
            Linear::from_weights(wte.embeddings().clone(), None)
        } else {
            linear(config.hidden_size, config.vocab_size, vb.pp("lm_head"))
                .map_err(candle_load_error)?
        };
        let ln_f = RmsNorm::new(config.hidden_size, config.rms_norm_eps, vb.pp("model.norm"))
            .map_err(candle_load_error)?;
        let blocks = (0..config.num_hidden_layers)
            .map(|index| Block::load(vb.pp(format!("model.layers.{index}")), config))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            wte,
            blocks,
            ln_f,
            lm_head,
        })
    }

    pub fn forward(
        &self,
        input_ids: &Tensor,
        index_pos: usize,
        cache: &mut InstrumentedLlamaCache,
        hooks: &CandleSteeringHooks,
        steering_overrides: &[SteeringVectorId],
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let (_batch, seq_len) = input_ids.dims2().map_err(candle_generate_error)?;
        let mut x = self.wte.forward(input_ids).map_err(candle_generate_error)?;
        let vectors = hooks.snapshot_vectors_for_request(steering_overrides)?;
        for (block_idx, block) in self.blocks.iter().enumerate() {
            x = block.forward(&x, index_pos, block_idx, cache, lora_stack, lora_overrides)?;
            let layer = LayerIndex::new(block_idx as u32);
            x = hooks.apply_record_and_capture_tensor(
                layer,
                HookPoint::ResidStream,
                &x,
                &vectors,
            )?;
        }
        let x = self.ln_f.forward(&x).map_err(candle_generate_error)?;
        let x = x
            .i((.., seq_len - 1, ..))
            .and_then(|tensor| tensor.contiguous())
            .map_err(candle_generate_error)?;
        let logits = self.lm_head.forward(&x).map_err(candle_generate_error)?;
        logits.to_dtype(DType::F32).map_err(candle_generate_error)
    }
}

#[derive(Debug, Clone)]
struct CausalSelfAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
    use_flash_attn: bool,
    max_position_embeddings: usize,
}

impl CausalSelfAttention {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self, ModelRuntimeError> {
        let size_in = config.hidden_size;
        let size_q = (config.hidden_size / config.num_attention_heads) * config.num_attention_heads;
        let size_kv =
            (config.hidden_size / config.num_attention_heads) * config.num_key_value_heads;
        Ok(Self {
            q_proj: linear(size_in, size_q, vb.pp("q_proj")).map_err(candle_load_error)?,
            k_proj: linear(size_in, size_kv, vb.pp("k_proj")).map_err(candle_load_error)?,
            v_proj: linear(size_in, size_kv, vb.pp("v_proj")).map_err(candle_load_error)?,
            o_proj: linear(size_q, size_in, vb.pp("o_proj")).map_err(candle_load_error)?,
            num_attention_heads: config.num_attention_heads,
            num_key_value_heads: config.num_key_value_heads,
            head_dim: config.hidden_size / config.num_attention_heads,
            use_flash_attn: config.use_flash_attn,
            max_position_embeddings: config.max_position_embeddings,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        index_pos: usize,
        block_idx: usize,
        cache: &mut InstrumentedLlamaCache,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        if self.use_flash_attn {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "candle flash attention in instrumented llama".to_string(),
                adapter: "candle".to_string(),
            });
        }

        let (batch_size, seq_len, hidden_size) = x.dims3().map_err(candle_generate_error)?;
        let q = self.q_proj.forward(x).map_err(candle_generate_error)?;
        let q = lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.self_attn.q_proj"),
            &q,
            x,
            lora_overrides,
        )?;
        let k = self.k_proj.forward(x).map_err(candle_generate_error)?;
        let k = lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.self_attn.k_proj"),
            &k,
            x,
            lora_overrides,
        )?;
        let v = self.v_proj.forward(x).map_err(candle_generate_error)?;
        let v = lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.self_attn.v_proj"),
            &v,
            x,
            lora_overrides,
        )?;

        let q = q
            .reshape((batch_size, seq_len, self.num_attention_heads, self.head_dim))
            .and_then(|tensor| tensor.transpose(1, 2))
            .and_then(|tensor| tensor.contiguous())
            .map_err(candle_generate_error)?;
        let k = k
            .reshape((batch_size, seq_len, self.num_key_value_heads, self.head_dim))
            .and_then(|tensor| tensor.transpose(1, 2))
            .and_then(|tensor| tensor.contiguous())
            .map_err(candle_generate_error)?;
        let mut v = v
            .reshape((batch_size, seq_len, self.num_key_value_heads, self.head_dim))
            .and_then(|tensor| tensor.transpose(1, 2))
            .map_err(candle_generate_error)?;

        let q = self.apply_rotary_emb(&q, index_pos, cache)?;
        let mut k = self.apply_rotary_emb(&k, index_pos, cache)?;

        if cache.use_kv_cache {
            if let Some((cache_k, cache_v)) = &cache.kvs[block_idx] {
                k = Tensor::cat(&[cache_k, &k], 2)
                    .and_then(|tensor| tensor.contiguous())
                    .map_err(candle_generate_error)?;
                v = Tensor::cat(&[cache_v, &v], 2).map_err(candle_generate_error)?;
                let k_seq_len = k.dims()[1];
                if k_seq_len > self.max_position_embeddings {
                    k = k
                        .narrow(
                            D::Minus1,
                            k_seq_len - self.max_position_embeddings,
                            self.max_position_embeddings,
                        )
                        .and_then(|tensor| tensor.contiguous())
                        .map_err(candle_generate_error)?;
                }
                let v_seq_len = v.dims()[1];
                if v_seq_len > 2 * self.max_position_embeddings {
                    v = v
                        .narrow(
                            D::Minus1,
                            v_seq_len - self.max_position_embeddings,
                            self.max_position_embeddings,
                        )
                        .and_then(|tensor| tensor.contiguous())
                        .map_err(candle_generate_error)?;
                }
            }
            cache.kvs[block_idx] = Some((k.clone(), v.clone()));
        }

        let k = repeat_kv(k, self.num_attention_heads / self.num_key_value_heads)
            .map_err(candle_generate_error)?;
        let v = repeat_kv(v, self.num_attention_heads / self.num_key_value_heads)
            .map_err(candle_generate_error)?;
        let in_dtype = q.dtype();
        let q = q.to_dtype(DType::F32).map_err(candle_generate_error)?;
        let k = k.to_dtype(DType::F32).map_err(candle_generate_error)?;
        let v = v.to_dtype(DType::F32).map_err(candle_generate_error)?;
        let att = (q
            .matmul(&k.t().map_err(candle_generate_error)?)
            .map_err(candle_generate_error)?
            / (self.head_dim as f64).sqrt())
        .map_err(candle_generate_error)?;
        let att = if seq_len == 1 {
            att
        } else {
            let mask = cache
                .mask(seq_len, index_pos)
                .and_then(|mask| mask.broadcast_as(att.shape()))
                .map_err(candle_generate_error)?;
            masked_fill(&att, &mask, f32::NEG_INFINITY)?
        };
        let att = candle_nn::ops::softmax_last_dim(&att).map_err(candle_generate_error)?;
        let y = att
            .matmul(&v.contiguous().map_err(candle_generate_error)?)
            .and_then(|tensor| tensor.to_dtype(in_dtype))
            .and_then(|tensor| tensor.transpose(1, 2))
            .and_then(|tensor| tensor.reshape((batch_size, seq_len, hidden_size)))
            .map_err(candle_generate_error)?;
        let output = self.o_proj.forward(&y).map_err(candle_generate_error)?;
        lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.self_attn.o_proj"),
            &output,
            &y,
            lora_overrides,
        )
    }

    fn apply_rotary_emb(
        &self,
        x: &Tensor,
        index_pos: usize,
        cache: &InstrumentedLlamaCache,
    ) -> Result<Tensor, ModelRuntimeError> {
        let (_batch, _heads, seq_len, _hidden) = x.dims4().map_err(candle_generate_error)?;
        let cos = cache
            .cos
            .narrow(0, index_pos, seq_len)
            .map_err(candle_generate_error)?;
        let sin = cache
            .sin
            .narrow(0, index_pos, seq_len)
            .map_err(candle_generate_error)?;
        candle_nn::rotary_emb::rope(x, &cos, &sin).map_err(candle_generate_error)
    }
}

#[derive(Debug, Clone)]
struct Mlp {
    c_fc1: Linear,
    c_fc2: Linear,
    c_proj: Linear,
}

impl Mlp {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self, ModelRuntimeError> {
        Ok(Self {
            c_fc1: linear(
                config.hidden_size,
                config.intermediate_size,
                vb.pp("gate_proj"),
            )
            .map_err(candle_load_error)?,
            c_fc2: linear(
                config.hidden_size,
                config.intermediate_size,
                vb.pp("up_proj"),
            )
            .map_err(candle_load_error)?,
            c_proj: linear(
                config.intermediate_size,
                config.hidden_size,
                vb.pp("down_proj"),
            )
            .map_err(candle_load_error)?,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        block_idx: usize,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let gate = self.c_fc1.forward(x).map_err(candle_generate_error)?;
        let gate = lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.mlp.gate_proj"),
            &gate,
            x,
            lora_overrides,
        )?;
        let gated = candle_nn::ops::silu(&gate).map_err(candle_generate_error)?;
        let up = self.c_fc2.forward(x).map_err(candle_generate_error)?;
        let up = lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.mlp.up_proj"),
            &up,
            x,
            lora_overrides,
        )?;
        let x = (gated * up).map_err(candle_generate_error)?;
        let output = self.c_proj.forward(&x).map_err(candle_generate_error)?;
        lora_stack.apply_to_linear_output(
            &format!("model.layers.{block_idx}.mlp.down_proj"),
            &output,
            &x,
            lora_overrides,
        )
    }
}

#[derive(Debug, Clone)]
struct Block {
    rms_1: RmsNorm,
    attn: CausalSelfAttention,
    rms_2: RmsNorm,
    mlp: Mlp,
}

impl Block {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self, ModelRuntimeError> {
        Ok(Self {
            rms_1: RmsNorm::new(
                config.hidden_size,
                config.rms_norm_eps,
                vb.pp("input_layernorm"),
            )
            .map_err(candle_load_error)?,
            attn: CausalSelfAttention::load(vb.pp("self_attn"), config)?,
            rms_2: RmsNorm::new(
                config.hidden_size,
                config.rms_norm_eps,
                vb.pp("post_attention_layernorm"),
            )
            .map_err(candle_load_error)?,
            mlp: Mlp::load(vb.pp("mlp"), config)?,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        index_pos: usize,
        block_idx: usize,
        cache: &mut InstrumentedLlamaCache,
        lora_stack: &CandleLoraStack,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let residual = x;
        let x = self.rms_1.forward(x).map_err(candle_generate_error)?;
        let x = (self
            .attn
            .forward(&x, index_pos, block_idx, cache, lora_stack, lora_overrides)?
            + residual)
            .map_err(candle_generate_error)?;
        let residual = &x;
        let x = self
            .rms_2
            .forward(&x)
            .map_err(candle_generate_error)
            .and_then(|x| self.mlp.forward(&x, block_idx, lora_stack, lora_overrides))?;
        (x + residual).map_err(candle_generate_error)
    }
}

fn calculate_default_inv_freq(config: &Config) -> Vec<f32> {
    let head_dim = config.hidden_size / config.num_attention_heads;
    (0..head_dim)
        .step_by(2)
        .map(|index| 1_f32 / config.rope_theta.powf(index as f32 / head_dim as f32))
        .collect()
}

fn masked_fill(
    on_false: &Tensor,
    mask: &Tensor,
    on_true: f32,
) -> Result<Tensor, ModelRuntimeError> {
    let shape = mask.shape();
    let on_true = Tensor::new(on_true, on_false.device())
        .and_then(|tensor| tensor.broadcast_as(shape.dims()))
        .map_err(candle_generate_error)?;
    mask.where_cond(&on_true, on_false)
        .map_err(candle_generate_error)
}

fn candle_load_error(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!("Candle instrumented Llama load failed: {error}"))
}

fn candle_generate_error(error: candle_core::Error) -> ModelRuntimeError {
    ModelRuntimeError::GenerateError(format!("Candle instrumented Llama forward failed: {error}"))
}
