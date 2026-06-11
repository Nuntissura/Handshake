//! MT-128 MemoryGraphFixtures.
//!
//! Reusable, deterministic scenario builders for the memory graph: the six
//! states a no-context model (or a visual-debug demo, or a validator) needs to
//! see exercised — contradictions, stale facts, fragmented subgraphs, false
//! bridge edges, unsupported claims, and a successful promotion. Each builder
//! seeds REAL authority rows on PostgreSQL through the committed substrate plus
//! the MemoryGraph storage; none of them mock or stub.
//!
//! These are product fixtures (callable from product code and tests), not test
//! files: they construct the scenarios the MT-128 contract enumerates so any
//! consumer can stand up a known memory-graph shape. A caller supplies the
//! workspace, the subject/object entities, and the evidence span (the minimum
//! citeable unit every claim needs); the builders return the ids they created so
//! assertions can target them.
//!
//! Every backing claim carries the REQUIRED evidence span, so the claim
//! evidence/lifecycle/conflict machinery holds for every fixture fact.

use sqlx::PgPool;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    KnowledgeClaimKind, KnowledgeClaimState, KnowledgeStore, NewKnowledgeClaim,
};
use crate::storage::knowledge_memory::{
    create_memory_fact, promote_memory_ontology_term, record_conflict_resolution_job,
    upsert_memory_ontology_term, ConflictResolutionOutcome, MemoryClaimAuthorityLabel, MemoryFact,
    MemoryFactObject, MemoryOntologyTerm, MemoryOntologyTermKind, NewMemoryFact,
    NewMemoryOntologyTerm,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{Database, StorageResult};

use super::conflict::run_symbolic_conflict_detection;

/// Inputs shared by every fixture builder: the workspace, an evidence span every
/// claim cites, and a deterministic label seed so ids/text don't collide across
/// fixtures in one workspace.
#[derive(Clone, Debug)]
pub struct FixtureContext {
    pub workspace_id: String,
    pub evidence_span_id: String,
    pub seed: String,
}

impl FixtureContext {
    fn claim_text(&self, suffix: &str) -> String {
        format!("[{}] {}", self.seed, suffix)
    }
}

/// Create an evidence-backed proposed claim citing the fixture span.
async fn seed_claim(
    db: &PostgresDatabase,
    ctx: &FixtureContext,
    text: &str,
) -> StorageResult<String> {
    let claim = db
        .create_knowledge_claim(NewKnowledgeClaim {
            workspace_id: ctx.workspace_id.clone(),
            claim_kind: KnowledgeClaimKind::ProductBehavior,
            claim_text: text.to_string(),
            subject_entity_id: None,
            temporal_qualifier: None,
            granularity_qualifier: None,
            confidence: 0.7,
            proposed_in_run: None,
            evidence_span_ids: vec![ctx.evidence_span_id.clone()],
        })
        .await?;
    Ok(claim.claim_id)
}

/// CONTRADICTION fixture: two facts on the same (subject, predicate) with
/// different objects, then run detection so both backing claims are `conflicted`
/// and a `knowledge_claim_conflict` exists.
#[derive(Clone, Debug)]
pub struct ContradictionFixture {
    pub fact_a: MemoryFact,
    pub fact_b: MemoryFact,
    pub conflict_ids: Vec<String>,
}

pub async fn contradiction(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
    subject_entity_id: &str,
) -> StorageResult<ContradictionFixture> {
    let claim_a = seed_claim(db, ctx, &ctx.claim_text("port is 5544")).await?;
    let claim_b = seed_claim(db, ctx, &ctx.claim_text("port is 5432")).await?;
    let fact_a = make_fact(
        pool,
        ctx,
        &claim_a,
        subject_entity_id,
        "default_port",
        MemoryFactObject::Literal {
            value: "5544".to_string(),
        },
        MemoryClaimAuthorityLabel::ModelSuggested,
    )
    .await?;
    let fact_b = make_fact(
        pool,
        ctx,
        &claim_b,
        subject_entity_id,
        "default_port",
        MemoryFactObject::Literal {
            value: "5432".to_string(),
        },
        MemoryClaimAuthorityLabel::ModelSuggested,
    )
    .await?;
    let detection = run_symbolic_conflict_detection(db, pool, &ctx.workspace_id, 100, None).await?;
    Ok(ContradictionFixture {
        fact_a,
        fact_b,
        conflict_ids: detection.conflict_ids,
    })
}

/// STALE FACT fixture: an accepted fact whose backing claim is then retired as
/// `stale` (a fact that was true but no longer is). Returns the retired claim id.
pub async fn stale_fact(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
    subject_entity_id: &str,
) -> StorageResult<(MemoryFact, String)> {
    let claim_id = seed_claim(db, ctx, &ctx.claim_text("uses legacy port 5432")).await?;
    let fact = make_fact(
        pool,
        ctx,
        &claim_id,
        subject_entity_id,
        "legacy_port",
        MemoryFactObject::Literal {
            value: "5432".to_string(),
        },
        MemoryClaimAuthorityLabel::Source,
    )
    .await?;
    // Accept then retire as stale (receipt-backed acceptance).
    let receipt = seed_receipt(db, ctx, "stale-accept").await?;
    db.transition_knowledge_claim(
        &claim_id,
        KnowledgeClaimState::Accepted,
        None,
        Some(&receipt),
    )
    .await?;
    let retired = db
        .transition_knowledge_claim(
            &claim_id,
            KnowledgeClaimState::Retired,
            Some(crate::storage::knowledge::KnowledgeClaimRetirement {
                reason: crate::storage::knowledge::KnowledgeClaimRetirementReason::Stale,
                superseded_by_claim_id: None,
            }),
            None,
        )
        .await?;
    Ok((fact, retired.claim_id))
}

/// FRAGMENTED SUBGRAPH fixture: two entities that co-occur in evidence but have
/// NO edge between them (disconnected components). Returns the two entity ids the
/// caller passed back, plus a fact on each so each is a real graph participant.
pub async fn fragmented_subgraph(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
    entity_a: &str,
    entity_b: &str,
) -> StorageResult<(MemoryFact, MemoryFact)> {
    let claim_a = seed_claim(db, ctx, &ctx.claim_text("island A property")).await?;
    let claim_b = seed_claim(db, ctx, &ctx.claim_text("island B property")).await?;
    let fact_a = make_fact(
        pool,
        ctx,
        &claim_a,
        entity_a,
        "property",
        MemoryFactObject::Literal {
            value: "alpha".to_string(),
        },
        MemoryClaimAuthorityLabel::Source,
    )
    .await?;
    let fact_b = make_fact(
        pool,
        ctx,
        &claim_b,
        entity_b,
        "property",
        MemoryFactObject::Literal {
            value: "beta".to_string(),
        },
        MemoryClaimAuthorityLabel::Source,
    )
    .await?;
    Ok((fact_a, fact_b))
}

/// UNSUPPORTED CLAIM fixture: a fact labelled `unsupported` (no surviving
/// evidence basis) — the kind that must be excluded from the stable fact graph.
pub async fn unsupported_claim(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
    subject_entity_id: &str,
) -> StorageResult<MemoryFact> {
    let claim_id = seed_claim(db, ctx, &ctx.claim_text("rumored owner unknown")).await?;
    make_fact(
        pool,
        ctx,
        &claim_id,
        subject_entity_id,
        "owner",
        MemoryFactObject::Literal {
            value: "unknown".to_string(),
        },
        MemoryClaimAuthorityLabel::Unsupported,
    )
    .await
}

/// SUCCESSFUL PROMOTION fixture: an operator-approved ontology term promoted to
/// `stable` with a receipt. Returns the promoted (stable) term.
pub async fn successful_promotion(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
) -> StorageResult<MemoryOntologyTerm> {
    let term = upsert_memory_ontology_term(
        pool,
        NewMemoryOntologyTerm {
            workspace_id: ctx.workspace_id.clone(),
            term_kind: MemoryOntologyTermKind::RelationClass,
            term_key: format!("{}_depends_on", ctx.seed),
            normalized_label: "depends on".to_string(),
            maps_to_edge_type: Some("depends_on".to_string()),
            maps_to_entity_kind: None,
            promotion_threshold: 3,
            operator_approved: true,
            detection_provenance: serde_json::json!({"fixture": "successful_promotion"}),
            seen_in_run: None,
        },
    )
    .await?;
    let receipt = seed_receipt(db, ctx, "promotion").await?;
    promote_memory_ontology_term(pool, &term.term_id, &receipt).await
}

/// FALSE BRIDGE EDGE fixture: a `relates_to` bridge edge between two entities
/// whose only "evidence" is a shared span, which is then RESOLVED AWAY as a false
/// bridge by retiring the edge — modelling the "discovered, then rejected"
/// bridge an operator/validator removes. Returns (edge_id, retired bool).
///
/// This deliberately creates a bridge then retires it, so a consumer can test
/// that a false bridge is recorded and removable (the edge lifecycle supports
/// `retired`); the bridge-decision log + the edge both reflect the false bridge.
pub async fn false_bridge_edge(
    db: &PostgresDatabase,
    _pool: &PgPool,
    ctx: &FixtureContext,
    entity_a: &str,
    entity_b: &str,
) -> StorageResult<String> {
    use crate::storage::knowledge::{KnowledgeEdgeLifecycle, KnowledgeEdgeType, NewKnowledgeEdge};
    // Create a proposed bridge backed by the shared span.
    let edge = db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: ctx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::RelatesTo,
            source_entity_id: entity_a.to_string(),
            target_entity_id: entity_b.to_string(),
            extractor_version: "false_bridge_fixture_v1".to_string(),
            confidence: 0.2,
            detected_in_run: None,
            evidence_span_ids: vec![ctx.evidence_span_id.clone()],
        })
        .await?;
    let edge = db
        .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Proposed, None)
        .await?;
    // Reject it as a false bridge: retire the edge.
    let retired = db
        .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Retired, None)
        .await?;
    Ok(retired.edge_id)
}

