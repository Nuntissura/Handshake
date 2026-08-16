//! WP-KERNEL-009 ProjectKnowledgeIndex storage (PostgresEventLedgerCore group,
//! MT-049..MT-064).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 "Project
//! Knowledge Index and Rich Document Authority" [ADD v02.192]. This module is
//! the durable PostgreSQL authority surface for the canonical record families
//! (KnowledgeSource, KnowledgeSpan, KnowledgeEntity, KnowledgeEdge,
//! KnowledgeClaim, MemoryPassage, RetrievalTrace, RichDocument,
//! EditorCodeNode) plus the WP-009 support surfaces (schema registry, index
//! runs, idempotency keys, wiki projections, context bundles).
//!
//! Why one file instead of touching `storage/postgres.rs` (kb003 precedent):
//! `postgres.rs` is the legacy single-file authority surface (~8.7k lines).
//! Keeping the WP-009 row types, SQL, and store trait in one reviewable unit
//! matches `storage/kb003_storage.rs` and keeps the MT contracts auditable.
//!
//! Trait purity (Master Spec 2.3.12.3): every method returns
//! `StorageResult<T>`; backend errors are converted to the opaque
//! `StorageError::Database` by the existing `From` impl, so no
//! provider-specific error type leaks. There is NO in-memory, SQLite, or
//! fixture fallback anywhere in this module: when PostgreSQL is unavailable
//! every method fails closed with a typed `StorageError` (MT-064).
//!
//! Namespace decision (MT-049): all tables use the `knowledge_` prefix in the
//! active schema; see migrations/0130_knowledge_schema_namespace.sql for the
//! full rationale recorded next to the boundary table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

use super::{StorageError, StorageResult};

/// Table prefix that defines the WP-009 PostgreSQL namespace boundary.
pub const KNOWLEDGE_TABLE_PREFIX: &str = "knowledge_";

// ---------------------------------------------------------------------------
// MT-049 KnowledgeSchemaNamespace: registry row + namespace verification.
// ---------------------------------------------------------------------------

/// One registered WP-009 table family (row of `knowledge_schema_registry`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeSchemaRegistryRow {
    pub family_key: String,
    pub table_name: String,
    pub record_family: String,
    pub authority_class: KnowledgeAuthorityClass,
    pub migration_file: String,
    pub wp_id: String,
    pub mt_id: String,
    pub registered_at: DateTime<Utc>,
}

/// Authority classification for a registered WP-009 table.
///
/// Spec 2.3.13.11: projections are NEVER authority. The registry records the
/// class so validators and the fail-closed guard can audit the boundary.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAuthorityClass {
    Authority,
    Projection,
    Support,
}

impl KnowledgeAuthorityClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authority => "authority",
            Self::Projection => "projection",
            Self::Support => "support",
        }
    }
}

impl FromStr for KnowledgeAuthorityClass {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "authority" => Ok(Self::Authority),
            "projection" => Ok(Self::Projection),
            "support" => Ok(Self::Support),
            _ => Err(StorageError::Validation(
                "invalid knowledge authority_class",
            )),
        }
    }
}

/// Result of the namespace boundary audit (MT-049 verification surface).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeNamespaceAudit {
    /// Registry rows currently present.
    pub registered: Vec<KnowledgeSchemaRegistryRow>,
    /// Registered tables that do not exist in the active schema.
    pub missing_tables: Vec<String>,
    /// `knowledge_`-prefixed tables present in the active schema that are not
    /// registered (namespace drift).
    pub unregistered_tables: Vec<String>,
}

impl KnowledgeNamespaceAudit {
    /// The namespace is sound when every registered table exists and no
    /// unregistered `knowledge_` table is present.
    pub fn is_sound(&self) -> bool {
        self.missing_tables.is_empty() && self.unregistered_tables.is_empty()
    }
}

// ---------------------------------------------------------------------------
// MT-050 ProjectSourceRootTables: managed project roots + allowlist policy.
// ---------------------------------------------------------------------------

/// Kind of a managed project root eligible for knowledge indexing.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRootKind {
    ProjectRepo,
    Governance,
    Artifacts,
    MediaLibrary,
    ExternalImport,
    OperatorFolder,
}

impl KnowledgeRootKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectRepo => "project_repo",
            Self::Governance => "governance",
            Self::Artifacts => "artifacts",
            Self::MediaLibrary => "media_library",
            Self::ExternalImport => "external_import",
            Self::OperatorFolder => "operator_folder",
        }
    }
}

impl FromStr for KnowledgeRootKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "project_repo" => Ok(Self::ProjectRepo),
            "governance" => Ok(Self::Governance),
            "artifacts" => Ok(Self::Artifacts),
            "media_library" => Ok(Self::MediaLibrary),
            "external_import" => Ok(Self::ExternalImport),
            "operator_folder" => Ok(Self::OperatorFolder),
            _ => Err(StorageError::Validation("invalid knowledge root_kind")),
        }
    }
}

/// Indexing eligibility of a source root.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexingEligibility {
    Eligible,
    Paused,
    Excluded,
}

impl KnowledgeIndexingEligibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Paused => "paused",
            Self::Excluded => "excluded",
        }
    }
}

impl FromStr for KnowledgeIndexingEligibility {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "eligible" => Ok(Self::Eligible),
            "paused" => Ok(Self::Paused),
            "excluded" => Ok(Self::Excluded),
            _ => Err(StorageError::Validation(
                "invalid knowledge indexing_eligibility",
            )),
        }
    }
}

/// A managed project root registered for knowledge indexing.
///
/// Path portability: `repo_relative_path` is a normalized repo-relative POSIX
/// path. Absolute path authority is rejected by both this module and the
/// `chk_knowledge_source_roots_path_portable` DB constraint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeSourceRoot {
    pub root_id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub root_kind: KnowledgeRootKind,
    pub repo_relative_path: String,
    pub path_normalization: String,
    pub allowlist_policy: Value,
    pub indexing_eligibility: KnowledgeIndexingEligibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeSourceRoot`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeSourceRoot {
    pub workspace_id: String,
    pub display_name: String,
    pub root_kind: KnowledgeRootKind,
    pub repo_relative_path: String,
    pub allowlist_policy: Value,
    pub indexing_eligibility: KnowledgeIndexingEligibility,
}

/// Normalizes and validates a repo-relative path for root/source authority.
///
/// Rules (mirror of `chk_knowledge_source_roots_path_portable`): forward
/// slashes only, no drive letter, no leading slash, no `..` escapes, no
/// surrounding whitespace. The empty string addresses the repo root itself.
pub fn normalize_repo_relative_path(path: &str) -> StorageResult<String> {
    let trimmed = path.trim();
    if trimmed != path {
        return Err(StorageError::Validation(
            "repo-relative path must not carry surrounding whitespace",
        ));
    }
    let normalized = trimmed.replace('\\', "/");
    let normalized = normalized.trim_end_matches('/').to_string();
    if normalized.chars().nth(1).map(|c| c == ':').unwrap_or(false) {
        return Err(StorageError::Validation(
            "absolute path authority is forbidden: drive letters are machine-local",
        ));
    }
    if normalized.starts_with('/') {
        return Err(StorageError::Validation(
            "absolute path authority is forbidden: paths must be repo-relative",
        ));
    }
    if normalized.split('/').any(|segment| segment == "..") {
        return Err(StorageError::Validation(
            "repo-relative path must not escape the root with '..'",
        ));
    }
    Ok(normalized)
}

fn new_knowledge_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

// ---------------------------------------------------------------------------
// MT-051 ProjectSourceFileTables: per-source records under managed roots.
// ---------------------------------------------------------------------------

/// Kind of an indexed knowledge source (spec 2.3.13.11 KnowledgeSource).
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

const KNOWLEDGE_SOURCE_COLUMNS: &str = r#"
    source_id, workspace_id, root_id, source_kind, relative_path,
    asset_id, loom_block_id, document_id, content_hash, size_bytes,
    provenance, permission_scope, redaction_state, parser_status,
    extraction_status, stale, last_index_receipt_event_id,
    source_modified_at, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-052 IndexRunLifecycleTables: durable index run lifecycle.
// ---------------------------------------------------------------------------

/// Lifecycle state of a knowledge index run.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexRunState {
    Started,
    Completed,
    Failed,
    Cancelled,
}

impl KnowledgeIndexRunState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Terminal states never transition again.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Started)
    }
}

impl FromStr for KnowledgeIndexRunState {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "started" => Ok(Self::Started),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(StorageError::Validation("invalid knowledge run_state")),
        }
    }
}

/// Result counters captured when an index run finishes.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeIndexRunCounts {
    pub sources_seen: i32,
    pub sources_indexed: i32,
    pub spans_extracted: i32,
    pub entities_detected: i32,
    pub edges_written: i32,
    pub claims_written: i32,
}

/// A durable knowledge index run record.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeIndexRun {
    pub index_run_id: String,
    pub workspace_id: String,
    pub root_id: Option<String>,
    pub run_state: KnowledgeIndexRunState,
    pub scope: Value,
    pub actor_kind: String,
    pub actor_id: String,
    pub worktree_id: Option<String>,
    pub restart_checkpoint: Option<Value>,
    pub counts: KnowledgeIndexRunCounts,
    pub error_capture: Option<Value>,
    pub start_receipt_event_id: Option<String>,
    pub finish_receipt_event_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Insert payload for [`KnowledgeIndexRun`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeIndexRun {
    pub workspace_id: String,
    pub root_id: Option<String>,
    pub scope: Value,
    pub actor_kind: String,
    pub actor_id: String,
    pub worktree_id: Option<String>,
    pub start_receipt_event_id: Option<String>,
}

/// Terminal outcome for [`KnowledgeStore::finish_knowledge_index_run`].
#[derive(Clone, Debug)]
pub enum KnowledgeIndexRunOutcome {
    Completed {
        counts: KnowledgeIndexRunCounts,
    },
    Failed {
        counts: KnowledgeIndexRunCounts,
        error_capture: Value,
    },
    Cancelled {
        counts: KnowledgeIndexRunCounts,
    },
}

impl KnowledgeIndexRunOutcome {
    fn state(&self) -> KnowledgeIndexRunState {
        match self {
            Self::Completed { .. } => KnowledgeIndexRunState::Completed,
            Self::Failed { .. } => KnowledgeIndexRunState::Failed,
            Self::Cancelled { .. } => KnowledgeIndexRunState::Cancelled,
        }
    }

    fn counts(&self) -> KnowledgeIndexRunCounts {
        match self {
            Self::Completed { counts }
            | Self::Failed { counts, .. }
            | Self::Cancelled { counts } => *counts,
        }
    }

    fn error_capture(&self) -> Option<&Value> {
        match self {
            Self::Failed { error_capture, .. } => Some(error_capture),
            _ => None,
        }
    }
}

const KNOWLEDGE_INDEX_RUN_COLUMNS: &str = r#"
    index_run_id, workspace_id, root_id, run_state, scope, actor_kind,
    actor_id, worktree_id, restart_checkpoint, sources_seen, sources_indexed,
    spans_extracted, entities_detected, edges_written, claims_written,
    error_capture, start_receipt_event_id, finish_receipt_event_id,
    started_at, finished_at
"#;

// ---------------------------------------------------------------------------
// MT-055 KnowledgeSpanTables: the minimum citeable evidence unit.
// ---------------------------------------------------------------------------

/// Kind of range a knowledge span addresses (spec 2.3.13.11 KnowledgeSpan).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSpanKind {
    Byte,
    Text,
    Ast,
    MediaTime,
    Page,
    Cell,
    RichDoc,
}

impl KnowledgeSpanKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Byte => "byte",
            Self::Text => "text",
            Self::Ast => "ast",
            Self::MediaTime => "media_time",
            Self::Page => "page",
            Self::Cell => "cell",
            Self::RichDoc => "rich_doc",
        }
    }
}

impl FromStr for KnowledgeSpanKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "byte" => Ok(Self::Byte),
            "text" => Ok(Self::Text),
            "ast" => Ok(Self::Ast),
            "media_time" => Ok(Self::MediaTime),
            "page" => Ok(Self::Page),
            "cell" => Ok(Self::Cell),
            "rich_doc" => Ok(Self::RichDoc),
            _ => Err(StorageError::Validation("invalid knowledge span_kind")),
        }
    }
}

/// A citeable evidence span anchored to a [`KnowledgeSource`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeSpan {
    pub span_id: String,
    pub source_id: String,
    pub span_kind: KnowledgeSpanKind,
    pub range_start: i64,
    pub range_end: i64,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub section_path: Option<String>,
    pub content_sha256: String,
    pub parser_version: String,
    pub extraction_receipt_event_id: Option<String>,
    pub index_run_id: Option<String>,
    pub display_snippet: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeSpan`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeSpan {
    pub source_id: String,
    pub span_kind: KnowledgeSpanKind,
    pub range_start: i64,
    pub range_end: i64,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub section_path: Option<String>,
    /// SHA-256 hex of the exact span content.
    pub content_sha256: String,
    pub parser_version: String,
    pub extraction_receipt_event_id: Option<String>,
    pub index_run_id: Option<String>,
    pub display_snippet: Option<String>,
}

