//! MT-189 contract-path MicroTaskExecutor orchestrator.
//!
//! This module wires the X.2 executor to the X.1 mailbox primitives through
//! explicit queue, lease, router, backpressure, checkpoint, and outcome side
//! effects. The older `crate::mt_executor::executor::run_job` surface remains
//! a single-job in-memory loop for compatibility; this module is the durable
//! orchestration entrypoint required by MT-189.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::fr_event_registry::FrEventId;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::mt_executor::{
    CompletionSignal, EscalationTier, FairScheduler, MicroTaskExecutor, MicroTaskJob,
    MicroTaskJobId, MicroTaskJobState, MicroTaskQueue, MtCancellationReason, MtCoderHandle,
    MtExecutionContext, MtIterationOutcome, MtLoopControl, MtLoopState, MtOutcome, MtOutcomeKind,
    MtOutcomeRecorder,
};
use crate::process_ledger::mt_loop_control::{MtLoopCheckpoint, MtLoopCheckpointRepo};
use crate::role_mailbox::RoleId;
use crate::role_mailbox_v1::{
    ArtifactPointer, BackpressureDecision, BackpressureGuard, CompletionState, DecisionOption,
    DecisionRequestBody, ExecutorIdentity, ExecutorRouter as MailboxExecutorRouter,
    FamilyEscalationTier, LeaseRequest, MessageFamily, MessageType, MicroTaskCompletionReportBody,
    MicroTaskEscalationBody, MicroTaskExecutorContractRef, MicroTaskRef,
    MicroTaskVerificationNeededBody, PriorAttemptRef, RoleMailboxClaimLeaseV1,
    RoleMailboxRepository, RoleMailboxThreadId, RouteDecision,
};

#[derive(Clone)]
pub struct CoderAuthorizationToken {
    executor_identity: ExecutorIdentity,
    issued_at_utc: chrono::DateTime<Utc>,
}

impl CoderAuthorizationToken {
    pub fn for_executor_identity(executor_identity: &ExecutorIdentity) -> Self {
        Self {
            executor_identity: executor_identity.clone(),
            issued_at_utc: Utc::now(),
        }
    }

    pub fn executor_identity(&self) -> &ExecutorIdentity {
        &self.executor_identity
    }

    pub fn issued_at_utc(&self) -> chrono::DateTime<Utc> {
        self.issued_at_utc
    }
}

#[derive(Clone)]
pub struct AuthorizedMtCoder {
    inner: Arc<dyn MtCoderHandle>,
    token: CoderAuthorizationToken,
}

impl AuthorizedMtCoder {
    pub fn new(inner: Arc<dyn MtCoderHandle>, token: CoderAuthorizationToken) -> Self {
        Self { inner, token }
    }

    pub fn token(&self) -> &CoderAuthorizationToken {
        &self.token
    }

