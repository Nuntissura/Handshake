//! MT-109 — INF-8 Self-Speculative Decoding technique-surface tests.
//!
//! Adversarial scenarios:
//!   - default None acceptable (DisabledOk)
//!   - Ngram + DraftModel accepted when supports_speculative_draft=true
//!   - Ngram + DraftModel rejected when supports_speculative_draft=false
//!     (e.g., CandleRuntime + SSMs)
//!   - Eagle3 rejected as deferred even when supports_speculative_draft=true
//!     (until adapter signals supports_eagle3=true)
//!   - Eagle3 accepted when adapter signals supports_eagle3=true
//!     (forward-compat smoke for the day llama.cpp PR #18039 merges)
//!   - mode-change receipt carries the canonical event_type string
//!   - FR registry round-trip for SPEC-MODE-CHANGE + SPEC-STATS
//!
//! The technique surface is engine-agnostic; tests build a stub
//! ModelRuntime that returns a hand-crafted ModelCapabilities and
//! validates dispatch on that.

use std::collections::HashMap;

use async_trait::async_trait;
use futures_util::stream;
use handshake_core::flight_recorder::fr_event_registry::FrEventId;
use handshake_core::model_runtime::techniques::speculative_decoding::{
    self, build_mode_change_receipt, validate_mode, SpeculativeModeValidation,
    FR_EVT_LLM_INFER_SPEC_MODE_CHANGE,
};
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec, LoraStackHandle,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, Score, SpeculativeMode,
    SteeringHookHandle, TokenStream,
};

#[test]
fn spec_validate_mode_none_is_always_disabled_ok() {
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(true, false));
    let outcome = validate_mode(&runtime, model_id, None).expect("None validates");
    assert!(matches!(outcome, SpeculativeModeValidation::DisabledOk));
}

#[test]
fn spec_validate_mode_ngram_accepted_when_supports_speculative_draft_true() {
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(true, false));
    let mode = SpeculativeMode::Ngram {
        lookback: 32,
        max_draft: 8,
    };
    let outcome = validate_mode(&runtime, model_id, Some(&mode)).expect("ngram accepted");
    assert!(matches!(
        outcome,
        SpeculativeModeValidation::Accepted { .. }
    ));
}

#[test]
fn spec_validate_mode_draft_model_accepted_when_supports_speculative_draft_true() {
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(true, false));
    let mode = SpeculativeMode::DraftModel {
        draft_id: ModelId::new_v7(),
        max_draft: 8,
    };
    let outcome = validate_mode(&runtime, model_id, Some(&mode)).expect("draft accepted");
    assert!(matches!(
        outcome,
        SpeculativeModeValidation::Accepted { .. }
    ));
}

#[test]
fn spec_validate_mode_ngram_rejected_when_supports_speculative_draft_false() {
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(false, false));
    let mode = SpeculativeMode::Ngram {
        lookback: 32,
        max_draft: 8,
    };
    let err = validate_mode(&runtime, model_id, Some(&mode))
        .expect_err("supports_speculative_draft=false must reject");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
                if capability == "speculative_decoding"
        ),
        "expected CapabilityNotSupported{{speculative_decoding}}, got {err:?}"
    );
}

#[test]
fn spec_validate_mode_eagle3_marked_deferred_until_supports_eagle3_true() {
    // Even when supports_speculative_draft=true, Eagle3 must reject as
    // deferred until the adapter explicitly signals supports_eagle3.
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(true, false));
    let mode = SpeculativeMode::Eagle3 { max_draft: 4 };
    let err = validate_mode(&runtime, model_id, Some(&mode))
        .expect_err("Eagle3 must be deferred until supports_eagle3=true");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
                if capability == "eagle3_deferred"
        ),
        "expected CapabilityNotSupported{{eagle3_deferred}}, got {err:?}"
    );
}

#[test]
fn spec_validate_mode_eagle3_accepted_when_adapter_signals_supports_eagle3_true() {
    // Forward-compat: the day llama.cpp PR #18039 merges and the
    // adapter flips supports_eagle3=true, the same technique surface
    // must accept Eagle3 without any code change here.
    let model_id = ModelId::new_v7();
    let runtime = StubRuntime::new(model_id, capabilities_with_spec(true, true));
    let mode = SpeculativeMode::Eagle3 { max_draft: 4 };
    let outcome = validate_mode(&runtime, model_id, Some(&mode))
        .expect("Eagle3 must be accepted when supports_eagle3=true");
    assert!(matches!(
        outcome,
        SpeculativeModeValidation::Accepted { .. }
    ));
}

#[test]
fn spec_build_mode_change_receipt_carries_canonical_event_type() {
    let model_id = ModelId::new_v7();
    let receipt = build_mode_change_receipt(
        model_id,
        None,
        Some(SpeculativeMode::Ngram {
            lookback: 16,
            max_draft: 4,
        }),
    );
    assert_eq!(receipt.event_type, FR_EVT_LLM_INFER_SPEC_MODE_CHANGE);
    assert_eq!(receipt.model_id, model_id);
    assert!(receipt.previous_mode.is_none());
    assert!(matches!(
        receipt.current_mode,
        Some(SpeculativeMode::Ngram { .. })
    ));
}

#[test]
fn spec_fr_registry_round_trips_canonical_event_ids() {
    // FR-EVT-LLM-INFER-SPEC-MODE-CHANGE + SPEC-STATS are technique-owned
    // string constants; SPEC-ACCEPT / SPEC-REJECT are already in the FR
    // registry from MT-077. Verify the two new event_type constants
    // referenced by the technique surface have stable spellings.
    assert_eq!(
        FR_EVT_LLM_INFER_SPEC_MODE_CHANGE,
        "FR-EVT-LLM-INFER-SPEC-MODE-CHANGE"
    );
    assert_eq!(
        speculative_decoding::FR_EVT_LLM_INFER_SPEC_STATS,
        "FR-EVT-LLM-INFER-SPEC-STATS"
    );
    // Round-trip the already-registered SPEC-ACCEPT through FrEventId
    // so the test catches drift if MT-077's variants get renamed.
    let parsed = FrEventId::from_str_id(speculative_decoding::FR_EVT_LLM_INFER_SPEC_ACCEPT)
        .expect("FR registry must accept SPEC-ACCEPT");
    assert_eq!(
        parsed.as_str(),
        speculative_decoding::FR_EVT_LLM_INFER_SPEC_ACCEPT
    );
}

// ----------------------------------------------------------------------------
// Stub runtime.
// ----------------------------------------------------------------------------

fn capabilities_with_spec(
    supports_speculative_draft: bool,
    supports_eagle3: bool,
) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: false,
        supports_kv_prefix_cache: false,
        supports_kv_quantization: Default::default(),
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft,
        supports_eagle3,
    }
}

struct StubRuntime {
    model_capabilities: HashMap<ModelId, ModelCapabilities>,
}

impl StubRuntime {
    fn new(model_id: ModelId, capabilities: ModelCapabilities) -> Self {
        Self {
            model_capabilities: HashMap::from([(model_id, capabilities)]),
        }
    }
}

#[async_trait]
impl ModelRuntime for StubRuntime {
    fn adapter_name(&self) -> &'static str {
        "spec_decoding_stub"
    }

    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Err(ModelRuntimeError::LoadError(
            "stub runtime does not load models".to_string(),
        ))
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.model_capabilities
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::LoadError(format!("unknown model {id}")))
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("spec-stub-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("spec-stub-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("spec-stub-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}
