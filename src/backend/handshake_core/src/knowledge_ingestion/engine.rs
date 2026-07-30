//! Ingestion engine: orchestrates allowlist enforcement, storage-layer
//! writes, and EventLedger receipts for the SourceIngestionAndEvidence group.
//!
//! The engine writes THROUGH the existing storage layer
//! (`storage::knowledge::KnowledgeStore` on `PostgresDatabase`) for the
//! pre-existing knowledge tables, and through [`KnowledgeIngestionStore`] for
//! the ingestion-owned tables (0160-0169). Every mutation leaves an
//! EventLedger receipt carrying actor, session, and correlation ids
//! (spec 2.3.13.11 backend-navigation receipt law).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use futures_util::stream::{self, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{query, Row};
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    KnowledgeExtractionStatus, KnowledgeIndexingEligibility, KnowledgeParserStatus,
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeRootKind, KnowledgeSource,
    KnowledgeSourceKind, KnowledgeSourceRoot, KnowledgeStore, NewKnowledgeSource,
    NewKnowledgeSourceRoot,
};
use crate::storage::postgres::{append_kernel_event_with_executor, PostgresDatabase};
use crate::storage::{Database, StorageError};

use super::allowlist::{CompiledRootPolicy, RootRegistrationPolicy};
use super::backpressure::IngestionLimits;
use super::governance::{parse_governance_json, parse_governance_jsonl, GovernanceSpanLimits};
use super::hashing::{compute_content_hashes, ContentHashes};
use super::kinds::{detect_governance_sub_kind, detect_kind, spec_for, IngestionSourceKind};
use super::notes::parse_note;
use super::paths::normalize_source_relative_path;
use super::pdf::{extract_pdf_text, PDF_EXTRACTOR_ID, PDF_EXTRACTOR_VERSION};
use super::receipts::{
    ExtractionReceipt, ExtractionStatus, IngestionErrorClass, NewExtractionReceipt,
};
use super::repair::{NewRepairEntry, RepairAttemptOutcome, RepairEntry, RepairReason};
use super::secrets::{
    redact_span_with_whole_file_findings, redact_text, scan_text, SecretScanReport,
};
use super::spans::{ExtractedSpan, SpanAnchor, SpanRedaction};
use super::store::{KnowledgeIngestionStore, NewPolicyDecision, PolicyDecision, StoredSpan};
use super::transcripts::parse_transcript_artifact;
use super::{new_ingestion_id, IngestionError, IngestionResult};

/// Backend-navigation context (spec 2.3.13.11): every engine mutation must
/// carry actor id, session id, and correlation id into its receipts.
#[derive(Clone, Debug)]
pub struct IngestionContext {
    pub actor: KernelActor,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub correlation_id: Option<String>,
}

impl IngestionContext {
    pub fn validate(&self) -> IngestionResult<()> {
        if self.actor.actor_id().trim().is_empty() {
            return Err(IngestionError::Validation(
                "ingestion context requires a non-empty actor id".to_string(),
            ));
        }
        if self.kernel_task_run_id.trim().is_empty() || self.session_run_id.trim().is_empty() {
            return Err(IngestionError::Validation(
                "ingestion context requires kernel_task_run_id and session_run_id".to_string(),
            ));
        }
        Ok(())
    }
}

/// Request to register a knowledge source root (MT-081 enforcement path).
#[derive(Clone, Debug)]
pub struct RootRegistrationRequest {
    pub workspace_id: String,
    pub display_name: String,
    pub root_kind: KnowledgeRootKind,
    pub repo_relative_path: String,
    /// Per-root FILE allowlist (0131 `allowlist_policy` JSONB shape).
    pub file_allowlist_policy: serde_json::Value,
    /// Explicit operator waving-through for approval-gated registrations.
    pub operator_approved: bool,
}

/// The ingestion engine. Cheap to construct per request: both handles wrap
/// pooled connections.
#[derive(Clone)]
pub struct IngestionEngine {
    db: Arc<PostgresDatabase>,
    store: KnowledgeIngestionStore,
}

/// Pure, side-effect-free preparation record for the code-nav batch writer.
/// Files are hashed and passed through the canonical ingestion extractor before
/// any PostgreSQL transaction begins; persistence phases consume this record
/// without re-reading or reinterpreting source bytes.
pub(crate) struct PreparedCodeNavFile {
    pub relative_path: String,
    /// Original UTF-8 payload retained for the downstream code-index batch.
    /// Keeping the already-read bytes avoids a second filesystem pass and
    /// ensures indexing uses the exact content that ingestion hashed.
    pub content: Vec<u8>,
    pub size_bytes: i64,
    pub content_hash: String,
    pub kind: Option<IngestionSourceKind>,
    pub extraction: ExtractionOutput,
    pub duration_ms: i64,
}

pub(crate) fn prepare_code_nav_file(
    root_kind: KnowledgeRootKind,
    relative_path: &str,
    bytes: &[u8],
    limits: &IngestionLimits,
) -> IngestionResult<PreparedCodeNavFile> {
    let normalized = normalize_source_relative_path(relative_path)?;
    let hashes = compute_content_hashes(bytes);
    let started = Instant::now();
    let kind = detect_kind(root_kind, &normalized);
    let extraction = run_extraction(root_kind, &normalized, bytes, &hashes, limits);
    Ok(PreparedCodeNavFile {
        relative_path: normalized,
        content: bytes.to_vec(),
        size_bytes: bytes.len() as i64,
        content_hash: hashes.raw_sha256,
        kind,
        extraction,
        duration_ms: started.elapsed().as_millis() as i64,
    })
}

impl IngestionEngine {
    pub fn new(db: Arc<PostgresDatabase>, store: KnowledgeIngestionStore) -> Self {
        Self { db, store }
    }

    /// Build an engine over one PostgreSQL handle: the ingestion store reuses
    /// the same pool (no second connection pool, no reconnect).
    pub fn from_database(db: Arc<PostgresDatabase>) -> Self {
        let store = KnowledgeIngestionStore::new(db.pool().clone());
        Self { db, store }
    }

    pub fn store(&self) -> &KnowledgeIngestionStore {
        &self.store
    }

    pub fn knowledge(&self) -> &PostgresDatabase {
        &self.db
    }

    pub(crate) async fn prepare_code_nav_files(
        &self,
        root: &KnowledgeSourceRoot,
        fs_anchor: &Path,
        limits: &IngestionLimits,
    ) -> IngestionResult<(Vec<PreparedCodeNavFile>, usize)> {
        let root_dir = if root.repo_relative_path.is_empty() {
            fs_anchor.to_path_buf()
        } else {
            fs_anchor.join(&root.repo_relative_path)
        };
        let filter = compile_file_allowlist(&root.allowlist_policy)?;
        let mut walked = Vec::new();
        let mut walk_errors = Vec::new();
        walk_files(&root_dir, &root_dir, &mut walked, &mut walk_errors);
        if let Some(error) = walk_errors.first() {
            return Err(IngestionError::Io {
                path: root_dir.display().to_string(),
                detail: error.clone(),
            });
        }
        let mut prepared = Vec::new();
        let mut skipped = 0usize;
        for (relative, absolute, _) in walked {
            let normalized = normalize_source_relative_path(&relative)?;
            if !filter.allows(&normalized) {
                skipped += 1;
                continue;
            }
            let bytes = std::fs::read(&absolute).map_err(|err| IngestionError::Io {
                path: absolute.display().to_string(),
                detail: err.to_string(),
            })?;
            prepared.push(prepare_code_nav_file(root.root_kind, &normalized, &bytes, limits)?);
        }
        Ok((prepared, skipped))
    }

