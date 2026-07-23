//! FEMS retrieval-capsule (MemoryPack) read client (WP-KERNEL-012 MT-063, cluster E9 — FEMS interop).
//!
//! ## What this is (READ-ONLY consumption of the Pillar 12 typed-memory capsule)
//!
//! This module defines the typed Rust client + deserialized model for the FEMS retrieval capsule
//! ([`MemoryPack`]) that Pillar 12 (FEMS — Fast Episodic Memory System) produces. The native editors
//! CONSUME this capsule inline so relevant project memory surfaces where a model or operator is editing
//! a document. This MT is **read-only consumption** — there is NO write/POST/PUT/DELETE to any memory
//! endpoint and NO direct store access (RISK-008/MC-007, AC-006). PostgreSQL/EventLedger is the only
//! durable backing store on the backend side; this client never touches it directly — it only consumes
//! the read API.
//!
//! ## The Pillar 12 MemoryPack contract this client models
//!
//! A [`MemoryPack`] is a retrieval capsule bounded to **<=500 tokens total** (advisory metadata the
//! client SURFACES, never recomputes) and **<=24 items** (a HARD cap the client enforces DEFENSIVELY
//! after decode — RISK-002/MC-001, AC-002). Each [`MemoryItem`] is one of three [`MemoryKind`]s —
//! `Episodic` (what happened), `Semantic` (durable facts), `Procedural` (how-to steps) — and is
//! **provenance-first**: it carries a human summary AND a machine [`MemorySource`] reference (a
//! `loom://`/`atelier://` URI, a document id + byte range, or an event id) the navigation bus can
//! resolve to a concrete editor target. An item with NO resolvable source still renders, but its source
//! link is disabled (RISK-003/MC-003) — see [`MemorySource::validate`] / [`MemoryItem::is_navigable`].
//!
//! ## The FEMS read endpoint EXISTS in this handshake_core build (MT-109 shipped it)
//!
//! The contract names `GET /workspaces/{workspace_id}/memory/pack?context=...` as the FEMS read route.
//! MT-109 SHIPPED that route (`src/backend/handshake_core/src/api/memory.rs`): it returns the REAL
//! `ace::MemoryPack` JSON and, when no pack is stored yet, a well-formed EMPTY pack (200) — NOT a 404 —
//! so an empty capsule is never mistaken for a missing route. This client is therefore modeled to DECODE
//! the real `ace::MemoryPack` item shape directly. [`MemoryClientError::EndpointMissing`] is retained
//! ONLY as a genuine route-absent 404 fallback (an unrouted build or a mis-configured base URL); it is
//! NOT "the designed primary path". A structured backend `{"error":"not_found"}` response is a
//! resource/state error and remains [`MemoryClientError::Http`] rather than being misreported as a
//! missing product capability. On an unstructured route-absent 404
//! [`MemoryClient::fetch_pack`] returns that typed blocker (never a panic, never a silent no-op —
//! RISK-001, RISK-005/MC-002, AC-005), the panel renders a calm banner, and the blocker is surfaced
//! upward so the WP validator sees it. The end-to-end fetch against the LIVE managed-PG MT-109 route is
//! proven separately (`NEEDS_MANAGED_RESOURCE_PROOF`; no backend is started here).
//!
//! ## This model is ALIGNED to the real backend shape (`ace::MemoryPack`)
//!
//! [`RawMemoryItem`] decodes the exact fields the backend emits: `memory_id` -> the client
//! [`MemoryItem::id`] (the FEMS item corpus emits the same canonical pointer under `item_id`, accepted as
//! a serde alias), the free-form `memory_class` -> one of the three rendered [`MemoryKind`]s (any
//! other/future class — e.g. the builder's `"working"` — is TOLERATED: skipped with a logged warning,
//! never a whole-capsule decode failure, see [`deserialize_tolerant_items`]), the human `summary`, and
//! the provenance array `source_refs: Vec<FemsSourceRef>` resolved into a navigable [`MemorySource`] (the
//! first non-empty canonical pointer `id` becomes the nav `uri`) so "Go to source" links are LIVE for
//! real items. The pack-level required `token_estimate` (`u32`) surfaces as the advisory <=500-token
//! budget signal (never recomputed). The legacy self-shaped `id`/`kind`/`source{...}` form is STILL
//! accepted (serde aliases on the id/kind fields + a fallback single-`source` object) so the widget /
//! transport / clamp / AccessKit fixtures keep proving those contracts unchanged.
//!
//! ## Proof posture: golden backend-shape decode + mock transport; live managed-PG fetch is separate
//!
//! The decode tests in `tests/test_relevant_memory.rs` include a GOLDEN decode of the backend's own FEMS
//! item serialization (`tests/fixtures/memory_capsule_e2e/sample_fems_items.json`) through THIS
//! deserializer, proving the client decodes the real `ace::MemoryPack` item shape with LIVE provenance —
//! the old `id`/`kind`/`source` model could NOT (it fails with `missing field id`). The in-process mock
//! `TcpListener` proves transport + the 404 typed-blocker fallback + the defensive <=24 clamp + the
//! render/AccessKit contract. The end-to-end fetch against the running managed-PG MT-109 route stays
//! `NEEDS_MANAGED_RESOURCE_PROOF` (batched separately).
//!
//! ## Reuse, no second HTTP stack (RISK-006/MC-005)
//!
//! [`MemoryClient`] holds a cloned [`reqwest::Client`] (the process-wide
//! [`crate::backend_client::shared_http_client`] pool) + the config-resolved
//! [`crate::backend_client::BACKEND_BASE_URL`] — exactly the pattern
//! [`crate::backend::knowledge_documents::KnowledgeDocumentsClient`] established. NO new reqwest stack,
//! NO new async runtime. The read identity headers reuse the shared `x-hsk-*` header constants.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::backend_client::{
    shared_http_client, BACKEND_BASE_URL, HSK_HEADER_ACTOR_ID, HSK_HEADER_KERNEL_TASK_RUN_ID,
    HSK_HEADER_SESSION_RUN_ID,
};

