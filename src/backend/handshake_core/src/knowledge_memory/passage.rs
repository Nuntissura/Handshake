//! MT-115 MemoryPassageSchema (product surface).
//!
//! MemoryPassage authority already exists in the committed substrate as
//! `knowledge_memory_passages` + `knowledge_passage_evidence` (migration 0138,
//! MT-057): bounded model-context text, ranking features, retrieval mode,
//! freshness, compaction policy, OCR/transcript metadata, extraction
//! confidence, failure receipts, and a REQUIRED derivation lineage
//! (source / claim / span refs) enforced by a commit-time trigger.
//!
//! The MemoryGraph does NOT duplicate that table. MT-115's job is to give the
//! memory layer a typed read over the existing passage + its evidence lineage,
//! and — because facts are backed 1:1 by claims — to resolve which memory facts
//! a passage cites (transitively, through its claim evidence refs). This is the
//! read side the passage/evidence graph projection (MT-118) and the backend API
//! (MT-126) consume.

use sqlx::{PgPool, Row};

use crate::storage::knowledge::{
    KnowledgeMemoryPassage, KnowledgePassageEvidenceRef, KnowledgeStore,
};
use crate::storage::knowledge_memory::{get_memory_fact_by_claim, MemoryFact};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

/// List the passage ids of a workspace (newest first), then load each through
/// the committed passage store so the typed `KnowledgeMemoryPassage` (with its
/// ranking features / retrieval mode / freshness / compaction policy) is used —
/// the evidence-graph projection (MT-118) needs the full passage record, not a
/// hand-rolled partial. The id list is a direct read over
/// `knowledge_memory_passages`; the records come from `KnowledgeStore`.
pub async fn load_passages_for_workspace(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<KnowledgeMemoryPassage>> {
    let rows = sqlx::query(
        r#"
        SELECT passage_id FROM knowledge_memory_passages
        WHERE workspace_id = $1
        ORDER BY created_at DESC, passage_id DESC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let db = PostgresDatabase::new(pool.clone());
    let mut passages = Vec::with_capacity(rows.len());
    for row in &rows {
        let passage_id: String = row.get("passage_id");
        if let Some(passage) = db.get_knowledge_memory_passage(&passage_id).await? {
            passages.push(passage);
        }
    }
    Ok(passages)
}

/// A passage together with its derivation lineage and the memory facts it
/// cites through its claim evidence refs.
#[derive(Clone, Debug, PartialEq)]
pub struct PassageWithEvidence {
    pub passage: KnowledgeMemoryPassage,
    pub evidence: Vec<KnowledgePassageEvidenceRef>,
    /// Memory facts reachable via this passage's `claim` evidence refs (a
    /// passage cites a claim; if that claim backs a fact, the fact is cited).
    pub cited_facts: Vec<MemoryFact>,
}

/// Load a passage, its lineage, and the facts it cites. Reuses the committed
/// passage store (`KnowledgeStore`) for the passage + lineage, then resolves
/// the fact behind each `claim` evidence ref.
pub async fn load_passage_with_evidence(
    db: &PostgresDatabase,
    pool: &PgPool,
    passage_id: &str,
) -> StorageResult<Option<PassageWithEvidence>> {
    let Some(passage) = db.get_knowledge_memory_passage(passage_id).await? else {
        return Ok(None);
    };
    let evidence = db.list_knowledge_passage_evidence(passage_id).await?;

    let mut cited_facts = Vec::new();
    for reference in &evidence {
        if let KnowledgePassageEvidenceRef::Claim { claim_id } = reference {
            if let Some(fact) = get_memory_fact_by_claim(pool, claim_id).await? {
                cited_facts.push(fact);
            }
        }
    }

    Ok(Some(PassageWithEvidence {
        passage,
        evidence,
        cited_facts,
    }))
}

/// List the passage ids that cite a given claim (reverse of a passage's claim
/// evidence ref). Used by the evidence-graph projection to walk
/// claim -> passages. Direct read over `knowledge_passage_evidence`.
pub async fn list_passages_citing_claim(
    pool: &PgPool,
    claim_id: &str,
) -> StorageResult<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT passage_id
        FROM knowledge_passage_evidence
        WHERE ref_kind = 'claim' AND claim_id = $1
        ORDER BY passage_id ASC
        "#,
    )
    .bind(claim_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("passage_id"))
        .collect())
}
