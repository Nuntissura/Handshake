//! WP-KERNEL-012 MT-040 (E6 — backend reuse wiring): the typed native Rust client for the EXISTING
//! handshake_core `/knowledge/crdt/*` collaborative-editing HTTP surface (WP-KERNEL-009 MT-067,
//! `api::knowledge_crdt`). This is FRONTEND WIRING ONLY — it binds the three CRDT transport routes the
//! backend already serves; it does NOT change the backend, and it does NOT implement any Yjs merge/apply
//! engine (the `yrs` merge engine is a separate later concern). A backend gap is a TYPED BLOCKER, never
//! a backend edit.
//!
//! Reachable as `handshake_native::backend::knowledge_crdt` (placed in the `backend/` client subdir +
//! re-exported via `backend/mod.rs`, CONSISTENT with the MT-037/038/039 client placement — NOT a root
//! `src/knowledge_crdt.rs` module).
//!
//! # THE LOAD-BEARING DATA-INTEGRITY RULE: a 409 push denial is a VALID DOMAIN OUTCOME (DO NOT CHANGE)
//!
//! A `409 Conflict` from `POST /knowledge/crdt/updates/push` is NOT a network error — it is the typed
//! [`YjsPushOutcomeV1::Denied`] domain outcome and is returned as **`Ok(YjsPushOutcomeV1::Denied { .. })`**,
//! NEVER mapped to `Err`. The whole conflict-resolution UI depends on the editor layer seeing the typed
//! [`YjsPushDenialV1`] (a [`YjsPushDenialReasonV1::StaleBase`] / [`YjsPushDenialReasonV1::EnvelopeInvalid`]
//! / etc.) so it can pull-merge-resubmit or adopt the server head. Folding a 409 into a generic `Err`
//! would HIDE data-loss risk behind an opaque error and break the conflict surface (red-team RISK-1 /
//! MC-1). Only a NON-2xx, NON-409 status maps to [`CrdtError::HttpError`]. A future contributor MUST NOT
//! "simplify" the 409 arm into the error path. The denial is also surfaced on the conflict bus (see
//! [`CrdtConflictListener`]) so a push denial is never silently swallowed (contract note 10).
//!
//! # Wire-shape provenance (SPEC-REALISM GATE — verified READ-ONLY against the real backend)
//!
//! Every type + route + status mapping below was verified READ-ONLY against the running backend source,
//! NOT taken from the MT contract prose (the prose described an older/idealized shape; the real backend
//! is canonical per the Spec-Realism gate). The verified facts that shape this client:
//!   * Routes — `src/backend/handshake_core/src/api/knowledge_crdt.rs::router_with_state` (L61-67):
//!     `POST /knowledge/crdt/updates/push` -> `push_update` (200 Stored/AlreadyStored, 409 Denied);
//!     `GET /knowledge/crdt/updates/pull` -> `pull_updates`; `GET /knowledge/crdt/conflict_state` ->
//!     `conflict_state`.
//!   * Identity scheme is CRDT-SPECIFIC, NOT the `x-hsk-*` headers of `knowledge_documents`: for PUSH the
//!     `actor_id` / `session_id` / `trace_id` live INSIDE the [`YjsUpdateEnvelopeV1`] BODY
//!     (`push_update` calls `require_navigation_ids(actor_id, session_id, trace_id)`, api L143); for PULL
//!     and CONFLICT_STATE the `actor_id` / `session_id` / `correlation_id` are QUERY PARAMS
//!     (`PullUpdatesQuery` / `ConflictStateQuery`, api L249-261 / L302-310). A missing/empty required id
//!     is a backend `400 knowledge_crdt_navigation_ids_required` (api `require_navigation_ids`, L105-124).
//!     This client adds NO `x-hsk-*` headers to these routes.
//!   * The envelope's binary update is the `update_b64: String` field (base64 STANDARD-alphabet, padded —
//!     the backend's `b64()` is `base64::engine::general_purpose::STANDARD`, yjs_bridge L55-56), NOT a
//!     `Vec<u8>` with a `#[serde(with=..)]` adapter. Likewise the head/state vectors are `String`
//!     (`state_vector_before` / `state_vector_after` on the envelope; `head_state_vector` on the
//!     outcome/pull/conflict responses), NOT raw byte arrays. This client mirrors the REAL wire shape
//!     (`update_b64`/`*_state_vector` as `String`) AND exposes ergonomic byte accessors
//!     ([`YjsUpdateEnvelopeV1::update_bytes`] / [`YjsUpdateEnvelopeV1::with_update_bytes`]) that
//!     encode/decode with the SAME `base64::STANDARD` engine so the editor never re-implements base64 and
//!     a byte-for-byte roundtrip is guaranteed (red-team RISK-2 / MC-2 — a url-safe or padless engine
//!     would silently drop updates).
//!   * `PullUpdatesResponse.result` is [`YjsUpdatePullResponseV1`] whose `updates` is a `Vec<`[`YjsUpdateEnvelopeV1`]`>`
//!     (the backend re-encodes stored update bytes back into full envelopes via `update_record_to_envelope`,
//!     yjs_bridge L694) — there is NO separate `StoredYjsUpdate` type in the real backend (the MT prose
//!     `StoredYjsUpdate` does not exist). An empty CRDT doc pulls `updates: []` + `head_update_seq: 0`,
//!     which is a NORMAL empty state, NOT an error (red-team RISK-8).
//!   * [`ConflictUiStateV1`] is `{schema_id, workspace_id, document_id, crdt_document_id, head_update_seq,
//!     head_state_vector, conflicts: Vec<ConflictUiEntryV1>}` (conflict_ui.rs L96-105) — the real type has
//!     NO `has_conflict: bool` wire field; "has a conflict" is derived from `!conflicts.is_empty()`,
//!     exposed as the [`ConflictUiStateV1::has_conflict`] convenience method.
//!   * [`KnowledgeNavigationReceiptV1`] (api L76-85) carries all SEVEN spec-2.3.13.11 audit fields
//!     (`receipt_kind, actor_id, session_id, correlation_id, target_authority_ref, operation,
//!     served_at_utc`) and is preserved in ALL THREE response structs even if the initial consumer does
//!     not read it (red-team RISK-7 / MC-4 — dropping it is an audit-trail gap).
//!
//! # Stateless adapter
//!
//! This module holds NO CRDT state. It is a stateless HTTP adapter sharing the WP-011
//! [`crate::backend_client`] base URL + the ONE process-wide [`crate::backend_client::shared_http_client`]
//! connection pool (the MT-037 REUSE-NOT-DUPLICATE transport concern). State (the open document, the last
//! seen `update_seq` the caller must track + pass as `since_update_seq` to avoid replaying old updates —
//! red-team RISK-3) lives in the editor layer that calls this. `since_update_seq` defaults to 0 and is
//! asserted on the outgoing query string (MC-5).
//!
//! # AccessKit (future conflict-resolution widget — DESIGN NOTE, NOT built this MT)
//!
//! This MT is the transport client ONLY — there is NO GUI here (no screenshot / no AccessKit nodes this
//! MT). When the collaborative-editing conflict-resolution panel is built (a later MT), it MUST expose
//! each [`ConflictUiEntryV1`] as an AccessKit node through the EXISTING `crate::accessibility` helpers
//! (`accessibility/mod.rs`, `accessibility/registry.rs`, `accessibility/live.rs`): an UNSEEN conflict
//! uses `accesskit::Role::Alert` (so a swarm agent / screen reader is notified), a seen one
//! `accesskit::Role::ListItem`, and each must support `accesskit::Action::Click` to accept/reject a
//! [`ConflictResolutionOptionV1`]. The panel registers those ids through the existing accessibility
//! registry rather than minting a parallel one.