/// The Pillar 12 hard cap on items in one retrieval capsule. The client enforces this DEFENSIVELY after
/// decode regardless of what the server returns (RISK-002/MC-001, AC-002).
pub const MEMORY_PACK_MAX_ITEMS: usize = 24;

/// The Pillar 12 advisory token budget for one retrieval capsule. The client SURFACES this as metadata
/// (and surfaces an over-budget signal) but NEVER recomputes token estimates — the budget is the
/// server's authority (per the contract: "treat the token budget as advisory metadata it surfaces").
pub const MEMORY_PACK_TOKEN_BUDGET: u32 = 500;
pub const MEMORY_PACK_SCHEMA_VERSION: &str = "hsk.memory_pack@0.1";

/// Read timeout for a single capsule fetch. A bounded timeout so a hung backend cannot stall the editor
/// frame loop (the fetch runs off the render path on the shared async runtime).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

/// The least-privileged read-only actor id used for the FEMS capsule read. A missing `x-hsk-actor-kind`
/// header is the least-privileged read-only actor server-side (the same least-privilege default the
/// knowledge-documents read path uses), so no write-capable actor-kind is ever attached on this path.
const FEMS_READ_ACTOR_ID: &str = "native-editor-fems-reader";
const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The Pillar 12 MemoryPack model (provenance-first, 3 kinds, <=24 items, <=500 token advisory).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The three Pillar 12 memory kinds. Serialized lowercase on the wire (`"episodic"` | `"semantic"` |
/// `"procedural"`) so the typed enum round-trips the FEMS capsule JSON exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    /// What happened: events, prior sessions, edits.
    Episodic,
    /// Durable facts / concepts.
    Semantic,
    /// How-to steps, recipes, workflows.
    Procedural,
}

impl MemoryKind {
    /// A short human/agent-readable badge for the kind (rendered as the item's kind badge).
    pub fn badge(&self) -> &'static str {
        match self {
            MemoryKind::Episodic => "EP",
            MemoryKind::Semantic => "SEM",
            MemoryKind::Procedural => "PROC",
        }
    }

    /// The section header label for a group of items of this kind.
    pub fn section_label(&self) -> &'static str {
        match self {
            MemoryKind::Episodic => "Episodic",
            MemoryKind::Semantic => "Semantic",
            MemoryKind::Procedural => "Procedural",
        }
    }

    /// Stable wire string for the kind (mirrors the serde lowercase representation). Used for the
    /// AccessKit value + the section iteration order.
    pub fn wire(&self) -> &'static str {
        match self {
            MemoryKind::Episodic => "episodic",
            MemoryKind::Semantic => "semantic",
            MemoryKind::Procedural => "procedural",
        }
    }

    /// Map a wire `memory_class`/`kind` string to one of the three rendered kinds, or `None` for any
    /// other class. The REAL backend ([`crate::ace::MemoryPack`]) emits `memory_class` as a free-form
    /// String and the builder already emits a 4th class (`"working"`); future classes are expected.
    /// Returning `None` (instead of a hard serde failure) is what lets a single unknown-class item be
    /// SKIPPED rather than aborting the entire capsule decode (see [`MemoryPack`]'s custom items
    /// deserializer). This is the must_fix #2 "tolerate the `working` class (and any future class)"
    /// floor; the panel only renders the three known kinds via [`Self::ORDER`].
    pub fn from_wire(s: &str) -> Option<MemoryKind> {
        match s {
            "episodic" => Some(MemoryKind::Episodic),
            "semantic" => Some(MemoryKind::Semantic),
            "procedural" => Some(MemoryKind::Procedural),
            _ => None,
        }
    }

    /// The three kinds in their fixed render order (Episodic, Semantic, Procedural).
    pub const ORDER: [MemoryKind; 3] = [
        MemoryKind::Episodic,
        MemoryKind::Semantic,
        MemoryKind::Procedural,
    ];
}

/// The provenance reference an item carries so the navigation bus can resolve it to a concrete editor
/// target. Provenance-first means AT LEAST ONE field must be present for an item to be navigable; an
/// item with all fields absent renders but its source link is disabled (RISK-003/MC-003). The
/// navigation precedence is: prefer [`Self::uri`], else [`Self::document_id`] + [`Self::byte_range`],
/// else [`Self::event_id`] (see [`Self::nav_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct MemorySource {
    /// A resolvable URI (`loom://...`, `atelier://...`). The highest-precedence navigation target.
    #[serde(default)]
    pub uri: Option<String>,
    /// A document id (paired with [`Self::byte_range`] to point at a span inside the document).
    #[serde(default)]
    pub document_id: Option<String>,
    /// A `(start, end)` byte range inside [`Self::document_id`]. Only meaningful with a `document_id`.
    #[serde(default)]
    pub byte_range: Option<(usize, usize)>,
    /// An event id (an EventLedger / Flight Recorder event the item derives from).
    #[serde(default)]
    pub event_id: Option<String>,
}

impl MemorySource {
    /// True when this source has at least one resolvable field (so the item is navigable). An item whose
    /// source fails this check renders with a DISABLED source link rather than a dead/clickable one
    /// (RISK-003/MC-003).
    pub fn validate(&self) -> bool {
        self.uri.is_some() || self.document_id.is_some() || self.event_id.is_some()
    }
}

/// One item in the retrieval capsule: a provenance-first memory atom of one [`MemoryKind`].
///
/// NOTE on decode tolerance (must_fix #2): the item's `kind` is decoded leniently through the raw
/// wire string (see [`RawMemoryItem`] + [`MemoryPack`]'s custom items deserializer), so a capsule item
/// carrying a class outside the three rendered kinds (e.g. the backend builder's `"working"` class, or
/// any future class) is SKIPPED with a logged warning rather than hard-failing the whole capsule
/// decode. A `MemoryItem` value therefore always carries one of the three known [`MemoryKind`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryItem {
    /// A stable id for the item (used as the AccessKit address suffix `mem-item-{id}` and to key the
    /// per-item source link). Item ids must be unique within a pack (the panel dedups defensively so a
    /// duplicate id cannot collide AccessKit addresses — RISK-007/MC-006).
    pub id: String,
    /// Which of the three Pillar 12 kinds this item is.
    pub kind: MemoryKind,
    /// The human/agent-readable one-line summary (always rendered, provenance-first).
    pub summary: String,
    /// The machine provenance reference (may be non-navigable — see [`MemorySource::validate`]).
    pub source: MemorySource,
    /// The retrieval relevance score, if the server supplied one (advisory; rendered subtly, never
    /// recomputed client-side).
    pub score: Option<f32>,
}

