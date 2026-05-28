#![cfg(feature = "candle-runtime-engine")]

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::model_runtime::{
    LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackOps, LoraStackSnapshot,
    LoraStackSnapshotEntry, LoraStrength, ModelId, ModelRuntimeError,
};

#[derive(Clone)]
pub struct CandleLoraStack {
    model_id: ModelId,
    base_model_tag: String,
    device: Device,
    valid_targets: Arc<Vec<String>>,
    state: Arc<Mutex<CandleLoraStackState>>,
}

#[derive(Clone, Default)]
struct CandleLoraStackState {
    active: Vec<MountedLora>,
}

#[derive(Clone)]
struct MountedLora {
    descriptor: LoraDescriptor,
    strength: LoraStrength,
    mounted_at_utc: DateTime<Utc>,
    adapter: CandleLoraAdapter,
}

#[derive(Clone)]
struct CandleLoraAdapter {
    target_modules: HashMap<String, CandleLoraTarget>,
}

#[derive(Clone)]
struct CandleLoraTarget {
    a: Tensor,
    b: Tensor,
    scaling: f32,
}

#[derive(Clone, Copy)]
enum LoraTensorKind {
    A,
    B,
}

#[derive(Default)]
struct LoraTensorPair {
    a_key: Option<String>,
    b_key: Option<String>,
}

#[derive(Default, Deserialize)]
struct PeftAdapterConfig {
    target_modules: Option<Value>,
    r: Option<u32>,
    lora_alpha: Option<f32>,
    base_model_name_or_path: Option<String>,
    peft_type: Option<String>,
}

impl CandleLoraStack {
    pub fn new(
        model_id: ModelId,
        base_model_tag: impl Into<String>,
        valid_targets: Vec<String>,
    ) -> Self {
        Self::new_for_device(model_id, base_model_tag, valid_targets, Device::Cpu)
    }

    pub fn new_for_device(
        model_id: ModelId,
        base_model_tag: impl Into<String>,
        valid_targets: Vec<String>,
        device: Device,
    ) -> Self {
        Self {
            model_id,
            base_model_tag: base_model_tag.into(),
            device,
            valid_targets: Arc::new(valid_targets),
            state: Arc::new(Mutex::new(CandleLoraStackState::default())),
        }
    }

    pub fn handle(&self) -> LoraStackHandle {
        LoraStackHandle::with_ops(
            format!("candle:{}:lora_stack", self.model_id),
            Arc::new(self.clone()),
        )
    }

    pub fn available_llama_targets(num_layers: usize) -> Vec<String> {
        let mut targets = Vec::with_capacity(num_layers * 7);
        for layer in 0..num_layers {
            for projection in ["q_proj", "k_proj", "v_proj", "o_proj"] {
                targets.push(format!("model.layers.{layer}.self_attn.{projection}"));
            }
            for projection in ["gate_proj", "up_proj", "down_proj"] {
                targets.push(format!("model.layers.{layer}.mlp.{projection}"));
            }
        }
        targets
    }

    /// MT-115 (INF-9 LoRA-for-SSM): valid LoRA targets for the owned Mamba2
    /// forward. candle's Mamba2 fuses x_proj/dt_proj/z into a single `in_proj`,
    /// so the realisable per-layer targets are the fused input projection
    /// (`in_proj`) and the output projection (`out_proj`). Naming mirrors the
    /// `mamba2_target()` helper used in the owned forward.
    pub fn available_mamba2_targets(num_layers: usize) -> Vec<String> {
        let mut targets = Vec::with_capacity(num_layers * 2);
        for layer in 0..num_layers {
            for projection in ["in_proj", "out_proj"] {
                targets.push(format!("backbone.layers.{layer}.mixer.{projection}"));
            }
        }
        targets
    }

    /// MT-115 (INF-9 LoRA-for-SSM): valid LoRA targets for the owned RWKV v5
    /// forward. RWKV v5 splits each layer into a time-mix (attention-like)
    /// block with `receptance`/`key`/`value`/`gate`/`output` Linears and a
    /// channel-mix (feed-forward) block with `receptance`/`key`/`value`
    /// Linears — these are the realisable per-layer PEFT targets (the
    /// time_mix_*/time_decay/time_faaaa interpolation tensors and the GroupNorm
    /// are scalar/diagonal and intentionally excluded, like Mamba2's A/D/conv).
    /// Naming mirrors the `rwkv_v5_target()` helper used in the owned forward.
    pub fn available_rwkv_targets(num_layers: usize) -> Vec<String> {
        let mut targets = Vec::with_capacity(num_layers * 8);
        for layer in 0..num_layers {
            for projection in ["receptance", "key", "value", "gate", "output"] {
                targets.push(format!("backbone.layers.{layer}.time_mix.{projection}"));
            }
            for projection in ["receptance", "key", "value"] {
                targets.push(format!("backbone.layers.{layer}.channel_mix.{projection}"));
            }
        }
        targets
    }

