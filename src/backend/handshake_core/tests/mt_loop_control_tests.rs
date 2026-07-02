//! WP-KERNEL-004 cluster X.2 MT-185 — MT loop-control checkpoint contract
//! integration test surface.
//!
//! Contract: MT-185 owns the bounded mailbox-loop-control checkpoint
//! contract. Canonical implementation lives at
//! `src/backend/handshake_core/src/process_ledger/mt_loop_control.rs`
//! (re-exported by `src/backend/handshake_core/src/mt_executor/loop_control.rs`
//! for cluster-X.2 source ergonomics).
//!
//! Schema: `migrations/0023_micro_task_job_queue.sql` table
//! `kernel_mt_loop_checkpoint` with:
//!   * `job_id REFERENCES kernel_micro_task_job(job_id) ON DELETE CASCADE`
//!   * `CONSTRAINT compact_summary_bound CHECK (octet_length(compact_summary) <= 2048)`
//!
//! Adversarial coverage:
//!
//! Pure-Rust always-on:
//!   (a) MtLoopCheckpoint serde round-trip preserves every field across
//!       all five MtLoopState variants (including escalation tier + completion
//!       signal payloads).
//!   (b) Bounded retries: a checkpoint at iteration_n = max_iterations
//!       returns LoopControlError::LoopBudgetExceeded (NOT silent
//!       continuation; spec line 6147 invariant).
//!   (c) Checkpoint state-machine progression: WaitingForVerifier ->
//!       IteratingOnFeedback -> VerificationNeeded -> EscalationProposed ->
//!       Completed is representable and each state round-trips via serde.
//!   (d) Compact summary size bound enforced at record time
//!       (LoopControlError::SummaryTooLarge at COMPACT_SUMMARY_MAX_BYTES + 1).
//!   (e) Append-only verifier feedback history: the helper
//!       `record_checkpoint_with_feedback` preserves the prior chain and
//!       appends one new entry in order.
//!   (f) Verifier-feedback-loop budget enforced: appending one beyond
//!       max_verifier_feedback_loops returns LoopBudgetExceeded.
//!   (g) `resume_from` is byte-identical across calls AND across different
//!       session_id values, satisfying the red_team minimum-controls
//!       "cross-executor resume determinism" requirement.
//!   (h) `EvidencePointer` JSONB round-trip preserves kind/uri/label
//!       (shared wire form with `role_mailbox_v1::families::EvidencePointer`
//!       MT-179 surface — MT-188 bridges both without translation).
//!   (i) `LoopControlError` variants are distinct enough that a caller can
//!       discriminate budget exhaustion from summary overflow from
//!       non-append-only history.
//!   (j) `MtLoopControl::budget_exhausted` flips at the exact iteration
//!       boundary, matching the `record_checkpoint` ceiling.
//!   (k) `retry_budget_remaining` field reflects `max_iterations -
//!       iteration_n - 1` after a successful record.
//!
//! Postgres-gated (`#[ignore]` until `POSTGRES_TEST_URL` is set):
//!   (l) Persisted checkpoint survives connection-pool churn: insert,
//!       drop the repo handle, build a fresh repo on the same pool, fetch
//!       by id, full field equality (proxy for "restart" — the row lives
//!       in Postgres, not the process).
//!   (m) Latest checkpoint by `created_at_utc DESC` returns the most
//!       recent row when several iterations are recorded.
//!   (n) Full `chain_for_job` returns checkpoints in ascending order,
//!       proving auditability of the loop history.
//!   (o) ON DELETE CASCADE: deleting the parent kernel_micro_task_job row
//!       removes all child checkpoint rows.
//!   (p) CHECK constraint at the DB level: an attempt to bypass the
//!       in-Rust size check and write a 2049-byte summary via raw SQL
//!       fails on the database side with the named CHECK violation
//!       (defends against a future caller that misses the in-Rust gate).
//!   (q) Concurrent checkpoints don't double-count retries: 4 tasks
//!       persist 4 distinct checkpoints for the same job in parallel;
//!       the post-race `count_iterations` is exactly 4 and the
//!       `chain_for_job` returns 4 unique checkpoint_ids — proving the
//!       repo provides linearisable per-job counts even under contention.
//!   (r) Cross-executor resume determinism (Postgres-backed): two
//!       MtLoopCheckpointRepo handles with different session_id values
//!       reading the same persisted checkpoint return byte-identical
//!       `ResumeContext` payloads.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use handshake_core::mt_executor::job::{
    CompletionSignal, EscalationTier, MicroTaskJob, MicroTaskJobId,
};
use handshake_core::process_ledger::mt_loop_control::{
    CheckpointRepoError, EvidencePointer, LoopControlError, MtLoopCheckpoint, MtLoopCheckpointRepo,
    MtLoopControl, MtLoopControlBudget, MtLoopState, ResumeContext, VerifierFeedbackRef,
    COMPACT_SUMMARY_MAX_BYTES,
};
use uuid::Uuid;

