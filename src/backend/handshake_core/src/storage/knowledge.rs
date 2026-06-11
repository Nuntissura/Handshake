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
}