const KNOWLEDGE_SPAN_COLUMNS: &str = r#"
    span_id, source_id, span_kind, range_start, range_end, line_start,
    line_end, section_path, content_sha256, parser_version,
    extraction_receipt_event_id, index_run_id, display_snippet, created_at
"#;

// ---------------------------------------------------------------------------
// MT-053 KnowledgeEntityTables: typed entities detected from spans.
// ---------------------------------------------------------------------------

/// Typed entity kinds (spec 2.3.13.11 KnowledgeEntity + MT-053 contract).
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

const KNOWLEDGE_ENTITY_COLUMNS: &str = r#"
    entity_id, workspace_id, entity_kind, entity_key, display_name,
    detection_provenance, lifecycle_state, primary_source_id,
    first_detected_in_run, last_detected_in_run, created_at, updated_at
"#;

/// `KNOWLEDGE_ENTITY_COLUMNS` qualified with an `e.` table alias for joined
/// selects (column names in the result stay unqualified, so `entity_from_pg`
/// reads them unchanged).
const KNOWLEDGE_ENTITY_COLUMNS_E: &str = r#"
    e.entity_id, e.workspace_id, e.entity_kind, e.entity_key, e.display_name,
    e.detection_provenance, e.lifecycle_state, e.primary_source_id,
    e.first_detected_in_run, e.last_detected_in_run, e.created_at, e.updated_at
"#;

// ---------------------------------------------------------------------------
// MT-054 KnowledgeEdgeTables: typed relationships with stable relationship_id.
// ---------------------------------------------------------------------------

/// Typed relationship kinds between knowledge entities.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEdgeType {
    Defines,
    References,
    Contains,
    DependsOn,
    Implements,
    Documents,
    Validates,
    DerivedFrom,
    Mentions,
    LinksTo,
    Supersedes,
    RelatesTo,
}

impl KnowledgeEdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Defines => "defines",
            Self::References => "references",
            Self::Contains => "contains",
            Self::DependsOn => "depends_on",
            Self::Implements => "implements",
            Self::Documents => "documents",
            Self::Validates => "validates",
            Self::DerivedFrom => "derived_from",
            Self::Mentions => "mentions",
            Self::LinksTo => "links_to",
            Self::Supersedes => "supersedes",
            Self::RelatesTo => "relates_to",
        }
    }
}

impl FromStr for KnowledgeEdgeType {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "defines" => Ok(Self::Defines),
            "references" => Ok(Self::References),
            "contains" => Ok(Self::Contains),
            "depends_on" => Ok(Self::DependsOn),
            "implements" => Ok(Self::Implements),
            "documents" => Ok(Self::Documents),
            "validates" => Ok(Self::Validates),
            "derived_from" => Ok(Self::DerivedFrom),
            "mentions" => Ok(Self::Mentions),
            "links_to" => Ok(Self::LinksTo),
            "supersedes" => Ok(Self::Supersedes),
            "relates_to" => Ok(Self::RelatesTo),
            _ => Err(StorageError::Validation("invalid knowledge edge_type")),
        }
    }
}

/// Lifecycle of a knowledge edge.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEdgeLifecycle {
    Proposed,
    Active,
    Conflicted,
    Retired,
}

impl KnowledgeEdgeLifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Conflicted => "conflicted",
            Self::Retired => "retired",
        }
    }
}

impl FromStr for KnowledgeEdgeLifecycle {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "active" => Ok(Self::Active),
            "conflicted" => Ok(Self::Conflicted),
            "retired" => Ok(Self::Retired),
            _ => Err(StorageError::Validation(
                "invalid knowledge edge lifecycle_state",
            )),
        }
    }
}

/// Derives the stable, deterministic `relationship_id` for a knowledge edge.
///
/// Stable across re-index runs because it hashes the entities' natural
/// identities (entity_kind + entity_key, MT-053), never row ids, timestamps,
/// or run ids — the same logical relationship re-extracted by any later index
/// run derives the same id.
///
/// Collision resistance (hardening, MT-054): entity keys are free text under a
/// single non-empty CHECK and legitimately contain the byte-level separators a
/// naive `a|b:c` join would use — file paths (`C:\...`), Rust FQNs
/// (`mod::item`), spec anchors, even literal `|`. A plain delimiter-joined
/// string is therefore NOT injective: edge `(file,"p") -> (folder,"x|folder:y")`
/// and edge `(file,"p|folder:x") -> (folder,"y")` would both flatten to the
/// same `...|file:p|folder:x|folder:y` and alias onto one `relationship_id`,
/// silently merging two distinct edges under
/// `UNIQUE (workspace_id, relationship_id)`.
///
/// The derivation is made injective by **length-prefixing every component**:
/// each field is emitted as `{byte_len}:{value}` and the fields are joined with
/// `|`. Because each value is preceded by its exact byte length, a parser (and
/// therefore the hash) can recover the original field boundaries unambiguously
/// no matter what bytes the value contains, so no choice of `|` or `:` inside
/// any entity key can ever produce the same canonical string as a different
/// tuple. The leading domain tag and the `_v2` version keep the namespace
/// stable and let the scheme be versioned if the framing ever changes.
///
/// Canonical preimage:
///
/// ```text
/// relationship_id = "KREL-" + sha256_hex(
///     "knowledge_edge_relationship_v2"
///     + "|" + len(edge_type)    + ":" + edge_type
///     + "|" + len(source_kind)  + ":" + source_kind
///     + "|" + len(source_key)   + ":" + source_key
///     + "|" + len(target_kind)  + ":" + target_kind
///     + "|" + len(target_key)   + ":" + target_key)
/// ```
///
/// where `len(x)` is the number of UTF-8 bytes in `x`. The derivation is
/// authoritative and mirrored in migrations/0136_knowledge_edges.sql.
///
/// NOTE: this v2 framing changes the hash of EVERY edge relative to the prior
/// unescaped-join scheme. That is intentional and safe on this pre-merge dev
/// branch (no production edge rows); the determinism/stability test
/// (`relationship_id_is_deterministic_across_reindex_runs`) asserts structure
/// and round-trip stability, never a frozen literal, so it still passes.
pub fn derive_knowledge_relationship_id(
    edge_type: KnowledgeEdgeType,
    source_kind: KnowledgeEntityKind,
    source_key: &str,
    target_kind: KnowledgeEntityKind,
    target_key: &str,
) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    // Length-prefixed, separator-injective canonical preimage. Each component
    // is framed as `{byte_len}:{value}`; the byte length restores field
    // boundaries regardless of which bytes the value contains, so no `|`/`:`
    // inside a free-text entity key can alias two distinct tuples.
    let mut canonical = String::from("knowledge_edge_relationship_v2");
    for component in [
        edge_type.as_str(),
        source_kind.as_str(),
        source_key,
        target_kind.as_str(),
        target_key,
    ] {
        // Infallible: writing to a String never errors.
        let _ = write!(canonical, "|{}:{}", component.len(), component);
    }

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("KREL-{}", hex::encode(hasher.finalize()))
}

/// A typed knowledge edge with REQUIRED span evidence.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeEdge {
    pub edge_id: String,
    pub workspace_id: String,
    pub relationship_id: String,
    pub edge_type: KnowledgeEdgeType,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub extractor_version: String,
    pub lifecycle_state: KnowledgeEdgeLifecycle,
    pub confidence: f64,
    pub conflict_marker: Option<Value>,
    pub created_in_run: Option<String>,
    pub last_seen_in_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for [`KnowledgeEdge`]. The relationship_id is derived, not
