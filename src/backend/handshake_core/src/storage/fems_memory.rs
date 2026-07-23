//! WP-KERNEL-012 MT-109 FEMS memory-pack + review-gated proposal storage.
//!
//! Durable PostgreSQL authority for the Front End Memory System (FEMS) surfaces the
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
//! PostgreSQL/EventLedger authority only — NO SQLite. JSONB columns are written as
//! canonical text with an explicit `::jsonb` cast and read back via `::text`, mirroring
//! the kernel event-ledger append/read pattern so no sqlx jsonb-codec feature is assumed.

use chrono::{DateTime, Utc};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::ace::{
    ArtifactHandle, FemsEntityRef, FemsSourceRef, FemsSourceRefKind, MemoryCommitAppliedOp,
    MemoryCommitOpStatus, MemoryCommitReport, MemoryMutationOp, MemoryPack, MemoryPackBudgets,
    MemoryPackDeterminismMode, MemoryPackItem, MemoryPackRebuildHint, MemoryPackRebuildHintReason,
    MemoryPolicy,
};
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{StorageError, StorageResult};

/// Verify that the versioned FEMS migration has installed the required tables.
///
/// Schema evolution belongs to SQLx migrations. Keeping request-time DDL/backfills out
/// of this path makes startup and concurrent proposal intake deterministic.
pub async fn ensure_fems_memory_schema(pool: &PgPool) -> StorageResult<()> {
    let ready: bool = sqlx::query_scalar(
        r#"
        SELECT to_regclass('fems_memory_packs') IS NOT NULL
           AND to_regclass('fems_memory_proposals') IS NOT NULL
           AND to_regclass('fems_memory_items') IS NOT NULL
           AND to_regclass('fems_memory_commit_reports') IS NOT NULL
           AND to_regclass('fems_memory_commit_fr_outbox') IS NOT NULL
           AND to_regclass('fems_memory_lifecycle_fr_outbox') IS NOT NULL
           AND EXISTS (
               SELECT 1 FROM information_schema.columns
               WHERE table_schema = current_schema()
                 AND table_name = 'fems_memory_proposals'
                 AND column_name = 'request_id'
                 AND is_nullable = 'NO'
           )
           AND EXISTS (
               SELECT 1 FROM pg_constraint
               WHERE conname = 'fk_fems_memory_packs_workspace'
                 AND conrelid = to_regclass('fems_memory_packs')
                 AND confrelid = to_regclass('workspaces')
                 AND confdeltype = 'c'
           )
           AND EXISTS (
               SELECT 1 FROM pg_constraint
               WHERE conname = 'fk_fems_memory_proposals_workspace'
                 AND conrelid = to_regclass('fems_memory_proposals')
                 AND confrelid = to_regclass('workspaces')
                 AND confdeltype = 'c'
           )
           AND EXISTS (
               SELECT 1 FROM pg_constraint
               WHERE conname = 'fk_fems_memory_items_workspace'
                 AND conrelid = to_regclass('fems_memory_items')
                 AND confrelid = to_regclass('workspaces')
                 AND confdeltype = 'c'
           )
           AND EXISTS (
               SELECT 1 FROM pg_index
               WHERE indexrelid = to_regclass('idx_fems_memory_proposals_ws_request')
                 AND indrelid = to_regclass('fems_memory_proposals')
                 AND indisunique
           )
        "#,
    )
    .fetch_one(pool)
    .await?;
    if !ready {
        return Err(StorageError::Migration(
            "FEMS memory schema is missing; run versioned migrations".to_owned(),
        ));
    }
    Ok(())
}

fn to_jsonb_text<T: Serialize>(value: &T) -> StorageResult<String> {
    serde_json::to_string(value).map_err(|err| StorageError::Serialization(err.to_string()))
}

// ---------------------------------------------------------------------------
// Memory packs (AC-109-2).
// ---------------------------------------------------------------------------

/// Insert an immutable stored memory pack keyed by its content-addressed `pack_id`.
/// Exact retries are accepted, while any attempt to bind the identity to different
/// workspace, scope, or bytes fails closed.
pub async fn upsert_memory_pack(
    pool: &PgPool,
    workspace_id: &str,
    scope_key: &str,
    pack: &MemoryPack,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let pack_json = to_jsonb_text(pack)?;
    let generated_at = chrono::DateTime::parse_from_rfc3339(&pack.generated_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let result = sqlx::query(
        r#"
        INSERT INTO fems_memory_packs (pack_id, workspace_id, scope_key, pack, generated_at)
        VALUES ($1, $2, $3, $4::jsonb, $5)
        ON CONFLICT (pack_id) DO NOTHING
        "#,
    )
    .bind(&pack.pack_id)
    .bind(workspace_id)
    .bind(scope_key)
    .bind(pack_json)
    .bind(generated_at)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        let existing: Option<(String, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT workspace_id, scope_key, pack::text, generated_at FROM fems_memory_packs WHERE pack_id = $1",
        )
        .bind(&pack.pack_id)
        .fetch_optional(pool)
        .await?;
        if existing
            .as_ref()
            .is_none_or(|(owner, stored_scope, stored_pack, stored_at)| {
                owner != workspace_id
                    || stored_scope != scope_key
                    || serde_json::from_str::<MemoryPack>(stored_pack).ok() != Some(pack.clone())
                    || *stored_at != generated_at
            })
        {
            return Err(StorageError::Conflict(
                "memory pack identity is bound to different evidence",
            ));
        }
    }
    Ok(())
}

/// Fetch the most recently created memory pack for `workspace_id`, optionally preferring an exact
/// `scope_key`. The workspace-level (`scope_key=''`) pack is also eligible for a context request so a
/// newer approved-memory commit supersedes an older context-specific empty projection.
pub async fn get_latest_memory_pack(
    pool: &PgPool,
    workspace_id: &str,
    scope_key: Option<&str>,
) -> StorageResult<Option<MemoryPack>> {
    ensure_fems_memory_schema(pool).await?;
    let row = if let Some(scope) = scope_key.filter(|s| !s.trim().is_empty()) {
        sqlx::query(
            r#"
            SELECT pack::text AS pack
            FROM fems_memory_packs
            WHERE workspace_id = $1 AND scope_key IN ($2, '')
            ORDER BY created_at DESC,
                     CASE WHEN scope_key = $2 THEN 0 ELSE 1 END,
                     pack_id DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(scope)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT pack::text AS pack
            FROM fems_memory_packs
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
    };

    match row {
        Some(row) => {
            let pack_text: String = row.try_get("pack")?;
            let pack: MemoryPack = serde_json::from_str(&pack_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(pack))
        }
        None => Ok(None),
    }
}

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
    let mut proposal = proposal.clone();
    if let Value::Object(object) = &mut proposal {
        object.remove("_receipt_identity");
        object.remove("review");
        object.insert("status".to_owned(), json!("pending_review"));
    }
    proposal
}