use serde::{Deserialize, Serialize};

use base64::Engine;

use crate::backend_client::{shared_http_client, BACKEND_BASE_URL};

// Re-export the verified backend schema/encoding constants so a caller building a fresh envelope uses
// the SAME literals the backend validates against (a divergent schema id is a backend 400). Mirrored
// from `yjs_bridge.rs` L51-53.
/// `YjsUpdateEnvelopeV1.schema_id` the backend requires (yjs_bridge `YJS_UPDATE_ENVELOPE_SCHEMA_ID`).
pub const YJS_UPDATE_ENVELOPE_SCHEMA_ID: &str = "hsk.kernel.knowledge_yjs_update_envelope@1";
/// `YjsUpdateEnvelopeV1.encoding` the backend requires (yjs_bridge `YJS_UPDATE_ENCODING_V1`).
pub const YJS_UPDATE_ENCODING_V1: &str = "yjs-update-v1";
/// `YjsPushDenialV1.schema_id` the backend stamps on a denial (yjs_bridge `YJS_PUSH_DENIAL_SCHEMA_ID`).
pub const YJS_PUSH_DENIAL_SCHEMA_ID: &str = "hsk.kernel.knowledge_yjs_push_denial@1";

/// The canonical base64 engine the backend uses for `update_b64` / state vectors (yjs_bridge `b64()` =
/// `base64::engine::general_purpose::STANDARD`). Using the EXACT engine guarantees byte-for-byte wire
/// compatibility — base64url or a padless variant would silently corrupt updates (RISK-2 / MC-2).
fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Receipt (spec 2.3.13.11 — common to all three responses; ALL SEVEN fields preserved).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The spec-2.3.13.11 backend-navigation receipt attached to EVERY CRDT response (`api::knowledge_crdt::
/// KnowledgeNavigationReceiptV1`, L76-85). All seven audit fields are kept so a CRDT operation stays
/// traceable to the agent that performed it (RISK-7 / MC-4); dropping any is an audit-trail gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeNavigationReceiptV1 {
    pub receipt_kind: String,
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
    pub target_authority_ref: String,
    pub operation: String,
    pub served_at_utc: String,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Envelope (`api::knowledge_crdt::PushUpdateRequest.envelope` -> `yjs_bridge::YjsUpdateEnvelopeV1`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// One Yjs update crossing the frontend/backend boundary. Mirrors the REAL backend `YjsUpdateEnvelopeV1`
/// (yjs_bridge L82-108) FIELD-FOR-FIELD on the wire: the binary update is `update_b64` (base64 STANDARD,
/// padded) and the state vectors are `state_vector_before` / `state_vector_after` as `String`. The
/// identity (`actor_id` / `session_id` / `trace_id`) lives IN this body for push (NOT in `x-hsk-*`
/// headers).
///
/// Build via [`Self::with_update_bytes`] so the binary update is base64-encoded with the backend's
/// engine; read the decoded bytes back via [`Self::update_bytes`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YjsUpdateEnvelopeV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    /// Client-generated stable id for this update (idempotency token).
    pub update_id: String,
    /// Canonical typed actor id (`kind:ident`). REQUIRED, non-empty (push pre-flight + backend 400).
    pub actor_id: String,
    /// Stable CRDT site id (the MT-065 derivation for (workspace, crdt document, actor)).
    pub site_id: String,
    /// REQUIRED, non-empty (push pre-flight + backend 400).
    pub session_id: String,
    /// REQUIRED, non-empty for push (the backend maps the push correlation onto `trace_id`).
    pub trace_id: String,
    pub document_schema_id: String,
    /// Yjs binary update, base64 STANDARD-alphabet (padded). Use [`Self::with_update_bytes`] /
    /// [`Self::update_bytes`] rather than touching this field directly so the engine never drifts.
    pub update_b64: String,
    /// sha256 hex of the decoded update bytes (the backend re-hashes + rejects a mismatch).
    pub update_sha256: String,
    /// Typed state vector the client had applied before this update (opaque `String`).
    pub state_vector_before: String,
    /// Typed state vector after this update (opaque `String`; must strictly dominate `before`).
    pub state_vector_after: String,
    pub encoding: String,
}

