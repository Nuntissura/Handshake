//! KB003 schema-id constants and shared event envelope.
//!
//! MT-007 declares stable schema IDs for the seven WP-KERNEL-003 record types
//! that the EventLedger, sandbox runner, validation runner, and promotion gate
//! exchange. MT-008 adds the EventLedger event-type constants plus the common
//! envelope shape every KB003 event carries (run_id, actor, session, task,
//! schema_version, timestamp, artifact_refs).
//!
//! Schema-id format follows the repo-local `hsk.kernel.<record>@<version>`
//! convention already used by CRDT/FEMS/DCC records (see
//! `hsk.kernel.crdt_snapshot_record@1` etc. in existing kernel tests).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// MT-007: KB003 record schema-id constants
// ---------------------------------------------------------------------------

pub const SCHEMA_KERNEL_SANDBOX_RUN_V1: &str = "hsk.kernel.sandbox_run@1";
pub const SCHEMA_KERNEL_SANDBOX_POLICY_V1: &str = "hsk.kernel.sandbox_policy@1";
pub const SCHEMA_KERNEL_SANDBOX_WORKSPACE_V1: &str = "hsk.kernel.sandbox_workspace@1";
pub const SCHEMA_KERNEL_SANDBOX_ARTIFACT_BUNDLE_V1: &str = "hsk.kernel.sandbox_artifact_bundle@1";
pub const SCHEMA_KERNEL_VALIDATION_RUN_V1: &str = "hsk.kernel.validation_run@1";
pub const SCHEMA_KERNEL_PROMOTION_DECISION_V1: &str = "hsk.kernel.promotion_decision@1";
pub const SCHEMA_KERNEL_PROMOTION_RECEIPT_V1: &str = "hsk.kernel.promotion_receipt@1";

pub const KB003_SCHEMA_IDS: &[&str] = &[
    SCHEMA_KERNEL_SANDBOX_RUN_V1,
    SCHEMA_KERNEL_SANDBOX_POLICY_V1,
    SCHEMA_KERNEL_SANDBOX_WORKSPACE_V1,
    SCHEMA_KERNEL_SANDBOX_ARTIFACT_BUNDLE_V1,
    SCHEMA_KERNEL_VALIDATION_RUN_V1,
    SCHEMA_KERNEL_PROMOTION_DECISION_V1,
    SCHEMA_KERNEL_PROMOTION_RECEIPT_V1,
];

// ---------------------------------------------------------------------------
// MT-008: EventLedger event-type constants for KB003 lifecycle
// ---------------------------------------------------------------------------

pub const EVENT_KB003_SANDBOX_RUN_REQUESTED: &str = "kb003.sandbox_run.requested";
pub const EVENT_KB003_SANDBOX_RUN_STARTED: &str = "kb003.sandbox_run.started";
pub const EVENT_KB003_SANDBOX_RUN_COMPLETED: &str = "kb003.sandbox_run.completed";
pub const EVENT_KB003_SANDBOX_RUN_REJECTED: &str = "kb003.sandbox_run.rejected";
pub const EVENT_KB003_VALIDATION_RUN_COMPLETED: &str = "kb003.validation_run.completed";
pub const EVENT_KB003_PROMOTION_DECIDED: &str = "kb003.promotion.decided";
pub const EVENT_KB003_PROMOTION_RECEIPT_ISSUED: &str = "kb003.promotion.receipt_issued";
pub const EVENT_KB003_PROMOTION_REJECTED: &str = "kb003.promotion.rejected";

pub const KB003_EVENT_TYPES: &[&str] = &[
    EVENT_KB003_SANDBOX_RUN_REQUESTED,
    EVENT_KB003_SANDBOX_RUN_STARTED,
    EVENT_KB003_SANDBOX_RUN_COMPLETED,
    EVENT_KB003_SANDBOX_RUN_REJECTED,
    EVENT_KB003_VALIDATION_RUN_COMPLETED,
    EVENT_KB003_PROMOTION_DECIDED,
    EVENT_KB003_PROMOTION_RECEIPT_ISSUED,
    EVENT_KB003_PROMOTION_REJECTED,
];

/// Common envelope every KB003 EventLedger event carries.
///
/// Field set required by MT-008 acceptance: run_id, actor, session, task,
/// schema_version, timestamp, artifact_refs. Wraps a typed payload by the
/// owning module (sandbox / validation / promotion) so this envelope stays
/// payload-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kb003EventEnvelope {
    pub event_type: String,
    pub schema_version: String,
    pub event_id: Uuid,
    pub run_id: Uuid,
    pub actor_id: String,
    pub session_id: String,
    pub task_id: String,
    pub timestamp_utc: String,
    pub artifact_refs: Vec<String>,
}

impl Kb003EventEnvelope {
    pub fn new(
        event_type: &str,
        run_id: Uuid,
        actor: &str,
        session: &str,
        task: &str,
        ts_utc: &str,
    ) -> Self {
        Self {
            event_type: event_type.to_string(),
            schema_version: "kb003_event_envelope_v1".to_string(),
            event_id: Uuid::now_v7(),
            run_id,
            actor_id: actor.to_string(),
            session_id: session.to_string(),
            task_id: task.to_string(),
            timestamp_utc: ts_utc.to_string(),
            artifact_refs: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_ids_are_versioned_and_namespaced() {
        for id in KB003_SCHEMA_IDS {
            assert!(
                id.starts_with("hsk.kernel."),
                "schema id {id} missing repo namespace"
            );
            assert!(id.contains('@'), "schema id {id} missing version separator");
            let (_, version) = id.rsplit_once('@').unwrap();
            assert!(!version.is_empty(), "schema id {id} has empty version");
        }
    }

    #[test]
    fn event_types_are_kb003_namespaced() {
        for ev in KB003_EVENT_TYPES {
            assert!(
                ev.starts_with("kb003."),
                "event type {ev} missing kb003 namespace"
            );
        }
    }

    #[test]
    fn envelope_carries_required_mt008_fields() {
        let run_id = Uuid::now_v7();
        let env = Kb003EventEnvelope::new(
            EVENT_KB003_SANDBOX_RUN_REQUESTED,
            run_id,
            "actor_kb",
            "ses_test",
            "task_test",
            "2026-05-17T00:00:00Z",
        );
        assert_eq!(env.event_type, EVENT_KB003_SANDBOX_RUN_REQUESTED);
        assert_eq!(env.run_id, run_id);
        assert_eq!(env.actor_id, "actor_kb");
        assert_eq!(env.session_id, "ses_test");
        assert_eq!(env.task_id, "task_test");
        assert!(!env.schema_version.is_empty());
        assert!(!env.timestamp_utc.is_empty());
        assert!(env.artifact_refs.is_empty());
        // Serializes cleanly.
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("event_type"));
        assert!(json.contains("artifact_refs"));
    }
}