/// Atomically insert a review-gated proposal and its canonical EventLedger receipt.
/// Always stored as `status='pending_review'`; this function NEVER writes to
/// `fems_memory_items` (the never-editor-direct invariant). Replaying the same
/// workspace/request identity returns the original row and receipt without duplication.
pub async fn insert_memory_proposal_with_receipt(
    pool: &PgPool,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
) -> StorageResult<StoredMemoryProposal> {
    insert_memory_proposal_with_receipt_inner(pool, proposal, receipt, false).await
}

async fn insert_memory_proposal_with_receipt_inner(
    pool: &PgPool,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
    force_failure_after_proposal_insert: bool,
) -> StorageResult<StoredMemoryProposal> {
    ensure_fems_memory_schema(pool).await?;
    let mut proposal = proposal.clone();
    stamp_receipt_identity(&mut proposal.proposal, &receipt)?;
    let proposal_json = to_jsonb_text(&proposal.proposal)?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO fems_memory_proposals (
            proposal_id, request_id, workspace_id, document_id, selection_start, selection_end,
            content_hash, memory_class, status, review_gated, proposal
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb)
        ON CONFLICT (workspace_id, request_id) DO NOTHING
        "#,
    )
    .bind(&proposal.proposal_id)
    .bind(&proposal.request_id)
    .bind(&proposal.workspace_id)
    .bind(&proposal.document_id)
    .bind(proposal.selection_start)
    .bind(proposal.selection_end)
    .bind(&proposal.content_hash)
    .bind(&proposal.memory_class)
    .bind(&proposal.status)
    .bind(proposal.review_gated)
    .bind(proposal_json)
    .execute(&mut *tx)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23503") =>
        {
            StorageError::NotFound("workspace")
        }
        _ => StorageError::from(error),
    })?;

    let stored = get_memory_proposal_by_request_with_executor(
        &mut tx,
        &proposal.workspace_id,
        &proposal.request_id,
    )
    .await?
    .ok_or(StorageError::Database(
        "proposal insert/readback produced no row".to_owned(),
    ))?;
    if !same_logical_proposal(&stored, &proposal) {
        return Err(StorageError::Conflict(
            "memory proposal request_id was reused with a different payload",
        ));
    }
    if force_failure_after_proposal_insert {
        return Err(StorageError::Database(
            "forced failure after proposal insert".to_owned(),
        ));
    }
    let receipt = receipt_for_stored_proposal(receipt, &stored)?;
    let existing_receipt = sqlx::query(
        r#"
            SELECT
                event_id,
                event_sequence,
                event_version,
                kernel_task_run_id,
                session_run_id,
                aggregate_type,
                aggregate_id,
                idempotency_key,
                event_type,
                actor_kind,
                actor_id,
                causation_id,
                correlation_id,
                payload_hash,
                source_component,
                payload::text AS payload,
                created_at
            FROM kernel_event_ledger
            WHERE idempotency_key = $1
            "#,
    )
    .bind(&receipt.idempotency_key)
    .fetch_optional(&mut *tx)
    .await?;
    let persisted_receipt = if let Some(row) = existing_receipt {
        let existing = crate::storage::postgres::map_kernel_event(row)?;
        validate_existing_proposal_receipt(&existing, &stored)?;
        existing
    } else {
        crate::storage::postgres::append_kernel_event_with_executor(&mut *tx, receipt).await?
    };
    let flight_recorder_event =
        build_memory_proposal_flight_recorder_event(&stored, &persisted_receipt)?;
    store_memory_lifecycle_outbox_event_with_executor(
        &mut tx,
        &stored.workspace_id,
        &stored.proposal_id,
        "FR-EVT-MEM-001",
        &flight_recorder_event,
    )
    .await?;
    tx.commit().await?;
    Ok(stored)
}

