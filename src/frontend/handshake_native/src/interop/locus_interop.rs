//! Editors <-> Locus (Pillar 6, structured work tracking) interop edge (WP-KERNEL-012 MT-068, cluster E10).
//!
//! ## What this is — the editors <-> Locus cross-reference edge
//!
//! Locus is the project's structured work-tracking pillar: it holds Work Packet (WP) and Microtask (MT)
//! records (e.g. `WP-KERNEL-012`, `MT-034`). This module lets a note or code comment cross-reference those
//! governed work units through a `locus://` addressing scheme that sits ALONGSIDE the two schemes already
//! built in this WP:
//!
//! - the MT-032 loom/everything-is-a-block scheme ([`crate::loom_address`] — `loom://` URIs + the
//!   `normalize` discipline), and
//! - the MT-034 code<->note cross-reference machinery ([`crate::interop::cross_ref`] — the `hsLink` atom by
//!   `ref_kind`, the resolution service, and the loom search-v2 reverse-lookup mechanism).
//!
//! Concretely:
//! - **(A) ref -> record:** a note or comment writes `locus://wp/WP-KERNEL-012` or `locus://mt/MT-034`;
//!   [`parse_locus_ref`] recognizes it, the editor renders it as a clickable Locus chip (the MT-034 CrossRef
//!   chip path, [`crate::rich_editor::wikilinks::inline_view`]), and clicking dispatches `open-locus-ref`
//!   through the EXISTING MT-031 [`crate::interop::InteractionBus`] + MT-030 nav seam to focus the WP/MT
//!   record. [`LocusInteropService::resolve_locus_ref`] reads the record's title/summary via the bound READ
//!   API.
//! - **(B) reverse lookup:** [`LocusInteropService::find_documents_referencing`] answers "which
//!   documents/code reference this WP/MT?" so a Locus panel (MT-073, downstream) can list referencing
//!   documents for a given work unit. It REUSES the MT-034 [`crate::interop::cross_ref::find_notes_with`]
//!   loom search-v2 mechanism keyed on the normalized `locus://` ref value — NOT a re-implemented scan.
//!
//! This edge is **READ + REFERENCE ONLY**: the editors never create, mutate, transition, or delete Locus
//! WP/MT records; they only resolve a `locus://` ref to a record title/summary and navigate to it, and
//! enumerate inbound references.
//!
//! ## VERIFIED BACKEND REALITY (KERNEL_BUILDER gate 2026-06-25): NO Locus READ HTTP routes exist
//!
//! VERIFIED read-only against `src/backend/handshake_core`: Locus (Pillar 6) has a kernel/governance DATA
//! MODEL (`kernel/locus_work_tracking_reset.rs` `LocusWorkPacketRecordV1`, `locus/mod.rs`,
//! `locus/task_board.rs` — Locus IS the WP/MT work-tracking, the same concept as the `.GOV` task packets),
//! but there are **NO HTTP routes** exposing it to the frontend-reachable API surface: the entire
//! `src/backend/handshake_core/src/api/` route surface has no `locus.rs`, and there is no
//! `GET /workspaces/{ws}/locus/work-packets/{id}` / `/locus/microtasks/{id}` route registered anywhere
//! (the only `locus` mention under `api/` is an internal `crate::workflows::locus` governance import in
//! `role_mailbox.rs`, not an HTTP route). Like FEMS (Pillar 12), Stage (Pillar 17), and Calendar (Pillar 2),
//! Locus is a separate system not yet wired into the frozen handshake_core HTTP surface.
//!
//! So [`LocusInteropService::resolve_locus_ref`] returns the FIRST-CLASS TYPED BLOCKER
//! [`LocusInteropError::LocusReadApiUnavailable`] naming the exact missing endpoint — the contract's
//! DESIGNED typed-blocker path (RISK-006/MC-006). The chip then renders GREYED-unavailable with a tooltip
//! naming the missing endpoint, NEVER a panic, NEVER a fabricated record. This is DISTINCT from a live 404
//! (record-not-found), which would grey the chip as `unresolved` once the route exists — the two failure
//! modes are kept apart (RISK-003/MC-003). NO backend route is added, NO Locus state is mutated, NO SQLite
//! is introduced (RISK-006). The parser + chip + node + reverse-lookup keying are PROVABLE NOW; the live
//! resolution against real `/locus/` routes is the documented gated blocker until the route is exposed.
//!
//! ## Reuse, do not fork (the contract's core constraint, AC-007)
//!
//! - **Normalizer (single key):** [`LocusRef::normalized`] is derived by [`normalize_locus_id`], which
//!   reuses the MT-032 [`crate::loom_address::LOOM_URI_SCHEME`] discipline + the same trim/lower-case
//!   normalization the MT-015 wikilink resolver uses ([`crate::rich_editor::wikilinks::resolver::normalize_target`]).
//!   The normalized value is the SINGLE shared key for both resolution AND reverse lookup so the two
//!   directions never disagree (RISK-001/MC-001). NO second normalizer is defined.
//! - **Cross-ref node/chip:** the `locus_ref` node IS the EXISTING [`crate::rich_editor::document_model::node::HsLinkNode`]
//!   `hsLink` atom (`ref_kind = "locus"`, `ref_value = locus://...`) — the sibling of the MT-034 `code_ref`
//!   node, rendered through the SAME CrossRef chip path (RISK-002/MC-002). [`DocumentRef`] mirrors the
//!   MT-034 [`crate::interop::cross_ref::NoteRef`] shape so the two cross-ref surfaces stay symmetric.
//! - **Reverse lookup:** reuses [`crate::interop::cross_ref::find_notes_with`] (the MT-034 loom search-v2
//!   mechanism) keyed on the normalized `locus://` ref value, de-duplicated on `(document_id, block_id)`.
//! - **HTTP stack:** the read client holds a cloned [`reqwest::Client`] (the process-wide
//!   [`crate::backend_client::shared_http_client`] pool) + the config-resolved
//!   [`crate::backend_client::BACKEND_BASE_URL`] — the exact MT-066/MT-067 sibling pattern. NO second
//!   reqwest stack, NO new async runtime.
//! - **Navigation:** `open-locus-ref` routes through the EXISTING MT-031 bus + MT-030 nav seam
//!   ([`crate::interop::CMD_OPEN_LOCUS_REF`], the stage-then-dispatch split) — NO new navigation channel
//!   (RISK-007/MC-007).
//!
//! ## URL id encoding (RISK-010/MC-010)
//!
//! WP/MT ids contain hyphens and uppercase segments (`WP-KERNEL-012`), all unreserved-or-safe but exercised
//! explicitly: [`LocusInteropService::resolve_path`] percent-encodes the id via the MT-034
//! [`crate::interop::cross_ref::percent_encode_symbol`] (the same dependency-free encoder) before embedding
//! it in the URL path, so the resolution call targets the correct, correctly-encoded path. A unit test
//! exercises `WP-KERNEL-012`.

