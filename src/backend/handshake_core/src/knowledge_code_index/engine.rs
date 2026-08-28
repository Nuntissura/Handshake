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
//! No SQLite, no external LSP. The engine reuses the shared embedded
//! [`SurrealDatabase`] handle.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    derive_knowledge_relationship_id, new_knowledge_id, KnowledgeCodeLanguage,
    KnowledgeCodeParseStatus, KnowledgeCodeRepairReason, KnowledgeEdgeType, KnowledgeEntityKind,
    KnowledgeExtractionStatus, KnowledgeIndexRunCounts, KnowledgeIndexRunOutcome,
    KnowledgeParserStatus, KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind,
    KnowledgeSpanKind, KnowledgeStore, NewKnowledgeCodeRepairEntry, NewKnowledgeEdge,
    NewKnowledgeEntity, NewKnowledgeIndexRun, NewKnowledgeSource, NewKnowledgeSpan,
    UpsertKnowledgeCodeFile,
};
use crate::storage::surreal::event_ledger::{prepare_event, LedgerWrite};
use crate::storage::surreal::SurrealDatabase;
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
    db: Arc<SurrealDatabase>,
}

fn stage_error<E>(
    stage: &'static str,
    workspace_id: &str,
    source_id: Option<&str>,
    error: E,
) -> CodeIndexError
where
    E: Into<CodeIndexError>,
{
    let error = error.into();
    tracing::error!(
        target: "handshake_core::code_nav_index",
        stage,
        workspace_id,
        source_id = source_id.unwrap_or(""),
        error = %error,
        "knowledge_code_index_stage_failed"
    );
    error
}