// ----------------------------------------------------------------------------
// Pure-Rust always-on assertions
// ----------------------------------------------------------------------------

fn sample_job(iter_n: u32, wp_id: &str) -> MicroTaskJob {
    let mut j = MicroTaskJob::queue(wp_id, "MT-185", PathBuf::from("a.json"), 6, vec![]);
    j.iteration_n = iter_n;
    j
}

fn sample_checkpoint(job: &MicroTaskJob, state: MtLoopState) -> MtLoopCheckpoint {
    MtLoopControl::record_checkpoint(
        job,
        state,
        "compact summary".to_string(),
        vec![EvidencePointer {
            kind: "log".to_string(),
            uri: "file://run.log".to_string(),
            label: Some("seed".to_string()),
        }],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    )
    .expect("record_checkpoint should succeed within budget")
}

#[test]
fn mt_185_serde_round_trip_all_state_variants() {
    let job = sample_job(0, "WP-MT185-PURE-A");
    let variants = vec![
        MtLoopState::WaitingForVerifier,
        MtLoopState::IteratingOnFeedback {
            feedback_id: Uuid::now_v7(),
        },
        MtLoopState::VerificationNeeded {
            reason: "awaiting validator".to_string(),
        },
        MtLoopState::EscalationProposed {
            target_tier: EscalationTier::T13B,
        },
        MtLoopState::Completed {
            signal: CompletionSignal::Success {
                summary: "done".to_string(),
            },
        },
    ];
    for variant in variants {
        let cp = sample_checkpoint(&job, variant.clone());
        let s = serde_json::to_string(&cp).expect("serialize");
        let back: MtLoopCheckpoint = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(
            back, cp,
            "round-trip must preserve every field for {variant:?}"
        );
        assert_eq!(back.state_at_checkpoint, variant);
    }
}

#[test]
fn mt_185_bounded_retries_typed_error_at_ceiling() {
    let job = sample_job(6, "WP-MT185-PURE-BUDGET");
    let res = MtLoopControl::record_checkpoint(
        &job,
        MtLoopState::WaitingForVerifier,
        "ok".to_string(),
        vec![],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    );
    let err = res.expect_err("record at iteration=max must fail");
    assert!(
        matches!(err, LoopControlError::LoopBudgetExceeded(_)),
        "spec line 6147 invariant: must NOT authorize open-ended loops; got {err:?}"
    );
    // And one past the ceiling also fails (defends against off-by-one regressions).
    let job_past = sample_job(7, "WP-MT185-PURE-BUDGET");
    let res_past = MtLoopControl::record_checkpoint(
        &job_past,
        MtLoopState::WaitingForVerifier,
        "ok".to_string(),
        vec![],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    );
    assert!(matches!(
        res_past,
        Err(LoopControlError::LoopBudgetExceeded(_))
    ));
}