/// supplied: callers cannot break determinism.
#[derive(Clone, Debug)]
pub struct NewKnowledgeEdge {
    pub workspace_id: String,
    pub edge_type: KnowledgeEdgeType,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub extractor_version: String,
    pub confidence: f64,
    pub detected_in_run: Option<String>,
    /// REQUIRED evidence: at least one span id (spec 2.3.13.11).
    pub evidence_span_ids: Vec<String>,
}

const KNOWLEDGE_EDGE_COLUMNS: &str = r#"
    edge_id, workspace_id, relationship_id, edge_type, source_entity_id,
    target_entity_id, extractor_version, lifecycle_state, confidence,
    conflict_marker, created_in_run, last_seen_in_run, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-056 KnowledgeClaimTables: claims with lifecycle + evidence lineage.
// ---------------------------------------------------------------------------

/// Claim subject kind (spec: "an assertion about a source, product behavior,
/// task, or operator workflow").
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeClaimKind {
    SourceFact,
    ProductBehavior,
    TaskState,
    OperatorWorkflow,
}

impl KnowledgeClaimKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceFact => "source_fact",
            Self::ProductBehavior => "product_behavior",
            Self::TaskState => "task_state",
            Self::OperatorWorkflow => "operator_workflow",
        }
    }
}

impl FromStr for KnowledgeClaimKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "source_fact" => Ok(Self::SourceFact),
            "product_behavior" => Ok(Self::ProductBehavior),
            "task_state" => Ok(Self::TaskState),
            "operator_workflow" => Ok(Self::OperatorWorkflow),
            _ => Err(StorageError::Validation("invalid knowledge claim_kind")),
        }
    }
}

/// Spec-canonical claim lifecycle (2.3.13.11).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeClaimState {
    Proposed,
    Accepted,
    Conflicted,
    Retired,
}

impl KnowledgeClaimState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Conflicted => "conflicted",
            Self::Retired => "retired",
        }
    }

    /// Allowed lifecycle transitions (documented in 0137 migration header):
    /// proposed -> accepted|conflicted|retired; accepted -> conflicted|retired;
    /// conflicted -> accepted|retired; retired -> terminal.
    pub fn can_transition_to(&self, to: KnowledgeClaimState) -> bool {
        matches!(
            (self, to),
            (Self::Proposed, Self::Accepted)
                | (Self::Proposed, Self::Conflicted)
                | (Self::Proposed, Self::Retired)
                | (Self::Accepted, Self::Conflicted)
                | (Self::Accepted, Self::Retired)
                | (Self::Conflicted, Self::Accepted)
                | (Self::Conflicted, Self::Retired)
        )
    }
}

impl FromStr for KnowledgeClaimState {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "conflicted" => Ok(Self::Conflicted),
            "retired" => Ok(Self::Retired),
            _ => Err(StorageError::Validation(
                "invalid knowledge claim lifecycle_state",
            )),
        }
    }
}

/// Why a claim was retired (MT-056 contract: rejected/superseded qualifiers).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeClaimRetirementReason {
    Rejected,
    Superseded,
    Stale,
    OperatorRetired,
}

impl KnowledgeClaimRetirementReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Stale => "stale",
            Self::OperatorRetired => "operator_retired",
        }
    }
}

impl FromStr for KnowledgeClaimRetirementReason {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            "stale" => Ok(Self::Stale),
            "operator_retired" => Ok(Self::OperatorRetired),
            _ => Err(StorageError::Validation(
                "invalid knowledge claim retirement_reason",
            )),
        }
    }
}

/// A knowledge claim with evidence lineage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeClaim {
    pub claim_id: String,
    pub workspace_id: String,
    pub claim_kind: KnowledgeClaimKind,
    pub claim_text: String,
    pub subject_entity_id: Option<String>,
    pub lifecycle_state: KnowledgeClaimState,
    pub temporal_qualifier: Option<Value>,
    pub granularity_qualifier: Option<String>,
    pub confidence: f64,
    pub retirement_reason: Option<KnowledgeClaimRetirementReason>,
    pub superseded_by_claim_id: Option<String>,
    pub proposed_in_run: Option<String>,
    pub resolution_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeClaim`] (born `proposed`).
#[derive(Clone, Debug)]
pub struct NewKnowledgeClaim {
    pub workspace_id: String,
    pub claim_kind: KnowledgeClaimKind,
    pub claim_text: String,
    pub subject_entity_id: Option<String>,
    pub temporal_qualifier: Option<Value>,
    pub granularity_qualifier: Option<String>,
    pub confidence: f64,
    pub proposed_in_run: Option<String>,
    /// REQUIRED evidence: at least one span id (spec 2.3.13.11).
    pub evidence_span_ids: Vec<String>,
}

/// Terminal transition payload for claims entering `retired`.
#[derive(Clone, Debug)]
pub struct KnowledgeClaimRetirement {
    pub reason: KnowledgeClaimRetirementReason,
    /// Required when reason is `Superseded`.
    pub superseded_by_claim_id: Option<String>,
}

/// A recorded conflict between two claims.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeClaimConflict {
    pub conflict_id: String,
    pub claim_id: String,
    pub conflicting_claim_id: String,
    pub detected_in_run: Option<String>,
    pub conflict_reason: String,
    pub resolution_receipt_event_id: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

const KNOWLEDGE_CLAIM_COLUMNS: &str = r#"
    claim_id, workspace_id, claim_kind, claim_text, subject_entity_id,
    lifecycle_state, temporal_qualifier, granularity_qualifier, confidence,
    retirement_reason, superseded_by_claim_id, proposed_in_run,
    resolution_receipt_event_id, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-057 PassageEvidenceTables: MemoryPassage records with derivation lineage.
// ---------------------------------------------------------------------------

/// Retrieval mode vocabulary (spec 2.3.14.1.4 / RetrievalTrace).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRetrievalMode {
    None,
    DirectLoad,
    ExactLookup,
    GraphTraversal,
    HybridRag,
}

impl KnowledgeRetrievalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::DirectLoad => "direct_load",
            Self::ExactLookup => "exact_lookup",
            Self::GraphTraversal => "graph_traversal",
            Self::HybridRag => "hybrid_rag",
        }
    }
}

impl FromStr for KnowledgeRetrievalMode {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "direct_load" => Ok(Self::DirectLoad),
            "exact_lookup" => Ok(Self::ExactLookup),
            "graph_traversal" => Ok(Self::GraphTraversal),
            "hybrid_rag" => Ok(Self::HybridRag),
            _ => Err(StorageError::Validation("invalid knowledge retrieval_mode")),
        }
    }
}

/// Compaction policy of a memory passage.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCompactionPolicy {
    Keep,
    Compactable,
    Expired,
}

impl KnowledgeCompactionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Compactable => "compactable",
            Self::Expired => "expired",
        }
    }
}

impl FromStr for KnowledgeCompactionPolicy {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "keep" => Ok(Self::Keep),
            "compactable" => Ok(Self::Compactable),
            "expired" => Ok(Self::Expired),
            _ => Err(StorageError::Validation(
                "invalid knowledge compaction_policy",
            )),
        }
    }
}

/// One derivation-lineage ref of a memory passage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "ref_kind", rename_all = "snake_case")]
pub enum KnowledgePassageEvidenceRef {
    Source { source_id: String },
    Claim { claim_id: String },
    Span { span_id: String },
}

/// A bounded passage eligible for model context (spec MemoryPassage).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeMemoryPassage {
    pub passage_id: String,
    pub workspace_id: String,
    pub passage_text: String,
    pub token_count: Option<i32>,
    pub ocr_transcript_metadata: Option<Value>,
    pub extraction_confidence: f64,
    pub ranking_features: Value,
    pub retrieval_mode: KnowledgeRetrievalMode,
    pub freshness_at: DateTime<Utc>,
    pub compaction_policy: KnowledgeCompactionPolicy,
    pub failure_receipt_event_id: Option<String>,
    pub derived_in_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeMemoryPassage`].
///
/// `Serialize` exists so idempotent writes (MT-062) can derive a canonical
/// request hash from the exact payload.
#[derive(Clone, Debug, Serialize)]
pub struct NewKnowledgeMemoryPassage {
    pub workspace_id: String,
    pub passage_text: String,
    pub token_count: Option<i32>,
    pub ocr_transcript_metadata: Option<Value>,
    pub extraction_confidence: f64,
    pub ranking_features: Value,
    pub retrieval_mode: KnowledgeRetrievalMode,
    pub compaction_policy: KnowledgeCompactionPolicy,
    pub failure_receipt_event_id: Option<String>,
    pub derived_in_run: Option<String>,
    /// REQUIRED derivation lineage: at least one source/claim/span ref.
    pub evidence: Vec<KnowledgePassageEvidenceRef>,
}

const KNOWLEDGE_PASSAGE_COLUMNS: &str = r#"
    passage_id, workspace_id, passage_text, token_count,
    ocr_transcript_metadata, extraction_confidence, ranking_features,
    retrieval_mode, freshness_at, compaction_policy,
    failure_receipt_event_id, derived_in_run, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-058 WikiProjectionTables: derived, staleable, regenerable views.
//
// PROJECTIONS ARE NEVER AUTHORITY (spec 2.3.13.11). The registry classifies
// `knowledge_wiki_projections` as `projection`; no authority table carries an
// FK into it, and deleting a projection row mutates nothing else. A stale or
// deleted projection is simply rebuilt from canonical records.
// ---------------------------------------------------------------------------

/// Kind of a generated wiki/Loom projection.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeProjectionKind {
    WikiPage,
    LoomView,
    GraphView,
    ManualPage,
    OperatorSummary,
}

impl KnowledgeProjectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WikiPage => "wiki_page",
            Self::LoomView => "loom_view",
            Self::GraphView => "graph_view",
            Self::ManualPage => "manual_page",
            Self::OperatorSummary => "operator_summary",
        }
    }
}

impl FromStr for KnowledgeProjectionKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wiki_page" => Ok(Self::WikiPage),
            "loom_view" => Ok(Self::LoomView),
            "graph_view" => Ok(Self::GraphView),
            "manual_page" => Ok(Self::ManualPage),
            "operator_summary" => Ok(Self::OperatorSummary),
            _ => Err(StorageError::Validation(
                "invalid knowledge projection_kind",
            )),
        }
    }
}

