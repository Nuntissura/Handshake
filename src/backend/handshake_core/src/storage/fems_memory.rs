//! WP-KERNEL-012 MT-109 FEMS memory-pack + review-gated proposal storage.
//!
//! Durable authority for the Front End Memory System (FEMS) surfaces the
//! native editors read/write:
//!
//! * `fems_memory_packs`   — a seeded/generated [`crate::ace::MemoryPack`] the
//!   `GET /workspaces/{ws}/memory/pack` route returns (AC-109-2). The pack JSON is
//!   the REAL `ace::MemoryPack` shape (`items[].memory_id` / `memory_class` /
//!   `source_refs`) so the native client has a pinned contract.
//! * `fems_memory_proposals` — review-gated memory-write PROPOSALS submitted from the
//!   editor (`POST /workspaces/{ws}/memory/proposals`, AC-109-3). Every row lands as
//!   `status='pending_review'`; only the explicit approved-proposal commit transaction below
//!   can promote it into a canonical memory item.
//! * `fems_memory_items`   — COMMITTED memory items. This table exists so the AC-109-3
//!   negative proof can assert that submitting a proposal does NOT mutate committed
//!   memory. Proposal intake NEVER writes here; the explicit commit path requires a
//!   durable approval and writes item/report/pack/EventLedger atomically.
//!
//! Single-store/EventLedger authority only — NO SQLite.
//!
//! Embedded SurrealDB is the only persistence authority for this module.
use chrono::{DateTime, Utc};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;

use crate::ace::{
    FemsEntityRef, FemsSourceRef, FemsSourceRefKind, MemoryCommitAppliedOp, MemoryCommitOpStatus,
    MemoryCommitReport, MemoryItemProvenance, MemoryMutationOp, MemoryPack, MemoryPackBudgets,
    MemoryPackDeterminismMode, MemoryPackItem, MemoryPackRebuildHint, MemoryPackRebuildHintReason,
    MemoryPolicy, MemoryWriteOp, MemoryWritePolicy, MemoryWriteProposal, PartialMemoryItem,
};
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{
    surreal::{bootstrap_schema, event_ledger, SurrealStorage},
    StorageError, StorageResult,
};

/// Verify that the versioned FEMS migration has installed the required tables.
///
/// Embedded schema readiness is checked through the configured SurrealDB namespace/database.

// ---------------------------------------------------------------------------
// Memory packs (AC-109-2).
// ---------------------------------------------------------------------------

/// Insert an immutable stored memory pack keyed by its content-addressed `pack_id`.
/// Exact retries are accepted, while any attempt to bind the identity to different
/// workspace, scope, or bytes fails closed.

/// Fetch the most recently created memory pack for `workspace_id`, optionally preferring an exact
/// `scope_key`. The workspace-level (`scope_key=''`) pack is also eligible for a context request so a
/// newer approved-memory commit supersedes an older context-specific empty projection.

// ---------------------------------------------------------------------------
// Review-gated proposals (AC-109-3).
// ---------------------------------------------------------------------------

/// A review-gated proposal row as stored/read from `fems_memory_proposals`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StoredMemoryProposal {
    pub proposal_id: String,
    pub request_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub selection_start: i64,
    pub selection_end: i64,
    pub content_hash: String,
    pub memory_class: String,
    pub status: String,
    pub review_gated: bool,
    pub created_at: DateTime<Utc>,
    pub proposal: Value,
}

impl Serialize for StoredMemoryProposal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StoredMemoryProposal", 12)?;
        state.serialize_field("proposal_id", &self.proposal_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("workspace_id", &self.workspace_id)?;
        state.serialize_field("document_id", &self.document_id)?;
        state.serialize_field("selection_start", &self.selection_start)?;
        state.serialize_field("selection_end", &self.selection_end)?;
        state.serialize_field("content_hash", &self.content_hash)?;
        state.serialize_field("memory_class", &self.memory_class)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("review_gated", &self.review_gated)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("proposal", &public_proposal_value(&self.proposal))?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct MemoryProposalReview {
    pub decision: String,
    pub reviewer_kind: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub reason: Option<String>,
    pub correlation_id: String,
}

#[derive(Debug, Clone)]
pub struct MemoryProposalReviewResult {
    pub proposal: StoredMemoryProposal,
    pub receipt: KernelEvent,
    pub reviewed_at: DateTime<Utc>,
}

fn public_proposal_value(proposal: &Value) -> Value {
    let mut proposal = proposal.clone();
    if let Value::Object(object) = &mut proposal {
        object.remove("_receipt_identity");
    }
    proposal
}

/// Immutable proposal artifact used by FR-EVT-MEM-001. Review state is a later lifecycle record and
/// cannot change the proposal hash on retry or restart recovery.
pub fn proposal_artifact_value(proposal: &Value) -> Value {
    if let Some(artifact) = proposal.get("_canonical_artifact") {
        return artifact.clone();
    }
    let mut proposal = proposal.clone();
    if let Value::Object(object) = &mut proposal {
        object.remove("_receipt_identity");
        object.remove("review");
        object.insert("status".to_owned(), json!("pending_review"));
    }
    proposal
}

/// Whether a row that never persisted a `_canonical_artifact` may have one rebuilt for it.
///
/// MT-118: healing is a RECOVERY path for a row that ALREADY EXISTS in the database. It is
/// never a mint path. A proposal being inserted for the first time always arrives from the
/// intake route with its own `_canonical_artifact`, so the insert branch passes
/// [`LegacyArtifactHeal::Deny`] and a first-time payload without an artifact keeps failing
/// closed exactly as it did before MT-118. This split is what keeps the legacy non-UUID
/// `proposal_id` admission (below) unreachable from the normal proposal path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyArtifactHeal {
    /// The row pre-existed this call; rebuild a missing artifact from durable columns.
    Allow,
    /// The row is being created now; a missing artifact is a caller defect, not legacy state.
    Deny,
}

/// Where the canonical artifact returned by [`proposal_canonical_artifact`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalArtifactOrigin {
    /// `_canonical_artifact` was persisted with the row by hardened intake. Authoritative.
    Durable,
    /// Pre-hardening row: no `_canonical_artifact` was ever persisted, so the artifact was
    /// rebuilt IN MEMORY from durable columns only and is never written back to the row.
    HealedFromDurableColumns,
    /// A row with no `_canonical_artifact` that either cannot be rebuilt from what was
    /// actually persisted, or is not eligible for healing. The raw payload is surfaced
    /// unchanged so every downstream canonical check still fails closed.
    UnhealedLegacyPayload,
}

/// A canonical proposal artifact plus the provenance of how it was obtained.
#[derive(Debug, Clone)]
pub struct ProposalArtifact {
    pub value: Value,
    pub origin: ProposalArtifactOrigin,
}

/// Resolve the canonical `hsk.memory_write_proposal@0.1` artifact for a stored proposal row.
///
/// MT-118: rows written before `_canonical_artifact` existed (added by MT-064 in `c7063c24`;
/// `_receipt_identity` had already landed in `892662e0`) carry no artifact at all, so
/// FR-EVT-MEM-001 could never be built for them and EVERY retry failed with
/// `memory proposal artifact is not hsk.memory_write_proposal@0.1: missing field
/// \`schema_version\``. The artifact is therefore rebuilt from durable columns only,
/// mirroring the "Pre-hardening rows cannot recover the headers" receipt branch in
/// [`receipt_for_stored_proposal`]: the heal is a pure function of the durable row, so
/// concurrent retries converge byte-for-byte, and nothing that was not persisted is invented.
///
/// The heal is deliberately IN MEMORY and is never written back to `fems_memory_proposals`.
/// Persisting it would promote a reconstruction to durable evidence, and it would also push
/// the NEXT retry onto the [`ProposalArtifactOrigin::Durable`] branch, which correctly
/// refuses a non-UUID `proposal_id` - a persisted heal would therefore break the very retry
/// convergence it exists to restore.
pub fn proposal_canonical_artifact(
    stored: &StoredMemoryProposal,
    heal: LegacyArtifactHeal,
) -> ProposalArtifact {
    if let Some(artifact) = stored.proposal.get("_canonical_artifact") {
        return ProposalArtifact {
            value: artifact.clone(),
            origin: ProposalArtifactOrigin::Durable,
        };
    }
    if heal == LegacyArtifactHeal::Allow {
        if let Some(value) = heal_legacy_proposal_artifact(stored) {
            return ProposalArtifact {
                value,
                origin: ProposalArtifactOrigin::HealedFromDurableColumns,
            };
        }
    }
    ProposalArtifact {
        value: proposal_artifact_value(&stored.proposal),
        origin: ProposalArtifactOrigin::UnhealedLegacyPayload,
    }
}

/// The intake job identity a pre-hardening row is recovered with, from durable state only.
///
/// Such a row never persisted the request's `hsk-kernel-task-run-id`. The healed receipt and
/// the healed canonical artifact both derive the same workspace-scoped default the live
/// intake route falls back to, from ONE definition, so the two can never drift apart.
/// HBR-PRIV: it carries NO actor identity, so healing cannot widen actor attribution beyond
/// what the row already persisted.
fn legacy_kernel_task_run_id(workspace_id: &str) -> String {
    format!("native-editor-fems-propose-{workspace_id}")
}

/// Rebuild the canonical proposal artifact for a pre-hardening row (AC-118-4).
///
/// EVERY value comes from a durable column of `fems_memory_proposals` - `proposal_id`,
/// `workspace_id`, `document_id`, `selection_start`, `selection_end`, `content_hash`,
/// `memory_class`, `review_gated`, `created_at`, and the durable `proposal` JSONB - or is a
/// fixed structural constant of the single-op editor proposal shape the intake route has
/// always emitted. Nothing is read from the live request, the session, or the clock.
///
/// Returns `None` when the row does not carry enough durable state to be rebuilt honestly.
/// The caller then falls back to the raw payload and fails closed, rather than inventing
/// provenance that was never persisted.
fn heal_legacy_proposal_artifact(stored: &StoredMemoryProposal) -> Option<Value> {
    let workspace_uuid = uuid::Uuid::parse_str(&stored.workspace_id).ok()?;
    let content = stored.proposal.get("content").and_then(Value::as_str)?;
    // Mirrors the intake route's class -> item_type mapping. An unknown class is NOT guessed.
    let (allow_procedural, item_type) = match stored.memory_class.as_str() {
        "procedural" => (true, "tool_protocol"),
        "episodic" => (false, "intent"),
        "semantic" => (false, "fact"),
        _ => return None,
    };
    let created_by_job_id = legacy_kernel_task_run_id(&stored.workspace_id);
    let scope_refs = vec![FemsEntityRef {
        artefact_type: "workspace".to_owned(),
        artefact_id: workspace_uuid,
        selector: "self".to_owned(),
    }];
    let source_refs = vec![FemsSourceRef {
        kind: FemsSourceRefKind::DocBlock,
        id: stored.document_id.clone(),
        hash: Some(stored.content_hash.clone()),
        selector: Some(format!(
            "bytes:{}-{}",
            stored.selection_start, stored.selection_end
        )),
        created_at: Some(stored.created_at.to_rfc3339()),
        classification: Some("low".to_owned()),
    }];
    let healed = MemoryWriteProposal {
        schema_version: "hsk.memory_write_proposal@0.1".to_owned(),
        proposal_id: stored.proposal_id.clone(),
        created_at: stored.created_at.to_rfc3339(),
        created_by_job_id: created_by_job_id.clone(),
        scope_refs: scope_refs.clone(),
        source_refs: source_refs.clone(),
        policy: MemoryWritePolicy {
            allow_procedural,
            // Read from the durable `review_gated` column rather than forced to `true`: a row
            // that was never review-gated must stay unhealable and fail closed downstream,
            // instead of having the review guarantee retroactively asserted on its behalf.
            require_human_review: stored.review_gated,
            max_ops: 1,
        },
        ops: vec![MemoryWriteOp {
            op: MemoryMutationOp::Add,
            temp_id: Some("m1".to_owned()),
            memory_id: None,
            item: PartialMemoryItem {
                memory_class: Some(stored.memory_class.clone()),
                item_type: Some(item_type.to_owned()),
                scope_refs: Some(scope_refs),
                content: Some(content.to_owned()),
                confidence: Some(1.0),
                trust_level: Some("user_asserted".to_owned()),
                provenance: Some(MemoryItemProvenance {
                    source_refs,
                    created_by_job_id,
                }),
                classification: Some("low".to_owned()),
                ..PartialMemoryItem::default()
            },
            rationale: "Editor selection proposed from source_refs[0]".to_owned(),
            confidence: 1.0,
            requires_review: stored.review_gated,
        }],
    };
    serde_json::to_value(healed).ok()
}

/// Atomically insert a review-gated proposal and its canonical EventLedger receipt.
/// Always stored as `status='pending_review'`; this function NEVER writes to
/// `fems_memory_items` (the never-editor-direct invariant). Replaying the same
/// workspace/request identity returns the original row and receipt without duplication.
pub async fn insert_memory_proposal_with_receipt(
    storage: &SurrealStorage,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
) -> StorageResult<StoredMemoryProposal> {
    insert_memory_proposal_with_receipt_inner(storage, proposal, receipt, false).await
}

