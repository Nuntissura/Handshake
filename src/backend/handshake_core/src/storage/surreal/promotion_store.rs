//! Atomic graph-fact promotion for the embedded SurrealDB authority store.

use surrealdb::types::{RecordId, SurrealValue};

use super::{event_ledger, SurrealStorage};
use crate::kernel::NewKernelEvent;
use crate::storage::{knowledge_crdt, StorageError, StorageResult};

const GRAPH_PROPOSALS_TABLE: &str = "knowledge_crdt_graph_proposals";

#[derive(SurrealValue)]
struct PromotionFactWrite {
    fact_id: String,
    proposal_id: RecordId,
    workspace_id: String,
    mutation_kind: String,
    fact_payload: serde_json::Value,
    source_span_refs: Vec<String>,
    confidence: f64,
    proposed_by: String,
    promoted_by: String,
}

#[derive(SurrealValue)]
struct PromotionBindings {
    requested: event_ledger::LedgerWrite,
    accepted: event_ledger::LedgerWrite,
    fact: PromotionFactWrite,
}

/// Appends the causation-linked promotion receipt pair, freezes the authority
/// fact, and advances the proposal in one transaction. Exact EventLedger
/// replay returns the first stored fact; conflicting idempotency-key reuse
/// aborts every mutation. Span ownership and retirement are rechecked inside
/// the same transaction so a preflight result cannot go stale before commit.
pub(crate) async fn promote_graph_fact_atomic(
    storage: &SurrealStorage,
    requested: NewKernelEvent,
    mut accepted: NewKernelEvent,
    fact: knowledge_crdt::NewPromotedFact,
) -> StorageResult<knowledge_crdt::PromotedFactRow> {
    let source_span_refs = knowledge_crdt::span_refs_from_json(&fact.source_span_refs)?;
    if source_span_refs.is_empty() {
        return Err(StorageError::Validation(
            "promoted fact source_span_refs must not be empty",
        ));
    }

    let (requested_candidate, requested_write) = event_ledger::prepare_event(requested)?;
    accepted.causation_id = Some(requested_candidate.event_id);
    let (_, accepted_write) = event_ledger::prepare_event(accepted)?;

    let bindings = PromotionBindings {
        requested: requested_write,
        accepted: accepted_write,
        fact: PromotionFactWrite {
            fact_id: fact.fact_id,
            proposal_id: RecordId::new(GRAPH_PROPOSALS_TABLE, fact.proposal_id),
            workspace_id: fact.workspace_id,
            mutation_kind: fact.mutation_kind,
            fact_payload: fact.fact_payload,
            source_span_refs,
            confidence: fact.confidence,
            proposed_by: fact.proposed_by,
            promoted_by: fact.promoted_by,
        },
    };

    let rows: Vec<knowledge_crdt::PromotedFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(PROMOTE_GRAPH_FACT_STATEMENT, bindings, 14)
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;

    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound(
            "promoted fact after atomic promotion",
        ))?
        .into_row()
}