    /// MT-115 (INF-9 LoRA-for-SSM): valid LoRA targets for the owned RWKV v7
    /// "Goose" forward. v7 replaces v5/v6's gate Linear with low-rank gate
    /// tensors (`g1`/`g2`) and adds the decay / ICL-rate / value-residual
    /// low-rank LoRA-style tensors (`w*`/`a*`/`v*`) plus the per-head key/bonus
    /// tensors (`k_k`/`k_a`/`r_k`) — none of which are genuine full Linears, so
    /// they are excluded as PEFT targets (same treatment as v5/v6's
    /// interpolation tensors and Mamba2's A/D/conv). The realisable per-layer
    /// PEFT targets are therefore the GENUINE full `candle_nn::Linear`
    /// projections v7 owns: time-mix `receptance`/`key`/`value`/`output`
    /// (each `[C, C]`) and channel-mix `key`/`value` (`[dim_ffn, C]` /
    /// `[C, dim_ffn]`). v7 has NO time-mix `gate` Linear and NO channel-mix
    /// `receptance` Linear, so this set differs from v5/v6 and gets its own
    /// helper. Naming mirrors the `rwkv_v7_target()` helper used in the owned
    /// forward.
    pub fn available_rwkv_v7_targets(num_layers: usize) -> Vec<String> {
        let mut targets = Vec::with_capacity(num_layers * 6);
        for layer in 0..num_layers {
            for projection in ["receptance", "key", "value", "output"] {
                targets.push(format!("backbone.layers.{layer}.time_mix.{projection}"));
            }
            for projection in ["key", "value"] {
                targets.push(format!("backbone.layers.{layer}.channel_mix.{projection}"));
            }
        }
        targets
    }

