//! MT-189 MicroTaskExecutor orchestrator integrating job queue + loop control +
//! cancellation + outcome recording.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use super::cancellation::{MtCancellationReason, MtCancellationToken, MtCanceller};
use super::job::{CompletionSignal, MicroTaskJob, MicroTaskJobState};
use super::loop_control::{MtLoopControl, MtLoopControlBudget, MtLoopState};
use super::outcome::{
    enact_decision, EscalationDecision, EscalationMailboxPoster, EscalationRouter, MtOutcome,
    MtOutcomeError, MtOutcomeKind, MtOutcomeRecorder,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtExecutionContext {
    pub iteration_n: u32,
    pub job_id: Uuid,
    pub wp_id: String,
    pub mt_id: String,
    pub session_id: Uuid,
    pub compact_summary_from_checkpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MtIterationOutcome {
    Success { summary: String },
    Failure { reason: String },
    NeedsVerification { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtIterationResult {
    pub outcome: MtIterationOutcome,
    pub evidence_pointers: Vec<String>,
}

#[derive(Debug, Clone, Error)]
pub enum MtCoderError {
    #[error("coder error: {0}")]
    Coder(String),
}

/// Pluggable coder handle. Cluster C wires LlamaCppRuntime/CandleRuntime
/// adapters; tests provide a stub.
#[async_trait]
pub trait MtCoderHandle: Send + Sync {
    async fn execute(&self, ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MtExecutorTerminalOutcome {
    Completed {
        signal: CompletionSignal,
    },
    Escalated {
        new_tier: super::job::EscalationTier,
    },
    HardGated {
        reason: String,
    },
    Cancelled {
        reason: MtCancellationReason,
    },
    Failed {
        reason: String,
    },
}

pub struct MicroTaskExecutor {
    pub canceller: Arc<MtCanceller>,
    pub budget: MtLoopControlBudget,
    pub escalation_router: EscalationRouter,
}

impl MicroTaskExecutor {
    pub fn new(
        canceller: Arc<MtCanceller>,
        budget: MtLoopControlBudget,
        escalation_router: EscalationRouter,
    ) -> Self {
        Self {
            canceller,
            budget,
            escalation_router,
        }
    }

    /// Run a single job to terminal outcome. Production wires the queue +
    /// mailbox + Postgres repository; this entrypoint is fully testable with
    /// a stub coder.
    pub async fn run_job(
        &self,
        job: MicroTaskJob,
        coder: Arc<dyn MtCoderHandle>,
        session_id: Uuid,
    ) -> MtExecutorTerminalOutcome {
        self.run_job_inner(job, coder, session_id, None).await
    }

    /// Run a job with a mailbox poster wired into escalation side effects.
    /// HardGate only returns after the decision_request post succeeds; without
    /// a mailbox thread it fails closed.
    pub async fn run_job_with_mailbox<P: EscalationMailboxPoster>(
        &self,
        job: MicroTaskJob,
        coder: Arc<dyn MtCoderHandle>,
        session_id: Uuid,
        mailbox_poster: &P,
    ) -> MtExecutorTerminalOutcome {
        self.run_job_inner(job, coder, session_id, Some(mailbox_poster))
            .await
    }

    async fn run_job_inner(
        &self,
        mut job: MicroTaskJob,
        coder: Arc<dyn MtCoderHandle>,
        session_id: Uuid,
        mailbox_poster: Option<&dyn EscalationMailboxPoster>,
    ) -> MtExecutorTerminalOutcome {
        let token: MtCancellationToken = self.canceller.register(job.job_id);
        let mut consecutive_failures = 0u32;
        loop {
            // (1) cancellation gate
            if token.is_cancelled() {
                return MtExecutorTerminalOutcome::Cancelled {
                    reason: token
                        .reason()
                        .unwrap_or(MtCancellationReason::SessionShutdown),
                };
            }
            // (2) iteration budget
            if job.iteration_n >= self.budget.max_iterations {
                return MtExecutorTerminalOutcome::Failed {
                    reason: format!("max iterations ({}) reached", self.budget.max_iterations),
                };
            }
            // (3) record checkpoint
            let _ = MtLoopControl::record_checkpoint(
                &job,
                MtLoopState::WaitingForVerifier,
                "iteration".to_string(),
                vec![],
                &self.budget,
                vec![],
                session_id,
            );
            // (4) invoke coder
            let ctx = MtExecutionContext {
                iteration_n: job.iteration_n,
                job_id: job.job_id.as_uuid(),
                wp_id: job.wp_id.clone(),
                mt_id: job.mt_id.clone(),
                session_id,
                compact_summary_from_checkpoint: None,
            };
            let iter_res = match coder.execute(ctx).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = MtOutcomeRecorder::record(
                        &job,
                        MtOutcome {
                            outcome_kind: MtOutcomeKind::Failure,
                            completion_signal: None,
                            run_ledger_ref: None,
                            evidence_pointers: vec![],
                        },
                        session_id,
                    );
                    return MtExecutorTerminalOutcome::Failed {
                        reason: format!("coder error: {e}"),
                    };
                }
            };
            // (5) classify
            match iter_res.outcome {
                MtIterationOutcome::Success { summary } => {
                    let _ = MtOutcomeRecorder::record(
                        &job,
                        MtOutcome {
                            outcome_kind: MtOutcomeKind::Success,
                            completion_signal: Some(CompletionSignal::Success {
                                summary: summary.clone(),
                            }),
                            run_ledger_ref: None,
                            evidence_pointers: iter_res.evidence_pointers,
                        },
                        session_id,
                    );
                    return MtExecutorTerminalOutcome::Completed {
                        signal: CompletionSignal::Success { summary },
                    };
                }
                MtIterationOutcome::Failure { reason: _reason } => {
                    consecutive_failures += 1;
                    let decision = self.escalation_router.route_with_budget(
                        &job,
                        consecutive_failures,
                        &self.budget,
                    );
                    if let Err(err) = self
                        .enact_decision_side_effect(mailbox_poster, &job, &decision)
                        .await
                    {
                        return MtExecutorTerminalOutcome::Failed {
                            reason: err.to_string(),
                        };
                    }
                    match decision {
                        EscalationDecision::Retry { .. } => {
                            job.iteration_n += 1;
                            continue;
                        }
                        EscalationDecision::EscalateTo { next_tier, lora_id } => {
                            job.escalation_tier = next_tier;
                            job.lora_id = lora_id;
                            job.state = MicroTaskJobState::Escalated;
                            return MtExecutorTerminalOutcome::Escalated {
                                new_tier: next_tier,
                            };
                        }
                        EscalationDecision::HardGate { reason } => {
                            return MtExecutorTerminalOutcome::HardGated { reason };
                        }
                        EscalationDecision::Abandon { reason } => {
                            return MtExecutorTerminalOutcome::Failed { reason };
                        }
                    }
                }
                MtIterationOutcome::NeedsVerification { reason: _r } => {
                    job.state = MicroTaskJobState::AwaitingVerification;
                    job.iteration_n += 1;
                    continue;
                }
            }
        }
    }

    async fn enact_decision_side_effect(
        &self,
        mailbox_poster: Option<&dyn EscalationMailboxPoster>,
        job: &MicroTaskJob,
        decision: &EscalationDecision,
    ) -> Result<(), MtOutcomeError> {
        match mailbox_poster {
            Some(poster) => {
                enact_decision(poster, job, decision).await?;
                Ok(())
            }
            None if matches!(decision, EscalationDecision::HardGate { .. }) => {
                Err(MtOutcomeError::MissingHardGateMailboxThread { job_id: job.job_id })
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mt_executor::job::EscalationTier;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct StubCoderSuccess;
    #[async_trait]
    impl MtCoderHandle for StubCoderSuccess {
        async fn execute(
            &self,
            _ctx: MtExecutionContext,
        ) -> Result<MtIterationResult, MtCoderError> {
            Ok(MtIterationResult {
                outcome: MtIterationOutcome::Success {
                    summary: "ok".to_string(),
                },
                evidence_pointers: vec![],
            })
        }
    }

    struct StubCoderAlwaysFail;
    #[async_trait]
    impl MtCoderHandle for StubCoderAlwaysFail {
        async fn execute(
            &self,
            _ctx: MtExecutionContext,
        ) -> Result<MtIterationResult, MtCoderError> {
            Ok(MtIterationResult {
                outcome: MtIterationOutcome::Failure {
                    reason: "boom".to_string(),
                },
                evidence_pointers: vec![],
            })
        }
    }

    struct StubCoderFailThenSucceed {
        calls: AtomicU32,
    }
    #[async_trait]
    impl MtCoderHandle for StubCoderFailThenSucceed {
        async fn execute(
            &self,
            _ctx: MtExecutionContext,
        ) -> Result<MtIterationResult, MtCoderError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < 1 {
                Ok(MtIterationResult {
                    outcome: MtIterationOutcome::Failure {
                        reason: "transient".to_string(),
                    },
                    evidence_pointers: vec![],
                })
            } else {
                Ok(MtIterationResult {
                    outcome: MtIterationOutcome::Success {
                        summary: "recovered".to_string(),
                    },
                    evidence_pointers: vec![],
                })
            }
        }
    }

    fn make_job() -> MicroTaskJob {
        MicroTaskJob::queue("WP-A", "MT-1", PathBuf::from("a.json"), 6, vec![])
    }

    #[tokio::test]
    async fn happy_path_success() {
        let exec = MicroTaskExecutor::new(
            Arc::new(MtCanceller::new()),
            MtLoopControlBudget::default(),
            EscalationRouter::default(),
        );
        let outcome = exec
            .run_job(make_job(), Arc::new(StubCoderSuccess), Uuid::now_v7())
            .await;
        assert!(matches!(
            outcome,
            MtExecutorTerminalOutcome::Completed { .. }
        ));
    }

    #[tokio::test]
    async fn failure_path_escalates_after_budget() {
        let exec = MicroTaskExecutor::new(
            Arc::new(MtCanceller::new()),
            MtLoopControlBudget::default(),
            EscalationRouter::default(),
        );
        let outcome = exec
            .run_job(make_job(), Arc::new(StubCoderAlwaysFail), Uuid::now_v7())
            .await;
        match outcome {
            MtExecutorTerminalOutcome::Escalated { new_tier } => {
                assert_eq!(new_tier, EscalationTier::T7BAlt);
            }
            other => panic!("expected Escalated; got {:?}", other),
        }
    }

    #[tokio::test]
    async fn cancellation_short_circuits_loop() {
        let canceller = Arc::new(MtCanceller::new());
        let job = make_job();
        // Pre-register and immediately cancel.
        canceller.register(job.job_id);
        canceller.request_cooperative(job.job_id, MtCancellationReason::SessionShutdown);
        let exec = MicroTaskExecutor::new(
            Arc::clone(&canceller),
            MtLoopControlBudget::default(),
            EscalationRouter::default(),
        );
        let outcome = exec
            .run_job(job, Arc::new(StubCoderSuccess), Uuid::now_v7())
            .await;
        assert!(matches!(
            outcome,
            MtExecutorTerminalOutcome::Cancelled { .. }
        ));
    }

    #[tokio::test]
    async fn recovery_after_transient_failure() {
        let exec = MicroTaskExecutor::new(
            Arc::new(MtCanceller::new()),
            MtLoopControlBudget::default(),
            EscalationRouter {
                max_failures_per_tier: 3,
                ..EscalationRouter::default()
            },
        );
        let coder = Arc::new(StubCoderFailThenSucceed {
            calls: AtomicU32::new(0),
        });
        let outcome = exec.run_job(make_job(), coder, Uuid::now_v7()).await;
        assert!(matches!(
            outcome,
            MtExecutorTerminalOutcome::Completed { .. }
        ));
    }
}
