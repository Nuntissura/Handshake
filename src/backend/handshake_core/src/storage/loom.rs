use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub asset_id: String,
    pub workspace_id: String,
    pub kind: String,
    pub mime: String,
    pub original_filename: Option<String>,
    pub content_hash: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub classification: String,
    pub exportable: bool,
    pub is_proxy_of: Option<String>,
    pub proxy_asset_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewAsset {
    pub workspace_id: String,
    pub kind: String,
    pub mime: String,
    pub original_filename: Option<String>,
    pub content_hash: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub classification: String,
    pub exportable: bool,
    pub is_proxy_of: Option<String>,
    pub proxy_asset_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomBlockContentType {
    Note,
    File,
    AnnotatedFile,
    TagHub,
    Journal,
}

impl LoomBlockContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoomBlockContentType::Note => "note",
            LoomBlockContentType::File => "file",
            LoomBlockContentType::AnnotatedFile => "annotated_file",
            LoomBlockContentType::TagHub => "tag_hub",
            LoomBlockContentType::Journal => "journal",
        }
    }
}

impl FromStr for LoomBlockContentType {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "note" => Ok(LoomBlockContentType::Note),
            "file" => Ok(LoomBlockContentType::File),
            "annotated_file" => Ok(LoomBlockContentType::AnnotatedFile),
            "tag_hub" => Ok(LoomBlockContentType::TagHub),
            "journal" => Ok(LoomBlockContentType::Journal),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid loom block content_type",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStatus {
    None,
    Pending,
    Generated,
    Failed,
}

impl PreviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreviewStatus::None => "none",
            PreviewStatus::Pending => "pending",
            PreviewStatus::Generated => "generated",
            PreviewStatus::Failed => "failed",
        }
    }
}

impl FromStr for PreviewStatus {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(PreviewStatus::None),
            "pending" => Ok(PreviewStatus::Pending),
            "generated" => Ok(PreviewStatus::Generated),
            "failed" => Ok(PreviewStatus::Failed),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid loom preview status",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomBlockDerivedGeneratedBy {
    pub model: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoomBlockDerived {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text_index: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<f64>,
    pub backlink_count: i64,
    pub mention_count: i64,
    pub tag_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_asset_id: Option<String>,
    pub preview_status: PreviewStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<LoomBlockDerivedGeneratedBy>,
}

impl Default for LoomBlockDerived {
    fn default() -> Self {
        Self {
            full_text_index: None,
            embedding_id: None,
            auto_tags: None,
            auto_caption: None,
            quality_score: None,
            backlink_count: 0,
            mention_count: 0,
            tag_count: 0,
            thumbnail_asset_id: None,
            proxy_asset_id: None,
            preview_status: PreviewStatus::None,
            generated_by: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomBlock {
    pub block_id: String,
    pub workspace_id: String,
    pub content_type: LoomBlockContentType,
    pub document_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: Option<String>,
    pub original_filename: Option<String>,
    pub content_hash: Option<String>,
    pub pinned: bool,
    /// MT-183: user-controlled ordinal for the reorderable Pins grid. `None`
    /// (un-ordered) pins sort after explicitly-ordered pins, then by recency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_order: Option<i32>,
    pub journal_date: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub imported_at: Option<DateTime<Utc>>,
    pub derived: LoomBlockDerived,
}

#[derive(Clone, Debug)]
pub struct NewLoomBlock {
    pub block_id: Option<String>,
    pub workspace_id: String,
    pub content_type: LoomBlockContentType,
    pub document_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: Option<String>,
    pub original_filename: Option<String>,
    pub content_hash: Option<String>,
    pub pinned: bool,
    pub journal_date: Option<String>,
    pub imported_at: Option<DateTime<Utc>>,
    pub derived: LoomBlockDerived,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoomBlockUpdate {
    pub title: Option<String>,
    pub pinned: Option<bool>,
    pub journal_date: Option<String>,
    /// MT-183: set/clear the Pins-grid ordinal. Send a value to position the
    /// pin; send `null` via the dedicated reorder endpoint to clear it.
    #[serde(default)]
    pub pin_order: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomEdgeType {
    Mention,
    Tag,
    SubTag,
    Parent,
    AiSuggested,
}

impl LoomEdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoomEdgeType::Mention => "mention",
            LoomEdgeType::Tag => "tag",
            LoomEdgeType::SubTag => "sub_tag",
            LoomEdgeType::Parent => "parent",
            LoomEdgeType::AiSuggested => "ai_suggested",
        }
    }
}

impl FromStr for LoomEdgeType {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "mention" => Ok(LoomEdgeType::Mention),
            "tag" => Ok(LoomEdgeType::Tag),
            "sub_tag" => Ok(LoomEdgeType::SubTag),
            "parent" => Ok(LoomEdgeType::Parent),
            "ai_suggested" => Ok(LoomEdgeType::AiSuggested),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid loom edge_type",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomEdgeCreatedBy {
    User,
    Ai,
}

impl LoomEdgeCreatedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoomEdgeCreatedBy::User => "user",
            LoomEdgeCreatedBy::Ai => "ai",
        }
    }
}

