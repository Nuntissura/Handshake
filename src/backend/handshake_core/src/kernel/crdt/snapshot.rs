use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::identity::CrdtWorkspaceIdentityV1;
use super::persistence::{
    sha256_hex, validate_crdt_update_record, CrdtReplayMetadataV1, CrdtReplayStepV1,
    CrdtStorageAuthorityPosture, CrdtUpdateRecordV1, CrdtUpdateRecordValidationError,
};

pub const CRDT_SNAPSHOT_RECORD_SCHEMA_ID: &str = "hsk.kernel.crdt_snapshot_record@1";
pub const CRDT_BOUNDED_REPLAY_PLAN_SCHEMA_ID: &str = "hsk.kernel.crdt_bounded_replay_plan@1";
pub const CRDT_COMPACTION_PLAN_SCHEMA_ID: &str = "hsk.kernel.crdt_compaction_plan@1";
pub const CRDT_APPLIED_COMPACTION_SCHEMA_ID: &str = "hsk.kernel.crdt_applied_compaction@1";
pub const CRDT_COMPACTED_UPDATE_AUDIT_SCHEMA_ID: &str = "hsk.kernel.crdt_compacted_update_audit@1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSnapshotRecordV1 {
    pub schema_id: String,
    pub snapshot_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub covered_update_seq: u64,
    pub state_vector: String,
    pub snapshot_sha256: String,
    pub snapshot_bytes_ref: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub promotion_evidence_update_ids: Vec<String>,
    pub storage_authority: CrdtStorageAuthorityPosture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtSnapshotRecordInputV1<'a> {
    pub identity: &'a CrdtWorkspaceIdentityV1,
    pub snapshot_id: &'a str,
    pub covered_update_seq: u64,
    pub snapshot_bytes: &'a [u8],
    pub snapshot_bytes_ref: &'a str,
    pub state_vector: &'a str,
    pub event_ledger_event_id: &'a str,
    pub promotion_evidence_update_ids: &'a [&'a str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtBoundedReplayPlanV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_snapshot_id: String,
    pub base_snapshot_state_vector: String,
    pub replay_from_update_seq: u64,
    pub source_authority: CrdtStorageAuthorityPosture,
    pub ordered_updates: Vec<CrdtReplayStepV1>,
    pub final_state_vector: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtCompactionAuditMode {
    EventLedgerAuditRefs,
    RetainOnly,
    DropWithoutAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtCompactionPolicyV1 {
    pub policy_id: String,
    pub compact_through_update_seq: u64,
    pub audit_mode: CrdtCompactionAuditMode,
    pub preserve_promotion_evidence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtCompactionDisposition {
    RetainForReplay,
    RetainForAudit,
    RetainPromotionEvidence,
    CompactWithAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtCompactionDecisionV1 {
    pub update_id: String,
    pub update_seq: u64,
    pub disposition: CrdtCompactionDisposition,
    pub audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtCompactionPlanV1 {
    pub schema_id: String,
    pub snapshot_id: String,
    pub policy_id: String,
    pub compact_through_update_seq: u64,
    pub decisions: Vec<CrdtCompactionDecisionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtCompactedUpdateAuditV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub update_seq: u64,
    pub update_sha256: String,
    pub update_bytes_ref: String,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub replay_metadata: CrdtReplayMetadataV1,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub audit_ref: String,
    pub storage_authority: CrdtStorageAuthorityPosture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtAppliedCompactionV1 {
    pub schema_id: String,
    pub snapshot_id: String,
    pub snapshot_sha256: String,
    pub snapshot_state_vector: String,
    pub policy_id: String,
    pub compact_through_update_seq: u64,
    pub retained_updates: Vec<CrdtUpdateRecordV1>,
    pub compacted_update_audits: Vec<CrdtCompactedUpdateAuditV1>,
    pub final_state_vector: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtSnapshotRecordValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtSnapshotReplayError {
    InvalidSnapshot {
        errors: Vec<CrdtSnapshotRecordValidationError>,
    },
    InvalidUpdate {
        update_id: String,
        errors: Vec<CrdtUpdateRecordValidationError>,
    },
    MixedIdentity {
        field: &'static str,
        expected: String,
        found: String,
    },
    DuplicateUpdateId {
        update_id: String,
    },
    DuplicateSequence {
        update_seq: u64,
    },
    SequenceGap {
        expected: u64,
        found: u64,
    },
    PromotionEvidenceWouldBeDropped {
        update_id: String,
    },
    CompactionWithoutAudit {
        update_id: String,
    },
    InvalidCompactionPlan {
        field: &'static str,
        message: String,
    },
    MissingCompactionDecision {
        update_id: String,
    },
    DuplicateCompactionDecision {
        update_id: String,
    },
}

pub fn new_crdt_snapshot_record(input: CrdtSnapshotRecordInputV1<'_>) -> CrdtSnapshotRecordV1 {
    CrdtSnapshotRecordV1 {
        schema_id: CRDT_SNAPSHOT_RECORD_SCHEMA_ID.to_string(),
        snapshot_id: input.snapshot_id.to_string(),
        workspace_id: input.identity.workspace_id.clone(),
        document_id: input.identity.document_id.clone(),
        crdt_document_id: input.identity.crdt_document_id.clone(),
        covered_update_seq: input.covered_update_seq,
        state_vector: input.state_vector.to_string(),
        snapshot_sha256: sha256_hex(input.snapshot_bytes),
        snapshot_bytes_ref: input.snapshot_bytes_ref.to_string(),
        actor_id: input.identity.actor_id.clone(),
        actor_kind: input.identity.actor_kind.clone(),
        event_ledger_stream_id: input
            .identity
            .authority_links
            .event_ledger_stream_id
            .clone(),
        event_ledger_event_id: input.event_ledger_event_id.to_string(),
        promotion_evidence_update_ids: input
            .promotion_evidence_update_ids
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        storage_authority: CrdtStorageAuthorityPosture::EmbeddedSurrealDb,
    }
}

pub fn validate_crdt_snapshot_record(
    snapshot: &CrdtSnapshotRecordV1,
) -> Result<(), Vec<CrdtSnapshotRecordValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &snapshot.schema_id);
    require_non_empty(&mut errors, "snapshot_id", &snapshot.snapshot_id);
    require_non_empty(&mut errors, "workspace_id", &snapshot.workspace_id);
    require_non_empty(&mut errors, "document_id", &snapshot.document_id);
    require_non_empty(&mut errors, "crdt_document_id", &snapshot.crdt_document_id);
    require_non_empty(&mut errors, "state_vector", &snapshot.state_vector);
    require_non_empty(&mut errors, "snapshot_sha256", &snapshot.snapshot_sha256);
    require_non_empty(
        &mut errors,
        "snapshot_bytes_ref",
        &snapshot.snapshot_bytes_ref,
    );
    require_non_empty(&mut errors, "actor_id", &snapshot.actor_id);
    require_non_empty(&mut errors, "actor_kind", &snapshot.actor_kind);
    require_non_empty(
        &mut errors,
        "event_ledger_stream_id",
        &snapshot.event_ledger_stream_id,
    );
    require_non_empty(
        &mut errors,
        "event_ledger_event_id",
        &snapshot.event_ledger_event_id,
    );

    if snapshot.schema_id != CRDT_SNAPSHOT_RECORD_SCHEMA_ID {
        errors.push(CrdtSnapshotRecordValidationError {
            field: "schema_id",
            message: "unexpected CRDT snapshot record schema",
        });
    }
    if !is_sha256_hex(&snapshot.snapshot_sha256) {
        errors.push(CrdtSnapshotRecordValidationError {
            field: "snapshot_sha256",
            message: "value must be a 64-character sha256 hex digest",
        });
    }
    if !snapshot.snapshot_bytes_ref.starts_with("surreal://") {
        errors.push(CrdtSnapshotRecordValidationError {
            field: "snapshot_bytes_ref",
            message: "CRDT snapshot bytes must be referenced from embedded SurrealDB storage",
        });
    }
    if snapshot.storage_authority != CrdtStorageAuthorityPosture::EmbeddedSurrealDb {
        errors.push(CrdtSnapshotRecordValidationError {
            field: "storage_authority",
            message: "CRDT snapshot authority must be embedded SurrealDB plus EventLedger",
        });
    }
    if snapshot
        .promotion_evidence_update_ids
        .iter()
        .any(|value| value.trim().is_empty())
    {
        errors.push(CrdtSnapshotRecordValidationError {
            field: "promotion_evidence_update_ids",
            message: "promotion evidence update ids must not be empty",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn build_snapshot_bounded_replay_plan(
    snapshot: &CrdtSnapshotRecordV1,
    updates: &[CrdtUpdateRecordV1],
) -> Result<CrdtBoundedReplayPlanV1, CrdtSnapshotReplayError> {
    validate_snapshot_and_updates(snapshot, updates)?;

    let mut ordered_updates: Vec<_> = updates
        .iter()
        .filter(|record| record.update_seq > snapshot.covered_update_seq)
        .cloned()
        .collect();
    ordered_updates.sort_by_key(|record| record.update_seq);

    let replay_from_update_seq = snapshot.covered_update_seq + 1;
    for (offset, record) in ordered_updates.iter().enumerate() {
        let expected = replay_from_update_seq + offset as u64;
        if record.update_seq != expected {
            return Err(CrdtSnapshotReplayError::SequenceGap {
                expected,
                found: record.update_seq,
            });
        }
    }

    let final_state_vector = ordered_updates
        .last()
        .map(|record| record.state_vector_after.clone())
        .unwrap_or_else(|| snapshot.state_vector.clone());

    Ok(CrdtBoundedReplayPlanV1 {
        schema_id: CRDT_BOUNDED_REPLAY_PLAN_SCHEMA_ID.to_string(),
        workspace_id: snapshot.workspace_id.clone(),
        document_id: snapshot.document_id.clone(),
        crdt_document_id: snapshot.crdt_document_id.clone(),
        base_snapshot_id: snapshot.snapshot_id.clone(),
        base_snapshot_state_vector: snapshot.state_vector.clone(),
        replay_from_update_seq,
        source_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
        ordered_updates: ordered_updates.into_iter().map(replay_step).collect(),
        final_state_vector,
    })
}

pub fn plan_crdt_compaction(
    snapshot: &CrdtSnapshotRecordV1,
    updates: &[CrdtUpdateRecordV1],
    policy: &CrdtCompactionPolicyV1,
) -> Result<CrdtCompactionPlanV1, CrdtSnapshotReplayError> {
    validate_snapshot_and_updates(snapshot, updates)?;

    let promotion_evidence: HashSet<_> = snapshot
        .promotion_evidence_update_ids
        .iter()
        .cloned()
        .collect();
    let mut ordered_updates = updates.to_vec();
    ordered_updates.sort_by_key(|record| record.update_seq);

    let mut decisions = Vec::new();
    for record in ordered_updates {
        let compactable = record.update_seq <= snapshot.covered_update_seq
            && record.update_seq <= policy.compact_through_update_seq;
        let disposition = if !compactable {
            CrdtCompactionDisposition::RetainForReplay
        } else if promotion_evidence.contains(&record.update_id) {
            if !policy.preserve_promotion_evidence {
                return Err(CrdtSnapshotReplayError::PromotionEvidenceWouldBeDropped {
                    update_id: record.update_id,
                });
            }
            CrdtCompactionDisposition::RetainPromotionEvidence
        } else {
            match policy.audit_mode {
                CrdtCompactionAuditMode::EventLedgerAuditRefs => {
                    CrdtCompactionDisposition::CompactWithAudit
                }
                CrdtCompactionAuditMode::RetainOnly => CrdtCompactionDisposition::RetainForAudit,
                CrdtCompactionAuditMode::DropWithoutAudit => {
                    return Err(CrdtSnapshotReplayError::CompactionWithoutAudit {
                        update_id: record.update_id,
                    });
                }
            }
        };

        decisions.push(CrdtCompactionDecisionV1 {
            audit_ref: format!(
                "eventledger://{}/{}",
                record.event_ledger_stream_id, record.event_ledger_event_id
            ),
            update_id: record.update_id,
            update_seq: record.update_seq,
            disposition,
        });
    }

    Ok(CrdtCompactionPlanV1 {
        schema_id: CRDT_COMPACTION_PLAN_SCHEMA_ID.to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        policy_id: policy.policy_id.clone(),
        compact_through_update_seq: policy.compact_through_update_seq,
        decisions,
    })
}

pub fn apply_crdt_compaction(
    snapshot: &CrdtSnapshotRecordV1,
    updates: &[CrdtUpdateRecordV1],
    plan: &CrdtCompactionPlanV1,
) -> Result<CrdtAppliedCompactionV1, CrdtSnapshotReplayError> {
    validate_snapshot_and_updates(snapshot, updates)?;
    if plan.schema_id != CRDT_COMPACTION_PLAN_SCHEMA_ID {
        return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
            field: "schema_id",
            message: "unexpected compaction plan schema".into(),
        });
    }
    if plan.snapshot_id != snapshot.snapshot_id {
        return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
            field: "snapshot_id",
            message: format!(
                "plan snapshot {} does not match {}",
                plan.snapshot_id, snapshot.snapshot_id
            ),
        });
    }

    let mut decisions = HashMap::with_capacity(plan.decisions.len());
    for decision in &plan.decisions {
        if decisions
            .insert(decision.update_id.as_str(), decision)
            .is_some()
        {
            return Err(CrdtSnapshotReplayError::DuplicateCompactionDecision {
                update_id: decision.update_id.clone(),
            });
        }
    }
    if decisions.len() != updates.len() {
        return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
            field: "decisions",
            message: format!(
                "plan has {} decisions for {} updates",
                decisions.len(),
                updates.len()
            ),
        });
    }

    let mut ordered_updates = updates.to_vec();
    ordered_updates.sort_by_key(|record| record.update_seq);
    let promotion_evidence = snapshot
        .promotion_evidence_update_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let mut retained_updates = Vec::new();
    let mut compacted_update_audits = Vec::new();
    for record in ordered_updates {
        let decision = decisions.get(record.update_id.as_str()).ok_or_else(|| {
            CrdtSnapshotReplayError::MissingCompactionDecision {
                update_id: record.update_id.clone(),
            }
        })?;
        if decision.update_seq != record.update_seq {
            return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
                field: "decisions[].update_seq",
                message: format!(
                    "decision for {} has seq {}, expected {}",
                    record.update_id, decision.update_seq, record.update_seq
                ),
            });
        }
        let expected_audit_ref = format!(
            "eventledger://{}/{}",
            record.event_ledger_stream_id, record.event_ledger_event_id
        );
        if decision.audit_ref != expected_audit_ref {
            return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
                field: "decisions[].audit_ref",
                message: format!("decision for {} has a forged audit ref", record.update_id),
            });
        }
        if decision.disposition == CrdtCompactionDisposition::CompactWithAudit {
            if promotion_evidence.contains(&record.update_id) {
                return Err(CrdtSnapshotReplayError::PromotionEvidenceWouldBeDropped {
                    update_id: record.update_id.clone(),
                });
            }
            if record.update_seq > snapshot.covered_update_seq
                || record.update_seq > plan.compact_through_update_seq
            {
                return Err(CrdtSnapshotReplayError::InvalidCompactionPlan {
                    field: "decisions[].disposition",
                    message: format!(
                        "decision attempts to compact replay-required update {}",
                        record.update_id
                    ),
                });
            }
            compacted_update_audits.push(CrdtCompactedUpdateAuditV1 {
                schema_id: CRDT_COMPACTED_UPDATE_AUDIT_SCHEMA_ID.into(),
                workspace_id: record.workspace_id,
                document_id: record.document_id,
                crdt_document_id: record.crdt_document_id,
                update_id: record.update_id,
                update_seq: record.update_seq,
                update_sha256: record.update_sha256,
                update_bytes_ref: record.update_bytes_ref,
                state_vector_before: record.state_vector_before,
                state_vector_after: record.state_vector_after,
                replay_metadata: record.replay_metadata,
                actor_id: record.actor_id,
                actor_kind: record.actor_kind,
                session_id: record.session_id,
                trace_id: record.trace_id,
                event_ledger_stream_id: record.event_ledger_stream_id,
                event_ledger_event_id: record.event_ledger_event_id,
                audit_ref: decision.audit_ref.clone(),
                storage_authority: record.storage_authority,
            });
        } else {
            retained_updates.push(record);
        }
    }

    let bounded_replay = build_snapshot_bounded_replay_plan(snapshot, &retained_updates)?;
    Ok(CrdtAppliedCompactionV1 {
        schema_id: CRDT_APPLIED_COMPACTION_SCHEMA_ID.into(),
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_sha256: snapshot.snapshot_sha256.clone(),
        snapshot_state_vector: snapshot.state_vector.clone(),
        policy_id: plan.policy_id.clone(),
        compact_through_update_seq: plan.compact_through_update_seq,
        retained_updates,
        compacted_update_audits,
        final_state_vector: bounded_replay.final_state_vector,
    })
}

