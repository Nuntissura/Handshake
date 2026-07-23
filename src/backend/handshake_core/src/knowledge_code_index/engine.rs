//! WP-KERNEL-009 CodeIndexingAndNavigation engine (orchestrator) + MT-108
//! partial-failure handling.
//!
//! Master Spec anchor: 2.3.13.11. The engine turns a registered code
//! `KnowledgeSource` into graph records: it parses the file (MT-097), extracts
//! symbols/docs/relationships (MT-098..MT-104), and writes each THROUGH the
//! storage layer (`storage::knowledge::KnowledgeStore`):
//!   * one `file` entity for the source file,
//!   * one `ast`-kind span per symbol (the citeable evidence unit),
//!   * one `symbol` entity per symbol, anchored to its span,
//!   * `contains` edges file -> symbol,
//!   * `references`/`depends_on`/`implements`/`validates`/`documents` edges
//!     (MT-104/102/103) once both endpoints resolve,
//!   * `text`-kind spans + `concept` entities for doc/TODO passages,
//!   * `schema`/`command`/`concept` entities for config facts (MT-101).
//!
//! The engine also maintains the per-file index state in `knowledge_code_files`
//! (0170) for staleness (MT-107). Every write leaves an EventLedger receipt
//! carrying actor/session/correlation identity (backend-navigation receipt law).
//!
//! MT-108 partial-failure: indexing a directory NEVER fails because one file
//! cannot be parsed OR cannot be read. A file whose grammar init/parse fails
//! (typed `CodeParseError`), whose tree-sitter FFI panics (caught via
//! `catch_unwind` in the parser and surfaced as a typed error), or whose bytes
//! are not valid UTF-8 / unreadable, is recorded with `parse_status = failed`, a
//! typed receipt, AND a durable `knowledge_code_repair_queue` entry (0230) that
//! holds it for re-parse with a typed reason class. A file that parses with
//! syntax errors but still yields symbols is `partial`. The run continues and
//! returns a per-file summary.
//!
//! No SQLite, no external LSP. The engine reuses the shared AppState pool (one
//! `PostgresDatabase`, no second pool).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    KnowledgeCodeLanguage, KnowledgeCodeParseStatus, KnowledgeCodeRepairReason, KnowledgeEdgeType,
    KnowledgeEntityKind, KnowledgeExtractionStatus, KnowledgeIndexRunOutcome,
    KnowledgeParserStatus, KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind,
    KnowledgeSpanKind, KnowledgeStore, NewKnowledgeCodeRepairEntry, NewKnowledgeEdge,
    NewKnowledgeEntity, NewKnowledgeSource, NewKnowledgeSpan, UpsertKnowledgeCodeFile,
    derive_knowledge_relationship_id,
};
use crate::storage::postgres::{append_kernel_event_with_executor, PostgresDatabase};
use crate::storage::{Database, StorageError};
use crate::swarm_orchestration::state_recovery::{
    AgentCapability, AgentLaneIdentity, ClaimScope, IndexingLeaseRecord, IndexingLeaseRequest,
    ParallelSwarmStateRecoveryStore, QuietBackgroundPolicy, QuietBackgroundWorkKind,
    QuietBackgroundWorkRecord, QuietBackgroundWorkRequest,
};

use super::config_schema::{detect_config_format, extract_config_facts, ConfigFactKind};
use super::docs_todo::{extract_doc_passages, extract_operator_strings, DocPassageKind};
use super::parser::{CodeLanguage, CodeParserAdapter};
use super::perf::{CodeIndexBudget, PerfSample};
use super::relationships::{extract_relationships, RelationshipKind};
use super::symbols::{extract_symbols, ExtractedSymbol, SymbolKind};
use super::tests_map::{extract_test_mappings, TestMapping};
use super::{CodeIndexError, CodeIndexResult, CODE_EXTRACTOR_VERSION};

use crate::knowledge_ingestion::engine::PreparedCodeNavFile;

/// Backend-navigation context (spec 2.3.13.11): every engine mutation carries
/// actor id, session id, and correlation id into its receipts.
#[derive(Clone, Debug)]
pub struct CodeIndexContext {
    pub actor: KernelActor,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub correlation_id: Option<String>,
}

impl CodeIndexContext {
    pub fn validate(&self) -> CodeIndexResult<()> {
        if self.actor.actor_id().trim().is_empty() {
            return Err(CodeIndexError::Validation(
                "code index context requires a non-empty actor id".to_string(),
            ));
        }
        if self.kernel_task_run_id.trim().is_empty() || self.session_run_id.trim().is_empty() {
            return Err(CodeIndexError::Validation(
                "code index context requires kernel_task_run_id and session_run_id".to_string(),
            ));
        }
        Ok(())
    }
}

/// The code-index engine. Cheap to construct; wraps a pooled handle.
pub struct CodeIndexEngine {
    db: Arc<PostgresDatabase>,
}

/// The outcome of indexing one code file.
#[derive(Debug, Clone)]
pub struct CodeFileIndexOutcome {
    pub source_id: String,
    pub relative_path: String,
    pub language: Option<CodeLanguage>,
    pub parse_status: KnowledgeCodeParseStatus,
    pub symbols_indexed: usize,
    pub edges_indexed: usize,
    pub doc_passages_indexed: usize,
    pub config_facts_indexed: usize,
    /// True when the file could not be parsed at all (MT-108) — the run still
    /// continued.
    pub failed: bool,
    pub failure_reason: Option<String>,
    pub receipt_event_id: String,
}

#[derive(Debug, Clone)]
pub struct QuietCodeIndexRun {
    pub index_run_id: String,
    pub indexing_lease: IndexingLeaseRecord,
    pub quiet_receipt: QuietBackgroundWorkRecord,
}

struct BatchParsedCodeFile {
    source_id: String,
    relative_path: String,
    language: CodeLanguage,
    parser_version: String,
    content_hash: String,
    symbols: Vec<ExtractedSymbol>,
    receipt: NewKernelEvent,
    perf_failure: Option<Value>,
}

struct BatchSpan {
    span_id: String,
    source_id: String,
    range_start: i64,
    range_end: i64,
    line_start: i32,
    line_end: i32,
    section_path: String,
    content_sha256: String,
    parser_version: String,
    receipt_event_id: String,
    index_run_id: String,
}

fn batch_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}

fn new_code_index_run_id() -> String {
    format!("KIR-{}", Uuid::now_v7().simple())
}

impl CodeIndexEngine {
    pub fn new(db: Arc<PostgresDatabase>) -> Self {
        Self { db }
    }

    pub fn from_database(db: Arc<PostgresDatabase>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &PostgresDatabase {
        &self.db
    }

    /// Append one EventLedger receipt event carrying the backend-navigation
    /// identity.
    async fn append_receipt_event(
        &self,
        ctx: &CodeIndexContext,
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: Value,
    ) -> CodeIndexResult<String> {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            event_type,
            ctx.actor.clone(),
        )
        .aggregate(aggregate_type, aggregate_id)
        .source_component("knowledge_code_index")
        .payload(payload);
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        let event = builder
            .build()
            .map_err(|err| CodeIndexError::Kernel(err.to_string()))?;
        let stored = self.db.append_kernel_event(event).await?;
        Ok(stored.event_id)
    }

    async fn append_terminal_receipt_event(
        &self,
        ctx: &CodeIndexContext,
        event_type: KernelEventType,
        index_run_id: &str,
        kind: &str,
    ) -> CodeIndexResult<String> {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            event_type,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_code_index_run", index_run_id)
        .idempotency_key(format!("knowledge-code-index:finish:{index_run_id}:{kind}"))
        .source_component("knowledge_code_index")
        .payload(json!({"kind": kind, "index_run_id": index_run_id}));
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        let event = builder
            .build()
            .map_err(|err| CodeIndexError::Kernel(err.to_string()))?;
        let stored = self.db.append_kernel_event(event).await?;
        Ok(stored.event_id)
    }

    /// Start a code-index run (reuses the shared knowledge_index_runs
    /// lifecycle). Returns the run id to thread through per-file indexing.
    pub async fn start_run(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        root_id: Option<&str>,
    ) -> CodeIndexResult<String> {
        ctx.validate()?;
        let index_run_id = new_code_index_run_id();
        self.start_run_with_id(ctx, &index_run_id, workspace_id, root_id)
            .await?;
        Ok(index_run_id)
    }

    /// Finish a code-index run with a terminal EventLedger receipt. Routes that
    /// fan out per-file work must call this on both success and failure so a
    /// client timeout or one bad file cannot leave a durable `started` run.
    pub async fn finish_run(
        &self,
        ctx: &CodeIndexContext,
        index_run_id: &str,
        outcome: KnowledgeIndexRunOutcome,
    ) -> CodeIndexResult<()> {
        let (event_type, kind) = match &outcome {
            KnowledgeIndexRunOutcome::Completed { .. } => (
                KernelEventType::KnowledgeIndexRunCompleted,
                "code_index_run_completed",
            ),
            KnowledgeIndexRunOutcome::Failed { .. } => (
                KernelEventType::KnowledgeIndexRunFailed,
                "code_index_run_failed",
            ),
            KnowledgeIndexRunOutcome::Cancelled { .. } => (
                KernelEventType::KnowledgeIndexRunCancelled,
                "code_index_run_cancelled",
            ),
        };
        let finish_receipt_event_id = self
            .append_terminal_receipt_event(ctx, event_type, index_run_id, kind)
            .await?;
        self.db
            .finish_knowledge_index_run(
                index_run_id,
                outcome,
                Some(finish_receipt_event_id.as_str()),
            )
            .await?;
        Ok(())
    }