impl YjsUpdateEnvelopeV1 {
    /// Decode the binary Yjs update from `update_b64` back into `Vec<u8>` using the backend's base64
    /// STANDARD engine. Errors as [`CrdtError::SerializationError`] if the field is not valid base64
    /// (which would mean a corrupted envelope).
    pub fn update_bytes(&self) -> Result<Vec<u8>, CrdtError> {
        b64()
            .decode(self.update_b64.as_bytes())
            .map_err(|e| CrdtError::SerializationError(format!("update_b64 is not valid base64: {e}")))
    }

    /// Set `update_b64` from raw bytes, encoding with the backend's base64 STANDARD engine. The inverse
    /// of [`Self::update_bytes`]; a `with_update_bytes(b).update_bytes()? == b` roundtrip is byte-for-byte
    /// exact (the MC-2 control).
    pub fn with_update_bytes(mut self, bytes: &[u8]) -> Self {
        self.update_b64 = b64().encode(bytes);
        self
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Push outcome (`yjs_bridge::YjsPushOutcomeV1`, tagged `#[serde(tag = "outcome", snake_case)]`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Typed reasons a push is refused (`yjs_bridge::YjsPushDenialReasonV1`, L404-420). A tagged enum keyed
/// by `code` (snake_case). Each variant drives a distinct conflict-resolution path in the UI; folding
/// them into a string would lose that.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum YjsPushDenialReasonV1 {
    /// Envelope failed structural validation (the messages name each failed field).
    EnvelopeInvalid { messages: Vec<String> },
    /// `state_vector_before` does not match the current head: pull, merge locally (Yjs), resubmit.
    StaleBase {
        head_update_seq: u64,
        head_state_vector: String,
        ordering: String,
    },
    /// Same `update_id` was stored before with different content.
    UpdateIdContentMismatch { update_id: String },
    /// Two writers raced for the same sequence slot; retry after refresh.
    SequenceSlotRace { attempted_seq: u64 },
}

/// A typed push denial (`yjs_bridge::YjsPushDenialV1`, L422-429). The payload the conflict UI renders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YjsPushDenialV1 {
    pub schema_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub actor_id: String,
    pub reason: YjsPushDenialReasonV1,
}

/// Outcome of a push: stored, replayed (idempotent), or DENIED (typed). Mirrors
/// `yjs_bridge::YjsPushOutcomeV1` (L431-450), tagged by `outcome` (snake_case).
///
/// CRITICAL: the backend returns HTTP 200 for `Stored` / `AlreadyStored` and HTTP **409** for `Denied`
/// (api `push_update` L157-162). The client returns ALL THREE as the `Ok` variant — a 409 `Denied` is a
/// valid domain outcome, NEVER an `Err` (the load-bearing rule, see the module doc).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum YjsPushOutcomeV1 {
    Stored {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    AlreadyStored {
        update_seq: u64,
        update_id: String,
        event_ledger_event_id: String,
        head_state_vector: String,
    },
    Denied {
        denial: YjsPushDenialV1,
    },
}

impl YjsPushOutcomeV1 {
    /// `true` iff this is a [`Self::Denied`] outcome (the editor shows the conflict UI on `true`).
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied { .. })
    }
}