async fn insert_memory_proposal_with_receipt_inner(
    storage: &SurrealStorage,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
    force_failure_after_proposal_insert: bool,
) -> StorageResult<StoredMemoryProposal> {
    ensure_fems_memory_schema(storage).await?;
    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let mut candidate = proposal.clone();
    stamp_receipt_identity(&mut candidate.proposal, &receipt)?;

    if let Some(stored) =
        proposal_by_request(storage, &candidate.workspace_id, &candidate.request_id).await?
    {
        if !same_logical_proposal(&stored, &candidate) {
            return Err(StorageError::Conflict(
                "memory proposal request_id was reused with a different payload",
            ));
        }
        let expected = receipt_for_stored_proposal(receipt, &stored)?;
        let persisted = event_ledger::get_by_idempotency(storage, &expected.idempotency_key)
            .await?
            .ok_or(StorageError::Conflict(
                "memory proposal exists without its EventLedger receipt",
            ))?;
        validate_existing_proposal_receipt(&persisted, &stored)?;
        ensure_lifecycle_outbox(
            storage,
            &stored,
            "FR-EVT-MEM-001",
            &build_memory_proposal_flight_recorder_event(
                &stored,
                &persisted,
                LegacyArtifactHeal::Allow,
            )?,
        )
        .await?;
        return Ok(stored);
    }
    if get_memory_proposal(storage, &candidate.proposal_id)
        .await?
        .is_some()
    {
        return Err(StorageError::Conflict(
            "memory proposal id is bound to a different request",
        ));
    }
    let receipt = receipt_for_stored_proposal(receipt, &candidate)?;
    let (persisted_receipt, ledger) = event_ledger::prepare_event(receipt)?;
    let event = build_memory_proposal_flight_recorder_event(
        &candidate,
        &persisted_receipt,
        LegacyArtifactHeal::Deny,
    )?;
    let bindings = ProposalInsertBindings {
        proposal_record: RecordId::new(PROPOSALS_TABLE, candidate.proposal_id.clone()),
        proposal: proposal_content(&candidate),
        force_failure_after_proposal_insert,
        ledger,
        outbox: lifecycle_outbox_write(
            &candidate.workspace_id,
            &candidate.proposal_id,
            "FR-EVT-MEM-001",
            &event,
        )?,
    };
    run_proposal_insert_transaction(storage, bindings).await?;
    Ok(candidate)
}

#[cfg(test)]
pub async fn insert_memory_proposal_with_receipt_forced_failure(
    storage: &SurrealStorage,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
) -> StorageResult<StoredMemoryProposal> {
    insert_memory_proposal_with_receipt_inner(storage, proposal, receipt, true).await
}

/// Atomically move a proposal out of `pending_review` and append the matching durable
/// EventLedger decision receipt. Exact retries return the original transition; a different
/// decision or reviewer identity is a conflict and cannot rewrite the audit record.
pub async fn review_memory_proposal_with_receipt(
    storage: &SurrealStorage,
    workspace_id: &str,
    proposal_id: &str,
    review: &MemoryProposalReview,
    mut receipt: NewKernelEvent,
) -> StorageResult<MemoryProposalReviewResult> {
    ensure_fems_memory_schema(storage).await?;
    let target_status = match review.decision.as_str() {
        "approved" => "approved",
        "rejected" => "rejected",
        _ => {
            return Err(StorageError::Validation(
                "memory proposal review decision must be approved or rejected",
            ));
        }
    };
    if !matches!(
        (review.reviewer_kind.as_str(), review.actor_kind.as_str()),
        ("user", "operator") | ("policy", "system")
    ) || review.actor_id.trim().is_empty()
        || review.actor_id.len() > 200
        || !review
            .actor_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        || review.correlation_id != format!("fems-memory-proposal-review:{proposal_id}")
        || review
            .reason
            .as_deref()
            .is_some_and(|reason| reason.trim().is_empty() || reason.len() > 1_000)
    {
        return Err(StorageError::Validation(
            "memory proposal review identity is invalid",
        ));
    }

    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let mut stored = get_memory_proposal(storage, proposal_id)
        .await?
        .filter(|stored| stored.workspace_id == workspace_id)
        .ok_or(StorageError::NotFound("memory proposal in workspace"))?;
    let expected_review = json!({
        "decision": review.decision,
        "reviewer_kind": review.reviewer_kind,
        "actor_kind": review.actor_kind,
        "actor_id": review.actor_id,
        "reason": review.reason,
        "correlation_id": review.correlation_id,
    });
    let is_new = stored.status == "pending_review";
    let reviewed_at = if is_new {
        let reviewed_at = Utc::now();
        let Value::Object(proposal) = &mut stored.proposal else {
            return Err(StorageError::Conflict(
                "memory proposal payload is not an object",
            ));
        };
        let mut persisted_review = expected_review.clone();
        persisted_review
            .as_object_mut()
            .expect("review literal is an object")
            .insert("reviewed_at".to_owned(), json!(reviewed_at));
        proposal.insert("review".to_owned(), persisted_review);
        proposal.insert("status".to_owned(), json!(target_status));
        stored.status = target_status.to_owned();
        reviewed_at
    } else if stored.status == target_status
        || (stored.status == "committed" && target_status == "approved")
    {
        let mut comparable = stored
            .proposal
            .get("review")
            .and_then(Value::as_object)
            .cloned()
            .map(Value::Object)
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal is missing review evidence",
            ))?;
        let reviewed_at = comparable
            .as_object_mut()
            .and_then(|value| value.remove("reviewed_at"))
            .and_then(|value| value.as_str().map(str::to_owned))
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc))
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal has invalid review timestamp",
            ))?;
        if comparable != expected_review {
            return Err(StorageError::Conflict(
                "memory proposal review retry does not match the persisted reviewer evidence",
            ));
        }
        reviewed_at
    } else {
        return Err(StorageError::Conflict(
            "memory proposal was already reviewed with a different decision",
        ));
    };

    receipt.aggregate_type = "fems_memory_proposal".to_owned();
    receipt.aggregate_id = stored.proposal_id.clone();
    receipt.idempotency_key = format!("fems-memory-proposal-review:{}", stored.proposal_id);
    receipt.event_type = if target_status == "approved" {
        KernelEventType::PromotionAccepted
    } else {
        KernelEventType::PromotionRejected
    };
    receipt.correlation_id = Some(review.correlation_id.clone());
    receipt.source_component = "fems_memory_proposal_review".to_owned();
    receipt.payload = json!({
        "receipt_kind": "fems_memory_write_review",
        "proposal_id": stored.proposal_id,
        "workspace_id": stored.workspace_id,
        "document_id": stored.document_id,
        "content_hash": stored.content_hash,
        "decision": review.decision,
        "reviewer_kind": review.reviewer_kind,
        "actor_kind": review.actor_kind,
        "actor_id": review.actor_id,
        "reason_present": review.reason.is_some(),
        "reviewed_at": reviewed_at,
    });
    receipt.payload_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&receipt.payload),
    );

    if !is_new {
        let persisted = event_ledger::get_by_idempotency(storage, &receipt.idempotency_key)
            .await?
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal exists without its EventLedger receipt",
            ))?;
        let candidate = KernelEvent::from_new(receipt);
        ensure_matching_receipt(&persisted, &candidate)?;
        let event = build_memory_review_flight_recorder_event(&stored, &persisted, reviewed_at)?;
        ensure_lifecycle_outbox(storage, &stored, "FR-EVT-MEM-002", &event).await?;
        return Ok(MemoryProposalReviewResult {
            proposal: stored,
            receipt: persisted,
            reviewed_at,
        });
    }

    let (persisted, ledger) = event_ledger::prepare_event(receipt)?;
    let event = build_memory_review_flight_recorder_event(&stored, &persisted, reviewed_at)?;
    let bindings = ProposalReviewBindings {
        proposal_record: RecordId::new(PROPOSALS_TABLE, stored.proposal_id.clone()),
        proposal: proposal_content(&stored),
        expected_status: "pending_review".to_owned(),
        ledger,
        outbox: lifecycle_outbox_write(workspace_id, proposal_id, "FR-EVT-MEM-002", &event)?,
    };
    run_proposal_review_transaction(storage, bindings).await?;
    let candidate_receipt = persisted;
    let persisted = event_ledger::get_by_idempotency(storage, &candidate_receipt.idempotency_key)
        .await?
        .ok_or(StorageError::Conflict(
            "reviewed memory proposal transaction committed without its EventLedger receipt",
        ))?;
    ensure_matching_receipt(&persisted, &candidate_receipt)?;
    Ok(MemoryProposalReviewResult {
        proposal: stored,
        receipt: persisted,
        reviewed_at,
    })
}

fn receipt_for_stored_proposal(
    mut receipt: NewKernelEvent,
    stored: &StoredMemoryProposal,
) -> StorageResult<NewKernelEvent> {
    if let Some(identity) = stored
        .proposal
        .get("_receipt_identity")
        .and_then(Value::as_object)
    {
        receipt.event_version = required_identity_string(identity, "event_version")?;
        receipt.kernel_task_run_id = required_identity_string(identity, "kernel_task_run_id")?;
        receipt.session_run_id = required_identity_string(identity, "session_run_id")?;
        let actor_id = required_identity_string(identity, "actor_id")?;
        receipt.actor = match required_identity_string(identity, "actor_kind")?.as_str() {
            "operator" => KernelActor::Operator(actor_id),
            "system" => KernelActor::System(actor_id),
            _ => {
                return Err(StorageError::Conflict(
                    "memory proposal receipt identity contains an invalid actor kind",
                ));
            }
        };
        receipt.causation_id = optional_identity_string(identity, "causation_id")?;
        receipt.correlation_id = optional_identity_string(identity, "correlation_id")?;
    } else {
        // Pre-hardening rows cannot recover the headers from the request that originally
        // persisted the proposal. Heal their missing receipt from durable proposal fields
        // only, so concurrent retries with different headers still converge byte-for-byte.
        let actor_id = stored
            .proposal
            .get("actor_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("native_editor")
            .to_owned();
        receipt.event_version = "kernel_event_v1".to_owned();
        receipt.kernel_task_run_id = legacy_kernel_task_run_id(&stored.workspace_id);
        receipt.session_run_id = "native-editor-session".to_owned();
        receipt.actor = KernelActor::Operator(actor_id);
        receipt.causation_id = None;
        receipt.correlation_id = None;
    }
    receipt.aggregate_id = stored.proposal_id.clone();
    receipt.idempotency_key = format!("fems-memory-proposal:{}", stored.proposal_id);
    receipt.aggregate_type = "fems_memory_proposal".to_owned();
    receipt.event_type = KernelEventType::ArtifactProposed;
    receipt.source_component = "fems_memory_proposal_intake".to_owned();
    receipt.payload = proposal_receipt_payload(stored);
    receipt.payload_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&receipt.payload),
    );
    receipt
        .validate()
        .map_err(|_| StorageError::Conflict("memory proposal receipt identity is invalid"))?;
    Ok(receipt)
}

fn stamp_receipt_identity(proposal: &mut Value, receipt: &NewKernelEvent) -> StorageResult<()> {
    let Value::Object(proposal) = proposal else {
        return Err(StorageError::Validation(
            "memory proposal payload must be a JSON object",
        ));
    };
    proposal.insert(
        "_receipt_identity".to_owned(),
        json!({
            "event_version": receipt.event_version,
            "kernel_task_run_id": receipt.kernel_task_run_id,
            "session_run_id": receipt.session_run_id,
            "actor_kind": receipt.actor.actor_kind(),
            "actor_id": receipt.actor.actor_id(),
            "causation_id": receipt.causation_id,
            "correlation_id": receipt.correlation_id,
        }),
    );
    Ok(())
}

fn required_identity_string(
    identity: &serde_json::Map<String, Value>,
    field: &'static str,
) -> StorageResult<String> {
    identity
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or(StorageError::Conflict(
            "memory proposal receipt identity is incomplete",
        ))
}

fn optional_identity_string(
    identity: &serde_json::Map<String, Value>,
    field: &'static str,
) -> StorageResult<Option<String>> {
    match identity.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        _ => Err(StorageError::Conflict(
            "memory proposal receipt identity is malformed",
        )),
    }
}

fn proposal_receipt_payload(stored: &StoredMemoryProposal) -> Value {
    json!({
        "receipt_kind": "fems_memory_write_proposal",
        "proposal_id": stored.proposal_id,
        "workspace_id": stored.workspace_id,
        "document_id": stored.document_id,
        "selection_start": stored.selection_start,
        "selection_end": stored.selection_end,
        "content_hash": stored.content_hash,
        "memory_class": stored.memory_class,
        "review_gated": stored.review_gated,
        // Intake receipts authenticate immutable proposal state. Review transitions are
        // separate append-only receipts and must not rewrite the intake identity.
        "status": "pending_review",
        "never_editor_direct": true,
    })
}

/// Receipt shape emitted before selection-range provenance was added to the canonical audit payload.
/// Existing append-only rows remain valid on exact retry; new receipts always use
/// [`proposal_receipt_payload`] and therefore carry the complete source range.
fn legacy_proposal_receipt_payload(stored: &StoredMemoryProposal) -> Value {
    json!({
        "receipt_kind": "fems_memory_write_proposal",
        "proposal_id": stored.proposal_id,
        "workspace_id": stored.workspace_id,
        "document_id": stored.document_id,
        "content_hash": stored.content_hash,
        "memory_class": stored.memory_class,
        "review_gated": stored.review_gated,
        "status": stored.status,
        "never_editor_direct": true,
    })
}

