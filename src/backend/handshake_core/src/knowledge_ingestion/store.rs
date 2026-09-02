//! Embedded SurrealDB store for the ingestion-owned tables.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;

use super::allowlist::{PolicyVerdictKind, RootRegistrationPolicy};
use super::receipts::{ExtractionReceipt, NewExtractionReceipt};
use super::repair::{NewRepairEntry, RepairAttemptOutcome, RepairEntry, RepairState};
use super::spans::{ExtractedSpan, SpanAnchor, SpanRedaction};
use super::{new_ingestion_id, IngestionError, IngestionResult};
use crate::ai_ready_data::chunking::sha256_hex;
use crate::storage::surreal::{SurrealDatabase, SurrealStorageError};
use crate::storage::StorageError;

const WORKSPACES: &str = "workspaces";
const SOURCES: &str = "knowledge_sources";
const EVENTS: &str = "kernel_event_ledger";
const POLICIES: &str = "knowledge_ingestion_root_policies";
const RECEIPTS: &str = "knowledge_ingestion_receipts";
static REPAIR_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

fn thing(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key.to_owned())
}
fn opt_thing(table: &str, key: Option<&str>) -> Option<RecordId> {
    key.map(|key| thing(table, key))
}
fn key(record: RecordId) -> IngestionResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(IngestionError::Validation(
            "SurrealDB record link does not have a string key".to_owned(),
        )),
    }
}
fn opt_key(record: Option<RecordId>) -> IngestionResult<Option<String>> {
    record.map(key).transpose()
}
fn storage_error(error: SurrealStorageError) -> IngestionError {
    IngestionError::Storage(StorageError::Database(error.to_string()))
}