    /// Terminalize a run with one bounded retry. A transient connection/lock
    /// failure must not strand the durable run in `started`; the operation is
    /// idempotent through the EventLedger key and the run-state guard.
    pub async fn finish_run_with_retry(
        &self,
        ctx: &CodeIndexContext,
        index_run_id: &str,
        outcome: KnowledgeIndexRunOutcome,
    ) -> CodeIndexResult<()> {
        const TERMINALIZATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        let first = match tokio::time::timeout(
            TERMINALIZATION_TIMEOUT,
            self.finish_run(ctx, index_run_id, outcome.clone()),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(CodeIndexError::Kernel(format!(
                "terminalization timed out after {}s",
                TERMINALIZATION_TIMEOUT.as_secs()
            ))),
        };
        match first {
            Ok(()) => Ok(()),
            Err(first_error) => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                tokio::time::timeout(
                    TERMINALIZATION_TIMEOUT,
                    self.finish_run(ctx, index_run_id, outcome),
                )
                .await
                .map_err(|_| {
                    CodeIndexError::Kernel(format!(
                        "terminalization retry timed out after {}s",
                        TERMINALIZATION_TIMEOUT.as_secs()
                    ))
                })?
                .map_err(|retry_error| {
                    CodeIndexError::Kernel(format!(
                        "terminalization failed after retry: {first_error}; {retry_error}"
                    ))
                })
            }
        }
    }

    /// Try the low-round-trip code-index writer used by the large-code
    /// navigation route. The guard is deliberately narrow: only clean code
    /// files with no relationship/doc/test/operator passages use it. Any
    /// richer input returns `Ok(None)` so the established per-file writer
    /// remains the semantic fallback.
    pub(crate) async fn try_index_prepared_batch(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        prepared: &[PreparedCodeNavFile],
        persisted_sources: &[(String, String)],
        index_run_id: &str,
    ) -> CodeIndexResult<Option<Vec<CodeFileIndexOutcome>>> {
        ctx.validate()?;
        if prepared.is_empty() || prepared.len() != persisted_sources.len() {
            return Ok(None);
        }

        let mut parsed = Vec::with_capacity(prepared.len());
        for (file, (source_id, relative_path)) in prepared.iter().zip(persisted_sources) {
            if &file.relative_path != relative_path {
                return Ok(None);
            }
            let Some(language) = super::parser::detect_code_language(relative_path) else {
                return Ok(None);
            };
            let Ok(text) = std::str::from_utf8(&file.content) else {
                return Ok(None);
            };
            let adapter = CodeParserAdapter::new(language);
            let parser_version = adapter.parser_version();
            let started = Instant::now();
            let Ok(tree) = adapter.parse(text) else {
                return Ok(None);
            };
            if tree.root_has_error {
                return Ok(None);
            }
            let symbols = extract_symbols(&tree, text);
            let mut docs = extract_doc_passages(text);
            let operators = extract_operator_strings(&tree, text);
            let relationships = extract_relationships(&tree, text, &symbols);
            let test_mappings = extract_test_mappings(&tree, text, &symbols);
            docs.extend(operators);
            if !docs.is_empty() || !relationships.is_empty() || !test_mappings.is_empty() {
                return Ok(None);
            }
            let perf = PerfSample::measure(
                &CodeIndexBudget::default(),
                relative_path,
                text.lines().count(),
                started.elapsed().as_secs_f64() * 1000.0,
            );
            let perf_failure = (!perf.within_budget).then(|| {
                perf_sample_json(&perf, &CodeIndexBudget::default())
            });
            let mut builder = NewKernelEvent::builder(
                ctx.kernel_task_run_id.clone(),
                ctx.session_run_id.clone(),
                KernelEventType::KnowledgeValidationRecorded,
                ctx.actor.clone(),
            )
            .aggregate("knowledge_code_index_file", source_id)
            .source_component("knowledge_code_index")
            .payload(json!({
                "kind": "code_file_indexed",
                "workspace_id": workspace_id,
                "source_id": source_id,
                "relative_path": relative_path,
                "language": language.as_str(),
                "parser_version": &parser_version,
                "parse_status": KnowledgeCodeParseStatus::Parsed.as_str(),
                "symbols": symbols.len(),
                "doc_passages": 0,
                "operator_strings": 0,
                "relationships": 0,
                "content_hash": &file.content_hash,
                "extractor_version": CODE_EXTRACTOR_VERSION,
                "perf_budget": perf_failure.clone(),
            }));
            if let Some(correlation_id) = &ctx.correlation_id {
                builder = builder.correlation_id(correlation_id.clone());
            }
            parsed.push(BatchParsedCodeFile {
                source_id: source_id.clone(),
                relative_path: relative_path.clone(),
                language,
                parser_version,
                content_hash: file.content_hash.clone(),
                symbols,
                receipt: builder
                    .build()
                    .map_err(|err| CodeIndexError::Kernel(err.to_string()))?,
                perf_failure,
            });
        }

        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let result = self
            .persist_prepared_batch_tx(&mut tx, workspace_id, index_run_id, parsed)
            .await;
        match result {
            Ok(outcomes) => {
                tx.commit().await.map_err(StorageError::from)?;
                Ok(Some(outcomes))
            }
            Err(error) => {
                let _ = tx.rollback().await;
                Err(error)
            }
        }
    }

    async fn persist_prepared_batch_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        workspace_id: &str,
        index_run_id: &str,
        parsed: Vec<BatchParsedCodeFile>,
    ) -> CodeIndexResult<Vec<CodeFileIndexOutcome>> {
        let events = parsed.iter().map(|item| item.receipt.clone()).collect::<Vec<_>>();
        let receipt_ids = append_batch_events(tx, &events).await?;

        let mut spans = Vec::new();
        let mut span_by_source = HashMap::new();
        for item in &parsed {
            let receipt_id = receipt_ids
                .get(&item.receipt.idempotency_key)
                .ok_or_else(|| CodeIndexError::Storage(StorageError::NotFound("code receipt event")))?;
            for symbol in &item.symbols {
                let span_id = batch_id("KSP");
                span_by_source.insert((item.source_id.clone(), symbol.symbol_path.clone()), span_id.clone());
                spans.push(BatchSpan {
                    span_id,
                    source_id: item.source_id.clone(),
                    range_start: symbol.start_byte as i64,
                    range_end: symbol.end_byte as i64,
                    line_start: symbol.start_line as i32,
                    line_end: symbol.end_line as i32,
                    section_path: symbol.symbol_path.clone(),
                    content_sha256: sha256_hex(format!("{}|{}|{}", symbol.node_kind, symbol.symbol_path, symbol.start_byte).as_bytes()),
                    parser_version: item.parser_version.clone(),
                    receipt_event_id: receipt_id.clone(),
                    index_run_id: index_run_id.to_string(),
                });
            }
        }
        insert_batch_spans(tx, &spans).await?;

        let mut entity_rows = Vec::new();
        for item in &parsed {
            entity_rows.push((
                item.source_id.clone(),
                KnowledgeEntityKind::File,
                format!("file:{}", item.relative_path),
                item.relative_path.clone(),
                json!({"extractor":"knowledge_code_index","extractor_version":CODE_EXTRACTOR_VERSION,"language":item.language.as_str()}),
                Some(item.source_id.clone()),
            ));
            for symbol in &item.symbols {
                entity_rows.push((
                    item.source_id.clone(),
                    KnowledgeEntityKind::Symbol,
                    symbol.entity_key(item.language, &item.relative_path),
                    symbol.name.clone(),
                    json!({"extractor":"knowledge_code_index","extractor_version":CODE_EXTRACTOR_VERSION,"language":item.language.as_str(),"symbol_kind":symbol.kind.as_str(),"node_kind":symbol.node_kind,"symbol_path":symbol.symbol_path}),
                    Some(item.source_id.clone()),
                ));
            }
        }
        let entity_ids = insert_batch_entities(tx, workspace_id, index_run_id, &entity_rows).await?;

        let mut entity_spans = Vec::new();
        let mut edges = Vec::new();
        let mut span_offset = 0usize;
        for item in &parsed {
            let file_key = format!("file:{}", item.relative_path);
            let file_id = entity_ids.get(&(KnowledgeEntityKind::File.as_str().to_string(), file_key.clone())).cloned().ok_or_else(|| CodeIndexError::Storage(StorageError::NotFound("file entity")))?;
            for symbol in &item.symbols {
                let symbol_key = symbol.entity_key(item.language, &item.relative_path);
                let symbol_id = entity_ids.get(&(KnowledgeEntityKind::Symbol.as_str().to_string(), symbol_key.clone())).cloned().ok_or_else(|| CodeIndexError::Storage(StorageError::NotFound("symbol entity")))?;
                let span = spans.get(span_offset).ok_or_else(|| CodeIndexError::Storage(StorageError::NotFound("symbol span")))?;
                span_offset += 1;
                entity_spans.push((symbol_id.clone(), span.span_id.clone()));
                let rel_id = derive_knowledge_relationship_id(KnowledgeEdgeType::Contains, KnowledgeEntityKind::File, &file_key, KnowledgeEntityKind::Symbol, &symbol_key);
                edges.push((rel_id, file_id.clone(), symbol_id, span.span_id.clone()));
            }
        }
        insert_batch_entity_spans(tx, index_run_id, &entity_spans).await?;
        let edge_ids = insert_batch_edges(tx, workspace_id, index_run_id, &edges).await?;
        insert_batch_edge_spans(tx, index_run_id, &edge_ids, &edges).await?;
        insert_batch_code_files(tx, workspace_id, index_run_id, &parsed, &receipt_ids, &entity_ids).await?;
        update_batch_sources(tx, &parsed, &receipt_ids).await?;

        let mut outcomes = Vec::with_capacity(parsed.len());
        for item in parsed {
            let receipt_id = receipt_ids.get(&item.receipt.idempotency_key).cloned().ok_or_else(|| CodeIndexError::Storage(StorageError::NotFound("code receipt event")))?;
            outcomes.push(CodeFileIndexOutcome {
                source_id: item.source_id,
                relative_path: item.relative_path,
                language: Some(item.language),
                parse_status: KnowledgeCodeParseStatus::Parsed,
                symbols_indexed: item.symbols.len(),
                edges_indexed: item.symbols.len(),
                doc_passages_indexed: 0,
                config_facts_indexed: 0,
                failed: false,
                failure_reason: None,
                receipt_event_id: receipt_id,
            });
        }
        Ok(outcomes)
    }

    async fn start_run_with_id(
        &self,
        ctx: &CodeIndexContext,
        index_run_id: &str,
        workspace_id: &str,
        root_id: Option<&str>,
    ) -> CodeIndexResult<()> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        if let Err(error) = self
            .start_run_with_id_tx(&mut tx, ctx, index_run_id, workspace_id, root_id)
            .await
        {
            let _ = tx.rollback().await;
            return Err(error);
        }
        tx.commit().await.map_err(StorageError::from)?;
        Ok(())
    }

    async fn start_run_with_id_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &CodeIndexContext,
        index_run_id: &str,
        workspace_id: &str,
        root_id: Option<&str>,
    ) -> CodeIndexResult<()> {
        let start_event = self.build_receipt_event(
            ctx,
            KernelEventType::KnowledgeIndexRunStarted,
            "knowledge_code_index_run",
            workspace_id,
            json!({
                "kind": "code_index_run_started",
                "workspace_id": workspace_id,
                "root_id": root_id,
                "extractor_version": CODE_EXTRACTOR_VERSION,
            }),
        )?;
        let start_event_id = match append_kernel_event_with_executor(&mut **tx, start_event).await {
            Ok(event) => event.event_id,
            Err(error) => return Err(error.into()),
        };
        let inserted = sqlx::query(
            r#"
            INSERT INTO knowledge_index_runs
                (index_run_id, workspace_id, root_id, scope, actor_kind,
                 actor_id, worktree_id, start_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(index_run_id)
        .bind(workspace_id)
        .bind(root_id)
        .bind(json!({"index_kind": "code", "extractor_version": CODE_EXTRACTOR_VERSION}))
        .bind(ctx.actor.actor_kind())
        .bind(ctx.actor.actor_id())
        .bind(Option::<String>::None)
        .bind(start_event_id)
        .execute(&mut **tx)
        .await;
        if let Err(error) = inserted {
            return Err(StorageError::from(error).into());
        }
        Ok(())
    }

    fn build_receipt_event(
        &self,
        ctx: &CodeIndexContext,
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: Value,
    ) -> CodeIndexResult<NewKernelEvent> {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            event_type,
            ctx.actor.clone(),
        )
        .aggregate(aggregate_type, aggregate_id)
        .source_component("knowledge_code_index")
        .payload(payload);
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        builder
            .build()
            .map_err(|err| CodeIndexError::Kernel(err.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_quiet_run(
        &self,
        ctx: &CodeIndexContext,
        swarm_state: &ParallelSwarmStateRecoveryStore,
        lane: AgentLaneIdentity,
        wp_id: &str,
        mt_id: &str,
        workspace_id: &str,
        root_id: Option<&str>,
        priority: i32,
        ttl_seconds: i64,
    ) -> CodeIndexResult<QuietCodeIndexRun> {
        ctx.validate()?;
        let index_run_id = new_code_index_run_id();
        let source_root_id = root_id.unwrap_or(workspace_id).to_string();
        let scope = ClaimScope::IndexRun {
            workspace_id: workspace_id.to_string(),
            source_root_id,
        };
        if !lane
            .capabilities()
            .contains(&AgentCapability::WriteLocalIndex)
        {
            return Err(CodeIndexError::Validation(format!(
                "quiet indexing lane {} lacks required capability {:?}",
                lane.lane_id,
                AgentCapability::WriteLocalIndex
            )));
        }
        swarm_state
            .reclaim_orphaned_indexing_leases()
            .await
            .map_err(|err| {
                CodeIndexError::Validation(format!("quiet indexing lease preflight failed: {err}"))
            })?;
        let lease_request = IndexingLeaseRequest {
            workspace_id: workspace_id.to_string(),
            wp_id: wp_id.to_string(),
            mt_id: mt_id.to_string(),
            scope,
            lane: lane.clone(),
            session_id: ctx.session_run_id.clone(),
            index_run_id: index_run_id.clone(),
            priority,
            ttl_seconds,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        };
        let quiet_request = QuietBackgroundWorkRequest {
            lane,
            workspace_id: workspace_id.to_string(),
            wp_id: wp_id.to_string(),
            mt_id: mt_id.to_string(),
            work_kind: QuietBackgroundWorkKind::Indexing,
            subject_id: index_run_id.clone(),
            session_id: ctx.session_run_id.clone(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
            evidence_ref: format!("knowledge-index-run://{index_run_id}"),
        };
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let indexing_lease = match swarm_state
            .try_acquire_indexing_lease_tx(&mut tx, &lease_request)
            .await
        {
            Ok(Some(record)) => record,
            Ok(None) => {
                let _ = tx.rollback().await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing run {index_run_id} did not acquire index lease"
                )));
            }
            Err(error) => {
                let _ = tx.rollback().await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing lease failed: {error}"
                )));
            }
        };
        if let Err(error) = self
            .start_run_with_id_tx(&mut tx, ctx, &index_run_id, workspace_id, root_id)
            .await
        {
            let _ = tx.rollback().await;
            return Err(error);
        }
        let quiet_receipt = match swarm_state
            .record_quiet_background_work_tx(&mut tx, quiet_request)
            .await
        {
            Ok(record) => record,
            Err(error) => {
                let _ = tx.rollback().await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing receipt failed: {error}"
                )));
            }
        };
        tx.commit().await.map_err(StorageError::from)?;
        Ok(QuietCodeIndexRun {
            index_run_id,
            indexing_lease,
            quiet_receipt,
        })
    }

    /// Index one code file's content. The source MUST already be registered as
    /// a `KnowledgeSource` (the ingestion group does this); the caller passes
    /// its `source_id`, the repo-relative path, and the exact file text.
    ///
    /// MT-108: a parse failure is captured (typed receipt + `failed` status)
    /// and returned in the outcome, never propagated as an error that would
    /// abort a directory run.
    pub async fn index_code_source(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        text: &str,
        index_run_id: Option<&str>,
    ) -> CodeIndexResult<CodeFileIndexOutcome> {
        ctx.validate()?;

        // Route: code file -> AST extraction; config file -> config facts.
        if let Some(language) = super::parser::detect_code_language(relative_path) {
            self.index_code_file(
                ctx,
                workspace_id,
                source_id,
                relative_path,
                text,
                language,
                index_run_id,
            )
            .await
        } else if let Some(format) = detect_config_format(relative_path) {
            self.index_config_file(
                ctx,
                workspace_id,
                source_id,
                relative_path,
                text,
                format,
                index_run_id,
            )
            .await
        } else {
            Err(CodeIndexError::Validation(format!(
                "path '{relative_path}' is neither a recognised code nor config file"
            )))
        }
    }

    /// The code-file (AST) indexing path.
    #[allow(clippy::too_many_arguments)]
    async fn index_code_file(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        text: &str,
        language: CodeLanguage,
        index_run_id: Option<&str>,
    ) -> CodeIndexResult<CodeFileIndexOutcome> {
        let content_hash = sha256_hex(text.as_bytes());
        let adapter = CodeParserAdapter::new(language);
        let parser_version = adapter.parser_version();
        let perf_started = Instant::now();

        // MT-108: a genuine parse failure (grammar init / no tree) OR a caught
        // tree-sitter FFI panic is captured here. `adapter.parse` wraps the FFI
        // in `catch_unwind` and returns a typed `Parse` error for a panic, so a
        // hostile file degrades to one failed file, never a dead run. A panic is
        // classified PANIC; any other parse failure is PARSE_ERROR.
        let tree = match adapter.parse(text) {
            Ok(tree) => tree,
            Err(err) => {
                let reason = err.to_string();
                let reason_class = if reason.contains("panicked") {
                    KnowledgeCodeRepairReason::Panic
                } else {
                    KnowledgeCodeRepairReason::ParseError
                };
                return self
                    .record_parse_failure(
                        ctx,
                        workspace_id,
                        source_id,
                        relative_path,
                        Some(language),
                        &content_hash,
                        &parser_version,
                        index_run_id,
                        &reason,
                        reason_class,
                    )
                    .await;
            }
        };

        let symbols = extract_symbols(&tree, text);
        // Doc/TODO/safety passages (line scanner) + operator-facing strings (AST
        // walk of output sinks). MT-103: operator strings are a DISTINCT passage
        // kind (`operator_string`) written as their own concept entity, never
        // merged with doc-comment or marker passages.
        let mut doc_passages = extract_doc_passages(text);
        let operator_strings = extract_operator_strings(&tree, text);
        let operator_string_count = operator_strings.len();
        doc_passages.extend(operator_strings);
        let relationships = extract_relationships(&tree, text, &symbols);
        let test_mappings = extract_test_mappings(&tree, text, &symbols);
        let perf_budget = CodeIndexBudget::default();
        let perf_sample = PerfSample::measure(
            &perf_budget,
            relative_path,
            text.lines().count(),
            perf_started.elapsed().as_secs_f64() * 1000.0,
        );
        let perf_budget_json = perf_sample_json(&perf_sample, &perf_budget);

        // A tree with syntax errors that still yielded symbols => partial.
        let parse_status = if tree.root_has_error {
            KnowledgeCodeParseStatus::Partial
        } else {
            KnowledgeCodeParseStatus::Parsed
        };

        // Receipt first (FK target for spans).
        let receipt_event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::KnowledgeValidationRecorded,
                "knowledge_code_index_file",
                source_id,
                json!({
                    "kind": "code_file_indexed",
                    "workspace_id": workspace_id,
                    "source_id": source_id,
                    "relative_path": relative_path,
                    "language": language.as_str(),
                    "parser_version": parser_version,
                    "parse_status": parse_status.as_str(),
                    "symbols": symbols.len(),
                    "doc_passages": doc_passages.len(),
                    "operator_strings": operator_string_count,
                    "relationships": relationships.len(),
                    "content_hash": content_hash,
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                    "perf_budget": perf_budget_json,
                }),
            )
            .await?;

        // The file entity (kind `file`).
        let file_entity = self
            .db
            .upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace_id.to_string(),
                entity_kind: KnowledgeEntityKind::File,
                entity_key: format!("file:{relative_path}"),
                display_name: relative_path.to_string(),
                detection_provenance: json!({
                    "extractor": "knowledge_code_index",
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                    "language": language.as_str(),
                }),
                primary_source_id: Some(source_id.to_string()),
                detected_in_run: index_run_id.map(|s| s.to_string()),
                evidence_span_ids: Vec::new(),
            })
            .await?;

        // Symbols: span + entity + contains edge each. Map symbol_path ->
        // (entity_id, span_id) so relationship resolution can wire edges.
        let mut symbol_index: HashMap<String, ResolvedSymbol> = HashMap::new();
        let mut test_symbol_index: HashMap<SymbolIdentityKey, ResolvedSymbol> = HashMap::new();
        for symbol in &symbols {
            let resolved = self
                .write_symbol(
                    ctx,
                    workspace_id,
                    source_id,
                    relative_path,
                    language,
                    &parser_version,
                    &receipt_event_id,
                    index_run_id,
                    symbol,
                )
                .await?;
            // contains edge: file -> symbol (evidence = the symbol span).
            self.db
                .upsert_knowledge_edge(NewKnowledgeEdge {
                    workspace_id: workspace_id.to_string(),
                    edge_type: KnowledgeEdgeType::Contains,
                    source_entity_id: file_entity.entity_id.clone(),
                    target_entity_id: resolved.entity_id.clone(),
                    extractor_version: CODE_EXTRACTOR_VERSION.to_string(),
                    confidence: 1.0,
                    detected_in_run: index_run_id.map(|s| s.to_string()),
                    evidence_span_ids: vec![resolved.span_id.clone()],
                })
                .await?;
            if symbol.kind == SymbolKind::Test {
                test_symbol_index.insert(symbol_identity_key(symbol), resolved.clone());
            }
            symbol_index
                .entry(symbol.symbol_path.clone())
                .or_insert(resolved);
        }

        // Doc/TODO passages: text span + concept entity, documents edge to the
        // file. Then resolve documents edges onto enclosing symbols when a
        // passage immediately precedes one (best-effort; file-level otherwise).
        let mut doc_passages_indexed = 0usize;
        for passage in &doc_passages {
            let span = self
                .db
                .create_knowledge_span(NewKnowledgeSpan {
                    source_id: source_id.to_string(),
                    span_kind: KnowledgeSpanKind::Text,
                    range_start: passage.byte_start as i64,
                    range_end: passage.byte_end as i64,
                    line_start: Some(passage.start_line as i32),
                    line_end: Some(passage.end_line as i32),
                    section_path: passage.marker.clone(),
                    content_sha256: sha256_hex(passage.text.as_bytes()),
                    parser_version: parser_version.clone(),
                    extraction_receipt_event_id: Some(receipt_event_id.clone()),
                    index_run_id: index_run_id.map(|s| s.to_string()),
                    display_snippet: Some(truncate_snippet(&passage.text)),
                })
                .await?;
            let entity = self
                .db
                .upsert_knowledge_entity(NewKnowledgeEntity {
                    workspace_id: workspace_id.to_string(),
                    entity_kind: KnowledgeEntityKind::Concept,
                    entity_key: passage.entity_key(relative_path),
                    display_name: truncate_snippet(&passage.text),
                    detection_provenance: json!({
                        "extractor": "knowledge_code_index",
                        "extractor_version": CODE_EXTRACTOR_VERSION,
                        "passage_kind": passage.kind.as_str(),
                        "marker": passage.marker,
                    }),
                    primary_source_id: Some(source_id.to_string()),
                    detected_in_run: index_run_id.map(|s| s.to_string()),
                    evidence_span_ids: vec![span.span_id.clone()],
                })
                .await?;
            // documents edge: passage -> file (or enclosing symbol).
            let target = symbols
                .iter()
                .find(|s| s.start_line == passage.end_line + 1)
                .and_then(|s| symbol_index.get(&s.symbol_path))
                .map(|r| r.entity_id.clone())
                .unwrap_or_else(|| file_entity.entity_id.clone());
            // Only doc comments produce `documents` edges; TODO/SAFETY markers
            // remain searchable concept entities (claims) without a documents
            // edge, matching their non-API nature.
            if passage.kind == DocPassageKind::DocComment {
                self.db
                    .upsert_knowledge_edge(NewKnowledgeEdge {
                        workspace_id: workspace_id.to_string(),
                        edge_type: KnowledgeEdgeType::Documents,
                        source_entity_id: entity.entity_id.clone(),
                        target_entity_id: target,
                        extractor_version: CODE_EXTRACTOR_VERSION.to_string(),
                        confidence: 0.8,
                        detected_in_run: index_run_id.map(|s| s.to_string()),
                        evidence_span_ids: vec![span.span_id.clone()],
                    })
                    .await?;
            }
            doc_passages_indexed += 1;
        }

        // Relationship edges (calls/imports/implements) resolved against the
        // workspace symbol entities.
        let mut edges_indexed = symbols.len(); // contains edges
        edges_indexed += self
            .write_relationships(
                ctx,
                workspace_id,
                source_id,
                &parser_version,
                &receipt_event_id,
                index_run_id,
                &relationships,
                &symbol_index,
                &file_entity.entity_id,
            )
            .await?;

        // Test mappings -> validates edges (test symbol -> tested symbol).
        edges_indexed += self
            .write_test_mappings(
                workspace_id,
                source_id,
                &parser_version,
                &receipt_event_id,
                index_run_id,
                &test_mappings,
                &symbol_index,
                &test_symbol_index,
            )
            .await?;

        // Per-source rollup + per-code-file index state.
        self.db
            .record_knowledge_source_index_receipt(
                source_id,
                KnowledgeParserStatus::Parsed,
                KnowledgeExtractionStatus::Extracted,
                &receipt_event_id,
            )
            .await?;
        self.db
            .upsert_knowledge_code_file(UpsertKnowledgeCodeFile {
                workspace_id: workspace_id.to_string(),
                source_id: source_id.to_string(),
                file_entity_id: Some(file_entity.entity_id.clone()),
                language: code_language_to_storage(language),
                indexed_content_hash: content_hash,
                parser_version,
                parse_status,
                symbols_indexed: symbols.len() as i32,
                edges_indexed: edges_indexed as i32,
                failure_detail: if perf_sample.within_budget {
                    None
                } else {
                    Some(json!({
                        "kind": "code_index_perf_budget_exceeded",
                        "perf_budget": perf_sample_json(&perf_sample, &perf_budget),
                    }))
                },
                last_indexed_in_run: index_run_id.map(|s| s.to_string()),
                last_index_receipt_event_id: Some(receipt_event_id.clone()),
            })
            .await?;

        Ok(CodeFileIndexOutcome {
            source_id: source_id.to_string(),
            relative_path: relative_path.to_string(),
            language: Some(language),
            parse_status,
            symbols_indexed: symbols.len(),
            edges_indexed,
            doc_passages_indexed,
            config_facts_indexed: 0,
            failed: false,
            failure_reason: None,
            receipt_event_id,
        })
    }

    /// Write one symbol's span + entity. Returns the resolved ids.
    #[allow(clippy::too_many_arguments)]
    async fn write_symbol(
        &self,
        _ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        language: CodeLanguage,
        parser_version: &str,
        receipt_event_id: &str,
        index_run_id: Option<&str>,
        symbol: &ExtractedSymbol,
    ) -> CodeIndexResult<ResolvedSymbol> {
        let snippet = "symbol definition";
        let span = self
            .db
            .create_knowledge_span(NewKnowledgeSpan {
                source_id: source_id.to_string(),
                span_kind: KnowledgeSpanKind::Ast,
                range_start: symbol.start_byte as i64,
                range_end: symbol.end_byte as i64,
                line_start: Some(symbol.start_line as i32),
                line_end: Some(symbol.end_line as i32),
                section_path: Some(symbol.symbol_path.clone()),
                content_sha256: sha256_hex(
                    format!(
                        "{}|{}|{}",
                        symbol.node_kind, symbol.symbol_path, symbol.start_byte
                    )
                    .as_bytes(),
                ),
                parser_version: parser_version.to_string(),
                extraction_receipt_event_id: Some(receipt_event_id.to_string()),
                index_run_id: index_run_id.map(|s| s.to_string()),
                display_snippet: Some(snippet.to_string()),
            })
            .await?;

        let entity = self
            .db
            .upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace_id.to_string(),
                entity_kind: KnowledgeEntityKind::Symbol,
                entity_key: symbol.entity_key(language, relative_path),
                display_name: symbol.name.clone(),
                detection_provenance: json!({
                    "extractor": "knowledge_code_index",
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                    "language": language.as_str(),
                    "symbol_kind": symbol.kind.as_str(),
                    "node_kind": symbol.node_kind,
                    "symbol_path": symbol.symbol_path,
                }),
                primary_source_id: Some(source_id.to_string()),
                detected_in_run: index_run_id.map(|s| s.to_string()),
                evidence_span_ids: vec![span.span_id.clone()],
            })
            .await?;
        self.db
            .replace_knowledge_entity_spans_for_source_kind(
                &entity.entity_id,
                source_id,
                KnowledgeSpanKind::Ast,
                std::slice::from_ref(&span.span_id),
                index_run_id,
            )
            .await?;

        Ok(ResolvedSymbol {
            entity_id: entity.entity_id,
            span_id: span.span_id,
            symbol_kind: symbol.kind,
        })
    }

    /// Resolve and write call/import/implements edges. Returns count written.
    #[allow(clippy::too_many_arguments)]
    async fn write_relationships(
        &self,
        _ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        parser_version: &str,
        receipt_event_id: &str,
        index_run_id: Option<&str>,
        relationships: &[super::relationships::RelationshipCandidate],
        symbol_index: &HashMap<String, ResolvedSymbol>,
        file_entity_id: &str,
    ) -> CodeIndexResult<usize> {
        let mut written = 0usize;
        for rel in relationships {
            // Resolve the source endpoint.
            let source_entity_id = match &rel.source_symbol_path {
                Some(path) => match symbol_index.get(path) {
                    Some(r) => r.entity_id.clone(),
                    None => file_entity_id.to_string(),
                },
                None => file_entity_id.to_string(),
            };

            // Resolve the target endpoint. Calls/implements resolve against a
            // symbol entity by simple name within this file's symbol set;
            // imports resolve to a `concept` module entity (created on demand).
            let (target_entity_id, edge_type, confidence) = match rel.kind {
                RelationshipKind::Calls => {
                    match resolve_symbol_by_name(symbol_index, &rel.target_name) {
                        Some(id) => (
                            id,
                            KnowledgeEdgeType::References,
                            rel.kind.default_confidence(),
                        ),
                        // Unresolved call target: skip (no false edge).
                        None => continue,
                    }
                }
                RelationshipKind::Implements => {
                    match resolve_symbol_by_name(symbol_index, &rel.target_name) {
                        Some(id) => (
                            id,
                            KnowledgeEdgeType::Implements,
                            rel.kind.default_confidence(),
                        ),
                        None => continue,
                    }
                }
                RelationshipKind::Imports => {
                    // Module entity (concept) keyed by the import path.
                    let module = self
                        .db
                        .upsert_knowledge_entity(NewKnowledgeEntity {
                            workspace_id: workspace_id.to_string(),
                            entity_kind: KnowledgeEntityKind::Concept,
                            entity_key: format!("module:{}", rel.target_name),
                            display_name: rel.target_name.clone(),
                            detection_provenance: json!({
                                "extractor": "knowledge_code_index",
                                "extractor_version": CODE_EXTRACTOR_VERSION,
                                "kind": "import_module",
                            }),
                            primary_source_id: None,
                            detected_in_run: index_run_id.map(|s| s.to_string()),
                            evidence_span_ids: Vec::new(),
                        })
                        .await?;
                    (
                        module.entity_id,
                        KnowledgeEdgeType::DependsOn,
                        rel.kind.default_confidence(),
                    )
                }
            };

            if source_entity_id == target_entity_id {
                // A symbol referencing itself recursively is not a useful edge.
                continue;
            }

            // Evidence span for the relationship (the call/import/impl site).
            let span = self
                .db
                .create_knowledge_span(NewKnowledgeSpan {
                    source_id: source_id.to_string(),
                    span_kind: KnowledgeSpanKind::Ast,
                    range_start: rel.start_byte as i64,
                    range_end: rel.end_byte as i64,
                    line_start: Some(rel.start_line as i32),
                    line_end: Some(rel.end_line as i32),
                    section_path: Some(format!("rel:{}", rel.kind.as_str())),
                    content_sha256: sha256_hex(
                        format!(
                            "{}|{}|{}",
                            rel.kind.as_str(),
                            rel.target_name,
                            rel.start_byte
                        )
                        .as_bytes(),
                    ),
                    parser_version: parser_version.to_string(),
                    extraction_receipt_event_id: Some(receipt_event_id.to_string()),
                    index_run_id: index_run_id.map(|s| s.to_string()),
                    display_snippet: Some(format!("{} {}", rel.kind.as_str(), rel.target_name)),
                })
                .await?;

            self.db
                .upsert_knowledge_edge(NewKnowledgeEdge {
                    workspace_id: workspace_id.to_string(),
                    edge_type,
                    source_entity_id,
                    target_entity_id,
                    extractor_version: CODE_EXTRACTOR_VERSION.to_string(),
                    confidence,
                    detected_in_run: index_run_id.map(|s| s.to_string()),
                    evidence_span_ids: vec![span.span_id],
                })
                .await?;
            written += 1;
        }
        Ok(written)
    }

    /// Resolve and write `validates` edges from test mappings (MT-102).
    #[allow(clippy::too_many_arguments)]
    async fn write_test_mappings(
        &self,
        workspace_id: &str,
        source_id: &str,
        parser_version: &str,
        receipt_event_id: &str,
        index_run_id: Option<&str>,
        mappings: &[TestMapping],
        symbol_index: &HashMap<String, ResolvedSymbol>,
        test_symbol_index: &HashMap<SymbolIdentityKey, ResolvedSymbol>,
    ) -> CodeIndexResult<usize> {
        let mut written = 0usize;
        for mapping in mappings {
            let test_key = test_mapping_identity_key(mapping);
            let Some(test) = test_symbol_index
                .get(&test_key)
                .or_else(|| symbol_index.get(&mapping.test_symbol_path))
            else {
                continue;
            };
            let test_label = test_mapping_label(mapping);
            for name in &mapping.referenced_names {
                let Some(target_id) = resolve_symbol_by_name(symbol_index, name) else {
                    continue;
                };
                if target_id == test.entity_id {
                    continue;
                }
                let span = self
                    .db
                    .create_knowledge_span(NewKnowledgeSpan {
                        source_id: source_id.to_string(),
                        span_kind: KnowledgeSpanKind::Ast,
                        range_start: mapping.start_byte as i64,
                        range_end: mapping.end_byte as i64,
                        line_start: Some(mapping.start_line as i32),
                        line_end: Some(mapping.end_line as i32),
                        section_path: Some(format!("test:{test_label}")),
                        content_sha256: sha256_hex(
                            format!(
                                "validates|{test_label}|{}|{}|{name}",
                                mapping.start_byte, mapping.end_byte
                            )
                            .as_bytes(),
                        ),
                        parser_version: parser_version.to_string(),
                        extraction_receipt_event_id: Some(receipt_event_id.to_string()),
                        index_run_id: index_run_id.map(|s| s.to_string()),
                        display_snippet: Some(format!("test {test_label} -> {name}")),
                    })
                    .await?;
                self.db
                    .upsert_knowledge_edge(NewKnowledgeEdge {
                        workspace_id: workspace_id.to_string(),
                        edge_type: KnowledgeEdgeType::Validates,
                        source_entity_id: test.entity_id.clone(),
                        target_entity_id: target_id,
                        extractor_version: CODE_EXTRACTOR_VERSION.to_string(),
                        confidence: 0.7,
                        detected_in_run: index_run_id.map(|s| s.to_string()),
                        evidence_span_ids: vec![span.span_id],
                    })
                    .await?;
                written += 1;
            }
        }
        Ok(written)
    }

    /// MT-108: record a parse/read FAILURE without aborting the run. Writes a
    /// `failed` source receipt + `failed` code-file state, ENQUEUES a durable
    /// code-index repair-queue entry (so the failed file is held for re-parse,
    /// not just flagged), and returns a failed outcome.
    ///
    /// `language` is the real [`CodeLanguage`] for a code-file failure, or
    /// `None` for a config/read failure that has no code language (the receipt
    /// then carries no misleading language tag — closing the config-receipt
    /// "says javascript for a failed .toml" accuracy bug).
    #[allow(clippy::too_many_arguments)]
    async fn record_parse_failure(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        language: Option<CodeLanguage>,
        content_hash: &str,
        parser_version: &str,
        index_run_id: Option<&str>,
        reason: &str,
        reason_class: KnowledgeCodeRepairReason,
    ) -> CodeIndexResult<CodeFileIndexOutcome> {
        let receipt_event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::KnowledgeValidationRecorded,
                "knowledge_code_index_file",
                source_id,
                json!({
                    "kind": "code_file_parse_failed",
                    "workspace_id": workspace_id,
                    "source_id": source_id,
                    "relative_path": relative_path,
                    "language": language.map(|l| l.as_str()),
                    "parser_version": parser_version,
                    "reason": reason,
                    "reason_class": reason_class.as_str(),
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                }),
            )
            .await?;

        self.db
            .record_knowledge_source_index_receipt(
                source_id,
                KnowledgeParserStatus::Failed,
                KnowledgeExtractionStatus::Failed,
                &receipt_event_id,
            )
            .await?;

        // The code-file state row needs a non-null language column; a failure
        // with no code language (config/read) is tagged with the storage
        // neutral default but the FAILURE RECEIPT + repair entry carry the real
        // cause, so no consumer is misled about a config/binary file's language.
        let storage_language = language
            .map(code_language_to_storage)
            .unwrap_or(KnowledgeCodeLanguage::Javascript);
        self.db
            .upsert_knowledge_code_file(UpsertKnowledgeCodeFile {
                workspace_id: workspace_id.to_string(),
                source_id: source_id.to_string(),
                file_entity_id: None,
                language: storage_language,
                indexed_content_hash: content_hash.to_string(),
                parser_version: parser_version.to_string(),
                parse_status: KnowledgeCodeParseStatus::Failed,
                symbols_indexed: 0,
                edges_indexed: 0,
                failure_detail: Some(json!({
                    "reason": reason,
                    "reason_class": reason_class.as_str(),
                })),
                last_indexed_in_run: index_run_id.map(|s| s.to_string()),
                last_index_receipt_event_id: Some(receipt_event_id.clone()),
            })
            .await?;

        // The durable repair surface: a re-failing file refreshes its open entry
        // (one per source); a previously dead-lettered file is reopened. This is
        // what holds the file for re-processing after the cause is fixed.
        self.db
            .enqueue_knowledge_code_repair(NewKnowledgeCodeRepairEntry {
                workspace_id: workspace_id.to_string(),
                source_id: source_id.to_string(),
                relative_path: relative_path.to_string(),
                reason_class,
                reason_detail: json!({
                    "reason": reason,
                    "language": language.map(|l| l.as_str()),
                    "parser_version": parser_version,
                }),
                enqueue_event_id: Some(receipt_event_id.clone()),
            })
            .await?;

        Ok(CodeFileIndexOutcome {
            source_id: source_id.to_string(),
            relative_path: relative_path.to_string(),
            language,
            parse_status: KnowledgeCodeParseStatus::Failed,
            symbols_indexed: 0,
            edges_indexed: 0,
            doc_passages_indexed: 0,
            config_facts_indexed: 0,
            failed: true,
            failure_reason: Some(reason.to_string()),
            receipt_event_id,
        })
    }

    /// The config-file (MT-101) indexing path: config keys / schema props /
    /// package scripts become `schema`/`command`/`concept` entities anchored to
    /// `byte`-kind spans.
    #[allow(clippy::too_many_arguments)]
    async fn index_config_file(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        text: &str,
        format: super::config_schema::ConfigFormat,
        index_run_id: Option<&str>,
    ) -> CodeIndexResult<CodeFileIndexOutcome> {
        let parser_version = format!("config_extractor_v1/{}", config_format_str(format));
        let facts = match extract_config_facts(format, relative_path, text) {
            Ok(facts) => facts,
            Err(reason) => {
                // A config file has NO CodeLanguage; pass `None` so the failure
                // receipt does not claim a (wrong) code language for a .toml/.json
                // /.yaml. The repair entry is classed CONFIG_PARSE_ERROR.
                return self
                    .record_parse_failure(
                        ctx,
                        workspace_id,
                        source_id,
                        relative_path,
                        None,
                        &sha256_hex(text.as_bytes()),
                        &parser_version,
                        index_run_id,
                        &format!("config parse failed: {reason}"),
                        KnowledgeCodeRepairReason::ConfigParseError,
                    )
                    .await;
            }
        };

        let receipt_event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::KnowledgeValidationRecorded,
                "knowledge_code_index_config",
                source_id,
                json!({
                    "kind": "config_file_indexed",
                    "workspace_id": workspace_id,
                    "source_id": source_id,
                    "relative_path": relative_path,
                    "format": config_format_str(format),
                    "facts": facts.len(),
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                }),
            )
            .await?;

        let file_entity = self
            .db
            .upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace_id.to_string(),
                entity_kind: KnowledgeEntityKind::File,
                entity_key: format!("file:{relative_path}"),
                display_name: relative_path.to_string(),
                detection_provenance: json!({
                    "extractor": "knowledge_code_index",
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                    "format": config_format_str(format),
                }),
                primary_source_id: Some(source_id.to_string()),
                detected_in_run: index_run_id.map(|s| s.to_string()),
                evidence_span_ids: Vec::new(),
            })
            .await?;

        let mut count = 0usize;
        for fact in &facts {
            let span = self
                .db
                .create_knowledge_span(NewKnowledgeSpan {
                    source_id: source_id.to_string(),
                    span_kind: KnowledgeSpanKind::Byte,
                    range_start: fact.byte_start as i64,
                    range_end: fact.byte_end as i64,
                    line_start: Some(fact.line as i32),
                    line_end: Some(fact.line as i32),
                    section_path: Some(fact.key_path.clone()),
                    content_sha256: sha256_hex(fact.key_path.as_bytes()),
                    parser_version: parser_version.clone(),
                    extraction_receipt_event_id: Some(receipt_event_id.clone()),
                    index_run_id: index_run_id.map(|s| s.to_string()),
                    display_snippet: Some(fact.key_path.clone()),
                })
                .await?;
            let entity_kind = match fact.fact_kind {
                ConfigFactKind::SchemaProperty
                | ConfigFactKind::MigrationTable
                | ConfigFactKind::MigrationIndex
                | ConfigFactKind::MigrationFunction
                | ConfigFactKind::MigrationTrigger => KnowledgeEntityKind::Schema,
                ConfigFactKind::PackageScript => KnowledgeEntityKind::Command,
                ConfigFactKind::ConfigKey | ConfigFactKind::TomlTable => {
                    KnowledgeEntityKind::Concept
                }
            };
            let entity = self
                .db
                .upsert_knowledge_entity(NewKnowledgeEntity {
                    workspace_id: workspace_id.to_string(),
                    entity_kind,
                    entity_key: fact.entity_key(relative_path),
                    display_name: fact.key_path.clone(),
                    detection_provenance: json!({
                        "extractor": "knowledge_code_index",
                        "extractor_version": CODE_EXTRACTOR_VERSION,
                        "config_fact_kind": fact.fact_kind.as_str(),
                    }),
                    primary_source_id: Some(source_id.to_string()),
                    detected_in_run: index_run_id.map(|s| s.to_string()),
                    evidence_span_ids: vec![span.span_id.clone()],
                })
                .await?;
            self.db
                .upsert_knowledge_edge(NewKnowledgeEdge {
                    workspace_id: workspace_id.to_string(),
                    edge_type: KnowledgeEdgeType::Contains,
                    source_entity_id: file_entity.entity_id.clone(),
                    target_entity_id: entity.entity_id.clone(),
                    extractor_version: CODE_EXTRACTOR_VERSION.to_string(),
                    confidence: 1.0,
                    detected_in_run: index_run_id.map(|s| s.to_string()),
                    evidence_span_ids: vec![span.span_id.clone()],
                })
                .await?;
            count += 1;
        }

        self.db
            .record_knowledge_source_index_receipt(
                source_id,
                KnowledgeParserStatus::Parsed,
                KnowledgeExtractionStatus::Extracted,
                &receipt_event_id,
            )
            .await?;
        // MT-101 hardening: emit the per-file index-state row for config files
        // too (language 'config'). Without it, staleness (MT-107) and the monaco
        // lens are blind to config sources. `edges_indexed` == contains edges
        // (one per fact); symbols_indexed stays 0 (config keys are entities, not
        // tree-sitter symbols).
        self.db
            .upsert_knowledge_code_file(UpsertKnowledgeCodeFile {
                workspace_id: workspace_id.to_string(),
                source_id: source_id.to_string(),
                file_entity_id: Some(file_entity.entity_id.clone()),
                language: KnowledgeCodeLanguage::Config,
                indexed_content_hash: sha256_hex(text.as_bytes()),
                parser_version: parser_version.clone(),
                parse_status: KnowledgeCodeParseStatus::Parsed,
                symbols_indexed: 0,
                edges_indexed: count as i32,
                failure_detail: None,
                last_indexed_in_run: index_run_id.map(|s| s.to_string()),
                last_index_receipt_event_id: Some(receipt_event_id.clone()),
            })
            .await?;

        Ok(CodeFileIndexOutcome {
            source_id: source_id.to_string(),
            relative_path: relative_path.to_string(),
            language: None,
            parse_status: KnowledgeCodeParseStatus::Parsed,
            symbols_indexed: 0,
            edges_indexed: count,
            doc_passages_indexed: 0,
            config_facts_indexed: count,
            failed: false,
            failure_reason: None,
            receipt_event_id,
        })
    }

    /// MT-108: record a READ failure (binary / non-UTF8 / unreadable file)
    /// without aborting a directory run. `read_and_index` calls this when
    /// `std::fs::read` rejects a file (OS error such as a permission denial) or
    /// the bytes are not valid UTF-8 (a binary file that happens to carry a code
    /// extension). Such a file would otherwise abort the whole pass; instead it
    /// is recorded with `parse_status = failed`, a typed receipt, and a durable
    /// READ_ERROR repair-queue entry, and the run continues.
    ///
    /// No code language is asserted: a binary/unreadable file's true language is
    /// unknown, so the failure receipt carries `language: null` and the real
    /// cause, rather than guessing javascript.
    pub async fn record_read_failure(
        &self,
        ctx: &CodeIndexContext,
        workspace_id: &str,
        source_id: &str,
        relative_path: &str,
        content_hash: &str,
        reason: &str,
    ) -> CodeIndexResult<CodeFileIndexOutcome> {
        ctx.validate()?;
        self.record_parse_failure(
            ctx,
            workspace_id,
            source_id,
            relative_path,
            None,
            content_hash,
            "read_failed",
            None,
            reason,
            KnowledgeCodeRepairReason::ReadError,
        )
        .await
    }

    /// Register a code/config source row for content (mirrors the ingestion
    /// engine's source upsert) so the fixtures + nav tests have a `source_id`
    /// to index against without running the full ingestion pass. The source is
    /// a `file`-kind KnowledgeSource under the given root.
    pub async fn register_code_source(
        &self,
        workspace_id: &str,
        root_id: Option<&str>,
        relative_path: &str,
        text: &str,
    ) -> CodeIndexResult<String> {
        let source = self
            .db
            .upsert_knowledge_source(NewKnowledgeSource {
                workspace_id: workspace_id.to_string(),
                root_id: root_id.map(|s| s.to_string()),
                source_kind: KnowledgeSourceKind::File,
                relative_path: Some(relative_path.to_string()),
                asset_id: None,
                loom_block_id: None,
                document_id: None,
                content_hash: sha256_hex(text.as_bytes()),
                size_bytes: Some(text.len() as i64),
                provenance: json!({
                    "discovered_by": "knowledge_code_index_test_register",
                }),
                permission_scope: KnowledgePermissionScope::Workspace,
                redaction_state: KnowledgeRedactionState::None,
                source_modified_at: None,
            })
            .await?;
        Ok(source.source_id)
    }
}

