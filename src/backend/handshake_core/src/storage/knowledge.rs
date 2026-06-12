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

/// `KNOWLEDGE_ENTITY_COLUMNS` qualified with an `e.` table alias for joined
/// selects (column names in the result stay unqualified, so `entity_from_pg`
/// reads them unchanged).
const KNOWLEDGE_ENTITY_COLUMNS_E: &str = r#"
    e.entity_id, e.workspace_id, e.entity_kind, e.entity_key, e.display_name,
    e.detection_provenance, e.lifecycle_state, e.primary_source_id,
    e.first_detected_in_run, e.last_detected_in_run, e.created_at, e.updated_at
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
        page_type: row.get("page_type"),
        compile_stamp: row.get("compile_stamp"),
        compile_recipe: row.get("compile_recipe"),
        page_links: row.get("page_links"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

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

/// A versioned ProseMirror/Tiptap document JSON authority record.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRichDocument {
    pub rich_document_id: String,
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

fn rich_document_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeRichDocument {
    KnowledgeRichDocument {
        rich_document_id: row.get("rich_document_id"),
        workspace_id: row.get("workspace_id"),
        document_id: row.get("document_id"),
        title: row.get("title"),
        schema_version: row.get("schema_version"),
        doc_version: row.get("doc_version"),
        content_json: row.get("content_json"),
        content_sha256: row.get("content_sha256"),
        crdt_document_id: row.get("crdt_document_id"),
        crdt_snapshot_id: row.get("crdt_snapshot_id"),
        promotion_receipt_event_id: row.get("promotion_receipt_event_id"),
        projection_refs: row.get("projection_refs"),
        project_ref: row.get("project_ref"),
        folder_ref: row.get("folder_ref"),
        authority_label: row.get("authority_label"),
        owner_actor_kind: row.get("owner_actor_kind"),
        owner_actor_id: row.get("owner_actor_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

const KNOWLEDGE_CODE_NODE_COLUMNS: &str = r#"
    code_node_id, rich_document_id, node_path, language_id, code_text,
    round_trip_sha256, worker_requirements, source_mapping, lint_diagnostics,
    created_at, updated_at
"#;

fn code_node_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeEditorCodeNode {
    KnowledgeEditorCodeNode {
        code_node_id: row.get("code_node_id"),
        rich_document_id: row.get("rich_document_id"),
        node_path: row.get("node_path"),
        language_id: row.get("language_id"),
        code_text: row.get("code_text"),
        round_trip_sha256: row.get("round_trip_sha256"),
        worker_requirements: row.get("worker_requirements"),
        source_mapping: row.get("source_mapping"),
        lint_diagnostics: row.get("lint_diagnostics"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
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

const KNOWLEDGE_DOCUMENT_EMBED_COLUMNS: &str = r#"
    embed_id, rich_document_id, block_id, ref_kind, ref_value, caption,
    repair_state, repair_reason, created_at, updated_at
"#;

fn document_embed_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeDocumentEmbed {
    KnowledgeDocumentEmbed {
        embed_id: row.get("embed_id"),
        rich_document_id: row.get("rich_document_id"),
        block_id: row.get("block_id"),
        ref_kind: row.get("ref_kind"),
        ref_value: row.get("ref_value"),
        caption: row.get("caption"),
        repair_state: row.get("repair_state"),
        repair_reason: row.get("repair_reason"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
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

const KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS: &str = r#"
    backlink_id, workspace_id, relationship_id, source_document_id, link_kind,
    target, block_id, created_at, updated_at
"#;

fn document_backlink_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeDocumentBacklink {
    KnowledgeDocumentBacklink {
        backlink_id: row.get("backlink_id"),
        workspace_id: row.get("workspace_id"),
        relationship_id: row.get("relationship_id"),
        source_document_id: row.get("source_document_id"),
        link_kind: row.get("link_kind"),
        target: row.get("target"),
        block_id: row.get("block_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

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

fn bundle_from_pg(row: &sqlx::postgres::PgRow) -> KnowledgeContextBundle {
    KnowledgeContextBundle {
        bundle_id: row.get("bundle_id"),
        workspace_id: row.get("workspace_id"),
        kernel_task_run_id: row.get("kernel_task_run_id"),
        session_run_id: row.get("session_run_id"),
        allowed_context: row.get("allowed_context"),
        context_hash: row.get("context_hash"),
        query_text: row.get("query_text"),
        token_budget: row.get("token_budget"),
        tokens_used: row.get("tokens_used"),
        build_receipt_event_id: row.get("build_receipt_event_id"),
        created_at: row.get("created_at"),
    }
}

fn bundle_item_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeContextBundleItem> {
    Ok(KnowledgeContextBundleItem {
        bundle_id: row.get("bundle_id"),
        item_ordinal: row.get("item_ordinal"),
        ref_kind: row.get::<String, _>("ref_kind").parse()?,
        ref_id: row.get("ref_id"),
        retrieval_decision: row.get::<String, _>("retrieval_decision").parse()?,
        relevance_score: row.get("relevance_score"),
        token_count: row.get("token_count"),
        citation: row.get("citation"),
    })
}

const KNOWLEDGE_TRACE_COLUMNS: &str = r#"
    trace_id, workspace_id, retrieval_mode, mode_reason, query_text,
    bundle_id, decisions, trace_receipt_event_id, created_at
"#;

fn trace_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeRetrievalTrace> {
    Ok(KnowledgeRetrievalTrace {
        trace_id: row.get("trace_id"),
        workspace_id: row.get("workspace_id"),
        retrieval_mode: row.get::<String, _>("retrieval_mode").parse()?,
        mode_reason: row.get("mode_reason"),
        query_text: row.get("query_text"),
        bundle_id: row.get("bundle_id"),
        decisions: row.get("decisions"),
        trace_receipt_event_id: row.get("trace_receipt_event_id"),
        created_at: row.get("created_at"),
    })
}

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

/// Inserts a memory passage plus its REQUIRED lineage inside the caller's
/// transaction (shared by the plain and idempotent create paths).
async fn insert_knowledge_memory_passage_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_passage: &NewKnowledgeMemoryPassage,
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
        .fetch_one(&mut **tx)
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
        .execute(&mut **tx)
        .await?;
    }
    Ok(passage)
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

/// Looks up a committed idempotency key. Same hash -> the prior result ref;
/// different hash -> typed Conflict (divergent duplicate).
async fn find_knowledge_idempotency_result(
    pool: &sqlx::PgPool,
    idempotency_key: &str,
    request_hash: &str,
) -> StorageResult<Option<(String, String)>> {
    let row = sqlx::query(
        r#"
        SELECT request_hash, result_ref_kind, result_ref_id
        FROM knowledge_idempotency_keys
        WHERE idempotency_key = $1
        "#,
    )
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_hash: String = row.get("request_hash");
    if stored_hash != request_hash {
        return Err(StorageError::Conflict(
            "knowledge idempotency key replayed with a different request payload",
        ));
    }
    Ok(Some((row.get("result_ref_kind"), row.get("result_ref_id"))))
}

/// Claims the key inside the caller's transaction AFTER the write. Returns
/// false when another writer holds the key (the caller must roll back and
/// re-read the winner's result).
async fn claim_knowledge_idempotency_key_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    idempotency_key: &str,
    workspace_id: &str,
    operation_kind: KnowledgeIdempotentOperationKind,
    request_hash: &str,
    result_ref_kind: &str,
    result_ref_id: &str,
) -> StorageResult<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO knowledge_idempotency_keys
            (idempotency_key, workspace_id, operation_kind, request_hash,
             result_ref_kind, result_ref_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(idempotency_key)
    .bind(workspace_id)
    .bind(operation_kind.as_str())
    .bind(request_hash)
    .bind(result_ref_kind)
    .bind(result_ref_id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
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

    async fn get_knowledge_rich_document(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>>;

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
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeRichDocument>>;
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

impl PostgresDatabase {
    /// MT-062 inner save: optimistic update + history append + key claim in
    /// ONE transaction. `Ok(None)` = the key claim lost a race and the whole
    /// write rolled back (no double-write).
    async fn save_knowledge_rich_document_version_with_key(
        &self,
        idempotency_key: &str,
        rich_document_id: &str,
        expected_version: i64,
        content_json: &Value,
        promotion_receipt_event_id: Option<&str>,
        request_hash: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>> {
        let content_sha256 = knowledge_canonical_json_sha256(content_json);

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            UPDATE knowledge_rich_documents
            SET doc_version = doc_version + 1,
                content_json = $3,
                content_sha256 = $4,
                promotion_receipt_event_id = $5,
                updated_at = NOW()
            WHERE rich_document_id = $1 AND doc_version = $2
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .bind(expected_version)
            .bind(content_json)
            .bind(&content_sha256)
            .bind(promotion_receipt_event_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(row) = row else {
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT doc_version FROM knowledge_rich_documents WHERE rich_document_id = $1",
            )
            .bind(rich_document_id)
            .fetch_optional(&mut *tx)
            .await?;
            return Err(match exists {
                Some(_) => StorageError::Conflict(
                    "knowledge rich document version conflict: expected_version is stale",
                ),
                None => StorageError::NotFound("knowledge rich document"),
            });
        };
        let document = rich_document_from_pg(&row);

        sqlx::query(
            r#"
            INSERT INTO knowledge_rich_document_versions
                (rich_document_id, doc_version, schema_version, content_json,
                 content_sha256, crdt_snapshot_id, promotion_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&document.rich_document_id)
        .bind(document.doc_version)
        .bind(&document.schema_version)
        .bind(&document.content_json)
        .bind(&document.content_sha256)
        .bind(&document.crdt_snapshot_id)
        .bind(&document.promotion_receipt_event_id)
        .execute(&mut *tx)
        .await?;

        let claimed = claim_knowledge_idempotency_key_tx(
            &mut tx,
            idempotency_key,
            &document.workspace_id,
            KnowledgeIdempotentOperationKind::RichDocumentSave,
            request_hash,
            "rich_document",
            &document.rich_document_id,
        )
        .await?;
        if !claimed {
            drop(tx);
            return Ok(None);
        }
        tx.commit().await?;
        Ok(Some(document))
    }
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

    async fn get_knowledge_source_by_document_id(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> StorageResult<Option<KnowledgeSource>> {
        // The rich-document linkage is provenance-keyed: the schema's
        // `document_id` column FKs the legacy `documents` table, so a
        // RichDocument (KRD-...) source carries its id in
        // `provenance.rich_document_id` instead (MT-154).
        let sql = format!(
            "SELECT {KNOWLEDGE_SOURCE_COLUMNS} FROM knowledge_sources
             WHERE workspace_id = $1
               AND source_kind = 'rich_document'
               AND provenance->>'rich_document_id' = $2
             ORDER BY created_at
             LIMIT 1"
        );
        let row = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(document_id)
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
        let mut tx = self.pool().begin().await?;
        let passage = insert_knowledge_memory_passage_tx(&mut tx, &new_passage).await?;
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

    async fn create_knowledge_rich_document(
        &self,
        new_document: NewKnowledgeRichDocument,
    ) -> StorageResult<KnowledgeRichDocument> {
        if new_document.title.trim() != new_document.title || new_document.title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge rich document title must be non-empty and trimmed",
            ));
        }
        if new_document.schema_version.trim() != new_document.schema_version
            || new_document.schema_version.is_empty()
        {
            return Err(StorageError::Validation(
                "knowledge rich document schema_version must be non-empty and trimmed",
            ));
        }
        // MT-145 identity defaults + validation. authority_label defaults to
        // 'promoted'; an owner is all-or-nothing (kind <-> id).
        let authority_label = new_document
            .authority_label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("promoted")
            .to_string();
        if !matches!(authority_label.as_str(), "draft" | "promoted" | "archived") {
            return Err(StorageError::Validation(
                "knowledge rich document authority_label must be draft|promoted|archived",
            ));
        }
        if new_document.owner_actor_kind.is_some() != new_document.owner_actor_id.is_some() {
            return Err(StorageError::Validation(
                "knowledge rich document owner_actor_kind and owner_actor_id must be set together",
            ));
        }
        let rich_document_id = new_knowledge_id("KRD");
        let content_sha256 = knowledge_canonical_json_sha256(&new_document.content_json);

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_rich_documents
                (rich_document_id, workspace_id, document_id, title,
                 schema_version, content_json, content_sha256,
                 crdt_document_id, crdt_snapshot_id, promotion_receipt_event_id,
                 project_ref, folder_ref, authority_label,
                 owner_actor_kind, owner_actor_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&rich_document_id)
            .bind(&new_document.workspace_id)
            .bind(&new_document.document_id)
            .bind(&new_document.title)
            .bind(&new_document.schema_version)
            .bind(&new_document.content_json)
            .bind(&content_sha256)
            .bind(&new_document.crdt_document_id)
            .bind(&new_document.crdt_snapshot_id)
            .bind(&new_document.promotion_receipt_event_id)
            .bind(&new_document.project_ref)
            .bind(&new_document.folder_ref)
            .bind(&authority_label)
            .bind(&new_document.owner_actor_kind)
            .bind(&new_document.owner_actor_id)
            .fetch_one(&mut *tx)
            .await?;
        let document = rich_document_from_pg(&row);

        // Revision 1 lands in the append-only history at creation so the
        // history is complete from the first promoted revision.
        sqlx::query(
            r#"
            INSERT INTO knowledge_rich_document_versions
                (rich_document_id, doc_version, schema_version, content_json,
                 content_sha256, crdt_snapshot_id, promotion_receipt_event_id)
            VALUES ($1, 1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&document.rich_document_id)
        .bind(&document.schema_version)
        .bind(&document.content_json)
        .bind(&document.content_sha256)
        .bind(&document.crdt_snapshot_id)
        .bind(&document.promotion_receipt_event_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(document)
    }

    async fn get_knowledge_rich_document(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_RICH_DOCUMENT_COLUMNS} FROM knowledge_rich_documents
             WHERE rich_document_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .fetch_optional(self.pool())
            .await?;
        Ok(row.as_ref().map(rich_document_from_pg))
    }

    async fn save_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        expected_version: i64,
        content_json: Value,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument> {
        let content_sha256 = knowledge_canonical_json_sha256(&content_json);

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            UPDATE knowledge_rich_documents
            SET doc_version = doc_version + 1,
                content_json = $3,
                content_sha256 = $4,
                promotion_receipt_event_id = $5,
                updated_at = NOW()
            WHERE rich_document_id = $1 AND doc_version = $2
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .bind(expected_version)
            .bind(&content_json)
            .bind(&content_sha256)
            .bind(promotion_receipt_event_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(row) = row else {
            // Distinguish a stale expected_version (typed Conflict, the
            // optimistic-concurrency fail-closed path) from a missing doc.
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT doc_version FROM knowledge_rich_documents WHERE rich_document_id = $1",
            )
            .bind(rich_document_id)
            .fetch_optional(&mut *tx)
            .await?;
            return Err(match exists {
                Some(_) => StorageError::Conflict(
                    "knowledge rich document version conflict: expected_version is stale",
                ),
                None => StorageError::NotFound("knowledge rich document"),
            });
        };
        let document = rich_document_from_pg(&row);

        sqlx::query(
            r#"
            INSERT INTO knowledge_rich_document_versions
                (rich_document_id, doc_version, schema_version, content_json,
                 content_sha256, crdt_snapshot_id, promotion_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&document.rich_document_id)
        .bind(document.doc_version)
        .bind(&document.schema_version)
        .bind(&document.content_json)
        .bind(&document.content_sha256)
        .bind(&document.crdt_snapshot_id)
        .bind(&document.promotion_receipt_event_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(document)
    }

    async fn list_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersion>> {
        let rows = sqlx::query(
            r#"
            SELECT rich_document_id, doc_version, schema_version, content_json,
                   content_sha256, crdt_snapshot_id, promotion_receipt_event_id,
                   created_at
            FROM knowledge_rich_document_versions
            WHERE rich_document_id = $1
            ORDER BY doc_version
            "#,
        )
        .bind(rich_document_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| KnowledgeRichDocumentVersion {
                rich_document_id: row.get("rich_document_id"),
                doc_version: row.get("doc_version"),
                schema_version: row.get("schema_version"),
                content_json: row.get("content_json"),
                content_sha256: row.get("content_sha256"),
                crdt_snapshot_id: row.get("crdt_snapshot_id"),
                promotion_receipt_event_id: row.get("promotion_receipt_event_id"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    async fn list_knowledge_rich_document_version_metas(
        &self,
        rich_document_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersionMeta>> {
        // MT-156: metadata only — content_json is deliberately NOT selected so
        // a long history can never balloon the response.
        let rows = sqlx::query(
            r#"
            SELECT rich_document_id, doc_version, schema_version,
                   content_sha256, crdt_snapshot_id, promotion_receipt_event_id,
                   created_at
            FROM knowledge_rich_document_versions
            WHERE rich_document_id = $1
            ORDER BY doc_version
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(rich_document_id)
        .bind(limit.max(0))
        .bind(offset.max(0))
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| KnowledgeRichDocumentVersionMeta {
                rich_document_id: row.get("rich_document_id"),
                doc_version: row.get("doc_version"),
                schema_version: row.get("schema_version"),
                content_sha256: row.get("content_sha256"),
                crdt_snapshot_id: row.get("crdt_snapshot_id"),
                promotion_receipt_event_id: row.get("promotion_receipt_event_id"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    async fn count_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_rich_document_versions WHERE rich_document_id = $1",
        )
        .bind(rich_document_id)
        .fetch_one(self.pool())
        .await?;
        Ok(count)
    }

    async fn get_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        doc_version: i64,
    ) -> StorageResult<Option<KnowledgeRichDocumentVersion>> {
        let row = sqlx::query(
            r#"
            SELECT rich_document_id, doc_version, schema_version, content_json,
                   content_sha256, crdt_snapshot_id, promotion_receipt_event_id,
                   created_at
            FROM knowledge_rich_document_versions
            WHERE rich_document_id = $1 AND doc_version = $2
            "#,
        )
        .bind(rich_document_id)
        .bind(doc_version)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(|row| KnowledgeRichDocumentVersion {
            rich_document_id: row.get("rich_document_id"),
            doc_version: row.get("doc_version"),
            schema_version: row.get("schema_version"),
            content_json: row.get("content_json"),
            content_sha256: row.get("content_sha256"),
            crdt_snapshot_id: row.get("crdt_snapshot_id"),
            promotion_receipt_event_id: row.get("promotion_receipt_event_id"),
            created_at: row.get("created_at"),
        }))
    }

    async fn rename_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        title: &str,
    ) -> StorageResult<KnowledgeRichDocument> {
        if title.trim() != title || title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge rich document title must be non-empty and trimmed",
            ));
        }
        let sql = format!(
            r#"
            UPDATE knowledge_rich_documents
            SET title = $2, updated_at = NOW()
            WHERE rich_document_id = $1
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .bind(title)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        Ok(rich_document_from_pg(&row))
    }

    async fn move_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument> {
        let sql = format!(
            r#"
            UPDATE knowledge_rich_documents
            SET project_ref = $2, folder_ref = $3, updated_at = NOW()
            WHERE rich_document_id = $1
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .bind(project_ref)
            .bind(folder_ref)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        Ok(rich_document_from_pg(&row))
    }

    async fn set_knowledge_rich_document_authority_label(
        &self,
        rich_document_id: &str,
        authority_label: &str,
    ) -> StorageResult<KnowledgeRichDocument> {
        if !matches!(authority_label, "draft" | "promoted" | "archived") {
            return Err(StorageError::Validation(
                "knowledge rich document authority_label must be draft|promoted|archived",
            ));
        }
        let sql = format!(
            r#"
            UPDATE knowledge_rich_documents
            SET authority_label = $2, updated_at = NOW()
            WHERE rich_document_id = $1
            RETURNING {KNOWLEDGE_RICH_DOCUMENT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(rich_document_id)
            .bind(authority_label)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        Ok(rich_document_from_pg(&row))
    }

    async fn list_knowledge_rich_documents(
        &self,
        workspace_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<Vec<KnowledgeRichDocument>> {
        // NULL-safe optional scoping: when an arg is None it is not filtered.
        let sql = format!(
            r#"
            SELECT {KNOWLEDGE_RICH_DOCUMENT_COLUMNS} FROM knowledge_rich_documents
            WHERE workspace_id = $1
              AND ($2::text IS NULL OR project_ref = $2)
              AND ($3::text IS NULL OR folder_ref = $3)
            ORDER BY updated_at DESC, rich_document_id
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(project_ref)
            .bind(folder_ref)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(rich_document_from_pg).collect())
    }

    async fn upsert_knowledge_editor_code_node(
        &self,
        upsert: UpsertEditorCodeNode,
    ) -> StorageResult<KnowledgeEditorCodeNode> {
        if upsert.node_path.trim() != upsert.node_path || upsert.node_path.is_empty() {
            return Err(StorageError::Validation(
                "knowledge editor code node node_path must be non-empty and trimmed",
            ));
        }
        if upsert.language_id.trim() != upsert.language_id || upsert.language_id.is_empty() {
            return Err(StorageError::Validation(
                "knowledge editor code node language_id must be non-empty and trimmed",
            ));
        }
        let code_node_id = new_knowledge_id("KCN");
        let round_trip_sha256 =
            crate::kernel::context_bundle::sha256_hex(upsert.code_text.as_bytes());
        let sql = format!(
            r#"
            INSERT INTO knowledge_editor_code_nodes
                (code_node_id, rich_document_id, node_path, language_id,
                 code_text, round_trip_sha256, worker_requirements,
                 source_mapping, lint_diagnostics)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (rich_document_id, node_path) DO UPDATE SET
                language_id = EXCLUDED.language_id,
                code_text = EXCLUDED.code_text,
                round_trip_sha256 = EXCLUDED.round_trip_sha256,
                worker_requirements = EXCLUDED.worker_requirements,
                source_mapping = EXCLUDED.source_mapping,
                lint_diagnostics = EXCLUDED.lint_diagnostics,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_CODE_NODE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&code_node_id)
            .bind(&upsert.rich_document_id)
            .bind(&upsert.node_path)
            .bind(&upsert.language_id)
            .bind(&upsert.code_text)
            .bind(&round_trip_sha256)
            .bind(&upsert.worker_requirements)
            .bind(&upsert.source_mapping)
            .bind(&upsert.lint_diagnostics)
            .fetch_one(self.pool())
            .await?;
        Ok(code_node_from_pg(&row))
    }

    async fn list_knowledge_editor_code_nodes(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeEditorCodeNode>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_CODE_NODE_COLUMNS} FROM knowledge_editor_code_nodes
             WHERE rich_document_id = $1 ORDER BY node_path"
        );
        let rows = sqlx::query(&sql)
            .bind(rich_document_id)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(code_node_from_pg).collect())
    }

    async fn upsert_knowledge_document_embed(
        &self,
        upsert: UpsertKnowledgeDocumentEmbed,
    ) -> StorageResult<KnowledgeDocumentEmbed> {
        if upsert.block_id.trim() != upsert.block_id || upsert.block_id.is_empty() {
            return Err(StorageError::Validation(
                "knowledge document embed block_id must be non-empty and trimmed",
            ));
        }
        if !matches!(
            upsert.ref_kind.as_str(),
            "artifact" | "media" | "source" | "url"
        ) {
            return Err(StorageError::Validation(
                "knowledge document embed ref_kind must be artifact|media|source|url",
            ));
        }
        let embed_id = new_knowledge_id("KEMB");
        let sql = format!(
            r#"
            INSERT INTO knowledge_document_embeds
                (embed_id, rich_document_id, block_id, ref_kind, ref_value,
                 caption)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (rich_document_id, block_id) DO UPDATE SET
                ref_kind = EXCLUDED.ref_kind,
                ref_value = EXCLUDED.ref_value,
                caption = EXCLUDED.caption,
                -- An upsert re-points the embed; resolution is fresh, so reset
                -- the repair state to ok (MT-153 repair through relink).
                repair_state = 'ok',
                repair_reason = NULL,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_DOCUMENT_EMBED_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&embed_id)
            .bind(&upsert.rich_document_id)
            .bind(&upsert.block_id)
            .bind(&upsert.ref_kind)
            .bind(&upsert.ref_value)
            .bind(&upsert.caption)
            .fetch_one(self.pool())
            .await?;
        Ok(document_embed_from_pg(&row))
    }

    async fn list_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_DOCUMENT_EMBED_COLUMNS} FROM knowledge_document_embeds
             WHERE rich_document_id = $1 ORDER BY block_id"
        );
        let rows = sqlx::query(&sql)
            .bind(rich_document_id)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(document_embed_from_pg).collect())
    }

    async fn set_knowledge_document_embed_repair_state(
        &self,
        embed_id: &str,
        broken_reason: Option<&str>,
    ) -> StorageResult<KnowledgeDocumentEmbed> {
        let (state, reason) = match broken_reason {
            Some(reason) => ("broken", Some(reason)),
            None => ("ok", None),
        };
        let sql = format!(
            r#"
            UPDATE knowledge_document_embeds
            SET repair_state = $2, repair_reason = $3, updated_at = NOW()
            WHERE embed_id = $1
            RETURNING {KNOWLEDGE_DOCUMENT_EMBED_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(embed_id)
            .bind(state)
            .bind(reason)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge document embed"))?;
        Ok(document_embed_from_pg(&row))
    }

    async fn replace_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentEmbed>,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        // Sync semantics (MT-152): the document content is the source of
        // truth, so a re-save is delete-all-for-document + insert in one
        // transaction (mirrors replace_knowledge_document_backlinks).
        for upsert in &upserts {
            if upsert.block_id.trim() != upsert.block_id || upsert.block_id.is_empty() {
                return Err(StorageError::Validation(
                    "knowledge document embed block_id must be non-empty and trimmed",
                ));
            }
            if !matches!(
                upsert.ref_kind.as_str(),
                "artifact" | "media" | "source" | "url"
            ) {
                return Err(StorageError::Validation(
                    "knowledge document embed ref_kind must be artifact|media|source|url",
                ));
            }
        }
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM knowledge_document_embeds WHERE rich_document_id = $1")
            .bind(rich_document_id)
            .execute(&mut *tx)
            .await?;
        let mut out = Vec::with_capacity(upserts.len());
        for upsert in upserts {
            let embed_id = new_knowledge_id("KEMB");
            let sql = format!(
                r#"
                INSERT INTO knowledge_document_embeds
                    (embed_id, rich_document_id, block_id, ref_kind, ref_value,
                     caption)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING {KNOWLEDGE_DOCUMENT_EMBED_COLUMNS}
                "#
            );
            let row = sqlx::query(&sql)
                .bind(&embed_id)
                .bind(rich_document_id)
                .bind(&upsert.block_id)
                .bind(&upsert.ref_kind)
                .bind(&upsert.ref_value)
                .bind(&upsert.caption)
                .fetch_one(&mut *tx)
                .await?;
            out.push(document_embed_from_pg(&row));
        }
        tx.commit().await?;
        Ok(out)
    }

    async fn list_knowledge_document_broken_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_DOCUMENT_EMBED_COLUMNS} FROM knowledge_document_embeds
             WHERE rich_document_id = $1 AND repair_state = 'broken' ORDER BY block_id"
        );
        let rows = sqlx::query(&sql)
            .bind(rich_document_id)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(document_embed_from_pg).collect())
    }

    async fn upsert_knowledge_document_backlink(
        &self,
        upsert: UpsertKnowledgeDocumentBacklink,
    ) -> StorageResult<KnowledgeDocumentBacklink> {
        let backlink_id = new_knowledge_id("KDBL");
        let sql = format!(
            r#"
            INSERT INTO knowledge_document_backlinks
                (backlink_id, workspace_id, relationship_id, source_document_id,
                 link_kind, target, block_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (workspace_id, relationship_id) DO UPDATE SET
                source_document_id = EXCLUDED.source_document_id,
                link_kind = EXCLUDED.link_kind,
                target = EXCLUDED.target,
                block_id = EXCLUDED.block_id,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&backlink_id)
            .bind(&upsert.workspace_id)
            .bind(&upsert.relationship_id)
            .bind(&upsert.source_document_id)
            .bind(&upsert.link_kind)
            .bind(&upsert.target)
            .bind(&upsert.block_id)
            .fetch_one(self.pool())
            .await?;
        Ok(document_backlink_from_pg(&row))
    }

    async fn replace_knowledge_document_backlinks(
        &self,
        source_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentBacklink>,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        // Rebuild semantics: the document content is the source of truth, so a
        // re-extract is delete-all-for-source + insert in one transaction.
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM knowledge_document_backlinks WHERE source_document_id = $1")
            .bind(source_document_id)
            .execute(&mut *tx)
            .await?;
        let mut out = Vec::with_capacity(upserts.len());
        for upsert in upserts {
            let backlink_id = new_knowledge_id("KDBL");
            let sql = format!(
                r#"
                INSERT INTO knowledge_document_backlinks
                    (backlink_id, workspace_id, relationship_id,
                     source_document_id, link_kind, target, block_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING {KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS}
                "#
            );
            let row = sqlx::query(&sql)
                .bind(&backlink_id)
                .bind(&upsert.workspace_id)
                .bind(&upsert.relationship_id)
                .bind(&upsert.source_document_id)
                .bind(&upsert.link_kind)
                .bind(&upsert.target)
                .bind(&upsert.block_id)
                .fetch_one(&mut *tx)
                .await?;
            out.push(document_backlink_from_pg(&row));
        }
        tx.commit().await?;
        Ok(out)
    }

    async fn list_knowledge_document_backlinks_from(
        &self,
        source_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS} FROM knowledge_document_backlinks
             WHERE source_document_id = $1 ORDER BY link_kind, target, block_id"
        );
        let rows = sqlx::query(&sql)
            .bind(source_document_id)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(document_backlink_from_pg).collect())
    }

    async fn list_knowledge_document_backlinks_to(
        &self,
        workspace_id: &str,
        link_kind: &str,
        target: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_DOCUMENT_BACKLINK_COLUMNS} FROM knowledge_document_backlinks
             WHERE workspace_id = $1 AND link_kind = $2 AND target = $3
             ORDER BY source_document_id, block_id"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(link_kind)
            .bind(target)
            .fetch_all(self.pool())
            .await?;
        Ok(rows.iter().map(document_backlink_from_pg).collect())
    }

    async fn record_knowledge_context_bundle(
        &self,
        new_bundle: NewKnowledgeContextBundle,
    ) -> StorageResult<KnowledgeContextBundle> {
        for item in &new_bundle.items {
            if item.ref_id.trim() != item.ref_id || item.ref_id.is_empty() {
                return Err(StorageError::Validation(
                    "knowledge bundle item ref_id must be non-empty and trimmed",
                ));
            }
        }
        let bundle = &new_bundle.bundle;

        let mut tx = self.pool().begin().await?;
        let sql = format!(
            r#"
            INSERT INTO knowledge_context_bundles
                (bundle_id, workspace_id, kernel_task_run_id, session_run_id,
                 allowed_context, context_hash, query_text, token_budget,
                 tokens_used, build_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING {KNOWLEDGE_BUNDLE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&bundle.context_bundle_id)
            .bind(&new_bundle.workspace_id)
            .bind(&bundle.kernel_task_run_id)
            .bind(&bundle.session_run_id)
            .bind(&bundle.allowed_context)
            .bind(&bundle.context_hash)
            .bind(&new_bundle.query_text)
            .bind(new_bundle.token_budget)
            .bind(new_bundle.tokens_used)
            .bind(&new_bundle.build_receipt_event_id)
            .fetch_one(&mut *tx)
            .await?;
        let stored = bundle_from_pg(&row);

        for (index, item) in new_bundle.items.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO knowledge_context_bundle_items
                    (bundle_id, item_ordinal, ref_kind, ref_id,
                     retrieval_decision, relevance_score, token_count, citation)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(&stored.bundle_id)
            .bind(index as i32)
            .bind(item.ref_kind.as_str())
            .bind(&item.ref_id)
            .bind(item.retrieval_decision.as_str())
            .bind(item.relevance_score)
            .bind(item.token_count)
            .bind(&item.citation)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(stored)
    }

    async fn get_knowledge_context_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Option<(KnowledgeContextBundle, Vec<KnowledgeContextBundleItem>)>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_BUNDLE_COLUMNS} FROM knowledge_context_bundles
             WHERE bundle_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(bundle_id)
            .fetch_optional(self.pool())
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let bundle = bundle_from_pg(&row);

        let item_rows = sqlx::query(
            r#"
            SELECT bundle_id, item_ordinal, ref_kind, ref_id,
                   retrieval_decision, relevance_score, token_count, citation
            FROM knowledge_context_bundle_items
            WHERE bundle_id = $1
            ORDER BY item_ordinal
            "#,
        )
        .bind(bundle_id)
        .fetch_all(self.pool())
        .await?;
        let items = item_rows
            .iter()
            .map(bundle_item_from_pg)
            .collect::<StorageResult<Vec<_>>>()?;
        Ok(Some((bundle, items)))
    }

    async fn record_knowledge_retrieval_trace(
        &self,
        new_trace: NewKnowledgeRetrievalTrace,
    ) -> StorageResult<KnowledgeRetrievalTrace> {
        if new_trace.mode_reason.trim() != new_trace.mode_reason || new_trace.mode_reason.is_empty()
        {
            return Err(StorageError::Validation(
                "knowledge retrieval trace mode_reason is a spec MUST: record why broader retrieval was used or skipped",
            ));
        }
        let trace_id = new_knowledge_id("KRT");
        let sql = format!(
            r#"
            INSERT INTO knowledge_retrieval_traces
                (trace_id, workspace_id, retrieval_mode, mode_reason,
                 query_text, bundle_id, decisions, trace_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING {KNOWLEDGE_TRACE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&trace_id)
            .bind(&new_trace.workspace_id)
            .bind(new_trace.retrieval_mode.as_str())
            .bind(&new_trace.mode_reason)
            .bind(&new_trace.query_text)
            .bind(&new_trace.bundle_id)
            .bind(&new_trace.decisions)
            .bind(&new_trace.trace_receipt_event_id)
            .fetch_one(self.pool())
            .await?;
        trace_from_pg(&row)
    }

    async fn list_knowledge_retrieval_traces_for_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Vec<KnowledgeRetrievalTrace>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_TRACE_COLUMNS} FROM knowledge_retrieval_traces
             WHERE bundle_id = $1 ORDER BY created_at"
        );
        let rows = sqlx::query(&sql)
            .bind(bundle_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(trace_from_pg).collect()
    }

    async fn create_knowledge_memory_passage_idempotent(
        &self,
        idempotency_key: &str,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeMemoryPassage>> {
        validate_knowledge_idempotency_key(idempotency_key)?;
        let request_hash = knowledge_request_hash("passage_write", &new_passage)?;

        let replay = |passage_id: String| async move {
            let passage = self
                .get_knowledge_memory_passage(&passage_id)
                .await?
                .ok_or(StorageError::NotFound(
                    "knowledge idempotency result passage",
                ))?;
            Ok(KnowledgeIdempotentWrite {
                value: passage,
                replayed: true,
            })
        };

        // Committed replay: return the prior result without writing.
        if let Some((_, passage_id)) =
            find_knowledge_idempotency_result(self.pool(), idempotency_key, &request_hash).await?
        {
            return replay(passage_id).await;
        }

        // Write + key claim in ONE transaction.
        let mut tx = self.pool().begin().await?;
        let passage = insert_knowledge_memory_passage_tx(&mut tx, &new_passage).await?;
        let claimed = claim_knowledge_idempotency_key_tx(
            &mut tx,
            idempotency_key,
            &new_passage.workspace_id,
            KnowledgeIdempotentOperationKind::PassageWrite,
            &request_hash,
            "memory_passage",
            &passage.passage_id,
        )
        .await?;
        if claimed {
            tx.commit().await?;
            return Ok(KnowledgeIdempotentWrite {
                value: passage,
                replayed: false,
            });
        }

        // Race lost: roll the whole write back (no double-write) and re-read
        // the winner's committed result.
        drop(tx);
        let (_, passage_id) =
            find_knowledge_idempotency_result(self.pool(), idempotency_key, &request_hash)
                .await?
                .ok_or(StorageError::Conflict(
                    "knowledge idempotency race lost without a committed winner row",
                ))?;
        replay(passage_id).await
    }

    async fn save_knowledge_rich_document_version_idempotent(
        &self,
        idempotency_key: &str,
        rich_document_id: &str,
        expected_version: i64,
        content_json: Value,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeRichDocument>> {
        validate_knowledge_idempotency_key(idempotency_key)?;
        let request_hash = knowledge_request_hash(
            "rich_document_save",
            &serde_json::json!({
                "rich_document_id": rich_document_id,
                "expected_version": expected_version,
                "content_json": content_json,
                "promotion_receipt_event_id": promotion_receipt_event_id,
            }),
        )?;

        let replay = |document_id: String| async move {
            let document = self
                .get_knowledge_rich_document(&document_id)
                .await?
                .ok_or(StorageError::NotFound(
                    "knowledge idempotency result rich document",
                ))?;
            Ok(KnowledgeIdempotentWrite {
                value: document,
                replayed: true,
            })
        };

        // Committed replay: return the prior result without writing.
        if let Some((_, document_id)) =
            find_knowledge_idempotency_result(self.pool(), idempotency_key, &request_hash).await?
        {
            return replay(document_id).await;
        }

        // The optimistic save + key claim in ONE transaction.
        let save_result = self
            .save_knowledge_rich_document_version_with_key(
                idempotency_key,
                rich_document_id,
                expected_version,
                &content_json,
                promotion_receipt_event_id,
                &request_hash,
            )
            .await;
        match save_result {
            Ok(Some(document)) => Ok(KnowledgeIdempotentWrite {
                value: document,
                replayed: false,
            }),
            // Race lost on the key claim: the write rolled back; re-read the
            // winner's committed result.
            Ok(None) => {
                let (_, document_id) =
                    find_knowledge_idempotency_result(self.pool(), idempotency_key, &request_hash)
                        .await?
                        .ok_or(StorageError::Conflict(
                            "knowledge idempotency race lost without a committed winner row",
                        ))?;
                replay(document_id).await
            }
            // A replayed save typically loses the optimistic version race
            // first (the winner already bumped doc_version). If the same
            // key+payload committed, that conflict IS the replay signal.
            Err(StorageError::Conflict(message)) => {
                if let Some((_, document_id)) =
                    find_knowledge_idempotency_result(self.pool(), idempotency_key, &request_hash)
                        .await?
                {
                    return replay(document_id).await;
                }
                Err(StorageError::Conflict(message))
            }
            Err(err) => Err(err),
        }
    }
}

// ===========================================================================
// WP-KERNEL-009 CodeIndexingAndNavigation (MT-097..MT-112) SHARED-FILE
// ADDITION.
//
// Added by the CodeIndexingAndNavigation group: row types + a dedicated
// `impl PostgresDatabase` block for the two code-index SUPPORT tables
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

fn code_file_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeCodeFile> {
    Ok(KnowledgeCodeFile {
        code_file_id: row.get("code_file_id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        file_entity_id: row.get("file_entity_id"),
        language: row.get::<String, _>("language").parse()?,
        indexed_content_hash: row.get("indexed_content_hash"),
        parser_version: row.get("parser_version"),
        parse_status: row.get::<String, _>("parse_status").parse()?,
        stale: row.get("stale"),
        symbols_indexed: row.get("symbols_indexed"),
        edges_indexed: row.get("edges_indexed"),
        failure_detail: row.get("failure_detail"),
        last_indexed_in_run: row.get("last_indexed_in_run"),
        last_index_receipt_event_id: row.get("last_index_receipt_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

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

fn scip_import_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeScipImport> {
    Ok(KnowledgeScipImport {
        scip_import_id: row.get("scip_import_id"),
        workspace_id: row.get("workspace_id"),
        artifact_format: match row.get::<String, _>("artifact_format").as_str() {
            "scip" => KnowledgeScipFormat::Scip,
            "lsif" => KnowledgeScipFormat::Lsif,
            _ => return Err(StorageError::Validation("invalid scip artifact_format")),
        },
        tool_name: row.get("tool_name"),
        tool_version: row.get("tool_version"),
        artifact_hash: row.get("artifact_hash"),
        status: row.get::<String, _>("status").parse()?,
        reason: row.get("reason"),
        symbols_imported: row.get("symbols_imported"),
        occurrences_imported: row.get("occurrences_imported"),
        edges_imported: row.get("edges_imported"),
        import_detail: row.get("import_detail"),
        imported_in_run: row.get("imported_in_run"),
        import_receipt_event_id: row.get("import_receipt_event_id"),
        created_at: row.get("created_at"),
    })
}

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

fn code_repair_from_pg(row: &sqlx::postgres::PgRow) -> StorageResult<KnowledgeCodeRepairEntry> {
    Ok(KnowledgeCodeRepairEntry {
        code_repair_id: row.get("code_repair_id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        relative_path: row.get("relative_path"),
        reason_class: row.get::<String, _>("reason_class").parse()?,
        reason_detail: row.get("reason_detail"),
        state: row.get("state"),
        attempts: row.get("attempts"),
        max_attempts: row.get("max_attempts"),
        last_attempt_at: row.get("last_attempt_at"),
        enqueue_event_id: row.get("enqueue_event_id"),
        resolved_receipt_event_id: row.get("resolved_receipt_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

impl PostgresDatabase {
    /// Upsert the per-code-file index state on `(source_id)`. A re-index with a
    /// new content hash / parser version replaces the indexed_* fields and
    /// CLEARS the stale marker (the file is now freshly indexed).
    pub async fn upsert_knowledge_code_file(
        &self,
        upsert: UpsertKnowledgeCodeFile,
    ) -> StorageResult<KnowledgeCodeFile> {
        if !is_sha256_hex(&upsert.indexed_content_hash) {
            return Err(StorageError::Validation(
                "knowledge code file indexed_content_hash must be a lowercase sha256 hex digest",
            ));
        }
        if upsert.parser_version.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge code file parser_version is required",
            ));
        }
        let code_file_id = new_knowledge_id("KCF");
        let sql = format!(
            r#"
            INSERT INTO knowledge_code_files
                (code_file_id, workspace_id, source_id, file_entity_id, language,
                 indexed_content_hash, parser_version, parse_status, stale,
                 symbols_indexed, edges_indexed, failure_detail,
                 last_indexed_in_run, last_index_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, $9, $10, $11, $12, $13)
            ON CONFLICT (source_id) DO UPDATE SET
                file_entity_id = COALESCE(EXCLUDED.file_entity_id,
                                          knowledge_code_files.file_entity_id),
                language = EXCLUDED.language,
                indexed_content_hash = EXCLUDED.indexed_content_hash,
                parser_version = EXCLUDED.parser_version,
                parse_status = EXCLUDED.parse_status,
                stale = FALSE,
                symbols_indexed = EXCLUDED.symbols_indexed,
                edges_indexed = EXCLUDED.edges_indexed,
                failure_detail = EXCLUDED.failure_detail,
                last_indexed_in_run = COALESCE(EXCLUDED.last_indexed_in_run,
                                               knowledge_code_files.last_indexed_in_run),
                last_index_receipt_event_id = COALESCE(
                    EXCLUDED.last_index_receipt_event_id,
                    knowledge_code_files.last_index_receipt_event_id),
                updated_at = NOW()
            RETURNING {KNOWLEDGE_CODE_FILE_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&code_file_id)
            .bind(&upsert.workspace_id)
            .bind(&upsert.source_id)
            .bind(&upsert.file_entity_id)
            .bind(upsert.language.as_str())
            .bind(&upsert.indexed_content_hash)
            .bind(&upsert.parser_version)
            .bind(upsert.parse_status.as_str())
            .bind(upsert.symbols_indexed)
            .bind(upsert.edges_indexed)
            .bind(&upsert.failure_detail)
            .bind(&upsert.last_indexed_in_run)
            .bind(&upsert.last_index_receipt_event_id)
            .fetch_one(self.pool())
            .await?;
        code_file_from_pg(&row)
    }

    pub async fn get_knowledge_code_file_by_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeCodeFile>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_CODE_FILE_COLUMNS} FROM knowledge_code_files WHERE source_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(code_file_from_pg).transpose()
    }

    pub async fn list_knowledge_code_files(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeCodeFile>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_CODE_FILE_COLUMNS} FROM knowledge_code_files
             WHERE workspace_id = $1 ORDER BY source_id"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(code_file_from_pg).collect()
    }

    /// Mark a code file stale (MT-107). Idempotent.
    pub async fn mark_knowledge_code_file_stale(
        &self,
        code_file_id: &str,
    ) -> StorageResult<KnowledgeCodeFile> {
        let sql = format!(
            "UPDATE knowledge_code_files SET stale = TRUE, updated_at = NOW()
             WHERE code_file_id = $1 RETURNING {KNOWLEDGE_CODE_FILE_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(code_file_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("knowledge code file"))?;
        code_file_from_pg(&row)
    }

    /// Record a SCIP/LSIF import attempt (MT-105). Never executes the artifact.
    pub async fn record_knowledge_scip_import(
        &self,
        new_import: NewKnowledgeScipImport,
    ) -> StorageResult<KnowledgeScipImport> {
        if !is_sha256_hex(&new_import.artifact_hash) {
            return Err(StorageError::Validation(
                "scip import artifact_hash must be a lowercase sha256 hex digest",
            ));
        }
        if new_import.status != KnowledgeScipImportStatus::Imported
            && new_import.reason.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(StorageError::Validation(
                "non-imported scip import must carry a reason",
            ));
        }
        let scip_import_id = new_knowledge_id("KSCIP");
        let sql = format!(
            r#"
            INSERT INTO knowledge_code_scip_imports
                (scip_import_id, workspace_id, artifact_format, tool_name,
                 tool_version, artifact_hash, status, reason, symbols_imported,
                 occurrences_imported, edges_imported, import_detail,
                 imported_in_run, import_receipt_event_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING {KNOWLEDGE_SCIP_IMPORT_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&scip_import_id)
            .bind(&new_import.workspace_id)
            .bind(new_import.artifact_format.as_str())
            .bind(&new_import.tool_name)
            .bind(&new_import.tool_version)
            .bind(&new_import.artifact_hash)
            .bind(new_import.status.as_str())
            .bind(&new_import.reason)
            .bind(new_import.symbols_imported)
            .bind(new_import.occurrences_imported)
            .bind(new_import.edges_imported)
            .bind(&new_import.import_detail)
            .bind(&new_import.imported_in_run)
            .bind(&new_import.import_receipt_event_id)
            .fetch_one(self.pool())
            .await?;
        scip_import_from_pg(&row)
    }

    pub async fn list_knowledge_scip_imports(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeScipImport>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_SCIP_IMPORT_COLUMNS} FROM knowledge_code_scip_imports
             WHERE workspace_id = $1 ORDER BY created_at DESC"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(scip_import_from_pg).collect()
    }

    /// MT-108: enqueue (or refresh) a code-index repair entry for a file whose
    /// parse/read failed. Mirrors the ingestion queue's enqueue semantics so a
    /// re-failing file updates its open entry instead of multiplying rows, and a
    /// previously dead-lettered file is reopened with a fresh retry budget. This
    /// is the durable recovery surface MT-108 requires (not just a status flag):
    /// the entry stays visible until the cause is fixed and the file re-indexes.
    pub async fn enqueue_knowledge_code_repair(
        &self,
        entry: NewKnowledgeCodeRepairEntry,
    ) -> StorageResult<KnowledgeCodeRepairEntry> {
        // (1) Refresh an existing OPEN entry first (one open entry per source by
        // the partial unique index).
        let refresh_sql = format!(
            "UPDATE knowledge_code_repair_queue
             SET reason_class = $2, reason_detail = $3, relative_path = $4,
                 enqueue_event_id = COALESCE($5, enqueue_event_id),
                 updated_at = NOW()
             WHERE source_id = $1 AND state IN ('queued', 'retrying')
             RETURNING {KNOWLEDGE_CODE_REPAIR_COLUMNS}"
        );
        if let Some(row) = sqlx::query(&refresh_sql)
            .bind(&entry.source_id)
            .bind(entry.reason_class.as_str())
            .bind(&entry.reason_detail)
            .bind(&entry.relative_path)
            .bind(&entry.enqueue_event_id)
            .fetch_optional(self.pool())
            .await?
        {
            return code_repair_from_pg(&row);
        }

        // (2) Reopen the most recent dead-letter entry for the same source
        // instead of inserting a new row (resets the retry budget). Step (1)
        // proved no open entry exists, so the partial unique index is safe.
        let reopen_sql = format!(
            "UPDATE knowledge_code_repair_queue
             SET state = 'queued', attempts = 0, reason_class = $2,
                 reason_detail = $3, relative_path = $4,
                 enqueue_event_id = COALESCE($5, enqueue_event_id),
                 resolved_receipt_event_id = NULL, updated_at = NOW()
             WHERE source_id = $1 AND state = 'dead_letter'
               AND code_repair_id = (
                   SELECT code_repair_id FROM knowledge_code_repair_queue
                   WHERE source_id = $1 AND state = 'dead_letter'
                   ORDER BY updated_at DESC, code_repair_id DESC
                   LIMIT 1
               )
             RETURNING {KNOWLEDGE_CODE_REPAIR_COLUMNS}"
        );
        if let Some(row) = sqlx::query(&reopen_sql)
            .bind(&entry.source_id)
            .bind(entry.reason_class.as_str())
            .bind(&entry.reason_detail)
            .bind(&entry.relative_path)
            .bind(&entry.enqueue_event_id)
            .fetch_optional(self.pool())
            .await?
        {
            return code_repair_from_pg(&row);
        }

        // (3) Fresh entry: first failure for this source.
        let code_repair_id = new_knowledge_id("KCRQ");
        let insert_sql = format!(
            "INSERT INTO knowledge_code_repair_queue
                 (code_repair_id, workspace_id, source_id, relative_path,
                  reason_class, reason_detail, enqueue_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING {KNOWLEDGE_CODE_REPAIR_COLUMNS}"
        );
        let row = sqlx::query(&insert_sql)
            .bind(&code_repair_id)
            .bind(&entry.workspace_id)
            .bind(&entry.source_id)
            .bind(&entry.relative_path)
            .bind(entry.reason_class.as_str())
            .bind(&entry.reason_detail)
            .bind(&entry.enqueue_event_id)
            .fetch_one(self.pool())
            .await?;
        code_repair_from_pg(&row)
    }

    /// MT-108: the open (queued/retrying) code-index repair entry for a source,
    /// if any. Lets the engine and a no-context model confirm a failed file is
    /// actually held for repair.
    pub async fn get_open_knowledge_code_repair(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeCodeRepairEntry>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_CODE_REPAIR_COLUMNS} FROM knowledge_code_repair_queue
             WHERE source_id = $1 AND state IN ('queued', 'retrying')
             ORDER BY updated_at DESC LIMIT 1"
        );
        let row = sqlx::query(&sql)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await?;
        row.as_ref().map(code_repair_from_pg).transpose()
    }

    /// MT-108: list code-index repair entries for a workspace (operator-visible
    /// queue), newest first.
    pub async fn list_knowledge_code_repairs(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeCodeRepairEntry>> {
        let sql = format!(
            "SELECT {KNOWLEDGE_CODE_REPAIR_COLUMNS} FROM knowledge_code_repair_queue
             WHERE workspace_id = $1 ORDER BY created_at DESC"
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(code_repair_from_pg).collect()
    }

    /// MT-108: resolve a code-index repair entry after a successful re-index.
    /// The resolving receipt is required by the table's CHECK; terminal states
    /// never transition again.
    pub async fn resolve_knowledge_code_repair(
        &self,
        code_repair_id: &str,
        resolved_receipt_event_id: &str,
    ) -> StorageResult<KnowledgeCodeRepairEntry> {
        let sql = format!(
            "UPDATE knowledge_code_repair_queue
             SET state = 'resolved', resolved_receipt_event_id = $2, updated_at = NOW()
             WHERE code_repair_id = $1 AND state IN ('queued', 'retrying')
             RETURNING {KNOWLEDGE_CODE_REPAIR_COLUMNS}"
        );
        let row = sqlx::query(&sql)
            .bind(code_repair_id)
            .bind(resolved_receipt_event_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(StorageError::NotFound("open knowledge code repair entry"))?;
        code_repair_from_pg(&row)
    }

    /// MT-106: DB-side symbol lookup for the nav API. The code-symbol entity_key
    /// is `{lang}:{relative_path}#{symbol_path}`, so the high-selectivity path
    /// and name filters push down to SQL instead of loading every symbol in the
    /// workspace and filtering in Rust (the adversarial-review DoS: a 100k-symbol
    /// repo transferred + heap-allocated on every lookup). A `path` filter
    /// matches the `:{path}#` segment; a `name` filter matches the trailing
    /// symbol-path segment (`%name`) OR display_name; results are bounded by
    /// `limit`. The trailing-segment match is refined caller-side for exactness,
    /// but the SQL has already cut the candidate set to the matching path/name.
    pub async fn lookup_code_symbols(
        &self,
        workspace_id: &str,
        name: Option<&str>,
        path: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<KnowledgeEntity>> {
        let mut sql = format!(
            "SELECT {KNOWLEDGE_ENTITY_COLUMNS} FROM knowledge_entities
             WHERE workspace_id = $1 AND entity_kind = 'symbol'"
        );
        // $2 path-segment LIKE, $3 name-suffix LIKE, $4 name equality on
        // display_name, $5 limit. Unused params are bound as NULL and guarded by
        // the `IS NULL OR` shape so the planner can still use the entity_key
        // index for the supplied filter(s).
        sql.push_str(
            " AND ($2::text IS NULL OR entity_key LIKE $2)
              AND ($3::text IS NULL OR entity_key LIKE $3 OR display_name = $4)
             ORDER BY entity_key
             LIMIT $5",
        );
        // `path` -> match the `:{path}#` segment anywhere in the key (escape the
        // LIKE metacharacters in the operator-supplied path).
        let path_like = path.map(|p| format!("%:{}#%", escape_like(p)));
        // `name` -> match a key ending in the simple name after `.`/`::`.
        let name_like = name.map(|n| format!("%{}", escape_like(n)));
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(path_like.as_deref())
            .bind(name_like.as_deref())
            .bind(name)
            .bind(limit.clamp(1, 10_000))
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(entity_from_pg).collect()
    }
}

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
// Inherent PostgresDatabase methods (the established code-files pattern: a
// separate auditable impl, NOT new KnowledgeStore trait surface). Everything
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

impl PostgresDatabase {
    /// Upsert a compiled wiki page on its `(workspace, kind='wiki_page',
    /// title)` identity. The row lands `fresh` with its stamp, recipe, links
    /// and receipt in ONE statement (no stampless intermediate state exists).
    pub async fn upsert_knowledge_wiki_page(
        &self,
        page: NewKnowledgeWikiPage,
    ) -> StorageResult<KnowledgeWikiProjection> {
        if page.title.trim() != page.title || page.title.is_empty() {
            return Err(StorageError::Validation(
                "wiki page title must be non-empty and trimmed",
            ));
        }
        if !is_sha256_hex(&page.staleness_hash) {
            return Err(StorageError::Validation(
                "wiki page staleness_hash must be lowercase sha256 hex",
            ));
        }
        if let Some(page_type) = page.page_type.as_deref() {
            if !KNOWLEDGE_WIKI_PAGE_TYPES.contains(&page_type) {
                return Err(StorageError::Validation("invalid wiki page_type"));
            }
        }
        // Fail-closed stamp shape check (mirror of the 0300 CHECK, so the
        // caller gets a typed Validation error instead of a constraint blow).
        let stamp_ok = page
            .compile_stamp
            .get("ledger_version")
            .map(|v| v.is_i64() || v.is_u64())
            .unwrap_or(false)
            && page
                .compile_stamp
                .get("cited_sources")
                .map(|v| v.is_array())
                .unwrap_or(false);
        if !stamp_ok {
            return Err(StorageError::Validation(
                "wiki page compile_stamp must carry ledger_version + cited_sources (LM-PWIKI-006)",
            ));
        }
        let projection_id = new_knowledge_id("KWP");
        let sql = format!(
            r#"
            INSERT INTO knowledge_wiki_projections
                (projection_id, workspace_id, projection_kind, title,
                 source_records, rendered_content, rebuild_status,
                 staleness_hash, rebuild_receipt_event_id, last_rebuilt_at,
                 page_type, compile_stamp, compile_recipe, page_links)
            VALUES ($1, $2, 'wiki_page', $3, $4, $5, 'fresh', $6, $7, NOW(),
                    $8, $9, $10, $11)
            ON CONFLICT (workspace_id, projection_kind, title) DO UPDATE SET
                source_records = EXCLUDED.source_records,
                rendered_content = EXCLUDED.rendered_content,
                rebuild_status = 'fresh',
                staleness_hash = EXCLUDED.staleness_hash,
                rebuild_receipt_event_id = EXCLUDED.rebuild_receipt_event_id,
                last_rebuilt_at = NOW(),
                page_type = EXCLUDED.page_type,
                compile_stamp = EXCLUDED.compile_stamp,
                compile_recipe = EXCLUDED.compile_recipe,
                page_links = EXCLUDED.page_links,
                updated_at = NOW()
            RETURNING {KNOWLEDGE_PROJECTION_COLUMNS}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(&projection_id)
            .bind(&page.workspace_id)
            .bind(&page.title)
            .bind(&page.source_records)
            .bind(&page.rendered_content)
            .bind(&page.staleness_hash)
            .bind(page.rebuild_receipt_event_id.as_deref())
            .bind(page.page_type.as_deref())
            .bind(&page.compile_stamp)
            .bind(page.compile_recipe.as_ref())
            .bind(&page.page_links)
            .fetch_one(self.pool())
            .await?;
        projection_from_pg(&row)
    }

    /// List a workspace's wiki pages (optionally one `page_type`), newest
    /// titles in deterministic order. `typed_only` filters to project-wiki
    /// pages (page_type IS NOT NULL).
    pub async fn list_knowledge_wiki_pages(
        &self,
        workspace_id: &str,
        page_type: Option<&str>,
        typed_only: bool,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        let sql = format!(
            r#"
            SELECT {KNOWLEDGE_PROJECTION_COLUMNS}
            FROM knowledge_wiki_projections
            WHERE workspace_id = $1
              AND projection_kind = 'wiki_page'
              AND ($2::text IS NULL OR page_type = $2)
              AND (NOT $3::bool OR page_type IS NOT NULL)
            ORDER BY COALESCE(page_type, 'zz') ASC, title ASC
            LIMIT $4 OFFSET $5
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(page_type)
            .bind(typed_only)
            .bind(limit.clamp(1, 2_000))
            .bind(offset.max(0))
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(projection_from_pg).collect()
    }

    /// MT-243 fan-out probe: pages whose compile stamp CITES the given
    /// authority record (`@>` containment over the GIN-indexed cited_sources).
    pub async fn list_knowledge_wiki_pages_citing(
        &self,
        workspace_id: &str,
        cited_kind: &str,
        cited_id: &str,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        let probe = serde_json::json!([{"kind": cited_kind, "id": cited_id}]);
        let sql = format!(
            r#"
            SELECT {KNOWLEDGE_PROJECTION_COLUMNS}
            FROM knowledge_wiki_projections
            WHERE workspace_id = $1
              AND projection_kind = 'wiki_page'
              AND compile_stamp IS NOT NULL
              AND (compile_stamp -> 'cited_sources') @> $2
            ORDER BY title ASC
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(&probe)
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(projection_from_pg).collect()
    }

    /// Replace a page's outbound wikilink set (same-pass backlink refresh,
    /// LM-PWIKI-010). Wholesale replacement keeps re-runs idempotent (no
    /// duplicate links can accumulate).
    pub async fn update_knowledge_wiki_page_links(
        &self,
        projection_id: &str,
        page_links: &Value,
    ) -> StorageResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE knowledge_wiki_projections
            SET page_links = $2, updated_at = NOW()
            WHERE projection_id = $1
            "#,
        )
        .bind(projection_id)
        .bind(page_links)
        .execute(self.pool())
        .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("knowledge wiki projection"));
        }
        Ok(())
    }

    /// The EventLedger source version: `MAX(event_sequence)` (0 on an empty
    /// ledger). Monotonic compile watermark for stamps (LM-PWIKI-006).
    pub async fn current_event_ledger_version(&self) -> StorageResult<i64> {
        let version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(event_sequence), 0) FROM kernel_event_ledger",
        )
        .fetch_one(self.pool())
        .await?;
        Ok(version)
    }

    /// Bootstrap-compiler input: every indexed code file of the workspace
    /// joined to its source identity (path + current content hash).
    pub async fn list_wiki_code_file_inputs(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<WikiCodeFileInput>> {
        let rows = sqlx::query(
            r#"
            SELECT cf.code_file_id, cf.source_id, cf.language, cf.parse_status,
                   cf.stale, cf.symbols_indexed,
                   s.relative_path, s.content_hash
            FROM knowledge_code_files cf
            JOIN knowledge_sources s ON s.source_id = cf.source_id
            WHERE cf.workspace_id = $1 AND s.relative_path IS NOT NULL
            ORDER BY s.relative_path ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(WikiCodeFileInput {
                    code_file_id: row.get("code_file_id"),
                    source_id: row.get("source_id"),
                    relative_path: row.get("relative_path"),
                    content_hash: row.get("content_hash"),
                    language: row.get::<String, _>("language").parse()?,
                    parse_status: row.get::<String, _>("parse_status").parse()?,
                    stale: row.get("stale"),
                    symbols_indexed: row.get("symbols_indexed"),
                })
            })
            .collect()
    }

    /// MT-243 regeneration input: like [`Self::list_wiki_code_file_inputs`]
    /// but scoped to an explicit source-id set (the page recipe's cited
    /// sources). Sources that vanished from authority simply do not return —
    /// the regenerated page drops them.
    pub async fn list_wiki_code_file_inputs_by_sources(
        &self,
        workspace_id: &str,
        source_ids: &[String],
    ) -> StorageResult<Vec<WikiCodeFileInput>> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            r#"
            SELECT cf.code_file_id, cf.source_id, cf.language, cf.parse_status,
                   cf.stale, cf.symbols_indexed,
                   s.relative_path, s.content_hash
            FROM knowledge_code_files cf
            JOIN knowledge_sources s ON s.source_id = cf.source_id
            WHERE cf.workspace_id = $1
              AND cf.source_id = ANY($2)
              AND s.relative_path IS NOT NULL
            ORDER BY s.relative_path ASC
            "#,
        )
        .bind(workspace_id)
        .bind(source_ids)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(WikiCodeFileInput {
                    code_file_id: row.get("code_file_id"),
                    source_id: row.get("source_id"),
                    relative_path: row.get("relative_path"),
                    content_hash: row.get("content_hash"),
                    language: row.get::<String, _>("language").parse()?,
                    parse_status: row.get::<String, _>("parse_status").parse()?,
                    stale: row.get("stale"),
                    symbols_indexed: row.get("symbols_indexed"),
                })
            })
            .collect()
    }

    /// Active entities of one kind anchored to a source, each with its LATEST
    /// evidence span on that source (definition span for symbols, passage span
    /// for concepts) — the precise citation row (entity id + span id + span
    /// content hash, LM-PWIKI-003).
    pub async fn list_wiki_source_entities_with_spans(
        &self,
        workspace_id: &str,
        source_id: &str,
        entity_kind: KnowledgeEntityKind,
    ) -> StorageResult<Vec<WikiEntityWithSpan>> {
        let sql = format!(
            r#"
            SELECT DISTINCT ON (e.entity_id)
                {KNOWLEDGE_ENTITY_COLUMNS_E},
                s.span_id AS wiki_span_id,
                s.content_sha256 AS wiki_span_content_sha256,
                s.line_start AS wiki_line_start,
                s.line_end AS wiki_line_end,
                s.section_path AS wiki_section_path
            FROM knowledge_entities e
            JOIN knowledge_entity_spans es ON es.entity_id = e.entity_id
            JOIN knowledge_spans s
              ON s.span_id = es.span_id AND s.source_id = $2
            WHERE e.workspace_id = $1
              AND e.primary_source_id = $2
              AND e.entity_kind = $3
              AND e.lifecycle_state = 'active'
            ORDER BY e.entity_id, s.created_at DESC, s.span_id DESC
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(source_id)
            .bind(entity_kind.as_str())
            .fetch_all(self.pool())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            out.push(WikiEntityWithSpan {
                entity: entity_from_pg(row)?,
                span_id: row.get("wiki_span_id"),
                span_content_sha256: row.get("wiki_span_content_sha256"),
                line_start: row.get("wiki_line_start"),
                line_end: row.get("wiki_line_end"),
                section_path: row.get("wiki_section_path"),
            });
        }
        // DISTINCT ON ordered by entity_id; re-sort by stable entity_key for
        // deterministic page rendering.
        out.sort_by(|a, b| a.entity.entity_key.cmp(&b.entity.entity_key));
        Ok(out)
    }

    /// Cross-source code-graph edges (references / depends_on / implements)
    /// for cross-module wikilinks. Bounded read (DoS guard).
    pub async fn list_wiki_cross_source_code_edges(
        &self,
        workspace_id: &str,
        limit: i64,
    ) -> StorageResult<Vec<WikiCrossSourceEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT ed.edge_type,
                   se.primary_source_id AS from_source_id,
                   te.primary_source_id AS to_source_id
            FROM knowledge_edges ed
            JOIN knowledge_entities se ON se.entity_id = ed.source_entity_id
            JOIN knowledge_entities te ON te.entity_id = ed.target_entity_id
            WHERE ed.workspace_id = $1
              AND ed.edge_type IN ('references', 'depends_on', 'implements')
              AND ed.lifecycle_state = 'active'
              AND se.primary_source_id IS NOT NULL
              AND te.primary_source_id IS NOT NULL
              AND se.primary_source_id <> te.primary_source_id
            ORDER BY se.primary_source_id, te.primary_source_id
            LIMIT $2
            "#,
        )
        .bind(workspace_id)
        .bind(limit.clamp(1, 100_000))
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(WikiCrossSourceEdge {
                    edge_type: row.get::<String, _>("edge_type").parse()?,
                    from_source_id: row.get("from_source_id"),
                    to_source_id: row.get("to_source_id"),
                })
            })
            .collect()
    }

    /// Drift resolver: current content hashes of `knowledge_sources` rows.
    pub async fn get_wiki_source_hashes(
        &self,
        source_ids: &[String],
    ) -> StorageResult<std::collections::HashMap<String, String>> {
        if source_ids.is_empty() {
            return Ok(Default::default());
        }
        let rows = sqlx::query(
            "SELECT source_id, content_hash FROM knowledge_sources
             WHERE source_id = ANY($1)",
        )
        .bind(source_ids)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("source_id"), r.get("content_hash")))
            .collect())
    }

    /// Drift resolver: entities by id with their owning source's CURRENT
    /// content hash (input to `knowledge_wiki::entity_content_hash`).
    pub async fn get_wiki_entity_states(
        &self,
        entity_ids: &[String],
    ) -> StorageResult<Vec<(KnowledgeEntity, Option<String>)>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            r#"
            SELECT {KNOWLEDGE_ENTITY_COLUMNS_E}, s.content_hash AS wiki_source_content_hash
            FROM knowledge_entities e
            LEFT JOIN knowledge_sources s ON s.source_id = e.primary_source_id
            WHERE e.entity_id = ANY($1)
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(entity_ids)
            .fetch_all(self.pool())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            out.push((entity_from_pg(row)?, row.get("wiki_source_content_hash")));
        }
        Ok(out)
    }

    /// Drift resolver: current loom-block content states.
    pub async fn get_wiki_loom_block_states(
        &self,
        workspace_id: &str,
        block_ids: &[String],
    ) -> StorageResult<Vec<WikiLoomBlockState>> {
        if block_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            r#"
            SELECT block_id, title, content_type, document_id, asset_id,
                   content_hash, derived_json
            FROM loom_blocks
            WHERE workspace_id = $1 AND block_id = ANY($2)
            "#,
        )
        .bind(workspace_id)
        .bind(block_ids)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                // `derived_json` is a TEXT column; lift full_text_index the
                // same way `map_loom_block` does (serde, default on parse
                // failure — never a hard error on a hostile blob).
                let derived_raw: String = row.get("derived_json");
                let full_text_index = serde_json::from_str::<Value>(&derived_raw)
                    .ok()
                    .and_then(|v| {
                        v.get("full_text_index")
                            .and_then(|t| t.as_str().map(|s| s.to_string()))
                    });
                WikiLoomBlockState {
                    block_id: row.get("block_id"),
                    title: row.get("title"),
                    content_type: row.get("content_type"),
                    full_text_index,
                    document_id: row.get("document_id"),
                    asset_id: row.get("asset_id"),
                    content_hash: row.get("content_hash"),
                }
            })
            .collect())
    }

    /// Drift resolver: current rich-document content hashes.
    pub async fn get_wiki_rich_document_hashes(
        &self,
        rich_document_ids: &[String],
    ) -> StorageResult<std::collections::HashMap<String, String>> {
        if rich_document_ids.is_empty() {
            return Ok(Default::default());
        }
        let rows = sqlx::query(
            "SELECT rich_document_id, content_sha256 FROM knowledge_rich_documents
             WHERE rich_document_id = ANY($1)",
        )
        .bind(rich_document_ids)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("rich_document_id"), r.get("content_sha256")))
            .collect())
    }
}
