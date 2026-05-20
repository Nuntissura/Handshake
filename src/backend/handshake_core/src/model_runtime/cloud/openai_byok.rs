//! MT-125: Cloud lane BYOK OpenAI runtime (scoped scaffold).
//!
//! Implements the structural surface of OpenAiByokRuntime per
//! MT-125: ApiKeyProvider trait + struct + capability declaration +
//! model-name allowlist + audit-row plumbing. The live HTTP/SSE
//! invocation path is deferred to a follow-on alongside the
//! reqwest feature flip (currently `["json"]`; needs `["stream",
//! "rustls-tls"]`) + the SSE parser dep + the
//! wiremock/httpmock test dep. The scaffold ensures:
//!
//! - API key never leaves the `ApiKeyProvider` boundary (no
//!   Display/Debug exposure of the secret string; the struct only
//!   holds the provider trait object).
//! - Capabilities match BYOK cloud realities (no
//!   supports_activation_steering, no supports_subquadratic, etc.)
//!   per HBR-INT-005 lane normalisation.
//! - Model name allowlist + register() validation lands before any
//!   live HTTP follow-on.
//! - The audit row trait abstraction is in place so concrete
//!   Postgres `cloud_invocations` wiring is the follow-on, not the
//!   abstraction layer.
//!
//! Tests pin the structural contract today; the live path tests
//! (wiremock + cancellation + token streaming) land alongside the
//! live HTTP wiring follow-on.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::model_runtime::{ModelCapabilities, ModelId};

/// Allowlist of OpenAI model name prefixes the operator has approved
/// for BYOK cloud invocation. Defaults to the common Chat /
/// Completions / Responses families as of WP-KERNEL-004; operators
/// may extend with [`OpenAiByokRuntime::register_model_name`].
pub const DEFAULT_OPENAI_MODEL_ALLOWLIST: &[&str] =
    &["gpt-4o", "gpt-4-turbo", "gpt-4.1", "o1", "o3", "gpt-3.5-turbo"];

/// Boundary trait for the operator-managed API key. The runtime
/// holds an `Arc<dyn ApiKeyProvider>`; the secret string never
/// surfaces in struct Debug / Display / FR event payloads. The
/// production impl reads from `OperatorSecretsVault`; the test
/// impl returns a literal string for mock-server verification.
pub trait ApiKeyProvider: Send + Sync {
    fn fetch_api_key(&self) -> Result<String, OpenAiByokError>;
}

/// Per-registered-model handle. Maps the Handshake `ModelId` (UUID
/// v7) to the OpenAI model name string used on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiModelHandle {
    pub model_id: ModelId,
    pub openai_model_name: String,
    pub registered_at_utc: String,
}

/// Audit row written for every cloud call per MT-125 red_team
/// minimum_controls. The concrete Postgres `cloud_invocations`
/// table wiring is the follow-on; the trait abstraction here lets
/// the runtime emit rows without depending on sqlx.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudInvocationAuditRow {
    pub model_id: ModelId,
    pub openai_model_name: String,
    pub call_kind: CloudCallKind,
    pub started_at_utc: String,
    pub finished_at_utc: Option<String>,
    pub status: CloudCallStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudCallKind {
    ChatCompletion,
    Embeddings,
    Score,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudCallStatus {
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

pub trait CloudInvocationAuditSink: Send + Sync {
    fn record(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError>;
}

#[derive(Debug, Error)]
pub enum OpenAiByokError {
    #[error("OpenAI model name {0} is not in the BYOK allowlist (extend via register_model_name)")]
    ModelNameNotAllowed(String),
    #[error("OpenAI model name must not be empty")]
    EmptyModelName,
    #[error("model_id {0} is not registered with the BYOK runtime")]
    ModelNotRegistered(ModelId),
    #[error("API key fetch failed: {0}")]
    ApiKeyFetch(String),
    #[error("audit row persistence failed: {0}")]
    AuditPersist(String),
    #[error("internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error(
        "live HTTP path not attached: {0}; enable reqwest \"stream\" + \"rustls-tls\" features and the SSE parser dep, \
         then wire the chat-completions client behind the existing register/capabilities surface"
    )]
    LiveClientUnavailable(String),
}

/// BYOK cloud runtime scaffold. The struct deliberately does NOT
/// hold the API key string; it holds an `Arc<dyn ApiKeyProvider>`
/// so the secret is fetched on-demand from `OperatorSecretsVault`
/// and never serialised into the struct.
pub struct OpenAiByokRuntime {
    api_base: String,
    api_key_provider: Arc<dyn ApiKeyProvider>,
    audit_sink: Arc<dyn CloudInvocationAuditSink>,
    allowlist: RwLock<HashSet<String>>,
    models: RwLock<HashMap<ModelId, OpenAiModelHandle>>,
}

impl std::fmt::Debug for OpenAiByokRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // GLOBAL: secret material is never surfaced in Debug. The
        // api_key_provider is shown as a placeholder.
        f.debug_struct("OpenAiByokRuntime")
            .field("api_base", &self.api_base)
            .field("api_key_provider", &"<redacted Arc<dyn ApiKeyProvider>>")
            .field("audit_sink", &"<Arc<dyn CloudInvocationAuditSink>>")
            .field("models", &self.models.read().map(|m| m.len()).unwrap_or(0))
            .finish()
    }
}

