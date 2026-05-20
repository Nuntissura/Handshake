//! MT-125 / MT-126: Cloud lane BYOK model runtime adapters.

pub mod anthropic_byok;
pub mod openai_byok;

pub use anthropic_byok::{
    AnthropicByokError, AnthropicByokRuntime, AnthropicModelHandle,
    DEFAULT_ANTHROPIC_MODEL_ALLOWLIST,
};
pub use openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime, OpenAiModelHandle,
    DEFAULT_OPENAI_MODEL_ALLOWLIST,
};
