//! FEMS memory-write PROPOSAL from the editor (WP-KERNEL-012 MT-064, cluster E9 — Pillar 12).
//!
//! ## What this is (the editor entry point into the FEMS proposal→review→commit loop)
//!
//! This module turns the current editor selection (the MT-031 [`SharedSelection`]) into a typed,
//! review-gated FEMS memory-write PROPOSAL and submits it to the governed FEMS write path. The editor
//! never writes a `MemoryItem` directly: approval first records the auditable review decision, then an
//! explicit commit request asks the backend to atomically write the canonical item, `MemoryCommitReport`,
//! strict `MemoryPack`, EventLedger receipt, and FR-EVT-MEM-003 projection.
//!
//! ## THE LOAD-BEARING INVARIANT (review-gated, never-editor-direct)
//!
//! - [`MemoryWriteProposal::is_review_gated`] is ALWAYS `true`. It is HARD-set `true` for the `Procedural`
//!   class (the spec requirement) and `true` for `Episodic`/`Semantic` too — the editor can NEVER set it
//!   `false` for any class. There is no constructor, setter, or method that yields a `review_gated=false`
//!   proposal (MC-002, AC-002).
//! - On a missing proposal endpoint the editor returns the typed blocker
//!   [`MemoryProposalError::MissingEndpoint`] and writes nothing — it does NOT fall back to a direct
//!   memory write (RISK-004, MC-004, AC-005).
//!
//! ## Live backend path
//!
//! `handshake_core::api::memory` owns `POST /workspaces/{id}/memory/proposals`. A successful request
//! stores a pending-review proposal and returns its durable proposal id. A 404 still maps to the typed
//! [`MemoryProposalError::MissingEndpoint`] for compatibility with a backend that has not mounted the
//! capability; no failure path falls back to a direct memory write.
//! [`review_proposal`] records an explicit native operator approval/rejection through the closed review
//! route. Approval then calls the separate approved-proposal commit route and returns both review and
//! commit receipt identities; rejection performs no commit.
//!
//! ## FR-EVT-MEM-001 transaction outbox
//!
//! handshake_core commits the proposal, EventLedger receipt, and normative FR-EVT-MEM-001 outbox row
//! in one PostgreSQL transaction. It projects that row idempotently and returns its durable event UUID
//! in the acknowledgement. [`submit_proposal_and_emit`] retains its historical signature but does not
//! enqueue a duplicate native-editor envelope.
//!
//! ## content_hash REUSES the MT-032 loom content-hash primitive (no second hashing scheme)
//!
//! [`MemorySourceProvenance::content_hash`] is computed by [`content_hash_of_selection`], which reuses
//! the MT-032/MT-020 canonical-JSON SHA-256 primitive
//! ([`crate::rich_editor::save::canonical_hash::canonical_content_sha256`], the SAME primitive
//! [`crate::loom_address::ContentHash::of_content_json`] uses for a Loom block) over the selected content
//! wrapped as a JSON string value. The result is lowercase hex (64 chars), byte-identical to the loom
//! block hash for identical content — so a proposal's hash matches the document's block hash for the same
//! content. NO second hashing crate/scheme is introduced (RISK-005, MC-005, AC-003).
//!
//! ## Off-thread submission (HBR-QUIET) — bounded by the MT-036 emitter's semaphore
//!
//! [`submit_proposal`] is `async` and is dispatched off the egui frame thread by the host (the same
//! pattern MT-036 uses); it never blocks the frame. The FR emit it triggers goes through the MT-036
//! emitter, which is itself semaphore-bounded (drop + error-ring on saturation) and never blocks/crashes
//! the frame (RISK-006, MC-006). A failed emit lands in the MT-036 error ring, never panics the frame.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::accessibility::emit_interactive_node;
use crate::event_emitter::{NativeEditorEvent, NativeEditorEventEmitter};
use crate::interop::SharedSelection;
use crate::rich_editor::save::canonical_hash::canonical_content_sha256;
use crate::theme::HsPalette;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AccessKit identities (HBR-SWARM, AC-007) — the dialog + class radios + confirm button.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// AccessKit author_id for the proposal confirmation dialog root (`Role::Dialog`, modal).
pub const FEMS_PROPOSE_DIALOG_AUTHOR_ID: &str = "fems-propose-dialog";

/// AccessKit author_id for the confirm button (`Role::Button`).
pub const FEMS_PROPOSE_CONFIRM_AUTHOR_ID: &str = "fems-propose-confirm";

/// AccessKit author_id for the cancel button (`Role::Button`).
pub const FEMS_PROPOSE_CANCEL_AUTHOR_ID: &str = "fems-propose-cancel";

/// AccessKit author_id for the mounted proposal outcome (`Role::Status`). Its value is a stable
/// semicolon-delimited machine projection containing `state`, `outcome`, and correlation ids.
pub const FEMS_PROPOSE_STATUS_AUTHOR_ID: &str = "fems-propose-status";

/// AccessKit author_id for the operator approval control shown after a proposal is durably queued.
pub const FEMS_REVIEW_APPROVE_AUTHOR_ID: &str = "fems-review-approve";

/// AccessKit author_id for the operator rejection control shown after a proposal is durably queued.
pub const FEMS_REVIEW_REJECT_AUTHOR_ID: &str = "fems-review-reject";

/// AccessKit author_id for the structured terminal/pending review status.
pub const FEMS_REVIEW_STATUS_AUTHOR_ID: &str = "fems-review-status";
/// AccessKit author_id for retrying a failed canonical pending-review list refresh.
pub const FEMS_REVIEW_REFRESH_RETRY_AUTHOR_ID: &str = "fems-review-refresh-retry";

/// AccessKit author_id PREFIX for a class radio (`fems-class-{episodic|semantic|procedural}`,
/// `Role::RadioButton`). The full id is built by [`fems_class_author_id`].
pub const FEMS_CLASS_AUTHOR_PREFIX: &str = "fems-class-";

// ── Fixed AccessKit `NodeId` for the dialog ROOT (the WP-011 registry convention — AC-007, MC-010) ──
// Every other shell modal dialog (command_palette/quick_switcher/settings) pins a fixed `NodeId` for its
// dialog ROOT container and enrolls it in `accessibility::registry::DECLARED_IDENTITIES` so the
// compile-time collision/coverage test proves the id is globally unique (de-duplicated, RISK-010). The
// proposal dialog root takes the FRESH band slot 25 (above the MT-022 search-rail band 22..=24, below the
// divider band 30..=31, strictly below the pane id base 100). A fixed-value `egui::Id`
// (`from_high_entropy_bits`) yields the same `NodeId` across frames + restarts. The dialog renders ONLY
// while the proposal dialog is open, so the default-seed live tree never contains it — but the collision
// test still covers it. The three class RADIOS + the confirm BUTTON are NON-container controls addressed
// by their stable author_id STRING in egui's HASHED id space (the exact convention the settings-dialog
// form controls + the palette command rows + the palette Close button use — see registry.rs), so they are
// NOT enumerated in DECLARED_IDENTITIES; the dialog ROOT is the one fixed-band identity this MT declares.

/// Fixed AccessKit/egui `NodeId` of the proposal dialog root (`Role::Dialog`, modal). Fresh band slot 25.
pub const FEMS_PROPOSE_DIALOG_NODE_ID: u64 = 25;

/// The dispatch id of the "Propose to Memory" command registered into the WP-011 command registry
/// (`fems.propose_to_memory`; palette-driven, no keybind — does NOT steal a VS Code binding, RISK-010).
pub const FEMS_PROPOSE_COMMAND_ID: &str = "fems.propose_to_memory";

/// The operator/model-facing label for the command + dialog title.
pub const FEMS_PROPOSE_COMMAND_LABEL: &str = "Propose to Memory";

/// Build the stable AccessKit author_id for a class radio (`fems-class-{episodic|semantic|procedural}`).
pub fn fems_class_author_id(class: MemoryClass) -> String {
    format!("{FEMS_CLASS_AUTHOR_PREFIX}{}", class.wire())
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The proposal data model.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The three Pillar 12 memory classes a proposal can target. Serialized lowercase on the wire
/// (`"episodic"` | `"semantic"` | `"procedural"`) so the typed enum round-trips the FEMS proposal body.
/// Mirrors [`crate::fems::memory_client::MemoryKind`] but is owned here because a proposal is a WRITE
/// payload (the read model lives in `memory_client`); keeping them distinct avoids coupling the write
/// path to the read path's tolerant-decode concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryClass {
    /// What happened: events, prior sessions, edits.
    Episodic,
    /// Durable facts / concepts.
    Semantic,
    /// How-to steps, recipes, workflows. Procedural-class proposals are ALWAYS review-gated (spec).
    Procedural,
}

impl MemoryClass {
    /// The stable lowercase wire string for the class (the value in the proposal `class` field + the
    /// AccessKit radio id suffix).
    pub fn wire(self) -> &'static str {
        match self {
            MemoryClass::Episodic => "episodic",
            MemoryClass::Semantic => "semantic",
            MemoryClass::Procedural => "procedural",
        }
    }

    /// The operator/model-facing radio label.
    pub fn label(self) -> &'static str {
        match self {
            MemoryClass::Episodic => "Episodic",
            MemoryClass::Semantic => "Semantic",
            MemoryClass::Procedural => "Procedural",
        }
    }

    /// The three classes in their fixed dialog order (Episodic default, then Semantic, then Procedural).
    pub const ORDER: [MemoryClass; 3] = [
        MemoryClass::Episodic,
        MemoryClass::Semantic,
        MemoryClass::Procedural,
    ];

    /// The default class for a new editor→memory proposal: `Episodic` (the most common editor case; the
    /// operator can switch to Semantic/Procedural in the dialog before confirming).
    pub const DEFAULT: MemoryClass = MemoryClass::Episodic;
}

/// Full source provenance for a memory-write proposal: WHERE the content came from, so the proposal can
/// be traced back to its exact origin and deduped/verified against the source document (RISK-003). Every
/// field is populated by [`build_proposal`] from the [`SharedSelection`]; the `content_hash` reuses the
/// MT-032 loom hash so it matches the document block hash for identical content (RISK-005, AC-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySourceProvenance {
    /// The canonical document/source the content was selected from. Rich text carries its persisted
    /// RichDocument id; code carries its PostgreSQL `KnowledgeSource` (`KSRC-*`) id separately from the
    /// filesystem tab key. A `BlockRef`/`NodeRef` carries its already-addressable block/node id.
    pub document_id: String,
    /// The start byte offset of the selection inside the document (a whole-block/whole-node selection
    /// uses `0`).
    pub selection_start: usize,
    /// The end byte offset of the selection inside the document (a whole-block/whole-node selection uses
    /// the content length, or `0` when the content is not materialized for a ref selection).
    pub selection_end: usize,
    /// The MT-032 loom content hash of the exact selected content (lowercase hex, 64 chars). Byte-
    /// identical to the document block hash for identical content (no second hashing scheme — AC-003).
    pub content_hash: String,
    /// Raw SHA-256 of the complete source document when the source is a canonical code file. The
    /// backend compares this to the `KnowledgeSource.content_hash` before trusting the selected slice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_content_hash: Option<String>,
    /// The pane that owns the selection (the editor surface instance the proposal originated from).
    pub pane_id: String,
    /// The workspace the proposal is scoped to (the path parameter of the proposal POST).
    pub workspace_id: String,
}