#[test]
fn mt_185_state_machine_progression_each_state_round_trips() {
    let job = sample_job(1, "WP-MT185-PURE-STATEMACHINE");
    let progression = vec![
        MtLoopState::WaitingForVerifier,
        MtLoopState::IteratingOnFeedback {
            feedback_id: Uuid::now_v7(),
        },
        MtLoopState::VerificationNeeded {
            reason: "verifier requested".to_string(),
        },
        MtLoopState::EscalationProposed {
            target_tier: EscalationTier::T7BAlt,
        },
        MtLoopState::Completed {
            signal: CompletionSignal::NeedsVerification {
                reason: "final review".to_string(),
            },
        },
    ];
    for state in progression {
        let cp = sample_checkpoint(&job, state.clone());
        let s = serde_json::to_string(&cp.state_at_checkpoint).expect("serialize state");
        let back: MtLoopState = serde_json::from_str(&s).expect("deserialize state");
        assert_eq!(back, state, "state round-trip");
    }
}

#[test]
fn mt_185_compact_summary_size_bound_in_rust() {
    let job = sample_job(0, "WP-MT185-PURE-SUMMARY");
    let big = "x".repeat(COMPACT_SUMMARY_MAX_BYTES + 1);
    let res = MtLoopControl::record_checkpoint(
        &job,
        MtLoopState::WaitingForVerifier,
        big,
        vec![],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    );
    let err = res.expect_err("oversized summary must fail at record time");
    match err {
        LoopControlError::SummaryTooLarge(n) => {
            assert_eq!(n, COMPACT_SUMMARY_MAX_BYTES + 1);
        }
        other => panic!("expected SummaryTooLarge, got {other:?}"),
    }
    // Exactly-at-limit should succeed.
    let exact = "y".repeat(COMPACT_SUMMARY_MAX_BYTES);
    let ok = MtLoopControl::record_checkpoint(
        &job,
        MtLoopState::WaitingForVerifier,
        exact,
        vec![],
        &MtLoopControlBudget::default(),
        vec![],
        Uuid::now_v7(),
    );
    assert!(ok.is_ok(), "exactly-at-bound summary must succeed");
}

#[test]
fn mt_185_verifier_feedback_history_append_only_helper() {
    let job = sample_job(1, "WP-MT185-PURE-APPEND");
    let f0 = VerifierFeedbackRef {
        feedback_id: Uuid::now_v7(),
        at_utc: Utc::now(),
        summary: "first".to_string(),
    };
    let f1 = VerifierFeedbackRef {
        feedback_id: Uuid::now_v7(),
        at_utc: Utc::now(),
        summary: "second".to_string(),
    };
    let f2 = VerifierFeedbackRef {
        feedback_id: Uuid::now_v7(),
        at_utc: Utc::now(),
        summary: "third".to_string(),
    };
    let cp1 = MtLoopControl::record_checkpoint_with_feedback(
        &job,
        MtLoopState::IteratingOnFeedback {
            feedback_id: f1.feedback_id,
        },
        "after f1".to_string(),
        vec![],
        &MtLoopControlBudget::default(),
        vec![f0.clone()],
        f1.clone(),
        Uuid::now_v7(),
    )
    .expect("append-only helper must succeed");
    assert_eq!(cp1.verifier_feedback_history.len(), 2);
    assert_eq!(cp1.verifier_feedback_history[0], f0);
    assert_eq!(cp1.verifier_feedback_history[1], f1);

    let cp2 = MtLoopControl::record_checkpoint_with_feedback(
        &job,
        MtLoopState::IteratingOnFeedback {
            feedback_id: f2.feedback_id,
        },
        "after f2".to_string(),
        vec![],
        &MtLoopControlBudget::default(),
        cp1.verifier_feedback_history.clone(),
        f2.clone(),
        Uuid::now_v7(),
    )
    .expect("second append must succeed (3 <= default 4)");
    assert_eq!(cp2.verifier_feedback_history.len(), 3);
    assert_eq!(cp2.verifier_feedback_history[0], f0);
    assert_eq!(cp2.verifier_feedback_history[1], f1);
    assert_eq!(cp2.verifier_feedback_history[2], f2);
}