impl FromStr for LoomEdgeCreatedBy {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user" => Ok(LoomEdgeCreatedBy::User),
            "ai" => Ok(LoomEdgeCreatedBy::Ai),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid loom edge created_by",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomSourceAnchor {
    pub document_id: String,
    pub block_id: String,
    pub offset_start: i64,
    pub offset_end: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomEdge {
    pub edge_id: String,
    pub workspace_id: String,
    pub source_block_id: String,
    pub target_block_id: String,
    pub edge_type: LoomEdgeType,
    pub created_by: LoomEdgeCreatedBy,
    pub created_at: DateTime<Utc>,
    pub crdt_site_id: Option<String>,
    pub source_anchor: Option<LoomSourceAnchor>,
}

#[derive(Clone, Debug)]
pub struct NewLoomEdge {
    pub edge_id: Option<String>,
    pub workspace_id: String,
    pub source_block_id: String,
    pub target_block_id: String,
    pub edge_type: LoomEdgeType,
    pub created_by: LoomEdgeCreatedBy,
    pub crdt_site_id: Option<String>,
    pub source_anchor: Option<LoomSourceAnchor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomViewType {
    All,
    Unlinked,
    Sorted,
    Pins,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoomViewFilters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<LoomBlockContentType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_from: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_to: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mention_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomViewGroup {
    pub edge_type: LoomEdgeType,
    pub target_block_id: String,
    pub blocks: Vec<LoomBlock>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "view_type", rename_all = "snake_case")]
pub enum LoomViewResponse {
    All { blocks: Vec<LoomBlock> },
    Unlinked { blocks: Vec<LoomBlock> },
    Pins { blocks: Vec<LoomBlock> },
    Sorted { groups: Vec<LoomViewGroup> },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoomSearchFilters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<LoomBlockContentType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mention_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backlink_depth: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomBlockSearchResult {
    pub block: LoomBlock,
    #[serde(default)]
    pub score: f64,
}

// ---------------------------------------------------------------------------
// MT-177 LoomBlockKnowledgeBridge
//
// The authority binding that makes the Loom surface resolve to the
// ProjectKnowledgeIndex (knowledge_entities) + EventLedger, rather than living
// as a parallel store. Per Master Spec §10.12 #9.1.1 (WP-KERNEL-009 authority
// supersession) the ONLY authority path is PostgreSQL + EventLedger; this is
// the positive binding for that rule.
// ---------------------------------------------------------------------------

/// The single authority backend for the Loom surface under WP-KERNEL-009.
///
/// There is intentionally only one variant. §10.12 #9.1.1 forbids any SQLite /
/// cache / offline / sidecar authority path for WP-009 Loom; the storage crate
/// compiles no `sqlite` module (see `storage/mod.rs`), so Postgres is the only
/// reachable backend and this enum makes that explicit and assertable in tests.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomAuthorityBackend {
    /// PostgreSQL + EventLedger — the sole WP-009 Loom authority.
    PostgresEventLedger,
}

impl LoomAuthorityBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoomAuthorityBackend::PostgresEventLedger => "postgres_event_ledger",
        }
    }

    /// True iff this backend is a single-source-of-truth authority path (never
    /// a cache / offline / sidecar). Always true: the only variant is the
    /// Postgres+EventLedger authority.
    pub fn is_authority(&self) -> bool {
        matches!(self, LoomAuthorityBackend::PostgresEventLedger)
    }
}