/// Resolve a contradiction fixture's conflict with a discard outcome + receipt,
/// so a consumer can stand up the "conflict then resolved" end-state.
pub async fn resolve_contradiction(
    db: &PostgresDatabase,
    pool: &PgPool,
    ctx: &FixtureContext,
    conflict_id: &str,
    kept_claim_id: &str,
    discarded_claim_id: &str,
) -> StorageResult<String> {
    let receipt = seed_receipt(db, ctx, "resolve").await?;
    // Receipt-back the committed conflict resolution and the job record.
    db.resolve_knowledge_claim_conflict(conflict_id, &receipt)
        .await?;
    let job = record_conflict_resolution_job(
        pool,
        &ctx.workspace_id,
        conflict_id,
        ConflictResolutionOutcome::Discard,
        Some(kept_claim_id),
        Some(discarded_claim_id),
        serde_json::json!({"fixture": "resolve_contradiction"}),
        &receipt,
    )
    .await?;
    Ok(job.job_id)
}

// --- shared helpers --------------------------------------------------------

async fn make_fact(
    pool: &PgPool,
    ctx: &FixtureContext,
    claim_id: &str,
    subject_entity_id: &str,
    predicate_key: &str,
    object: MemoryFactObject,
    authority_label: MemoryClaimAuthorityLabel,
) -> StorageResult<MemoryFact> {
    create_memory_fact(
        pool,
        NewMemoryFact {
            workspace_id: ctx.workspace_id.clone(),
            claim_id: claim_id.to_string(),
            subject_entity_id: subject_entity_id.to_string(),
            predicate_key: predicate_key.to_string(),
            predicate_term_id: None,
            object,
            qualifiers: serde_json::json!({}),
            authority_label,
            extractor_version: "fixture_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
}

/// Append a real EventLedger receipt the fixtures use for receipt-backed
/// acceptances/resolutions/promotions.
async fn seed_receipt(
    db: &PostgresDatabase,
    ctx: &FixtureContext,
    kind: &str,
) -> StorageResult<String> {
    let suffix = uuid::Uuid::now_v7();
    let event = NewKernelEvent::builder(
        format!("KTR-FIX-{kind}-{suffix}"),
        format!("SR-FIX-{kind}-{suffix}"),
        KernelEventType::ValidationRecorded,
        KernelActor::ValidationRunner(format!("fixture-{kind}")),
    )
    .aggregate("knowledge_memory_fixture", ctx.seed.clone())
    .idempotency_key(format!("idem-fix-{kind}-{suffix}"))
    .payload(serde_json::json!({"fixture": kind}))
    .build()
    .map_err(|err| crate::storage::StorageError::Validation(leak_build_error(err)))?;
    let stored = db.append_kernel_event(event).await?;
    Ok(stored.event_id)
}

/// The kernel event builder returns a String error; StorageError::Validation
/// wants &'static str. Fixtures only fail here on a programmer error (malformed
/// builder), so collapse to a single static reason.
fn leak_build_error(_err: impl std::fmt::Display) -> &'static str {
    "fixture kernel event build failed"
}
