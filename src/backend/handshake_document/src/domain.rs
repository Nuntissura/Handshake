use chrono::{DateTime, Utc};
use handshake_storage_support::{StorageError, StorageResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSourceKind {
    File,
    Asset,
    RichDocument,
    LoomBlock,
    ExternalImport,
    OperatorArtifact,
}

impl KnowledgeSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Asset => "asset",
            Self::RichDocument => "rich_document",
            Self::LoomBlock => "loom_block",
            Self::ExternalImport => "external_import",
            Self::OperatorArtifact => "operator_artifact",
        }
    }
}

impl FromStr for KnowledgeSourceKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "file" => Ok(Self::File),
            "asset" => Ok(Self::Asset),
            "rich_document" => Ok(Self::RichDocument),
            "loom_block" => Ok(Self::LoomBlock),
            "external_import" => Ok(Self::ExternalImport),
            "operator_artifact" => Ok(Self::OperatorArtifact),
            _ => Err(StorageError::Validation("invalid knowledge source_kind")),
        }
    }
}

/// Parser status of a knowledge source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeParserStatus {
    Pending,
    Parsed,
    Failed,
    Skipped,
}

impl KnowledgeParserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Parsed => "parsed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl FromStr for KnowledgeParserStatus {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "parsed" => Ok(Self::Parsed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(StorageError::Validation("invalid knowledge parser_status")),
        }
    }
}

/// Extraction status of a knowledge source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeExtractionStatus {
    Pending,
    Extracted,
    Failed,
    Skipped,
}

impl KnowledgeExtractionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Extracted => "extracted",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl FromStr for KnowledgeExtractionStatus {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "extracted" => Ok(Self::Extracted),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(StorageError::Validation(
                "invalid knowledge extraction_status",
            )),
        }
    }
}

/// Permission scope of a knowledge source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgePermissionScope {
    Workspace,
    OperatorPrivate,
    Shared,
}

impl KnowledgePermissionScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::OperatorPrivate => "operator_private",
            Self::Shared => "shared",
        }
    }
}

impl FromStr for KnowledgePermissionScope {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "workspace" => Ok(Self::Workspace),
            "operator_private" => Ok(Self::OperatorPrivate),
            "shared" => Ok(Self::Shared),
            _ => Err(StorageError::Validation(
                "invalid knowledge permission_scope",
            )),
        }
    }
}

/// Redaction state of a knowledge source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRedactionState {
    None,
    Partial,
    Redacted,
}

impl KnowledgeRedactionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Partial => "partial",
            Self::Redacted => "redacted",
        }
    }
}

impl FromStr for KnowledgeRedactionState {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "partial" => Ok(Self::Partial),
            "redacted" => Ok(Self::Redacted),
            _ => Err(StorageError::Validation(
                "invalid knowledge redaction_state",
            )),
        }
    }
}

