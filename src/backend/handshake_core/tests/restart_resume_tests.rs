use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    session_checkpoint::{
        CheckpointStateKind, EventLedgerRow, ReplayError, RestartResumeOrchestrator,
        ResumableSession, ResumeError, ResumeReport, SessionCheckpoint,
    },
};
use serde_json::json;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
struct Candidate {
    session_id: Uuid,
    checkpoint: SessionCheckpoint,
    events: Vec<EventLedgerRow>,
    state: CandidateState,
    orphan_processes: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateState {
    Running,
    Resumed,
    RecoveryFailed,
}

struct HarnessBroker {
    candidates: Mutex<Vec<Candidate>>,
    order: Mutex<Vec<String>>,
    reports: Mutex<Vec<ResumeReport>>,
    decision_requests: Mutex<Vec<(Uuid, ResumeError)>>,
    fr_events: Mutex<Vec<FrEventId>>,
}

impl HarnessBroker {
    fn new(candidates: Vec<Candidate>) -> Self {
        Self {
            candidates: Mutex::new(candidates),
            order: Mutex::new(Vec::new()),
            reports: Mutex::new(Vec::new()),
            decision_requests: Mutex::new(Vec::new()),
            fr_events: Mutex::new(Vec::new()),
        }
    }

    fn decision_requests(&self) -> Vec<(Uuid, ResumeError)> {
        self.decision_requests.lock().unwrap().clone()
    }

    fn order(&self) -> Vec<String> {
        self.order.lock().unwrap().clone()
    }

    fn reports(&self) -> Vec<ResumeReport> {
        self.reports.lock().unwrap().clone()
    }

    fn fr_events(&self) -> Vec<FrEventId> {
        self.fr_events.lock().unwrap().clone()
    }
}

impl ResumableSession for HarnessBroker {
    type State = i64;

    fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)> {
        self.candidates
            .lock()
            .unwrap()
            .iter()
            .filter(|candidate| candidate.state == CandidateState::Running)
            .map(|candidate| {
                (
                    candidate.session_id,
                    candidate.checkpoint.clone(),
                    candidate.events.clone(),
                )
            })
            .collect()
    }

    fn apply_event(&self, state: &mut i64, event: &EventLedgerRow) -> Result<(), ReplayError> {
        let by = event.payload.get("by").and_then(|value| value.as_i64()).unwrap_or(0);
        *state += by;
        Ok(())
    }

    fn seed_state(&self, checkpoint: &SessionCheckpoint) -> i64 {
        checkpoint
            .compact_state
            .get("counter")
            .and_then(|value| value.as_i64())
            .unwrap_or(0)
    }

    fn reclaim_orphan_processes(&self, session_id: Uuid) -> Result<u32, String> {
        let orphan_processes = self
            .candidates
            .lock()
            .unwrap()
            .iter()
            .find(|candidate| candidate.session_id == session_id)
            .map(|candidate| candidate.orphan_processes)
            .unwrap_or(0);
        self.order
            .lock()
            .unwrap()
            .push(format!("reclaim:{session_id}:{orphan_processes}"));
        Ok(orphan_processes)
    }

    fn resume(&self, session_id: Uuid, _final_state: i64) -> Result<(), String> {
        self.order.lock().unwrap().push(format!("resume:{session_id}"));
        let mut candidates = self.candidates.lock().unwrap();
        let candidate = candidates
            .iter_mut()
            .find(|candidate| candidate.session_id == session_id)
            .ok_or_else(|| "missing session".to_string())?;
        candidate.state = CandidateState::Resumed;
        Ok(())
    }

    fn mark_recovery_failed(&self, session_id: Uuid, _error: &ResumeError) -> Result<(), String> {
        let mut candidates = self.candidates.lock().unwrap();
        let candidate = candidates
            .iter_mut()
            .find(|candidate| candidate.session_id == session_id)
            .ok_or_else(|| "missing session".to_string())?;
        candidate.state = CandidateState::RecoveryFailed;
        Ok(())
    }

    fn request_operator_decision(
        &self,
        session_id: Uuid,
        error: &ResumeError,
    ) -> Result<(), String> {
        self.decision_requests
            .lock()
            .unwrap()
            .push((session_id, error.clone()));
        Ok(())
    }

