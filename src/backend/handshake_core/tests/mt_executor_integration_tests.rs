//! MT-189 adversarial integration tests for the contract-path orchestrator.

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::mt_executor::{
    EscalationStep, EscalationTier, FairScheduler, MicroTaskExecutor, MicroTaskJob, MicroTaskJobId,
    MicroTaskJobState, MtCancellationReason, MtCanceller, MtCoderError, MtCoderHandle,
    MtExecutionContext, MtIterationOutcome, MtIterationResult, MtLoopControlBudget, MtOutcome,
    MtOutcomeKind, MtOutcomeRecorder, StarvationConfig,
};
use handshake_core::process_ledger::mt_executor::{
    AuthorizedMtCoder, CoderAuthorizationToken, MtExecutorIo, MtExecutorRunConfig,
    MtExecutorRunError, MtExecutorRunOutcome, PostgresMtExecutorIo,
};
use handshake_core::process_ledger::mt_loop_control::{MtLoopCheckpoint, MtLoopCheckpointRepo};
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    BackpressureConfig, BackpressureDecision, BackpressureGuard, BackpressureReceipt, ClaimMode,
    CompletionState, ExecutorIdentity, ExecutorKind, FamilyEscalationTier, LinkedRecordKind,
    MessageFamily, MessageType, MicroTaskCompletionReportBody, MicroTaskEscalationBody,
    MicroTaskVerificationNeededBody, ResponseAuthorityScope, RoleMailboxClaimLeaseV1,
    RoleMailboxRepository, RoleMailboxThread, RoleMailboxThreadId, TakeoverPolicy,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct RecordingIo {
    jobs: Mutex<HashMap<Uuid, MicroTaskJob>>,
    next_job: Mutex<Option<Uuid>>,
    lease: Mutex<Option<RoleMailboxClaimLeaseV1>>,
    checkpoints: Mutex<Vec<MtLoopCheckpoint>>,
    outcomes: Mutex<Vec<MtOutcome>>,
    completion_reports: Mutex<Vec<MicroTaskCompletionReportBody>>,
    verification_posts: Mutex<Vec<MicroTaskVerificationNeededBody>>,
    escalation_posts: Mutex<Vec<MicroTaskEscalationBody>>,
    hardgate_posts: Mutex<Vec<String>>,
    released_leases: Mutex<Vec<Uuid>>,
    exec_errors: Mutex<Vec<String>>,
    lease_error: Mutex<Option<String>>,
    post_error: Mutex<Option<String>>,
    deferred_receipt: Mutex<Option<BackpressureReceipt>>,
}

impl RecordingIo {
    fn with_job(job: MicroTaskJob) -> Self {
        let job_id = job.job_id.as_uuid();
        let mut jobs = HashMap::new();
        jobs.insert(job_id, job);
        Self {
            jobs: Mutex::new(jobs),
            next_job: Mutex::new(Some(job_id)),
            ..Self::default()
        }
    }

    fn job(&self, job_id: Uuid) -> MicroTaskJob {
        self.jobs.lock().unwrap().get(&job_id).unwrap().clone()
    }

    fn set_lease_error(&self, err: String) {
        *self.lease_error.lock().unwrap() = Some(err);
    }

    fn set_post_error(&self, err: String) {
        *self.post_error.lock().unwrap() = Some(err);
    }

    fn set_deferred_receipt(&self, receipt: BackpressureReceipt) {
        *self.deferred_receipt.lock().unwrap() = Some(receipt);
    }

