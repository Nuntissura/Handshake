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
//! Authority: every durable effect lands in the store/EventLedger through
//! the kernel CRDT stores; these handlers never hold draft authority in
//! process memory.
//!
//! The `knowledge_crdt_*` reads and writes use the application's embedded
//! SurrealDB authority and share its EventLedger.
//!
//! MT-154 (Master Spec 02-system-architecture.md:2758/2773/2776, LM-RLS-001/002): every route
//! authorizes the exact RichDocument (Update+fs.write to push, Read+fs.read to read) and its
//! workspace through the ResourceBroker BEFORE any table access, then runs every read/write as the
//! account's record user with EventLedger receipts attributed to the session principal. Every
//! failure, including a write SurrealDB silently drops, is the constant denial.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::kernel::crdt::conflict_ui::{compute_conflict_ui_state, ConflictUiStateV1};
use crate::kernel::crdt::save_semantics::{
    save_rich_document_draft, KnowledgeDraftSaveOutcomeV1, KnowledgeSaveDecisionV1,
};
use crate::kernel::crdt::yjs_bridge::{
    pull_yjs_updates, read_draft_head, YjsPushDenialReasonV1, YjsPushDenialV1, YjsPushOutcomeV1,
    YjsUpdateEnvelopeV1, YjsUpdatePullResponseV1, YJS_PUSH_DENIAL_SCHEMA_ID,
};
use crate::storage::knowledge_crdt::list_denial_receipts_for_document;
use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
use crate::storage::surreal::SurrealStorage;
use crate::storage::Database;
use crate::AppState;

/// Narrow state for the knowledge CRDT routes: the Database trait object for
/// EventLedger + kernel CRDT stores, plus the shared embedded store for the
/// WP-009 `knowledge_crdt_*` tables. Kept for callers that build the router
/// from storage handles only; [`router_with_state`] wraps it in an [`AppState`]
/// so the same ResourceBroker authorization applies.
#[derive(Clone)]
pub struct KnowledgeCrdtApiState {
    pub db: Arc<dyn Database>,
    pub pool: SurrealStorage,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/knowledge/crdt/updates/push", post(push_update))
        .route("/knowledge/crdt/updates/pull", get(pull_updates))
        .route("/knowledge/crdt/conflict_state", get(conflict_state))
        .with_state(state)
}

/// Router over the narrow state. The storage handles are wrapped in an [`AppState`] with a quiet
/// recorder, a disabled LLM client and the default capability registry; authorization is
/// identical to [`routes`].
pub fn router_with_state(state: KnowledgeCrdtApiState) -> Router {
    let recorder = Arc::new(QuietRecorder);
    routes(AppState {
        storage: state.db,
        surreal: state.pool,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(crate::llm::DisabledLlmClient::new(
            "knowledge-crdt".to_owned(),
            "the knowledge CRDT router does not use a model".to_owned(),
        )),
        capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
        session_registry: Arc::new(crate::workflows::SessionRegistry::new(
            crate::workflows::SessionSchedulerConfig::default(),
        )),
    })
}

/// No-op recorder for [`router_with_state`]; the CRDT routes record through the EventLedger.
struct QuietRecorder;

#[async_trait::async_trait]
impl crate::flight_recorder::FlightRecorder for QuietRecorder {
    async fn record_event(
        &self,
        _event: crate::flight_recorder::FlightRecorderEvent,
    ) -> Result<(), crate::flight_recorder::RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: crate::flight_recorder::EventFilter,
    ) -> Result<
        Vec<crate::flight_recorder::FlightRecorderEvent>,
        crate::flight_recorder::RecorderError,
    > {
        Ok(Vec::new())
    }
}

#[async_trait::async_trait]
impl crate::diagnostics::DiagnosticsStore for QuietRecorder {
    async fn record_diagnostic(
        &self,
        _diag: crate::diagnostics::Diagnostic,
    ) -> Result<(), crate::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: crate::diagnostics::DiagFilter,
    ) -> Result<Vec<crate::diagnostics::ProblemGroup>, crate::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: uuid::Uuid,
    ) -> Result<crate::diagnostics::Diagnostic, crate::storage::StorageError> {
        Err(crate::storage::StorageError::NotFound("diagnostic"))
    }

    async fn list_diagnostics(
        &self,
        _filter: crate::diagnostics::DiagFilter,
    ) -> Result<Vec<crate::diagnostics::Diagnostic>, crate::storage::StorageError> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// MT-154 shared knowledge account authority (CRDT, ingestion, memory, retrieval).