/// A typed, review-gated FEMS memory-write proposal built from an editor selection. The editor submits
/// this to the review-gated FEMS write path; the commit is downstream and review-gated, never
/// editor-direct. [`Self::is_review_gated`] is ALWAYS `true` (the load-bearing invariant — MC-002,
/// AC-002); no deserializer or public field can construct a false-gated value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWriteProposal {
    /// The Pillar 12 class this proposal targets.
    pub class: MemoryClass,
    /// The exact selected content the proposal carries.
    pub content: String,
    /// Full source provenance (document_id + selection range + content hash + pane + workspace).
    pub source: MemorySourceProvenance,
    /// Transient complete code-document snapshot used only by the backend provenance gate. It is sent
    /// with the intake request but is not included in the stored proposal or Flight Recorder payload.
    pub source_document_content: Option<String>,
    /// Private zero-sized proof that this value came through the review-gated constructor. Keeping this
    /// private and non-deserializable prevents untrusted JSON or a public struct literal from forging an
    /// editor-direct proposal. The wire and FR projections always emit the literal `true`.
    review_gate: ReviewGate,
    /// The acting operator/model session id (for attribution on the review queue).
    pub actor_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewGate;

impl MemoryWriteProposal {
    /// True iff this proposal is review-gated (ALWAYS true — the invariant). Exposed so a test/consumer
    /// can assert the never-editor-direct contract without reaching into the field.
    pub fn is_review_gated(&self) -> bool {
        true
    }

    /// Legacy native-editor correlation payload retained for decoding older proof data. New proposal
    /// submissions do not emit this envelope: handshake_core projects the normative FR-EVT-MEM-001
    /// from the PostgreSQL transaction outbox and returns its durable event UUID in [`ProposalAck`].
    pub fn fr_payload(&self, proposal_id: &str) -> JsonValue {
        json!({
            "action": "memory_write_proposed",
            "proposal_id": proposal_id,
            "status": "pending_review",
            "class": self.class.wire(),
            "document_id": self.source.document_id,
            "selection_start": self.source.selection_start,
            "selection_end": self.source.selection_end,
            "content_hash": self.source.content_hash,
            "review_gated": true,
            "pane_id": self.source.pane_id,
        })
    }

    /// Rebuild a legacy native-editor correlation event for historical proof-data compatibility.
    /// Production submission uses the backend-owned normative event and never calls this method.
    pub fn fr_event(&self, proposal_id: &str) -> NativeEditorEvent {
        use crate::event_emitter::NativeEditorAction;
        let mut event = NativeEditorEvent::new(
            NativeEditorAction::MemoryWriteProposed,
            self.source.pane_id.clone(),
            self.actor_id.clone(),
            self.source.workspace_id.clone(),
            self.fr_payload(proposal_id),
        );
        if let Some(event_id) = stable_proposal_event_id(proposal_id) {
            event.event_id = event_id;
        }
        event
    }
}

/// Map the backend's stable `PROP-<sha256>` identity to a standards-shaped UUIDv8. A retry of the same
/// review-gated proposal must address the same Flight Recorder/EventLedger event instead of producing a
/// second observability row. UUIDv8 is the RFC custom namespace; the 122 payload bits remain derived from
/// the proposal digest while the version and variant bits are normalized.
fn stable_proposal_event_id(proposal_id: &str) -> Option<String> {
    let digest = proposal_id.strip_prefix("PROP-")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&digest[offset..offset + 2], 16).ok()?;
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(uuid::Uuid::from_bytes(bytes).to_string())
}

/// The typed outcome of a proposal build/submit. [`Self::MissingEndpoint`] is the FIRST-CLASS TYPED
/// BLOCKER (RISK-004, MC-004, AC-005): returned when the FEMS proposal write route is absent (a 404 / a
/// route-absent / capability-missing response). It is NEVER swallowed and NEVER a reason to write memory
/// directly — the editor surfaces it and writes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryProposalError {
    /// The FEMS proposal write route is absent in this handshake_core build (the TYPED BLOCKER). Carries
    /// the path probed so the validator sees exactly which route is missing.
    MissingEndpoint { probed_path: String },
    /// The review route exists, but the durable proposal is no longer present in this workspace.
    ReviewTargetMissing { probed_path: String },
    /// Another reviewer already recorded a different terminal decision for this proposal.
    ReviewConflict(String),
    /// The server returned a syntactically valid acknowledgement that did not match the requested
    /// proposal/decision identity. Treat this as terminal uncertainty rather than reporting success.
    ReviewAckMismatch(String),
    /// The [`SharedSelection`] was [`SharedSelection::None`] — there is nothing to propose. The dialog
    /// is not opened / the command is a no-op in this state (never a fabricated empty proposal).
    NoSelection,
    /// A text selection identifies its owning pane but the SharedSelection contract does not carry the
    /// pane's active document id. The app must resolve that id from the owning pane's active tab before
    /// building a proposal; using the pane id as a document id would forge provenance.
    MissingDocumentIdentity { pane_id: String },
    /// The materialized selection is empty, so it cannot carry reviewable memory content.
    EmptySelection,
    /// SharedSelection byte offsets must cover exactly the selected UTF-8 bytes.
    SelectionRangeMismatch {
        start: usize,
        end: usize,
        content_bytes: usize,
    },
    /// The proposal POST reached the server but failed (non-2xx that is NOT a 404, transport, or decode).
    /// Carries the reason. NOT a typed blocker — an ordinary submit failure surfaced to the operator.
    SubmitFailed(String),
}

impl std::fmt::Display for MemoryProposalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEndpoint { probed_path } => write!(
                f,
                "FEMS proposal write endpoint not present in this build (probed {probed_path})"
            ),
            Self::ReviewTargetMissing { probed_path } => write!(
                f,
                "FEMS proposal is no longer available for review (probed {probed_path})"
            ),
            Self::ReviewConflict(reason) => {
                write!(f, "FEMS proposal already has a conflicting review: {reason}")
            }
            Self::ReviewAckMismatch(reason) => {
                write!(f, "FEMS proposal review acknowledgement mismatch: {reason}")
            }
            Self::NoSelection => write!(f, "no selection to propose to memory"),
            Self::MissingDocumentIdentity { pane_id } => write!(
                f,
                "selection in pane {pane_id} has no authoritative active document identity"
            ),
            Self::EmptySelection => write!(f, "memory proposal selection must not be empty"),
            Self::SelectionRangeMismatch {
                start,
                end,
                content_bytes,
            } => write!(
                f,
                "selection byte range {start}..{end} does not cover the {content_bytes}-byte content"
            ),
            Self::SubmitFailed(reason) => write!(f, "proposal submit failed: {reason}"),
        }
    }
}

impl std::error::Error for MemoryProposalError {}

impl MemoryProposalError {
    /// True when this is the typed-blocker variant (the editor surfaces it and writes nothing).
    pub fn is_missing_endpoint(&self) -> bool {
        matches!(self, MemoryProposalError::MissingEndpoint { .. })
    }
}

/// The server's acknowledgement of a submitted proposal (the review queue accepted it). The commit is
/// still downstream + review-gated; this only confirms the PROPOSAL was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposalAck {
    /// The id the review queue assigned the proposal (carried into the FR-EVT-MEM-001 event).
    pub proposal_id: String,
    /// The proposal's review status (e.g. `"pending_review"`). Never `"committed"` from the editor path.
    pub status: String,
    /// Canonical proposal creation time returned from PostgreSQL. Retries return the original value so
    /// the correlated native-editor event can replay the exact same immutable envelope.
    pub created_at: String,
    /// Durable canonical FR-EVT-MEM-001 UUID projected by handshake_core's transactional outbox.
    pub flight_recorder_event_id: String,
}

fn is_canonical_proposal_id(value: &str) -> bool {
    value.strip_prefix("PROP-").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn validate_proposal_ack(ack: &ProposalAck) -> Result<(), MemoryProposalError> {
    if !is_canonical_proposal_id(&ack.proposal_id) {
        return Err(MemoryProposalError::ReviewAckMismatch(format!(
            "proposal acknowledgement id is not canonical: {}",
            ack.proposal_id
        )));
    }
    if !matches!(
        ack.status.as_str(),
        "pending_review" | "approved" | "rejected" | "committed"
    ) {
        return Err(MemoryProposalError::ReviewAckMismatch(format!(
            "proposal acknowledgement status expected a canonical lifecycle state, received {}",
            ack.status
        )));
    }
    if chrono::DateTime::parse_from_rfc3339(&ack.created_at).is_err() {
        return Err(MemoryProposalError::ReviewAckMismatch(
            "proposal acknowledgement created_at expected RFC3339".to_owned(),
        ));
    }
    if uuid::Uuid::parse_str(&ack.flight_recorder_event_id).is_err() {
        return Err(MemoryProposalError::ReviewAckMismatch(
            "proposal acknowledgement Flight Recorder event id expected UUID".to_owned(),
        ));
    }
    Ok(())
}

/// The two operator decisions accepted by the closed FEMS proposal-review route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReviewDecision {
    Approved,
    Rejected,
}

impl ProposalReviewDecision {
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

/// Durable acknowledgement returned by the FEMS proposal-review route. These identities bridge the
/// native control to the exact EventLedger and Flight Recorder evidence rows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposalReviewAck {
    pub proposal_id: String,
    pub status: String,
    pub decision: ProposalReviewDecision,
    pub reviewer_kind: String,
    pub actor_id: String,
    pub correlation_id: String,
    pub event_ledger_event_id: String,
    pub flight_recorder_event_id: String,
    pub reviewed_at: String,
    /// Present only when an approval completed the explicit governed commit step.
    #[serde(default)]
    pub commit: Option<ProposalCommitAck>,
}

/// Durable acknowledgement returned by the explicit approved-proposal commit route.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposalCommitAck {
    pub proposal_id: String,
    pub status: String,
    pub commit_id: String,
    pub memory_id: String,
    pub memory_pack_id: String,
    pub memory_pack_hash: String,
    pub commit_report: MemoryCommitReport,
    pub commit_report_hash: String,
    pub event_ledger_event_id: String,
    pub flight_recorder_event_id: String,
    pub committed_at: String,
}