    fn maybe_post_failure(&self) -> Result<(), MtExecutorRunError> {
        if let Some(err) = self.post_error.lock().unwrap().take() {
            return Err(MtExecutorRunError::Mailbox(err));
        }
        if let Some(receipt) = self.deferred_receipt.lock().unwrap().take() {
            return Err(MtExecutorRunError::Deferred {
                reason: format!("backpressure deferred by receipt {}", receipt.receipt_id),
                retry_after_secs: receipt.retry_after_secs,
                backpressure_receipt_id: Some(receipt.receipt_id),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl MtExecutorIo for RecordingIo {
    async fn claim_next(
        &self,
        executor_identity: &ExecutorIdentity,
    ) -> Result<Option<MicroTaskJob>, MtExecutorRunError> {
        let Some(job_id) = self.next_job.lock().unwrap().take() else {
            return Ok(None);
        };
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id).unwrap();
        job.state = MicroTaskJobState::Claimed;
        job.claimed_by_session = Some(executor_identity.session_id);
        job.claimed_at_utc = Some(Utc::now());
        Ok(Some(job.clone()))
    }

    async fn mark_running(
        &self,
        job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
    ) -> Result<(), MtExecutorRunError> {
        self.update_state(job.job_id, MicroTaskJobState::Running, None)
            .await
    }

    async fn acquire_mailbox_lease(
        &self,
        job: &MicroTaskJob,
        executor_identity: &ExecutorIdentity,
        lease_duration_secs: u32,
    ) -> Result<RoleMailboxClaimLeaseV1, MtExecutorRunError> {
        if let Some(err) = self.lease_error.lock().unwrap().take() {
            return Err(MtExecutorRunError::Lease(err));
        }
        let thread_id = job
            .mailbox_thread_id
            .ok_or_else(|| MtExecutorRunError::MissingMailboxThread(job.job_id.as_uuid()))?;
        let now = Utc::now();
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id,
            holder_executor_kind: executor_identity.executor_kind,
            holder_role_id: executor_identity.role_id.clone(),
            holder_session_id: executor_identity.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        *self.lease.lock().unwrap() = Some(lease.clone());
        Ok(lease)
    }

    async fn persist_checkpoint(
        &self,
        checkpoint: &MtLoopCheckpoint,
    ) -> Result<(), MtExecutorRunError> {
        self.checkpoints.lock().unwrap().push(checkpoint.clone());
        Ok(())
    }

    async fn persist_outcome(
        &self,
        _job: &MicroTaskJob,
        outcome: MtOutcome,
        _session_id: Uuid,
    ) -> Result<(), MtExecutorRunError> {
        self.outcomes.lock().unwrap().push(outcome);
        Ok(())
    }

    async fn post_completion_report(
        &self,
        job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
        summary: String,
        evidence_pointers: Vec<String>,
    ) -> Result<Uuid, MtExecutorRunError> {
        self.maybe_post_failure()?;
        let body = MicroTaskCompletionReportBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            outcome_summary: summary,
            artifacts: evidence_pointers
                .into_iter()
                .enumerate()
                .map(
                    |(idx, uri)| handshake_core::role_mailbox_v1::ArtifactPointer {
                        artifact_id: format!("artifact-{idx}"),
                        uri,
                        content_hash: None,
                    },
                )
                .collect(),
            completion_state: CompletionState::Completed,
        };
        self.completion_reports.lock().unwrap().push(body);
        Ok(Uuid::now_v7())
    }

    async fn post_verification_needed(
        &self,
        job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        self.maybe_post_failure()?;
        let body = MicroTaskVerificationNeededBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            reason,
            verifier_target_role: RoleId::Validator,
        };
        self.verification_posts.lock().unwrap().push(body);
        Ok(Uuid::now_v7())
    }

    async fn post_escalation(
        &self,
        job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
        _from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        self.maybe_post_failure()?;
        let body = MicroTaskEscalationBody {
            mt_ref: mt_ref(job),
            micro_task_executor_contract_ref: contract_ref(job),
            escalation_target: to_mailbox_tier(to_tier),
            reason,
            prior_attempts: vec![],
        };
        self.escalation_posts.lock().unwrap().push(body);
        Ok(Uuid::now_v7())
    }

    async fn post_hardgate_decision(
        &self,
        _job: &MicroTaskJob,
        _executor_identity: &ExecutorIdentity,
        reason: String,
    ) -> Result<Uuid, MtExecutorRunError> {
        self.maybe_post_failure()?;
        self.hardgate_posts.lock().unwrap().push(reason);
        Ok(Uuid::now_v7())
    }

    async fn update_state(
        &self,
        job_id: MicroTaskJobId,
        new_state: MicroTaskJobState,
        reason: Option<String>,
    ) -> Result<(), MtExecutorRunError> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id.as_uuid()).unwrap();
        job.state = new_state;
        job.updated_at_utc = Utc::now();
        job.progress_artifact_ref = reason;
        Ok(())
    }

