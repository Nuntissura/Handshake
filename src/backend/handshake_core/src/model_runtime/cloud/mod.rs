//! MT-125 / MT-126 / MT-127 / MT-128: Cloud lane BYOK model runtime adapters.

use std::sync::Arc;

use crate::flight_recorder::FlightRecorder;

pub mod agent_activity;
pub mod anthropic_byok;
pub mod cli_bridge_runtime;
pub mod consent_gate;
pub mod official_cli_bridge;
pub mod openai_byok;
pub mod secrets_vault;

/// MT-125 remediation: per-session, per-lane consent context threaded
/// into a cloud lane adapter so it can enforce the operator
/// [`ConsentGate`] before issuing a live HTTP call. The gate and
/// provider are shared (Arc) so all adapters can consult the same
/// in-memory consent map; `session_id` scopes the per-session-per-lane
/// decision (see [`consent_gate`]).
#[derive(Clone)]
pub struct CloudConsentContext {
    pub gate: Arc<ConsentGate>,
    pub provider: Arc<dyn ConsentProvider>,
    pub session_id: String,
}

/// MT-125 remediation: shared observability bundle threaded into a
/// cloud lane adapter so it can (1) emit
/// `FR-EVT-LLM-INFER-{START,TOKEN,END}` events through the
/// [`FlightRecorder`] for HBR-INT-005 lane normalisation and (2)
/// optionally enforce the operator [`CloudConsentContext`] before the
/// live HTTP call. Anthropic / official-CLI adapters reuse this same
/// type so the cloud lane shares one observability surface.
#[derive(Clone)]
pub struct CloudLaneObservability {
    pub flight_recorder: Arc<dyn FlightRecorder>,
    pub consent: Option<CloudConsentContext>,
}

pub use anthropic_byok::{
    AnthropicByokError, AnthropicByokRuntime, AnthropicModelHandle, ANTHROPIC_API_KEY_HEADER,
    ANTHROPIC_API_VERSION, ANTHROPIC_MESSAGES_PATH, ANTHROPIC_VERSION_HEADER,
    DEFAULT_ANTHROPIC_MODEL_ALLOWLIST,
};
pub use agent_activity::{parse_line as parse_agent_activity_line, AgentActivity, AgentActivityKind};
pub use cli_bridge_runtime::CliBridgeModelRuntime;
pub use consent_gate::{ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider};
pub use official_cli_bridge::{
    CliBridgeConfig, CliBridgeHandle, CliInvocationReceipt, CliKind, CliOutputFormat,
    CliSubprocessSpawner, LiveCliSpawner, OfficialCliBridgeError, OfficialCliBridgeRuntime,
};
pub use openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime, OpenAiModelHandle,
    DEFAULT_OPENAI_MODEL_ALLOWLIST, OPENAI_CHAT_COMPLETIONS_PATH, OPENAI_EMBEDDINGS_PATH,
};
pub use secrets_vault::{
    InMemorySecretsVault, SecretsVault, SecretsVaultError, VaultApiKeyProvider,
};
