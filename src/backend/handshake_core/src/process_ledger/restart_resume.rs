//! MT-193 process-ledger-facing restart-resume exports.
//!
//! The orchestration implementation lives in `session_checkpoint::restart`
//! because replay, checkpoint state, and restart reporting share that type
//! boundary. This module gives process-ledger callers the contract-owned
//! import path requested by MT-193 without duplicating orchestration logic.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::{future::Future, sync::Arc, time::Duration};
use surrealdb::types::SurrealValue;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    flight_recorder::fr_event_registry::FrEventId,
    process_ledger::{
        LedgerEvent, ProcessLedgerStore, ReclaimProcessStore, ReclaimableProcess,
        SurrealProcessLedgerStore,
    },
    role_mailbox::RoleId,
    role_mailbox_v1::{
        ClaimMode, DecisionOption, DecisionRequestBody, ExecutorKind, LinkedRecordKind,
        MessageFamily, MessageType, ResponseAuthorityScope, RoleMailboxRepository,
        RoleMailboxThread, TakeoverPolicy,
    },
    sandbox::{AdapterId, ProcessHandle, SandboxAdapterRegistry},
    session_checkpoint::{
        ApplyOutcome, CheckpointStateKind, EventLedgerRow, IdempotencyKey, IdempotencyLedger,
        IdempotencyLedgerError, ReplayError, SessionCheckpoint, SessionCheckpointId,
        SideEffectKind,
    },
    storage::surreal::{SurrealStorage, SurrealStorageError},
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

fn resumable_states() -> Vec<String> {
    RESUMABLE_STATES
        .iter()
        .map(|state| (*state).to_owned())
        .collect()
}