use std::sync::Arc;
use std::time::Duration;

use crate::backend_client::{
    shared_http_client, BACKEND_BASE_URL, HSK_HEADER_ACTOR_ID, HSK_HEADER_KERNEL_TASK_RUN_ID,
    HSK_HEADER_SESSION_RUN_ID,
};
use crate::interop::cross_ref::{
    find_notes_with, percent_encode_symbol, CrossRefError, FindNotesHttp, FindNotesSearch, NoteRef,
};

/// The `hsLink` `ref_kind` a Locus cross-reference atom carries (the discriminator the ref -> record
/// dispatch keys on). Registered in `wikilinks/parser.rs` so `[[locus:...]]` parses to it; the SIBLING of
/// [`crate::interop::cross_ref::CODE_REF_KIND`] (`"code"`).
pub const LOCUS_REF_KIND: &str = "locus";

/// The URI scheme a Locus reference is addressable under (`locus://wp/{id}` / `locus://mt/{id}`). Sits
/// ALONGSIDE the MT-032 [`crate::loom_address::LOOM_URI_SCHEME`] (`loom://`), kept consistent with that
/// scheme's `{scheme}//{kind}/{id}` shape.
pub const LOCUS_URI_SCHEME: &str = "locus://";

/// The read timeout for a Locus record read (a bounded timeout so a hung backend cannot stall the editor
/// frame loop — the same bound the MT-066/MT-067 sibling clients use).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

/// The least-privileged read-only actor id used for the Locus record reads (no `x-hsk-actor-kind` =>
/// read-only server-side, the same least-privilege default the FEMS/Stage/Calendar read paths use).
const LOCUS_READ_ACTOR_ID: &str = "native-editor-locus-reader";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Domain types — the work-unit ref, the resolved record, and the inbound-reference row.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Which kind of Locus work unit a [`LocusRef`] addresses. A `locus://wp/{id}` is a [`Self::WorkPacket`];
/// a `locus://mt/{id}` is a [`Self::Microtask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocusRefKind {
    /// A Work Packet record (`locus://wp/{id}`, e.g. `WP-KERNEL-012`).
    WorkPacket,
    /// A Microtask record (`locus://mt/{id}`, e.g. `MT-034`).
    Microtask,
}

impl LocusRefKind {
    /// The stable kebab-style token for the kind (the chip attr + AccessKit id vocabulary). `"wp"` for a
    /// Work Packet, `"mt"` for a Microtask — matching the `locus://{kind}/{id}` path segment.
    pub fn as_str(&self) -> &'static str {
        match self {
            LocusRefKind::WorkPacket => "wp",
            LocusRefKind::Microtask => "mt",
        }
    }

    /// Parse the `{kind}` path segment of a `locus://` URI back into a [`LocusRefKind`]. Case-insensitive
    /// (so `WP`/`wp` both resolve). `None` for an unrecognized segment.
    pub fn from_segment(segment: &str) -> Option<Self> {
        match segment.trim().to_ascii_lowercase().as_str() {
            "wp" => Some(LocusRefKind::WorkPacket),
            "mt" => Some(LocusRefKind::Microtask),
            _ => None,
        }
    }

    /// The READ-API resource segment this kind reads from (`work-packets` for a WP, `microtasks` for an
    /// MT) — the path the resolution endpoint uses. Built here so the typed blocker names the right route.
    pub fn read_resource(&self) -> &'static str {
        match self {
            LocusRefKind::WorkPacket => "work-packets",
            LocusRefKind::Microtask => "microtasks",
        }
    }
}

/// A parsed Locus reference: the work-unit kind, the raw form as authored, the work-unit id, and the
/// canonical normalized form. The `normalized` value is the SINGLE shared key for both resolution and
/// reverse lookup (RISK-001/MC-001), so the two directions never disagree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocusRef {
    /// The kind of work unit (WP / MT).
    pub kind: LocusRefKind,
    /// The raw reference exactly as authored (e.g. `"locus://wp/WP-KERNEL-012"` or the bare-id shorthand).
    pub raw: String,
    /// The work-unit id (e.g. `"WP-KERNEL-012"` or `"MT-034"`), trimmed (case preserved — WP/MT ids are
    /// uppercase-significant, so the id itself is NOT lower-cased; only [`Self::normalized`] is).
    pub id: String,
    /// The canonical normalized `locus://{kind}/{id}` form — the single key shared by resolution + reverse
    /// lookup (derived by [`normalize_locus_id`]; trim + collapse-whitespace + lower-case, consistent with
    /// the MT-032/MT-015 normalization discipline).
    pub normalized: String,
}

