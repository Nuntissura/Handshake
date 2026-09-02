//! WP-KERNEL-005 MT-160 / MT-163 / MT-169: embedded SurrealDB round-trip proofs
//! for the typed Model-Workflow-Diagnostics runtime surfaces.
//!
//! These MTs are TYPED RUNTIME surfaces (persisted rows + EventLedger events),
//! never governance markdown:
//!   * MT-160 -- a governed local/remote OpenAI-compatible model config whose
//!     api key is stored ONLY as a redacted ref and never echoed into an event.
//!   * MT-163 -- the draft/preview/validate/apply/reject/rollback apply state
//!     machine; legal transitions succeed, illegal ones are rejected.
//!   * MT-169 -- the synthetic-input guard: an authorized op records a row;
//!     an unauthorized op is rejected (but still leaves an audit row).
//!
//! The isolated harness supplies the canonical schema for every test.

mod atelier_surreal_support;

use handshake_core::atelier::settings::{
    model_workflow_event_family, ModelApplyState, NewModelConfig, NewSyntheticInput,
    SyntheticInputOp,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::storage::Database;
use uuid::Uuid;

/// Create the shared isolated embedded-store preamble every test runs against.
async fn connected_store() -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness)
}

/// MT-160: a governed model config round-trips through the embedded store with the api
/// key stored ONLY as a redacted ref, and the emitted event never contains the
/// raw key.
#[tokio::test]
async fn mt160_model_config_round_trips_with_redacted_api_key() {
    let (store, harness) = connected_store().await;

    let config_id = format!("model-config-{}", Uuid::now_v7());
    let raw_api_key = "sk-super-secret-key-value-do-not-leak";
    let new = NewModelConfig {
        config_id: config_id.clone(),
        base_url: "model-config://providers/openai-compatible".to_string(),
        model: "gpt-oss-20b".to_string(),
        api_key: raw_api_key.to_string(),
        system_prompt: "You are a governed Handshake assistant.".to_string(),
        timeout_ms: 30_000,
    };

    let recorded = store
        .record_model_config(&new)
        .await
        .expect("record model config");

    // The raw key is never stored: only a redacted sha256 ref.
    assert!(
        !recorded.api_key_ref.contains(raw_api_key),
        "api_key_ref must not contain the raw key"
    );
    assert!(
        recorded.api_key_ref.starts_with("sha256:"),
        "api_key_ref must be a redacted sha256 handle, got {}",
        recorded.api_key_ref
    );

    // Round-trips through the embedded store unchanged.
    let reloaded = store
        .get_model_config(&config_id)
        .await
        .expect("get model config")
        .expect("model config must exist");
    assert_eq!(reloaded, recorded, "model config must round-trip");
    assert_eq!(reloaded.base_url, new.base_url);
    assert_eq!(reloaded.model, new.model);
    assert_eq!(reloaded.system_prompt, new.system_prompt);
    assert_eq!(reloaded.timeout_ms, 30_000);

    // The recorded event redacts the api key in its payload.
    let count = store
        .count_events_for_aggregate(
            model_workflow_event_family::MODEL_CONFIG_RECORDED,
            "atelier_model_config",
            &config_id,
        )
        .await
        .expect("count model-config events");
    assert_eq!(count, 1, "exactly one MODEL_CONFIG_RECORDED event");

    let raw_secret = raw_api_key.to_string();
    let events = harness
        .database
        .list_kernel_events_for_aggregate("atelier_model_config", &config_id)
        .await
        .expect("scan event payload for leaked secret");
    assert!(
        events
            .iter()
            .all(|event| !event.payload.to_string().contains(&raw_secret)),
        "raw api key must never appear in the event payload"
    );
}