#[derive(SurrealValue)]
struct CodeFileStateBindings {
    new_rid: RecordId,
    workspace_id: RecordId,
    source_id: RecordId,
    file_entity_id: Option<RecordId>,
    language: String,
    indexed_content_hash: String,
    parser_version: String,
    parse_status: String,
    symbols_indexed: i64,
    edges_indexed: i64,
    failure_detail: Option<Value>,
    last_indexed_in_run: Option<RecordId>,
    last_index_receipt_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct CodeRepairBindings {
    new_rid: RecordId,
    code_repair_id: String,
    workspace_id: RecordId,
    source_id: RecordId,
    relative_path: String,
    reason_class: String,
    reason_detail: Value,
    enqueue_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct CleanCodeBatchSymbolWrite {
    span: RecordId,
    span_id: String,
    symbol_entity: RecordId,
    symbol_entity_id: String,
    symbol_key: String,
    display_name: String,
    provenance: Value,
    range_start: i64,
    range_end: i64,
    line_start: i64,
    line_end: i64,
    section_path: String,
    content_sha256: String,
    edge: RecordId,
    edge_id: String,
    relationship_id: String,
}

#[derive(SurrealValue)]
struct CleanCodeBatchFileWrite {
    source: RecordId,
    event: LedgerWrite,
    file_entity: RecordId,
    file_entity_id: String,
    file_key: String,
    relative_path: String,
    language: String,
    parser_version: String,
    content_hash: String,
    file_provenance: Value,
    failure_detail: Option<Value>,
    code_file: RecordId,
    code_file_id: String,
    symbols: Vec<CleanCodeBatchSymbolWrite>,
}

#[derive(SurrealValue)]
struct CleanCodeBatchBindings {
    workspace: RecordId,
    index_run: RecordId,
    files: Vec<CleanCodeBatchFileWrite>,
}

const PERSIST_CLEAN_CODE_BATCH: &str = "BEGIN TRANSACTION; \
    RETURN { \
      FOR $file IN $files { \
        LET $existing_event = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $file.event.idempotency_key LIMIT 1)[0]; \
        IF $existing_event = NONE { CREATE $file.event.record CONTENT { event_id: $file.event.event_id, event_version: $file.event.event_version, kernel_task_run_id: $file.event.kernel_task_run_id, session_run_id: $file.event.session_run_id, aggregate_type: $file.event.aggregate_type, aggregate_id: $file.event.aggregate_id, idempotency_key: $file.event.idempotency_key, event_type: $file.event.event_type, actor_kind: $file.event.actor_kind, actor_id: $file.event.actor_id, causation_id: $file.event.causation_id, correlation_id: $file.event.correlation_id, payload_hash: $file.event.payload_hash, source_component: $file.event.source_component, payload: $file.event.payload, created_at: $file.event.created_at } RETURN NONE; }; \
        LET $receipt = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $file.event.idempotency_key LIMIT 1)[0]; \
        LET $existing_file_entity = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = 'file' AND entity_key = $file.file_key LIMIT 1)[0]; \
        IF $existing_file_entity = NONE { CREATE $file.file_entity CONTENT { entity_id: $file.file_entity_id, workspace_id: $workspace, entity_kind: 'file', entity_key: $file.file_key, display_name: $file.relative_path, detection_provenance: $file.file_provenance, primary_source_id: $file.source, first_detected_in_run: $index_run, last_detected_in_run: $index_run } RETURN NONE; } ELSE { UPDATE $existing_file_entity SET display_name = $file.relative_path, detection_provenance = $file.file_provenance, primary_source_id = $file.source, last_detected_in_run = $index_run, lifecycle_state = 'active', updated_at = time::now() RETURN NONE; }; \
        LET $file_ref = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = 'file' AND entity_key = $file.file_key LIMIT 1)[0]; \
        FOR $symbol IN $file.symbols { \
          CREATE $symbol.span CONTENT { span_id: $symbol.span_id, source_id: $file.source, span_kind: 'ast', range_start: $symbol.range_start, range_end: $symbol.range_end, line_start: $symbol.line_start, line_end: $symbol.line_end, section_path: $symbol.section_path, content_sha256: $symbol.content_sha256, parser_version: $file.parser_version, extraction_receipt_event_id: $receipt, index_run_id: $index_run, display_snippet: 'symbol definition' } RETURN NONE; \
          LET $existing_symbol = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = 'symbol' AND entity_key = $symbol.symbol_key LIMIT 1)[0]; \
          IF $existing_symbol = NONE { CREATE $symbol.symbol_entity CONTENT { entity_id: $symbol.symbol_entity_id, workspace_id: $workspace, entity_kind: 'symbol', entity_key: $symbol.symbol_key, display_name: $symbol.display_name, detection_provenance: $symbol.provenance, primary_source_id: $file.source, first_detected_in_run: $index_run, last_detected_in_run: $index_run } RETURN NONE; } ELSE { UPDATE $existing_symbol SET display_name = $symbol.display_name, detection_provenance = $symbol.provenance, primary_source_id = $file.source, last_detected_in_run = $index_run, lifecycle_state = 'active', updated_at = time::now() RETURN NONE; }; \
          LET $symbol_ref = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = 'symbol' AND entity_key = $symbol.symbol_key LIMIT 1)[0]; \
          IF (SELECT VALUE id FROM knowledge_entity_spans WHERE entity_id = $symbol_ref AND span_id = $symbol.span LIMIT 1)[0] = NONE { CREATE knowledge_entity_spans CONTENT { entity_id: $symbol_ref, span_id: $symbol.span, detected_in_run: $index_run } RETURN NONE; }; \
          LET $existing_edge = (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $symbol.relationship_id LIMIT 1)[0]; \
          IF $existing_edge = NONE { CREATE $symbol.edge CONTENT { edge_id: $symbol.edge_id, workspace_id: $workspace, relationship_id: $symbol.relationship_id, edge_type: 'contains', source_entity_id: $file_ref, target_entity_id: $symbol_ref, extractor_version: 'code_index_extractor_v1', confidence: 1.0, created_in_run: $index_run, last_seen_in_run: $index_run } RETURN NONE; } ELSE { UPDATE $existing_edge SET confidence = 1.0, extractor_version = 'code_index_extractor_v1', last_seen_in_run = $index_run, updated_at = time::now() RETURN NONE; }; \
          LET $edge_ref = (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $symbol.relationship_id LIMIT 1)[0]; \
          IF (SELECT VALUE id FROM knowledge_edge_spans WHERE edge_id = $edge_ref AND span_id = $symbol.span LIMIT 1)[0] = NONE { CREATE knowledge_edge_spans CONTENT { edge_id: $edge_ref, span_id: $symbol.span, recorded_in_run: $index_run } RETURN NONE; }; \
        }; \
        LET $existing_code_file = (SELECT VALUE id FROM knowledge_code_files WHERE source_id = $file.source LIMIT 1)[0]; \
        IF $existing_code_file = NONE { CREATE $file.code_file CONTENT { code_file_id: $file.code_file_id, workspace_id: $workspace, source_id: $file.source, file_entity_id: $file_ref, language: $file.language, indexed_content_hash: $file.content_hash, parser_version: $file.parser_version, parse_status: 'parsed', stale: false, symbols_indexed: array::len($file.symbols), edges_indexed: array::len($file.symbols), failure_detail: $file.failure_detail, last_indexed_in_run: $index_run, last_index_receipt_event_id: $receipt } RETURN NONE; } ELSE { UPDATE $existing_code_file SET file_entity_id = $file_ref, language = $file.language, indexed_content_hash = $file.content_hash, parser_version = $file.parser_version, parse_status = 'parsed', stale = false, symbols_indexed = array::len($file.symbols), edges_indexed = array::len($file.symbols), failure_detail = $file.failure_detail, last_indexed_in_run = $index_run, last_index_receipt_event_id = $receipt, updated_at = time::now() RETURN NONE; }; \
        UPDATE $file.source SET parser_status = 'parsed', extraction_status = 'extracted', last_index_receipt_event_id = $receipt, updated_at = time::now() RETURN NONE; \
      }; \
      RETURN true; \
    }; \
    COMMIT TRANSACTION;";

const UPSERT_CODE_FILE_STATE: &str = "BEGIN TRANSACTION; \
    LET $existing = (SELECT VALUE id FROM knowledge_code_files WHERE source_id = $source_id LIMIT 1)[0]; \
    LET $rid = $existing ?? $new_rid; \
    IF $existing IS NONE { \
      CREATE $new_rid CONTENT { code_file_id: record::id($new_rid), workspace_id: $workspace_id, source_id: $source_id, file_entity_id: $file_entity_id, language: $language, indexed_content_hash: $indexed_content_hash, parser_version: $parser_version, parse_status: $parse_status, stale: false, symbols_indexed: $symbols_indexed, edges_indexed: $edges_indexed, failure_detail: $failure_detail, last_indexed_in_run: $last_indexed_in_run, last_index_receipt_event_id: $last_index_receipt_event_id }; \
    } ELSE { \
      UPDATE $rid SET file_entity_id = $file_entity_id ?? file_entity_id, language = $language, indexed_content_hash = $indexed_content_hash, parser_version = $parser_version, parse_status = $parse_status, stale = false, symbols_indexed = $symbols_indexed, edges_indexed = $edges_indexed, failure_detail = $failure_detail, last_indexed_in_run = $last_indexed_in_run ?? last_indexed_in_run, last_index_receipt_event_id = $last_index_receipt_event_id ?? last_index_receipt_event_id, updated_at = time::now(); \
    }; \
    COMMIT TRANSACTION;";

const ENQUEUE_CODE_REPAIR: &str = "BEGIN TRANSACTION; \
    LET $open = (SELECT VALUE id FROM knowledge_code_repair_queue WHERE source_id = $source_id AND state IN ['queued', 'retrying'] ORDER BY updated_at DESC LIMIT 1)[0]; \
    LET $dead = (SELECT VALUE id FROM knowledge_code_repair_queue WHERE source_id = $source_id AND state = 'dead_letter' ORDER BY updated_at DESC, code_repair_id DESC LIMIT 1)[0]; \
    LET $rid = $open ?? $dead ?? $new_rid; \
    IF $open != NONE { \
      UPDATE $rid SET reason_class = $reason_class, reason_detail = $reason_detail, relative_path = $relative_path, enqueue_event_id = $enqueue_event_id ?? enqueue_event_id, updated_at = time::now(); \
    } ELSE IF $dead != NONE { \
      UPDATE $rid SET state = 'queued', attempts = 0, reason_class = $reason_class, reason_detail = $reason_detail, relative_path = $relative_path, enqueue_event_id = $enqueue_event_id ?? enqueue_event_id, resolved_receipt_event_id = NONE, updated_at = time::now(); \
    } ELSE { \
      CREATE $rid CONTENT { code_repair_id: $code_repair_id, workspace_id: $workspace_id, source_id: $source_id, relative_path: $relative_path, reason_class: $reason_class, reason_detail: $reason_detail, enqueue_event_id: $enqueue_event_id }; \
    }; \
    COMMIT TRANSACTION;";

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

impl CodeIndexEngine {
    pub fn new(db: Arc<SurrealDatabase>) -> Self {
        Self { db }
    }

    pub fn from_database(db: Arc<SurrealDatabase>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &SurrealDatabase {
        &self.db
    }

    async fn persist_code_file_state(
        &self,
        upsert: UpsertKnowledgeCodeFile,
    ) -> CodeIndexResult<()> {
        if upsert.indexed_content_hash.len() != 64
            || !upsert
                .indexed_content_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(StorageError::Validation(
                "knowledge code file indexed_content_hash must be a lowercase sha256 hex digest",
            )
            .into());
        }
        if upsert.parser_version.trim().is_empty() {
            return Err(
                StorageError::Validation("knowledge code file parser_version is required").into(),
            );
        }

        let code_file_id = format!("KCF-{}", &sha256_hex(upsert.source_id.as_bytes())[..32]);
        let bindings = CodeFileStateBindings {
            new_rid: RecordId::new("knowledge_code_files", code_file_id),
            workspace_id: RecordId::new("workspaces", upsert.workspace_id),
            source_id: RecordId::new("knowledge_sources", upsert.source_id),
            file_entity_id: upsert
                .file_entity_id
                .map(|id| RecordId::new("knowledge_entities", id)),
            language: upsert.language.as_str().to_owned(),
            indexed_content_hash: upsert.indexed_content_hash,
            parser_version: upsert.parser_version,
            parse_status: upsert.parse_status.as_str().to_owned(),
            symbols_indexed: i64::from(upsert.symbols_indexed),
            edges_indexed: i64::from(upsert.edges_indexed),
            failure_detail: upsert.failure_detail,
            last_indexed_in_run: upsert
                .last_indexed_in_run
                .map(|id| RecordId::new("knowledge_index_runs", id)),
            last_index_receipt_event_id: upsert
                .last_index_receipt_event_id
                .map(|id| RecordId::new("kernel_event_ledger", id)),
        };
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(
                            UPSERT_CODE_FILE_STATE,
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?;
        Ok(())
    }

    async fn enqueue_code_repair(&self, entry: NewKnowledgeCodeRepairEntry) -> CodeIndexResult<()> {
        let identity = format!(
            "{}:{}",
            entry.source_id,
            entry.enqueue_event_id.as_deref().unwrap_or("no-event")
        );
        let code_repair_id = format!("KCRQ-{}", &sha256_hex(identity.as_bytes())[..32]);
        let bindings = CodeRepairBindings {
            new_rid: RecordId::new("knowledge_code_repair_queue", code_repair_id.clone()),
            code_repair_id,
            workspace_id: RecordId::new("workspaces", entry.workspace_id),
            source_id: RecordId::new("knowledge_sources", entry.source_id),
            relative_path: entry.relative_path,
            reason_class: entry.reason_class.as_str().to_owned(),
            reason_detail: entry.reason_detail,
            enqueue_event_id: entry
                .enqueue_event_id
                .map(|id| RecordId::new("kernel_event_ledger", id)),
        };
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(ENQUEUE_CODE_REPAIR, bindings)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?;
        Ok(())
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
        let start_event_id = self
            .db
            .append_kernel_event(start_event)
            .await
            .map_err(|error| {
                stage_error("start_run.append_start_receipt", workspace_id, None, error)
            })?
            .event_id;
        let run = self
            .db
            .start_knowledge_index_run(NewKnowledgeIndexRun {
                workspace_id: workspace_id.to_string(),
                root_id: root_id.map(str::to_string),
                scope: json!({
                    "index_kind": "code",
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                }),
                actor_kind: ctx.actor.actor_kind().to_string(),
                actor_id: ctx.actor.actor_id().to_string(),
                worktree_id: None,
                start_receipt_event_id: Some(start_event_id),
            })
            .await
            .map_err(|error| {
                stage_error("start_run.create_index_run", workspace_id, None, error)
            })?;
        Ok(run.index_run_id)
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
            .await
            .map_err(|error| stage_error("finish_run.append_terminal_receipt", "", None, error))?;
        self.db
            .finish_knowledge_index_run(
                index_run_id,
                outcome,
                Some(finish_receipt_event_id.as_str()),
            )
            .await
            .map_err(|error| stage_error("finish_run.update_index_run", "", None, error))?;
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

    /// Use one embedded-SurrealDB transaction for the narrow clean-code shape
    /// exercised by large repository indexing. Richer files still return
    /// `None` and use the established per-file writer, preserving its complete
    /// relationship, passage, test-mapping, config, and failure semantics.
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

        let mut writes = Vec::with_capacity(prepared.len());
        let mut outcomes = Vec::with_capacity(prepared.len());
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

            let budget = CodeIndexBudget::default();
            let perf = PerfSample::measure(
                &budget,
                relative_path,
                text.lines().count(),
                started.elapsed().as_secs_f64() * 1000.0,
            );
            let failure_detail = (!perf.within_budget).then(|| {
                json!({
                    "kind": "code_index_perf_budget_exceeded",
                    "perf_budget": perf_sample_json(&perf, &budget),
                })
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
                "perf_budget": perf_sample_json(&perf, &budget),
            }));
            if let Some(correlation_id) = &ctx.correlation_id {
                builder = builder.correlation_id(correlation_id.clone());
            }
            let event = builder
                .build()
                .map_err(|error| CodeIndexError::Kernel(error.to_string()))?;
            let (stored_event, event_write) = prepare_event(event).map_err(CodeIndexError::from)?;

            let file_key = format!("file:{relative_path}");
            let file_entity_id = new_knowledge_id("KEN");
            let mut symbol_writes = Vec::with_capacity(symbols.len());
            for symbol in &symbols {
                let symbol_key = symbol.entity_key(language, relative_path);
                let relationship_id = derive_knowledge_relationship_id(
                    KnowledgeEdgeType::Contains,
                    KnowledgeEntityKind::File,
                    &file_key,
                    KnowledgeEntityKind::Symbol,
                    &symbol_key,
                );
                let span_id = new_knowledge_id("KSP");
                let symbol_entity_id = new_knowledge_id("KEN");
                let edge_id = new_knowledge_id("KED");
                symbol_writes.push(CleanCodeBatchSymbolWrite {
                    span: RecordId::new("knowledge_spans", span_id.clone()),
                    span_id,
                    symbol_entity: RecordId::new("knowledge_entities", symbol_entity_id.clone()),
                    symbol_entity_id,
                    symbol_key,
                    display_name: symbol.name.clone(),
                    provenance: json!({
                        "extractor": "knowledge_code_index",
                        "extractor_version": CODE_EXTRACTOR_VERSION,
                        "language": language.as_str(),
                        "symbol_kind": symbol.kind.as_str(),
                        "node_kind": symbol.node_kind,
                        "symbol_path": symbol.symbol_path,
                    }),
                    range_start: symbol.start_byte as i64,
                    range_end: symbol.end_byte as i64,
                    line_start: i64::from(symbol.start_line),
                    line_end: i64::from(symbol.end_line),
                    section_path: symbol.symbol_path.clone(),
                    content_sha256: sha256_hex(
                        format!(
                            "{}|{}|{}",
                            symbol.node_kind, symbol.symbol_path, symbol.start_byte
                        )
                        .as_bytes(),
                    ),
                    edge: RecordId::new("knowledge_edges", edge_id.clone()),
                    edge_id,
                    relationship_id,
                });
            }
            let code_file_id = format!("KCF-{}", &sha256_hex(source_id.as_bytes())[..32]);
            writes.push(CleanCodeBatchFileWrite {
                source: RecordId::new("knowledge_sources", source_id.clone()),
                event: event_write,
                file_entity: RecordId::new("knowledge_entities", file_entity_id.clone()),
                file_entity_id,
                file_key,
                relative_path: relative_path.clone(),
                language: code_language_to_storage(language).as_str().to_owned(),
                parser_version,
                content_hash: file.content_hash.clone(),
                file_provenance: json!({
                    "extractor": "knowledge_code_index",
                    "extractor_version": CODE_EXTRACTOR_VERSION,
                    "language": language.as_str(),
                }),
                failure_detail,
                code_file: RecordId::new("knowledge_code_files", code_file_id.clone()),
                code_file_id,
                symbols: symbol_writes,
            });
            outcomes.push(CodeFileIndexOutcome {
                source_id: source_id.clone(),
                relative_path: relative_path.clone(),
                language: Some(language),
                parse_status: KnowledgeCodeParseStatus::Parsed,
                symbols_indexed: symbols.len(),
                edges_indexed: symbols.len(),
                doc_passages_indexed: 0,
                config_facts_indexed: 0,
                failed: false,
                failure_reason: None,
                receipt_event_id: stored_event.event_id,
            });
        }

        let bindings = CleanCodeBatchBindings {
            workspace: RecordId::new("workspaces", workspace_id.to_owned()),
            index_run: RecordId::new("knowledge_index_runs", index_run_id.to_owned()),
            files: writes,
        };
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values_at::<bool, _>(PERSIST_CLEAN_CODE_BATCH, bindings, 1)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?;
        Ok(Some(outcomes))
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
        let index_run_id = self.start_run(ctx, workspace_id, root_id).await?;
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
            lane: lane.clone(),
            workspace_id: workspace_id.to_string(),
            wp_id: wp_id.to_string(),
            mt_id: mt_id.to_string(),
            work_kind: QuietBackgroundWorkKind::Indexing,
            subject_id: index_run_id.clone(),
            session_id: ctx.session_run_id.clone(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
            evidence_ref: format!("knowledge-index-run://{index_run_id}"),
        };
        let indexing_lease = match swarm_state.try_acquire_indexing_lease(lease_request).await {
            Ok(Some(record)) => record,
            Ok(None) => {
                let _ = self
                    .finish_run_with_retry(
                        ctx,
                        &index_run_id,
                        KnowledgeIndexRunOutcome::Cancelled {
                            counts: KnowledgeIndexRunCounts::default(),
                        },
                    )
                    .await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing run {index_run_id} did not acquire index lease"
                )));
            }
            Err(error) => {
                let _ = self
                    .finish_run_with_retry(
                        ctx,
                        &index_run_id,
                        KnowledgeIndexRunOutcome::Cancelled {
                            counts: KnowledgeIndexRunCounts::default(),
                        },
                    )
                    .await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing lease failed: {error}"
                )));
            }
        };
        let quiet_receipt = match swarm_state
            .record_quiet_background_work(quiet_request)
            .await
        {
            Ok(record) => record,
            Err(error) => {
                let _ = swarm_state
                    .complete_indexing_lease(&indexing_lease.lease_id, &lane)
                    .await;
                let _ = self
                    .finish_run_with_retry(
                        ctx,
                        &index_run_id,
                        KnowledgeIndexRunOutcome::Cancelled {
                            counts: KnowledgeIndexRunCounts::default(),
                        },
                    )
                    .await;
                return Err(CodeIndexError::Validation(format!(
                    "quiet indexing receipt failed: {error}"
                )));
            }
        };
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
            .await
            .map_err(|error| {
                stage_error(
                    "index_code_file.append_file_receipt",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;

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
            .await
            .map_err(|error| {
                stage_error(
                    "index_code_file.upsert_file_entity",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;

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
                .await
                .map_err(|error| {
                    stage_error(
                        "index_code_file.write_symbol",
                        workspace_id,
                        Some(source_id),
                        error,
                    )
                })?;
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
                .await
                .map_err(|error| {
                    stage_error(
                        "index_code_file.upsert_contains_edge",
                        workspace_id,
                        Some(source_id),
                        error,
                    )
                })?;
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
            .await
            .map_err(|error| {
                stage_error(
                    "index_code_file.record_source_receipt",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;
        self.persist_code_file_state(UpsertKnowledgeCodeFile {
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
        .await
        .map_err(|error| {
            stage_error(
                "index_code_file.persist_code_file_state",
                workspace_id,
                Some(source_id),
                error,
            )
        })?;

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
            .await
            .map_err(|error| {
                stage_error(
                    "write_symbol.create_span",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;

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
            .await
            .map_err(|error| {
                stage_error(
                    "write_symbol.upsert_entity",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;
        self.db
            .replace_knowledge_entity_spans_for_source_kind(
                &entity.entity_id,
                source_id,
                KnowledgeSpanKind::Ast,
                std::slice::from_ref(&span.span_id),
                index_run_id,
            )
            .await
            .map_err(|error| {
                stage_error(
                    "write_symbol.replace_entity_spans",
                    workspace_id,
                    Some(source_id),
                    error,
                )
            })?;

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
        self.persist_code_file_state(UpsertKnowledgeCodeFile {
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
        self.enqueue_code_repair(NewKnowledgeCodeRepairEntry {
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
        self.persist_code_file_state(UpsertKnowledgeCodeFile {
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