impl LocusRef {
    /// The canonical `locus://{kind}/{id}` URI for this ref (the original-case id form). Distinct from
    /// [`Self::normalized`] (which lower-cases for keying); this is the display/round-trip form.
    pub fn to_uri(&self) -> String {
        format!("{LOCUS_URI_SCHEME}{}/{}", self.kind.as_str(), self.id)
    }
}

/// The resolved, display-facing projection of a Locus WP/MT record (the [`LocusInteropService::resolve_locus_ref`]
/// success output). Carries the title/summary/status the chip tooltip + a downstream Locus panel render.
/// Built from the bound READ API response (once the route exists).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocusRecord {
    /// The kind of work unit this record is (WP / MT).
    pub kind: LocusRefKind,
    /// The work-unit id (e.g. `"WP-KERNEL-012"`).
    pub id: String,
    /// The record's display title (non-empty for a resolved record — the AC-002 assertion).
    pub title: String,
    /// The record's summary, when the backend carries one.
    pub summary: Option<String>,
    /// The record's lifecycle status, when the backend carries one (e.g. `"Ready for Dev"`).
    pub status: Option<String>,
}

/// A document/code block that references a given WP/MT — the reverse-lookup result row (the contract's
/// `DocumentRef`). MIRRORS the MT-034 [`NoteRef`] shape (`{document_id, document_title, block_id, excerpt}`)
/// so the two cross-ref surfaces stay symmetric (RISK-002/MC-002). Built from an MT-034 [`NoteRef`] via
/// [`Self::from_note_ref`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRef {
    /// The referencing document id (the `open-document` navigation target).
    pub document_id: String,
    /// The document's display title (falls back to the block id when untitled).
    pub document_title: String,
    /// The referencing block id, when distinct from the document id (mirrors `NoteRef.block_id`). `None`
    /// only if a future shape carries no block id; today the loom hit always carries one.
    pub block_id: Option<String>,
    /// A short excerpt centered on the `locus://` mention (the search highlight, markers stripped).
    pub excerpt: String,
}

