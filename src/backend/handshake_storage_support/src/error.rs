#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("conflict: {0}")]
    Conflict(&'static str),
    /// Conflict code with owned diagnostic context; public codes stay stable.
    #[error("conflict: {code}; {detail}")]
    ConflictDetails { code: &'static str, detail: String },
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("mutation guard blocked: {0}")]
    Guard(&'static str),
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Opaque database error - hides provider-specific types [§2.3.12.3 Trait Purity]
    #[error("database error: {0}")]
    Database(String),
    /// Opaque migration error - hides provider-specific types [§2.3.12.3 Trait Purity]
    #[error("migration error: {0}")]
    Migration(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
pub type StorageResult<T> = Result<T, StorageError>;
