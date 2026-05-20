//! MT-126: Cloud lane BYOK Anthropic Messages API runtime (scoped scaffold).
//!
//! Same shape as MT-125 [`super::openai_byok`]: ApiKeyProvider boundary
//! + model-name allowlist + capability declaration + audit row trait
//! + LiveClientUnavailable scaffold for the chat path. The live
//! `messages` POST + SSE streaming wiring is the same follow-on that
//! gates MT-125 (reqwest "stream" + "rustls-tls" features + an SSE
//! parser dep + wiremock test dep + full ModelRuntime trait impl).
//!
//! Per HBR-INT-005 lane normalisation, this adapter exposes the same
//! `ModelCapabilities` shape as the OpenAI BYOK adapter; only the
//! model-name allowlist and the wire-format differ.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use thiserror::Error;

use super::openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink,
};
use crate::model_runtime::{ModelCapabilities, ModelId};

/// Allowlist of Anthropic model name prefixes the operator has
/// approved for BYOK cloud invocation. Defaults to the Claude
/// families current as of WP-KERNEL-004; operators may extend via
/// [`AnthropicByokRuntime::register_model_name`].
pub const DEFAULT_ANTHROPIC_MODEL_ALLOWLIST: &[&str] = &[
    "claude-3-opus",
    "claude-3-sonnet",
    "claude-3-haiku",
    "claude-3.5-sonnet",
    "claude-3.5-haiku",
    "claude-3.7",
    "claude-opus-4",
    "claude-sonnet-4",
    "claude-haiku-4",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnthropicModelHandle {
    pub model_id: ModelId,
    pub anthropic_model_name: String,
    pub registered_at_utc: String,
}

#[derive(Debug, Error)]
pub enum AnthropicByokError {
    #[error(
        "Anthropic model name {0} is not in the BYOK allowlist (extend via register_model_name)"
    )]
    ModelNameNotAllowed(String),
    #[error("Anthropic model name must not be empty")]
    EmptyModelName,
    #[error("model_id {0} is not registered with the Anthropic BYOK runtime")]
    ModelNotRegistered(ModelId),
    #[error("API key fetch failed: {0}")]
    ApiKeyFetch(String),
    #[error("audit row persistence failed: {0}")]
    AuditPersist(String),
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error(
        "live HTTP path not attached: {0}; enable reqwest \"stream\" + \"rustls-tls\" features and \
         the SSE parser dep, then wire the messages-streaming client behind the existing register/\
         capabilities surface"
    )]
    LiveClientUnavailable(String),
}

impl From<super::openai_byok::OpenAiByokError> for AnthropicByokError {
    fn from(value: super::openai_byok::OpenAiByokError) -> Self {
        // ApiKeyFetch + audit are the only variants we transit
        // through the shared sink/provider traits; map them onto
        // the same shape on our side so the API stays Anthropic-
        // focused at the boundary.
        use super::openai_byok::OpenAiByokError as O;
        match value {
            O::ApiKeyFetch(msg) => AnthropicByokError::ApiKeyFetch(msg),
            O::AuditPersist(msg) => AnthropicByokError::AuditPersist(msg),
            O::LockPoisoned(msg) => AnthropicByokError::LockPoisoned(msg),
            other => AnthropicByokError::ApiKeyFetch(format!("{other}")),
        }
    }
}

pub struct AnthropicByokRuntime {
    api_base: String,
    api_key_provider: Arc<dyn ApiKeyProvider>,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    allowlist: RwLock<HashSet<String>>,
    models: RwLock<HashMap<ModelId, AnthropicModelHandle>>,
}

impl std::fmt::Debug for AnthropicByokRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicByokRuntime")
            .field("api_base", &self.api_base)
            .field("api_key_provider", &"<redacted Arc<dyn ApiKeyProvider>>")
            .field("audit_sink", &"<Arc<dyn CloudInvocationAuditSink>>")
            .field("models", &self.models.read().map(|m| m.len()).unwrap_or(0))
            .finish()
    }
}

