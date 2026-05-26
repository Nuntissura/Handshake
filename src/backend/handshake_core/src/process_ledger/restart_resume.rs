//! MT-193 process-ledger-facing restart-resume exports.
//!
//! The orchestration implementation lives in `session_checkpoint::restart`
//! because replay, checkpoint state, and restart reporting share that type
//! boundary. This module gives process-ledger callers the contract-owned
//! import path requested by MT-193 without duplicating orchestration logic.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{postgres::PgPool, Row};
use std::{future::Future, sync::Arc, time::Duration};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    flight_recorder::fr_event_registry::FrEventId,
    role_mailbox::RoleId,
    role_mailbox_v1::{
        ClaimMode, DecisionOption, DecisionRequestBody, ExecutorKind, LinkedRecordKind,
        MessageFamily, MessageType, ResponseAuthorityScope, RoleMailboxRepository,
        RoleMailboxThread, TakeoverPolicy,
    },
    session_checkpoint::{
        ApplyOutcome, CheckpointStateKind, EventLedgerRow, IdempotencyKey, IdempotencyLedger,
        IdempotencyLedgerError, ReplayError, SessionCheckpoint, SessionCheckpointId,
        SideEffectKind,
    },
};

pub use crate::session_checkpoint::{
    OperatorDecisionRequest, OrphanReclaimInfo, RestartResumeOrchestrator, ResumableSession,
    ResumeError, ResumeReport, ResumedSessionInfo,
};

const RESUMABLE_STATES: &[&str] = &[
    "CLAIMED",
    "RUNNING",
    "AWAITING_VERIFICATION",
    "PAUSED",
    "CANCELLATION_REQUESTED",
];

