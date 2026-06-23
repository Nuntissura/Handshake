//! WP-KERNEL-012 MT-038 (E6 — backend reuse wiring): the CONSOLIDATED typed native Rust client for the
//! EXISTING handshake_core `/workspaces/:ws/loom/*` HTTP surface (WP-KERNEL-009 Loom, `api::loom`).
//! This is FRONTEND WIRING ONLY — it binds routes the backend already serves; it does NOT change the
//! backend (a backend gap is a typed blocker, never a backend edit).
//!
//! ## Why this module exists (REUSE-NOT-DUPLICATE — the biggest overlap in the WP)
//!
//! ~25 of the 46 loom routes are ALREADY bound + verified by prior MTs (MT-021..032) through the WP-011
//! egui-thread `RequestSpec` / `GetRequestSpec` + delivery-`cell` builder clients in
//! [`crate::backend_client`] (`LoomBlockClient`, `LoomFolderClient`, `LoomTagClient`,
//! `LoomSidebarClient`, `LoomGraphClient`, `CanvasBoardClient`, `LoomWikiClient`, `BlockViewClient`,
//! `LoomSearchV2Client`, `LoomGraphSearchClient`, `WorkspaceSearchClient`) plus the dedicated UI modules
//! ([`crate::loom_graph`], [`crate::loom_search_v2`], [`crate::quick_switcher`]). Those builder clients
//! are the egui-thread render/dispatch surface MT-021..032 call sites depend on; MOVING them would break
//! those call sites, so this module does NOT move or re-implement them. It RE-EXPORTS them (see the
//! `pub use` block below) so `handshake_native::backend::loom` is the ONE unified namespace a navigator /
//! editor pane reaches for the whole Loom surface.
//!
//! On top of that re-export, this module ADDS the genuinely-missing routes no prior MT bound — as a
//! stateless ASYNC [`LoomClient`] adapter (the MT-037 [`crate::backend::knowledge_documents`] pattern),
//! reusing the ONE process-wide [`crate::backend_client::shared_http_client`] connection pool (no second
//! HTTP stack) and the config-resolved [`crate::backend_client::BACKEND_BASE_URL`] (never a hardcoded
//! host — GLOBAL-PORTABILITY-004). The genuinely-new routes:
//!   * block CRUD the egui clients did not bind as async: create / delete block, the read of one block;
//!   * `PUT  /loom/journals/:date`            (open_daily_journal — daily-note open);
//!   * `GET  /loom/blocks/:id/knowledge`      (knowledge-bridge — ProjectKnowledgeIndex/EventLedger);
//!   * `GET  /loom/blocks/:id/transclusion`   (note transclusion read-through; unresolved is FIRST-CLASS);
//!   * `POST/DELETE /loom/edges[/:edge_id]`   (create / delete edge);
//!   * `PUT/DELETE /loom/folders/:fid/blocks/:bid` (add / remove block ↔ folder membership);
//!   * the wiki extras `POST .../wiki/bootstrap | drift-check | fanout`, `POST .../wiki/:id/regenerate`,
//!     `GET .../wiki/:id/stale`, `DELETE .../wiki/:id`;
//!   * `POST /loom/import/markdown`           (vault-never-authority markdown import);
//!   * `GET  /loom/visual-debug`              (bounded navigation visual-debug snapshot).
//!
//! The MT contract also names the read routes the editor/navigator consume directly off the network
//! thread (transclusion / breadcrumbs / backlinks / search-v2 / view / quick-switcher), so the async
//! adapter binds those READS too — these are NEW async transports for the editor/test consumer, NOT a
//! fork of the egui-cell builders (no prior `async fn get_loom_block` / `loom_search_v2` existed).
//!
//! ## No identity headers on Loom (verified)
//!
//! Unlike `/knowledge/documents/*` (MT-037), the Loom routes use `WriteContext::human(None)` server-side
//! and do NOT enforce the `x-hsk-actor-*` header contract (verified READ-ONLY against
//! `src/backend/handshake_core/src/api/loom.rs` — no `doc_context`-style required-header guard on the
//! loom handlers). This client therefore attaches NO identity headers (RISK: adding headers the backend
//! does not require). If a future backend revision adds an actor-header guard to loom, that is a typed
//! blocker, not a silent header add here.
//!
//! ## Enum tolerance (RISK-1 / MC-1)
//!
//! [`LoomBlockContentType`] and [`LoomEdgeType`] mirror the backend storage enums but carry a
//! `#[serde(other)] Unknown` catch-all under `#[serde(rename_all = "snake_case")]`, so a NEW backend
//! variant deserializes to `Unknown` instead of failing the whole response (the MT-022 lesson: the real
//! storage set is `note | file | annotated_file | tag_hub | journal | canvas | view_def`, and additions
//! are tolerated). The MT prose named a different illustrative set; the REAL storage variants in
//! `storage/loom.rs::LoomBlockContentType` (verified READ-ONLY) are authoritative under the Spec-Realism
//! gate, and `Unknown` covers any drift either way.
//!
//! ## Unresolved transclusion is a FIRST-CLASS Ok (RISK-3 / MC-2 — the MT-025 lesson)
//!
//! [`LoomTransclusionResponse`] with `resolved == false` is a typed, visible unresolved state returned as
//! `Ok` (the backend returns HTTP 200 with `resolved: false` + an `unresolved_reason`), NEVER mapped to a
//! transport [`LoomError`]. A blank render would be a silent failure; the unresolved reason drives a
//! visible "unresolved" indicator instead.
//!
//! ## Coupling-avoidance (RISK-4/6 / MC-3/4)
//!
//! Graph (`traverse` / `local` / `global`), wiki staleness/projection, block-view, tag-hub, and search
//! result bodies are typed as [`serde_json::Value`] in this client (the MT control): a graph/wiki shape
//! change does not break compilation, and a per-widget typed extractor is added when the consuming
//! widget is built. The externally-meaningful CORE shapes the editor must reason about structurally
//! ([`LoomBlock`], [`LoomEdge`], [`LoomFolder`], [`LoomTransclusionResponse`], [`LoomKnowledgeBridge`],
//! [`LoomMarkdownImport`], [`LoomBreadcrumbTrail`]) are fully typed.
//!
//! ## Search-v2 never fabricates an embedding (RISK-7)
//!
//! The REAL `POST /loom/search-v2` body (verified) is `{query, content_type?, tag_ids, graph_boost,
//! limit, offset}` — there is NO client-supplied embedding field; the backend computes the query
//! embedding server-side from `query` through the operator's configured model (typed decline ->
//! keyword/trigram fallback). [`LoomSearchV2Request`] therefore mirrors that real body and carries NO
//! fabricated embedding vector.
//!
//! ## Stateless adapter
//!
//! [`LoomClient`] holds ONLY a shared [`reqwest::Client`] + the base URL — NO workspace/block state.
//! `workspace_id` is ALWAYS a function parameter; state (the open note, the selected block, the graph
//! viewport) lives in the editor/navigator layer that calls this.
//!
//! ## AccessKit requirement for FUTURE widget authors (this MT ships NO widget)
//!
//! This MT is backend client code ONLY — there is no egui widget and therefore no screenshot/AccessKit
//! proof here. When a calling widget is built (block navigator, graph panel, tag filter, transclusion
//! host, daily-note opener), EACH interactive element MUST receive a stable AccessKit `author_id`, role,
//! and actions through the EXISTING accessibility modules ([`crate::accessibility::registry`],
//! `crate::accessibility::snapshot`) — reuse the WP-011 surface, do NOT fork it. The graph pane reuses
//! [`crate::loom_graph`] (its [`crate::loom_graph::loom_node_author_id`]); the quick-switcher reuses
//! [`crate::quick_switcher`]; the search pane reuses [`crate::loom_search_v2`] (its
//! [`crate::loom_search_v2::result_author_id`] / `facet_author_id`).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend_client::{shared_http_client, BACKEND_BASE_URL};