async fn append_batch_events(
    tx: &mut Transaction<'_, Postgres>,
    events: &[NewKernelEvent],
) -> CodeIndexResult<HashMap<String, String>> {
    let mut keys = Vec::with_capacity(events.len());
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO kernel_event_ledger (event_id,event_version,kernel_task_run_id,session_run_id,aggregate_type,aggregate_id,idempotency_key,event_type,actor_kind,actor_id,causation_id,correlation_id,payload_hash,source_component,payload,created_at) ",
    );
    query.push_values(events, |mut b, event| {
        event
            .validate()
            .expect("validated kernel event builder output");
        let kernel = KernelEvent::from_new(event.clone());
        keys.push(event.idempotency_key.clone());
        b.push_bind(kernel.event_id)
            .push_bind(event.event_version.clone())
            .push_bind(event.kernel_task_run_id.clone())
            .push_bind(event.session_run_id.clone())
            .push_bind(event.aggregate_type.clone())
            .push_bind(event.aggregate_id.clone())
            .push_bind(event.idempotency_key.clone())
            .push_bind(event.event_type.as_str())
            .push_bind(event.actor.actor_kind())
            .push_bind(event.actor.actor_id())
            .push_bind(event.causation_id.clone())
            .push_bind(event.correlation_id.clone())
            .push_bind(event.payload_hash.clone())
            .push_bind(event.source_component.clone())
            .push_bind(sqlx::types::Json(event.payload.clone()))
            .push_bind(chrono::Utc::now());
    });
    query.push(" ON CONFLICT (idempotency_key) DO NOTHING");
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    let rows = sqlx::query(
        "SELECT idempotency_key,event_id,event_version,kernel_task_run_id,session_run_id,aggregate_type,aggregate_id,event_type,actor_kind,actor_id,causation_id,correlation_id,payload_hash,source_component FROM kernel_event_ledger WHERE idempotency_key = ANY($1)",
    )
    .bind(&keys)
    .fetch_all(&mut **tx)
    .await
    .map_err(StorageError::from)?;
    let mut result = HashMap::with_capacity(rows.len());
    let expected = events
        .iter()
        .map(|event| (event.idempotency_key.as_str(), event))
        .collect::<HashMap<_, _>>();
    for row in rows {
        let key: String = row.get("idempotency_key");
        let event = expected
            .get(key.as_str())
            .ok_or(StorageError::NotFound("kernel event idempotency key"))?;
        let matches = row.get::<String, _>("event_version") == event.event_version
            && row.get::<String, _>("kernel_task_run_id") == event.kernel_task_run_id
            && row.get::<String, _>("session_run_id") == event.session_run_id
            && row.get::<String, _>("aggregate_type") == event.aggregate_type
            && row.get::<String, _>("aggregate_id") == event.aggregate_id
            && row.get::<String, _>("event_type") == event.event_type.as_str()
            && row.get::<String, _>("actor_kind") == event.actor.actor_kind()
            && row.get::<String, _>("actor_id") == event.actor.actor_id()
            && row.get::<Option<String>, _>("causation_id") == event.causation_id
            && row.get::<Option<String>, _>("correlation_id") == event.correlation_id
            && row.get::<String, _>("payload_hash") == event.payload_hash
            && row.get::<String, _>("source_component") == event.source_component;
        if !matches {
            return Err(CodeIndexError::Storage(StorageError::Conflict(
                "kernel event idempotency conflict",
            )));
        }
        result.insert(key, row.get("event_id"));
    }
    Ok(result)
}