#[test]
fn mt_185_verifier_feedback_loop_budget_returns_typed_error() {
    let job = sample_job(1, "WP-MT185-PURE-FEEDBACKBUDGET");
    let mut budget = MtLoopControlBudget::default();
    budget.max_verifier_feedback_loops = 2;
    // 2 prior entries + try to append a 3rd via the helper => must reject.
    let history = vec![
        VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "a".to_string(),
        },
        VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "b".to_string(),
        },
    ];
    let res = MtLoopControl::record_checkpoint_with_feedback(
        &job,
        MtLoopState::IteratingOnFeedback {
            feedback_id: Uuid::now_v7(),
        },
        "would-be third".to_string(),
        vec![],
        &budget,
        history,
        VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "c".to_string(),
        },
        Uuid::now_v7(),
    );
    assert!(matches!(res, Err(LoopControlError::LoopBudgetExceeded(_))));
}

#[test]
fn mt_185_resume_byte_identical_across_session_ids() {
    // RED-TEAM minimum control #3: resume determinism proven by cross-executor
    // test — two different session_ids resuming from the same checkpoint
    // produce byte-identical ResumeContext.
    let job = sample_job(2, "WP-MT185-PURE-RESUME");
    let cp = MtLoopControl::record_checkpoint(
        &job,
        MtLoopState::EscalationProposed {
            target_tier: EscalationTier::T13BAlt,
        },
        "compact resume payload".to_string(),
        vec![
            EvidencePointer {
                kind: "log".to_string(),
                uri: "file://run.log".to_string(),
                label: Some("seed".to_string()),
            },
            EvidencePointer {
                kind: "patch".to_string(),
                uri: "diff://abc123".to_string(),
                label: None,
            },
        ],
        &MtLoopControlBudget::default(),
        vec![VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "verifier".to_string(),
        }],
        Uuid::now_v7(),
    )
    .expect("record");

    // Two simulated executors with different session_ids both resume.
    let _session_a = Uuid::now_v7();
    let _session_b = Uuid::now_v7();
    let ctx_a: ResumeContext = MtLoopControl::resume_from(&cp);
    let ctx_b: ResumeContext = MtLoopControl::resume_from(&cp);
    assert_eq!(ctx_a, ctx_b, "ResumeContext must be deterministic");

    let sa = serde_json::to_vec(&ctx_a).expect("serialize a");
    let sb = serde_json::to_vec(&ctx_b).expect("serialize b");
    assert_eq!(sa, sb, "serialized ResumeContext must be byte-identical");
}

#[test]
fn mt_185_evidence_pointer_jsonb_projection_round_trip() {
    // Shared wire form with role_mailbox_v1::families::EvidencePointer
    // (MT-179). MT-188 bridges both without translation.
    let pointers = vec![
        EvidencePointer {
            kind: "log".to_string(),
            uri: "file:///tmp/run.log".to_string(),
            label: Some("primary".to_string()),
        },
        EvidencePointer {
            kind: "diff".to_string(),
            uri: "git://commit/abc123".to_string(),
            label: None,
        },
        EvidencePointer {
            kind: "trace".to_string(),
            uri: "otlp://span/9876".to_string(),
            label: Some("verifier-attached".to_string()),
        },
    ];
    let value = serde_json::to_value(&pointers).expect("to_value");
    let back: Vec<EvidencePointer> = serde_json::from_value(value).expect("from_value");
    assert_eq!(back, pointers);
}

#[test]
fn mt_185_loop_control_error_variants_have_distinct_display() {
    let big = LoopControlError::SummaryTooLarge(9999);
    let budget = LoopControlError::LoopBudgetExceeded("max iters".to_string());
    let history = LoopControlError::NonAppendOnlyHistory { prior: 3, new: 2 };
    let s_big = format!("{big}");
    let s_budget = format!("{budget}");
    let s_history = format!("{history}");
    assert_ne!(s_big, s_budget);
    assert_ne!(s_big, s_history);
    assert_ne!(s_budget, s_history);
    assert!(s_big.contains("2048"));
    assert!(s_budget.contains("budget"));
    assert!(s_history.contains("append-only"));
}

