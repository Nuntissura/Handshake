use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

use crate::ai_ready_data::chunking::{
    chunk_code_treesitter, chunk_document_header_recursive, compute_silver_id, Chunk, CodeLanguage,
};
use crate::ai_ready_data::embedding::{
    embed_text_deterministic, EmbeddingArtifact, DEFAULT_EMBEDDING_DIMENSIONS,
    DEFAULT_EMBEDDING_MAX_INPUT_TOKENS,
};
use crate::ai_ready_data::indexing::{
    build_keyword_index, build_vector_index, tokenize_keyword, GraphArtifact, GraphEdge,
    KeywordIndexConfig, VectorIndexConfig, VectorIndexEntry,
};
use crate::ai_ready_data::paths::ShadowWorkspacePaths;
use crate::ai_ready_data::quality::QualitySLOs;
use crate::ai_ready_data::records::{
    EmbeddingModelRecord, EmbeddingModelStatus, IngestionSourceType, NewBronzeRecord,
    NewSilverRecord, ValidationStatus,
};
use crate::ai_ready_data::retrieval::{
    build_hybrid_results, hybrid_fuse_rrf, keyword_search, vector_search, HybridQuery,
    HybridRetrievalParams, HybridWeights,
};
use crate::ai_ready_data::{AiReadyDataError, AI_READY_DATA_VALIDATOR_VERSION};
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::storage::{Database, WriteContext};