/// Atomically move a proposal out of `pending_review` and append the matching durable
/// EventLedger decision receipt. Exact retries return the original transition; a different
/// decision or reviewer identity is a conflict and cannot rewrite the audit record.
pub async fn review_memory_proposal_with_receipt(
    pool: &PgPool,
    workspace_id: &str,
    proposal_id: &str,
    review: &MemoryProposalReview,
    mut receipt: NewKernelEvent,
) -> StorageResult<MemoryProposalReviewResult> {
    ensure_fems_memory_schema(pool).await?;
    let target_status = match review.decision.as_str() {
        "approved" => "approved",
        "rejected" => "rejected",
        _ => {
            return Err(StorageError::Validation(
                "memory proposal review decision must be approved or rejected",
            ))
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
            .is_some_and(|reason| reason.trim().is_empty() || reason.len() > 1000)
    {
        return Err(StorageError::Validation(
            "memory proposal review identity is invalid",
        ));
    }

    let mut tx = pool.begin().await?;
    let mut stored =
        get_memory_proposal_for_review_with_executor(&mut tx, workspace_id, proposal_id)
            .await?
            .ok_or(StorageError::NotFound("memory proposal in workspace"))?;

    let expected_review = json!({
        "decision": review.decision,
        "reviewer_kind": review.reviewer_kind,
        "actor_kind": review.actor_kind,
        "actor_id": review.actor_id,
        "reason": review.reason,
        "correlation_id": review.correlation_id,
    });

    let reviewed_at = if stored.status == "pending_review" {
        let reviewed_at = Utc::now();
        let Value::Object(proposal) = &mut stored.proposal else {
            return Err(StorageError::Conflict(
                "memory proposal payload is not an object",
            ));
        };
        let mut persisted_review = expected_review.clone();
        persisted_review
            .as_object_mut()
            .ok_or(StorageError::Serialization(
                "memory review metadata was not an object".to_owned(),
            ))?
            .insert("reviewed_at".to_owned(), json!(reviewed_at));
        proposal.insert("review".to_owned(), persisted_review);
        proposal.insert("status".to_owned(), json!(target_status));
        stored.status = target_status.to_owned();
        let proposal_json = to_jsonb_text(&stored.proposal)?;
        sqlx::query(
            r#"
            UPDATE fems_memory_proposals
            SET status = $1, proposal = $2::jsonb
            WHERE workspace_id = $3 AND proposal_id = $4 AND status = 'pending_review'
            "#,
        )
        .bind(target_status)
        .bind(proposal_json)
        .bind(workspace_id)
        .bind(proposal_id)
        .execute(&mut *tx)
        .await?;
        reviewed_at
    } else if stored.status == target_status
        || (stored.status == "committed" && target_status == "approved")
    {
        let persisted_review = stored
            .proposal
            .get("review")
            .and_then(Value::as_object)
            .ok_or(StorageError::Conflict(
                "reviewed memory proposal is missing review evidence",
            ))?;
        let mut comparable = Value::Object(persisted_review.clone());
        let reviewed_at = comparable
            .as_object_mut()
            .and_then(|review| review.remove("reviewed_at"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
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
    let receipt =
        crate::storage::postgres::append_kernel_event_with_executor(&mut *tx, receipt).await?;
    let flight_recorder_event =
        build_memory_review_flight_recorder_event(&stored, &receipt, reviewed_at)?;
    store_memory_lifecycle_outbox_event_with_executor(
        &mut tx,
        &stored.workspace_id,
        &stored.proposal_id,
        "FR-EVT-MEM-002",
        &flight_recorder_event,
    )
    .await?;
    tx.commit().await?;

    Ok(MemoryProposalReviewResult {
        proposal: stored,
        receipt,
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
                ))
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
        receipt.kernel_task_run_id = format!("native-editor-fems-propose-{}", stored.workspace_id);
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

#[cfg(test)]
pub(crate) async fn insert_memory_proposal_with_receipt_forced_failure(
    pool: &PgPool,
    proposal: &StoredMemoryProposal,
    receipt: NewKernelEvent,
) -> StorageResult<StoredMemoryProposal> {
    insert_memory_proposal_with_receipt_inner(pool, proposal, receipt, true).await
}

/// Read a stored proposal by id (used by the AC-109-3 proofs).
pub async fn get_memory_proposal(
    pool: &PgPool,
    proposal_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    ensure_fems_memory_schema(pool).await?;
    let row = sqlx::query(
        r#"
        SELECT proposal_id, request_id, workspace_id, document_id, selection_start, selection_end,
               content_hash, memory_class, status, review_gated, created_at,
               proposal::text AS proposal
        FROM fems_memory_proposals
        WHERE proposal_id = $1
        "#,
    )
    .bind(proposal_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let proposal_text: String = row.try_get("proposal")?;
            let proposal_value: Value = serde_json::from_str(&proposal_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(StoredMemoryProposal {
                proposal_id: row.try_get("proposal_id")?,
                request_id: row.try_get("request_id")?,
                workspace_id: row.try_get("workspace_id")?,
                document_id: row.try_get("document_id")?,
                selection_start: row.try_get("selection_start")?,
                selection_end: row.try_get("selection_end")?,
                content_hash: row.try_get("content_hash")?,
                memory_class: row.try_get("memory_class")?,
                status: row.try_get("status")?,
                review_gated: row.try_get("review_gated")?,
                created_at: row.try_get("created_at")?,
                proposal: proposal_value,
            }))
        }
        None => Ok(None),
    }
}

/// List a bounded workspace projection of actionable memory proposals. Approved proposals sort
/// first so an interrupted review->commit sequence is recovered before new pending reviews.
pub async fn list_memory_proposals(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<StoredMemoryProposal>> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT proposal_id, request_id, workspace_id, document_id, selection_start, selection_end,
               content_hash, memory_class, status, review_gated, created_at,
               proposal::text AS proposal
        FROM fems_memory_proposals
        WHERE workspace_id = $1 AND status IN ('pending_review', 'approved')
        ORDER BY CASE WHEN status = 'approved' THEN 0 ELSE 1 END, created_at DESC, proposal_id DESC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_memory_proposal_row).collect()
}

async fn get_memory_proposal_by_request_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    request_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    let row = sqlx::query(
        r#"
        SELECT proposal_id, request_id, workspace_id, document_id, selection_start, selection_end,
               content_hash, memory_class, status, review_gated, created_at,
               proposal::text AS proposal
        FROM fems_memory_proposals
        WHERE workspace_id = $1 AND request_id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(request_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(map_memory_proposal_row).transpose()
}

async fn get_memory_proposal_for_review_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    proposal_id: &str,
) -> StorageResult<Option<StoredMemoryProposal>> {
    let row = sqlx::query(
        r#"
        SELECT proposal_id, request_id, workspace_id, document_id, selection_start, selection_end,
               content_hash, memory_class, status, review_gated, created_at,
               proposal::text AS proposal
        FROM fems_memory_proposals
        WHERE workspace_id = $1 AND proposal_id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(proposal_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(map_memory_proposal_row).transpose()
}

fn map_memory_proposal_row(row: sqlx::postgres::PgRow) -> StorageResult<StoredMemoryProposal> {
    let proposal_text: String = row.try_get("proposal")?;
    let proposal_value: Value = serde_json::from_str(&proposal_text)
        .map_err(|err| StorageError::Serialization(err.to_string()))?;
    Ok(StoredMemoryProposal {
        proposal_id: row.try_get("proposal_id")?,
        request_id: row.try_get("request_id")?,
        workspace_id: row.try_get("workspace_id")?,
        document_id: row.try_get("document_id")?,
        selection_start: row.try_get("selection_start")?,
        selection_end: row.try_get("selection_end")?,
        content_hash: row.try_get("content_hash")?,
        memory_class: row.try_get("memory_class")?,
        status: row.try_get("status")?,
        review_gated: row.try_get("review_gated")?,
        created_at: row.try_get("created_at")?,
        proposal: proposal_value,
    })
}

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

fn memory_proposal_artifact_id(proposal_id: &str) -> uuid::Uuid {
    stable_uuid(&format!("fems:proposals:{proposal_id}"))
}

fn memory_proposal_artifact_path(workspace_id: &str, proposal_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/proposals/{proposal_id}/artifact")
}

fn build_memory_proposal_flight_recorder_event(
    proposal: &StoredMemoryProposal,
    receipt: &KernelEvent,
) -> StorageResult<FlightRecorderEvent> {
    let artifact = proposal_artifact_value(&proposal.proposal);
    let proposal_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&artifact),
    );
    let workspace_uuid = uuid::Uuid::parse_str(&proposal.workspace_id)
        .map_err(|_| StorageError::Conflict("memory proposal workspace id is not a UUID"))?;
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::MemoryWriteProposed,
        flight_recorder_actor(receipt),
        stable_uuid(&format!("fems-memory-proposal:{}", proposal.proposal_id)),
        json!({
            "type": "memory_write_proposed",
            "event_code": "FR-EVT-MEM-001",
            "proposal_id": proposal.proposal_id,
            "proposal_hash": proposal_hash,
            "artifact_ref": {
                "artifact_id": memory_proposal_artifact_id(&proposal.proposal_id),
                "path": memory_proposal_artifact_path(&proposal.workspace_id, &proposal.proposal_id),
            },
            "scope_refs": [{
                "artefact_type": "workspace",
                "artefact_id": workspace_uuid,
                "selector": "self",
            }],
            "op_count": 1,
            "requires_review_count": 1,
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

fn memory_commit_report_artifact_id(commit_id: &str) -> uuid::Uuid {
    stable_uuid(&format!("fems:commits:{commit_id}"))
}

fn memory_commit_report_artifact_path(workspace_id: &str, commit_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/commits/{commit_id}/report")
}

fn memory_pack_artifact_handle(pack_id: &str) -> ArtifactHandle {
    ArtifactHandle::new(
        stable_uuid(&format!("fems:packs:{pack_id}")),
        format!(".handshake/fems/packs/{pack_id}.json"),
    )
}

fn event_json_hash(event: &FlightRecorderEvent) -> StorageResult<String> {
    let value = serde_json::to_value(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&value),
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
    let changed_memory_ids_hash = hex::encode(Sha256::digest(
        crate::kernel::context_bundle::canonical_json_bytes(&json!([memory_id])),
    ));
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
            "artifact_ref": {
                "artifact_id": memory_commit_report_artifact_id(&commit_report.commit_id),
                "path": memory_commit_report_artifact_path(workspace_id, &commit_report.commit_id),
            },
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
            "artifact_ref": memory_pack_artifact_handle(&pack.pack_id),
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

async fn load_memory_commit_receipt_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    proposal_id: &str,
) -> StorageResult<KernelEvent> {
    let row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id,
               aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id,
               causation_id, correlation_id, payload_hash, source_component,
               payload::text AS payload, created_at
        FROM kernel_event_ledger
        WHERE idempotency_key = $1
        "#,
    )
    .bind(format!("fems-memory-commit:{proposal_id}"))
    .fetch_one(&mut **tx)
    .await?;
    crate::storage::postgres::map_kernel_event(row)
}

async fn load_memory_pack_by_id_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    pack_id: &str,
) -> StorageResult<MemoryPack> {
    let row = sqlx::query(
        "SELECT pack::text AS pack FROM fems_memory_packs WHERE workspace_id = $1 AND pack_id = $2",
    )
    .bind(workspace_id)
    .bind(pack_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(StorageError::Conflict(
        "memory commit receipt references a missing memory pack",
    ))?;
    let pack_text: String = row.try_get("pack")?;
    let pack: MemoryPack = serde_json::from_str(&pack_text)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let computed_hash = pack
        .compute_hash()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    if computed_hash != pack.memory_pack_hash {
        return Err(StorageError::Conflict(
            "memory commit receipt references a corrupt memory pack",
        ));
    }
    Ok(pack)
}

async fn store_memory_lifecycle_outbox_event_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    proposal_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let event_hash = event_json_hash(event)?;
    let event_json = to_jsonb_text(event)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO fems_memory_lifecycle_fr_outbox
            (event_id, workspace_id, proposal_id, event_code, event, event_hash, created_at)
        VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7)
        ON CONFLICT (proposal_id, event_code) DO NOTHING
        "#,
    )
    .bind(event.event_id.to_string())
    .bind(workspace_id)
    .bind(proposal_id)
    .bind(event_code)
    .bind(event_json)
    .bind(&event_hash)
    .bind(event.timestamp)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(());
    }
    let row = sqlx::query(
        r#"
        SELECT event_id, workspace_id, event::text AS event, event_hash
        FROM fems_memory_lifecycle_fr_outbox
        WHERE proposal_id = $1 AND event_code = $2
        FOR UPDATE
        "#,
    )
    .bind(proposal_id)
    .bind(event_code)
    .fetch_one(&mut **tx)
    .await?;
    let stored_event_text: String = row.try_get("event")?;
    let stored_event: FlightRecorderEvent = serde_json::from_str(&stored_event_text)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let stored_hash: String = row.try_get("event_hash")?;
    if row.try_get::<String, _>("event_id")? != event.event_id.to_string()
        || row.try_get::<String, _>("workspace_id")? != workspace_id
        || !same_memory_commit_event(&stored_event, event)
        || stored_hash != event_hash
        || event_json_hash(&stored_event)? != stored_hash
    {
        return Err(StorageError::Conflict(
            "memory lifecycle outbox identity is bound to different evidence",
        ));
    }
    Ok(())
}

async fn store_memory_commit_outbox_event_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    proposal_id: &str,
    commit_id: &str,
    event_code: &str,
    event: &FlightRecorderEvent,
) -> StorageResult<()> {
    let event_hash = event_json_hash(event)?;
    let event_json = to_jsonb_text(event)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO fems_memory_commit_fr_outbox
            (event_id, workspace_id, proposal_id, commit_id, event_code, event, event_hash, created_at)
        VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8)
        ON CONFLICT (proposal_id, event_code) DO NOTHING
        "#,
    )
    .bind(event.event_id.to_string())
    .bind(workspace_id)
    .bind(proposal_id)
    .bind(commit_id)
    .bind(event_code)
    .bind(event_json)
    .bind(&event_hash)
    .bind(event.timestamp)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(());
    }

    let row = sqlx::query(
        r#"
        SELECT event_id, workspace_id, commit_id, event::text AS event, event_hash
        FROM fems_memory_commit_fr_outbox
        WHERE proposal_id = $1 AND event_code = $2
        FOR UPDATE
        "#,
    )
    .bind(proposal_id)
    .bind(event_code)
    .fetch_one(&mut **tx)
    .await?;
    let stored_event_text: String = row.try_get("event")?;
    let stored_event: FlightRecorderEvent = serde_json::from_str(&stored_event_text)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let stored_hash: String = row.try_get("event_hash")?;
    if row.try_get::<String, _>("event_id")? != event.event_id.to_string()
        || row.try_get::<String, _>("workspace_id")? != workspace_id
        || row.try_get::<String, _>("commit_id")? != commit_id
        || !same_memory_commit_event(&stored_event, event)
        || stored_hash != event_hash
        || event_json_hash(&stored_event)? != stored_hash
    {
        return Err(StorageError::Conflict(
            "memory commit outbox identity is bound to different evidence",
        ));
    }
    Ok(())
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