    async fn execute(
        &self,
        ctx: MtExecutionContext,
    ) -> Result<crate::mt_executor::MtIterationResult, crate::mt_executor::MtCoderError> {
        self.inner.execute(ctx).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum MtExecutorRunOutcome {
    NoWork,
    Completed {
        job_id: Uuid,
        signal: CompletionSignal,
    },
    AwaitingVerification {
        job_id: Uuid,
        message_id: Uuid,
    },
    Escalated {
        job_id: Uuid,
        new_tier: EscalationTier,
    },
    HardGated {
        job_id: Uuid,
        reason: String,
    },
    Cancelled {
        job_id: Uuid,
        reason: MtCancellationReason,
    },
    Deferred {
        job_id: Option<Uuid>,
        reason: String,
        retry_after_secs: u32,
        backpressure_receipt_id: Option<Uuid>,
    },
    Failed {
        job_id: Option<Uuid>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MtExecutorRunError {
    #[error("deferred: {reason}")]
    Deferred {
        reason: String,
        retry_after_secs: u32,
        backpressure_receipt_id: Option<Uuid>,
    },
    #[error("queue error: {0}")]
    Queue(String),
    #[error("mailbox error: {0}")]
    Mailbox(String),
    #[error("lease error: {0}")]
    Lease(String),
    #[error("routing denied: {0}")]
    RoutingDenied(String),
    #[error("checkpoint error: {0}")]
    Checkpoint(String),
    #[error("outcome error: {0}")]
    Outcome(String),
    #[error("coder error: {0}")]
    Coder(String),
    #[error("flight recorder error: {0}")]
    FlightRecorder(String),
    #[error("missing mailbox thread for job {0}")]
    MissingMailboxThread(Uuid),
}

impl MtExecutorRunError {
    fn into_outcome(self, job_id: Option<Uuid>) -> MtExecutorRunOutcome {
        match self {
            MtExecutorRunError::Deferred {
                reason,
                retry_after_secs,
                backpressure_receipt_id,
            } => MtExecutorRunOutcome::Deferred {
                job_id,
                reason,
                retry_after_secs,
                backpressure_receipt_id,
            },
            other => MtExecutorRunOutcome::Failed {
                job_id,
                reason: other.to_string(),
            },
        }
    }
}

pub struct MtExecutorRunConfig<'a, IO: MtExecutorIo + ?Sized> {
    pub io: &'a IO,
    pub coder: AuthorizedMtCoder,
    pub lease_duration_secs: u32,
    pub max_jobs: u32,
}

impl<'a, IO: MtExecutorIo + ?Sized> MtExecutorRunConfig<'a, IO> {
    pub fn new(io: &'a IO, coder: AuthorizedMtCoder) -> Self {
        Self {
            io,
            coder,
            lease_duration_secs: 300,
            max_jobs: 1,
        }
    }
}

#[async_trait]
pub trait MtExecutorIo: Send + Sync {
    async fn claim_next(
        &self,
        executor_identity: &ExecutorIdentity,
    ) -> Result<Option<MicroTaskJob>, MtExecutorRunError>;

    async fn mark_running(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
    ) -> Result<(), MtExecutorRunError>;

    async fn acquire_mailbox_lease(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease_duration_secs: u32,
    ) -> Result<RoleMailboxClaimLeaseV1, MtExecutorRunError>;

    async fn persist_checkpoint(
        &self,
        checkpoint: &MtLoopCheckpoint,
    ) -> Result<(), MtExecutorRunError>;

    async fn persist_outcome(
        &self,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
    ) -> Result<(), MtExecutorRunError>;

    async fn post_completion_report(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        summary: String,
        evidence_pointers: Vec<String>,
    ) -> Result<Uuid, MtExecutorRunError>;

    async fn post_verification_needed(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError>;

    async fn post_escalation(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError>;

    async fn post_hardgate_decision(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError>;

    async fn update_state(
        &self,
        job_id: MicroTaskJobId,
        new_state: MicroTaskJobState,
        reason: Option<String>,
    ) -> Result<(), MtExecutorRunError>;

    async fn escalate_job(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
    ) -> Result<(), MtExecutorRunError>;

    async fn hard_gate_job(
        &self,
        job_id: MicroTaskJobId,
        reason: String,
        decision_request_message_id: Uuid,
    ) -> Result<(), MtExecutorRunError>;

    async fn release_mailbox_lease(&self, lease_id: Uuid) -> Result<(), MtExecutorRunError>;

    async fn record_exec_error(
        &self,
        job: Option<&MicroTaskJob>,
        executor_identity: &ExecutorIdentity,
        error: &str,
    ) -> Result<(), MtExecutorRunError>;
}

impl MicroTaskExecutor {
    pub async fn run<IO: MtExecutorIo + ?Sized>(
        &self,
        executor_identity: ExecutorIdentity,
        config: MtExecutorRunConfig<'_, IO>,
    ) -> MtExecutorRunOutcome {
        if config.coder.token().executor_identity().session_id != executor_identity.session_id {
            let err = MtExecutorRunError::RoutingDenied(
                "authorized coder token does not match executor session".to_string(),
            );
            let _ = config
                .io
                .record_exec_error(None, &executor_identity, &err.to_string())
                .await;
            return err.into_outcome(None);
        }

        let max_jobs = config.max_jobs.max(1);
        for _ in 0..max_jobs {
            let job = match config.io.claim_next(&executor_identity).await {
                Ok(Some(job)) => job,
                Ok(None) => return MtExecutorRunOutcome::NoWork,
                Err(err) => {
                    let _ = config
                        .io
                        .record_exec_error(None, &executor_identity, &err.to_string())
                        .await;
                    return err.into_outcome(None);
                }
            };

            return self
                .run_claimed_job(executor_identity.clone(), &config, job)
                .await;
        }
        MtExecutorRunOutcome::NoWork
    }

    async fn run_claimed_job<IO: MtExecutorIo + ?Sized>(
        &self,
        executor_identity: ExecutorIdentity,
        config: &MtExecutorRunConfig<'_, IO>,
        mut job: MicroTaskJob,
    ) -> MtExecutorRunOutcome {
        let job_uuid = job.job_id.as_uuid();
        let lease = match config
            .io
            .acquire_mailbox_lease(&job, &executor_identity, config.lease_duration_secs)
            .await
        {
            Ok(lease) => lease,
            Err(err) => {
                let _ = config
                    .io
                    .update_state(
                        job.job_id,
                        MicroTaskJobState::Queued,
                        Some(format!("executor deferred before lease: {err}")),
                    )
                    .await;
                let _ = config
                    .io
                    .record_exec_error(Some(&job), &executor_identity, &err.to_string())
                    .await;
                return err.into_outcome(Some(job_uuid));
            }
        };

        if let Err(err) = config.io.mark_running(&job, &executor_identity).await {
            let _ = config.io.release_mailbox_lease(lease.lease_id).await;
            let _ = config
                .io
                .record_exec_error(Some(&job), &executor_identity, &err.to_string())
                .await;
            return err.into_outcome(Some(job_uuid));
        }

        let token = self.canceller.register(job.job_id);
        let mut consecutive_failures = 0u32;

        loop {
            if token.is_cancelled() {
                return self
                    .cancel_job(config.io, &job, &executor_identity, &lease, token.reason())
                    .await;
            }
            if job.iteration_n >= self.budget.max_iterations {
                return self
                    .fail_job(
                        config.io,
                        &job,
                        &executor_identity,
                        &lease,
                        format!("max iterations ({}) reached", self.budget.max_iterations),
                    )
                    .await;
            }

            let checkpoint = match MtLoopControl::record_checkpoint(
                &job,
                MtLoopState::WaitingForVerifier,
                "mt executor iteration boundary".to_string(),
                vec![],
                &self.budget,
                vec![],
                executor_identity.session_id,
            ) {
                Ok(checkpoint) => checkpoint,
                Err(err) => {
                    return self
                        .fail_job(
                            config.io,
                            &job,
                            &executor_identity,
                            &lease,
                            format!("checkpoint build failed: {err}"),
                        )
                        .await;
                }
            };
            if let Err(err) = config.io.persist_checkpoint(&checkpoint).await {
                return self
                    .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                    .await;
            }

            let ctx = MtExecutionContext {
                iteration_n: job.iteration_n,
                job_id: job.job_id.as_uuid(),
                wp_id: job.wp_id.clone(),
                mt_id: job.mt_id.clone(),
                session_id: executor_identity.session_id,
                compact_summary_from_checkpoint: Some(checkpoint.compact_summary.clone()),
            };

            let iter_res = match config.coder.execute(ctx).await {
                Ok(result) => result,
                Err(err) => {
                    return self
                        .fail_job(
                            config.io,
                            &job,
                            &executor_identity,
                            &lease,
                            format!("coder error: {err}"),
                        )
                        .await;
                }
            };

            if token.is_cancelled() {
                return self
                    .cancel_job(config.io, &job, &executor_identity, &lease, token.reason())
                    .await;
            }

            match iter_res.outcome {
                MtIterationOutcome::Success { summary } => {
                    let outcome = MtOutcome {
                        outcome_kind: MtOutcomeKind::Success,
                        completion_signal: Some(CompletionSignal::Success {
                            summary: summary.clone(),
                        }),
                        run_ledger_ref: None,
                        evidence_pointers: iter_res.evidence_pointers.clone(),
                    };
                    if let Err(err) = config
                        .io
                        .persist_outcome(&job, outcome, executor_identity.session_id)
                        .await
                    {
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }
                    if let Err(err) = config
                        .io
                        .post_completion_report(
                            &job,
                            &executor_identity,
                            summary.clone(),
                            iter_res.evidence_pointers,
                        )
                        .await
                    {
                        if matches!(err, MtExecutorRunError::Deferred { .. }) {
                            return self
                                .defer_job(config.io, &job, &executor_identity, &lease, err)
                                .await;
                        }
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }
                    if let Err(err) = config
                        .io
                        .update_state(
                            job.job_id,
                            MicroTaskJobState::Completed,
                            Some(summary.clone()),
                        )
                        .await
                    {
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }
                    if let Err(err) = config.io.release_mailbox_lease(lease.lease_id).await {
                        let _ = config
                            .io
                            .record_exec_error(Some(&job), &executor_identity, &err.to_string())
                            .await;
                        return err.into_outcome(Some(job_uuid));
                    }
                    return MtExecutorRunOutcome::Completed {
                        job_id: job_uuid,
                        signal: CompletionSignal::Success { summary },
                    };
                }
                MtIterationOutcome::NeedsVerification { reason } => {
                    let outcome = MtOutcome {
                        outcome_kind: MtOutcomeKind::Verification,
                        completion_signal: Some(CompletionSignal::NeedsVerification {
                            reason: reason.clone(),
                        }),
                        run_ledger_ref: None,
                        evidence_pointers: iter_res.evidence_pointers,
                    };
                    if let Err(err) = config
                        .io
                        .persist_outcome(&job, outcome, executor_identity.session_id)
                        .await
                    {
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }
                    let message_id = match config
                        .io
                        .post_verification_needed(&job, &executor_identity, reason.clone())
                        .await
                    {
                        Ok(message_id) => message_id,
                        Err(err) => {
                            if matches!(err, MtExecutorRunError::Deferred { .. }) {
                                return self
                                    .defer_job(config.io, &job, &executor_identity, &lease, err)
                                    .await;
                            }
                            return self
                                .fail_job(
                                    config.io,
                                    &job,
                                    &executor_identity,
                                    &lease,
                                    err.to_string(),
                                )
                                .await;
                        }
                    };
                    if let Err(err) = config
                        .io
                        .update_state(
                            job.job_id,
                            MicroTaskJobState::AwaitingVerification,
                            Some(reason),
                        )
                        .await
                    {
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }
                    let _ = config.io.release_mailbox_lease(lease.lease_id).await;
                    return MtExecutorRunOutcome::AwaitingVerification {
                        job_id: job_uuid,
                        message_id,
                    };
                }
                MtIterationOutcome::Failure { reason } => {
                    consecutive_failures += 1;
                    let outcome = MtOutcome {
                        outcome_kind: MtOutcomeKind::Failure,
                        completion_signal: Some(CompletionSignal::Failure {
                            reason: reason.clone(),
                        }),
                        run_ledger_ref: None,
                        evidence_pointers: iter_res.evidence_pointers,
                    };
                    if let Err(err) = config
                        .io
                        .persist_outcome(&job, outcome, executor_identity.session_id)
                        .await
                    {
                        return self
                            .fail_job(config.io, &job, &executor_identity, &lease, err.to_string())
                            .await;
                    }

                    let decision = self.escalation_router.route_with_budget(
                        &job,
                        consecutive_failures,
                        &self.budget,
                    );
                    match decision {
                        crate::mt_executor::outcome::EscalationDecision::Retry { .. } => {
                            job.iteration_n += 1;
                            continue;
                        }
                        crate::mt_executor::outcome::EscalationDecision::EscalateTo {
                            next_tier,
                            lora_id,
                        } => {
                            let from_tier = job.escalation_tier;
                            if let Err(err) = config
                                .io
                                .post_escalation(
                                    &job,
                                    &executor_identity,
                                    from_tier,
                                    next_tier,
                                    reason.clone(),
                                )
                                .await
                            {
                                if matches!(err, MtExecutorRunError::Deferred { .. }) {
                                    return self
                                        .defer_job(config.io, &job, &executor_identity, &lease, err)
                                        .await;
                                }
                                return self
                                    .fail_job(
                                        config.io,
                                        &job,
                                        &executor_identity,
                                        &lease,
                                        err.to_string(),
                                    )
                                    .await;
                            }
                            if let Err(err) = config
                                .io
                                .escalate_job(
                                    job.job_id,
                                    next_tier,
                                    reason.clone(),
                                    lora_id.clone(),
                                )
                                .await
                            {
                                return self
                                    .fail_job(
                                        config.io,
                                        &job,
                                        &executor_identity,
                                        &lease,
                                        err.to_string(),
                                    )
                                    .await;
                            }
                            let _ = config.io.release_mailbox_lease(lease.lease_id).await;
                            return MtExecutorRunOutcome::Escalated {
                                job_id: job_uuid,
                                new_tier: next_tier,
                            };
                        }
                        crate::mt_executor::outcome::EscalationDecision::HardGate { reason } => {
                            let message_id = match config
                                .io
                                .post_hardgate_decision(&job, &executor_identity, reason.clone())
                                .await
                            {
                                Ok(message_id) => message_id,
                                Err(err) => {
                                    if matches!(err, MtExecutorRunError::Deferred { .. }) {
                                        return self
                                            .defer_job(
                                                config.io,
                                                &job,
                                                &executor_identity,
                                                &lease,
                                                err,
                                            )
                                            .await;
                                    }
                                    return self
                                        .fail_job(
                                            config.io,
                                            &job,
                                            &executor_identity,
                                            &lease,
                                            err.to_string(),
                                        )
                                        .await;
                                }
                            };
                            if let Err(err) = config
                                .io
                                .hard_gate_job(job.job_id, reason.clone(), message_id)
                                .await
                            {
                                return self
                                    .fail_job(
                                        config.io,
                                        &job,
                                        &executor_identity,
                                        &lease,
                                        err.to_string(),
                                    )
                                    .await;
                            }
                            let _ = config.io.release_mailbox_lease(lease.lease_id).await;
                            return MtExecutorRunOutcome::HardGated {
                                job_id: job_uuid,
                                reason,
                            };
                        }
                        crate::mt_executor::outcome::EscalationDecision::Abandon { reason } => {
                            return self
                                .fail_job(config.io, &job, &executor_identity, &lease, reason)
                                .await;
                        }
                    }
                }
            }
        }
    }

    async fn cancel_job<IO: MtExecutorIo + ?Sized>(
        &self,
        io: &IO,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease: &RoleMailboxClaimLeaseV1,
        reason: Option<MtCancellationReason>,
    ) -> MtExecutorRunOutcome {
        let reason = reason.unwrap_or(MtCancellationReason::SessionShutdown);
        let outcome = MtOutcome {
            outcome_kind: MtOutcomeKind::Cancellation,
            completion_signal: None,
            run_ledger_ref: None,
            evidence_pointers: vec![],
        };
        let _ = io
            .persist_outcome(job, outcome, executor_identity.session_id)
            .await;
        let _ = io
            .update_state(
                job.job_id,
                MicroTaskJobState::Cancelled,
                Some(format!("cancelled: {reason:?}")),
            )
            .await;
        let _ = io.release_mailbox_lease(lease.lease_id).await;
        MtExecutorRunOutcome::Cancelled {
            job_id: job.job_id.as_uuid(),
            reason,
        }
    }

    async fn defer_job<IO: MtExecutorIo + ?Sized>(
        &self,
        io: &IO,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease: &RoleMailboxClaimLeaseV1,
        err: MtExecutorRunError,
    ) -> MtExecutorRunOutcome {
        let _ = io
            .update_state(
                job.job_id,
                MicroTaskJobState::Queued,
                Some(format!("MT-189 soft defer: {err}")),
            )
            .await;
        let _ = io
            .record_exec_error(Some(job), executor_identity, &err.to_string())
            .await;
        let _ = io.release_mailbox_lease(lease.lease_id).await;
        err.into_outcome(Some(job.job_id.as_uuid()))
    }

    async fn fail_job<IO: MtExecutorIo + ?Sized>(
        &self,
        io: &IO,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease: &RoleMailboxClaimLeaseV1,
        reason: String,
    ) -> MtExecutorRunOutcome {
        let outcome = MtOutcome {
            outcome_kind: MtOutcomeKind::Failure,
            completion_signal: Some(CompletionSignal::Failure {
                reason: reason.clone(),
            }),
            run_ledger_ref: None,
            evidence_pointers: vec![],
        };
        let _ = io
            .persist_outcome(job, outcome, executor_identity.session_id)
            .await;
        let _ = io
            .update_state(job.job_id, MicroTaskJobState::Failed, Some(reason.clone()))
            .await;
        let _ = io
            .record_exec_error(Some(job), executor_identity, &reason)
            .await;
        let _ = io.release_mailbox_lease(lease.lease_id).await;
        MtExecutorRunOutcome::Failed {
            job_id: Some(job.job_id.as_uuid()),
            reason,
        }
    }
}

pub struct PostgresMtExecutorIo<'a> {
    queue: &'a MicroTaskQueue,
    scheduler: &'a FairScheduler,
    mailbox_repo: &'a RoleMailboxRepository,
    checkpoint_repo: &'a MtLoopCheckpointRepo,
    backpressure: &'a BackpressureGuard,
    flight_recorder: Option<&'a dyn FlightRecorder>,
}

impl<'a> PostgresMtExecutorIo<'a> {
    pub fn new(
        queue: &'a MicroTaskQueue,
        scheduler: &'a FairScheduler,
        mailbox_repo: &'a RoleMailboxRepository,
        checkpoint_repo: &'a MtLoopCheckpointRepo,
        backpressure: &'a BackpressureGuard,
    ) -> Self {
        Self {
            queue,
            scheduler,
            mailbox_repo,
            checkpoint_repo,
            backpressure,
            flight_recorder: None,
        }
    }

    pub fn with_flight_recorder(mut self, recorder: &'a dyn FlightRecorder) -> Self {
        self.flight_recorder = Some(recorder);
        self
    }

    pub fn pool(&self) -> &PgPool {
        self.queue.pool()
    }

    async fn append_checked_message(
        &self,
        thread_id: Uuid,
        message_type: MessageType,
        from_role: RoleId,
        to_roles: Vec<RoleId>,
        body: MessageFamily,
    ) -> Result<Uuid, MtExecutorRunError> {
        let _ = body
            .encode_bounded()
            .map_err(|err| MtExecutorRunError::Mailbox(err.to_string()))?;
        for role in &to_roles {
            self.check_outgoing(role).await?;
        }
        let message = self
            .mailbox_repo
            .append_message(
                RoleMailboxThreadId(thread_id),
                message_type,
                from_role,
                to_roles,
                serde_json::to_value(body).map_err(|err| {
                    MtExecutorRunError::Mailbox(format!("encode message family: {err}"))
                })?,
            )
            .await
            .map_err(|err| MtExecutorRunError::Mailbox(err.to_string()))?;
        Ok(message.message_id.as_uuid())
    }

    async fn check_outgoing(&self, role: &RoleId) -> Result<(), MtExecutorRunError> {
        let (decision, receipt) = self
            .backpressure
            .check_via_repo(self.mailbox_repo, role)
            .await
            .map_err(|err| MtExecutorRunError::Mailbox(err.to_string()))?;
        match decision {
            BackpressureDecision::Allow => Ok(()),
            BackpressureDecision::Deny {
                reason,
                retry_after_secs,
                observed_pending,
            } => Err(MtExecutorRunError::Deferred {
                reason: format!(
                    "backpressure deferred for role {role}: {reason:?}; pending={observed_pending}"
                ),
                retry_after_secs,
                backpressure_receipt_id: receipt.map(|r| r.receipt_id),
            }),
        }
    }
}

#[async_trait]
impl MtExecutorIo for PostgresMtExecutorIo<'_> {
    async fn claim_next(
        &self,
        executor_identity: &ExecutorIdentity,
    ) -> Result<Option<MicroTaskJob>, MtExecutorRunError> {
        let Some(job_id) = self
            .scheduler
            .claim_next_priority(self.queue.pool(), executor_identity.session_id)
            .await
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))?
        else {
            return Ok(None);
        };
        self.queue
            .get_job(job_id)
            .await
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))?
            .ok_or_else(|| MtExecutorRunError::Queue(format!("claimed job {job_id} not found")))
            .map(Some)
    }