/// Lifecycle states which expose a native operator action. Terminal proposals are deliberately not
/// representable here, so a replayed rejected/committed acknowledgement cannot revive review controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionableProposalLifecycle {
    PendingReview,
    Approved,
}

impl ActionableProposalLifecycle {
    pub const fn wire(self) -> &'static str {
        match self {
            Self::PendingReview => "pending_review",
            Self::Approved => "approved",
        }
    }
}

/// Typed native projection of the backend `MemoryCommitReport` contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCommitReport {
    pub schema_version: String,
    pub commit_id: String,
    pub created_at: String,
    pub source_proposal_id: String,
    pub applied_ops: Vec<MemoryCommitAppliedOp>,
    pub warnings: Vec<String>,
    pub pack_rebuild_hints: Vec<MemoryPackRebuildHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCommitAppliedOp {
    pub op: String,
    pub memory_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_version: Option<u32>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPackRebuildHint {
    pub scope_ref: MemoryCommitScopeRef,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCommitScopeRef {
    pub artefact_type: String,
    pub artefact_id: String,
    pub selector: String,
}

/// Minimal canonical row needed to recover an actionable native review/commit after dismissal,
/// restart, or workspace rebinding. Extra backend projection fields are deliberately ignored.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActionableProposalSummary {
    pub proposal_id: String,
    pub workspace_id: String,
    pub status: ActionableProposalLifecycle,
    pub review_gated: bool,
    pub created_at: String,
}

const REVIEW_ACTOR_ID: &str = "native-editor-fems-reviewer";

fn validate_review_ack(
    ack: &ProposalReviewAck,
    proposal_id: &str,
    decision: ProposalReviewDecision,
) -> Result<(), MemoryProposalError> {
    let expected_correlation = format!("fems-memory-proposal-review:{proposal_id}");
    let mismatch = if ack.proposal_id != proposal_id {
        Some(format!(
            "proposal_id expected {proposal_id}, received {}",
            ack.proposal_id
        ))
    } else if ack.decision != decision {
        Some(format!(
            "decision expected {}, received {}",
            decision.wire(),
            ack.decision.wire()
        ))
    } else if ack.status != decision.wire()
        && !(decision == ProposalReviewDecision::Approved && ack.status == "committed")
    {
        Some(format!(
            "status expected {} lifecycle-compatible state, received {}",
            decision.wire(),
            ack.status
        ))
    } else if ack.reviewer_kind != "user" {
        Some(format!(
            "reviewer_kind expected user, received {}",
            ack.reviewer_kind
        ))
    } else if ack.actor_id != REVIEW_ACTOR_ID {
        Some(format!(
            "actor_id expected {REVIEW_ACTOR_ID}, received {}",
            ack.actor_id
        ))
    } else if ack.correlation_id != expected_correlation {
        Some(format!(
            "correlation_id expected {expected_correlation}, received {}",
            ack.correlation_id
        ))
    } else if !ack
        .event_ledger_event_id
        .strip_prefix("KE-")
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    {
        Some("event_ledger_event_id expected KE-<uuid>".to_owned())
    } else if uuid::Uuid::parse_str(&ack.flight_recorder_event_id).is_err() {
        Some("flight_recorder_event_id expected UUID".to_owned())
    } else if chrono::DateTime::parse_from_rfc3339(&ack.reviewed_at).is_err() {
        Some("reviewed_at expected RFC3339".to_owned())
    } else {
        None
    };
    match mismatch {
        Some(reason) => Err(MemoryProposalError::ReviewAckMismatch(reason)),
        None => Ok(()),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The PURE proposal builder (no I/O, no async — trivially unit-testable, AC-001).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The MT-032 loom content hash of a selected content string (lowercase hex, 64 chars), byte-identical
/// to the document block hash for identical content. REUSES [`canonical_content_sha256`] (the MT-020/
/// MT-032 canonical-JSON SHA-256 primitive — the SAME primitive
/// [`crate::loom_address::ContentHash::of_content_json`] uses for a Loom block), wrapping the content as
/// a JSON string value so the hashed bytes are the canonical JSON encoding of that content. NO second
/// hashing scheme is introduced (RISK-005, MC-005, AC-003).
pub fn content_hash_of_selection(content: &str) -> String {
    canonical_content_sha256(&JsonValue::String(content.to_owned()))
}

/// Build a review-gated memory-write proposal from the current [`SharedSelection`]. PURE: no I/O, no
/// async, no side effects — trivially unit-testable (AC-001). Submission and the FR emit consume its
/// output as separate steps.
///
/// Provenance is read from the selection variant:
/// - [`SharedSelection::TextRange`] gives the exact byte range + selected text but not a document id.
///   The live shell must resolve the owning pane's active-tab `content_id` and call
///   [`build_proposal_for_document`]; pane identity is not document provenance.
/// - [`SharedSelection::BlockRef`] / [`SharedSelection::NodeRef`] resolve their `document_id` from the
///   block/node id and use a whole-block/whole-node range (`0..content_len`, where the content is the
///   ref's loom address — the materialized block text is not carried by the ref variant, so the content
///   is the loom URI the host can resolve; the range is `0..content.len()`).
///
/// `review_gated` is ALWAYS `true` (hard-set true for `Procedural`, and true for every other class — the
/// editor can never propose a non-review-gated write, MC-002/AC-002). Returns
/// [`MemoryProposalError::NoSelection`] for [`SharedSelection::None`].
pub fn build_proposal(
    sel: &SharedSelection,
    class: MemoryClass,
    workspace_id: &str,
    actor_id: &str,
) -> Result<MemoryWriteProposal, MemoryProposalError> {
    build_proposal_with_text_document_id(sel, class, workspace_id, actor_id, None, None)
}

/// Build a proposal while supplying the owning text pane's authoritative active document identity.
///
/// `SharedSelection::TextRange` deliberately carries only pane/surface/range/text. The shell owns the
/// pane -> active-tab -> document mapping, so the live product path must resolve that mapping and call
/// this function. BlockRef and NodeRef already carry their canonical addressable identity and ignore the
/// optional text-document argument.
pub fn build_proposal_for_document(
    sel: &SharedSelection,
    class: MemoryClass,
    workspace_id: &str,
    actor_id: &str,
    document_id: &str,
) -> Result<MemoryWriteProposal, MemoryProposalError> {
    build_proposal_with_text_document_id(
        sel,
        class,
        workspace_id,
        actor_id,
        Some(document_id),
        None,
    )
}

/// Build a text proposal with a complete mounted code-document snapshot. The snapshot is hashed here
/// and later verified by the backend against the canonical `KnowledgeSource` row before range slicing.
pub fn build_proposal_for_document_snapshot(
    sel: &SharedSelection,
    class: MemoryClass,
    workspace_id: &str,
    actor_id: &str,
    document_id: &str,
    document_content: String,
) -> Result<MemoryWriteProposal, MemoryProposalError> {
    build_proposal_with_text_document_id(
        sel,
        class,
        workspace_id,
        actor_id,
        Some(document_id),
        Some(document_content),
    )
}

fn build_proposal_with_text_document_id(
    sel: &SharedSelection,
    class: MemoryClass,
    workspace_id: &str,
    actor_id: &str,
    text_document_id: Option<&str>,
    source_document_content: Option<String>,
) -> Result<MemoryWriteProposal, MemoryProposalError> {
    let (document_id, pane_id, selection_start, selection_end, content) = match sel {
        SharedSelection::None => return Err(MemoryProposalError::NoSelection),
        SharedSelection::TextRange {
            pane_id,
            start,
            end,
            text,
            ..
        } => {
            // A pane id is a surface-instance identity, never document provenance. Every TextRange caller
            // must supply the owning pane's authoritative active content_id. Fail closed even in fixtures:
            // accepting pane identity here would make a test-only shortcut reachable as false provenance.
            let document_id = match text_document_id {
                Some(document_id) if !document_id.trim().is_empty() => document_id.trim(),
                Some(_) | None => {
                    return Err(MemoryProposalError::MissingDocumentIdentity {
                        pane_id: pane_id.to_string(),
                    })
                }
            };
            (
                document_id.to_owned(),
                pane_id.to_string(),
                *start,
                *end,
                text.clone(),
            )
        }
        SharedSelection::BlockRef { pane_id, block_id } => {
            // A whole-block selection: the block id IS the document/block id (loom-addressable). The
            // content is the loom address of the block (the ref variant does not carry the block text);
            // the range is the whole content.
            let content = format!("loom://{block_id}");
            (
                block_id.clone(),
                pane_id.to_string(),
                0,
                content.len(),
                content,
            )
        }
        SharedSelection::NodeRef {
            pane_id, node_id, ..
        } => {
            // A whole-node selection (graph/canvas): the node id IS the document/block id. Same shape as
            // BlockRef.
            let content = format!("loom://{node_id}");
            (
                node_id.clone(),
                pane_id.to_string(),
                0,
                content.len(),
                content,
            )
        }
    };

    if content.is_empty() {
        return Err(MemoryProposalError::EmptySelection);
    }
    let range_len = selection_end.checked_sub(selection_start).ok_or(
        MemoryProposalError::SelectionRangeMismatch {
            start: selection_start,
            end: selection_end,
            content_bytes: content.len(),
        },
    )?;
    if range_len != content.len() {
        return Err(MemoryProposalError::SelectionRangeMismatch {
            start: selection_start,
            end: selection_end,
            content_bytes: content.len(),
        });
    }

    let content_hash = content_hash_of_selection(&content);

    let document_content_hash = source_document_content.as_ref().map(|content| {
        Sha256::digest(content.as_bytes()).iter().fold(
            String::with_capacity(64),
            |mut encoded, byte| {
                use std::fmt::Write as _;
                let _ = write!(&mut encoded, "{byte:02x}");
                encoded
            },
        )
    });

    Ok(MemoryWriteProposal {
        class,
        content,
        source: MemorySourceProvenance {
            document_id,
            selection_start,
            selection_end,
            content_hash,
            document_content_hash,
            pane_id,
            workspace_id: workspace_id.to_owned(),
        },
        source_document_content,
        // THE LOAD-BEARING INVARIANT: review_gated is ALWAYS true. There is no path — no constructor, no
        // setter, no class — that yields review_gated=false from the editor. Procedural is review-gated
        // by spec; every other class is review-gated too because the commit is never editor-direct.
        review_gate: ReviewGate,
        actor_id: actor_id.to_owned(),
    })
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The submit path (off-thread, typed-blocker on a missing endpoint, FR emit on success).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The least-privileged actor id used for the proposal write identity headers (so swarm/operator co-work
/// is attributable on the review queue). A proposal is a WRITE-CAPABLE action, so the
/// `x-hsk-actor-kind=human` write-capable kind is attached (unlike the read-only FEMS capsule read).
const FEMS_PROPOSE_ACTOR_KIND: &str = "human";
const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";

/// Read timeout for a single proposal submit. A bounded timeout so a hung backend cannot stall the
/// editor (the submit runs off the frame thread on the shared async runtime).
const SUBMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// The proposal write path for a workspace (the documented review-gated FEMS write route). Built here so
/// the `MissingEndpoint` blocker can report the exact probed path.
pub fn proposal_path(workspace_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/proposals")
}

/// The closed native review route for one durable proposal.
pub fn proposal_review_path(workspace_id: &str, proposal_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/proposals/{proposal_id}/review")
}

/// The explicit commit route. It accepts only a previously approved proposal.
pub fn proposal_commit_path(workspace_id: &str, proposal_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/proposals/{proposal_id}/commit")
}

/// Read the bounded canonical actionable projection used to recover pending review and approved commit.
pub async fn list_actionable_proposals(
    workspace_id: &str,
    client: &HandshakeCoreClient,
) -> Result<Vec<ActionableProposalSummary>, MemoryProposalError> {
    let path = format!("{}?limit=200", proposal_path(workspace_id));
    let response = client
        .authenticated(client.client.get(client.url(&path)).timeout(SUBMIT_TIMEOUT))
        .send()
        .await
        .map_err(|error| {
            MemoryProposalError::SubmitFailed(format!("review list transport: {error}"))
        })?;
    let status = response.status();
    if !status.is_success() {
        let code = status.as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(MemoryProposalError::SubmitFailed(format!(
            "review list status {code}: {text}"
        )));
    }
    let rows = response
        .json::<Vec<ActionableProposalSummary>>()
        .await
        .map_err(|error| {
            MemoryProposalError::SubmitFailed(format!("review list decode: {error}"))
        })?;
    if let Some(invalid) = rows.iter().find(|row| {
        row.workspace_id != workspace_id
            || !row.review_gated
            || !is_canonical_proposal_id(&row.proposal_id)
            || chrono::DateTime::parse_from_rfc3339(&row.created_at).is_err()
    }) {
        return Err(MemoryProposalError::ReviewAckMismatch(format!(
            "actionable projection returned invalid row proposal={} workspace={} status={} review_gated={}",
            invalid.proposal_id, invalid.workspace_id, invalid.status.wire(), invalid.review_gated
        )));
    }
    Ok(rows)
}

/// The minimal typed HTTP client for the proposal submit. Holds ONLY a shared [`reqwest::Client`] (the
/// process-wide [`crate::backend_client::shared_http_client`] pool — NO second HTTP stack, RISK-008-style
/// fork avoidance) + the config-resolved base URL — the same pattern
/// [`crate::fems::memory_client::MemoryClient`] established (MT-063). This is the MT-037
/// `HandshakeCoreClient` HTTP wiring reused for the proposal POST (not a new stack).
#[derive(Clone)]
pub struct HandshakeCoreClient {
    client: reqwest::Client,
    base_url: String,
    session_token: Option<String>,
}

impl Default for HandshakeCoreClient {
    fn default() -> Self {
        Self::production()
    }
}

impl HandshakeCoreClient {
    /// Construct against the production backend base URL (the same config-resolved
    /// [`crate::backend_client::BACKEND_BASE_URL`] every native client uses — not hardcoded here),
    /// sharing the ONE process-wide connection pool.
    pub fn production() -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: crate::backend_client::BACKEND_BASE_URL.to_owned(),
            session_token: None,
        }
    }

    /// Construct against an explicit base URL on a FRESH client (tests point this at a mock server). The
    /// base URL is the authority for the host — never hardcoded at a call site (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_token: None,
        }
    }

    /// Bind every FEMS request to the authenticated native MCP session.
    pub fn with_session_token(mut self, session_token: impl Into<String>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    fn authenticated(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(session_token) = &self.session_token {
            request = request.header(HSK_HEADER_SESSION_TOKEN, session_token);
        }
        request
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Record an operator approval/rejection through the production FEMS review route. Rejection returns
/// the review receipt only. Approval then invokes the separate explicit commit route and returns both
/// immutable receipts; an exact retry converges on those same identities.
pub async fn review_proposal(
    workspace_id: &str,
    proposal_id: &str,
    decision: ProposalReviewDecision,
    client: &HandshakeCoreClient,
) -> Result<ProposalReviewAck, MemoryProposalError> {
    use crate::backend_client::{
        HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID,
        HSK_HEADER_SESSION_RUN_ID,
    };

    let path = proposal_review_path(workspace_id, proposal_id);
    let response = client
        .authenticated(
            client
                .client
                .post(client.url(&path))
                .timeout(SUBMIT_TIMEOUT)
                .header(HSK_HEADER_ACTOR_ID, REVIEW_ACTOR_ID)
                .header(HSK_HEADER_ACTOR_KIND, "operator")
                .header(
                    HSK_HEADER_KERNEL_TASK_RUN_ID,
                    format!("native-editor-fems-review-{workspace_id}"),
                )
                .header(HSK_HEADER_SESSION_RUN_ID, "native-editor-session")
                .json(&json!({
                    "decision": decision.wire(),
                    "reviewer_kind": "user",
                    "reason": format!("Native editor operator {} decision", decision.wire()),
                })),
        )
        .send()
        .await
        .map_err(|error| MemoryProposalError::SubmitFailed(format!("review transport: {error}")))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        let text = response.text().await.unwrap_or_default();
        let canonical_missing = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .is_some_and(|body| {
                body.get("error").and_then(serde_json::Value::as_str) == Some("not_found")
                    && body.get("detail").and_then(serde_json::Value::as_str)
                        == Some("memory proposal in workspace")
            });
        return if canonical_missing {
            Err(MemoryProposalError::ReviewTargetMissing { probed_path: path })
        } else {
            Err(MemoryProposalError::SubmitFailed(format!(
                "review route status 404: {text}"
            )))
        };
    }
    if status == reqwest::StatusCode::CONFLICT {
        let text = response.text().await.unwrap_or_default();
        return Err(MemoryProposalError::ReviewConflict(text));
    }
    if !status.is_success() {
        let code = status.as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(MemoryProposalError::SubmitFailed(format!(
            "review status {code}: {text}"
        )));
    }
    let mut ack = response
        .json::<ProposalReviewAck>()
        .await
        .map_err(|error| {
            MemoryProposalError::SubmitFailed(format!("review ack decode: {error}"))
        })?;
    validate_review_ack(&ack, proposal_id, decision)?;
    if decision == ProposalReviewDecision::Approved {
        ack.commit = Some(commit_proposal(workspace_id, proposal_id, client).await?);
    }
    Ok(ack)
}

async fn commit_proposal(
    workspace_id: &str,
    proposal_id: &str,
    client: &HandshakeCoreClient,
) -> Result<ProposalCommitAck, MemoryProposalError> {
    use crate::backend_client::{
        HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID,
        HSK_HEADER_SESSION_RUN_ID,
    };

    let path = proposal_commit_path(workspace_id, proposal_id);
    let response = client
        .authenticated(
            client
                .client
                .post(client.url(&path))
                .timeout(SUBMIT_TIMEOUT)
                .header(HSK_HEADER_ACTOR_ID, REVIEW_ACTOR_ID)
                .header(HSK_HEADER_ACTOR_KIND, "operator")
                .header(
                    HSK_HEADER_KERNEL_TASK_RUN_ID,
                    format!("native-editor-fems-commit-{workspace_id}"),
                )
                .header(HSK_HEADER_SESSION_RUN_ID, "native-editor-session"),
        )
        .send()
        .await
        .map_err(|error| MemoryProposalError::SubmitFailed(format!("commit transport: {error}")))?;
    let status = response.status();
    if status == reqwest::StatusCode::CONFLICT {
        let text = response.text().await.unwrap_or_default();
        return Err(MemoryProposalError::ReviewConflict(text));
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(MemoryProposalError::ReviewTargetMissing { probed_path: path });
    }
    if !status.is_success() {
        let code = status.as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(MemoryProposalError::SubmitFailed(format!(
            "commit status {code}: {text}"
        )));
    }
    let ack = response
        .json::<ProposalCommitAck>()
        .await
        .map_err(|error| {
            MemoryProposalError::SubmitFailed(format!("commit ack decode: {error}"))
        })?;
    validate_commit_ack(&ack, proposal_id)?;
    Ok(ack)
}

/// Resume the only valid action for a durable approved proposal recovered after interruption.
pub async fn commit_approved_proposal(
    workspace_id: &str,
    proposal_id: &str,
    client: &HandshakeCoreClient,
) -> Result<ProposalCommitAck, MemoryProposalError> {
    commit_proposal(workspace_id, proposal_id, client).await
}

fn validate_commit_ack(
    ack: &ProposalCommitAck,
    proposal_id: &str,
) -> Result<(), MemoryProposalError> {
    let canonical_hash = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    let report = &ack.commit_report;
    let report_hash = compute_memory_commit_report_hash(report)
        .map_err(MemoryProposalError::ReviewAckMismatch)?;
    let canonical = ack.proposal_id == proposal_id
        && ack.status == "committed"
        && uuid::Uuid::parse_str(&ack.commit_id).is_ok()
        && uuid::Uuid::parse_str(&ack.memory_id).is_ok()
        && uuid::Uuid::parse_str(&ack.memory_pack_id).is_ok()
        && canonical_hash(&ack.memory_pack_hash)
        && canonical_hash(&ack.commit_report_hash)
        && ack
            .event_ledger_event_id
            .strip_prefix("KE-")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        && uuid::Uuid::parse_str(&ack.flight_recorder_event_id).is_ok()
        && chrono::DateTime::parse_from_rfc3339(&ack.committed_at).is_ok()
        && report.schema_version == "hsk.memory_commit_report@0.1"
        && report.commit_id == ack.commit_id
        && report.source_proposal_id == ack.proposal_id
        && report.created_at == ack.committed_at
        && report_hash == ack.commit_report_hash
        && report.applied_ops.len() == 1
        && report.applied_ops[0].op == "add"
        && report.applied_ops[0].memory_id == ack.memory_id
        && report.applied_ops[0].previous_version.is_none()
        && report.applied_ops[0].new_version == Some(1)
        && report.applied_ops[0].status == "applied"
        && report.applied_ops[0].reason.is_none()
        && !report.pack_rebuild_hints.is_empty()
        && report.pack_rebuild_hints.iter().all(|hint| {
            hint.reason == "memory_changed"
                && uuid::Uuid::parse_str(&hint.scope_ref.artefact_id).is_ok()
        });
    if !canonical {
        return Err(MemoryProposalError::ReviewAckMismatch(
            "proposal commit acknowledgement is not canonically bound".to_owned(),
        ));
    }
    Ok(())
}

/// Re-hash a dereferenced commit-report artifact with the exact backend NFC canonical JSON contract.
pub fn compute_memory_commit_report_hash(report: &MemoryCommitReport) -> Result<String, String> {
    serde_json::to_value(report)
        .map(|value| sha256_hex(&canonical_json_bytes_nfc(&value)))
        .map_err(|error| error.to_string())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Byte-for-byte port of the backend's NFC canonical JSON used by `MemoryCommitReport::compute_hash`.
fn canonical_json_bytes_nfc(value: &JsonValue) -> Vec<u8> {
    fn write_string(out: &mut String, value: &str) {
        out.push('"');
        for ch in value.nfc() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\u{08}' => out.push_str("\\b"),
                '\u{0c}' => out.push_str("\\f"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", ch as u32)),
                ch if (ch as u32) <= 0x7f => out.push(ch),
                ch if (ch as u32) <= 0xffff => out.push_str(&format!("\\u{:04X}", ch as u32)),
                ch => {
                    let code = (ch as u32) - 0x1_0000;
                    out.push_str(&format!(
                        "\\u{:04X}\\u{:04X}",
                        0xd800 + ((code >> 10) & 0x3ff),
                        0xdc00 + (code & 0x3ff)
                    ));
                }
            }
        }
        out.push('"');
    }
    fn write_value(out: &mut String, value: &JsonValue) {
        match value {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            JsonValue::Number(number) => {
                if let Some(value) = number.as_i64() {
                    out.push_str(&value.to_string());
                } else if let Some(value) = number.as_u64() {
                    out.push_str(&value.to_string());
                } else if let Some(value) = number.as_f64() {
                    out.push_str(&format!("{:.6}", if value == 0.0 { 0.0 } else { value }));
                } else {
                    out.push_str(&number.to_string());
                }
            }
            JsonValue::String(value) => write_string(out, value),
            JsonValue::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_value(out, value);
                }
                out.push(']');
            }
            JsonValue::Object(map) => {
                out.push('{');
                let mut keys = map
                    .keys()
                    .map(|key| (key, key.nfc().collect::<String>()))
                    .collect::<Vec<_>>();
                keys.sort_by(|(a_raw, a_norm), (b_raw, b_norm)| {
                    a_norm.cmp(b_norm).then_with(|| a_raw.cmp(b_raw))
                });
                for (index, (key, _)) in keys.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(out, key);
                    out.push(':');
                    write_value(out, &map[*key]);
                }
                out.push('}');
            }
        }
    }
    let mut out = String::new();
    write_value(&mut out, value);
    out.into_bytes()
}

