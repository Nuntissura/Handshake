//! MT-195 Crash recovery integration test harness module.
//!
//! Provides a `CrashRecoveryHarness` that drives one failure-mode scenario at
//! a time. The full Postgres-backed harness lives in
//! `tests/crash_recovery_e2e_postgres_tests.rs` and is `#[ignore]`-gated on
//! `POSTGRES_TEST_URL`. This module owns the failure-mode taxonomy and a
//! deterministic in-process simulator suitable for unit-level coverage.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

use super::checkpoint::{CheckpointStateKind, SessionCheckpoint};
use super::replay::{EventLedgerRow, ReplayError};
use super::restart::{RestartResumeOrchestrator, ResumableSession, ResumeReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashRecoveryScenario {
    CleanShutdown,
    SigkillMidIteration,
    PostgresLoss,
    OrphanProcess,
    EventSeqGap,
    IdempotencyConflict,
    OperatorCancelDuringRecovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryEvidence {
    pub scenario: CrashRecoveryScenario,
    pub report: ResumeReport,
}

pub struct CrashRecoveryHarness {
    pub scenario: CrashRecoveryScenario,
}

struct HarnessBroker {
    sessions: Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)>,
    resumed: Mutex<Vec<Uuid>>,
}

impl ResumableSession for HarnessBroker {
    type State = serde_json::Value;
    fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)> {
        self.sessions.clone()
    }
    fn apply_event(
        &self,
        _state: &mut serde_json::Value,
        _event: &EventLedgerRow,
    ) -> Result<(), ReplayError> {
        Ok(())
    }
    fn seed_state(&self, _cp: &SessionCheckpoint) -> serde_json::Value {
        serde_json::json!({})
    }
    fn resume(&self, session_id: Uuid, _final_state: serde_json::Value) -> Result<(), String> {
        self.resumed.lock().unwrap().push(session_id);
        Ok(())
    }
}

impl CrashRecoveryHarness {
    pub fn new(scenario: CrashRecoveryScenario) -> Self {
        Self { scenario }
    }

    /// Run the scenario through the in-process simulator and return the
    /// recovery evidence. Real Postgres-backed scenarios run via the
    /// `crash_recovery_e2e_postgres_tests` suite.
    pub fn simulate(&self) -> RecoveryEvidence {
        let broker = self.build_broker_for_scenario();
        let report = RestartResumeOrchestrator::run(&broker);
        RecoveryEvidence {
            scenario: self.scenario,
            report,
        }
    }

    fn build_broker_for_scenario(&self) -> HarnessBroker {
        let s = Uuid::now_v7();
        let cp = SessionCheckpoint::new(
            s,
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let events = match self.scenario {
            CrashRecoveryScenario::CleanShutdown => vec![event(s, 1), event(s, 2)],
            CrashRecoveryScenario::SigkillMidIteration => vec![event(s, 1)],
            CrashRecoveryScenario::PostgresLoss => vec![],
            CrashRecoveryScenario::OrphanProcess => vec![event(s, 1)],
            CrashRecoveryScenario::EventSeqGap => vec![event(s, 1), event(s, 3)],
            CrashRecoveryScenario::IdempotencyConflict => vec![event(s, 1)],
            CrashRecoveryScenario::OperatorCancelDuringRecovery => vec![event(s, 1)],
        };
        HarnessBroker {
            sessions: vec![(s, cp, events)],
            resumed: Mutex::new(Vec::new()),
        }
    }
}

fn event(session: Uuid, seq: i64) -> EventLedgerRow {
    EventLedgerRow {
        event_id: format!("E-{seq}"),
        event_sequence: seq,
        session_id: session,
        event_type: "noop".to_string(),
        payload: serde_json::Value::Null,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_shutdown_resumes_all() {
        let h = CrashRecoveryHarness::new(CrashRecoveryScenario::CleanShutdown);
        let ev = h.simulate();
        assert_eq!(ev.report.sessions_resumed.len(), 1);
        assert!(ev.report.sessions_recovery_failed.is_empty());
    }

    #[test]
    fn event_seq_gap_marks_recovery_failed() {
        let h = CrashRecoveryHarness::new(CrashRecoveryScenario::EventSeqGap);
        let ev = h.simulate();
        assert!(!ev.report.sessions_recovery_failed.is_empty());
    }
}