/// A registered knowledge source (file/asset/rich doc/Loom block/import).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeSource {
    pub source_id: String,
    pub workspace_id: String,
    pub root_id: Option<String>,
    pub source_kind: KnowledgeSourceKind,
    pub relative_path: Option<String>,
    pub asset_id: Option<String>,
    pub loom_block_id: Option<String>,
    pub document_id: Option<String>,
    pub content_hash: String,
    pub size_bytes: Option<i64>,
    pub provenance: Value,
    pub permission_scope: KnowledgePermissionScope,
    pub redaction_state: KnowledgeRedactionState,
    pub parser_status: KnowledgeParserStatus,
    pub extraction_status: KnowledgeExtractionStatus,
    pub stale: bool,
    pub last_index_receipt_event_id: Option<String>,
    pub source_modified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert/upsert payload for [`KnowledgeSource`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeSource {
    pub workspace_id: String,
    pub root_id: Option<String>,
    pub source_kind: KnowledgeSourceKind,
    pub relative_path: Option<String>,
    pub asset_id: Option<String>,
    pub loom_block_id: Option<String>,
    pub document_id: Option<String>,
    /// SHA-256 hex digest of the source content (lowercase, 64 chars).
    pub content_hash: String,
    pub size_bytes: Option<i64>,
    pub provenance: Value,
    pub permission_scope: KnowledgePermissionScope,
    pub redaction_state: KnowledgeRedactionState,
    pub source_modified_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEntityKind {
    Symbol,
    Concept,
    File,
    Folder,
    Project,
    Person,
    Role,
    Task,
    Api,
    Schema,
    Command,
    Media,
    ManualEntry,
    ProductPrimitive,
    SpecTopic,
    WorkPacket,
    MicroTask,
    TaskboardRow,
    RichDocument,
    LoomBlock,
    UserManualPage,
}

impl KnowledgeEntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Symbol => "symbol",
            Self::Concept => "concept",
            Self::File => "file",
            Self::Folder => "folder",
            Self::Project => "project",
            Self::Person => "person",
            Self::Role => "role",
            Self::Task => "task",
            Self::Api => "api",
            Self::Schema => "schema",
            Self::Command => "command",
            Self::Media => "media",
            Self::ManualEntry => "manual_entry",
            Self::ProductPrimitive => "product_primitive",
            Self::SpecTopic => "spec_topic",
            Self::WorkPacket => "work_packet",
            Self::MicroTask => "micro_task",
            Self::TaskboardRow => "taskboard_row",
            Self::RichDocument => "rich_document",
            Self::LoomBlock => "loom_block",
            Self::UserManualPage => "user_manual_page",
        }
    }

    pub fn all() -> &'static [KnowledgeEntityKind] {
        &[
            Self::Symbol,
            Self::Concept,
            Self::File,
            Self::Folder,
            Self::Project,
            Self::Person,
            Self::Role,
            Self::Task,
            Self::Api,
            Self::Schema,
            Self::Command,
            Self::Media,
            Self::ManualEntry,
            Self::ProductPrimitive,
            Self::SpecTopic,
            Self::WorkPacket,
            Self::MicroTask,
            Self::TaskboardRow,
            Self::RichDocument,
            Self::LoomBlock,
            Self::UserManualPage,
        ]
    }
}

impl FromStr for KnowledgeEntityKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::all()
            .iter()
            .find(|kind| kind.as_str() == value)
            .copied()
            .ok_or(StorageError::Validation("invalid knowledge entity_kind"))
    }
}

/// Lifecycle of a knowledge entity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEntityLifecycle {
    Active,
    Retired,
}

impl KnowledgeEntityLifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }
}

impl FromStr for KnowledgeEntityLifecycle {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "retired" => Ok(Self::Retired),
            _ => Err(StorageError::Validation(
                "invalid knowledge entity lifecycle_state",
            )),
        }
    }
}