    async fn escalate_job(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
    ) -> Result<(), MtExecutorRunError> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id.as_uuid()).unwrap();
        let from_tier = job.escalation_tier;
        job.escalation_tier = new_tier;
        job.state = MicroTaskJobState::Escalated;
        job.lora_id = lora_id;
        job.escalation_history.push(EscalationStep {
            from_tier,
            to_tier: new_tier,
            reason,
            recorded_at_utc: Utc::now(),
        });
        Ok(())
    }

    async fn hard_gate_job(
        &self,
        job_id: MicroTaskJobId,
        reason: String,
        _decision_request_message_id: Uuid,
    ) -> Result<(), MtExecutorRunError> {
        self.escalate_job(job_id, EscalationTier::HardGate, reason, None)
            .await
    }

    async fn release_mailbox_lease(&self, lease_id: Uuid) -> Result<(), MtExecutorRunError> {
        self.released_leases.lock().unwrap().push(lease_id);
        if let Some(lease) = self.lease.lock().unwrap().as_mut() {
            if lease.lease_id == lease_id {
                lease.released_at_utc = Some(Utc::now());
            }
        }
        Ok(())
    }

    async fn record_exec_error(
        &self,
        _job: Option<&MicroTaskJob>,
        _executor_identity: &ExecutorIdentity,
        error: &str,
    ) -> Result<(), MtExecutorRunError> {
        self.exec_errors.lock().unwrap().push(error.to_string());
        Ok(())
    }
}

struct SuccessCoder {
    summary: &'static str,
    evidence: Vec<String>,
}

#[async_trait]
impl MtCoderHandle for SuccessCoder {
    async fn execute(&self, _ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError> {
        Ok(MtIterationResult {
            outcome: MtIterationOutcome::Success {
                summary: self.summary.to_string(),
            },
            evidence_pointers: self.evidence.clone(),
        })
    }
}

struct AlwaysFailCoder {
    reason: &'static str,
}

#[async_trait]
impl MtCoderHandle for AlwaysFailCoder {
    async fn execute(&self, _ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError> {
        Ok(MtIterationResult {
            outcome: MtIterationOutcome::Failure {
                reason: self.reason.to_string(),
            },
            evidence_pointers: vec!["artifact://mt-189/failure-log".to_string()],
        })
    }
}

struct CancelsDuringExecutionCoder {
    canceller: Arc<MtCanceller>,
    calls: AtomicU32,
}

#[async_trait]
impl MtCoderHandle for CancelsDuringExecutionCoder {
    async fn execute(&self, ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.canceller.request_cooperative(
            MicroTaskJobId(ctx.job_id),
            MtCancellationReason::OperatorRequested {
                operator_id: "mt-189-test".to_string(),
            },
        );
        Ok(MtIterationResult {
            outcome: MtIterationOutcome::Success {
                summary: "coder finished after cancellation request".to_string(),
            },
            evidence_pointers: vec!["artifact://mt-189/should-not-complete".to_string()],
        })
    }
}

fn identity(session_id: Uuid) -> ExecutorIdentity {
    ExecutorIdentity {
        executor_kind: ExecutorKind::LocalSmallModel,
        role_id: RoleId::Coder,
        session_id,
        capabilities: vec!["mt-executor".to_string()],
    }
}

fn claimed_job(session_id: Uuid) -> MicroTaskJob {
    let mut job = MicroTaskJob::queue(
        "WP-KERNEL-004",
        "MT-189",
        PathBuf::from("task_packets/WP-KERNEL-004/MT-189.json"),
        6,
        vec!["rust".to_string(), "orchestrator".to_string()],
    );
    job.state = MicroTaskJobState::Queued;
    job.claimed_by_session = Some(session_id);
    job.claimed_at_utc = Some(Utc::now());
    job.mailbox_thread_id = Some(Uuid::now_v7());
    job
}

fn executor_with_budget(canceller: Arc<MtCanceller>, max_failures: u32) -> MicroTaskExecutor {
    MicroTaskExecutor::new(
        canceller,
        MtLoopControlBudget {
            max_consecutive_failures: max_failures,
            max_iterations: 6,
            ..MtLoopControlBudget::default()
        },
        handshake_core::mt_executor::EscalationRouter::default(),
    )
}

fn run_config<'a>(
    io: &'a RecordingIo,
    coder: Arc<dyn MtCoderHandle>,
    id: &ExecutorIdentity,
) -> MtExecutorRunConfig<'a, RecordingIo> {
    MtExecutorRunConfig::new(
        io,
        AuthorizedMtCoder::new(coder, CoderAuthorizationToken::for_executor_identity(id)),
    )
}

