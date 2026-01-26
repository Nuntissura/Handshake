-- Phase 1: AI-Ready Data Architecture baseline (Bronze/Silver + model registry)

CREATE TABLE IF NOT EXISTS ai_embedding_models (
    model_id TEXT NOT NULL,
    model_version TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    max_input_tokens INTEGER NOT NULL,
    content_types_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'deprecated', 'retired')),
    introduced_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    compatible_with_json TEXT NOT NULL,
    PRIMARY KEY (model_id, model_version)
);

CREATE TABLE IF NOT EXISTS ai_embedding_registry (
    id TEXT PRIMARY KEY,
    current_default_model_id TEXT NOT NULL,
    current_default_model_version TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ai_bronze_records (
    bronze_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    content_type TEXT NOT NULL,
    content_encoding TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    original_filename TEXT,
    artifact_path TEXT NOT NULL,
    ingested_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ingestion_source_type TEXT NOT NULL CHECK (ingestion_source_type IN ('user', 'connector', 'system')),
    ingestion_source_id TEXT,
    ingestion_method TEXT NOT NULL,
    external_source_json TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at TIMESTAMP,
    retention_policy TEXT NOT NULL DEFAULT 'default'
);

CREATE TABLE IF NOT EXISTS ai_silver_records (
    silver_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    bronze_ref TEXT NOT NULL REFERENCES ai_bronze_records(bronze_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL,
    token_count INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    byte_start BIGINT NOT NULL,
    byte_end BIGINT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    chunk_artifact_path TEXT NOT NULL,
    embedding_artifact_path TEXT NOT NULL,
    embedding_model_id TEXT NOT NULL,
    embedding_model_version TEXT NOT NULL,
    embedding_dimensions INTEGER NOT NULL,
    embedding_compute_latency_ms BIGINT NOT NULL,
    chunking_strategy TEXT NOT NULL,
    chunking_version TEXT NOT NULL,
    processing_pipeline_version TEXT NOT NULL,
    processed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processing_duration_ms BIGINT NOT NULL,
    metadata_json TEXT NOT NULL,
    validation_status TEXT NOT NULL CHECK (validation_status IN ('passed', 'failed', 'warning', 'pending')),
    validation_failed_checks_json TEXT NOT NULL,
    validated_at TIMESTAMP NOT NULL,
    validator_version TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    superseded_by TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ai_bronze_workspace ON ai_bronze_records(workspace_id);
CREATE INDEX IF NOT EXISTS idx_ai_bronze_content_hash ON ai_bronze_records(content_hash);
CREATE INDEX IF NOT EXISTS idx_ai_silver_workspace ON ai_silver_records(workspace_id);
CREATE INDEX IF NOT EXISTS idx_ai_silver_bronze_ref ON ai_silver_records(bronze_ref);
CREATE INDEX IF NOT EXISTS idx_ai_silver_model ON ai_silver_records(embedding_model_id, embedding_model_version);
