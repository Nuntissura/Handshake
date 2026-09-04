//! MT-193 process-ledger-facing restart-resume exports.
//!
//! The orchestration implementation lives in `session_checkpoint::restart`
//! because replay, checkpoint state, and restart reporting share that type
//! boundary. This module gives process-ledger callers the contract-owned
//! import path requested by MT-193 without duplicating orchestration logic.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::time::Duration;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::fr_event_registry::FrEventId;
use crate::process_ledger::ReclaimResourceScope;
use crate::session_checkpoint::{
    CheckpointStateKind, EventLedgerRow, ReplayError, SessionCheckpoint, SessionCheckpointId,
};
use crate::storage::surreal::{SurrealProcessLedgerStore, SurrealStorage};

pub use crate::session_checkpoint::{
    OperatorDecisionRequest, OrphanReclaimInfo, RestartResumeOrchestrator, ResumableSession,
    ResumeError, ResumeReport, ResumedSessionInfo,
};

const RESUMABLE_STATES: [&str; 5] = [
    "CLAIMED",
    "RUNNING",
    "AWAITING_VERIFICATION",
    "PAUSED",
    "CANCELLATION_REQUESTED",
];

pub const RESTART_RESUME_BOOT_TIMEOUT_DEFAULT: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum RestartResumeRuntimeError {
    #[error("restart-resume Surreal store error: {0}")]
    Store(String),
    #[error("restart-resume serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("restart-resume invalid state kind: {0}")]
    InvalidStateKind(String),
    #[error("restart-resume invalid session_run_id {session_run_id}: {reason}")]
    InvalidSessionRunId {
        session_run_id: String,
        reason: String,
    },
}

#[derive(Debug)]
pub enum BoundedRestartResumeOutcome {
    Completed(ResumeReport),
    TimedOut {
        timeout: Duration,
        report: ResumeReport,
        evidence_persisted: bool,
    },
}

impl BoundedRestartResumeOutcome {
    pub fn report(&self) -> &ResumeReport {
        match self {
            Self::Completed(report) | Self::TimedOut { report, .. } => report,
        }
    }

    pub fn timed_out(&self) -> bool {
        matches!(self, Self::TimedOut { .. })
    }
}

#[derive(Clone)]
pub struct SurrealRestartResumeRunner {
    storage: SurrealStorage,
    resource_scope: ReclaimResourceScope,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ResumeCandidateBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    resumable_states: Vec<String>,
}

#[derive(Debug, SurrealValue)]
struct SessionBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    session_id: Uuid,
    session_run_id: String,
    after_sequence: i64,
}

#[derive(Debug, SurrealValue)]
struct ResumeCandidateRow {
    id: RecordId,
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    /// Projected only because SurrealDB requires every `ORDER BY` idiom to be
    /// present in the statement's selection; the boot pass resumes oldest-first.
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct CheckpointRow {
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: Value,
    state_kind: String,
    pending_artifacts: Vec<String>,
    created_at_utc: DateTime<Utc>,
    created_by_process: i64,
    schema_version: i64,
}

#[derive(Debug, SurrealValue)]
struct EventRow {
    event_id: String,
    event_sequence: i64,
    event_type: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Clone)]
struct SurrealResumeCandidate {
    queue_record: RecordId,
    session_id: Uuid,
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    checkpoint: Option<SessionCheckpoint>,
}

const LOAD_RESUME_CANDIDATES: &str = r#"
SELECT id, session_run_id, kernel_task_run_id, adapter_id, state, created_at
FROM kernel_session_queue
WHERE state IN $resumable_states
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY created_at, session_run_id;
"#;

const LOAD_LATEST_CHECKPOINT: &str = r#"
SELECT checkpoint_id, session_id, model_session_id, last_event_ledger_seq,
    compact_state, state_kind, pending_artifacts, created_at_utc,
    created_by_process, schema_version
FROM kernel_session_checkpoint
WHERE session_id = $session_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY created_at_utc DESC LIMIT 1;
"#;

const LOAD_EVENTS_AFTER_CHECKPOINT: &str = r#"
SELECT event_id, event_sequence, event_type, payload, created_at
FROM kernel_event_ledger
WHERE session_run_id = $session_run_id
    AND event_sequence > $after_sequence
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY event_sequence;
"#;