fn validate_existing_proposal_receipt(
    existing: &KernelEvent,
    stored: &StoredMemoryProposal,
) -> StorageResult<()> {
    let canonical_payload = proposal_receipt_payload(stored);
    let canonical_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&canonical_payload),
    );
    let legacy_payload = legacy_proposal_receipt_payload(stored);
    let legacy_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&legacy_payload),
    );
    let structural = NewKernelEvent {
        event_version: existing.event_version.clone(),
        kernel_task_run_id: existing.kernel_task_run_id.clone(),
        session_run_id: existing.session_run_id.clone(),
        aggregate_type: existing.aggregate_type.clone(),
        aggregate_id: existing.aggregate_id.clone(),
        idempotency_key: existing.idempotency_key.clone(),
        event_type: existing.event_type.clone(),
        actor: existing.actor.clone(),
        causation_id: existing.causation_id.clone(),
        correlation_id: existing.correlation_id.clone(),
        payload_hash: existing.payload_hash.clone(),
        source_component: existing.source_component.clone(),
        payload: existing.payload.clone(),
    };
    let structurally_valid = existing.event_sequence > 0
        && existing
            .event_id
            .strip_prefix("KE-")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        && structural.validate().is_ok();
    let closed_receipt_valid = existing.event_version == "kernel_event_v1"
        && existing.aggregate_type == "fems_memory_proposal"
        && existing.aggregate_id == stored.proposal_id
        && existing.idempotency_key == format!("fems-memory-proposal:{}", stored.proposal_id)
        && existing.event_type == KernelEventType::ArtifactProposed
        && existing.source_component == "fems_memory_proposal_intake"
        && ((existing.payload == canonical_payload && existing.payload_hash == canonical_hash)
            || (existing.payload == legacy_payload && existing.payload_hash == legacy_hash));

    let identity_valid = if stored.proposal.get("_receipt_identity").is_some() {
        let expected = receipt_for_stored_proposal(structural.clone(), stored)?;
        existing.event_version == expected.event_version
            && existing.kernel_task_run_id == expected.kernel_task_run_id
            && existing.session_run_id == expected.session_run_id
            && existing.actor == expected.actor
            && existing.causation_id == expected.causation_id
            && existing.correlation_id == expected.correlation_id
    } else {
        // Compatibility for pre-hardening rows: their proposal JSON did not retain the
        // original request headers. Validate the only actor variants the intake ever
        // emitted, require non-empty immutable identities, and still require the exact
        // proposal-derived payload/hash above. No audit row is rewritten.
        matches!(
            &existing.actor,
            KernelActor::Operator(_) | KernelActor::System(_)
        ) && !existing.actor.actor_id().trim().is_empty()
            && !existing.kernel_task_run_id.trim().is_empty()
            && !existing.session_run_id.trim().is_empty()
            && existing.causation_id.is_none()
            && existing.correlation_id.is_none()
    };

    if structurally_valid && closed_receipt_valid && identity_valid {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "memory proposal receipt idempotency key is bound to non-authentic audit evidence",
        ))
    }
}

fn same_logical_proposal(stored: &StoredMemoryProposal, incoming: &StoredMemoryProposal) -> bool {
    fn without_server_identity(value: &Value) -> Value {
        let mut value = value.clone();
        if let Value::Object(object) = &mut value {
            object.remove("proposal_id");
            object.remove("_canonical_artifact");
            object.remove("_receipt_identity");
            // The router derives actor_id from the live native binding. It belongs to the
            // immutable receipt identity, not the caller's authoritative proposal payload, so
            // an exact request-id retry from a later authenticated session must still converge.
            object.remove("actor_id");
            // Review evidence is server-owned mutable state. It is deliberately excluded
            // from intake replay equality so submit -> review -> identical submit converges.
            object.remove("review");
            object.remove("status");
        }
        value
    }

    stored.request_id == incoming.request_id
        && stored.workspace_id == incoming.workspace_id
        && stored.document_id == incoming.document_id
        && stored.selection_start == incoming.selection_start
        && stored.selection_end == incoming.selection_end
        && stored.content_hash == incoming.content_hash
        && stored.memory_class == incoming.memory_class
        && stored.review_gated == incoming.review_gated
        && without_server_identity(&stored.proposal) == without_server_identity(&incoming.proposal)
}

/// Read a stored proposal by id (used by the AC-109-3 proofs).

/// List a bounded workspace projection of actionable memory proposals. Approved proposals sort
/// first so an interrupted review->commit sequence is recovered before new pending reviews.

// ---------------------------------------------------------------------------
// Committed memory items (proposals must NEVER write here — AC-109-3 negative proof).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MemoryProposalCommitResult {
    pub proposal: StoredMemoryProposal,
    pub memory_item: Value,
    pub memory_pack: MemoryPack,
    pub commit_report: MemoryCommitReport,
    pub commit_report_hash: String,
    pub receipt: KernelEvent,
    pub flight_recorder_event: FlightRecorderEvent,
    pub memory_pack_flight_recorder_event: FlightRecorderEvent,
}

fn flight_recorder_actor(receipt: &KernelEvent) -> FlightRecorderActor {
    match &receipt.actor {
        KernelActor::Operator(_) => FlightRecorderActor::Human,
        KernelActor::System(_) => FlightRecorderActor::System,
        _ => FlightRecorderActor::Agent,
    }
}

fn build_memory_proposal_flight_recorder_event(
    proposal: &StoredMemoryProposal,
    receipt: &KernelEvent,
    heal: LegacyArtifactHeal,
) -> StorageResult<FlightRecorderEvent> {
    let artifact = proposal_canonical_artifact(proposal, heal);
    // MT-118 AC-118-2 TRIPWIRE. A non-UUID `proposal_id` is admitted ONLY when the artifact
    // was rebuilt for a pre-existing row that never persisted one. Any artifact that WAS
    // persisted - including one persisted on a pre-existing row - still has to satisfy the
    // UUID contract, so the relaxation is unreachable for every proposal that carries its own
    // `_canonical_artifact`, and unreachable on the insert/mint path in any case.
    let legacy_proposal_id_admitted =
        artifact.origin == ProposalArtifactOrigin::HealedFromDurableColumns;
    let canonical: MemoryWriteProposal =
        serde_json::from_value(artifact.value).map_err(|error| {
            StorageError::Serialization(format!(
                "memory proposal artifact is not hsk.memory_write_proposal@0.1: {error}"
            ))
        })?;
    if canonical.schema_version != "hsk.memory_write_proposal@0.1"
        || canonical.proposal_id != proposal.proposal_id
        || (!legacy_proposal_id_admitted && uuid::Uuid::parse_str(&canonical.proposal_id).is_err())
        || canonical.ops.is_empty()
        || canonical.ops.len() > canonical.policy.max_ops as usize
        || !canonical.policy.require_human_review
        || canonical
            .ops
            .iter()
            .any(|op| !op.requires_review || !(0.0..=1.0).contains(&op.confidence))
    {
        return Err(StorageError::Conflict(
            "memory proposal artifact violates the canonical proposal contract",
        ));
    }
    let proposal_hash = canonical
        .compute_hash()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let op_count = canonical.ops.len() as u64;
    let requires_review_count = canonical.ops.iter().filter(|op| op.requires_review).count() as u64;
    let artifact_ref = format!("artifact://sha256/{proposal_hash}");
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::MemoryWriteProposed,
        flight_recorder_actor(receipt),
        stable_uuid(&format!("fems-memory-proposal:{}", proposal.proposal_id)),
        json!({
            "type": "memory_write_proposed",
            "event_code": "FR-EVT-MEM-001",
            "proposal_id": proposal.proposal_id,
            "proposal_hash": proposal_hash,
            "artifact_ref": artifact_ref,
            "scope_refs": canonical.scope_refs,
            "op_count": op_count,
            "requires_review_count": requires_review_count,
        }),
    )
    .with_actor_id(receipt.actor.actor_id().to_owned())
    .with_wsids(vec![proposal.workspace_id.clone()]);
    event.event_id = stable_uuid(&format!(
        "fems-memory-proposal-event:{}",
        proposal.proposal_id
    ));
    event.timestamp = proposal.created_at;
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn build_memory_review_flight_recorder_event(
    proposal: &StoredMemoryProposal,
    receipt: &KernelEvent,
    reviewed_at: DateTime<Utc>,
) -> StorageResult<FlightRecorderEvent> {
    let review = proposal
        .proposal
        .get("review")
        .and_then(Value::as_object)
        .ok_or(StorageError::Conflict(
            "reviewed memory proposal is missing review evidence",
        ))?;
    let decision = review
        .get("decision")
        .and_then(Value::as_str)
        .ok_or(StorageError::Conflict(
            "reviewed memory proposal is missing its decision",
        ))?;
    let reviewer_kind =
        review
            .get("reviewer_kind")
            .and_then(Value::as_str)
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal is missing its reviewer kind",
            ))?;
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::MemoryWriteReviewed,
        flight_recorder_actor(receipt),
        stable_uuid(&format!(
            "fems-memory-proposal-review:{}",
            proposal.proposal_id
        )),
        json!({
            "type": "memory_write_reviewed",
            "event_code": "FR-EVT-MEM-002",
            "proposal_id": proposal.proposal_id,
            "decision": decision,
            "reviewer_kind": reviewer_kind,
        }),
    )
    .with_actor_id(receipt.actor.actor_id().to_owned())
    .with_wsids(vec![proposal.workspace_id.clone()]);
    event.event_id = stable_uuid(&format!(
        "fems-memory-proposal-review-event:{}",
        proposal.proposal_id
    ));
    event.timestamp = reviewed_at;
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn event_json_hash(event: &FlightRecorderEvent) -> StorageResult<String> {
    let value = serde_json::to_value(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&value),
    ))
}

pub(crate) fn canonical_changed_memory_ids_hash<'a>(
    memory_ids: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut memory_ids = memory_ids
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    memory_ids.sort();
    memory_ids.dedup();
    hex::encode(Sha256::digest(
        crate::kernel::context_bundle::canonical_json_bytes(&json!(memory_ids)),
    ))
}

fn build_memory_commit_flight_recorder_event(
    workspace_id: &str,
    proposal_id: &str,
    memory_id: &str,
    commit_report: &MemoryCommitReport,
    commit_report_hash: &str,
    receipt: &KernelEvent,
) -> StorageResult<FlightRecorderEvent> {
    let committed_at = DateTime::parse_from_rfc3339(&commit_report.created_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::Conflict("memory commit report has an invalid created_at"))?;
    let actor = flight_recorder_actor(receipt);
    let changed_memory_ids_hash = canonical_changed_memory_ids_hash([memory_id]);
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::MemoryWriteCommitted,
        actor,
        stable_uuid(&format!("fems-memory-proposal:{proposal_id}")),
        json!({
            "type": "memory_write_committed",
            "event_code": "FR-EVT-MEM-003",
            "commit_id": commit_report.commit_id,
            "proposal_id": proposal_id,
            "commit_report_hash": commit_report_hash,
            "artifact_ref": format!("artifact://sha256/{commit_report_hash}"),
            "changed_memory_ids_hash": changed_memory_ids_hash,
        }),
    )
    .with_actor_id(receipt.actor.actor_id().to_owned())
    .with_wsids(vec![workspace_id.to_owned()]);
    event.event_id = stable_uuid(&format!("fems-memory-proposal-commit-event:{proposal_id}"));
    event.timestamp = committed_at;
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn build_memory_pack_flight_recorder_event(
    workspace_id: &str,
    proposal_id: &str,
    commit_id: &str,
    pack: &MemoryPack,
    commit_event: &FlightRecorderEvent,
) -> StorageResult<FlightRecorderEvent> {
    let truncation_occurred = pack
        .warnings
        .iter()
        .any(|warning| warning == "memory_pack_truncated_to_24_items");
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::MemoryPackBuilt,
        commit_event.actor.clone(),
        commit_event.trace_id,
        json!({
            "type": "memory_pack_built",
            "event_code": "FR-EVT-MEM-004",
            "pack_id": pack.pack_id,
            "memory_pack_hash": pack.memory_pack_hash,
            "artifact_ref": format!("artifact://sha256/{}", pack.memory_pack_hash),
            "memory_policy": pack.memory_policy.as_str(),
            "scope_refs": pack.scope_refs,
            "item_count": pack.items.len(),
            "token_estimate": pack.token_estimate,
            "truncation_occurred": truncation_occurred,
        }),
    )
    .with_actor_id(commit_event.actor_id.clone())
    .with_wsids(vec![workspace_id.to_owned()]);
    event.event_id = stable_uuid(&format!(
        "fems-memory-pack-built-event:{commit_id}:{}",
        pack.pack_id
    ));
    event.timestamp = commit_event.timestamp + chrono::Duration::microseconds(1);
    event.activity_span_id = Some(format!("fems-memory-proposal:{proposal_id}"));
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

fn same_memory_commit_event(left: &FlightRecorderEvent, right: &FlightRecorderEvent) -> bool {
    left.event_id == right.event_id
        && left.trace_id == right.trace_id
        && left.timestamp.timestamp_micros() == right.timestamp.timestamp_micros()
        && left.actor == right.actor
        && left.actor_id == right.actor_id
        && left.event_type == right.event_type
        && left.job_id == right.job_id
        && left.workflow_id == right.workflow_id
        && left.model_id == right.model_id
        && left.model_session_id == right.model_session_id
        && left.wsids == right.wsids
        && left.activity_span_id == right.activity_span_id
        && left.session_span_id == right.session_span_id
        && left.capability_id == right.capability_id
        && left.policy_decision_id == right.policy_decision_id
        && left.payload == right.payload
}

const OUTBOX_ERROR_MAX_CHARS: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLifecyclePublicationState {
    Published,
    Pending,
    Quarantined,
    Missing,
}

/// Explicitly retry a quarantined lifecycle projection for an identical authoritative proposal.
/// The proposal row and immutable event payload remain unchanged; only the delivery-attempt state is
/// reset so the bounded reconciler can try the exact event again.

fn bounded_outbox_error(error: &str) -> String {
    error.chars().take(OUTBOX_ERROR_MAX_CHARS).collect()
}

/// Restart/upgrade healing for proposal and review rows created before their transactional outbox
/// landed, and for fault-injection tests that remove an outbox row after the authoritative commit.

/// Upgrade/restart recovery for commits written before the transactional outbox migration, or for
/// any row whose projection envelope was deliberately removed by a fault-injection proof. The
/// existing-commit branch reconstructs the envelope only from immutable report/EventLedger/pack
/// evidence; the placeholder receipt below is never used for an already committed proposal.

fn stable_uuid(seed: &str) -> uuid::Uuid {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn bounded_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn memory_item_type(memory_class: &str) -> &'static str {
    match memory_class {
        "procedural" => "tool_protocol",
        "episodic" => "intent",
        _ => "fact",
    }
}