impl MemoryItem {
    /// True when the item's source resolves to a navigable target (delegates to
    /// [`MemorySource::validate`]).
    pub fn is_navigable(&self) -> bool {
        self.source.validate()
    }
}

/// The raw wire form of one capsule item, used ONLY for tolerant decode. It is modeled on the REAL
/// backend `ace::MemoryPackItem`: the id field decodes `memory_id` (the FEMS item corpus emits the same
/// pointer under `item_id`, and the legacy self-shaped fixtures use `id` — both accepted as serde
/// aliases); the kind field decodes the free-form `memory_class` (legacy `kind` accepted as an alias) so
/// an unknown/future class does NOT fail the serde decode of the whole capsule; provenance decodes the
/// backend's `source_refs: Vec<FemsSourceRef>` array AND the legacy single `source` object. The
/// conversion to a typed [`MemoryItem`] in [`MemoryPack`]'s items deserializer drops (and logs) any item
/// whose class is not one of the three rendered [`MemoryKind`]s (must_fix #2).
#[derive(Debug, Clone, Deserialize)]
struct RawMemoryItem {
    /// The item id. Primary wire key is the backend's `memory_id`; `item_id` (the FEMS item corpus) and
    /// `id` (legacy self-shaped fixtures) are accepted as aliases so both the real pack and the widget
    /// fixtures decode into the same client id.
    #[serde(rename = "memory_id", alias = "item_id", alias = "id")]
    id: String,
    /// The memory class. Primary wire key is the backend's free-form `memory_class`; the legacy `kind`
    /// key is accepted as an alias. Mapped to a rendered [`MemoryKind`] (or skipped) in [`Self::into_typed`].
    #[serde(rename = "memory_class", alias = "kind")]
    kind: String,
    summary: String,
    /// The legacy single provenance object (self-shaped fixtures/tests). Used verbatim when it is
    /// navigable; otherwise provenance is resolved from [`Self::source_refs`].
    #[serde(default)]
    source: MemorySource,
    /// The REAL backend provenance array (`ace::FemsSourceRef`). Resolved into a navigable
    /// [`MemorySource`] when the legacy `source` object is absent/non-navigable.
    #[serde(default)]
    source_refs: Vec<RawSourceRef>,
    /// Advisory retrieval score (present in the FEMS item corpus; the real `ace::MemoryPackItem` has no
    /// per-item score, so `None` there).
    #[serde(default)]
    score: Option<f32>,
}

/// The raw wire form of one backend provenance pointer (`ace::FemsSourceRef`). Only the canonical `id`
/// pointer is needed to build a nav target; the other FemsSourceRef fields (`kind`/`hash`/`selector`/
/// `created_at`/`classification`) are carried by the backend but ignored here (serde skips unknown
/// fields). `id` is defaulted so a malformed/empty ref degrades to non-navigable rather than failing the
/// whole capsule decode.
#[derive(Debug, Clone, Deserialize)]
struct RawSourceRef {
    #[serde(default)]
    id: String,
}

impl RawMemoryItem {
    /// Convert to a typed [`MemoryItem`], or `None` if the wire `memory_class` is not one of the three
    /// rendered kinds (an unknown/future class — e.g. the backend's `"working"`). `None` items are
    /// skipped + logged by the caller rather than failing the whole capsule decode. Provenance is
    /// resolved provenance-first: the legacy single `source` object wins when navigable, otherwise the
    /// backend `source_refs` array is resolved (first non-empty canonical pointer id -> nav `uri`).
    fn into_typed(self) -> Option<MemoryItem> {
        let kind = MemoryKind::from_wire(&self.kind)?;
        let source = if self.source.validate() {
            self.source
        } else {
            resolve_source_from_refs(&self.source_refs)
        };
        Some(MemoryItem {
            id: self.id,
            kind,
            summary: self.summary,
            source,
            score: self.score,
        })
    }
}

/// Resolve a navigable [`MemorySource`] from the backend's `source_refs` array (`ace::FemsSourceRef`).
/// The FemsSourceRef `id` is the canonical provenance pointer (a resolvable string such as `loom://...`,
/// `hbr://...`, `packet://...`, or a doc-block id); the FIRST ref with a non-empty id becomes the nav
/// `uri` (the highest-precedence `MemoryNavTarget` the panel resolves), making "Go to source" LIVE for
/// real items. An empty array (or all-empty ids) yields a non-navigable source -> the row renders a
/// DISABLED source link (RISK-003/MC-003), never a dead/clickable one.
fn resolve_source_from_refs(refs: &[RawSourceRef]) -> MemorySource {
    for r in refs {
        let id = r.id.trim();
        if !id.is_empty() {
            return MemorySource {
                uri: Some(id.to_string()),
                ..Default::default()
            };
        }
    }
    MemorySource::default()
}

