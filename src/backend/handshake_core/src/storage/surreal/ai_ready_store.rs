use std::str::FromStr;

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::ai_ready_data::records::{
    BronzeRecord, EmbeddingModelRecord, EmbeddingModelStatus, EmbeddingRegistry,
    IngestionSourceType, NewBronzeRecord, NewSilverRecord, SilverRecord, ValidationStatus,
};
use crate::storage::{StorageError, StorageResult, WriteContext};

const BRONZE: &str = "ai_bronze_records";
const SILVER: &str = "ai_silver_records";
const EMBEDDING_MODELS: &str = "ai_embedding_models";

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct BronzeRow {
    id: RecordId,
    workspace_id: RecordId,
    content_hash: String,
    content_type: String,
    content_encoding: String,
    size_bytes: i64,
    original_filename: Option<String>,
    artifact_path: String,
    ingested_at: Datetime,
    ingestion_source_type: String,
    ingestion_source_id: Option<String>,
    ingestion_method: String,
    external_source_json: Option<String>,
    is_deleted: bool,
    deleted_at: Option<Datetime>,
    retention_policy: String,
}

#[derive(SurrealValue)]
struct BronzeCreateBindings {
    record: RecordId,
    workspace: RecordId,
    content_hash: String,
    content_type: String,
    content_encoding: String,
    size_bytes: i64,
    original_filename: Option<String>,
    artifact_path: String,
    ingested_at: Datetime,
    ingestion_source_type: String,
    ingestion_source_id: Option<String>,
    ingestion_method: String,
    external_source_json: Option<String>,
    retention_policy: String,
}

#[derive(SurrealValue)]
struct DeleteBindings {
    record: RecordId,
    now: Datetime,
}

#[derive(SurrealValue)]
struct SilverRow {
    id: RecordId,
    workspace_id: RecordId,
    bronze_ref: RecordId,
    chunk_index: i64,
    total_chunks: i64,
    token_count: i64,
    content_hash: String,
    byte_start: i64,
    byte_end: i64,
    line_start: i64,
    line_end: i64,
    chunk_artifact_path: String,
    embedding_artifact_path: String,
    embedding_model_id: String,
    embedding_model_version: String,
    embedding_dimensions: i64,
    embedding_compute_latency_ms: i64,
    chunking_strategy: String,
    chunking_version: String,
    processing_pipeline_version: String,
    processed_at: Datetime,
    processing_duration_ms: i64,
    metadata_json: String,
    validation_status: String,
    validation_failed_checks_json: Vec<String>,
    validated_at: Datetime,
    validator_version: String,
    is_current: bool,
    superseded_by: Option<String>,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct SilverCreateBindings {
    record: RecordId,
    workspace: RecordId,
    bronze: RecordId,
    chunk_index: i64,
    total_chunks: i64,
    token_count: i64,
    content_hash: String,
    byte_start: i64,
    byte_end: i64,
    line_start: i64,
    line_end: i64,
    chunk_artifact_path: String,
    embedding_artifact_path: String,
    embedding_model_id: String,
    embedding_model_version: String,
    embedding_dimensions: i64,
    embedding_compute_latency_ms: i64,
    chunking_strategy: String,
    chunking_version: String,
    processing_pipeline_version: String,
    processed_at: Datetime,
    processing_duration_ms: i64,
    metadata_json: String,
    validation_status: String,
    validation_failed_checks: Vec<String>,
    validated_at: Datetime,
    validator_version: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct BronzeBinding {
    bronze: RecordId,
}

#[derive(SurrealValue)]
struct SupersedeBindings {
    old: RecordId,
    new: RecordId,
    new_id: String,
}

#[derive(SurrealValue)]
struct EmbeddingRow {
    model_id: String,
    model_version: String,
    dimensions: i64,
    max_input_tokens: i64,
    content_types_json: Vec<String>,
    status: String,
    introduced_at: Datetime,
    compatible_with_json: Vec<String>,
}

#[derive(SurrealValue)]
struct EmbeddingBindings {
    record: RecordId,
    model_id: String,
    model_version: String,
    dimensions: i64,
    max_input_tokens: i64,
    content_types: Vec<String>,
    status: String,
    introduced_at: Datetime,
    compatible_with: Vec<String>,
}

#[derive(SurrealValue)]
struct EmbeddingLookupBindings {
    model_id: String,
    model_version: String,
    now: Datetime,
}

#[derive(SurrealValue)]
struct RegistryRow {
    current_default_model_id: String,
    current_default_model_version: String,
    updated_at: Datetime,
}

pub(crate) async fn create_bronze(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    record: NewBronzeRecord,
) -> StorageResult<BronzeRecord> {
    storage
        .inner
        .guard
        .validate_write(ctx, &record.bronze_id)
        .await
        .map_err(StorageError::from)?;
    let size_bytes = i64::try_from(record.size_bytes)
        .map_err(|_| StorageError::Validation("bronze record size exceeds i64"))?;
    let bindings = BronzeCreateBindings {
        record: RecordId::new(BRONZE, record.bronze_id),
        workspace: RecordId::new("workspaces", record.workspace_id),
        content_hash: record.content_hash,
        content_type: record.content_type,
        content_encoding: record.content_encoding,
        size_bytes,
        original_filename: record.original_filename,
        artifact_path: record.artifact_path,
        ingested_at: Datetime::from(chrono::Utc::now()),
        ingestion_source_type: record.ingestion_source_type.as_str().to_owned(),
        ingestion_source_id: record.ingestion_source_id,
        ingestion_method: record.ingestion_method,
        external_source_json: record.external_source_json,
        retention_policy: record.retention_policy,
    };
    let rows: Vec<BronzeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-AI-READY-DUPLICATE'; }; \
                         CREATE $record SET bronze_id = record::id($record), workspace_id = $workspace, content_hash = $content_hash, \
                         content_type = $content_type, content_encoding = $content_encoding, size_bytes = $size_bytes, \
                         original_filename = $original_filename, artifact_path = $artifact_path, \
                         ingested_at = $ingested_at, ingestion_source_type = $ingestion_source_type, \
                         ingestion_source_id = $ingestion_source_id, ingestion_method = $ingestion_method, \
                         external_source_json = $external_source_json, is_deleted = false, deleted_at = NONE, \
                         retention_policy = $retention_policy RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        2,
                    )
                    .await
            })
        })
        .await
        .map_err(map_ready_error)?;
    rows.into_iter()
        .next()
        .map(map_bronze)
        .transpose()?
        .ok_or_else(|| StorageError::Database("bronze create returned no row".to_owned()))
}