async fn insert_batch_spans(
    tx: &mut Transaction<'_, Postgres>,
    spans: &[BatchSpan],
) -> CodeIndexResult<()> {
    if spans.is_empty() {
        return Ok(());
    }
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_spans (span_id,source_id,span_kind,range_start,range_end,line_start,line_end,section_path,content_sha256,parser_version,extraction_receipt_event_id,index_run_id,display_snippet) ",
    );
    query.push_values(spans, |mut b, span| {
        b.push_bind(span.span_id.clone())
            .push_bind(span.source_id.clone())
            .push_bind(KnowledgeSpanKind::Ast.as_str())
            .push_bind(span.range_start)
            .push_bind(span.range_end)
            .push_bind(span.line_start)
            .push_bind(span.line_end)
            .push_bind(Some(span.section_path.clone()))
            .push_bind(span.content_sha256.clone())
            .push_bind(span.parser_version.clone())
            .push_bind(Some(span.receipt_event_id.clone()))
            .push_bind(Some(span.index_run_id.clone()))
            .push_bind(Some("symbol definition".to_string()));
    });
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    Ok(())
}

type BatchEntityRow = (String, KnowledgeEntityKind, String, String, Value, Option<String>);

async fn insert_batch_entities(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    index_run_id: &str,
    rows: &[BatchEntityRow],
) -> CodeIndexResult<HashMap<(String, String), String>> {
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_entities (entity_id,workspace_id,entity_kind,entity_key,display_name,detection_provenance,primary_source_id,first_detected_in_run,last_detected_in_run) ",
    );
    query.push_values(rows, |mut b, row| {
        b.push_bind(batch_id("KEN"))
            .push_bind(workspace_id)
            .push_bind(row.1.as_str())
            .push_bind(row.2.clone())
            .push_bind(row.3.clone())
            .push_bind(row.4.clone())
            .push_bind(row.5.clone())
            .push_bind(Some(index_run_id))
            .push_bind(Some(index_run_id));
    });
    query.push(" ON CONFLICT (workspace_id,entity_kind,entity_key) DO UPDATE SET display_name=EXCLUDED.display_name,detection_provenance=EXCLUDED.detection_provenance,primary_source_id=COALESCE(EXCLUDED.primary_source_id,knowledge_entities.primary_source_id),last_detected_in_run=COALESCE(EXCLUDED.last_detected_in_run,knowledge_entities.last_detected_in_run),lifecycle_state='active',updated_at=NOW() RETURNING entity_id,entity_kind,entity_key");
    let result = query.build().fetch_all(&mut **tx).await.map_err(StorageError::from)?;
    Ok(result
        .into_iter()
        .map(|row| {
            (
                (row.get::<String, _>("entity_kind"), row.get::<String, _>("entity_key")),
                row.get("entity_id"),
            )
        })
        .collect())
}