pub const AI_READY_PROCESSING_PIPELINE_VERSION: &str = "ai_ready_pipeline_v1";
pub const AI_READY_CHUNKING_VERSION: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenQuerySpec {
    pub query: String,
    pub expected_ids: Vec<String>,
    pub expected_mrr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIngestSpec {
    pub workspace_id: String,
    pub paths: Vec<String>,
    pub embedding_model_id: String,
    pub embedding_model_version: String,
    pub retrieval_query: Option<HybridQuery>,
    pub golden_query: Option<GoldenQuerySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIngestResult {
    pub workspace_id: String,
    pub bronze_created: u64,
    pub silver_created: u64,
    pub silver_superseded: u64,
    pub embedding_computed: u64,
    pub validation_failed: u64,
    pub relationships_extracted: u64,
    pub indexes_written: u64,
    pub retrieval_ran: bool,
    pub golden_query_failed_emitted: bool,
}

#[derive(Debug, Clone, Default)]
struct IngestCounters {
    bronze_created: u64,
    silver_created: u64,
    silver_superseded: u64,
    embedding_computed: u64,
    validation_failed: u64,
    relationships_extracted: u64,
    indexes_written: u64,
}

pub struct AiReadyDataPipeline<'a> {
    pub paths: ShadowWorkspacePaths,
    storage: &'a dyn Database,
    write_context: &'a WriteContext,
    flight_recorder: &'a dyn FlightRecorder,
    trace_id: Uuid,
    job_id: Option<String>,
    workflow_id: Option<String>,
}

impl<'a> AiReadyDataPipeline<'a> {
    pub fn new(
        paths: ShadowWorkspacePaths,
        storage: &'a dyn Database,
        write_context: &'a WriteContext,
        flight_recorder: &'a dyn FlightRecorder,
        trace_id: Uuid,
        job_id: Option<String>,
        workflow_id: Option<String>,
    ) -> Self {
        Self {
            paths,
            storage,
            write_context,
            flight_recorder,
            trace_id,
            job_id,
            workflow_id,
        }
    }

    pub async fn run_doc_ingest(
        &self,
        spec: DocIngestSpec,
    ) -> Result<DocIngestResult, AiReadyDataError> {
        if spec.workspace_id.trim().is_empty() {
            return Err(AiReadyDataError::InvalidInput("workspace_id"));
        }

        fs::create_dir_all(self.paths.bronze_dir())?;
        fs::create_dir_all(self.paths.silver_dir())?;
        fs::create_dir_all(self.paths.gold_indexes_dir())?;
        fs::create_dir_all(self.paths.gold_graph_dir())?;

        let mut counters = IngestCounters::default();
        self.ensure_embedding_model(&spec.embedding_model_id, &spec.embedding_model_version)
            .await?;

        let model_changed = self
            .maybe_change_default_embedding_model(
                &spec.embedding_model_id,
                &spec.embedding_model_version,
            )
            .await?;

        if model_changed {
            let superseded = self
                .reembed_workspace(&spec.embedding_model_id, &spec.embedding_model_version)
                .await?;
            counters.silver_superseded = counters.silver_superseded.saturating_add(superseded);
        }

        let mut graph_edges: Vec<GraphEdge> = Vec::new();
        for rel_path in &spec.paths {
            self.ingest_path(
                rel_path,
                &spec.embedding_model_id,
                &spec.embedding_model_version,
                &mut counters,
                &mut graph_edges,
            )
            .await?;
        }

        counters.indexes_written = counters.indexes_written.saturating_add(
            self.write_indexes(
                &spec.embedding_model_id,
                &spec.embedding_model_version,
                &graph_edges,
            )
            .await?,
        );

        self.emit_quality_metrics().await?;

        let mut retrieval_ran = false;
        let mut golden_query_failed_emitted = false;

        if let Some(query) = spec.retrieval_query {
            retrieval_ran = true;
            let response = self
                .hybrid_search(
                    &query,
                    &spec.embedding_model_id,
                    &spec.embedding_model_version,
                )
                .await?;
            if let Some(golden) = spec.golden_query {
                golden_query_failed_emitted = self
                    .evaluate_golden_query(&golden, &response.retrieved_ids)
                    .await?;
            }
        } else if let Some(golden) = spec.golden_query {
            let default_query = HybridQuery {
                query: golden.query.clone(),
                query_intent: "factual_lookup".to_string(),
                weights: HybridWeights {
                    vector: 0.5,
                    keyword: 0.5,
                    graph: 0.0,
                },
                retrieval: HybridRetrievalParams {
                    k: 10,
                    vector_candidates: 20,
                    keyword_candidates: 20,
                    graph_hops: 0,
                },
                rerank: false,
            };
            retrieval_ran = true;
            let response = self
                .hybrid_search(
                    &default_query,
                    &spec.embedding_model_id,
                    &spec.embedding_model_version,
                )
                .await?;
            golden_query_failed_emitted = self
                .evaluate_golden_query(&golden, &response.retrieved_ids)
                .await?;
        }

        Ok(DocIngestResult {
            workspace_id: spec.workspace_id,
            bronze_created: counters.bronze_created,
            silver_created: counters.silver_created,
            silver_superseded: counters.silver_superseded,
            embedding_computed: counters.embedding_computed,
            validation_failed: counters.validation_failed,
            relationships_extracted: counters.relationships_extracted,
            indexes_written: counters.indexes_written,
            retrieval_ran,
            golden_query_failed_emitted,
        })
    }

    async fn record_data_event(
        &self,
        event_type: FlightRecorderEventType,
        payload: Value,
    ) -> Result<(), AiReadyDataError> {
        let mut event = FlightRecorderEvent::new(
            event_type,
            FlightRecorderActor::System,
            self.trace_id,
            payload,
        )
        .with_wsids(vec![self.paths.workspace_id().to_string()]);

        if let Some(job_id) = self.job_id.as_ref() {
            event = event.with_job_id(job_id.clone());
        }
        if let Some(workflow_id) = self.workflow_id.as_ref() {
            event = event.with_workflow_id(workflow_id.clone());
        }

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|err| AiReadyDataError::Recorder(err.to_string()))
    }

    async fn ensure_embedding_model(
        &self,
        model_id: &str,
        model_version: &str,
    ) -> Result<(), AiReadyDataError> {
        let model = EmbeddingModelRecord {
            model_id: model_id.to_string(),
            model_version: model_version.to_string(),
            dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
            max_input_tokens: DEFAULT_EMBEDDING_MAX_INPUT_TOKENS,
            content_types: vec![
                "code/rust".to_string(),
                "code/typescript".to_string(),
                "code/javascript".to_string(),
                "document/markdown".to_string(),
            ],
            status: EmbeddingModelStatus::Active,
            introduced_at: Utc::now(),
            compatible_with: vec![format!("{model_id}:{model_version}")],
        };

        self.storage
            .upsert_ai_embedding_model(self.write_context, model)
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
        Ok(())
    }

    async fn maybe_change_default_embedding_model(
        &self,
        model_id: &str,
        model_version: &str,
    ) -> Result<bool, AiReadyDataError> {
        let current = self
            .storage
            .get_ai_embedding_registry()
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        let Some(current) = current else {
            self.storage
                .set_ai_embedding_default_model(self.write_context, model_id, model_version)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
            return Ok(false);
        };

        if current.current_default_model_id == model_id
            && current.current_default_model_version == model_version
        {
            return Ok(false);
        }

        let records = self
            .storage
            .list_ai_silver_records(self.paths.workspace_id())
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
        let affected = records.iter().filter(|record| record.is_current).count() as u64;

        self.storage
            .set_ai_embedding_default_model(self.write_context, model_id, model_version)
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        self.record_data_event(
            FlightRecorderEventType::DataEmbeddingModelChanged,
            json!({
                "type": "data_embedding_model_changed",
                "from_model_id": current.current_default_model_id,
                "from_model_version": current.current_default_model_version,
                "to_model_id": model_id,
                "to_model_version": model_version,
                "affected_silver_records": affected,
            }),
        )
        .await?;

        self.record_data_event(
            FlightRecorderEventType::DataReembeddingTriggered,
            json!({
                "type": "data_reembedding_triggered",
                "model_id": model_id,
                "model_version": model_version,
                "affected_silver_records": affected,
            }),
        )
        .await?;

        Ok(true)
    }

    async fn reembed_workspace(
        &self,
        model_id: &str,
        model_version: &str,
    ) -> Result<u64, AiReadyDataError> {
        let records = self
            .storage
            .list_ai_silver_records(self.paths.workspace_id())
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        let mut superseded: u64 = 0;
        for record in records.into_iter().filter(|record| record.is_current) {
            let chunk_abs = self
                .paths
                .handshake_root()
                .join(PathBuf::from(&record.chunk_artifact_path));
            let chunk_text = fs::read_to_string(&chunk_abs).unwrap_or_default();

            let new_silver_id = compute_silver_id(
                &record.bronze_ref,
                &record.chunking_strategy,
                record.chunk_index,
                record.byte_start as usize,
                record.byte_end as usize,
                &record.content_hash,
                AI_READY_PROCESSING_PIPELINE_VERSION,
                model_id,
                model_version,
            );

            let (vector, was_truncated) = embed_text_deterministic(
                &chunk_text,
                model_id,
                model_version,
                DEFAULT_EMBEDDING_DIMENSIONS,
                DEFAULT_EMBEDDING_MAX_INPUT_TOKENS,
            );

            let embedding_abs =
                self.paths
                    .silver_embedding_artifact_path(&new_silver_id, model_id, model_version);
            let embedding_artifact = EmbeddingArtifact {
                schema_version: "1.0".to_string(),
                model_id: model_id.to_string(),
                model_version: model_version.to_string(),
                dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
                vector,
            };
            write_json_atomic(&embedding_abs, &embedding_artifact)?;

            let chunk_abs_new = self.paths.silver_chunk_artifact_path(&new_silver_id);
            write_bytes_atomic(&chunk_abs_new, chunk_text.as_bytes())?;

            let new_record = NewSilverRecord {
                silver_id: new_silver_id.clone(),
                workspace_id: record.workspace_id.clone(),
                bronze_ref: record.bronze_ref.clone(),
                chunk_index: record.chunk_index,
                total_chunks: record.total_chunks,
                token_count: record.token_count,
                content_hash: record.content_hash.clone(),
                byte_start: record.byte_start,
                byte_end: record.byte_end,
                line_start: record.line_start,
                line_end: record.line_end,
                chunk_artifact_path: self.paths.to_root_relative(&chunk_abs_new),
                embedding_artifact_path: self.paths.to_root_relative(&embedding_abs),
                embedding_model_id: model_id.to_string(),
                embedding_model_version: model_version.to_string(),
                embedding_dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
                embedding_compute_latency_ms: 0,
                chunking_strategy: record.chunking_strategy.clone(),
                chunking_version: record.chunking_version.clone(),
                processing_pipeline_version: AI_READY_PROCESSING_PIPELINE_VERSION.to_string(),
                processing_duration_ms: 0,
                metadata_json: record.metadata_json.clone(),
                validation_status: record.validation_status,
                validation_failed_checks_json: record.validation_failed_checks_json.clone(),
                validator_version: record.validator_version.clone(),
            };

            self.storage
                .create_ai_silver_record(self.write_context, new_record)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

            self.storage
                .supersede_ai_silver_record(self.write_context, &record.silver_id, &new_silver_id)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

            superseded = superseded.saturating_add(1);

            self.record_data_event(
                FlightRecorderEventType::DataSilverUpdated,
                json!({
                    "type": "data_silver_updated",
                    "superseded_silver_id": record.silver_id,
                    "new_silver_id": new_silver_id,
                    "bronze_ref": record.bronze_ref,
                    "chunking_strategy": record.chunking_strategy,
                    "processing_duration_ms": 0,
                }),
            )
            .await?;

            self.record_data_event(
                FlightRecorderEventType::DataEmbeddingComputed,
                json!({
                    "type": "data_embedding_computed",
                    "silver_id": new_silver_id,
                    "model_id": model_id,
                    "model_version": model_version,
                    "dimensions": DEFAULT_EMBEDDING_DIMENSIONS,
                    "compute_latency_ms": 0,
                    "was_truncated": was_truncated,
                }),
            )
            .await?;
        }

        Ok(superseded)
    }

    async fn ingest_path(
        &self,
        rel_path: &str,
        model_id: &str,
        model_version: &str,
        counters: &mut IngestCounters,
        graph_edges: &mut Vec<GraphEdge>,
    ) -> Result<(), AiReadyDataError> {
        let relative = validate_relative_path(rel_path)?;
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        let abs_source = self.paths.handshake_root().join(&relative);
        let bytes =
            fs::read(&abs_source).map_err(|err| AiReadyDataError::Filesystem(err.to_string()))?;
        let content_hash = sha256_hex(&bytes);

        let bronze_id = compute_bronze_id(self.paths.workspace_id(), &relative_str, &content_hash);
        let bronze_abs = self.paths.bronze_artifact_path(&bronze_id);

        if !bronze_abs.exists() {
            write_bytes_atomic(&bronze_abs, &bytes)?;
        }

        let existing = self
            .storage
            .get_ai_bronze_record(&bronze_id)
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        if existing.is_none() {
            let content_type = classify_content_type(&relative).to_string();
            let record = NewBronzeRecord {
                bronze_id: bronze_id.clone(),
                workspace_id: self.paths.workspace_id().to_string(),
                content_hash: content_hash.clone(),
                content_type: content_type.clone(),
                content_encoding: if std::str::from_utf8(&bytes).is_ok() {
                    "utf-8".to_string()
                } else {
                    "binary".to_string()
                },
                size_bytes: bytes.len() as u64,
                original_filename: Some(rel_path.to_string()),
                artifact_path: self.paths.to_root_relative(&bronze_abs),
                ingestion_source_type: IngestionSourceType::System,
                ingestion_source_id: None,
                ingestion_method: "file_import".to_string(),
                external_source_json: None,
                retention_policy: "default".to_string(),
            };

            self.storage
                .create_ai_bronze_record(self.write_context, record)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
            counters.bronze_created = counters.bronze_created.saturating_add(1);

            self.record_data_event(
                FlightRecorderEventType::DataBronzeCreated,
                json!({
                    "type": "data_bronze_created",
                    "bronze_id": bronze_id,
                    "content_type": content_type,
                    "content_hash": content_hash,
                    "size_bytes": bytes.len() as u64,
                    "ingestion_source": "system",
                    "ingestion_method": "file_import",
                }),
            )
            .await?;
        }

        let Ok(source_text) = std::str::from_utf8(&bytes) else {
            return Ok(());
        };

        match classify_code_language(&relative) {
            CodeSupport::Supported(language) => {
                let chunks = match chunk_code_treesitter(
                    &bronze_id,
                    source_text,
                    language,
                    AI_READY_PROCESSING_PIPELINE_VERSION,
                    model_id,
                    model_version,
                ) {
                    Ok(chunks) => chunks,
                    Err(err) => {
                        counters.validation_failed = counters.validation_failed.saturating_add(1);
                        let failed_silver_id = compute_silver_id(
                            &bronze_id,
                            "code_ast_treesitter_v1",
                            0,
                            0,
                            0,
                            &sha256_hex(source_text.as_bytes()),
                            AI_READY_PROCESSING_PIPELINE_VERSION,
                            model_id,
                            model_version,
                        );
                        self.record_data_event(
                            FlightRecorderEventType::DataValidationFailed,
                            json!({
                                "type": "data_validation_failed",
                                "silver_id": failed_silver_id,
                                "failed_checks": [format!("chunking:{err}")],
                                "validator_version": AI_READY_DATA_VALIDATOR_VERSION,
                            }),
                        )
                        .await?;
                        return Ok(());
                    }
                };

                self.persist_chunks_and_embeddings(
                    &bronze_id,
                    &relative_str,
                    &chunks,
                    model_id,
                    model_version,
                    counters,
                )
                .await?;
                self.extract_import_relationships(source_text, &bronze_id, graph_edges, counters)
                    .await?;
            }
            CodeSupport::UnsupportedCode => {
                counters.validation_failed = counters.validation_failed.saturating_add(1);
                let failed_silver_id = compute_silver_id(
                    &bronze_id,
                    "code_ast_treesitter_v1",
                    0,
                    0,
                    0,
                    &sha256_hex(source_text.as_bytes()),
                    AI_READY_PROCESSING_PIPELINE_VERSION,
                    model_id,
                    model_version,
                );
                self.record_data_event(
                    FlightRecorderEventType::DataValidationFailed,
                    json!({
                        "type": "data_validation_failed",
                        "silver_id": failed_silver_id,
                        "failed_checks": ["unsupported_code_language"],
                        "validator_version": AI_READY_DATA_VALIDATOR_VERSION,
                    }),
                )
                .await?;
            }
            CodeSupport::NotCode => {
                let chunks = chunk_document_header_recursive(
                    &bronze_id,
                    source_text,
                    AI_READY_PROCESSING_PIPELINE_VERSION,
                    model_id,
                    model_version,
                )?;
                self.persist_chunks_and_embeddings(
                    &bronze_id,
                    &relative_str,
                    &chunks,
                    model_id,
                    model_version,
                    counters,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn persist_chunks_and_embeddings(
        &self,
        bronze_id: &str,
        source_file_path: &str,
        chunks: &[Chunk],
        model_id: &str,
        model_version: &str,
        counters: &mut IngestCounters,
    ) -> Result<(), AiReadyDataError> {
        for chunk in chunks {
            let chunk_abs = self.paths.silver_chunk_artifact_path(&chunk.silver_id);
            write_bytes_atomic(&chunk_abs, chunk.text.as_bytes())?;

            let (vector, was_truncated) = embed_text_deterministic(
                &chunk.text,
                model_id,
                model_version,
                DEFAULT_EMBEDDING_DIMENSIONS,
                DEFAULT_EMBEDDING_MAX_INPUT_TOKENS,
            );
            let embedding_abs = self.paths.silver_embedding_artifact_path(
                &chunk.silver_id,
                model_id,
                model_version,
            );
            let embedding_artifact = EmbeddingArtifact {
                schema_version: "1.0".to_string(),
                model_id: model_id.to_string(),
                model_version: model_version.to_string(),
                dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
                vector,
            };
            write_json_atomic(&embedding_abs, &embedding_artifact)?;

            let metadata_json = json!({
                "file_path": source_file_path,
                "byte_start": chunk.byte_start,
                "byte_end": chunk.byte_end,
                "line_start": chunk.line_start,
                "line_end": chunk.line_end,
            })
            .to_string();

            let record = NewSilverRecord {
                silver_id: chunk.silver_id.clone(),
                workspace_id: self.paths.workspace_id().to_string(),
                bronze_ref: bronze_id.to_string(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                token_count: chunk.token_count,
                content_hash: chunk.content_hash.clone(),
                byte_start: chunk.byte_start as u64,
                byte_end: chunk.byte_end as u64,
                line_start: chunk.line_start,
                line_end: chunk.line_end,
                chunk_artifact_path: self.paths.to_root_relative(&chunk_abs),
                embedding_artifact_path: self.paths.to_root_relative(&embedding_abs),
                embedding_model_id: model_id.to_string(),
                embedding_model_version: model_version.to_string(),
                embedding_dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
                embedding_compute_latency_ms: 0,
                chunking_strategy: chunk.strategy_id.clone(),
                chunking_version: AI_READY_CHUNKING_VERSION.to_string(),
                processing_pipeline_version: AI_READY_PROCESSING_PIPELINE_VERSION.to_string(),
                processing_duration_ms: 0,
                metadata_json,
                validation_status: ValidationStatus::Passed,
                validation_failed_checks_json: "[]".to_string(),
                validator_version: AI_READY_DATA_VALIDATOR_VERSION.to_string(),
            };

            let existing = self
                .storage
                .get_ai_silver_record(&chunk.silver_id)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
            if existing.is_none() {
                self.storage
                    .create_ai_silver_record(self.write_context, record)
                    .await
                    .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
                counters.silver_created = counters.silver_created.saturating_add(1);

                self.record_data_event(
                    FlightRecorderEventType::DataSilverCreated,
                    json!({
                        "type": "data_silver_created",
                        "silver_id": chunk.silver_id,
                        "bronze_ref": bronze_id,
                        "chunk_index": chunk.chunk_index,
                        "total_chunks": chunk.total_chunks,
                        "token_count": chunk.token_count,
                        "chunking_strategy": chunk.strategy_id,
                        "processing_duration_ms": 0,
                    }),
                )
                .await?;
            }

            counters.embedding_computed = counters.embedding_computed.saturating_add(1);
            self.record_data_event(
                FlightRecorderEventType::DataEmbeddingComputed,
                json!({
                    "type": "data_embedding_computed",
                    "silver_id": chunk.silver_id,
                    "model_id": model_id,
                    "model_version": model_version,
                    "dimensions": DEFAULT_EMBEDDING_DIMENSIONS,
                    "compute_latency_ms": 0,
                    "was_truncated": was_truncated,
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn extract_import_relationships(
        &self,
        source_text: &str,
        bronze_id: &str,
        graph_edges: &mut Vec<GraphEdge>,
        counters: &mut IngestCounters,
    ) -> Result<(), AiReadyDataError> {
        let mut seen: HashSet<String> = HashSet::new();
        for line in source_text.lines() {
            let trimmed = line.trim();
            let imported = if let Some(rest) = trimmed.strip_prefix("use ") {
                rest.split(';').next().map(str::trim)
            } else if trimmed.starts_with("import ") {
                trimmed
                    .split("from")
                    .last()
                    .map(str::trim)
                    .map(|value| value.trim_matches(['\"', '\'', ';']))
            } else {
                None
            };

            let Some(imported) = imported else {
                continue;
            };
            if imported.is_empty() || !seen.insert(imported.to_string()) {
                continue;
            }

            let target_id = format!(
                "ent_{}",
                deterministic_uuid_for_str(&format!("import:{imported}"))
            );
            let edge = GraphEdge {
                relationship_type: "imports".to_string(),
                source_id: bronze_id.to_string(),
                target_id: target_id.clone(),
                confidence: None,
            };
            graph_edges.push(edge);
            counters.relationships_extracted = counters.relationships_extracted.saturating_add(1);

            self.record_data_event(
                FlightRecorderEventType::DataRelationshipExtracted,
                json!({
                    "type": "data_relationship_extracted",
                    "relationship_type": "imports",
                    "source_id": bronze_id,
                    "target_id": target_id,
                    "confidence": null,
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn write_indexes(
        &self,
        model_id: &str,
        model_version: &str,
        graph_edges: &[GraphEdge],
    ) -> Result<u64, AiReadyDataError> {
        let records = self
            .storage
            .list_ai_silver_records(self.paths.workspace_id())
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        let mut documents: Vec<(String, String)> = Vec::new();
        let mut vectors: Vec<VectorIndexEntry> = Vec::new();
        for record in records.iter().filter(|record| record.is_current) {
            let chunk_abs = self
                .paths
                .handshake_root()
                .join(PathBuf::from(&record.chunk_artifact_path));
            if let Ok(text) = fs::read_to_string(&chunk_abs) {
                documents.push((record.silver_id.clone(), text));
            }

            let embedding_abs = self
                .paths
                .handshake_root()
                .join(PathBuf::from(&record.embedding_artifact_path));
            if let Ok(raw) = fs::read_to_string(&embedding_abs) {
                if let Ok(artifact) = serde_json::from_str::<EmbeddingArtifact>(&raw) {
                    vectors.push(VectorIndexEntry {
                        silver_id: record.silver_id.clone(),
                        vector: artifact.vector,
                    });
                }
            }
        }

        let keyword_index = build_keyword_index(documents, KeywordIndexConfig::default());
        let keyword_abs = self.paths.keyword_index_path();
        let keyword_existed = keyword_abs.exists();
        write_json_atomic(&keyword_abs, &keyword_index)?;

        if keyword_existed {
            self.record_data_event(
                FlightRecorderEventType::DataIndexUpdated,
                json!({
                    "type": "data_index_updated",
                    "index_kind": "keyword",
                    "update_kind": "update",
                    "records_affected": keyword_index.doc_count,
                    "duration_ms": 0,
                }),
            )
            .await?;
        } else {
            self.record_data_event(
                FlightRecorderEventType::DataIndexRebuilt,
                json!({
                    "type": "data_index_rebuilt",
                    "index_kind": "keyword",
                    "records_indexed": keyword_index.doc_count,
                    "duration_ms": 0,
                }),
            )
            .await?;
        }

        let mut vector_config = VectorIndexConfig::default();
        vector_config.dimensions = DEFAULT_EMBEDDING_DIMENSIONS;
        let vector_index = build_vector_index(
            vectors,
            vector_config,
            model_id.to_string(),
            model_version.to_string(),
        );
        let vector_abs = self.paths.vector_index_path(model_id, model_version);
        let vector_existed = vector_abs.exists();
        write_json_atomic(&vector_abs, &vector_index)?;

        if vector_existed {
            self.record_data_event(
                FlightRecorderEventType::DataIndexUpdated,
                json!({
                    "type": "data_index_updated",
                    "index_kind": "vector",
                    "update_kind": "update",
                    "records_affected": vector_index.entries.len() as u64,
                    "duration_ms": 0,
                }),
            )
            .await?;
        } else {
            self.record_data_event(
                FlightRecorderEventType::DataIndexRebuilt,
                json!({
                    "type": "data_index_rebuilt",
                    "index_kind": "vector",
                    "records_indexed": vector_index.entries.len() as u64,
                    "duration_ms": 0,
                }),
            )
            .await?;
        }

        let graph_abs = self.paths.graph_index_path();
        let graph_existed = graph_abs.exists();
        let mut edges = graph_edges.to_vec();
        edges.sort_by(|left, right| {
            left.relationship_type
                .cmp(&right.relationship_type)
                .then_with(|| left.source_id.cmp(&right.source_id))
                .then_with(|| left.target_id.cmp(&right.target_id))
        });
        let graph = GraphArtifact {
            schema_version: "1.0".to_string(),
            edges,
        };
        write_json_atomic(&graph_abs, &graph)?;

        if graph_existed {
            self.record_data_event(
                FlightRecorderEventType::DataIndexUpdated,
                json!({
                    "type": "data_index_updated",
                    "index_kind": "graph",
                    "update_kind": "update",
                    "records_affected": graph.edges.len() as u64,
                    "duration_ms": 0,
                }),
            )
            .await?;
        } else {
            self.record_data_event(
                FlightRecorderEventType::DataIndexRebuilt,
                json!({
                    "type": "data_index_rebuilt",
                    "index_kind": "graph",
                    "records_indexed": graph.edges.len() as u64,
                    "duration_ms": 0,
                }),
            )
            .await?;
        }

        Ok(3)
    }

    async fn emit_quality_metrics(&self) -> Result<(), AiReadyDataError> {
        let slo = QualitySLOs::default();
        let records = self
            .storage
            .list_ai_silver_records(self.paths.workspace_id())
            .await
            .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;

        let total_attempted = records.len().max(1) as f64;
        let passed = records
            .iter()
            .filter(|record| record.validation_status == ValidationStatus::Passed)
            .count() as f64;
        let pass_rate = passed / total_attempted;

        if pass_rate < slo.min_validation_pass_rate {
            self.record_data_event(
                FlightRecorderEventType::DataQualityDegradation,
                json!({
                    "type": "data_quality_degradation",
                    "metric_name": "validation_pass_rate",
                    "current_value": pass_rate,
                    "threshold": slo.min_validation_pass_rate,
                    "slo_target": slo.min_validation_pass_rate,
                }),
            )
            .await?;
        }

        let mut completeness_values: Vec<f64> = Vec::new();
        for record in &records {
            if let Ok(value) = serde_json::from_str::<Value>(&record.metadata_json) {
                if let Some(map) = value.as_object() {
                    let fields: Vec<&str> = map.keys().map(|key| key.as_str()).collect();
                    let required = [
                        "file_path",
                        "byte_start",
                        "byte_end",
                        "line_start",
                        "line_end",
                    ];
                    let ratio = crate::ai_ready_data::quality::metadata_completeness_ratio(
                        &required, &fields,
                    );
                    completeness_values.push(ratio);
                }
            }
        }
        let average = if completeness_values.is_empty() {
            0.0
        } else {
            completeness_values.iter().sum::<f64>() / completeness_values.len() as f64
        };

        if average < slo.min_metadata_completeness {
            self.record_data_event(
                FlightRecorderEventType::DataQualityDegradation,
                json!({
                    "type": "data_quality_degradation",
                    "metric_name": "metadata_completeness",
                    "current_value": average,
                    "threshold": slo.min_metadata_completeness,
                    "slo_target": slo.min_metadata_completeness,
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn hybrid_search(
        &self,
        query: &HybridQuery,
        model_id: &str,
        model_version: &str,
    ) -> Result<HybridSearchResponse, AiReadyDataError> {
        let vector_index_abs = self.paths.vector_index_path(model_id, model_version);
        let keyword_index_abs = self.paths.keyword_index_path();

        let vector_raw = fs::read_to_string(&vector_index_abs)?;
        let keyword_raw = fs::read_to_string(&keyword_index_abs)?;

        let vector_index = serde_json::from_str::<
            crate::ai_ready_data::indexing::VectorIndexArtifact,
        >(&vector_raw)?;
        let keyword_index = serde_json::from_str::<
            crate::ai_ready_data::indexing::KeywordIndexArtifact,
        >(&keyword_raw)?;

        let normalized = crate::ace::normalize_query(&query.query);
        let query_hash = sha256_hex(normalized.as_bytes());
        let request_id = compute_request_id(self.paths.workspace_id(), &query_hash, query);

        let (query_vector, _was_truncated) = embed_text_deterministic(
            &query.query,
            model_id,
            model_version,
            DEFAULT_EMBEDDING_DIMENSIONS,
            DEFAULT_EMBEDDING_MAX_INPUT_TOKENS,
        );
        let vector_candidates = vector_search(
            &vector_index,
            &query_vector,
            query.retrieval.vector_candidates,
        );

        let query_tokens = tokenize_keyword(&query.query);
        let keyword_candidates = keyword_search(
            &keyword_index,
            &query_tokens,
            query.retrieval.keyword_candidates,
        );

        let graph_candidates: Vec<(String, f64)> = Vec::new();
        let fused = hybrid_fuse_rrf(
            &vector_candidates,
            &keyword_candidates,
            &graph_candidates,
            &query.weights,
            query.retrieval.k,
        );
        let results = build_hybrid_results(&fused, &vector_candidates, &keyword_candidates);

        self.record_data_event(
            FlightRecorderEventType::DataRetrievalExecuted,
            json!({
                "type": "data_retrieval_executed",
                "request_id": request_id.clone(),
                "query_hash": query_hash,
                "query_intent": query.query_intent,
                "weights": {
                    "vector": query.weights.vector,
                    "keyword": query.weights.keyword,
                    "graph": query.weights.graph,
                },
                "results": {
                    "vector_candidates": vector_candidates.len(),
                    "keyword_candidates": keyword_candidates.len(),
                    "after_fusion": results.len(),
                    "final_count": results.len(),
                },
                "latency": {
                    "embedding_ms": 0,
                    "vector_search_ms": 0,
                    "keyword_search_ms": 0,
                    "total_ms": 0,
                },
                "reranking_used": false,
            }),
        )
        .await?;

        let retrieved_ids: Vec<String> = results
            .iter()
            .map(|result| result.silver_id.clone())
            .collect();

        let mut context_size_tokens: u64 = 0;
        for result in &results {
            let record = self
                .storage
                .get_ai_silver_record(&result.silver_id)
                .await
                .map_err(|err| AiReadyDataError::Storage(err.to_string()))?;
            let Some(record) = record else {
                continue;
            };
            let chunk_abs = self
                .paths
                .handshake_root()
                .join(PathBuf::from(&record.chunk_artifact_path));
            if let Ok(text) = fs::read_to_string(&chunk_abs) {
                context_size_tokens = context_size_tokens
                    .saturating_add(crate::ai_ready_data::chunking::estimate_tokens(&text) as u64);
            }
        }

        self.record_data_event(
            FlightRecorderEventType::DataContextAssembled,
            json!({
                "type": "data_context_assembled",
                "request_id": request_id.clone(),
                "selected_chunks": results.len(),
                "context_size_tokens": context_size_tokens,
            }),
        )
        .await?;

        let task_relevance_score = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|result| result.final_score).sum::<f64>() / results.len() as f64
        };

        let redundancy_score = redundancy_score_from_results(&results);
        let pollution_score = redundancy_score;
        let threshold = 0.5;
        if pollution_score > threshold {
            self.record_data_event(
                FlightRecorderEventType::DataPollutionAlert,
                json!({
                    "type": "data_pollution_alert",
                    "request_id": request_id.clone(),
                    "pollution_score": pollution_score,
                    "threshold": threshold,
                    "metrics": {
                        "task_relevance_score": task_relevance_score,
                        "drift_score": 0.0,
                        "redundancy_score": redundancy_score,
                        "stale_content_ratio": 0.0,
                    },
                    "context_size_tokens": context_size_tokens,
                }),
            )
            .await?;
        }

        Ok(HybridSearchResponse { retrieved_ids })
    }

    async fn evaluate_golden_query(
        &self,
        spec: &GoldenQuerySpec,
        retrieved_ids: &[String],
    ) -> Result<bool, AiReadyDataError> {
        let normalized = crate::ace::normalize_query(&spec.query);
        let query_hash = sha256_hex(normalized.as_bytes());
        let actual_mrr = mean_reciprocal_rank(&spec.expected_ids, retrieved_ids);
        let regression = actual_mrr < spec.expected_mrr;
        if regression {
            self.record_data_event(
                FlightRecorderEventType::DataGoldenQueryFailed,
                json!({
                    "type": "data_golden_query_failed",
                    "query_hash": query_hash,
                    "expected_ids": spec.expected_ids,
                    "retrieved_ids": retrieved_ids,
                    "expected_mrr": spec.expected_mrr,
                    "actual_mrr": actual_mrr,
                    "regression_from_baseline": true,
                }),
            )
            .await?;
        }
        Ok(regression)
    }
}

#[derive(Debug, Clone)]
struct HybridSearchResponse {
    retrieved_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum CodeSupport {
    Supported(CodeLanguage),
    UnsupportedCode,
    NotCode,
}

fn classify_code_language(path: &Path) -> CodeSupport {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "rs" => CodeSupport::Supported(CodeLanguage::Rust),
        "ts" | "tsx" => CodeSupport::Supported(CodeLanguage::TypeScript),
        "js" | "jsx" => CodeSupport::Supported(CodeLanguage::JavaScript),
        "md" | "markdown" | "txt" | "rst" | "adoc" | "toml" | "json" | "yaml" | "yml" => {
            CodeSupport::NotCode
        }
        "py" | "go" | "java" | "kt" | "kts" | "c" | "cc" | "cpp" | "cxx" | "h" | "hpp" | "cs"
        | "rb" | "php" | "swift" | "scala" | "lua" | "sh" | "bash" | "ps1" => {
            CodeSupport::UnsupportedCode
        }
        _ => CodeSupport::NotCode,
    }
}

fn classify_content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "md" | "markdown" => "text/markdown",
        "rs" => "text/x-rust",
        "ts" | "tsx" => "text/typescript",
        "js" | "jsx" => "text/javascript",
        "json" => "application/json",
        _ => "text/plain",
    }
}

fn validate_relative_path(rel_path: &str) -> Result<PathBuf, AiReadyDataError> {
    let raw = rel_path.trim();
    if raw.is_empty() {
        return Err(AiReadyDataError::InvalidInput("path"));
    }

    let sanitized = raw.replace('\\', "/");
    let candidate = PathBuf::from(sanitized);

    let mut out = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(AiReadyDataError::InvalidInput("path"));
            }
            Component::CurDir => {}
            Component::Normal(value) => out.push(value),
        }
    }

    if out.as_os_str().is_empty() {
        return Err(AiReadyDataError::InvalidInput("path"));
    }

    Ok(out)
}

fn compute_bronze_id(workspace_id: &str, rel_path: &str, content_hash: &str) -> String {
    let raw =
        format!("workspace_id={workspace_id}\nrel_path={rel_path}\ncontent_hash={content_hash}\n");
    format!("brz_{}", deterministic_uuid_for_str(&raw))
}

fn compute_request_id(workspace_id: &str, query_hash: &str, query: &HybridQuery) -> String {
    let raw = format!(
        "workspace_id={workspace_id}\nquery_hash={query_hash}\nquery_intent={}\nweights_vector={}\nweights_keyword={}\nweights_graph={}\nk={}\nvector_candidates={}\nkeyword_candidates={}\ngraph_hops={}\nrerank={}\n",
        query.query_intent,
        query.weights.vector,
        query.weights.keyword,
        query.weights.graph,
        query.retrieval.k,
        query.retrieval.vector_candidates,
        query.retrieval.keyword_candidates,
        query.retrieval.graph_hops,
        query.rerank
    );
    format!("req_{}", deterministic_uuid_for_str(&raw))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), AiReadyDataError> {
    let Some(parent) = path.parent() else {
        return Err(AiReadyDataError::InvalidInput("path"));
    };
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or(AiReadyDataError::InvalidInput("path"))?
        .to_string_lossy();
    let tmp_path = path.with_file_name(format!("{file_name}.tmp-{}", Uuid::new_v4()));

    fs::write(&tmp_path, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AiReadyDataError> {
    let bytes = serde_json::to_vec(value)?;
    write_bytes_atomic(path, &bytes)
}

fn deterministic_uuid_for_str(value: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn redundancy_score_from_results(
    results: &[crate::ai_ready_data::retrieval::HybridSearchResult],
) -> f64 {
    if results.len() <= 1 {
        return 0.0;
    }
    let unique: HashSet<&str> = results
        .iter()
        .map(|result| result.silver_id.as_str())
        .collect();
    1.0 - (unique.len() as f64 / results.len() as f64)
}

fn mean_reciprocal_rank(expected_ids: &[String], retrieved_ids: &[String]) -> f64 {
    let expected: HashSet<&str> = expected_ids.iter().map(|value| value.as_str()).collect();
    for (idx, retrieved_id) in retrieved_ids.iter().enumerate() {
        if expected.contains(retrieved_id.as_str()) {
            return 1.0 / ((idx + 1) as f64);
        }
    }
    0.0
}