fn proposal_reviewed_at(proposal: &StoredMemoryProposal) -> StorageResult<DateTime<Utc>> {
    proposal
        .proposal
        .get("review")
        .and_then(|review| review.get("reviewed_at"))
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .ok_or(StorageError::Conflict(
            "approved memory proposal is missing canonical review evidence",
        ))
}

fn build_memory_item(
    proposal: &StoredMemoryProposal,
    memory_id: &str,
    reviewed_at: DateTime<Utc>,
) -> StorageResult<(Value, MemoryPackItem, FemsEntityRef)> {
    let workspace_uuid = uuid::Uuid::parse_str(&proposal.workspace_id).map_err(|_| {
        StorageError::Validation("memory proposal workspace id is not a canonical UUID")
    })?;
    let scope_ref = FemsEntityRef {
        artefact_type: "workspace".to_owned(),
        artefact_id: workspace_uuid,
        selector: "workspace".to_owned(),
    };
    let source_ref = FemsSourceRef {
        kind: FemsSourceRefKind::DocBlock,
        id: proposal.document_id.clone(),
        hash: Some(proposal.content_hash.clone()),
        selector: Some(format!(
            "bytes:{}-{}",
            proposal.selection_start, proposal.selection_end
        )),
        created_at: Some(proposal.created_at.to_rfc3339()),
        classification: Some("low".to_owned()),
    };
    let content = proposal
        .proposal
        .get("content")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(StorageError::Conflict(
            "approved memory proposal has no immutable content",
        ))?;
    let trust_level = if proposal.memory_class == "procedural" {
        "local_authoritative"
    } else {
        "user_asserted"
    };
    let item_type = memory_item_type(&proposal.memory_class);
    let pack_item = MemoryPackItem {
        memory_id: memory_id.to_owned(),
        memory_class: proposal.memory_class.clone(),
        item_type: item_type.to_owned(),
        summary: bounded_chars(content, 240),
        content: bounded_chars(content, 600),
        structured: None,
        trust_level: trust_level.to_owned(),
        confidence: 1.0,
        scope_refs: vec![scope_ref.clone()],
        source_refs: vec![source_ref.clone()],
        pinned: proposal.memory_class == "procedural",
        last_verified_at: Some(reviewed_at.to_rfc3339()),
    };
    let item = json!({
        "memory_id": memory_id,
        "memory_class": proposal.memory_class,
        "type": item_type,
        "summary": pack_item.summary,
        "content": content,
        "structured": Value::Null,
        "confidence": 1.0,
        "trust_level": trust_level,
        "scope_refs": pack_item.scope_refs,
        "source_refs": pack_item.source_refs,
        "provenance": {
            "source_refs": pack_item.source_refs,
            "created_by_job_id": proposal.proposal_id,
        },
        "classification": "low",
        "valid_from": proposal.created_at.to_rfc3339(),
        "last_verified_at": reviewed_at.to_rfc3339(),
        "status": "active",
        "version": 1,
        "review_approved": true,
    });
    Ok((item, pack_item, scope_ref))
}

/// Commit exactly one approved proposal. The canonical item, immutable commit report,
/// strict MemoryPack projection, proposal terminal state, and EventLedger receipt are
/// written in one transaction. Exact retries return the original result.
pub async fn commit_memory_proposal_with_receipt(
    storage: &SurrealStorage,
    workspace_id: &str,
    proposal_id: &str,
    mut receipt: NewKernelEvent,
) -> StorageResult<MemoryProposalCommitResult> {
    ensure_fems_memory_schema(storage).await?;
    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let mut proposal = get_memory_proposal(storage, proposal_id)
        .await?
        .filter(|proposal| proposal.workspace_id == workspace_id)
        .ok_or(StorageError::NotFound("memory proposal in workspace"))?;
    if proposal.status != "approved" && proposal.status != "committed" {
        return Err(StorageError::Conflict(
            "memory proposal must be approved before commit",
        ));
    }
    let reviewed_at = proposal_reviewed_at(&proposal)?;
    let memory_id = stable_uuid(&format!("fems-memory-item:{proposal_id}")).to_string();
    let commit_id = stable_uuid(&format!("fems-memory-commit:{proposal_id}")).to_string();
    let (memory_item, pack_item, scope_ref) =
        build_memory_item(&proposal, &memory_id, reviewed_at)?;

    if let Some(report_row) = select_report_by_proposal(storage, proposal_id).await? {
        let stored_commit_id = report_row.commit_id.clone();
        let stored_memory_id = record_key(report_row.memory_id, "commit report memory")?;
        let commit_report: MemoryCommitReport = serde_json::from_value(report_row.report)?;
        let computed_hash = commit_report
            .compute_hash()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        if stored_commit_id != commit_id
            || stored_memory_id != memory_id
            || commit_report.source_proposal_id != proposal_id
            || report_row.report_hash != computed_hash
        {
            return Err(StorageError::Conflict(
                "memory proposal commit identity is bound to different evidence",
            ));
        }
        let item = select_item(storage, &memory_id)
            .await?
            .ok_or(StorageError::Conflict(
                "memory proposal commit references a missing item",
            ))?;
        if record_key(item.workspace_id, "memory item workspace")? != workspace_id
            || item.item != memory_item
        {
            return Err(StorageError::Conflict(
                "memory proposal commit references different item evidence",
            ));
        }
        let persisted =
            event_ledger::get_by_idempotency(storage, &format!("fems-memory-commit:{proposal_id}"))
                .await?
                .ok_or(StorageError::Conflict(
                    "memory proposal commit is missing its EventLedger receipt",
                ))?;
        let payload = persisted.payload.as_object().ok_or(StorageError::Conflict(
            "memory commit receipt payload is not an object",
        ))?;
        let payload_string = |key: &str| payload.get(key).and_then(Value::as_str);
        if payload_string("workspace_id") != Some(workspace_id)
            || payload_string("proposal_id") != Some(proposal_id)
            || payload_string("commit_id") != Some(commit_id.as_str())
            || payload_string("memory_id") != Some(memory_id.as_str())
            || payload_string("commit_report_hash") != Some(computed_hash.as_str())
        {
            return Err(StorageError::Conflict(
                "memory commit receipt is bound to different evidence",
            ));
        }
        let pack_id = payload_string("memory_pack_id").ok_or(StorageError::Conflict(
            "memory commit receipt is missing memory_pack_id",
        ))?;
        let pack_row =
            select_pack(storage, workspace_id, pack_id)
                .await?
                .ok_or(StorageError::Conflict(
                    "memory commit receipt references a missing memory pack",
                ))?;
        if record_key(pack_row.workspace_id, "memory pack workspace")? != workspace_id {
            return Err(StorageError::Conflict(
                "memory commit receipt references another workspace's memory pack",
            ));
        }
        let memory_pack: MemoryPack = serde_json::from_value(pack_row.pack)?;
        let pack_hash = memory_pack
            .compute_hash()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        if pack_hash != memory_pack.memory_pack_hash
            || payload_string("memory_pack_hash") != Some(pack_hash.as_str())
        {
            return Err(StorageError::Conflict(
                "memory commit receipt references a corrupt memory pack",
            ));
        }
        let committed_event = build_memory_commit_flight_recorder_event(
            workspace_id,
            proposal_id,
            &memory_id,
            &commit_report,
            &computed_hash,
            &persisted,
        )?;
        let packed_event = build_memory_pack_flight_recorder_event(
            workspace_id,
            proposal_id,
            &commit_id,
            &memory_pack,
            &committed_event,
        )?;
        create_commit_outbox_if_absent(
            storage,
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-003",
            &committed_event,
        )
        .await?;
        create_commit_outbox_if_absent(
            storage,
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-004",
            &packed_event,
        )
        .await?;
        return Ok(MemoryProposalCommitResult {
            proposal,
            memory_item,
            memory_pack,
            commit_report,
            commit_report_hash: computed_hash,
            receipt: persisted,
            flight_recorder_event: committed_event,
            memory_pack_flight_recorder_event: packed_event,
        });
    }

    if proposal.status != "approved" {
        return Err(StorageError::Conflict(
            "committed memory proposal is missing its immutable report",
        ));
    }
    if select_item(storage, &memory_id).await?.is_some() {
        return Err(StorageError::Conflict(
            "memory item identity is bound without a matching commit report",
        ));
    }
    let now = Utc::now();
    let committed_at = if now <= reviewed_at {
        reviewed_at + chrono::Duration::microseconds(1)
    } else {
        now
    };
    let commit_report = MemoryCommitReport {
        schema_version: "hsk.memory_commit_report@0.1".to_owned(),
        commit_id: commit_id.clone(),
        created_at: committed_at.to_rfc3339(),
        source_proposal_id: proposal_id.to_owned(),
        applied_ops: vec![MemoryCommitAppliedOp {
            op: MemoryMutationOp::Add,
            memory_id: memory_id.clone(),
            previous_version: None,
            new_version: Some(1),
            status: MemoryCommitOpStatus::Applied,
            reason: None,
        }],
        warnings: Vec::new(),
        pack_rebuild_hints: vec![MemoryPackRebuildHint {
            scope_ref: scope_ref.clone(),
            reason: MemoryPackRebuildHintReason::MemoryChanged,
        }],
    };
    let commit_report_hash = commit_report
        .compute_hash()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let memory_pack =
        build_memory_pack(storage, workspace_id, committed_at, scope_ref, pack_item).await?;

    proposal.status = "committed".to_owned();
    if let Value::Object(object) = &mut proposal.proposal {
        object.insert("status".to_owned(), json!("committed"));
    }
    receipt.aggregate_type = "fems_memory_commit".to_owned();
    receipt.aggregate_id = commit_id.clone();
    receipt.idempotency_key = format!("fems-memory-commit:{proposal_id}");
    receipt.event_type = KernelEventType::ArtifactStored;
    receipt.correlation_id = Some(format!("fems-memory-proposal:{proposal_id}"));
    receipt.source_component = "fems_memory_proposal_commit".to_owned();
    receipt.payload = json!({
        "receipt_kind": "fems_memory_write_committed",
        "fr_event_id": "FR-EVT-MEM-003",
        "memory_pack_fr_event_id": "FR-EVT-MEM-004",
        "workspace_id": workspace_id,
        "proposal_id": proposal_id,
        "commit_id": commit_id,
        "memory_id": memory_id,
        "memory_pack_id": memory_pack.pack_id,
        "memory_pack_hash": memory_pack.memory_pack_hash,
        "commit_report_hash": commit_report_hash,
    });
    receipt.payload_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&receipt.payload),
    );
    let (persisted, ledger) = event_ledger::prepare_event(receipt)?;
    let committed_event = build_memory_commit_flight_recorder_event(
        workspace_id,
        proposal_id,
        &memory_id,
        &commit_report,
        &commit_report_hash,
        &persisted,
    )?;
    let packed_event = build_memory_pack_flight_recorder_event(
        workspace_id,
        proposal_id,
        &commit_id,
        &memory_pack,
        &committed_event,
    )?;
    let stamp = Datetime::from(committed_at);
    let bindings = CommitBindings {
        item_record: RecordId::new(ITEMS_TABLE, memory_id.clone()),
        item: ItemContent {
            memory_id: memory_id.clone(),
            workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
            item: memory_item.clone(),
            created_at: stamp.clone(),
            updated_at: stamp.clone(),
        },
        report_record: RecordId::new(REPORTS_TABLE, commit_id.clone()),
        report: ReportContent {
            commit_id: commit_id.clone(),
            workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
            proposal_id: RecordId::new(PROPOSALS_TABLE, proposal_id),
            memory_id: RecordId::new(ITEMS_TABLE, memory_id.clone()),
            report: serde_json::to_value(&commit_report)?,
            report_hash: commit_report_hash.clone(),
            created_at: stamp.clone(),
        },
        proposal_record: RecordId::new(PROPOSALS_TABLE, proposal_id),
        proposal: proposal_content(&proposal),
        expected_status: "approved".to_owned(),
        pack_record: RecordId::new(PACKS_TABLE, memory_pack.pack_id.clone()),
        pack: PackContent {
            pack_id: memory_pack.pack_id.clone(),
            workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
            scope_key: String::new(),
            pack: serde_json::to_value(&memory_pack)?,
            generated_at: stamp.clone(),
            created_at: stamp,
        },
        ledger,
        committed_outbox: commit_outbox_write(
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-003",
            &committed_event,
        )?,
        packed_outbox: commit_outbox_write(
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-004",
            &packed_event,
        )?,
    };
    run_commit_transaction(storage, bindings).await?;
    let candidate_receipt = persisted;
    let persisted = event_ledger::get_by_idempotency(storage, &candidate_receipt.idempotency_key)
        .await?
        .ok_or(StorageError::Conflict(
            "memory commit transaction committed without its EventLedger receipt",
        ))?;
    ensure_matching_receipt(&persisted, &candidate_receipt)?;
    Ok(MemoryProposalCommitResult {
        proposal,
        memory_item,
        memory_pack,
        commit_report,
        commit_report_hash,
        receipt: persisted,
        flight_recorder_event: committed_event,
        memory_pack_flight_recorder_event: packed_event,
    })
}

/// Insert or replace a COMMITTED memory item. This is only reachable from a downstream
/// review/commit path (out of MT-109 scope) or a test seeding a committed item to prove
/// a proposal cannot mutate it.

/// Read a committed memory item's JSON by id (AC-109-3 negative proof: unchanged after a
/// proposal is submitted).

/// Count committed memory items for a workspace (AC-109-3 negative proof: submitting a
/// proposal does not increase the committed-item count).

// ---------------------------------------------------------------------------
// Embedded SurrealDB persistence (WP-KERNEL-012 MT-136).
// ---------------------------------------------------------------------------

const WORKSPACES_TABLE: &str = "workspaces";
const PACKS_TABLE: &str = "fems_memory_packs";
const PROPOSALS_TABLE: &str = "fems_memory_proposals";
const ITEMS_TABLE: &str = "fems_memory_items";
const REPORTS_TABLE: &str = "fems_memory_commit_reports";
const LIFECYCLE_OUTBOX_TABLE: &str = "fems_memory_lifecycle_fr_outbox";
const COMMIT_OUTBOX_TABLE: &str = "fems_memory_commit_fr_outbox";

