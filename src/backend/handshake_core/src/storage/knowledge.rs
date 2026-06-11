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
use sqlx::Row;
use std::str::FromStr;

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
}