const PROMOTE_GRAPH_FACT_STATEMENT: &str = "BEGIN TRANSACTION; \
    LET $requested_stored = (SELECT * FROM kernel_event_ledger \
      WHERE idempotency_key = $requested.idempotency_key LIMIT 1)[0]; \
    IF $requested_stored != NONE { \
      IF $requested_stored.event_version != $requested.event_version \
         OR $requested_stored.kernel_task_run_id != $requested.kernel_task_run_id \
         OR $requested_stored.session_run_id != $requested.session_run_id \
         OR $requested_stored.aggregate_type != $requested.aggregate_type \
         OR $requested_stored.aggregate_id != $requested.aggregate_id \
         OR $requested_stored.event_type != $requested.event_type \
         OR $requested_stored.actor_kind != $requested.actor_kind \
         OR $requested_stored.actor_id != $requested.actor_id \
         OR $requested_stored.causation_id != $requested.causation_id \
         OR $requested_stored.correlation_id != $requested.correlation_id \
         OR $requested_stored.payload_hash != $requested.payload_hash \
         OR $requested_stored.source_component != $requested.source_component { \
        THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
      }; \
    } ELSE { \
      CREATE $requested.record CONTENT { \
        event_id: $requested.event_id, event_version: $requested.event_version, \
        kernel_task_run_id: $requested.kernel_task_run_id, \
        session_run_id: $requested.session_run_id, aggregate_type: $requested.aggregate_type, \
        aggregate_id: $requested.aggregate_id, idempotency_key: $requested.idempotency_key, \
        event_type: $requested.event_type, actor_kind: $requested.actor_kind, \
        actor_id: $requested.actor_id, causation_id: $requested.causation_id, \
        correlation_id: $requested.correlation_id, payload_hash: $requested.payload_hash, \
        source_component: $requested.source_component, payload: $requested.payload, \
        created_at: $requested.created_at \
      } RETURN NONE; \
    }; \
    LET $actual_requested = (SELECT * FROM kernel_event_ledger \
      WHERE idempotency_key = $requested.idempotency_key LIMIT 1)[0]; \
    LET $accepted_stored = (SELECT * FROM kernel_event_ledger \
      WHERE idempotency_key = $accepted.idempotency_key LIMIT 1)[0]; \
    IF $accepted_stored != NONE { \
      IF $accepted_stored.event_version != $accepted.event_version \
         OR $accepted_stored.kernel_task_run_id != $accepted.kernel_task_run_id \
         OR $accepted_stored.session_run_id != $accepted.session_run_id \
         OR $accepted_stored.aggregate_type != $accepted.aggregate_type \
         OR $accepted_stored.aggregate_id != $accepted.aggregate_id \
         OR $accepted_stored.event_type != $accepted.event_type \
         OR $accepted_stored.actor_kind != $accepted.actor_kind \
         OR $accepted_stored.actor_id != $accepted.actor_id \
         OR $accepted_stored.causation_id != $actual_requested.event_id \
         OR $accepted_stored.correlation_id != $accepted.correlation_id \
         OR $accepted_stored.payload_hash != $accepted.payload_hash \
         OR $accepted_stored.source_component != $accepted.source_component { \
        THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
      }; \
    } ELSE { \
      CREATE $accepted.record CONTENT { \
        event_id: $accepted.event_id, event_version: $accepted.event_version, \
        kernel_task_run_id: $accepted.kernel_task_run_id, \
        session_run_id: $accepted.session_run_id, aggregate_type: $accepted.aggregate_type, \
        aggregate_id: $accepted.aggregate_id, idempotency_key: $accepted.idempotency_key, \
        event_type: $accepted.event_type, actor_kind: $accepted.actor_kind, \
        actor_id: $accepted.actor_id, causation_id: $actual_requested.event_id, \
        correlation_id: $accepted.correlation_id, payload_hash: $accepted.payload_hash, \
        source_component: $accepted.source_component, payload: $accepted.payload, \
        created_at: $accepted.created_at \
      } RETURN NONE; \
    }; \
    LET $actual_accepted = (SELECT * FROM kernel_event_ledger \
      WHERE idempotency_key = $accepted.idempotency_key LIMIT 1)[0]; \
    LET $proposal = (SELECT * FROM $fact.proposal_id LIMIT 1)[0]; \
    IF $proposal = NONE \
       OR ($proposal.review_state != 'approved' AND $proposal.review_state != 'promoted') \
       OR $proposal.workspace_id != $fact.workspace_id \
       OR $proposal.mutation_kind != $fact.mutation_kind \
       OR $proposal.mutation_payload != $fact.fact_payload \
       OR $proposal.source_span_refs != $fact.source_span_refs \
       OR $proposal.confidence != $fact.confidence \
       OR $proposal.actor_id != $fact.proposed_by { \
      THROW 'HSK-PROMOTION-PROPOSAL-MISMATCH'; \
    }; \
    FOR $span_ref IN $fact.source_span_refs { \
      LET $span = (SELECT source_id.workspace_id AS workspace_id, \
          source_id.stale AS stale FROM knowledge_spans \
          WHERE span_id = $span_ref LIMIT 1)[0]; \
      IF $span = NONE OR record::id($span.workspace_id) != $fact.workspace_id \
         OR $span.stale = true { \
        THROW 'HSK-PROMOTION-SPAN-INVALID'; \
      }; \
    }; \
    LET $existing_fact = (SELECT * FROM knowledge_crdt_promoted_facts \
      WHERE proposal_id = $fact.proposal_id LIMIT 1)[0]; \
    IF $existing_fact = NONE { \
      CREATE type::record('knowledge_crdt_promoted_facts', $fact.fact_id) CONTENT { \
        fact_id: $fact.fact_id, proposal_id: $fact.proposal_id, \
        workspace_id: $fact.workspace_id, mutation_kind: $fact.mutation_kind, \
        fact_payload: $fact.fact_payload, source_span_refs: $fact.source_span_refs, \
        confidence: $fact.confidence, proposed_by: $fact.proposed_by, \
        promoted_by: $fact.promoted_by, \
        promotion_requested_event_id: $actual_requested.id, \
        promotion_accepted_event_id: $actual_accepted.id \
      } RETURN NONE; \
    } ELSE IF $existing_fact.workspace_id != $proposal.workspace_id \
       OR $existing_fact.mutation_kind != $proposal.mutation_kind \
       OR $existing_fact.fact_payload != $proposal.mutation_payload \
       OR $existing_fact.source_span_refs != $proposal.source_span_refs \
       OR $existing_fact.confidence != $proposal.confidence \
       OR $existing_fact.proposed_by != $proposal.actor_id { \
      THROW 'HSK-PROMOTION-EXISTING-FACT-MISMATCH'; \
    }; \
    UPDATE knowledge_crdt_graph_proposals SET review_state = 'promoted' \
      WHERE id = $fact.proposal_id AND review_state = 'approved'; \
    COMMIT TRANSACTION; \
    SELECT * FROM knowledge_crdt_promoted_facts \
      WHERE proposal_id = $fact.proposal_id LIMIT 1;";
