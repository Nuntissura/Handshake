//! MT-125 cross-crate integration smoke for the BYOK cloud runtime
//! scaffold. Exhaustive coverage lives in the inline tests in
//! `model_runtime::cloud::openai_byok::tests`; this file pins the
//! cross-crate API surface + the red_team minimum_controls.

use std::sync::{Arc, Mutex};

use handshake_core::model_runtime::cloud::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime,
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

fn fixture(sink: Arc<CapturingSink>) -> OpenAiByokRuntime {
    OpenAiByokRuntime::new(
        "https://api.openai.com/v1",
        Arc::new(StaticKey {
            key: "sk-test-NEVER-LOG-THIS-KEY".to_string(),
        }),
        sink,
    )
}

#[test]
fn cloud_openai_capabilities_match_byok_realities_no_steering_no_lora() {
    let caps = OpenAiByokRuntime::cloud_capabilities();
    assert!(!caps.supports_lora);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
    // KV prefix cache is implicit via OpenAI's prompt caching.
    assert!(caps.supports_kv_prefix_cache);
}

#[test]
fn cloud_openai_runtime_debug_redacts_api_key() {
    // MT-125 red_team minimum_controls[0]: API key not logged. Even
    // a developer formatting the runtime with {:?} must not see the
    // secret.
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    let dbg = format!("{runtime:?}");
    assert!(dbg.contains("<redacted"), "{dbg}");
    assert!(!dbg.contains("sk-test-NEVER-LOG-THIS-KEY"), "{dbg}");
}

#[test]
fn cloud_openai_runtime_register_handle_validates_allowlist() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    runtime
        .register_handle("gpt-4o-2024-08-06", "2026-05-20T05:00:00Z")
        .expect("allowed");
    let err = runtime
        .register_handle("definitely-not-openai", "2026-05-20T05:00:00Z")
        .expect_err("not allowed");
    assert!(matches!(err, OpenAiByokError::ModelNameNotAllowed(_)));
}

#[test]
fn cloud_openai_audit_sink_records_call_lifecycle() {
    // MT-125 red_team minimum_controls[1]: cloud_invocations audit
    // row written per call. The integration here pins that the
    // runtime forwards the row through the trait sink; the concrete
    // Postgres impl wiring is a follow-on.
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink.clone());
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T05:00:00Z")
        .unwrap();
    runtime
        .record_audit(CloudInvocationAuditRow {
            model_id: handle.model_id,
            openai_model_name: handle.openai_model_name.clone(),
            call_kind: CloudCallKind::ChatCompletion,
            started_at_utc: "2026-05-20T05:00:00Z".to_string(),
            finished_at_utc: Some("2026-05-20T05:00:01Z".to_string()),
            status: CloudCallStatus::Succeeded,
        })
        .expect("audit ok");
    let captured = sink.rows.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].openai_model_name, "gpt-4o");
}

#[test]
fn cloud_openai_live_chat_returns_live_client_unavailable_until_wired() {
    let sink = Arc::new(CapturingSink::default());
    let runtime = fixture(sink);
    let handle = runtime
        .register_handle("gpt-4o", "2026-05-20T05:00:00Z")
        .unwrap();
    let err = runtime
        .chat_completions_stream(handle.model_id)
        .expect_err("not yet wired");
    assert!(matches!(err, OpenAiByokError::LiveClientUnavailable(_)));
}
