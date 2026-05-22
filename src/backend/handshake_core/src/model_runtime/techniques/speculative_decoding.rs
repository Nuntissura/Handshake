//! MT-109 — INF-8 Self-Speculative Decoding public technique surface.
//!
//! Lifts the per-adapter speculative-decoding implementation (currently
//! `LlamaCppRuntime` via MT-077; `CandleRuntime` reports
//! `supports_speculative_draft=false`) into a ModelRuntime-level technique
//! API for the Inference Lab Work Profile knob
//! `settings.exec_policy.speculative`.
//!
//! Per operator E-4 + AC-INFER-LAB-8-TECHNIQUES.g + AC-MODEL-CAP-DECL:
//! - Conservative default: `settings.exec_policy.speculative: Option<SpeculativeMode>`
//!   defaults to `None` (disabled). Operator must opt-in per Work Profile.
//! - Eagle3 sub-toggle is structurally present but always rejected with
//!   `CapabilityNotSupported{ capability: "eagle3_deferred" }` until
//!   llama.cpp PR #18039 merges and the adapter signals
//!   `supports_eagle3=true`.
//! - Ngram + DraftModel modes pass through to the per-adapter
//!   speculative-plan validator when the adapter declares
//!   `supports_speculative_draft=true`.
//!
//! This MT lands the validation surface + the canonical FR event
//! constants. The override-store + stats-reader live in the Tauri
//! layer (commands/speculative.rs) so the technique module stays
//! pure and engine-agnostic. Wiring the operator's chosen mode into
//! the per-request GenerateRequest is deferred to a follow-up MT
//! (the MT-077 GenerateRequest path already accepts the mode; only
//! the Work Profile -> request bridge is missing).

use serde::{Deserialize, Serialize};

use crate::model_runtime::{ModelId, ModelRuntime, ModelRuntimeError, SpeculativeMode};

/// FR event id for "draft accepted" — already present in the FR
/// registry as `FrEventId::LlmInferSpecAccept` from MT-077. The
/// technique surface re-exposes the canonical string spelling so
/// downstream (Tauri, frontend) does not need to enumerate
/// FrEventId variants directly.
pub const FR_EVT_LLM_INFER_SPEC_ACCEPT: &str = "FR-EVT-LLM-INFER-SPEC-ACCEPT";
pub const FR_EVT_LLM_INFER_SPEC_REJECT: &str = "FR-EVT-LLM-INFER-SPEC-REJECT";

/// FR event id for "operator changed the saved speculative mode".
/// Sampled per change (not per token), so emission is naturally bounded.
pub const FR_EVT_LLM_INFER_SPEC_MODE_CHANGE: &str = "FR-EVT-LLM-INFER-SPEC-MODE-CHANGE";

/// FR event id for per-request stats snapshot. The MT-109 contract calls
/// for this to be SAMPLED (red_team minimum control: "FR stats sampled —
/// no per-token flood"). The technique surface declares the constant;
/// the actual sampling decision is owned by the generate path (MT-077).
pub const FR_EVT_LLM_INFER_SPEC_STATS: &str = "FR-EVT-LLM-INFER-SPEC-STATS";

/// Outcome of `validate_mode` — whether the operator's chosen mode is
/// acceptable for the model's adapter + declared capabilities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeculativeModeValidation {
    /// `None` was supplied — speculative decoding disabled. Always
    /// acceptable (the conservative default).
    DisabledOk,
    /// `Some(Ngram | DraftModel)` was supplied and the adapter declares
    /// `supports_speculative_draft=true`. The technique surface accepts
    /// the mode; the generate path will validate per-request shape
    /// constraints (max_draft bounds, draft_id existence, etc.).
    Accepted { mode: SpeculativeMode },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeculativeModeChangeReceipt {
    pub model_id: ModelId,
    pub event_type: String,
    pub previous_mode: Option<SpeculativeMode>,
    pub current_mode: Option<SpeculativeMode>,
}

/// Validate that the operator's chosen speculative mode is acceptable
/// for the given model. Routes through the model's declared
/// capabilities so the UI knob can show / hide / mark-deferred per
/// adapter without the technique surface owning adapter-specific
/// knowledge.
///
/// Semantics:
/// - `None` → always `DisabledOk`
/// - `Some(Eagle3 { .. })` → always `CapabilityNotSupported { capability:
///   "eagle3_deferred", .. }` until the adapter signals
///   `supports_eagle3 = true`
/// - `Some(Ngram | DraftModel)` requires `supports_speculative_draft=true`;
///   otherwise `CapabilityNotSupported { capability:
///   "speculative_decoding", .. }`
pub fn validate_mode(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    mode: Option<&SpeculativeMode>,
) -> Result<SpeculativeModeValidation, ModelRuntimeError> {
    let Some(mode) = mode else {
        return Ok(SpeculativeModeValidation::DisabledOk);
    };
    let capabilities = runtime.capabilities(model_id)?;
    match mode {
        SpeculativeMode::Eagle3 { .. } => {
            if !capabilities.supports_eagle3 {
                return Err(ModelRuntimeError::CapabilityNotSupported {
                    capability: "eagle3_deferred".to_string(),
                    adapter: runtime.adapter_name().to_string(),
                });
            }
            Ok(SpeculativeModeValidation::Accepted { mode: mode.clone() })
        }
        SpeculativeMode::Ngram { .. } | SpeculativeMode::DraftModel { .. } => {
            if !capabilities.supports_speculative_draft {
                return Err(ModelRuntimeError::CapabilityNotSupported {
                    capability: "speculative_decoding".to_string(),
                    adapter: runtime.adapter_name().to_string(),
                });
            }
            Ok(SpeculativeModeValidation::Accepted { mode: mode.clone() })
        }
    }
}

/// Build the mode-change receipt the IPC layer returns to the frontend
/// when the operator updates the Work Profile knob. The technique
/// surface owns the FR event_type string so it stays consistent
/// across IPC, FR, and future wire-format projections.
pub fn build_mode_change_receipt(
    model_id: ModelId,
    previous_mode: Option<SpeculativeMode>,
    current_mode: Option<SpeculativeMode>,
) -> SpeculativeModeChangeReceipt {
    SpeculativeModeChangeReceipt {
        model_id,
        event_type: FR_EVT_LLM_INFER_SPEC_MODE_CHANGE.to_string(),
        previous_mode,
        current_mode,
    }
}