fn mt_ref(job: &MicroTaskJob) -> handshake_core::role_mailbox_v1::MicroTaskRef {
    handshake_core::role_mailbox_v1::MicroTaskRef {
        wp_id: job.wp_id.clone(),
        mt_id: job.mt_id.clone(),
        iteration_n: job.iteration_n,
    }
}

fn contract_ref(
    job: &MicroTaskJob,
) -> handshake_core::role_mailbox_v1::MicroTaskExecutorContractRef {
    handshake_core::role_mailbox_v1::MicroTaskExecutorContractRef {
        job_id: job.job_id.as_uuid(),
        mt_id: job.mt_id.clone(),
        wp_id: job.wp_id.clone(),
    }
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

#[tokio::test]
async fn mt_189_happy_path_posts_completion_report_and_releases_lease() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let job_id = job.job_id.as_uuid();
    let io = RecordingIo::with_job(job);
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 3);

    let outcome = exec
        .run(
            id.clone(),
            run_config(
                &io,
                Arc::new(SuccessCoder {
                    summary: "done",
                    evidence: vec!["artifact://mt-189/output".to_string()],
                }),
                &id,
            ),
        )
        .await;

    assert!(matches!(outcome, MtExecutorRunOutcome::Completed { .. }));
    assert_eq!(io.completion_reports.lock().unwrap().len(), 1);
    assert_eq!(
        io.completion_reports.lock().unwrap()[0].outcome_summary,
        "done"
    );
    assert_eq!(io.released_leases.lock().unwrap().len(), 1);
    assert_eq!(io.job(job_id).state, MicroTaskJobState::Completed);
    assert!(io
        .outcomes
        .lock()
        .unwrap()
        .iter()
        .any(|o| o.outcome_kind == MtOutcomeKind::Success));
}

#[tokio::test]
async fn mt_189_failure_path_posts_escalation_and_persists_new_tier() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let job_id = job.job_id.as_uuid();
    let io = RecordingIo::with_job(job);
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 1);

    let outcome = exec
        .run(
            id.clone(),
            run_config(
                &io,
                Arc::new(AlwaysFailCoder {
                    reason: "validator rejected patch",
                }),
                &id,
            ),
        )
        .await;

    assert_eq!(
        outcome,
        MtExecutorRunOutcome::Escalated {
            job_id,
            new_tier: EscalationTier::T7BAlt
        }
    );
    assert_eq!(io.escalation_posts.lock().unwrap().len(), 1);
    let stored = io.job(job_id);
    assert_eq!(stored.escalation_tier, EscalationTier::T7BAlt);
    assert_eq!(stored.state, MicroTaskJobState::Escalated);
}

