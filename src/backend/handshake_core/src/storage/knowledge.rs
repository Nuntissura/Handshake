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
}