async fn insert_batch_entity_spans(
    tx: &mut Transaction<'_, Postgres>,
    index_run_id: &str,
    rows: &[(String, String)],
) -> CodeIndexResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_entity_spans (entity_id,span_id,detected_in_run) ",
    );
    query.push_values(rows, |mut b, row| {
        b.push_bind(row.0.clone())
            .push_bind(row.1.clone())
            .push_bind(Some(index_run_id));
    });
    query.push(" ON CONFLICT (entity_id,span_id) DO UPDATE SET detected_in_run=EXCLUDED.detected_in_run");
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    Ok(())
}

async fn insert_batch_edges(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    index_run_id: &str,
    rows: &[(String, String, String, String)],
) -> CodeIndexResult<HashMap<String, String>> {
    if rows.is_empty() {
        return Ok(HashMap::new());
    }
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_edges (edge_id,workspace_id,relationship_id,edge_type,source_entity_id,target_entity_id,extractor_version,confidence,created_in_run,last_seen_in_run) ",
    );
    query.push_values(rows, |mut b, row| {
        b.push_bind(batch_id("KED"))
            .push_bind(workspace_id)
            .push_bind(row.0.clone())
            .push_bind(KnowledgeEdgeType::Contains.as_str())
            .push_bind(row.1.clone())
            .push_bind(row.2.clone())
            .push_bind(CODE_EXTRACTOR_VERSION)
            .push_bind(1.0_f64)
            .push_bind(Some(index_run_id))
            .push_bind(Some(index_run_id));
    });
    query.push(" ON CONFLICT (workspace_id,relationship_id) DO UPDATE SET confidence=EXCLUDED.confidence,extractor_version=EXCLUDED.extractor_version,last_seen_in_run=COALESCE(EXCLUDED.last_seen_in_run,knowledge_edges.last_seen_in_run),updated_at=NOW() RETURNING edge_id,relationship_id");
    let result = query.build().fetch_all(&mut **tx).await.map_err(StorageError::from)?;
    Ok(result
        .into_iter()
        .map(|row| (row.get("relationship_id"), row.get("edge_id")))
        .collect())
}