#[tokio::test]
async fn mt_189_cancellation_mid_loop_returns_cancelled_with_no_completion() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let io = RecordingIo::with_job(job);
    let canceller = Arc::new(MtCanceller::new());
    let coder = Arc::new(CancelsDuringExecutionCoder {
        canceller: Arc::clone(&canceller),
        calls: AtomicU32::new(0),
    });
    let exec = executor_with_budget(Arc::clone(&canceller), 3);

    let outcome = exec
        .run(id.clone(), run_config(&io, coder.clone(), &id))
        .await;

    assert_eq!(coder.calls.load(Ordering::SeqCst), 1);
    assert!(matches!(outcome, MtExecutorRunOutcome::Cancelled { .. }));
    assert!(io.completion_reports.lock().unwrap().is_empty());
    assert!(io
        .outcomes
        .lock()
        .unwrap()
        .iter()
        .any(|o| o.outcome_kind == MtOutcomeKind::Cancellation));
}

#[tokio::test]
async fn mt_189_backpressure_denial_returns_deferred_terminal_not_panic() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let io = RecordingIo::with_job(job);
    let decision = BackpressureDecision::Deny {
        reason: handshake_core::role_mailbox_v1::BackpressureDenyReason::InboxFull,
        retry_after_secs: 5,
        observed_pending: 256,
    };
    let receipt =
        BackpressureReceipt::from_decision(&RoleId::Orchestrator, &decision, Utc::now()).unwrap();
    io.set_deferred_receipt(receipt.clone());
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 3);

    let outcome = exec
        .run(
            id.clone(),
            run_config(
                &io,
                Arc::new(SuccessCoder {
                    summary: "should defer",
                    evidence: vec![],
                }),
                &id,
            ),
        )
        .await;

    assert!(matches!(
        outcome,
        MtExecutorRunOutcome::Deferred {
            backpressure_receipt_id: Some(id),
            ..
        } if id == receipt.receipt_id
    ));
    assert!(io.completion_reports.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mt_189_router_failure_returns_structured_failure_not_panic() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let io = RecordingIo::with_job(job);
    io.set_post_error("simulated router persistence outage".to_string());
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 1);

    let outcome = exec
        .run(
            id.clone(),
            run_config(
                &io,
                Arc::new(AlwaysFailCoder {
                    reason: "force escalation",
                }),
                &id,
            ),
        )
        .await;

    assert!(matches!(
        outcome,
        MtExecutorRunOutcome::Failed { ref reason, .. }
            if reason.contains("simulated router persistence outage")
    ));
    assert!(io
        .exec_errors
        .lock()
        .unwrap()
        .iter()
        .any(|e| e.contains("simulated router persistence outage")));
}

#[tokio::test]
async fn mt_189_lease_failure_returns_structured_failure_not_panic() {
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    let job = claimed_job(session_id);
    let holder = Uuid::now_v7();
    let io = RecordingIo::with_job(job);
    io.set_lease_error(format!("lease held by {holder}"));
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 3);

    let outcome = exec
        .run(
            id.clone(),
            run_config(
                &io,
                Arc::new(SuccessCoder {
                    summary: "should not execute without a lease",
                    evidence: vec![],
                }),
                &id,
            ),
        )
        .await;

    assert!(matches!(
        outcome,
        MtExecutorRunOutcome::Failed { ref reason, .. }
            if reason.contains("lease held by") && reason.contains(&holder.to_string())
    ));
}

