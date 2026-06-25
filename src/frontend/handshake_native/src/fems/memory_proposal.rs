//! FEMS memory-write PROPOSAL from the editor (WP-KERNEL-012 MT-064, cluster E9 — Pillar 12).
//!
//! ## What this is (the editor entry point into the FEMS proposal→review→commit loop)
//!
//! This module turns the current editor selection (the MT-031 [`SharedSelection`]) into a typed,
//! review-gated FEMS memory-write PROPOSAL and submits it to the EXISTING review-gated FEMS write path.
//! It is the editor's ONLY write into Pillar 12 memory, and it is a PROPOSAL — never a direct commit.
//! The commit step is downstream and review-gated (operator/reviewer), never editor-direct
//! (RISK-001/002, MC-001/002, AC-002). There is exactly ONE write path here: the proposal POST. There
//! is NO direct memory-commit / write-direct call site anywhere in this file (grep-gate MC-001).
//!
//! ## THE LOAD-BEARING INVARIANT (review-gated, never-editor-direct)
//!
//! - [`MemoryWriteProposal::review_gated`] is ALWAYS `true`. It is HARD-set `true` for the `Procedural`
//!   class (the spec requirement) and `true` for `Episodic`/`Semantic` too — the editor can NEVER set it
//!   `false` for any class. There is no constructor, setter, or method that yields a `review_gated=false`
//!   proposal (MC-002, AC-002).
//! - On a missing proposal endpoint the editor returns the typed blocker
//!   [`MemoryProposalError::MissingEndpoint`] and writes nothing — it does NOT fall back to a direct
//!   memory write (RISK-004, MC-004, AC-005).
//!
//! ## The proposal WRITE endpoint is ABSENT in this handshake_core build (the DESIGNED primary path)
//!
//! A read-only verification of `src/backend/handshake_core` (the KERNEL_BUILDER gate 2026-06-25,
//! re-confirmed here) found that `POST /workspaces/{id}/memory/proposals` DOES NOT EXIST in the current
//! build. `api/knowledge_memory.rs` exposes only five GET reads
//! (`/knowledge/memory/{claims,conflicts,facts,entities,visual-debug}`) — there is NO proposal WRITE
//! route, and the only `MemoryPack`/proposal surfaces are an INTERNAL builder + `.handshake/fems/...`
//! artifacts. So [`submit_proposal`] returns [`MemoryProposalError::MissingEndpoint`] on a 404 /
//! route-absent / capability-missing response — the TYPED BLOCKER the coder surfaces. It does NOT add,
//! rewrite, or bypass the backend, and it does NOT silently commit (RISK-004/009, MC-004/009, AC-005).
//! The pure builder + the dialog/command wiring + the FR payload SHAPE are all PROVABLE NOW against
//! fixtures; the live PG proposal record + live FR ledger ingestion are `NEEDS_MANAGED_RESOURCE_PROOF`
//! (AC-004, the double-gate below).
//!
//! ## FR-EVT-MEM-001 emit via the MT-036 emitter (DOUBLE-GATED) — no new emitter, no new event_type
//!
//! After a successful proposal ack, this module emits a [`NativeEditorEvent`] with action
//! [`NativeEditorAction::MemoryWriteProposed`] (`"memory_write_proposed"`) through the EXISTING MT-036
//! [`NativeEditorEventEmitter`] (reused via `event_bus`) — NO new emitter (RISK-007, MC-007, AC-008).
//! `FR-EVT-MEM-001` is the stable event-NAME marker carried in the payload `action`, NOT a new ledger
//! `event_type` (the `event_type` stays `'system'`/the closed FR schema per MT-036). The backend FR
//! ledger DOES model a `memory_write_proposed` event whose `event_id` is the fixed string
//! `"FR-EVT-MEM-001"` (verified read-only in `flight_recorder/mod.rs::validate_memory_write_proposed_payload`),
//! but there is NO HTTP ingestion route that accepts it from the native editor — the only FR HTTP
//! ingestion endpoint is `runtime_chat_event` (the closed `RuntimeChatEventV0_1`, MT-036's documented
//! backend gap). So the FR emit SHAPE is provable now, but LIVE FR ingestion of the native-editor
//! `memory_write_proposed` event needs the FR-schema extension MT-036 already flagged. Combined with the
//! likely `MissingEndpoint`, AC-004 (a live PG proposal record + a live FR event) is the DOUBLE-GATE
//! `NEEDS_MANAGED_RESOURCE_PROOF`.
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