#[derive(Debug, SurrealValue)]
struct ResumeMutationBindings {
    queue_record: RecordId,
    checkpoint_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    resumable_states: Vec<String>,
    checkpoint: Value,
}

#[derive(Debug, SurrealValue)]
struct FailureMutationBindings {
    queue_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    error: Value,
}

#[derive(Debug, SurrealValue)]
struct ReportBindings {
    report_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    report: Value,
}

const RESUME_CANDIDATE_TRANSACTION: &str = r#"
BEGIN TRANSACTION;
LET $queue = UPDATE ONLY $queue_record SET
    state = 'RETRY_SCHEDULED',
    claimed_by = NONE,
    lease_expires_at = NONE,
    available_at = time::now(),
    updated_at = time::now()
WHERE state IN $resumable_states
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
IF $queue = NONE { THROW 'RESTART_RESUME_QUEUE_SCOPE_OR_STATE_MISMATCH'; };
LET $existing = SELECT * FROM ONLY $checkpoint_record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND checkpoint_id = <uuid>$checkpoint.checkpoint_id
    AND session_id = <uuid>$checkpoint.session_id
    AND last_event_ledger_seq = $checkpoint.last_event_ledger_seq
    AND compact_state = $checkpoint.compact_state;
IF $existing = NONE {
    IF record::exists($checkpoint_record) {
        THROW 'RESTART_RESUME_CHECKPOINT_IDEMPOTENCY_CONFLICT';
    };
    CREATE $checkpoint_record CONTENT {
        checkpoint_id: <uuid>$checkpoint.checkpoint_id,
        session_id: <uuid>$checkpoint.session_id,
        model_session_id: <uuid>$checkpoint.model_session_id,
        last_event_ledger_seq: $checkpoint.last_event_ledger_seq,
        compact_state: $checkpoint.compact_state,
        state_kind: $checkpoint.state_kind,
        pending_artifacts: $checkpoint.pending_artifacts,
        created_at_utc: <datetime>$checkpoint.created_at_utc,
        created_by_process: $checkpoint.created_by_process,
        schema_version: $checkpoint.schema_version,
        owner_account_id: $owner_account_id,
        actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id,
        access_space_id: $access_space_id,
        workspace_id: $workspace_id
    };
};
LET $verified = SELECT * FROM ONLY $checkpoint_record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND checkpoint_id = <uuid>$checkpoint.checkpoint_id
    AND session_id = <uuid>$checkpoint.session_id
    AND last_event_ledger_seq = $checkpoint.last_event_ledger_seq
    AND compact_state = $checkpoint.compact_state;
IF $verified = NONE {
    THROW 'RESTART_RESUME_CHECKPOINT_VERIFICATION_MISMATCH';
};
RETURN 1;
COMMIT TRANSACTION;
"#;

const RECORD_RESUME_FAILURE: &str = r#"
UPDATE ONLY $queue_record SET
    state = 'FAILED',
    claimed_by = NONE,
    lease_expires_at = NONE,
    recovery_error = $error,
    updated_at = time::now()
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

const PERSIST_RESUME_REPORT: &str = r#"
LET $existing = SELECT * FROM ONLY $report_record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND report_id = <uuid>$report.report_id
    AND sessions_examined = $report.sessions_examined
    AND total_replay_events = $report.total_replay_events;
IF $existing = NONE {
    IF record::exists($report_record) {
        THROW 'RESTART_RESUME_REPORT_IDEMPOTENCY_CONFLICT';
    };
    CREATE $report_record CONTENT {
        report_id: <uuid>$report.report_id,
        sessions_examined: $report.sessions_examined,
        sessions_resumed: $report.sessions_resumed,
        sessions_recovery_failed: $report.sessions_recovery_failed,
        total_replay_events: $report.total_replay_events,
        total_duration_ms: $report.total_duration_ms,
        started_at_utc: <datetime>$report.started_at_utc,
        completed_at_utc: <datetime>$report.completed_at_utc,
        orphan_reclaims: $report.orphan_reclaims,
        operator_decision_requests: $report.operator_decision_requests,
        fr_events_emitted: $report.fr_events_emitted,
        schema_version: $report.schema_version,
        owner_account_id: $owner_account_id,
        actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id,
        access_space_id: $access_space_id,
        workspace_id: $workspace_id
    };
};
-- `FROM $report_record`, not `FROM ONLY`: `ONLY` yields the bare record (or
-- NONE), so `array::len` would receive a record/NONE instead of an array and the
-- acknowledgement could never be read back as an int.
RETURN array::len(SELECT VALUE id FROM $report_record
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id);
"#;