/// `POST /knowledge/crdt/updates/push` body (`api::knowledge_crdt::PushUpdateRequest`, L126-129).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushUpdateRequest {
    pub envelope: YjsUpdateEnvelopeV1,
}

/// `POST /knowledge/crdt/updates/push` response (`api::knowledge_crdt::PushUpdateResponse`, L131-135).
/// Returned for BOTH 200 (Stored/AlreadyStored) and 409 (Denied) — the status distinguishes them but
/// the body shape is identical, so the client parses this for either status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushUpdateResponse {
    pub result: YjsPushOutcomeV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Pull (`api::knowledge_crdt::PullUpdatesResponse` -> `yjs_bridge::YjsUpdatePullResponseV1`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// `GET /knowledge/crdt/updates/pull` replay feed (`yjs_bridge::YjsUpdatePullResponseV1`, L655-665). The
/// `updates` are full re-encoded [`YjsUpdateEnvelopeV1`] (NOT a separate `StoredYjsUpdate` type — that MT
/// prose type does not exist in the real backend). An empty CRDT doc returns `updates: []` +
/// `head_update_seq: 0`, which is a NORMAL empty state, NOT an error (RISK-8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YjsUpdatePullResponseV1 {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub since_update_seq: u64,
    pub updates: Vec<YjsUpdateEnvelopeV1>,
    pub head_update_seq: u64,
    pub head_state_vector: String,
}

/// `GET /knowledge/crdt/updates/pull` response (`api::knowledge_crdt::PullUpdatesResponse`, L263-267).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullUpdatesResponse {
    pub result: YjsUpdatePullResponseV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

/// The query parameters for `GET /knowledge/crdt/updates/pull` (`api::knowledge_crdt::PullUpdatesQuery`,
/// L249-261). `actor_id` / `session_id` / `correlation_id` are the required CRDT identity (query params,
/// NOT headers); `since_update_seq` defaults to 0 (replay from the start) and is asserted on the wire.
#[derive(Debug, Clone)]
pub struct PullUpdatesParams {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub document_schema_id: String,
    /// `None` -> the backend default 0 (replay from the start); `Some(n)` -> only updates with
    /// `update_seq > n` (the caller tracks the last seen seq to avoid replaying old updates, RISK-3).
    pub since_update_seq: Option<u64>,
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Conflict state (`api::knowledge_crdt::ConflictStateResponse` -> `conflict_ui::ConflictUiStateV1`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Conflict kinds the UI distinguishes (`conflict_ui::ConflictUiKindV1`, L22-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictUiKindV1 {
    StaleDraftSave,
    ConcurrentDraftFork,
    AheadOfHeadSave,
    LeaseWriteDenied,
    AiEditPromotionDenied,
}

/// An actor participating in a conflict (`conflict_ui::ConflictUiActorV1`, L46-51).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiActorV1 {
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
}

/// A revision reference the UI renders side-by-side (`conflict_ui::ConflictUiRevisionV1`, L54-62).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiRevisionV1 {
    /// `base` (common ancestor), `ours` (server head), `theirs` (the denied writer's attempt).
    pub label: String,
    pub update_seq: Option<u64>,
    pub update_id: Option<String>,
    pub state_vector: String,
}

/// Resolution options the UI offers (`conflict_ui::ConflictResolutionOptionV1`, L65-78), tagged by
/// `option` (snake_case). Each entry maps to the AccessKit `Action::Click` accept/reject the future
/// conflict panel exposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "option", rename_all = "snake_case")]
pub enum ConflictResolutionOptionV1 {
    PullMergeResubmit { pull_since_update_seq: u64 },
    AdoptServerHead,
    PushMissingUpdatesFirst,
    ResolveLeaseFirst { lease_id: String },
    ReviewProposal { proposal_id: String },
}

/// One renderable conflict entry, backed 1:1 by a durable denial receipt
/// (`conflict_ui::ConflictUiEntryV1`, L81-93).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiEntryV1 {
    pub conflict_id: String,
    pub kind: ConflictUiKindV1,
    pub detected_at_utc: String,
    pub conflicting_actors: Vec<ConflictUiActorV1>,
    pub base: Option<ConflictUiRevisionV1>,
    pub ours: Option<ConflictUiRevisionV1>,
    pub theirs: Option<ConflictUiRevisionV1>,
    pub resolution_options: Vec<ConflictResolutionOptionV1>,
    pub denial_receipt_id: String,
    pub event_ledger_event_id: String,
}