/// The queryable, idempotent authority link between a `LoomBlock` and its
/// ProjectKnowledgeIndex entity (`knowledge_entities`, entity_kind=`loom_block`)
/// plus the EventLedger receipt that proves the bridge/index operation.
///
/// Backed by the `loom_block_knowledge_bridge` table (migration 0292).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoomKnowledgeBridge {
    /// The bridged LoomBlock (`loom_blocks.block_id`).
    pub block_id: String,
    pub workspace_id: String,
    /// The ProjectKnowledgeIndex authority handle
    /// (`knowledge_entities.entity_id`, KEN-...).
    pub entity_id: String,
    /// EventLedger receipt id (`kernel_event_ledger.event_id`) for the
    /// `KNOWLEDGE_LOOM_BLOCK_INDEXED` event.
    pub index_event_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// MT-178 BacklinkComputation
//
// Master Spec §10.12 [LM-BACK-001..003] / Pattern H-5: every block surface
// shows the blocks referencing it (linked mentions) with a surrounding-text
// context snippet, PLUS unlinked mentions — text occurrences of a block's
// title/aliases that are NOT yet formal links, surfaced so one click converts
// them to edges (Obsidian unlinked-mentions idiom; feeds the Unlinked view).
// Authority is the loom_edges + loom_blocks Postgres store (no parallel index).
// ---------------------------------------------------------------------------

/// A linked backlink: an incoming `LoomEdge` (MENTION/TAG/...) together with the
/// referencing source block and a surrounding-text context snippet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomBacklink {
    /// The incoming edge whose `target_block_id` is the block being viewed.
    pub edge: LoomEdge,
    /// The block that references the viewed block (the edge source).
    pub source_block: LoomBlock,
    /// Surrounding-text snippet for the reference. `None` when the source block
    /// exposes no readable text (e.g. an asset-only file block with no title).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_snippet: Option<String>,
}

/// An unlinked mention: a block whose searchable text contains the viewed
/// block's title (or an alias) but which has NO formal MENTION/TAG edge to it.
/// One click converts it to a real edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomUnlinkedMention {
    /// The block that mentions the viewed block's title in plain text.
    pub source_block: LoomBlock,
    /// The matched term (the viewed block's title/alias).
    pub matched_term: String,
    /// A surrounding-text snippet showing the unlinked occurrence.
    pub snippet: String,
    /// Char offset of the match within the source block's scanned text.
    pub match_offset: i64,
}

/// Marker bounding a context snippet window (chars of surrounding context on
/// each side of a match). Kept small so backlink panels stay snappy and a
/// snippet never leaks an entire document.
pub const LOOM_SNIPPET_CONTEXT_CHARS: usize = 48;

// ---------------------------------------------------------------------------
// MT-179 LocalGraphApi + MT-180 GlobalGraphApi
//
// Master Spec §10.12 §9.4 (recursive CTE) / [LM-GRAPH-001] (local neighborhood
// with filters/edge-types/depth/stale-markers/source-citations) and §7.1.4.3 /
// [LM-GRAPH-002] (project global graph with performance limits + hub
// suppression). Authority = loom_edges + loom_blocks (+ the MT-177 bridge for
// citations). These are read projections over Postgres; never a parallel store.
// ---------------------------------------------------------------------------