const OUTBOX_QUARANTINE_AFTER_ATTEMPTS: i64 = 3;
const OUTBOX_ERROR_MAX_CHARS: usize = 1_000;

fn bounded_outbox_error(error: &str) -> String {
    error.chars().take(OUTBOX_ERROR_MAX_CHARS).collect()
}

async fn record_memory_lifecycle_event_failure_by_id(
    pool: &PgPool,
    workspace_id: &str,
    event_id: &str,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let updated = sqlx::query(
        r#"
        UPDATE fems_memory_lifecycle_fr_outbox
        SET attempt_count = attempt_count + 1,
            last_error = $3,
            last_error_at = now(),
            quarantined_at = CASE
                WHEN $4 OR attempt_count + 1 >= $5 THEN COALESCE(quarantined_at, now())
                ELSE quarantined_at
            END
        WHERE workspace_id = $1 AND event_id = $2 AND published_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(event_id)
    .bind(bounded_outbox_error(error))
    .bind(quarantine_now)
    .bind(OUTBOX_QUARANTINE_AFTER_ATTEMPTS)
    .execute(pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(StorageError::NotFound(
            "memory lifecycle flight-recorder outbox event",
        ));
    }
    Ok(())
}

pub async fn record_memory_lifecycle_event_failure(
    pool: &PgPool,
    workspace_id: &str,
    event_id: uuid::Uuid,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    record_memory_lifecycle_event_failure_by_id(
        pool,
        workspace_id,
        &event_id.to_string(),
        error,
        quarantine_now,
    )
    .await
}

async fn decode_lifecycle_outbox_rows(
    pool: &PgPool,
    rows: Vec<sqlx::postgres::PgRow>,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let event_id_text: String = row.try_get("event_id")?;
        let workspace_id: String = row.try_get("workspace_id")?;
        let decoded_event = (|| -> StorageResult<FlightRecorderEvent> {
            let event_id = uuid::Uuid::parse_str(&event_id_text)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            let event_text: String = row.try_get("event")?;
            let stored_hash: String = row.try_get("event_hash")?;
            let event: FlightRecorderEvent = serde_json::from_str(&event_text)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            if event.event_id != event_id || event_json_hash(&event)? != stored_hash {
                return Err(StorageError::Conflict(
                    "memory lifecycle outbox event hash or identity does not match its envelope",
                ));
            }
            event
                .validate()
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            Ok(event)
        })();
        match decoded_event {
            Ok(event) => decoded.push((workspace_id, event)),
            Err(error) => {
                record_memory_lifecycle_event_failure_by_id(
                    pool,
                    &workspace_id,
                    &event_id_text,
                    &error.to_string(),
                    true,
                )
                .await?;
                tracing::error!(
                    target: "handshake_core::fems_memory",
                    workspace_id,
                    event_id = %event_id_text,
                    error = %error,
                    "fems_memory_lifecycle_outbox_event_quarantined"
                );
            }
        }
    }
    Ok(decoded)
}