// ════════════════════════════════════════════════════════════════════════════════════════════════
// RE-EXPORT the existing bound clients (MT-021..032) so this module is the ONE unified Loom namespace.
// These are NOT moved or re-implemented — re-exporting preserves every MT-021..032 call site that
// imports them from `crate::backend_client` while ALSO making `handshake_native::backend::loom::<X>`
// resolve to the SAME type. (REUSE-NOT-DUPLICATE: a re-implemented get_loom_block/search/canvas is a
// failure; this is a re-export.)
// ════════════════════════════════════════════════════════════════════════════════════════════════

pub use crate::backend_client::{
    BlockViewClient, BlockViewRecordData, CanvasBoardClient, CanvasBoardData, CanvasClient,
    LoomBlockClient, LoomBlockFlag, LoomFolderClient, LoomGraphClient, LoomGraphData,
    LoomGraphSearchHit, LoomSearchBlock, LoomSearchV2Body, LoomSearchV2Client, LoomSearchV2Hit,
    LoomSearchV2Response, LoomSidebarClient, LoomTagClient, LoomWikiClient, WikiProjection,
    WorkspaceSearchClient,
};
// The BlockCollectionView (MT-262) model types live in their canonical module; re-export them here so
// the unified `handshake_native::backend::loom` namespace also surfaces them (NOT re-defined).
pub use crate::graph::block_collection_view::{BlockViewDefinition, BlockViewResults};

/// Per-request timeout. A loom call must not hang the caller's worker; on timeout the client returns a
/// [`LoomError::Transport`] the navigator/editor surfaces as a transient error.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Core typed enums (mirror storage/loom.rs; #[serde(other)] catch-all tolerates backend additions).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A LoomBlock's content kind. Mirrors `storage::loom::LoomBlockContentType` (verified READ-ONLY:
/// `note | file | annotated_file | tag_hub | journal | canvas | view_def`). The [`Self::Unknown`]
/// `#[serde(other)]` catch-all makes an unrecognized backend variant deserialize to `Unknown` instead
/// of failing the whole response (RISK-1 / MC-1 — forward-compatible against new content types).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoomBlockContentType {
    Note,
    File,
    AnnotatedFile,
    TagHub,
    Journal,
    Canvas,
    ViewDef,
    /// Any content_type this client build does not know (a backend that added a variant). Tolerated
    /// (never a deserialization panic); a widget treats it as a generic block.
    #[serde(other)]
    Unknown,
}

/// A LoomEdge's relation kind. Mirrors `storage::loom::LoomEdgeType` (verified READ-ONLY:
/// `mention | tag | sub_tag | parent | ai_suggested`). The [`Self::Unknown`] `#[serde(other)]`
/// catch-all tolerates a new backend edge type without failing the response (RISK-1 / MC-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoomEdgeType {
    Mention,
    Tag,
    SubTag,
    Parent,
    AiSuggested,
    /// Any edge_type this client build does not know. Tolerated, never a deserialization panic.
    #[serde(other)]
    Unknown,
}

/// Who created a [`LoomEdge`] (`user` | `ai`). Mirrors `storage::loom::LoomEdgeCreatedBy`. Tolerant of
/// an unknown origin via [`Self::Unknown`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoomEdgeCreatedBy {
    User,
    Ai,
    #[serde(other)]
    Unknown,
}

/// A folder's content sort mode. Mirrors `storage::loom::LoomFolderSortMode`. Tolerant via
/// [`Self::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoomFolderSortMode {
    NameAsc,
    NameDesc,
    CreatedDesc,
    UpdatedDesc,
    Manual,
    #[serde(other)]
    Unknown,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Core typed response shapes (the externally-meaningful structural surfaces the editor reasons about).
// All nested/large/unstable bodies (derived metrics, graph, wiki, view-defs) stay serde_json::Value.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A Loom block (one `loom_blocks` row). Mirrors `storage::loom::LoomBlock`. The `derived` metrics and
/// timestamps are kept as a [`serde_json::Value`] to avoid coupling the client to the full
/// `LoomBlockDerived` projection (which carries optional AI/preview fields that evolve); the
/// editor-meaningful identity + flags + journal_date are typed.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomBlock {
    pub block_id: String,
    pub workspace_id: String,
    pub content_type: LoomBlockContentType,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub journal_date: Option<String>,
    /// Derived metrics (`backlink_count`, `mention_count`, `tag_count`, preview/AI fields). Kept as a
    /// `Value` to avoid coupling to the evolving `LoomBlockDerived` schema.
    #[serde(default)]
    pub derived: Value,
}

