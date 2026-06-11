//! WP-KERNEL-009 MT-073 CRDTAndConcurrencyCore-073-OfflineDraftStateBoundary.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
//! "SQLite, embedded local relational stores, browser storage ... MUST NOT
//! satisfy ProjectKnowledgeIndex authority, replay, validation, cache, or
//! fixture requirements."
//!
//! Boundary: client-side draft state (the in-memory Yjs document, the open
//! editor tab) MAY exist transiently before promotion; the durable home of
//! draft state is PostgreSQL (`kernel_crdt_updates` + `kernel_crdt_snapshots`)
//! and nothing else. After a crash/offline period the client reconnects and
//! replays its buffered update envelopes through [`replay_offline_envelopes`]:
//! every update lands (no draft loss), resubmissions of already-stored
//! updates are idempotent, and authority surfaces (promoted facts, EventLedger
//! promotion events) are untouched by replay — replay is draft-only.
//!
//! The machine-readable contract below follows the
//! `kernel_crdt_postgres_update_log_contract` precedent so validators can
//! assert the boundary without parsing prose.

use serde::{Deserialize, Serialize};

use crate::storage::Database;

use super::yjs_bridge::{
    push_yjs_update, KnowledgeCrdtFlowError, YjsPushOutcomeV1, YjsUpdateEnvelopeV1,
};

pub const OFFLINE_DRAFT_BOUNDARY_CONTRACT_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_offline_draft_boundary_contract@1";
pub const OFFLINE_REPLAY_REPORT_SCHEMA_ID: &str = "hsk.kernel.knowledge_offline_replay_report@1";

/// Machine-readable offline draft-state boundary contract. (Serialize-only:
/// the contract is emitted by this module, never parsed back from JSON.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfflineDraftBoundaryContractV1 {
    pub schema_id: String,
    /// Client surfaces where transient (pre-durable) draft state MAY live.
    pub allowed_transient_surfaces: Vec<&'static str>,
    /// The ONLY durable draft authority surfaces.
    pub durable_draft_authority_tables: Vec<&'static str>,
    /// Surfaces that MUST NOT hold durable draft state or authority.
    pub denied_durable_surfaces: Vec<&'static str>,
    /// Ordered reconnect protocol a no-context client follows.
    pub reconnect_protocol: Vec<&'static str>,
    /// Replay MUST NOT touch these authority surfaces.
    pub replay_must_not_mutate: Vec<&'static str>,
}