pub(super) fn validate_span_redaction(span: &ExtractedSpan) -> IngestionResult<()> {
    if span.redaction == SpanRedaction::Redacted && !span.content.contains("[REDACTED:") {
        return Err(IngestionError::Validation(
            "redacted span content must contain a [REDACTED:<kind>] marker".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub struct KnowledgeIngestionStore {
    db: Arc<SurrealDatabase>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRootPolicy {
    pub policy_id: String,
    pub workspace_id: String,
    pub policy_version: i32,
    pub policy: RootRegistrationPolicy,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecision {
    pub decision_id: String,
    pub workspace_id: String,
    pub policy_id: Option<String>,
    pub candidate_path: String,
    pub root_kind: String,
    pub verdict: PolicyVerdictKind,
    pub matched_pattern: Option<String>,
    pub operator_approved: bool,
    pub actor_kind: String,
    pub actor_id: String,
    pub receipt_event_id: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewPolicyDecision {
    pub workspace_id: String,
    pub policy_id: Option<String>,
    pub candidate_path: String,
    pub root_kind: String,
    pub verdict: PolicyVerdictKind,
    pub matched_pattern: Option<String>,
    pub operator_approved: bool,
    pub actor_kind: String,
    pub actor_id: String,
    pub receipt_event_id: Option<String>,
}

#[derive(SurrealValue)]
struct PolicyRecord {
    policy_id: String,
    workspace_id: RecordId,
    policy_version: i64,
    allow_patterns: Vec<String>,
    deny_patterns: Vec<String>,
    require_operator_approval: bool,
    active: bool,
    created_at: Datetime,
    updated_at: Datetime,
}
fn policy_from_record(row: PolicyRecord) -> IngestionResult<StoredRootPolicy> {
    Ok(StoredRootPolicy {
        policy_id: row.policy_id,
        workspace_id: key(row.workspace_id)?,
        policy_version: i32::try_from(row.policy_version).map_err(|_| {
            IngestionError::Validation("policy_version exceeds i32 range".to_owned())
        })?,
        policy: RootRegistrationPolicy {
            allow_patterns: row.allow_patterns,
            deny_patterns: row.deny_patterns,
            require_operator_approval: row.require_operator_approval,
        },
        active: row.active,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct DecisionRecord {
    decision_id: String,
    workspace_id: RecordId,
    policy_id: Option<RecordId>,
    candidate_path: String,
    root_kind: String,
    verdict: String,
    matched_pattern: Option<String>,
    operator_approved: bool,
    actor_kind: String,
    actor_id: String,
    receipt_event_id: Option<RecordId>,
    decided_at: Datetime,
}
fn decision_from_record(row: DecisionRecord) -> IngestionResult<PolicyDecision> {
    Ok(PolicyDecision {
        decision_id: row.decision_id,
        workspace_id: key(row.workspace_id)?,
        policy_id: opt_key(row.policy_id)?,
        candidate_path: row.candidate_path,
        root_kind: row.root_kind,
        verdict: row.verdict.parse()?,
        matched_pattern: row.matched_pattern,
        operator_approved: row.operator_approved,
        actor_kind: row.actor_kind,
        actor_id: row.actor_id,
        receipt_event_id: opt_key(row.receipt_event_id)?,
        decided_at: row.decided_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct ReceiptRecord {
    receipt_id: String,
    workspace_id: RecordId,
    source_id: RecordId,
    ingestion_run_token: Option<String>,
    extractor_id: String,
    extractor_version: String,
    status: String,
    error_class: Option<String>,
    error_detail: Option<Value>,
    spans_produced: i64,
    spans_failed: i64,
    redaction_count: i64,
    content_hash: String,
    duration_ms: i64,
    receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}
fn receipt_from_record(row: ReceiptRecord) -> IngestionResult<ExtractionReceipt> {
    let count = |value: i64, name: &str| {
        i32::try_from(value)
            .map_err(|_| IngestionError::Validation(format!("{name} exceeds i32 range")))
    };
    Ok(ExtractionReceipt {
        receipt_id: row.receipt_id,
        workspace_id: key(row.workspace_id)?,
        source_id: key(row.source_id)?,
        ingestion_run_token: row.ingestion_run_token,
        extractor_id: row.extractor_id,
        extractor_version: row.extractor_version,
        status: row.status.parse()?,
        error_class: row.error_class.map(|value| value.parse()).transpose()?,
        error_detail: row.error_detail,
        spans_produced: count(row.spans_produced, "spans_produced")?,
        spans_failed: count(row.spans_failed, "spans_failed")?,
        redaction_count: count(row.redaction_count, "redaction_count")?,
        content_hash: row.content_hash,
        duration_ms: row.duration_ms,
        receipt_event_id: opt_key(row.receipt_event_id)?,
        created_at: row.created_at.into_inner(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoredSpan {
    pub span_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub receipt_id: String,
    pub span_index: i32,
    pub anchor: SpanAnchor,
    pub byte_start: Option<i64>,
    pub byte_end: Option<i64>,
    pub content: String,
    pub content_hash: String,
    pub redaction_state: super::spans::SpanRedaction,
    pub link_candidates: Value,
    pub created_at: DateTime<Utc>,
}
#[derive(SurrealValue)]
struct SpanRecord {
    span_id: String,
    workspace_id: RecordId,
    source_id: RecordId,
    receipt_id: RecordId,
    span_index: i64,
    anchor: Value,
    byte_start: Option<i64>,
    byte_end: Option<i64>,
    content: String,
    content_hash: String,
    redaction_state: String,
    link_candidates: Value,
    created_at: Datetime,
}
fn span_from_record(row: SpanRecord) -> IngestionResult<StoredSpan> {
    Ok(StoredSpan {
        span_id: row.span_id,
        workspace_id: key(row.workspace_id)?,
        source_id: key(row.source_id)?,
        receipt_id: key(row.receipt_id)?,
        span_index: i32::try_from(row.span_index)
            .map_err(|_| IngestionError::Validation("span_index exceeds i32 range".to_owned()))?,
        anchor: SpanAnchor::from_json(&row.anchor)?,
        byte_start: row.byte_start,
        byte_end: row.byte_end,
        content: row.content,
        content_hash: row.content_hash,
        redaction_state: row.redaction_state.parse()?,
        link_candidates: row.link_candidates,
        created_at: row.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct RepairRecord {
    repair_id: String,
    workspace_id: RecordId,
    source_id: RecordId,
    receipt_id: Option<RecordId>,
    reason_class: String,
    reason_detail: Value,
    state: String,
    attempts: i64,
    max_attempts: i64,
    last_attempt_at: Option<Datetime>,
    resolved_receipt_id: Option<RecordId>,
    enqueue_event_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}
fn repair_from_record(row: RepairRecord) -> IngestionResult<RepairEntry> {
    Ok(RepairEntry {
        repair_id: row.repair_id,
        workspace_id: key(row.workspace_id)?,
        source_id: key(row.source_id)?,
        receipt_id: opt_key(row.receipt_id)?,
        reason_class: row.reason_class.parse()?,
        reason_detail: row.reason_detail,
        state: row.state.parse()?,
        attempts: i32::try_from(row.attempts)
            .map_err(|_| IngestionError::Validation("attempts exceeds i32 range".to_owned()))?,
        max_attempts: i32::try_from(row.max_attempts)
            .map_err(|_| IngestionError::Validation("max_attempts exceeds i32 range".to_owned()))?,
        last_attempt_at: row.last_attempt_at.map(Datetime::into_inner),
        resolved_receipt_id: opt_key(row.resolved_receipt_id)?,
        enqueue_event_id: opt_key(row.enqueue_event_id)?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

impl KnowledgeIngestionStore {
    pub fn new(db: Arc<SurrealDatabase>) -> Self {
        Self { db }
    }
    pub fn database(&self) -> &Arc<SurrealDatabase> {
        &self.db
    }

    async fn rows<R, B>(&self, statement: &'static str, bindings: B) -> IngestionResult<Vec<R>>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
            .map_err(storage_error)
    }
    async fn rows_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        index: usize,
    ) -> IngestionResult<Vec<R>>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values_at(statement, bindings, index).await })
            })
            .await
            .map_err(storage_error)
    }

    pub async fn activate_root_policy(
        &self,
        workspace_id: &str,
        policy: &RootRegistrationPolicy,
    ) -> IngestionResult<StoredRootPolicy> {
        #[derive(SurrealValue)]
        struct Bindings {
            policy_id: String,
            workspace: RecordId,
            allow: Vec<String>,
            deny: Vec<String>,
            approval: bool,
        }
        let rows: Vec<PolicyRecord> = self.rows_at(
            "BEGIN TRANSACTION; LET $previous = (SELECT VALUE policy_version FROM knowledge_ingestion_root_policies WHERE workspace_id = $workspace AND active = true LIMIT 1)[0]; UPDATE knowledge_ingestion_root_policies SET active = false, updated_at = time::now() WHERE workspace_id = $workspace AND active = true; CREATE type::record('knowledge_ingestion_root_policies', $policy_id) CONTENT { policy_id: $policy_id, workspace_id: $workspace, policy_version: IF $previous = NONE { 1 } ELSE { $previous + 1 }, allow_patterns: $allow, deny_patterns: $deny, require_operator_approval: $approval } RETURN AFTER; COMMIT TRANSACTION;",
            Bindings { policy_id: new_ingestion_id("KIP"), workspace: thing(WORKSPACES, workspace_id), allow: policy.allow_patterns.clone(), deny: policy.deny_patterns.clone(), approval: policy.require_operator_approval }, 3).await?;
        rows.into_iter()
            .next()
            .map(policy_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::NotFound(
                "activated ingestion root policy",
            )))
    }

    pub async fn get_active_root_policy(
        &self,
        workspace_id: &str,
    ) -> IngestionResult<Option<StoredRootPolicy>> {
        #[derive(SurrealValue)]
        struct Bindings {
            workspace: RecordId,
        }
        self.rows::<PolicyRecord, _>("SELECT * FROM knowledge_ingestion_root_policies WHERE workspace_id = $workspace AND active = true LIMIT 1;", Bindings { workspace: thing(WORKSPACES, workspace_id) }).await?.into_iter().next().map(policy_from_record).transpose()
    }

    pub async fn record_policy_decision(
        &self,
        decision: NewPolicyDecision,
    ) -> IngestionResult<PolicyDecision> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            workspace: RecordId,
            policy: Option<RecordId>,
            path: String,
            root_kind: String,
            verdict: String,
            matched: Option<String>,
            approved: bool,
            actor_kind: String,
            actor_id: String,
            event: Option<RecordId>,
        }
        let rows: Vec<DecisionRecord> = self.rows("CREATE type::record('knowledge_ingestion_policy_decisions', $id) CONTENT { decision_id: $id, workspace_id: $workspace, policy_id: $policy, candidate_path: $path, root_kind: $root_kind, verdict: $verdict, matched_pattern: $matched, operator_approved: $approved, actor_kind: $actor_kind, actor_id: $actor_id, receipt_event_id: $event } RETURN AFTER;",
            Bindings { id: new_ingestion_id("KIPD"), workspace: thing(WORKSPACES, &decision.workspace_id), policy: opt_thing(POLICIES, decision.policy_id.as_deref()), path: decision.candidate_path, root_kind: decision.root_kind, verdict: decision.verdict.as_str().to_owned(), matched: decision.matched_pattern, approved: decision.operator_approved, actor_kind: decision.actor_kind, actor_id: decision.actor_id, event: opt_thing(EVENTS, decision.receipt_event_id.as_deref()) }).await?;
        rows.into_iter()
            .next()
            .map(decision_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::NotFound(
                "ingestion policy decision",
            )))
    }

    pub async fn list_policy_decisions(
        &self,
        workspace_id: &str,
        limit: i64,
    ) -> IngestionResult<Vec<PolicyDecision>> {
        #[derive(SurrealValue)]
        struct Bindings {
            workspace: RecordId,
            limit: i64,
        }
        self.rows::<DecisionRecord, _>("SELECT * FROM knowledge_ingestion_policy_decisions WHERE workspace_id = $workspace ORDER BY decided_at DESC, decision_id DESC LIMIT $limit;", Bindings { workspace: thing(WORKSPACES, workspace_id), limit: limit.clamp(1, 10_000) }).await?.into_iter().map(decision_from_record).collect()
    }

    pub async fn record_extraction_receipt(
        &self,
        receipt: NewExtractionReceipt,
        receipt_event_id: Option<&str>,
    ) -> IngestionResult<ExtractionReceipt> {
        receipt.validate()?;
        let receipt_event_id = receipt_event_id.ok_or_else(|| {
            IngestionError::Validation(
                "extraction receipt requires a receipt EventLedger event".to_owned(),
            )
        })?;
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            workspace: RecordId,
            source: RecordId,
            run: Option<String>,
            extractor: String,
            version: String,
            status: String,
            error_class: Option<String>,
            error_detail: Option<Value>,
            produced: i32,
            failed: i32,
            redactions: i32,
            hash: String,
            duration: i64,
            event: Option<RecordId>,
        }
        let rows: Vec<ReceiptRecord> = self.rows("CREATE type::record('knowledge_ingestion_receipts', $id) CONTENT { receipt_id: $id, workspace_id: $workspace, source_id: $source, ingestion_run_token: $run, extractor_id: $extractor, extractor_version: $version, status: $status, error_class: $error_class, error_detail: $error_detail, spans_produced: $produced, spans_failed: $failed, redaction_count: $redactions, content_hash: $hash, duration_ms: $duration, receipt_event_id: $event } RETURN AFTER;",
            Bindings { id: new_ingestion_id("KIRC"), workspace: thing(WORKSPACES, &receipt.workspace_id), source: thing(SOURCES, &receipt.source_id), run: receipt.ingestion_run_token, extractor: receipt.extractor_id, version: receipt.extractor_version, status: receipt.status.as_str().to_owned(), error_class: receipt.error_class.map(|value| value.as_str().to_owned()), error_detail: receipt.error_detail, produced: receipt.spans_produced, failed: receipt.spans_failed, redactions: receipt.redaction_count, hash: receipt.content_hash, duration: receipt.duration_ms, event: Some(thing(EVENTS, receipt_event_id)) }).await?;
        rows.into_iter()
            .next()
            .map(receipt_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::NotFound(
                "extraction receipt",
            )))
    }

    pub async fn list_extraction_receipts(
        &self,
        source_id: &str,
        limit: i64,
    ) -> IngestionResult<Vec<ExtractionReceipt>> {
        #[derive(SurrealValue)]
        struct Bindings {
            source: RecordId,
            limit: i64,
        }
        self.rows::<ReceiptRecord, _>("SELECT * FROM knowledge_ingestion_receipts WHERE source_id = $source ORDER BY created_at DESC, receipt_id DESC LIMIT $limit;", Bindings { source: thing(SOURCES, source_id), limit: limit.clamp(1, 10_000) }).await?.into_iter().map(receipt_from_record).collect()
    }
    pub async fn get_extraction_receipt(
        &self,
        receipt_id: &str,
    ) -> IngestionResult<Option<ExtractionReceipt>> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
        }
        self.rows::<ReceiptRecord, _>(
            "SELECT * FROM knowledge_ingestion_receipts WHERE receipt_id = $id LIMIT 1;",
            Bindings {
                id: receipt_id.to_owned(),
            },
        )
        .await?
        .into_iter()
        .next()
        .map(receipt_from_record)
        .transpose()
    }

    pub async fn replace_source_spans(
        &self,
        workspace_id: &str,
        source_id: &str,
        receipt_id: &str,
        spans: &[ExtractedSpan],
    ) -> IngestionResult<Vec<StoredSpan>> {
        for span in spans {
            validate_span_redaction(span)?;
        }
        #[derive(SurrealValue)]
        struct SpanInput {
            span_id: String,
            workspace_id: RecordId,
            source_id: RecordId,
            receipt_id: RecordId,
            span_index: i64,
            anchor_kind: String,
            anchor: Value,
            byte_start: Option<i64>,
            byte_end: Option<i64>,
            content: String,
            content_hash: String,
            redaction_state: String,
            link_candidates: Value,
        }
        #[derive(SurrealValue)]
        struct Bindings {
            source: RecordId,
            spans: Vec<SpanInput>,
        }
        let source = thing(SOURCES, source_id);
        let inputs = spans
            .iter()
            .enumerate()
            .map(|(index, span)| SpanInput {
                span_id: new_ingestion_id("KISP"),
                workspace_id: thing(WORKSPACES, workspace_id),
                source_id: source.clone(),
                receipt_id: thing(RECEIPTS, receipt_id),
                span_index: index as i64,
                anchor_kind: span.anchor.kind_str().to_owned(),
                anchor: span.anchor.to_json(),
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                content: span.content.clone(),
                content_hash: sha256_hex(span.content.as_bytes()),
                redaction_state: span.redaction.as_str().to_owned(),
                link_candidates: serde_json::json!(span.link_candidates),
            })
            .collect();
        let rows: Vec<SpanRecord> = self.rows_at("BEGIN TRANSACTION; DELETE knowledge_ingestion_spans WHERE source_id = $source; FOR $span IN $spans { CREATE type::record('knowledge_ingestion_spans', $span.span_id) CONTENT $span; }; SELECT * FROM knowledge_ingestion_spans WHERE source_id = $source ORDER BY span_index; COMMIT TRANSACTION;", Bindings { source, spans: inputs }, 3).await?;
        rows.into_iter().map(span_from_record).collect()
    }
    pub async fn list_source_spans(&self, source_id: &str) -> IngestionResult<Vec<StoredSpan>> {
        #[derive(SurrealValue)]
        struct Bindings {
            source: RecordId,
        }
        self.rows::<SpanRecord, _>("SELECT * FROM knowledge_ingestion_spans WHERE source_id = $source ORDER BY span_index;", Bindings { source: thing(SOURCES, source_id) }).await?.into_iter().map(span_from_record).collect()
    }

    pub async fn enqueue_repair(&self, entry: NewRepairEntry) -> IngestionResult<RepairEntry> {
        if entry.enqueue_event_id.is_none() {
            return Err(IngestionError::Validation(
                "repair queue entry requires an enqueue EventLedger event".to_owned(),
            ));
        }
        let _guard = REPAIR_MUTATION_LOCK.lock().await;
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            workspace: RecordId,
            source: RecordId,
            receipt: Option<RecordId>,
            reason: String,
            detail: Value,
            max: i32,
            event: Option<RecordId>,
        }
        let rows: Vec<RepairRecord> = self.rows_at("BEGIN TRANSACTION; LET $open = (SELECT VALUE id FROM knowledge_ingestion_repair_queue WHERE source_id = $source AND state IN ['queued', 'retrying'] LIMIT 1)[0]; LET $dead = (SELECT VALUE id FROM knowledge_ingestion_repair_queue WHERE source_id = $source AND reason_class = $reason AND state = 'dead_letter' ORDER BY updated_at DESC, repair_id DESC LIMIT 1)[0]; IF $open != NONE { UPDATE $open SET reason_class = $reason, reason_detail = $detail, receipt_id = $receipt, enqueue_event_id = IF $event = NONE { enqueue_event_id } ELSE { $event }, updated_at = time::now() RETURN AFTER; } ELSE IF $dead != NONE { UPDATE $dead SET state = 'queued', attempts = 0, reason_detail = $detail, receipt_id = $receipt, enqueue_event_id = IF $event = NONE { enqueue_event_id } ELSE { $event }, resolved_receipt_id = NONE, updated_at = time::now() RETURN AFTER; } ELSE { CREATE type::record('knowledge_ingestion_repair_queue', $id) CONTENT { repair_id: $id, workspace_id: $workspace, source_id: $source, receipt_id: $receipt, reason_class: $reason, reason_detail: $detail, max_attempts: $max, enqueue_event_id: $event } RETURN AFTER; }; COMMIT TRANSACTION;",
            Bindings { id: new_ingestion_id("KIRQ"), workspace: thing(WORKSPACES, &entry.workspace_id), source: thing(SOURCES, &entry.source_id), receipt: opt_thing(RECEIPTS, entry.receipt_id.as_deref()), reason: entry.reason_class.as_str().to_owned(), detail: entry.reason_detail, max: entry.max_attempts, event: opt_thing(EVENTS, entry.enqueue_event_id.as_deref()) }, 3).await?;
        rows.into_iter()
            .next()
            .map(repair_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::NotFound(
                "repair queue mutation",
            )))
    }
    pub async fn get_repair_entry(&self, repair_id: &str) -> IngestionResult<Option<RepairEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
        }
        self.rows::<RepairRecord, _>(
            "SELECT * FROM knowledge_ingestion_repair_queue WHERE repair_id = $id LIMIT 1;",
            Bindings {
                id: repair_id.to_owned(),
            },
        )
        .await?
        .into_iter()
        .next()
        .map(repair_from_record)
        .transpose()
    }
    pub async fn list_repair_entries(
        &self,
        workspace_id: &str,
        state: Option<RepairState>,
        limit: i64,
    ) -> IngestionResult<Vec<RepairEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            workspace: RecordId,
            state: Option<String>,
            limit: i64,
        }
        self.rows::<RepairRecord, _>("SELECT * FROM knowledge_ingestion_repair_queue WHERE workspace_id = $workspace AND ($state = NONE OR state = $state) ORDER BY created_at DESC LIMIT $limit;", Bindings { workspace: thing(WORKSPACES, workspace_id), state: state.map(|value| value.as_str().to_owned()), limit: limit.clamp(1, 10_000) }).await?.into_iter().map(repair_from_record).collect()
    }
    pub async fn begin_repair_attempt(&self, repair_id: &str) -> IngestionResult<RepairEntry> {
        let _guard = REPAIR_MUTATION_LOCK.lock().await;
        let current = self
            .get_repair_entry(repair_id)
            .await?
            .ok_or(IngestionError::Storage(StorageError::NotFound(
                "repair entry",
            )))?;
        if current.state.is_terminal() {
            return Err(IngestionError::Storage(StorageError::Conflict(
                "repair entry is terminal; retries are over",
            )));
        }
        if current.attempts >= current.max_attempts {
            let _ = self.dead_letter_repair_unlocked(repair_id).await?;
            return Err(IngestionError::Storage(StorageError::Conflict(
                "repair attempts exhausted; entry dead-lettered",
            )));
        }
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            expected: i32,
        }
        let rows: Vec<RepairRecord> = self.rows("UPDATE knowledge_ingestion_repair_queue SET state = 'retrying', attempts += 1, last_attempt_at = time::now(), updated_at = time::now() WHERE repair_id = $id AND state IN ['queued', 'retrying'] AND attempts = $expected RETURN AFTER;", Bindings { id: repair_id.to_owned(), expected: current.attempts }).await?;
        rows.into_iter()
            .next()
            .map(repair_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::Conflict(
                "repair entry changed while claiming retry",
            )))
    }
    pub async fn settle_repair_attempt(
        &self,
        repair_id: &str,
        outcome: RepairAttemptOutcome,
    ) -> IngestionResult<RepairEntry> {
        let _guard = REPAIR_MUTATION_LOCK.lock().await;
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            receipt: Option<RecordId>,
            detail: Option<Value>,
        }
        let (statement, receipt, detail) = match outcome {
            RepairAttemptOutcome::Resolved { resolved_receipt_id } => ("UPDATE knowledge_ingestion_repair_queue SET state = 'resolved', resolved_receipt_id = $receipt, updated_at = time::now() WHERE repair_id = $id AND state = 'retrying' RETURN AFTER;", Some(thing(RECEIPTS, &resolved_receipt_id)), None),
            RepairAttemptOutcome::FailedAgain { receipt_id, reason_detail } => ("UPDATE knowledge_ingestion_repair_queue SET state = IF attempts >= max_attempts { 'dead_letter' } ELSE { 'queued' }, reason_detail = $detail, receipt_id = IF $receipt = NONE { receipt_id } ELSE { $receipt }, updated_at = time::now() WHERE repair_id = $id AND state = 'retrying' RETURN AFTER;", opt_thing(RECEIPTS, receipt_id.as_deref()), Some(reason_detail)),
        };
        let rows: Vec<RepairRecord> = self
            .rows(
                statement,
                Bindings {
                    id: repair_id.to_owned(),
                    receipt,
                    detail,
                },
            )
            .await?;
        rows.into_iter()
            .next()
            .map(repair_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::Conflict(
                "repair entry is not in a retrying state",
            )))
    }
    async fn dead_letter_repair_unlocked(&self, repair_id: &str) -> IngestionResult<RepairEntry> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
        }
        let rows: Vec<RepairRecord> = self.rows("UPDATE knowledge_ingestion_repair_queue SET state = 'dead_letter', updated_at = time::now() WHERE repair_id = $id AND state IN ['queued', 'retrying'] RETURN AFTER;", Bindings { id: repair_id.to_owned() }).await?;
        rows.into_iter()
            .next()
            .map(repair_from_record)
            .transpose()?
            .ok_or(IngestionError::Storage(StorageError::Conflict(
                "repair entry is not open; cannot dead-letter",
            )))
    }
    pub async fn dead_letter_repair(&self, repair_id: &str) -> IngestionResult<RepairEntry> {
        let _guard = REPAIR_MUTATION_LOCK.lock().await;
        self.dead_letter_repair_unlocked(repair_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use crate::knowledge_ingestion::receipts::ExtractionStatus;
    use crate::knowledge_ingestion::spans::SpanAnchor;
    use crate::storage::knowledge::{
        KnowledgeIndexingEligibility, KnowledgePermissionScope, KnowledgeRedactionState,
        KnowledgeRootKind, KnowledgeSourceKind, KnowledgeStore, NewKnowledgeSource,
        NewKnowledgeSourceRoot,
    };
    use crate::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};
    use crate::storage::{Database, NewWorkspace, WriteContext};
    use serde_json::json;

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid ingestion test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap ingestion schema");
        storage
    }

    #[tokio::test]
    async fn ingestion_rows_survive_reopen_and_span_replace_rolls_back() {
        let directory = tempfile::tempdir().expect("temporary ingestion store root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let db = Arc::new(SurrealDatabase::new(storage.clone()));
        let workspace = db
            .create_workspace(
                &WriteContext::human(None),
                NewWorkspace {
                    name: "Ingestion durability".to_owned(),
                },
            )
            .await
            .expect("create workspace");
        let root = db
            .create_knowledge_source_root(NewKnowledgeSourceRoot {
                workspace_id: workspace.id.clone(),
                display_name: "Repo".to_owned(),
                root_kind: KnowledgeRootKind::ProjectRepo,
                repo_relative_path: String::new(),
                allowlist_policy: json!({"include":["**/*"],"exclude":[]}),
                indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
            })
            .await
            .expect("create source root");
        let source = db
            .upsert_knowledge_source(NewKnowledgeSource {
                workspace_id: workspace.id.clone(),
                root_id: Some(root.root_id),
                source_kind: KnowledgeSourceKind::File,
                relative_path: Some("src/lib.rs".to_owned()),
                asset_id: None,
                loom_block_id: None,
                document_id: None,
                content_hash: "a".repeat(64),
                size_bytes: Some(12),
                provenance: json!({"test":"mt-137"}),
                permission_scope: KnowledgePermissionScope::Workspace,
                redaction_state: KnowledgeRedactionState::None,
                source_modified_at: None,
            })
            .await
            .expect("create source");
        let store = KnowledgeIngestionStore::new(Arc::clone(&db));
        let policy = store
            .activate_root_policy(
                &workspace.id,
                &RootRegistrationPolicy {
                    allow_patterns: vec!["**/*".to_owned()],
                    deny_patterns: vec!["target/**".to_owned()],
                    require_operator_approval: false,
                },
            )
            .await
            .expect("activate policy");
        let receipt_event = db
            .append_kernel_event(
                NewKernelEvent::builder(
                    "ingestion-store-test-task",
                    "ingestion-store-test-session",
                    KernelEventType::ValidationRecorded,
                    KernelActor::System("ingestion-store-test".to_owned()),
                )
                .aggregate("knowledge_source", source.source_id.clone())
                .source_component("knowledge_ingestion_store_test")
                .payload(json!({"kind":"extraction_receipt"}))
                .build()
                .expect("build receipt event"),
            )
            .await
            .expect("record receipt event");
        let receipt = store
            .record_extraction_receipt(
                NewExtractionReceipt {
                    workspace_id: workspace.id.clone(),
                    source_id: source.source_id.clone(),
                    ingestion_run_token: Some("run-reopen".to_owned()),
                    extractor_id: "rust-test".to_owned(),
                    extractor_version: "1".to_owned(),
                    status: ExtractionStatus::Success,
                    error_class: None,
                    error_detail: None,
                    spans_produced: 1,
                    spans_failed: 0,
                    redaction_count: 0,
                    content_hash: "a".repeat(64),
                    duration_ms: 1,
                },
                Some(&receipt_event.event_id),
            )
            .await
            .expect("record receipt");
        let original = ExtractedSpan::new(
            SpanAnchor::LineRange {
                line_start: 1,
                line_end: 1,
                heading_path: Vec::new(),
            },
            "durable span",
        )
        .with_bytes(0, 12);
        store
            .replace_source_spans(
                &workspace.id,
                &source.source_id,
                &receipt.receipt_id,
                &[original],
            )
            .await
            .expect("store original span");

        let invalid = ExtractedSpan::new(
            SpanAnchor::LineRange {
                line_start: 1,
                line_end: 1,
                heading_path: Vec::new(),
            },
            "must roll back",
        )
        .with_bytes(-1, 1);
        assert!(store
            .replace_source_spans(
                &workspace.id,
                &source.source_id,
                &receipt.receipt_id,
                &[invalid],
            )
            .await
            .is_err());
        let after_failure = store
            .list_source_spans(&source.source_id)
            .await
            .expect("read spans after failed replacement");
        assert_eq!(after_failure.len(), 1);
        assert_eq!(after_failure[0].content, "durable span");

        storage.shutdown().await.expect("close ingestion store");
        drop(store);
        drop(db);
        drop(storage);

        let reopened = open(&path).await;
        let reopened_store =
            KnowledgeIngestionStore::new(Arc::new(SurrealDatabase::new(reopened.clone())));
        let persisted_policy = reopened_store
            .get_active_root_policy(&workspace.id)
            .await
            .expect("read reopened policy")
            .expect("durable policy");
        assert_eq!(persisted_policy.policy_id, policy.policy_id);
        let persisted_receipt = reopened_store
            .get_extraction_receipt(&receipt.receipt_id)
            .await
            .expect("read reopened receipt")
            .expect("durable receipt");
        assert_eq!(persisted_receipt.source_id, source.source_id);
        let persisted_spans = reopened_store
            .list_source_spans(&source.source_id)
            .await
            .expect("read reopened spans");
        assert_eq!(persisted_spans.len(), 1);
        assert_eq!(persisted_spans[0].content, "durable span");
        reopened.shutdown().await.expect("close reopened store");
    }
}
