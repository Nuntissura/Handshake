//! MT-125 / MT-126 / MT-127: Cloud lane BYOK model runtime adapters.

pub mod anthropic_byok;
pub mod official_cli_bridge;
pub mod openai_byok;

pub use anthropic_byok::{
    AnthropicByokError, AnthropicByokRuntime, AnthropicModelHandle,
    DEFAULT_ANTHROPIC_MODEL_ALLOWLIST,
};
pub use official_cli_bridge::{
    CliBridgeConfig, CliBridgeHandle, CliInvocationReceipt, CliKind, CliOutputFormat,
    CliSubprocessSpawner, OfficialCliBridgeError, OfficialCliBridgeRuntime,
};
pub use openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime, OpenAiModelHandle,
    DEFAULT_OPENAI_MODEL_ALLOWLIST,
};
