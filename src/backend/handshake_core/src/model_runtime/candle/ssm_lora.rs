//! MT-115 — INF-9 LoRA-for-SSM validation surface.
//!
//! Per refinement INF-9 feature_parity_detail ("LoRA mount/unmount for SSMs
//! (LoRA-for-SSM work)") + operator E-2 FULL FEATURE PARITY.
//!
//! Scope discipline: this MT lands the validation surface — the
//! `SSMArchitectureTag` enum, the per-architecture target-module name
//! lists, the validator that rejects mismatched target_modules, the
//! PEFT scaling formula constant — and the typed deferral marker for
//! the actual weight-application path. The mount/unmount surface that
//! threads the LoRA delta through each variant's candle forward pass
//! lands in a follow-on MT once the candle-transformers Mamba2 / RWKV
//! model wrappers expose hookable weight slots (today they don't —
//! tracked in cross_handle_finding-style note on the follow-on MT).
//!
//! This mirrors MT-111's EAGLE-3 upgrade-hook pattern: scaffold the
//! upgrade path now, flip `capabilities.supports_lora=true` for SSM
//! variants in the follow-on MT when the actual delta application
//! lands.
//!
//! Per-architecture target module names (research-frontier):
//! - Mamba2  = `{in_proj, x_proj, dt_proj, out_proj}` (Linear
//!             projections — the SSM A/D/conv parameters are NOT good
//!             LoRA targets due to scalar/diagonal structure).
//! - RWKV v5 = `{time_mix.*.weight, channel_mix.*.weight}` family
//!             (Linear projections in the time-mix / channel-mix
//!             blocks; analogous to attention/MLP split).
//! - RWKV v6 = same as v5 (v6 reorganises decay tensors but keeps the
//!             time-mix / channel-mix Linear surface).
//! - RWKV v7 = same as v5/v6 (v7 Goose/G1 adjusts time-mix decay-vector
//!             eval; LoRA-targetable Linears stay in time-mix /
//!             channel-mix).
//!
//! PEFT scaling formula (uniform with transformer LoRA from MT-064/MT-084):
//!   y = base.forward(x) + scaling * strength * (B @ (A @ x))
//!   scaling = lora_alpha / rank (operator binds lora_alpha at mount
//!     time; rank is the LoraDescriptor.rank field).

use serde::{Deserialize, Serialize};

use crate::model_runtime::ModelRuntimeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SSMArchitectureTag {
    Mamba2,
    RwkvV5,
    RwkvV6,
    RwkvV7,
}

impl SSMArchitectureTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mamba2 => "mamba2",
            Self::RwkvV5 => "rwkv_v5",
            Self::RwkvV6 => "rwkv_v6",
            Self::RwkvV7 => "rwkv_v7",
        }
    }

    pub fn valid_target_modules(&self) -> &'static [&'static str] {
        match self {
            Self::Mamba2 => MAMBA2_VALID_TARGET_MODULES,
            Self::RwkvV5 | Self::RwkvV6 | Self::RwkvV7 => RWKV_VALID_TARGET_MODULES,
        }
    }
}

/// Mamba2 LoRA-targetable Linear projections per the contract narrative.
/// The SSM A/D/conv parameters are intentionally excluded — their
/// scalar/diagonal structure makes them unsuitable PEFT targets.
pub const MAMBA2_VALID_TARGET_MODULES: &[&str] = &["in_proj", "x_proj", "dt_proj", "out_proj"];

/// RWKV v5/v6/v7 LoRA-targetable module name patterns. Operator-supplied
/// target_modules entries are matched as either the exact name or as a
/// `{module}.{layer_index}.weight` form; the validator accepts either
/// shape and the per-layer expansion lands in the follow-on weight-
/// application MT.
pub const RWKV_VALID_TARGET_MODULES: &[&str] = &[
    "time_mix.key.weight",
    "time_mix.value.weight",
    "time_mix.receptance.weight",
    "time_mix.output.weight",
    "channel_mix.key.weight",
    "channel_mix.value.weight",
    "channel_mix.receptance.weight",
];

/// PEFT scaling formula doc-string — single source of truth so the
/// follow-on MT (weight application) and the operator-facing manual
/// stay aligned.
pub const SSM_LORA_PEFT_FORMULA: &str =
    "y = base.forward(x) + scaling * strength * (B @ (A @ x)); scaling = lora_alpha / rank";