impl DocumentRef {
    /// Build a [`DocumentRef`] from an MT-034 [`NoteRef`] (the reverse-lookup reuses the MT-034 search +
    /// hit mapping, so a hit arrives as a `NoteRef`; this is the symmetric projection). The block id is
    /// always present from a loom hit, so it is carried as `Some`.
    pub fn from_note_ref(note: NoteRef) -> Self {
        Self {
            document_id: note.document_id,
            document_title: note.document_title,
            block_id: Some(note.block_id),
            excerpt: note.excerpt,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Pure parser + normalizer (no I/O, no async — trivially unit-testable).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Normalize a Locus `{kind}/{id}` form into the canonical lower-cased `locus://{kind}/{id}` key (the
/// single shared key for resolution + reverse lookup, RISK-001/MC-001). REUSES the MT-015/MT-032
/// normalization discipline ([`crate::rich_editor::wikilinks::resolver::normalize_target`] — trim +
/// collapse-whitespace + unicode lower-case) rather than adding a second normalizer (AC-007). The scheme
/// prefix is taken from this module's [`LOCUS_URI_SCHEME`], kept consistent with the MT-032
/// [`crate::loom_address::LOOM_URI_SCHEME`] shape.
pub fn normalize_locus_id(kind: LocusRefKind, id: &str) -> String {
    // Reuse the MT-015 wikilink resolver normalizer (the SAME normalization the rest of the editor's
    // cross-ref keying uses) so the key is consistent across schemes and there is no second normalizer.
    let normalized_id = crate::rich_editor::wikilinks::resolver::normalize_target(id);
    format!("{LOCUS_URI_SCHEME}{}/{}", kind.as_str(), normalized_id)
}

/// Parse a Locus reference, the PURE side-effect-free entry point (AC-001). Recognizes:
///
/// - the full URI form `locus://wp/{id}` -> [`LocusRef`] (kind=WorkPacket), `locus://mt/{id}` ->
///   (kind=Microtask),
/// - the prefix-stripped `{kind}/{id}` form the WIKILINK parser emits for `[[locus:wp/WP-KERNEL-012]]`
///   (the wikilink stores everything after the `locus:` prefix as `ref_value`, so the authored atom carries
///   `ref_value="wp/WP-KERNEL-012"` — this branch makes every chip/dispatch helper that re-parses
///   `ref_value` key on the SAME canonical `normalized` as the URI form), and
/// - the bare-id shorthand the MT-032 scheme already normalizes consistently: a bare `WP-...` -> a WP ref,
///   a bare `MT-...` -> an MT ref (so a comment can write the bare id and still resolve — kept consistent
///   with the loom scheme's bare-id normalization).
///
/// Returns `None` for any string that is not a Locus reference (wrong scheme, missing the `/` separator,
/// an unrecognized `{kind}` segment, or an empty id). The `normalized` field is the canonical lower-cased
/// key from [`normalize_locus_id`]; the `id` field preserves the original case (WP/MT ids are
/// uppercase-significant).
pub fn parse_locus_ref(raw: &str) -> Option<LocusRef> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1) The full `locus://{kind}/{id}` URI form.
    if let Some(rest) = trimmed.strip_prefix(LOCUS_URI_SCHEME) {
        let (kind_seg, id_seg) = rest.split_once('/')?;
        let kind = LocusRefKind::from_segment(kind_seg)?;
        let id = id_seg.trim();
        if id.is_empty() {
            return None;
        }
        return Some(LocusRef {
            kind,
            raw: trimmed.to_owned(),
            id: id.to_owned(),
            normalized: normalize_locus_id(kind, id),
        });
    }

    // 2) The prefix-stripped `{kind}/{id}` form the WIKILINK parser emits for `[[locus:wp/WP-KERNEL-012]]`.
    //    The wikilink machinery stores everything AFTER the `locus:` prefix as `ref_value` (group 2 of the
    //    regex), so an authored `[[locus:wp/WP-KERNEL-012]]` round-trips an hsLink atom with
    //    `ref_value="wp/WP-KERNEL-012"` (NOT the full `locus://` URI). This branch recognizes that exact
    //    authoring form so every chip/dispatch helper that re-parses `ref_value` keys on the SAME canonical
    //    `normalized` value as the URI/bare-id forms (the single-shared-key invariant, RISK-001/MC-001). The
    //    leading segment must be a known kind (`wp`/`mt`); a bare `path/to/file.ts` value (a code/file ref)
    //    has no `wp`/`mt` leading segment and so does NOT match here, falling through to `None`.
    if let Some((kind_seg, id_seg)) = trimmed.split_once('/') {
        if let Some(kind) = LocusRefKind::from_segment(kind_seg) {
            let id = id_seg.trim();
            if !id.is_empty() {
                return Some(LocusRef {
                    kind,
                    raw: trimmed.to_owned(),
                    id: id.to_owned(),
                    normalized: normalize_locus_id(kind, id),
                });
            }
        }
    }

    // 3) The bare-id shorthand (consistent with the MT-032 scheme's bare-id normalization): infer the kind
    //    from the id prefix. `WP-...` -> WorkPacket, `MT-...` -> Microtask. The id keeps its original case.
    let upper = trimmed.to_ascii_uppercase();
    let kind = if upper.starts_with("WP-") {
        LocusRefKind::WorkPacket
    } else if upper.starts_with("MT-") {
        LocusRefKind::Microtask
    } else {
        return None;
    };
    Some(LocusRef {
        kind,
        raw: trimmed.to_owned(),
        id: trimmed.to_owned(),
        normalized: normalize_locus_id(kind, trimmed),
    })
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error — LocusReadApiUnavailable is the first-class typed blocker (DISTINCT from a live 404).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Why a Locus interop operation failed. Every variant renders as a VISIBLE state (greyed chip / typed
/// error), never a silent no-op or a panic.
///
/// [`Self::LocusReadApiUnavailable`] is the FIRST-CLASS TYPED BLOCKER (RISK-006/MC-006, AC-005): the Locus
/// work-packets/microtasks GET endpoint is ABSENT from the frontend-reachable surface in this build (not
/// merely a 404 for a missing id). It carries the exact probed endpoint so the validator + operator see
/// which route is missing. It is DISTINCT from [`Self::NotFound`] (a live-endpoint 404 = record-not-found),
/// so the chip can tell "feature not exposed" (greyed-unavailable, tooltip names the endpoint) apart from
/// "record deleted" (greyed-unresolved) — the two failure modes are never conflated (RISK-003/MC-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocusInteropError {
    /// No workspace bound — a Locus read needs a workspace id.
    NoWorkspace,
    /// THE TYPED BLOCKER: the Locus READ endpoint is absent from the frontend-reachable API surface in this
    /// build (the work-packets/microtasks GET route is not registered at all — not a per-id 404). Carries
    /// the exact probed endpoint path. NO backend route is added; NO record is fabricated.
    LocusReadApiUnavailable { endpoint: String },
    /// A live-endpoint 404 / empty projection: the route EXISTS but the WP/MT record was not found (a
    /// deleted/unknown id). DISTINCT from [`Self::LocusReadApiUnavailable`] — this greys the chip as
    /// `unresolved`, NOT as `unavailable` (RISK-003/MC-003). Reserved for when the route is exposed.
    NotFound { id: String },
    /// A non-success HTTP status that is NOT the typed endpoint-absent blocker or a 404 (e.g. a 500, a 403).
    Http { status: u16 },
    /// A decode failure on a success body (the wire shape did not match the domain type).
    Decode(String),
    /// A transport-layer failure (connect / timeout / TLS).
    Transport(String),
    /// The reverse-lookup search failed (propagated from the reused MT-034 [`CrossRefError`], never
    /// swallowed). The reverse lookup reuses the MT-034 loom search-v2 mechanism; its failure surfaces here.
    ReverseLookup(String),
}

impl std::fmt::Display for LocusInteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoWorkspace => write!(
                f,
                "Locus interop: no workspace context (a Locus read needs a workspace id)"
            ),
            Self::LocusReadApiUnavailable { endpoint } => write!(
                f,
                "Locus read endpoint not present in this build (probed {endpoint})"
            ),
            Self::NotFound { id } => write!(f, "Locus record not found: {id}"),
            Self::Http { status } => write!(f, "Locus interop: HTTP {status}"),
            Self::Decode(why) => write!(f, "Locus interop decode error: {why}"),
            Self::Transport(why) => write!(f, "Locus interop transport error: {why}"),
            Self::ReverseLookup(why) => write!(f, "Locus reverse-lookup error: {why}"),
        }
    }
}