/// Submit a review-gated proposal to the EXISTING FEMS write path
/// (`POST /workspaces/{workspace_id}/memory/proposals`). Runs OFF the egui frame thread (the host
/// dispatches it on the shared runtime; this fn is `async` and never blocks the frame).
///
/// Behavior contract:
/// - A 404 / route-absent response maps to [`MemoryProposalError::MissingEndpoint`] — the TYPED BLOCKER
///   (RISK-004, MC-004, AC-005). It does NOT commit and does NOT fall back to any direct memory write.
/// - A 2xx body is decoded into a [`ProposalAck`] carrying the backend's already-durable
///   FR-EVT-MEM-001 UUID.
/// - Any other non-success status / transport / decode failure maps to
///   [`MemoryProposalError::SubmitFailed`] (an ordinary failure surfaced to the operator — never a
///   silent commit, never a fallback write).
pub async fn submit_proposal(
    proposal: &MemoryWriteProposal,
    client: &HandshakeCoreClient,
) -> Result<ProposalAck, MemoryProposalError> {
    use crate::backend_client::{
        HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID,
        HSK_HEADER_SESSION_RUN_ID,
    };

    let workspace_id = &proposal.source.workspace_id;
    let path = proposal_path(workspace_id);
    let url = client.url(&path);

    // The typed proposal body (class, content, source provenance, review_gated, actor_id). review_gated
    // is serialized as true (the invariant); the backend's review queue is the authority for the commit.
    let mut body = json!({
        "class": proposal.class.wire(),
        "content": proposal.content,
        "source": proposal.source,
        "review_gated": true,
        "actor_id": proposal.actor_id,
    });
    if let Some(document_content) = proposal.source_document_content.as_ref() {
        body["source_document_content"] = JsonValue::String(document_content.clone());
    }

    let resp = client
        .authenticated(
            client
                .client
                .post(&url)
                .timeout(SUBMIT_TIMEOUT)
                .header(HSK_HEADER_ACTOR_ID, proposal.actor_id.as_str())
                .header(HSK_HEADER_ACTOR_KIND, FEMS_PROPOSE_ACTOR_KIND)
                .header(
                    HSK_HEADER_KERNEL_TASK_RUN_ID,
                    format!("native-editor-fems-propose-{workspace_id}"),
                )
                .header(HSK_HEADER_SESSION_RUN_ID, "native-editor-session")
                .json(&body),
        )
        .send()
        .await
        .map_err(|e| MemoryProposalError::SubmitFailed(format!("transport: {e}")))?;

    let status = resp.status();

    // The TYPED BLOCKER: a 404 means the documented FEMS proposal write route is absent in this build.
    // This is the DESIGNED primary path. Surface it; NEVER commit, NEVER fall back to a direct write.
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(MemoryProposalError::MissingEndpoint { probed_path: path });
    }

    if !status.is_success() {
        let code = status.as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(MemoryProposalError::SubmitFailed(format!(
            "status {code}: {text}"
        )));
    }

    let ack = resp
        .json::<ProposalAck>()
        .await
        .map_err(|e| MemoryProposalError::SubmitFailed(format!("ack decode: {e}")))?;
    validate_proposal_ack(&ack)?;
    Ok(ack)
}