/// Rebuild lifecycle of a projection.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRebuildStatus {
    Fresh,
    Stale,
    Rebuilding,
    Failed,
}

impl KnowledgeRebuildStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Rebuilding => "rebuilding",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for KnowledgeRebuildStatus {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "fresh" => Ok(Self::Fresh),
            "stale" => Ok(Self::Stale),
            "rebuilding" => Ok(Self::Rebuilding),
            "failed" => Ok(Self::Failed),
            _ => Err(StorageError::Validation("invalid knowledge rebuild_status")),
        }
    }
}

/// A generated wiki/Loom projection row. NEVER authority.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeWikiProjection {
    pub projection_id: String,
    pub workspace_id: String,
    pub projection_kind: KnowledgeProjectionKind,
    pub title: String,
    /// Stable refs into the authority records this projection renders:
    /// `[{"record_family": ..., "record_id": ...}, ...]`.
    pub source_records: Value,
    /// The rendered, regenerable content.
    pub rendered_content: String,
    pub rebuild_status: KnowledgeRebuildStatus,
    /// sha256 over the render inputs at render time; a mismatch against
    /// current authority state marks the projection stale.
    pub staleness_hash: String,
    pub rebuild_receipt_event_id: Option<String>,
    pub last_rebuilt_at: Option<DateTime<Utc>>,
    /// MT-241 (migration 0300): typed compiled page kind
    /// (`module|concept|flow|entity|decision|index`); `None` for untyped
    /// MT-184 Loom topic pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_type: Option<String>,
    /// MT-242 (LM-PWIKI-006) compile stamp: EventLedger source version + the
    /// exact cited-source set (ids + content hashes) the page compiled from.
    /// Structurally REQUIRED for typed pages
    /// (`chk_knowledge_wiki_projections_stamp_guard`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_stamp: Option<Value>,
    /// MT-243 deterministic compile-input descriptor so fan-out can
    /// regenerate one page from current authority without a full bootstrap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_recipe: Option<Value>,
    /// Outbound wikilinks `[{"title": ..., "projection_id": ...}]` (backlinks
    /// derive by reverse lookup).
    #[serde(default)]
    pub page_links: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for [`KnowledgeWikiProjection`].
#[derive(Clone, Debug, Serialize)]
pub struct NewKnowledgeWikiProjection {
    pub workspace_id: String,
    pub projection_kind: KnowledgeProjectionKind,
    pub title: String,
    pub source_records: Value,
    pub rendered_content: String,
    pub staleness_hash: String,
}

const KNOWLEDGE_PROJECTION_COLUMNS: &str = r#"
    projection_id, workspace_id, projection_kind, title, source_records,
    rendered_content, rebuild_status, staleness_hash,
    rebuild_receipt_event_id, last_rebuilt_at, page_type, compile_stamp,
    compile_recipe, page_links, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-059 RichDocumentTables: versioned RichDocument JSON authority +
// EditorCodeNode payloads (spec 2.3.13.11 RichDocument / EditorCodeNode).
//
// Versioning model: `knowledge_rich_documents` holds the CURRENT authority
// revision; `knowledge_rich_document_versions` is the append-only promoted
// revision history (v1 is recorded at creation). Saves are optimistic
// (expected_version) so concurrent writers fail closed with a typed
// `StorageError::Conflict` instead of overwriting each other.
// ---------------------------------------------------------------------------

/// sha256 over the canonical JSON encoding of a value (same canonical form
/// as kernel ContextBundle hashing, so content hashes are replayable).
fn knowledge_canonical_json_sha256(content: &Value) -> String {
    crate::kernel::context_bundle::sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(
        content,
    ))
}

fn normalized_locus_link_search_value(ref_kind: Option<&str>, ref_value: &str) -> Option<String> {
    if ref_kind != Some("locus") {
        return None;
    }
    let trimmed = ref_value.trim();
    let (kind, id) = if let Some(rest) = trimmed.strip_prefix("locus://") {
        let (kind, id) = rest.split_once('/')?;
        (kind, id)
    } else if let Some((kind, id)) = trimmed.split_once('/') {
        (kind, id)
    } else {
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("WP-") {
            ("wp", trimmed)
        } else if upper.starts_with("MT-") {
            ("mt", trimmed)
        } else {
            return None;
        }
    };
    let kind = kind.trim().to_ascii_lowercase();
    if kind != "wp" && kind != "mt" {
        return None;
    }
    let normalized_id = id
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    (!normalized_id.is_empty()).then(|| format!("locus://{kind}/{normalized_id}"))
}

fn collect_rich_document_link_search_values(node: &Value, values: &mut Vec<String>) {
    if node.get("type").and_then(Value::as_str) == Some("hsLink") {
        let attrs = node.get("attrs");
        let ref_kind = attrs
            .and_then(|attrs| attrs.get("refKind"))
            .and_then(Value::as_str);
        if let Some(value) = attrs
            .and_then(|attrs| attrs.get("refValue"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            values.push(value.to_string());
            if let Some(normalized) = normalized_locus_link_search_value(ref_kind, value) {
                values.push(normalized);
            }
        }
    }
    if let Some(children) = node.get("content").and_then(Value::as_array) {
        for child in children {
            collect_rich_document_link_search_values(child, values);
        }
    }
}

fn rich_document_loom_projection(
    title: &str,
    content_json: &Value,
) -> StorageResult<(String, String)> {
    let full_text = crate::knowledge_document::block_tree::extract_plain_text(content_json);
    let full_text = full_text.trim().to_string();
    let derived = super::LoomBlockDerived {
        full_text_index: (!full_text.is_empty()).then(|| full_text.clone()),
        ..super::LoomBlockDerived::default()
    };
    // `extract_plain_text` intentionally indexes the operator-facing link label. Reverse lookup also
    // needs the persisted structured identity (`refValue`), because compact chip labels such as "WP" or
    // "MT" do not contain `locus://wp/...` / `locus://mt/...`. Prefix-stripped authored Locus values are
    // indexed alongside their canonical normalized URI. Keep these values search-only so rendered text and
    // preview semantics remain unchanged.
    let mut link_values = Vec::new();
    collect_rich_document_link_search_values(content_json, &mut link_values);
    link_values.sort();
    link_values.dedup();
    let mut search_parts = vec![title.to_string()];
    if !full_text.is_empty() {
        search_parts.push(full_text);
    }
    search_parts.extend(link_values);
    let search_text = search_parts.join("\n");
    Ok((serde_json::to_string(&derived)?, search_text))
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

const KNOWLEDGE_RICH_DOCUMENT_COLUMNS: &str = r#"
    rich_document_id, workspace_id, document_id, title, schema_version,
    doc_version, content_json, content_sha256, crdt_document_id,
    crdt_snapshot_id, promotion_receipt_event_id, projection_refs,
    project_ref, folder_ref, authority_label, owner_actor_kind, owner_actor_id,
    created_at, updated_at
"#;

const KNOWLEDGE_RICH_DOCUMENT_DRAFT_COLUMNS: &str = r#"
    rich_document_id, workspace_id, base_doc_version, base_content_sha256,
    draft_content_json, draft_content_sha256, actor_kind, actor_id,
    kernel_task_run_id, session_run_id, created_at, updated_at
"#;

const KNOWLEDGE_CODE_NODE_COLUMNS: &str = r#"
    code_node_id, rich_document_id, node_path, language_id, code_text,
    round_trip_sha256, worker_requirements, source_mapping, lint_diagnostics,
    created_at, updated_at
"#;

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

const KNOWLEDGE_DOCUMENT_EMBED_COLUMNS: &str = r#"
    embed_id, rich_document_id, block_id, ref_kind, ref_value, caption,
    repair_state, repair_reason, created_at, updated_at
"#;

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

const KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS: &str = r#"
    backlink_id, workspace_id, relationship_id, source_document_id, link_kind,
    target, block_id, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// MT-060 ContextBundleTables: durable bundle runs, per-item retrieval
// decisions, token budgets, citations, and replayable RetrievalTraces.
//
// The BUNDLE CONTENT is a projection (spec 2.3.13.11); these tables are the
// durable RUN/DECISION evidence, which is authority. Bundles persist the
// exact kernel ContextBundle V1 shape: bundle_id is derived from the
// canonical-JSON content hash (CTX- + first 16 hex), enforced by a DB CHECK.
// ---------------------------------------------------------------------------

/// What a context-bundle item points at.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBundleItemRefKind {
    Source,
    Span,
    Claim,
    Passage,
    Entity,
}

impl KnowledgeBundleItemRefKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Span => "span",
            Self::Claim => "claim",
            Self::Passage => "passage",
            Self::Entity => "entity",
        }
    }
}

impl FromStr for KnowledgeBundleItemRefKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "source" => Ok(Self::Source),
            "span" => Ok(Self::Span),
            "claim" => Ok(Self::Claim),
            "passage" => Ok(Self::Passage),
            "entity" => Ok(Self::Entity),
            _ => Err(StorageError::Validation(
                "invalid knowledge bundle item ref_kind",
            )),
        }
    }
}

/// Per-item retrieval decision inside a bundle build.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBundleItemDecision {
    Included,
    ExcludedBudget,
    ExcludedRelevance,
    ExcludedRedacted,
}

impl KnowledgeBundleItemDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Included => "included",
            Self::ExcludedBudget => "excluded_budget",
            Self::ExcludedRelevance => "excluded_relevance",
            Self::ExcludedRedacted => "excluded_redacted",
        }
    }
}

impl FromStr for KnowledgeBundleItemDecision {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "included" => Ok(Self::Included),
            "excluded_budget" => Ok(Self::ExcludedBudget),
            "excluded_relevance" => Ok(Self::ExcludedRelevance),
            "excluded_redacted" => Ok(Self::ExcludedRedacted),
            _ => Err(StorageError::Validation(
                "invalid knowledge bundle item retrieval_decision",
            )),
        }
    }
}