impl std::error::Error for LocusInteropError {}

impl LocusInteropError {
    /// True when this is the typed-blocker variant (the chip renders greyed-UNAVAILABLE with a tooltip
    /// naming the missing endpoint, and the blocker is surfaced to the WP validator). DISTINCT from a live
    /// [`Self::NotFound`] 404 (RISK-003/MC-003).
    pub fn is_read_api_unavailable(&self) -> bool {
        matches!(self, LocusInteropError::LocusReadApiUnavailable { .. })
    }

    /// True when the chip should render GREYED but NOT as the typed blocker: a live-endpoint 404
    /// (record-not-found). The chip shows an `unresolved` affordance (the deleted/unknown record case).
    pub fn is_record_not_found(&self) -> bool {
        matches!(self, LocusInteropError::NotFound { .. })
    }

    /// The endpoint the typed blocker probed, when this is the blocker variant (for the chip tooltip + the
    /// validator report). `None` for the other variants.
    pub fn unavailable_endpoint(&self) -> Option<&str> {
        match self {
            LocusInteropError::LocusReadApiUnavailable { endpoint } => Some(endpoint),
            _ => None,
        }
    }

    /// The stable greyed-chip tooltip naming the missing Locus READ endpoint (AC-005). Used by the chip
    /// renderer + the User Manual + the tests so they reference the same copy.
    pub fn unavailable_tooltip(&self) -> String {
        match self {
            LocusInteropError::LocusReadApiUnavailable { endpoint } => {
                format!("Locus record unavailable — backend endpoint not exposed ({endpoint})")
            }
            other => other.to_string(),
        }
    }
}

/// Map a reused MT-034 [`CrossRefError`] (from the reverse-lookup search) into the Locus error model. A
/// search failure surfaces as [`LocusInteropError::ReverseLookup`] (propagated, never swallowed). A
/// missing-workspace is mapped to [`LocusInteropError::NoWorkspace`] so the two error models agree.
impl From<CrossRefError> for LocusInteropError {
    fn from(e: CrossRefError) -> Self {
        match e {
            CrossRefError::NoWorkspace => LocusInteropError::NoWorkspace,
            other => LocusInteropError::ReverseLookup(other.to_string()),
        }
    }
}