#[derive(Debug, Error)]
pub enum RestartResumeRuntimeError {
    #[error("postgres restart-resume error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("restart-resume serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("restart-resume mailbox error: {0}")]
    Mailbox(#[from] crate::role_mailbox_v1::MailboxError),
    #[error("restart-resume idempotency error: {0}")]
    Idempotency(#[from] IdempotencyLedgerError),
    #[error("restart-resume side effect {table} failed: {error}")]
    SideEffectFailed { table: String, error: String },
    #[error("restart-resume invalid state kind: {0}")]
    InvalidStateKind(String),
    #[error("restart-resume invalid session_run_id {session_run_id}: {reason}")]
    InvalidSessionRunId {
        session_run_id: String,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct RestartResumeDbBackoffPolicy {
    pub max_attempts: u32,
    pub delay: Duration,
}

impl RestartResumeDbBackoffPolicy {
    pub fn new(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            delay,
        }
    }
}

#[derive(Debug)]
pub struct RestartResumeDbBackoffEvidence {
    pub db_unavailable_attempts: u32,
    pub backoff_observed: bool,
    pub backoff_delay_ms: Vec<u64>,
    pub report: ResumeReport,
}

#[derive(Clone)]
pub struct PostgresRestartResumeRunner {
    pool: PgPool,
    idempotency: Arc<IdempotencyLedger>,
}

impl PostgresRestartResumeRunner {
    pub fn new(pool: PgPool) -> Self {
        Self {
            idempotency: Arc::new(IdempotencyLedger::new(pool.clone())),
            pool,
        }
    }

    pub async fn run(&self) -> Result<ResumeReport, RestartResumeRuntimeError> {
        self.run_with_preface_events(&[]).await
    }

    pub async fn run_with_db_backoff<F, Fut>(
        mut pool_factory: F,
        policy: RestartResumeDbBackoffPolicy,
    ) -> Result<RestartResumeDbBackoffEvidence, RestartResumeRuntimeError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<PgPool, RestartResumeRuntimeError>>,
    {
        let mut db_unavailable_attempts = 0;
        let mut backoff_delay_ms = Vec::new();
        let mut preface_events = Vec::new();

        for attempt in 1..=policy.max_attempts {
            let pool = match pool_factory().await {
                Ok(pool) => pool,
                Err(error)
                    if attempt < policy.max_attempts
                        && is_transient_db_unavailable_error(&error) =>
                {
                    record_db_unavailable_backoff(
                        policy.delay,
                        &mut db_unavailable_attempts,
                        &mut backoff_delay_ms,
                        &mut preface_events,
                    )
                    .await;
                    continue;
                }
                Err(error) => return Err(error),
            };

            let runner = Self::new(pool);
            match runner.run_with_preface_events(&preface_events).await {
                Ok(report) => {
                    return Ok(RestartResumeDbBackoffEvidence {
                        db_unavailable_attempts,
                        backoff_observed: !backoff_delay_ms.is_empty(),
                        backoff_delay_ms,
                        report,
                    });
                }
                Err(error)
                    if attempt < policy.max_attempts
                        && is_transient_db_unavailable_error(&error) =>
                {
                    record_db_unavailable_backoff(
                        policy.delay,
                        &mut db_unavailable_attempts,
                        &mut backoff_delay_ms,
                        &mut preface_events,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("RestartResumeDbBackoffPolicy always has at least one attempt")
    }

    async fn run_with_preface_events(
        &self,
        preface_events: &[FrEventId],
    ) -> Result<ResumeReport, RestartResumeRuntimeError> {
        let started_at_utc = Utc::now();
        let started = std::time::Instant::now();
        let candidates = self.load_candidates().await?;
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

        for event_id in preface_events {
            emit_report_event(&mut report, *event_id);
        }
        emit_report_event(&mut report, FrEventId::RestartResumeStarted);

        for candidate in candidates {
            let processes_reclaimed = self.reclaim_orphans(&candidate).await?;
            report.orphan_reclaims.push(OrphanReclaimInfo {
                session_id: candidate.session_id,
                processes_reclaimed,
                reclaimed_at_utc: Utc::now(),
            });

            let Some(checkpoint) = candidate.checkpoint.clone() else {
                self.record_failure(&mut report, &candidate, ResumeError::NoCheckpoint)
                    .await?;
                continue;
            };

            let events = self
                .load_events_after_checkpoint(
                    &candidate.session_run_id,
                    candidate.session_id,
                    checkpoint.last_event_ledger_seq,
                )
                .await?;
            let global_sequences = self
                .load_global_sequences_through(
                    checkpoint.last_event_ledger_seq,
                    events
                        .last()
                        .map(|event| event.event_sequence)
                        .unwrap_or(checkpoint.last_event_ledger_seq),
                )
                .await?;
            let replay_result =
                execute_postgres_global_replay(&checkpoint, &events, &global_sequences);
            match replay_result {
                Ok(result) => {
                    self.resume_candidate(
                        &candidate,
                        &checkpoint,
                        &result.final_state,
                        result.final_seq,
                    )
                    .await?;
                    report.sessions_resumed.push(ResumedSessionInfo {
                        session_id: candidate.session_id,
                        events_applied: result.applied_count,
                        final_seq: result.final_seq,
                    });
                    report.total_replay_events += result.applied_count as u64;
                    emit_report_event(&mut report, FrEventId::RestartResumeSessionResumed);
                }
                Err(error) => {
                    self.record_failure(&mut report, &candidate, ResumeError::ReplayError(error))
                        .await?;
                }
            }
        }

        report.total_duration_ms = started.elapsed().as_millis() as u64;
        report.completed_at_utc = Utc::now();
        emit_report_event(&mut report, FrEventId::RestartResumeCompleted);
        self.persist_report(&report).await?;
        Ok(report)
    }

    async fn load_candidates(
        &self,
    ) -> Result<Vec<PostgresResumeCandidate>, RestartResumeRuntimeError> {
        let rows = sqlx::query(
            r#"
            SELECT session_run_id, kernel_task_run_id, adapter_id, state
            FROM kernel_session_queue
            WHERE state = ANY($1)
            ORDER BY created_at, session_run_id
            "#,
        )
        .bind(RESUMABLE_STATES)
        .fetch_all(&self.pool)
        .await?;

        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let session_run_id: String = row.get("session_run_id");
            let session_id = parse_session_uuid(&session_run_id)?;
            let checkpoint = self.load_latest_checkpoint(session_id).await?;
            candidates.push(PostgresResumeCandidate {
                session_id,
                session_run_id,
                kernel_task_run_id: row.get("kernel_task_run_id"),
                adapter_id: row.get("adapter_id"),
                state: row.get("state"),
                checkpoint,
            });
        }
        Ok(candidates)
    }

    async fn load_latest_checkpoint(
        &self,
        session_id: Uuid,
    ) -> Result<Option<SessionCheckpoint>, RestartResumeRuntimeError> {
        let row = sqlx::query(
            r#"
            SELECT
                checkpoint_id,
                session_id,
                model_session_id,
                last_event_ledger_seq,
                compact_state,
                state_kind,
                pending_artifacts,
                created_at_utc,
                created_by_process,
                schema_version
            FROM kernel_session_checkpoint
            WHERE session_id = $1
            ORDER BY created_at_utc DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_checkpoint_row).transpose()
    }

    async fn load_events_after_checkpoint(
        &self,
        session_run_id: &str,
        session_id: Uuid,
        last_event_ledger_seq: i64,
    ) -> Result<Vec<EventLedgerRow>, RestartResumeRuntimeError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, event_sequence, event_type, payload, created_at
            FROM kernel_event_ledger
            WHERE session_run_id = $1
              AND event_sequence > $2
            ORDER BY event_sequence
            "#,
        )
        .bind(session_run_id)
        .bind(last_event_ledger_seq)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| EventLedgerRow {
                event_id: row.get("event_id"),
                event_sequence: row.get("event_sequence"),
                session_id,
                event_type: row.get("event_type"),
                payload: row.get("payload"),
                created_at: timestamp_column(&row, "created_at"),
            })
            .collect())
    }

    async fn load_global_sequences_through(
        &self,
        after_seq: i64,
        through_seq: i64,
    ) -> Result<Vec<i64>, RestartResumeRuntimeError> {
        if through_seq <= after_seq {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT event_sequence
            FROM kernel_event_ledger
            WHERE event_sequence > $1
              AND event_sequence <= $2
            ORDER BY event_sequence
            "#,
        )
        .bind(after_seq)
        .bind(through_seq)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.get("event_sequence"))
            .collect())
    }