static FEMS_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceScopeBinding {
    workspace: RecordId,
    scope: Option<String>,
}

#[derive(SurrealValue)]
struct WorkspaceLimitBinding {
    workspace: RecordId,
    limit: i64,
}

#[derive(SurrealValue)]
struct OutboxListBinding {
    workspace: Option<RecordId>,
    limit: i64,
}

#[derive(SurrealValue)]
struct WorkspaceValueBinding {
    workspace: RecordId,
    value: String,
}

#[derive(SurrealValue)]
struct ProposalEventBinding {
    proposal: RecordId,
    event_code: String,
}

#[derive(SurrealValue)]
struct EventMutationBinding {
    workspace: RecordId,
    event_id: String,
    error: Option<String>,
    quarantine: bool,
    now: Datetime,
}

#[derive(SurrealValue)]
struct OutboxIdentityBinding {
    workspace: RecordId,
    event_id: String,
}

#[derive(SurrealValue)]
struct FailureMutationBinding {
    workspace: RecordId,
    event_id: String,
    error: String,
    expected_attempt_count: i64,
    next_attempt_count: i64,
    quarantined_at: Option<Datetime>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct AttemptCountRow {
    attempt_count: i64,
}

#[derive(SurrealValue)]
struct LimitBinding {
    limit: i64,
}

#[derive(SurrealValue)]
struct PackContent {
    pack_id: String,
    workspace_id: RecordId,
    scope_key: String,
    pack: Value,
    generated_at: Datetime,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct PackRow {
    pack_id: String,
    workspace_id: RecordId,
    scope_key: String,
    pack: Value,
    generated_at: Datetime,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct ProposalContent {
    proposal_id: String,
    request_id: String,
    workspace_id: RecordId,
    document_id: String,
    selection_start: i64,
    selection_end: i64,
    content_hash: String,
    memory_class: String,
    status: String,
    review_gated: bool,
    created_at: Datetime,
    proposal: Value,
}

#[derive(SurrealValue)]
struct ProposalRow {
    proposal_id: String,
    request_id: String,
    workspace_id: RecordId,
    document_id: String,
    selection_start: i64,
    selection_end: i64,
    content_hash: String,
    memory_class: String,
    status: String,
    review_gated: bool,
    created_at: Datetime,
    proposal: Value,
}

#[derive(SurrealValue)]
struct ItemContent {
    memory_id: String,
    workspace_id: RecordId,
    item: Value,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct ItemRow {
    memory_id: String,
    workspace_id: RecordId,
    item: Value,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CountRow {
    count: i64,
}

#[derive(SurrealValue)]
struct ReportRow {
    commit_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    memory_id: RecordId,
    report: Value,
    report_hash: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct ReportContent {
    commit_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    memory_id: RecordId,
    report: Value,
    report_hash: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct OutboxRow {
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    commit_id: Option<RecordId>,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
    published_at: Option<Datetime>,
    attempt_count: i64,
    last_error: Option<String>,
    last_error_at: Option<Datetime>,
    quarantined_at: Option<Datetime>,
}

#[derive(SurrealValue)]
struct LifecycleOutboxRow {
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
    published_at: Option<Datetime>,
    attempt_count: i64,
    last_error: Option<String>,
    last_error_at: Option<Datetime>,
    quarantined_at: Option<Datetime>,
}

#[derive(Clone, SurrealValue)]
struct LifecycleOutboxWrite {
    record: RecordId,
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct LifecycleOutboxContent {
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
}

impl From<LifecycleOutboxWrite> for LifecycleOutboxContent {
    fn from(value: LifecycleOutboxWrite) -> Self {
        Self {
            event_id: value.event_id,
            workspace_id: value.workspace_id,
            proposal_id: value.proposal_id,
            event_code: value.event_code,
            event: value.event,
            event_hash: value.event_hash,
            created_at: value.created_at,
        }
    }
}

#[derive(Clone, SurrealValue)]
struct CommitOutboxWrite {
    record: RecordId,
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    commit_id: RecordId,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct CommitOutboxContent {
    event_id: String,
    workspace_id: RecordId,
    proposal_id: RecordId,
    commit_id: RecordId,
    event_code: String,
    event: Value,
    event_hash: String,
    created_at: Datetime,
}

impl From<CommitOutboxWrite> for CommitOutboxContent {
    fn from(value: CommitOutboxWrite) -> Self {
        Self {
            event_id: value.event_id,
            workspace_id: value.workspace_id,
            proposal_id: value.proposal_id,
            commit_id: value.commit_id,
            event_code: value.event_code,
            event: value.event,
            event_hash: value.event_hash,
            created_at: value.created_at,
        }
    }
}

#[derive(SurrealValue)]
struct ProposalInsertBindings {
    proposal_record: RecordId,
    proposal: ProposalContent,
    force_failure_after_proposal_insert: bool,
    ledger: event_ledger::LedgerWrite,
    outbox: LifecycleOutboxWrite,
}

#[derive(SurrealValue)]
struct ProposalReviewBindings {
    proposal_record: RecordId,
    proposal: ProposalContent,
    expected_status: String,
    ledger: event_ledger::LedgerWrite,
    outbox: LifecycleOutboxWrite,
}

#[derive(SurrealValue)]
struct CommitBindings {
    item_record: RecordId,
    item: ItemContent,
    report_record: RecordId,
    report: ReportContent,
    proposal_record: RecordId,
    proposal: ProposalContent,
    expected_status: String,
    pack_record: RecordId,
    pack: PackContent,
    ledger: event_ledger::LedgerWrite,
    committed_outbox: CommitOutboxWrite,
    packed_outbox: CommitOutboxWrite,
}

#[derive(Clone, Copy)]
enum OutboxKind {
    Lifecycle,
    Commit,
}

async fn run_proposal_insert_transaction(
    storage: &SurrealStorage,
    bindings: ProposalInsertBindings,
) -> StorageResult<()> {
    let rows: Vec<ProposalRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         CREATE $proposal_record CONTENT { \
                            proposal_id: $proposal.proposal_id, request_id: $proposal.request_id, \
                            workspace_id: $proposal.workspace_id, document_id: $proposal.document_id, \
                            selection_start: $proposal.selection_start, selection_end: $proposal.selection_end, \
                            content_hash: $proposal.content_hash, memory_class: $proposal.memory_class, \
                            status: $proposal.status, review_gated: $proposal.review_gated, \
                            created_at: $proposal.created_at, proposal: $proposal.proposal \
                         } RETURN AFTER; \
                         IF $force_failure_after_proposal_insert { \
                            THROW 'forced failure after proposal insert'; \
                         }; \
                         CREATE $ledger.record CONTENT { \
                            event_id: $ledger.event_id, event_version: $ledger.event_version, \
                            kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, \
                            aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, \
                            idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, \
                            actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, \
                            causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, \
                            payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, \
                            payload: $ledger.payload, created_at: $ledger.created_at \
                         }; \
                         CREATE $outbox.record CONTENT { \
                            event_id: $outbox.event_id, workspace_id: $outbox.workspace_id, \
                            proposal_id: $outbox.proposal_id, event_code: $outbox.event_code, \
                            event: $outbox.event, event_hash: $outbox.event_hash, created_at: $outbox.created_at \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if rows.len() == 1 {
        Ok(())
    } else {
        Err(StorageError::Database(
            "proposal transaction did not create exactly one row".to_owned(),
        ))
    }
}

async fn run_proposal_review_transaction(
    storage: &SurrealStorage,
    bindings: ProposalReviewBindings,
) -> StorageResult<()> {
    let _: Vec<surrealdb::types::Value> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF array::len((UPDATE $proposal_record CONTENT { \
                            proposal_id: $proposal.proposal_id, request_id: $proposal.request_id, \
                            workspace_id: $proposal.workspace_id, document_id: $proposal.document_id, \
                            selection_start: $proposal.selection_start, selection_end: $proposal.selection_end, \
                            content_hash: $proposal.content_hash, memory_class: $proposal.memory_class, \
                            status: $proposal.status, review_gated: $proposal.review_gated, \
                            created_at: $proposal.created_at, proposal: $proposal.proposal \
                         } WHERE status = $expected_status RETURN AFTER)) != 1 { \
                            THROW 'HSK-FEMS-REVIEW-STATE'; \
                         }; \
                         CREATE $ledger.record CONTENT { \
                            event_id: $ledger.event_id, event_version: $ledger.event_version, \
                            kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, \
                            aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, \
                            idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, \
                            actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, \
                            causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, \
                            payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, \
                            payload: $ledger.payload, created_at: $ledger.created_at \
                         }; \
                         CREATE $outbox.record CONTENT { \
                            event_id: $outbox.event_id, workspace_id: $outbox.workspace_id, \
                            proposal_id: $outbox.proposal_id, event_code: $outbox.event_code, \
                            event: $outbox.event, event_hash: $outbox.event_hash, created_at: $outbox.created_at \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| {
            if error.to_string().contains("HSK-FEMS-REVIEW-STATE") {
                StorageError::Conflict(
                    "memory proposal review lost its pending-review transition",
                )
            } else {
                StorageError::from(error)
            }
        })?;
    Ok(())
}

pub async fn ensure_fems_memory_schema(storage: &SurrealStorage) -> StorageResult<()> {
    bootstrap_schema(storage)
        .await
        .map(|_| ())
        .map_err(|error| StorageError::Migration(error.to_string()))
}

pub async fn upsert_memory_pack(
    storage: &SurrealStorage,
    workspace_id: &str,
    scope_key: &str,
    pack: &MemoryPack,
) -> StorageResult<()> {
    ensure_fems_memory_schema(storage).await?;
    let generated_at = DateTime::parse_from_rfc3339(&pack.generated_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::Validation("memory pack generated_at is invalid"))?;
    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let record_id = pack.pack_id.clone();
    let pack_value = serde_json::to_value(pack)?;
    if let Some(existing) = select_pack(storage, workspace_id, &record_id).await? {
        if record_key(existing.workspace_id, "memory pack workspace")? != workspace_id
            || existing.scope_key != scope_key
            || existing.pack != pack_value
        {
            return Err(StorageError::Conflict(
                "memory pack id is bound to different immutable content",
            ));
        }
        return Ok(());
    }
    let now = Datetime::from(Utc::now());
    let content = PackContent {
        pack_id: pack.pack_id.clone(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
        scope_key: scope_key.to_owned(),
        pack: pack_value,
        generated_at: Datetime::from(generated_at),
        created_at: now,
    };
    let id = pack.pack_id.clone();
    let created: Option<PackRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.create_if_absent(PACKS_TABLE, &id, content).await })
        })
        .await
        .map_err(StorageError::from)?;
    if created.is_some() {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "memory pack id was concurrently bound to different content",
        ))
    }
}

pub async fn get_latest_memory_pack(
    storage: &SurrealStorage,
    workspace_id: &str,
    scope_key: Option<&str>,
) -> StorageResult<Option<MemoryPack>> {
    ensure_fems_memory_schema(storage).await?;
    let bindings = WorkspaceScopeBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        scope: scope_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
    };
    let row: Option<PackRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT pack_id, workspace_id, scope_key, pack, generated_at, created_at \
                         FROM fems_memory_packs WHERE workspace_id = $workspace \
                         AND ($scope = NONE OR scope_key = $scope OR scope_key = '') \
                         ORDER BY created_at DESC, scope_key DESC, pack_id DESC LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(|row| serde_json::from_value(row.pack).map_err(StorageError::from))
        .transpose()
}

pub async fn get_memory_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    let id = proposal_id.to_owned();
    let row: Option<ProposalRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.select_one(PROPOSALS_TABLE, &id).await })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(proposal_from_row).transpose()
}

pub async fn list_memory_proposals(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<StoredMemoryProposal>> {
    let bindings = WorkspaceLimitBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        limit: limit.clamp(1, 200),
    };
    let rows: Vec<ProposalRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT proposal_id, request_id, workspace_id, document_id, selection_start, \
                         selection_end, content_hash, memory_class, status, review_gated, created_at, proposal \
                         FROM fems_memory_proposals WHERE workspace_id = $workspace \
                         ORDER BY created_at DESC, proposal_id ASC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(proposal_from_row).collect()
}

pub async fn upsert_memory_item(
    storage: &SurrealStorage,
    workspace_id: &str,
    memory_id: &str,
    item: &Value,
) -> StorageResult<()> {
    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let now = Datetime::from(Utc::now());
    let created_at = if let Some(existing) = select_item(storage, memory_id).await? {
        if record_key(existing.workspace_id, "memory item workspace")? != workspace_id {
            return Err(StorageError::Conflict(
                "memory item id belongs to a different workspace",
            ));
        }
        existing.created_at
    } else {
        now.clone()
    };
    let content = ItemContent {
        memory_id: memory_id.to_owned(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
        item: item.clone(),
        created_at,
        updated_at: now,
    };
    let id = memory_id.to_owned();
    let _: Option<ItemRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.upsert_one(ITEMS_TABLE, &id, content).await })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(())
}

pub async fn get_memory_item(
    storage: &SurrealStorage,
    workspace_id: &str,
    memory_id: &str,
) -> StorageResult<Option<Value>> {
    let Some(row) = select_item(storage, memory_id).await? else {
        return Ok(None);
    };
    if record_key(row.workspace_id.clone(), "memory item workspace")? != workspace_id {
        return Ok(None);
    }
    Ok(Some(row.item))
}

pub async fn count_memory_items(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<i64> {
    let bindings = WorkspaceBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
    };
    let row: Option<CountRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT count() AS count FROM fems_memory_items \
                         WHERE workspace_id = $workspace GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(row.map_or(0, |row| row.count))
}