/// Submit a proposal and return the canonical FR-EVT-MEM-001 UUID that handshake_core persisted through
/// its transaction outbox before acknowledging the request. The emitter parameter remains for source
/// compatibility with existing callers, but no duplicate native-editor event is queued.
pub async fn submit_proposal_and_emit(
    proposal: &MemoryWriteProposal,
    client: &HandshakeCoreClient,
    _emitter: &NativeEditorEventEmitter,
) -> Result<ProposalSubmitOutcome, MemoryProposalError> {
    let ack = submit_proposal(proposal, client).await?;
    // handshake_core owns the canonical FR-EVT-MEM-001. Its transactional outbox is committed with
    // the proposal and the API acknowledges only after the projection is durable, eliminating the
    // former frontend-after-ack crash gap and duplicate non-normative native-editor event.
    let event_id = ack.flight_recorder_event_id.clone();
    Ok(ProposalSubmitOutcome::EventPersisted { ack, event_id })
}

/// The two materially different outcomes after the proposal POST succeeds. The proposal may already be
/// durable even when the FR event cannot be queued, so collapsing both states into `ProposalAck` would
/// let the UI claim correlated success that did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalSubmitOutcome {
    /// The proposal and its canonical backend-projected event are both durable.
    EventPersisted { ack: ProposalAck, event_id: String },
    /// The proposal was accepted, but the correlated event was rejected before queueing (workspace
    /// mismatch, backpressure, missing runtime, or a closed worker). This is partial success, not a
    /// correlated-success result.
    EventRejected {
        ack: ProposalAck,
        error: crate::event_emitter::EmitError,
    },
    /// The proposal is durable, but the queued event reached a terminal transport failure.
    EventPersistenceFailed {
        ack: ProposalAck,
        event_id: String,
        error: crate::event_emitter::EmitError,
    },
    /// The proposal is durable and the event was queued, but no final persistence receipt arrived
    /// inside the hard bound. Persistence is unknown and must not be presented as success.
    EventPersistenceTimedOut {
        ack: ProposalAck,
        event_id: String,
        timeout_ms: u64,
    },
}

impl ProposalSubmitOutcome {
    pub fn ack(&self) -> &ProposalAck {
        match self {
            Self::EventPersisted { ack, .. }
            | Self::EventRejected { ack, .. }
            | Self::EventPersistenceFailed { ack, .. }
            | Self::EventPersistenceTimedOut { ack, .. } => ack,
        }
    }

    pub fn event_was_persisted(&self) -> bool {
        matches!(self, Self::EventPersisted { .. })
    }
}

impl std::ops::Deref for ProposalSubmitOutcome {
    type Target = ProposalAck;

