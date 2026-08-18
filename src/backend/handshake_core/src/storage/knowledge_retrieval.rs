//! WP-KERNEL-009 MT-138 RetrievalTraceModel — the storage-facing persistence of
//! a [`QueryPlan`] + [`RetrievalTrace`] into the committed
//! `knowledge_retrieval_traces` table (migration 0141, MT-060).
//!
//! This file is the RetrievalContextAndRanking group's OWN storage surface. It
//! does NOT edit `storage/knowledge.rs` or `storage/knowledge_memory.rs`; it
//! reuses their public `KnowledgeStore` API. Specifically it wraps
//! [`KnowledgeStore::record_knowledge_retrieval_trace`] so the planner/compiler
//! product logic can persist a fully replayable trace without re-deriving the
//! `mode_reason` / `decisions` projection at every call site.
//!
//! Authority (spec 2.3.13.11): the `knowledge_retrieval_traces` row is the
//! durable authority. `retrieval_mode` is one of the five spec strings,
//! `mode_reason` records why broader retrieval was used or skipped (a DB CHECK
//! rejects empty), and `decisions` carries the full replayable QueryPlan +
//! RetrievalTrace JSON ([`RetrievalTrace::to_decisions_json`]). EventLedger
//! linkage is via `trace_receipt_event_id`.

use std::collections::{BTreeMap, BTreeSet};

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::surreal::{SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_retrieval::plan::{QueryPlan, RetrievalTrace};
use crate::storage::knowledge::{
    KnowledgeBundleItemDecision, KnowledgeBundleItemRefKind, KnowledgeContextBundle,
    KnowledgeContextBundleItem, KnowledgePassageEvidenceRef, KnowledgeRetrievalMode,
    KnowledgeRetrievalTrace, KnowledgeStore, NewKnowledgeRetrievalTrace,
};
use crate::storage::{Database, StorageError, StorageResult};

const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";
const WORKSPACES_TABLE: &str = "workspaces";

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

/// The plain id behind a record link.
///
/// Foreign keys in the embedded store are `record<table>` links, not opaque
/// strings, so the string form every public struct here exposes has to be
/// recovered from the key rather than from `Display` (which renders SurrealQL
/// and would quote the value).
fn record_key(record_id: RecordId, reason: &'static str) -> StorageResult<String> {
    let RecordIdKey::String(key) = record_id.key else {
        return Err(StorageError::Conflict(reason));
    };
    Ok(key)
}

fn optional_record_key(
    record_id: Option<RecordId>,
    reason: &'static str,
) -> StorageResult<Option<String>> {
    record_id.map(|id| record_key(id, reason)).transpose()
}

/// Reconstructed bundle replay: the persisted bundle and item rows, the
/// original typed QueryPlan/RetrievalTrace from decisions JSONB, resolved
/// evidence anchors, and EventLedger receipts used by the run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayedContextBundle {
    pub bundle: KnowledgeContextBundle,
    pub items: Vec<KnowledgeContextBundleItem>,
    pub query_plan: QueryPlan,
    pub retrieval_trace: RetrievalTrace,
    pub evidence: Vec<ReplayedBundleEvidence>,
    pub receipt_events: Vec<ReplayReceiptEvent>,
}

/// Evidence anchor recovered from committed source/span rows for one bundle
/// item. Unsupported, excluded, or non-span items may lack a span/source pair,
/// but supported included span items must resolve fully or replay fails closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayedBundleEvidence {
    pub ref_kind: KnowledgeBundleItemRefKind,
    pub ref_id: String,
    pub retrieval_decision: KnowledgeBundleItemDecision,
    pub span_id: Option<String>,
    pub source_id: Option<String>,
    pub source_path: Option<String>,
    pub content_sha256: Option<String>,
    pub citation: Option<String>,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
}

/// Minimal EventLedger receipt row needed to prove the replay was backed by
/// committed ledger events without widening the global Database trait.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayReceiptEvent {
    pub event_id: String,
    pub event_type: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload_hash: String,
}

/// Map the planner's [`crate::memory::retrieval_mode::QueryRetrievalMode`] onto
/// the durable storage enum (the five spec strings). Planner postures
/// (`PassageFallback`, `Blocked`) collapse onto `hybrid_rag` / `none`
/// respectively via the mode's `to_storage_str`, parsed back into the storage
/// enum so the column constraint is always satisfied.
pub fn storage_mode_of(plan: &QueryPlan) -> StorageResult<KnowledgeRetrievalMode> {
    plan.retrieval_mode
        .to_storage_str()
        .parse::<KnowledgeRetrievalMode>()
}