    pub fn ensure_overrides_mounted(&self, ids: &[LoraId]) -> Result<(), ModelRuntimeError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mounted = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?
            .active
            .iter()
            .map(|entry| entry.descriptor.id)
            .collect::<HashSet<_>>();
        let missing = ids
            .iter()
            .copied()
            .filter(|id| !mounted.contains(id))
            .map(|id| id.to_string())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(lora_error(format!(
                "Candle LoRA override ids are not mounted: {}",
                missing.join(", ")
            )))
        }
    }

    pub fn apply_to_linear_output(
        &self,
        target: &str,
        base_output: &Tensor,
        input: &Tensor,
        lora_overrides: &[LoraId],
    ) -> Result<Tensor, ModelRuntimeError> {
        let active = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?
            .active
            .clone();
        let mut output = base_output.clone();
        for mounted in active {
            if !lora_overrides.is_empty() && !lora_overrides.contains(&mounted.descriptor.id) {
                continue;
            }
            let Some(lora_target) = mounted.adapter.target_modules.get(target) else {
                continue;
            };
            output = apply_lora_delta_to_linear_output(
                &output,
                input,
                &lora_target.a,
                &lora_target.b,
                lora_target.scaling * mounted.strength.value(),
                target,
            )?;
        }
        Ok(output)
    }

    fn load_adapter(&self, desc: &LoraDescriptor) -> Result<CandleLoraAdapter, ModelRuntimeError> {
        if desc.license_tag.as_str().trim().is_empty() {
            return Err(lora_error("LoRA descriptor license tag must not be empty"));
        }
        if desc.base_model_compat.as_str() != self.base_model_tag {
            return Err(lora_error(format!(
                "LoRA base model mismatch: descriptor={}, loaded={}",
                desc.base_model_compat.as_str(),
                self.base_model_tag
            )));
        }
        let config = load_peft_adapter_config(&desc.artifact_path)?;
        if let Some(peft_type) = config.peft_type.as_deref() {
            if !peft_type.eq_ignore_ascii_case("LORA") {
                return Err(lora_error(format!(
                    "unsupported PEFT adapter type for Candle LoRA: {peft_type}"
                )));
            }
        }
        if let Some(config_base) = config.base_model_name_or_path.as_deref() {
            if !config_base.trim().is_empty()
                && config_base.trim() != desc.base_model_compat.as_str()
            {
                return Err(lora_error(format!(
                    "LoRA base model mismatch: adapter_config={}, descriptor={}",
                    config_base.trim(),
                    desc.base_model_compat.as_str()
                )));
            }
        }
        let requested_modules = config
            .target_modules()
            .unwrap_or_else(|| desc.target_modules.clone());
        if requested_modules.is_empty() {
            return Err(lora_error(
                "LoRA descriptor target_modules must not be empty",
            ));
        }
        let expected_rank = config.r.unwrap_or(desc.rank);
        if expected_rank == 0 {
            return Err(lora_error("LoRA rank must be greater than zero"));
        }
        if config.r.is_some() && expected_rank != desc.rank {
            return Err(lora_error(format!(
                "LoRA rank mismatch: descriptor={}, adapter_config={}",
                desc.rank, expected_rank
            )));
        }
        let scaling = config.lora_alpha.unwrap_or(expected_rank as f32) / expected_rank as f32;
        validate_sha256(&desc.artifact_path, desc.sha256)?;
        let tensors = candle_core::safetensors::load(&desc.artifact_path, &self.device)
            .map_err(|error| lora_error(format!("failed to load LoRA safetensors: {error}")))?;

        let mut expected_targets = HashSet::new();
        let mut missing_targets = Vec::new();
        for requested in &requested_modules {
            let expanded = self.expand_target(requested);
            if expanded.is_empty() {
                missing_targets.push(requested.clone());
                continue;
            }
            expected_targets.extend(expanded);
        }

        if !missing_targets.is_empty() {
            return Err(lora_error(format!(
                "missing target modules for Candle LoRA: {} (available: {})",
                missing_targets.join(", "),
                self.valid_targets.join(", ")
            )));
        }

        let mut tensor_pairs = HashMap::<String, LoraTensorPair>::new();
        let mut extra_targets = HashSet::new();
        for key in tensors.keys() {
            let Some((raw_target, kind)) = parse_lora_tensor_key(key) else {
                continue;
            };
            let normalized = normalize_peft_target(raw_target);
            let expanded = self.expand_target(&normalized);
            if expanded.is_empty() {
                extra_targets.insert(normalized);
                continue;
            }
            if expanded.len() > 1 {
                return Err(lora_error(format!(
                    "ambiguous LoRA tensor target {normalized}; tensor keys must identify a single layer target"
                )));
            }
            let target = expanded.into_iter().next().expect("checked length");
            if !expected_targets.contains(&target) {
                extra_targets.insert(target);
                continue;
            }
            let pair = tensor_pairs.entry(target).or_default();
            match kind {
                LoraTensorKind::A => pair.a_key = Some(key.clone()),
                LoraTensorKind::B => pair.b_key = Some(key.clone()),
            }
        }
        if !extra_targets.is_empty() {
            let mut extra_targets = extra_targets.into_iter().collect::<Vec<_>>();
            extra_targets.sort();
            return Err(lora_error(format!(
                "extra target modules for Candle LoRA: {}",
                extra_targets.join(", ")
            )));
        }

        let mut target_modules = HashMap::new();
        let mut targets = expected_targets.into_iter().collect::<Vec<_>>();
        targets.sort();
        for target in targets {
            let Some(pair) = tensor_pairs.get(&target) else {
                return Err(lora_error(format!(
                    "missing LoRA tensor pair for target module {target}"
                )));
            };
            let Some(a_key) = pair.a_key.as_deref() else {
                return Err(lora_error(format!(
                    "missing LoRA tensor {target}.lora_A.weight"
                )));
            };
            let Some(b_key) = pair.b_key.as_deref() else {
                return Err(lora_error(format!(
                    "missing LoRA tensor {target}.lora_B.weight"
                )));
            };
            let a = tensors
                .get(a_key)
                .cloned()
                .ok_or_else(|| lora_error(format!("missing LoRA tensor {a_key}")))?;
            let b = tensors
                .get(b_key)
                .cloned()
                .ok_or_else(|| lora_error(format!("missing LoRA tensor {b_key}")))?;
            validate_lora_shapes(expected_rank, &target, &a, &b)?;
            target_modules.insert(target, CandleLoraTarget { a, b, scaling });
        }
        Ok(CandleLoraAdapter { target_modules })
    }

    fn expand_target(&self, requested: &str) -> Vec<String> {
        if self.valid_targets.iter().any(|target| target == requested) {
            return vec![requested.to_string()];
        }
        let suffix = format!(".{requested}");
        self.valid_targets
            .iter()
            .filter(|target| target.ends_with(&suffix))
            .cloned()
            .collect::<Vec<_>>()
    }

    fn snapshot_locked(state: &CandleLoraStackState) -> LoraStackSnapshot {
        LoraStackSnapshot {
            entries: state
                .active
                .iter()
                .map(|entry| LoraStackSnapshotEntry {
                    descriptor: entry.descriptor.clone(),
                    strength: entry.strength.clone(),
                    mounted_at_utc: entry.mounted_at_utc,
                })
                .collect(),
        }
    }
}