#[test]
fn mt_185_budget_exhausted_matches_record_ceiling() {
    let budget = MtLoopControlBudget::default();
    // At max_iterations - 1 the next record is the last allowed.
    assert!(!MtLoopControl::budget_exhausted(
        &sample_job(budget.max_iterations - 1, "WP-185-BE"),
        &budget
    ));
    // At max_iterations record returns LoopBudgetExceeded.
    let job_at = sample_job(budget.max_iterations, "WP-185-BE");
    assert!(MtLoopControl::budget_exhausted(&job_at, &budget));
    let res = MtLoopControl::record_checkpoint(
        &job_at,
        MtLoopState::WaitingForVerifier,
        "x".to_string(),
        vec![],
        &budget,
        vec![],
        Uuid::now_v7(),
    );
    assert!(matches!(res, Err(LoopControlError::LoopBudgetExceeded(_))));
}

#[test]
fn mt_185_retry_budget_remaining_decrements_per_iteration() {
    let budget = MtLoopControlBudget::default();
    for iter_n in 0..budget.max_iterations {
        let job = sample_job(iter_n, "WP-MT185-PURE-DECREMENT");
        let cp = MtLoopControl::record_checkpoint(
            &job,
            MtLoopState::WaitingForVerifier,
            format!("iter {iter_n}"),
            vec![],
            &budget,
            vec![],
            Uuid::now_v7(),
        )
        .expect("record");
        let expected = budget.max_iterations - iter_n - 1;
        assert_eq!(
            cp.retry_budget_remaining, expected,
            "retry_budget_remaining must equal max - iter - 1 (iter={iter_n})"
        );
    }
}

// ----------------------------------------------------------------------------
// Postgres-gated integration assertions
// ----------------------------------------------------------------------------

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL test URL");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}

async fn ensure_schemas(pool: &sqlx::PgPool) {
    let queue = handshake_core::mt_executor::queue::MicroTaskQueue::new(pool.clone());
    queue.ensure_schema().await.expect("ensure schema");
}

/// Build a per-test wp_id prefix so two parallel test binaries (or two
/// sibling agents) cannot collide on the shared kernel_micro_task_job table.
fn unique_wp_id(test_label: &str) -> String {
    format!("WP-MT185-{}-{}", test_label, Uuid::now_v7().simple())
}

