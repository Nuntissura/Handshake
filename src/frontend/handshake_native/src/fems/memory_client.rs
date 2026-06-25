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
//! ## The FEMS read endpoint is ABSENT in this handshake_core build (the DESIGNED primary path)
//!
//! The contract names `GET /workspaces/{workspace_id}/memory/pack?context=...` as the existing FEMS read
//! route. A read-only verification of `src/backend/handshake_core` (the KERNEL_BUILDER gate 2026-06-25)
//! found that this route DOES NOT EXIST in the current build: handshake_core exposes the
//! knowledge-graph memory reads (`/knowledge/memory/claims|conflicts|facts|entities|visual-debug`) and
//! an INTERNAL `MemoryPack` builder (`ace/`, `memory/builder.rs`), but NO HTTP retrieval-capsule read
//! route. FEMS (Pillar 12) is a separate system not yet wired into the frozen handshake_core HTTP
//! surface. Per the contract's PRIMARY designed path, [`MemoryClient::fetch_pack`] therefore returns
//! [`MemoryClientError::EndpointMissing`] (the TYPED BLOCKER variant) on a 404 / feature-absent sentinel
//! — it does NOT add the route, does NOT rewrite the backend, and does NOT panic or silently no-op
//! (RISK-001, RISK-005/MC-002, AC-005). The panel maps `EndpointMissing` to a calm empty-state banner
//! and surfaces the blocker upward so the WP validator sees it. The client/model/parsing/clamp are all
//! proven against FIXTURE JSON regardless of whether the live endpoint exists; the live fetch (if the
//! route is ever added) is `NEEDS_MANAGED_RESOURCE_PROOF`.
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

/// Read timeout for a single capsule fetch. A bounded timeout so a hung backend cannot stall the editor
/// frame loop (the fetch runs off the render path on the shared async runtime).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

/// The least-privileged read-only actor id used for the FEMS capsule read. A missing `x-hsk-actor-kind`
/// header is the least-privileged read-only actor server-side (the same least-privilege default the
/// knowledge-documents read path uses), so no write-capable actor-kind is ever attached on this path.
const FEMS_READ_ACTOR_ID: &str = "native-editor-fems-reader";

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

    /// The three kinds in their fixed render order (Episodic, Semantic, Procedural).
    pub const ORDER: [MemoryKind; 3] =
        [MemoryKind::Episodic, MemoryKind::Semantic, MemoryKind::Procedural];
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    #[serde(default)]
    pub source: MemorySource,
    /// The retrieval relevance score, if the server supplied one (advisory; rendered subtly, never
    /// recomputed client-side).
    #[serde(default)]
    pub score: Option<f32>,
}

impl MemoryItem {
    /// True when the item's source resolves to a navigable target (delegates to
    /// [`MemorySource::validate`]).
    pub fn is_navigable(&self) -> bool {
        self.source.validate()
    }
}

/// The deserialized retrieval capsule (the Pillar 12 MemoryPack). `token_estimate` is ADVISORY metadata
/// surfaced by the client (never recomputed); `truncated` is `true` if the client clamped the item list
/// to [`MEMORY_PACK_MAX_ITEMS`] after decode (or if the server already marked it truncated).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MemoryPack {
    /// The capsule items (already clamped to <=24 by [`MemoryClient::fetch_pack`]).
    #[serde(default)]
    pub items: Vec<MemoryItem>,
    /// The advisory total token estimate the server reported (<=500 by the Pillar 12 budget). Surfaced
    /// as metadata; the client never recomputes it.
    #[serde(default)]
    pub token_estimate: Option<u32>,
    /// True if the item list was truncated (by the server OR the client's defensive clamp).
    #[serde(default)]
    pub truncated: bool,
    /// The context key the server keyed this capsule on (echoes the request context so a stale response
    /// can be detected). Defaults to empty if the server omits it.
    #[serde(default)]
    pub context_key: String,
}

impl MemoryPack {
    /// An empty pack (no items), used for the neutral "no relevant memory" render state.
    pub fn empty(context_key: impl Into<String>) -> Self {
        Self {
            items: Vec::new(),
            token_estimate: None,
            truncated: false,
            context_key: context_key.into(),
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
            self.cursor_byte.map(|c| c.to_string()).unwrap_or_else(|| "-".to_owned()),
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
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (the WP-011 backend client's pool) so the app
    /// shares ONE connection pool rather than minting a second HTTP stack (RISK-006/MC-005).
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Override the session run id attached to the read identity headers (so swarm/operator co-work is
    /// attributable). Returns `self` for builder-style chaining.
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
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
    /// - A 404 (or feature-absent sentinel) maps to [`MemoryClientError::EndpointMissing`] — the TYPED
    ///   BLOCKER, never a panic or silent no-op (RISK-001, RISK-005/MC-002, AC-005). This is the DESIGNED
    ///   PRIMARY PATH in the current build, where the route does not exist.
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
        let builder = self
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

        let resp = builder
            .send()
            .await
            .map_err(|e| MemoryClientError::Transport(e.to_string()))?;
        let status = resp.status();

        // The TYPED BLOCKER: a 404 means the documented FEMS read route is absent in this build. This is
        // the DESIGNED primary path (FEMS = Pillar 12, separate from the frozen handshake_core HTTP
        // surface). Surface it as the typed blocker — never panic, never silently no-op.
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(MemoryClientError::EndpointMissing { probed_path: path });
        }

        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MemoryClientError::Http { status: code, body });
        }

        let mut pack = resp
            .json::<MemoryPack>()
            .await
            .map_err(|e| MemoryClientError::Decode(e.to_string()))?;

        // DEFENSIVE CLAMP (RISK-002/MC-001, AC-002): enforce the <=24 cap client-side regardless of
        // server behavior. If the server returned more, truncate, mark truncated, and log a warning.
        clamp_pack_items(&mut pack);
        Ok(pack)
    }
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
        assert!(pack.truncated, "AC-002: truncated must be set true after clamp");
        assert_eq!(dropped, 6, "AC-002: 6 items dropped (30 - 24)");
    }

    /// A within-cap pack is untouched by the clamp (truncated stays false, no drops).
    #[test]
    fn within_cap_not_clamped() {
        let items: Vec<_> = (0..10)
            .map(|n| json!({"id": format!("i{n}"), "kind": "semantic", "summary": "x", "source": {"uri": "loom://x"}}))
            .collect();
        let mut pack: MemoryPack = serde_json::from_value(json!({"context_key": "k", "items": items})).unwrap();
        let dropped = clamp_pack_items(&mut pack);
        assert_eq!(dropped, 0);
        assert_eq!(pack.items.len(), 10);
        assert!(!pack.truncated);
    }

    /// Provenance precedence + non-navigable handling (RISK-003/MC-003): a source validates iff it has a
    /// uri, document_id, or event_id; an all-absent source is non-navigable.
    #[test]
    fn provenance_validate_and_precedence() {
        let nav_uri = MemorySource { uri: Some("loom://b".into()), ..Default::default() };
        let nav_doc = MemorySource { document_id: Some("D".into()), byte_range: Some((1, 2)), ..Default::default() };
        let nav_evt = MemorySource { event_id: Some("EV".into()), ..Default::default() };
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
        assert!(!pack.over_token_budget(), "absent estimate is not over budget");
    }

    /// EndpointMissing is the typed-blocker variant.
    #[test]
    fn endpoint_missing_is_typed_blocker() {
        let err = MemoryClientError::EndpointMissing { probed_path: "/workspaces/W/memory/pack".into() };
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
        assert!(ctx.selection_text.as_ref().unwrap().chars().count() <= 512, "selection bounded");
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
}