#[test]
fn mt_189_escalation_tier_mapping_stays_wire_compatible_with_mailbox_family() {
    let pairs = [
        (EscalationTier::T7B, FamilyEscalationTier::T7B),
        (EscalationTier::T7BAlt, FamilyEscalationTier::T7BAlt),
        (EscalationTier::T13B, FamilyEscalationTier::T13B),
        (EscalationTier::T13BAlt, FamilyEscalationTier::T13BAlt),
        (EscalationTier::T32B, FamilyEscalationTier::T32B),
        (EscalationTier::HardGate, FamilyEscalationTier::HardGate),
    ];

    for (mt_tier, mailbox_tier) in pairs {
        let mt_wire = serde_json::to_string(&mt_tier).unwrap();
        let mailbox_wire = serde_json::to_string(&mailbox_tier).unwrap();
        assert_eq!(mt_wire, mailbox_wire);
    }
}

#[test]
fn mt_189_coder_surface_requires_authorization_marker() {
    fn accepts_authorized(_: AuthorizedMtCoder) {}
    let session_id = Uuid::now_v7();
    let id = identity(session_id);
    accepts_authorized(AuthorizedMtCoder::new(
        Arc::new(SuccessCoder {
            summary: "ok",
            evidence: vec![],
        }),
        CoderAuthorizationToken::for_executor_identity(&id),
    ));
}