/// Deferred-to-follow-on-MT marker. The mount() call returns this until
/// the actual weight-application MT lands and flips
/// `capabilities.supports_lora = true` for SSM variants.
pub const SSM_LORA_MOUNT_DEFERRED_MARKER: &str = "ssm_lora_mount_disabled_pending_followon";

/// Validate that the operator-supplied target_modules are recognised
/// for the given SSM architecture. The validator accepts:
/// - Exact-name matches against the architecture's valid_target_modules
///   list (e.g., "in_proj" for Mamba2).
/// - Per-layer expansions of the form `{module}.{layer_index}.weight`
///   for RWKV families (e.g., "time_mix.key.weight" → also accepts
///   "time_mix.0.key.weight").
///
/// Returns `Ok(())` when every entry matches a recognised pattern; an
/// `Err(ModelRuntimeError::LoraStackError)` listing the offending name
/// when one or more entries are unrecognised. This is the "Architecture
/// tag validation enforced" + "Per-architecture target-module name
/// list explicit" red_team minimum control.
pub fn validate_target_modules_for_architecture(
    arch: SSMArchitectureTag,
    target_modules: &[String],
) -> Result<(), ModelRuntimeError> {
    if target_modules.is_empty() {
        return Err(ModelRuntimeError::LoraStackError(
            "LoRA target_modules must not be empty for SSM architectures".to_string(),
        ));
    }
    let valid = arch.valid_target_modules();
    let mut rejected: Vec<String> = Vec::new();
    for module in target_modules {
        let trimmed = module.trim();
        if trimmed.is_empty() {
            rejected.push(module.clone());
            continue;
        }
        let matches_exact = valid.iter().any(|expected| trimmed == *expected);
        let matches_layered = matches_layer_indexed_form(trimmed, valid);
        if !matches_exact && !matches_layered {
            rejected.push(module.clone());
        }
    }
    if rejected.is_empty() {
        Ok(())
    } else {
        Err(ModelRuntimeError::LoraStackError(format!(
            "LoRA target_modules {rejected:?} are not valid for SSM architecture {} (valid: {valid:?})",
            arch.as_str()
        )))
    }
}

/// Convenience: return the deferral-marker error for the mount/unmount
/// path until the weight-application MT lands.
pub fn ssm_lora_mount_deferred_error(arch: SSMArchitectureTag) -> ModelRuntimeError {
    ModelRuntimeError::CapabilityNotSupported {
        capability: SSM_LORA_MOUNT_DEFERRED_MARKER.to_string(),
        adapter: format!("candle_{}", arch.as_str()),
    }
}

fn matches_layer_indexed_form(candidate: &str, valid: &[&str]) -> bool {
    // Accept `module.{N}.weight` where the bare `module.weight` form is
    // in the valid list (covers the RWKV per-layer expansion case).
    // Example: candidate = "time_mix.0.key.weight"; valid contains
    // "time_mix.key.weight" → match.
    for expected in valid {
        let Some((prefix, suffix)) = expected.rsplit_once('.') else {
            continue;
        };
        // expected = "{prefix}.{suffix}", e.g., prefix="time_mix.key",
        // suffix="weight". Accept candidate forms where the layer index
        // is injected between prefix and suffix.
        let Some((cand_prefix, cand_suffix)) = candidate.rsplit_once('.') else {
            continue;
        };
        if cand_suffix != suffix {
            continue;
        }
        // cand_prefix should be `{base}.{index}` where prefix is split
        // into base + the same final component as suffix. We re-derive
        // `base` by splitting prefix on the LAST `.`.
        let Some((base_prefix, last_component)) = prefix.rsplit_once('.') else {
            // Prefix is single-token; candidate prefix must be of the
            // form `{prefix}.{index}` for a numeric N.
            if let Some((cand_base, cand_index)) = cand_prefix.rsplit_once('.') {
                if cand_base == prefix && cand_index.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
            continue;
        };
        // Expected prefix = "{base_prefix}.{last_component}" — candidate
        // must reorder to "{base_prefix}.{index}.{last_component}".
        let synthesized = format!("{base_prefix}.");
        if let Some(remaining) = cand_prefix.strip_prefix(&synthesized) {
            if let Some((index, tail)) = remaining.rsplit_once('.') {
                if tail == last_component && index.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
        }
    }
    false
}