pub fn knowledge_offline_draft_boundary_contract() -> OfflineDraftBoundaryContractV1 {
    OfflineDraftBoundaryContractV1 {
        schema_id: OFFLINE_DRAFT_BOUNDARY_CONTRACT_SCHEMA_ID.to_string(),
        allowed_transient_surfaces: vec![
            "in_memory_yjs_document",
            "open_editor_tab_state",
            "in_flight_update_buffer",
        ],
        durable_draft_authority_tables: vec!["kernel_crdt_updates", "kernel_crdt_snapshots"],
        denied_durable_surfaces: vec![
            "browser_local_storage",
            "browser_indexed_db_authority",
            "sqlite_file",
            "filesystem_draft_files",
            "markdown_mirror",
            "provider_chat_history",
        ],
        reconnect_protocol: vec![
            "pull_yjs_updates(since_update_seq = last acknowledged seq)",
            "merge pulled updates into the local Yjs document (client-side CRDT merge)",
            "rebase buffered local updates onto the new head state vector",
            "replay_offline_envelopes(buffered envelopes, in causal order)",
            "verify head_state_vector matches the local materialized vector",
        ],
        replay_must_not_mutate: vec![
            "knowledge_crdt_promoted_facts",
            "kernel_event_ledger promotion decisions",
            "knowledge_crdt_graph_proposals review_state",
            "knowledge_crdt_ai_edit_proposals review_state",
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineBoundaryContractError {
    pub field: &'static str,
    pub message: &'static str,
}

/// Validator hook: the contract must keep PostgreSQL as the only durable
/// draft authority and must never allow a denied surface to appear in the
/// allowed/durable lists.
pub fn validate_offline_draft_boundary_contract(
    contract: &OfflineDraftBoundaryContractV1,
) -> Result<(), Vec<OfflineBoundaryContractError>> {
    let mut errors = Vec::new();
    if contract.schema_id != OFFLINE_DRAFT_BOUNDARY_CONTRACT_SCHEMA_ID {
        errors.push(OfflineBoundaryContractError {
            field: "schema_id",
            message: "unexpected offline boundary contract schema",
        });
    }
    if contract.durable_draft_authority_tables
        != vec!["kernel_crdt_updates", "kernel_crdt_snapshots"]
    {
        errors.push(OfflineBoundaryContractError {
            field: "durable_draft_authority_tables",
            message: "durable draft authority must be exactly the Postgres CRDT tables",
        });
    }
    for denied in &contract.denied_durable_surfaces {
        if contract.allowed_transient_surfaces.contains(denied)
            || contract.durable_draft_authority_tables.contains(denied)
        {
            errors.push(OfflineBoundaryContractError {
                field: "denied_durable_surfaces",
                message: "a denied surface appears in an allowed list",
            });
        }
    }
    if contract.reconnect_protocol.is_empty() {
        errors.push(OfflineBoundaryContractError {
            field: "reconnect_protocol",
            message: "reconnect protocol must not be empty",
        });
    }
    if contract.replay_must_not_mutate.is_empty() {
        errors.push(OfflineBoundaryContractError {
            field: "replay_must_not_mutate",
            message: "replay no-mutate list must not be empty",
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Per-envelope replay verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum OfflineReplayVerdictV1 {
    Stored { update_id: String, update_seq: u64 },
    AlreadyStored { update_id: String, update_seq: u64 },
    Denied { update_id: String, reason: String },
}

/// Result of replaying a reconnecting client's buffered envelopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineReplayReportV1 {
    pub schema_id: String,
    pub crdt_document_id: String,
    pub verdicts: Vec<OfflineReplayVerdictV1>,
    pub stored_count: usize,
    pub already_stored_count: usize,
    pub denied_count: usize,
    pub head_state_vector: String,
}

/// Replay buffered offline envelopes in order. Stops semantics: every
/// envelope gets a verdict (no silent drops); denials carry the typed
/// reason so the client can rebase and resubmit exactly the failed tail.
pub async fn replay_offline_envelopes(
    db: &(dyn Database + '_),
    envelopes: &[YjsUpdateEnvelopeV1],
) -> Result<OfflineReplayReportV1, KnowledgeCrdtFlowError> {
    let crdt_document_id = envelopes
        .first()
        .map(|envelope| envelope.crdt_document_id.clone())
        .unwrap_or_default();
    let mut verdicts = Vec::with_capacity(envelopes.len());
    let mut head_state_vector = String::new();
    let mut stored_count = 0;
    let mut already_stored_count = 0;
    let mut denied_count = 0;

    for envelope in envelopes {
        match push_yjs_update(db, envelope).await? {
            YjsPushOutcomeV1::Stored {
                update_seq,
                update_id,
                head_state_vector: head,
                ..
            } => {
                stored_count += 1;
                head_state_vector = head;
                verdicts.push(OfflineReplayVerdictV1::Stored {
                    update_id,
                    update_seq,
                });
            }
            YjsPushOutcomeV1::AlreadyStored {
                update_seq,
                update_id,
                head_state_vector: head,
                ..
            } => {
                already_stored_count += 1;
                head_state_vector = head;
                verdicts.push(OfflineReplayVerdictV1::AlreadyStored {
                    update_id,
                    update_seq,
                });
            }
            YjsPushOutcomeV1::Denied { denial } => {
                denied_count += 1;
                verdicts.push(OfflineReplayVerdictV1::Denied {
                    update_id: denial.update_id.clone(),
                    reason: serde_json::to_string(&denial.reason)
                        .unwrap_or_else(|_| "unserializable denial".to_string()),
                });
            }
        }
    }

    Ok(OfflineReplayReportV1 {
        schema_id: OFFLINE_REPLAY_REPORT_SCHEMA_ID.to_string(),
        crdt_document_id,
        verdicts,
        stored_count,
        already_stored_count,
        denied_count,
        head_state_vector,
    })
}
