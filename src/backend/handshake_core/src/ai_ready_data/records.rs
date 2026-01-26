use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionSourceType {
    User,
    Connector,
    System,
}

impl IngestionSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IngestionSourceType::User => "user",
            IngestionSourceType::Connector => "connector",
            IngestionSourceType::System => "system",
        }
    }
}

impl FromStr for IngestionSourceType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user" => Ok(IngestionSourceType::User),
            "connector" => Ok(IngestionSourceType::Connector),
            "system" => Ok(IngestionSourceType::System),
            _ => Err("invalid ingestion_source_type"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMethod {
    UserCreate,
    FileImport,
    ApiIngest,
    ConnectorSync,
    SystemGenerate,
}

impl IngestionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            IngestionMethod::UserCreate => "user_create",
            IngestionMethod::FileImport => "file_import",
            IngestionMethod::ApiIngest => "api_ingest",
            IngestionMethod::ConnectorSync => "connector_sync",
            IngestionMethod::SystemGenerate => "system_generate",
        }
    }
}

impl FromStr for IngestionMethod {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user_create" => Ok(IngestionMethod::UserCreate),
            "file_import" => Ok(IngestionMethod::FileImport),
            "api_ingest" => Ok(IngestionMethod::ApiIngest),
            "connector_sync" => Ok(IngestionMethod::ConnectorSync),
            "system_generate" => Ok(IngestionMethod::SystemGenerate),
            _ => Err("invalid ingestion_method"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
    Warning,
    Pending,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationStatus::Passed => "passed",
            ValidationStatus::Failed => "failed",
            ValidationStatus::Warning => "warning",
            ValidationStatus::Pending => "pending",
        }
    }
}

impl FromStr for ValidationStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "passed" => Ok(ValidationStatus::Passed),
            "failed" => Ok(ValidationStatus::Failed),
            "warning" => Ok(ValidationStatus::Warning),
            "pending" => Ok(ValidationStatus::Pending),
            _ => Err("invalid validation_status"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingModelStatus {
    Active,
    Deprecated,
    Retired,
}

impl EmbeddingModelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmbeddingModelStatus::Active => "active",
            EmbeddingModelStatus::Deprecated => "deprecated",
            EmbeddingModelStatus::Retired => "retired",
        }
    }
}

impl FromStr for EmbeddingModelStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(EmbeddingModelStatus::Active),
            "deprecated" => Ok(EmbeddingModelStatus::Deprecated),
            "retired" => Ok(EmbeddingModelStatus::Retired),
            _ => Err("invalid embedding_model_status"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelRecord {
    pub model_id: String,
    pub model_version: String,
    pub dimensions: u32,
    pub max_input_tokens: u32,
    pub content_types: Vec<String>,
    pub status: EmbeddingModelStatus,
    pub introduced_at: DateTime<Utc>,
    pub compatible_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRegistry {
    pub current_default_model_id: String,
    pub current_default_model_version: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BronzeRecord {
    pub bronze_id: String,
    pub workspace_id: String,
    pub content_hash: String,
    pub content_type: String,
    pub content_encoding: String,
    pub size_bytes: u64,
    pub original_filename: Option<String>,
    pub artifact_path: String,
    pub ingested_at: DateTime<Utc>,
    pub ingestion_source_type: IngestionSourceType,
    pub ingestion_source_id: Option<String>,
    pub ingestion_method: String,
    pub external_source_json: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub retention_policy: String,
}

#[derive(Debug, Clone)]
pub struct NewBronzeRecord {
    pub bronze_id: String,
    pub workspace_id: String,
    pub content_hash: String,
    pub content_type: String,
    pub content_encoding: String,
    pub size_bytes: u64,
    pub original_filename: Option<String>,
    pub artifact_path: String,
    pub ingestion_source_type: IngestionSourceType,
    pub ingestion_source_id: Option<String>,
    pub ingestion_method: String,
    pub external_source_json: Option<String>,
    pub retention_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilverRecord {
    pub silver_id: String,
    pub workspace_id: String,
    pub bronze_ref: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub token_count: u32,
    pub content_hash: String,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
    pub chunk_artifact_path: String,
    pub embedding_artifact_path: String,
    pub embedding_model_id: String,
    pub embedding_model_version: String,
    pub embedding_dimensions: u32,
    pub embedding_compute_latency_ms: u64,
    pub chunking_strategy: String,
    pub chunking_version: String,
    pub processing_pipeline_version: String,
    pub processed_at: DateTime<Utc>,
    pub processing_duration_ms: u64,
    pub metadata_json: String,
    pub validation_status: ValidationStatus,
    pub validation_failed_checks_json: String,
    pub validated_at: DateTime<Utc>,
    pub validator_version: String,
    pub is_current: bool,
    pub superseded_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewSilverRecord {
    pub silver_id: String,
    pub workspace_id: String,
    pub bronze_ref: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub token_count: u32,
    pub content_hash: String,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
    pub chunk_artifact_path: String,
    pub embedding_artifact_path: String,
    pub embedding_model_id: String,
    pub embedding_model_version: String,
    pub embedding_dimensions: u32,
    pub embedding_compute_latency_ms: u64,
    pub chunking_strategy: String,
    pub chunking_version: String,
    pub processing_pipeline_version: String,
    pub processing_duration_ms: u64,
    pub metadata_json: String,
    pub validation_status: ValidationStatus,
    pub validation_failed_checks_json: String,
    pub validator_version: String,
}
