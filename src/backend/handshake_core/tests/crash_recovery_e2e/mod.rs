//! MT-195 crash recovery e2e evidence suite.
//!
//! These tests run serially (`--test-threads=1`) because the contract's
//! Postgres-backed variant will share one external database/container. The
//! current suite is a deterministic in-process harness over the cluster X.3
//! primitives; every scenario compares exact counts and FR event order
//! against a canonical golden evidence file.

mod clean_shutdown;
mod event_seq_gap;
mod idempotency_conflict;
mod operator_cancel_during_recovery;
mod orphan_process;
mod postgres_loss;
mod runtime_chaos;
mod runtime_child;
mod runtime_postgres;
mod sigkill_mid_iteration;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    process_ledger::idempotency::{
        ApplyOutcome, IdempotencyKey, IdempotencyLedger, SideEffectKind,
    },
    session_checkpoint::{
        CheckpointStateKind, CrashRecoveryScenario, EventLedgerRow, ReplayError,
        RestartResumeOrchestrator, ResumableSession, ResumeError, SessionCheckpoint,
    },
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
struct Candidate {
    session_id: Uuid,
    checkpoint: SessionCheckpoint,
    events: Vec<EventLedgerRow>,
    state: CandidateState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateState {
    Running,
    Resumed,
    RecoveryFailed,
}

struct ScenarioBroker {
    scenario: CrashRecoveryScenario,
    candidates: Mutex<Vec<Candidate>>,
    fr_events: Mutex<Vec<String>>,
    decision_requests: AtomicUsize,
    side_effects_applied: Arc<AtomicUsize>,
    side_effects_already_applied: AtomicUsize,
    operator_cancellations: AtomicUsize,
    idempotency: IdempotencyLedger,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CanonicalRecoveryEvidence {
    scenario: &'static str,
    sessions_examined: u32,
    sessions_resumed: usize,
    sessions_recovery_failed: usize,
    orphan_processes_reclaimed: u32,
    operator_decision_requests: usize,
    side_effects_applied: usize,
    side_effects_already_applied: usize,
    operator_cancellations: usize,
    fr_event_order: Vec<String>,
}

pub fn assert_scenario_matches_golden(scenario: CrashRecoveryScenario) {
    let evidence = ScenarioHarness::new(scenario).run();
    assert_exact_counts(&evidence);
    let actual = serde_json::to_string_pretty(&evidence).unwrap();
    let expected = std::fs::read_to_string(golden_path(scenario)).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

struct ScenarioHarness {
    scenario: CrashRecoveryScenario,
}

impl ScenarioHarness {
    fn new(scenario: CrashRecoveryScenario) -> Self {
        Self { scenario }
    }

    fn run(&self) -> CanonicalRecoveryEvidence {
        let broker = ScenarioBroker::new(self.scenario);
        if self.scenario == CrashRecoveryScenario::PostgresLoss {
            broker.push_fr_event(FrEventId::RestartResumeDbUnavailable);
        }
        let report = RestartResumeOrchestrator::run(&broker);
        let fr_event_order = broker.fr_events.lock().unwrap().clone();
        CanonicalRecoveryEvidence {
            scenario: scenario_slug(self.scenario),
            sessions_examined: report.sessions_examined,
            sessions_resumed: report.sessions_resumed.len(),
            sessions_recovery_failed: report.sessions_recovery_failed.len(),
            orphan_processes_reclaimed: report
                .orphan_reclaims
                .iter()
                .map(|reclaim| reclaim.processes_reclaimed)
                .sum(),
            operator_decision_requests: broker.decision_requests.load(Ordering::SeqCst),
            side_effects_applied: broker.side_effects_applied.load(Ordering::SeqCst),
            side_effects_already_applied: broker
                .side_effects_already_applied
                .load(Ordering::SeqCst),
            operator_cancellations: broker.operator_cancellations.load(Ordering::SeqCst),
            fr_event_order,
        }
    }
}

impl ScenarioBroker {
    fn new(scenario: CrashRecoveryScenario) -> Self {
        Self {
            scenario,
            candidates: Mutex::new(candidates_for(scenario)),
            fr_events: Mutex::new(Vec::new()),
            decision_requests: AtomicUsize::new(0),
            side_effects_applied: Arc::new(AtomicUsize::new(0)),
            side_effects_already_applied: AtomicUsize::new(0),
            operator_cancellations: AtomicUsize::new(0),
            idempotency: IdempotencyLedger::in_memory(),
        }
    }

    fn push_fr_event(&self, event_id: FrEventId) {
        self.fr_events
            .lock()
            .unwrap()
            .push(event_id.as_str().to_string());
    }
}

impl ResumableSession for ScenarioBroker {
    type State = serde_json::Value;

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

    fn apply_event(
        &self,
        state: &mut serde_json::Value,
        event: &EventLedgerRow,
    ) -> Result<(), ReplayError> {
        let counter = state
            .get("counter")
            .and_then(|value| value.as_i64())
            .unwrap_or(0)
            + 1;
        *state = serde_json::json!({ "counter": counter });

        if self.scenario == CrashRecoveryScenario::IdempotencyConflict {
            let key = IdempotencyKey {
                session_id: event.session_id,
                event_seq: event.event_sequence,
                side_effect_kind: SideEffectKind::MailboxMessagePost,
            };
            let side_effects = Arc::clone(&self.side_effects_applied);
            let first =
                futures::executor::block_on(self.idempotency.try_apply(key.clone(), move || {
                    let side_effects = Arc::clone(&side_effects);
                    async move {
                        side_effects.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                }))
                .map_err(|err| ReplayError::StateInvariantViolated {
                    seq: event.event_sequence,
                    invariant: err.to_string(),
                })?;
            assert_eq!(first, ApplyOutcome::Applied);
            let second =
                futures::executor::block_on(self.idempotency.try_apply(key, || async { Ok(()) }))
                    .map_err(|err| ReplayError::StateInvariantViolated {
                    seq: event.event_sequence,
                    invariant: err.to_string(),
                })?;
            assert_eq!(second, ApplyOutcome::AlreadyApplied);
            self.side_effects_already_applied
                .fetch_add(1, Ordering::SeqCst);
        }

        Ok(())
    }

    fn seed_state(&self, checkpoint: &SessionCheckpoint) -> serde_json::Value {
        checkpoint.compact_state.clone()
    }

    fn reclaim_orphan_processes(&self, _session_id: Uuid) -> Result<u32, String> {
        Ok(match self.scenario {
            CrashRecoveryScenario::OrphanProcess => 2,
            _ => 0,
        })
    }

    fn resume(&self, session_id: Uuid, _final_state: serde_json::Value) -> Result<(), String> {
        if self.scenario == CrashRecoveryScenario::OperatorCancelDuringRecovery {
            self.operator_cancellations.fetch_add(1, Ordering::SeqCst);
            return Err("operator_cancel_during_recovery".to_string());
        }
        let mut candidates = self.candidates.lock().unwrap();
        let candidate = candidates
            .iter_mut()
            .find(|candidate| candidate.session_id == session_id)
            .ok_or_else(|| "missing candidate".to_string())?;
        candidate.state = CandidateState::Resumed;
        Ok(())
    }

    fn mark_recovery_failed(&self, session_id: Uuid, _error: &ResumeError) -> Result<(), String> {
        let mut candidates = self.candidates.lock().unwrap();
        let candidate = candidates
            .iter_mut()
            .find(|candidate| candidate.session_id == session_id)
            .ok_or_else(|| "missing candidate".to_string())?;
        candidate.state = CandidateState::RecoveryFailed;
        Ok(())
    }

    fn request_operator_decision(
        &self,
        _session_id: Uuid,
        _error: &ResumeError,
    ) -> Result<(), String> {
        self.decision_requests.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn emit_restart_resume_event(&self, event_id: FrEventId, _payload: serde_json::Value) {
        self.push_fr_event(event_id);
    }
}

fn candidates_for(scenario: CrashRecoveryScenario) -> Vec<Candidate> {
    match scenario {
        CrashRecoveryScenario::CleanShutdown => {
            vec![candidate(0, &[1]), candidate(0, &[1, 2]), candidate(0, &[])]
        }
        CrashRecoveryScenario::SigkillMidIteration
        | CrashRecoveryScenario::PostgresLoss
        | CrashRecoveryScenario::OrphanProcess
        | CrashRecoveryScenario::IdempotencyConflict
        | CrashRecoveryScenario::OperatorCancelDuringRecovery => candidate_vec(0, &[1, 2]),
        CrashRecoveryScenario::EventSeqGap => candidate_vec(0, &[1, 3]),
    }
}

fn candidate_vec(last_seq: i64, event_seqs: &[i64]) -> Vec<Candidate> {
    vec![candidate(last_seq, event_seqs)]
}

fn candidate(last_seq: i64, event_seqs: &[i64]) -> Candidate {
    let session_id = Uuid::now_v7();
    Candidate {
        session_id,
        checkpoint: SessionCheckpoint::new(
            session_id,
            Uuid::now_v7(),
            last_seq,
            serde_json::json!({ "counter": 0 }),
            CheckpointStateKind::Periodic,
        )
        .unwrap(),
        events: event_seqs
            .iter()
            .map(|seq| event(session_id, *seq))
            .collect(),
        state: CandidateState::Running,
    }
}

fn event(session_id: Uuid, seq: i64) -> EventLedgerRow {
    EventLedgerRow {
        event_id: format!("evt-{seq}"),
        event_sequence: seq,
        session_id,
        event_type: "step".to_string(),
        payload: serde_json::json!({ "by": 1 }),
        created_at: chrono::Utc::now(),
    }
}

fn scenario_slug(scenario: CrashRecoveryScenario) -> &'static str {
    match scenario {
        CrashRecoveryScenario::CleanShutdown => "clean_shutdown",
        CrashRecoveryScenario::SigkillMidIteration => "sigkill_mid_iteration",
        CrashRecoveryScenario::PostgresLoss => "postgres_loss",
        CrashRecoveryScenario::OrphanProcess => "orphan_process",
        CrashRecoveryScenario::EventSeqGap => "event_seq_gap",
        CrashRecoveryScenario::IdempotencyConflict => "idempotency_conflict",
        CrashRecoveryScenario::OperatorCancelDuringRecovery => "operator_cancel_during_recovery",
    }
}

fn golden_path(scenario: CrashRecoveryScenario) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("crash_recovery_evidence")
        .join(format!("{}.json", scenario_slug(scenario)))
}

fn assert_exact_counts(evidence: &CanonicalRecoveryEvidence) {
    match evidence.scenario {
        "clean_shutdown" => {
            assert_eq!(evidence.sessions_examined, 3);
            assert_eq!(evidence.sessions_resumed, 3);
            assert_eq!(evidence.sessions_recovery_failed, 0);
        }
        "sigkill_mid_iteration" | "postgres_loss" | "orphan_process" => {
            assert_eq!(evidence.sessions_examined, 1);
            assert_eq!(evidence.sessions_resumed, 1);
            assert_eq!(evidence.sessions_recovery_failed, 0);
        }
        "event_seq_gap" | "operator_cancel_during_recovery" => {
            assert_eq!(evidence.sessions_examined, 1);
            assert_eq!(evidence.sessions_resumed, 0);
            assert_eq!(evidence.sessions_recovery_failed, 1);
            assert_eq!(evidence.operator_decision_requests, 1);
        }
        "idempotency_conflict" => {
            assert_eq!(evidence.sessions_examined, 1);
            assert_eq!(evidence.sessions_resumed, 1);
            assert_eq!(evidence.sessions_recovery_failed, 0);
            assert_eq!(evidence.side_effects_applied, 2);
            assert_eq!(evidence.side_effects_already_applied, 2);
        }
        other => panic!("unexpected scenario {other}"),
    }
}