    async fn reclaim_orphans(
        &self,
        candidate: &PostgresResumeCandidate,
    ) -> Result<u32, RestartResumeRuntimeError> {
        let mut tx = self.pool.begin().await?;
        let inserted = insert_idempotency_key_in_tx(
            &mut tx,
            postgres_write_key(candidate.session_id, 0, "kernel_process_lifecycle"),
        )
        .await?;
        if inserted == 0 {
            tx.rollback().await?;
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            UPDATE kernel_process_lifecycle
            SET stopped_at = NOW(),
                exit_code = COALESCE(exit_code, -1),
                stop_reason = COALESCE(stop_reason, 'reclaim')
            WHERE parent_session_id = $1
              AND stopped_at IS NULL
            "#,
        )
        .bind(&candidate.session_run_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(u32::try_from(result.rows_affected()).unwrap_or(u32::MAX))
    }

    async fn resume_candidate(
        &self,
        candidate: &PostgresResumeCandidate,
        checkpoint: &SessionCheckpoint,
        final_state: &Value,
        final_seq: i64,
    ) -> Result<(), RestartResumeRuntimeError> {
        let mut tx = self.pool.begin().await?;
        let queue_inserted = insert_idempotency_key_in_tx(
            &mut tx,
            postgres_write_key(candidate.session_id, final_seq, "kernel_session_queue"),
        )
        .await?;
        let checkpoint_inserted = insert_idempotency_key_in_tx(
            &mut tx,
            postgres_write_key(candidate.session_id, final_seq, "kernel_session_checkpoint"),
        )
        .await?;
        if queue_inserted == 0 && checkpoint_inserted == 0 {
            tx.rollback().await?;
            return Ok(());
        }

        sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET state = 'FAILED',
                updated_at = NOW()
            WHERE session_run_id = $1
              AND state = ANY($2)
            "#,
        )
        .bind(&candidate.session_run_id)
        .bind(RESUMABLE_STATES)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET state = 'RETRY_SCHEDULED',
                claimed_by = NULL,
                lease_expires_at = NULL,
                available_at = NOW(),
                updated_at = NOW()
            WHERE session_run_id = $1
              AND state = 'FAILED'
            "#,
        )
        .bind(&candidate.session_run_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO kernel_session_checkpoint (
                checkpoint_id,
                session_id,
                model_session_id,
                last_event_ledger_seq,
                compact_state,
                state_kind,
                pending_artifacts,
                created_at_utc,
                created_by_process,
                schema_version
            )
            VALUES ($1, $2, $3, $4, $5, 'post_failure', $6, NOW(), $7, $8)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(candidate.session_id)
        .bind(checkpoint.model_session_id)
        .bind(final_seq)
        .bind(final_state)
        .bind(serde_json::to_value(&checkpoint.pending_artifacts)?)
        .bind(std::process::id() as i32)
        .bind(i32::from(checkpoint.schema_version))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn record_failure(
        &self,
        report: &mut ResumeReport,
        candidate: &PostgresResumeCandidate,
        error: ResumeError,
    ) -> Result<(), RestartResumeRuntimeError> {
        report
            .sessions_recovery_failed
            .push((candidate.session_id, error.clone()));

        let failure_seq = failure_idempotency_seq(candidate, &error);
        self.apply_idempotent_postgres_write(
            candidate.session_id,
            failure_seq,
            "role_mailbox_message",
            || async {
                self.post_operator_decision(candidate, &error)
                    .await
                    .map_err(|err| err.to_string())
            },
        )
        .await?;
        self.apply_idempotent_postgres_write(
            candidate.session_id,
            failure_seq,
            "kernel_session_queue",
            || async {
                sqlx::query(
                    r#"
                    UPDATE kernel_session_queue
                    SET state = 'FAILED',
                        claimed_by = NULL,
                        lease_expires_at = NULL,
                        updated_at = NOW()
                    WHERE session_run_id = $1
                    "#,
                )
                .bind(&candidate.session_run_id)
                .execute(&self.pool)
                .await
                .map_err(|err| err.to_string())?;
                Ok(())
            },
        )
        .await?;
        report
            .operator_decision_requests
            .push(OperatorDecisionRequest {
                session_id: candidate.session_id,
                reason: error,
                options: vec![
                    "cancel_session".to_string(),
                    "manual_repair_then_retry".to_string(),
                    "retry_recovery".to_string(),
                ],
                requested_at_utc: Utc::now(),
            });
        emit_report_event(report, FrEventId::RestartResumeSessionRecoveryFailed);
        Ok(())
    }

    async fn post_operator_decision(
        &self,
        candidate: &PostgresResumeCandidate,
        error: &ResumeError,
    ) -> Result<(), RestartResumeRuntimeError> {
        let existing: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT m.message_id
            FROM role_mailbox_thread t
            JOIN role_mailbox_message m ON m.thread_id = t.thread_id
            WHERE t.linked_record_id = $1
              AND m.message_type = 'decision_request'
              AND m.body->>'session_run_id' = $2
            ORDER BY m.created_at_utc
            LIMIT 1
            "#,
        )
        .bind(candidate.session_id.to_string())
        .bind(&candidate.session_run_id)
        .fetch_optional(&self.pool)
        .await?;
        if existing.is_some() {
            return Ok(());
        }

        let repo = RoleMailboxRepository::new(self.pool.clone());
        let thread = RoleMailboxThread::open(
            format!("Restart recovery decision for {}", candidate.session_run_id),
            LinkedRecordKind::Freeform,
            Some(candidate.session_id.to_string()),
            vec![ExecutorKind::Operator],
            ClaimMode::Open,
            TakeoverPolicy::Never,
            ResponseAuthorityScope::OperatorOnly,
        );
        let thread = repo.create_thread(thread).await?;

        let family = MessageFamily::DecisionRequest(DecisionRequestBody {
            question: format!(
                "Restart recovery failed for {}; choose recovery handling.",
                candidate.session_run_id
            ),
            options: vec![
                DecisionOption {
                    option_id: "cancel_session".to_string(),
                    label: "Cancel session".to_string(),
                    detail: Some(
                        "Mark the session cancelled and do not replay further.".to_string(),
                    ),
                },
                DecisionOption {
                    option_id: "manual_repair_then_retry".to_string(),
                    label: "Repair then retry".to_string(),
                    detail: Some(
                        "Repair the authoritative rows, then retry restart recovery.".to_string(),
                    ),
                },
                DecisionOption {
                    option_id: "retry_recovery".to_string(),
                    label: "Retry recovery".to_string(),
                    detail: Some("Retry with the current authoritative rows.".to_string()),
                },
            ],
            decision_authority_role: RoleId::Operator,
            deadline_utc: None,
        });
        let mut body = serde_json::to_value(family)?;
        if let Some(map) = body.as_object_mut() {
            map.insert("resume_error".to_string(), serde_json::to_value(error)?);
            map.insert(
                "session_run_id".to_string(),
                Value::String(candidate.session_run_id.clone()),
            );
            map.insert(
                "previous_state".to_string(),
                Value::String(candidate.state.clone()),
            );
            map.insert(
                "adapter_id".to_string(),
                Value::String(candidate.adapter_id.clone()),
            );
            map.insert(
                "kernel_task_run_id".to_string(),
                Value::String(candidate.kernel_task_run_id.clone()),
            );
        }

        repo.append_message(
            thread.thread_id,
            MessageType::DecisionRequest,
            RoleId::Orchestrator,
            vec![RoleId::Operator],
            body,
        )
        .await?;
        Ok(())
    }