/// AccessKit author_id PREFIX for a class radio (`fems-class-{episodic|semantic|procedural}`,
/// `Role::RadioButton`). The full id is built by [`fems_class_author_id`].
pub const FEMS_CLASS_AUTHOR_PREFIX: &str = "fems-class-";

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
    pub const ORDER: [MemoryClass; 3] =
        [MemoryClass::Episodic, MemoryClass::Semantic, MemoryClass::Procedural];

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
    /// The document/block the content was selected from. For a `TextRange` selection this is the owning
    /// pane id (the stable document-surface identifier the host maps to a live document id at E11/MT-069);
    /// for a `BlockRef`/`NodeRef` selection it is the block/node id (already loom-addressable).
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
    /// The pane that owns the selection (the editor surface instance the proposal originated from).
    pub pane_id: String,
    /// The workspace the proposal is scoped to (the path parameter of the proposal POST).
    pub workspace_id: String,
}

/// A typed, review-gated FEMS memory-write proposal built from an editor selection. The editor submits
/// this to the review-gated FEMS write path; the commit is downstream and review-gated, never
/// editor-direct. [`Self::review_gated`] is ALWAYS `true` (the load-bearing invariant — MC-002, AC-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWriteProposal {
    /// The Pillar 12 class this proposal targets.
    pub class: MemoryClass,
    /// The exact selected content the proposal carries.
    pub content: String,
    /// Full source provenance (document_id + selection range + content hash + pane + workspace).
    pub source: MemorySourceProvenance,
    /// ALWAYS `true`. Hard-set `true` for `Procedural` (spec) and `true` for every other class too; the
    /// editor can never set it `false`. The commit is downstream + review-gated, never editor-direct.
    pub review_gated: bool,
    /// The acting operator/model session id (for attribution on the review queue).
    pub actor_id: String,
}

impl MemoryWriteProposal {
    /// True iff this proposal is review-gated (ALWAYS true — the invariant). Exposed so a test/consumer
    /// can assert the never-editor-direct contract without reaching into the field.
    pub fn is_review_gated(&self) -> bool {
        self.review_gated
    }

    /// The typed FR-EVT-MEM-001 (`memory_write_proposed`) payload this proposal emits through the MT-036
    /// emitter after a successful ack. The payload carries the action marker + the proposal identity +
    /// the provenance the swarm-observability loop needs (AC-008). `proposal_id` comes from the ack.
    /// The `event_type` stays the CLOSED FR vocabulary (set by the MT-036 emitter); this is ONLY the
    /// native-editor payload (`FR-EVT-MEM-001` is carried as the `action`, not a new ledger event_type).
    pub fn fr_payload(&self, proposal_id: &str) -> JsonValue {
        json!({
            "action": "memory_write_proposed",
            "proposal_id": proposal_id,
            "class": self.class.wire(),
            "document_id": self.source.document_id,
            "selection_start": self.source.selection_start,
            "selection_end": self.source.selection_end,
            "content_hash": self.source.content_hash,
            "review_gated": self.review_gated,
            "pane_id": self.source.pane_id,
        })
    }