pub async fn list_all_pending_memory_lifecycle_events(
    pool: &PgPool,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT event_id, workspace_id, event::text AS event, event_hash
        FROM fems_memory_lifecycle_fr_outbox
        WHERE published_at IS NULL AND quarantined_at IS NULL
        ORDER BY created_at ASC, event_id ASC
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    decode_lifecycle_outbox_rows(pool, rows).await
}

pub async fn list_pending_memory_lifecycle_events(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FlightRecorderEvent>> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT event_id, workspace_id, event::text AS event, event_hash
        FROM fems_memory_lifecycle_fr_outbox
        WHERE workspace_id = $1 AND published_at IS NULL AND quarantined_at IS NULL
        ORDER BY created_at ASC, event_id ASC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(decode_lifecycle_outbox_rows(pool, rows)
        .await?
        .into_iter()
        .map(|(_, event)| event)
        .collect())
}

pub async fn mark_memory_lifecycle_event_published(
    pool: &PgPool,
    workspace_id: &str,
    event_id: uuid::Uuid,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let updated = sqlx::query(
        r#"
        UPDATE fems_memory_lifecycle_fr_outbox
        SET published_at = COALESCE(published_at, GREATEST(now(), created_at))
        WHERE workspace_id = $1 AND event_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(event_id.to_string())
    .execute(pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(StorageError::NotFound(
            "memory lifecycle flight-recorder outbox event",
        ));
    }
    Ok(())
}

async fn load_kernel_event_by_idempotency_key(
    pool: &PgPool,
    idempotency_key: &str,
) -> StorageResult<KernelEvent> {
    let row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id,
               aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id,
               causation_id, correlation_id, payload_hash, source_component,
               payload::text AS payload, created_at
        FROM kernel_event_ledger
        WHERE idempotency_key = $1
        "#,
    )
    .bind(idempotency_key)
    .fetch_one(pool)
    .await?;
    crate::storage::postgres::map_kernel_event(row)
}

/// Restart/upgrade healing for proposal and review rows created before their transactional outbox
/// landed, and for fault-injection tests that remove an outbox row after the authoritative commit.
pub async fn recover_missing_memory_lifecycle_outbox_events(pool: &PgPool) -> StorageResult<u64> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT proposal.proposal_id, proposal.workspace_id,
               proposal.proposal ? 'review' AS has_review,
               proposed.event_id IS NULL AS missing_proposed,
               reviewed.event_id IS NULL AS missing_reviewed
        FROM fems_memory_proposals proposal
        LEFT JOIN fems_memory_lifecycle_fr_outbox proposed
          ON proposed.proposal_id = proposal.proposal_id
         AND proposed.event_code = 'FR-EVT-MEM-001'
        LEFT JOIN fems_memory_lifecycle_fr_outbox reviewed
          ON reviewed.proposal_id = proposal.proposal_id
         AND reviewed.event_code = 'FR-EVT-MEM-002'
        WHERE proposed.event_id IS NULL
           OR (proposal.proposal ? 'review' AND reviewed.event_id IS NULL)
        ORDER BY proposal.created_at ASC, proposal.proposal_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    let mut recovered = 0u64;
    for row in rows {
        let proposal_id: String = row.try_get("proposal_id")?;
        let workspace_id: String = row.try_get("workspace_id")?;
        let has_review: bool = row.try_get("has_review")?;
        let missing_proposed: bool = row.try_get("missing_proposed")?;
        let missing_reviewed: bool = row.try_get("missing_reviewed")?;
        let stored = get_memory_proposal(pool, &proposal_id)
            .await?
            .ok_or(StorageError::NotFound("memory proposal"))?;
        // Resolve the authoritative EventLedger receipts before opening the write
        // transaction. This recovery path must also work with a one-connection pool.
        let proposed_event = if missing_proposed {
            let receipt = load_kernel_event_by_idempotency_key(
                pool,
                &format!("fems-memory-proposal:{proposal_id}"),
            )
            .await?;
            Some(build_memory_proposal_flight_recorder_event(
                &stored, &receipt,
            )?)
        } else {
            None
        };
        let reviewed_event = if has_review && missing_reviewed {
            let receipt = load_kernel_event_by_idempotency_key(
                pool,
                &format!("fems-memory-proposal-review:{proposal_id}"),
            )
            .await?;
            let reviewed_at = stored
                .proposal
                .get("review")
                .and_then(Value::as_object)
                .and_then(|review| review.get("reviewed_at"))
                .and_then(Value::as_str)
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc))
                .ok_or(StorageError::Conflict(
                    "reviewed memory proposal has invalid review timestamp",
                ))?;
            Some(build_memory_review_flight_recorder_event(
                &stored,
                &receipt,
                reviewed_at,
            )?)
        } else {
            None
        };
        let mut tx = pool.begin().await?;
        if let Some(event) = proposed_event {
            store_memory_lifecycle_outbox_event_with_executor(
                &mut tx,
                &workspace_id,
                &proposal_id,
                "FR-EVT-MEM-001",
                &event,
            )
            .await?;
            recovered += 1;
        }
        if let Some(event) = reviewed_event {
            store_memory_lifecycle_outbox_event_with_executor(
                &mut tx,
                &workspace_id,
                &proposal_id,
                "FR-EVT-MEM-002",
                &event,
            )
            .await?;
            recovered += 1;
        }
        tx.commit().await?;
    }
    Ok(recovered)
}

pub async fn list_pending_memory_commit_events(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FlightRecorderEvent>> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT event_id, workspace_id, event::text AS event, event_hash
        FROM fems_memory_commit_fr_outbox current_event
        WHERE workspace_id = $1
          AND published_at IS NULL
          AND quarantined_at IS NULL
          AND (
              event_code = 'FR-EVT-MEM-003'
              OR EXISTS (
                  SELECT 1 FROM fems_memory_commit_fr_outbox committed
                  WHERE committed.commit_id = current_event.commit_id
                    AND committed.event_code = 'FR-EVT-MEM-003'
                    AND committed.published_at IS NOT NULL
              )
          )
        ORDER BY created_at ASC,
                 CASE event_code WHEN 'FR-EVT-MEM-003' THEN 0 ELSE 1 END,
                 event_id ASC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(decode_commit_outbox_rows(pool, rows)
        .await?
        .into_iter()
        .map(|(_, event)| event)
        .collect())
}