#[derive(Debug, Error)]
pub enum RestartResumeRuntimeError {
    #[error("restart-resume storage error: {0}")]
    Storage(#[from] SurrealStorageError),
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

#[async_trait]
pub trait StartupProcessCleanup: Send + Sync + 'static {
    async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String>;
}

/// Production startup bridge from durable process-ledger identity to a
/// restart-aware cleanup contract. Synthetic in-process model rows are already
/// gone with the prior backend boot. Sandboxed rows use the adapter's durable
/// external identity/containment path; ordinary live-handle `kill` is never
/// called, and a PID-only kill that could hit a reused PID is never substituted.
pub struct RegistryStartupProcessCleanup {
    registry: Arc<SandboxAdapterRegistry>,
}

impl RegistryStartupProcessCleanup {
    pub fn new(registry: Arc<SandboxAdapterRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl StartupProcessCleanup for RegistryStartupProcessCleanup {
    async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String> {
        let (adapter_id, sandbox_internal_id) = match (
            process.sandbox_adapter_id.as_deref(),
            process.sandbox_internal_id.as_deref(),
        ) {
            // These two engines are recorded with synthetic PIDs by
            // ProductionModelSessionFactory. Their runtime lives inside the old
            // backend process (or is a remote cloud request), so that prior boot
            // ending is itself the authoritative cleanup event.
            (None, None)
                if matches!(
                    process.engine_kind,
                    crate::process_ledger::ProcessEngineKind::LlamaCpp
                        | crate::process_ledger::ProcessEngineKind::Candle
                ) =>
            {
                return Ok(())
            }
            (Some(adapter_id), Some(sandbox_internal_id)) => {
                (adapter_id.to_owned(), sandbox_internal_id.to_owned())
            }
            (None, None) => {
                return Err(format!(
                    "process {} ({}) has no restart-safe external identity",
                    process.process_uuid,
                    process.engine_kind.as_str()
                ))
            }
            _ => {
                return Err(format!(
                    "process {} has partial durable sandbox identity; refusing unsafe cleanup",
                    process.process_uuid
                ))
            }
        };
        let adapter_id = AdapterId::new(adapter_id);
        let adapter = self.registry.get(&adapter_id).ok_or_else(|| {
            format!(
                "sandbox adapter {adapter_id} for process {} is unavailable during startup recovery",
                process.process_uuid
            )
        })?;
        let handle = ProcessHandle {
            id: process.process_uuid,
            adapter_id,
            pid: process.os_pid,
            sandbox_internal_id,
            spawned_at_utc: process.started_at,
        };
        adapter
            .cleanup_after_restart(&handle)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[async_trait]
pub trait RestartOrphanReclaimer: Send + Sync + 'static {
    async fn reclaim_session(&self, session_run_id: &str)
        -> Result<u32, RestartResumeRuntimeError>;
}

#[async_trait]
pub trait RestartReclaimStore: Send + Sync + 'static {
    async fn claim_active(
        &self,
        session_run_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, crate::process_ledger::ProcessLedgerError>;
    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), crate::process_ledger::ProcessLedgerError>;
    async fn write_reclaim_stop(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), crate::process_ledger::ProcessLedgerError>;
    async fn abandon(
        &self,
        processes: &[ReclaimableProcess],
    ) -> Result<(), crate::process_ledger::ProcessLedgerError>;
}

#[async_trait]
impl RestartReclaimStore for SurrealProcessLedgerStore {
    async fn claim_active(
        &self,
        session_run_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, crate::process_ledger::ProcessLedgerError> {
        self.active_processes_for_session(session_run_id).await
    }

    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
        self.mark_reclaim_cleanup_completed(process).await
    }

    async fn write_reclaim_stop(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
        self.write_batch(vec![LedgerEvent::Stop(process.reclaim_stop(-1))])
            .await
    }

    async fn abandon(
        &self,
        processes: &[ReclaimableProcess],
    ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
        self.abandon_reclaim_claims(processes).await
    }
}

/// Adapter-aware startup reclaim implementation. Restart-safe cleanup is the
/// predecessor of a durable `reclaim_killed` marker and the timestamp-guarded
/// terminal STOP. Once the marker commits, a STOP failure leaves it in place so
/// a later pass finalizes the row without repeating external cleanup.
pub struct SurrealRestartOrphanReclaimer {
    store: Arc<dyn RestartReclaimStore>,
    process_cleanup: Arc<dyn StartupProcessCleanup>,
}

impl SurrealRestartOrphanReclaimer {
    pub fn new<S>(store: Arc<S>, process_cleanup: Arc<dyn StartupProcessCleanup>) -> Self
    where
        S: RestartReclaimStore,
    {
        Self {
            store,
            process_cleanup,
        }
    }

    async fn release_after_failure(
        &self,
        claimed: &[ReclaimableProcess],
        cause: String,
    ) -> RestartResumeRuntimeError {
        let detail = match self.store.abandon(claimed).await {
            Ok(()) => cause,
            Err(release_error) => {
                format!("{cause}; exact reclaim-claim release also failed: {release_error}")
            }
        };
        RestartResumeRuntimeError::SideEffectFailed {
            table: "kernel_process_lifecycle".to_owned(),
            error: detail,
        }
    }
}

#[async_trait]
impl RestartOrphanReclaimer for SurrealRestartOrphanReclaimer {
    async fn reclaim_session(
        &self,
        session_run_id: &str,
    ) -> Result<u32, RestartResumeRuntimeError> {
        let claimed = self
            .store
            .claim_active(session_run_id)
            .await
            .map_err(|error| RestartResumeRuntimeError::SideEffectFailed {
                table: "kernel_process_lifecycle".to_owned(),
                error: error.to_string(),
            })?;
        let mut reclaimed = 0_u32;
        for (index, process) in claimed.iter().enumerate() {
            if !process.reclaim_cleanup_completed {
                if let Err(error) = self.process_cleanup.cleanup(process).await {
                    return Err(self
                        .release_after_failure(
                            &claimed[index..],
                            format!(
                                "restart cleanup failed for process {}: {error}",
                                process.process_uuid
                            ),
                        )
                        .await);
                }
                if let Err(error) = self.store.mark_cleanup_completed(process).await {
                    return Err(self
                        .release_after_failure(
                            &claimed[index..],
                            format!(
                                "durable post-cleanup marker failed for process {}: {error}",
                                process.process_uuid
                            ),
                        )
                        .await);
                }
            }
            if let Err(error) = self.store.write_reclaim_stop(process).await {
                return Err(self
                    .release_after_failure(
                        &claimed[index.saturating_add(1)..],
                        format!(
                            "durable reclaim STOP failed after cleanup marker for process {}: {error}",
                            process.process_uuid
                        ),
                    )
                    .await);
            }
            reclaimed = reclaimed.saturating_add(1);
        }
        Ok(reclaimed)
    }
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
pub struct SurrealRestartResumeRunner {
    storage: SurrealStorage,
    idempotency: Arc<IdempotencyLedger>,
    orphan_reclaimer: Arc<dyn RestartOrphanReclaimer>,
}

impl SurrealRestartResumeRunner {
    pub fn new(storage: SurrealStorage, orphan_reclaimer: Arc<dyn RestartOrphanReclaimer>) -> Self {
        Self {
            idempotency: Arc::new(IdempotencyLedger::new(storage.clone())),
            storage,
            orphan_reclaimer,
        }
    }

    pub async fn run(&self) -> Result<ResumeReport, RestartResumeRuntimeError> {
        self.run_with_preface_events(&[]).await
    }

    pub async fn run_with_db_backoff<F, Fut>(
        mut storage_factory: F,
        policy: RestartResumeDbBackoffPolicy,
        orphan_reclaimer: Arc<dyn RestartOrphanReclaimer>,
    ) -> Result<RestartResumeDbBackoffEvidence, RestartResumeRuntimeError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<SurrealStorage, RestartResumeRuntimeError>>,
    {
        let mut db_unavailable_attempts = 0;
        let mut backoff_delay_ms = Vec::new();
        let mut preface_events = Vec::new();

        for attempt in 1..=policy.max_attempts {
            let storage = match storage_factory().await {
                Ok(storage) => storage,
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

            let runner = Self::new(storage, Arc::clone(&orphan_reclaimer));
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
            let replay_result = execute_global_replay(&checkpoint, &events, &global_sequences);
            match replay_result {
                Ok(result) => {
                    let resumed = self
                        .resume_candidate(
                            &candidate,
                            &checkpoint,
                            &result.final_state,
                            result.final_seq,
                        )
                        .await?;
                    if resumed {
                        report.sessions_resumed.push(ResumedSessionInfo {
                            session_id: candidate.session_id,
                            events_applied: result.applied_count,
                            final_seq: result.final_seq,
                        });
                        report.total_replay_events += result.applied_count as u64;
                        emit_report_event(&mut report, FrEventId::RestartResumeSessionResumed);
                    }
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

    async fn load_candidates(&self) -> Result<Vec<ResumeCandidate>, RestartResumeRuntimeError> {
        #[derive(SurrealValue)]
        struct CandidatesBindings {
            states: Vec<String>,
        }

        let rows: Vec<CandidateRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT session_run_id, kernel_task_run_id, adapter_id, state, attempt_count, created_at \
                             FROM kernel_session_queue WHERE state IN $states \
                             ORDER BY created_at, session_run_id;",
                            CandidatesBindings {
                                states: resumable_states(),
                            },
                        )
                        .await
                })
            })
            .await?;

        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let session_id = parse_session_uuid(&row.session_run_id)?;
            let checkpoint = self.load_latest_checkpoint(session_id).await?;
            candidates.push(ResumeCandidate {
                session_id,
                session_run_id: row.session_run_id,
                kernel_task_run_id: row.kernel_task_run_id,
                adapter_id: row.adapter_id,
                state: row.state,
                attempt_count: row.attempt_count,
                checkpoint,
            });
        }
        Ok(candidates)
    }

    async fn load_latest_checkpoint(
        &self,
        session_id: Uuid,
    ) -> Result<Option<SessionCheckpoint>, RestartResumeRuntimeError> {
        #[derive(SurrealValue)]
        struct LatestCheckpointBindings {
            session_id: Uuid,
        }

        let row: Option<CheckpointRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT checkpoint_id, session_id, model_session_id, \
                             last_event_ledger_seq, compact_state, state_kind, \
                             pending_artifacts, created_at_utc, created_by_process, \
                             schema_version \
                             FROM kernel_session_checkpoint WHERE session_id = $session_id \
                             ORDER BY created_at_utc DESC LIMIT 1;",
                            LatestCheckpointBindings { session_id },
                        )
                        .await
                })
            })
            .await?;

        row.map(CheckpointRow::into_checkpoint).transpose()
    }

    async fn load_events_after_checkpoint(
        &self,
        session_run_id: &str,
        session_id: Uuid,
        last_event_ledger_seq: i64,
    ) -> Result<Vec<EventLedgerRow>, RestartResumeRuntimeError> {
        #[derive(SurrealValue)]
        struct EventsBindings {
            session_run_id: String,
            after_seq: i64,
        }

        let bindings = EventsBindings {
            session_run_id: session_run_id.to_owned(),
            after_seq: last_event_ledger_seq,
        };
        let rows: Vec<LedgerEventRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT event_id, event_sequence, event_type, payload, created_at \
                             FROM kernel_event_ledger WHERE session_run_id = $session_run_id \
                             AND event_sequence > $after_seq ORDER BY event_sequence;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| EventLedgerRow {
                event_id: row.event_id,
                event_sequence: row.event_sequence,
                session_id,
                event_type: row.event_type,
                payload: row.payload,
                created_at: row.created_at,
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

        #[derive(SurrealValue)]
        struct GlobalSequenceBindings {
            after_seq: i64,
            through_seq: i64,
        }

        Ok(self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT VALUE event_sequence FROM kernel_event_ledger \
                             WHERE event_sequence > $after_seq \
                             AND event_sequence <= $through_seq ORDER BY event_sequence;",
                            GlobalSequenceBindings {
                                after_seq,
                                through_seq,
                            },
                        )
                        .await
                })
            })
            .await?)
    }

    /// Runs the adapter-aware external kill seam before the exact durable STOP.
    /// The process-ledger claim itself supplies replay exclusion; terminal rows
    /// simply yield zero on a repeated recovery pass.
    async fn reclaim_orphans(
        &self,
        candidate: &ResumeCandidate,
    ) -> Result<u32, RestartResumeRuntimeError> {
        self.orphan_reclaimer
            .reclaim_session(&candidate.session_run_id)
            .await
    }

    /// Requeues the session and appends the post-failure checkpoint.
    ///
    /// All writes and both idempotency claims run inside one atomic Surreal
    /// statement: the resume applies atomically, a replayed resume that finds
    /// both claims present is a no-op, and any schema rejection rolls back the
    /// statement including its claims.
    async fn resume_candidate(
        &self,
        candidate: &ResumeCandidate,
        checkpoint: &SessionCheckpoint,
        final_state: &Value,
        final_seq: i64,
    ) -> Result<bool, RestartResumeRuntimeError> {
        let queue_claim = recovery_store_write_key(candidate, final_seq, "kernel_session_queue");
        let checkpoint_claim =
            recovery_store_write_key(candidate, final_seq, "kernel_session_checkpoint");
        let new_checkpoint_id = Uuid::now_v7();
        let bindings = ResumeCandidateBindings {
            queue_claim_id: queue_claim.ledger_record_id(),
            queue_side_effect_kind: queue_claim.side_effect_storage_key(),
            checkpoint_claim_id: checkpoint_claim.ledger_record_id(),
            checkpoint_side_effect_kind: checkpoint_claim.side_effect_storage_key(),
            session_id: candidate.session_id,
            session_run_id: candidate.session_run_id.clone(),
            states: resumable_states(),
            event_seq: final_seq,
            new_checkpoint_id: new_checkpoint_id.to_string(),
            new_checkpoint_uuid: new_checkpoint_id,
            model_session_id: checkpoint.model_session_id,
            compact_state: final_state.clone(),
            pending_artifacts: checkpoint.pending_artifacts.clone(),
            created_by_process: i64::from(std::process::id()),
            schema_version: i64::from(checkpoint.schema_version),
        };

        let applied: Option<bool> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "RETURN { \
                             LET $queue_claim = type::record('kernel_idempotency_ledger', $queue_claim_id); \
                             LET $checkpoint_claim = type::record('kernel_idempotency_ledger', $checkpoint_claim_id); \
                             IF record::exists($queue_claim) \
                                AND record::exists($checkpoint_claim) { RETURN false; }; \
                                 IF !record::exists($queue_claim) { \
                                     CREATE $queue_claim CONTENT { \
                                         session_id: $session_id, \
                                         event_seq: $event_seq, \
                                         side_effect_kind: $queue_side_effect_kind, \
                                         applied_at_utc: time::now() \
                                     }; \
                                 }; \
                                 IF !record::exists($checkpoint_claim) { \
                                     CREATE $checkpoint_claim CONTENT { \
                                         session_id: $session_id, \
                                         event_seq: $event_seq, \
                                         side_effect_kind: $checkpoint_side_effect_kind, \
                                         applied_at_utc: time::now() \
                                     }; \
                                 }; \
                                 UPDATE kernel_session_queue SET \
                                     state = 'FAILED', updated_at = time::now() \
                                 WHERE session_run_id = $session_run_id AND state IN $states; \
                                 UPDATE kernel_session_queue SET \
                                     state = 'RETRY_SCHEDULED', claimed_by = NONE, \
                                     lease_expires_at = NONE, available_at = time::now(), \
                                     updated_at = time::now() \
                                 WHERE session_run_id = $session_run_id AND state = 'FAILED'; \
                                 CREATE type::record('kernel_session_checkpoint', $new_checkpoint_id) CONTENT { \
                                     checkpoint_id: $new_checkpoint_uuid, \
                                     session_id: $session_id, \
                                     model_session_id: $model_session_id, \
                                     last_event_ledger_seq: $event_seq, \
                                     compact_state: $compact_state, \
                                     state_kind: 'post_failure', \
                                     pending_artifacts: $pending_artifacts, \
                                     created_at_utc: time::now(), \
                                     created_by_process: $created_by_process, \
                                     schema_version: $schema_version \
                                 } RETURN NONE; \
                             RETURN true; \
                             };",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        Ok(applied.unwrap_or(false))
    }

    async fn record_failure(
        &self,
        report: &mut ResumeReport,
        candidate: &ResumeCandidate,
        error: ResumeError,
    ) -> Result<(), RestartResumeRuntimeError> {
        report
            .sessions_recovery_failed
            .push((candidate.session_id, error.clone()));

        let failure_seq = failure_idempotency_seq(candidate, &error);
        self.apply_idempotent_store_write(
            candidate,
            failure_seq,
            "role_mailbox_message",
            || async {
                self.post_operator_decision(candidate, &error)
                    .await
                    .map_err(|err| err.to_string())
            },
        )
        .await?;
        self.apply_idempotent_store_write(
            candidate,
            failure_seq,
            "kernel_session_queue",
            || async {
                #[derive(SurrealValue)]
                struct FailSessionBindings {
                    session_run_id: String,
                }

                let bindings = FailSessionBindings {
                    session_run_id: candidate.session_run_id.clone(),
                };
                self.storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .execute_returning(
                                    "UPDATE kernel_session_queue SET \
                                     state = 'FAILED', claimed_by = NONE, \
                                     lease_expires_at = NONE, updated_at = time::now() \
                                     WHERE session_run_id = $session_run_id;",
                                    bindings,
                                )
                                .await
                        })
                    })
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
        candidate: &ResumeCandidate,
        error: &ResumeError,
    ) -> Result<(), RestartResumeRuntimeError> {
        #[derive(SurrealValue)]
        struct ExistingDecisionBindings {
            linked_record_id: String,
            session_run_id: String,
        }

        let bindings = ExistingDecisionBindings {
            linked_record_id: candidate.session_id.to_string(),
            session_run_id: candidate.session_run_id.clone(),
        };
        // The previous SQL join walked message -> thread; the record link on
        // `thread_id` expresses the same constraint as a field traversal.
        let existing: Option<Uuid> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT VALUE message_id FROM role_mailbox_message \
                             WHERE message_type = 'decision_request' \
                             AND body.session_run_id = $session_run_id \
                             AND thread_id.linked_record_id = $linked_record_id \
                             ORDER BY created_at_utc LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if existing.is_some() {
            return Ok(());
        }

        let repo = RoleMailboxRepository::new(self.storage.clone());
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

    async fn apply_idempotent_store_write<F, Fut>(
        &self,
        candidate: &ResumeCandidate,
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
            .try_apply(recovery_store_write_key(candidate, event_seq, table), op)
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
        let record_id = report.report_id.to_string();
        let row = RestartResumeReportRow {
            report_id: report.report_id,
            sessions_examined: i64::from(report.sessions_examined),
            sessions_resumed: serde_json::to_value(&report.sessions_resumed)?,
            sessions_recovery_failed: serde_json::to_value(&report.sessions_recovery_failed)?,
            orphan_reclaims: serde_json::to_value(&report.orphan_reclaims)?,
            operator_decision_requests: serde_json::to_value(&report.operator_decision_requests)?,
            fr_events_emitted: report.fr_events_emitted.clone(),
            total_replay_events: i64::try_from(report.total_replay_events).unwrap_or(i64::MAX),
            total_duration_ms: i64::try_from(report.total_duration_ms).unwrap_or(i64::MAX),
            started_at_utc: report.started_at_utc,
            completed_at_utc: report.completed_at_utc,
            schema_version: 2,
        };
        let _: Option<RestartResumeReportRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .upsert_one("kernel_restart_resume_report", &record_id, row)
                        .await
                })
            })
            .await?;
        Ok(())
    }
}