    async fn mark_running(
        &self,
        job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
    ) -> Result<(), MtExecutorRunError> {
        self.queue
            .update_state(
                job.job_id,
                MicroTaskJobState::Running,
                Some("MT-189 executor started job".to_string()),
            )
            .await
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))
    }

    async fn acquire_mailbox_lease(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease_duration_secs: u32,
    ) -> Result<RoleMailboxClaimLeaseV1, MtExecutorRunError> {
        let thread_uuid = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let thread_id = RoleMailboxThreadId(thread_uuid);
        let thread = self
            .mailbox_repo
            .get_thread(thread_id)
            .await
            .map_err(|err| MtExecutorRunError::Mailbox(err.to_string()))?
            .ok_or_else(|| {
                MtExecutorRunError::Mailbox(format!("thread {thread_uuid} not found"))
            })?;
        let active = self
            .mailbox_repo
            .get_active_lease_for_thread(thread_id)
            .await
            .map_err(|err| MtExecutorRunError::Mailbox(err.to_string()))?;
        let decision =
            MailboxExecutorRouter::decide(&thread, active.as_ref(), executor_identity, Utc::now());
        let request = LeaseRequest {
            executor_kind: executor_identity.executor_kind,
            role_id: executor_identity.role_id.clone(),
            session_id: executor_identity.session_id,
            lease_duration_secs,
        };
        match decision {
            RouteDecision::MayClaim { .. } => self
                .mailbox_repo
                .acquire_lease(thread_id, request)
                .await
                .map_err(|err| MtExecutorRunError::Lease(err.to_string())),
            RouteDecision::MayRespondInExistingLease { .. } => match active {
                Some(lease) => Ok(lease),
                None => self
                    .mailbox_repo
                    .acquire_lease(thread_id, request)
                    .await
                    .map_err(|err| MtExecutorRunError::Lease(err.to_string())),
            },
            RouteDecision::MayTakeover {
                predecessor_lease_id,
                ..
            } => self
                .mailbox_repo
                .takeover_lease(
                    thread_id,
                    request,
                    predecessor_lease_id,
                    "MT-189 executor router-approved takeover".to_string(),
                )
                .await
                .map_err(|err| MtExecutorRunError::Lease(err.to_string())),
            RouteDecision::MustWait { reason } => Err(MtExecutorRunError::Deferred {
                reason: format!("mailbox route must wait: {reason:?}"),
                retry_after_secs: 5,
                backpressure_receipt_id: None,
            }),
            RouteDecision::Denied { reason, detail } => Err(MtExecutorRunError::RoutingDenied(
                format!("{reason:?}: {detail}"),
            )),
        }
    }

    async fn persist_checkpoint(
        &self,
        checkpoint: &MtLoopCheckpoint,
    ) -> Result<(), MtExecutorRunError> {
        self.checkpoint_repo
            .persist(checkpoint)
            .await
            .map_err(|err| MtExecutorRunError::Checkpoint(err.to_string()))
    }

    async fn persist_outcome(
        &self,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
    ) -> Result<(), MtExecutorRunError> {
        MtOutcomeRecorder::persist(self.queue.pool(), job, outcome, session_id)
            .await
            .map(|_| ())
            .map_err(|err| MtExecutorRunError::Outcome(err.to_string()))
    }

    async fn post_completion_report(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        summary: String,
        evidence_pointers: Vec<String>,
    ) -> Result<Uuid, MtExecutorRunError> {
        let thread_id = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let body = MessageFamily::MicroTaskCompletionReport(MicroTaskCompletionReportBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            outcome_summary: summary,
            artifacts: artifacts_from_evidence(&job.mt_id, evidence_pointers),
            completion_state: CompletionState::Completed,
        });
        self.append_checked_message(
            thread_id,
            MessageType::MicroTaskCompletionReport,
            executor_identity.role_id.clone(),
            vec![RoleId::Orchestrator],
            body,
        )
        .await
    }

    async fn post_verification_needed(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        let thread_id = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let body = MessageFamily::MicroTaskVerificationNeeded(MicroTaskVerificationNeededBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            reason,
            verifier_target_role: RoleId::Validator,
        });
        self.append_checked_message(
            thread_id,
            MessageType::MicroTaskVerificationNeeded,
            executor_identity.role_id.clone(),
            vec![RoleId::Validator],
            body,
        )
        .await
    }

    async fn post_escalation(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        let thread_id = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let body = MessageFamily::MicroTaskEscalation(MicroTaskEscalationBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            escalation_target: to_mailbox_tier(to_tier),
            reason,
            prior_attempts: vec![PriorAttemptRef {
                attempt_id: Uuid::now_v7(),
                tier: to_mailbox_tier(from_tier),
                outcome_summary: format!("failed at {}", from_tier.as_str()),
            }],
        });
        self.append_checked_message(
            thread_id,
            MessageType::MicroTaskEscalation,
            executor_identity.role_id.clone(),
            vec![RoleId::Orchestrator],
            body,
        )
        .await
    }

    async fn post_hardgate_decision(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        let thread_id = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let body = MessageFamily::DecisionRequest(DecisionRequestBody {
            question: format!(
                "HardGate requested for {} {} after escalation exhaustion: {}",
                job.wp_id, job.mt_id, reason
            ),
            options: vec![
                DecisionOption {
                    option_id: "approve_hardgate".to_string(),
                    label: "Approve HardGate".to_string(),
                    detail: Some("Commit the MT to HardGated.".to_string()),
                },
                DecisionOption {
                    option_id: "retry_same_tier".to_string(),
                    label: "Retry Same Tier".to_string(),
                    detail: Some("Return the MT to the current tier.".to_string()),
                },
                DecisionOption {
                    option_id: "abandon_failed".to_string(),
                    label: "Mark Failed".to_string(),
                    detail: Some("Stop the MT and preserve the outcome trail.".to_string()),
                },
            ],
            decision_authority_role: RoleId::Operator,
            deadline_utc: None,
        });
        self.append_checked_message(
            thread_id,
            MessageType::DecisionRequest,
            executor_identity.role_id.clone(),
            vec![RoleId::Operator],
            body,
        )
        .await
    }

    async fn update_state(
        &self,
        job_id: MicroTaskJobId,
        new_state: MicroTaskJobState,
        reason: Option<String>,
    ) -> Result<(), MtExecutorRunError> {
        self.queue
            .update_state(job_id, new_state, reason)
            .await
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))
    }

    async fn escalate_job(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
    ) -> Result<(), MtExecutorRunError> {
        self.queue
            .escalate_with_lora(job_id, new_tier, reason, lora_id)
            .await
            .map(|_| ())
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))
    }

    async fn hard_gate_job(
        &self,
        job_id: MicroTaskJobId,
        reason: String,
        decision_request_message_id: Uuid,
    ) -> Result<(), MtExecutorRunError> {
        self.queue
            .hard_gate_after_mailbox_post(job_id, reason, decision_request_message_id)
            .await
            .map(|_| ())
            .map_err(|err| MtExecutorRunError::Queue(err.to_string()))
    }

    async fn release_mailbox_lease(&self, lease_id: Uuid) -> Result<(), MtExecutorRunError> {
        self.mailbox_repo
            .release_lease(lease_id)
            .await
            .map_err(|err| MtExecutorRunError::Lease(err.to_string()))
    }

    async fn record_exec_error(
        &self,
        job: Option<&MicroTaskJob>,
        executor_identity: &ExecutorIdentity,
        error: &str,
    ) -> Result<(), MtExecutorRunError> {
        let Some(recorder) = self.flight_recorder else {
            return Ok(());
        };
        let mt_id = job.map(|j| j.mt_id.clone()).unwrap_or_default();
        let payload = json!({
            "event_id": FrEventId::MtExecError.as_str(),
            "mt_id": mt_id,
            "error": error,
            "wp_id": job.map(|j| j.wp_id.clone()),
            "job_id": job.map(|j| j.job_id.as_uuid().to_string()),
            "session_id": executor_identity.session_id.to_string(),
        });
        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::MicroTaskLoopFailed,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            payload,
        )
        .with_actor_id(executor_identity.role_id.to_string());
        if let Some(job) = job {
            event = event.with_job_id(job.job_id.to_string());
        }
        recorder
            .record_event(event)
            .await
            .map_err(|err| MtExecutorRunError::FlightRecorder(err.to_string()))
    }
}