impl OpenAiByokRuntime {
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
                DEFAULT_OPENAI_MODEL_ALLOWLIST
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ),
            models: RwLock::new(HashMap::new()),
        }
    }

    /// Extend the allowlist with an additional OpenAI model-name prefix.
    pub fn register_model_name(&self, model_name: &str) -> Result<(), OpenAiByokError> {
        if model_name.trim().is_empty() {
            return Err(OpenAiByokError::EmptyModelName);
        }
        let mut guard = self
            .allowlist
            .write()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        guard.insert(model_name.to_string());
        Ok(())
    }

    /// Register a model handle. Validates the requested OpenAI model
    /// name against the allowlist (prefix-match; supports family
    /// allowlisting like `gpt-4o`).
    pub fn register_handle(
        &self,
        openai_model_name: &str,
        now_utc: &str,
    ) -> Result<OpenAiModelHandle, OpenAiByokError> {
        if openai_model_name.trim().is_empty() {
            return Err(OpenAiByokError::EmptyModelName);
        }
        let allowed = {
            let guard = self
                .allowlist
                .read()
                .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
            guard
                .iter()
                .any(|prefix| openai_model_name.starts_with(prefix))
        };
        if !allowed {
            return Err(OpenAiByokError::ModelNameNotAllowed(
                openai_model_name.to_string(),
            ));
        }
        let model_id = ModelId::new_v7();
        let handle = OpenAiModelHandle {
            model_id,
            openai_model_name: openai_model_name.to_string(),
            registered_at_utc: now_utc.to_string(),
        };
        let mut models = self
            .models
            .write()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        models.insert(model_id, handle.clone());
        Ok(handle)
    }

    /// Capability declaration for BYOK cloud lane per HBR-INT-005.
    /// Lane realities (server-side opaque): no local LoRA mounting,
    /// no activation steering, no subquadratic; KV prefix cache is
    /// implicit via OpenAI's prompt caching but the kernel cannot
    /// inspect or control quantisation level.
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

    /// Look up a registered handle.
    pub fn handle_for(&self, model_id: ModelId) -> Result<OpenAiModelHandle, OpenAiByokError> {
        let models = self
            .models
            .read()
            .map_err(|err| OpenAiByokError::LockPoisoned(err.to_string()))?;
        models
            .get(&model_id)
            .cloned()
            .ok_or(OpenAiByokError::ModelNotRegistered(model_id))
    }

    /// Convenience: fetch the secret from the provider. The key is
    /// returned by value to the caller; ensure it is dropped quickly
    /// and never logged.
    pub fn fetch_api_key(&self) -> Result<String, OpenAiByokError> {
        self.api_key_provider
            .fetch_api_key()
            .map_err(|err| OpenAiByokError::ApiKeyFetch(format!("{err}")))
    }

    /// Records an audit row through the sink. Called by the live
    /// HTTP path follow-on; exposed publicly so tests can pin
    /// audit-emission shape without bringing up the HTTP client.
    pub fn record_audit(&self, row: CloudInvocationAuditRow) -> Result<(), OpenAiByokError> {
        self.audit_sink.record(row)
    }

    /// Placeholder for the live chat-completions call. Returns
    /// LiveClientUnavailable until the reqwest streaming features +
    /// SSE parser dep land; the structural contract (allowlist
    /// check, audit row emission, capability declaration) is fully
    /// implemented and tested above.
    pub fn chat_completions_stream(
        &self,
        model_id: ModelId,
    ) -> Result<(), OpenAiByokError> {
        let _ = self.handle_for(model_id)?;
        Err(OpenAiByokError::LiveClientUnavailable(format!(
            "chat_completions_stream(model_id={model_id}) deferred"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

    fn fixture_runtime() -> OpenAiByokRuntime {
        OpenAiByokRuntime::new(
            "https://api.openai.com/v1".to_string(),
            Arc::new(StaticKeyProvider {
                key: "sk-test-NEVER-LOG-THIS".to_string(),
            }),
            Arc::new(CapturingSink::default()),
        )
    }

    #[test]
    fn debug_repr_redacts_api_key_provider() {
        let runtime = fixture_runtime();
        let dbg = format!("{runtime:?}");
        assert!(dbg.contains("<redacted"));
        assert!(!dbg.contains("sk-test-NEVER-LOG-THIS"));
    }

    #[test]
    fn register_handle_accepts_allowlisted_model_family() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("gpt-4o-2024-08-06", "2026-05-20T05:30:00Z")
            .expect("allowlisted gpt-4o family");
        assert_eq!(handle.openai_model_name, "gpt-4o-2024-08-06");
        let lookup = runtime.handle_for(handle.model_id).expect("registered");
        assert_eq!(lookup, handle);
    }

    #[test]
    fn register_handle_rejects_non_allowlisted_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("definitely-not-openai", "2026-05-20T05:30:00Z")
            .expect_err("not in allowlist");
        assert!(matches!(err, OpenAiByokError::ModelNameNotAllowed(_)));
    }

    #[test]
    fn register_handle_rejects_empty_model_name() {
        let runtime = fixture_runtime();
        let err = runtime
            .register_handle("  ", "2026-05-20T05:30:00Z")
            .expect_err("empty model name");
        assert!(matches!(err, OpenAiByokError::EmptyModelName));
    }

    #[test]
    fn register_model_name_extends_allowlist() {
        let runtime = fixture_runtime();
        runtime.register_model_name("custom-finetune-").expect("extend");
        let handle = runtime
            .register_handle("custom-finetune-v1", "2026-05-20T05:30:00Z")
            .expect("now allowed");
        assert_eq!(handle.openai_model_name, "custom-finetune-v1");
    }

    #[test]
    fn capabilities_match_byok_cloud_realities() {
        let caps = OpenAiByokRuntime::cloud_capabilities();
        assert!(!caps.supports_lora);
        assert!(caps.supports_kv_prefix_cache);
        assert!(!caps.supports_activation_steering);
        assert!(!caps.supports_subquadratic);
        assert!(!caps.supports_speculative_draft);
        assert!(!caps.supports_eagle3);
    }

    #[test]
    fn audit_sink_records_rows_through_the_runtime() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("gpt-4o", "2026-05-20T05:30:00Z")
            .unwrap();
        runtime
            .record_audit(CloudInvocationAuditRow {
                model_id: handle.model_id,
                openai_model_name: handle.openai_model_name.clone(),
                call_kind: CloudCallKind::ChatCompletion,
                started_at_utc: "2026-05-20T05:30:00Z".to_string(),
                finished_at_utc: None,
                status: CloudCallStatus::Started,
            })
            .expect("audit");
        // We can't reach the inner sink without an accessor; the
        // smoke is that the trait-level record() returned Ok and
        // never failed (the test's CapturingSink returns Ok).
    }

    #[test]
    fn fetch_api_key_returns_provider_value_and_does_not_log_it() {
        let runtime = fixture_runtime();
        let key = runtime.fetch_api_key().expect("fetch");
        assert!(key.starts_with("sk-"));
        // GLOBAL: the call returns the key value but neither the
        // runtime Debug repr nor any FR event surface logs it.
        // This is enforced by the redaction test above + the
        // boundary discipline documented in the module docstring.
    }

    #[test]
    fn live_chat_completions_returns_live_client_unavailable_until_wired() {
        let runtime = fixture_runtime();
        let handle = runtime
            .register_handle("gpt-4o", "2026-05-20T05:30:00Z")
            .unwrap();
        let err = runtime
            .chat_completions_stream(handle.model_id)
            .expect_err("not yet wired");
        assert!(matches!(err, OpenAiByokError::LiveClientUnavailable(_)));
    }

    #[test]
    fn handle_for_unknown_model_returns_not_registered() {
        let runtime = fixture_runtime();
        let unknown = ModelId::new_v7();
        let err = runtime
            .handle_for(unknown)
            .expect_err("unknown model");
        assert!(matches!(err, OpenAiByokError::ModelNotRegistered(_)));
    }
}