impl SurrealRestartResumeRunner {
    pub fn new(storage: SurrealStorage, resource_scope: ReclaimResourceScope) -> Self {
        Self {
            storage,
            resource_scope,
        }
    }

    pub async fn open(
        storage: SurrealStorage,
        resource_scope: ReclaimResourceScope,
    ) -> Result<Self, RestartResumeRuntimeError> {
        SurrealProcessLedgerStore::open(storage.clone())
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        Ok(Self::new(storage, resource_scope))
    }

    pub async fn run(&self) -> Result<ResumeReport, RestartResumeRuntimeError> {
        self.run_with_preface_events(&[]).await
    }

    pub async fn run_with_bound(
        &self,
        timeout: Duration,
    ) -> Result<BoundedRestartResumeOutcome, RestartResumeRuntimeError> {
        let started_at_utc = Utc::now();
        let started = std::time::Instant::now();
        match tokio::time::timeout(timeout, self.run()).await {
            Ok(Ok(report)) => Ok(BoundedRestartResumeOutcome::Completed(report)),
            Ok(Err(error)) => Err(error),
            Err(_) => {
                let mut report = empty_report(started_at_utc, started.elapsed());
                emit_report_event(&mut report, FrEventId::RestartResumeStarted);
                let evidence_persisted = self.persist_report(&report).await.is_ok();
                Ok(BoundedRestartResumeOutcome::TimedOut {
                    timeout,
                    report,
                    evidence_persisted,
                })
            }
        }
    }

    async fn run_with_preface_events(
        &self,
        preface_events: &[FrEventId],
    ) -> Result<ResumeReport, RestartResumeRuntimeError> {
        let started_at_utc = Utc::now();
        let started = std::time::Instant::now();
        let candidates = self.load_candidates().await?;
        let mut report = empty_report(started_at_utc, Duration::ZERO);
        report.sessions_examined = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
        for event_id in preface_events {
            emit_report_event(&mut report, *event_id);
        }
        emit_report_event(&mut report, FrEventId::RestartResumeStarted);

        for candidate in candidates {
            let Some(checkpoint) = candidate.checkpoint.clone() else {
                self.record_failure(&mut report, &candidate, ResumeError::NoCheckpoint)
                    .await?;
                continue;
            };
            let events = self
                .load_events_after_checkpoint(&candidate, checkpoint.last_event_ledger_seq)
                .await?;
            match replay_scoped_events(&checkpoint, &events) {
                Ok((final_state, final_seq, applied_count)) => {
                    self.resume_candidate(&candidate, &checkpoint, final_state, final_seq)
                        .await?;
                    report.sessions_resumed.push(ResumedSessionInfo {
                        session_id: candidate.session_id,
                        events_applied: applied_count,
                        final_seq,
                    });
                    report.total_replay_events = report
                        .total_replay_events
                        .saturating_add(u64::from(applied_count));
                    emit_report_event(&mut report, FrEventId::RestartResumeSessionResumed);
                }
                Err(error) => {
                    self.record_failure(&mut report, &candidate, ResumeError::ReplayError(error))
                        .await?;
                }
            }
        }

        report.total_duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        report.completed_at_utc = Utc::now();
        emit_report_event(&mut report, FrEventId::RestartResumeCompleted);
        self.persist_report(&report).await?;
        Ok(report)
    }

    fn exact_scope_bindings(&self) -> ExactScopeBindings {
        ExactScopeBindings {
            owner_account_id: self.resource_scope.account_uuid.to_string(),
            actor_principal_id: self.resource_scope.actor_uuid.to_string(),
            authenticated_session_id: self.resource_scope.session_uuid.to_string(),
            access_space_id: self.resource_scope.access_space_uuid.to_string(),
            workspace_id: self.resource_scope.workspace_id.clone(),
        }
    }