#[test]
fn mt_189_message_family_bodies_encode_bounded() {
    let session_id = Uuid::now_v7();
    let job = claimed_job(session_id);
    let body = MessageFamily::MicroTaskCompletionReport(MicroTaskCompletionReportBody {
        mt_ref: mt_ref(&job),
        micro_task_executor_contract_ref: contract_ref(&job),
        outcome_summary: "done".to_string(),
        artifacts: vec![],
        completion_state: CompletionState::Completed,
    });
    body.encode_bounded().unwrap();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL and a live Postgres schema"]
async fn mt_189_postgres_orchestrator_persists_completion_and_releases_claim() {
    let pool = mt_189_postgres_pool().await;
    ensure_mt_189_postgres_schema(&pool).await;

    let queue = handshake_core::mt_executor::MicroTaskQueue::new(pool.clone());
    let scheduler = FairScheduler::new(StarvationConfig::default());
    let mailbox_repo = RoleMailboxRepository::new(pool.clone());
    let checkpoint_repo = MtLoopCheckpointRepo::new(pool.clone());
    let backpressure = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 100_000,
        tokens_per_second: 100_000,
        burst_capacity: 100_000,
    });

    let wp = mt_189_unique_wp_id("pg-orchestrator");
    let session_id = Uuid::now_v7();
    let executor_identity = identity(session_id);
    let thread = RoleMailboxThread::open(
        format!("mt-189-postgres-{session_id}"),
        LinkedRecordKind::Mt,
        Some("MT-189".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let thread_id = thread.thread_id;
    mailbox_repo
        .create_thread(thread)
        .await
        .expect("create thread");

    let mut job = MicroTaskJob::queue(
        &wp,
        "MT-189",
        PathBuf::from("task_packets/WP-KERNEL-004/MT-189.json"),
        6,
        vec!["postgres".to_string(), "orchestrator".to_string()],
    );
    job.mailbox_thread_id = Some(thread_id.as_uuid());
    job.escalation_tier = EscalationTier::HardGate;
    job.created_at_utc = Utc::now() - chrono::Duration::days(30);
    job.updated_at_utc = job.created_at_utc;
    let job_id = job.job_id;
    queue.enqueue(&job).await.expect("enqueue job");

    let postgres_io = PostgresMtExecutorIo::new(
        &queue,
        &scheduler,
        &mailbox_repo,
        &checkpoint_repo,
        &backpressure,
    );
    let exec = executor_with_budget(Arc::new(MtCanceller::new()), 3);
    let outcome = exec
        .run(
            executor_identity.clone(),
            MtExecutorRunConfig::new(
                &postgres_io,
                AuthorizedMtCoder::new(
                    Arc::new(SuccessCoder {
                        summary: "postgres completion persisted",
                        evidence: vec!["artifact://mt-189/postgres-output".to_string()],
                    }),
                    CoderAuthorizationToken::for_executor_identity(&executor_identity),
                ),
            ),
        )
        .await;

    assert_eq!(
        outcome,
        MtExecutorRunOutcome::Completed {
            job_id: job_id.as_uuid(),
            signal: handshake_core::mt_executor::CompletionSignal::Success {
                summary: "postgres completion persisted".to_string()
            }
        }
    );

    let stored_job = queue
        .get_job(job_id)
        .await
        .expect("get stored job")
        .expect("stored job exists");
    assert_eq!(stored_job.state, MicroTaskJobState::Completed);
    assert_eq!(stored_job.claimed_by_session, Some(session_id));

    let outcomes = MtOutcomeRecorder::list_for_job(&pool, job_id)
        .await
        .expect("list persisted outcomes");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].outcome_kind, MtOutcomeKind::Success);
    assert_eq!(
        outcomes[0].evidence_pointers,
        vec!["artifact://mt-189/postgres-output".to_string()]
    );

    let checkpoint = checkpoint_repo
        .latest_for_job(job_id)
        .await
        .expect("latest checkpoint query")
        .expect("checkpoint persisted");
    assert_eq!(checkpoint.job_id, job_id);
    assert_eq!(checkpoint.created_by_session, session_id);

    let messages = mailbox_repo
        .list_thread_messages(thread_id)
        .await
        .expect("list thread messages");
    let completion_message = messages
        .iter()
        .find(|message| message.message_type == MessageType::MicroTaskCompletionReport)
        .expect("completion report message exists");
    assert_eq!(completion_message.from_role, RoleId::Coder);
    assert_eq!(completion_message.to_roles, vec![RoleId::Orchestrator]);
    let completion_family: MessageFamily =
        serde_json::from_value(completion_message.body.clone()).expect("completion body decodes");
    match completion_family {
        MessageFamily::MicroTaskCompletionReport(body) => {
            assert_eq!(body.mt_ref.mt_id, "MT-189");
            assert_eq!(body.outcome_summary, "postgres completion persisted");
            assert_eq!(body.completion_state, CompletionState::Completed);
            assert_eq!(body.artifacts.len(), 1);
            assert_eq!(body.artifacts[0].uri, "artifact://mt-189/postgres-output");
        }
        other => panic!("expected completion report family, got {other:?}"),
    }

    assert!(
        mailbox_repo
            .get_active_lease_for_thread(thread_id)
            .await
            .expect("active lease query")
            .is_none(),
        "executor must release its active mailbox lease"
    );
    let (released_leases,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM role_mailbox_claim_lease WHERE thread_id = $1 AND released_at_utc IS NOT NULL",
    )
    .bind(thread_id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count released leases");
    assert_eq!(released_leases, 1);

    cleanup_mt_189_postgres_rows(&pool, &wp, thread_id).await;
}

async fn mt_189_postgres_pool() -> sqlx::PgPool {
    let url =
        std::env::var("POSTGRES_TEST_URL").expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}

async fn ensure_mt_189_postgres_schema(pool: &sqlx::PgPool) {
    let queue = handshake_core::mt_executor::MicroTaskQueue::new(pool.clone());
    queue.ensure_schema().await.expect("ensure queue schema");
    FairScheduler::new(StarvationConfig::default())
        .ensure_schema(pool)
        .await
        .expect("ensure scheduler schema");
    RoleMailboxRepository::new(pool.clone())
        .ensure_schema()
        .await
        .expect("ensure mailbox schema");
    MtOutcomeRecorder::ensure_schema(pool)
        .await
        .expect("ensure outcome schema");
}

fn mt_189_unique_wp_id(test_label: &str) -> String {
    format!("WP-MT189-{}-{}", test_label, Uuid::now_v7().simple())
}

async fn cleanup_mt_189_postgres_rows(
    pool: &sqlx::PgPool,
    wp: &str,
    thread_id: RoleMailboxThreadId,
) {
    let _ = sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(wp)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM role_mailbox_thread WHERE thread_id = $1")
        .bind(thread_id.as_uuid())
        .execute(pool)
        .await;
}