pub async fn memory_lifecycle_publication_state(
    storage: &SurrealStorage,
    proposal_id: &str,
    event_code: &str,
) -> StorageResult<MemoryLifecyclePublicationState> {
    Ok(
        match lookup_lifecycle_outbox(storage, proposal_id, event_code).await? {
            None => MemoryLifecyclePublicationState::Missing,
            Some(row) if row.published_at.is_some() => MemoryLifecyclePublicationState::Published,
            Some(row) if row.quarantined_at.is_some() => {
                MemoryLifecyclePublicationState::Quarantined
            }
            Some(_) => MemoryLifecyclePublicationState::Pending,
        },
    )
}

pub async fn requeue_quarantined_memory_lifecycle_event(
    storage: &SurrealStorage,
    proposal_id: &str,
    event_code: &str,
) -> StorageResult<bool> {
    let bindings = ProposalEventBinding {
        proposal: RecordId::new(PROPOSALS_TABLE, proposal_id),
        event_code: event_code.to_owned(),
    };
    let count = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE fems_memory_lifecycle_fr_outbox SET attempt_count = 0, \
                         last_error = NONE, last_error_at = NONE, quarantined_at = NONE \
                         WHERE proposal_id = $proposal AND event_code = $event_code \
                         AND published_at = NONE AND quarantined_at != NONE RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(count == 1)
}

pub async fn record_memory_lifecycle_event_failure(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: uuid::Uuid,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    record_outbox_failure(
        storage,
        OutboxKind::Lifecycle,
        workspace_id,
        &event_id.to_string(),
        error,
        quarantine_now,
    )
    .await
}

pub async fn list_all_pending_memory_lifecycle_events(
    storage: &SurrealStorage,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let rows = list_lifecycle_rows(storage, None, limit).await?;
    decode_lifecycle_rows(storage, rows).await
}

pub async fn list_pending_memory_lifecycle_events(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FlightRecorderEvent>> {
    Ok(decode_lifecycle_rows(
        storage,
        list_lifecycle_rows(storage, Some(workspace_id), limit).await?,
    )
    .await?
    .into_iter()
    .map(|(_, event)| event)
    .collect())
}

pub async fn mark_memory_lifecycle_event_published(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: uuid::Uuid,
) -> StorageResult<()> {
    mark_outbox_published(
        storage,
        OutboxKind::Lifecycle,
        workspace_id,
        &event_id.to_string(),
    )
    .await
}

pub async fn recover_missing_memory_lifecycle_outbox_events(
    storage: &SurrealStorage,
) -> StorageResult<u64> {
    ensure_fems_memory_schema(storage).await?;
    let rows: Vec<ProposalRow> = storage
        .with_data_operation(|database| {
            Box::pin(async move { database.select_all(PROPOSALS_TABLE).await })
        })
        .await
        .map_err(StorageError::from)?;
    let mut recovered = 0;
    for row in rows {
        let stored = proposal_from_row(row)?;
        if lookup_lifecycle_outbox(storage, &stored.proposal_id, "FR-EVT-MEM-001")
            .await?
            .is_none()
        {
            let receipt = event_ledger::get_by_idempotency(
                storage,
                &format!("fems-memory-proposal:{}", stored.proposal_id),
            )
            .await?
            .ok_or(StorageError::Conflict(
                "memory proposal is missing its EventLedger receipt",
            ))?;
            let event = build_memory_proposal_flight_recorder_event(
                &stored,
                &receipt,
                LegacyArtifactHeal::Allow,
            )?;
            create_lifecycle_outbox_if_absent(storage, &stored, "FR-EVT-MEM-001", &event).await?;
            recovered += 1;
        }
        if stored.proposal.get("review").is_some()
            && lookup_lifecycle_outbox(storage, &stored.proposal_id, "FR-EVT-MEM-002")
                .await?
                .is_none()
        {
            let receipt = event_ledger::get_by_idempotency(
                storage,
                &format!("fems-memory-proposal-review:{}", stored.proposal_id),
            )
            .await?
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal is missing its EventLedger receipt",
            ))?;
            let reviewed_at = proposal_reviewed_at(&stored)?;
            let event = build_memory_review_flight_recorder_event(&stored, &receipt, reviewed_at)?;
            create_lifecycle_outbox_if_absent(storage, &stored, "FR-EVT-MEM-002", &event).await?;
            recovered += 1;
        }
    }
    Ok(recovered)
}

pub async fn list_pending_memory_commit_events(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FlightRecorderEvent>> {
    Ok(decode_commit_rows(
        storage,
        list_commit_rows(storage, Some(workspace_id), limit).await?,
    )
    .await?
    .into_iter()
    .map(|(_, event)| event)
    .collect())
}

pub async fn list_all_pending_memory_commit_events(
    storage: &SurrealStorage,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let rows = list_commit_rows(storage, None, limit).await?;
    decode_commit_rows(storage, rows).await
}

pub async fn record_memory_commit_event_failure(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: uuid::Uuid,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    record_outbox_failure(
        storage,
        OutboxKind::Commit,
        workspace_id,
        &event_id.to_string(),
        error,
        quarantine_now,
    )
    .await
}

pub async fn recover_missing_memory_commit_outbox_events(
    storage: &SurrealStorage,
) -> StorageResult<u64> {
    ensure_fems_memory_schema(storage).await?;
    let rows: Vec<ReportRow> = storage
        .with_data_operation(|database| {
            Box::pin(async move { database.select_all(REPORTS_TABLE).await })
        })
        .await
        .map_err(StorageError::from)?;
    let mut recovered = 0;
    for row in rows {
        let workspace_id = record_key(row.workspace_id, "commit report workspace")?;
        let proposal_id = record_key(row.proposal_id, "commit report proposal")?;
        let missing_commit = lookup_commit_outbox(storage, &proposal_id, "FR-EVT-MEM-003")
            .await?
            .is_none();
        let missing_pack = lookup_commit_outbox(storage, &proposal_id, "FR-EVT-MEM-004")
            .await?
            .is_none();
        if missing_commit || missing_pack {
            let placeholder = NewKernelEvent::builder(
                "fems-memory-startup-recovery",
                "fems-memory-startup-recovery",
                KernelEventType::ArtifactStored,
                KernelActor::System("fems-memory-startup-recovery".to_owned()),
            )
            .idempotency_key(format!("unused-fems-memory-recovery:{proposal_id}"))
            .source_component("fems_memory_startup_recovery")
            .payload(json!({"proposal_id": proposal_id}))
            .build()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
            commit_memory_proposal_with_receipt(storage, &workspace_id, &proposal_id, placeholder)
                .await?;
            recovered += u64::from(missing_commit) + u64::from(missing_pack);
        }
    }
    Ok(recovered)
}

pub async fn mark_memory_commit_event_published(
    storage: &SurrealStorage,
    workspace_id: &str,
    event_id: uuid::Uuid,
) -> StorageResult<()> {
    mark_outbox_published(
        storage,
        OutboxKind::Commit,
        workspace_id,
        &event_id.to_string(),
    )
    .await
}

pub async fn get_memory_commit_report(
    storage: &SurrealStorage,
    workspace_id: &str,
    commit_id: &str,
) -> StorageResult<Option<MemoryCommitReport>> {
    let bindings = WorkspaceValueBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        value: commit_id.to_owned(),
    };
    let row: Option<ReportRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT commit_id, workspace_id, proposal_id, memory_id, report, report_hash, created_at \
                         FROM fems_memory_commit_reports WHERE workspace_id = $workspace \
                         AND commit_id = $value LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(|row| {
        let report: MemoryCommitReport = serde_json::from_value(row.report)?;
        let hash = report
            .compute_hash()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        if hash != row.report_hash {
            return Err(StorageError::Conflict(
                "memory commit report hash does not match stored evidence",
            ));
        }
        Ok(report)
    })
    .transpose()
}

fn lifecycle_outbox_write(
    workspace_id: &str,
    proposal_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<LifecycleOutboxWrite> {
    Ok(LifecycleOutboxWrite {
        record: RecordId::new(LIFECYCLE_OUTBOX_TABLE, event.event_id.to_string()),
        event_id: event.event_id.to_string(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
        proposal_id: RecordId::new(PROPOSALS_TABLE, proposal_id),
        event_code: event_code.to_owned(),
        event: serde_json::to_value(event)?,
        event_hash: event_json_hash(event)?,
        created_at: Datetime::from(event.timestamp),
    })
}

fn commit_outbox_write(
    workspace_id: &str,
    proposal_id: &str,
    commit_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<CommitOutboxWrite> {
    Ok(CommitOutboxWrite {
        record: RecordId::new(COMMIT_OUTBOX_TABLE, event.event_id.to_string()),
        event_id: event.event_id.to_string(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, workspace_id),
        proposal_id: RecordId::new(PROPOSALS_TABLE, proposal_id),
        commit_id: RecordId::new(REPORTS_TABLE, commit_id),
        event_code: event_code.to_owned(),
        event: serde_json::to_value(event)?,
        event_hash: event_json_hash(event)?,
        created_at: Datetime::from(event.timestamp),
    })
}

async fn ensure_lifecycle_outbox(
    storage: &SurrealStorage,
    proposal: &StoredMemoryProposal,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let bindings = ProposalEventBinding {
        proposal: RecordId::new(PROPOSALS_TABLE, proposal.proposal_id.clone()),
        event_code: event_code.to_owned(),
    };
    let row: Option<LifecycleOutboxRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, workspace_id, proposal_id, event_code, event, event_hash, \
                         created_at, published_at, attempt_count, last_error, last_error_at, quarantined_at \
                         FROM fems_memory_lifecycle_fr_outbox WHERE proposal_id = $proposal \
                         AND event_code = $event_code LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let row = row.ok_or(StorageError::Conflict(
        "memory proposal exists without its lifecycle outbox event",
    ))?;
    let stored: FlightRecorderEvent = serde_json::from_value(row.event)?;
    if row.event_id != event.event_id.to_string()
        || record_key(row.workspace_id, "lifecycle outbox workspace")? != proposal.workspace_id
        || record_key(row.proposal_id, "lifecycle outbox proposal")? != proposal.proposal_id
        || row.event_code != event_code
        || !same_memory_commit_event(&stored, event)
        || row.event_hash != event_json_hash(event)?
        || event_json_hash(&stored)? != row.event_hash
    {
        return Err(StorageError::Conflict(
            "memory lifecycle outbox identity is bound to different evidence",
        ));
    }
    Ok(())
}

fn ensure_matching_receipt(stored: &KernelEvent, candidate: &KernelEvent) -> StorageResult<()> {
    if stored.event_version == candidate.event_version
        && stored.kernel_task_run_id == candidate.kernel_task_run_id
        && stored.session_run_id == candidate.session_run_id
        && stored.aggregate_type == candidate.aggregate_type
        && stored.aggregate_id == candidate.aggregate_id
        && stored.idempotency_key == candidate.idempotency_key
        && stored.event_type == candidate.event_type
        && stored.actor == candidate.actor
        && stored.causation_id == candidate.causation_id
        && stored.correlation_id == candidate.correlation_id
        && stored.payload_hash == candidate.payload_hash
        && stored.source_component == candidate.source_component
        && stored.payload == candidate.payload
    {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "kernel event idempotency key was reused with different event content",
        ))
    }
}

async fn lookup_lifecycle_outbox(
    storage: &SurrealStorage,
    proposal_id: &str,
    event_code: &str,
) -> StorageResult<Option<LifecycleOutboxRow>> {
    let bindings = ProposalEventBinding {
        proposal: RecordId::new(PROPOSALS_TABLE, proposal_id),
        event_code: event_code.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, workspace_id, proposal_id, event_code, event, event_hash, \
                         created_at, published_at, attempt_count, last_error, last_error_at, quarantined_at \
                         FROM fems_memory_lifecycle_fr_outbox WHERE proposal_id = $proposal \
                         AND event_code = $event_code LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn lookup_commit_outbox(
    storage: &SurrealStorage,
    proposal_id: &str,
    event_code: &str,
) -> StorageResult<Option<OutboxRow>> {
    let bindings = ProposalEventBinding {
        proposal: RecordId::new(PROPOSALS_TABLE, proposal_id),
        event_code: event_code.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, workspace_id, proposal_id, commit_id, event_code, event, \
                         event_hash, created_at, published_at, attempt_count, last_error, \
                         last_error_at, quarantined_at FROM fems_memory_commit_fr_outbox \
                         WHERE proposal_id = $proposal AND event_code = $event_code LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn create_lifecycle_outbox_if_absent(
    storage: &SurrealStorage,
    proposal: &StoredMemoryProposal,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let write = lifecycle_outbox_write(
        &proposal.workspace_id,
        &proposal.proposal_id,
        event_code,
        event,
    )?;
    let id = write.event_id.clone();
    let content = LifecycleOutboxContent::from(write);
    let created: Option<LifecycleOutboxRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .create_if_absent(LIFECYCLE_OUTBOX_TABLE, &id, content)
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if created.is_none() {
        ensure_lifecycle_outbox(storage, proposal, event_code, event).await?;
    }
    Ok(())
}

async fn list_lifecycle_rows(
    storage: &SurrealStorage,
    workspace_id: Option<&str>,
    limit: i64,
) -> StorageResult<Vec<LifecycleOutboxRow>> {
    let bindings = OutboxListBinding {
        workspace: workspace_id.map(|id| RecordId::new(WORKSPACES_TABLE, id)),
        limit: limit.clamp(1, 200),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, workspace_id, proposal_id, event_code, event, event_hash, \
                         created_at, published_at, attempt_count, last_error, last_error_at, quarantined_at \
                         FROM fems_memory_lifecycle_fr_outbox \
                         WHERE published_at = NONE AND quarantined_at = NONE \
                         AND ($workspace = NONE OR workspace_id = $workspace) \
                         ORDER BY created_at ASC, event_id ASC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn list_commit_rows(
    storage: &SurrealStorage,
    workspace_id: Option<&str>,
    limit: i64,
) -> StorageResult<Vec<OutboxRow>> {
    let bindings = OutboxListBinding {
        workspace: workspace_id.map(|id| RecordId::new(WORKSPACES_TABLE, id)),
        limit: limit.clamp(1, 200),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, workspace_id, proposal_id, commit_id, event_code, event, \
                         event_hash, created_at, published_at, attempt_count, last_error, \
                         last_error_at, quarantined_at FROM fems_memory_commit_fr_outbox \
                         WHERE published_at = NONE AND quarantined_at = NONE \
                         AND ($workspace = NONE OR workspace_id = $workspace) \
                         AND (event_code = 'FR-EVT-MEM-003' OR commit_id IN \
                            (SELECT VALUE commit_id FROM fems_memory_commit_fr_outbox \
                             WHERE event_code = 'FR-EVT-MEM-003' AND published_at != NONE)) \
                         ORDER BY created_at ASC, event_code ASC, event_id ASC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn decode_lifecycle_rows(
    storage: &SurrealStorage,
    rows: Vec<LifecycleOutboxRow>,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let workspace_id = record_key(row.workspace_id, "lifecycle outbox workspace")?;
        match decode_outbox_event(&row.event_id, &row.event_hash, row.event) {
            Ok(event) => decoded.push((workspace_id, event)),
            Err(error) => {
                record_outbox_failure(
                    storage,
                    OutboxKind::Lifecycle,
                    &workspace_id,
                    &row.event_id,
                    &error.to_string(),
                    true,
                )
                .await?;
            }
        }
    }
    Ok(decoded)
}

async fn decode_commit_rows(
    storage: &SurrealStorage,
    rows: Vec<OutboxRow>,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let workspace_id = record_key(row.workspace_id, "commit outbox workspace")?;
        match decode_outbox_event(&row.event_id, &row.event_hash, row.event) {
            Ok(event) => decoded.push((workspace_id, event)),
            Err(error) => {
                record_outbox_failure(
                    storage,
                    OutboxKind::Commit,
                    &workspace_id,
                    &row.event_id,
                    &error.to_string(),
                    true,
                )
                .await?;
            }
        }
    }
    Ok(decoded)
}

fn decode_outbox_event(
    event_id: &str,
    event_hash: &str,
    value: Value,
) -> StorageResult<FlightRecorderEvent> {
    let expected_id = uuid::Uuid::parse_str(event_id)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let event: FlightRecorderEvent = serde_json::from_value(value)?;
    if event.event_id != expected_id || event_json_hash(&event)? != event_hash {
        return Err(StorageError::Conflict(
            "memory outbox event hash or identity does not match its envelope",
        ));
    }
    event
        .validate()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(event)
}

async fn record_outbox_failure(
    storage: &SurrealStorage,
    kind: OutboxKind,
    workspace_id: &str,
    event_id: &str,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    let _serial = FEMS_MUTATION_LOCK.lock().await;
    let identity = OutboxIdentityBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_owned(),
    };
    let select = match kind {
        OutboxKind::Lifecycle => {
            "SELECT attempt_count FROM fems_memory_lifecycle_fr_outbox \
             WHERE workspace_id = $workspace AND event_id = $event_id \
             AND published_at = NONE LIMIT 1;"
        }
        OutboxKind::Commit => {
            "SELECT attempt_count FROM fems_memory_commit_fr_outbox \
             WHERE workspace_id = $workspace AND event_id = $event_id \
             AND published_at = NONE LIMIT 1;"
        }
    };
    let current: Option<AttemptCountRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.query_first(select, identity).await })
        })
        .await
        .map_err(StorageError::from)?;
    let Some(current) = current else {
        return Err(StorageError::NotFound(match kind {
            OutboxKind::Lifecycle => "memory lifecycle flight-recorder outbox event",
            OutboxKind::Commit => "memory commit flight-recorder outbox event",
        }));
    };
    let now = Datetime::from(Utc::now());
    let next_attempt_count = current.attempt_count.saturating_add(1);
    let bindings = FailureMutationBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_owned(),
        error: bounded_outbox_error(error),
        expected_attempt_count: current.attempt_count,
        next_attempt_count,
        quarantined_at: (quarantine_now || next_attempt_count >= 3).then(|| now.clone()),
        now,
    };
    let statement = match kind {
        OutboxKind::Lifecycle => {
            "UPDATE fems_memory_lifecycle_fr_outbox SET attempt_count = $next_attempt_count, \
             last_error = $error, last_error_at = $now, \
             quarantined_at = IF $quarantined_at != NONE { $quarantined_at } ELSE { quarantined_at } \
             WHERE workspace_id = $workspace AND event_id = $event_id \
             AND published_at = NONE AND attempt_count = $expected_attempt_count RETURN AFTER;"
        }
        OutboxKind::Commit => {
            "UPDATE fems_memory_commit_fr_outbox SET attempt_count = $next_attempt_count, \
             last_error = $error, last_error_at = $now, \
             quarantined_at = IF $quarantined_at != NONE { $quarantined_at } ELSE { quarantined_at } \
             WHERE workspace_id = $workspace AND event_id = $event_id \
             AND published_at = NONE AND attempt_count = $expected_attempt_count RETURN AFTER;"
        }
    };
    let count = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.execute_returning(statement, bindings).await })
        })
        .await
        .map_err(StorageError::from)?;
    if count == 1 {
        Ok(())
    } else {
        Err(StorageError::Conflict(
            "memory outbox failure attempt lost its compare-and-set",
        ))
    }
}