#[derive(SurrealValue)]
struct ResumeCandidateBindings {
    queue_claim_id: String,
    queue_side_effect_kind: String,
    checkpoint_claim_id: String,
    checkpoint_side_effect_kind: String,
    session_id: Uuid,
    session_run_id: String,
    states: Vec<String>,
    event_seq: i64,
    new_checkpoint_id: String,
    new_checkpoint_uuid: Uuid,
    model_session_id: Uuid,
    compact_state: Value,
    pending_artifacts: Vec<String>,
    created_by_process: i64,
    schema_version: i64,
}

/// One `kernel_session_queue` projection used to seed resume candidates.
#[derive(SurrealValue)]
struct CandidateRow {
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    attempt_count: i64,
    created_at: DateTime<Utc>,
}

/// One `kernel_session_checkpoint` projection; field types mirror the
/// `SCHEMAFULL` table definition, so the hand conversions the PostgreSQL
/// version needed are gone.
#[derive(SurrealValue)]
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

impl CheckpointRow {
    fn into_checkpoint(self) -> Result<SessionCheckpoint, RestartResumeRuntimeError> {
        Ok(SessionCheckpoint {
            checkpoint_id: SessionCheckpointId(self.checkpoint_id),
            session_id: self.session_id,
            model_session_id: self.model_session_id,
            last_event_ledger_seq: self.last_event_ledger_seq,
            compact_state: self.compact_state,
            state_kind: parse_checkpoint_state_kind(&self.state_kind)?,
            pending_artifacts: self.pending_artifacts,
            created_at_utc: self.created_at_utc,
            created_by_process: i32::try_from(self.created_by_process).unwrap_or(i32::MAX),
            schema_version: u16::try_from(self.schema_version).unwrap_or(u16::MAX),
        })
    }
}