/// A node in a Loom graph projection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomGraphNode {
    pub block: LoomBlock,
    /// BFS distance from the start block (0 = start). For the global graph this
    /// is 0 for every node (no single origin).
    pub depth: u32,
    /// Total number of edges (in + out) touching this node within the returned
    /// graph — used by the UI for sizing and by hub suppression.
    pub degree: u32,
    /// Stale marker: the node is NOT bridged to the ProjectKnowledgeIndex
    /// (no loom_block_knowledge_bridge row), i.e. it is not yet an indexed
    /// authority entity. A no-context reader can see un-indexed nodes at a
    /// glance (§10.12 stale markers).
    pub stale: bool,
    /// Source citation: the ProjectKnowledgeIndex entity id this node resolves
    /// to (`knowledge_entities.entity_id`), when bridged. `None` => stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

/// An edge in a Loom graph projection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomGraphEdge {
    pub edge: LoomEdge,
    /// Stale marker: the edge is an unconfirmed AI suggestion
    /// (`ai_suggested`, DerivedContent per [LM-EDGE-002]) — not user-authored
    /// authority until promoted.
    pub stale: bool,
}

/// A Loom graph projection (local neighborhood or global project graph).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomGraph {
    pub nodes: Vec<LoomGraphNode>,
    pub edges: Vec<LoomGraphEdge>,
    /// True when a performance limit clipped the result (more nodes/edges
    /// exist than were returned). The UI shows a "graph truncated" affordance.
    pub truncated: bool,
    /// Block ids suppressed as hubs (degree above the threshold) in the global
    /// graph. Empty for the local graph.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suppressed_hub_ids: Vec<String>,
}

/// Default node cap for the global graph (performance limit, [LM-GRAPH-002]).
pub const LOOM_GLOBAL_GRAPH_DEFAULT_NODE_LIMIT: u32 = 500;
/// Hard ceiling on the global-graph node cap.
pub const LOOM_GLOBAL_GRAPH_MAX_NODE_LIMIT: u32 = 5000;
/// Default degree above which a node is suppressed as a hub in the global graph.
pub const LOOM_GLOBAL_GRAPH_DEFAULT_HUB_DEGREE: u32 = 50;

// ---------------------------------------------------------------------------
// MT-182 TagsAndTagHubs
//
// Master Spec §10.12 [LM-TAG-001..005] / §7.1.4.3: a #tag is a first-class
// LoomBlock (content_type=tag_hub) carrying its own content, sub-tags (SUB_TAG
// edges) and backlinks ("even your tags are just notes"; closest external
// analog: Tana supertags). #tag edges are categorical; SUB_TAG models the
// nested-tag hierarchy (e.g. #project/alpha => SUB_TAG(child=alpha,
// parent=project)). Authority = loom_blocks + loom_edges.
// ---------------------------------------------------------------------------

/// A tag hub surface: the tag block itself plus its sub-tags (nested-tag
/// children), the blocks tagged with it, and its backlink count.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoomTagHub {
    /// The tag_hub LoomBlock (its title is the tag name; it carries its own
    /// content like any note via its document_id).
    pub block: LoomBlock,
    /// Direct child tags: blocks that are the SOURCE of a SUB_TAG edge whose
    /// TARGET is this tag (the nested-tag hierarchy, [LM-TAG-003]).
    pub sub_tags: Vec<LoomBlock>,
    /// Blocks tagged with this tag: SOURCES of TAG edges whose TARGET is this
    /// tag. (Direct only; nested membership is exposed via list_blocks_for_tag
    /// with include_subtags.)
    pub tagged_blocks: Vec<LoomBlock>,
    /// Number of incoming edges (backlinks) to this tag hub.
    pub backlink_count: i64,
}