/// A typed knowledge entity with stable (workspace, kind, key) identity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeEntity {
    pub entity_id: String,
    pub workspace_id: String,
    pub entity_kind: KnowledgeEntityKind,
    pub entity_key: String,
    pub display_name: String,
    pub detection_provenance: Value,
    pub lifecycle_state: KnowledgeEntityLifecycle,
    pub primary_source_id: Option<String>,
    pub first_detected_in_run: Option<String>,
    pub last_detected_in_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for [`KnowledgeEntity`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeEntity {
    pub workspace_id: String,
    pub entity_kind: KnowledgeEntityKind,
    pub entity_key: String,
    pub display_name: String,
    pub detection_provenance: Value,
    pub primary_source_id: Option<String>,
    pub detected_in_run: Option<String>,
    /// Detection evidence: span ids this entity was detected from.
    pub evidence_span_ids: Vec<String>,
}

/// A versioned ProseMirror/Tiptap document JSON authority record.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRichDocument {
    pub rich_document_id: String,
    /// Stable Loom address for this document. RichDocument identity and its
    /// LoomBlock projection deliberately share one id.
    pub block_id: String,
    pub workspace_id: String,
    /// Optional anchor to the legacy `documents` surface.
    pub document_id: Option<String>,
    pub title: String,
    /// ProseMirror/Tiptap schema version token (e.g. `hsk_richdoc_v1`).
    pub schema_version: String,
    pub doc_version: i64,
    /// The document JSON authority (ProseMirror doc node).
    pub content_json: Value,
    /// sha256 over the canonical JSON of `content_json`.
    pub content_sha256: String,
    /// Soft refs into kernel CRDT storage (composite PK there; the CRDT
    /// promotion bridge owns that integrity).
    pub crdt_document_id: Option<String>,
    pub crdt_snapshot_id: Option<String>,
    /// EventLedger promotion receipt for the CURRENT revision.
    pub promotion_receipt_event_id: Option<String>,
    /// Outbound projection refs: `[{"projection_id": "KWP-..."}, ...]`.
    pub projection_refs: Value,
    /// MT-145 RichDocumentIdentityModel: project membership (a stable project
    /// id / token, never an absolute path).
    pub project_ref: Option<String>,
    /// MT-145: folder membership (a stable, workspace-relative folder token,
    /// never an absolute path).
    pub folder_ref: Option<String>,
    /// MT-145: authority classification (`draft` | `promoted` | `archived`).
    pub authority_label: String,
    /// MT-145: owning actor kind (operator/local_model/cloud_model/validator/
    /// system); all-or-nothing with `owner_actor_id`.
    pub owner_actor_kind: Option<String>,
    /// MT-145: owning actor id.
    pub owner_actor_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeRichDocument`].
#[derive(Clone, Debug, Default, Serialize)]
pub struct NewKnowledgeRichDocument {
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub title: String,
    pub schema_version: String,
    pub content_json: Value,
    pub crdt_document_id: Option<String>,
    pub crdt_snapshot_id: Option<String>,
    pub promotion_receipt_event_id: Option<String>,
    /// MT-145 RichDocumentIdentityModel fields. Defaults: no project/folder, an
    /// `promoted` authority label, no owner. Use
    /// [`NewKnowledgeRichDocument::with_identity`] to set them.
    #[serde(default)]
    pub project_ref: Option<String>,
    #[serde(default)]
    pub folder_ref: Option<String>,
    /// `draft` | `promoted` | `archived`; defaults to `promoted` when empty.
    #[serde(default)]
    pub authority_label: Option<String>,
    #[serde(default)]
    pub owner_actor_kind: Option<String>,
    #[serde(default)]
    pub owner_actor_id: Option<String>,
}

/// One promoted revision in the append-only version history.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRichDocumentVersion {
    pub rich_document_id: String,
    pub doc_version: i64,
    pub schema_version: String,
    pub content_json: Value,
    pub content_sha256: String,
    pub crdt_snapshot_id: Option<String>,
    pub promotion_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Version-history METADATA without the content body (adversarial-v2 MT-156:
/// the history list endpoint must not return every version's full
/// `content_json` — that is a response-size DoS on long-lived documents). A
/// single version body is lazily loaded through
/// [`KnowledgeStore::get_knowledge_rich_document_version`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRichDocumentVersionMeta {
    pub rich_document_id: String,
    pub doc_version: i64,
    pub schema_version: String,
    pub content_sha256: String,
    pub crdt_snapshot_id: Option<String>,
    pub promotion_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Backend-persisted unsaved editor draft for crash recovery (MT-255). This is
/// support state, not a promoted RichDocument revision.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRichDocumentDraft {
    pub rich_document_id: String,
    pub workspace_id: String,
    pub base_doc_version: i64,
    pub base_content_sha256: String,
    pub draft_content_json: Value,
    pub draft_content_sha256: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpsertKnowledgeRichDocumentDraft {
    pub rich_document_id: String,
    pub base_doc_version: i64,
    pub base_content_sha256: String,
    pub content_json: Value,
    pub actor_kind: String,
    pub actor_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
}

/// A Monaco-backed code block embedded in a RichDocument.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeEditorCodeNode {
    pub code_node_id: String,
    pub rich_document_id: String,
    /// Stable node path inside the document block tree (e.g. `body.3.code`).
    pub node_path: String,
    pub language_id: String,
    pub code_text: String,
    /// sha256 over `code_text`: the editor round-trip integrity hash. A
    /// Monaco mount/unmount cycle must reproduce this hash or the round-trip
    /// failed.
    pub round_trip_sha256: String,
    /// Worker/bundling requirements: `{"worker": "ts", "bundled": true}`.
    pub worker_requirements: Value,
    /// Source mapping back into project sources, when the block mirrors one.
    pub source_mapping: Option<Value>,
    pub lint_diagnostics: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for [`KnowledgeEditorCodeNode`]; the round-trip hash is
/// always recomputed from the exact code text.
#[derive(Clone, Debug, Serialize)]
pub struct UpsertEditorCodeNode {
    pub rich_document_id: String,
    pub node_path: String,
    pub language_id: String,
    pub code_text: String,
    pub worker_requirements: Value,
    pub source_mapping: Option<Value>,
    pub lint_diagnostics: Value,
}

// ---------------------------------------------------------------------------
// MT-152 EmbedReferenceModel + MT-153 BrokenEmbedRepairState:
// knowledge_document_embeds (migration 0281). Embeds are TYPED references
// (artifact/media/source id or typed http(s) URL), never absolute paths; a
// missing target is a repairable 'broken' row with a reason.
// ---------------------------------------------------------------------------

/// A typed embed reference attached to a document embed block (MT-152/153).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeDocumentEmbed {
    pub embed_id: String,
    pub rich_document_id: String,
    /// MT-148 stable block id of the embed block.
    pub block_id: String,
    /// `artifact` | `media` | `source` | `url`.
    pub ref_kind: String,
    /// The id or typed http(s) URL; never an absolute path (DB-enforced).
    pub ref_value: String,
    pub caption: Option<String>,
    /// `ok` | `broken` (MT-153).
    pub repair_state: String,
    pub repair_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for a document embed (MT-152). The `repair_state`/`reason`
/// are set through the dedicated repair-state method, not on upsert.
#[derive(Clone, Debug, Serialize)]
pub struct UpsertKnowledgeDocumentEmbed {
    pub rich_document_id: String,
    pub block_id: String,
    pub ref_kind: String,
    pub ref_value: String,
    pub caption: Option<String>,
}

// ---------------------------------------------------------------------------
// MT-155 DocumentBacklinkBridge: knowledge_document_backlinks (migration
// 0282). Document-scoped backlinks keyed by a STABLE relationship_id derived
// from the document content (deterministic across re-extraction).
// ---------------------------------------------------------------------------

/// A persisted document backlink edge (MT-155).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeDocumentBacklink {
    pub backlink_id: String,
    pub workspace_id: String,
    /// Stable, deterministic across re-extraction (`KDLNK-...`).
    pub relationship_id: String,
    pub source_document_id: String,
    /// `file|folder|project|spec|wp|symbol|wikilink|mention|tag`.
    pub link_kind: String,
    pub target: String,
    /// MT-148 stable block id the reference came from.
    pub block_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for a document backlink (MT-155). The `relationship_id` is
/// supplied by the caller (derived in `knowledge_document::backlink`), and the
/// upsert is keyed on `(workspace_id, relationship_id)`.
#[derive(Clone, Debug, Serialize)]
pub struct UpsertKnowledgeDocumentBacklink {
    pub workspace_id: String,
    pub relationship_id: String,
    pub source_document_id: String,
    pub link_kind: String,
    pub target: String,
    pub block_id: String,
}

// ---------------------------------------------------------------------------
// MT-060 ContextBundleTables: durable bundle runs, per-item retrieval

pub fn new_knowledge_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::now_v7().simple())
}

pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