/// The deserialized retrieval capsule. This decodes the REAL backend `ace::MemoryPack` JSON. The client
/// retains and validates `schema_version`, `pack_id`, and `memory_pack_hash` before tolerant item decode;
/// presentation-irrelevant fields (`generated_at`/`determinism_mode`/`memory_policy`/`scope_refs`/
/// `budgets`/`warnings`) remain ignored. `token_estimate` is ADVISORY metadata surfaced by the client
/// (never recomputed); `truncated`
/// is `true` if the client clamped the item list to [`MEMORY_PACK_MAX_ITEMS`] after decode (or if the
/// server already marked it truncated). `truncated`/`context_key` are client-only bookkeeping the
/// backend pack does not carry (they default).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MemoryPack {
    /// Canonical FEMS pack schema. Live HTTP responses must equal
    /// [`MEMORY_PACK_SCHEMA_VERSION`]; legacy direct fixtures may omit it.
    #[serde(default)]
    pub schema_version: String,
    /// Stable authoritative pack identity. Live HTTP responses must provide a non-empty id.
    #[serde(default)]
    pub pack_id: String,
    /// The capsule items (already clamped to <=24 by [`MemoryClient::fetch_pack`]). Decoded via
    /// [`deserialize_tolerant_items`]: an item whose `memory_class` is outside the three rendered kinds
    /// (an unknown/future class such as the backend builder's `"working"`) is SKIPPED with a logged
    /// warning rather than aborting the whole capsule decode (must_fix #2).
    #[serde(default, deserialize_with = "deserialize_tolerant_items")]
    pub items: Vec<MemoryItem>,
    /// The advisory total token estimate (<=500 by the Pillar 12 budget). The backend `ace::MemoryPack`
    /// emits this as a REQUIRED `u32`, which decodes here as `Some(u32)`; the legacy fixtures / empty
    /// packs that omit it decode as `None`. Surfaced as metadata; the client never recomputes it.
    #[serde(default)]
    pub token_estimate: Option<u32>,
    /// True if the item list was truncated (by the server OR the client's defensive clamp).
    #[serde(default)]
    pub truncated: bool,
    /// The context key the server keyed this capsule on (echoes the request context so a stale response
    /// can be detected). Defaults to empty if the server omits it.
    #[serde(default)]
    pub context_key: String,
    /// SHA-256 of the complete canonical backend envelope with this field replaced by JSON null.
    #[serde(default)]
    pub memory_pack_hash: String,
}

impl MemoryPack {
    /// An empty pack (no items), used for the neutral "no relevant memory" render state.
    pub fn empty(context_key: impl Into<String>) -> Self {
        Self {
            schema_version: String::new(),
            pack_id: String::new(),
            items: Vec::new(),
            token_estimate: None,
            truncated: false,
            context_key: context_key.into(),
            memory_pack_hash: String::new(),
        }
    }

    /// The items of one [`MemoryKind`], in their original order. Used by the panel to render grouped
    /// sections (Episodic / Semantic / Procedural).
    pub fn items_of_kind(&self, kind: MemoryKind) -> impl Iterator<Item = &MemoryItem> {
        self.items.iter().filter(move |i| i.kind == kind)
    }

    /// True if the advisory `token_estimate` exceeds the Pillar 12 budget. Surfaced to the operator as a
    /// subtle over-budget signal; the client does NOT recompute or alter the estimate (advisory only).
    pub fn over_token_budget(&self) -> bool {
        self.token_estimate
            .map(|t| t > MEMORY_PACK_TOKEN_BUDGET)
            .unwrap_or(false)
    }
}

/// Tolerantly deserialize the capsule item list (must_fix #2). Each item is decoded with `kind` as a
/// free-form wire String (matching the real backend's free-form `memory_class`); an item whose class is
/// not one of the three rendered [`MemoryKind`]s (an unknown or future class, e.g. the backend builder's
/// `"working"`) is SKIPPED with a logged warning rather than aborting the WHOLE capsule decode. This is
/// the defensive posture the review requires: one unknown-class item must not zero out all relevant
/// memory. Items that DO map to a known kind are preserved in order.
fn deserialize_tolerant_items<'de, D>(deserializer: D) -> Result<Vec<MemoryItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Vec<RawMemoryItem> = Vec::deserialize(deserializer)?;
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        let raw_kind = item.kind.clone();
        let raw_id = item.id.clone();
        match item.into_typed() {
            Some(typed) => out.push(typed),
            None => {
                tracing::warn!(
                    item_id = %raw_id,
                    memory_class = %raw_kind,
                    "MT-063 FEMS capsule item dropped: unknown memory_class '{raw_kind}' (not episodic/semantic/procedural) — item skipped, capsule decode continues"
                );
            }
        }
    }
    Ok(out)
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The request context built from the active editor focus + shared selection.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The editing context a capsule is keyed on, built from the active editor pane's focus + the MT-031
/// [`crate::interop::SharedSelection`]. Serialized into the read query (`document_id`, `selection_text`,
/// `cursor_byte`); the workspace id is the path parameter. Comparing two contexts (it is `PartialEq`)
/// drives the panel's debounce (skip a refresh when the context is unchanged — RISK-004/MC-004).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryContext {
    /// The workspace whose memory is being retrieved (the path parameter — always required).
    pub workspace_id: String,
    /// The active document id, if a document surface holds focus.
    pub document_id: Option<String>,
    /// The current selection text, if a span is selected (bounded by the caller — the bus materializes
    /// the selected string already).
    pub selection_text: Option<String>,
    /// The caret byte offset inside the active document, if known.
    pub cursor_byte: Option<usize>,
}

