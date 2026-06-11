//! MT-127 MemoryGraphVisualDebugPayload.
//!
//! A typed BACKEND payload that exposes the memory-graph state to visual/debug
//! surfaces with stable IDs and source citations (spec 2.3.13.11: diagnostic
//! tooling must expose structured state for a no-context model). This is the
//! backend SHAPE + its builder + a test — NOT React wiring. The frontend
//! visual-debug surface (and MT-126's API) serialize this payload.
//!
//! The payload is a PROJECTION (never authority): it is assembled from the
//! ontology / fact / passage-evidence graph projections plus the claim-state
//! counts, and every node carries the stable authority id it came from so a
//! no-context model can jump from any visual node back to its evidence.
//!
//! Determinism: the payload is a pure function of the current authority state in
//! one workspace, bounded by a `limit`. Re-rendering it never mutates anything.

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

use super::projection::{
    build_fact_graph, build_ontology_graph, build_passage_evidence_graph, FactGraphProjection,
    OntologyGraphProjection, PassageEvidenceGraphProjection,
};

/// Per-lifecycle-state counts of the backing knowledge claims, so a visual
/// surface can show how much of the graph is proposed / accepted / conflicted /
/// retired at a glance.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimStateCounts {
    pub proposed: i64,
    pub accepted: i64,
    pub conflicted: i64,
    pub retired: i64,
}

/// Per-authority-label counts of memory facts (MT-125 labels), so the surface
/// can distinguish source/operator-approved facts from model-suggested or
/// unsupported ones.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactLabelCounts {
    pub source: i64,
    pub derived: i64,
    pub model_suggested: i64,
    pub operator_approved: i64,
    pub deprecated: i64,
    pub superseded: i64,
    pub unsupported: i64,
}

/// The complete memory-graph visual-debug payload for one workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryGraphVisualDebugPayload {
    pub workspace_id: String,
    /// Schema id so a no-context consumer can validate the payload shape.
    pub schema_id: &'static str,
    pub ontology_graph: OntologyGraphProjection,
    pub fact_graph: FactGraphProjection,
    pub passage_evidence_graph: PassageEvidenceGraphProjection,
    pub claim_state_counts: ClaimStateCounts,
    pub fact_label_counts: FactLabelCounts,
    /// Count of open (unresolved) claim conflicts — the "repair queue" size the
    /// visual surface highlights (MT-126 repair-queue view shares this signal).
    pub open_conflict_count: i64,
    /// Always "projection": this payload is never authority (spec 2.3.13.11).
    pub authority_class: &'static str,
}

pub const MEMORY_GRAPH_VISUAL_DEBUG_SCHEMA_ID: &str = "hsk.memory_graph_visual_debug@1";

/// Build the memory-graph visual-debug payload for a workspace. Composes the
/// three graph projections and the claim/fact/conflict counts. `trusted_only`
/// is forwarded to the fact graph (a "stable view" vs the full view).
pub async fn build_memory_graph_visual_debug(
    db: &PostgresDatabase,
    pool: &PgPool,
    workspace_id: &str,
    trusted_only: bool,
    limit: i64,
) -> StorageResult<MemoryGraphVisualDebugPayload> {
    let ontology_graph = build_ontology_graph(pool, workspace_id, false, limit).await?;
    let fact_graph = build_fact_graph(db, pool, workspace_id, trusted_only, limit).await?;
    let passage_evidence_graph =
        build_passage_evidence_graph(db, pool, workspace_id, limit).await?;
    let claim_state_counts = claim_state_counts(pool, workspace_id).await?;
    let fact_label_counts = fact_label_counts(pool, workspace_id).await?;
    let open_conflict_count = open_conflict_count(pool, workspace_id).await?;

    Ok(MemoryGraphVisualDebugPayload {
        workspace_id: workspace_id.to_string(),
        schema_id: MEMORY_GRAPH_VISUAL_DEBUG_SCHEMA_ID,
        ontology_graph,
        fact_graph,
        passage_evidence_graph,
        claim_state_counts,
        fact_label_counts,
        open_conflict_count,
        authority_class: "projection",
    })
}

/// Count claims by lifecycle state in a workspace.
async fn claim_state_counts(pool: &PgPool, workspace_id: &str) -> StorageResult<ClaimStateCounts> {
    let rows = sqlx::query(
        r#"
        SELECT lifecycle_state, COUNT(*) AS n
        FROM knowledge_claims
        WHERE workspace_id = $1
        GROUP BY lifecycle_state
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    let mut counts = ClaimStateCounts::default();
    for row in &rows {
        let state: String = row.get("lifecycle_state");
        let n: i64 = row.get("n");
        match state.as_str() {
            "proposed" => counts.proposed = n,
            "accepted" => counts.accepted = n,
            "conflicted" => counts.conflicted = n,
            "retired" => counts.retired = n,
            _ => {}
        }
    }
    Ok(counts)
}

/// Count memory facts by authority label in a workspace.
async fn fact_label_counts(pool: &PgPool, workspace_id: &str) -> StorageResult<FactLabelCounts> {
    let rows = sqlx::query(
        r#"
        SELECT authority_label, COUNT(*) AS n
        FROM knowledge_memory_facts
        WHERE workspace_id = $1
        GROUP BY authority_label
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    let mut counts = FactLabelCounts::default();
    for row in &rows {
        let label: String = row.get("authority_label");
        let n: i64 = row.get("n");
        match label.as_str() {
            "source" => counts.source = n,
            "derived" => counts.derived = n,
            "model_suggested" => counts.model_suggested = n,
            "operator_approved" => counts.operator_approved = n,
            "deprecated" => counts.deprecated = n,
            "superseded" => counts.superseded = n,
            "unsupported" => counts.unsupported = n,
            _ => {}
        }
    }
    Ok(counts)
}

/// Count open (unresolved) claim conflicts in a workspace — the repair-queue
/// signal. A conflict is open while `resolved_at IS NULL`.
async fn open_conflict_count(pool: &PgPool, workspace_id: &str) -> StorageResult<i64> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) AS n
        FROM knowledge_claim_conflicts kcc
        JOIN knowledge_claims kc ON kc.claim_id = kcc.claim_id
        WHERE kc.workspace_id = $1 AND kcc.resolved_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>("n"))
}
