pub mod chunking;
pub mod embedding;
pub mod indexing;
pub mod paths;
pub mod pipeline;
pub mod quality;
pub mod records;
pub mod retrieval;

use thiserror::Error;

/// Validator version for FR-EVT-DATA-008 (data_validation_failed).
pub const AI_READY_DATA_VALIDATOR_VERSION: &str = "ai_ready_data_v1";

#[derive(Debug, Error)]
pub enum AiReadyDataError {
    #[error("HSK-ARD-001: invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("HSK-ARD-002: chunking failed: {0}")]
    Chunking(&'static str),
    #[error("HSK-ARD-003: filesystem error: {0}")]
    Filesystem(String),
    #[error("HSK-ARD-004: storage error: {0}")]
    Storage(String),
    #[error("HSK-ARD-005: recorder error: {0}")]
    Recorder(String),
    #[error("HSK-ARD-006: serialization error: {0}")]
    Serialization(String),
}

impl From<std::io::Error> for AiReadyDataError {
    fn from(value: std::io::Error) -> Self {
        AiReadyDataError::Filesystem(value.to_string())
    }
}

impl From<serde_json::Error> for AiReadyDataError {
    fn from(value: serde_json::Error) -> Self {
        AiReadyDataError::Serialization(value.to_string())
    }
}