/// A Loom edge (one `loom_edges` row). Mirrors `storage::loom::LoomEdge`. `source_anchor` is a `Value`
/// (the editor reads it positionally per-widget) to avoid coupling to the anchor schema.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomEdge {
    pub edge_id: String,
    pub workspace_id: String,
    pub source_block_id: String,
    pub target_block_id: String,
    pub edge_type: LoomEdgeType,
    pub created_by: LoomEdgeCreatedBy,
    #[serde(default)]
    pub crdt_site_id: Option<String>,
    #[serde(default)]
    pub source_anchor: Value,
}

/// A Loom folder node (one `loom_folders` row). Mirrors `storage::loom::LoomFolder`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomFolder {
    pub folder_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub parent_folder_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    pub sort_mode: LoomFolderSortMode,
    #[serde(default)]
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub project_ref: Option<String>,
}

/// The note-transclusion read-through response (`GET /loom/blocks/:id/transclusion`). Mirrors the
/// backend `LoomTransclusionResponse` (verified READ-ONLY, `api/loom.rs`). **`resolved == false` is a
/// FIRST-CLASS Ok result** (the backend returns HTTP 200 with `resolved: false` + an
/// `unresolved_reason`), never a transport error (RISK-3 / MC-2). The host editor renders a visible
/// "unresolved" indicator from `unresolved_reason` instead of a silent blank.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomTransclusionResponse {
    pub block_id: String,
    pub workspace_id: String,
    /// The source rich document id the block resolves to (the edit target), when any.
    #[serde(default)]
    pub source_document_id: Option<String>,
    /// The current version of the source document (for optimistic save), when resolved.
    #[serde(default)]
    pub source_doc_version: Option<i64>,
    /// The live source document JSON (ProseMirror doc node), or `None` when the block resolves to no
    /// rich document. Kept as a `Value` (the rich-doc schema is owned by the knowledge-documents client).
    #[serde(default)]
    pub content_json: Option<Value>,
    /// `true` only when the block resolves to a real source rich document whose content was read
    /// through; `false` is the typed, VISIBLE unresolved state (never a silent blank).
    pub resolved: bool,
    /// A typed reason when `resolved` is false (e.g. `loom_block_has_no_source_document`,
    /// `source_rich_document_missing`). Absent when resolved.
    #[serde(default)]
    pub unresolved_reason: Option<String>,
}

/// The ProjectKnowledgeIndex / EventLedger authority bridge for a block
/// (`GET /loom/blocks/:id/knowledge`). Mirrors `storage::loom::LoomKnowledgeBridge`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomKnowledgeBridge {
    pub block_id: String,
    pub workspace_id: String,
    /// The ProjectKnowledgeIndex authority handle (`knowledge_entities.entity_id`, KEN-...).
    pub entity_id: String,
    /// EventLedger receipt id for the `KNOWLEDGE_LOOM_BLOCK_INDEXED` event.
    pub index_event_id: String,
}

/// The result of importing markdown into Loom authority (`POST /loom/import/markdown`). Mirrors
/// `storage::loom::LoomMarkdownImport`. The markdown source is NEVER authority — only the returned
/// PostgreSQL authority rows are (vault-never-authority, MT-187).
#[derive(Debug, Clone, Deserialize)]
pub struct LoomMarkdownImport {
    /// The new authority LoomBlock (content_type = note), bridged to the ProjectKnowledgeIndex.
    pub block: LoomBlock,
    /// The backing RichDocument authority record id (KRD-...).
    pub rich_document_id: String,
    /// Import warnings (e.g. unsupported markdown features), human-readable.
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// A navigation breadcrumb crumb (`workspace | project | folder | block | entity`). Mirrors
/// `storage::loom::LoomBreadcrumb`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomBreadcrumb {
    pub kind: String,
    pub id: String,
    pub label: String,
}

/// The breadcrumb trail for a block (`GET /loom/blocks/:id/breadcrumbs`). Mirrors
/// `storage::loom::LoomBreadcrumbTrail`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomBreadcrumbTrail {
    pub block_id: String,
    pub crumbs: Vec<LoomBreadcrumb>,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Request bodies (mirror the backend `*Request` / `*Body` structs — verified against api/loom.rs).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// `POST /workspaces/:ws/loom/blocks` body (mirrors `CreateLoomBlockRequest`). `content_type` is the
/// only required field; the optional fields are omitted from the wire when `None`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateLoomBlockRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    pub content_type: LoomBlockContentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_date: Option<String>,
}

/// `PATCH /workspaces/:ws/loom/blocks/:id` body (mirrors the backend `LoomBlockUpdate` patch).
///
/// **`add_tags` / `remove_tags` are tag-hub block IDs, NOT edge IDs** (RISK-2 / resolved server-side:
/// the backend resolves a tag-hub block id to the underlying TAG edge internally). Empty vecs are
/// omitted from the wire. The `title` / `pinned` / `favorite` / `journal_date` / `pin_order` fields use
/// the same patch semantics as `LoomBlockUpdate` (an absent field is unchanged).
#[derive(Debug, Clone, Default, Serialize)]
pub struct LoomBlockPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_order: Option<i32>,
    /// Tag-hub BLOCK IDs to attach (NOT edge ids — RISK-2). Omitted from the wire when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub add_tags: Vec<String>,
    /// Tag-hub BLOCK IDs to detach (NOT edge ids — RISK-2). Omitted from the wire when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remove_tags: Vec<String>,
}

/// `POST /workspaces/:ws/loom/edges` body (mirrors `CreateLoomEdgeRequest`). `created_by` is REQUIRED
/// by the backend (verified). `target_title` lets the backend auto-create a missing mention/tag target.
#[derive(Debug, Clone, Serialize)]
pub struct CreateLoomEdgeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_id: Option<String>,
    pub source_block_id: String,
    pub target_block_id: String,
    pub edge_type: LoomEdgeType,
    pub created_by: LoomEdgeCreatedBy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crdt_site_id: Option<String>,
    /// Source anchor (a `Value`; the editor builds it per-widget). Omitted when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_anchor: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_title: Option<String>,
}

/// `POST /workspaces/:ws/loom/folders` body (mirrors `CreateLoomFolderRequest`).
#[derive(Debug, Clone, Serialize)]
pub struct CreateLoomFolderRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_mode: Option<LoomFolderSortMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<String>,
}

/// `POST /workspaces/:ws/loom/wiki` body (mirrors `CompileWikiRequest`): compile a wiki projection from
/// a title + a citation set of source block ids.
#[derive(Debug, Clone, Serialize)]
pub struct CompileWikiRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub block_ids: Vec<String>,
}