/// Persist a QueryPlan + RetrievalTrace as a durable, replayable
/// `knowledge_retrieval_traces` row. The `mode_reason` and `decisions` are
/// derived from the trace so a caller cannot accidentally persist an empty
/// reason (which the DB CHECK would reject anyway).
///
/// * `bundle_id` — the context bundle this trace produced, when one was built
///   (the FK is `ON DELETE SET NULL`).
/// * `trace_receipt_event_id` — the EventLedger receipt for this retrieval.
pub async fn record_retrieval_trace(
    store: &dyn KnowledgeStore,
    ledger: &dyn Database,
    storage: &SurrealStorage,
    workspace_id: &str,
    plan: &QueryPlan,
    trace: &RetrievalTrace,
    bundle_id: Option<String>,
    trace_receipt_event_id: Option<String>,
) -> StorageResult<KnowledgeRetrievalTrace> {
    // Defensive: a non-hybrid mode without a reason is a planner bug; refuse to
    // persist a misleading trace (mirrors the spec A0.5 invariant).
    plan.validate().map_err(StorageError::Validation)?;

    let new = NewKnowledgeRetrievalTrace {
        workspace_id: workspace_id.to_string(),
        retrieval_mode: storage_mode_of(plan)?,
        mode_reason: trace.mode_reason(),
        query_text: Some(plan.query_text.clone()),
        bundle_id,
        decisions: trace.to_decisions_json(plan),
        trace_receipt_event_id,
    };
    let stored_trace = store.record_knowledge_retrieval_trace(new).await?;
    if stored_trace.trace_receipt_event_id.is_some() {
        return Ok(stored_trace);
    }

    let Some(bundle_id) = stored_trace.bundle_id.as_deref() else {
        return Ok(stored_trace);
    };
    let (bundle, _) = store
        .get_knowledge_context_bundle(bundle_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge context bundle"))?;
    let event = NewKernelEvent::builder(
        &bundle.kernel_task_run_id,
        &bundle.session_run_id,
        KernelEventType::KnowledgeRetrievalTraceRecorded,
        KernelActor::System("knowledge-retrieval".to_string()),
    )
    .aggregate("knowledge_retrieval_trace", stored_trace.trace_id.clone())
    .idempotency_key(format!(
        "knowledge-retrieval-trace:{}",
        stored_trace.trace_id
    ))
    .payload(serde_json::json!({
        "trace_id": stored_trace.trace_id.clone(),
        "bundle_id": bundle.bundle_id,
        "query_plan_id": plan.plan_id,
        "retrieval_mode": plan.retrieval_mode.to_storage_str(),
    }))
    .build()
    .map_err(|_| StorageError::Validation("knowledge retrieval trace receipt event is invalid"))?;
    let receipt = ledger.append_kernel_event(event).await?;
    // `RETURN AFTER` is how the embedded store reports affected rows: a trace id
    // that no longer resolves yields zero returned rows instead of silently
    // succeeding, which is the same lost-row signal the relational UPDATE gave.
    let bindings = BindTraceReceipt {
        receipt: RecordId::new(KERNEL_EVENT_LEDGER_TABLE, receipt.event_id.as_str()),
        trace_id: stored_trace.trace_id.clone(),
    };
    let affected = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE knowledge_retrieval_traces SET trace_receipt_event_id = $receipt \
                         WHERE trace_id = $trace_id RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    if affected != 1 {
        return Err(StorageError::NotFound("knowledge retrieval trace"));
    }

    Ok(KnowledgeRetrievalTrace {
        trace_receipt_event_id: Some(receipt.event_id),
        ..stored_trace
    })
}

#[derive(SurrealValue)]
struct BindTraceReceipt {
    receipt: RecordId,
    trace_id: String,
}

#[derive(SurrealValue)]
struct BindTraceId {
    trace_id: String,
}

#[derive(SurrealValue)]
struct BindEventId {
    event_id: String,
}

/// The stored `knowledge_retrieval_traces` projection.
///
/// `workspace_id`, `bundle_id` and `trace_receipt_event_id` are record links in
/// the embedded store (`ASSERT record::exists`), so the foreign-key guarantee
/// PostgreSQL enforced is still enforced here; the public
/// [`KnowledgeRetrievalTrace`] keeps its string shape and is rebuilt from the
/// link keys.
#[derive(SurrealValue)]
struct RetrievalTraceRecord {
    trace_id: String,
    workspace_id: RecordId,
    retrieval_mode: String,
    mode_reason: String,
    query_text: Option<String>,
    bundle_id: Option<RecordId>,
    decisions: Value,
    trace_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}

impl RetrievalTraceRecord {
    fn into_trace(self) -> StorageResult<KnowledgeRetrievalTrace> {
        Ok(KnowledgeRetrievalTrace {
            trace_id: self.trace_id,
            workspace_id: record_key(
                self.workspace_id,
                "knowledge retrieval trace workspace link is not a string key",
            )?,
            retrieval_mode: self.retrieval_mode.parse()?,
            mode_reason: self.mode_reason,
            query_text: self.query_text,
            bundle_id: optional_record_key(
                self.bundle_id,
                "knowledge retrieval trace bundle link is not a string key",
            )?,
            decisions: self.decisions,
            trace_receipt_event_id: optional_record_key(
                self.trace_receipt_event_id,
                "knowledge retrieval trace receipt link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
        })
    }
}

/// Load every trace bound to a bundle (replay entry point for the debug API).
/// Thin pass-through to the committed store so the retrieval group has one
/// import surface for its reads.
pub async fn traces_for_bundle(
    store: &dyn KnowledgeStore,
    bundle_id: &str,
) -> StorageResult<Vec<KnowledgeRetrievalTrace>> {
    store
        .list_knowledge_retrieval_traces_for_bundle(bundle_id)
        .await
}

/// Reconstruct a persisted context bundle from a trace id, proving the durable
/// row is replayable from durable authority rather than only display JSON.
pub async fn replay_context_bundle_from_trace(
    store: &dyn KnowledgeStore,
    storage: &SurrealStorage,
    trace_id: &str,
) -> StorageResult<ReplayedContextBundle> {
    if trace_id.trim() != trace_id || trace_id.is_empty() {
        return Err(StorageError::Validation(
            "knowledge retrieval trace_id must be non-empty and trimmed",
        ));
    }

    let trace = get_trace_by_id(storage, trace_id).await?;
    let bundle_id = trace.bundle_id.as_deref().ok_or(StorageError::Validation(
        "knowledge retrieval trace is not bound to a context bundle",
    ))?;
    let (bundle, items) = store
        .get_knowledge_context_bundle(bundle_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge context bundle"))?;
    if trace.workspace_id != bundle.workspace_id {
        return Err(StorageError::Validation(
            "knowledge retrieval trace workspace does not match context bundle workspace",
        ));
    }

    let query_plan: QueryPlan =
        serde_json::from_value(trace.decisions.get("query_plan").cloned().ok_or(
            StorageError::Validation("knowledge retrieval trace decisions missing query_plan"),
        )?)
        .map_err(|_| StorageError::Validation("knowledge retrieval trace query_plan is invalid"))?;
    let retrieval_trace: RetrievalTrace =
        serde_json::from_value(trace.decisions.get("retrieval_trace").cloned().ok_or(
            StorageError::Validation("knowledge retrieval trace decisions missing retrieval_trace"),
        )?)
        .map_err(|_| {
            StorageError::Validation("knowledge retrieval trace retrieval_trace is invalid")
        })?;
    query_plan.validate().map_err(StorageError::Validation)?;

    if retrieval_trace.query_plan_id != query_plan.plan_id {
        return Err(StorageError::Validation(
            "knowledge retrieval trace query_plan_id does not match query_plan.plan_id",
        ));
    }
    if bundle.allowed_context["query_plan_id"].as_str() != Some(query_plan.plan_id.as_str()) {
        return Err(StorageError::Validation(
            "context bundle query_plan_id does not match retrieval trace query_plan",
        ));
    }

    let candidates_by_id: BTreeMap<&str, &str> = retrieval_trace
        .candidates
        .iter()
        .map(|candidate| (candidate.candidate_id.as_str(), candidate.kind.as_str()))
        .collect();
    let mut selected_ids = BTreeSet::new();
    for selected in &retrieval_trace.selected {
        if !selected_ids.insert(selected.candidate_id.as_str()) {
            return Err(StorageError::Validation(
                "retrieval trace selected evidence contains duplicate candidate ids",
            ));
        }
        let item = items
            .iter()
            .find(|item| item.ref_id == selected.candidate_id)
            .ok_or(StorageError::Validation(
                "retrieval trace selected evidence is missing from context bundle items",
            ))?;
        if item.retrieval_decision != KnowledgeBundleItemDecision::Included {
            return Err(StorageError::Validation(
                "retrieval trace selected evidence was not included in context bundle",
            ));
        }
        let candidate_kind = candidates_by_id.get(selected.candidate_id.as_str()).ok_or(
            StorageError::Validation(
                "retrieval trace selected evidence is missing its ranked candidate",
            ),
        )?;
        let Some(candidate_ref_kind) = bundle_ref_kind_from_candidate_kind(candidate_kind) else {
            return Err(StorageError::Validation(
                "retrieval trace selected evidence has unknown candidate kind",
            ));
        };
        if candidate_ref_kind != item.ref_kind {
            return Err(StorageError::Validation(
                "retrieval trace selected evidence kind does not match context bundle item",
            ));
        }
    }
    let included_item_ids: BTreeSet<&str> = items
        .iter()
        .filter(|item| item.retrieval_decision == KnowledgeBundleItemDecision::Included)
        .map(|item| item.ref_id.as_str())
        .collect();
    if selected_ids != included_item_ids {
        return Err(StorageError::Validation(
            "retrieval trace selected evidence does not match included context bundle items",
        ));
    }

    let evidence = replay_evidence_for_items(store, &bundle.workspace_id, &items).await?;
    let mut receipt_events = Vec::new();
    let build_receipt_event_id =
        bundle
            .build_receipt_event_id
            .as_deref()
            .ok_or(StorageError::Validation(
                "context bundle build_receipt_event_id is required for replay",
            ))?;
    receipt_events.push(
        get_receipt_event(
            storage,
            build_receipt_event_id,
            KernelEventType::ContextBundleRecorded,
            "context_bundle",
            &bundle.bundle_id,
            &bundle.kernel_task_run_id,
            &bundle.session_run_id,
        )
        .await?,
    );
    let trace_receipt_event_id =
        trace
            .trace_receipt_event_id
            .as_deref()
            .ok_or(StorageError::Validation(
                "knowledge retrieval trace trace_receipt_event_id is required for replay",
            ))?;
    receipt_events.push(
        get_receipt_event(
            storage,
            trace_receipt_event_id,
            KernelEventType::KnowledgeRetrievalTraceRecorded,
            "knowledge_retrieval_trace",
            &trace.trace_id,
            &bundle.kernel_task_run_id,
            &bundle.session_run_id,
        )
        .await?,
    );

    Ok(ReplayedContextBundle {
        bundle,
        items,
        query_plan,
        retrieval_trace,
        evidence,
        receipt_events,
    })
}

async fn get_trace_by_id(
    storage: &SurrealStorage,
    trace_id: &str,
) -> StorageResult<KnowledgeRetrievalTrace> {
    let bindings = BindTraceId {
        trace_id: trace_id.to_owned(),
    };
    let record: Option<RetrievalTraceRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT trace_id, workspace_id, retrieval_mode, mode_reason, query_text, \
                         bundle_id, decisions, trace_receipt_event_id, created_at \
                         FROM knowledge_retrieval_traces WHERE trace_id = $trace_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record
        .ok_or(StorageError::NotFound("knowledge retrieval trace"))?
        .into_trace()
}

async fn replay_evidence_for_items(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    items: &[KnowledgeContextBundleItem],
) -> StorageResult<Vec<ReplayedBundleEvidence>> {
    let mut evidence = Vec::with_capacity(items.len());
    for item in items {
        if !item.supported || item.retrieval_decision != KnowledgeBundleItemDecision::Included {
            evidence.push(ReplayedBundleEvidence {
                ref_kind: item.ref_kind,
                ref_id: item.ref_id.clone(),
                retrieval_decision: item.retrieval_decision,
                span_id: None,
                source_id: None,
                source_path: None,
                content_sha256: None,
                citation: item.citation.clone(),
                supported: item.supported,
                unsupported_reason: item.unsupported_reason.clone(),
            });
            continue;
        }

        evidence.push(match item.ref_kind {
            KnowledgeBundleItemRefKind::Span => {
                replay_span_evidence(db, bundle_workspace_id, item, &item.ref_id).await?
            }
            KnowledgeBundleItemRefKind::Source => {
                replay_source_evidence(db, bundle_workspace_id, item, &item.ref_id).await?
            }
            KnowledgeBundleItemRefKind::Passage => {
                replay_passage_evidence(db, bundle_workspace_id, item).await?
            }
            KnowledgeBundleItemRefKind::Entity => {
                replay_entity_evidence(db, bundle_workspace_id, item).await?
            }
            KnowledgeBundleItemRefKind::Claim => {
                replay_claim_evidence(db, bundle_workspace_id, item).await?
            }
        });
    }
    Ok(evidence)
}

async fn replay_span_evidence(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
    span_id: &str,
) -> StorageResult<ReplayedBundleEvidence> {
    let span = db
        .get_knowledge_span(span_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge span"))?;
    let source = db
        .get_knowledge_source(&span.source_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge source"))?;
    if source.workspace_id != bundle_workspace_id {
        return Err(StorageError::Validation(
            "knowledge span source workspace does not match context bundle workspace",
        ));
    }
    Ok(ReplayedBundleEvidence {
        ref_kind: item.ref_kind,
        ref_id: item.ref_id.clone(),
        retrieval_decision: item.retrieval_decision,
        span_id: Some(span.span_id),
        source_id: Some(source.source_id),
        source_path: source.relative_path,
        content_sha256: Some(span.content_sha256),
        citation: item.citation.clone(),
        supported: item.supported,
        unsupported_reason: item.unsupported_reason.clone(),
    })
}

async fn replay_source_evidence(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
    source_id: &str,
) -> StorageResult<ReplayedBundleEvidence> {
    let source = db
        .get_knowledge_source(source_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge source"))?;
    if source.workspace_id != bundle_workspace_id {
        return Err(StorageError::Validation(
            "knowledge source workspace does not match context bundle workspace",
        ));
    }
    Ok(ReplayedBundleEvidence {
        ref_kind: item.ref_kind,
        ref_id: item.ref_id.clone(),
        retrieval_decision: item.retrieval_decision,
        span_id: None,
        source_id: Some(source.source_id),
        source_path: source.relative_path,
        content_sha256: Some(source.content_hash),
        citation: item.citation.clone(),
        supported: item.supported,
        unsupported_reason: item.unsupported_reason.clone(),
    })
}

async fn replay_passage_evidence(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
) -> StorageResult<ReplayedBundleEvidence> {
    let passage = db
        .get_knowledge_memory_passage(&item.ref_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge memory passage"))?;
    if passage.workspace_id != bundle_workspace_id {
        return Err(StorageError::Validation(
            "knowledge passage workspace does not match context bundle workspace",
        ));
    }
    let evidence_refs = db.list_knowledge_passage_evidence(&item.ref_id).await?;
    replay_first_lineage_ref(db, bundle_workspace_id, item, evidence_refs).await
}

async fn replay_entity_evidence(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
) -> StorageResult<ReplayedBundleEvidence> {
    if let Some(edge) = db
        .get_knowledge_edge_by_relationship_id(bundle_workspace_id, &item.ref_id)
        .await?
    {
        let span_ids = db.list_knowledge_edge_span_ids(&edge.edge_id).await?;
        if let Some(span_id) = span_ids.first() {
            return replay_span_evidence(db, bundle_workspace_id, item, span_id).await;
        }
        return Err(StorageError::Validation(
            "supported graph edge context bundle item lacks replayable evidence",
        ));
    }

    let entity = db
        .get_knowledge_entity(&item.ref_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge entity"))?;
    if entity.workspace_id != bundle_workspace_id {
        return Err(StorageError::Validation(
            "knowledge entity workspace does not match context bundle workspace",
        ));
    }
    let span_ids = db.list_knowledge_entity_span_ids(&item.ref_id).await?;
    if let Some(span_id) = span_ids.first() {
        return replay_span_evidence(db, bundle_workspace_id, item, span_id).await;
    }
    if let Some(source_id) = entity.primary_source_id.as_deref() {
        return replay_source_evidence(db, bundle_workspace_id, item, source_id).await;
    }
    Err(StorageError::Validation(
        "supported entity context bundle item lacks replayable evidence",
    ))
}

async fn replay_claim_evidence(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
) -> StorageResult<ReplayedBundleEvidence> {
    let claim = db
        .get_knowledge_claim(&item.ref_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge claim"))?;
    if claim.workspace_id != bundle_workspace_id {
        return Err(StorageError::Validation(
            "knowledge claim workspace does not match context bundle workspace",
        ));
    }
    let span_ids = db.list_knowledge_claim_span_ids(&item.ref_id).await?;
    if let Some(span_id) = span_ids.first() {
        return replay_span_evidence(db, bundle_workspace_id, item, span_id).await;
    }
    Err(StorageError::Validation(
        "supported claim context bundle item lacks replayable evidence",
    ))
}

async fn replay_first_lineage_ref(
    db: &dyn KnowledgeStore,
    bundle_workspace_id: &str,
    item: &KnowledgeContextBundleItem,
    evidence_refs: Vec<KnowledgePassageEvidenceRef>,
) -> StorageResult<ReplayedBundleEvidence> {
    let mut first_source_id: Option<String> = None;
    for evidence_ref in evidence_refs {
        match evidence_ref {
            KnowledgePassageEvidenceRef::Span { span_id } => {
                return replay_span_evidence(db, bundle_workspace_id, item, &span_id).await;
            }
            KnowledgePassageEvidenceRef::Source { source_id } => {
                if first_source_id.is_none() {
                    first_source_id = Some(source_id);
                }
            }
            KnowledgePassageEvidenceRef::Claim { claim_id } => {
                let span_ids = db.list_knowledge_claim_span_ids(&claim_id).await?;
                if let Some(span_id) = span_ids.first() {
                    return replay_span_evidence(db, bundle_workspace_id, item, span_id).await;
                }
            }
        }
    }
    if let Some(source_id) = first_source_id {
        return replay_source_evidence(db, bundle_workspace_id, item, &source_id).await;
    }
    Err(StorageError::Validation(
        "supported passage context bundle item lacks replayable evidence",
    ))
}

fn bundle_ref_kind_from_candidate_kind(kind: &str) -> Option<KnowledgeBundleItemRefKind> {
    match kind {
        "source_ref" => Some(KnowledgeBundleItemRefKind::Source),
        "span_ref" => Some(KnowledgeBundleItemRefKind::Span),
        "claim_ref" => Some(KnowledgeBundleItemRefKind::Claim),
        "passage_ref" => Some(KnowledgeBundleItemRefKind::Passage),
        "entity_ref" => Some(KnowledgeBundleItemRefKind::Entity),
        _ => None,
    }
}

#[derive(SurrealValue)]
struct ReceiptEventRecord {
    event_id: String,
    event_type: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    payload_hash: String,
}

async fn get_receipt_event(
    storage: &SurrealStorage,
    event_id: &str,
    expected_event_type: KernelEventType,
    expected_aggregate_type: &str,
    expected_aggregate_id: &str,
    expected_kernel_task_run_id: &str,
    expected_session_run_id: &str,
) -> StorageResult<ReplayReceiptEvent> {
    let bindings = BindEventId {
        event_id: event_id.to_owned(),
    };
    let row: Option<ReceiptEventRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, event_type, kernel_task_run_id, session_run_id, \
                         aggregate_type, aggregate_id, payload_hash FROM kernel_event_ledger \
                         WHERE event_id = $event_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let row = row.ok_or(StorageError::NotFound("kernel event receipt"))?;

    let event_type: String = row.event_type;
    let kernel_task_run_id: String = row.kernel_task_run_id;
    let session_run_id: String = row.session_run_id;
    let aggregate_type: String = row.aggregate_type;
    let aggregate_id: String = row.aggregate_id;
    if event_type != expected_event_type.as_str() {
        return Err(StorageError::Validation(
            "kernel event receipt event_type does not match replay expectation",
        ));
    }
    if kernel_task_run_id != expected_kernel_task_run_id
        || session_run_id != expected_session_run_id
    {
        return Err(StorageError::Validation(
            "kernel event receipt run context does not match replay expectation",
        ));
    }
    if aggregate_type != expected_aggregate_type {
        return Err(StorageError::Validation(
            "kernel event receipt aggregate_type does not match replay expectation",
        ));
    }
    if aggregate_id != expected_aggregate_id {
        return Err(StorageError::Validation(
            "kernel event receipt aggregate_id does not match replay expectation",
        ));
    }

    Ok(ReplayReceiptEvent {
        event_id: row.event_id,
        event_type,
        kernel_task_run_id,
        session_run_id,
        aggregate_type,
        aggregate_id,
        payload_hash: row.payload_hash,
    })
}

// ===========================================================================
// MT-140 SemanticCatalogBridge storage: the backend-queryable routing-contract
// catalog (table 0260). A catalog entry is authority; the planner resolves a
// query to a route through these rows, not prompt-only helper text.
// ===========================================================================

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// The kind of catalog entry (spec SemanticCatalogEntry.kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticCatalogKind {
    EntityType,
    Index,
    View,
    Tool,
}

impl SemanticCatalogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EntityType => "entity_type",
            Self::Index => "index",
            Self::View => "view",
            Self::Tool => "tool",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "entity_type" => Ok(Self::EntityType),
            "index" => Ok(Self::Index),
            "view" => Ok(Self::View),
            "tool" => Ok(Self::Tool),
            _ => Err(StorageError::Validation("invalid semantic catalog kind")),
        }
    }
}

/// A backend routing-contract catalog entry (authority row).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticCatalogEntry {
    pub entry_id: String,
    pub workspace_id: String,
    pub entry_kind: SemanticCatalogKind,
    pub name: String,
    pub version: i32,
    pub description: String,
    /// Backend query routes (knowledge_graph | shadow_ws_lexical |
    /// shadow_ws_vector | bounded_read | sql_query).
    pub query_routes: Vec<String>,
    pub supported_selectors: Vec<String>,
    pub default_budgets: Option<Value>,
    pub examples: Value,
    pub lifecycle_state: String,
}

/// Insert payload for a catalog entry.
#[derive(Clone, Debug)]
pub struct NewSemanticCatalogEntry {
    pub workspace_id: String,
    pub entry_kind: SemanticCatalogKind,
    pub name: String,
    pub version: i32,
    pub description: String,
    pub query_routes: Vec<String>,
    pub supported_selectors: Vec<String>,
    pub default_budgets: Option<Value>,
    pub examples: Value,
}

/// The stored `knowledge_semantic_catalog_entries` projection.
///
/// `query_routes` / `supported_selectors` are `array<string>` in the embedded
/// schema, so they no longer travel as JSON text that has to be re-parsed on
/// every read; the store now types them.
#[derive(SurrealValue)]
struct SemanticCatalogRecord {
    entry_id: String,
    workspace_id: RecordId,
    entry_kind: String,
    name: String,
    version: i64,
    description: String,
    query_routes: Vec<String>,
    supported_selectors: Vec<String>,
    default_budgets: Option<Value>,
    examples: Value,
    lifecycle_state: String,
}

impl SemanticCatalogRecord {
    fn into_entry(self) -> StorageResult<SemanticCatalogEntry> {
        Ok(SemanticCatalogEntry {
            entry_id: self.entry_id,
            workspace_id: record_key(
                self.workspace_id,
                "semantic catalog workspace link is not a string key",
            )?,
            entry_kind: SemanticCatalogKind::from_db(self.entry_kind.as_str())?,
            name: self.name,
            version: i32::try_from(self.version)
                .map_err(|_| StorageError::Validation("semantic catalog version is out of range"))?,
            description: self.description,
            query_routes: self.query_routes,
            supported_selectors: self.supported_selectors,
            default_budgets: self.default_budgets,
            examples: self.examples,
            lifecycle_state: self.lifecycle_state,
        })
    }
}

#[derive(SurrealValue)]
struct CatalogUpsertBindings {
    entry_id: String,
    workspace: RecordId,
    entry_kind: String,
    name: String,
    version: i64,
    description: String,
    query_routes: Vec<String>,
    supported_selectors: Vec<String>,
    default_budgets: Option<Value>,
    examples: Value,
}

#[derive(SurrealValue)]
struct CatalogLookupBindings {
    workspace: RecordId,
    name: String,
}

#[derive(SurrealValue)]
struct CatalogListBindings {
    workspace: RecordId,
    limit: i64,
}

/// Upsert a catalog entry (by workspace+name+version). Routing contracts are
/// authoritative and must be queryable, not prompt-only (folded stub intent).
///
/// The `ON CONFLICT (workspace_id, name, version) DO UPDATE` of the relational
/// original becomes ONE statement whose existence test and write cannot be
/// interleaved, so two callers racing on the same natural key still converge on
/// a single row rather than both minting an `entry_id`. The natural key is
/// backed by `idx_knowledge_semantic_catalog_name_version UNIQUE`, which is the
/// server-side backstop exactly as the relational unique index was.
pub async fn upsert_semantic_catalog_entry(
    storage: &SurrealStorage,
    new: NewSemanticCatalogEntry,
) -> StorageResult<SemanticCatalogEntry> {
    if new.name.trim() != new.name || new.name.is_empty() {
        return Err(StorageError::Validation(
            "semantic catalog name must be non-empty and trimmed",
        ));
    }
    let entry_id = format!("KSC-{}", Uuid::now_v7().simple());
    let bindings = CatalogUpsertBindings {
        entry_id,
        workspace: RecordId::new(WORKSPACES_TABLE, new.workspace_id.as_str()),
        entry_kind: new.entry_kind.as_str().to_owned(),
        name: new.name.clone(),
        version: i64::from(new.version),
        description: new.description.clone(),
        query_routes: new.query_routes.clone(),
        supported_selectors: new.supported_selectors.clone(),
        default_budgets: new.default_budgets.clone(),
        examples: new.examples.clone(),
    };
    let rows: Vec<SemanticCatalogRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "IF array::len((SELECT VALUE entry_id FROM \
                         knowledge_semantic_catalog_entries WHERE workspace_id = $workspace \
                         AND name = $name AND version = $version)) = 0 { \
                           CREATE type::thing('knowledge_semantic_catalog_entries', $entry_id) \
                           CONTENT { entry_id: $entry_id, workspace_id: $workspace, \
                             entry_kind: $entry_kind, name: $name, version: $version, \
                             description: $description, query_routes: $query_routes, \
                             supported_selectors: $supported_selectors, \
                             default_budgets: $default_budgets, examples: $examples, \
                             lifecycle_state: 'active' } \
                         } ELSE { \
                           UPDATE knowledge_semantic_catalog_entries SET \
                             entry_kind = $entry_kind, description = $description, \
                             query_routes = $query_routes, \
                             supported_selectors = $supported_selectors, \
                             default_budgets = $default_budgets, examples = $examples, \
                             lifecycle_state = 'active', last_updated_at = time::now() \
                           WHERE workspace_id = $workspace AND name = $name \
                             AND version = $version RETURN AFTER \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "semantic catalog upsert produced no row".to_owned(),
        ))?
        .into_entry()
}

/// Resolve a catalog entry by name (highest active version). Returns the backend
/// routing contract the planner uses to choose a route deterministically.
pub async fn resolve_semantic_catalog_entry(
    storage: &SurrealStorage,
    workspace_id: &str,
    name: &str,
) -> StorageResult<Option<SemanticCatalogEntry>> {
    let bindings = CatalogLookupBindings {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        name: name.to_owned(),
    };
    let record: Option<SemanticCatalogRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT entry_id, workspace_id, entry_kind, name, version, description, \
                         query_routes, supported_selectors, default_budgets, examples, \
                         lifecycle_state FROM knowledge_semantic_catalog_entries \
                         WHERE workspace_id = $workspace AND name = $name \
                         AND lifecycle_state = 'active' ORDER BY version DESC LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(SemanticCatalogRecord::into_entry).transpose()
}

/// List active catalog entries for a workspace (bounded), for the debug API and
/// the planner's route resolution.
pub async fn list_semantic_catalog_entries(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<SemanticCatalogEntry>> {
    let bindings = CatalogListBindings {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        limit,
    };
    let records: Vec<SemanticCatalogRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT entry_id, workspace_id, entry_kind, name, version, description, \
                         query_routes, supported_selectors, default_budgets, examples, \
                         lifecycle_state FROM knowledge_semantic_catalog_entries \
                         WHERE workspace_id = $workspace AND lifecycle_state = 'active' \
                         ORDER BY name ASC, version DESC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(SemanticCatalogRecord::into_entry)
        .collect()
}