    fn add_scope_fields(&self, value: &mut Value) -> Result<(), RestartResumeRuntimeError> {
        let object = value.as_object_mut().ok_or_else(|| {
            RestartResumeRuntimeError::Store("scoped Surreal payload is not an object".to_owned())
        })?;
        object.insert(
            "owner_account_id".to_owned(),
            Value::String(self.resource_scope.account_uuid.to_string()),
        );
        object.insert(
            "actor_principal_id".to_owned(),
            Value::String(self.resource_scope.actor_uuid.to_string()),
        );
        object.insert(
            "authenticated_session_id".to_owned(),
            Value::String(self.resource_scope.session_uuid.to_string()),
        );
        object.insert(
            "access_space_id".to_owned(),
            Value::String(self.resource_scope.access_space_uuid.to_string()),
        );
        object.insert(
            "workspace_id".to_owned(),
            Value::String(self.resource_scope.workspace_id.clone()),
        );
        Ok(())
    }
}

fn empty_report(started_at_utc: DateTime<Utc>, elapsed: Duration) -> ResumeReport {
    ResumeReport {
        report_id: Uuid::now_v7(),
        sessions_examined: 0,
        sessions_resumed: Vec::new(),
        sessions_recovery_failed: Vec::new(),
        orphan_reclaims: Vec::new(),
        operator_decision_requests: Vec::new(),
        fr_events_emitted: Vec::new(),
        total_replay_events: 0,
        total_duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        started_at_utc,
        completed_at_utc: Utc::now(),
    }
}

fn parse_session_uuid(session_run_id: &str) -> Result<Uuid, RestartResumeRuntimeError> {
    let raw = session_run_id
        .strip_prefix("session-")
        .unwrap_or(session_run_id);
    Uuid::parse_str(raw).map_err(|error| RestartResumeRuntimeError::InvalidSessionRunId {
        session_run_id: session_run_id.to_owned(),
        reason: error.to_string(),
    })
}

fn checkpoint_from_row(row: CheckpointRow) -> Result<SessionCheckpoint, RestartResumeRuntimeError> {
    let created_by_process = i32::try_from(row.created_by_process).map_err(|_| {
        RestartResumeRuntimeError::Store(
            "restart-resume checkpoint created_by_process is outside i32".to_owned(),
        )
    })?;
    let schema_version = u16::try_from(row.schema_version).map_err(|_| {
        RestartResumeRuntimeError::Store(
            "restart-resume checkpoint schema_version is outside u16".to_owned(),
        )
    })?;
    Ok(SessionCheckpoint {
        checkpoint_id: SessionCheckpointId(row.checkpoint_id),
        session_id: row.session_id,
        model_session_id: row.model_session_id,
        last_event_ledger_seq: row.last_event_ledger_seq,
        compact_state: row.compact_state,
        state_kind: checkpoint_state_kind(&row.state_kind)?,
        pending_artifacts: row.pending_artifacts,
        created_at_utc: row.created_at_utc,
        created_by_process,
        schema_version,
    })
}

fn checkpoint_state_kind(value: &str) -> Result<CheckpointStateKind, RestartResumeRuntimeError> {
    match value {
        "periodic" => Ok(CheckpointStateKind::Periodic),
        "event_triggered" => Ok(CheckpointStateKind::EventTriggered),
        "pre_shutdown" => Ok(CheckpointStateKind::PreShutdown),
        "post_failure" => Ok(CheckpointStateKind::PostFailure),
        other => Err(RestartResumeRuntimeError::InvalidStateKind(
            other.to_owned(),
        )),
    }
}

fn replay_scoped_events(
    checkpoint: &SessionCheckpoint,
    events: &[EventLedgerRow],
) -> Result<(Value, i64, u32), ReplayError> {
    let mut state = checkpoint.compact_state.clone();
    let mut final_seq = checkpoint.last_event_ledger_seq;
    let mut applied_count = 0u32;
    for event in events {
        if event.event_sequence <= final_seq {
            return Err(ReplayError::StateInvariantViolated {
                seq: event.event_sequence,
                invariant: "scoped replay event sequence is not strictly increasing".to_owned(),
            });
        }
        apply_json_replay_event(&mut state, event)?;
        final_seq = event.event_sequence;
        applied_count = applied_count.saturating_add(1);
    }
    Ok((state, final_seq, applied_count))
}

