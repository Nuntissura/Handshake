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
//! `StorageResult<T>`; sqlx errors are converted to the opaque
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
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

use super::postgres::PostgresDatabase;
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
    if normalized
        .chars()
        .nth(1)
        .map(|c| c == ':')
        .unwrap_or(false)
    {
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

fn source_root_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeSourceRoot> {
    Ok(KnowledgeSourceRoot {
        root_id: row.get("root_id"),
        workspace_id: row.get("workspace_id"),
        display_name: row.get("display_name"),
        root_kind: row.get::<String, _>("root_kind").parse()?,
        repo_relative_path: row.get("repo_relative_path"),
        path_normalization: row.get("path_normalization"),
        allowlist_policy: row.get("allowlist_policy"),
        indexing_eligibility: row.get::<String, _>("indexing_eligibility").parse()?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn new_knowledge_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
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

fn source_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeSource> {
    Ok(KnowledgeSource {
        source_id: row.get("source_id"),
        workspace_id: row.get("workspace_id"),
        root_id: row.get("root_id"),
        source_kind: row.get::<String, _>("source_kind").parse()?,
        relative_path: row.get("relative_path"),
        asset_id: row.get("asset_id"),
        loom_block_id: row.get("loom_block_id"),
        document_id: row.get("document_id"),
        content_hash: row.get("content_hash"),
        size_bytes: row.get("size_bytes"),
        provenance: row.get("provenance"),
        permission_scope: row.get::<String, _>("permission_scope").parse()?,
        redaction_state: row.get::<String, _>("redaction_state").parse()?,
        parser_status: row.get::<String, _>("parser_status").parse()?,
        extraction_status: row.get::<String, _>("extraction_status").parse()?,
        stale: row.get("stale"),
        last_index_receipt_event_id: row.get("last_index_receipt_event_id"),
        source_modified_at: row.get("source_modified_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

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
            Self::Completed { counts } | Self::Failed { counts, .. } | Self::Cancelled { counts } => {
                *counts
            }
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

fn index_run_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeIndexRun> {
    Ok(KnowledgeIndexRun {
        index_run_id: row.get("index_run_id"),
        workspace_id: row.get("workspace_id"),
        root_id: row.get("root_id"),
        run_state: row.get::<String, _>("run_state").parse()?,
        scope: row.get("scope"),
        actor_kind: row.get("actor_kind"),
        actor_id: row.get("actor_id"),
        worktree_id: row.get("worktree_id"),
        restart_checkpoint: row.get("restart_checkpoint"),
        counts: KnowledgeIndexRunCounts {
            sources_seen: row.get("sources_seen"),
            sources_indexed: row.get("sources_indexed"),
            spans_extracted: row.get("spans_extracted"),
            entities_detected: row.get("entities_detected"),
            edges_written: row.get("edges_written"),
            claims_written: row.get("claims_written"),
        },
        error_capture: row.get("error_capture"),
        start_receipt_event_id: row.get("start_receipt_event_id"),
        finish_receipt_event_id: row.get("finish_receipt_event_id"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
    })
}

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

fn span_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeSpan> {
    Ok(KnowledgeSpan {
        span_id: row.get("span_id"),
        source_id: row.get("source_id"),
        span_kind: row.get::<String, _>("span_kind").parse()?,
        range_start: row.get("range_start"),
        range_end: row.get("range_end"),
        line_start: row.get("line_start"),
        line_end: row.get("line_end"),
        section_path: row.get("section_path"),
        content_sha256: row.get("content_sha256"),
        parser_version: row.get("parser_version"),
        extraction_receipt_event_id: row.get("extraction_receipt_event_id"),
        index_run_id: row.get("index_run_id"),
        display_snippet: row.get("display_snippet"),
        created_at: row.get("created_at"),
    })
}

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

fn entity_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeEntity> {
    Ok(KnowledgeEntity {
        entity_id: row.get("entity_id"),
        workspace_id: row.get("workspace_id"),
        entity_kind: row.get::<String, _>("entity_kind").parse()?,
        entity_key: row.get("entity_key"),
        display_name: row.get("display_name"),
        detection_provenance: row.get("detection_provenance"),
        lifecycle_state: row.get::<String, _>("lifecycle_state").parse()?,
        primary_source_id: row.get("primary_source_id"),
        first_detected_in_run: row.get("first_detected_in_run"),
        last_detected_in_run: row.get("last_detected_in_run"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

// ---------------------------------------------------------------------------
// KnowledgeStore trait: the WP-009 storage surface on PostgresDatabase.
// ---------------------------------------------------------------------------

/// WP-009 ProjectKnowledgeIndex storage operations.
///
/// Implemented for [`PostgresDatabase`] only: PostgreSQL plus EventLedger is
/// canonical for all durable WP-009 state. There is intentionally no other
/// implementor and no fallback implementor (MT-064 fail-closed).
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

    async fn get_knowledge_source(
        &self,
        source_id: &str,
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

    async fn get_knowledge_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Option<KnowledgeEntity>>;

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
    async fn list_knowledge_entity_span_ids(&self, entity_id: &str)
        -> StorageResult<Vec<String>>;

    /// Marks an entity retired (it stops participating in new detection).
    async fn retire_knowledge_entity(&self, entity_id: &str) -> StorageResult<KnowledgeEntity>;
}

fn registry_row_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeSchemaRegistryRow> {
    Ok(KnowledgeSchemaRegistryRow {
        family_key: row.get("family_key"),
        table_name: row.get("table_name"),
        record_family: row.get("record_family"),
        authority_class: row
            .get::<String, _>("authority_class")
            .parse::<KnowledgeAuthorityClass>()?,
        migration_file: row.get("migration_file"),
        wp_id: row.get("wp_id"),
        mt_id: row.get("mt_id"),
        registered_at: row.get("registered_at"),
    })
}

#[async_trait]
impl KnowledgeStore for PostgresDatabase {
    async fn list_knowledge_schema_registry(
        &self,
    ) -> StorageResult<Vec<KnowledgeSchemaRegistryRow>> {
        let rows = sqlx::query(
            r#"
            SELECT family_key, table_name, record_family, authority_class,
                   migration_file, wp_id, mt_id, registered_at
            FROM knowledge_schema_registry
            ORDER BY family_key
            "#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(registry_row_from_pg).collect()
    }

    async fn audit_knowledge_namespace(&self) -> StorageResult<KnowledgeNamespaceAudit> {
        let registered = self.list_knowledge_schema_registry().await?;

        let present: Vec<String> = sqlx::query(
            r#"
            SELECT table_name::text AS table_name
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_type = 'BASE TABLE'
              AND table_name LIKE 'knowledge\_%' ESCAPE '\'
            ORDER BY table_name
            "#,
        )
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect();

        let missing_tables = registered
            .iter()
            .filter(|row| !present.contains(&row.table_name))
            .map(|row| row.table_name.clone())
            .collect();
        let unregistered_tables = present
            .iter()
            .filter(|table| !registered.iter().any(|row| &row.table_name == *table))
            .cloned()
            .collect();

        Ok(KnowledgeNamespaceAudit {
            registered,
            missing_tables,
            unregistered_tables,
        })
    }

    async fn create_knowledge_source_root(
        &self,
        new_root: NewKnowledgeSourceRoot,
    ) -> StorageResult<KnowledgeSourceRoot> {
        if new_root.display_name.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge source root display_name is required",
            ));
        }
        let repo_relative_path = normalize_repo_relative_path(&new_root.repo_relative_path)?;
        let root_id = new_knowledge_id("KSR");

        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_source_roots
                (root_id, workspace_id, display_name, root_kind,
                 repo_relative_path, allowlist_policy, indexing_eligibility)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING root_id, workspace_id, display_name, root_kind,
                      repo_relative_path, path_normalization, allowlist_policy,
                      indexing_eligibility, created_at, updated_at
            "#,
        )
        .bind(&root_id)
        .bind(&new_root.workspace_id)
        .bind(&new_root.display_name)
        .bind(new_root.root_kind.as_str())
        .bind(&repo_relative_path)
        .bind(&new_root.allowlist_policy)
        .bind(new_root.indexing_eligibility.as_str())
        .fetch_one(self.pool())
        .await?;
        source_root_from_pg(&row)
    }

    async fn get_knowledge_source_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Option<KnowledgeSourceRoot>> {
        let row = sqlx::query(
            r#"
            SELECT root_id, workspace_id, display_name, root_kind,
                   repo_relative_path, path_normalization, allowlist_policy,
                   indexing_eligibility, created_at, updated_at
            FROM knowledge_source_roots
            WHERE root_id = $1
            "#,
        )
        .bind(root_id)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(source_root_from_pg).transpose()
    }

    async fn list_knowledge_source_roots(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeSourceRoot>> {
        let rows = sqlx::query(
            r#"
            SELECT root_id, workspace_id, display_name, root_kind,
                   repo_relative_path, path_normalization, allowlist_policy,
                   indexing_eligibility, created_at, updated_at
            FROM knowledge_source_roots
            WHERE workspace_id = $1
            ORDER BY repo_relative_path
            "#,
        )
        .bind(workspace_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(source_root_from_pg).collect()
    }

    async fn set_knowledge_root_eligibility(
        &self,
        root_id: &str,
        eligibility: KnowledgeIndexingEligibility,
    ) -> StorageResult<KnowledgeSourceRoot> {
        let row = sqlx::query(
            r#"
            UPDATE knowledge_source_roots
            SET indexing_eligibility = $2, updated_at = NOW()
            WHERE root_id = $1
            RETURNING root_id, workspace_id, display_name, root_kind,
                      repo_relative_path, path_normalization, allowlist_policy,
                      indexing_eligibility, created_at, updated_at
            "#,
        )
        .bind(root_id)
        .bind(eligibility.as_str())
        .fetch_optional(self.pool())
        .await?
        .ok_or(StorageError::NotFound("knowledge source root"))?;
        source_root_from_pg(&row)
    }

    async fn upsert_knowledge_source(
        &self,
        new_source: NewKnowledgeSource,
    ) -> StorageResult<KnowledgeSource> {
        if !is_sha256_hex(&new_source.content_hash) {
            return Err(StorageError::Validation(
                "knowledge source content_hash must be a lowercase sha256 hex digest",
            ));
        }
        let relative_path = new_source
            .relative_path
            .as_deref()
            .map(normalize_repo_relative_path)
            .transpose()?;
        if matches!(new_source.source_kind, KnowledgeSourceKind::File)
            && (new_source.root_id.is_none() || relative_path.is_none())
        {
            return Err(StorageError::Validation(
                "file-kind knowledge sources require root_id and relative_path",
            ));
        }
        let source_id = new_knowledge_id("KSRC");

        let sql = format!(
            r#"
            INSERT INTO knowledge_sources
                (source_id, workspace_id, root_id, source_kind, relative_path,
                 asset_id, loom_block_id, document_id, content_hash, size_bytes,
                 provenance, permission_scope, redaction_state, source_modified_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (root_id, relative_path) WHERE relative_path IS NOT NULL
            DO UPDATE SET
                content_hash = EXCLUDED.content_hash,
                size_bytes = EXCLUDED.size_bytes,
                provenance = EXCLUDED.provenance,
                permission_scope = EXCLUDED.permission_scope,
                redaction_state = EXCLUDED.redaction_state,
                source_modified_at = EXCLUDED.source_modified_at,
                parser_status = 'pending',
                extraction_status = 'pending',
                stale = FALSE,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_SOURCE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&source_id)
            .bind(&new_source.workspace_id)
            .bind(&new_source.root_id)
            .bind(new_source.source_kind.as_str())
            .bind(&relative_path)
            .bind(&new_source.asset_id)
            .bind(&new_source.loom_block_id)
            .bind(&new_source.document_id)
            .bind(&new_source.content_hash)
            .bind(new_source.size_bytes)
            .bind(&new_source.provenance)
            .bind(new_source.permission_scope.as_str())
            .bind(new_source.redaction_state.as_str())
            .bind(new_source.source_modified_at)
            .fetch_one(self.pool())
            .await?;
        source_from_pg(&row)
    }

    async fn get_knowledge_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeSource>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_SOURCE_COLUMNS} FROM knowledge_sources WHERE source_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(source_from_pg).transpose()
    }

    async fn list_knowledge_sources_for_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Vec<KnowledgeSource>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_SOURCE_COLUMNS} FROM knowledge_sources
             WHERE root_id = $1 ORDER BY relative_path"
        );
        let rows = sqlx::query(&sql).bind(root_id).fetch_all(self.pool()).await?;
        rows.iter().map(source_from_pg).collect()
    }

    async fn mark_knowledge_source_stale(&self, source_id: &str) -> StorageResult<KnowledgeSource> {
        let sql = format!(
            "UPDATE knowledge_sources SET stale = TRUE, updated_at = NOW()
             WHERE source_id = $1 RETURNING {KNOWLEDGE_SOURCE_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge source"))?;
        source_from_pg(&row)
    }

    async fn record_knowledge_source_index_receipt(
        &self,
        source_id: &str,
        parser_status: KnowledgeParserStatus,
        extraction_status: KnowledgeExtractionStatus,
        receipt_event_id: &str,
    ) -> StorageResult<KnowledgeSource> {
        let sql = format!(
            r#"
            UPDATE knowledge_sources
            SET parser_status = $2,
                extraction_status = $3,
                last_index_receipt_event_id = $4,
                stale = FALSE,
                updated_at = NOW()
            WHERE source_id = $1
            RETURNING {KNOWLEDGE_SOURCE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(source_id)
            .bind(parser_status.as_str())
            .bind(extraction_status.as_str())
            .bind(receipt_event_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge source"))?;
        source_from_pg(&row)
    }

    async fn start_knowledge_index_run(
        &self,
        new_run: NewKnowledgeIndexRun,
    ) -> StorageResult<KnowledgeIndexRun> {
        if new_run.actor_kind.trim().is_empty() || new_run.actor_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge index run requires actor_kind and actor_id",
            ));
        }
        let index_run_id = new_knowledge_id("KIR");
        let sql = format!(
            r#"
            INSERT INTO knowledge_index_runs
                (index_run_id, workspace_id, root_id, scope, actor_kind,
                 actor_id, worktree_id, start_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING {KNOWLEDGE_INDEX_RUN_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&index_run_id)
            .bind(&new_run.workspace_id)
            .bind(&new_run.root_id)
            .bind(&new_run.scope)
            .bind(&new_run.actor_kind)
            .bind(&new_run.actor_id)
            .bind(&new_run.worktree_id)
            .bind(&new_run.start_receipt_event_id)
            .fetch_one(self.pool())
            .await?;
        index_run_from_pg(&row)
    }

    async fn get_knowledge_index_run(
        &self,
        index_run_id: &str,
    ) -> StorageResult<Option<KnowledgeIndexRun>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_INDEX_RUN_COLUMNS} FROM knowledge_index_runs WHERE index_run_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(index_run_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(index_run_from_pg).transpose()
    }

    async fn checkpoint_knowledge_index_run(
        &self,
        index_run_id: &str,
        restart_checkpoint: Value,
    ) -> StorageResult<KnowledgeIndexRun> {
        let sql = format!(
            r#"
            UPDATE knowledge_index_runs
            SET restart_checkpoint = $2
            WHERE index_run_id = $1 AND run_state = 'started'
            RETURNING {KNOWLEDGE_INDEX_RUN_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(index_run_id)
            .bind(&restart_checkpoint)
            .fetch_optional(self.pool())
            .await?;
        match row {
            Some(row) => index_run_from_pg(&row),
            None => {
                if self.get_knowledge_index_run(index_run_id).await?.is_some() {
                    Err(StorageError::Conflict(
                        "knowledge index run is terminal; checkpoints only apply to started runs",
                    ))
                } else {
                    Err(StorageError::NotFound("knowledge index run"))
                }
            }
        }
    }

    async fn finish_knowledge_index_run(
        &self,
        index_run_id: &str,
        outcome: KnowledgeIndexRunOutcome,
        finish_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIndexRun> {
        let state = outcome.state();
        let counts = outcome.counts();
        let sql = format!(
            r#"
            UPDATE knowledge_index_runs
            SET run_state = $2,
                sources_seen = $3,
                sources_indexed = $4,
                spans_extracted = $5,
                entities_detected = $6,
                edges_written = $7,
                claims_written = $8,
                error_capture = $9,
                finish_receipt_event_id = $10,
                restart_checkpoint = NULL,
                finished_at = NOW()
            WHERE index_run_id = $1 AND run_state = 'started'
            RETURNING {KNOWLEDGE_INDEX_RUN_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(index_run_id)
            .bind(state.as_str())
            .bind(counts.sources_seen)
            .bind(counts.sources_indexed)
            .bind(counts.spans_extracted)
            .bind(counts.entities_detected)
            .bind(counts.edges_written)
            .bind(counts.claims_written)
            .bind(outcome.error_capture())
            .bind(finish_receipt_event_id)
            .fetch_optional(self.pool())
            .await?;
        match row {
            Some(row) => index_run_from_pg(&row),
            None => {
                if self.get_knowledge_index_run(index_run_id).await?.is_some() {
                    Err(StorageError::Conflict(
                        "knowledge index run lifecycle violation: run is already terminal",
                    ))
                } else {
                    Err(StorageError::NotFound("knowledge index run"))
                }
            }
        }
    }

    async fn create_knowledge_span(
        &self,
        new_span: NewKnowledgeSpan,
    ) -> StorageResult<KnowledgeSpan> {
        if !is_sha256_hex(&new_span.content_sha256) {
            return Err(StorageError::Validation(
                "knowledge span content_sha256 must be a lowercase sha256 hex digest",
            ));
        }
        if new_span.parser_version.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge span parser_version is required",
            ));
        }
        if new_span.range_end < new_span.range_start || new_span.range_start < 0 {
            return Err(StorageError::Validation(
                "knowledge span range must satisfy 0 <= range_start <= range_end",
            ));
        }
        let span_id = new_knowledge_id("KSP");
        let sql = format!(
            r#"
            INSERT INTO knowledge_spans
                (span_id, source_id, span_kind, range_start, range_end,
                 line_start, line_end, section_path, content_sha256,
                 parser_version, extraction_receipt_event_id, index_run_id,
                 display_snippet)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING {KNOWLEDGE_SPAN_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&span_id)
            .bind(&new_span.source_id)
            .bind(new_span.span_kind.as_str())
            .bind(new_span.range_start)
            .bind(new_span.range_end)
            .bind(new_span.line_start)
            .bind(new_span.line_end)
            .bind(&new_span.section_path)
            .bind(&new_span.content_sha256)
            .bind(&new_span.parser_version)
            .bind(&new_span.extraction_receipt_event_id)
            .bind(&new_span.index_run_id)
            .bind(&new_span.display_snippet)
            .fetch_one(self.pool())
            .await?;
        span_from_pg(&row)
    }

    async fn get_knowledge_span(&self, span_id: &str) -> StorageResult<Option<KnowledgeSpan>> {
        let sql =
            format!("SELECT {KNOWLEDGE_SPAN_COLUMNS} FROM knowledge_spans WHERE span_id = $1");
        let row = sqlx::query(&sql)
            .bind(span_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(span_from_pg).transpose()
    }

    async fn list_knowledge_spans_for_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Vec<KnowledgeSpan>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_SPAN_COLUMNS} FROM knowledge_spans
             WHERE source_id = $1 ORDER BY range_start, range_end"
        );
        let rows = sqlx::query(&sql)
            .bind(source_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(span_from_pg).collect()
    }

    async fn upsert_knowledge_entity(
        &self,
        new_entity: NewKnowledgeEntity,
    ) -> StorageResult<KnowledgeEntity> {
        if new_entity.entity_key.trim().is_empty() || new_entity.entity_key.trim() != new_entity.entity_key {
            return Err(StorageError::Validation(
                "knowledge entity_key must be non-empty without surrounding whitespace",
            ));
        }
        if new_entity.display_name.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge entity display_name is required",
            ));
        }
        let entity_id = new_knowledge_id("KEN");

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_entities
                (entity_id, workspace_id, entity_kind, entity_key, display_name,
                 detection_provenance, primary_source_id,
                 first_detected_in_run, last_detected_in_run)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            ON CONFLICT (workspace_id, entity_kind, entity_key)
            DO UPDATE SET
                display_name = EXCLUDED.display_name,
                detection_provenance = EXCLUDED.detection_provenance,
                primary_source_id = COALESCE(EXCLUDED.primary_source_id,
                                             knowledge_entities.primary_source_id),
                last_detected_in_run = COALESCE(EXCLUDED.last_detected_in_run,
                                                knowledge_entities.last_detected_in_run),
                lifecycle_state = 'active',
                updated_at = NOW()
            RETURNING {KNOWLEDGE_ENTITY_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&entity_id)
            .bind(&new_entity.workspace_id)
            .bind(new_entity.entity_kind.as_str())
            .bind(&new_entity.entity_key)
            .bind(&new_entity.display_name)
            .bind(&new_entity.detection_provenance)
            .bind(&new_entity.primary_source_id)
            .bind(&new_entity.detected_in_run)
            .fetch_one(&mut *tx)
            .await?;
        let entity = entity_from_pg(&row)?;

        for span_id in &new_entity.evidence_span_ids {
            sqlx::query(
                r#"
                INSERT INTO knowledge_entity_spans (entity_id, span_id, detected_in_run)
                VALUES ($1, $2, $3)
                ON CONFLICT (entity_id, span_id) DO NOTHING
                "#,
            )
            .bind(&entity.entity_id)
            .bind(span_id)
            .bind(&new_entity.detected_in_run)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(entity)
    }

    async fn get_knowledge_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Option<KnowledgeEntity>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_ENTITY_COLUMNS} FROM knowledge_entities WHERE entity_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(entity_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(entity_from_pg).transpose()
    }

    async fn get_knowledge_entity_by_identity(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
        entity_key: &str,
    ) -> StorageResult<Option<KnowledgeEntity>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_ENTITY_COLUMNS} FROM knowledge_entities
             WHERE workspace_id = $1 AND entity_kind = $2 AND entity_key = $3"
        );
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(entity_kind.as_str())
            .bind(entity_key)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(entity_from_pg).transpose()
    }

    async fn list_knowledge_entities_by_kind(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
    ) -> StorageResult<Vec<KnowledgeEntity>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_ENTITY_COLUMNS} FROM knowledge_entities
             WHERE workspace_id = $1 AND entity_kind = $2 ORDER BY entity_key"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(entity_kind.as_str())
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(entity_from_pg).collect()
    }

    async fn list_knowledge_entity_span_ids(
        &self,
        entity_id: &str,
    ) -> StorageResult<Vec<String>> {
        let rows = sqlx::query(
            "SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $1 ORDER BY span_id",
        )
        .bind(entity_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("span_id"))
            .collect())
    }

    async fn retire_knowledge_entity(&self, entity_id: &str) -> StorageResult<KnowledgeEntity> {
        let sql = format!(
            "UPDATE knowledge_entities
             SET lifecycle_state = 'retired', updated_at = NOW()
             WHERE entity_id = $1
             RETURNING {KNOWLEDGE_ENTITY_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(entity_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge entity"))?;
        entity_from_pg(&row)
    }
}