/// `POST /workspaces/:ws/loom/import/markdown` body (mirrors `ImportMarkdownRequest`).
#[derive(Debug, Clone, Serialize)]
pub struct ImportMarkdownRequest {
    pub title: String,
    pub markdown: String,
}

/// `POST /workspaces/:ws/loom/search-v2` body. Mirrors the REAL backend `LoomSearchV2Body` (verified:
/// `{query, content_type?, tag_ids, graph_boost, limit, offset}`). There is **NO client-supplied
/// embedding field** — the backend computes the query embedding server-side from `query` through the
/// operator's configured model (typed decline -> keyword/trigram fallback). The client NEVER fabricates
/// an embedding (RISK-7).
#[derive(Debug, Clone, Default, Serialize)]
pub struct LoomSearchV2Request {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<LoomBlockContentType>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub graph_boost: f64,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed result of one `/loom/*` call. HTTP statuses map to DISTINCT variants so the editor reacts
/// correctly — a 404 (missing block/edge) is not mistaken for a transport failure, and a 400 (bad
/// request, e.g. an invalid journal date or a tag target that is not a tag hub) is distinct.
///
/// Note: an unresolved transclusion is NOT an error — it is a [`LoomTransclusionResponse`] with
/// `resolved == false` returned as `Ok` (RISK-3 / MC-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoomError {
    /// HTTP 400 — a malformed request (e.g. an invalid journal date, a tag edge whose target is not a
    /// tag hub, an invalid source anchor, a missing required search/visual-debug param). Carries the
    /// backend detail.
    BadRequest(String),
    /// HTTP 404 — the block / edge / folder / projection does not exist.
    NotFound(String),
    /// HTTP 409 — a typed conflict (e.g. the AI-job no-model decline). Carries the backend code/detail.
    Conflict(String),
    /// HTTP 5xx — the backend failed internally.
    Server(String),
    /// A non-success status that is none of the above mapped codes.
    UnexpectedStatus { status: u16, body: String },
    /// A transport failure (connect / timeout / TLS) — the request never reached a status.
    Transport(String),
    /// The response body could not be parsed into the expected typed shape.
    Parse(String),
}