fn validate_snapshot_and_updates(
    snapshot: &CrdtSnapshotRecordV1,
    updates: &[CrdtUpdateRecordV1],
) -> Result<(), CrdtSnapshotReplayError> {
    if let Err(errors) = validate_crdt_snapshot_record(snapshot) {
        return Err(CrdtSnapshotReplayError::InvalidSnapshot { errors });
    }

    let mut seen_update_ids = HashSet::new();
    let mut seen_sequences = HashSet::new();
    for record in updates {
        if let Err(errors) = validate_crdt_update_record(record) {
            return Err(CrdtSnapshotReplayError::InvalidUpdate {
                update_id: record.update_id.clone(),
                errors,
            });
        }
        require_same_identity("workspace_id", &snapshot.workspace_id, &record.workspace_id)?;
        require_same_identity("document_id", &snapshot.document_id, &record.document_id)?;
        require_same_identity(
            "crdt_document_id",
            &snapshot.crdt_document_id,
            &record.crdt_document_id,
        )?;

        if !seen_update_ids.insert(record.update_id.clone()) {
            return Err(CrdtSnapshotReplayError::DuplicateUpdateId {
                update_id: record.update_id.clone(),
            });
        }
        if !seen_sequences.insert(record.update_seq) {
            return Err(CrdtSnapshotReplayError::DuplicateSequence {
                update_seq: record.update_seq,
            });
        }
    }
    Ok(())
}

fn replay_step(record: CrdtUpdateRecordV1) -> CrdtReplayStepV1 {
    CrdtReplayStepV1 {
        update_id: record.update_id,
        update_seq: record.update_seq,
        update_sha256: record.update_sha256,
        update_bytes_ref: record.update_bytes_ref,
        actor_id: record.actor_id,
        actor_kind: record.actor_kind,
        session_id: record.session_id,
        trace_id: record.trace_id,
        state_vector_after: record.state_vector_after,
        event_ledger_event_id: record.event_ledger_event_id,
    }
}

fn require_same_identity(
    field: &'static str,
    expected: &str,
    found: &str,
) -> Result<(), CrdtSnapshotReplayError> {
    if expected == found {
        Ok(())
    } else {
        Err(CrdtSnapshotReplayError::MixedIdentity {
            field,
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_non_empty(
    errors: &mut Vec<CrdtSnapshotRecordValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(CrdtSnapshotRecordValidationError {
            field,
            message: "value must not be empty",
        });
    }
}