    /// Persist prepared code-nav extraction records in one PostgreSQL
    /// transaction. EventLedger events remain per-file (each source/receipt
    /// keeps its own immutable evidence id); only the commit boundary is
    /// shared to remove autocommit latency.
    pub(crate) async fn persist_code_nav_batch(
        &self,
        ctx: &IngestionContext,
        root: &KnowledgeSourceRoot,
        run_token: &str,
        prepared: &[PreparedCodeNavFile],
    ) -> IngestionResult<Vec<(String, String)>> {
        ctx.validate()?;
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let start_event = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            KernelEventType::ValidationRecorded,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_ingestion_run", run_token)
        .source_component("knowledge_ingestion")
        .payload(json!({"kind":"ingestion_run_started","workspace_id":root.workspace_id,"root_id":root.root_id,"run_token":run_token}))
        .build()
        .map_err(|err| IngestionError::Kernel(err.to_string()))?;
        let start_event_id = append_kernel_event_with_executor(&mut *tx, start_event)
            .await
            .map_err(IngestionError::from)?
            .event_id;
        let result: IngestionResult<Vec<(String, String)>> = async {
            let mut sources = Vec::with_capacity(prepared.len());
            for file in prepared {
            // Resolve the canonical source row before emitting the receipt. The
            // root/path unique key is the idempotency boundary for re-indexing;
            // retries must reuse the existing source_id so all child receipts,
            // spans, and code-index rows continue to point at the same source.
            let candidate_source_id = format!("KSRC-{}", Uuid::now_v7().simple());
            let parser_status = match file.extraction.status {
                ExtractionStatus::Success | ExtractionStatus::Partial => "parsed",
                ExtractionStatus::Failed => "failed",
                _ => "pending",
            };
            let extraction_status = match file.extraction.status {
                ExtractionStatus::Success | ExtractionStatus::Partial => "extracted",
                ExtractionStatus::Failed => "failed",
                _ => "pending",
            };
            let source_id: String = query(
                "INSERT INTO knowledge_sources (source_id,workspace_id,root_id,source_kind,relative_path,content_hash,size_bytes,provenance,permission_scope,redaction_state,parser_status,extraction_status,last_index_receipt_event_id) VALUES ($1,$2,$3,'file',$4,$5,$6,$7,'workspace',$8,$9,$10,NULL) ON CONFLICT (root_id,relative_path) WHERE relative_path IS NOT NULL DO UPDATE SET workspace_id=EXCLUDED.workspace_id,content_hash=EXCLUDED.content_hash,size_bytes=EXCLUDED.size_bytes,provenance=EXCLUDED.provenance,redaction_state=EXCLUDED.redaction_state,parser_status=EXCLUDED.parser_status,extraction_status=EXCLUDED.extraction_status,stale=FALSE,updated_at=NOW() RETURNING source_id",
            )
            .bind(&candidate_source_id)
            .bind(&root.workspace_id)
            .bind(&root.root_id)
            .bind(&file.relative_path)
            .bind(&file.content_hash)
            .bind(file.size_bytes)
            .bind(json!({"discovered_by":"knowledge_code_nav_batch","run_token":run_token}))
            .bind(file.extraction.redaction_state.as_str())
            .bind(parser_status)
            .bind(extraction_status)
            .fetch_one(&mut *tx)
            .await
            .map_err(StorageError::from)?
            .try_get("source_id")
            .map_err(StorageError::from)?;
            let receipt_id = new_ingestion_id("KIRC");
            let event = NewKernelEvent::builder(
                ctx.kernel_task_run_id.clone(),
                ctx.session_run_id.clone(),
                KernelEventType::ValidationRecorded,
                ctx.actor.clone(),
            )
            .aggregate("knowledge_ingestion_receipt", &source_id)
            .source_component("knowledge_ingestion")
            .payload(json!({
                "kind": "extraction_receipt",
                "workspace_id": root.workspace_id,
                "root_id": root.root_id,
                "source_id": source_id,
                "relative_path": file.relative_path,
                "status": file.extraction.status.as_str(),
                "spans_produced": file.extraction.spans.len(),
                "run_token": run_token,
            }))
            .build()
            .map_err(|err| IngestionError::Kernel(err.to_string()))?;
            let event_id = append_kernel_event_with_executor(&mut *tx, event)
                .await
                .map_err(IngestionError::from)?
                .event_id;
            query("UPDATE knowledge_sources SET last_index_receipt_event_id=$1,updated_at=NOW() WHERE source_id=$2")
                .bind(&event_id)
                .bind(&source_id)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::from)?;
            query("INSERT INTO knowledge_ingestion_receipts (receipt_id,workspace_id,source_id,ingestion_run_token,extractor_id,extractor_version,status,error_class,error_detail,spans_produced,spans_failed,redaction_count,content_hash,duration_ms,receipt_event_id) VALUES ($1,$2,$3,$4,'code_file_windows','v1',$5,$6,$7,$8,$9,$10,$11,$12,$13)")
                .bind(&receipt_id).bind(&root.workspace_id).bind(&source_id).bind(run_token)
                .bind(file.extraction.status.as_str()).bind(file.extraction.error_class.map(|e| e.as_str()))
                .bind(&file.extraction.error_detail).bind(file.extraction.spans.len() as i32)
                .bind(file.extraction.spans_failed).bind(file.extraction.redaction_count as i32)
                .bind(&file.content_hash).bind(file.duration_ms).bind(&event_id)
                .execute(&mut *tx).await.map_err(StorageError::from)?;
            for (index, span) in file.extraction.spans.iter().enumerate() {
                let span_id = new_ingestion_id("KISP");
                query("INSERT INTO knowledge_ingestion_spans (span_id,workspace_id,source_id,receipt_id,span_index,anchor_kind,anchor,byte_start,byte_end,content,content_hash,redaction_state,link_candidates) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
                    .bind(&span_id).bind(&root.workspace_id).bind(&source_id).bind(&receipt_id)
                    .bind(index as i32).bind(span.anchor.kind_str()).bind(span.anchor.to_json())
                    .bind(span.byte_start).bind(span.byte_end).bind(&span.content)
                    .bind(hex::encode(Sha256::digest(span.content.as_bytes()))).bind(span.redaction.as_str())
                    .bind(serde_json::json!(span.link_candidates)).execute(&mut *tx).await.map_err(StorageError::from)?;
            }
                sources.push((source_id, file.relative_path.clone()));
            }
            // Reconcile sources that belonged to this root on the prior pass but were not
            // observed in the current canonical walk. The optimized code-nav batch must retain
            // the same deleted/moved lifecycle semantics as the general ingestion path: both the
            // KnowledgeSource authority row and its served code-file index state become stale in
            // the same transaction, so nav can never continue reporting the old symbol as fresh.
            let seen_paths: Vec<String> = prepared
                .iter()
                .map(|file| file.relative_path.clone())
                .collect();
            let stale_candidates = query(
                "SELECT source_id, relative_path, content_hash
                 FROM knowledge_sources
                 WHERE root_id = $1
                   AND relative_path IS NOT NULL
                   AND stale = FALSE
                   AND NOT (relative_path = ANY($2::text[]))
                 ORDER BY relative_path
                 FOR UPDATE",
            )
            .bind(&root.root_id)
            .bind(&seen_paths)
            .fetch_all(&mut *tx)
            .await
            .map_err(StorageError::from)?;
            let stale_marked = stale_candidates.len();
            for candidate in stale_candidates {
                let source_id: String = candidate
                    .try_get("source_id")
                    .map_err(StorageError::from)?;
                let relative_path: String = candidate
                    .try_get("relative_path")
                    .map_err(StorageError::from)?;
                let content_hash: String = candidate
                    .try_get("content_hash")
                    .map_err(StorageError::from)?;
                let moved_to = prepared
                    .iter()
                    .find(|file| {
                        file.content_hash == content_hash && file.relative_path != relative_path
                    })
                    .map(|file| file.relative_path.clone());
                let disposition = if moved_to.is_some() {
                    "moved"
                } else {
                    "deleted"
                };

                query(
                    "UPDATE knowledge_sources
                     SET stale = TRUE, updated_at = NOW()
                     WHERE source_id = $1",
                )
                .bind(&source_id)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::from)?;
                query(
                    "UPDATE knowledge_code_files
                     SET stale = TRUE, updated_at = NOW()
                     WHERE source_id = $1",
                )
                .bind(&source_id)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::from)?;

                let mut stale_event = NewKernelEvent::builder(
                    ctx.kernel_task_run_id.clone(),
                    ctx.session_run_id.clone(),
                    KernelEventType::ValidationRecorded,
                    ctx.actor.clone(),
                )
                .aggregate("knowledge_source_lifecycle", &source_id)
                .source_component("knowledge_ingestion")
                .payload(json!({
                    "kind": "source_stale_marked",
                    "disposition": disposition,
                    "workspace_id": root.workspace_id,
                    "root_id": root.root_id,
                    "source_id": source_id,
                    "relative_path": relative_path,
                    "moved_to": moved_to,
                    "run_token": run_token,
                }));
                if let Some(correlation_id) = &ctx.correlation_id {
                    stale_event = stale_event.correlation_id(correlation_id.clone());
                }
                let stale_event = stale_event
                    .build()
                    .map_err(|err| IngestionError::Kernel(err.to_string()))?;
                append_kernel_event_with_executor(&mut *tx, stale_event)
                    .await
                    .map_err(IngestionError::from)?;
            }

