//! WP-KERNEL-009 MT-067 ProjectKnowledgeIndex CRDT HTTP surface.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
//! "ProjectKnowledgeIndex MUST provide backend navigation APIs for
//! no-context local and cloud agents. A backend navigation call MUST carry
//! actor id, session id, correlation id, target authority ref, intended
//! operation, and a typed receipt."
//!
//! Routes (all JSON, all loopback-served by the Handshake backend itself —
//! no external relay, MT-078):
//!   * POST /knowledge/crdt/updates/push  — ingest one Yjs update envelope.
//!   * GET  /knowledge/crdt/updates/pull  — replay feed since a sequence.
//!   * GET  /knowledge/crdt/conflict_state — typed conflict UI payload
//!     (MT-075), computed from CRDT metadata + durable denial receipts.
//!
//! Authority: every durable effect lands in PostgreSQL/EventLedger through
//! the kernel CRDT stores; these handlers never hold draft authority in
//! process memory.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::kernel::crdt::conflict_ui::{compute_conflict_ui_state, ConflictUiStateV1};
use crate::kernel::crdt::save_semantics::{
    save_rich_document_draft, KnowledgeDraftSaveOutcomeV1, KnowledgeSaveDecisionV1,
};
use crate::kernel::crdt::yjs_bridge::{
    pull_yjs_updates, read_draft_head, YjsPushDenialReasonV1, YjsPushDenialV1, YjsPushOutcomeV1,
    YjsUpdateEnvelopeV1, YjsUpdatePullResponseV1, YJS_PUSH_DENIAL_SCHEMA_ID,
};
use crate::storage::knowledge_crdt::list_denial_receipts_for_document;
use crate::storage::Database;
use crate::AppState;

/// Narrow state for the knowledge CRDT routes: the Database trait object for
/// EventLedger + kernel CRDT stores, plus the shared PostgreSQL pool for the
/// WP-009 `knowledge_crdt_*` tables. Tests construct this directly from the
/// postgres fixture; the app constructs it from [`AppState`].
#[derive(Clone)]
pub struct KnowledgeCrdtApiState {
    pub db: Arc<dyn Database>,
    pub pool: sqlx::PgPool,
}

pub fn routes(state: AppState) -> Router {
    router_with_state(KnowledgeCrdtApiState {
        db: state.storage.clone(),
        pool: state.postgres_pool.clone(),
    })
}

/// Router over the narrow state (test entrypoint).
pub fn router_with_state(state: KnowledgeCrdtApiState) -> Router {
    Router::new()
        .route("/knowledge/crdt/updates/push", post(push_update))
        .route("/knowledge/crdt/updates/pull", get(pull_updates))
        .route("/knowledge/crdt/conflict_state", get(conflict_state))
        .with_state(state)
}

#[derive(Debug, Serialize)]
pub struct KnowledgeCrdtErrorResponse {
    pub code: &'static str,
    pub message: String,
}

/// Spec 2.3.13.11 backend-navigation receipt attached to responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNavigationReceiptV1 {
    pub receipt_kind: String,
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
    pub target_authority_ref: String,
    pub operation: String,
    pub served_at_utc: String,
}

pub(crate) fn navigation_receipt(
    actor_id: &str,
    session_id: &str,
    correlation_id: &str,
    target_authority_ref: String,
    operation: &str,
) -> KnowledgeNavigationReceiptV1 {
    KnowledgeNavigationReceiptV1 {
        receipt_kind: "knowledge_crdt_navigation_receipt_v1".to_string(),
        actor_id: actor_id.to_string(),
        session_id: session_id.to_string(),
        correlation_id: correlation_id.to_string(),
        target_authority_ref,
        operation: operation.to_string(),
        served_at_utc: Utc::now().to_rfc3339(),
    }
}

pub(crate) fn require_navigation_ids(
    actor_id: &str,
    session_id: &str,
    correlation_id: &str,
) -> Result<(), (StatusCode, Json<KnowledgeCrdtErrorResponse>)> {
    if actor_id.trim().is_empty()
        || session_id.trim().is_empty()
        || correlation_id.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(KnowledgeCrdtErrorResponse {
                code: "knowledge_crdt_navigation_ids_required",
                message: "actor_id, session_id and correlation_id are required (spec 2.3.13.11)"
                    .to_string(),
            }),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct PushUpdateRequest {
    pub envelope: YjsUpdateEnvelopeV1,
}

#[derive(Debug, Serialize)]
pub struct PushUpdateResponse {
    pub result: YjsPushOutcomeV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

async fn push_update(
    State(state): State<KnowledgeCrdtApiState>,
    Json(request): Json<PushUpdateRequest>,
) -> Result<(StatusCode, Json<PushUpdateResponse>), (StatusCode, Json<KnowledgeCrdtErrorResponse>)>
{
    let envelope = request.envelope;
    require_navigation_ids(&envelope.actor_id, &envelope.session_id, &envelope.trace_id)?;
    let receipt = navigation_receipt(
        &envelope.actor_id,
        &envelope.session_id,
        &envelope.trace_id,
        format!(
            "postgres://kernel_crdt_updates/{}",
            envelope.crdt_document_id
        ),
        "push_update",
    );
    match save_rich_document_draft(state.db.as_ref(), &state.pool, &envelope).await {
        Ok(outcome) => {
            let result = yjs_push_outcome_from_draft_outcome(&envelope, outcome);
            let status = match &result {
                YjsPushOutcomeV1::Stored { .. } | YjsPushOutcomeV1::AlreadyStored { .. } => {
                    StatusCode::OK
                }
                YjsPushOutcomeV1::Denied { .. } => StatusCode::CONFLICT,
            };
            Ok((status, Json(PushUpdateResponse { result, receipt })))
        }
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(KnowledgeCrdtErrorResponse {
                code: "knowledge_crdt_push_failed",
                message: error.to_string(),
            }),
        )),
    }
}