/// MT-163: the apply state machine accepts the legal transition chain
/// (DRAFT->PREVIEW->VALIDATED->APPLIED->ROLLED_BACK) and rejects illegal
/// transitions (e.g. DRAFT->APPLIED, and any transition out of a terminal
/// state).
#[tokio::test]
async fn mt163_apply_state_machine_enforces_legal_transitions() {
    let (store, _harness) = connected_store().await;

    // ---- legal chain succeeds ----
    let apply_id = format!("model-apply-{}", Uuid::now_v7());
    let drafted = store
        .draft_model_apply(&apply_id, "model-suggestion://patch/abc123")
        .await
        .expect("draft apply");
    assert_eq!(drafted.state, ModelApplyState::Draft);

    let previewed = store
        .advance_apply_state(&apply_id, ModelApplyState::Preview, None)
        .await
        .expect("DRAFT->PREVIEW must be legal");
    assert_eq!(previewed.state, ModelApplyState::Preview);

    let validated = store
        .advance_apply_state(&apply_id, ModelApplyState::Validated, None)
        .await
        .expect("PREVIEW->VALIDATED must be legal");
    assert_eq!(validated.state, ModelApplyState::Validated);

    let applied = store
        .advance_apply_state(
            &apply_id,
            ModelApplyState::Applied,
            Some("model-apply-evidence://run/xyz"),
        )
        .await
        .expect("VALIDATED->APPLIED must be legal");
    assert_eq!(applied.state, ModelApplyState::Applied);
    assert_eq!(
        applied.evidence_ref.as_deref(),
        Some("model-apply-evidence://run/xyz"),
        "evidence_ref must be recorded on the row"
    );

    let rolled_back = store
        .advance_apply_state(&apply_id, ModelApplyState::RolledBack, None)
        .await
        .expect("APPLIED->ROLLED_BACK must be legal");
    assert_eq!(rolled_back.state, ModelApplyState::RolledBack);

    // ---- illegal: out of a terminal state ----
    let err = store
        .advance_apply_state(&apply_id, ModelApplyState::Applied, None)
        .await
        .expect_err("ROLLED_BACK->APPLIED must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "illegal transition must produce a Validation error, got {err:?}"
    );

    // ---- illegal: skip-ahead from a fresh DRAFT ----
    let skip_id = format!("model-apply-{}", Uuid::now_v7());
    store
        .draft_model_apply(&skip_id, "model-suggestion://patch/skip")
        .await
        .expect("draft skip apply");
    let skip_err = store
        .advance_apply_state(&skip_id, ModelApplyState::Applied, None)
        .await
        .expect_err("DRAFT->APPLIED must be rejected");
    assert!(
        matches!(skip_err, AtelierError::Validation(_)),
        "skip-ahead transition must produce a Validation error, got {skip_err:?}"
    );

    // ---- legal: any non-terminal state may be rejected ----
    let reject_id = format!("model-apply-{}", Uuid::now_v7());
    store
        .draft_model_apply(&reject_id, "model-suggestion://patch/reject")
        .await
        .expect("draft reject apply");
    let rejected = store
        .advance_apply_state(&reject_id, ModelApplyState::Rejected, None)
        .await
        .expect("DRAFT->REJECTED must be legal");
    assert_eq!(rejected.state, ModelApplyState::Rejected);
}

/// MT-169: an authorized synthetic-input op records a governed row; an
/// unauthorized op is rejected by the guard but still leaves an audit row.
#[tokio::test]
async fn mt169_synthetic_input_guard_records_and_rejects_unauthorized() {
    let (store, _harness) = connected_store().await;

    // ---- authorized op records and passes the guard ----
    let authorized = NewSyntheticInput {
        op: SyntheticInputOp::TypeText,
        target_ref: "synthetic-input-target://panel/search-field".to_string(),
        authorized: true,
    };
    let record = store
        .guard_synthetic_input(&authorized)
        .await
        .expect("authorized synthetic input must pass the guard");
    assert_eq!(record.op, SyntheticInputOp::TypeText);
    assert!(record.authorized, "recorded op must be authorized");

    let reloaded_count = store
        .count_events_for_aggregate(
            model_workflow_event_family::SYNTHETIC_INPUT_RECORDED,
            "atelier_synthetic_input_guard",
            &record.guard_id,
        )
        .await
        .expect("count synthetic-input events");
    assert_eq!(reloaded_count, 1, "authorized op must emit one audit event");

    // ---- unauthorized op is rejected by the guard ----
    let unauthorized = NewSyntheticInput {
        op: SyntheticInputOp::InjectKey,
        target_ref: "synthetic-input-target://panel/danger-field".to_string(),
        authorized: false,
    };
    let audit_events_before = store
        .count_events(model_workflow_event_family::SYNTHETIC_INPUT_RECORDED)
        .await
        .expect("count synthetic-input audit events before rejection");
    let err = store
        .guard_synthetic_input(&unauthorized)
        .await
        .expect_err("unauthorized synthetic input must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "unauthorized op must produce a Validation error, got {err:?}"
    );

    // ...but the rejection still left a governed, auditable row (record before
    // reject), so unauthorized synthetic input is never silent.
    let audit_events_after = store
        .count_events(model_workflow_event_family::SYNTHETIC_INPUT_RECORDED)
        .await
        .expect("count synthetic-input audit events after rejection");
    assert!(
        audit_events_after > audit_events_before,
        "an unauthorized op must still leave a governed audit row"
    );
}