impl MemoryContext {
    /// Build a context for a workspace with no document focus (the bare-workspace capsule).
    pub fn for_workspace(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            ..Self::default()
        }
    }

    /// Build the request from the active document focus + the shared selection. The selection text is
    /// bounded to a sane length so a huge selection cannot bloat the query string; `cursor_byte` is the
    /// caret offset. This is a pure mapping (no IO) so it is unit-provable.
    pub fn from_focus(
        workspace_id: impl Into<String>,
        document_id: Option<String>,
        selection_text: Option<String>,
        cursor_byte: Option<usize>,
    ) -> Self {
        const MAX_SELECTION_QUERY_LEN: usize = 512;
        let selection_text = selection_text.map(|s| {
            if s.chars().count() > MAX_SELECTION_QUERY_LEN {
                s.chars().take(MAX_SELECTION_QUERY_LEN).collect()
            } else {
                s
            }
        });
        Self {
            workspace_id: workspace_id.into(),
            document_id,
            selection_text,
            cursor_byte,
        }
    }

    /// A stable, human-readable key for this context (used to detect a stale response and as the
    /// debounce comparison anchor). Pure; no IO.
    pub fn context_key(&self) -> String {
        format!(
            "ws={}|doc={}|cur={}|sel_len={}",
            self.workspace_id,
            self.document_id.as_deref().unwrap_or("-"),
            self.cursor_byte
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            self.selection_text.as_ref().map(|s| s.len()).unwrap_or(0),
        )
    }

    /// The query-parameter pairs for the read request (`document_id`, `selection_text`, `cursor_byte`),
    /// each present only when its field is `Some`. Pure; the client appends these to the GET request.
    fn query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs: Vec<(&'static str, String)> = Vec::new();
        // The context key is always sent so the server can echo it back (stale-response detection).
        pairs.push(("context", self.context_key()));
        if let Some(doc) = &self.document_id {
            pairs.push(("document_id", doc.clone()));
        }
        if let Some(sel) = &self.selection_text {
            pairs.push(("selection_text", sel.clone()));
        }
        if let Some(cur) = self.cursor_byte {
            pairs.push(("cursor_byte", cur.to_string()));
        }
        pairs
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error — EndpointMissing is the first-class TYPED BLOCKER variant.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed outcome of a [`MemoryClient::fetch_pack`] call.
///
/// [`Self::EndpointMissing`] is the FIRST-CLASS TYPED BLOCKER (RISK-005/MC-002, AC-005): it is returned
/// when the FEMS read route is absent (a 404 on the documented path, or a feature-not-present sentinel).
/// It is NEVER an error to swallow — the panel maps it to a visible empty-state banner AND surfaces it
/// upward to the WP validator. The other variants are ordinary HTTP/transport/decode failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryClientError {
    /// The FEMS read route is absent in this handshake_core build (404 / feature-absent). The TYPED
    /// BLOCKER. Carries the path probed so the validator sees exactly which route is missing.
    EndpointMissing { probed_path: String },
    /// A non-404 HTTP error status from the read route. Carries the numeric status (a `StatusCode`
    /// equivalent — kept as `u16` so the variant does not couple callers to the reqwest type) + body.
    Http { status: u16, body: String },
    /// The response body could not be decoded into a [`MemoryPack`].
    Decode(String),
    /// A transport failure (connect / timeout / TLS) — the request never reached a status.
    Transport(String),
    /// The server returned more than [`MEMORY_PACK_MAX_ITEMS`] items. Informational: `fetch_pack` does
    /// NOT fail on this — it CLAMPS and sets `truncated=true` (AC-002) and the over-cap count is logged.
    /// This variant exists so a caller that wants to assert the over-cap condition can, but the normal
    /// path returns a clamped `Ok(MemoryPack)`, never this error.
    OverCap { returned: usize },
}

impl std::fmt::Display for MemoryClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndpointMissing { probed_path } => write!(
                f,
                "FEMS read endpoint not present in this build (probed {probed_path})"
            ),
            Self::Http { status, body } => write!(f, "FEMS read HTTP {status}: {body}"),
            Self::Decode(e) => write!(f, "FEMS capsule decode error: {e}"),
            Self::Transport(e) => write!(f, "FEMS read transport error: {e}"),
            Self::OverCap { returned } => {
                write!(f, "FEMS capsule over cap: server returned {returned} items")
            }
        }
    }
}

impl std::error::Error for MemoryClientError {}

impl MemoryClientError {
    /// True when this is the typed-blocker variant (the panel renders the empty-state banner and the
    /// blocker is surfaced to the WP validator).
    pub fn is_endpoint_missing(&self) -> bool {
        matches!(self, MemoryClientError::EndpointMissing { .. })
    }
}

/// A typed result alias for the memory client.
pub type MemoryResult<T> = Result<T, MemoryClientError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The read client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The stateless typed read client for the FEMS retrieval-capsule route. Holds ONLY a shared
/// [`reqwest::Client`] (the process-wide [`crate::backend_client::shared_http_client`] pool — NO second
/// HTTP stack, RISK-006/MC-005) + the config-resolved base URL — exactly the
/// [`crate::backend::knowledge_documents::KnowledgeDocumentsClient`] pattern. READ-ONLY: it only ever
/// issues a GET (RISK-008/MC-007, AC-006).
#[derive(Clone)]
pub struct MemoryClient {
    client: reqwest::Client,
    base_url: String,
    session_run_id: String,
    session_token: Option<String>,
}

impl Default for MemoryClient {
    fn default() -> Self {
        Self::production()
    }
}

