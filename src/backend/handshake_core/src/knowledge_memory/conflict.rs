//! MT-121 ConflictCandidateSearch, MT-122 ConflictDetectionAgentJob,
//! MT-123 ConflictResolutionAgentJob (product orchestration).
//!
//! The deterministic candidate search, the typed detection/resolution job
//! records, and the storage CRUD live in `storage::knowledge_memory`. This
//! module orchestrates them into the two "agent jobs":
//!
//! * `run_symbolic_conflict_detection` — runs the deterministic candidate
//!   search (facts sharing a subject+predicate but disagreeing on the object),
//!   records each candidate as a committed `knowledge_claim_conflict` (which
//!   moves both backing claims into `conflicted` via the 0137 substrate), and
//!   files a typed detection-job record linking the conflicts it produced.
//!
//! * resolution is recorded via `storage::knowledge_memory::
//!   record_conflict_resolution_job` plus the committed
//!   `resolve_knowledge_claim_conflict` receipt path; this module re-exports the
//!   vocabulary so callers import one `knowledge_memory::conflict` path.
//!
//! These are NOT spawned LLM agents: a job is a typed record + deterministic
//! logic. LLM-driven semantic detection is a future runtime concern; the
//! contract notes "embedding/vector-like evidence where available".

use sqlx::PgPool;

use crate::storage::knowledge::KnowledgeStore;
use crate::storage::knowledge_memory::{
    find_fact_conflict_candidates, record_conflict_detection_job, ConflictDetectionJob,
    ConflictDetectionKind, FactConflictCandidate,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;
use serde_json::json;

pub use crate::storage::knowledge_memory::{
    list_conflict_detection_findings, list_conflict_resolution_jobs,
    record_conflict_resolution_job, ConflictResolutionJob, ConflictResolutionOutcome,
};

/// Result of a symbolic conflict-detection pass: the typed job record and the
/// candidate pairs it acted on (each now a committed claim conflict).
#[derive(Clone, Debug)]
pub struct SymbolicDetectionResult {
    pub job: ConflictDetectionJob,
    pub candidates: Vec<FactConflictCandidate>,
    /// The conflict ids recorded (one per candidate pair, skipping pairs whose
    /// claims were already in conflict so the pass is idempotent).
    pub conflict_ids: Vec<String>,
}

/// MT-121 + MT-122: run a deterministic symbolic conflict-detection pass over a
/// workspace's memory facts. For each candidate pair (same subject+predicate,
/// different object) it records a committed `knowledge_claim_conflict` between
/// the two backing claims, then files a detection-job record linking those
/// conflicts.
///
/// Idempotent: a candidate whose claim pair already has a recorded conflict
/// (the 0137 `uq_knowledge_claim_conflicts_pair`) is skipped, not duplicated,
/// so re-running the pass does not error or double-count.
pub async fn run_symbolic_conflict_detection(
    db: &PostgresDatabase,
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
    detection_receipt_event_id: Option<&str>,
) -> StorageResult<SymbolicDetectionResult> {
    let candidates = find_fact_conflict_candidates(pool, workspace_id, limit).await?;

    let mut conflict_ids = Vec::new();
    for candidate in &candidates {
        if claims_already_conflicting(db, &candidate.claim_id_a, &candidate.claim_id_b).await? {
            continue;
        }
        let conflict = db
            .record_knowledge_claim_conflict(
                &candidate.claim_id_a,
                &candidate.claim_id_b,
                &format!(
                    "symbolic conflict: subject {} predicate '{}' disagrees ({} vs {})",
                    candidate.subject_entity_id,
                    candidate.predicate_key,
                    candidate.object_a,
                    candidate.object_b
                ),
                None,
            )
            .await?;
        conflict_ids.push(conflict.conflict_id);
    }

    let job = record_conflict_detection_job(
        pool,
        workspace_id,
        ConflictDetectionKind::Symbolic,
        candidates.len() as i32,
        json!({"strategy": "subject_predicate_object_mismatch"}),
        &conflict_ids,
        detection_receipt_event_id,
    )
    .await?;

    Ok(SymbolicDetectionResult {
        job,
        candidates,
        conflict_ids,
    })
}

/// Whether two claims already have a recorded conflict in either direction
/// (the committed conflict store enforces a unique unordered pair).
async fn claims_already_conflicting(
    db: &PostgresDatabase,
    claim_a: &str,
    claim_b: &str,
) -> StorageResult<bool> {
    let conflicts = db.list_knowledge_claim_conflicts(claim_a).await?;
    Ok(conflicts.iter().any(|conflict| {
        (conflict.claim_id == claim_a && conflict.conflicting_claim_id == claim_b)
            || (conflict.claim_id == claim_b && conflict.conflicting_claim_id == claim_a)
    }))
}
