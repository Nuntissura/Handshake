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
/// Derivation (documented in migrations/0136_knowledge_edges.sql and stable
/// across re-index runs because it hashes the entities' natural identities,
/// never row ids or timestamps):
///
/// ```text
/// relationship_id = "KREL-" + sha256_hex(
///     "knowledge_edge_relationship_v1|{edge_type}|{source_kind}:{source_key}|{target_kind}:{target_key}")
/// ```
pub fn derive_knowledge_relationship_id(
    edge_type: KnowledgeEdgeType,
    source_kind: KnowledgeEntityKind,
    source_key: &str,
    target_kind: KnowledgeEntityKind,
    target_key: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let canonical = format!(
        "knowledge_edge_relationship_v1|{}|{}:{}|{}:{}",
        edge_type.as_str(),
        source_kind.as_str(),
        source_key,
        target_kind.as_str(),
        target_key
    );
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

fn edge_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeEdge> {
    Ok(KnowledgeEdge {
        edge_id: row.get("edge_id"),
        workspace_id: row.get("workspace_id"),
        relationship_id: row.get("relationship_id"),
        edge_type: row.get::<String, _>("edge_type").parse()?,
        source_entity_id: row.get("source_entity_id"),
        target_entity_id: row.get("target_entity_id"),
        extractor_version: row.get("extractor_version"),
        lifecycle_state: row.get::<String, _>("lifecycle_state").parse()?,
        confidence: row.get("confidence"),
        conflict_marker: row.get("conflict_marker"),
        created_in_run: row.get("created_in_run"),
        last_seen_in_run: row.get("last_seen_in_run"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

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

fn claim_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeClaim> {
    Ok(KnowledgeClaim {
        claim_id: row.get("claim_id"),
        workspace_id: row.get("workspace_id"),
        claim_kind: row.get::<String, _>("claim_kind").parse()?,
        claim_text: row.get("claim_text"),
        subject_entity_id: row.get("subject_entity_id"),
        lifecycle_state: row.get::<String, _>("lifecycle_state").parse()?,
        temporal_qualifier: row.get("temporal_qualifier"),
        granularity_qualifier: row.get("granularity_qualifier"),
        confidence: row.get("confidence"),
        retirement_reason: row
            .get::<Option<String>, _>("retirement_reason")
            .map(|value| value.parse())
            .transpose()?,
        superseded_by_claim_id: row.get("superseded_by_claim_id"),
        proposed_in_run: row.get("proposed_in_run"),
        resolution_receipt_event_id: row.get("resolution_receipt_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn claim_conflict_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeClaimConflict {
    KnowledgeClaimConflict {
        conflict_id: row.get("conflict_id"),
        claim_id: row.get("claim_id"),
        conflicting_claim_id: row.get("conflicting_claim_id"),
        detected_in_run: row.get("detected_in_run"),
        conflict_reason: row.get("conflict_reason"),
        resolution_receipt_event_id: row.get("resolution_receipt_event_id"),
        detected_at: row.get("detected_at"),
        resolved_at: row.get("resolved_at"),
    }
}

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
#[derive(Clone, Debug)]
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

fn passage_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeMemoryPassage> {
    Ok(KnowledgeMemoryPassage {
        passage_id: row.get("passage_id"),
        workspace_id: row.get("workspace_id"),
        passage_text: row.get("passage_text"),
        token_count: row.get("token_count"),
        ocr_transcript_metadata: row.get("ocr_transcript_metadata"),
        extraction_confidence: row.get("extraction_confidence"),
        ranking_features: row.get("ranking_features"),
        retrieval_mode: row.get::<String, _>("retrieval_mode").parse()?,
        freshness_at: row.get("freshness_at"),
        compaction_policy: row.get::<String, _>("compaction_policy").parse()?,
        failure_receipt_event_id: row.get("failure_receipt_event_id"),
        derived_in_run: row.get("derived_in_run"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

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
    rebuild_receipt_event_id, last_rebuilt_at, created_at, updated_at
"#;

fn projection_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeWikiProjection> {
    Ok(KnowledgeWikiProjection {
        projection_id: row.get("projection_id"),
        workspace_id: row.get("workspace_id"),
        projection_kind: row.get::<String, _>("projection_kind").parse()?,
        title: row.get("title"),
        source_records: row.get("source_records"),
        rendered_content: row.get("rendered_content"),
        rebuild_status: row.get::<String, _>("rebuild_status").parse()?,
        staleness_hash: row.get("staleness_hash"),
        rebuild_receipt_event_id: row.get("rebuild_receipt_event_id"),
        last_rebuilt_at: row.get("last_rebuilt_at"),
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

    async fn get_knowledge_source(&self, source_id: &str)
        -> StorageResult<Option<KnowledgeSource>>;

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
        let rows = sqlx::query(&sql)
            .bind(root_id)
            .fetch_all(self.pool())
            .await?;
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
        if new_entity.entity_key.trim().is_empty()
            || new_entity.entity_key.trim() != new_entity.entity_key
        {
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

    async fn list_knowledge_entity_span_ids(&self, entity_id: &str) -> StorageResult<Vec<String>> {
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

    async fn upsert_knowledge_edge(
        &self,
        new_edge: NewKnowledgeEdge,
    ) -> StorageResult<KnowledgeEdge> {
        if new_edge.evidence_span_ids.is_empty() {
            return Err(StorageError::Validation(
                "knowledge edge MUST carry at least one source span ref (spec 2.3.13.11)",
            ));
        }
        if !(0.0..=1.0).contains(&new_edge.confidence) {
            return Err(StorageError::Validation(
                "knowledge edge confidence must be within [0.0, 1.0]",
            ));
        }
        if new_edge.extractor_version.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge edge extractor_version is required",
            ));
        }

        // Resolve both entities' stable natural identities for the
        // deterministic relationship_id derivation.
        let source = self
            .get_knowledge_entity(&new_edge.source_entity_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge edge source entity"))?;
        let target = self
            .get_knowledge_entity(&new_edge.target_entity_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge edge target entity"))?;
        if source.workspace_id != new_edge.workspace_id
            || target.workspace_id != new_edge.workspace_id
        {
            return Err(StorageError::Validation(
                "knowledge edge entities must belong to the edge workspace",
            ));
        }
        let relationship_id = derive_knowledge_relationship_id(
            new_edge.edge_type,
            source.entity_kind,
            &source.entity_key,
            target.entity_kind,
            &target.entity_key,
        );
        let edge_id = new_knowledge_id("KED");

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_edges
                (edge_id, workspace_id, relationship_id, edge_type,
                 source_entity_id, target_entity_id, extractor_version,
                 confidence, created_in_run, last_seen_in_run)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            ON CONFLICT (workspace_id, relationship_id)
            DO UPDATE SET
                confidence = EXCLUDED.confidence,
                extractor_version = EXCLUDED.extractor_version,
                last_seen_in_run = COALESCE(EXCLUDED.last_seen_in_run,
                                            knowledge_edges.last_seen_in_run),
                updated_at = NOW()
            RETURNING {KNOWLEDGE_EDGE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&edge_id)
            .bind(&new_edge.workspace_id)
            .bind(&relationship_id)
            .bind(new_edge.edge_type.as_str())
            .bind(&new_edge.source_entity_id)
            .bind(&new_edge.target_entity_id)
            .bind(&new_edge.extractor_version)
            .bind(new_edge.confidence)
            .bind(&new_edge.detected_in_run)
            .fetch_one(&mut *tx)
            .await?;
        let edge = edge_from_pg(&row)?;

        for span_id in &new_edge.evidence_span_ids {
            sqlx::query(
                r#"
                INSERT INTO knowledge_edge_spans (edge_id, span_id, recorded_in_run)
                VALUES ($1, $2, $3)
                ON CONFLICT (edge_id, span_id) DO NOTHING
                "#,
            )
            .bind(&edge.edge_id)
            .bind(span_id)
            .bind(&new_edge.detected_in_run)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(edge)
    }

    async fn get_knowledge_edge(&self, edge_id: &str) -> StorageResult<Option<KnowledgeEdge>> {
        let sql =
            format!("SELECT {KNOWLEDGE_EDGE_COLUMNS} FROM knowledge_edges WHERE edge_id = $1");
        let row = sqlx::query(&sql)
            .bind(edge_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(edge_from_pg).transpose()
    }

    async fn get_knowledge_edge_by_relationship_id(
        &self,
        workspace_id: &str,
        relationship_id: &str,
    ) -> StorageResult<Option<KnowledgeEdge>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_EDGE_COLUMNS} FROM knowledge_edges
             WHERE workspace_id = $1 AND relationship_id = $2"
        );
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(relationship_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(edge_from_pg).transpose()
    }

    async fn list_knowledge_edges_for_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Vec<KnowledgeEdge>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_EDGE_COLUMNS} FROM knowledge_edges
             WHERE source_entity_id = $1 OR target_entity_id = $1
             ORDER BY relationship_id"
        );
        let rows = sqlx::query(&sql)
            .bind(entity_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(edge_from_pg).collect()
    }

    async fn list_knowledge_edge_span_ids(&self, edge_id: &str) -> StorageResult<Vec<String>> {
        let rows = sqlx::query(
            "SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $1 ORDER BY span_id",
        )
        .bind(edge_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("span_id"))
            .collect())
    }

    async fn set_knowledge_edge_lifecycle(
        &self,
        edge_id: &str,
        lifecycle: KnowledgeEdgeLifecycle,
        conflict_marker: Option<Value>,
    ) -> StorageResult<KnowledgeEdge> {
        if matches!(lifecycle, KnowledgeEdgeLifecycle::Conflicted) && conflict_marker.is_none() {
            return Err(StorageError::Validation(
                "conflicted knowledge edges must carry a conflict marker",
            ));
        }
        let sql = format!(
            r#"
            UPDATE knowledge_edges
            SET lifecycle_state = $2,
                conflict_marker = $3,
                updated_at = NOW()
            WHERE edge_id = $1
            RETURNING {KNOWLEDGE_EDGE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(edge_id)
            .bind(lifecycle.as_str())
            .bind(&conflict_marker)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge edge"))?;
        edge_from_pg(&row)
    }

    async fn create_knowledge_claim(
        &self,
        new_claim: NewKnowledgeClaim,
    ) -> StorageResult<KnowledgeClaim> {
        if new_claim.evidence_span_ids.is_empty() {
            return Err(StorageError::Validation(
                "knowledge claim MUST carry evidence spans (spec 2.3.13.11)",
            ));
        }
        if new_claim.claim_text.trim().is_empty() {
            return Err(StorageError::Validation("knowledge claim_text is required"));
        }
        if !(0.0..=1.0).contains(&new_claim.confidence) {
            return Err(StorageError::Validation(
                "knowledge claim confidence must be within [0.0, 1.0]",
            ));
        }
        let claim_id = new_knowledge_id("KCL");

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_claims
                (claim_id, workspace_id, claim_kind, claim_text,
                 subject_entity_id, temporal_qualifier, granularity_qualifier,
                 confidence, proposed_in_run)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING {KNOWLEDGE_CLAIM_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&claim_id)
            .bind(&new_claim.workspace_id)
            .bind(new_claim.claim_kind.as_str())
            .bind(&new_claim.claim_text)
            .bind(&new_claim.subject_entity_id)
            .bind(&new_claim.temporal_qualifier)
            .bind(&new_claim.granularity_qualifier)
            .bind(new_claim.confidence)
            .bind(&new_claim.proposed_in_run)
            .fetch_one(&mut *tx)
            .await?;
        let claim = claim_from_pg(&row)?;

        for span_id in &new_claim.evidence_span_ids {
            sqlx::query(
                "INSERT INTO knowledge_claim_spans (claim_id, span_id)
                 VALUES ($1, $2) ON CONFLICT (claim_id, span_id) DO NOTHING",
            )
            .bind(&claim.claim_id)
            .bind(span_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(claim)
    }

    async fn get_knowledge_claim(&self, claim_id: &str) -> StorageResult<Option<KnowledgeClaim>> {
        let sql =
            format!("SELECT {KNOWLEDGE_CLAIM_COLUMNS} FROM knowledge_claims WHERE claim_id = $1");
        let row = sqlx::query(&sql)
            .bind(claim_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(claim_from_pg).transpose()
    }

    async fn list_knowledge_claim_span_ids(&self, claim_id: &str) -> StorageResult<Vec<String>> {
        let rows = sqlx::query(
            "SELECT span_id FROM knowledge_claim_spans WHERE claim_id = $1 ORDER BY span_id",
        )
        .bind(claim_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("span_id"))
            .collect())
    }

    async fn transition_knowledge_claim(
        &self,
        claim_id: &str,
        to_state: KnowledgeClaimState,
        retirement: Option<KnowledgeClaimRetirement>,
        resolution_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeClaim> {
        let current = self
            .get_knowledge_claim(claim_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge claim"))?;
        if !current.lifecycle_state.can_transition_to(to_state) {
            return Err(StorageError::Conflict(
                "knowledge claim lifecycle violation: transition not allowed",
            ));
        }
        let (retirement_reason, superseded_by) = match (to_state, retirement) {
            (KnowledgeClaimState::Retired, Some(retirement)) => {
                if matches!(
                    retirement.reason,
                    KnowledgeClaimRetirementReason::Superseded
                ) && retirement.superseded_by_claim_id.is_none()
                {
                    return Err(StorageError::Validation(
                        "superseded claims must name superseded_by_claim_id",
                    ));
                }
                if !matches!(
                    retirement.reason,
                    KnowledgeClaimRetirementReason::Superseded
                ) && retirement.superseded_by_claim_id.is_some()
                {
                    return Err(StorageError::Validation(
                        "superseded_by_claim_id requires retirement reason 'superseded'",
                    ));
                }
                (
                    Some(retirement.reason.as_str()),
                    retirement.superseded_by_claim_id,
                )
            }
            (KnowledgeClaimState::Retired, None) => {
                return Err(StorageError::Validation(
                    "retiring a knowledge claim requires a retirement reason",
                ));
            }
            (_, Some(_)) => {
                return Err(StorageError::Validation(
                    "retirement payload only applies when entering 'retired'",
                ));
            }
            (_, None) => (None, None),
        };

        // Optimistic transition: WHERE pins the observed source state so a
        // concurrent transition cannot be silently overwritten.
        let sql = format!(
            r#"
            UPDATE knowledge_claims
            SET lifecycle_state = $3,
                retirement_reason = $4,
                superseded_by_claim_id = $5,
                resolution_receipt_event_id = COALESCE($6, resolution_receipt_event_id),
                updated_at = NOW()
            WHERE claim_id = $1 AND lifecycle_state = $2
            RETURNING {KNOWLEDGE_CLAIM_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(claim_id)
            .bind(current.lifecycle_state.as_str())
            .bind(to_state.as_str())
            .bind(retirement_reason)
            .bind(&superseded_by)
            .bind(resolution_receipt_event_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::Conflict(
                "knowledge claim lifecycle violation: state changed concurrently",
            ))?;
        claim_from_pg(&row)
    }

    async fn record_knowledge_claim_conflict(
        &self,
        claim_id: &str,
        conflicting_claim_id: &str,
        conflict_reason: &str,
        detected_in_run: Option<&str>,
    ) -> StorageResult<KnowledgeClaimConflict> {
        if claim_id == conflicting_claim_id {
            return Err(StorageError::Validation(
                "a knowledge claim cannot conflict with itself",
            ));
        }
        if conflict_reason.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge claim conflict_reason is required",
            ));
        }
        let conflict_id = new_knowledge_id("KCC");
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_claim_conflicts
                (conflict_id, claim_id, conflicting_claim_id, conflict_reason,
                 detected_in_run)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING conflict_id, claim_id, conflicting_claim_id,
                      detected_in_run, conflict_reason,
                      resolution_receipt_event_id, detected_at, resolved_at
            "#,
        )
        .bind(&conflict_id)
        .bind(claim_id)
        .bind(conflicting_claim_id)
        .bind(conflict_reason)
        .bind(detected_in_run)
        .fetch_one(&mut *tx)
        .await?;
        let conflict = claim_conflict_from_pg(&row);

        // Both claims move to 'conflicted' unless already retired.
        for id in [claim_id, conflicting_claim_id] {
            sqlx::query(
                "UPDATE knowledge_claims
                 SET lifecycle_state = 'conflicted', updated_at = NOW()
                 WHERE claim_id = $1 AND lifecycle_state IN ('proposed', 'accepted')",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(conflict)
    }

    async fn resolve_knowledge_claim_conflict(
        &self,
        conflict_id: &str,
        resolution_receipt_event_id: &str,
    ) -> StorageResult<KnowledgeClaimConflict> {
        let row = sqlx::query(
            r#"
            UPDATE knowledge_claim_conflicts
            SET resolution_receipt_event_id = $2, resolved_at = NOW()
            WHERE conflict_id = $1 AND resolved_at IS NULL
            RETURNING conflict_id, claim_id, conflicting_claim_id,
                      detected_in_run, conflict_reason,
                      resolution_receipt_event_id, detected_at, resolved_at
            "#,
        )
        .bind(conflict_id)
        .bind(resolution_receipt_event_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(claim_conflict_from_pg(&row)),
            None => {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM knowledge_claim_conflicts WHERE conflict_id = $1)",
                )
                .bind(conflict_id)
                .fetch_one(self.pool())
                .await?;
                if exists {
                    Err(StorageError::Conflict(
                        "knowledge claim conflict is already resolved",
                    ))
                } else {
                    Err(StorageError::NotFound("knowledge claim conflict"))
                }
            }
        }
    }

    async fn list_knowledge_claim_conflicts(
        &self,
        claim_id: &str,
    ) -> StorageResult<Vec<KnowledgeClaimConflict>> {
        let rows = sqlx::query(
            r#"
            SELECT conflict_id, claim_id, conflicting_claim_id, detected_in_run,
                   conflict_reason, resolution_receipt_event_id, detected_at,
                   resolved_at
            FROM knowledge_claim_conflicts
            WHERE claim_id = $1 OR conflicting_claim_id = $1
            ORDER BY detected_at
            "#,
        )
        .bind(claim_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(claim_conflict_from_pg).collect())
    }

    async fn create_knowledge_memory_passage(
        &self,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeMemoryPassage> {
        if new_passage.evidence.is_empty() {
            return Err(StorageError::Validation(
                "knowledge memory passages are derived from sources and claims; evidence is required (spec 2.3.13.11)",
            ));
        }
        if new_passage.passage_text.is_empty() {
            return Err(StorageError::Validation(
                "knowledge passage_text is required",
            ));
        }
        if !(0.0..=1.0).contains(&new_passage.extraction_confidence) {
            return Err(StorageError::Validation(
                "knowledge passage extraction_confidence must be within [0.0, 1.0]",
            ));
        }
        let passage_id = new_knowledge_id("KMP");

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_memory_passages
                (passage_id, workspace_id, passage_text, token_count,
                 ocr_transcript_metadata, extraction_confidence,
                 ranking_features, retrieval_mode, compaction_policy,
                 failure_receipt_event_id, derived_in_run)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING {KNOWLEDGE_PASSAGE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&passage_id)
            .bind(&new_passage.workspace_id)
            .bind(&new_passage.passage_text)
            .bind(new_passage.token_count)
            .bind(&new_passage.ocr_transcript_metadata)
            .bind(new_passage.extraction_confidence)
            .bind(&new_passage.ranking_features)
            .bind(new_passage.retrieval_mode.as_str())
            .bind(new_passage.compaction_policy.as_str())
            .bind(&new_passage.failure_receipt_event_id)
            .bind(&new_passage.derived_in_run)
            .fetch_one(&mut *tx)
            .await?;
        let passage = passage_from_pg(&row)?;

        for (index, evidence) in new_passage.evidence.iter().enumerate() {
            let (ref_kind, source_id, claim_id, span_id) = match evidence {
                KnowledgePassageEvidenceRef::Source { source_id } => {
                    ("source", Some(source_id.as_str()), None, None)
                }
                KnowledgePassageEvidenceRef::Claim { claim_id } => {
                    ("claim", None, Some(claim_id.as_str()), None)
                }
                KnowledgePassageEvidenceRef::Span { span_id } => {
                    ("span", None, None, Some(span_id.as_str()))
                }
            };
            sqlx::query(
                r#"
                INSERT INTO knowledge_passage_evidence
                    (passage_id, ref_kind, source_id, claim_id, span_id, ordinal)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&passage.passage_id)
            .bind(ref_kind)
            .bind(source_id)
            .bind(claim_id)
            .bind(span_id)
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(passage)
    }

    async fn get_knowledge_memory_passage(
        &self,
        passage_id: &str,
    ) -> StorageResult<Option<KnowledgeMemoryPassage>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_PASSAGE_COLUMNS} FROM knowledge_memory_passages
             WHERE passage_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(passage_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(passage_from_pg).transpose()
    }

    async fn list_knowledge_passage_evidence(
        &self,
        passage_id: &str,
    ) -> StorageResult<Vec<KnowledgePassageEvidenceRef>> {
        let rows = sqlx::query(
            r#"
            SELECT ref_kind, source_id, claim_id, span_id
            FROM knowledge_passage_evidence
            WHERE passage_id = $1
            ORDER BY ordinal
            "#,
        )
        .bind(passage_id)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter()
            .map(|row| {
                let ref_kind: String = row.get("ref_kind");
                match ref_kind.as_str() {
                    "source" => Ok(KnowledgePassageEvidenceRef::Source {
                        source_id: row.get::<Option<String>, _>("source_id").ok_or(
                            StorageError::Validation("passage evidence row missing source_id"),
                        )?,
                    }),
                    "claim" => Ok(KnowledgePassageEvidenceRef::Claim {
                        claim_id: row.get::<Option<String>, _>("claim_id").ok_or(
                            StorageError::Validation("passage evidence row missing claim_id"),
                        )?,
                    }),
                    "span" => Ok(KnowledgePassageEvidenceRef::Span {
                        span_id: row.get::<Option<String>, _>("span_id").ok_or(
                            StorageError::Validation("passage evidence row missing span_id"),
                        )?,
                    }),
                    _ => Err(StorageError::Validation(
                        "invalid knowledge passage evidence ref_kind",
                    )),
                }
            })
            .collect()
    }

    async fn set_knowledge_passage_compaction(
        &self,
        passage_id: &str,
        compaction_policy: KnowledgeCompactionPolicy,
        refresh_freshness: bool,
    ) -> StorageResult<KnowledgeMemoryPassage> {
        let sql = format!(
            r#"
            UPDATE knowledge_memory_passages
            SET compaction_policy = $2,
                freshness_at = CASE WHEN $3 THEN NOW() ELSE freshness_at END,
                updated_at = NOW()
            WHERE passage_id = $1
            RETURNING {KNOWLEDGE_PASSAGE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(passage_id)
            .bind(compaction_policy.as_str())
            .bind(refresh_freshness)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge memory passage"))?;
        passage_from_pg(&row)
    }

    async fn upsert_knowledge_wiki_projection(
        &self,
        new_projection: NewKnowledgeWikiProjection,
    ) -> StorageResult<KnowledgeWikiProjection> {
        if new_projection.title.trim() != new_projection.title || new_projection.title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge projection title must be non-empty and trimmed",
            ));
        }
        if !is_sha256_hex(&new_projection.staleness_hash) {
            return Err(StorageError::Validation(
                "knowledge projection staleness_hash must be lowercase sha256 hex",
            ));
        }
        let projection_id = new_knowledge_id("KWP");
        let sql = format!(
            r#"
            INSERT INTO knowledge_wiki_projections
                (projection_id, workspace_id, projection_kind, title,
                 source_records, rendered_content, rebuild_status, staleness_hash)
            VALUES ($1, $2, $3, $4, $5, $6, 'stale', $7)
            ON CONFLICT (workspace_id, projection_kind, title) DO UPDATE SET
                source_records = EXCLUDED.source_records,
                rendered_content = EXCLUDED.rendered_content,
                rebuild_status = 'stale',
                staleness_hash = EXCLUDED.staleness_hash,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_PROJECTION_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&projection_id)
            .bind(&new_projection.workspace_id)
            .bind(new_projection.projection_kind.as_str())
            .bind(&new_projection.title)
            .bind(&new_projection.source_records)
            .bind(&new_projection.rendered_content)
            .bind(&new_projection.staleness_hash)
            .fetch_one(self.pool())
            .await?;
        projection_from_pg(&row)
    }

    async fn get_knowledge_wiki_projection(
        &self,
        projection_id: &str,
    ) -> StorageResult<Option<KnowledgeWikiProjection>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_PROJECTION_COLUMNS} FROM knowledge_wiki_projections
             WHERE projection_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(projection_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(projection_from_pg).transpose()
    }

    async fn mark_knowledge_projection_rebuilt(
        &self,
        projection_id: &str,
        staleness_hash: &str,
        rendered_content: &str,
        rebuild_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeWikiProjection> {
        if !is_sha256_hex(staleness_hash) {
            return Err(StorageError::Validation(
                "knowledge projection staleness_hash must be lowercase sha256 hex",
            ));
        }
        let sql = format!(
            r#"
            UPDATE knowledge_wiki_projections
            SET rebuild_status = 'fresh',
                staleness_hash = $2,
                rendered_content = $3,
                rebuild_receipt_event_id = $4,
                last_rebuilt_at = NOW(),
                updated_at = NOW()
            WHERE projection_id = $1
            RETURNING {KNOWLEDGE_PROJECTION_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(projection_id)
            .bind(staleness_hash)
            .bind(rendered_content)
            .bind(rebuild_receipt_event_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge wiki projection"))?;
        projection_from_pg(&row)
    }

    async fn set_knowledge_projection_rebuild_status(
        &self,
        projection_id: &str,
        rebuild_status: KnowledgeRebuildStatus,
    ) -> StorageResult<KnowledgeWikiProjection> {
        let sql = format!(
            r#"
            UPDATE knowledge_wiki_projections
            SET rebuild_status = $2, updated_at = NOW()
            WHERE projection_id = $1
            RETURNING {KNOWLEDGE_PROJECTION_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(projection_id)
            .bind(rebuild_status.as_str())
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge wiki projection"))?;
        projection_from_pg(&row)
    }

    async fn delete_knowledge_wiki_projection(&self, projection_id: &str) -> StorageResult<()> {
        let result = sqlx::query("DELETE FROM knowledge_wiki_projections WHERE projection_id = $1")
            .bind(projection_id)
            .execute(self.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("knowledge wiki projection"));
        }
        Ok(())
    }
}