/// The typed conflict payload for one document (`conflict_ui::ConflictUiStateV1`, L96-105). The real
/// backend type has NO `has_conflict` wire field; use the [`Self::has_conflict`] / [`Self::denial_count`]
/// convenience methods (derived from `conflicts`) to drive the conflict UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictUiStateV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub head_update_seq: u64,
    pub head_state_vector: String,
    pub conflicts: Vec<ConflictUiEntryV1>,
}

impl ConflictUiStateV1 {
    /// `true` iff there is at least one conflict to resolve (derived from `conflicts`, since the real
    /// backend payload has no `has_conflict` field). This is the flag the conflict UI shows on.
    pub fn has_conflict(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Number of outstanding conflict entries (each backed by a durable denial receipt).
    pub fn denial_count(&self) -> usize {
        self.conflicts.len()
    }
}

/// `GET /knowledge/crdt/conflict_state` response (`api::knowledge_crdt::ConflictStateResponse`, L312-316).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictStateResponse {
    pub result: ConflictUiStateV1,
    pub receipt: KnowledgeNavigationReceiptV1,
}

/// The query parameters for `GET /knowledge/crdt/conflict_state`
/// (`api::knowledge_crdt::ConflictStateQuery`, L302-310). Read-only diagnostic; `actor_id` / `session_id`
/// / `correlation_id` are the required CRDT identity (query params, NOT headers).
#[derive(Debug, Clone)]
pub struct ConflictStateParams {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub session_id: String,
    pub correlation_id: String,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed failure of one `/knowledge/crdt/*` call. NOTE: a 409 push DENIAL is NOT here — it is the
/// `Ok(YjsPushOutcomeV1::Denied)` domain outcome (the load-bearing rule). Only a genuine failure (a
/// non-2xx-non-409 status, a transport error, a parse failure, or a client-side identity guard) is an
/// `Err`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtError {
    /// A non-success status that is NOT the 200/409 push contract (e.g. 400 missing identity, 404, 5xx).
    /// Carries the status + body so the caller can surface the backend `code`/`message`.
    HttpError { status: u16, body: String },
    /// A response (or `update_b64`) could not be (de)serialized into the expected typed shape.
    SerializationError(String),
    /// A CLIENT-SIDE pre-flight guard rejected the request BEFORE sending: a required identity field
    /// (`actor_id` / `session_id` / `trace_id` for push; `actor_id` / `session_id` / `correlation_id`
    /// for pull/conflict_state) was empty. Sending it would be a guaranteed backend 400, so the client
    /// never opens the socket (RISK-5 / MC-3). Carries the name(s) of the empty field(s).
    IdentityMissing(String),
    /// A transport failure (connect / timeout / TLS) — the request never reached a status.
    Transport(String),
}

impl std::fmt::Display for CrdtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError { status, body } => write!(f, "crdt http error {status}: {body}"),
            Self::SerializationError(e) => write!(f, "crdt serialization error: {e}"),
            Self::IdentityMissing(fields) => {
                write!(f, "crdt identity missing (empty required field(s)): {fields}")
            }
            Self::Transport(e) => write!(f, "crdt transport error: {e}"),
        }
    }
}

impl std::error::Error for CrdtError {}

/// A typed result alias for this client.
pub type CrdtResult<T> = Result<T, CrdtError>;