    async fn apply_idempotent_postgres_write<F, Fut>(
        &self,
        session_id: Uuid,
        event_seq: i64,
        table: &'static str,
        op: F,
    ) -> Result<ApplyOutcome, RestartResumeRuntimeError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let outcome = self
            .idempotency
            .try_apply(postgres_write_key(session_id, event_seq, table), op)
            .await?;
        if let ApplyOutcome::Failed { error } = outcome {
            return Err(RestartResumeRuntimeError::SideEffectFailed {
                table: table.to_string(),
                error,
            });
        }
        Ok(outcome)
    }

    async fn persist_report(&self, report: &ResumeReport) -> Result<(), RestartResumeRuntimeError> {
        sqlx::query(
            r#"
            INSERT INTO kernel_restart_resume_report (
                report_id,
                sessions_examined,
                sessions_resumed,
                sessions_recovery_failed,
                orphan_reclaims,
                operator_decision_requests,
                fr_events_emitted,
                total_replay_events,
                total_duration_ms,
                started_at_utc,
                completed_at_utc,
                schema_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 2)
            "#,
        )
        .bind(report.report_id)
        .bind(i32::try_from(report.sessions_examined).unwrap_or(i32::MAX))
        .bind(serde_json::to_value(&report.sessions_resumed)?)
        .bind(serde_json::to_value(&report.sessions_recovery_failed)?)
        .bind(serde_json::to_value(&report.orphan_reclaims)?)
        .bind(serde_json::to_value(&report.operator_decision_requests)?)
        .bind(serde_json::to_value(&report.fr_events_emitted)?)
        .bind(i64::try_from(report.total_replay_events).unwrap_or(i64::MAX))
        .bind(i64::try_from(report.total_duration_ms).unwrap_or(i64::MAX))
        .bind(report.started_at_utc)
        .bind(report.completed_at_utc)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
struct PostgresResumeCandidate {
    session_id: Uuid,
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    checkpoint: Option<SessionCheckpoint>,
}

struct PostgresReplayResult {
    final_state: Value,
    final_seq: i64,
    applied_count: u32,
}

fn execute_postgres_global_replay(
    checkpoint: &SessionCheckpoint,
    events: &[EventLedgerRow],
    global_sequences: &[i64],
) -> Result<PostgresReplayResult, ReplayError> {
    if let Some(max_seq) = events.last().map(|event| event.event_sequence) {
        if let Some(missing) = first_missing_global_sequence(
            checkpoint.last_event_ledger_seq,
            max_seq,
            global_sequences,
        ) {
            return Err(ReplayError::MissingEvent {
                gap_at_seq: missing,
            });
        }
    }

    let mut state = checkpoint.compact_state.clone();
    let mut final_seq = checkpoint.last_event_ledger_seq;
    let mut applied_count = 0;
    for event in events {
        apply_json_replay_event(&mut state, event)?;
        final_seq = event.event_sequence;
        applied_count += 1;
    }
    Ok(PostgresReplayResult {
        final_state: state,
        final_seq,
        applied_count,
    })
}

fn first_missing_global_sequence(
    after_seq: i64,
    through_seq: i64,
    global_sequences: &[i64],
) -> Option<i64> {
    let mut expected = after_seq + 1;
    for seq in global_sequences {
        if *seq < expected {
            continue;
        }
        if *seq > through_seq {
            break;
        }
        if *seq > expected {
            return Some(expected);
        }
        expected += 1;
    }
    (expected <= through_seq).then_some(expected)
}

fn failure_idempotency_seq(candidate: &PostgresResumeCandidate, error: &ResumeError) -> i64 {
    match error {
        ResumeError::ReplayError(ReplayError::EventNotApplicable { seq, .. })
        | ResumeError::ReplayError(ReplayError::StateInvariantViolated { seq, .. }) => *seq,
        ResumeError::ReplayError(ReplayError::MissingEvent { gap_at_seq }) => *gap_at_seq,
        _ => candidate
            .checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.last_event_ledger_seq)
            .unwrap_or(0),
    }
}