impl MemoryClient {
    /// Construct against the production backend base URL (the same config-resolved
    /// [`crate::backend_client::BACKEND_BASE_URL`] every native client uses — not hardcoded here),
    /// sharing the ONE process-wide [`crate::backend_client::shared_http_client`] connection pool.
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool). The base URL is the authority for the host — never hardcoded at a call
    /// site (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_run_id: "native-editor-session".to_owned(),
            session_token: None,
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (the WP-011 backend client's pool) so the app
    /// shares ONE connection pool rather than minting a second HTTP stack (RISK-006/MC-005).
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            session_run_id: "native-editor-session".to_owned(),
            session_token: None,
        }
    }

    /// Override the session run id attached to the read identity headers (so swarm/operator co-work is
    /// attributable). Returns `self` for builder-style chaining.
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
        self
    }

    /// Bind requests to the live native MCP session. Production callers must set this; tests may
    /// leave it absent when the mock transport intentionally does not enforce authentication.
    pub fn with_session_token(mut self, session_token: impl Into<String>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// The capsule read path for a workspace (the documented FEMS read route). Built here so the
    /// `EndpointMissing` blocker can report the exact probed path.
    pub fn pack_path(workspace_id: &str) -> String {
        format!("/workspaces/{workspace_id}/memory/pack")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Fetch the retrieval capsule for `workspace_id` keyed on `context`. READ-ONLY: this issues a
    /// single GET and never a write verb (RISK-008/MC-007, AC-006).
    ///
    /// Behavior contract:
    /// - An unstructured 404 maps to [`MemoryClientError::EndpointMissing`] — the TYPED BLOCKER, never a
    ///   panic or silent no-op (RISK-001, RISK-005/MC-002, AC-005). A structured backend
    ///   `{"error":"not_found"}` 404 remains [`MemoryClientError::Http`] so an unknown/deleted workspace
    ///   is never confused with an unrouted product feature.
    /// - A success body is decoded into a [`MemoryPack`], then the item list is DEFENSIVELY CLAMPED to
    ///   [`MEMORY_PACK_MAX_ITEMS`] regardless of what the server returned (truncate + `truncated=true` +
    ///   a logged warning — RISK-002/MC-001, AC-002).
    /// - Other non-success statuses map to [`MemoryClientError::Http`]; transport failures to
    ///   [`MemoryClientError::Transport`]; decode failures to [`MemoryClientError::Decode`].
    pub async fn fetch_pack(
        &self,
        workspace_id: &str,
        context: &MemoryContext,
    ) -> MemoryResult<MemoryPack> {
        let path = Self::pack_path(workspace_id);
        let url = self.url(&path);
        let mut builder = self
            .client
            .get(&url)
            .query(&context.query_pairs())
            .timeout(REQUEST_TIMEOUT)
            // READ identity: the least-privileged read-only actor (no x-hsk-actor-kind => read-only
            // server-side). NEVER a write-capable actor-kind on this read path.
            .header(HSK_HEADER_ACTOR_ID, FEMS_READ_ACTOR_ID)
            .header(
                HSK_HEADER_KERNEL_TASK_RUN_ID,
                format!("native-editor-fems-{workspace_id}"),
            )
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id);
        if let Some(session_token) = &self.session_token {
            builder = builder.header(HSK_HEADER_SESSION_TOKEN, session_token);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| MemoryClientError::Transport(e.to_string()))?;
        let status = resp.status();

        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::NOT_FOUND {
                let canonical_resource_missing = serde_json::from_str::<JsonValue>(&body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("error")
                            .and_then(JsonValue::as_str)
                            .map(str::to_owned)
                    })
                    .is_some_and(|error| error == "not_found");
                if !canonical_resource_missing {
                    return Err(MemoryClientError::EndpointMissing { probed_path: path });
                }
            }
            return Err(MemoryClientError::Http { status: code, body });
        }

        let body = resp
            .bytes()
            .await
            .map_err(|e| MemoryClientError::Transport(e.to_string()))?;
        let envelope: JsonValue =
            serde_json::from_slice(&body).map_err(|e| MemoryClientError::Decode(e.to_string()))?;
        validate_authoritative_memory_pack(&envelope)?;
        let mut pack: MemoryPack = serde_json::from_value(envelope)
            .map_err(|e| MemoryClientError::Decode(e.to_string()))?;

        // DEFENSIVE CLAMP (RISK-002/MC-001, AC-002): enforce the <=24 cap client-side regardless of
        // server behavior. If the server returned more, truncate, mark truncated, and log a warning.
        clamp_pack_items(&mut pack);
        Ok(pack)
    }
}

fn validate_authoritative_memory_pack(envelope: &JsonValue) -> MemoryResult<()> {
    let object = envelope
        .as_object()
        .ok_or_else(|| MemoryClientError::Decode("MemoryPack envelope must be an object".into()))?;
    let schema_version = object
        .get("schema_version")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| MemoryClientError::Decode("MemoryPack schema_version is required".into()))?;
    if schema_version != MEMORY_PACK_SCHEMA_VERSION {
        return Err(MemoryClientError::Decode(format!(
            "unsupported MemoryPack schema_version {schema_version:?}; expected {MEMORY_PACK_SCHEMA_VERSION}"
        )));
    }
    let pack_id = object
        .get("pack_id")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or_else(|| MemoryClientError::Decode("MemoryPack pack_id is required".into()))?;
    if !pack_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(MemoryClientError::Decode(
            "MemoryPack pack_id contains unsafe characters".into(),
        ));
    }
    let stored_hash = object
        .get("memory_pack_hash")
        .and_then(JsonValue::as_str)
        .filter(|value| {
            value.len() == 64
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
        })
        .ok_or_else(|| {
            MemoryClientError::Decode(
                "MemoryPack memory_pack_hash must be lowercase SHA-256".into(),
            )
        })?;
    let computed_hash = compute_memory_pack_hash(envelope)?;
    if computed_hash != stored_hash {
        return Err(MemoryClientError::Decode(format!(
            "MemoryPack memory_pack_hash mismatch for pack_id {pack_id}"
        )));
    }
    Ok(())
}