/// A typed result alias for Locus interop operations.
pub type LocusResult<T> = Result<T, LocusInteropError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The interop service — the READ-ONLY resolution + reverse-lookup client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The Locus interop service (the contract's resolution + reverse-lookup surface). Holds:
/// - a cloned [`reqwest::Client`] (the process-wide [`shared_http_client`] pool — NO second HTTP stack) +
///   the config-resolved base URL for the Locus record reads, and
/// - the workspace the WP/MT records belong to.
///
/// All methods are async and READ-ONLY (a single GET, never a write verb). In THIS build the Locus READ
/// routes are absent, so [`Self::resolve_locus_ref`] returns the designed typed blocker
/// [`LocusInteropError::LocusReadApiUnavailable`]. The reverse lookup is the REAL MT-034 search mechanism,
/// keyed on the normalized `locus://` ref.
#[derive(Clone)]
pub struct LocusInteropService {
    /// The shared HTTP pool (the WP-011 `backend_client` pool — no second stack).
    http: reqwest::Client,
    /// The config-resolved backend base URL (never hardcoded at a call site — GLOBAL-PORTABILITY-004).
    base_url: String,
    /// The workspace the Locus records + the referencing documents belong to.
    workspace_id: String,
    /// The reverse-lookup search backend (the MT-034 loom search-v2 mechanism). An `Arc<dyn ...>` so a test
    /// injects a counted in-memory mock (NO backend) and production uses the reqwest impl.
    reverse_lookup: Arc<dyn FindNotesSearch>,
    /// The session run id on the read identity headers (so swarm/operator co-work is attributable).
    session_run_id: String,
}

impl LocusInteropService {
    /// Construct against the production backend (the config-resolved [`BACKEND_BASE_URL`], the shared
    /// [`shared_http_client`] pool) for `workspace_id`, with the production MT-034 loom search-v2 reverse
    /// lookup ([`FindNotesHttp`]).
    pub fn production(workspace_id: impl Into<String>) -> Self {
        Self {
            http: shared_http_client(),
            base_url: BACKEND_BASE_URL.to_owned(),
            workspace_id: workspace_id.into(),
            reverse_lookup: Arc::new(FindNotesHttp::production()),
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool), with an injected reverse-lookup search backend (a counted mock in tests).
    /// The base URL is the host authority — never hardcoded at a call site (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(
        base_url: impl Into<String>,
        workspace_id: impl Into<String>,
        reverse_lookup: Arc<dyn FindNotesSearch>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            workspace_id: workspace_id.into(),
            reverse_lookup,
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Override the session run id on the read identity headers (builder-style).
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
        self
    }

    /// The workspace this service binds.
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The Locus record read path for a workspace + ref (the documented — currently absent — route). The
    /// WP/MT id is PERCENT-ENCODED (RISK-010/MC-010) via the MT-034 [`percent_encode_symbol`] before it is
    /// embedded, so an id with hyphens/uppercase (`WP-KERNEL-012`) targets the correct, correctly-encoded
    /// path. Built here so [`LocusInteropError::LocusReadApiUnavailable`] can report the exact probed path.
    pub fn resolve_path(workspace_id: &str, r: &LocusRef) -> String {
        format!(
            "/workspaces/{}/locus/{}/{}",
            workspace_id,
            r.kind.read_resource(),
            percent_encode_symbol(&r.id),
        )
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Resolve a [`LocusRef`] to its [`LocusRecord`] (title/summary/status) via the bound READ API (AC-002).
    /// WorkPacket -> `GET /workspaces/{ws}/locus/work-packets/{id}`; Microtask ->
    /// `GET /workspaces/{ws}/locus/microtasks/{id}`. READ-ONLY: a single GET, never a write verb.
    ///
    /// Behavior contract (the two failure modes kept DISTINCT, RISK-003/MC-003):
    /// - The route ABSENT from the reachable surface (404 / 501 / route-not-registered) -> the TYPED
    ///   BLOCKER [`LocusInteropError::LocusReadApiUnavailable`] naming the endpoint (the DESIGNED PRIMARY
    ///   PATH in this build — the chip renders greyed-UNAVAILABLE, no panic, no fabricated record).
    /// - The route EXISTS but the id is unknown (a per-id 404 once the route is live) -> ...this is the
    ///   SAME wire status as an absent route over plain HTTP, so the absent-route blocker is the honest
    ///   classification while no `/locus/` route exists at all. When the route is added, the handler can
    ///   return a body distinguishing the two; until then, the absent-route blocker is correct and the
    ///   [`LocusInteropError::NotFound`] variant exists + is unit-tested for that future live-404 path.
    ///
    /// A decode failure on a success body is [`LocusInteropError::Decode`]; a transport failure is
    /// [`LocusInteropError::Transport`].
    pub async fn resolve_locus_ref(&self, r: &LocusRef) -> LocusResult<LocusRecord> {
        if self.workspace_id.trim().is_empty() {
            return Err(LocusInteropError::NoWorkspace);
        }
        let path = Self::resolve_path(&self.workspace_id, r);
        let url = self.url(&path);
        let resp = self
            .http
            .get(&url)
            .timeout(REQUEST_TIMEOUT)
            // READ identity: least-privileged read-only actor (no x-hsk-actor-kind => read-only).
            .header(HSK_HEADER_ACTOR_ID, LOCUS_READ_ACTOR_ID)
            .header(
                HSK_HEADER_KERNEL_TASK_RUN_ID,
                format!("native-editor-locus-{}", self.workspace_id),
            )
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id)
            .send()
            .await
            .map_err(|e| LocusInteropError::Transport(e.to_string()))?;
        let status = resp.status();

        // THE TYPED BLOCKER (BROAD detection — RISK-006/MC-006): 404 (route absent) OR 501 (not
        // implemented) both mean the Locus READ route is not present in this build. Because NO `/locus/`
        // HTTP route exists at all on the reachable surface (VERIFIED), a 404 here is the route being
        // absent, not a per-id miss — surface it as the typed blocker naming the endpoint. Never panic,
        // never fabricate. (When the route is exposed, its handler distinguishes per-id 404 in the body;
        // that future live-404 path maps to LocusInteropError::NotFound.)
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
        {
            return Err(LocusInteropError::LocusReadApiUnavailable { endpoint: path });
        }
        if !status.is_success() {
            return Err(LocusInteropError::Http {
                status: status.as_u16(),
            });
        }
        let body: LocusRecordWire = resp
            .json()
            .await
            .map_err(|e| LocusInteropError::Decode(e.to_string()))?;
        Ok(body.into_record(r))
    }

    /// Find the documents/code blocks that reference a given WP/MT (the reverse-lookup direction, AC-004).
    /// REUSES the MT-034 [`find_notes_with`] loom search-v2 mechanism keyed on the NORMALIZED `locus://`
    /// ref value (the single shared key — RISK-001), restricted to rich-doc content types, and de-duplicates
    /// the results on `(document_id, block_id)`. Returns the documents whose stored content carries the
    /// `locus://` ref to the given work unit.
    ///
    /// An empty (zero-hit) result is `Ok(vec![])` (the honest "no documents reference this" state). A search
    /// failure surfaces as [`LocusInteropError::ReverseLookup`] (propagated from the reused
    /// [`CrossRefError`], never swallowed). An empty workspace -> [`LocusInteropError::NoWorkspace`].
    pub async fn find_documents_referencing(&self, r: &LocusRef) -> LocusResult<Vec<DocumentRef>> {
        if self.workspace_id.trim().is_empty() {
            return Err(LocusInteropError::NoWorkspace);
        }
        // The SINGLE shared key (RISK-001): the normalized `locus://` ref value, the same key resolution
        // uses. The MT-034 search keys on this value, restricted to rich-doc content types (RISK-1 reuse).
        let notes = find_notes_with(
            self.reverse_lookup.as_ref(),
            &r.normalized,
            &self.workspace_id,
        )
        .await
        .map_err(LocusInteropError::from)?;
        // De-duplicate on (document_id, block_id) (AC-004) — a ref mentioned in both a `note` and a
        // `journal` block of the same document is listed once per (doc, block).
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for note in notes {
            let key = (note.document_id.clone(), note.block_id.clone());
            if seen.insert(key) {
                out.push(DocumentRef::from_note_ref(note));
            }
        }
        Ok(out)
    }
}

/// The wire shape a Locus record GET body decodes into (once the route exists). Tolerant `#[serde(default)]`
/// fields so a partial body still decodes; the kind + id come from the [`LocusRef`] the read was issued for
/// (the request authority), not re-derived from the body.
#[derive(Debug, Clone, serde::Deserialize)]
struct LocusRecordWire {
    /// The record's display title.
    #[serde(default)]
    title: String,
    /// The record's summary, when present.
    #[serde(default)]
    summary: Option<String>,
    /// The record's lifecycle status, when present.
    #[serde(default)]
    status: Option<String>,
}

impl LocusRecordWire {
    /// Project the wire body into a [`LocusRecord`] for the [`LocusRef`] the read resolved (the kind + id
    /// come from the request, the title/summary/status from the body). An empty summary string normalizes
    /// to `None` so an absent summary is honestly absent.
    fn into_record(self, r: &LocusRef) -> LocusRecord {
        let summary = self.summary.filter(|s| !s.trim().is_empty());
        let status = self.status.filter(|s| !s.trim().is_empty());
        LocusRecord {
            kind: r.kind,
            id: r.id.clone(),
            title: self.title,
            summary,
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parser (AC-001) ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_locus_ref_wp_and_mt_uri_forms() {
        // AC-001: `locus://wp/WP-KERNEL-012` -> WorkPacket; `locus://mt/MT-034` -> Microtask. The id keeps
        // its original case; the normalized field is the canonical lower-cased `locus://` key.
        let wp = parse_locus_ref("locus://wp/WP-KERNEL-012").expect("a valid wp ref");
        assert_eq!(wp.kind, LocusRefKind::WorkPacket);
        assert_eq!(wp.id, "WP-KERNEL-012", "the id preserves original case");
        assert_eq!(
            wp.normalized, "locus://wp/wp-kernel-012",
            "the normalized key is lower-cased"
        );
        assert_eq!(wp.to_uri(), "locus://wp/WP-KERNEL-012");

        let mt = parse_locus_ref("locus://mt/MT-034").expect("a valid mt ref");
        assert_eq!(mt.kind, LocusRefKind::Microtask);
        assert_eq!(mt.id, "MT-034");
        assert_eq!(mt.normalized, "locus://mt/mt-034");
    }

    #[test]
    fn parse_locus_ref_is_case_insensitive_on_kind_segment() {
        // The `{kind}` segment is case-insensitive (WP/wp both resolve), consistent with the rest of the
        // scheme; the id case is preserved.
        let wp = parse_locus_ref("locus://WP/WP-7").expect("parseable");
        assert_eq!(wp.kind, LocusRefKind::WorkPacket);
        assert_eq!(wp.id, "WP-7");
    }

    #[test]
    fn parse_locus_ref_bare_id_shorthand() {
        // The bare-id shorthand (consistent with the MT-032 scheme's bare-id normalization): a bare WP/MT
        // id infers the kind from the prefix and normalizes to the same key as the full URI form.
        let wp = parse_locus_ref("WP-KERNEL-012").expect("bare wp id");
        assert_eq!(wp.kind, LocusRefKind::WorkPacket);
        assert_eq!(wp.id, "WP-KERNEL-012");
        assert_eq!(
            wp.normalized, "locus://wp/wp-kernel-012",
            "bare id normalizes to the same key as the URI form"
        );
        let mt = parse_locus_ref("MT-034").expect("bare mt id");
        assert_eq!(mt.kind, LocusRefKind::Microtask);
        assert_eq!(mt.normalized, "locus://mt/mt-034");
        // The bare id and the full URI form for the same work unit normalize to the SAME key (so the two
        // authoring forms never disagree in resolution/reverse lookup).
        assert_eq!(
            parse_locus_ref("WP-KERNEL-012").unwrap().normalized,
            parse_locus_ref("locus://wp/WP-KERNEL-012")
                .unwrap()
                .normalized
        );
    }

    #[test]
    fn parse_locus_ref_rejects_invalid() {
        // AC-001: an invalid scheme / shape / kind returns None (never a panic).
        assert!(parse_locus_ref("https://wp/WP-1").is_none(), "wrong scheme");
        assert!(
            parse_locus_ref("loom://ws/blk").is_none(),
            "the loom scheme is not a locus ref"
        );
        assert!(parse_locus_ref("locus://wp").is_none(), "no id segment");
        assert!(
            parse_locus_ref("locus://zz/WP-1").is_none(),
            "unknown kind segment"
        );
        assert!(parse_locus_ref("locus://wp/").is_none(), "empty id");
        assert!(parse_locus_ref("just text").is_none(), "not a ref");
        assert!(parse_locus_ref("").is_none(), "empty");
        assert!(
            parse_locus_ref("FOO-123").is_none(),
            "a non-WP/MT bare id is not a locus ref"
        );
    }

    #[test]
    fn normalize_is_the_single_shared_key() {
        // RISK-001/MC-001: the normalized value used for resolution and reverse lookup is the SAME for a
        // given work unit regardless of input casing/whitespace, so the two directions never disagree.
        let a = parse_locus_ref("locus://wp/WP-KERNEL-012").unwrap();
        let b = parse_locus_ref("  locus://WP/wp-kernel-012  ").unwrap();
        assert_eq!(
            a.normalized, b.normalized,
            "casing + whitespace collapse to one key"
        );
        assert_eq!(
            a.normalized,
            normalize_locus_id(LocusRefKind::WorkPacket, "WP-KERNEL-012")
        );
    }

    // ── Typed blocker (AC-005) + two failure modes distinct (RISK-003) ───────────────────────────────

    #[test]
    fn read_api_unavailable_is_distinct_typed_blocker() {
        let blocker = LocusInteropError::LocusReadApiUnavailable {
            endpoint: "/workspaces/WS-1/locus/work-packets/WP-KERNEL-012".into(),
        };
        assert!(blocker.is_read_api_unavailable());
        assert!(
            !blocker.is_record_not_found(),
            "the blocker is NOT a record-not-found 404"
        );
        // A live-endpoint 404 is the OTHER failure mode — distinct, greys the chip differently.
        let not_found = LocusInteropError::NotFound { id: "WP-9".into() };
        assert!(not_found.is_record_not_found());
        assert!(
            !not_found.is_read_api_unavailable(),
            "a 404 is NOT the typed blocker (RISK-003)"
        );
        // The blocker names the probed endpoint (for the chip tooltip + the validator).
        assert_eq!(
            blocker.unavailable_endpoint(),
            Some("/workspaces/WS-1/locus/work-packets/WP-KERNEL-012")
        );
        assert!(blocker
            .unavailable_tooltip()
            .contains("/locus/work-packets/WP-KERNEL-012"));
        assert!(blocker.unavailable_tooltip().contains("not exposed"));
    }

    // ── Read path encoding (RISK-010) ────────────────────────────────────────────────────────────────

    #[test]
    fn resolve_path_percent_encodes_wp_mt_ids() {
        // RISK-010/MC-010: a WP id with hyphens + uppercase segments embeds in the URL correctly-encoded.
        let wp = parse_locus_ref("locus://wp/WP-KERNEL-012").unwrap();
        let path = LocusInteropService::resolve_path("WS-1", &wp);
        // The hyphen is an unreserved URL char (it survives), the segments are the documented route shape.
        assert_eq!(path, "/workspaces/WS-1/locus/work-packets/WP-KERNEL-012");
        let mt = parse_locus_ref("locus://mt/MT-034").unwrap();
        assert_eq!(
            LocusInteropService::resolve_path("WS-1", &mt),
            "/workspaces/WS-1/locus/microtasks/MT-034"
        );
        // An id containing a reserved char (defensive — a malformed id) IS percent-encoded so it can never
        // break routing (a `/` in an id would otherwise inject a path segment).
        let weird = LocusRef {
            kind: LocusRefKind::WorkPacket,
            raw: "locus://wp/A/B".into(),
            id: "A/B".into(),
            normalized: "locus://wp/a/b".into(),
        };
        assert_eq!(
            LocusInteropService::resolve_path("WS-1", &weird),
            "/workspaces/WS-1/locus/work-packets/A%2FB",
            "a reserved char in the id is percent-encoded (no path injection)"
        );
    }

    #[test]
    fn read_resource_segment_per_kind() {
        assert_eq!(LocusRefKind::WorkPacket.read_resource(), "work-packets");
        assert_eq!(LocusRefKind::Microtask.read_resource(), "microtasks");
        assert_eq!(LocusRefKind::WorkPacket.as_str(), "wp");
        assert_eq!(LocusRefKind::Microtask.as_str(), "mt");
    }

    // ── DocumentRef mirrors NoteRef (RISK-002) ───────────────────────────────────────────────────────

    #[test]
    fn document_ref_mirrors_note_ref_shape() {
        let note = NoteRef {
            block_id: "BLK-1".into(),
            document_id: "DOC-1".into(),
            document_title: "Design".into(),
            excerpt: "see locus://mt/MT-034 here".into(),
        };
        let d = DocumentRef::from_note_ref(note);
        assert_eq!(d.document_id, "DOC-1");
        assert_eq!(d.document_title, "Design");
        assert_eq!(d.block_id.as_deref(), Some("BLK-1"));
        assert_eq!(d.excerpt, "see locus://mt/MT-034 here");
    }

    // ── Wire decode + record projection ──────────────────────────────────────────────────────────────

    #[test]
    fn locus_record_wire_projects_with_ref_identity() {
        // The kind + id come from the LocusRef the read resolved (the request authority); the
        // title/summary/status from the body. An empty summary/status normalizes to None.
        let wire: LocusRecordWire = serde_json::from_value(serde_json::json!({
            "title": "Native Editors",
            "summary": "Rebuild the editors natively",
            "status": "Ready for Dev"
        }))
        .unwrap();
        let r = parse_locus_ref("locus://wp/WP-KERNEL-012").unwrap();
        let rec = wire.into_record(&r);
        assert_eq!(rec.kind, LocusRefKind::WorkPacket);
        assert_eq!(rec.id, "WP-KERNEL-012");
        assert_eq!(rec.title, "Native Editors");
        assert_eq!(rec.summary.as_deref(), Some("Rebuild the editors natively"));
        assert_eq!(rec.status.as_deref(), Some("Ready for Dev"));

        // A body with a blank summary -> None (honestly absent, not an empty string).
        let blank: LocusRecordWire =
            serde_json::from_value(serde_json::json!({ "title": "T", "summary": "  " })).unwrap();
        let rec2 = blank.into_record(&r);
        assert_eq!(rec2.summary, None, "a blank summary is honestly absent");
        assert_eq!(rec2.status, None, "an absent status stays None");
    }

    #[test]
    fn cross_ref_error_maps_to_locus_error() {
        // A reused MT-034 search NoWorkspace maps to the Locus NoWorkspace; any other maps to ReverseLookup.
        assert_eq!(
            LocusInteropError::from(CrossRefError::NoWorkspace),
            LocusInteropError::NoWorkspace
        );
        assert!(matches!(
            LocusInteropError::from(CrossRefError::Backend("down".into())),
            LocusInteropError::ReverseLookup(_)
        ));
    }
}