async fn mark_outbox_published(
    storage: &SurrealStorage,
    kind: OutboxKind,
    workspace_id: &str,
    event_id: &str,
) -> StorageResult<()> {
    let bindings = EventMutationBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        event_id: event_id.to_owned(),
        error: None,
        quarantine: false,
        now: Datetime::from(Utc::now()),
    };
    let statement = match kind {
        OutboxKind::Lifecycle => {
            "UPDATE fems_memory_lifecycle_fr_outbox SET \
             published_at = IF published_at = NONE { $now } ELSE { published_at } \
             WHERE workspace_id = $workspace AND event_id = $event_id RETURN AFTER;"
        }
        OutboxKind::Commit => {
            "UPDATE fems_memory_commit_fr_outbox SET \
             published_at = IF published_at = NONE { $now } ELSE { published_at } \
             WHERE workspace_id = $workspace AND event_id = $event_id RETURN AFTER;"
        }
    };
    let count = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.execute_returning(statement, bindings).await })
        })
        .await
        .map_err(StorageError::from)?;
    if count == 1 {
        Ok(())
    } else {
        Err(StorageError::NotFound(match kind {
            OutboxKind::Lifecycle => "memory lifecycle flight-recorder outbox event",
            OutboxKind::Commit => "memory commit flight-recorder outbox event",
        }))
    }
}

async fn select_report_by_proposal(
    storage: &SurrealStorage,
    proposal_id: &str,
) -> StorageResult<Option<ReportRow>> {
    #[derive(SurrealValue)]
    struct Binding {
        proposal: RecordId,
    }
    let bindings = Binding {
        proposal: RecordId::new(PROPOSALS_TABLE, proposal_id),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT commit_id, workspace_id, proposal_id, memory_id, report, \
                         report_hash, created_at FROM fems_memory_commit_reports \
                         WHERE proposal_id = $proposal LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn build_memory_pack(
    storage: &SurrealStorage,
    workspace_id: &str,
    generated_at: DateTime<Utc>,
    scope_ref: FemsEntityRef,
    candidate: MemoryPackItem,
) -> StorageResult<MemoryPack> {
    let rows: Vec<ItemRow> = storage
        .with_data_operation(|database| {
            Box::pin(async move { database.select_all(ITEMS_TABLE).await })
        })
        .await
        .map_err(StorageError::from)?;
    let mut invalid_items = 0usize;
    let mut items = Vec::new();
    for row in rows {
        if record_key(row.workspace_id, "memory item workspace")? != workspace_id
            || row.item.get("status").and_then(Value::as_str) == Some("inactive")
        {
            continue;
        }
        match serde_json::from_value::<MemoryPackItem>(row.item) {
            Ok(mut item) => {
                item.summary = bounded_chars(&item.summary, 240);
                item.content = bounded_chars(&item.content, 600);
                items.push(item);
            }
            Err(_) => invalid_items += 1,
        }
    }
    items.retain(|item| item.memory_id != candidate.memory_id);
    items.push(candidate);
    items.sort_by(|left, right| left.memory_id.cmp(&right.memory_id));
    let total_valid = items.len();
    items.truncate(24);
    let token_estimate = items
        .iter()
        .map(|item| ((item.summary.chars().count() + item.content.chars().count() + 3) / 4) as u32)
        .sum::<u32>()
        .min(500);
    let mut warnings = Vec::new();
    if total_valid > items.len() {
        warnings.push("memory_pack_truncated_to_24_items".to_owned());
    }
    if invalid_items > 0 {
        warnings.push(format!("ignored_{invalid_items}_invalid_memory_items"));
    }
    let generated_at = generated_at.to_rfc3339();
    let identity = json!({
        "schema_version": "hsk.memory_pack@0.1",
        "workspace_id": workspace_id,
        "generated_at": generated_at,
        "determinism_mode": MemoryPackDeterminismMode::Strict,
        "memory_policy": MemoryPolicy::WorkspaceScoped,
        "scope_refs": [scope_ref.clone()],
        "budgets": {
            "max_tokens": 500,
            "max_items": 24,
            "max_items_per_type": {},
        },
        "items": items,
        "token_estimate": token_estimate,
        "warnings": warnings,
    });
    let content_address =
        crate::llm::sha256_hex(crate::llm::canonical_json_bytes_nfc(&identity).as_slice());
    let mut pack = MemoryPack {
        schema_version: "hsk.memory_pack@0.1".to_owned(),
        pack_id: stable_uuid(&format!(
            "fems-memory-pack:{workspace_id}:{content_address}"
        ))
        .to_string(),
        generated_at,
        determinism_mode: MemoryPackDeterminismMode::Strict,
        memory_policy: MemoryPolicy::WorkspaceScoped,
        scope_refs: vec![scope_ref],
        budgets: MemoryPackBudgets {
            max_tokens: 500,
            max_items: 24,
            max_items_per_type: std::collections::BTreeMap::new(),
        },
        items,
        token_estimate,
        memory_pack_hash: String::new(),
        warnings,
    };
    pack.memory_pack_hash = pack
        .compute_hash()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(pack)
}

async fn run_commit_transaction(
    storage: &SurrealStorage,
    bindings: CommitBindings,
) -> StorageResult<()> {
    let rows: Vec<surrealdb::types::Value> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         CREATE $item_record CONTENT { memory_id: $item.memory_id, \
                            workspace_id: $item.workspace_id, item: $item.item, \
                            created_at: $item.created_at, updated_at: $item.updated_at } RETURN AFTER; \
                         CREATE $report_record CONTENT { commit_id: $report.commit_id, \
                            workspace_id: $report.workspace_id, proposal_id: $report.proposal_id, \
                            memory_id: $report.memory_id, report: $report.report, \
                            report_hash: $report.report_hash, created_at: $report.created_at }; \
                         IF array::len((UPDATE $proposal_record CONTENT { \
                            proposal_id: $proposal.proposal_id, request_id: $proposal.request_id, \
                            workspace_id: $proposal.workspace_id, document_id: $proposal.document_id, \
                            selection_start: $proposal.selection_start, selection_end: $proposal.selection_end, \
                            content_hash: $proposal.content_hash, memory_class: $proposal.memory_class, \
                            status: $proposal.status, review_gated: $proposal.review_gated, \
                            created_at: $proposal.created_at, proposal: $proposal.proposal \
                         } WHERE status = $expected_status RETURN AFTER)) != 1 { \
                            THROW 'HSK-FEMS-COMMIT-STATE'; \
                         }; \
                         CREATE $pack_record CONTENT { pack_id: $pack.pack_id, \
                            workspace_id: $pack.workspace_id, scope_key: $pack.scope_key, \
                            pack: $pack.pack, generated_at: $pack.generated_at, created_at: $pack.created_at }; \
                         CREATE $ledger.record CONTENT { event_id: $ledger.event_id, \
                            event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, \
                            session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, \
                            aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, \
                            event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, \
                            actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, \
                            correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, \
                            source_component: $ledger.source_component, payload: $ledger.payload, \
                            created_at: $ledger.created_at }; \
                         CREATE $committed_outbox.record CONTENT { event_id: $committed_outbox.event_id, \
                            workspace_id: $committed_outbox.workspace_id, proposal_id: $committed_outbox.proposal_id, \
                            commit_id: $committed_outbox.commit_id, event_code: $committed_outbox.event_code, \
                            event: $committed_outbox.event, event_hash: $committed_outbox.event_hash, \
                            created_at: $committed_outbox.created_at }; \
                         CREATE $packed_outbox.record CONTENT { event_id: $packed_outbox.event_id, \
                            workspace_id: $packed_outbox.workspace_id, proposal_id: $packed_outbox.proposal_id, \
                            commit_id: $packed_outbox.commit_id, event_code: $packed_outbox.event_code, \
                            event: $packed_outbox.event, event_hash: $packed_outbox.event_hash, \
                            created_at: $packed_outbox.created_at }; \
                         COMMIT TRANSACTION;",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| {
            if error.to_string().contains("HSK-FEMS-COMMIT-STATE") {
                StorageError::Conflict(
                    "memory proposal commit lost its approved-state transition",
                )
            } else {
                StorageError::from(error)
            }
        })?;
    if rows.len() == 1 {
        Ok(())
    } else {
        Err(StorageError::Database(
            "memory commit transaction did not create exactly one item".to_owned(),
        ))
    }
}