/// A persisted context bundle run (kernel ContextBundle V1 shape + retrieval
/// evidence).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeContextBundle {
    pub bundle_id: String,
    pub workspace_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub allowed_context: Value,
    pub context_hash: String,
    pub query_text: Option<String>,
    pub token_budget: Option<i32>,
    pub tokens_used: Option<i32>,
    pub build_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// One recorded item decision of a bundle build.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeContextBundleItem {
    pub bundle_id: String,
    pub item_ordinal: i32,
    pub ref_kind: KnowledgeBundleItemRefKind,
    pub ref_id: String,
    pub retrieval_decision: KnowledgeBundleItemDecision,
    pub relevance_score: Option<f64>,
    pub token_count: Option<i32>,
    pub citation: Option<String>,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
}

/// Insert payload for one bundle item (ordinal is assigned by position).
#[derive(Clone, Debug, Serialize)]
pub struct NewKnowledgeContextBundleItem {
    pub ref_kind: KnowledgeBundleItemRefKind,
    pub ref_id: String,
    pub retrieval_decision: KnowledgeBundleItemDecision,
    pub relevance_score: Option<f64>,
    pub token_count: Option<i32>,
    pub citation: Option<String>,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
}

/// Insert payload for a bundle run: the REAL kernel V1 bundle plus the WP-009
/// retrieval evidence.
#[derive(Clone, Debug)]
pub struct NewKnowledgeContextBundle {
    pub workspace_id: String,
    /// The kernel V1 bundle; persisted exactly as constructed (id, hash,
    /// run ids, allowed_context).
    pub bundle: crate::kernel::context_bundle::ContextBundle,
    pub query_text: Option<String>,
    pub token_budget: Option<i32>,
    pub tokens_used: Option<i32>,
    pub build_receipt_event_id: Option<String>,
    pub items: Vec<NewKnowledgeContextBundleItem>,
}