    fn deref(&self) -> &Self::Target {
        self.ack()
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// egui command + dialog wiring (AC-006, AC-007).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The "Propose to Memory" command descriptor, registered into the WP-011 command palette catalog.
/// Palette-driven (`keybind: None` — does NOT steal a VS Code binding, RISK-010). The handler opens the
/// confirmation dialog over the current selection; on confirm it dispatches [`submit_proposal_and_emit`]
/// off the frame thread (the live focus→selection→confirm→submit wiring lands at E11/MT-069 like every
/// other pane). This is the catalog row that makes the action SEEABLE + addressable in the palette
/// (HBR-SWARM, AC-006).
pub const PROPOSE_TO_MEMORY_COMMAND: crate::command_registry::AppCommand =
    crate::command_registry::AppCommand {
        id: FEMS_PROPOSE_COMMAND_ID,
        kind: crate::command_registry::CommandKind::App,
        label: FEMS_PROPOSE_COMMAND_LABEL,
        description: "Propose the current selection as a review-gated FEMS memory write (never a direct commit).",
        keywords: &["memory", "fems", "propose", "pillar 12", "review", "episodic", "semantic", "procedural"],
        stable_id: "hs-fems-palette-propose-to-memory",
        disabled: false,
    };

/// The state a [`ProposeToMemoryDialog`] holds while open: the selection it operates on, the currently
/// picked class, and the proposal it previews (rebuilt when the class changes). Confirming dispatches the
/// submit; cancelling discards it. The dialog NEVER commits — it only builds + submits a proposal.
#[derive(Debug, Clone)]
pub struct ProposeToMemoryDialog {
    /// The class the operator picked (default [`MemoryClass::DEFAULT`] = Episodic).
    pub class: MemoryClass,
    /// The proposal previewed for the current class (built from the selection; rebuilt on class change).
    pub proposal: MemoryWriteProposal,
}

impl ProposeToMemoryDialog {
    /// Open the dialog over `selection`, defaulting to [`MemoryClass::DEFAULT`]. Returns
    /// [`MemoryProposalError::NoSelection`] when there is no selection and
    /// [`MemoryProposalError::MissingDocumentIdentity`] for a text range whose host did not supply the
    /// active document through [`Self::open_for_document`]. `workspace_id`/`actor_id` come from live app
    /// state; a fabricated empty proposal or pane-id provenance is never accepted.
    pub fn open(
        selection: &SharedSelection,
        workspace_id: &str,
        actor_id: &str,
    ) -> Result<Self, MemoryProposalError> {
        let class = MemoryClass::DEFAULT;
        let proposal = build_proposal(selection, class, workspace_id, actor_id)?;
        Ok(Self { class, proposal })
    }

    /// Open the live dialog using the authoritative active document id resolved by the shell for the
    /// selection's owning pane. This is the production TextRange path; it never substitutes pane identity
    /// for document provenance.
    pub fn open_for_document(
        selection: &SharedSelection,
        document_id: &str,
        workspace_id: &str,
        actor_id: &str,
    ) -> Result<Self, MemoryProposalError> {
        let class = MemoryClass::DEFAULT;
        if matches!(selection, SharedSelection::TextRange { .. }) && document_id.trim().is_empty() {
            let pane_id = selection
                .pane_id()
                .map(|pane| pane.to_string())
                .unwrap_or_default();
            return Err(MemoryProposalError::MissingDocumentIdentity { pane_id });
        }
        let proposal =
            build_proposal_for_document(selection, class, workspace_id, actor_id, document_id)?;
        Ok(Self { class, proposal })
    }

    /// Open the live dialog for a canonical code source, retaining the complete mounted snapshot only
    /// for the backend's raw-hash and exact-slice provenance validation.
    pub fn open_for_document_snapshot(
        selection: &SharedSelection,
        document_id: &str,
        document_content: String,
        workspace_id: &str,
        actor_id: &str,
    ) -> Result<Self, MemoryProposalError> {
        let class = MemoryClass::DEFAULT;
        if matches!(selection, SharedSelection::TextRange { .. }) && document_id.trim().is_empty() {
            let pane_id = selection
                .pane_id()
                .map(|pane| pane.to_string())
                .unwrap_or_default();
            return Err(MemoryProposalError::MissingDocumentIdentity { pane_id });
        }
        let proposal = build_proposal_for_document_snapshot(
            selection,
            class,
            workspace_id,
            actor_id,
            document_id,
            document_content,
        )?;
        Ok(Self { class, proposal })
    }

    /// Switch the picked class, rebuilding the previewed proposal (so the previewed content_hash +
    /// review_gated reflect the new class). The selection is re-read from the existing proposal's source
    /// (the content + provenance are unchanged by a class switch; only `class` differs).
    pub fn set_class(&mut self, class: MemoryClass) {
        if class == self.class && class == self.proposal.class {
            return;
        }
        self.class = class;
        if self.proposal.class != class {
            // Rebuild keeping the same content + provenance; only the class changes. review_gated stays
            // true. `radio_value` updates `self.class` before this method is called, so the proposal class
            // is the authoritative no-op check here.
            self.proposal = MemoryWriteProposal {
                class,
                ..self.proposal.clone()
            };
        }
    }

    /// The outcome of one [`Self::show`] frame: what the operator did this frame.
    ///
    /// `Confirmed` carries the proposal to submit; the host dispatches [`submit_proposal_and_emit`] off
    /// the frame thread. `Cancelled` closes the dialog with no write. `Pending` keeps it open.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> ProposeDialogOutcome {
        let mut outcome = ProposeDialogOutcome::Pending;

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(FEMS_PROPOSE_COMMAND_LABEL)
                    .color(palette.text)
                    .strong(),
            );
            ui.label(
                egui::RichText::new(
                    "Review-gated proposal — the editor never commits memory directly.",
                )
                .color(palette.text_subtle),
            );

            // Class radios (Episodic default). Each is a Role::RadioButton addressed by the stable
            // fems-class-{class} author_id (AC-007). egui's radio_value derives the interactive role +
            // actions; emit_interactive_node adds the stable author_id without overwriting them. Like the
            // settings-dialog form controls, the radios live in egui's hashed id space (author_id-keyed),
            // not the fixed registry band.
            ui.horizontal(|ui| {
                for class in MemoryClass::ORDER {
                    let author_id = fems_class_author_id(class);
                    // Give each radio its own stable id scope. The previous anonymous auto-id worked in
                    // the isolated widget harness but could be overwritten in the complete mounted app
                    // tree, leaving the control absent from the MCP/AccessKit snapshot. The real egui
                    // radio remains the interactive node; this scope only makes its identity collision-
                    // proof across frames and surrounding work-surface widgets.
                    let resp = ui
                        .push_id(&author_id, |ui| {
                            ui.radio_value(&mut self.class, class, class.label())
                        })
                        .inner;
                    emit_interactive_node(ui.ctx(), resp.id, &author_id);
                    ui.ctx().accesskit_node_builder(resp.id, |node| {
                        node.clear_disabled();
                    });
                    if ui.input(|input| {
                        input
                            .accesskit_action_requests(resp.id, egui::accesskit::Action::Click)
                            .next()
                            .is_some()
                    }) {
                        self.class = class;
                    }
                }
            });
            // Keep the previewed proposal's class in sync with the radio selection.
            if self.proposal.class != self.class {
                self.set_class(self.class);
            }

            // Preview: the selected content + the computed content_hash (short prefix) so the operator
            // sees exactly what will be proposed.
            ui.separator();
            let preview = preview_text(&self.proposal.content);
            ui.label(egui::RichText::new(preview).color(palette.text));
            ui.label(
                egui::RichText::new(format!(
                    "hash {} · {}",
                    short_hash(&self.proposal.source.content_hash),
                    self.class.label()
                ))
                .color(palette.text_subtle),
            );

            ui.separator();
            ui.horizontal(|ui| {
                // The confirm button (Role::Button) addressed by the stable fems-propose-confirm
                // author_id (AC-007); like the radios it lives in egui's hashed id space.
                let confirm = ui.button("Propose");
                emit_interactive_node(ui.ctx(), confirm.id, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
                ui.ctx().accesskit_node_builder(confirm.id, |node| {
                    node.clear_disabled();
                });
                let confirm_accesskit = ui.input(|input| {
                    input
                        .accesskit_action_requests(confirm.id, egui::accesskit::Action::Click)
                        .next()
                        .is_some()
                });
                if confirm.clicked() || confirm_accesskit {
                    outcome = ProposeDialogOutcome::Confirmed(Box::new(self.proposal.clone()));
                }
                let cancel = ui.button("Cancel");
                emit_interactive_node(ui.ctx(), cancel.id, FEMS_PROPOSE_CANCEL_AUTHOR_ID);
                ui.ctx().accesskit_node_builder(cancel.id, |node| {
                    node.clear_disabled();
                });
                let cancel_accesskit = ui.input(|input| {
                    input
                        .accesskit_action_requests(cancel.id, egui::accesskit::Action::Click)
                        .next()
                        .is_some()
                });
                if cancel.clicked() || cancel_accesskit {
                    outcome = ProposeDialogOutcome::Cancelled;
                }
            });
        });

        // The dialog ROOT node (Role::Dialog, modal) — emitted EXACTLY ONCE on its fixed registry NodeId
        // (the command_palette::emit_dialog_node precedent). A swarm agent querying the tree by
        // `fems-propose-dialog` gets exactly ONE match (RISK-010, HBR-SWARM); the previous build emitted a
        // SECOND node carrying the same author_id, which broke deterministic addressing. The fixed NodeId
        // is declared in accessibility::registry::DECLARED_IDENTITIES so the collision/coverage test
        // proves it is globally unique (MC-010).
        let dialog_id = unsafe { egui::Id::from_high_entropy_bits(FEMS_PROPOSE_DIALOG_NODE_ID) };
        ui.ctx().accesskit_node_builder(dialog_id, |node| {
            node.set_role(egui::accesskit::Role::Dialog);
            node.set_author_id(FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
            node.set_label(FEMS_PROPOSE_COMMAND_LABEL.to_owned());
            node.set_modal();
        });

        outcome
    }
}

/// What the operator did in one [`ProposeToMemoryDialog::show`] frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposeDialogOutcome {
    /// The dialog stays open (no decision this frame).
    Pending,
    /// The operator confirmed — submit this proposal (off the frame thread).
    Confirmed(Box<MemoryWriteProposal>),
    /// The operator cancelled — close the dialog with no write.
    Cancelled,
}

/// A bounded preview of the selected content for the dialog (so a huge selection cannot bloat the
/// dialog). First 200 chars, ellipsized.
fn preview_text(content: &str) -> String {
    const MAX: usize = 200;
    if content.chars().count() > MAX {
        let head: String = content.chars().take(MAX).collect();
        format!("{head}…")
    } else {
        content.to_owned()
    }
}

/// The first 8 chars of a content hash for compact display (char-boundary safe).
fn short_hash(hash: &str) -> &str {
    match hash.char_indices().nth(8) {
        Some((idx, _)) => &hash[..idx],
        None => hash,
    }
}

/// The runtime [`crate::interop::CommandDescriptor`] for the "Propose to Memory" command. The shell's
/// palette dispatch arm and shell startup register this on the live MT-031
/// [`crate::interop::InteractionBus`]. The handler captures the workspace-versioned selection and
/// emitter, then requests a repaint so the mounted app drains the request and opens the
/// [`ProposeToMemoryDialog`] next frame (the same stage-then-drain split the MT-033 route-to-stage /
/// MT-032 open-document commands use). It
/// performs NO direct memory write (the only write path is the review-gated proposal POST). Palette-
/// driven: NO keybind (does not steal a VS Code binding — RISK-010).
pub fn propose_to_memory_descriptor() -> crate::interop::CommandDescriptor {
    crate::interop::CommandDescriptor {
        id: FEMS_PROPOSE_COMMAND_ID,
        name: "ProposeToMemory",
        label: FEMS_PROPOSE_COMMAND_LABEL.to_owned(),
        keywords: vec![
            "memory".to_owned(),
            "fems".to_owned(),
            "propose".to_owned(),
            "review".to_owned(),
        ],
        keybind: None,
        handler: Arc::new(
            |ctx: &egui::Context, bus: &mut crate::interop::InteractionBus| {
                bus.request_memory_proposal();
                ctx.request_repaint();
            },
        ),
    }
}