fn yjs_push_outcome_from_draft_outcome(
    envelope: &YjsUpdateEnvelopeV1,
    outcome: KnowledgeDraftSaveOutcomeV1,
) -> YjsPushOutcomeV1 {
    match outcome {
        KnowledgeDraftSaveOutcomeV1::Accepted {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        } => YjsPushOutcomeV1::Stored {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        },
        KnowledgeDraftSaveOutcomeV1::AlreadyApplied {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        } => YjsPushOutcomeV1::AlreadyStored {
            update_seq,
            update_id,
            event_ledger_event_id,
            head_state_vector,
        },
        KnowledgeDraftSaveOutcomeV1::Conflict {
            decision,
            head_update_seq,
            head_state_vector,
            ..
        } => denied_push_outcome(
            envelope,
            YjsPushDenialReasonV1::StaleBase {
                head_update_seq,
                head_state_vector,
                ordering: yjs_ordering_for_save_decision(&decision).to_string(),
            },
        ),
        KnowledgeDraftSaveOutcomeV1::Rejected { reason } => denied_push_outcome(envelope, reason),
        KnowledgeDraftSaveOutcomeV1::LeaseDenied { denial } => denied_push_outcome(
            envelope,
            YjsPushDenialReasonV1::EnvelopeInvalid {
                messages: vec![format!("lease write denied: {:?}", denial.reason)],
            },
        ),
    }
}

fn denied_push_outcome(
    envelope: &YjsUpdateEnvelopeV1,
    reason: YjsPushDenialReasonV1,
) -> YjsPushOutcomeV1 {
    YjsPushOutcomeV1::Denied {
        denial: YjsPushDenialV1 {
            schema_id: YJS_PUSH_DENIAL_SCHEMA_ID.to_string(),
            crdt_document_id: envelope.crdt_document_id.clone(),
            update_id: envelope.update_id.clone(),
            actor_id: envelope.actor_id.clone(),
            reason,
        },
    }
}

fn yjs_ordering_for_save_decision(decision: &KnowledgeSaveDecisionV1) -> &'static str {
    match decision {
        KnowledgeSaveDecisionV1::FastForward => "Equal",
        KnowledgeSaveDecisionV1::StaleWrite { .. } => "Dominates",
        KnowledgeSaveDecisionV1::AheadOfHead { .. } => "DominatedBy",
        KnowledgeSaveDecisionV1::ConcurrentFork { .. } => "Concurrent",
    }
}

#[derive(Debug, Deserialize)]
pub struct PullUpdatesQuery {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    #[serde(default)]
    pub since_update_seq: u64,
    pub document_schema_id: String,
    // Spec 2.3.13.11 backend-navigation identification (required).
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
}

#[derive(Debug, Serialize)]
pub struct PullUpdatesResponse {
    pub result: YjsUpdatePullResponseV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

async fn pull_updates(
    State(state): State<KnowledgeCrdtApiState>,
    Query(query): Query<PullUpdatesQuery>,
) -> Result<Json<PullUpdatesResponse>, (StatusCode, Json<KnowledgeCrdtErrorResponse>)> {
    require_navigation_ids(&query.actor_id, &query.session_id, &query.correlation_id)?;
    let result = pull_yjs_updates(
        state.db.as_ref(),
        &query.workspace_id,
        &query.document_id,
        &query.crdt_document_id,
        query.since_update_seq,
        &query.document_schema_id,
    )
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(KnowledgeCrdtErrorResponse {
                code: "knowledge_crdt_pull_failed",
                message: error.to_string(),
            }),
        )
    })?;
    let receipt = navigation_receipt(
        &query.actor_id,
        &query.session_id,
        &query.correlation_id,
        format!("postgres://kernel_crdt_updates/{}", query.crdt_document_id),
        "pull_updates",
    );
    Ok(Json(PullUpdatesResponse { result, receipt }))
}

#[derive(Debug, Deserialize)]
pub struct ConflictStateQuery {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
}

#[derive(Debug, Serialize)]
pub struct ConflictStateResponse {
    pub result: ConflictUiStateV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

async fn conflict_state(
    State(state): State<KnowledgeCrdtApiState>,
    Query(query): Query<ConflictStateQuery>,
) -> Result<Json<ConflictStateResponse>, (StatusCode, Json<KnowledgeCrdtErrorResponse>)> {
    require_navigation_ids(&query.actor_id, &query.session_id, &query.correlation_id)?;
    let head = read_draft_head(
        state.db.as_ref(),
        &query.workspace_id,
        &query.document_id,
        &query.crdt_document_id,
    )
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(KnowledgeCrdtErrorResponse {
                code: "knowledge_crdt_head_failed",
                message: error.to_string(),
            }),
        )
    })?;
    let receipts = list_denial_receipts_for_document(&state.pool, &query.crdt_document_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(KnowledgeCrdtErrorResponse {
                    code: "knowledge_crdt_receipts_failed",
                    message: error.to_string(),
                }),
            )
        })?;
    let result = compute_conflict_ui_state(
        &query.workspace_id,
        &query.document_id,
        &query.crdt_document_id,
        &head,
        &receipts,
    );
    let receipt = navigation_receipt(
        &query.actor_id,
        &query.session_id,
        &query.correlation_id,
        format!(
            "postgres://knowledge_crdt_denial_receipts/{}",
            query.crdt_document_id
        ),
        "conflict_state",
    );
    Ok(Json(ConflictStateResponse { result, receipt }))
}
