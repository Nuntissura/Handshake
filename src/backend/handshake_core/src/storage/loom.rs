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