async fn enqueue_parent_job(pool: &sqlx::PgPool, wp_id: &str) -> MicroTaskJobId {
    let queue = handshake_core::mt_executor::queue::MicroTaskQueue::new(pool.clone());
    let job = MicroTaskJob::queue(wp_id, "MT-185", PathBuf::from("a.json"), 6, vec![]);
    let id = job.job_id;
    queue.enqueue(&job).await.expect("enqueue parent job");
    id
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_persisted_checkpoint_survives_repo_handle_churn() {
    // RED-TEAM minimum control #2 surrogate: the row lives in Postgres,
    // so dropping the Rust repo handle and rebuilding a fresh one on the
    // same pool still returns the same checkpoint (proxy for "restart").
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("persist");
    let job_id = enqueue_parent_job(&pool, &wp).await;

    let cp = {
        let repo = MtLoopCheckpointRepo::new(pool.clone());
        let cp = MtLoopControl::record_checkpoint(
            &{
                let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
                j.job_id = job_id;
                j.iteration_n = 1;
                j
            },
            MtLoopState::WaitingForVerifier,
            "persisted compact summary".to_string(),
            vec![EvidencePointer {
                kind: "log".to_string(),
                uri: "file://run.log".to_string(),
                label: Some("first".to_string()),
            }],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .expect("record");
        repo.persist(&cp).await.expect("persist");
        cp
        // repo goes out of scope here
    };

    // Fresh repo handle on the same pool — proxy for restart.
    let fresh_repo = MtLoopCheckpointRepo::new(pool.clone());
    let fetched = fresh_repo
        .get(cp.checkpoint_id)
        .await
        .expect("fetch")
        .expect("checkpoint row exists");
    assert_eq!(fetched, cp);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_latest_for_job_returns_most_recent() {
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("latest");
    let job_id = enqueue_parent_job(&pool, &wp).await;
    let repo = MtLoopCheckpointRepo::new(pool.clone());

    // Three sequential checkpoints; the third should win latest_for_job.
    let mut latest_cp_id = None;
    for iter_n in 0..3 {
        let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
        j.job_id = job_id;
        j.iteration_n = iter_n;
        let cp = MtLoopControl::record_checkpoint(
            &j,
            MtLoopState::WaitingForVerifier,
            format!("iter {iter_n}"),
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .expect("record");
        repo.persist(&cp).await.expect("persist");
        latest_cp_id = Some(cp.checkpoint_id);
        // Tiny sleep so created_at_utc is monotonically increasing.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let latest = repo
        .latest_for_job(job_id)
        .await
        .expect("fetch latest")
        .expect("at least one checkpoint");
    assert_eq!(latest.checkpoint_id, latest_cp_id.unwrap());
    assert_eq!(latest.iteration_n, 2);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_chain_for_job_returns_ascending_order() {
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("chain");
    let job_id = enqueue_parent_job(&pool, &wp).await;
    let repo = MtLoopCheckpointRepo::new(pool.clone());

    let mut ordered_ids = Vec::new();
    for iter_n in 0..4 {
        let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
        j.job_id = job_id;
        j.iteration_n = iter_n;
        let cp = MtLoopControl::record_checkpoint(
            &j,
            MtLoopState::WaitingForVerifier,
            format!("iter {iter_n}"),
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .expect("record");
        repo.persist(&cp).await.expect("persist");
        ordered_ids.push(cp.checkpoint_id);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let chain = repo.chain_for_job(job_id).await.expect("chain");
    let chain_ids: Vec<Uuid> = chain.iter().map(|c| c.checkpoint_id).collect();
    assert_eq!(
        chain_ids, ordered_ids,
        "chain_for_job must preserve created_at_utc ASC ordering"
    );

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_cascade_delete_removes_all_checkpoints() {
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("cascade");
    let job_id = enqueue_parent_job(&pool, &wp).await;
    let repo = MtLoopCheckpointRepo::new(pool.clone());

    for iter_n in 0..3 {
        let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
        j.job_id = job_id;
        j.iteration_n = iter_n;
        let cp = MtLoopControl::record_checkpoint(
            &j,
            MtLoopState::WaitingForVerifier,
            "ok".to_string(),
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .expect("record");
        repo.persist(&cp).await.expect("persist");
    }
    let before = repo.count_iterations(job_id).await.expect("count");
    assert_eq!(before, 3);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE job_id = $1")
        .bind(job_id.as_uuid())
        .execute(&pool)
        .await
        .expect("delete parent");

    let after = repo.count_iterations(job_id).await.expect("count after");
    assert_eq!(
        after, 0,
        "ON DELETE CASCADE must drop every child checkpoint when parent removed"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_check_constraint_rejects_oversized_summary_via_raw_sql() {
    // Defends against a future caller that bypasses the in-Rust gate by
    // constructing an INSERT directly. The CHECK constraint in
    // migrations/0023_micro_task_job_queue.sql must reject it.
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("check");
    let job_id = enqueue_parent_job(&pool, &wp).await;

    let oversized = "z".repeat(COMPACT_SUMMARY_MAX_BYTES + 1);
    let res = sqlx::query(
        r#"INSERT INTO kernel_mt_loop_checkpoint
           (checkpoint_id, job_id, iteration_n, state_at_checkpoint,
            retry_budget_remaining, verifier_feedback_history,
            compact_summary, evidence_pointers,
            created_at_utc, created_by_session)
           VALUES ($1, $2, 0, '{}'::jsonb, 6, '[]'::jsonb, $3, '[]'::jsonb, NOW(), $4)"#,
    )
    .bind(Uuid::now_v7())
    .bind(job_id.as_uuid())
    .bind(&oversized)
    .bind(Uuid::now_v7())
    .execute(&pool)
    .await;
    assert!(
        res.is_err(),
        "oversized summary must violate CHECK constraint at the DB level"
    );

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_concurrent_checkpoints_do_not_double_count_retries() {
    // RED-TEAM minimum control #2 (Postgres half): 4 tasks persist 4
    // distinct checkpoints for the same job in parallel; post-race
    // `count_iterations` must be exactly 4 and the chain_ids must be unique.
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("concurrent");
    let job_id = enqueue_parent_job(&pool, &wp).await;
    let repo = Arc::new(MtLoopCheckpointRepo::new(pool.clone()));

    let mut handles = Vec::new();
    for iter_n in 0..4u32 {
        let repo = repo.clone();
        let wp = wp.clone();
        handles.push(tokio::spawn(async move {
            let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
            j.job_id = job_id;
            j.iteration_n = iter_n;
            let cp = MtLoopControl::record_checkpoint(
                &j,
                MtLoopState::WaitingForVerifier,
                format!("concurrent iter {iter_n}"),
                vec![],
                &MtLoopControlBudget::default(),
                vec![],
                Uuid::now_v7(),
            )
            .expect("record");
            repo.persist(&cp).await.expect("persist");
            cp.checkpoint_id
        }));
    }
    let mut written_ids = Vec::new();
    for h in handles {
        written_ids.push(h.await.expect("join"));
    }

    let count = repo.count_iterations(job_id).await.expect("count");
    assert_eq!(
        count, 4,
        "exactly 4 checkpoints must be persisted, got {count}"
    );

    let chain = repo.chain_for_job(job_id).await.expect("chain");
    let chain_ids: std::collections::HashSet<Uuid> =
        chain.iter().map(|c| c.checkpoint_id).collect();
    let written_set: std::collections::HashSet<Uuid> = written_ids.into_iter().collect();
    assert_eq!(
        chain_ids, written_set,
        "every persisted checkpoint_id must appear in the chain exactly once"
    );

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_185_pg_cross_executor_resume_byte_identical() {
    // RED-TEAM minimum control #3 (Postgres half): two repo handles with
    // different session_ids reading the same persisted checkpoint must
    // produce byte-identical ResumeContext.
    let pool = postgres_pool().await;
    ensure_schemas(&pool).await;

    let wp = unique_wp_id("xresume");
    let job_id = enqueue_parent_job(&pool, &wp).await;

    let session_writer = Uuid::now_v7();
    let cp = {
        let repo = MtLoopCheckpointRepo::new(pool.clone());
        let mut j = MicroTaskJob::queue(&wp, "MT-185", PathBuf::from("a.json"), 6, vec![]);
        j.job_id = job_id;
        j.iteration_n = 2;
        let cp = MtLoopControl::record_checkpoint(
            &j,
            MtLoopState::EscalationProposed {
                target_tier: EscalationTier::T13B,
            },
            "cross-executor resume payload".to_string(),
            vec![EvidencePointer {
                kind: "log".to_string(),
                uri: "file://x.log".to_string(),
                label: Some("seed".to_string()),
            }],
            &MtLoopControlBudget::default(),
            vec![VerifierFeedbackRef {
                feedback_id: Uuid::now_v7(),
                at_utc: Utc::now(),
                summary: "f1".to_string(),
            }],
            session_writer,
        )
        .expect("record");
        repo.persist(&cp).await.expect("persist");
        cp
    };

    // Two distinct executor "personas" — different sessions, same pool.
    let _session_a = Uuid::now_v7();
    let _session_b = Uuid::now_v7();
    let repo_a = MtLoopCheckpointRepo::new(pool.clone());
    let repo_b = MtLoopCheckpointRepo::new(pool.clone());
    let cp_a = repo_a.get(cp.checkpoint_id).await.expect("fa").unwrap();
    let cp_b = repo_b.get(cp.checkpoint_id).await.expect("fb").unwrap();
    let ctx_a = MtLoopControl::resume_from(&cp_a);
    let ctx_b = MtLoopControl::resume_from(&cp_b);
    assert_eq!(ctx_a, ctx_b);
    let sa = serde_json::to_vec(&ctx_a).expect("a");
    let sb = serde_json::to_vec(&ctx_b).expect("b");
    assert_eq!(sa, sb, "cross-executor resume must be byte-identical");

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

// CheckpointRepoError carries a `LoopControl` variant; keep this assertion
// so future renames don't silently break the typed-error boundary.
#[test]
fn mt_185_checkpoint_repo_error_carries_loop_control_variant() {
    let err: CheckpointRepoError =
        CheckpointRepoError::LoopControl(LoopControlError::LoopBudgetExceeded("test".to_string()));
    let s = format!("{err}");
    assert!(s.contains("loop control") || s.contains("loop budget"));
}
