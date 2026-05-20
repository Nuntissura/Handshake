//! MT-125: Cloud lane BYOK model runtime adapters.

pub mod openai_byok;

pub use openai_byok::{
    ApiKeyProvider, CloudCallKind, CloudCallStatus, CloudInvocationAuditRow,
    CloudInvocationAuditSink, OpenAiByokError, OpenAiByokRuntime, OpenAiModelHandle,
    DEFAULT_OPENAI_MODEL_ALLOWLIST,
};