pub async fn list_all_pending_memory_commit_events(
    pool: &PgPool,
    limit: i64,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT event_id, workspace_id, event::text AS event, event_hash
        FROM fems_memory_commit_fr_outbox current_event
        WHERE published_at IS NULL
          AND quarantined_at IS NULL
          AND (
              event_code = 'FR-EVT-MEM-003'
              OR EXISTS (
                  SELECT 1 FROM fems_memory_commit_fr_outbox committed
                  WHERE committed.commit_id = current_event.commit_id
                    AND committed.event_code = 'FR-EVT-MEM-003'
                    AND committed.published_at IS NOT NULL
              )
          )
        ORDER BY created_at ASC,
                 CASE event_code WHEN 'FR-EVT-MEM-003' THEN 0 ELSE 1 END,
                 event_id ASC
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    decode_commit_outbox_rows(pool, rows).await
}

async fn record_memory_commit_event_failure_by_id(
    pool: &PgPool,
    workspace_id: &str,
    event_id: &str,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let updated = sqlx::query(
        r#"
        UPDATE fems_memory_commit_fr_outbox
        SET attempt_count = attempt_count + 1,
            last_error = $3,
            last_error_at = now(),
            quarantined_at = CASE
                WHEN $4 OR attempt_count + 1 >= $5 THEN COALESCE(quarantined_at, now())
                ELSE quarantined_at
            END
        WHERE workspace_id = $1 AND event_id = $2 AND published_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(event_id)
    .bind(bounded_outbox_error(error))
    .bind(quarantine_now)
    .bind(OUTBOX_QUARANTINE_AFTER_ATTEMPTS)
    .execute(pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(StorageError::NotFound(
            "memory commit flight-recorder outbox event",
        ));
    }
    Ok(())
}

pub async fn record_memory_commit_event_failure(
    pool: &PgPool,
    workspace_id: &str,
    event_id: uuid::Uuid,
    error: &str,
    quarantine_now: bool,
) -> StorageResult<()> {
    record_memory_commit_event_failure_by_id(
        pool,
        workspace_id,
        &event_id.to_string(),
        error,
        quarantine_now,
    )
    .await
}

async fn decode_commit_outbox_rows(
    pool: &PgPool,
    rows: Vec<sqlx::postgres::PgRow>,
) -> StorageResult<Vec<(String, FlightRecorderEvent)>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let event_id_text: String = row.try_get("event_id")?;
        let workspace_id: String = row.try_get("workspace_id")?;
        let decoded_event = (|| -> StorageResult<FlightRecorderEvent> {
            let event_id = uuid::Uuid::parse_str(&event_id_text)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            let event_text: String = row.try_get("event")?;
            let stored_hash: String = row.try_get("event_hash")?;
            let event: FlightRecorderEvent = serde_json::from_str(&event_text)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            if event.event_id != event_id || event_json_hash(&event)? != stored_hash {
                return Err(StorageError::Conflict(
                    "memory commit outbox event hash or identity does not match its envelope",
                ));
            }
            event
                .validate()
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            Ok(event)
        })();
        match decoded_event {
            Ok(event) => decoded.push((workspace_id, event)),
            Err(error) => {
                record_memory_commit_event_failure_by_id(
                    pool,
                    &workspace_id,
                    &event_id_text,
                    &error.to_string(),
                    true,
                )
                .await?;
                tracing::error!(
                    target: "handshake_core::fems_memory",
                    workspace_id,
                    event_id = %event_id_text,
                    error = %error,
                    "fems_memory_commit_outbox_event_quarantined"
                );
            }
        }
    }
    Ok(decoded)
}