/// Build a surrounding-text snippet for a match located by `match_start`
/// (char index) and `match_len` (char length) within `text`. Returns a window
/// of up to `LOOM_SNIPPET_CONTEXT_CHARS` characters of context on each side,
/// with leading/trailing ellipses when the window is clipped. Operates on
/// Unicode scalar values (chars), never byte offsets, so multi-byte text is
/// never split mid-character.
pub fn loom_context_snippet(text: &str, match_start: usize, match_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || match_start >= chars.len() {
        return text.trim().chars().take(LOOM_SNIPPET_CONTEXT_CHARS * 2).collect();
    }
    let match_end = match_start.saturating_add(match_len).min(chars.len());
    let window_start = match_start.saturating_sub(LOOM_SNIPPET_CONTEXT_CHARS);
    let window_end = match_end
        .saturating_add(LOOM_SNIPPET_CONTEXT_CHARS)
        .min(chars.len());

    let mut snippet = String::new();
    if window_start > 0 {
        snippet.push_str("...");
    }
    let core: String = chars[window_start..window_end].iter().collect();
    snippet.push_str(core.trim());
    if window_end < chars.len() {
        snippet.push_str("...");
    }
    snippet
}

/// Find the first case-insensitive, word-boundary occurrence of `term` in
/// `text`, returning `(char_start, char_len)`. Word-boundary means the match is
/// not flanked by alphanumeric characters (so "plan" does not match "planning"
/// or "explanation"). Returns `None` when `term` is empty or absent.
///
/// This is the unlinked-mention detector: a plain-text occurrence of a block's
/// title that is a candidate for promotion to a formal edge.
pub fn loom_find_unlinked_term(text: &str, term: &str) -> Option<(usize, usize)> {
    let term_trimmed = term.trim();
    if term_trimmed.is_empty() {
        return None;
    }
    let hay: Vec<char> = text.chars().collect();
    let needle: Vec<char> = term_trimmed.to_lowercase().chars().collect();
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    let hay_lower: Vec<char> = hay
        .iter()
        .flat_map(|c| c.to_lowercase())
        .collect();
    // to_lowercase can change length for some scripts; fall back to a simple
    // per-char lowercase comparison aligned to the original char vector to keep
    // offsets meaningful. Guard the rare expansion case by recomputing on the
    // original-length lowercase view.
    let hay_lc: Vec<char> = hay.iter().map(|c| c.to_ascii_lowercase()).collect();
    let scan: &Vec<char> = if hay_lc.len() == hay.len() {
        &hay_lc
    } else {
        &hay_lower
    };

    let is_word = |c: char| c.is_alphanumeric();
    let last_start = scan.len().saturating_sub(needle.len());
    for start in 0..=last_start {
        if scan[start..start + needle.len()] == needle[..] {
            let before_ok = start == 0 || !is_word(scan[start - 1]);
            let after_idx = start + needle.len();
            let after_ok = after_idx >= scan.len() || !is_word(scan[after_idx]);
            if before_ok && after_ok {
                return Some((start, needle.len()));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// MT-181 FolderTreeAndColorLabels
//
// Master Spec §7.1.4.3 / MT-181: a persistent Loom folder hierarchy with color
// labels, sort modes, and project membership ("links are the new folders" but
// an explicit tree is still offered). Authority = PostgreSQL (loom_folders +
// loom_folder_members). An organizational overlay over LoomBlocks, never a
// second source of block truth.
// ---------------------------------------------------------------------------

/// Sort mode for a folder's contents.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoomFolderSortMode {
    NameAsc,
    NameDesc,
    CreatedDesc,
    UpdatedDesc,
    Manual,
}

impl LoomFolderSortMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoomFolderSortMode::NameAsc => "name_asc",
            LoomFolderSortMode::NameDesc => "name_desc",
            LoomFolderSortMode::CreatedDesc => "created_desc",
            LoomFolderSortMode::UpdatedDesc => "updated_desc",
            LoomFolderSortMode::Manual => "manual",
        }
    }
}

impl FromStr for LoomFolderSortMode {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "name_asc" => Ok(LoomFolderSortMode::NameAsc),
            "name_desc" => Ok(LoomFolderSortMode::NameDesc),
            "created_desc" => Ok(LoomFolderSortMode::CreatedDesc),
            "updated_desc" => Ok(LoomFolderSortMode::UpdatedDesc),
            "manual" => Ok(LoomFolderSortMode::Manual),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid loom folder sort_mode",
            )),
        }
    }
}