// ---------------------------------------------------------------------------

/// The authenticated account authority of one knowledge request, bound to exactly one workspace.
/// Every protected read/write of the request runs inside [`KnowledgeAccount::run`] as the
/// account's record user, never as root.
pub(crate) struct KnowledgeAccount {
    pub(crate) authority: crate::api::authority::AuthorizedResourceContext,
    pub(crate) workspace_id: String,
}

impl KnowledgeAccount {
    /// Authorizes `action` + `capability` on the workspace resource through the ResourceBroker.
    /// Runs before any table, filesystem or process access; every failure is the constant denial.
    pub(crate) async fn workspace(
        state: &AppState,
        headers: &HeaderMap,
        workspace_id: &str,
        action: ResourceAction,
        capability: &'static str,
    ) -> Result<Self, (StatusCode, Json<Value>)> {
        if workspace_id.trim().is_empty() || workspace_id.trim() != workspace_id {
            return Err(crate::api::authority::constant_denial());
        }
        let authority = crate::api::authority::authorize_request(
            state,
            headers,
            capability,
            ResourceKind::Workspace,
            workspace_id,
            action,
        )
        .await
        .map_err(|_| crate::api::authority::constant_denial())?;
        Ok(Self {
            authority,
            workspace_id: workspace_id.to_owned(),
        })
    }

    /// The exact RichDocument grant plus its workspace: the document must be an active protected
    /// resource of this account whose parent workspace is `workspace_id`. The returned authority is
    /// the workspace one (receipts and record-user scope bind to the workspace).
    pub(crate) async fn rich_document(
        state: &AppState,
        headers: &HeaderMap,
        workspace_id: &str,
        document_id: &str,
        action: ResourceAction,
        capability: &'static str,
    ) -> Result<Self, (StatusCode, Json<Value>)> {
        if document_id.trim().is_empty() || document_id.trim() != document_id {
            return Err(crate::api::authority::constant_denial());
        }
        let document = crate::api::authority::authorize_request(
            state,
            headers,
            capability,
            ResourceKind::RichDocument,
            document_id,
            action,
        )
        .await
        .map_err(|_| crate::api::authority::constant_denial())?;
        match state
            .surreal
            .authorized_document_workspace(
                &document.resource_id,
                &document.account_id,
                &document.access_space_id,
            )
            .await
        {
            Ok(Some(workspace)) if workspace == workspace_id => {}
            _ => return Err(crate::api::authority::constant_denial()),
        }
        let workspace_action = if matches!(action, ResourceAction::Read) {
            ResourceAction::Read
        } else {
            ResourceAction::Update
        };
        Self::workspace(state, headers, workspace_id, workspace_action, capability).await
    }

    /// The session principal every receipt written by this request carries.
    pub(crate) fn session_actor(&self) -> crate::kernel::KernelActor {
        match self.authority.actor_kind.as_str() {
            "system" => crate::kernel::KernelActor::System(self.authority.actor_id.clone()),
            _ => crate::kernel::KernelActor::Operator(self.authority.actor_id.clone()),
        }
    }

    /// The authenticated session id (the backend-navigation session run of this request).
    pub(crate) fn session_id(&self) -> &str {
        &self.authority.session_id
    }

    /// Runs `operation` as the account's record user (table permissions apply), with receipts
    /// stamped with the session principal and bound to the authorized workspace.
    pub(crate) async fn run<T>(
        &self,
        state: &AppState,
        operation: impl std::future::Future<Output = T>,
    ) -> T {
        state
            .surreal
            .with_record_user_scope(
                self.authority.record_user_scope.clone(),
                crate::storage::surreal::event_ledger::with_loom_session_receipt(
                    self.session_actor(),
                    self.workspace_id.clone(),
                    operation,
                ),
            )
            .await
    }
}

/// F3 (SurrealDB 3.2.0 silently ignores a denied CREATE/UPDATE/DELETE): a record-user write that
/// returned no row, or a broker/guard denial, is the constant denial, never a 200/404/500.
pub(crate) fn is_record_user_denial(message: &str) -> bool {
    message.contains("HSK-403")
        || message.contains("returned no row")
        || message.contains("returned no record")
}

fn crdt_denied() -> (StatusCode, Json<Value>) {
    crate::api::authority::constant_denial()
}

#[derive(Debug, Serialize)]
pub struct KnowledgeCrdtErrorResponse {
    pub code: &'static str,
    pub message: String,
}