/// Upgrade/restart recovery for commits written before the transactional outbox migration, or for
/// any row whose projection envelope was deliberately removed by a fault-injection proof. The
/// existing-commit branch reconstructs the envelope only from immutable report/EventLedger/pack
/// evidence; the placeholder receipt below is never used for an already committed proposal.
pub async fn recover_missing_memory_commit_outbox_events(pool: &PgPool) -> StorageResult<u64> {
    ensure_fems_memory_schema(pool).await?;
    let rows = sqlx::query(
        r#"
        SELECT report.workspace_id, report.proposal_id
        FROM fems_memory_commit_reports report
        LEFT JOIN fems_memory_commit_fr_outbox committed
          ON committed.proposal_id = report.proposal_id
         AND committed.event_code = 'FR-EVT-MEM-003'
        LEFT JOIN fems_memory_commit_fr_outbox packed
          ON packed.proposal_id = report.proposal_id
         AND packed.event_code = 'FR-EVT-MEM-004'
        WHERE committed.proposal_id IS NULL OR packed.proposal_id IS NULL
        ORDER BY report.created_at ASC, report.commit_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    let mut recovered = 0u64;
    for row in rows {
        let workspace_id: String = row.try_get("workspace_id")?;
        let proposal_id: String = row.try_get("proposal_id")?;
        let placeholder_receipt = NewKernelEvent::builder(
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
        commit_memory_proposal_with_receipt(pool, &workspace_id, &proposal_id, placeholder_receipt)
            .await?;
        recovered += 1;
    }
    Ok(recovered)
}

pub async fn mark_memory_commit_event_published(
    pool: &PgPool,
    workspace_id: &str,
    event_id: uuid::Uuid,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let updated = sqlx::query(
        r#"
        UPDATE fems_memory_commit_fr_outbox
        SET published_at = COALESCE(published_at, GREATEST(now(), created_at))
        WHERE workspace_id = $1 AND event_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(event_id.to_string())
    .execute(pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(StorageError::NotFound(
            "memory commit flight-recorder outbox event",
        ));
    }
    Ok(())
}

pub async fn get_memory_commit_report(
    pool: &PgPool,
    workspace_id: &str,
    commit_id: &str,
) -> StorageResult<Option<MemoryCommitReport>> {
    ensure_fems_memory_schema(pool).await?;
    let row = sqlx::query(
        r#"
        SELECT report::text AS report, report_hash
        FROM fems_memory_commit_reports
        WHERE workspace_id = $1 AND commit_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(commit_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        let report_text: String = row.try_get("report")?;
        let stored_hash: String = row.try_get("report_hash")?;
        let report: MemoryCommitReport = serde_json::from_str(&report_text)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let computed_hash = report
            .compute_hash()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        if report.commit_id != commit_id || computed_hash != stored_hash {
            return Err(StorageError::Conflict(
                "memory commit report artifact failed identity or hash validation",
            ));
        }
        Ok(report)
    })
    .transpose()
}

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

async fn build_memory_pack_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    generated_at: DateTime<Utc>,
    scope_ref: FemsEntityRef,
) -> StorageResult<MemoryPack> {
    let rows = sqlx::query(
        r#"
        SELECT item::text AS item
        FROM fems_memory_items
        WHERE workspace_id = $1 AND COALESCE(item ->> 'status', 'active') = 'active'
        ORDER BY memory_id ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(&mut **tx)
    .await?;
    let total = rows.len();
    let mut invalid_items = 0usize;
    let mut items = Vec::new();
    for row in rows {
        let text: String = row.try_get("item")?;
        match serde_json::from_str::<MemoryPackItem>(&text) {
            Ok(mut item) => {
                item.summary = bounded_chars(&item.summary, 240);
                item.content = bounded_chars(&item.content, 600);
                items.push(item);
            }
            Err(_) => invalid_items += 1,
        }
    }
    items.sort_by(|left, right| left.memory_id.cmp(&right.memory_id));
    items.truncate(24);
    let token_estimate = items
        .iter()
        .map(|item| ((item.summary.chars().count() + item.content.chars().count() + 3) / 4) as u32)
        .sum::<u32>()
        .min(500);
    let mut warnings = Vec::new();
    if total.saturating_sub(invalid_items) > items.len() {
        warnings.push("memory_pack_truncated_to_24_items".to_owned());
    }
    if invalid_items > 0 {
        warnings.push(format!("ignored_{invalid_items}_invalid_memory_items"));
    }
    let generated_at = generated_at.to_rfc3339();
    let identity_value = json!({
        "schema_version": "hsk.memory_pack@0.1",
        "workspace_id": workspace_id,
        "generated_at": &generated_at,
        "determinism_mode": MemoryPackDeterminismMode::Strict,
        "memory_policy": MemoryPolicy::WorkspaceScoped,
        "scope_refs": [scope_ref.clone()],
        "budgets": {
            "max_tokens": 500,
            "max_items": 24,
            "max_items_per_type": {},
        },
        "items": &items,
        "token_estimate": token_estimate,
        "warnings": &warnings,
    });
    let content_address =
        crate::llm::sha256_hex(crate::llm::canonical_json_bytes_nfc(&identity_value).as_slice());
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

async fn store_memory_pack_immutable_with_executor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    scope_key: &str,
    pack: &MemoryPack,
) -> StorageResult<()> {
    let generated_at = DateTime::parse_from_rfc3339(&pack.generated_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::Conflict("memory pack has invalid generated_at"))?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO fems_memory_packs (pack_id, workspace_id, scope_key, pack, generated_at)
        VALUES ($1, $2, $3, $4::jsonb, $5)
        ON CONFLICT (pack_id) DO NOTHING
        "#,
    )
    .bind(&pack.pack_id)
    .bind(workspace_id)
    .bind(scope_key)
    .bind(to_jsonb_text(pack)?)
    .bind(generated_at)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(());
    }
    let existing: Option<(String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT workspace_id, scope_key, pack::text, generated_at FROM fems_memory_packs WHERE pack_id = $1 FOR UPDATE",
    )
    .bind(&pack.pack_id)
    .fetch_optional(&mut **tx)
    .await?;
    if existing
        .as_ref()
        .is_none_or(|(owner, stored_scope, stored_pack, stored_at)| {
            owner != workspace_id
                || stored_scope != scope_key
                || serde_json::from_str::<MemoryPack>(stored_pack).ok() != Some(pack.clone())
                || *stored_at != generated_at
        })
    {
        return Err(StorageError::Conflict(
            "memory pack identity is bound to different evidence",
        ));
    }
    Ok(())
}

/// Commit exactly one approved proposal. The canonical item, immutable commit report,
/// strict MemoryPack projection, proposal terminal state, and EventLedger receipt are
/// written in one PostgreSQL transaction. Exact retries return the original result.
pub async fn commit_memory_proposal_with_receipt(
    pool: &PgPool,
    workspace_id: &str,
    proposal_id: &str,
    mut receipt: NewKernelEvent,
) -> StorageResult<MemoryProposalCommitResult> {
    ensure_fems_memory_schema(pool).await?;
    let mut tx = pool.begin().await?;
    // Pack reconstruction reads every committed item in a workspace. Serialize
    // workspace commits so two proposals cannot each publish a pack that omits
    // the other transaction's newly committed item.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("fems-memory-commit:{workspace_id}"))
        .execute(&mut *tx)
        .await?;
    let mut proposal =
        get_memory_proposal_for_review_with_executor(&mut tx, workspace_id, proposal_id)
            .await?
            .ok_or(StorageError::NotFound("memory proposal in workspace"))?;

    let existing_report = sqlx::query(
        r#"
        SELECT commit_id, memory_id, report::text AS report, report_hash
        FROM fems_memory_commit_reports
        WHERE workspace_id = $1 AND proposal_id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await?;
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

    if let Some(row) = existing_report {
        let stored_commit_id: String = row.try_get("commit_id")?;
        let stored_memory_id: String = row.try_get("memory_id")?;
        let stored_report_text: String = row.try_get("report")?;
        let stored_hash: String = row.try_get("report_hash")?;
        let commit_report: MemoryCommitReport = serde_json::from_str(&stored_report_text)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let computed_report_hash = commit_report
            .compute_hash()
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        if stored_commit_id != commit_id
            || stored_memory_id != memory_id
            || commit_report.source_proposal_id != proposal_id
            || stored_hash != computed_report_hash
        {
            return Err(StorageError::Conflict(
                "memory proposal commit identity is bound to different evidence",
            ));
        }

        let stored_item: Option<(String, String)> = sqlx::query_as(
            "SELECT workspace_id, item::text FROM fems_memory_items WHERE memory_id = $1 FOR UPDATE",
        )
        .bind(&memory_id)
        .fetch_optional(&mut *tx)
        .await?;
        if stored_item.as_ref().is_none_or(|(owner, item)| {
            owner != workspace_id
                || serde_json::from_str::<Value>(item).ok() != Some(memory_item.clone())
        }) {
            return Err(StorageError::Conflict(
                "memory proposal commit references different item evidence",
            ));
        }

        let receipt = load_memory_commit_receipt_with_executor(&mut tx, proposal_id).await?;
        let payload = receipt.payload.as_object().ok_or(StorageError::Conflict(
            "memory commit receipt payload is not an object",
        ))?;
        let payload_string = |key: &str| payload.get(key).and_then(Value::as_str);
        if payload_string("workspace_id") != Some(workspace_id)
            || payload_string("proposal_id") != Some(proposal_id)
            || payload_string("commit_id") != Some(commit_id.as_str())
            || payload_string("memory_id") != Some(memory_id.as_str())
            || payload_string("commit_report_hash") != Some(stored_hash.as_str())
        {
            return Err(StorageError::Conflict(
                "memory commit receipt is bound to different evidence",
            ));
        }
        let pack_id = payload_string("memory_pack_id").ok_or(StorageError::Conflict(
            "memory commit receipt is missing memory_pack_id",
        ))?;
        let memory_pack =
            load_memory_pack_by_id_with_executor(&mut tx, workspace_id, pack_id).await?;
        if payload_string("memory_pack_hash") != Some(memory_pack.memory_pack_hash.as_str()) {
            return Err(StorageError::Conflict(
                "memory commit receipt memory pack hash does not match its original pack",
            ));
        }
        let flight_recorder_event = build_memory_commit_flight_recorder_event(
            workspace_id,
            proposal_id,
            &memory_id,
            &commit_report,
            &stored_hash,
            &receipt,
        )?;
        store_memory_commit_outbox_event_with_executor(
            &mut tx,
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-003",
            &flight_recorder_event,
        )
        .await?;
        let memory_pack_flight_recorder_event = build_memory_pack_flight_recorder_event(
            workspace_id,
            proposal_id,
            &commit_id,
            &memory_pack,
            &flight_recorder_event,
        )?;
        store_memory_commit_outbox_event_with_executor(
            &mut tx,
            workspace_id,
            proposal_id,
            &commit_id,
            "FR-EVT-MEM-004",
            &memory_pack_flight_recorder_event,
        )
        .await?;
        tx.commit().await?;
        return Ok(MemoryProposalCommitResult {
            proposal,
            memory_item,
            memory_pack,
            commit_report,
            commit_report_hash: stored_hash,
            receipt,
            flight_recorder_event,
            memory_pack_flight_recorder_event,
        });
    }

    let now = Utc::now();
    let committed_at = if now <= reviewed_at {
        reviewed_at + chrono::Duration::microseconds(1)
    } else {
        now
    };
    let applied = MemoryCommitAppliedOp {
        op: MemoryMutationOp::Add,
        memory_id: memory_id.clone(),
        previous_version: None,
        new_version: Some(1),
        status: MemoryCommitOpStatus::Applied,
        reason: None,
    };
    let commit_report = MemoryCommitReport {
        schema_version: "hsk.memory_commit_report@0.1".to_owned(),
        commit_id: commit_id.clone(),
        created_at: committed_at.to_rfc3339(),
        source_proposal_id: proposal_id.to_owned(),
        applied_ops: vec![applied],
        warnings: Vec::new(),
        pack_rebuild_hints: vec![MemoryPackRebuildHint {
            scope_ref: scope_ref.clone(),
            reason: MemoryPackRebuildHintReason::MemoryChanged,
        }],
    };
    let commit_report_hash = commit_report
        .compute_hash()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;

    let item_json = to_jsonb_text(&memory_item)?;
    let inserted = sqlx::query(
        r#"
            INSERT INTO fems_memory_items (memory_id, workspace_id, item)
            VALUES ($1, $2, $3::jsonb)
            ON CONFLICT (memory_id) DO NOTHING
            "#,
    )
    .bind(&memory_id)
    .bind(workspace_id)
    .bind(item_json)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        let existing: Option<(String, String)> = sqlx::query_as(
                "SELECT workspace_id, item::text FROM fems_memory_items WHERE memory_id = $1 FOR UPDATE",
            )
            .bind(&memory_id)
            .fetch_optional(&mut *tx)
        .await?;
        if existing.as_ref().is_none_or(|(owner, item)| {
            owner != workspace_id
                || serde_json::from_str::<Value>(item).ok() != Some(memory_item.clone())
        }) {
            return Err(StorageError::Conflict(
                "memory item identity is bound to different evidence",
            ));
        }
    }
    sqlx::query(
        r#"
            INSERT INTO fems_memory_commit_reports
                (commit_id, workspace_id, proposal_id, memory_id, report, report_hash, created_at)
            VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7)
            "#,
    )
    .bind(&commit_id)
    .bind(workspace_id)
    .bind(proposal_id)
    .bind(&memory_id)
    .bind(to_jsonb_text(&commit_report)?)
    .bind(&commit_report_hash)
    .bind(committed_at)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
            "UPDATE fems_memory_proposals SET status = 'committed', proposal = jsonb_set(proposal, '{status}', '\"committed\"'::jsonb, true) WHERE workspace_id = $1 AND proposal_id = $2 AND status = 'approved'",
        )
        .bind(workspace_id)
        .bind(proposal_id)
    .execute(&mut *tx)
    .await?;
    proposal.status = "committed".to_owned();
    if let Value::Object(object) = &mut proposal.proposal {
        object.insert("status".to_owned(), json!("committed"));
    }

    let memory_pack =
        build_memory_pack_with_executor(&mut tx, workspace_id, committed_at, scope_ref).await?;
    store_memory_pack_immutable_with_executor(&mut tx, workspace_id, "", &memory_pack).await?;

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
    let receipt =
        crate::storage::postgres::append_kernel_event_with_executor(&mut *tx, receipt).await?;
    let flight_recorder_event = build_memory_commit_flight_recorder_event(
        workspace_id,
        proposal_id,
        &memory_id,
        &commit_report,
        &commit_report_hash,
        &receipt,
    )?;
    store_memory_commit_outbox_event_with_executor(
        &mut tx,
        workspace_id,
        proposal_id,
        &commit_id,
        "FR-EVT-MEM-003",
        &flight_recorder_event,
    )
    .await?;
    let memory_pack_flight_recorder_event = build_memory_pack_flight_recorder_event(
        workspace_id,
        proposal_id,
        &commit_id,
        &memory_pack,
        &flight_recorder_event,
    )?;
    store_memory_commit_outbox_event_with_executor(
        &mut tx,
        workspace_id,
        proposal_id,
        &commit_id,
        "FR-EVT-MEM-004",
        &memory_pack_flight_recorder_event,
    )
    .await?;
    tx.commit().await?;

    let _ = pack_item;
    Ok(MemoryProposalCommitResult {
        proposal,
        memory_item,
        memory_pack,
        commit_report,
        commit_report_hash,
        receipt,
        flight_recorder_event,
        memory_pack_flight_recorder_event,
    })
}

