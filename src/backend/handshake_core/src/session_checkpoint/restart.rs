//! MT-193 Restart resume orchestrator integrating with KERNEL-001 SessionBroker.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::fr_event_registry::FrEventId;

use super::checkpoint::SessionCheckpoint;
use super::replay::{EventLedgerRow, ReplayError, StateReplayer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResumeError {
    ReplayError(ReplayError),
    SessionApplyError { reason: String },
    NoCheckpoint,
    OrphanProcessReclaim { details: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumedSessionInfo {
    pub session_id: Uuid,
    pub events_applied: u32,
    pub final_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanReclaimInfo {
    pub session_id: Uuid,
    pub processes_reclaimed: u32,
    pub reclaimed_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDecisionRequest {
    pub session_id: Uuid,
    pub reason: ResumeError,
    pub options: Vec<String>,
    pub requested_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeReport {
    pub report_id: Uuid,
    pub sessions_examined: u32,
    pub sessions_resumed: Vec<ResumedSessionInfo>,
    pub sessions_recovery_failed: Vec<(Uuid, ResumeError)>,
    pub orphan_reclaims: Vec<OrphanReclaimInfo>,
    pub operator_decision_requests: Vec<OperatorDecisionRequest>,
    pub fr_events_emitted: Vec<String>,
    pub total_replay_events: u64,
    pub total_duration_ms: u64,
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum RestartError {
    #[error("internal: {0}")]
    Internal(String),
}

/// SessionBroker integration trait. KERNEL-001 will implement this.
pub trait ResumableSession: Send + Sync {
    type State: Clone + serde::Serialize;
    fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)>;
    fn apply_event(
        &self,
        state: &mut Self::State,
        event: &EventLedgerRow,
    ) -> Result<(), ReplayError>;
    fn seed_state(&self, checkpoint: &SessionCheckpoint) -> Self::State;
    fn reclaim_orphan_processes(&self, _session_id: Uuid) -> Result<u32, String> {
        Ok(0)
    }
    fn resume(&self, session_id: Uuid, final_state: Self::State) -> Result<(), String>;
    fn mark_recovery_failed(&self, _session_id: Uuid, _error: &ResumeError) -> Result<(), String> {
        Ok(())
    }
    fn request_operator_decision(
        &self,
        _session_id: Uuid,
        _error: &ResumeError,
    ) -> Result<(), String> {
        Ok(())
    }
    fn persist_resume_report(&self, _report: &ResumeReport) -> Result<(), String> {
        Ok(())
    }
    fn emit_restart_resume_event(&self, _event_id: FrEventId, _payload: Value) {}
}

pub struct RestartResumeOrchestrator;

impl RestartResumeOrchestrator {
    pub fn run<S: ResumableSession>(broker: &S) -> ResumeReport {
        let started_at_utc = Utc::now();
        let started = std::time::Instant::now();
        let candidates = broker.list_resumable_sessions();
        let mut report = ResumeReport {
            report_id: Uuid::now_v7(),
            sessions_examined: candidates.len() as u32,
            sessions_resumed: Vec::new(),
            sessions_recovery_failed: Vec::new(),
            orphan_reclaims: Vec::new(),
            operator_decision_requests: Vec::new(),
            fr_events_emitted: Vec::new(),
            total_replay_events: 0,
            total_duration_ms: 0,
            started_at_utc,
            completed_at_utc: Utc::now(),
        };
        let started_payload = json!({
            "report_id": report.report_id,
            "sessions_examined": report.sessions_examined,
            "started_at_utc": report.started_at_utc,
        });
        Self::emit_event(
            broker,
            &mut report,
            FrEventId::RestartResumeStarted,
            started_payload,
        );
        for (session_id, checkpoint, events) in candidates {
            match broker.reclaim_orphan_processes(session_id) {
                Ok(processes_reclaimed) => {
                    report.orphan_reclaims.push(OrphanReclaimInfo {
                        session_id,
                        processes_reclaimed,
                        reclaimed_at_utc: Utc::now(),
                    });
                }
                Err(details) => {
                    Self::record_failure(
                        broker,
                        &mut report,
                        session_id,
                        ResumeError::OrphanProcessReclaim { details },
                    );
                    continue;
                }
            }

            let plan = StateReplayer::plan(checkpoint.clone(), &events);
            let state = broker.seed_state(&checkpoint);
            match StateReplayer::execute(plan, state, |st, ev| broker.apply_event(st, ev)) {
                Ok(result) => match broker.resume(session_id, result.final_state) {
                    Ok(()) => {
                        report.sessions_resumed.push(ResumedSessionInfo {
                            session_id,
                            events_applied: result.applied_count,
                            final_seq: result.final_seq,
                        });
                        report.total_replay_events += result.applied_count as u64;
                        let session_resumed_payload = json!({
                            "report_id": report.report_id,
                            "session_id": session_id,
                            "events_applied": result.applied_count,
                            "final_seq": result.final_seq,
                        });
                        Self::emit_event(
                            broker,
                            &mut report,
                            FrEventId::RestartResumeSessionResumed,
                            session_resumed_payload,
                        );
                    }
                    Err(reason) => {
                        Self::record_failure(
                            broker,
                            &mut report,
                            session_id,
                            ResumeError::SessionApplyError { reason },
                        );
                    }
                },
                Err(e) => {
                    Self::record_failure(
                        broker,
                        &mut report,
                        session_id,
                        ResumeError::ReplayError(e),
                    );
                }
            }
        }
        report.total_duration_ms = started.elapsed().as_millis() as u64;
        report.completed_at_utc = Utc::now();
        let completed_payload = json!({
            "report_id": report.report_id,
            "sessions_examined": report.sessions_examined,
            "sessions_resumed": report.sessions_resumed.len(),
            "sessions_recovery_failed": report.sessions_recovery_failed.len(),
            "total_replay_events": report.total_replay_events,
            "total_duration_ms": report.total_duration_ms,
            "completed_at_utc": report.completed_at_utc,
        });
        Self::emit_event(
            broker,
            &mut report,
            FrEventId::RestartResumeCompleted,
            completed_payload,
        );
        let _ = broker.persist_resume_report(&report);
        report
    }

    fn record_failure<S: ResumableSession>(
        broker: &S,
        report: &mut ResumeReport,
        session_id: Uuid,
        error: ResumeError,
    ) {
        report
            .sessions_recovery_failed
            .push((session_id, error.clone()));
        let _ = broker.mark_recovery_failed(session_id, &error);
        if broker.request_operator_decision(session_id, &error).is_ok() {
            report
                .operator_decision_requests
                .push(OperatorDecisionRequest {
                    session_id,
                    reason: error.clone(),
                    options: vec![
                        "cancel_session".to_string(),
                        "manual_repair_then_retry".to_string(),
                        "retry_recovery".to_string(),
                    ],
                    requested_at_utc: Utc::now(),
                });
        }
        let failure_payload = json!({
            "report_id": report.report_id,
            "session_id": session_id,
            "error": error,
        });
        Self::emit_event(
            broker,
            report,
            FrEventId::RestartResumeSessionRecoveryFailed,
            failure_payload,
        );
    }

    fn emit_event<S: ResumableSession>(
        broker: &S,
        report: &mut ResumeReport,
        event_id: FrEventId,
        payload: Value,
    ) {
        broker.emit_restart_resume_event(event_id, payload);
        report.fr_events_emitted.push(event_id.as_str().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_checkpoint::checkpoint::CheckpointStateKind;
    use std::sync::Mutex;
    struct MtCounter {
        sessions: Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)>,
        resumed: Mutex<Vec<Uuid>>,
    }

    impl ResumableSession for MtCounter {
        type State = i64;
        fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)> {
            self.sessions.clone()
        }
        fn apply_event(&self, state: &mut i64, event: &EventLedgerRow) -> Result<(), ReplayError> {
            let by = event
                .payload
                .get("by")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            *state += by;
            Ok(())
        }
        fn seed_state(&self, _cp: &SessionCheckpoint) -> i64 {
            0
        }
        fn resume(&self, session_id: Uuid, _final_state: i64) -> Result<(), String> {
            self.resumed.lock().unwrap().push(session_id);
            Ok(())
        }
    }

    fn make_event(session: Uuid, seq: i64, by: i64) -> EventLedgerRow {
        EventLedgerRow {
            event_id: format!("E-{seq}"),
            event_sequence: seq,
            session_id: session,
            event_type: "inc".to_string(),
            payload: serde_json::json!({"by": by}),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn resume_3_mid_run_sessions() {
        let s1 = Uuid::now_v7();
        let s2 = Uuid::now_v7();
        let s3 = Uuid::now_v7();
        let cp1 = SessionCheckpoint::new(
            s1,
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let cp2 = SessionCheckpoint::new(
            s2,
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let cp3 = SessionCheckpoint::new(
            s3,
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let sessions = vec![
            (s1, cp1, vec![make_event(s1, 1, 10), make_event(s1, 2, 20)]),
            (s2, cp2, vec![make_event(s2, 1, 5)]),
            (s3, cp3, vec![]),
        ];
        let broker = MtCounter {
            sessions,
            resumed: Mutex::new(Vec::new()),
        };
        let report = RestartResumeOrchestrator::run(&broker);
        assert_eq!(report.sessions_examined, 3);
        assert_eq!(report.sessions_resumed.len(), 3);
        assert!(report.sessions_recovery_failed.is_empty());
    }

    #[test]
    fn gap_detection_marks_recovery_failed() {
        let s1 = Uuid::now_v7();
        let cp1 = SessionCheckpoint::new(
            s1,
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let sessions = vec![(s1, cp1, vec![make_event(s1, 1, 10), make_event(s1, 3, 30)])];
        let broker = MtCounter {
            sessions,
            resumed: Mutex::new(Vec::new()),
        };
        let report = RestartResumeOrchestrator::run(&broker);
        assert_eq!(report.sessions_recovery_failed.len(), 1);
    }
}