/// A Loom folder node (one row of `loom_folders`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoomFolder {
    pub folder_id: String,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_folder_id: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub sort_mode: LoomFolderSortMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create payload for a Loom folder.
#[derive(Clone, Debug)]
pub struct NewLoomFolder {
    pub folder_id: Option<String>,
    pub workspace_id: String,
    pub parent_folder_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub sort_mode: LoomFolderSortMode,
    pub sort_order: Option<i32>,
    pub project_ref: Option<String>,
}

/// Partial update for a Loom folder. `None` leaves a field unchanged; the
/// service-layer move/recolor endpoints decide which fields to send.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoomFolderUpdate {
    #[serde(default)]
    pub name: Option<String>,
    /// Wrap to distinguish "set color" (Some(Some)) from "clear color"
    /// (Some(None)) from "leave unchanged" (None).
    #[serde(default)]
    pub color: Option<Option<String>>,
    #[serde(default)]
    pub sort_mode: Option<LoomFolderSortMode>,
    #[serde(default)]
    pub sort_order: Option<Option<i32>>,
    /// Re-parent the folder (move within the tree). `Some(None)` makes it a
    /// root; `Some(Some(id))` nests it under `id` (cycle-checked).
    #[serde(default)]
    pub parent_folder_id: Option<Option<String>>,
    #[serde(default)]
    pub project_ref: Option<Option<String>>,
}

#[cfg(test)]
mod mt178_helper_tests {
    use super::*;

    #[test]
    fn snippet_windows_around_match_with_ellipses() {
        let text = "The quick brown fox jumps over the lazy dog and keeps running far away here";
        // match "fox" (start char 16, len 3)
        let start = text.find("fox").map(|b| text[..b].chars().count()).unwrap();
        let snip = loom_context_snippet(text, start, 3);
        assert!(snip.contains("fox"), "snippet keeps the match: {snip}");
        // short text on the left -> no leading ellipsis; long tail -> trailing ellipsis
        assert!(snip.starts_with("The"), "left context not clipped: {snip}");
        assert!(snip.ends_with("..."), "right context clipped: {snip}");
    }

    #[test]
    fn snippet_handles_match_at_start_and_short_text() {
        let snip = loom_context_snippet("hello world", 0, 5);
        assert_eq!(snip, "hello world");
        assert!(!snip.starts_with("..."));
        assert!(!snip.ends_with("..."));
    }

    #[test]
    fn snippet_never_splits_multibyte_chars() {
        let text = "café au lait résumé naïve façade";
        let start = text.chars().position(|c| c == 'r').unwrap();
        let snip = loom_context_snippet(text, start, 6);
        // valid UTF-8 string out (no panic, accents preserved)
        assert!(snip.contains("résumé"), "multibyte preserved: {snip}");
    }

    #[test]
    fn unlinked_term_matches_on_word_boundary_only() {
        // exact word match
        assert!(loom_find_unlinked_term("see the Roadmap today", "Roadmap").is_some());
        // case-insensitive
        assert!(loom_find_unlinked_term("the ROADMAP is set", "roadmap").is_some());
        // NOT a substring of a larger word
        assert!(loom_find_unlinked_term("planning is hard", "plan").is_none());
        assert!(loom_find_unlinked_term("the explanation", "plan").is_none());
        // empty term never matches
        assert!(loom_find_unlinked_term("anything", "").is_none());
        assert!(loom_find_unlinked_term("anything", "   ").is_none());
    }

    #[test]
    fn unlinked_term_returns_char_offset() {
        let text = "alpha beta gamma";
        let (start, len) = loom_find_unlinked_term(text, "beta").expect("match");
        assert_eq!(start, 6);
        assert_eq!(len, 4);
    }
}