/// One `kernel_event_ledger` projection for replay.
#[derive(SurrealValue)]
struct LedgerEventRow {
    event_id: String,
    event_sequence: i64,
    event_type: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

/// One `kernel_restart_resume_report` record; mirrors the `SCHEMAFULL` table.
#[derive(SurrealValue)]
struct RestartResumeReportRow {
    report_id: Uuid,
    sessions_examined: i64,
    sessions_resumed: Value,
    sessions_recovery_failed: Value,
    orphan_reclaims: Value,
    operator_decision_requests: Value,
    fr_events_emitted: Vec<String>,
    total_replay_events: i64,
    total_duration_ms: i64,
    started_at_utc: DateTime<Utc>,
    completed_at_utc: DateTime<Utc>,
    schema_version: i64,
}

#[derive(Clone)]
struct ResumeCandidate {
    session_id: Uuid,
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    attempt_count: i64,
    checkpoint: Option<SessionCheckpoint>,
}

struct GlobalReplayResult {
    final_state: Value,
    final_seq: i64,
    applied_count: u32,
}

fn execute_global_replay(
    checkpoint: &SessionCheckpoint,
    events: &[EventLedgerRow],
    global_sequences: &[i64],
) -> Result<GlobalReplayResult, ReplayError> {
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
    Ok(GlobalReplayResult {
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

fn failure_idempotency_seq(candidate: &ResumeCandidate, error: &ResumeError) -> i64 {
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

fn store_write_key(session_id: Uuid, event_seq: i64, table: &str) -> IdempotencyKey {
    IdempotencyKey {
        session_id,
        event_seq,
        side_effect_kind: SideEffectKind::store_write_table(table),
    }
}

/// Builds a restart side-effect key for one durable queue claim generation.
///
/// `kernel_session_queue.attempt_count` is incremented atomically whenever the
/// broker claims or reclaims the queue row. It therefore stays stable while one
/// crashed attempt is replayed, but changes before the same session is retried.
/// Including it in the side-effect target prevents a committed claim for an
/// earlier crash from suppressing orphan reclamation, requeue, checkpoint, or
/// failure handling for a later crash whose event sequence is unchanged.
fn recovery_store_write_key(
    candidate: &ResumeCandidate,
    event_seq: i64,
    table: &str,
) -> IdempotencyKey {
    let target = format!("{table}@recovery-attempt:{}", candidate.attempt_count);
    store_write_key(candidate.session_id, event_seq, &target)
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
    matches!(
        error,
        RestartResumeRuntimeError::Storage(
            SurrealStorageError::Closed
                | SurrealStorageError::Io { .. }
                | SurrealStorageError::ShutdownStillInProgress { .. }
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

    #[derive(Default)]
    struct RecordingStartupKill {
        killed: Mutex<Vec<Uuid>>,
    }

    impl RecordingStartupKill {
        fn killed(&self) -> Vec<Uuid> {
            self.killed.lock().expect("kill spy lock").clone()
        }
    }

    #[async_trait]
    impl StartupProcessCleanup for RecordingStartupKill {
        async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String> {
            self.killed
                .lock()
                .expect("kill spy lock")
                .push(process.process_uuid);
            Ok(())
        }
    }

    struct FailingStartupKill;

    #[async_trait]
    impl StartupProcessCleanup for FailingStartupKill {
        async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String> {
            Err(format!(
                "injected kill failure for {}",
                process.process_uuid
            ))
        }
    }

    struct FailStopOnceStore {
        inner: Arc<SurrealProcessLedgerStore>,
        fail_next_stop: AtomicBool,
    }

    impl FailStopOnceStore {
        fn new(inner: Arc<SurrealProcessLedgerStore>) -> Self {
            Self {
                inner,
                fail_next_stop: AtomicBool::new(true),
            }
        }
    }

    #[async_trait]
    impl RestartReclaimStore for FailStopOnceStore {
        async fn claim_active(
            &self,
            session_run_id: &str,
        ) -> Result<Vec<ReclaimableProcess>, crate::process_ledger::ProcessLedgerError> {
            self.inner
                .active_processes_for_session(session_run_id)
                .await
        }

        async fn mark_cleanup_completed(
            &self,
            process: &ReclaimableProcess,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            self.inner.mark_reclaim_cleanup_completed(process).await
        }

        async fn write_reclaim_stop(
            &self,
            process: &ReclaimableProcess,
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            if self.fail_next_stop.swap(false, Ordering::SeqCst) {
                return Err(crate::process_ledger::ProcessLedgerError::Store(
                    "injected STOP persistence failure after cleanup marker".to_string(),
                ));
            }
            self.inner
                .write_batch(vec![LedgerEvent::Stop(process.reclaim_stop(-1))])
                .await
        }

        async fn abandon(
            &self,
            processes: &[ReclaimableProcess],
        ) -> Result<(), crate::process_ledger::ProcessLedgerError> {
            self.inner.abandon_reclaim_claims(processes).await
        }
    }

    fn cleanup_process(
        engine_kind: crate::process_ledger::ProcessEngineKind,
        sandbox_adapter_id: Option<&str>,
        sandbox_internal_id: Option<&str>,
    ) -> ReclaimableProcess {
        ReclaimableProcess {
            process_uuid: Uuid::now_v7(),
            os_pid: Some(77_777),
            parent_session_id: "SR-mt137-cleanup-contract".to_string(),
            parent_process_id: None,
            sandbox_adapter_id: sandbox_adapter_id.map(str::to_string),
            sandbox_internal_id: sandbox_internal_id.map(str::to_string),
            engine_kind,
            started_at: Utc::now(),
            model_artifact_sha256: None,
            work_profile_id: None,
            owner_role: "mt137-restart-proof".to_string(),
            owner_wp: Some("WP-KERNEL-012".to_string()),
            role_id: None,
            wp_id: Some("WP-KERNEL-012".to_string()),
            mt_id: Some("MT-137".to_string()),
            sandbox_capabilities_snapshot: json!({}),
            metadata_jsonb: json!({}),
            reclaim_claimed_at: Utc::now(),
            reclaim_expected_reason: "reclaim_claimed:mt137-fixture".to_owned(),
            reclaim_expected_killed_reason: "reclaim_killed:mt137-fixture".to_owned(),
            reclaim_cleanup_completed: false,
        }
    }

    #[tokio::test]
    async fn mt137_registry_cleanup_uses_fresh_adapter_restart_contract() {
        use crate::sandbox::{
            SandboxAdapter, Signal, WindowsNativeJailAdapter, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
        };

        let adapter_id = AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID);
        let adapter = Arc::new(WindowsNativeJailAdapter::unavailable(
            "fresh restart-proof adapter",
        ));
        let handle = ProcessHandle {
            id: Uuid::now_v7(),
            adapter_id: adapter_id.clone(),
            pid: Some(44_444),
            sandbox_internal_id: "handshake.mt046.restartproof".to_string(),
            spawned_at_utc: Utc::now(),
        };
        assert!(
            adapter.kill(&handle, Signal::Kill).await.is_err(),
            "ordinary kill on a fresh adapter has no live-handle authority"
        );

        let mut registry = SandboxAdapterRegistry::new(adapter_id);
        registry.register(adapter as Arc<dyn SandboxAdapter>);
        let cleanup = RegistryStartupProcessCleanup::new(Arc::new(registry));
        let mut persisted = cleanup_process(
            crate::process_ledger::ProcessEngineKind::SandboxContainer,
            Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            Some("handshake.mt046.restartproof"),
        );
        persisted.process_uuid = handle.id;
        cleanup
            .cleanup(&persisted)
            .await
            .expect("restart contract does not consult the empty live-handle map");

        cleanup
            .cleanup(&cleanup_process(
                crate::process_ledger::ProcessEngineKind::Candle,
                None,
                None,
            ))
            .await
            .expect("in-process model state conclusively ended with the prior boot");
        assert!(cleanup
            .cleanup(&cleanup_process(
                crate::process_ledger::ProcessEngineKind::HelperSubprocess,
                Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
                None,
            ))
            .await
            .is_err());
    }

    fn test_runner(
        storage: SurrealStorage,
        kill: Arc<RecordingStartupKill>,
    ) -> SurrealRestartResumeRunner {
        let store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let reclaimer = Arc::new(SurrealRestartOrphanReclaimer::new(store, kill));
        SurrealRestartResumeRunner::new(storage, reclaimer)
    }

    #[derive(Clone, SurrealValue)]
    struct TestQueueRow {
        session_run_id: String,
        kernel_task_run_id: String,
        adapter_id: String,
        state: String,
        claimed_by: Option<String>,
        lease_expires_at: Option<DateTime<Utc>>,
        attempt_count: i64,
        available_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Clone, SurrealValue)]
    struct TestProcessRow {
        process_uuid: Uuid,
        os_pid: Option<i64>,
        parent_session_id: Option<String>,
        engine_kind: String,
        started_at: DateTime<Utc>,
        stopped_at: Option<DateTime<Utc>>,
        exit_code: Option<i64>,
        owner_role: String,
        stop_reason: Option<String>,
    }

    #[derive(SurrealValue)]
    struct TestCheckpointEffect {
        checkpoint_id: Uuid,
        last_event_ledger_seq: i64,
        state_kind: String,
    }

    #[derive(SurrealValue)]
    struct TestProcessEffect {
        process_uuid: Uuid,
        stopped_at: Option<DateTime<Utc>>,
        exit_code: Option<i64>,
        stop_reason: Option<String>,
    }

    #[derive(SurrealValue)]
    struct RetryBindings {
        session_run_id: String,
    }

    #[derive(SurrealValue)]
    struct TestSessionBindings {
        session_id: Uuid,
    }

    #[derive(SurrealValue)]
    struct EmptyBindings {}

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid restart-resume test path"),
        )
        .await
        .expect("open embedded restart-resume store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap restart-resume schema");
        storage
    }

    fn active_process(process_uuid: Uuid, session_run_id: &str, os_pid: i64) -> TestProcessRow {
        TestProcessRow {
            process_uuid,
            os_pid: Some(os_pid),
            parent_session_id: Some(session_run_id.to_owned()),
            engine_kind: "helper_subprocess".to_owned(),
            started_at: Utc::now(),
            stopped_at: None,
            exit_code: None,
            owner_role: "mt137-restart-proof".to_owned(),
            stop_reason: None,
        }
    }

    async fn seed_initial_crash(
        storage: &SurrealStorage,
        session_id: Uuid,
        model_session_id: Uuid,
        process_uuid: Uuid,
    ) {
        let session_run_id = format!("SR-{session_id}");
        let queue_record_id = session_run_id.clone();
        let checkpoint_id = Uuid::now_v7();
        let checkpoint_record_id = checkpoint_id.to_string();
        let process_record_id = process_uuid.to_string();
        let now = Utc::now();
        let queue = TestQueueRow {
            session_run_id: session_run_id.clone(),
            kernel_task_run_id: "KTR-MT137-RESTART".to_owned(),
            adapter_id: "mt137-test-adapter".to_owned(),
            state: "RUNNING".to_owned(),
            claimed_by: Some("mt137-first-attempt".to_owned()),
            lease_expires_at: Some(now),
            attempt_count: 1,
            available_at: now,
            created_at: now,
            updated_at: now,
        };
        let checkpoint = CheckpointRow {
            checkpoint_id,
            session_id,
            model_session_id,
            last_event_ledger_seq: 0,
            compact_state: json!({"attempt": 1, "counter": 0}),
            state_kind: "periodic".to_owned(),
            pending_artifacts: vec!["artifact-A".to_owned()],
            created_at_utc: now,
            created_by_process: 137,
            schema_version: 1,
        };
        let process = active_process(process_uuid, &session_run_id, 13_701);

        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    let _: Option<TestQueueRow> = database
                        .upsert_one("kernel_session_queue", &queue_record_id, queue)
                        .await?;
                    let _: Option<CheckpointRow> = database
                        .upsert_one(
                            "kernel_session_checkpoint",
                            &checkpoint_record_id,
                            checkpoint,
                        )
                        .await?;
                    let _: Option<TestProcessRow> = database
                        .upsert_one("kernel_process_lifecycle", &process_record_id, process)
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed first crashed attempt");
    }

    async fn seed_retry_crash(storage: &SurrealStorage, session_run_id: &str, process_uuid: Uuid) {
        let process_record_id = process_uuid.to_string();
        let process = active_process(process_uuid, session_run_id, 13_702);
        let session_run_id = session_run_id.to_owned();
        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE kernel_session_queue SET state = 'RUNNING', \
                             claimed_by = 'mt137-second-attempt', \
                             lease_expires_at = time::now(), attempt_count += 1, \
                             updated_at = time::now() \
                             WHERE session_run_id = $session_run_id RETURN AFTER;",
                            RetryBindings { session_run_id },
                        )
                        .await?;
                    let _: Option<TestProcessRow> = database
                        .upsert_one("kernel_process_lifecycle", &process_record_id, process)
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed retried crashed attempt");
    }

    #[tokio::test]
    async fn mt137_startup_kill_failure_releases_claim_without_false_terminal() {
        let directory = tempfile::tempdir().expect("temporary failed-kill root");
        let storage = open(&directory.path().join("store")).await;
        let session_run_id = "SR-mt137-failed-kill";
        let process_uuid = Uuid::now_v7();
        let process_record_id = process_uuid.to_string();
        let process = active_process(process_uuid, session_run_id, 13_703);
        storage
            .with_data_operation({
                let process_record_id = process_record_id.clone();
                move |database| {
                    Box::pin(async move {
                        let _: Option<TestProcessRow> = database
                            .upsert_one("kernel_process_lifecycle", &process_record_id, process)
                            .await?;
                        Ok(())
                    })
                }
            })
            .await
            .expect("seed failed-kill process");

        let store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let reclaimer = SurrealRestartOrphanReclaimer::new(store, Arc::new(FailingStartupKill));
        let error = reclaimer
            .reclaim_session(session_run_id)
            .await
            .expect_err("kill failure must fail startup orphan recovery");
        assert!(error.to_string().contains("injected kill failure"));

        let row: TestProcessEffect = storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one("kernel_process_lifecycle", &process_record_id)
                        .await
                })
            })
            .await
            .expect("read process after failed kill")
            .expect("failed-kill process still exists");
        assert_eq!(row.process_uuid, process_uuid);
        assert!(row.stopped_at.is_none());
        assert!(row.exit_code.is_none());
        assert!(row.stop_reason.is_none());

        storage.shutdown().await.expect("close failed-kill store");
    }

    #[tokio::test]
    async fn mt137_same_boot_sentinel_is_an_error_until_exact_abandon_converges() {
        let directory = tempfile::tempdir().expect("temporary same-boot convergence root");
        let storage = open(&directory.path().join("store")).await;
        let session_run_id = "SR-mt137-same-boot-convergence";
        let process_uuid = Uuid::now_v7();
        let process_record_id = process_uuid.to_string();
        storage
            .with_data_operation({
                let process_record_id = process_record_id.clone();
                let process = active_process(process_uuid, session_run_id, 13_705);
                move |database| {
                    Box::pin(async move {
                        let _: Option<TestProcessRow> = database
                            .upsert_one("kernel_process_lifecycle", &process_record_id, process)
                            .await?;
                        Ok(())
                    })
                }
            })
            .await
            .expect("seed same-boot process");

        let store = SurrealProcessLedgerStore::new(storage.clone());
        let first_claim = store
            .active_processes_for_session(session_run_id)
            .await
            .expect("first same-boot claim");
        assert_eq!(first_claim.len(), 1);
        let error = store
            .active_processes_for_session(session_run_id)
            .await
            .expect_err("an outstanding same-boot sentinel must not look like zero work");
        assert!(error
            .to_string()
            .contains("same-boot reclaim claims have not converged"));

        store
            .abandon_reclaim_claims(&first_claim)
            .await
            .expect("exact abandon restores the first claim");
        store
            .abandon_reclaim_claims(&first_claim)
            .await
            .expect("zero-row repeat abandon proves canonical convergence");
        let reclaimed = store
            .active_processes_for_session(session_run_id)
            .await
            .expect("same-boot process is reclaimable after convergence");
        assert_eq!(reclaimed.len(), 1);
        store
            .abandon_reclaim_claims(&reclaimed)
            .await
            .expect("release final proof claim");
        drop(store);
        storage
            .shutdown()
            .await
            .expect("close same-boot convergence store");
    }

    #[tokio::test]
    async fn mt137_mixed_abandon_restores_active_and_exact_prior_cleanup_marker() {
        let directory = tempfile::tempdir().expect("temporary mixed abandon root");
        let storage = open(&directory.path().join("store")).await;
        let session_run_id = "SR-mt137-mixed-abandon";
        let active_uuid = Uuid::now_v7();
        let marked_uuid = Uuid::now_v7();
        let active_record_id = active_uuid.to_string();
        let marked_record_id = marked_uuid.to_string();
        let prior_owner = Uuid::now_v7();
        let prior_marker = format!("reclaim_killed:{prior_owner}");
        let prior_stopped_at = Utc::now();
        let active = active_process(active_uuid, session_run_id, 13_706);
        let mut marked = active_process(marked_uuid, session_run_id, 13_707);
        marked.stopped_at = Some(prior_stopped_at);
        marked.stop_reason = Some(prior_marker.clone());
        storage
            .with_data_operation({
                let active_record_id = active_record_id.clone();
                let marked_record_id = marked_record_id.clone();
                move |database| {
                    Box::pin(async move {
                        let _: Option<TestProcessRow> = database
                            .upsert_one("kernel_process_lifecycle", &active_record_id, active)
                            .await?;
                        let _: Option<TestProcessRow> = database
                            .upsert_one("kernel_process_lifecycle", &marked_record_id, marked)
                            .await?;
                        Ok(())
                    })
                }
            })
            .await
            .expect("seed mixed active and cleanup-marker rows");

        let store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let reclaimer =
            SurrealRestartOrphanReclaimer::new(Arc::clone(&store), Arc::new(FailingStartupKill));
        reclaimer
            .reclaim_session(session_run_id)
            .await
            .expect_err("injected cleanup failure abandons the complete mixed claim");

        let (active_after, marked_after): (TestProcessEffect, TestProcessEffect) = storage
            .with_data_operation({
                let active_record_id = active_record_id.clone();
                let marked_record_id = marked_record_id.clone();
                move |database| {
                    Box::pin(async move {
                        let active_after = database
                            .select_one("kernel_process_lifecycle", &active_record_id)
                            .await?
                            .expect("active row survives abandon");
                        let marked_after = database
                            .select_one("kernel_process_lifecycle", &marked_record_id)
                            .await?
                            .expect("marked row survives abandon");
                        Ok((active_after, marked_after))
                    })
                }
            })
            .await
            .expect("read exact mixed abandon state");
        assert!(active_after.stopped_at.is_none());
        assert!(active_after.stop_reason.is_none());
        assert_eq!(marked_after.stopped_at, Some(prior_stopped_at));
        assert_eq!(
            marked_after.stop_reason.as_deref(),
            Some(prior_marker.as_str())
        );
        assert!(marked_after.exit_code.is_none());

        let proof_claim = store
            .active_processes_for_session(session_run_id)
            .await
            .expect("both exact prior states remain reclaimable");
        assert_eq!(proof_claim.len(), 2);
        assert_eq!(
            proof_claim
                .iter()
                .filter(|process| process.reclaim_cleanup_completed)
                .count(),
            1
        );
        store
            .abandon_reclaim_claims(&proof_claim)
            .await
            .expect("release mixed proof claim");
        drop(reclaimer);
        drop(store);
        storage.shutdown().await.expect("close mixed abandon store");
    }

    #[tokio::test]
    async fn mt137_cleanup_success_stop_failure_reopens_without_second_cleanup() {
        let directory = tempfile::tempdir().expect("temporary post-cleanup failure root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let session_run_id = "SR-mt137-post-cleanup-stop-failure";
        let process_uuid = Uuid::now_v7();
        let process_record_id = process_uuid.to_string();
        let process = active_process(process_uuid, session_run_id, 13_704);
        storage
            .with_data_operation({
                let process_record_id = process_record_id.clone();
                move |database| {
                    Box::pin(async move {
                        let _: Option<TestProcessRow> = database
                            .upsert_one("kernel_process_lifecycle", &process_record_id, process)
                            .await?;
                        Ok(())
                    })
                }
            })
            .await
            .expect("seed post-cleanup STOP failure process");

        let cleanup = Arc::new(RecordingStartupKill::default());
        let real_store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let failing_store = Arc::new(FailStopOnceStore::new(Arc::clone(&real_store)));
        let first_reclaimer = SurrealRestartOrphanReclaimer::new(
            Arc::clone(&failing_store),
            Arc::clone(&cleanup) as Arc<dyn StartupProcessCleanup>,
        );
        let error = first_reclaimer
            .reclaim_session(session_run_id)
            .await
            .expect_err("injected STOP failure must fail the first recovery pass");
        assert!(error
            .to_string()
            .contains("injected STOP persistence failure after cleanup marker"));
        assert_eq!(cleanup.killed(), vec![process_uuid]);

        let marker: TestProcessEffect = storage
            .with_data_operation({
                let process_record_id = process_record_id.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .select_one("kernel_process_lifecycle", &process_record_id)
                            .await
                    })
                }
            })
            .await
            .expect("read durable post-cleanup marker")
            .expect("marked process exists");
        assert!(marker.exit_code.is_none());
        assert!(marker
            .stop_reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("reclaim_killed:")));

        drop(first_reclaimer);
        drop(failing_store);
        drop(real_store);
        storage
            .shutdown()
            .await
            .expect("close post-cleanup failure store");
        drop(storage);

        let reopened = open(&path).await;
        let reopened_store = Arc::new(SurrealProcessLedgerStore::new(reopened.clone()));
        let second_reclaimer = SurrealRestartOrphanReclaimer::new(
            reopened_store,
            Arc::clone(&cleanup) as Arc<dyn StartupProcessCleanup>,
        );
        assert_eq!(
            second_reclaimer
                .reclaim_session(session_run_id)
                .await
                .expect("reopened recovery finalizes durable marker"),
            1
        );
        assert_eq!(
            cleanup.killed(),
            vec![process_uuid],
            "durable cleanup marker must suppress a second external cleanup"
        );

        let terminal: TestProcessEffect = reopened
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one("kernel_process_lifecycle", &process_record_id)
                        .await
                })
            })
            .await
            .expect("read terminal process after reopen")
            .expect("terminal process exists");
        assert_eq!(terminal.process_uuid, process_uuid);
        assert_eq!(terminal.exit_code, Some(-1));
        assert_eq!(terminal.stop_reason.as_deref(), Some("reclaim"));
        assert!(terminal.stopped_at.is_some());
        reopened
            .shutdown()
            .await
            .expect("close reopened post-cleanup store");
    }

    #[tokio::test]
    async fn mt137_same_session_second_crash_is_recovered_after_store_reopen() {
        let directory = tempfile::tempdir().expect("temporary restart-resume root");
        let path = directory.path().join("store");
        let session_id = Uuid::now_v7();
        let model_session_id = Uuid::now_v7();
        let session_run_id = format!("SR-{session_id}");
        let first_process_id = Uuid::now_v7();
        let second_process_id = Uuid::now_v7();

        let seeded = open(&path).await;
        seed_initial_crash(&seeded, session_id, model_session_id, first_process_id).await;
        let precrash_store = SurrealProcessLedgerStore::new(seeded.clone());
        let precrash_claim = precrash_store
            .active_processes_for_session_at_with_owner(&session_run_id, Utc::now(), Uuid::nil())
            .await
            .expect("persist first reclaim claim before simulated crash");
        assert_eq!(precrash_claim.len(), 1);
        seeded.shutdown().await.expect("close seeded store");
        drop(precrash_store);
        drop(seeded);

        let first_reopen = open(&path).await;
        let kill_spy = Arc::new(RecordingStartupKill::default());
        let first_runner = test_runner(first_reopen.clone(), Arc::clone(&kill_spy));
        let first_attempt = first_runner
            .load_candidates()
            .await
            .expect("load first crash candidate")
            .pop()
            .expect("first crash candidate exists");
        let first_checkpoint = first_attempt
            .checkpoint
            .clone()
            .expect("first attempt has checkpoint");
        let first_report = first_runner.run().await.expect("recover first crash");
        assert_eq!(first_report.sessions_examined, 1);
        assert_eq!(first_report.sessions_resumed.len(), 1);
        assert_eq!(first_report.orphan_reclaims[0].processes_reclaimed, 1);
        assert_eq!(
            kill_spy.killed(),
            vec![first_process_id],
            "startup must invoke the real kill seam before terminal STOP"
        );
        assert_eq!(
            first_runner
                .reclaim_orphans(&first_attempt)
                .await
                .expect("replay first orphan reclaim"),
            0,
            "the same crash-attempt claim must deduplicate"
        );
        assert!(
            !first_runner
                .resume_candidate(
                    &first_attempt,
                    &first_checkpoint,
                    &first_checkpoint.compact_state,
                    first_checkpoint.last_event_ledger_seq,
                )
                .await
                .expect("replay first resume transaction"),
            "the same crash-attempt resume must be a no-op"
        );
        drop(first_runner);
        first_reopen
            .shutdown()
            .await
            .expect("close first recovery store");
        drop(first_reopen);

        let retry_store = open(&path).await;
        seed_retry_crash(&retry_store, &session_run_id, second_process_id).await;
        let retry_process_store = SurrealProcessLedgerStore::new(retry_store.clone());
        let retry_precrash_claim = retry_process_store
            .active_processes_for_session_at_with_owner(&session_run_id, Utc::now(), Uuid::nil())
            .await
            .expect("persist retry reclaim claim before simulated crash");
        assert_eq!(retry_precrash_claim.len(), 1);
        retry_store
            .shutdown()
            .await
            .expect("close retried crashed store");
        drop(retry_process_store);
        drop(retry_store);

        let second_reopen = open(&path).await;
        let second_runner = test_runner(second_reopen.clone(), Arc::clone(&kill_spy));
        let second_report = second_runner.run().await.expect("recover second crash");
        assert_eq!(second_report.sessions_examined, 1);
        assert_eq!(second_report.sessions_resumed.len(), 1);
        assert_eq!(second_report.sessions_resumed[0].final_seq, 0);
        assert_eq!(second_report.orphan_reclaims[0].processes_reclaimed, 1);
        assert_eq!(
            kill_spy.killed(),
            vec![first_process_id, second_process_id],
            "each crashed attempt must be killed exactly once before terminalization"
        );

        let session_run_id_for_read = session_run_id.clone();
        let (queue, checkpoints, processes, reports): (
            TestQueueRow,
            Vec<TestCheckpointEffect>,
            Vec<TestProcessEffect>,
            Vec<Uuid>,
        ) = second_reopen
            .with_data_operation(move |database| {
                Box::pin(async move {
                    let queue = database
                        .query_first(
                            "SELECT session_run_id, kernel_task_run_id, adapter_id, state, \
                             claimed_by, lease_expires_at, attempt_count, available_at, \
                             created_at, updated_at FROM kernel_session_queue \
                             WHERE session_run_id = $session_run_id LIMIT 1;",
                            RetryBindings {
                                session_run_id: session_run_id_for_read.clone(),
                            },
                        )
                        .await?
                        .expect("queue row survives recovery");
                    let checkpoints = database
                        .query_values(
                            "SELECT checkpoint_id, last_event_ledger_seq, state_kind \
                             FROM kernel_session_checkpoint WHERE session_id = $session_id;",
                            TestSessionBindings { session_id },
                        )
                        .await?;
                    let processes = database
                        .query_values(
                            "SELECT process_uuid, stopped_at, exit_code, stop_reason \
                             FROM kernel_process_lifecycle \
                             WHERE parent_session_id = $session_run_id;",
                            RetryBindings {
                                session_run_id: session_run_id_for_read,
                            },
                        )
                        .await?;
                    let reports = database
                        .query_values(
                            "SELECT VALUE report_id FROM kernel_restart_resume_report;",
                            EmptyBindings {},
                        )
                        .await?;
                    Ok((queue, checkpoints, processes, reports))
                })
            })
            .await
            .expect("read durable restart effects");

        assert_eq!(queue.state, "RETRY_SCHEDULED");
        assert_eq!(queue.attempt_count, 2);
        assert!(queue.claimed_by.is_none());
        assert!(queue.lease_expires_at.is_none());
        assert_eq!(checkpoints.len(), 3);
        assert_eq!(
            checkpoints
                .iter()
                .map(|checkpoint| checkpoint.checkpoint_id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );
        assert_eq!(
            checkpoints
                .iter()
                .filter(|checkpoint| checkpoint.state_kind == "post_failure")
                .count(),
            2
        );
        assert!(checkpoints
            .iter()
            .all(|checkpoint| checkpoint.last_event_ledger_seq == 0));
        assert_eq!(processes.len(), 2);
        assert!(processes.iter().any(|process| {
            process.process_uuid == first_process_id
                && process.stopped_at.is_some()
                && process.exit_code == Some(-1)
                && process.stop_reason.as_deref() == Some("reclaim")
        }));
        assert!(processes.iter().any(|process| {
            process.process_uuid == second_process_id
                && process.stopped_at.is_some()
                && process.exit_code == Some(-1)
                && process.stop_reason.as_deref() == Some("reclaim")
        }));
        assert_eq!(reports.len(), 2);
        assert!(reports.contains(&first_report.report_id));
        assert!(reports.contains(&second_report.report_id));

        drop(second_runner);
        second_reopen
            .shutdown()
            .await
            .expect("close second recovery store");
    }
}