fn mt_ref(job: &MicroTaskJob) -> MicroTaskRef {
    MicroTaskRef {
        wp_id: job.wp_id.clone(),
        mt_id: job.mt_id.clone(),
        iteration_n: job.iteration_n,
    }
}

fn contract_ref(job: &MicroTaskJob) -> MicroTaskExecutorContractRef {
    MicroTaskExecutorContractRef {
        job_id: job.job_id.as_uuid(),
        mt_id: job.mt_id.clone(),
        wp_id: job.wp_id.clone(),
    }
}

fn artifacts_from_evidence(mt_id: &str, evidence_pointers: Vec<String>) -> Vec<ArtifactPointer> {
    evidence_pointers
        .into_iter()
        .enumerate()
        .map(|(idx, uri)| ArtifactPointer {
            artifact_id: format!("{mt_id}-artifact-{idx}"),
            uri,
            content_hash: None,
        })
        .collect()
}

fn to_mailbox_tier(tier: EscalationTier) -> FamilyEscalationTier {
    match tier {
        EscalationTier::T7B => FamilyEscalationTier::T7B,
        EscalationTier::T7BAlt => FamilyEscalationTier::T7BAlt,
        EscalationTier::T13B => FamilyEscalationTier::T13B,
        EscalationTier::T13BAlt => FamilyEscalationTier::T13BAlt,
        EscalationTier::T32B => FamilyEscalationTier::T32B,
        EscalationTier::HardGate => FamilyEscalationTier::HardGate,
    }
}