/// Insert or replace a COMMITTED memory item. This is only reachable from a downstream
/// review/commit path (out of MT-109 scope) or a test seeding a committed item to prove
/// a proposal cannot mutate it.
pub async fn upsert_memory_item(
    pool: &PgPool,
    workspace_id: &str,
    memory_id: &str,
    item: &Value,
) -> StorageResult<()> {
    ensure_fems_memory_schema(pool).await?;
    let item_json = to_jsonb_text(item)?;
    let result = sqlx::query(
        r#"
        INSERT INTO fems_memory_items (memory_id, workspace_id, item)
        VALUES ($1, $2, $3::jsonb)
        ON CONFLICT (memory_id) DO UPDATE SET
            item = EXCLUDED.item,
            updated_at = now()
        WHERE fems_memory_items.workspace_id = EXCLUDED.workspace_id
        "#,
    )
    .bind(memory_id)
    .bind(workspace_id)
    .bind(item_json)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(StorageError::Conflict(
            "memory item id belongs to a different workspace",
        ));
    }
    Ok(())
}

/// Read a committed memory item's JSON by id (AC-109-3 negative proof: unchanged after a
/// proposal is submitted).
pub async fn get_memory_item(
    pool: &PgPool,
    workspace_id: &str,
    memory_id: &str,
) -> StorageResult<Option<Value>> {
    ensure_fems_memory_schema(pool).await?;
    let row = sqlx::query(
        r#"
        SELECT item::text AS item
        FROM fems_memory_items
        WHERE workspace_id = $1 AND memory_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(memory_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let item_text: String = row.try_get("item")?;
            let item: Value = serde_json::from_str(&item_text)
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            Ok(Some(item))
        }
        None => Ok(None),
    }
}

/// Count committed memory items for a workspace (AC-109-3 negative proof: submitting a
/// proposal does not increase the committed-item count).
pub async fn count_memory_items(pool: &PgPool, workspace_id: &str) -> StorageResult<i64> {
    ensure_fems_memory_schema(pool).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fems_memory_items WHERE workspace_id = $1")
            .bind(workspace_id)
            .fetch_one(pool)
            .await?;
    Ok(count)
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
}
