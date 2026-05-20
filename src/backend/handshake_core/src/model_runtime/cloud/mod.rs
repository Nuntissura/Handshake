//! MT-125 / MT-126 / MT-127 / MT-128: Cloud lane BYOK model runtime adapters.

pub mod anthropic_byok;
pub mod consent_gate;
pub mod official_cli_bridge;
pub mod openai_byok;
pub mod secrets_vault;

pub use anthropic_byok::{
    AnthropicByokError, AnthropicByokRuntime, AnthropicModelHandle,
    DEFAULT_ANTHROPIC_MODEL_ALLOWLIST,
};
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
