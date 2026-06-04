use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error("model load failed: {0}")]
    LoadError(String),
    #[error("model unload failed: {0}")]
    UnloadError(String),
    #[error("model generation failed: {0}")]
    GenerateError(String),
    #[error("model scoring failed: {0}")]
    ScoreError(String),
    #[error("model embedding failed: {0}")]
    EmbedError(String),
    #[error("capability {capability} is not supported by adapter {adapter}")]
    CapabilityNotSupported { capability: String, adapter: String },
    #[error("KV cache operation failed: {0}")]
    KvCacheError(String),
    #[error("LoRA stack operation failed: {0}")]
    LoraStackError(String),
    #[error("steering hook operation failed: {0}")]
    SteeringHookError(String),
    #[error("model operation cancelled")]
    Cancelled,
    #[error("model adapter mismatch: expected {expected}, got {got}")]
    AdapterMismatch { expected: String, got: String },
}