#[async_trait]
impl LoraStackOps for CandleLoraStack {
    async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        let adapter = self.load_adapter(&desc)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?;
        state.active.retain(|entry| entry.descriptor.id != desc.id);
        state.active.push(MountedLora {
            descriptor: desc,
            strength,
            mounted_at_utc: Utc::now(),
            adapter,
        });
        Ok(())
    }

    async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?;
        let before = state.active.len();
        state.active.retain(|entry| entry.descriptor.id != id);
        if state.active.len() == before {
            return Err(lora_error(format!("unknown Candle LoRA id {id}")));
        }
        Ok(())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        self.state
            .lock()
            .map(|state| {
                state
                    .active
                    .iter()
                    .map(|entry| LoraStackEntry {
                        id: entry.descriptor.id,
                        strength: entry.strength.clone(),
                        mounted_at_utc: entry.mounted_at_utc,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn set_strength(
        &self,
        id: LoraId,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?;
        let Some(entry) = state
            .active
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
        else {
            return Err(lora_error(format!("unknown Candle LoRA id {id}")));
        };
        entry.strength = strength;
        Ok(())
    }

    async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        let mut replacement = Vec::with_capacity(new_stack.len());
        for (desc, strength) in new_stack {
            let adapter = self.load_adapter(&desc)?;
            replacement.push(MountedLora {
                descriptor: desc,
                strength,
                mounted_at_utc: Utc::now(),
                adapter,
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("Candle LoRA stack lock is poisoned"))?;
        let previous = Self::snapshot_locked(&state);
        state.active = replacement;
        Ok(previous)
    }
}

pub fn apply_lora_delta_to_linear_output(
    base_output: &Tensor,
    input: &Tensor,
    a: &Tensor,
    b: &Tensor,
    scale: f32,
    target: &str,
) -> Result<Tensor, ModelRuntimeError> {
    let (rank, in_dim) = a.dims2().map_err(|error| {
        lora_error(format!(
            "LoRA A tensor for {target} must be rank-2: {error}"
        ))
    })?;
    let (out_dim, b_rank) = b.dims2().map_err(|error| {
        lora_error(format!(
            "LoRA B tensor for {target} must be rank-2: {error}"
        ))
    })?;
    if rank != b_rank {
        return Err(lora_error(format!(
            "LoRA rank mismatch for {target}: A rank {rank}, B rank {b_rank}"
        )));
    }
    let dims = input.dims();
    let rows = match dims {
        [rows, actual_in] if *actual_in == in_dim => *rows,
        [batch, seq, actual_in] if *actual_in == in_dim => batch * seq,
        _ => {
            return Err(lora_error(format!(
                "LoRA input shape mismatch for {target}: expected last dim {in_dim}, got {dims:?}"
            )))
        }
    };
    let input_2d = input
        .reshape((rows, in_dim))
        .map_err(|error| lora_error(format!("LoRA input reshape failed for {target}: {error}")))?;
    let a = a
        .to_device(input.device())
        .and_then(|tensor| tensor.to_dtype(input.dtype()))
        .map_err(|error| {
            lora_error(format!(
                "LoRA A device/dtype cast failed for {target}: {error}"
            ))
        })?;
    let b = b
        .to_device(input.device())
        .and_then(|tensor| tensor.to_dtype(input.dtype()))
        .map_err(|error| {
            lora_error(format!(
                "LoRA B device/dtype cast failed for {target}: {error}"
            ))
        })?;
    let delta = input_2d
        .matmul(&a.t().map_err(|error| {
            lora_error(format!("LoRA A transpose failed for {target}: {error}"))
        })?)
        .and_then(|tensor| tensor.matmul(&b.t()?))
        .and_then(|tensor| (tensor * scale as f64))
        .map_err(|error| lora_error(format!("LoRA delta matmul failed for {target}: {error}")))?;
    let delta = match dims {
        [_, _] => delta,
        [batch, seq, _] => delta.reshape((*batch, *seq, out_dim)).map_err(|error| {
            lora_error(format!("LoRA delta reshape failed for {target}: {error}"))
        })?,
        _ => unreachable!("input rank checked above"),
    };
    (base_output + delta).map_err(|error| {
        lora_error(format!(
            "LoRA delta add failed for {target}; base shape {:?}: {error}",
            base_output.dims()
        ))
    })
}

impl PeftAdapterConfig {
    fn target_modules(&self) -> Option<Vec<String>> {
        let value = self.target_modules.as_ref()?;
        match value {
            Value::String(target) => {
                let trimmed = target.trim();
                (!trimmed.is_empty()).then(|| vec![trimmed.to_string()])
            }
            Value::Array(targets) => {
                let parsed = targets
                    .iter()
                    .filter_map(|target| target.as_str())
                    .map(str::trim)
                    .filter(|target| !target.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                (!parsed.is_empty()).then_some(parsed)
            }
            _ => None,
        }
    }
}

fn load_peft_adapter_config(path: &Path) -> Result<PeftAdapterConfig, ModelRuntimeError> {
    let Some(parent) = path.parent() else {
        return Ok(PeftAdapterConfig::default());
    };
    let config_path = parent.join("adapter_config.json");
    if !config_path.exists() {
        return Ok(PeftAdapterConfig::default());
    }
    let bytes = std::fs::read(&config_path).map_err(|error| {
        lora_error(format!(
            "failed to read PEFT adapter config {}: {error}",
            config_path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        lora_error(format!(
            "failed to parse PEFT adapter config {}: {error}",
            config_path.display()
        ))
    })
}

fn parse_lora_tensor_key(key: &str) -> Option<(&str, LoraTensorKind)> {
    for (suffix, kind) in [
        (".lora_A.default.weight", LoraTensorKind::A),
        (".lora_B.default.weight", LoraTensorKind::B),
        (".lora_A.weight", LoraTensorKind::A),
        (".lora_B.weight", LoraTensorKind::B),
    ] {
        if let Some(target) = key.strip_suffix(suffix) {
            return Some((target, kind));
        }
    }
    None
}

fn normalize_peft_target(target: &str) -> String {
    let mut normalized = target
        .strip_prefix("base_model.model.")
        .unwrap_or(target)
        .to_string();
    while let Some(stripped) = normalized.strip_prefix("model.model.layers.") {
        normalized = format!("model.layers.{stripped}");
    }
    normalized
}

fn validate_lora_shapes(
    expected_rank: u32,
    target: &str,
    a: &Tensor,
    b: &Tensor,
) -> Result<(), ModelRuntimeError> {
    let (rank, _in_dim) = a.dims2().map_err(|error| {
        lora_error(format!(
            "LoRA A tensor for {target} must be rank-2: {error}"
        ))
    })?;
    let (_out_dim, b_rank) = b.dims2().map_err(|error| {
        lora_error(format!(
            "LoRA B tensor for {target} must be rank-2: {error}"
        ))
    })?;
    if rank != b_rank || rank as u32 != expected_rank {
        return Err(lora_error(format!(
            "LoRA rank mismatch for {target}: descriptor={}, A={}, B={}",
            expected_rank, rank, b_rank
        )));
    }
    Ok(())
}

fn validate_sha256(path: &Path, expected: [u8; 32]) -> Result<(), ModelRuntimeError> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|error| {
        lora_error(format!(
            "failed to read LoRA artifact {}: {error}",
            path.display()
        ))
    })?;
    let actual: [u8; 32] = Sha256::digest(&bytes).into();
    if actual != expected {
        return Err(lora_error(format!(
            "LoRA artifact sha256 mismatch for {}",
            path.display()
        )));
    }
    Ok(())
}

fn lora_error(message: impl Into<String>) -> ModelRuntimeError {
    ModelRuntimeError::LoraStackError(message.into())
}