impl AnthropicByokRuntime {
    pub fn new(
        api_base: impl Into<String>,
        api_key_provider: Arc<dyn ApiKeyProvider>,
        audit_sink: Arc<dyn CloudInvocationAuditSink>,
    ) -> Self {
        Self {
            api_base: api_base.into(),
            api_key_provider,
            audit_sink,
            allowlist: RwLock::new(
                DEFAULT_ANTHROPIC_MODEL_ALLOWLIST
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ),
            models: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_model_name(&self, model_name: &str) -> Result<(), AnthropicByokError> {
        if model_name.trim().is_empty() {
            return Err(AnthropicByokError::EmptyModelName);
        }
        let mut guard = self
            .allowlist
            .write()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        guard.insert(model_name.to_string());
        Ok(())
    }

    pub fn register_handle(
        &self,
        anthropic_model_name: &str,
        now_utc: &str,
    ) -> Result<AnthropicModelHandle, AnthropicByokError> {
        if anthropic_model_name.trim().is_empty() {
            return Err(AnthropicByokError::EmptyModelName);
        }
        let allowed = {
            let guard = self
                .allowlist
                .read()
                .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
            guard
                .iter()
                .any(|prefix| anthropic_model_name.starts_with(prefix))
        };
        if !allowed {
            return Err(AnthropicByokError::ModelNameNotAllowed(
                anthropic_model_name.to_string(),
            ));
        }
        let model_id = ModelId::new_v7();
        let handle = AnthropicModelHandle {
            model_id,
            anthropic_model_name: anthropic_model_name.to_string(),
            registered_at_utc: now_utc.to_string(),
        };
        let mut models = self
            .models
            .write()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        models.insert(model_id, handle.clone());
        Ok(handle)
    }

    /// Capability declaration for the Anthropic BYOK cloud lane.
    /// HBR-INT-005 lane normalisation: same capability set as
    /// OpenAI BYOK (server-side opaque; no local LoRA / steering /
    /// subquadratic / speculative draft).
    pub fn cloud_capabilities() -> ModelCapabilities {
        ModelCapabilities {
            supports_lora: false,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: crate::model_runtime::KvQuantSupport::None,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        }
    }

    pub fn handle_for(
        &self,
        model_id: ModelId,
    ) -> Result<AnthropicModelHandle, AnthropicByokError> {
        let models = self
            .models
            .read()
            .map_err(|err| AnthropicByokError::LockPoisoned(err.to_string()))?;
        models
            .get(&model_id)
            .cloned()
            .ok_or(AnthropicByokError::ModelNotRegistered(model_id))
    }

    pub fn fetch_api_key(&self) -> Result<String, AnthropicByokError> {
        self.api_key_provider
            .fetch_api_key()
            .map_err(|err| AnthropicByokError::ApiKeyFetch(format!("{err}")))
    }

    pub fn record_audit(&self, row: CloudInvocationAuditRow) -> Result<(), AnthropicByokError> {
        self.audit_sink
            .record(row)
            .map_err(AnthropicByokError::from)
    }

    /// Placeholder for the live messages-streaming call. Returns
    /// LiveClientUnavailable until the reqwest feature flip + SSE
    /// parser dep land; the structural contract (allowlist check,
    /// audit row emission, capability declaration) is fully
    /// implemented and tested.
    pub fn messages_stream(&self, model_id: ModelId) -> Result<(), AnthropicByokError> {
        let _ = self.handle_for(model_id)?;
        Err(AnthropicByokError::LiveClientUnavailable(format!(
            "messages_stream(model_id={model_id}) deferred"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use super::super::openai_byok::OpenAiByokError;

    struct StaticKeyProvider {
        key: String,
    }
    impl ApiKeyProvider for StaticKeyProvider {
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

    fn fixture_runtime() -> AnthropicByokRuntime {
        AnthropicByokRuntime::new(
            "https://api.anthropic.com/v1".to_string(),
            Arc::new(StaticKeyProvider {
                key: "sk-ant-NEVER-LOG-THIS-KEY".to_string(),
            }),
            Arc::new(CapturingSink::default()),
        )
    }

    #[test]
    fn debug_repr_redacts_api_key_provider() {
        let runtime = fixture_runtime();
        let dbg = format!("{runtime:?}");
        assert!(dbg.contains("<redacted"));
        assert!(!dbg.contains("sk-ant-NEVER-LOG-THIS-KEY"));
    }

    #[test]
    fn register_handle_accepts_allowlisted_claude_family() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("claude-opus-4-7-20260101", "2026-05-20T06:00:00Z")
            .expect("allowlisted claude-opus-4 family");
        assert_eq!(handle.anthropic_model_name, "claude-opus-4-7-20260101");
        let lookup = runtime.handle_for(handle.model_id).expect("registered");
        assert_eq!(lookup, handle);
    }

    #[test]
    fn register_handle_rejects_non_allowlisted_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("not-claude", "2026-05-20T06:00:00Z")
            .expect_err("not in allowlist");
        assert!(matches!(err, AnthropicByokError::ModelNameNotAllowed(_)));
    }

    #[test]
    fn register_handle_rejects_empty_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("  ", "2026-05-20T06:00:00Z")
            .expect_err("empty model name");
        assert!(matches!(err, AnthropicByokError::EmptyModelName));
    }

    #[test]
    fn register_model_name_extends_allowlist() {
        let runtime = fixture_runtime();
        runtime
            .register_model_name("claude-custom-")
            .expect("extend allowlist");
        let handle = runtime
            .register_handle("claude-custom-v1", "2026-05-20T06:00:00Z")
            .expect("now allowed");
        assert_eq!(handle.anthropic_model_name, "claude-custom-v1");
    }

    #[test]
    fn capabilities_match_byok_cloud_realities() {
        let caps = AnthropicByokRuntime::cloud_capabilities();
        assert!(!caps.supports_lora);
        assert!(caps.supports_kv_prefix_cache);
        assert!(!caps.supports_activation_steering);
        assert!(!caps.supports_subquadratic);
        assert!(!caps.supports_speculative_draft);
        assert!(!caps.supports_eagle3);
    }

    #[test]
    fn audit_sink_records_call_lifecycle() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("claude-3.5-sonnet", "2026-05-20T06:00:00Z")
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
    }

    #[test]
    fn live_messages_stream_returns_live_client_unavailable_until_wired() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("claude-opus-4", "2026-05-20T06:00:00Z")
            .unwrap();
        let err = runtime
            .messages_stream(handle.model_id)
            .expect_err("not yet wired");
        assert!(matches!(err, AnthropicByokError::LiveClientUnavailable(_)));
    }

    #[test]
    fn handle_for_unknown_model_returns_not_registered() {
        let runtime = fixture_runtime();
        let unknown = ModelId::new_v7();
        let err = runtime
            .handle_for(unknown)
            .expect_err("unknown model");
        assert!(matches!(err, AnthropicByokError::ModelNotRegistered(_)));
    }
}
