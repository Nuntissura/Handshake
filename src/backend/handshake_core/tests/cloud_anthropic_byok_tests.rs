//! MT-126 cross-crate integration smoke for the Anthropic BYOK
//! runtime scaffold. Exhaustive coverage lives in the inline tests
//! in `model_runtime::cloud::anthropic_byok::tests`; this file pins
//! the cross-crate API surface + the red_team minimum_controls.

use std::sync::{Arc, Mutex};

use handshake_core::model_runtime::cloud::{
    AnthropicByokError, AnthropicByokRuntime, ApiKeyProvider, CloudCallKind, CloudCallStatus,
    CloudInvocationAuditRow, CloudInvocationAuditSink, OpenAiByokError,
};

struct StaticKey {
    key: String,
}
impl ApiKeyProvider for StaticKey {
    fn fetch_api_key(&self) -> Result<String, OpenAiByokError> {
        Ok(self.key.clone())
    }
}

#[derive(Default)]
struct CapturingSink {
    rows: Mutex<Vec<CloudInvocationAuditRow>>,
}
impl CloudInvocationAuditSink for CapturingSink {
    fn record(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError> {
        self.rows.lock().unwrap().push(row);
        Ok(())
    }
}

fn fixture(sink: Arc<CapturingSink>) -> AnthropicByokRuntime {
    AnthropicByokRuntime::new(
        "https://api.anthropic.com/v1",
        Arc::new(StaticKey {
            key: "sk-ant-test-NEVER-LOG-THIS-KEY".to_string(),
        }),
        sink,
    )
}

#[test]
fn cloud_anthropic_capabilities_match_byok_realities() {
    let caps = AnthropicByokRuntime::cloud_capabilities();
    assert!(!caps.supports_lora);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
    assert!(caps.supports_kv_prefix_cache);
}

#[test]
fn cloud_anthropic_runtime_debug_redacts_api_key() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    let dbg = format!("{runtime:?}");
    assert!(dbg.contains("<redacted"), "{dbg}");
    assert!(!dbg.contains("sk-ant-test-NEVER-LOG-THIS-KEY"), "{dbg}");
}

#[test]
fn cloud_anthropic_runtime_register_handle_validates_allowlist() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    runtime
        .register_handle("claude-3.5-sonnet-20251022", "2026-05-20T06:00:00Z")
        .expect("claude-3.5-sonnet family allowed");
    runtime
        .register_handle("claude-opus-4-7", "2026-05-20T06:00:00Z")
        .expect("claude-opus-4 family allowed");
    let err = runtime
        .register_handle("not-claude", "2026-05-20T06:00:00Z")
        .expect_err("not allowed");
    assert!(matches!(err, AnthropicByokError::ModelNameNotAllowed(_)));
}

#[test]
fn cloud_anthropic_audit_sink_records_call_lifecycle() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink.clone());
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T06:00:00Z")
        .unwrap();
    runtime
        .record_audit(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.anthropic_model_name.clone(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: "2026-05-20T06:00:00Z".to_string(),
            finished_at_utc: Some("2026-05-20T06:00:01Z".to_string()),
            status: CloudCallStatus::Succeeded,
        })
        .expect("audit ok");
    let rows = sink.rows.lock().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].openai_model_name, "claude-opus-4");
}

#[test]
fn cloud_anthropic_live_messages_returns_unavailable_until_wired() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    let handle = runtime
        .register_handle("claude-opus-4", "2026-05-20T06:00:00Z")
        .unwrap();
    let err = runtime
        .messages_stream(handle.model_id)
        .expect_err("not yet wired");
    assert!(matches!(err, AnthropicByokError::LiveClientUnavailable(_)));
}