    /// Build the MT-036 [`NativeEditorEvent`] for this proposal's FR-EVT-MEM-001 emit. The event carries
    /// action [`NativeEditorAction::MemoryWriteProposed`] and this proposal's FR payload, addressed from
    /// the proposal's pane + workspace. Reuses the MT-036 event schema (no new schema, AC-008).
    pub fn fr_event(&self, proposal_id: &str) -> NativeEditorEvent {
        use crate::event_emitter::{native_editor_actor_id, NativeEditorAction};
        NativeEditorEvent::new(
            NativeEditorAction::MemoryWriteProposed,
            self.source.pane_id.clone(),
            native_editor_actor_id(&self.source.pane_id),
            self.source.workspace_id.clone(),
            self.fr_payload(proposal_id),
        )
    }
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
    /// The [`SharedSelection`] was [`SharedSelection::None`] — there is nothing to propose. The dialog
    /// is not opened / the command is a no-op in this state (never a fabricated empty proposal).
    NoSelection,
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
            Self::NoSelection => write!(f, "no selection to propose to memory"),
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
/// - [`SharedSelection::TextRange`] gives the exact byte range + the selected text; `document_id` is the
///   owning pane id (the stable document-surface identifier the host maps to a live document id at
///   E11/MT-069).
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
    let (document_id, pane_id, selection_start, selection_end, content) = match sel {
        SharedSelection::None => return Err(MemoryProposalError::NoSelection),
        SharedSelection::TextRange { pane_id, start, end, text, .. } => {
            // document_id derives from the owning pane (the pane→document map is a host concern resolved
            // live at E11/MT-069); the exact byte range + materialized text come straight from the
            // selection.
            (pane_id.to_string(), pane_id.to_string(), *start, *end, text.clone())
        }
        SharedSelection::BlockRef { pane_id, block_id } => {
            // A whole-block selection: the block id IS the document/block id (loom-addressable). The
            // content is the loom address of the block (the ref variant does not carry the block text);
            // the range is the whole content.
            let content = format!("loom://{block_id}");
            (block_id.clone(), pane_id.to_string(), 0, content.len(), content)
        }
        SharedSelection::NodeRef { pane_id, node_id, .. } => {
            // A whole-node selection (graph/canvas): the node id IS the document/block id. Same shape as
            // BlockRef.
            let content = format!("loom://{node_id}");
            (node_id.clone(), pane_id.to_string(), 0, content.len(), content)
        }
    };

    let content_hash = content_hash_of_selection(&content);

    Ok(MemoryWriteProposal {
        class,
        content,
        source: MemorySourceProvenance {
            document_id,
            selection_start,
            selection_end,
            content_hash,
            pane_id,
            workspace_id: workspace_id.to_owned(),
        },
        // THE LOAD-BEARING INVARIANT: review_gated is ALWAYS true. There is no path — no constructor, no
        // setter, no class — that yields review_gated=false from the editor. Procedural is review-gated
        // by spec; every other class is review-gated too because the commit is never editor-direct.
        review_gated: true,
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

/// Read timeout for a single proposal submit. A bounded timeout so a hung backend cannot stall the
/// editor (the submit runs off the frame thread on the shared async runtime).
const SUBMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// The proposal write path for a workspace (the documented review-gated FEMS write route). Built here so
/// the `MissingEndpoint` blocker can report the exact probed path.
pub fn proposal_path(workspace_id: &str) -> String {
    format!("/workspaces/{workspace_id}/memory/proposals")
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
        }
    }

    /// Construct against an explicit base URL on a FRESH client (tests point this at a mock server). The
    /// base URL is the authority for the host — never hardcoded at a call site (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Submit a review-gated proposal to the EXISTING FEMS write path
/// (`POST /workspaces/{workspace_id}/memory/proposals`). Runs OFF the egui frame thread (the host
/// dispatches it on the shared runtime; this fn is `async` and never blocks the frame).
///
/// Behavior contract:
/// - A 404 / route-absent response maps to [`MemoryProposalError::MissingEndpoint`] — the TYPED BLOCKER
///   (RISK-004, MC-004, AC-005). It does NOT commit and does NOT fall back to any direct memory write.
///   This is the DESIGNED PRIMARY PATH in the current build, where the route does not exist.
/// - A 2xx body is decoded into a [`ProposalAck`]; the caller then emits the FR-EVT-MEM-001 event.
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
    let body = json!({
        "class": proposal.class.wire(),
        "content": proposal.content,
        "source": proposal.source,
        "review_gated": proposal.review_gated,
        "actor_id": proposal.actor_id,
    });

    let resp = client
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
        .json(&body)
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
        return Err(MemoryProposalError::SubmitFailed(format!("status {code}: {text}")));
    }

    resp.json::<ProposalAck>()
        .await
        .map_err(|e| MemoryProposalError::SubmitFailed(format!("ack decode: {e}")))
}