fn postgres_write_key(session_id: Uuid, event_seq: i64, table: &str) -> IdempotencyKey {
    IdempotencyKey {
        session_id,
        event_seq,
        side_effect_kind: SideEffectKind::postgres_write_table(table),
    }
}

async fn insert_idempotency_key_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: IdempotencyKey,
) -> Result<u64, sqlx::Error> {
    let side_effect_storage_key = key.side_effect_storage_key();
    let result = sqlx::query(
        r#"
        INSERT INTO kernel_idempotency_ledger
            (session_id, event_seq, side_effect_kind)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(key.session_id)
    .bind(key.event_seq)
    .bind(side_effect_storage_key)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

fn parse_session_uuid(session_run_id: &str) -> Result<Uuid, RestartResumeRuntimeError> {
    let raw = session_run_id
        .strip_prefix("SR-")
        .unwrap_or(session_run_id)
        .trim();
    Uuid::parse_str(raw).map_err(|error| RestartResumeRuntimeError::InvalidSessionRunId {
        session_run_id: session_run_id.to_string(),
        reason: error.to_string(),
    })
}

fn map_checkpoint_row(
    row: sqlx::postgres::PgRow,
) -> Result<SessionCheckpoint, RestartResumeRuntimeError> {
    let pending_artifacts: Value = row.get("pending_artifacts");
    Ok(SessionCheckpoint {
        checkpoint_id: SessionCheckpointId(row.get("checkpoint_id")),
        session_id: row.get("session_id"),
        model_session_id: row.get("model_session_id"),
        last_event_ledger_seq: row.get("last_event_ledger_seq"),
        compact_state: row.get("compact_state"),
        state_kind: parse_checkpoint_state_kind(row.get::<String, _>("state_kind").as_str())?,
        pending_artifacts: serde_json::from_value(pending_artifacts)?,
        created_at_utc: row.get("created_at_utc"),
        created_by_process: row.get("created_by_process"),
        schema_version: row.get::<i32, _>("schema_version") as u16,
    })
}

fn parse_checkpoint_state_kind(
    value: &str,
) -> Result<CheckpointStateKind, RestartResumeRuntimeError> {
    match value {
        "periodic" => Ok(CheckpointStateKind::Periodic),
        "event_triggered" => Ok(CheckpointStateKind::EventTriggered),
        "pre_shutdown" => Ok(CheckpointStateKind::PreShutdown),
        "post_failure" => Ok(CheckpointStateKind::PostFailure),
        other => Err(RestartResumeRuntimeError::InvalidStateKind(
            other.to_string(),
        )),
    }
}

fn timestamp_column(row: &sqlx::postgres::PgRow, column: &str) -> DateTime<Utc> {
    let naive: chrono::NaiveDateTime = row.get(column);
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
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
                invariant: "state_patch requires object compact_state".to_string(),
            });
        };
        for (key, value) in patch {
            state_object.insert(key.clone(), value.clone());
        }
    }

    Ok(())
}

