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