/// A replayable retrieval trace (spec 2.3.13.11 RetrievalTrace).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRetrievalTrace {
    pub trace_id: String,
    pub workspace_id: String,
    pub retrieval_mode: KnowledgeRetrievalMode,
    /// Spec MUST: why broader retrieval was used or skipped.
    pub mode_reason: String,
    pub query_text: Option<String>,
    pub bundle_id: Option<String>,
    /// Replayable decision log: `[{"step": ..., "action": ...}, ...]`.
    pub decisions: Value,
    pub trace_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeRetrievalTrace`].
#[derive(Clone, Debug, Serialize)]
pub struct NewKnowledgeRetrievalTrace {
    pub workspace_id: String,
    pub retrieval_mode: KnowledgeRetrievalMode,
    pub mode_reason: String,
    pub query_text: Option<String>,
    pub bundle_id: Option<String>,
    pub decisions: Value,
    pub trace_receipt_event_id: Option<String>,
}

const KNOWLEDGE_BUNDLE_COLUMNS: &str = r#"
    bundle_id, workspace_id, kernel_task_run_id, session_run_id,
    allowed_context, context_hash, query_text, token_budget, tokens_used,
    build_receipt_event_id, created_at
"#;

const KNOWLEDGE_TRACE_COLUMNS: &str = r#"
    trace_id, workspace_id, retrieval_mode, mode_reason, query_text,
    bundle_id, decisions, trace_receipt_event_id, created_at
"#;

// ---------------------------------------------------------------------------
// MT-062 TransactionalIdempotencyKeys: replay-safe knowledge mutations.
//
// Discipline (documented next to the table in 0142):
//   1. The caller supplies an idempotency_key; the request_hash is derived
//      here as sha256 over the canonical JSON of the exact request payload.
//   2. The write and the key row commit in ONE transaction.
//   3. A replay with the SAME key + SAME request_hash returns the prior
//      result without writing anything.
//   4. The SAME key with a DIFFERENT request_hash is a typed Conflict
//      (divergent duplicate), mirroring kernel_event_ledger semantics.
//   5. Two racing writers on one key: the loser's key insert hits
//      ON CONFLICT DO NOTHING (after blocking on the winner's commit), the
//      loser's whole transaction rolls back (no double-write), and the
//      winner's result is re-read and returned as a replay.
//
// Unique-constraint coverage for the four contract surfaces:
//   * parallel indexing  -> passage_write engine here (+ span/source unique
//     identities from MT-051/MT-055);
//   * editor saves       -> rich_document_save engine here (+ optimistic
//     doc_version);
//   * graph writes       -> deterministic relationship_id upsert (MT-054);
//   * bundle builds      -> content-derived bundle_id PK + id/hash CHECK
//     (MT-060).
// ---------------------------------------------------------------------------

/// Operation vocabulary of `knowledge_idempotency_keys.operation_kind`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIdempotentOperationKind {
    IndexRunStart,
    SourceUpsert,
    SpanWrite,
    EntityWrite,
    EdgeWrite,
    ClaimWrite,
    PassageWrite,
    ProjectionWrite,
    RichDocumentSave,
    BundleBuild,
}

impl KnowledgeIdempotentOperationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IndexRunStart => "index_run_start",
            Self::SourceUpsert => "source_upsert",
            Self::SpanWrite => "span_write",
            Self::EntityWrite => "entity_write",
            Self::EdgeWrite => "edge_write",
            Self::ClaimWrite => "claim_write",
            Self::PassageWrite => "passage_write",
            Self::ProjectionWrite => "projection_write",
            Self::RichDocumentSave => "rich_document_save",
            Self::BundleBuild => "bundle_build",
        }
    }
}

/// Outcome of an idempotent knowledge write.
#[derive(Clone, Debug, PartialEq)]
pub struct KnowledgeIdempotentWrite<T> {
    pub value: T,
    /// True when the idempotency key already existed and the prior result
    /// was returned without writing anything.
    pub replayed: bool,
}

/// sha256 over the canonical JSON of an idempotent request payload.
fn knowledge_request_hash<T: Serialize>(operation: &str, payload: &T) -> StorageResult<String> {
    let value = serde_json::to_value(payload)?;
    Ok(knowledge_canonical_json_sha256(&serde_json::json!({
        "operation": operation,
        "payload": value,
    })))
}

fn validate_knowledge_idempotency_key(idempotency_key: &str) -> StorageResult<()> {
    if idempotency_key.trim() != idempotency_key || idempotency_key.is_empty() {
        return Err(StorageError::Validation(
            "knowledge idempotency_key must be non-empty and trimmed",
        ));
    }
    Ok(())
}

const RICH_DOCUMENT_RESULT_REF_KIND: &str = "rich_document";
const RICH_DOCUMENT_VERSION_RESULT_REF_KIND: &str = "rich_document_version";

fn rich_document_version_result_ref_id(rich_document_id: &str, doc_version: i64) -> String {
    format!("{rich_document_id}:{doc_version}")
}

fn parse_rich_document_version_result_ref_id(ref_id: &str) -> StorageResult<(String, i64)> {
    let Some((rich_document_id, doc_version)) = ref_id.rsplit_once(':') else {
        return Err(StorageError::Validation(
            "rich document version idempotency result ref is malformed",
        ));
    };
    if rich_document_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "rich document version idempotency result ref is missing document id",
        ));
    }
    let doc_version = doc_version.parse::<i64>().map_err(|_| {
        StorageError::Validation(
            "rich document version idempotency result ref has malformed doc_version",
        )
    })?;
    Ok((rich_document_id.to_string(), doc_version))
}

fn rich_document_crdt_id_change_requested(
    existing_crdt_document_id: Option<&str>,
    requested_crdt_document_id: Option<&str>,
) -> bool {
    matches!(
        (existing_crdt_document_id, requested_crdt_document_id),
        (Some(existing), Some(requested)) if existing != requested
    )
}

// ---------------------------------------------------------------------------
// KnowledgeStore trait: the WP-009 storage surface.
// ---------------------------------------------------------------------------

/// WP-009 ProjectKnowledgeIndex storage operations.
///
/// WP-KERNEL-012 MT-136: the former `impl KnowledgeStore for PostgresDatabase`
/// was removed together with the physically deleted PostgreSQL backend. This
/// trait currently has NO implementor; the SurrealDB/EventLedger implementor
/// is still to be written under `storage/surreal/`. The removed PostgreSQL
/// bodies remain the reference for the required table/column shapes and are
/// recoverable from git at commit 1af216a1.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    // -- MT-049 namespace ---------------------------------------------------
    async fn list_knowledge_schema_registry(
        &self,
    ) -> StorageResult<Vec<KnowledgeSchemaRegistryRow>>;

    /// Audits the `knowledge_` namespace boundary in the active schema.
    async fn audit_knowledge_namespace(&self) -> StorageResult<KnowledgeNamespaceAudit>;

    // -- MT-050 source roots ------------------------------------------------
    async fn create_knowledge_source_root(
        &self,
        new_root: NewKnowledgeSourceRoot,
    ) -> StorageResult<KnowledgeSourceRoot>;

    async fn get_knowledge_source_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Option<KnowledgeSourceRoot>>;

    async fn list_knowledge_source_roots(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeSourceRoot>>;

    /// Updates eligibility (eligible/paused/excluded) and bumps `updated_at`.
    async fn set_knowledge_root_eligibility(
        &self,
        root_id: &str,
        eligibility: KnowledgeIndexingEligibility,
    ) -> StorageResult<KnowledgeSourceRoot>;

    // -- MT-051 sources -------------------------------------------------------
    /// Registers or refreshes a knowledge source. File-kind sources upsert on
    /// `(root_id, relative_path)`: a re-index with a new content hash updates
    /// the row in place, resets parser/extraction status to `pending`, and
    /// clears the stale marker.
    async fn upsert_knowledge_source(
        &self,
        new_source: NewKnowledgeSource,
    ) -> StorageResult<KnowledgeSource>;

    async fn get_knowledge_source(&self, source_id: &str)
        -> StorageResult<Option<KnowledgeSource>>;

    /// Looks up the knowledge source indexing a RichDocument (adversarial-v2
    /// MT-154: documents are first-class Project-Knowledge-Index sources; the
    /// document save path keeps this row fresh / stale-marked).
    async fn get_knowledge_source_by_document_id(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> StorageResult<Option<KnowledgeSource>>;

    async fn list_knowledge_sources_for_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Vec<KnowledgeSource>>;

    /// Marks a source stale (content changed since last index).
    async fn mark_knowledge_source_stale(&self, source_id: &str) -> StorageResult<KnowledgeSource>;

    /// Records the index receipt for a source: parser/extraction outcome plus
    /// the EventLedger receipt ref (FK-enforced replayable evidence).
    async fn record_knowledge_source_index_receipt(
        &self,
        source_id: &str,
        parser_status: KnowledgeParserStatus,
        extraction_status: KnowledgeExtractionStatus,
        receipt_event_id: &str,
    ) -> StorageResult<KnowledgeSource>;

    // -- MT-052 index runs ----------------------------------------------------
    /// Starts a new index run in `started` state.
    async fn start_knowledge_index_run(
        &self,
        new_run: NewKnowledgeIndexRun,
    ) -> StorageResult<KnowledgeIndexRun>;

    async fn get_knowledge_index_run(
        &self,
        index_run_id: &str,
    ) -> StorageResult<Option<KnowledgeIndexRun>>;

    /// Persists a restart checkpoint on a still-running run.
    async fn checkpoint_knowledge_index_run(
        &self,
        index_run_id: &str,
        restart_checkpoint: Value,
    ) -> StorageResult<KnowledgeIndexRun>;

    /// Moves a run from `started` into a terminal state. Guarded: finishing a
    /// run that is not in `started` state is a typed `Conflict` (terminal
    /// states are terminal), enforced via an optimistic `WHERE run_state`.
    async fn finish_knowledge_index_run(
        &self,
        index_run_id: &str,
        outcome: KnowledgeIndexRunOutcome,
        finish_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIndexRun>;

    // -- MT-055 spans ---------------------------------------------------------
    async fn create_knowledge_span(
        &self,
        new_span: NewKnowledgeSpan,
    ) -> StorageResult<KnowledgeSpan>;

    async fn get_knowledge_span(&self, span_id: &str) -> StorageResult<Option<KnowledgeSpan>>;

    async fn list_knowledge_spans_for_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Vec<KnowledgeSpan>>;

    // -- MT-053 entities --------------------------------------------------------
    /// Upserts an entity on its stable (workspace, kind, key) identity and
    /// links the detection evidence spans transactionally. Re-detection in a
    /// later run keeps `entity_id` stable, refreshes provenance and
    /// `last_detected_in_run`, and merges new evidence spans.
    async fn upsert_knowledge_entity(
        &self,
        new_entity: NewKnowledgeEntity,
    ) -> StorageResult<KnowledgeEntity>;

    async fn get_knowledge_entity(&self, entity_id: &str)
        -> StorageResult<Option<KnowledgeEntity>>;

    async fn get_knowledge_entity_by_identity(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
        entity_key: &str,
    ) -> StorageResult<Option<KnowledgeEntity>>;

    async fn list_knowledge_entities_by_kind(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
    ) -> StorageResult<Vec<KnowledgeEntity>>;

    /// Lists the evidence span ids an entity was detected from.
    async fn list_knowledge_entity_span_ids(&self, entity_id: &str) -> StorageResult<Vec<String>>;

    /// Replaces the entity evidence spans scoped to one source and span kind.
    ///
    /// Normal entity upsert is merge-oriented because many knowledge entities
    /// are legitimately supported by multiple spans. Code definitions are
    /// different: a stable symbol re-detected in the same source must have one
    /// current AST definition span, not every historical location.
    async fn replace_knowledge_entity_spans_for_source_kind(
        &self,
        entity_id: &str,
        source_id: &str,
        span_kind: KnowledgeSpanKind,
        evidence_span_ids: &[String],
        detected_in_run: Option<&str>,
    ) -> StorageResult<()>;

    /// Marks an entity retired (it stops participating in new detection).
    async fn retire_knowledge_entity(&self, entity_id: &str) -> StorageResult<KnowledgeEntity>;

    // -- MT-054 edges -----------------------------------------------------------
    /// Upserts a typed edge with its REQUIRED span evidence in one
    /// transaction. The stable `relationship_id` is derived from
    /// (edge_type, source identity, target identity) — see
    /// [`derive_knowledge_relationship_id`] — so a re-extracted relationship
    /// updates the same row (confidence, extractor_version, last_seen_in_run)
    /// instead of duplicating it. Fails closed with a typed Validation error
    /// when `evidence_span_ids` is empty.
    async fn upsert_knowledge_edge(
        &self,
        new_edge: NewKnowledgeEdge,
    ) -> StorageResult<KnowledgeEdge>;

    async fn get_knowledge_edge(&self, edge_id: &str) -> StorageResult<Option<KnowledgeEdge>>;

    async fn get_knowledge_edge_by_relationship_id(
        &self,
        workspace_id: &str,
        relationship_id: &str,
    ) -> StorageResult<Option<KnowledgeEdge>>;

    /// Lists edges touching an entity (as source or target).
    async fn list_knowledge_edges_for_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Vec<KnowledgeEdge>>;

    /// Lists the evidence span ids attached to an edge.
    async fn list_knowledge_edge_span_ids(&self, edge_id: &str) -> StorageResult<Vec<String>>;

    /// Updates edge lifecycle; entering `conflicted` requires a conflict
    /// marker, leaving it clears the marker.
    async fn set_knowledge_edge_lifecycle(
        &self,
        edge_id: &str,
        lifecycle: KnowledgeEdgeLifecycle,
        conflict_marker: Option<Value>,
    ) -> StorageResult<KnowledgeEdge>;

    // -- MT-056 claims ------------------------------------------------------------
    /// Creates a claim (born `proposed`) with its REQUIRED evidence spans in
    /// one transaction. Fails closed with a typed Validation error when
    /// `evidence_span_ids` is empty.
    async fn create_knowledge_claim(
        &self,
        new_claim: NewKnowledgeClaim,
    ) -> StorageResult<KnowledgeClaim>;

    async fn get_knowledge_claim(&self, claim_id: &str) -> StorageResult<Option<KnowledgeClaim>>;

    /// Lists the evidence span ids attached to a claim.
    async fn list_knowledge_claim_span_ids(&self, claim_id: &str) -> StorageResult<Vec<String>>;

    /// Guarded lifecycle transition (proposed -> accepted|conflicted|retired,
    /// accepted -> conflicted|retired, conflicted -> accepted|retired,
    /// retired terminal). Invalid transitions are typed `Conflict` errors.
    /// `retirement` is required when entering `retired`;
    /// `resolution_receipt_event_id` records the EventLedger receipt that
    /// authorized the transition.
    async fn transition_knowledge_claim(
        &self,
        claim_id: &str,
        to_state: KnowledgeClaimState,
        retirement: Option<KnowledgeClaimRetirement>,
        resolution_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeClaim>;

    /// Records a conflict between two claims and moves both into the
    /// `conflicted` lifecycle state transactionally.
    async fn record_knowledge_claim_conflict(
        &self,
        claim_id: &str,
        conflicting_claim_id: &str,
        conflict_reason: &str,
        detected_in_run: Option<&str>,
    ) -> StorageResult<KnowledgeClaimConflict>;

    /// Resolves a recorded conflict with an EventLedger receipt ref.
    async fn resolve_knowledge_claim_conflict(
        &self,
        conflict_id: &str,
        resolution_receipt_event_id: &str,
    ) -> StorageResult<KnowledgeClaimConflict>;

    async fn list_knowledge_claim_conflicts(
        &self,
        claim_id: &str,
    ) -> StorageResult<Vec<KnowledgeClaimConflict>>;

    // -- MT-057 memory passages -----------------------------------------------------
    /// Creates a memory passage with its REQUIRED derivation lineage
    /// (sources/claims/spans) in one transaction.
    async fn create_knowledge_memory_passage(
        &self,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeMemoryPassage>;

    async fn get_knowledge_memory_passage(
        &self,
        passage_id: &str,
    ) -> StorageResult<Option<KnowledgeMemoryPassage>>;

    /// Lists the derivation lineage of a passage in insertion order.
    async fn list_knowledge_passage_evidence(
        &self,
        passage_id: &str,
    ) -> StorageResult<Vec<KnowledgePassageEvidenceRef>>;

    /// Refreshes passage freshness and/or compaction policy.
    async fn set_knowledge_passage_compaction(
        &self,
        passage_id: &str,
        compaction_policy: KnowledgeCompactionPolicy,
        refresh_freshness: bool,
    ) -> StorageResult<KnowledgeMemoryPassage>;

    // -- MT-058 wiki projections (NEVER authority) ----------------------------------
    /// Upserts a projection by its stable (workspace, kind, title) identity.
    /// A re-upsert replaces the render inputs and marks the projection stale.
    async fn upsert_knowledge_wiki_projection(
        &self,
        new_projection: NewKnowledgeWikiProjection,
    ) -> StorageResult<KnowledgeWikiProjection>;

    async fn get_knowledge_wiki_projection(
        &self,
        projection_id: &str,
    ) -> StorageResult<Option<KnowledgeWikiProjection>>;

    /// Records a completed rebuild: fresh content, new staleness hash, and an
    /// optional EventLedger rebuild receipt.
    async fn mark_knowledge_projection_rebuilt(
        &self,
        projection_id: &str,
        staleness_hash: &str,
        rendered_content: &str,
        rebuild_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeWikiProjection>;

    /// Moves a projection through stale/rebuilding/failed without touching
    /// the rendered content.
    async fn set_knowledge_projection_rebuild_status(
        &self,
        projection_id: &str,
        rebuild_status: KnowledgeRebuildStatus,
    ) -> StorageResult<KnowledgeWikiProjection>;

    /// Deletes a projection row. Projections are regenerable; deleting one
    /// MUST NOT mutate authority records (spec 2.3.13.11).
    async fn delete_knowledge_wiki_projection(&self, projection_id: &str) -> StorageResult<()>;

    // -- MT-059 rich documents + editor code nodes ----------------------------------
    /// Creates a rich document at `doc_version = 1` and records revision 1 in
    /// the append-only history, in one transaction.
    async fn create_knowledge_rich_document(
        &self,
        new_document: NewKnowledgeRichDocument,
    ) -> StorageResult<KnowledgeRichDocument>;

    /// Wikilink create-if-absent authority path. Concurrent callers for the same workspace and
    /// normalized title serialize inside PostgreSQL; one creates and every loser receives that same
    /// document. Pre-existing ambiguous duplicate titles fail closed instead of picking one silently.
    async fn create_knowledge_rich_document_if_title_absent(
        &self,
        new_document: NewKnowledgeRichDocument,
    ) -> StorageResult<(KnowledgeRichDocument, bool)>;

    async fn get_knowledge_rich_document(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>>;

    /// MT-258 transclusion: resolve the rich document that ANCHORS to a legacy
    /// `documents` row (its `document_id` foreign-key anchor) within a
    /// workspace. A LoomBlock's `document_id` is that same legacy anchor, so a
    /// note block resolves to its source rich document by this join — no new
    /// table, no new column. The newest anchored revision wins when more than
    /// one rich document shares an anchor.
    async fn get_knowledge_rich_document_by_document_id(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>>;

    async fn get_knowledge_rich_document_draft(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocumentDraft>>;

    async fn upsert_knowledge_rich_document_draft(
        &self,
        upsert: UpsertKnowledgeRichDocumentDraft,
    ) -> StorageResult<KnowledgeRichDocumentDraft>;

    async fn clear_knowledge_rich_document_draft(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<bool>;

    /// Optimistic-concurrency save: succeeds only when `expected_version`
    /// matches the current `doc_version`; bumps the version, recomputes the
    /// content hash, and appends the revision (with its EventLedger promotion
    /// receipt) to the append-only history. A stale `expected_version` fails
    /// closed with a typed `StorageError::Conflict`.
    async fn save_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        expected_version: i64,
        content_json: Value,
        crdt_document_id: Option<&str>,
        crdt_snapshot_id: Option<&str>,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument>;

    /// Lists the append-only promoted revision history in version order.
    async fn list_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersion>>;

    /// Paginated revision-history METADATA in version order (adversarial-v2
    /// MT-156): no content bodies, bounded by `limit`/`offset`.
    async fn list_knowledge_rich_document_version_metas(
        &self,
        rich_document_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersionMeta>>;

    /// Total number of revisions in the document's history (MT-156 pagination).
    async fn count_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<i64>;

    /// Loads ONE revision including its full content body (MT-156 lazy body
    /// load — the list endpoint returns metadata only).
    async fn get_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        doc_version: i64,
    ) -> StorageResult<Option<KnowledgeRichDocumentVersion>>;

    /// MT-157 batch op: rename a document (title only). Does NOT bump
    /// doc_version (content is unchanged); a safe metadata-only op.
    async fn rename_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        title: &str,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> StorageResult<KnowledgeRichDocument>;

    /// MT-157 batch op: move a document to a project/folder. `None` for an arg
    /// clears that membership; `Some(value)` sets it. Metadata-only.
    async fn move_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument>;

    /// MT-157 batch op: set a document's authority label
    /// (`draft`|`promoted`|`archived`). Metadata-only.
    async fn set_knowledge_rich_document_authority_label(
        &self,
        rich_document_id: &str,
        authority_label: &str,
    ) -> StorageResult<KnowledgeRichDocument>;

    /// Lists a workspace's rich documents, optionally scoped to a project/
    /// folder (MT-145 membership lookup, MT-157 batch targeting).
    async fn list_knowledge_rich_documents(
        &self,
        workspace_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<Vec<KnowledgeRichDocument>>;

    /// Upserts a Monaco code node by its stable (document, node_path)
    /// identity; the round-trip integrity hash is recomputed from the exact
    /// code text on every write.
    async fn upsert_knowledge_editor_code_node(
        &self,
        upsert: UpsertEditorCodeNode,
    ) -> StorageResult<KnowledgeEditorCodeNode>;

    async fn list_knowledge_editor_code_nodes(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeEditorCodeNode>>;

    // -- MT-152/153 document embeds (typed refs + broken-embed repair) ---------------
    /// Upserts a typed embed reference by its stable `(document, block_id)`
    /// identity (MT-152). Absolute-path targets are rejected by the DB CHECK;
    /// a re-save of the document upserts the embed for that block in place.
    async fn upsert_knowledge_document_embed(
        &self,
        upsert: UpsertKnowledgeDocumentEmbed,
    ) -> StorageResult<KnowledgeDocumentEmbed>;

    async fn list_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>>;

    /// Marks an embed broken (MT-153) with a repair reason, or repairs it back
    /// to `ok` (pass `None` for the reason). Returns the updated embed.
    async fn set_knowledge_document_embed_repair_state(
        &self,
        embed_id: &str,
        broken_reason: Option<&str>,
    ) -> StorageResult<KnowledgeDocumentEmbed>;

    /// Lists only the broken embeds for a document (the repair queue, MT-153).
    async fn list_knowledge_document_broken_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>>;

    /// Replaces ALL embed references for a document with the supplied set in
    /// one transaction (adversarial-v2 MT-152: the document content is the
    /// source of truth — the save path re-projects content_json embed blocks
    /// through the EmbedTarget law and syncs the side table, so the table can
    /// never drift from what documents actually contain). Returns the
    /// persisted embeds.
    async fn replace_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentEmbed>,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>>;

    // -- MT-155 document backlinks (stable relationship id) --------------------------
    /// Upserts a document backlink by its stable `(workspace, relationship_id)`
    /// identity (MT-155). The relationship id is caller-derived and stable
    /// across re-extraction runs.
    async fn upsert_knowledge_document_backlink(
        &self,
        upsert: UpsertKnowledgeDocumentBacklink,
    ) -> StorageResult<KnowledgeDocumentBacklink>;

    /// Replaces ALL backlinks for a source document with the supplied set in
    /// one transaction (MT-155 rebuild: the document content is the source of
    /// truth, so a re-extract is delete-all + insert, idempotent). Returns the
    /// persisted backlinks.
    async fn replace_knowledge_document_backlinks(
        &self,
        source_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentBacklink>,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>>;

    /// Lists the backlinks a source document emits (MT-155).
    async fn list_knowledge_document_backlinks_from(
        &self,
        source_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>>;

    /// Reverse lookup: who links TO this target (MT-155 backlink direction).
    async fn list_knowledge_document_backlinks_to(
        &self,
        workspace_id: &str,
        link_kind: &str,
        target: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>>;

    // -- MT-060 context bundles + retrieval traces ----------------------------------
    /// Persists a kernel ContextBundle V1 run with its per-item retrieval
    /// decisions in one transaction.
    async fn record_knowledge_context_bundle(
        &self,
        new_bundle: NewKnowledgeContextBundle,
    ) -> StorageResult<KnowledgeContextBundle>;

    /// Fetches a bundle run plus its item decisions in ordinal order.
    async fn get_knowledge_context_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Option<(KnowledgeContextBundle, Vec<KnowledgeContextBundleItem>)>>;

    /// Records a replayable retrieval trace; `mode_reason` is a spec MUST
    /// (why broader retrieval was used or skipped).
    async fn record_knowledge_retrieval_trace(
        &self,
        new_trace: NewKnowledgeRetrievalTrace,
    ) -> StorageResult<KnowledgeRetrievalTrace>;

    async fn list_knowledge_retrieval_traces_for_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Vec<KnowledgeRetrievalTrace>>;

    // -- MT-062 transactional idempotency keys --------------------------------------
    /// Idempotent passage write (parallel-indexing surface): the write and
    /// the key row commit in one transaction; a replay with the same key and
    /// payload returns the prior passage without writing anything; the same
    /// key with a different payload is a typed Conflict.
    async fn create_knowledge_memory_passage_idempotent(
        &self,
        idempotency_key: &str,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeMemoryPassage>>;

    /// Idempotent editor save (rich_document_save surface): replaying the
    /// same save (same key + same payload) returns the already-promoted
    /// revision instead of a version conflict, and never double-writes the
    /// version history.
    async fn save_knowledge_rich_document_version_idempotent(
        &self,
        idempotency_key: &str,
        rich_document_id: &str,
        expected_version: i64,
        content_json: Value,
        crdt_document_id: Option<&str>,
        crdt_snapshot_id: Option<&str>,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeRichDocument>>;
}

// ===========================================================================
// WP-KERNEL-009 CodeIndexingAndNavigation (MT-097..MT-112) SHARED-FILE
// ADDITION.
//
// Added by the CodeIndexingAndNavigation group: row types for the two
// code-index SUPPORT tables
// (knowledge_code_files = 0170, knowledge_code_scip_imports = 0171). These are
// the only durable tables this group owns; symbols/spans/edges use the shared
// authority tables above through the existing KnowledgeStore trait. This block
// is intentionally self-contained (separate `impl`, not new trait methods) so
// the addition is auditable and does not perturb the KnowledgeStore trait
// surface consumed elsewhere.
// ===========================================================================

/// Code language of an indexed code file (mirror of
/// `knowledge_code_index::parser::CodeLanguage`, kept as a plain string here so
/// the storage layer does not depend on the code-index module).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCodeLanguage {
    Rust,
    Javascript,
    Typescript,
    Tsx,
    /// MT-101: a config/schema file (json/yaml/toml). It has no tree-sitter
    /// CodeLanguage, but it still gets a `knowledge_code_files` index-state row
    /// so staleness (MT-107) and the lens cover config sources too.
    Config,
}

impl KnowledgeCodeLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Javascript => "javascript",
            Self::Typescript => "typescript",
            Self::Tsx => "tsx",
            Self::Config => "config",
        }
    }
}

impl FromStr for KnowledgeCodeLanguage {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rust" => Ok(Self::Rust),
            "javascript" => Ok(Self::Javascript),
            "typescript" => Ok(Self::Typescript),
            "tsx" => Ok(Self::Tsx),
            "config" => Ok(Self::Config),
            _ => Err(StorageError::Validation("invalid knowledge code language")),
        }
    }
}

/// Per-file parse outcome (MT-108 partial-failure handling).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCodeParseStatus {
    Parsed,
    Partial,
    Failed,
}

impl KnowledgeCodeParseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Parsed => "parsed",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for KnowledgeCodeParseStatus {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "parsed" => Ok(Self::Parsed),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            _ => Err(StorageError::Validation(
                "invalid knowledge code parse_status",
            )),
        }
    }
}

/// One row of `knowledge_code_files` (per-code-file index state).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeCodeFile {
    pub code_file_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub file_entity_id: Option<String>,
    pub language: KnowledgeCodeLanguage,
    pub indexed_content_hash: String,
    pub parser_version: String,
    pub parse_status: KnowledgeCodeParseStatus,
    pub stale: bool,
    pub symbols_indexed: i32,
    pub edges_indexed: i32,
    pub failure_detail: Option<Value>,
    pub last_indexed_in_run: Option<String>,
    pub last_index_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert payload for [`KnowledgeCodeFile`].
#[derive(Clone, Debug)]
pub struct UpsertKnowledgeCodeFile {
    pub workspace_id: String,
    pub source_id: String,
    pub file_entity_id: Option<String>,
    pub language: KnowledgeCodeLanguage,
    /// Raw sha256 hex of the file content the index reflects.
    pub indexed_content_hash: String,
    pub parser_version: String,
    pub parse_status: KnowledgeCodeParseStatus,
    pub symbols_indexed: i32,
    pub edges_indexed: i32,
    pub failure_detail: Option<Value>,
    pub last_indexed_in_run: Option<String>,
    pub last_index_receipt_event_id: Option<String>,
}

const KNOWLEDGE_CODE_FILE_COLUMNS: &str = r#"
    code_file_id, workspace_id, source_id, file_entity_id, language,
    indexed_content_hash, parser_version, parse_status, stale,
    symbols_indexed, edges_indexed, failure_detail, last_indexed_in_run,
    last_index_receipt_event_id, created_at, updated_at
"#;

/// SCIP/LSIF import format (MT-105).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeScipFormat {
    Scip,
    Lsif,
}

impl KnowledgeScipFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scip => "scip",
            Self::Lsif => "lsif",
        }
    }
}

/// Outcome of a SCIP/LSIF import attempt.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeScipImportStatus {
    Imported,
    Partial,
    Rejected,
}

impl KnowledgeScipImportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Partial => "partial",
            Self::Rejected => "rejected",
        }
    }
}

impl FromStr for KnowledgeScipImportStatus {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "imported" => Ok(Self::Imported),
            "partial" => Ok(Self::Partial),
            "rejected" => Ok(Self::Rejected),
            _ => Err(StorageError::Validation("invalid scip import_status")),
        }
    }
}

/// One row of `knowledge_code_scip_imports`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeScipImport {
    pub scip_import_id: String,
    pub workspace_id: String,
    pub artifact_format: KnowledgeScipFormat,
    pub tool_name: Option<String>,
    pub tool_version: Option<String>,
    pub artifact_hash: String,
    pub status: KnowledgeScipImportStatus,
    pub reason: Option<String>,
    pub symbols_imported: i32,
    pub occurrences_imported: i32,
    pub edges_imported: i32,
    pub import_detail: Option<Value>,
    pub imported_in_run: Option<String>,
    pub import_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`KnowledgeScipImport`].
#[derive(Clone, Debug)]
pub struct NewKnowledgeScipImport {
    pub workspace_id: String,
    pub artifact_format: KnowledgeScipFormat,
    pub tool_name: Option<String>,
    pub tool_version: Option<String>,
    pub artifact_hash: String,
    pub status: KnowledgeScipImportStatus,
    pub reason: Option<String>,
    pub symbols_imported: i32,
    pub occurrences_imported: i32,
    pub edges_imported: i32,
    pub import_detail: Option<Value>,
    pub imported_in_run: Option<String>,
    pub import_receipt_event_id: Option<String>,
}

const KNOWLEDGE_SCIP_IMPORT_COLUMNS: &str = r#"
    scip_import_id, workspace_id, artifact_format, tool_name, tool_version,
    artifact_hash, status, reason, symbols_imported, occurrences_imported,
    edges_imported, import_detail, imported_in_run, import_receipt_event_id,
    created_at
"#;

// ===========================================================================
// MT-108 code-index repair queue (`knowledge_code_repair_queue`, 0230).
//
// The CODE-INDEX equivalent of the ingestion repair queue
// (`knowledge_ingestion_repair_queue`, owned by `knowledge_ingestion`): a
// durable, backend-visible surface for files whose code-index PARSE failed
// (grammar init / no tree / FFI panic) or whose READ failed (binary / non-UTF8
// / unreadable / config-parse). This is what makes MT-108 a real recovery
// surface rather than a status flag: a no-context model can list open
// code-index repair work and re-run the parse pass after the cause is fixed.
//
// Lifecycle mirrors the ingestion queue (enqueue refreshes an open entry,
// reopens a dead-letter for the same source, else inserts fresh).
// ===========================================================================

/// Why a file sits in the code-index repair queue.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCodeRepairReason {
    /// Tree-sitter could not produce a tree (grammar init / no tree).
    ParseError,
    /// The file could not be read as UTF-8 text (binary / wrong encoding / OS
    /// read error).
    ReadError,
    /// The tree-sitter FFI itself panicked on this input.
    Panic,
    /// A config/schema file failed typed parsing (TOML/JSON/YAML).
    ConfigParseError,
}

impl KnowledgeCodeRepairReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParseError => "PARSE_ERROR",
            Self::ReadError => "READ_ERROR",
            Self::Panic => "PANIC",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
        }
    }
}

impl FromStr for KnowledgeCodeRepairReason {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PARSE_ERROR" => Ok(Self::ParseError),
            "READ_ERROR" => Ok(Self::ReadError),
            "PANIC" => Ok(Self::Panic),
            "CONFIG_PARSE_ERROR" => Ok(Self::ConfigParseError),
            _ => Err(StorageError::Validation(
                "invalid knowledge code repair reason_class",
            )),
        }
    }
}

/// One row of `knowledge_code_repair_queue`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeCodeRepairEntry {
    pub code_repair_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub relative_path: String,
    pub reason_class: KnowledgeCodeRepairReason,
    pub reason_detail: Value,
    pub state: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub enqueue_event_id: Option<String>,
    pub resolved_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Enqueue payload for a code-index repair entry.
#[derive(Clone, Debug)]
pub struct NewKnowledgeCodeRepairEntry {
    pub workspace_id: String,
    pub source_id: String,
    pub relative_path: String,
    pub reason_class: KnowledgeCodeRepairReason,
    pub reason_detail: Value,
    pub enqueue_event_id: Option<String>,
}

const KNOWLEDGE_CODE_REPAIR_COLUMNS: &str = r#"
    code_repair_id, workspace_id, source_id, relative_path, reason_class,
    reason_detail, state, attempts, max_attempts, last_attempt_at,
    enqueue_event_id, resolved_receipt_event_id, created_at, updated_at
"#;

/// Escape PostgreSQL `LIKE` metacharacters (`%`, `_`, `\`) in an operator-
/// supplied literal so a symbol name/path containing them matches literally and
/// cannot widen the scan. The default escape char `\` is escaped first.
fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// ===========================================================================
// WP-KERNEL-009 MT-241/242/243 ProjectWikiCompile storage support
// (Master Spec §10.12 Section 17 [LM-PWIKI-001..013]).
//
// Row types only. The inherent PostgreSQL methods that used to accompany them
// were removed with the deleted backend (WP-KERNEL-012 MT-136). Everything
// writes into the EXISTING `knowledge_wiki_projections` projection store
// (0139 + 0300) — no parallel wiki infrastructure (LM-PWIKI-005).
// ===========================================================================

/// Allowed typed page kinds (mirrors `chk_knowledge_wiki_projections_page_type`
/// and `knowledge_wiki::WikiPageType`; kept as strings here so the storage
/// layer does not depend on the compile module).
const KNOWLEDGE_WIKI_PAGE_TYPES: [&str; 6] =
    ["module", "concept", "flow", "entity", "decision", "index"];

/// Upsert payload for a compiled, STAMPED wiki page.
///
/// SHIP-TOGETHER GUARD (LM-PWIKI-009): `compile_stamp` is NOT `Option` — a
/// compile output without its drift/staleness stamp cannot be expressed at
/// this layer, and migration 0300's CHECK enforces the same at the database.
#[derive(Clone, Debug)]
pub struct NewKnowledgeWikiPage {
    pub workspace_id: String,
    pub title: String,
    /// `module|concept|flow|entity|decision|index`; `None` only for untyped
    /// MT-184 Loom topic pages (which are still stamped).
    pub page_type: Option<String>,
    /// Citation refs `[{"record_family", "record_id", "content_hash", ...}]`.
    pub source_records: Value,
    pub rendered_content: String,
    /// Legacy MT-184 staleness hash (sha256 hex; kept for back-compat).
    pub staleness_hash: String,
    /// MT-242 stamp (`knowledge_wiki::WikiCompileStamp::to_value()`).
    pub compile_stamp: Value,
    /// MT-243 deterministic compile-input descriptor.
    pub compile_recipe: Option<Value>,
    /// Outbound wikilinks `[{"title", "projection_id"}]`.
    pub page_links: Value,
    /// EventLedger compile receipt this build references (LM-PWIKI-012).
    pub rebuild_receipt_event_id: Option<String>,
}

/// One indexed code file + its source identity, as bootstrap-compiler input.
#[derive(Clone, Debug)]
pub struct WikiCodeFileInput {
    pub code_file_id: String,
    pub source_id: String,
    pub relative_path: String,
    /// The source's current content hash (citation hash for `source` kind).
    pub content_hash: String,
    pub language: KnowledgeCodeLanguage,
    pub parse_status: KnowledgeCodeParseStatus,
    pub stale: bool,
    pub symbols_indexed: i32,
}

/// An entity plus its latest evidence span on a given source (compiler input
/// for symbol/concept listings with span citations).
#[derive(Clone, Debug)]
pub struct WikiEntityWithSpan {
    pub entity: KnowledgeEntity,
    pub span_id: String,
    pub span_content_sha256: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub section_path: Option<String>,
}

/// A code-graph edge whose endpoints live on DIFFERENT sources (drives
/// cross-module wikilinks).
#[derive(Clone, Debug)]
pub struct WikiCrossSourceEdge {
    pub edge_type: KnowledgeEdgeType,
    pub from_source_id: String,
    pub to_source_id: String,
}

/// Current loom-block content state for drift hashing (lean row; content-
/// bearing fields only).
#[derive(Clone, Debug)]
pub struct WikiLoomBlockState {
    pub block_id: String,
    pub title: Option<String>,
    pub content_type: String,
    pub full_text_index: Option<String>,
    pub document_id: Option<String>,
    pub asset_id: Option<String>,
    pub content_hash: Option<String>,
}