type CrdtApiError = (StatusCode, Json<Value>);

fn crdt_error(code: &'static str, message: String) -> CrdtApiError {
    if is_record_user_denial(&message) {
        return crdt_denied();
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"code": code, "message": message})),
    )
}

fn navigation_ids_error(error: (StatusCode, Json<KnowledgeCrdtErrorResponse>)) -> CrdtApiError {
    let (status, Json(body)) = error;
    (
        status,
        Json(json!({"code": body.code, "message": body.message})),
    )
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PushUpdateRequest>,
) -> Result<(StatusCode, Json<PushUpdateResponse>), CrdtApiError> {
    let envelope = request.envelope;
    let account = KnowledgeAccount::rich_document(
        &state,
        &headers,
        &envelope.workspace_id,
        &envelope.document_id,
        ResourceAction::Update,
        "fs.write",
    )
    .await?;
    require_navigation_ids(&envelope.actor_id, &envelope.session_id, &envelope.trace_id)
        .map_err(navigation_ids_error)?;
    let receipt = navigation_receipt(
        &envelope.actor_id,
        &envelope.session_id,
        &envelope.trace_id,
        format!(
            "surreal://kernel_crdt_updates/{}",
            envelope.crdt_document_id
        ),
        "push_update",
    );
    let outcome = account
        .run(
            &state,
            save_rich_document_draft(state.storage.as_ref(), &state.surreal, &envelope),
        )
        .await
        .map_err(|error| crdt_error("knowledge_crdt_push_failed", error.to_string()))?;
    let result = yjs_push_outcome_from_draft_outcome(&envelope, outcome);
    let status = match &result {
        YjsPushOutcomeV1::Stored { .. } | YjsPushOutcomeV1::AlreadyStored { .. } => StatusCode::OK,
        YjsPushOutcomeV1::Denied { .. } => StatusCode::CONFLICT,
    };
    Ok((status, Json(PushUpdateResponse { result, receipt })))
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PullUpdatesQuery>,
) -> Result<Json<PullUpdatesResponse>, CrdtApiError> {
    let account = KnowledgeAccount::rich_document(
        &state,
        &headers,
        &query.workspace_id,
        &query.document_id,
        ResourceAction::Read,
        "fs.read",
    )
    .await?;
    require_navigation_ids(&query.actor_id, &query.session_id, &query.correlation_id)
        .map_err(navigation_ids_error)?;
    let result = account
        .run(
            &state,
            pull_yjs_updates(
                state.storage.as_ref(),
                &query.workspace_id,
                &query.document_id,
                &query.crdt_document_id,
                query.since_update_seq,
                &query.document_schema_id,
            ),
        )
        .await
        .map_err(|error| crdt_error("knowledge_crdt_pull_failed", error.to_string()))?;
    let receipt = navigation_receipt(
        &query.actor_id,
        &query.session_id,
        &query.correlation_id,
        format!("surreal://kernel_crdt_updates/{}", query.crdt_document_id),
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ConflictStateQuery>,
) -> Result<Json<ConflictStateResponse>, CrdtApiError> {
    let account = KnowledgeAccount::rich_document(
        &state,
        &headers,
        &query.workspace_id,
        &query.document_id,
        ResourceAction::Read,
        "fs.read",
    )
    .await?;
    require_navigation_ids(&query.actor_id, &query.session_id, &query.correlation_id)
        .map_err(navigation_ids_error)?;
    let head = account
        .run(
            &state,
            read_draft_head(
                state.storage.as_ref(),
                &query.workspace_id,
                &query.document_id,
                &query.crdt_document_id,
            ),
        )
        .await
        .map_err(|error| crdt_error("knowledge_crdt_head_failed", error.to_string()))?;
    let receipts = account
        .run(
            &state,
            list_denial_receipts_for_document(&state.surreal, &query.crdt_document_id),
        )
        .await
        .map_err(|error| crdt_error("knowledge_crdt_receipts_failed", error.to_string()))?
        .into_iter()
        // Only the authorized document's receipts: a crdt_document_id is caller-supplied.
        .filter(|receipt| {
            receipt.workspace_id == query.workspace_id
                && receipt.document_id.as_deref() == Some(query.document_id.as_str())
        })
        .collect::<Vec<_>>();
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
            "surreal://knowledge_crdt_denial_receipts/{}",
            query.crdt_document_id
        ),
        "conflict_state",
    );
    Ok(Json(ConflictStateResponse { result, receipt }))
}