fn emit_report_event(report: &mut ResumeReport, event_id: FrEventId) {
    report.fr_events_emitted.push(event_id.as_str().to_string());
}

async fn record_db_unavailable_backoff(
    delay: Duration,
    db_unavailable_attempts: &mut u32,
    backoff_delay_ms: &mut Vec<u64>,
    preface_events: &mut Vec<FrEventId>,
) {
    *db_unavailable_attempts += 1;
    backoff_delay_ms.push(delay.as_millis() as u64);
    if !preface_events.contains(&FrEventId::RestartResumeDbUnavailable) {
        preface_events.push(FrEventId::RestartResumeDbUnavailable);
    }
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
}

fn is_transient_db_unavailable_error(error: &RestartResumeRuntimeError) -> bool {
    match error {
        RestartResumeRuntimeError::Sqlx(sqlx::Error::PoolClosed)
        | RestartResumeRuntimeError::Sqlx(sqlx::Error::PoolTimedOut)
        | RestartResumeRuntimeError::Sqlx(sqlx::Error::Io(_))
        | RestartResumeRuntimeError::Sqlx(sqlx::Error::Tls(_)) => true,
        RestartResumeRuntimeError::Sqlx(sqlx::Error::Database(db_error)) => db_error
            .code()
            .as_deref()
            .is_some_and(|code| code.starts_with("08")),
        _ => false,
    }
}