impl std::fmt::Display for LoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(d) => write!(f, "bad request (400): {d}"),
            Self::NotFound(d) => write!(f, "not found (404): {d}"),
            Self::Conflict(d) => write!(f, "conflict (409): {d}"),
            Self::Server(d) => write!(f, "server error (5xx): {d}"),
            Self::UnexpectedStatus { status, body } => write!(f, "unexpected status {status}: {body}"),
            Self::Transport(e) => write!(f, "transport error: {e}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

impl std::error::Error for LoomError {}

/// A typed result alias for this client.
pub type LoomResult<T> = Result<T, LoomError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The stateless async typed client for the genuinely-new + editor-consumed `/workspaces/:ws/loom/*`
/// routes. Holds ONLY a shared [`reqwest::Client`] (cheaply cloneable; an `Arc` internally) and the base
/// URL — NO workspace/block state. The base URL is resolved from
/// [`crate::backend_client::BACKEND_BASE_URL`] (config/environment via the WP-011 backend client), NEVER
/// hardcoded at a call site (GLOBAL-PORTABILITY-004). `workspace_id` is ALWAYS a function parameter.
///
/// This adapter does NOT carry identity headers: the Loom backend uses `WriteContext::human(None)` and
/// does not enforce the `x-hsk-actor-*` contract (verified — unlike `/knowledge/documents/*`).
#[derive(Clone)]
pub struct LoomClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for LoomClient {
    fn default() -> Self {
        Self::production()
    }
}

impl LoomClient {
    /// Construct against the production backend base URL (the same `BACKEND_BASE_URL` every other native
    /// client uses — config-resolved, not hardcoded here), sharing the ONE process-wide
    /// [`crate::backend_client::shared_http_client`] connection pool rather than minting a second reqwest
    /// stack (the REUSE-NOT-DUPLICATE pool concern).
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool; the production path uses [`Self::production`], which shares the
    /// process-wide pool via [`Self::with_client`]). The base URL is the authority for the host — a
    /// function never hardcodes one.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (e.g. the one the WP-011
    /// [`crate::backend_client`] owns) so the app shares ONE connection pool rather than minting a
    /// second HTTP stack.
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    /// Build a full URL. `workspace_id` and path ids are interpolated as path SEGMENTS; reqwest
    /// percent-encodes them when building the request (workspace/block ids may contain hyphens but not
    /// slashes — RISK: a raw slash would break the path; ids here are opaque tokens without slashes).
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    // ── Block CRUD ──────────────────────────────────────────────────────────────────────────────

    /// Calls `POST /workspaces/:ws/loom/blocks` (create_loom_block) -> the created [`LoomBlock`].
    pub async fn create_block(
        &self,
        workspace_id: &str,
        body: &CreateLoomBlockRequest,
    ) -> LoomResult<LoomBlock> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/blocks")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id` (get_loom_block) -> the [`LoomBlock`].
    pub async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> LoomResult<LoomBlock> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/blocks/{block_id}")));
        self.send_json(builder).await
    }

    /// Calls `PATCH /workspaces/:ws/loom/blocks/:block_id` (patch_loom_block) -> the updated
    /// [`LoomBlock`]. `add_tags`/`remove_tags` are tag-hub BLOCK ids, not edge ids (RISK-2).
    pub async fn patch_loom_block(
        &self,
        workspace_id: &str,
        block_id: &str,
        body: &LoomBlockPatchRequest,
    ) -> LoomResult<LoomBlock> {
        let builder = self
            .client
            .patch(self.url(&format!("/workspaces/{workspace_id}/loom/blocks/{block_id}")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `DELETE /workspaces/:ws/loom/blocks/:block_id` (delete_loom_block) -> the `{status}` ack.
    pub async fn delete_loom_block(&self, workspace_id: &str, block_id: &str) -> LoomResult<Value> {
        let builder = self
            .client
            .delete(self.url(&format!("/workspaces/{workspace_id}/loom/blocks/{block_id}")));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id/knowledge` (get_loom_block_knowledge_bridge) ->
    /// the [`LoomKnowledgeBridge`] (ProjectKnowledgeIndex entity + EventLedger receipt).
    pub async fn get_loom_block_knowledge_bridge(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> LoomResult<LoomKnowledgeBridge> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/blocks/{block_id}/knowledge"
        )));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id/transclusion` (get_loom_block_transclusion) ->
    /// the [`LoomTransclusionResponse`]. **`resolved == false` is a FIRST-CLASS Ok** (the unresolved
    /// state), never an error (RISK-3 / MC-2).
    pub async fn get_loom_block_transclusion(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> LoomResult<LoomTransclusionResponse> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/blocks/{block_id}/transclusion"
        )));
        self.send_json(builder).await
    }

    /// Calls `PUT /workspaces/:ws/loom/blocks/:block_id/pin-order` (set_loom_block_pin_order) -> the
    /// updated [`LoomBlock`]. Body is `{ordinal}`.
    pub async fn set_loom_block_pin_order(
        &self,
        workspace_id: &str,
        block_id: &str,
        ordinal: i32,
    ) -> LoomResult<LoomBlock> {
        let builder = self
            .client
            .put(self.url(&format!(
                "/workspaces/{workspace_id}/loom/blocks/{block_id}/pin-order"
            )))
            .json(&serde_json::json!({ "ordinal": ordinal }));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id/breadcrumbs` (get_loom_block_breadcrumbs) ->
    /// the [`LoomBreadcrumbTrail`].
    pub async fn get_loom_block_breadcrumbs(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> LoomResult<LoomBreadcrumbTrail> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/blocks/{block_id}/breadcrumbs"
        )));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id/backlinks` (get_loom_block_backlinks) -> the
    /// backlink list as a `Value` (the `LoomBacklink` projection is read per-widget).
    pub async fn get_loom_block_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/blocks/{block_id}/backlinks"
        )));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/blocks/:block_id/unlinked-mentions`
    /// (scan_loom_block_unlinked_mentions) -> the unlinked-mention list as a `Value`.
    pub async fn scan_loom_block_unlinked_mentions(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/blocks/{block_id}/unlinked-mentions"
        )));
        self.send_json(builder).await
    }

    // ── Daily journal ─────────────────────────────────────────────────────────────────────────────

    /// Calls `PUT /workspaces/:ws/loom/journals/:journal_date` (open_daily_journal) -> the daily journal
    /// [`LoomBlock`] (idempotent open-or-create). `journal_date` is `YYYY-MM-DD` (a malformed date is a
    /// backend 400 -> [`LoomError::BadRequest`]).
    pub async fn open_daily_journal(
        &self,
        workspace_id: &str,
        journal_date: &str,
    ) -> LoomResult<LoomBlock> {
        let builder = self.client.put(self.url(&format!(
            "/workspaces/{workspace_id}/loom/journals/{journal_date}"
        )));
        self.send_json(builder).await
    }

    // ── Folders ─────────────────────────────────────────────────────────────────────────────────

    /// Calls `GET /workspaces/:ws/loom/folders` (list_loom_folders) -> the [`LoomFolder`] list.
    pub async fn list_loom_folders(&self, workspace_id: &str) -> LoomResult<Vec<LoomFolder>> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/folders")));
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/folders` (create_loom_folder) -> the created [`LoomFolder`].
    pub async fn create_loom_folder(
        &self,
        workspace_id: &str,
        body: &CreateLoomFolderRequest,
    ) -> LoomResult<LoomFolder> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/folders")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/folders/:folder_id` (get_loom_folder) -> the [`LoomFolder`].
    pub async fn get_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> LoomResult<LoomFolder> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/folders/{folder_id}")));
        self.send_json(builder).await
    }

    /// Calls `PATCH /workspaces/:ws/loom/folders/:folder_id` (update_loom_folder) -> the updated
    /// [`LoomFolder`]. `update` is the backend `LoomFolderUpdate` patch body (a `Value` so the
    /// double-option color/sort/parent fields are built by the folder widget without coupling here).
    pub async fn update_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        update: &Value,
    ) -> LoomResult<LoomFolder> {
        let builder = self
            .client
            .patch(self.url(&format!("/workspaces/{workspace_id}/loom/folders/{folder_id}")))
            .json(update);
        self.send_json(builder).await
    }

    /// Calls `DELETE /workspaces/:ws/loom/folders/:folder_id` (delete_loom_folder) -> the `{status}` ack.
    pub async fn delete_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .delete(self.url(&format!("/workspaces/{workspace_id}/loom/folders/{folder_id}")));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/folders/:folder_id/blocks` (list_loom_folder_blocks) -> the
    /// folder's member [`LoomBlock`]s. `limit`/`offset` are optional pagination query params (omitted
    /// when `None`; the backend applies its own defaults).
    pub async fn list_loom_folder_blocks(
        &self,
        workspace_id: &str,
        folder_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> LoomResult<Vec<LoomBlock>> {
        let mut builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/folders/{folder_id}/blocks"
        )));
        builder = apply_paging(builder, limit, offset);
        self.send_json(builder).await
    }

    /// Calls `PUT /workspaces/:ws/loom/folders/:folder_id/blocks/:block_id` (add_block_to_loom_folder)
    /// -> the `{status}` ack. `sort_order` is the optional manual ordinal (omitted when `None`).
    pub async fn add_block_to_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
        sort_order: Option<i32>,
    ) -> LoomResult<Value> {
        let body = match sort_order {
            Some(order) => serde_json::json!({ "sort_order": order }),
            None => serde_json::json!({}),
        };
        let builder = self
            .client
            .put(self.url(&format!(
                "/workspaces/{workspace_id}/loom/folders/{folder_id}/blocks/{block_id}"
            )))
            .json(&body);
        self.send_json(builder).await
    }

    /// Calls `DELETE /workspaces/:ws/loom/folders/:folder_id/blocks/:block_id`
    /// (remove_block_from_loom_folder) -> the `{status}` ack.
    pub async fn remove_block_from_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.delete(self.url(&format!(
            "/workspaces/{workspace_id}/loom/folders/{folder_id}/blocks/{block_id}"
        )));
        self.send_json(builder).await
    }

    // ── Tags ──────────────────────────────────────────────────────────────────────────────────────

    /// Calls `GET /workspaces/:ws/loom/tags` (list_loom_tag_hubs) -> the tag-hub [`LoomBlock`]s.
    pub async fn list_loom_tag_hubs(&self, workspace_id: &str) -> LoomResult<Vec<LoomBlock>> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/tags")));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/tags/:tag_block_id` (get_loom_tag_hub) -> the tag-hub detail as a
    /// `Value` (the `LoomTagHub` aggregate — block + sub_tags + tagged_block_count + backlinks — is read
    /// per-widget to avoid coupling to the aggregate schema).
    pub async fn get_loom_tag_hub(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/tags/{tag_block_id}")));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/tags/:tag_block_id/blocks` (list_loom_blocks_for_tag) -> the
    /// [`LoomBlock`]s tagged with this hub.
    pub async fn list_loom_blocks_for_tag(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
    ) -> LoomResult<Vec<LoomBlock>> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/tags/{tag_block_id}/blocks"
        )));
        self.send_json(builder).await
    }

    // ── Edges ─────────────────────────────────────────────────────────────────────────────────────

    /// Calls `POST /workspaces/:ws/loom/edges` (create_loom_edge) -> the created [`LoomEdge`].
    pub async fn create_loom_edge(
        &self,
        workspace_id: &str,
        body: &CreateLoomEdgeRequest,
    ) -> LoomResult<LoomEdge> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/edges")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `DELETE /workspaces/:ws/loom/edges/:edge_id` (delete_loom_edge) -> the deleted [`LoomEdge`].
    pub async fn delete_loom_edge(
        &self,
        workspace_id: &str,
        edge_id: &str,
    ) -> LoomResult<LoomEdge> {
        let builder = self
            .client
            .delete(self.url(&format!("/workspaces/{workspace_id}/loom/edges/{edge_id}")));
        self.send_json(builder).await
    }

    // ── Graph (Value bodies — RISK-4/6 / MC-4: typed extractors are added per-widget) ──────────────

    /// Calls `GET /workspaces/:ws/loom/graph/traverse` (traverse_loom_graph) -> the traversal result as
    /// a `Value`. A typed graph model is extracted per-widget; the native graph pane reuses
    /// [`crate::loom_graph`].
    pub async fn traverse_loom_graph(&self, workspace_id: &str, query: &[(&str, &str)]) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/graph/traverse")))
            .query(query);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/graph/local` (local_loom_graph) -> the local-neighborhood graph
    /// as a `Value`.
    pub async fn local_loom_graph(&self, workspace_id: &str, query: &[(&str, &str)]) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/graph/local")))
            .query(query);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/graph/global` (global_loom_graph) -> the global project graph as
    /// a `Value`. The global graph body can be large; the consuming pane deserializes off the UI thread
    /// (RISK-6).
    pub async fn global_loom_graph(&self, workspace_id: &str, query: &[(&str, &str)]) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/graph/global")))
            .query(query);
        self.send_json(builder).await
    }

    // ── Views + search ─────────────────────────────────────────────────────────────────────────────

    /// Calls `GET /workspaces/:ws/loom/views/:view_type` (query_loom_view) -> the view result as a
    /// `Value` (the `LoomViewResponse` union is read per-widget).
    pub async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: &str,
        query: &[(&str, &str)],
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/views/{view_type}")))
            .query(query);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/search` (search_loom_blocks) -> the keyword search result list as
    /// a `Value`. `q` is the query string.
    pub async fn search_loom_blocks(&self, workspace_id: &str, q: &str) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/search")))
            .query(&[("q", q)]);
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/search-v2` (loom_search_v2) -> the hybrid search result as a
    /// `Value` (hits + content_type_facets + semantic_available + total). Typed as `Value` so a hit-shape
    /// change does not break compilation; the search pane ([`crate::loom_search_v2`]) reads the hits. The
    /// request body NEVER carries a fabricated embedding (RISK-7) — the backend computes it from `query`.
    pub async fn loom_search_v2(
        &self,
        workspace_id: &str,
        body: &LoomSearchV2Request,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/search-v2")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/quick-switcher/recents` (list_quick_switcher_recents) -> the
    /// recents list as a `Value`. `limit` is an optional query param (omitted when `None`).
    pub async fn list_quick_switcher_recents(
        &self,
        workspace_id: &str,
        limit: Option<u32>,
    ) -> LoomResult<Value> {
        let mut builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/quick-switcher/recents"
        )));
        if let Some(limit) = limit {
            builder = builder.query(&[("limit", limit)]);
        }
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/quick-switcher/recents` (record_quick_switcher_recent) -> the
    /// recorded recent as a `Value`. `input` is the backend `QuickSwitcherRecentInput` body (a `Value`
    /// so the recent-kind enums are built by the quick-switcher widget — [`crate::quick_switcher`]).
    pub async fn record_quick_switcher_recent(
        &self,
        workspace_id: &str,
        input: &Value,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!(
                "/workspaces/{workspace_id}/loom/quick-switcher/recents"
            )))
            .json(input);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/visual-debug` (loom_visual_debug_snapshot) -> the bounded
    /// navigation visual-debug snapshot as a `Value`. The backend REQUIRES non-empty `start_block_id` +
    /// `q` query params (a missing one is a 400 -> [`LoomError::BadRequest`]); `limit` is optional.
    pub async fn loom_visual_debug_snapshot(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        q: &str,
        limit: Option<u32>,
    ) -> LoomResult<Value> {
        let mut builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/visual-debug")))
            .query(&[("start_block_id", start_block_id), ("q", q)]);
        if let Some(limit) = limit {
            builder = builder.query(&[("limit", limit)]);
        }
        self.send_json(builder).await
    }

    // ── Block collection views (MT-262) — Value bodies (per-widget typed extraction). ──────────────

    /// Calls `POST /workspaces/:ws/loom/views/definitions` (create_block_view) -> the created view record
    /// as a `Value`. `body` is the backend create-view body (built by the view widget).
    pub async fn create_block_view(&self, workspace_id: &str, body: &Value) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/views/definitions")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/views/definitions/:block_id` (get_block_view) -> the view record
    /// as a `Value`.
    pub async fn get_block_view(&self, workspace_id: &str, block_id: &str) -> LoomResult<Value> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/views/definitions/{block_id}"
        )));
        self.send_json(builder).await
    }

    /// Calls `PATCH /workspaces/:ws/loom/views/definitions/:block_id` (update_block_view) -> the updated
    /// view record as a `Value`.
    pub async fn update_block_view(
        &self,
        workspace_id: &str,
        block_id: &str,
        body: &Value,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .patch(self.url(&format!(
                "/workspaces/{workspace_id}/loom/views/definitions/{block_id}"
            )))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/views/definitions/:block_id/results` (query_block_view_results)
    /// -> the materialized rows as a `Value`.
    pub async fn query_block_view_results(
        &self,
        workspace_id: &str,
        block_id: &str,
        body: &Value,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!(
                "/workspaces/{workspace_id}/loom/views/definitions/{block_id}/results"
            )))
            .json(body);
        self.send_json(builder).await
    }

    // ── Wiki projection (MT-184/241/242/243) — Value bodies (staleness_verdict coupling-avoid). ────

    /// Calls `GET /workspaces/:ws/loom/wiki` (list_loom_wiki_pages) -> the wiki page list as a `Value`.
    /// Each page carries a `staleness_verdict` the client keeps as opaque JSON (RISK-4 / MC-3: avoid
    /// coupling to the `WikiStalenessVerdict` enum).
    pub async fn list_loom_wiki_pages(&self, workspace_id: &str) -> LoomResult<Value> {
        let builder = self
            .client
            .get(self.url(&format!("/workspaces/{workspace_id}/loom/wiki")));
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/wiki` (compile_loom_wiki_projection) -> the compiled
    /// `ServedWikiPage` as a `Value` (carries `staleness_verdict` — kept opaque, MC-3).
    pub async fn compile_loom_wiki_projection(
        &self,
        workspace_id: &str,
        body: &CompileWikiRequest,
    ) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/wiki")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/wiki/bootstrap` (bootstrap_project_wiki) -> the bootstrap outcome
    /// as a `Value`. `body` is the optional `{page_token_budget?}` (pass `serde_json::Value::Null` /
    /// `json!({})` for defaults).
    pub async fn bootstrap_project_wiki(&self, workspace_id: &str, body: &Value) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/wiki/bootstrap")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/wiki/drift-check` (project_wiki_drift_check) -> the drift report
    /// as a `Value`. `body` is the optional `{persist?}`.
    pub async fn project_wiki_drift_check(&self, workspace_id: &str, body: &Value) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/wiki/drift-check")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/wiki/fanout` (project_wiki_fanout) -> the fan-out outcome as a
    /// `Value`. `body` is `{source_kind, source_id, budget?}`.
    pub async fn project_wiki_fanout(&self, workspace_id: &str, body: &Value) -> LoomResult<Value> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/wiki/fanout")))
            .json(body);
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/wiki/:projection_id` (get_loom_wiki_projection) -> the served wiki
    /// page as a `Value`.
    pub async fn get_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/wiki/{projection_id}"
        )));
        self.send_json(builder).await
    }

    /// Calls `DELETE /workspaces/:ws/loom/wiki/:projection_id` (delete_loom_wiki_projection) -> the
    /// `{status}` ack. Deleting a projection mutates NO authority record (a projection is never
    /// authority).
    pub async fn delete_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.delete(self.url(&format!(
            "/workspaces/{workspace_id}/loom/wiki/{projection_id}"
        )));
        self.send_json(builder).await
    }

    /// Calls `POST /workspaces/:ws/loom/wiki/:projection_id/regenerate`
    /// (regenerate_loom_wiki_projection) -> the regenerated served wiki page as a `Value`.
    pub async fn regenerate_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.post(self.url(&format!(
            "/workspaces/{workspace_id}/loom/wiki/{projection_id}/regenerate"
        )));
        self.send_json(builder).await
    }

    /// Calls `GET /workspaces/:ws/loom/wiki/:projection_id/stale` (loom_wiki_projection_stale) -> the
    /// staleness verdict as a `Value` (`{projection_id, stale, verdict}`; `verdict` kept opaque — MC-3).
    pub async fn loom_wiki_projection_stale(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> LoomResult<Value> {
        let builder = self.client.get(self.url(&format!(
            "/workspaces/{workspace_id}/loom/wiki/{projection_id}/stale"
        )));
        self.send_json(builder).await
    }

    // ── Markdown import ─────────────────────────────────────────────────────────────────────────────

    /// Calls `POST /workspaces/:ws/loom/import/markdown` (import_markdown_to_loom) -> the
    /// [`LoomMarkdownImport`] (the new authority block + backing rich-doc id + warnings). The markdown
    /// source is NEVER authority (vault-never-authority, MT-187).
    pub async fn import_markdown_to_loom(
        &self,
        workspace_id: &str,
        body: &ImportMarkdownRequest,
    ) -> LoomResult<LoomMarkdownImport> {
        let builder = self
            .client
            .post(self.url(&format!("/workspaces/{workspace_id}/loom/import/markdown")))
            .json(body);
        self.send_json(builder).await
    }

    // ── Shared send + status-mapping + parse path. ──────────────────────────────────────────────────

    /// Send a built request (timeout attached), map the HTTP status to a typed [`LoomError`], and
    /// deserialize a success body into `T`. The body is parsed exactly once. 400/404/409 map to their
    /// distinct variants carrying the backend detail; 5xx maps to [`LoomError::Server`]; any other
    /// non-success status to [`LoomError::UnexpectedStatus`].
    async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> LoomResult<T> {
        let resp = builder
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| LoomError::Transport(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<T>()
                .await
                .map_err(|e| LoomError::Parse(e.to_string()));
        }
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(map_error_status(code, &body))
    }
}

/// Attach optional `limit`/`offset` pagination query params (only when present). The backend applies its
/// own sane defaults when a param is absent (documented in the calling fns).
fn apply_paging(
    mut builder: reqwest::RequestBuilder,
    limit: Option<u32>,
    offset: Option<u32>,
) -> reqwest::RequestBuilder {
    if let Some(limit) = limit {
        builder = builder.query(&[("limit", limit)]);
    }
    if let Some(offset) = offset {
        builder = builder.query(&[("offset", offset)]);
    }
    builder
}

/// Map a non-success status + body into the typed [`LoomError`]. Pure (no IO) so the status-to-variant
/// contract is unit-provable without a live socket. The backend error body shape is
/// `{"error": "<code>"}` or `{"detail": "..."}`; the detail/code is surfaced when present.
fn map_error_status(status: u16, body: &str) -> LoomError {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let detail = parsed
        .as_ref()
        .and_then(|v| {
            v.get("detail")
                .or_else(|| v.get("reason"))
                .or_else(|| v.get("error"))
        })
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string());
    match status {
        400 => LoomError::BadRequest(detail),
        404 => LoomError::NotFound(detail),
        409 => LoomError::Conflict(detail),
        500..=599 => LoomError::Server(detail),
        other => LoomError::UnexpectedStatus {
            status: other,
            body: body.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit proofs for the enum-tolerance + status-mapping + body-serialization contracts that DO
    //! NOT need a socket. The wire-level proofs (mock-server round-trips for block CRUD, edge
    //! create/delete, tag patch, transclusion unresolved, search-v2, journal open, import/markdown)
    //! live in `tests/test_loom.rs` against REAL backend payload shapes.

    use super::*;
    use serde_json::json;

    #[test]
    fn content_type_unknown_variant_does_not_panic_on_new_backend_value() {
        // The known set deserializes 1:1 with the backend snake_case names.
        for (raw, expect) in [
            ("note", LoomBlockContentType::Note),
            ("file", LoomBlockContentType::File),
            ("annotated_file", LoomBlockContentType::AnnotatedFile),
            ("tag_hub", LoomBlockContentType::TagHub),
            ("journal", LoomBlockContentType::Journal),
            ("canvas", LoomBlockContentType::Canvas),
            ("view_def", LoomBlockContentType::ViewDef),
        ] {
            let parsed: LoomBlockContentType = serde_json::from_value(json!(raw)).unwrap();
            assert_eq!(parsed, expect, "{raw} must map to its known variant");
        }
        // RISK-1 / MC-1: an UNKNOWN backend variant deserializes to Unknown, never an error/panic.
        let future: LoomBlockContentType =
            serde_json::from_value(json!("a_brand_new_backend_type")).expect("unknown must not fail");
        assert_eq!(future, LoomBlockContentType::Unknown);
    }

    #[test]
    fn edge_type_unknown_variant_does_not_panic_on_new_backend_value() {
        for (raw, expect) in [
            ("mention", LoomEdgeType::Mention),
            ("tag", LoomEdgeType::Tag),
            ("sub_tag", LoomEdgeType::SubTag),
            ("parent", LoomEdgeType::Parent),
            ("ai_suggested", LoomEdgeType::AiSuggested),
        ] {
            let parsed: LoomEdgeType = serde_json::from_value(json!(raw)).unwrap();
            assert_eq!(parsed, expect, "{raw} must map to its known variant");
        }
        let future: LoomEdgeType =
            serde_json::from_value(json!("some_future_edge")).expect("unknown edge must not fail");
        assert_eq!(future, LoomEdgeType::Unknown);
    }

    #[test]
    fn status_map_400_404_409_5xx_are_distinct_variants() {
        assert!(matches!(
            map_error_status(400, &json!({"error": "HSK-400-LOOM-JOURNAL-DATE"}).to_string()),
            LoomError::BadRequest(_)
        ));
        assert!(matches!(
            map_error_status(404, &json!({"detail": "not_found"}).to_string()),
            LoomError::NotFound(_)
        ));
        assert!(matches!(
            map_error_status(409, &json!({"error": "HSK-409-LOOM-AI-NO-MODEL"}).to_string()),
            LoomError::Conflict(_)
        ));
        assert!(matches!(
            map_error_status(500, &json!({"error": "internal_error"}).to_string()),
            LoomError::Server(_)
        ));
        assert!(matches!(
            map_error_status(418, "teapot"),
            LoomError::UnexpectedStatus { status: 418, .. }
        ));
    }

    #[test]
    fn patch_block_tags_serialize_as_block_ids_and_omit_when_empty() {
        // RISK-2: add_tags/remove_tags are tag-hub BLOCK ids, sent as a string array (not edge ids).
        let patch = LoomBlockPatchRequest {
            add_tags: vec!["BLK-tag-hub-1".into()],
            remove_tags: vec![],
            ..Default::default()
        };
        let v = serde_json::to_value(&patch).unwrap();
        assert_eq!(v["add_tags"], json!(["BLK-tag-hub-1"]), "add_tags is a block-id array");
        assert!(
            v.get("remove_tags").is_none(),
            "an empty remove_tags is OMITTED from the wire: {v}"
        );
        assert!(v.get("title").is_none(), "an absent title is omitted (unchanged): {v}");
    }

    #[test]
    fn search_v2_request_has_no_fabricated_embedding_field() {
        // RISK-7: the client NEVER sends an embedding; the body mirrors the real backend LoomSearchV2Body.
        let req = LoomSearchV2Request {
            query: "design notes".into(),
            ..Default::default()
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["query"], json!("design notes"));
        assert!(
            v.get("embedding").is_none() && v.get("query_embedding").is_none(),
            "the search-v2 body must NOT carry a fabricated embedding: {v}"
        );
        // content_type omitted when None; an empty tag_ids omitted.
        assert!(v.get("content_type").is_none(), "absent content_type omitted: {v}");
        assert!(v.get("tag_ids").is_none(), "empty tag_ids omitted: {v}");
    }

    #[test]
    fn create_block_request_omits_absent_optionals_and_sends_content_type() {
        let body = CreateLoomBlockRequest {
            block_id: None,
            content_type: LoomBlockContentType::Note,
            document_id: None,
            asset_id: None,
            title: Some("Untitled".into()),
            pinned: None,
            journal_date: None,
        };
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["content_type"], json!("note"), "content_type serializes snake_case");
        assert_eq!(v["title"], json!("Untitled"));
        assert!(v.get("block_id").is_none(), "absent block_id omitted: {v}");
        assert!(v.get("pinned").is_none(), "absent pinned omitted: {v}");
    }

    #[test]
    fn transclusion_unresolved_is_first_class_ok_shape() {
        // RISK-3 / MC-2: an unresolved transclusion (resolved=false + reason) deserializes cleanly as a
        // typed Ok value, NOT an error. Mirrors the backend 200 body for a block with no source document.
        let body = json!({
            "block_id": "BLK-1",
            "workspace_id": "WS-1",
            "source_document_id": null,
            "source_doc_version": null,
            "content_json": null,
            "resolved": false,
            "unresolved_reason": "loom_block_has_no_source_document"
        });
        let parsed: LoomTransclusionResponse = serde_json::from_value(body).expect("unresolved must parse");
        assert!(!parsed.resolved, "resolved=false is preserved");
        assert_eq!(
            parsed.unresolved_reason.as_deref(),
            Some("loom_block_has_no_source_document"),
            "the typed unresolved reason drives a visible indicator, not a blank"
        );
        assert!(parsed.content_json.is_none(), "no source content when unresolved");
    }
}