/// Collect the names of any EMPTY (after trim) required identity fields. Used by the pre-flight guard so
/// the client never sends a request the backend will 400 (RISK-5 / MC-3).
fn empty_identity_fields(pairs: &[(&'static str, &str)]) -> Vec<&'static str> {
    pairs
        .iter()
        .filter(|(_, v)| v.trim().is_empty())
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Conflict bus seam (contract note 10 — a push denial MUST NOT be silently swallowed).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A listener the editor layer registers so a push [`YjsPushOutcomeV1::Denied`] immediately triggers the
/// conflict UI (contract note 10 — "do NOT silently swallow denials"). The existing
/// `crate::event_bus::ShellEventBus` carries only document/canvas/bookmark deletions (no CRDT-conflict
/// variant), and `crate::interop::interaction_bus` is an egui-frame-thread `Arc<Mutex<..>>` that this
/// async, non-egui transport client must not lock off-thread; so the denial is surfaced through this
/// minimal injected callback seam instead. The editor wires it (in the egui layer) to push the denial
/// onto whichever bus the conflict panel drains. When no listener is registered the client still returns
/// the typed `Ok(Denied)` (the outcome is never lost), so this seam is an ADDITIONAL notification, not the
/// only path.
pub type CrdtConflictListener = std::sync::Arc<dyn Fn(&YjsPushDenialV1) + Send + Sync>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The stateless typed client for the `/knowledge/crdt/*` collaborative-editing surface. Holds ONLY a
/// shared [`reqwest::Client`] (cheaply cloneable; an `Arc` internally), the base URL (config-resolved
/// from [`crate::backend_client::BACKEND_BASE_URL`], NEVER hardcoded at a call site —
/// GLOBAL-PORTABILITY-004), and an OPTIONAL [`CrdtConflictListener`] — NO CRDT state.
#[derive(Clone)]
pub struct KnowledgeCrdtClient {
    client: reqwest::Client,
    base_url: String,
    on_denied: Option<CrdtConflictListener>,
}

impl Default for KnowledgeCrdtClient {
    fn default() -> Self {
        Self::production()
    }
}

impl KnowledgeCrdtClient {
    /// Construct against the production backend base URL (the same `BACKEND_BASE_URL` every other native
    /// client uses — config-resolved, not hardcoded here), sharing the ONE process-wide
    /// [`crate::backend_client::shared_http_client`] connection pool rather than minting a second reqwest
    /// stack (the MT-037 REUSE-NOT-DUPLICATE pool concern).
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool). The base URL is the authority for the host — a function never hardcodes
    /// one.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            on_denied: None,
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (e.g. the WP-011 [`crate::backend_client`] pool)
    /// so the app shares ONE connection pool.
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            on_denied: None,
        }
    }

    /// Register the conflict listener (contract note 10). Returns `self` for builder chaining. When a
    /// push returns [`YjsPushOutcomeV1::Denied`], the client invokes this listener with the typed
    /// [`YjsPushDenialV1`] BEFORE returning, so the editor triggers the conflict UI immediately and the
    /// denial is never silently swallowed.
    pub fn with_conflict_listener(mut self, listener: CrdtConflictListener) -> Self {
        self.on_denied = Some(listener);
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// `POST /knowledge/crdt/updates/push` — ingest one Yjs update envelope.
    ///
    /// A 200 deserializes to `Ok(Stored | AlreadyStored)`; a **409** deserializes to
    /// `Ok(`[`YjsPushOutcomeV1::Denied`]`)` (NOT an `Err` — the load-bearing rule) and additionally fires
    /// the registered [`CrdtConflictListener`]; any OTHER non-2xx status is `Err(CrdtError::HttpError)`.
    ///
    /// Pre-flight: the envelope's `actor_id` / `session_id` / `trace_id` MUST be non-empty (the CRDT push
    /// identity lives in the body) or this returns `Err(CrdtError::IdentityMissing)` BEFORE opening a
    /// socket (RISK-5 / MC-3).
    pub async fn push_update(
        &self,
        envelope: &YjsUpdateEnvelopeV1,
    ) -> CrdtResult<YjsPushOutcomeV1> {
        let missing = empty_identity_fields(&[
            ("actor_id", &envelope.actor_id),
            ("session_id", &envelope.session_id),
            ("trace_id", &envelope.trace_id),
        ]);
        if !missing.is_empty() {
            return Err(CrdtError::IdentityMissing(missing.join(", ")));
        }

        let body = PushUpdateRequest {
            envelope: envelope.clone(),
        };
        let resp = self
            .client
            .post(self.url("/knowledge/crdt/updates/push"))
            .json(&body)
            .send()
            .await
            .map_err(|e| CrdtError::Transport(e.to_string()))?;

        let status = resp.status();
        let code = status.as_u16();
        // 200 (Stored/AlreadyStored) AND 409 (Denied) BOTH carry a PushUpdateResponse body; the status
        // distinguishes them. Parse the body for either, and return Denied as Ok (the load-bearing rule).
        if status.is_success() || code == 409 {
            let parsed: PushUpdateResponse = resp
                .json()
                .await
                .map_err(|e| CrdtError::SerializationError(e.to_string()))?;
            if let YjsPushOutcomeV1::Denied { denial } = &parsed.result {
                if let Some(listener) = &self.on_denied {
                    listener(denial);
                }
            }
            return Ok(parsed.result);
        }
        // Any OTHER non-2xx (400 missing identity, 404, 5xx, ...) is a genuine error.
        let body = resp.text().await.unwrap_or_default();
        Err(CrdtError::HttpError { status: code, body })
    }

    /// `GET /knowledge/crdt/updates/pull` — replay feed of updates with `update_seq > since_update_seq`.
    ///
    /// `since_update_seq` defaults to 0 (replay from the start) when `None`; the chosen value is asserted
    /// on the outgoing `?since_update_seq=` query (MC-5). The caller tracks the last seen `update_seq`
    /// and passes it on the next pull to avoid replaying old updates (RISK-3). An empty CRDT doc returns
    /// `updates: []` + `head_update_seq: 0` — a NORMAL empty state, NOT an error (RISK-8).
    pub async fn pull_updates(
        &self,
        params: &PullUpdatesParams,
    ) -> CrdtResult<YjsUpdatePullResponseV1> {
        let missing = empty_identity_fields(&[
            ("actor_id", &params.actor_id),
            ("session_id", &params.session_id),
            ("correlation_id", &params.correlation_id),
        ]);
        if !missing.is_empty() {
            return Err(CrdtError::IdentityMissing(missing.join(", ")));
        }

        let since = params.since_update_seq.unwrap_or(0);
        let query: Vec<(&str, String)> = vec![
            ("workspace_id", params.workspace_id.clone()),
            ("document_id", params.document_id.clone()),
            ("crdt_document_id", params.crdt_document_id.clone()),
            ("since_update_seq", since.to_string()),
            ("document_schema_id", params.document_schema_id.clone()),
            ("actor_id", params.actor_id.clone()),
            ("session_id", params.session_id.clone()),
            ("correlation_id", params.correlation_id.clone()),
        ];
        let resp = self
            .client
            .get(self.url("/knowledge/crdt/updates/pull"))
            .query(&query)
            .send()
            .await
            .map_err(|e| CrdtError::Transport(e.to_string()))?;

        let parsed: PullUpdatesResponse = self.parse_success(resp).await?;
        Ok(parsed.result)
    }

    /// `GET /knowledge/crdt/conflict_state` — the typed conflict UI payload (read-only diagnostic). The
    /// result drives the conflict resolution panel ([`ConflictUiStateV1::has_conflict`] / `conflicts`).
    pub async fn conflict_state(
        &self,
        params: &ConflictStateParams,
    ) -> CrdtResult<ConflictUiStateV1> {
        let missing = empty_identity_fields(&[
            ("actor_id", &params.actor_id),
            ("session_id", &params.session_id),
            ("correlation_id", &params.correlation_id),
        ]);
        if !missing.is_empty() {
            return Err(CrdtError::IdentityMissing(missing.join(", ")));
        }

        let query: Vec<(&str, String)> = vec![
            ("workspace_id", params.workspace_id.clone()),
            ("document_id", params.document_id.clone()),
            ("crdt_document_id", params.crdt_document_id.clone()),
            ("actor_id", params.actor_id.clone()),
            ("session_id", params.session_id.clone()),
            ("correlation_id", params.correlation_id.clone()),
        ];
        let resp = self
            .client
            .get(self.url("/knowledge/crdt/conflict_state"))
            .query(&query)
            .send()
            .await
            .map_err(|e| CrdtError::Transport(e.to_string()))?;

        let parsed: ConflictStateResponse = self.parse_success(resp).await?;
        Ok(parsed.result)
    }

    /// Parse a 2xx body into `T`; map any non-2xx status to [`CrdtError::HttpError`]. Used by the pull /
    /// conflict_state reads (which have NO 409 domain outcome — only push does). The body is read once.
    async fn parse_success<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> CrdtResult<T> {
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<T>()
                .await
                .map_err(|e| CrdtError::SerializationError(e.to_string()));
        }
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(CrdtError::HttpError { status: code, body })
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit proofs for the (de)serialization contracts that DO NOT need a socket. The wire-level
    //! proofs (mock-server round-trips: push 200/409, pull base64 decode, conflict_state, since_update_seq
    //! on the wire, identity pre-flight) live in `tests/test_knowledge_crdt.rs` against REAL backend
    //! payload shapes.

    use super::*;
    use serde_json::json;

    #[test]
    fn base64_roundtrip_uses_standard_engine_byte_for_byte() {
        // A spread of bytes incl. 0x00, 0xFF, and values that DIFFER between STANDARD and url-safe
        // alphabets (0x3E -> '+'/'-' boundary, 0x3F -> '/'/'_' boundary) so a url-safe engine would FAIL.
        let raw: Vec<u8> = vec![0x00, 0xFF, 0x3E, 0x3F, 0xFB, 0xEF, b'h', b'i', 0x10, 0x80];
        let env = YjsUpdateEnvelopeV1 {
            schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
            workspace_id: "WS-1".into(),
            document_id: "DOC-1".into(),
            crdt_document_id: "KCRDT-1".into(),
            update_id: "U-1".into(),
            actor_id: "operator:ilja".into(),
            site_id: "SITE-1".into(),
            session_id: "SESS-1".into(),
            trace_id: "TRACE-1".into(),
            document_schema_id: "prosemirror-v1".into(),
            update_b64: String::new(),
            update_sha256: String::new(),
            state_vector_before: String::new(),
            state_vector_after: String::new(),
            encoding: YJS_UPDATE_ENCODING_V1.to_string(),
        }
        .with_update_bytes(&raw);

        // STANDARD alphabet encodes 0x3E/0x3F as '+'/'/'; assert we are NOT url-safe.
        assert!(
            env.update_b64.contains('+') || env.update_b64.contains('/'),
            "STANDARD alphabet must use +// (got {}); a url-safe engine would silently corrupt updates",
            env.update_b64
        );
        // Roundtrip is byte-for-byte exact.
        assert_eq!(env.update_bytes().unwrap(), raw, "base64 STANDARD roundtrip must be lossless");
    }

    #[test]
    fn push_outcome_tagged_union_matches_backend_outcome_tags() {
        // The backend tags the enum by `outcome` (snake_case). Verify each variant deserializes from the
        // real tag string — a divergent tag would silently fail to parse the backend's response.
        let stored: YjsPushOutcomeV1 = serde_json::from_value(json!({
            "outcome": "stored",
            "update_seq": 7u64,
            "update_id": "U-7",
            "event_ledger_event_id": "EVT-7",
            "head_state_vector": "sv-after"
        }))
        .unwrap();
        assert!(matches!(stored, YjsPushOutcomeV1::Stored { update_seq: 7, .. }));

        let already: YjsPushOutcomeV1 = serde_json::from_value(json!({
            "outcome": "already_stored",
            "update_seq": 7u64,
            "update_id": "U-7",
            "event_ledger_event_id": "EVT-7",
            "head_state_vector": "sv"
        }))
        .unwrap();
        assert!(matches!(already, YjsPushOutcomeV1::AlreadyStored { .. }));

        let denied: YjsPushOutcomeV1 = serde_json::from_value(json!({
            "outcome": "denied",
            "denial": {
                "schema_id": YJS_PUSH_DENIAL_SCHEMA_ID,
                "crdt_document_id": "KCRDT-1",
                "update_id": "U-1",
                "actor_id": "operator:ilja",
                "reason": {"code": "stale_base", "head_update_seq": 3u64, "head_state_vector": "sv", "ordering": "Concurrent"}
            }
        }))
        .unwrap();
        match denied {
            YjsPushOutcomeV1::Denied { denial } => {
                assert!(matches!(denial.reason, YjsPushDenialReasonV1::StaleBase { .. }));
                assert_eq!(denial.update_id, "U-1");
            }
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    #[test]
    fn denial_reason_tagged_by_code_snake_case() {
        let stale: YjsPushDenialReasonV1 = serde_json::from_value(json!({
            "code": "stale_base",
            "head_update_seq": 5u64,
            "head_state_vector": "sv",
            "ordering": "Dominates"
        }))
        .unwrap();
        assert!(matches!(stale, YjsPushDenialReasonV1::StaleBase { head_update_seq: 5, .. }));

        let invalid: YjsPushDenialReasonV1 = serde_json::from_value(json!({
            "code": "envelope_invalid",
            "messages": ["update bytes must not be empty"]
        }))
        .unwrap();
        assert!(matches!(invalid, YjsPushDenialReasonV1::EnvelopeInvalid { .. }));
    }

    #[test]
    fn conflict_state_has_conflict_derived_from_conflicts() {
        let empty = ConflictUiStateV1 {
            schema_id: "hsk.kernel.knowledge_conflict_ui_state@1".into(),
            workspace_id: "WS-1".into(),
            document_id: "DOC-1".into(),
            crdt_document_id: "KCRDT-1".into(),
            head_update_seq: 0,
            head_state_vector: String::new(),
            conflicts: vec![],
        };
        assert!(!empty.has_conflict(), "no conflicts -> has_conflict false");
        assert_eq!(empty.denial_count(), 0);
    }

    #[test]
    fn identity_pre_flight_detects_empty_fields() {
        // Whitespace-only counts as empty (the backend trims).
        let missing = empty_identity_fields(&[
            ("actor_id", "operator:ilja"),
            ("session_id", "   "),
            ("trace_id", ""),
        ]);
        assert_eq!(missing, vec!["session_id", "trace_id"]);
    }
}