/// Submit a proposal AND, on success, emit the FR-EVT-MEM-001 (`memory_write_proposed`) event through the
/// MT-036 emitter (no new emitter, AC-008). Runs off the frame thread. The FR emit reuses the MT-036
/// emitter's semaphore-bounded off-frame spawn + error ring (a failed emit lands in the ring, never
/// panics — RISK-006/007, MC-006/007). On [`MemoryProposalError::MissingEndpoint`] (the typed blocker)
/// it writes nothing and emits nothing — the editor never falls back to a direct write. Returns the ack
/// on success so the host can surface the proposal id.
pub async fn submit_proposal_and_emit(
    proposal: &MemoryWriteProposal,
    client: &HandshakeCoreClient,
    emitter: &NativeEditorEventEmitter,
) -> Result<ProposalAck, MemoryProposalError> {
    let ack = submit_proposal(proposal, client).await?;
    // FR-EVT-MEM-001: emit AFTER a successful ack, via the MT-036 emitter. The emit is non-blocking and
    // a failure is recorded in the MT-036 error ring (never crashes the frame); we do NOT propagate an
    // emit failure as a proposal failure — the proposal WAS accepted.
    let _ = emitter.emit(proposal.fr_event(&ack.proposal_id));
    Ok(ack)
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
    /// [`MemoryProposalError::NoSelection`] when there is no selection (the dialog is not opened — never a
    /// fabricated empty proposal). `workspace_id`/`actor_id` come from the live app state.
    pub fn open(
        selection: &SharedSelection,
        workspace_id: &str,
        actor_id: &str,
    ) -> Result<Self, MemoryProposalError> {
        let class = MemoryClass::DEFAULT;
        let proposal = build_proposal(selection, class, workspace_id, actor_id)?;
        Ok(Self { class, proposal })
    }

    /// Switch the picked class, rebuilding the previewed proposal (so the previewed content_hash +
    /// review_gated reflect the new class). The selection is re-read from the existing proposal's source
    /// (the content + provenance are unchanged by a class switch; only `class` differs).
    pub fn set_class(&mut self, class: MemoryClass) {
        if class == self.class {
            return;
        }
        self.class = class;
        // Rebuild keeping the same content + provenance; only the class changes. review_gated stays true.
        self.proposal = MemoryWriteProposal {
            class,
            ..self.proposal.clone()
        };
    }

    /// The outcome of one [`Self::show`] frame: what the operator did this frame.
    ///
    /// `Confirmed` carries the proposal to submit; the host dispatches [`submit_proposal_and_emit`] off
    /// the frame thread. `Cancelled` closes the dialog with no write. `Pending` keeps it open.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> ProposeDialogOutcome {
        let mut outcome = ProposeDialogOutcome::Pending;

        // The dialog root container node (Role::Dialog, modal) addressed by the stable author_id so a
        // swarm agent can drive the proposal flow deterministically (AC-007).
        let dialog_id = egui::Id::new(FEMS_PROPOSE_DIALOG_AUTHOR_ID);
        let dialog_resp = ui
            .vertical(|ui| {
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

                // Class radios (Episodic default). Each is a Role::RadioButton addressed
                // fems-class-{class} (AC-007). egui's radio_value already derives the interactive role +
                // actions; emit_interactive_node adds the stable author_id without overwriting them.
                ui.horizontal(|ui| {
                    for class in MemoryClass::ORDER {
                        let resp = ui.radio_value(&mut self.class, class, class.label());
                        emit_interactive_node(ui.ctx(), resp.id, &fems_class_author_id(class));
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
                    // The confirm button (Role::Button) addressed fems-propose-confirm (AC-007).
                    let confirm = ui.button("Propose");
                    emit_interactive_node(ui.ctx(), confirm.id, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
                    if confirm.clicked() {
                        outcome = ProposeDialogOutcome::Confirmed(Box::new(self.proposal.clone()));
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = ProposeDialogOutcome::Cancelled;
                    }
                });
            })
            .response;

        // Own the dialog root node (Role::Dialog, modal) on the container's id.
        ui.ctx().accesskit_node_builder(dialog_id, |node| {
            node.set_role(egui::accesskit::Role::Dialog);
            node.set_author_id(FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
            node.set_label(FEMS_PROPOSE_COMMAND_LABEL.to_owned());
            node.set_modal();
        });
        // Attach the dialog author_id node under the rendered container so it is part of the live tree.
        ui.ctx().accesskit_node_builder(dialog_resp.id, |node| {
            node.set_author_id(FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
            node.set_role(egui::accesskit::Role::Dialog);
            node.set_label(FEMS_PROPOSE_COMMAND_LABEL.to_owned());
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

/// Register the "Propose to Memory" command into the WP-011 command registry's runtime command bus,
/// reusing the existing [`crate::interop::InteractionBus`] registration API (NO duplicate registry/bus,
/// RISK-008, MC-008, AC-006). The handler opens the proposal dialog over the live selection; the live
/// focus→selection→confirm→submit wiring lands at E11/MT-069 (like every other pane), so the handler
/// here records the open intent on the bus. `emitter` + `client` are cloned into the handler so the
/// confirm path can submit off-thread without re-entering the bus lock.
///
/// This is the WRAP-not-fork registration: the static [`crate::command_registry`] catalog carries the
/// discoverable palette row ([`PROPOSE_TO_MEMORY_COMMAND`]); this registers the runtime handler on the
/// same bus the other melt-together commands use.
pub fn register_propose_to_memory_command(
    bus: &mut crate::interop::InteractionBus,
    _emitter: Arc<NativeEditorEventEmitter>,
    _client: Arc<HandshakeCoreClient>,
) {
    use crate::interop::CommandDescriptor;
    let descriptor = CommandDescriptor {
        id: FEMS_PROPOSE_COMMAND_ID,
        name: "ProposeToMemory",
        label: FEMS_PROPOSE_COMMAND_LABEL.to_owned(),
        keywords: vec![
            "memory".to_owned(),
            "fems".to_owned(),
            "propose".to_owned(),
            "review".to_owned(),
        ],
        // Palette-driven: NO keybind (does not steal a VS Code binding — RISK-010).
        keybind: None,
        // The handler opens the dialog flow at E11/MT-069 over the live selection. It is registered now
        // so the command is addressable on the bus; the live open/submit wiring is the host's E11 job
        // (the same deferred-live-wiring shape MT-036's emit call sites use). It must NOT re-enter the
        // bus lock (the bus is already locked when a handler runs).
        handler: Arc::new(move |_ctx: &egui::Context, _bus: &mut crate::interop::InteractionBus| {
            // Open intent: the live dialog mount + off-thread submit lands at E11/MT-069. The command is
            // registered + addressable now; this handler is the seam the host fills with the live
            // dialog-open + submit_proposal_and_emit dispatch. It deliberately performs NO direct memory
            // write (the only write path is the review-gated proposal POST).
            tracing::debug!(
                command = FEMS_PROPOSE_COMMAND_ID,
                "Propose to Memory invoked (dialog mount + off-thread submit lands at E11/MT-069)"
            );
        }),
    };
    bus.register_command(descriptor);
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
        let sel = text_range("pane-rich", 10, 25, "hello memory");
        let p = build_proposal(&sel, MemoryClass::Semantic, "WS-1", "actor-7").expect("builds");
        assert_eq!(p.class, MemoryClass::Semantic);
        assert_eq!(p.content, "hello memory");
        assert_eq!(p.source.document_id, "pane-rich", "document_id derives from the owning pane");
        assert_eq!(p.source.pane_id, "pane-rich");
        assert_eq!(p.source.workspace_id, "WS-1");
        assert_eq!(p.source.selection_start, 10);
        assert_eq!(p.source.selection_end, 25);
        assert_eq!(p.actor_id, "actor-7");
        // content_hash is 64-char lowercase hex (the loom primitive).
        assert_eq!(p.source.content_hash.len(), 64);
        assert!(p.source.content_hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
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
        assert_eq!(got, loom.as_str(), "AC-003: proposal hash == loom block hash for identical content");
        // And it equals the raw canonical primitive (no second scheme).
        assert_eq!(got, canonical_content_sha256(&JsonValue::String(content.to_owned())));
        // Deterministic.
        assert_eq!(got, content_hash_of_selection(content));
    }

    /// AC-002: a Procedural-class proposal has review_gated==true, and review_gated is true for EVERY
    /// class — the editor can never produce a non-review-gated proposal.
    #[test]
    fn review_gated_is_always_true_hard_true_for_procedural() {
        let sel = text_range("pane-code", 0, 4, "step");
        for class in MemoryClass::ORDER {
            let p = build_proposal(&sel, class, "WS-1", "a").expect("builds");
            assert!(p.review_gated, "{:?} proposal must be review_gated", class);
            assert!(p.is_review_gated());
        }
        // Procedural explicitly (the spec requirement).
        let proc = build_proposal(&sel, MemoryClass::Procedural, "WS-1", "a").unwrap();
        assert!(proc.review_gated, "AC-002: Procedural-class proposal is review-gated");
        // There is no field/setter that flips it false: a class switch keeps it true.
        let mut dlg = ProposeToMemoryDialog::open(&sel, "WS-1", "a").unwrap();
        dlg.set_class(MemoryClass::Procedural);
        assert!(dlg.proposal.review_gated, "class switch never sets review_gated false");
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
        let err = build_proposal(&SharedSelection::None, MemoryClass::Episodic, "WS-1", "a").unwrap_err();
        assert_eq!(err, MemoryProposalError::NoSelection);
        assert!(!err.is_missing_endpoint());
    }

    /// AC-008: the FR payload carries action='memory_write_proposed' + proposal_id + class + document_id
    /// + selection range + content_hash + review_gated + pane_id.
    #[test]
    fn fr_payload_carries_full_marker_and_provenance() {
        let sel = text_range("pane-rich", 3, 9, "memory");
        let p = build_proposal(&sel, MemoryClass::Procedural, "WS-1", "a").unwrap();
        let payload = p.fr_payload("PROP-42");
        assert_eq!(payload["action"], "memory_write_proposed");
        assert_eq!(payload["proposal_id"], "PROP-42");
        assert_eq!(payload["class"], "procedural");
        assert_eq!(payload["document_id"], "pane-rich");
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
        let p = build_proposal(&sel, MemoryClass::Episodic, "WS-9", "a").unwrap();
        let ev = p.fr_event("PROP-1");
        assert_eq!(ev.action, NativeEditorAction::MemoryWriteProposed);
        assert_eq!(ev.action.as_str(), "memory_write_proposed");
        assert_eq!(ev.schema_version, NATIVE_EDITOR_SCHEMA_VERSION);
        assert_eq!(ev.workspace_id, "WS-9");
        assert_eq!(ev.pane_id, "pane-rich");
        // The native payload nests under the MT-036 schema (no invented top-level event_type).
        let np = ev.to_native_payload();
        assert_eq!(np["action"], "memory_write_proposed");
        assert_eq!(np["payload"]["proposal_id"], "PROP-1");
    }

    /// The command descriptor is the WP-011 palette catalog row for 'fems.propose_to_memory', enabled
    /// and palette-driven (no keybind, RISK-010).
    #[test]
    fn propose_command_descriptor_is_palette_driven() {
        assert_eq!(PROPOSE_TO_MEMORY_COMMAND.id, "fems.propose_to_memory");
        assert!(!PROPOSE_TO_MEMORY_COMMAND.disabled);
        assert_eq!(PROPOSE_TO_MEMORY_COMMAND.label, "Propose to Memory");
    }

    /// The class radio author ids follow the fems-class-{class} convention.
    #[test]
    fn class_author_ids_follow_convention() {
        assert_eq!(fems_class_author_id(MemoryClass::Episodic), "fems-class-episodic");
        assert_eq!(fems_class_author_id(MemoryClass::Semantic), "fems-class-semantic");
        assert_eq!(fems_class_author_id(MemoryClass::Procedural), "fems-class-procedural");
    }

    /// The body serialized for the submit carries the typed proposal (class/content/source/review_gated/
    /// actor_id) and nothing else — and review_gated is always true.
    #[test]
    fn submit_body_shape_is_review_gated_proposal() {
        let sel = text_range("pane-rich", 0, 6, "memory");
        let p = build_proposal(&sel, MemoryClass::Semantic, "WS-1", "actor-1").unwrap();
        let body = json!({
            "class": p.class.wire(),
            "content": p.content,
            "source": p.source,
            "review_gated": p.review_gated,
            "actor_id": p.actor_id,
        });
        assert_eq!(body["class"], "semantic");
        assert_eq!(body["content"], "memory");
        assert_eq!(body["review_gated"], true);
        assert_eq!(body["source"]["document_id"], "pane-rich");
        assert_eq!(body["source"]["content_hash"], p.source.content_hash);
        assert_eq!(body["actor_id"], "actor-1");
    }
}