async fn create_commit_outbox_if_absent(
    storage: &SurrealStorage,
    workspace_id: &str,
    proposal_id: &str,
    commit_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let write = commit_outbox_write(workspace_id, proposal_id, commit_id, event_code, event)?;
    let id = write.event_id.clone();
    let content = CommitOutboxContent::from(write);
    let created: Option<OutboxRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .create_if_absent(COMMIT_OUTBOX_TABLE, &id, content)
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if created.is_none() {
        ensure_commit_outbox(
            storage,
            workspace_id,
            proposal_id,
            commit_id,
            event_code,
            event,
        )
        .await?;
    }
    Ok(())
}

async fn ensure_commit_outbox(
    storage: &SurrealStorage,
    workspace_id: &str,
    proposal_id: &str,
    commit_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let row = lookup_commit_outbox(storage, proposal_id, event_code)
        .await?
        .ok_or(StorageError::Conflict(
            "memory commit exists without its outbox event",
        ))?;
    let stored: FlightRecorderEvent = serde_json::from_value(row.event)?;
    if row.event_id != event.event_id.to_string()
        || record_key(row.workspace_id, "commit outbox workspace")? != workspace_id
        || record_key(row.proposal_id, "commit outbox proposal")? != proposal_id
        || row
            .commit_id
            .ok_or(StorageError::Conflict(
                "commit outbox is missing commit identity",
            ))
            .and_then(|record| record_key(record, "commit outbox commit"))?
            != commit_id
        || row.event_code != event_code
        || !same_memory_commit_event(&stored, event)
        || row.event_hash != event_json_hash(event)?
        || event_json_hash(&stored)? != row.event_hash
    {
        return Err(StorageError::Conflict(
            "memory commit outbox identity is bound to different evidence",
        ));
    }
    Ok(())
}

async fn select_pack(
    storage: &SurrealStorage,
    _workspace_id: &str,
    pack_id: &str,
) -> StorageResult<Option<PackRow>> {
    let id = pack_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.select_one(PACKS_TABLE, &id).await })
        })
        .await
        .map_err(StorageError::from)
}

async fn select_item(storage: &SurrealStorage, memory_id: &str) -> StorageResult<Option<ItemRow>> {
    let id = memory_id.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.select_one(ITEMS_TABLE, &id).await })
        })
        .await
        .map_err(StorageError::from)
}

async fn proposal_by_request(
    storage: &SurrealStorage,
    workspace_id: &str,
    request_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    let bindings = WorkspaceValueBinding {
        workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
        value: request_id.to_owned(),
    };
    let row: Option<ProposalRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT proposal_id, request_id, workspace_id, document_id, selection_start, \
                         selection_end, content_hash, memory_class, status, review_gated, created_at, proposal \
                         FROM fems_memory_proposals WHERE workspace_id = $workspace \
                         AND request_id = $value LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(proposal_from_row).transpose()
}

fn proposal_from_row(row: ProposalRow) -> StorageResult<StoredMemoryProposal> {
    Ok(StoredMemoryProposal {
        proposal_id: row.proposal_id,
        request_id: row.request_id,
        workspace_id: record_key(row.workspace_id, "memory proposal workspace")?,
        document_id: row.document_id,
        selection_start: row.selection_start,
        selection_end: row.selection_end,
        content_hash: row.content_hash,
        memory_class: row.memory_class,
        status: row.status,
        review_gated: row.review_gated,
        created_at: row.created_at.into_inner(),
        proposal: row.proposal,
    })
}

fn proposal_content(proposal: &StoredMemoryProposal) -> ProposalContent {
    ProposalContent {
        proposal_id: proposal.proposal_id.clone(),
        request_id: proposal.request_id.clone(),
        workspace_id: RecordId::new(WORKSPACES_TABLE, proposal.workspace_id.clone()),
        document_id: proposal.document_id.clone(),
        selection_start: proposal.selection_start,
        selection_end: proposal.selection_end,
        content_hash: proposal.content_hash.clone(),
        memory_class: proposal.memory_class.clone(),
        status: proposal.status.clone(),
        review_gated: proposal.review_gated,
        created_at: Datetime::from(proposal.created_at),
        proposal: proposal.proposal.clone(),
    }
}

fn record_key(record: RecordId, field: &str) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(format!(
            "{field} is not a string record key"
        ))),
    }
}

#[cfg(test)]
mod receipt_authenticity_tests {
    use super::*;

    fn fixture() -> (StoredMemoryProposal, KernelEvent) {
        let mut stored = StoredMemoryProposal {
            proposal_id: "PROP-authenticity".to_owned(),
            request_id: "request-authenticity".to_owned(),
            workspace_id: "workspace-authenticity".to_owned(),
            document_id: "document-authenticity".to_owned(),
            selection_start: 2,
            selection_end: 9,
            content_hash: "a".repeat(64),
            memory_class: "semantic".to_owned(),
            status: "pending_review".to_owned(),
            review_gated: true,
            created_at: Utc::now(),
            proposal: json!({
                "proposal_id": "PROP-authenticity",
                "request_id": "request-authenticity",
                "workspace_id": "workspace-authenticity",
                "actor_id": "operator-authenticity",
            }),
        };
        let receipt = NewKernelEvent::builder(
            "task-authenticity",
            "session-authenticity",
            KernelEventType::ArtifactProposed,
            KernelActor::Operator("operator-authenticity".to_owned()),
        )
        .aggregate("fems_memory_proposal", stored.proposal_id.clone())
        .idempotency_key(format!("fems-memory-proposal:{}", stored.proposal_id))
        .source_component("fems_memory_proposal_intake")
        .payload(proposal_receipt_payload(&stored))
        .build()
        .expect("valid receipt fixture");
        stamp_receipt_identity(&mut stored.proposal, &receipt)
            .expect("receipt identity stamps proposal");
        let receipt = receipt_for_stored_proposal(receipt, &stored)
            .expect("stored identity rebuilds receipt");
        let mut event = KernelEvent::from_new(receipt);
        event.event_sequence = 1;
        (stored, event)
    }

    #[test]
    fn complete_receipt_authenticity_rejects_every_immutable_dimension() {
        let (stored, event) = fixture();
        validate_existing_proposal_receipt(&event, &stored).expect("baseline authentic receipt");

        let mut mutations: Vec<(&str, KernelEvent)> = Vec::new();
        let mut changed = event.clone();
        changed.event_id = "not-a-kernel-event-id".to_owned();
        mutations.push(("event_id", changed));
        let mut changed = event.clone();
        changed.event_sequence = 0;
        mutations.push(("event_sequence", changed));
        let mut changed = event.clone();
        changed.event_version = "kernel_event_v2".to_owned();
        mutations.push(("event_version", changed));
        let mut changed = event.clone();
        changed.kernel_task_run_id = "other-task".to_owned();
        mutations.push(("kernel_task_run_id", changed));
        let mut changed = event.clone();
        changed.session_run_id = "other-session".to_owned();
        mutations.push(("session_run_id", changed));
        let mut changed = event.clone();
        changed.aggregate_type = "other".to_owned();
        mutations.push(("aggregate_type", changed));
        let mut changed = event.clone();
        changed.aggregate_id = "other".to_owned();
        mutations.push(("aggregate_id", changed));
        let mut changed = event.clone();
        changed.idempotency_key = "other".to_owned();
        mutations.push(("idempotency_key", changed));
        let mut changed = event.clone();
        changed.event_type = KernelEventType::ArtifactStored;
        mutations.push(("event_type", changed));
        let mut changed = event.clone();
        changed.actor = KernelActor::System("operator-authenticity".to_owned());
        mutations.push(("actor_kind", changed));
        let mut changed = event.clone();
        changed.actor = KernelActor::Operator("other-actor".to_owned());
        mutations.push(("actor_id", changed));
        let mut changed = event.clone();
        changed.causation_id = Some("other-cause".to_owned());
        mutations.push(("causation_id", changed));
        let mut changed = event.clone();
        changed.correlation_id = Some("other-correlation".to_owned());
        mutations.push(("correlation_id", changed));
        let mut changed = event.clone();
        changed.payload_hash = "0".repeat(64);
        mutations.push(("payload_hash", changed));
        let mut changed = event.clone();
        changed.source_component = "other-source".to_owned();
        mutations.push(("source_component", changed));
        let mut changed = event.clone();
        changed.payload["workspace_id"] = json!("other-workspace");
        mutations.push(("payload", changed));

        for (dimension, changed) in mutations {
            assert!(
                validate_existing_proposal_receipt(&changed, &stored).is_err(),
                "receipt accepted mutated {dimension}"
            );
        }
    }

    #[test]
    fn retry_headers_do_not_replace_original_receipt_identity() {
        let (stored, event) = fixture();
        let retry = NewKernelEvent::builder(
            "new-retry-task",
            "new-retry-session",
            KernelEventType::ArtifactProposed,
            KernelActor::System("new-retry-actor".to_owned()),
        )
        .build()
        .expect("valid retry receipt");
        let rebuilt =
            receipt_for_stored_proposal(retry, &stored).expect("original identity is recoverable");
        assert_eq!(rebuilt.kernel_task_run_id, event.kernel_task_run_id);
        assert_eq!(rebuilt.session_run_id, event.session_run_id);
        assert_eq!(rebuilt.actor, event.actor);
    }

    #[test]
    fn legacy_receipt_healing_is_deterministic_across_retry_headers() {
        let (mut stored, _) = fixture();
        stored
            .proposal
            .as_object_mut()
            .expect("proposal object")
            .remove("_receipt_identity");
        let first = NewKernelEvent::builder(
            "retry-task-a",
            "retry-session-a",
            KernelEventType::ArtifactProposed,
            KernelActor::Operator("retry-actor-a".to_owned()),
        )
        .build()
        .expect("first retry receipt");
        let second = NewKernelEvent::builder(
            "retry-task-b",
            "retry-session-b",
            KernelEventType::ArtifactProposed,
            KernelActor::System("retry-actor-b".to_owned()),
        )
        .build()
        .expect("second retry receipt");

        let first = receipt_for_stored_proposal(first, &stored).expect("first legacy healing");
        let second = receipt_for_stored_proposal(second, &stored).expect("second legacy healing");
        assert_eq!(first.event_version, second.event_version);
        assert_eq!(first.kernel_task_run_id, second.kernel_task_run_id);
        assert_eq!(first.session_run_id, second.session_run_id);
        assert_eq!(first.actor, second.actor);
        assert_eq!(first.causation_id, second.causation_id);
        assert_eq!(first.correlation_id, second.correlation_id);
        assert_eq!(first.payload_hash, second.payload_hash);
        assert_eq!(
            first.kernel_task_run_id,
            "native-editor-fems-propose-workspace-authenticity"
        );
        assert_eq!(first.session_run_id, "native-editor-session");
        assert_eq!(
            first.actor,
            KernelActor::Operator("operator-authenticity".to_owned())
        );
    }

    #[test]
    fn public_proposal_serialization_never_leaks_receipt_identity() {
        let (stored, _) = fixture();
        assert!(stored.proposal.get("_receipt_identity").is_some());
        let public = serde_json::to_value(&stored).expect("public proposal serializes");
        assert!(
            public["proposal"].get("_receipt_identity").is_none(),
            "storage-only receipt identity leaked through proposal serialization"
        );
        assert_eq!(public["proposal"]["request_id"], "request-authenticity");
    }

    #[test]
    fn changed_memory_ids_hash_sorts_and_deduplicates_multiple_ids() {
        let canonical = canonical_changed_memory_ids_hash(["mem-b", "mem-a", "mem-b"]);
        let reordered = canonical_changed_memory_ids_hash(["mem-a", "mem-b"]);
        let independent = hex::encode(Sha256::digest(
            crate::kernel::context_bundle::canonical_json_bytes(&json!(["mem-a", "mem-b"])),
        ));
        assert_eq!(canonical, reordered);
        assert_eq!(canonical, independent);
    }

    #[tokio::test]
    async fn memory_pack_survives_embedded_store_shutdown_and_reopen() {
        use crate::storage::{
            surreal::{SurrealDatabase, SurrealStorageConfig},
            Database, NewWorkspace, WriteContext,
        };

        let directory = tempfile::tempdir().expect("temporary MT-136 FEMS root");
        let path = directory.path().join("store");
        let config = SurrealStorageConfig::with_path(&path).expect("valid embedded test path");
        let storage = SurrealStorage::open(config.clone())
            .await
            .expect("open embedded store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded schema");
        let database = SurrealDatabase::new(storage.clone());
        let workspace = database
            .create_workspace(
                &WriteContext::human(Some("mt-136-operator".to_owned())),
                NewWorkspace {
                    name: "MT-136 FEMS Proof".to_owned(),
                },
            )
            .await
            .expect("create FEMS proof workspace");
        let mut pack = MemoryPack {
            schema_version: "hsk.memory_pack@0.1".to_owned(),
            pack_id: "MPK-mt-136-reopen".to_owned(),
            generated_at: Utc::now().to_rfc3339(),
            determinism_mode: MemoryPackDeterminismMode::Strict,
            memory_policy: MemoryPolicy::WorkspaceScoped,
            scope_refs: Vec::new(),
            budgets: MemoryPackBudgets {
                max_tokens: 256,
                max_items: 4,
                max_items_per_type: std::collections::BTreeMap::new(),
            },
            items: Vec::new(),
            token_estimate: 0,
            memory_pack_hash: String::new(),
            warnings: vec!["mt-136-close-reopen-proof".to_owned()],
        };
        pack.memory_pack_hash = pack.compute_hash().expect("hash FEMS proof pack");
        upsert_memory_pack(&storage, &workspace.id, "", &pack)
            .await
            .expect("store FEMS proof pack");
        drop(database);
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = SurrealStorage::open(config)
            .await
            .expect("reopen embedded store");
        bootstrap_schema(&reopened)
            .await
            .expect("verify reopened schema");
        let persisted = get_latest_memory_pack(&reopened, &workspace.id, None)
            .await
            .expect("read reopened FEMS pack")
            .expect("durable FEMS pack");
        assert_eq!(persisted, pack);
        reopened.shutdown().await.expect("close reopened store");
    }
}