async fn insert_batch_edge_spans(
    tx: &mut Transaction<'_, Postgres>,
    index_run_id: &str,
    edge_ids: &HashMap<String, String>,
    rows: &[(String, String, String, String)],
) -> CodeIndexResult<()> {
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_edge_spans (edge_id,span_id,recorded_in_run) ",
    );
    let mut count = 0usize;
    query.push(" VALUES ");
    for row in rows {
        let Some(edge_id) = edge_ids.get(&row.0) else { continue };
        if count > 0 { query.push(","); }
        query.push("(").push_bind(edge_id).push(",").push_bind(&row.3).push(",").push_bind(Some(index_run_id)).push(")");
        count += 1;
    }
    if count == 0 { return Ok(()); }
    query.push(" ON CONFLICT (edge_id,span_id) DO UPDATE SET recorded_in_run=EXCLUDED.recorded_in_run");
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    Ok(())
}

async fn insert_batch_code_files(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    index_run_id: &str,
    parsed: &[BatchParsedCodeFile],
    receipt_ids: &HashMap<String, String>,
    entity_ids: &HashMap<(String, String), String>,
) -> CodeIndexResult<()> {
    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO knowledge_code_files (code_file_id,workspace_id,source_id,file_entity_id,language,indexed_content_hash,parser_version,parse_status,stale,symbols_indexed,edges_indexed,failure_detail,last_indexed_in_run,last_index_receipt_event_id) ",
    );
    query.push_values(parsed, |mut b, item| {
        let key = (KnowledgeEntityKind::File.as_str().to_string(), format!("file:{}", item.relative_path));
        let receipt = receipt_ids.get(&item.receipt.idempotency_key).cloned();
        b.push_bind(batch_id("KCF"))
            .push_bind(workspace_id)
            .push_bind(item.source_id.clone())
            .push_bind(entity_ids.get(&key).cloned())
            .push_bind(code_language_to_storage(item.language).as_str())
            .push_bind(item.content_hash.clone())
            .push_bind(item.parser_version.clone())
            .push_bind(KnowledgeCodeParseStatus::Parsed.as_str())
            .push_bind(false)
            .push_bind(item.symbols.len() as i32)
            .push_bind(item.symbols.len() as i32)
            .push_bind(item.perf_failure.clone())
            .push_bind(Some(index_run_id))
            .push_bind(receipt);
    });
    query.push(" ON CONFLICT (source_id) DO UPDATE SET file_entity_id=COALESCE(EXCLUDED.file_entity_id,knowledge_code_files.file_entity_id),language=EXCLUDED.language,indexed_content_hash=EXCLUDED.indexed_content_hash,parser_version=EXCLUDED.parser_version,parse_status=EXCLUDED.parse_status,stale=FALSE,symbols_indexed=EXCLUDED.symbols_indexed,edges_indexed=EXCLUDED.edges_indexed,failure_detail=EXCLUDED.failure_detail,last_indexed_in_run=COALESCE(EXCLUDED.last_indexed_in_run,knowledge_code_files.last_indexed_in_run),last_index_receipt_event_id=COALESCE(EXCLUDED.last_index_receipt_event_id,knowledge_code_files.last_index_receipt_event_id),updated_at=NOW()");
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    Ok(())
}

