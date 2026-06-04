//! WP-KERNEL-004 cluster X.2 (MT-184..MT-189) integration tests.

use async_trait::async_trait;
use handshake_core::mt_executor::{
    cancellation::{MtCancellationReason, MtCanceller},
    executor::{
        MicroTaskExecutor, MtCoderError, MtCoderHandle, MtExecutionContext,
        MtExecutorTerminalOutcome, MtIterationOutcome, MtIterationResult,
    },
    job::{EscalationTier, MicroTaskJob, MicroTaskJobState},
    loop_control::{MtLoopControl, MtLoopControlBudget, MtLoopState},
    outcome::{
        EscalationDecision, EscalationMailboxPoster, EscalationRouter, MtOutcome, MtOutcomeError,
        MtOutcomeKind, MtOutcomeRecorder,
    },
    scheduler::{FairScheduler, StarvationConfig, StarvationGuard},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[test]
fn mt_184_escalation_chain_monotonic() {
    let mut t = EscalationTier::T7B;
    let mut steps = 0;
    while let Some(n) = t.next() {
        t = n;
        steps += 1;
    }
    assert_eq!(steps, 5);
    assert!(EscalationTier::HardGate.next().is_none());
}

#[test]
fn mt_185_loop_budget_enforced() {
    let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    job.iteration_n = 6;
    let res = MtLoopControl::record_checkpoint(
        &job,
        MtLoopState::WaitingForVerifier,
        "ok".to_string(),
        vec![],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    );
    assert!(res.is_err());
}

#[test]
fn mt_186_cooperative_cancellation_idempotent() {
    let c = MtCanceller::new();
    let id = handshake_core::mt_executor::job::MicroTaskJobId::new_v7();
    let token = c.register(id);
    assert!(!token.is_cancelled());
    let r1 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
    let r2 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
    assert!(r1);
    assert!(!r2);
    assert!(token.is_cancelled());
}

#[test]
fn mt_187_starvation_guard_emits_once() {
    let g = StarvationGuard::new(StarvationConfig {
        starvation_threshold_secs: 60,
        ..StarvationConfig::default()
    });
    let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    job.created_at_utc = chrono::Utc::now() - chrono::Duration::seconds(120);
    let s1 = g.check(&job, chrono::Utc::now());
    let s2 = g.check(&job, chrono::Utc::now());
    assert!(s1.is_some());
    assert!(s2.is_none());
}

#[test]
fn mt_187_fair_scheduler_picks_old_high_tier() {
    let s = FairScheduler::new(StarvationConfig::default());
    let mut fresh = MicroTaskJob::queue("W-fresh", "M", PathBuf::from("a.json"), 6, vec![]);
    fresh.escalation_tier = EscalationTier::T7B;
    let mut old = MicroTaskJob::queue("W-old", "M", PathBuf::from("a.json"), 6, vec![]);
    old.escalation_tier = EscalationTier::T32B;
    old.created_at_utc = chrono::Utc::now() - chrono::Duration::minutes(5);
    let cands = vec![fresh, old];
    let pick = s
        .pick_next(&cands, chrono::Utc::now(), &HashMap::new())
        .unwrap();
    assert_eq!(pick.wp_id, "W-old");
}

#[test]
fn mt_188_success_emits_distillation_candidate() {
    let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    let (rec, cand) = MtOutcomeRecorder::record(
        &job,
        MtOutcome {
            outcome_kind: MtOutcomeKind::Success,
            completion_signal: None,
            run_ledger_ref: None,
            evidence_pointers: vec![],
        },
        Uuid::now_v7(),
    );
    assert!(cand.is_some());
    assert_eq!(rec.outcome_kind, MtOutcomeKind::Success);
}

#[test]
fn mt_188_escalation_router_hardgates_at_t32b_exhaustion() {
    let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    job.escalation_tier = EscalationTier::T32B;
    let r = EscalationRouter::default();
    let d = r.route(&job, 3);
    assert!(matches!(d, EscalationDecision::HardGate { .. }));
}

struct StubSuccessCoder;
#[async_trait]
impl MtCoderHandle for StubSuccessCoder {
    async fn execute(&self, _ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError> {
        Ok(MtIterationResult {
            outcome: MtIterationOutcome::Success {
                summary: "done".to_string(),
            },
            evidence_pointers: vec![],
        })
    }
}

struct StubAlwaysFailCoder;
#[async_trait]
impl MtCoderHandle for StubAlwaysFailCoder {
    async fn execute(&self, _ctx: MtExecutionContext) -> Result<MtIterationResult, MtCoderError> {
        Ok(MtIterationResult {
            outcome: MtIterationOutcome::Failure {
                reason: "repeatable failure".to_string(),
            },
            evidence_pointers: vec![],
        })
    }
}

#[derive(Default)]
struct CapturingMailboxPoster {
    posts: Mutex<Vec<String>>,
}

#[async_trait]
impl EscalationMailboxPoster for CapturingMailboxPoster {
    async fn post_micro_task_escalation(
        &self,
        _thread_id: Uuid,
        job: &MicroTaskJob,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        self.posts.lock().unwrap().push(format!(
            "micro_task_escalation:{}:{}:{:?}->{:?}:{}",
            job.wp_id, job.mt_id, from_tier, to_tier, reason
        ));
        Ok(Uuid::now_v7())
    }

    async fn post_hardgate_decision_request(
        &self,
        _thread_id: Uuid,
        job: &MicroTaskJob,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        self.posts.lock().unwrap().push(format!(
            "decision_request:{}:{}:{}",
            job.wp_id, job.mt_id, reason
        ));
        Ok(Uuid::now_v7())
    }
}

#[tokio::test]
async fn mt_189_happy_path_executor() {
    let exec = MicroTaskExecutor::new(
        Arc::new(MtCanceller::new()),
        MtLoopControlBudget::default(),
        EscalationRouter::default(),
    );
    let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    let outcome = exec
        .run_job(job, Arc::new(StubSuccessCoder), Uuid::now_v7())
        .await;
    assert!(matches!(
        outcome,
        handshake_core::mt_executor::executor::MtExecutorTerminalOutcome::Completed { .. }
    ));
}

#[tokio::test]
async fn mt_188_executor_uses_mt185_failure_budget_for_escalation() {
    let exec = MicroTaskExecutor::new(
        Arc::new(MtCanceller::new()),
        MtLoopControlBudget {
            max_consecutive_failures: 1,
            max_iterations: 6,
            ..MtLoopControlBudget::default()
        },
        EscalationRouter {
            max_failures_per_tier: 99,
            ..EscalationRouter::default()
        },
    );
    let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    let outcome = exec
        .run_job(job, Arc::new(StubAlwaysFailCoder), Uuid::now_v7())
        .await;
    match outcome {
        MtExecutorTerminalOutcome::Escalated { new_tier } => {
            assert_eq!(new_tier, EscalationTier::T7BAlt);
        }
        other => panic!("expected budget-driven Escalated, got {:?}", other),
    }
}

#[tokio::test]
async fn mt_188_executor_hardgate_without_mailbox_fails_closed() {
    let exec = MicroTaskExecutor::new(
        Arc::new(MtCanceller::new()),
        MtLoopControlBudget {
            max_consecutive_failures: 1,
            max_iterations: 6,
            ..MtLoopControlBudget::default()
        },
        EscalationRouter::default(),
    );
    let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    job.escalation_tier = EscalationTier::T32B;
    let outcome = exec
        .run_job(job, Arc::new(StubAlwaysFailCoder), Uuid::now_v7())
        .await;
    match outcome {
        MtExecutorTerminalOutcome::Failed { reason } => {
            assert!(
                reason.contains("requires a mailbox_thread_id"),
                "HardGate without mailbox must fail closed with actionable reason, got {reason}"
            );
        }
        other => panic!(
            "expected Failed when HardGate has no mailbox, got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn mt_188_executor_hardgate_with_mailbox_posts_decision_request() {
    let exec = MicroTaskExecutor::new(
        Arc::new(MtCanceller::new()),
        MtLoopControlBudget {
            max_consecutive_failures: 1,
            max_iterations: 6,
            ..MtLoopControlBudget::default()
        },
        EscalationRouter::default(),
    );
    let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    job.escalation_tier = EscalationTier::T32B;
    job.mailbox_thread_id = Some(Uuid::now_v7());
    let poster = CapturingMailboxPoster::default();

    let outcome = exec
        .run_job_with_mailbox(job, Arc::new(StubAlwaysFailCoder), Uuid::now_v7(), &poster)
        .await;

    assert!(matches!(
        outcome,
        MtExecutorTerminalOutcome::HardGated { .. }
    ));
    let posts = poster.posts.lock().unwrap();
    assert_eq!(posts.len(), 1);
    assert!(
        posts[0].starts_with("decision_request:W:M:"),
        "HardGate transition must post decision_request before terminal return, got {:?}",
        *posts
    );
}

#[test]
fn mt_184_job_state_default_is_queued() {
    let j = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    assert_eq!(j.state, MicroTaskJobState::Queued);
}

// Postgres-gated.

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_184_queue_atomic_claim() {
    use handshake_core::mt_executor::queue::MicroTaskQueue;
    let pool = sqlx::PgPool::connect(
        &std::env::var("POSTGRES_TEST_URL")
            .expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set"),
    )
    .await
    .unwrap();
    let q = MicroTaskQueue::new(pool);
    q.ensure_schema().await.unwrap();
    let job = MicroTaskJob::queue("W-A", "MT-1", PathBuf::from("a.json"), 6, vec![]);
    q.enqueue(&job).await.unwrap();
    let claimed = q.claim_next(Uuid::now_v7()).await.unwrap();
    assert!(claimed.is_some());
    let again = q.claim_next(Uuid::now_v7()).await.unwrap();
    assert!(again.is_none(), "second claim must find queue empty");
}
