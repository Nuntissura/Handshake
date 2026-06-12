//! WP-KERNEL-009 MT-138 RetrievalTraceModel — the storage-facing persistence of
//! a [`QueryPlan`] + [`RetrievalTrace`] into the committed
//! `knowledge_retrieval_traces` table (migration 0141, MT-060).
//!
//! This file is the RetrievalContextAndRanking group's OWN storage surface. It
//! does NOT edit `storage/knowledge.rs` or `storage/knowledge_memory.rs`; it
//! reuses their public `KnowledgeStore` API. Specifically it wraps
//! [`KnowledgeStore::record_knowledge_retrieval_trace`] so the planner/compiler
//! product logic can persist a fully replayable trace without re-deriving the
//! `mode_reason` / `decisions` projection at every call site.
//!
//! Authority (spec 2.3.13.11): the `knowledge_retrieval_traces` row is the
//! durable authority. `retrieval_mode` is one of the five spec strings,
//! `mode_reason` records why broader retrieval was used or skipped (a DB CHECK
//! rejects empty), and `decisions` carries the full replayable QueryPlan +
//! RetrievalTrace JSON ([`RetrievalTrace::to_decisions_json`]). EventLedger
//! linkage is via `trace_receipt_event_id`.

use crate::knowledge_retrieval::plan::{QueryPlan, RetrievalTrace};
use crate::storage::knowledge::{
    KnowledgeRetrievalMode, KnowledgeRetrievalTrace, KnowledgeStore, NewKnowledgeRetrievalTrace,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{StorageError, StorageResult};

/// Map the planner's [`crate::memory::retrieval_mode::QueryRetrievalMode`] onto
/// the durable storage enum (the five spec strings). Planner postures
/// (`PassageFallback`, `Blocked`) collapse onto `hybrid_rag` / `none`
/// respectively via the mode's `to_storage_str`, parsed back into the storage
/// enum so the column constraint is always satisfied.
pub fn storage_mode_of(plan: &QueryPlan) -> StorageResult<KnowledgeRetrievalMode> {
    plan.retrieval_mode
        .to_storage_str()
        .parse::<KnowledgeRetrievalMode>()
}

/// Persist a QueryPlan + RetrievalTrace as a durable, replayable
/// `knowledge_retrieval_traces` row. The `mode_reason` and `decisions` are
/// derived from the trace so a caller cannot accidentally persist an empty
/// reason (which the DB CHECK would reject anyway).
///
/// * `bundle_id` — the context bundle this trace produced, when one was built
///   (the FK is `ON DELETE SET NULL`).
/// * `trace_receipt_event_id` — the EventLedger receipt for this retrieval.
pub async fn record_retrieval_trace(
    db: &PostgresDatabase,
    workspace_id: &str,
    plan: &QueryPlan,
    trace: &RetrievalTrace,
    bundle_id: Option<String>,
    trace_receipt_event_id: Option<String>,
) -> StorageResult<KnowledgeRetrievalTrace> {
    // Defensive: a non-hybrid mode without a reason is a planner bug; refuse to
    // persist a misleading trace (mirrors the spec A0.5 invariant).
    plan.validate().map_err(StorageError::Validation)?;

    let new = NewKnowledgeRetrievalTrace {
        workspace_id: workspace_id.to_string(),
        retrieval_mode: storage_mode_of(plan)?,
        mode_reason: trace.mode_reason(),
        query_text: Some(plan.query_text.clone()),
        bundle_id,
        decisions: trace.to_decisions_json(plan),
        trace_receipt_event_id,
    };
    db.record_knowledge_retrieval_trace(new).await
}

/// Load every trace bound to a bundle (replay entry point for the debug API).
/// Thin pass-through to the committed store so the retrieval group has one
/// import surface for its reads.
pub async fn traces_for_bundle(
    db: &PostgresDatabase,
    bundle_id: &str,
) -> StorageResult<Vec<KnowledgeRetrievalTrace>> {
    db.list_knowledge_retrieval_traces_for_bundle(bundle_id)
        .await
}

// ===========================================================================
// MT-140 SemanticCatalogBridge storage: the backend-queryable routing-contract
// catalog (table 0260). A catalog entry is authority; the planner resolves a
// query to a route through these rows, not prompt-only helper text.
// ===========================================================================

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// The kind of catalog entry (spec SemanticCatalogEntry.kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticCatalogKind {
    EntityType,
    Index,
    View,
    Tool,
}

impl SemanticCatalogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EntityType => "entity_type",
            Self::Index => "index",
            Self::View => "view",
            Self::Tool => "tool",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "entity_type" => Ok(Self::EntityType),
            "index" => Ok(Self::Index),
            "view" => Ok(Self::View),
            "tool" => Ok(Self::Tool),
            _ => Err(StorageError::Validation("invalid semantic catalog kind")),
        }
    }
}