fn apply_json_replay_event(state: &mut Value, event: &EventLedgerRow) -> Result<(), ReplayError> {
    if let Some(by) = event.payload.get("by").and_then(Value::as_i64) {
        let counter = state.get("counter").and_then(Value::as_i64).unwrap_or(0);
        *state = json!({ "counter": counter + by });
        return Ok(());
    }
    if let Some(patch) = event.payload.get("state_patch").and_then(Value::as_object) {
        let Some(state_object) = state.as_object_mut() else {
            return Err(ReplayError::StateInvariantViolated {
                seq: event.event_sequence,
                invariant: "state_patch requires object compact_state".to_owned(),
            });
        };
        for (key, value) in patch {
            state_object.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

fn emit_report_event(report: &mut ResumeReport, event_id: FrEventId) {
    report.fr_events_emitted.push(event_id.as_str().to_owned());
}

#[cfg(test)]
mod surreal_scope_static_tests {
    use super::*;

    #[test]
    fn every_restart_resume_statement_binds_all_five_scope_fields() {
        for statement in [
            LOAD_RESUME_CANDIDATES,
            LOAD_LATEST_CHECKPOINT,
            LOAD_EVENTS_AFTER_CHECKPOINT,
            RESUME_CANDIDATE_TRANSACTION,
            RECORD_RESUME_FAILURE,
            PERSIST_RESUME_REPORT,
        ] {
            for predicate in [
                "owner_account_id = $owner_account_id",
                "actor_principal_id = $actor_principal_id",
                "authenticated_session_id = $authenticated_session_id",
                "access_space_id = $access_space_id",
                "workspace_id = $workspace_id",
            ] {
                assert!(statement.contains(predicate));
            }
        }
    }
}

impl SurrealRestartResumeRunner {
    async fn load_candidates(
        &self,
    ) -> Result<Vec<SurrealResumeCandidate>, RestartResumeRuntimeError> {
        let scope = self.exact_scope_bindings();
        let bindings = ResumeCandidateBindings {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            resumable_states: RESUMABLE_STATES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<ResumeCandidateRow, _>(LOAD_RESUME_CANDIDATES, bindings)
                        .await
                })
            })
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let session_id = parse_session_uuid(&row.session_run_id)?;
            let checkpoint = self.load_latest_checkpoint(session_id).await?;
            candidates.push(SurrealResumeCandidate {
                queue_record: row.id,
                session_id,
                session_run_id: row.session_run_id,
                kernel_task_run_id: row.kernel_task_run_id,
                adapter_id: row.adapter_id,
                state: row.state,
                checkpoint,
            });
        }
        Ok(candidates)
    }

    async fn load_latest_checkpoint(
        &self,
        session_id: Uuid,
    ) -> Result<Option<SessionCheckpoint>, RestartResumeRuntimeError> {
        let scope = self.exact_scope_bindings();
        let bindings = SessionBindings {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            session_id,
            session_run_id: String::new(),
            after_sequence: 0,
        };
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<CheckpointRow, _>(LOAD_LATEST_CHECKPOINT, bindings)
                        .await
                })
            })
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        row.map(checkpoint_from_row).transpose()
    }

    async fn load_events_after_checkpoint(
        &self,
        candidate: &SurrealResumeCandidate,
        after_sequence: i64,
    ) -> Result<Vec<EventLedgerRow>, RestartResumeRuntimeError> {
        let scope = self.exact_scope_bindings();
        let bindings = SessionBindings {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            session_id: candidate.session_id,
            session_run_id: candidate.session_run_id.clone(),
            after_sequence,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<EventRow, _>(LOAD_EVENTS_AFTER_CHECKPOINT, bindings)
                        .await
                })
            })
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|row| EventLedgerRow {
                event_id: row.event_id,
                event_sequence: row.event_sequence,
                session_id: candidate.session_id,
                event_type: row.event_type,
                payload: row.payload,
                created_at: row.created_at,
            })
            .collect())
    }
}