/// Register the "Propose to Memory" command into the WP-011 command registry's runtime command bus,
/// reusing the existing [`crate::interop::InteractionBus`] registration API (NO duplicate registry/bus,
/// RISK-008, MC-008, AC-006). Idempotent (last registration wins). Called during shell frames and from
/// the palette dispatch arm, so MCP/shared-bus callers and the palette use the same live handler.
///
/// This is the WRAP-not-fork registration: the static [`crate::command_registry`] catalog carries the
/// discoverable palette row ([`PROPOSE_TO_MEMORY_COMMAND`]); this registers the runtime handler on the
/// same bus the other melt-together commands use.
pub fn register_propose_to_memory_command(bus: &mut crate::interop::InteractionBus) {
    bus.register_command(propose_to_memory_descriptor());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::{EditorSurfaceKind, SharedSelection};

    fn pane(id: &str) -> crate::pane_registry::PaneId {
        std::sync::Arc::from(id)
    }

    fn text_range(pane_id: &str, start: usize, end: usize, text: &str) -> SharedSelection {
        SharedSelection::TextRange {
            pane_id: pane(pane_id),
            surface: EditorSurfaceKind::RichText,
            start,
            end,
            text: text.to_owned(),
        }
    }

    /// AC-001: build_proposal is pure over the selection and sets the class + FULL provenance from a
    /// TextRange selection.
    #[test]
    fn build_proposal_sets_class_and_full_provenance_from_text_range() {
        let sel = text_range("pane-rich", 10, 22, "hello memory");
        let p =
            build_proposal_for_document(&sel, MemoryClass::Semantic, "WS-1", "actor-7", "DOC-1")
                .expect("builds");
        assert_eq!(p.class, MemoryClass::Semantic);
        assert_eq!(p.content, "hello memory");
        assert_eq!(
            p.source.document_id, "DOC-1",
            "document_id comes from the owning pane's active tab"
        );
        assert_eq!(p.source.pane_id, "pane-rich");
        assert_eq!(p.source.workspace_id, "WS-1");
        assert_eq!(p.source.selection_start, 10);
        assert_eq!(p.source.selection_end, 22);
        assert_eq!(p.actor_id, "actor-7");
        // content_hash is 64-char lowercase hex (the loom primitive).
        assert_eq!(p.source.content_hash.len(), 64);
        assert!(p
            .source
            .content_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        // It is a real, non-zero hash of the content.
        assert_ne!(p.source.content_hash, "0".repeat(64));
    }

    /// AC-003: content_hash REUSES the loom/MT-032 content-hash primitive — the hash of a known string
    /// equals the loom ContentHash (canonical_content_sha256 of the same JSON string value). No second
    /// hashing scheme.
    #[test]
    fn content_hash_reuses_loom_primitive_and_matches_block_hash() {
        let content = "Aria is the protagonist";
        let got = content_hash_of_selection(content);
        // The loom block hash for the SAME content (a JSON string value) — byte-identical.
        let loom = crate::loom_address::ContentHash::of_content_json(&JsonValue::String(
            content.to_owned(),
        ));
        assert_eq!(
            got,
            loom.as_str(),
            "AC-003: proposal hash == loom block hash for identical content"
        );
        // And it equals the raw canonical primitive (no second scheme).
        assert_eq!(
            got,
            canonical_content_sha256(&JsonValue::String(content.to_owned()))
        );
        // Deterministic.
        assert_eq!(got, content_hash_of_selection(content));
    }

    /// AC-002: a Procedural-class proposal has review_gated==true, and review_gated is true for EVERY
    /// class — the editor can never produce a non-review-gated proposal.
    #[test]
    fn review_gated_is_always_true_hard_true_for_procedural() {
        let sel = text_range("pane-code", 0, 4, "step");
        for class in MemoryClass::ORDER {
            let p = build_proposal_for_document(&sel, class, "WS-1", "a", "DOC-1").expect("builds");
            assert!(
                p.is_review_gated(),
                "{:?} proposal must be review_gated",
                class
            );
            assert!(p.is_review_gated());
        }
        // Procedural explicitly (the spec requirement).
        let proc = build_proposal_for_document(&sel, MemoryClass::Procedural, "WS-1", "a", "DOC-1")
            .unwrap();
        assert!(
            proc.is_review_gated(),
            "AC-002: Procedural-class proposal is review-gated"
        );
        // There is no field/setter that flips it false: a class switch keeps it true.
        let mut dlg = ProposeToMemoryDialog::open_for_document(&sel, "DOC-1", "WS-1", "a").unwrap();
        dlg.set_class(MemoryClass::Procedural);
        assert!(
            dlg.proposal.is_review_gated(),
            "class switch never sets review_gated false"
        );
    }

    /// AC-001 (other variants): BlockRef / NodeRef resolve document_id from the block/node id with a
    /// whole-content range.
    #[test]
    fn build_proposal_handles_block_and_node_refs() {
        let block = SharedSelection::BlockRef {
            pane_id: pane("pane-rich"),
            block_id: "blk-7".to_owned(),
        };
        let p = build_proposal(&block, MemoryClass::Episodic, "WS-1", "a").unwrap();
        assert_eq!(p.source.document_id, "blk-7");
        assert_eq!(p.content, "loom://blk-7");
        assert_eq!(p.source.selection_start, 0);
        assert_eq!(p.source.selection_end, "loom://blk-7".len());

        let node = SharedSelection::NodeRef {
            pane_id: pane("pane-canvas"),
            surface: EditorSurfaceKind::Canvas,
            node_id: "node-9".to_owned(),
        };
        let pn = build_proposal(&node, MemoryClass::Episodic, "WS-1", "a").unwrap();
        assert_eq!(pn.source.document_id, "node-9");
        assert_eq!(pn.content, "loom://node-9");
    }

    /// AC-001 / NoSelection: build_proposal over SharedSelection::None returns NoSelection (no fabricated
    /// empty proposal).
    #[test]
    fn build_proposal_none_selection_is_no_selection() {
        let err =
            build_proposal(&SharedSelection::None, MemoryClass::Episodic, "WS-1", "a").unwrap_err();
        assert_eq!(err, MemoryProposalError::NoSelection);
        assert!(!err.is_missing_endpoint());
    }

    #[test]
    fn build_proposal_rejects_empty_and_mismatched_utf8_byte_ranges() {
        let empty = text_range("pane-rich", 7, 7, "");
        assert_eq!(
            build_proposal_for_document(&empty, MemoryClass::Semantic, "WS-1", "actor", "DOC-1")
                .unwrap_err(),
            MemoryProposalError::EmptySelection
        );

        let unicode = "é🙂"; // 6 UTF-8 bytes, 2 Unicode scalar values.
        let wrong_scalar_range = text_range("pane-rich", 10, 12, unicode);
        assert_eq!(
            build_proposal_for_document(
                &wrong_scalar_range,
                MemoryClass::Semantic,
                "WS-1",
                "actor",
                "DOC-1"
            )
            .unwrap_err(),
            MemoryProposalError::SelectionRangeMismatch {
                start: 10,
                end: 12,
                content_bytes: 6,
            }
        );

        let byte_range = text_range("pane-rich", 10, 16, unicode);
        let proposal = build_proposal_for_document(
            &byte_range,
            MemoryClass::Semantic,
            "WS-1",
            "actor",
            "DOC-1",
        )
        .expect("UTF-8 byte offsets are authoritative");
        assert_eq!(
            proposal.source.selection_end - proposal.source.selection_start,
            6
        );
    }

    #[test]
    fn text_range_without_authoritative_document_identity_fails_closed() {
        let sel = text_range("pane-rich", 0, 6, "memory");
        assert_eq!(
            build_proposal(&sel, MemoryClass::Episodic, "WS-1", "a").unwrap_err(),
            MemoryProposalError::MissingDocumentIdentity {
                pane_id: "pane-rich".to_owned()
            }
        );
    }

    /// AC-008: the FR payload carries action='memory_write_proposed' + proposal_id + class + document_id
    /// + pending-review status + selection range + content_hash + review_gated + pane_id.
    #[test]
    fn fr_payload_carries_full_marker_and_provenance() {
        let sel = text_range("pane-rich", 3, 9, "memory");
        let p = build_proposal_for_document(&sel, MemoryClass::Procedural, "WS-1", "a", "DOC-1")
            .unwrap();
        let payload = p.fr_payload("PROP-42");
        assert_eq!(payload["action"], "memory_write_proposed");
        assert_eq!(payload["proposal_id"], "PROP-42");
        assert_eq!(payload["status"], "pending_review");
        assert_eq!(payload["class"], "procedural");
        assert_eq!(payload["document_id"], "DOC-1");
        assert_eq!(payload["selection_start"], 3);
        assert_eq!(payload["selection_end"], 9);
        assert_eq!(payload["content_hash"], p.source.content_hash);
        assert_eq!(payload["review_gated"], true);
        assert_eq!(payload["pane_id"], "pane-rich");
    }

    /// AC-008: the FR event reuses the MT-036 emitter schema with action MemoryWriteProposed and the
    /// native-editor schema version (no new emitter, no new schema).
    #[test]
    fn fr_event_uses_mt036_schema_and_action() {
        use crate::event_emitter::{NativeEditorAction, NATIVE_EDITOR_SCHEMA_VERSION};
        let sel = text_range("pane-rich", 0, 6, "memory");
        let p =
            build_proposal_for_document(&sel, MemoryClass::Episodic, "WS-9", "a", "DOC-9").unwrap();
        let proposal_id = format!("PROP-{}", "1a".repeat(32));
        let ev = p.fr_event(&proposal_id);
        let replay = p.fr_event(&proposal_id);
        assert_eq!(ev.action, NativeEditorAction::MemoryWriteProposed);
        assert_eq!(ev.action.as_str(), "memory_write_proposed");
        assert_eq!(ev.schema_version, NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(ev.workspace_id, "WS-9");
        assert_eq!(ev.pane_id, "pane-rich");
        assert_eq!(
            ev.actor_id, p.actor_id,
            "the persisted proposal and FR event must share one canonical actor identity"
        );
        assert_eq!(
            replay.event_id, ev.event_id,
            "a proposal replay must address the same correlated Flight Recorder event"
        );
        assert_ne!(
            uuid::Uuid::parse_str(&ev.event_id).unwrap(),
            uuid::Uuid::nil()
        );
        // The native payload nests under the MT-036 schema (no invented top-level event_type).
        let np = ev.to_native_payload();
        assert_eq!(np["action"], "memory_write_proposed");
        assert_eq!(np["payload"]["proposal_id"], proposal_id);
    }

    /// The command descriptor is the WP-011 palette catalog row for 'fems.propose_to_memory', enabled
    /// and palette-driven (no keybind, RISK-010). Asserted against the RUNTIME catalog
    /// ([`crate::command_registry::all_commands`]) — the actual list the palette + dispatcher read — not
    /// against the const's own fields (a const-on-assert is optimized out, clippy::assertions_on_constants
    /// under `-D warnings`); this also proves the row really lives in the shared catalog.
    #[test]
    fn propose_command_descriptor_is_palette_driven() {
        let row = crate::command_registry::all_commands()
            .iter()
            .find(|c| c.id == FEMS_PROPOSE_COMMAND_ID)
            .expect("AC-006: 'fems.propose_to_memory' is registered in the shared palette catalog");
        assert!(
            !row.disabled,
            "AC-006: the Propose-to-Memory palette row is enabled (runnable)"
        );
        assert_eq!(row.label, FEMS_PROPOSE_COMMAND_LABEL);
        assert_eq!(row.kind, crate::command_registry::CommandKind::App);
    }

    #[test]
    fn shared_bus_command_captures_workspace_versioned_selection() {
        let ctx = egui::Context::default();
        let mut bus = crate::interop::InteractionBus::new();
        assert!(bus.bind_workspace("workspace-command"));
        let selection = SharedSelection::BlockRef {
            pane_id: pane("pane-command"),
            block_id: "block-command".to_owned(),
        };
        assert!(bus.set_selection(selection.clone()));
        register_propose_to_memory_command(&mut bus);
        assert!(bus.dispatch_command(&ctx, FEMS_PROPOSE_COMMAND_ID));
        let request = bus
            .take_pending_memory_proposal_request()
            .expect("shared command stages one proposal-open request");
        assert_eq!(request.workspace_id, "workspace-command");
        assert_eq!(request.workspace_generation, bus.workspace_generation());
        assert_eq!(request.selection, selection);
    }

    /// The class radio author ids follow the fems-class-{class} convention.
    #[test]
    fn class_author_ids_follow_convention() {
        assert_eq!(
            fems_class_author_id(MemoryClass::Episodic),
            "fems-class-episodic"
        );
        assert_eq!(
            fems_class_author_id(MemoryClass::Semantic),
            "fems-class-semantic"
        );
        assert_eq!(
            fems_class_author_id(MemoryClass::Procedural),
            "fems-class-procedural"
        );
    }

    /// The body serialized for the submit carries the typed proposal (class/content/source/review_gated/
    /// actor_id) and nothing else — and review_gated is always true.
    #[test]
    fn submit_body_shape_is_review_gated_proposal() {
        let sel = text_range("pane-rich", 0, 6, "memory");
        let p =
            build_proposal_for_document(&sel, MemoryClass::Semantic, "WS-1", "actor-1", "DOC-1")
                .unwrap();
        let body = json!({
            "class": p.class.wire(),
            "content": p.content,
            "source": p.source,
            "review_gated": p.is_review_gated(),
            "actor_id": p.actor_id,
        });
        assert_eq!(body["class"], "semantic");
        assert_eq!(body["content"], "memory");
        assert_eq!(body["review_gated"], true);
        assert_eq!(body["source"]["document_id"], "DOC-1");
        assert_eq!(body["source"]["content_hash"], p.source.content_hash);
        assert_eq!(body["actor_id"], "actor-1");
    }

    fn valid_review_ack() -> ProposalReviewAck {
        ProposalReviewAck {
            proposal_id: "proposal-1".to_owned(),
            status: "approved".to_owned(),
            decision: ProposalReviewDecision::Approved,
            reviewer_kind: "user".to_owned(),
            actor_id: REVIEW_ACTOR_ID.to_owned(),
            correlation_id: "fems-memory-proposal-review:proposal-1".to_owned(),
            event_ledger_event_id: "KE-550e8400-e29b-41d4-a716-446655440000".to_owned(),
            flight_recorder_event_id: "550e8400-e29b-41d4-a716-446655440001".to_owned(),
            reviewed_at: "2026-07-17T00:00:00Z".to_owned(),
            commit: None,
        }
    }

    #[test]
    fn review_ack_must_match_requested_identity_and_durable_receipts() {
        let ack = valid_review_ack();
        assert_eq!(
            validate_review_ack(&ack, "proposal-1", ProposalReviewDecision::Approved),
            Ok(())
        );

        let mutations: Vec<Box<dyn Fn(&mut ProposalReviewAck)>> = vec![
            Box::new(|ack| ack.proposal_id = "other".to_owned()),
            Box::new(|ack| ack.decision = ProposalReviewDecision::Rejected),
            Box::new(|ack| ack.status = "pending_review".to_owned()),
            Box::new(|ack| ack.reviewer_kind = "policy".to_owned()),
            Box::new(|ack| ack.actor_id = "other-actor".to_owned()),
            Box::new(|ack| ack.correlation_id = "wrong".to_owned()),
            Box::new(|ack| ack.event_ledger_event_id.clear()),
            Box::new(|ack| ack.flight_recorder_event_id.clear()),
            Box::new(|ack| ack.reviewed_at.clear()),
        ];
        for mutate in mutations {
            let mut mismatched = valid_review_ack();
            mutate(&mut mismatched);
            assert!(matches!(
                validate_review_ack(&mismatched, "proposal-1", ProposalReviewDecision::Approved),
                Err(MemoryProposalError::ReviewAckMismatch(_))
            ));
        }
    }

    #[test]
    fn exact_approved_review_retry_accepts_committed_lifecycle_only_for_approval() {
        let mut committed = valid_review_ack();
        committed.status = "committed".to_owned();
        assert_eq!(
            validate_review_ack(&committed, "proposal-1", ProposalReviewDecision::Approved),
            Ok(())
        );

        committed.decision = ProposalReviewDecision::Rejected;
        assert!(matches!(
            validate_review_ack(&committed, "proposal-1", ProposalReviewDecision::Rejected),
            Err(MemoryProposalError::ReviewAckMismatch(_))
        ));
    }

    fn valid_commit_ack() -> ProposalCommitAck {
        let proposal_id = format!("PROP-{}", "a".repeat(64));
        let commit_id = "550e8400-e29b-41d4-a716-446655440010".to_owned();
        let memory_id = "550e8400-e29b-41d4-a716-446655440011".to_owned();
        let committed_at = "2026-07-17T00:00:00+00:00".to_owned();
        let report = MemoryCommitReport {
            schema_version: "hsk.memory_commit_report@0.1".to_owned(),
            commit_id: commit_id.clone(),
            created_at: committed_at.clone(),
            source_proposal_id: proposal_id.clone(),
            applied_ops: vec![MemoryCommitAppliedOp {
                op: "add".to_owned(),
                memory_id: memory_id.clone(),
                previous_version: None,
                new_version: Some(1),
                status: "applied".to_owned(),
                reason: None,
            }],
            warnings: Vec::new(),
            pack_rebuild_hints: vec![MemoryPackRebuildHint {
                scope_ref: MemoryCommitScopeRef {
                    artefact_type: "workspace".to_owned(),
                    artefact_id: "550e8400-e29b-41d4-a716-446655440012".to_owned(),
                    selector: "workspace".to_owned(),
                },
                reason: "memory_changed".to_owned(),
            }],
        };
        let report_hash = sha256_hex(&canonical_json_bytes_nfc(
            &serde_json::to_value(&report).unwrap(),
        ));
        ProposalCommitAck {
            proposal_id,
            status: "committed".to_owned(),
            commit_id,
            memory_id,
            memory_pack_id: "550e8400-e29b-41d4-a716-446655440013".to_owned(),
            memory_pack_hash: "b".repeat(64),
            commit_report: report,
            commit_report_hash: report_hash,
            event_ledger_event_id: "KE-550e8400-e29b-41d4-a716-446655440014".to_owned(),
            flight_recorder_event_id: "550e8400-e29b-41d4-a716-446655440015".to_owned(),
            committed_at,
        }
    }

    fn rehash_commit_report(ack: &mut ProposalCommitAck) {
        ack.commit_report_hash = sha256_hex(&canonical_json_bytes_nfc(
            &serde_json::to_value(&ack.commit_report).unwrap(),
        ));
    }

    #[test]
    fn commit_ack_recomputes_report_hash_and_binds_all_receipt_identities() {
        let canonical = valid_commit_ack();
        assert_eq!(
            validate_commit_ack(&canonical, &canonical.proposal_id),
            Ok(())
        );

        let mut wrong_hash = canonical.clone();
        wrong_hash.commit_report_hash = "c".repeat(64);
        assert!(validate_commit_ack(&wrong_hash, &canonical.proposal_id).is_err());

        let mut wrong_commit = canonical.clone();
        wrong_commit.commit_report.commit_id = "550e8400-e29b-41d4-a716-446655440099".to_owned();
        rehash_commit_report(&mut wrong_commit);
        assert!(validate_commit_ack(&wrong_commit, &canonical.proposal_id).is_err());

        let mut wrong_proposal = canonical.clone();
        wrong_proposal.commit_report.source_proposal_id = format!("PROP-{}", "d".repeat(64));
        rehash_commit_report(&mut wrong_proposal);
        assert!(validate_commit_ack(&wrong_proposal, &canonical.proposal_id).is_err());

        let mut wrong_memory = canonical.clone();
        wrong_memory.commit_report.applied_ops[0].memory_id =
            "550e8400-e29b-41d4-a716-446655440098".to_owned();
        rehash_commit_report(&mut wrong_memory);
        assert!(validate_commit_ack(&wrong_memory, &canonical.proposal_id).is_err());

        let mut uppercase_hash = canonical.clone();
        uppercase_hash.memory_pack_hash.make_ascii_uppercase();
        assert!(validate_commit_ack(&uppercase_hash, &canonical.proposal_id).is_err());
    }

    #[test]
    fn proposal_ack_requires_canonical_identity_and_lifecycle_status() {
        let canonical = ProposalAck {
            proposal_id: format!("PROP-{}", "a".repeat(64)),
            status: "pending_review".to_owned(),
            created_at: "2026-07-17T00:00:00Z".to_owned(),
            flight_recorder_event_id: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        };
        assert_eq!(validate_proposal_ack(&canonical), Ok(()));
        for ack in [
            ProposalAck {
                proposal_id: "PROP-short".to_owned(),
                status: "pending_review".to_owned(),
                created_at: canonical.created_at.clone(),
                flight_recorder_event_id: canonical.flight_recorder_event_id.clone(),
            },
            ProposalAck {
                proposal_id: format!("PROP-{}", "A".repeat(64)),
                status: "pending_review".to_owned(),
                created_at: canonical.created_at.clone(),
                flight_recorder_event_id: canonical.flight_recorder_event_id.clone(),
            },
            ProposalAck {
                proposal_id: format!("PROP-{}", "b".repeat(64)),
                status: "not-a-lifecycle-state".to_owned(),
                created_at: canonical.created_at.clone(),
                flight_recorder_event_id: canonical.flight_recorder_event_id.clone(),
            },
            ProposalAck {
                proposal_id: format!("PROP-{}", "b".repeat(64)),
                status: "pending_review".to_owned(),
                created_at: "not-a-timestamp".to_owned(),
                flight_recorder_event_id: canonical.flight_recorder_event_id.clone(),
            },
        ] {
            assert!(matches!(
                validate_proposal_ack(&ack),
                Err(MemoryProposalError::ReviewAckMismatch(_))
            ));
        }
    }
}