async fn update_batch_sources(
    tx: &mut Transaction<'_, Postgres>,
    parsed: &[BatchParsedCodeFile],
    receipt_ids: &HashMap<String, String>,
) -> CodeIndexResult<()> {
    let mut query = QueryBuilder::<Postgres>::new(
        "UPDATE knowledge_sources AS s SET parser_status=v.parser_status,extraction_status=v.extraction_status,last_index_receipt_event_id=v.receipt_event_id,updated_at=NOW() FROM (",
    );
    query.push_values(parsed, |mut b, item| {
        b.push_bind(item.source_id.clone())
            .push_bind("parsed")
            .push_bind("extracted")
            .push_bind(receipt_ids.get(&item.receipt.idempotency_key).cloned());
    });
    query.push(") AS v(source_id,parser_status,extraction_status,receipt_event_id) WHERE s.source_id=v.source_id");
    query.build().execute(&mut **tx).await.map_err(StorageError::from)?;
    Ok(())
}

/// A symbol resolved to its durable ids.
#[derive(Clone, Debug)]
struct ResolvedSymbol {
    entity_id: String,
    span_id: String,
    #[allow(dead_code)]
    symbol_kind: SymbolKind,
}

type SymbolIdentityKey = (String, Option<String>, usize, usize);

fn symbol_identity_key(symbol: &ExtractedSymbol) -> SymbolIdentityKey {
    (
        symbol.symbol_path.clone(),
        symbol.disambiguator.clone(),
        symbol.start_byte,
        symbol.end_byte,
    )
}