impl SurrealRestartResumeRunner {
    async fn resume_candidate(
        &self,
        candidate: &SurrealResumeCandidate,
        checkpoint: &SessionCheckpoint,
        final_state: Value,
        final_seq: i64,
    ) -> Result<(), RestartResumeRuntimeError> {
        let mut checkpoint_value = json!({
            "checkpoint_id": Uuid::now_v7(),
            "session_id": candidate.session_id,
            "model_session_id": checkpoint.model_session_id,
            "last_event_ledger_seq": final_seq,
            "compact_state": final_state,
            "state_kind": "post_failure",
            "pending_artifacts": checkpoint.pending_artifacts.clone(),
            "created_at_utc": Utc::now(),
            "created_by_process": i64::from(std::process::id()),
            "schema_version": checkpoint.schema_version,
        });
        self.add_scope_fields(&mut checkpoint_value)?;
        let checkpoint_id = checkpoint_value
            .get("checkpoint_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| {
                RestartResumeRuntimeError::Store(
                    "restart-resume checkpoint identity was not serialized".to_owned(),
                )
            })?;
        let scope = self.exact_scope_bindings();
        let bindings = ResumeMutationBindings {
            queue_record: candidate.queue_record.clone(),
            checkpoint_record: RecordId::new(
                "kernel_session_checkpoint",
                checkpoint_id.to_string(),
            ),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            resumable_states: RESUMABLE_STATES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            checkpoint: checkpoint_value,
        };
        let acknowledgements = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        // Result 7 is `RETURN 1`. SurrealDB indexes BEGIN (0) and
                        // COMMIT (8) as their own results, so reading 8 yields the
                        // COMMIT's NONE rather than the acknowledgement.
                        .query_values_at::<i64, _>(RESUME_CANDIDATE_TRANSACTION, bindings, 7)
                        .await
                })
            })
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        if acknowledgements.as_slice() != [1] {
            return Err(RestartResumeRuntimeError::Store(format!(
                "restart-resume transaction acknowledgement mismatch: {acknowledgements:?}"
            )));
        }
        Ok(())
    }

    async fn record_failure(
        &self,
        report: &mut ResumeReport,
        candidate: &SurrealResumeCandidate,
        error: ResumeError,
    ) -> Result<(), RestartResumeRuntimeError> {
        let scope = self.exact_scope_bindings();
        let bindings = FailureMutationBindings {
            queue_record: candidate.queue_record.clone(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            error: serde_json::to_value(&error)?,
        };
        let affected = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .execute_returning(RECORD_RESUME_FAILURE, bindings)
                        .await
                })
            })
            .await
            .map_err(|store_error| RestartResumeRuntimeError::Store(store_error.to_string()))?;
        if affected != 1 {
            return Err(RestartResumeRuntimeError::Store(format!(
                "restart-resume failure row escaped exact scope for {}",
                candidate.session_run_id
            )));
        }
        report
            .sessions_recovery_failed
            .push((candidate.session_id, error.clone()));
        report
            .operator_decision_requests
            .push(OperatorDecisionRequest {
                session_id: candidate.session_id,
                reason: error,
                options: vec![
                    "cancel_session".to_owned(),
                    "manual_repair_then_retry".to_owned(),
                    "retry_recovery".to_owned(),
                ],
                requested_at_utc: Utc::now(),
            });
        emit_report_event(report, FrEventId::RestartResumeSessionRecoveryFailed);
        Ok(())
    }

    async fn persist_report(&self, report: &ResumeReport) -> Result<(), RestartResumeRuntimeError> {
        let mut report_value = serde_json::to_value(report)?;
        self.add_scope_fields(&mut report_value)?;
        if let Some(object) = report_value.as_object_mut() {
            object.insert("schema_version".to_owned(), Value::from(2));
        }
        let scope = self.exact_scope_bindings();
        let bindings = ReportBindings {
            report_record: RecordId::new(
                "kernel_restart_resume_report",
                report.report_id.to_string(),
            ),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            report: report_value,
        };
        let acknowledgements = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<i64, _>(PERSIST_RESUME_REPORT, bindings, 2)
                        .await
                })
            })
            .await
            .map_err(|error| RestartResumeRuntimeError::Store(error.to_string()))?;
        if acknowledgements.as_slice() != [1] {
            return Err(RestartResumeRuntimeError::Store(format!(
                "restart-resume report acknowledgement mismatch: {acknowledgements:?}"
            )));
        }
        Ok(())
    }
}
