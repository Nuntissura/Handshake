//! MT-128 cross-crate integration smoke for the cloud SecretsVault
//! + ConsentGate scaffolds. Exhaustive coverage lives in the inline
//! tests in `model_runtime::cloud::secrets_vault::tests` +
//! `model_runtime::cloud::consent_gate::tests`; this file pins the
//! cross-crate API + the red_team minimum_controls + the wire-up to
//! the BYOK runtimes from MT-125 / MT-126.

use std::sync::{Arc, Mutex};

use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, CloudInvocationAuditSink, ConsentDecision, ConsentGate, ConsentGateError,
    ConsentProvider, InMemorySecretsVault, OpenAiByokError, OpenAiByokRuntime, SecretsVault,
    VaultApiKeyProvider,
};

#[derive(Default)]
struct NopSink;
impl CloudInvocationAuditSink for NopSink {
    fn record(
        &self,
        _row: handshake_core::model_runtime::cloud::CloudInvocationAuditRow,
    ) -> Result<(), OpenAiByokError> {
        Ok(())
    }
}

#[test]
fn secrets_vault_round_trips_secret_value() {
    let vault = InMemorySecretsVault::default();
    vault
        .put("openai", "sk-test-round-trip".to_string())
        .unwrap();
    let value = vault.get("openai").expect("get");
    assert_eq!(value, "sk-test-round-trip");
}

#[test]
fn vault_api_key_provider_wires_into_openai_byok_runtime() {
    // MT-128 red_team: end-to-end VaultApiKeyProvider plugs into
    // the BYOK runtime constructor without leaking the key into
    // Debug output.
    let vault = Arc::new(InMemorySecretsVault::default());
    vault
        .put("openai-lane", "sk-DO-NOT-LOG".to_string())
        .unwrap();
    let provider: Arc<dyn ApiKeyProvider> =
        Arc::new(VaultApiKeyProvider::new(vault.clone(), "openai-lane"));
    let runtime = OpenAiByokRuntime::new(
        "https://api.openai.com/v1",
        provider,
        Arc::new(NopSink::default()),
    );
    let dbg = format!("{runtime:?}");
    assert!(!dbg.contains("sk-DO-NOT-LOG"));
}

struct AlwaysApprove {
    prompts: Mutex<u32>,
}
impl ConsentProvider for AlwaysApprove {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        *self.prompts.lock().unwrap() += 1;
        Ok(ConsentDecision::Approved)
    }
}

struct AlwaysDeny;
impl ConsentProvider for AlwaysDeny {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Denied)
    }
}

#[test]
fn consent_gate_first_call_prompts_then_caches() {
    let gate = ConsentGate::new();
    let provider = AlwaysApprove {
        prompts: Mutex::new(0),
    };
    gate.check_or_prompt("session-A", "openai", &provider)
        .unwrap();
    gate.check_or_prompt("session-A", "openai", &provider)
        .unwrap();
    assert_eq!(*provider.prompts.lock().unwrap(), 1);
}

#[test]
fn consent_gate_denial_short_circuits_subsequent_calls() {
    let gate = ConsentGate::new();
    let provider = AlwaysDeny;
    let err = gate
        .check_or_prompt("session-A", "openai", &provider)
        .expect_err("denied");
    assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
    let err = gate
        .check_or_prompt("session-A", "openai", &provider)
        .expect_err("cached denial");
    assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
}

#[test]
fn consent_gate_is_per_session_per_lane_not_global() {
    let gate = ConsentGate::new();
    let approved = AlwaysApprove {
        prompts: Mutex::new(0),
    };
    gate.check_or_prompt("session-A", "openai", &approved)
        .unwrap();
    let denied = AlwaysDeny;
    let err = gate
        .check_or_prompt("session-A", "anthropic", &denied)
        .expect_err("different lane denies");
    assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
    let err = gate
        .check_or_prompt("session-B", "openai", &denied)
        .expect_err("different session denies");
    assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
}

#[test]
fn consent_gate_drop_session_clears_all_lanes() {
    let gate = ConsentGate::new();
    let approved = AlwaysApprove {
        prompts: Mutex::new(0),
    };
    gate.check_or_prompt("session-A", "openai", &approved)
        .unwrap();
    gate.check_or_prompt("session-A", "anthropic", &approved)
        .unwrap();
    gate.drop_session("session-A").unwrap();
    let prior = *approved.prompts.lock().unwrap();
    gate.check_or_prompt("session-A", "openai", &approved)
        .unwrap();
    assert_eq!(*approved.prompts.lock().unwrap(), prior + 1);
}