pub(crate) async fn get_bronze(
    storage: &SurrealStorage,
    bronze_id: &str,
) -> StorageResult<Option<BronzeRecord>> {
    let row: Option<BronzeRow> = storage
        .with_data_operation({
            let bronze_id = bronze_id.to_owned();
            move |database| Box::pin(async move { database.select_one(BRONZE, &bronze_id).await })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_bronze).transpose()
}

pub(crate) async fn list_bronze(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<BronzeRecord>> {
    let rows: Vec<BronzeRow> = storage
        .with_data_operation({
            let workspace_id = workspace_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT * FROM ai_bronze_records WHERE workspace_id = $workspace \
                             ORDER BY ingested_at ASC, id ASC;",
                            WorkspaceBinding {
                                workspace: RecordId::new("workspaces", workspace_id),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_bronze).collect()
}

pub(crate) async fn mark_bronze_deleted(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    bronze_id: &str,
) -> StorageResult<()> {
    storage
        .inner
        .guard
        .validate_write(ctx, bronze_id)
        .await
        .map_err(StorageError::from)?;
    let rows: Vec<BronzeRow> = storage
        .with_data_operation({
            let bronze_id = bronze_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "UPDATE $record SET is_deleted = true, deleted_at = $now RETURN AFTER;",
                            DeleteBindings {
                                record: RecordId::new(BRONZE, bronze_id),
                                now: Datetime::from(chrono::Utc::now()),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    if rows.is_empty() {
        Err(StorageError::NotFound("ai_bronze_record"))
    } else {
        Ok(())
    }
}

pub(crate) async fn create_silver(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    record: NewSilverRecord,
) -> StorageResult<SilverRecord> {
    storage
        .inner
        .guard
        .validate_write(ctx, &record.silver_id)
        .await
        .map_err(StorageError::from)?;
    let failed_checks: Vec<String> = serde_json::from_str(&record.validation_failed_checks_json)?;
    let now = chrono::Utc::now();
    let bindings = SilverCreateBindings {
        record: RecordId::new(SILVER, record.silver_id),
        workspace: RecordId::new("workspaces", record.workspace_id),
        bronze: RecordId::new(BRONZE, record.bronze_ref),
        chunk_index: i64::from(record.chunk_index),
        total_chunks: i64::from(record.total_chunks),
        token_count: i64::from(record.token_count),
        content_hash: record.content_hash,
        byte_start: i64::try_from(record.byte_start)
            .map_err(|_| StorageError::Validation("silver byte_start exceeds i64"))?,
        byte_end: i64::try_from(record.byte_end)
            .map_err(|_| StorageError::Validation("silver byte_end exceeds i64"))?,
        line_start: i64::from(record.line_start),
        line_end: i64::from(record.line_end),
        chunk_artifact_path: record.chunk_artifact_path,
        embedding_artifact_path: record.embedding_artifact_path,
        embedding_model_id: record.embedding_model_id,
        embedding_model_version: record.embedding_model_version,
        embedding_dimensions: i64::from(record.embedding_dimensions),
        embedding_compute_latency_ms: i64::try_from(record.embedding_compute_latency_ms)
            .map_err(|_| StorageError::Validation("embedding latency exceeds i64"))?,
        chunking_strategy: record.chunking_strategy,
        chunking_version: record.chunking_version,
        processing_pipeline_version: record.processing_pipeline_version,
        processed_at: Datetime::from(now),
        processing_duration_ms: i64::try_from(record.processing_duration_ms)
            .map_err(|_| StorageError::Validation("processing duration exceeds i64"))?,
        metadata_json: record.metadata_json,
        validation_status: record.validation_status.as_str().to_owned(),
        validation_failed_checks: failed_checks,
        validated_at: Datetime::from(now),
        validator_version: record.validator_version,
        created_at: Datetime::from(now),
    };
    let row: Option<SilverRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "CREATE $record SET silver_id = record::id($record), workspace_id = $workspace, bronze_ref = $bronze, \
                         chunk_index = $chunk_index, total_chunks = $total_chunks, token_count = $token_count, \
                         content_hash = $content_hash, byte_start = $byte_start, byte_end = $byte_end, \
                         line_start = $line_start, line_end = $line_end, chunk_artifact_path = $chunk_artifact_path, \
                         embedding_artifact_path = $embedding_artifact_path, embedding_model_id = $embedding_model_id, \
                         embedding_model_version = $embedding_model_version, embedding_dimensions = $embedding_dimensions, \
                         embedding_compute_latency_ms = $embedding_compute_latency_ms, \
                         chunking_strategy = $chunking_strategy, chunking_version = $chunking_version, \
                         processing_pipeline_version = $processing_pipeline_version, processed_at = $processed_at, \
                         processing_duration_ms = $processing_duration_ms, metadata_json = $metadata_json, \
                         validation_status = $validation_status, \
                         validation_failed_checks_json = $validation_failed_checks, validated_at = $validated_at, \
                         validator_version = $validator_version, is_current = true, superseded_by = NONE, \
                         created_at = $created_at RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_ready_error)?;
    row.map(map_silver)
        .transpose()?
        .ok_or_else(|| StorageError::Database("silver create returned no row".to_owned()))
}

pub(crate) async fn get_silver(
    storage: &SurrealStorage,
    silver_id: &str,
) -> StorageResult<Option<SilverRecord>> {
    let row: Option<SilverRow> = storage
        .with_data_operation({
            let silver_id = silver_id.to_owned();
            move |database| Box::pin(async move { database.select_one(SILVER, &silver_id).await })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_silver).transpose()
}

pub(crate) async fn list_silver_by_bronze(
    storage: &SurrealStorage,
    bronze_id: &str,
) -> StorageResult<Vec<SilverRecord>> {
    let rows: Vec<SilverRow> = storage
        .with_data_operation({
            let bronze_id = bronze_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT * FROM ai_silver_records WHERE bronze_ref = $bronze \
                             ORDER BY chunk_index ASC, id ASC;",
                            BronzeBinding {
                                bronze: RecordId::new(BRONZE, bronze_id),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_silver).collect()
}

pub(crate) async fn list_silver(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<SilverRecord>> {
    let rows: Vec<SilverRow> = storage
        .with_data_operation({
            let workspace_id = workspace_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT * FROM ai_silver_records WHERE workspace_id = $workspace \
                             ORDER BY created_at ASC, id ASC;",
                            WorkspaceBinding {
                                workspace: RecordId::new("workspaces", workspace_id),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_silver).collect()
}

pub(crate) async fn supersede_silver(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    old_id: &str,
    new_id: &str,
) -> StorageResult<()> {
    storage
        .inner
        .guard
        .validate_write(ctx, old_id)
        .await
        .map_err(StorageError::from)?;
    let rows: Vec<SilverRow> = storage
        .with_data_operation({
            let old_id = old_id.to_owned();
            let new_id = new_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            "BEGIN TRANSACTION; \
                             IF (SELECT VALUE id FROM $old)[0] = NONE { THROW 'HSK-SILVER-OLD-MISSING'; }; \
                             IF (SELECT VALUE id FROM $new)[0] = NONE { THROW 'HSK-SILVER-NEW-MISSING'; }; \
                             UPDATE $old SET is_current = false, superseded_by = $new_id RETURN AFTER; \
                             COMMIT TRANSACTION;",
                            SupersedeBindings {
                                old: RecordId::new(SILVER, old_id),
                                new: RecordId::new(SILVER, new_id.clone()),
                                new_id,
                            },
                            3,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_ready_error)?;
    if rows.is_empty() {
        Err(StorageError::Database(
            "silver supersede returned no row".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub(crate) async fn upsert_embedding_model(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    model: EmbeddingModelRecord,
) -> StorageResult<()> {
    let key = format!("embedding_model:{}@{}", model.model_id, model.model_version);
    storage
        .inner
        .guard
        .validate_write(ctx, &key)
        .await
        .map_err(StorageError::from)?;
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPSERT $record SET model_id = $model_id, model_version = $model_version, \
                         dimensions = $dimensions, max_input_tokens = $max_input_tokens, \
                         content_types_json = $content_types, status = $status, \
                         introduced_at = introduced_at ?? $introduced_at, compatible_with_json = $compatible_with \
                         RETURN AFTER;",
                        EmbeddingBindings {
                            record: RecordId::new(EMBEDDING_MODELS, key),
                            model_id: model.model_id,
                            model_version: model.model_version,
                            dimensions: i64::from(model.dimensions),
                            max_input_tokens: i64::from(model.max_input_tokens),
                            content_types: model.content_types,
                            status: model.status.as_str().to_owned(),
                            introduced_at: Datetime::from(model.introduced_at),
                            compatible_with: model.compatible_with,
                        },
                    )
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(map_ready_error)
}

pub(crate) async fn list_embedding_models(
    storage: &SurrealStorage,
) -> StorageResult<Vec<EmbeddingModelRecord>> {
    let rows: Vec<EmbeddingRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM ai_embedding_models ORDER BY model_id ASC, model_version ASC;",
                        RecordBinding {
                            record: RecordId::new(EMBEDDING_MODELS, "unused"),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_embedding).collect()
}

pub(crate) async fn set_default_embedding_model(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    model_id: &str,
    model_version: &str,
) -> StorageResult<()> {
    storage
        .inner
        .guard
        .validate_write(ctx, "ai_embedding_registry")
        .await
        .map_err(StorageError::from)?;
    let rows: Vec<RegistryRow> = storage
        .with_data_operation({
            let model_id = model_id.to_owned();
            let model_version = model_version.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            "BEGIN TRANSACTION; \
                             IF (SELECT VALUE id FROM ai_embedding_models WHERE model_id = $model_id \
                                 AND model_version = $model_version LIMIT 1)[0] = NONE { \
                                THROW 'HSK-EMBEDDING-MODEL-MISSING'; \
                             }; \
                             UPSERT ai_embedding_registry:global SET current_default_model_id = $model_id, \
                               current_default_model_version = $model_version, updated_at = $now RETURN AFTER; \
                             COMMIT TRANSACTION;",
                            EmbeddingLookupBindings {
                                model_id,
                                model_version,
                                now: Datetime::from(chrono::Utc::now()),
                            },
                            2,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_ready_error)?;
    if rows.is_empty() {
        Err(StorageError::Database(
            "embedding registry upsert returned no row".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub(crate) async fn get_embedding_registry(
    storage: &SurrealStorage,
) -> StorageResult<Option<EmbeddingRegistry>> {
    let row: Option<RegistryRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.select_one("ai_embedding_registry", "global").await })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(row.map(|row| EmbeddingRegistry {
        current_default_model_id: row.current_default_model_id,
        current_default_model_version: row.current_default_model_version,
        updated_at: row.updated_at.into_inner(),
    }))
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "AI-ready record has a non-string id".to_owned(),
        )),
    }
}

fn non_negative_u64(value: i64, label: &'static str) -> StorageResult<u64> {
    u64::try_from(value).map_err(|_| StorageError::Validation(label))
}

fn non_negative_u32(value: i64, label: &'static str) -> StorageResult<u32> {
    u32::try_from(value).map_err(|_| StorageError::Validation(label))
}

fn map_bronze(row: BronzeRow) -> StorageResult<BronzeRecord> {
    Ok(BronzeRecord {
        bronze_id: key(row.id)?,
        workspace_id: key(row.workspace_id)?,
        content_hash: row.content_hash,
        content_type: row.content_type,
        content_encoding: row.content_encoding,
        size_bytes: non_negative_u64(row.size_bytes, "invalid bronze size")?,
        original_filename: row.original_filename,
        artifact_path: row.artifact_path,
        ingested_at: row.ingested_at.into_inner(),
        ingestion_source_type: IngestionSourceType::from_str(&row.ingestion_source_type)
            .map_err(|_| StorageError::Validation("invalid ingestion source type"))?,
        ingestion_source_id: row.ingestion_source_id,
        ingestion_method: row.ingestion_method,
        external_source_json: row.external_source_json,
        is_deleted: row.is_deleted,
        deleted_at: row.deleted_at.map(|value| value.into_inner()),
        retention_policy: row.retention_policy,
    })
}

fn map_silver(row: SilverRow) -> StorageResult<SilverRecord> {
    Ok(SilverRecord {
        silver_id: key(row.id)?,
        workspace_id: key(row.workspace_id)?,
        bronze_ref: key(row.bronze_ref)?,
        chunk_index: non_negative_u32(row.chunk_index, "invalid silver chunk_index")?,
        total_chunks: non_negative_u32(row.total_chunks, "invalid silver total_chunks")?,
        token_count: non_negative_u32(row.token_count, "invalid silver token_count")?,
        content_hash: row.content_hash,
        byte_start: non_negative_u64(row.byte_start, "invalid silver byte_start")?,
        byte_end: non_negative_u64(row.byte_end, "invalid silver byte_end")?,
        line_start: non_negative_u32(row.line_start, "invalid silver line_start")?,
        line_end: non_negative_u32(row.line_end, "invalid silver line_end")?,
        chunk_artifact_path: row.chunk_artifact_path,
        embedding_artifact_path: row.embedding_artifact_path,
        embedding_model_id: row.embedding_model_id,
        embedding_model_version: row.embedding_model_version,
        embedding_dimensions: non_negative_u32(
            row.embedding_dimensions,
            "invalid embedding dimensions",
        )?,
        embedding_compute_latency_ms: non_negative_u64(
            row.embedding_compute_latency_ms,
            "invalid embedding latency",
        )?,
        chunking_strategy: row.chunking_strategy,
        chunking_version: row.chunking_version,
        processing_pipeline_version: row.processing_pipeline_version,
        processed_at: row.processed_at.into_inner(),
        processing_duration_ms: non_negative_u64(
            row.processing_duration_ms,
            "invalid processing duration",
        )?,
        metadata_json: row.metadata_json,
        validation_status: ValidationStatus::from_str(&row.validation_status)
            .map_err(|_| StorageError::Validation("invalid validation status"))?,
        validation_failed_checks_json: serde_json::to_string(&row.validation_failed_checks_json)?,
        validated_at: row.validated_at.into_inner(),
        validator_version: row.validator_version,
        is_current: row.is_current,
        superseded_by: row.superseded_by,
        created_at: row.created_at.into_inner(),
    })
}

fn map_embedding(row: EmbeddingRow) -> StorageResult<EmbeddingModelRecord> {
    Ok(EmbeddingModelRecord {
        model_id: row.model_id,
        model_version: row.model_version,
        dimensions: non_negative_u32(row.dimensions, "invalid embedding dimensions")?,
        max_input_tokens: non_negative_u32(row.max_input_tokens, "invalid max input tokens")?,
        content_types: row.content_types_json,
        status: EmbeddingModelStatus::from_str(&row.status)
            .map_err(|_| StorageError::Validation("invalid embedding model status"))?,
        introduced_at: row.introduced_at.into_inner(),
        compatible_with: row.compatible_with_json,
    })
}

fn map_ready_error(error: super::SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-SILVER-OLD-MISSING") {
        StorageError::NotFound("ai_silver_record")
    } else if message.contains("HSK-SILVER-NEW-MISSING") {
        StorageError::NotFound("new_ai_silver_record")
    } else if message.contains("HSK-EMBEDDING-MODEL-MISSING") {
        StorageError::NotFound("ai_embedding_model")
    } else if message.contains("HSK-AI-READY-DUPLICATE") || message.contains("already contains") {
        StorageError::Conflict("AI-ready record already exists")
    } else if message.contains("record::exists") {
        StorageError::NotFound("ai_ready_parent")
    } else {
        StorageError::from(error)
    }
}