/// Compute the canonical backend-compatible MemoryPack hash. The hash field itself is replaced by
/// JSON null, matching `handshake_core::ace::MemoryPack::compute_hash` exactly.
pub fn compute_memory_pack_hash(envelope: &JsonValue) -> MemoryResult<String> {
    let mut hash_envelope = envelope.clone();
    hash_envelope
        .as_object_mut()
        .ok_or_else(|| MemoryClientError::Decode("MemoryPack envelope must be an object".into()))?
        .insert("memory_pack_hash".into(), JsonValue::Null);
    Ok(sha256_hex(&canonical_json_bytes_nfc(&hash_envelope)))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

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

/// Defensively clamp a decoded [`MemoryPack`] to [`MEMORY_PACK_MAX_ITEMS`] items (RISK-002/MC-001,
/// AC-002). If the list was over cap, it is truncated, `truncated` is set `true`, and a warning is
/// logged with the dropped count. Extracted as a pure function so the clamp contract is unit-provable
/// without a live socket. Returns the number of items dropped (0 if already within cap).
pub fn clamp_pack_items(pack: &mut MemoryPack) -> usize {
    if pack.items.len() > MEMORY_PACK_MAX_ITEMS {
        let returned = pack.items.len();
        let dropped = returned - MEMORY_PACK_MAX_ITEMS;
        pack.items.truncate(MEMORY_PACK_MAX_ITEMS);
        pack.truncated = true;
        tracing::warn!(
            returned,
            dropped,
            cap = MEMORY_PACK_MAX_ITEMS,
            context_key = %pack.context_key,
            "MT-063 FEMS MemoryPack over cap: server returned {returned} items, clamped to {MEMORY_PACK_MAX_ITEMS} (dropped {dropped})"
        );
        dropped
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit proofs for the MemoryPack model + the defensive clamp + the typed-error contract that
    //! do NOT need a socket. The live-fetch path (404 -> EndpointMissing, success -> decode+clamp) is
    //! proven against a mock server in `tests/test_relevant_memory.rs`.

    use super::*;
    use serde_json::json;

    fn authoritative_envelope(items: JsonValue, pack_id: &str) -> JsonValue {
        let mut value = json!({
            "schema_version": MEMORY_PACK_SCHEMA_VERSION,
            "pack_id": pack_id,
            "items": items,
            "token_estimate": 0,
            "memory_pack_hash": null
        });
        let hash = compute_memory_pack_hash(&value).expect("canonical pack hash");
        value
            .as_object_mut()
            .expect("pack object")
            .insert("memory_pack_hash".into(), json!(hash));
        value
    }

    #[test]
    fn authoritative_empty_and_full_pack_envelopes_validate() {
        let empty = authoritative_envelope(json!([]), "PACK-empty");
        validate_authoritative_memory_pack(&empty).expect("empty pack envelope is authoritative");

        let full = authoritative_envelope(
            json!([{
                "memory_id": "MEM-1",
                "memory_class": "semantic",
                "summary": "fact",
                "source_refs": []
            }]),
            "PACK-full",
        );
        validate_authoritative_memory_pack(&full).expect("full pack envelope is authoritative");
    }

    #[test]
    fn authoritative_pack_rejects_schema_identity_and_hash_tampering() {
        let valid = authoritative_envelope(json!([]), "PACK-valid");
        for field in ["schema_version", "pack_id", "memory_pack_hash"] {
            let mut changed = valid.clone();
            changed.as_object_mut().expect("pack object").insert(
                field.into(),
                match field {
                    "schema_version" => json!("fems.memory_pack@0.1"),
                    "pack_id" => json!(""),
                    _ => json!("0".repeat(64)),
                },
            );
            assert!(
                validate_authoritative_memory_pack(&changed).is_err(),
                "tampered {field} must fail closed"
            );
        }
    }

    /// AC-001: a fixture with one episodic, one semantic, one procedural item decodes into the typed
    /// MemoryKind enum.
    #[test]
    fn ac001_parses_all_three_kinds() {
        let raw = json!({
            "context_key": "ws=W1|doc=D1",
            "token_estimate": 320,
            "truncated": false,
            "items": [
                {"id": "i1", "kind": "episodic", "summary": "edited intro", "source": {"event_id": "EV-1"}},
                {"id": "i2", "kind": "semantic", "summary": "Aria is the protagonist", "source": {"uri": "loom://block/aria"}},
                {"id": "i3", "kind": "procedural", "summary": "how to render", "source": {"document_id": "D9", "byte_range": [10, 40]}}
            ]
        });
        let pack: MemoryPack = serde_json::from_value(raw).expect("fixture must decode");
        assert_eq!(pack.items.len(), 3);
        assert_eq!(pack.items[0].kind, MemoryKind::Episodic);
        assert_eq!(pack.items[1].kind, MemoryKind::Semantic);
        assert_eq!(pack.items[2].kind, MemoryKind::Procedural);
        assert_eq!(pack.token_estimate, Some(320));
        // The advisory budget is surfaced, not recomputed.
        assert!(!pack.over_token_budget());
    }

    /// AC-002: a 30-item fixture clamps to exactly 24, sets truncated=true, drops 6.
    #[test]
    fn ac002_clamps_over_cap_to_24() {
        let items: Vec<_> = (0..30)
            .map(|n| {
                json!({"id": format!("i{n}"), "kind": "episodic", "summary": format!("item {n}"), "source": {"event_id": format!("EV-{n}")}})
            })
            .collect();
        let raw = json!({"context_key": "k", "truncated": false, "items": items});
        let mut pack: MemoryPack = serde_json::from_value(raw).expect("decode");
        assert_eq!(pack.items.len(), 30, "decoded all 30 before clamp");
        let dropped = clamp_pack_items(&mut pack);
        assert_eq!(pack.items.len(), MEMORY_PACK_MAX_ITEMS);
        assert_eq!(pack.items.len(), 24);
        assert!(
            pack.truncated,
            "AC-002: truncated must be set true after clamp"
        );
        assert_eq!(dropped, 6, "AC-002: 6 items dropped (30 - 24)");
    }

    /// A within-cap pack is untouched by the clamp (truncated stays false, no drops).
    #[test]
    fn within_cap_not_clamped() {
        let items: Vec<_> = (0..10)
            .map(|n| json!({"id": format!("i{n}"), "kind": "semantic", "summary": "x", "source": {"uri": "loom://x"}}))
            .collect();
        let mut pack: MemoryPack =
            serde_json::from_value(json!({"context_key": "k", "items": items})).unwrap();
        let dropped = clamp_pack_items(&mut pack);
        assert_eq!(dropped, 0);
        assert_eq!(pack.items.len(), 10);
        assert!(!pack.truncated);
    }

    /// Provenance precedence + non-navigable handling (RISK-003/MC-003): a source validates iff it has a
    /// uri, document_id, or event_id; an all-absent source is non-navigable.
    #[test]
    fn provenance_validate_and_precedence() {
        let nav_uri = MemorySource {
            uri: Some("loom://b".into()),
            ..Default::default()
        };
        let nav_doc = MemorySource {
            document_id: Some("D".into()),
            byte_range: Some((1, 2)),
            ..Default::default()
        };
        let nav_evt = MemorySource {
            event_id: Some("EV".into()),
            ..Default::default()
        };
        let dead = MemorySource::default();
        assert!(nav_uri.validate());
        assert!(nav_doc.validate());
        assert!(nav_evt.validate());
        assert!(!dead.validate(), "all-absent source must be non-navigable");
    }

    /// over_token_budget surfaces (does not recompute) the advisory budget signal.
    #[test]
    fn over_budget_is_advisory_signal() {
        let mut pack = MemoryPack::empty("k");
        pack.token_estimate = Some(600);
        assert!(pack.over_token_budget(), "600 > 500 budget");
        pack.token_estimate = Some(400);
        assert!(!pack.over_token_budget());
        pack.token_estimate = None;
        assert!(
            !pack.over_token_budget(),
            "absent estimate is not over budget"
        );
    }

    /// EndpointMissing is the typed-blocker variant.
    #[test]
    fn endpoint_missing_is_typed_blocker() {
        let err = MemoryClientError::EndpointMissing {
            probed_path: "/workspaces/W/memory/pack".into(),
        };
        assert!(err.is_endpoint_missing());
        assert!(!MemoryClientError::Decode("x".into()).is_endpoint_missing());
        // The display string names the probed path so the validator sees the exact missing route.
        assert!(err.to_string().contains("/workspaces/W/memory/pack"));
    }

    /// The context maps focus -> query and bounds an oversized selection.
    #[test]
    fn context_from_focus_bounds_selection() {
        let huge = "x".repeat(2000);
        let ctx = MemoryContext::from_focus("W1", Some("D1".into()), Some(huge), Some(42));
        assert_eq!(ctx.workspace_id, "W1");
        assert_eq!(ctx.document_id.as_deref(), Some("D1"));
        assert_eq!(ctx.cursor_byte, Some(42));
        assert!(
            ctx.selection_text.as_ref().unwrap().chars().count() <= 512,
            "selection bounded"
        );
        // The query carries the context + the present fields only.
        let pairs = ctx.query_pairs();
        assert!(pairs.iter().any(|(k, _)| *k == "document_id"));
        assert!(pairs.iter().any(|(k, _)| *k == "cursor_byte"));
        assert!(pairs.iter().any(|(k, _)| *k == "context"));
    }

    /// Two equal contexts compare equal (the debounce anchor); a cursor move makes them differ.
    #[test]
    fn context_equality_drives_debounce() {
        let a = MemoryContext::from_focus("W", Some("D".into()), None, Some(1));
        let b = MemoryContext::from_focus("W", Some("D".into()), None, Some(1));
        let c = MemoryContext::from_focus("W", Some("D".into()), None, Some(2));
        assert_eq!(a, b, "identical contexts are equal (refresh skipped)");
        assert_ne!(a, c, "a cursor move changes the context (refresh fires)");
        assert_ne!(a.context_key(), c.context_key());
    }

    /// items_of_kind groups correctly across the three kinds.
    #[test]
    fn items_grouped_by_kind() {
        let raw = json!({"context_key": "k", "items": [
            {"id": "a", "kind": "episodic", "summary": "1", "source": {"event_id": "E"}},
            {"id": "b", "kind": "procedural", "summary": "2", "source": {"uri": "loom://x"}},
            {"id": "c", "kind": "episodic", "summary": "3", "source": {"event_id": "E2"}}
        ]});
        let pack: MemoryPack = serde_json::from_value(raw).unwrap();
        assert_eq!(pack.items_of_kind(MemoryKind::Episodic).count(), 2);
        assert_eq!(pack.items_of_kind(MemoryKind::Procedural).count(), 1);
        assert_eq!(pack.items_of_kind(MemoryKind::Semantic).count(), 0);
    }

    /// must_fix #2 FLOOR: an unknown `memory_class` (here the backend builder's `"working"` class — see
    /// `ace/mod.rs:1927,2024,2095`) does NOT hard-fail the whole capsule decode. The unknown-class item
    /// is SKIPPED (logged) and the three known-kind items still decode. Before this fix, the strict
    /// 3-variant enum would abort the entire pack with `unknown variant 'working'`, zeroing out ALL
    /// relevant memory because of one item.
    #[test]
    fn unknown_memory_class_is_skipped_not_fatal() {
        let raw = json!({
            "context_key": "k",
            "items": [
                {"id": "ep", "kind": "episodic", "summary": "ok", "source": {"event_id": "E"}},
                // A 4th class the real builder emits — must be tolerated (skipped), not a decode error.
                {"id": "work", "kind": "working", "summary": "scratch", "source": {"event_id": "W"}},
                {"id": "sem", "kind": "semantic", "summary": "fact", "source": {"uri": "loom://x"}},
                // A future/unknown class — also tolerated.
                {"id": "future", "kind": "telemetric", "summary": "future", "source": {"event_id": "F"}}
            ]
        });
        let pack: MemoryPack = serde_json::from_value(raw)
            .expect("must_fix #2: an unknown class must NOT fail decode");
        assert_eq!(
            pack.items.len(),
            2,
            "only the two known-kind items survive (working + telemetric dropped)"
        );
        assert_eq!(pack.items_of_kind(MemoryKind::Episodic).count(), 1);
        assert_eq!(pack.items_of_kind(MemoryKind::Semantic).count(), 1);
        // The dropped item ids are gone (no panic, no fatal).
        assert!(pack
            .items
            .iter()
            .all(|i| i.id != "work" && i.id != "future"));
    }

    /// must_fix #2: a capsule whose items are ALL unknown classes decodes to an EMPTY pack (each item
    /// skipped + logged), NOT a decode error. The panel then renders the neutral "no relevant memory"
    /// state rather than the generic error label.
    #[test]
    fn all_unknown_classes_decode_to_empty_pack() {
        let raw = json!({
            "context_key": "k",
            "items": [
                {"id": "w1", "kind": "working", "summary": "a", "source": {"event_id": "E1"}},
                {"id": "w2", "kind": "working", "summary": "b", "source": {"event_id": "E2"}}
            ]
        });
        let pack: MemoryPack =
            serde_json::from_value(raw).expect("all-unknown must decode to empty, not fail");
        assert!(
            pack.items.is_empty(),
            "all unknown-class items skipped -> empty pack (neutral state)"
        );
    }

    /// [`MemoryKind::from_wire`] maps the three rendered kinds and returns `None` for anything else
    /// (the tolerance primitive the capsule decoder uses).
    #[test]
    fn from_wire_maps_known_and_rejects_unknown() {
        assert_eq!(
            MemoryKind::from_wire("episodic"),
            Some(MemoryKind::Episodic)
        );
        assert_eq!(
            MemoryKind::from_wire("semantic"),
            Some(MemoryKind::Semantic)
        );
        assert_eq!(
            MemoryKind::from_wire("procedural"),
            Some(MemoryKind::Procedural)
        );
        assert_eq!(
            MemoryKind::from_wire("working"),
            None,
            "backend's 4th class is not a rendered kind"
        );
        assert_eq!(
            MemoryKind::from_wire("EPISODIC"),
            None,
            "wire is lowercase; case-sensitive"
        );
        assert_eq!(MemoryKind::from_wire(""), None);
    }
}