    fn persist_resume_report(&self, report: &ResumeReport) -> Result<(), String> {
        self.reports.lock().unwrap().push(report.clone());
        Ok(())
    }

    fn emit_restart_resume_event(&self, event_id: FrEventId, _payload: serde_json::Value) {
        self.fr_events.lock().unwrap().push(event_id);
    }
}

fn checkpoint(session_id: Uuid, last_seq: i64, counter: i64) -> SessionCheckpoint {
    SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        last_seq,
        json!({ "counter": counter }),
        CheckpointStateKind::Periodic,
    )
    .unwrap()
}

fn event(session_id: Uuid, seq: i64, by: i64) -> EventLedgerRow {
    EventLedgerRow {
        event_id: format!("evt-{seq}"),
        event_sequence: seq,
        session_id,
        event_type: "increment".to_string(),
        payload: json!({ "by": by }),
        created_at: chrono::Utc::now(),
    }
}

#[test]
fn mt193_persists_report_and_emits_typed_fr_events_on_success() {
    let session_id = Uuid::now_v7();
    let broker = HarnessBroker::new(vec![Candidate {
        session_id,
        checkpoint: checkpoint(session_id, 0, 10),
        events: vec![event(session_id, 1, 5), event(session_id, 2, 7)],
        state: CandidateState::Running,
        orphan_processes: 0,
    }]);

    let report = RestartResumeOrchestrator::run(&broker);

    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert_eq!(report.sessions_recovery_failed.len(), 0);
    assert_eq!(report.report_id.get_version_num(), 7);
    assert_eq!(broker.reports(), vec![report.clone()]);
    let fr_events = broker.fr_events();
    assert_eq!(fr_events.first(), Some(&FrEventId::RestartResumeStarted));
    assert!(fr_events.contains(&FrEventId::RestartResumeSessionResumed));
    assert_eq!(fr_events.last(), Some(&FrEventId::RestartResumeCompleted));
}

#[test]
fn mt193_recovery_failure_marks_state_and_posts_operator_decision_request() {
    let session_id = Uuid::now_v7();
    let broker = HarnessBroker::new(vec![Candidate {
        session_id,
        checkpoint: checkpoint(session_id, 0, 0),
        events: vec![event(session_id, 1, 5), event(session_id, 3, 7)],
        state: CandidateState::Running,
        orphan_processes: 0,
    }]);

    let report = RestartResumeOrchestrator::run(&broker);

    assert_eq!(report.sessions_resumed.len(), 0);
    assert_eq!(report.sessions_recovery_failed.len(), 1);
    assert_eq!(broker.decision_requests().len(), 1);
    assert_eq!(broker.decision_requests()[0].0, session_id);
    assert!(matches!(
        broker.decision_requests()[0].1,
        ResumeError::ReplayError(ReplayError::MissingEvent { gap_at_seq: 2 })
    ));
    assert!(broker
        .fr_events()
        .contains(&FrEventId::RestartResumeSessionRecoveryFailed));
    assert_eq!(broker.reports(), vec![report]);
}

#[test]
fn mt193_reclaims_orphans_before_resume_and_second_run_is_idempotent() {
    let session_id = Uuid::now_v7();
    let broker = HarnessBroker::new(vec![Candidate {
        session_id,
        checkpoint: checkpoint(session_id, 0, 0),
        events: vec![event(session_id, 1, 1)],
        state: CandidateState::Running,
        orphan_processes: 2,
    }]);

    let first = RestartResumeOrchestrator::run(&broker);
    let second = RestartResumeOrchestrator::run(&broker);

    assert_eq!(first.sessions_examined, 1);
    assert_eq!(first.sessions_resumed.len(), 1);
    assert_eq!(first.orphan_reclaims[0].processes_reclaimed, 2);
    assert_eq!(second.sessions_examined, 0);
    assert_eq!(second.sessions_resumed.len(), 0);
    let order = broker.order();
    assert_eq!(order[0], format!("reclaim:{session_id}:2"));
    assert_eq!(order[1], format!("resume:{session_id}"));
    assert_eq!(broker.reports().len(), 2);
}