fn test_mapping_identity_key(mapping: &TestMapping) -> SymbolIdentityKey {
    (
        mapping.test_symbol_path.clone(),
        mapping.test_disambiguator.clone(),
        mapping.start_byte,
        mapping.end_byte,
    )
}

fn test_mapping_label(mapping: &TestMapping) -> String {
    match &mapping.test_disambiguator {
        Some(disambiguator) => format!("{}~{disambiguator}", mapping.test_symbol_path),
        None => mapping.test_symbol_path.clone(),
    }
}

/// Resolve a target symbol by SIMPLE name against this file's indexed symbols.
/// Matches the last path segment so `Foo::bar` resolves on `bar`. Returns the
/// first match (deterministic by the index insertion order; ambiguous names
/// keep the call edge at the file-scoped confidence the caller assigned).
fn resolve_symbol_by_name(
    symbol_index: &HashMap<String, ResolvedSymbol>,
    name: &str,
) -> Option<String> {
    // Exact path match first.
    if let Some(r) = symbol_index.get(name) {
        return Some(r.entity_id.clone());
    }
    // Last-segment match (sorted for determinism).
    let mut candidates: Vec<(&String, &ResolvedSymbol)> = symbol_index
        .iter()
        .filter(|(path, _)| {
            path.rsplit(['.', ':'])
                .next()
                .map(|seg| seg == name)
                .unwrap_or(false)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.cmp(b.0));
    candidates.first().map(|(_, r)| r.entity_id.clone())
}

fn code_language_to_storage(language: CodeLanguage) -> KnowledgeCodeLanguage {
    match language {
        CodeLanguage::Rust => KnowledgeCodeLanguage::Rust,
        CodeLanguage::JavaScript => KnowledgeCodeLanguage::Javascript,
        CodeLanguage::TypeScript => KnowledgeCodeLanguage::Typescript,
        CodeLanguage::Tsx => KnowledgeCodeLanguage::Tsx,
    }
}

fn config_format_str(format: super::config_schema::ConfigFormat) -> &'static str {
    use super::config_schema::ConfigFormat;
    match format {
        ConfigFormat::Json => "json",
        ConfigFormat::Yaml => "yaml",
        ConfigFormat::Toml => "toml",
        ConfigFormat::Sql => "sql",
    }
}

fn truncate_snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= 120 {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(117).collect();
        format!("{truncated}...")
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn perf_sample_json(sample: &PerfSample, budget: &CodeIndexBudget) -> Value {
    json!({
        "relative_path": &sample.relative_path,
        "line_count": sample.line_count,
        "elapsed_ms": sample.elapsed_ms,
        "allowed_ms": sample.allowed_ms,
        "within_budget": sample.within_budget,
        "budget": {
            "max_ms_per_kloc": budget.max_ms_per_kloc,
            "fixed_overhead_ms": budget.fixed_overhead_ms,
        },
    })
}

/// Read a file under a runtime anchor and index it (convenience for a
/// directory run; the anchor is machine-local runtime config, never stored).
///
/// MT-108: a file that cannot be read as UTF-8 text (a binary file that carries
/// a code extension, or an OS read error such as a permission denial) does NOT
/// abort the run. We read raw bytes first so the content hash is always
/// available for the failure receipt, then attempt a UTF-8 decode; a decode or
/// IO failure is recorded as a `failed` file (typed receipt + repair-queue
/// entry) through [`CodeIndexEngine::record_read_failure`] and the run
/// continues with the remaining files.
pub async fn read_and_index(
    engine: &CodeIndexEngine,
    ctx: &CodeIndexContext,
    workspace_id: &str,
    source_id: &str,
    relative_path: &str,
    fs_anchor: &Path,
    index_run_id: Option<&str>,
) -> CodeIndexResult<CodeFileIndexOutcome> {
    let abs = fs_anchor.join(relative_path);
    // Read raw bytes so a non-UTF8 (binary) file still yields a content hash for
    // the failure receipt instead of aborting the whole directory run.
    let bytes = match std::fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(err) => {
            let content_hash = sha256_hex(abs.display().to_string().as_bytes());
            return engine
                .record_read_failure(
                    ctx,
                    workspace_id,
                    source_id,
                    relative_path,
                    &content_hash,
                    &format!("file read failed at {}: {err}", abs.display()),
                )
                .await;
        }
    };
    let content_hash = sha256_hex(&bytes);
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            return engine
                .record_read_failure(
                    ctx,
                    workspace_id,
                    source_id,
                    relative_path,
                    &content_hash,
                    &format!(
                        "file '{relative_path}' is not valid UTF-8 (binary or wrong encoding): {err}"
                    ),
                )
                .await;
        }
    };
    engine
        .index_code_source(
            ctx,
            workspace_id,
            source_id,
            relative_path,
            &text,
            index_run_id,
        )
        .await
}