/// A backend routing-contract catalog entry (authority row).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticCatalogEntry {
    pub entry_id: String,
    pub workspace_id: String,
    pub entry_kind: SemanticCatalogKind,
    pub name: String,
    pub version: i32,
    pub description: String,
    /// Backend query routes (knowledge_graph | shadow_ws_lexical |
    /// shadow_ws_vector | bounded_read | sql_query).
    pub query_routes: Vec<String>,
    pub supported_selectors: Vec<String>,
    pub default_budgets: Option<Value>,
    pub examples: Value,
    pub lifecycle_state: String,
}

/// Insert payload for a catalog entry.
#[derive(Clone, Debug)]
pub struct NewSemanticCatalogEntry {
    pub workspace_id: String,
    pub entry_kind: SemanticCatalogKind,
    pub name: String,
    pub version: i32,
    pub description: String,
    pub query_routes: Vec<String>,
    pub supported_selectors: Vec<String>,
    pub default_budgets: Option<Value>,
    pub examples: Value,
}

fn catalog_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<SemanticCatalogEntry> {
    Ok(SemanticCatalogEntry {
        entry_id: row.get("entry_id"),
        workspace_id: row.get("workspace_id"),
        entry_kind: SemanticCatalogKind::from_db(row.get::<String, _>("entry_kind").as_str())?,
        name: row.get("name"),
        version: row.get("version"),
        description: row.get("description"),
        query_routes: serde_json::from_value(row.get("query_routes"))
            .map_err(|_| StorageError::Validation("invalid query_routes json"))?,
        supported_selectors: serde_json::from_value(row.get("supported_selectors"))
            .map_err(|_| StorageError::Validation("invalid supported_selectors json"))?,
        default_budgets: row.get("default_budgets"),
        examples: row.get("examples"),
        lifecycle_state: row.get("lifecycle_state"),
    })
}

/// Upsert a catalog entry (by workspace+name+version). Routing contracts are
/// authoritative and must be queryable, not prompt-only (folded stub intent).
pub async fn upsert_semantic_catalog_entry(
    pool: &PgPool,
    new: NewSemanticCatalogEntry,
) -> StorageResult<SemanticCatalogEntry> {
    if new.name.trim() != new.name || new.name.is_empty() {
        return Err(StorageError::Validation(
            "semantic catalog name must be non-empty and trimmed",
        ));
    }
    let entry_id = format!("KSC-{}", Uuid::now_v7().simple());
    let routes = serde_json::to_value(&new.query_routes)
        .map_err(|_| StorageError::Validation("query_routes not serializable"))?;
    let selectors = serde_json::to_value(&new.supported_selectors)
        .map_err(|_| StorageError::Validation("supported_selectors not serializable"))?;
    let row = sqlx::query(
        r#"
        INSERT INTO knowledge_semantic_catalog_entries
            (entry_id, workspace_id, entry_kind, name, version, description,
             query_routes, supported_selectors, default_budgets, examples)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (workspace_id, name, version) DO UPDATE SET
            entry_kind = EXCLUDED.entry_kind,
            description = EXCLUDED.description,
            query_routes = EXCLUDED.query_routes,
            supported_selectors = EXCLUDED.supported_selectors,
            default_budgets = EXCLUDED.default_budgets,
            examples = EXCLUDED.examples,
            lifecycle_state = 'active',
            last_updated_at = NOW()
        RETURNING entry_id, workspace_id, entry_kind, name, version, description,
                  query_routes, supported_selectors, default_budgets, examples,
                  lifecycle_state
        "#,
    )
    .bind(&entry_id)
    .bind(&new.workspace_id)
    .bind(new.entry_kind.as_str())
    .bind(&new.name)
    .bind(new.version)
    .bind(&new.description)
    .bind(&routes)
    .bind(&selectors)
    .bind(&new.default_budgets)
    .bind(&new.examples)
    .fetch_one(pool)
    .await?;
    catalog_from_row(&row)
}

/// Resolve a catalog entry by name (highest active version). Returns the backend
/// routing contract the planner uses to choose a route deterministically.
pub async fn resolve_semantic_catalog_entry(
    pool: &PgPool,
    workspace_id: &str,
    name: &str,
) -> StorageResult<Option<SemanticCatalogEntry>> {
    let row = sqlx::query(
        r#"
        SELECT entry_id, workspace_id, entry_kind, name, version, description,
               query_routes, supported_selectors, default_budgets, examples,
               lifecycle_state
        FROM knowledge_semantic_catalog_entries
        WHERE workspace_id = $1 AND name = $2 AND lifecycle_state = 'active'
        ORDER BY version DESC
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(catalog_from_row).transpose()
}

/// List active catalog entries for a workspace (bounded), for the debug API and
/// the planner's route resolution.
pub async fn list_semantic_catalog_entries(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<SemanticCatalogEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT entry_id, workspace_id, entry_kind, name, version, description,
               query_routes, supported_selectors, default_budgets, examples,
               lifecycle_state
        FROM knowledge_semantic_catalog_entries
        WHERE workspace_id = $1 AND lifecycle_state = 'active'
        ORDER BY name ASC, version DESC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter().map(catalog_from_row).collect()
}