            let finish_event = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            KernelEventType::ValidationRecorded,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_ingestion_run", run_token)
        .causation_id(start_event_id.clone())
        .source_component("knowledge_ingestion")
        .payload(json!({
            "kind": "ingestion_run_finished",
            "workspace_id": root.workspace_id,
            "root_id": root.root_id,
            "run_token": run_token,
            "files_ingested": sources.len(),
            "stale_marked": stale_marked,
        }))
        .build()
        .map_err(|err| IngestionError::Kernel(err.to_string()))?;
            append_kernel_event_with_executor(&mut *tx, finish_event)
                .await
                .map_err(IngestionError::from)?;
            Ok(sources)
        }
        .await;

        match result {
            Ok(sources) => {
                tx.commit().await.map_err(StorageError::from)?;
                Ok(sources)
            }
            Err(error) => {
                // Roll back all partial source/receipt/span writes before
                // recording a terminal failure against the already committed
                // start event. The caller still receives the original error.
                let _ = tx.rollback().await;
                self.append_ingestion_failure_receipt(
                    ctx,
                    root,
                    run_token,
                    &start_event_id,
                    &error,
                )
                .await;
                Err(error)
            }
        }
    }

    /// Append one ingestion EventLedger receipt event.
    pub(crate) async fn append_receipt_event(
        &self,
        ctx: &IngestionContext,
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> IngestionResult<String> {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            event_type,
            ctx.actor.clone(),
        )
        .aggregate(aggregate_type, aggregate_id)
        .source_component("knowledge_ingestion")
        .payload(payload);
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        let event = builder
            .build()
            .map_err(|err| IngestionError::Kernel(err.to_string()))?;
        let stored = self.db.append_kernel_event(event).await?;
        Ok(stored.event_id)
    }

    /// Close an ingestion pass with a durable terminal failure receipt. This
    /// is deliberately best-effort: the original storage/IO error remains the
    /// caller-visible failure, while a healthy ledger records that the started
    /// pass did not finish successfully.
    async fn append_ingestion_failure_receipt(
        &self,
        ctx: &IngestionContext,
        root: &KnowledgeSourceRoot,
        run_token: &str,
        start_event_id: &str,
        error: &IngestionError,
    ) {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            KernelEventType::ValidationRecorded,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_ingestion_run", run_token)
        .causation_id(start_event_id.to_string())
        .source_component("knowledge_ingestion")
        .payload(json!({
            "kind": "ingestion_run_failed",
            "workspace_id": root.workspace_id,
            "root_id": root.root_id,
            "run_token": run_token,
            "error": error.to_string(),
        }));
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        if let Ok(event) = builder.build() {
            let _ = self.db.append_kernel_event(event).await;
        }
    }

    /// MT-081: enforce the workspace root allowlist, persist the decision
    /// (durable row + EventLedger receipt), then register the root through
    /// the storage layer. Denials return a typed error carrying the durable
    /// decision id — never a silent skip.
    pub async fn register_root(
        &self,
        ctx: &IngestionContext,
        request: RootRegistrationRequest,
    ) -> IngestionResult<(KnowledgeSourceRoot, PolicyDecision)> {
        ctx.validate()?;
        let candidate_path =
            crate::storage::knowledge::normalize_repo_relative_path(&request.repo_relative_path)?;

        let stored_policy = self
            .store
            .get_active_root_policy(&request.workspace_id)
            .await?;
        let (policy_id, policy) = match &stored_policy {
            Some(stored) => (Some(stored.policy_id.clone()), stored.policy.clone()),
            None => (None, RootRegistrationPolicy::default()),
        };
        let compiled = CompiledRootPolicy::compile(policy)?;
        let verdict = compiled.evaluate(
            &candidate_path,
            request.root_kind,
            request.operator_approved,
        );

        let event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::ValidationRecorded,
                "knowledge_root_registration",
                if candidate_path.is_empty() {
                    "."
                } else {
                    &candidate_path
                },
                json!({
                    "kind": "root_registration_policy_decision",
                    "workspace_id": request.workspace_id,
                    "candidate_path": candidate_path,
                    "root_kind": request.root_kind.as_str(),
                    "verdict": verdict.kind.as_str(),
                    "matched_pattern": verdict.matched_pattern,
                    "operator_approved": request.operator_approved,
                    "policy_id": policy_id,
                }),
            )
            .await?;

        let decision = self
            .store
            .record_policy_decision(NewPolicyDecision {
                workspace_id: request.workspace_id.clone(),
                policy_id,
                candidate_path: candidate_path.clone(),
                root_kind: request.root_kind.as_str().to_string(),
                verdict: verdict.kind,
                matched_pattern: verdict.matched_pattern.clone(),
                operator_approved: request.operator_approved,
                actor_kind: ctx.actor.actor_kind().to_string(),
                actor_id: ctx.actor.actor_id().to_string(),
                receipt_event_id: Some(event_id),
            })
            .await?;

        if !verdict.kind.is_allowed() {
            return Err(IngestionError::PolicyDenied {
                verdict: verdict.kind,
                candidate_path,
                matched_pattern: verdict.matched_pattern,
                decision_id: decision.decision_id,
            });
        }

        let root = self
            .db
            .create_knowledge_source_root(NewKnowledgeSourceRoot {
                workspace_id: request.workspace_id,
                display_name: request.display_name,
                root_kind: request.root_kind,
                repo_relative_path: candidate_path,
                allowlist_policy: request.file_allowlist_policy,
                indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
            })
            .await?;
        Ok((root, decision))
    }

    /// Ingest one file payload under a registered root: the full
    /// MT-082..MT-092 pipeline. Pure extraction first, then durable
    /// persistence (source upsert through the storage layer, receipt + ledger
    /// event, spans, rollup statuses, repair enqueue).
    pub async fn ingest_file_bytes(
        &self,
        ctx: &IngestionContext,
        root: &KnowledgeSourceRoot,
        relative_path: &str,
        bytes: &[u8],
        run_token: &str,
        limits: &IngestionLimits,
        suppress_repair: bool,
    ) -> IngestionResult<FileIngestOutcome> {
        ctx.validate()?;
        let normalized = normalize_source_relative_path(relative_path)?;
        let hashes = compute_content_hashes(bytes);
        let started = Instant::now();
        let extraction = run_extraction(root.root_kind, &normalized, bytes, &hashes, limits);
        let duration_ms = started.elapsed().as_millis() as i64;
        self.persist_attempt(
            ctx,
            root,
            &normalized,
            &hashes,
            bytes.len() as i64,
            run_token,
            extraction,
            duration_ms,
            suppress_repair,
        )
        .await
    }

    /// Ingest a file from disk. Oversize files (per metadata) are
    /// stream-hashed (bounded memory, never loaded whole) and recorded as
    /// typed deferrals (MT-092 no-OOM guarantee).
    #[allow(clippy::too_many_arguments)]
    async fn ingest_file_from_disk(
        &self,
        ctx: &IngestionContext,
        root: &KnowledgeSourceRoot,
        relative_path: &str,
        absolute_path: &Path,
        size_bytes: u64,
        run_token: &str,
        limits: &IngestionLimits,
        suppress_repair: bool,
    ) -> IngestionResult<FileIngestOutcome> {
        let normalized = normalize_source_relative_path(relative_path)?;
        let kind = detect_kind(root.root_kind, &normalized);
        let byte_limit = kind
            .map(|k| limits.byte_limit_for(k))
            .unwrap_or(limits.max_bytes);

        if size_bytes > byte_limit {
            // Stream-hash without loading the file (same SHA-256 authority).
            let raw_sha256 = stream_sha256(absolute_path)?;
            let hashes = ContentHashes {
                raw_sha256,
                normalized_text_sha256: None,
                is_text: false,
            };
            let deferral_detail = json!({
                "reason": "oversize_bytes",
                "limit": byte_limit,
                "actual": size_bytes,
            });
            let extraction = ExtractionOutput {
                kind,
                status: ExtractionStatus::Deferred,
                error_class: Some(IngestionErrorClass::Oversize),
                error_detail: Some(deferral_detail),
                spans: Vec::new(),
                spans_failed: 0,
                redaction_count: 0,
                redaction_state: KnowledgeRedactionState::None,
            };
            return self
                .persist_attempt(
                    ctx,
                    root,
                    &normalized,
                    &hashes,
                    size_bytes as i64,
                    run_token,
                    extraction,
                    0,
                    suppress_repair,
                )
                .await;
        }

        let bytes = std::fs::read(absolute_path).map_err(|err| IngestionError::Io {
            path: absolute_path.display().to_string(),
            detail: err.to_string(),
        })?;
        self.ingest_file_bytes(
            ctx,
            root,
            &normalized,
            &bytes,
            run_token,
            limits,
            suppress_repair,
        )
        .await
    }

    /// Persist one extraction attempt: source upsert THROUGH the storage
    /// layer, EventLedger receipt event, receipt row, span replacement,
    /// per-source rollup statuses, repair-queue enqueue.
    #[allow(clippy::too_many_arguments)]
    async fn persist_attempt(
        &self,
        ctx: &IngestionContext,
        root: &KnowledgeSourceRoot,
        normalized_path: &str,
        hashes: &ContentHashes,
        size_bytes: i64,
        run_token: &str,
        extraction: ExtractionOutput,
        duration_ms: i64,
        suppress_repair: bool,
    ) -> IngestionResult<FileIngestOutcome> {
        let (extractor_id, extractor_version) = extractor_identity(extraction.kind);
        let mut provenance = json!({
            "discovered_by": "knowledge_ingestion_v1",
            "run_token": run_token,
            "ingestion_kind": extraction.kind.map(|k| k.as_str()),
            "normalized_text_sha256": hashes.normalized_text_sha256,
        });
        if extraction.kind == Some(IngestionSourceKind::GovernanceArtifact) {
            provenance["governance_sub_kind"] =
                json!(detect_governance_sub_kind(normalized_path).map(|s| s.as_str()));
        }
        if extraction.kind == Some(IngestionSourceKind::OperatorResearchNote) {
            // MT-090: operator research notes are context, not authority,
            // until a spec enrichment promotes them.
            provenance["authority"] = json!("non_normative_context");
        }

        let source = self
            .db
            .upsert_knowledge_source(NewKnowledgeSource {
                workspace_id: root.workspace_id.clone(),
                root_id: Some(root.root_id.clone()),
                source_kind: KnowledgeSourceKind::File,
                relative_path: Some(normalized_path.to_string()),
                asset_id: None,
                loom_block_id: None,
                document_id: None,
                content_hash: hashes.raw_sha256.clone(),
                size_bytes: Some(size_bytes),
                provenance,
                permission_scope: KnowledgePermissionScope::Workspace,
                redaction_state: extraction.redaction_state,
                source_modified_at: None,
            })
            .await?;

        let event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::ValidationRecorded,
                "knowledge_ingestion_receipt",
                &source.source_id,
                json!({
                    "kind": "extraction_receipt",
                    "workspace_id": root.workspace_id,
                    "root_id": root.root_id,
                    "source_id": source.source_id,
                    "relative_path": normalized_path,
                    "ingestion_kind": extraction.kind.map(|k| k.as_str()),
                    "extractor_id": extractor_id,
                    "extractor_version": extractor_version,
                    "status": extraction.status.as_str(),
                    "error_class": extraction.error_class.map(|c| c.as_str()),
                    "spans_produced": extraction.spans.len(),
                    "spans_failed": extraction.spans_failed,
                    "redaction_count": extraction.redaction_count,
                    "content_hash": hashes.raw_sha256,
                    "run_token": run_token,
                }),
            )
            .await?;

        let receipt = self
            .store
            .record_extraction_receipt(
                NewExtractionReceipt {
                    workspace_id: root.workspace_id.clone(),
                    source_id: source.source_id.clone(),
                    ingestion_run_token: Some(run_token.to_string()),
                    extractor_id: extractor_id.to_string(),
                    extractor_version: extractor_version.to_string(),
                    status: extraction.status,
                    error_class: extraction.error_class,
                    error_detail: extraction.error_detail.clone(),
                    spans_produced: extraction.spans.len() as i32,
                    spans_failed: extraction.spans_failed,
                    redaction_count: extraction.redaction_count,
                    content_hash: hashes.raw_sha256.clone(),
                    duration_ms,
                },
                Some(&event_id),
            )
            .await?;

        let stored_spans = if extraction.spans.is_empty() {
            Vec::new()
        } else {
            self.store
                .replace_source_spans(
                    &root.workspace_id,
                    &source.source_id,
                    &receipt.receipt_id,
                    &extraction.spans,
                )
                .await?
        };

        // Per-source rollup (storage layer): deferred attempts leave the
        // source pending; everything else lands a terminal parser/extraction
        // status plus the FK-bound ledger receipt.
        let source = match extraction.status {
            ExtractionStatus::Deferred => source,
            ExtractionStatus::Success | ExtractionStatus::Partial => {
                self.db
                    .record_knowledge_source_index_receipt(
                        &source.source_id,
                        KnowledgeParserStatus::Parsed,
                        KnowledgeExtractionStatus::Extracted,
                        &event_id,
                    )
                    .await?
            }
            ExtractionStatus::Failed => {
                self.db
                    .record_knowledge_source_index_receipt(
                        &source.source_id,
                        KnowledgeParserStatus::Failed,
                        KnowledgeExtractionStatus::Failed,
                        &event_id,
                    )
                    .await?
            }
            ExtractionStatus::Skipped | ExtractionStatus::Blocked => {
                self.db
                    .record_knowledge_source_index_receipt(
                        &source.source_id,
                        KnowledgeParserStatus::Skipped,
                        KnowledgeExtractionStatus::Skipped,
                        &event_id,
                    )
                    .await?
            }
        };

        let repair = if !suppress_repair {
            match (extraction.status, extraction.error_class) {
                (
                    ExtractionStatus::Failed
                    | ExtractionStatus::Partial
                    | ExtractionStatus::Deferred,
                    Some(class),
                ) if class.is_repairable() => Some(
                    self.store
                        .enqueue_repair(NewRepairEntry {
                            workspace_id: root.workspace_id.clone(),
                            source_id: source.source_id.clone(),
                            receipt_id: Some(receipt.receipt_id.clone()),
                            reason_class: RepairReason::Extraction(class),
                            reason_detail: extraction
                                .error_detail
                                .clone()
                                .unwrap_or_else(|| json!({})),
                            max_attempts: 3,
                            enqueue_event_id: Some(event_id.clone()),
                        })
                        .await?,
                ),
                _ => None,
            }
        } else {
            None
        };

        Ok(FileIngestOutcome {
            source,
            receipt,
            spans: stored_spans,
            repair,
        })
    }

    /// MT-093 + orchestration: run a full ingestion pass over a registered
    /// root anchored at `fs_anchor` (the machine-local checkout root —
    /// runtime configuration, never stored authority). Walks eligible files,
    /// ingests each, then detects deleted/moved sources (stale markers +
    /// events, no hard deletes).
    pub async fn run_ingestion_pass(
        &self,
        ctx: &IngestionContext,
        root_id: &str,
        fs_anchor: &Path,
        limits: &IngestionLimits,
    ) -> IngestionResult<IngestionPassSummary> {
        ctx.validate()?;
        let root =
            self.db
                .get_knowledge_source_root(root_id)
                .await?
                .ok_or(IngestionError::Storage(
                    crate::storage::StorageError::NotFound("knowledge source root"),
                ))?;
        if root.indexing_eligibility != KnowledgeIndexingEligibility::Eligible {
            return Err(IngestionError::Validation(format!(
                "root {root_id} is not eligible for indexing ({})",
                root.indexing_eligibility.as_str()
            )));
        }

        let run_token = new_ingestion_id("KIRUN");
        let start_event_id = self
            .append_receipt_event(
                ctx,
                KernelEventType::ValidationRecorded,
                "knowledge_ingestion_run",
                &run_token,
                json!({
                    "kind": "ingestion_run_started",
                    "workspace_id": root.workspace_id,
                    "root_id": root.root_id,
                    "run_token": run_token,
                }),
            )
            .await?;

        let root_dir = if root.repo_relative_path.is_empty() {
            fs_anchor.to_path_buf()
        } else {
            fs_anchor.join(&root.repo_relative_path)
        };
        if !root_dir.is_dir() {
            let error = IngestionError::Io {
                path: root_dir.display().to_string(),
                detail: "registered root does not exist on disk under the runtime anchor"
                    .to_string(),
            };
            self.append_ingestion_failure_receipt(
                ctx,
                &root,
                &run_token,
                &start_event_id,
                &error,
            )
            .await;
            return Err(error);
        }

        let file_filter = compile_file_allowlist(&root.allowlist_policy)?;
        let mut walked: Vec<(String, PathBuf, u64)> = Vec::new();
        let mut walk_errors: Vec<String> = Vec::new();
        walk_files(&root_dir, &root_dir, &mut walked, &mut walk_errors);

        let mut outcomes: Vec<FileIngestOutcome> = Vec::new();
        let mut skipped_by_allowlist = 0usize;
        let mut invalid_paths: Vec<String> = Vec::new();
        let mut ingest_inputs: Vec<(String, PathBuf, u64)> = Vec::new();
        for (rel_path, abs_path, size) in walked {
            let normalized = match normalize_source_relative_path(&rel_path) {
                Ok(normalized) => normalized,
                Err(err) => {
                    invalid_paths.push(format!("{rel_path}: {err}"));
                    continue;
                }
            };
            if !file_filter.allows(&normalized) {
                skipped_by_allowlist += 1;
                continue;
            }
            ingest_inputs.push((normalized, abs_path, size));
        }

        // Extraction and persistence are independent per source. Keep the
        // canonical walk, allowlist, and stale-source reconciliation ordered,
        // but overlap bounded file work so a large codebase does not spend the
        // whole route serially waiting on one pooled connection at a time.
        // Every future is drained before the first error is returned; this
        // prevents fail-fast cancellation from leaving sibling source writes
        // half-completed or receipts missing.
        let engine = Arc::new(self.clone());
        let context = ctx.clone();
        let root_for_tasks = root.clone();
        let run_token_for_tasks = run_token.clone();
        let limits_for_tasks = *limits;
        let ingestion_concurrency = crate::storage::configured_postgres_parallelism();
        let ingest_results = stream::iter(ingest_inputs)
            .map(|(normalized, abs_path, size)| {
                let engine = engine.clone();
                let context = context.clone();
                let root = root_for_tasks.clone();
                let run_token = run_token_for_tasks.clone();
                async move {
                    engine
                        .ingest_file_from_disk(
                            &context,
                            &root,
                            &normalized,
                            &abs_path,
                            size,
                            &run_token,
                            &limits_for_tasks,
                            false,
                        )
                        .await
                }
            })
            .buffer_unordered(ingestion_concurrency)
            .collect::<Vec<_>>()
            .await;
        let mut first_ingestion_error = None;
        for result in ingest_results {
            match result {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => {
                    first_ingestion_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = first_ingestion_error {
            self.append_ingestion_failure_receipt(
                ctx,
                &root,
                &run_token,
                &start_event_id,
                &error,
            )
            .await;
            return Err(error);
        }

        // MT-093: deleted/moved detection over the storage layer.
        let seen_paths: HashSet<String> = outcomes
            .iter()
            .filter_map(|o| o.source.relative_path.clone())
            .collect();
        let existing = self
            .db
            .list_knowledge_sources_for_root(&root.root_id)
            .await?;
        let mut stale_marked: Vec<StaleSourceMark> = Vec::new();
        for source in existing {
            let Some(path) = source.relative_path.clone() else {
                continue;
            };
            if seen_paths.contains(&path) || source.stale {
                continue;
            }
            // Same content at a new path => moved; otherwise deleted.
            let moved_to = outcomes
                .iter()
                .find(|o| {
                    o.source.content_hash == source.content_hash
                        && o.source.source_id != source.source_id
                })
                .and_then(|o| o.source.relative_path.clone());
            let disposition = if moved_to.is_some() {
                "moved"
            } else {
                "deleted"
            };
            self.db
                .mark_knowledge_source_stale(&source.source_id)
                .await?;
            let event_id = self
                .append_receipt_event(
                    ctx,
                    KernelEventType::ValidationRecorded,
                    "knowledge_source_lifecycle",
                    &source.source_id,
                    json!({
                        "kind": "source_stale_marked",
                        "disposition": disposition,
                        "workspace_id": root.workspace_id,
                        "root_id": root.root_id,
                        "source_id": source.source_id,
                        "relative_path": path,
                        "moved_to": moved_to,
                        "run_token": run_token,
                    }),
                )
                .await?;
            stale_marked.push(StaleSourceMark {
                source_id: source.source_id,
                relative_path: path,
                disposition: disposition.to_string(),
                moved_to,
                event_id,
            });
        }

        let summary_counts = json!({
            "files_ingested": outcomes.len(),
            "skipped_by_allowlist": skipped_by_allowlist,
            "invalid_paths": invalid_paths.len(),
            "walk_errors": walk_errors.len(),
            "stale_marked": stale_marked.len(),
            "success": count_status(&outcomes, ExtractionStatus::Success),
            "partial": count_status(&outcomes, ExtractionStatus::Partial),
            "failed": count_status(&outcomes, ExtractionStatus::Failed),
            "deferred": count_status(&outcomes, ExtractionStatus::Deferred),
            "skipped": count_status(&outcomes, ExtractionStatus::Skipped),
            "blocked": count_status(&outcomes, ExtractionStatus::Blocked),
        });
        let finish_event_id = {
            let mut builder = NewKernelEvent::builder(
                ctx.kernel_task_run_id.clone(),
                ctx.session_run_id.clone(),
                KernelEventType::ValidationRecorded,
                ctx.actor.clone(),
            )
            .aggregate("knowledge_ingestion_run", run_token.clone())
            .causation_id(start_event_id.clone())
            .source_component("knowledge_ingestion")
            .payload(json!({
                "kind": "ingestion_run_finished",
                "workspace_id": root.workspace_id,
                "root_id": root.root_id,
                "run_token": run_token,
                "counts": summary_counts,
            }));
            if let Some(correlation_id) = &ctx.correlation_id {
                builder = builder.correlation_id(correlation_id.clone());
            }
            let event = builder
                .build()
                .map_err(|err| IngestionError::Kernel(err.to_string()))?;
            self.db.append_kernel_event(event).await?.event_id
        };

        Ok(IngestionPassSummary {
            run_token,
            root_id: root.root_id,
            workspace_id: root.workspace_id,
            outcomes,
            stale_marked,
            skipped_by_allowlist,
            invalid_paths,
            walk_errors,
            start_event_id,
            finish_event_id,
        })
    }

    /// MT-094: retry one repair-queue entry. Claims the attempt (budgeted),
    /// re-runs the ingest for the source's file, settles the entry
    /// (resolved / requeued / dead-lettered) and returns both.
    pub async fn retry_repair(
        &self,
        ctx: &IngestionContext,
        repair_id: &str,
        fs_anchor: &Path,
        limits: &IngestionLimits,
    ) -> IngestionResult<(RepairEntry, Option<FileIngestOutcome>)> {
        ctx.validate()?;
        let entry = self.store.begin_repair_attempt(repair_id).await?;
        let source = self
            .db
            .get_knowledge_source(&entry.source_id)
            .await?
            .ok_or(IngestionError::Storage(
                crate::storage::StorageError::NotFound("knowledge source for repair entry"),
            ))?;
        let root_id = source
            .root_id
            .clone()
            .ok_or_else(|| IngestionError::Validation("repair source has no root".to_string()))?;
        let root =
            self.db
                .get_knowledge_source_root(&root_id)
                .await?
                .ok_or(IngestionError::Storage(
                    crate::storage::StorageError::NotFound("root for repair entry"),
                ))?;
        let relative_path = source.relative_path.clone().ok_or_else(|| {
            IngestionError::Validation("repair source has no relative path".to_string())
        })?;

        let root_dir = if root.repo_relative_path.is_empty() {
            fs_anchor.to_path_buf()
        } else {
            fs_anchor.join(&root.repo_relative_path)
        };
        let absolute = root_dir.join(&relative_path);
        let metadata = match std::fs::metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(err) => {
                // File vanished: the entry stays open (or dead-letters) with
                // a missing-asset reason.
                let settled = self
                    .store
                    .settle_repair_attempt(
                        repair_id,
                        RepairAttemptOutcome::FailedAgain {
                            receipt_id: None,
                            reason_detail: json!({
                                "reason": "MISSING_ASSET",
                                "path": relative_path,
                                "io_error": err.to_string(),
                            }),
                        },
                    )
                    .await?;
                return Ok((settled, None));
            }
        };

        let outcome = self
            .ingest_file_from_disk(
                ctx,
                &root,
                &relative_path,
                &absolute,
                metadata.len(),
                &format!("KIRETRY-{}", entry.attempts),
                limits,
                true,
            )
            .await?;

        let settled = match outcome.receipt.status {
            // Partial resolves too: the retry produced spans and the
            // residual loss is documented on the receipt — keeping the entry
            // open would loop forever on identical content.
            ExtractionStatus::Success | ExtractionStatus::Partial => {
                self.store
                    .settle_repair_attempt(
                        repair_id,
                        RepairAttemptOutcome::Resolved {
                            resolved_receipt_id: outcome.receipt.receipt_id.clone(),
                        },
                    )
                    .await?
            }
            _ => {
                self.store
                    .settle_repair_attempt(
                        repair_id,
                        RepairAttemptOutcome::FailedAgain {
                            receipt_id: Some(outcome.receipt.receipt_id.clone()),
                            reason_detail: outcome
                                .receipt
                                .error_detail
                                .clone()
                                .unwrap_or_else(|| json!({})),
                        },
                    )
                    .await?
            }
        };
        Ok((settled, Some(outcome)))
    }
}

// ---------------------------------------------------------------------------
// Pure extraction phase (no DB).
// ---------------------------------------------------------------------------

/// Outcome of the pure extraction phase.
struct ExtractionOutput {
    kind: Option<IngestionSourceKind>,
    status: ExtractionStatus,
    error_class: Option<IngestionErrorClass>,
    error_detail: Option<Value>,
    spans: Vec<ExtractedSpan>,
    spans_failed: i32,
    redaction_count: i32,
    redaction_state: KnowledgeRedactionState,
}

impl ExtractionOutput {
    fn failure(
        kind: Option<IngestionSourceKind>,
        status: ExtractionStatus,
        class: IngestionErrorClass,
        detail: Value,
    ) -> Self {
        Self {
            kind,
            status,
            error_class: Some(class),
            error_detail: Some(detail),
            spans: Vec::new(),
            spans_failed: 0,
            redaction_count: 0,
            redaction_state: KnowledgeRedactionState::None,
        }
    }
}

/// Extractor identity per kind (recorded on receipts; version-bumped when
/// extractor behavior changes).
fn extractor_identity(kind: Option<IngestionSourceKind>) -> (&'static str, &'static str) {
    match kind {
        Some(IngestionSourceKind::Pdf) => (PDF_EXTRACTOR_ID, PDF_EXTRACTOR_VERSION),
        Some(IngestionSourceKind::MediaTranscript) => ("media_transcript", "v1"),
        Some(IngestionSourceKind::GovernanceArtifact) => ("governance_artifact", "v1"),
        Some(IngestionSourceKind::OperatorResearchNote) => ("operator_research_note", "v1"),
        Some(IngestionSourceKind::MarkdownText) => ("markdown_text", "v1"),
        Some(IngestionSourceKind::CodeFile) => ("code_file_windows", "v1"),
        Some(IngestionSourceKind::RichDocument) => ("rich_document", "v1"),
        Some(IngestionSourceKind::ExternalImport) => ("external_import_stat", "v1"),
        None => ("kind_detector", "v1"),
    }
}

/// Fixed-window line spans for code files (120 lines per span).
fn code_spans(text: &str) -> Vec<ExtractedSpan> {
    const WINDOW: usize = 120;
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let mut spans = Vec::new();
    let mut byte_cursor = 0usize;
    // Pre-compute line byte offsets.
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    for line in &lines {
        offsets.push(byte_cursor);
        byte_cursor += line.len() + 1; // '\n'
    }
    offsets.push(text.len());

    for window_start in (0..lines.len()).step_by(WINDOW) {
        let window_end = (window_start + WINDOW).min(lines.len());
        let content = lines[window_start..window_end].join("\n");
        if content.trim().is_empty() {
            continue;
        }
        let byte_start = offsets[window_start] as i64;
        let byte_end = (offsets[window_end].min(text.len())) as i64;
        spans.push(
            ExtractedSpan::new(
                SpanAnchor::LineRange {
                    line_start: (window_start + 1) as u32,
                    line_end: window_end as u32,
                    heading_path: Vec::new(),
                },
                content,
            )
            .with_bytes(byte_start, byte_end),
        );
    }
    spans
}

/// Apply the MT-091 redaction pass over extracted spans using WHOLE-FILE
/// findings (#1 cross-span boundary fix). Spans carry their byte offset into
/// the original scanned text; whole-file findings are mapped onto each span's
/// byte range so a secret that STRADDLES a window boundary — and therefore
/// matches no per-window re-scan — is still excised from every span it
/// touches. Spans WITHOUT byte offsets (PDF pages, transcript cues, JSON
/// pointers) are natural units extracted from their own text and fall back to
/// a per-span scan. Returns the total redaction count.
fn redact_spans(spans: &mut [ExtractedSpan], whole_file: &SecretScanReport) -> i32 {
    let mut total = 0i32;
    for span in spans.iter_mut() {
        match span.byte_start {
            Some(byte_start) if byte_start >= 0 => {
                let outcome = redact_span_with_whole_file_findings(
                    &span.content,
                    byte_start as usize,
                    whole_file,
                );
                if outcome.redactions > 0 {
                    total += outcome.redactions as i32;
                    span.content = outcome.content;
                    span.redaction = SpanRedaction::Redacted;
                }
            }
            // No source-byte mapping: scan the span's own content directly.
            _ => {
                let report = scan_text(&span.content);
                if !report.is_clean() {
                    total += report.redaction_count() as i32;
                    span.content = redact_text(&span.content, &report);
                    span.redaction = SpanRedaction::Redacted;
                }
            }
        }
    }
    total
}

/// The pure extraction pipeline: kind detection, backpressure, secret
/// preflight, per-kind span extraction, span-level redaction.
fn run_extraction(
    root_kind: KnowledgeRootKind,
    normalized_path: &str,
    bytes: &[u8],
    hashes: &ContentHashes,
    limits: &IngestionLimits,
) -> ExtractionOutput {
    let Some(kind) = detect_kind(root_kind, normalized_path) else {
        return ExtractionOutput::failure(
            None,
            ExtractionStatus::Skipped,
            IngestionErrorClass::UnsupportedFormat,
            json!({"path": normalized_path, "reason": "no ingestion kind claims this file"}),
        );
    };
    let spec = spec_for(kind);

    if let Err(deferral) = limits.check_size(kind, bytes.len() as u64) {
        return ExtractionOutput::failure(
            Some(kind),
            ExtractionStatus::Deferred,
            IngestionErrorClass::Oversize,
            deferral.detail_json(),
        );
    }

    // Text decode + whole-file secret preflight for text-based kinds.
    let text: Option<&str> = if hashes.is_text {
        // Safe: is_text == valid UTF-8 (computed by compute_content_hashes).
        std::str::from_utf8(bytes).ok()
    } else {
        None
    };

    let needs_text = !matches!(
        kind,
        IngestionSourceKind::Pdf | IngestionSourceKind::ExternalImport
    );
    let Some(text) = text.or(if needs_text { None } else { Some("") }) else {
        return ExtractionOutput::failure(
            Some(kind),
            ExtractionStatus::Failed,
            IngestionErrorClass::ParseError,
            json!({"reason": "binary content where text was required", "kind": kind.as_str()}),
        );
    };

    if needs_text {
        if let Err(deferral) = limits.check_lines(text.lines().count() as u64) {
            return ExtractionOutput::failure(
                Some(kind),
                ExtractionStatus::Deferred,
                IngestionErrorClass::Oversize,
                deferral.detail_json(),
            );
        }
    }

    // MT-091 whole-file preflight: scan ONCE over the full contiguous text.
    // The report is reused for (a) the HIGH-severity block decision and (b)
    // the span-level redaction pass below, so MEDIUM findings that straddle a
    // window boundary (#1) are caught — they are visible to the whole-file
    // scan even when no single window holds the whole secret.
    let whole_file_report: Option<SecretScanReport> =
        if spec.capabilities.secret_scan && needs_text {
            let report = scan_text(text);
            if report.must_block() {
                return ExtractionOutput {
                    kind: Some(kind),
                    status: ExtractionStatus::Blocked,
                    error_class: Some(IngestionErrorClass::SecretBlocked),
                    error_detail: Some(secret_block_detail(&report)),
                    spans: Vec::new(),
                    spans_failed: 0,
                    redaction_count: report.redaction_count() as i32,
                    redaction_state: KnowledgeRedactionState::Redacted,
                };
            }
            Some(report)
        } else {
            None
        };

    // Per-kind extraction over the RAW text (anchors reference the source;
    // redaction below only rewrites stored content).
    let mut output = match kind {
        IngestionSourceKind::CodeFile => ExtractionOutput {
            kind: Some(kind),
            status: ExtractionStatus::Success,
            error_class: None,
            error_detail: None,
            spans: code_spans(text),
            spans_failed: 0,
            redaction_count: 0,
            redaction_state: KnowledgeRedactionState::None,
        },
        IngestionSourceKind::MarkdownText | IngestionSourceKind::OperatorResearchNote => {
            let parse = parse_note(text);
            ExtractionOutput {
                kind: Some(kind),
                status: ExtractionStatus::Success,
                error_class: None,
                error_detail: Some(json!({
                    "headings_seen": parse.headings_seen,
                    "link_candidates": parse.link_candidates,
                })),
                spans: parse.spans,
                spans_failed: 0,
                redaction_count: 0,
                redaction_state: KnowledgeRedactionState::None,
            }
        }
        IngestionSourceKind::GovernanceArtifact => {
            let is_jsonl = normalized_path.to_ascii_lowercase().ends_with(".jsonl");
            let parse_result = if is_jsonl {
                parse_governance_jsonl(text, &GovernanceSpanLimits::default())
            } else {
                parse_governance_json(text, &GovernanceSpanLimits::default())
            };
            match parse_result {
                Ok(parse) => {
                    let lossy = is_jsonl && parse.skipped_nodes > 0;
                    ExtractionOutput {
                        kind: Some(kind),
                        status: if lossy {
                            ExtractionStatus::Partial
                        } else {
                            ExtractionStatus::Success
                        },
                        error_class: if lossy {
                            Some(IngestionErrorClass::ParseError)
                        } else {
                            None
                        },
                        error_detail: Some(json!({"skipped_nodes": parse.skipped_nodes})),
                        spans_failed: parse.skipped_nodes as i32,
                        spans: parse.spans,
                        redaction_count: 0,
                        redaction_state: KnowledgeRedactionState::None,
                    }
                }
                Err(err) => ExtractionOutput::failure(
                    Some(kind),
                    ExtractionStatus::Failed,
                    err.class,
                    json!({"detail": err.detail}),
                ),
            }
        }
        IngestionSourceKind::MediaTranscript => {
            match parse_transcript_artifact(normalized_path, text) {
                Ok(parse) => {
                    let spans: Vec<ExtractedSpan> = parse
                        .cues
                        .iter()
                        .map(|cue| {
                            ExtractedSpan::new(
                                SpanAnchor::MediaTime {
                                    start_ms: cue.start_ms,
                                    end_ms: cue.end_ms,
                                    cue_index: cue.index,
                                },
                                cue.text.clone(),
                            )
                        })
                        .collect();
                    let lossy = !parse.malformed.is_empty();
                    ExtractionOutput {
                        kind: Some(kind),
                        status: if lossy {
                            ExtractionStatus::Partial
                        } else {
                            ExtractionStatus::Success
                        },
                        error_class: if lossy {
                            Some(IngestionErrorClass::MalformedCue)
                        } else {
                            None
                        },
                        error_detail: Some(json!({
                            "format": parse.format.as_str(),
                            "malformed_cues": parse.malformed,
                        })),
                        spans_failed: parse.malformed.len() as i32,
                        spans,
                        redaction_count: 0,
                        redaction_state: KnowledgeRedactionState::None,
                    }
                }
                Err(err) => ExtractionOutput::failure(
                    Some(kind),
                    ExtractionStatus::Failed,
                    err.class,
                    json!({"detail": err.detail}),
                ),
            }
        }
        IngestionSourceKind::Pdf => match extract_pdf_text(bytes) {
            Ok(extraction) => {
                let spans: Vec<ExtractedSpan> = extraction
                    .pages
                    .iter()
                    .map(|page| {
                        ExtractedSpan::new(
                            SpanAnchor::PdfPage { page: page.page },
                            page.text.clone(),
                        )
                    })
                    .collect();
                let partial = extraction.is_partial();
                // MT-091 for PDFs: preflight over the EXTRACTED text.
                let concatenated: String = extraction
                    .pages
                    .iter()
                    .map(|p| p.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let report = scan_text(&concatenated);
                if report.must_block() {
                    ExtractionOutput {
                        kind: Some(kind),
                        status: ExtractionStatus::Blocked,
                        error_class: Some(IngestionErrorClass::SecretBlocked),
                        error_detail: Some(secret_block_detail(&report)),
                        spans: Vec::new(),
                        spans_failed: 0,
                        redaction_count: report.redaction_count() as i32,
                        redaction_state: KnowledgeRedactionState::Redacted,
                    }
                } else {
                    ExtractionOutput {
                        kind: Some(kind),
                        status: if partial {
                            ExtractionStatus::Partial
                        } else {
                            ExtractionStatus::Success
                        },
                        error_class: if partial {
                            Some(IngestionErrorClass::NoTextLayer)
                        } else {
                            None
                        },
                        error_detail: Some(json!({
                            "failed_pages": extraction.failed_pages,
                            "report": extraction.report.detail_json(),
                        })),
                        spans_failed: extraction.failed_pages.len() as i32,
                        spans,
                        redaction_count: 0,
                        redaction_state: KnowledgeRedactionState::None,
                    }
                }
            }
            Err(err) => ExtractionOutput::failure(
                Some(kind),
                ExtractionStatus::Failed,
                err.class,
                json!({"detail": err.detail}),
            ),
        },
        IngestionSourceKind::RichDocument | IngestionSourceKind::ExternalImport => {
            // File-backed external imports register hash+provenance only
            // (capability span_extraction = false). Rich documents are
            // ingested from their authority table, not from files.
            ExtractionOutput {
                kind: Some(kind),
                status: ExtractionStatus::Success,
                error_class: None,
                error_detail: Some(json!({"reason": "registered without span extraction"})),
                spans: Vec::new(),
                spans_failed: 0,
                redaction_count: 0,
                redaction_state: KnowledgeRedactionState::None,
            }
        }
    };

    // MT-091 span-level redaction pass (MEDIUM severity rewrites content).
    // Whole-file findings drive redaction for byte-anchored spans (#1); kinds
    // without a whole-file scan (PDF page text, external imports) fall back to
    // a per-span scan inside redact_spans against this empty report.
    if spec.capabilities.secret_scan && !output.spans.is_empty() {
        let empty = SecretScanReport {
            findings: Vec::new(),
        };
        let report = whole_file_report.as_ref().unwrap_or(&empty);
        let redactions = redact_spans(&mut output.spans, report);
        if redactions > 0 {
            output.redaction_count += redactions;
            output.redaction_state = KnowledgeRedactionState::Partial;
        }
    }
    output
}

fn secret_block_detail(report: &SecretScanReport) -> Value {
    json!({
        "reason": "high-severity secret material detected; content not stored",
        "findings": report
            .findings
            .iter()
            .map(|f| json!({
                "kind": f.kind.as_str(),
                "line": f.line,
                "matched_len": f.matched_len,
            }))
            .collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// Filesystem walking + helpers.
// ---------------------------------------------------------------------------

/// Directories never walked (derived outputs / VCS internals).
const ALWAYS_SKIPPED_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__"];

struct FileAllowlist {
    include: globset::GlobSet,
    exclude: globset::GlobSet,
}

impl FileAllowlist {
    fn allows(&self, path: &str) -> bool {
        if self.exclude.is_match(path) {
            return false;
        }
        self.include.is_match(path)
    }
}

/// Compile the per-root FILE allowlist (`knowledge_source_roots.
/// allowlist_policy`, 0131 shape `{"include": [...], "exclude": [...]}`).
fn compile_file_allowlist(policy: &Value) -> IngestionResult<FileAllowlist> {
    fn set_from(value: Option<&Value>, default_all: bool) -> IngestionResult<globset::GlobSet> {
        let mut builder = globset::GlobSetBuilder::new();
        match value.and_then(|v| v.as_array()) {
            Some(items) => {
                for item in items {
                    let Some(pattern) = item.as_str() else {
                        return Err(IngestionError::Validation(
                            "file allowlist patterns must be strings".to_string(),
                        ));
                    };
                    let glob = globset::Glob::new(pattern).map_err(|err| {
                        IngestionError::Validation(format!(
                            "invalid file allowlist glob '{pattern}': {err}"
                        ))
                    })?;
                    builder.add(glob);
                }
            }
            None if default_all => {
                builder.add(globset::Glob::new("**/*").expect("static glob"));
            }
            None => {}
        }
        builder
            .build()
            .map_err(|err| IngestionError::Validation(format!("allowlist globset: {err}")))
    }
    Ok(FileAllowlist {
        include: set_from(policy.get("include"), true)?,
        exclude: set_from(policy.get("exclude"), false)?,
    })
}

/// Recursive walk collecting `(root-relative POSIX path, absolute path,
/// size)`. Symlinks are skipped (no cycle/escape risk); per-entry IO errors
/// are collected, never silently dropped.
fn walk_files(
    dir: &Path,
    base: &Path,
    out: &mut Vec<(String, PathBuf, u64)>,
    errors: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("{}: {err}", dir.display()));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                errors.push(format!("{}: {err}", dir.display()));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if ALWAYS_SKIPPED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk_files(&path, base, out, errors);
        } else if file_type.is_file() {
            let Ok(relative) = path.strip_prefix(base) else {
                errors.push(format!("{}: escaped the walk base", path.display()));
                continue;
            };
            let rel_posix = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let size = match entry.metadata() {
                Ok(metadata) => metadata.len(),
                Err(err) => {
                    errors.push(format!("{}: {err}", path.display()));
                    continue;
                }
            };
            out.push((rel_posix, path, size));
        }
    }
}

/// Streaming SHA-256 of a file (64 KiB chunks): the MT-092 oversize path
/// hashes without loading the file. Same algorithm/authority as
/// `hashing::compute_content_hashes().raw_sha256`.
fn stream_sha256(path: &Path) -> IngestionResult<String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|err| IngestionError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|err| IngestionError::Io {
            path: path.display().to_string(),
            detail: err.to_string(),
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn count_status(outcomes: &[FileIngestOutcome], status: ExtractionStatus) -> usize {
    outcomes
        .iter()
        .filter(|o| o.receipt.status == status)
        .count()
}

// ---------------------------------------------------------------------------
// Outcome types.
// ---------------------------------------------------------------------------

/// Durable result of one file ingestion attempt.
#[derive(Debug)]
pub struct FileIngestOutcome {
    pub source: KnowledgeSource,
    pub receipt: ExtractionReceipt,
    pub spans: Vec<StoredSpan>,
    pub repair: Option<RepairEntry>,
}

/// One source marked stale during MT-093 detection.
#[derive(Clone, Debug)]
pub struct StaleSourceMark {
    pub source_id: String,
    pub relative_path: String,
    /// `moved` (same content found at a new path) or `deleted`.
    pub disposition: String,
    pub moved_to: Option<String>,
    pub event_id: String,
}

/// Summary of one ingestion pass.
pub struct IngestionPassSummary {
    pub run_token: String,
    pub root_id: String,
    pub workspace_id: String,
    pub outcomes: Vec<FileIngestOutcome>,
    pub stale_marked: Vec<StaleSourceMark>,
    pub skipped_by_allowlist: usize,
    pub invalid_paths: Vec<String>,
    pub walk_errors: Vec<String>,
    pub start_event_id: String,
    pub finish_event_id: String,
}
