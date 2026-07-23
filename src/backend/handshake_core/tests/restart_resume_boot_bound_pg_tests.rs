//! WP-1 MT-003 remediation (step c): the startup restart-resume boot pass must
//! be hard-bounded by an outer wall clock, fail closed on timeout, record durable
//! evidence, and never hang or panic.
//!
//! Proof path: REAL Handshake-managed PostgreSQL (auto-started by the harness via
//! `knowledge_pg_support`; SKIP loudly only when the PostgreSQL binaries are
//! genuinely absent). Each test runs in its own isolated schema with the
//! restart-resume migration subset applied, mirroring
//! `restart_resume_postgres_tests.rs`.
//!
//! Determinism: a resumable candidate's orphan-reclaim UPDATE is blocked behind a
//! held `SELECT ... FOR UPDATE` row lock on a second connection, so the boot pass
//! cannot complete within the bound. The only way `run_with_bound` returns without
//! hanging is the hard timeout firing.

#[allow(dead_code)]
mod knowledge_pg_support;

use std::time::{Duration, Instant};

use handshake_core::process_ledger::restart_resume::{
    BoundedRestartResumeOutcome, PostgresRestartResumeRunner,
};
use serde_json::Value;
use sqlx::Connection;
use uuid::Uuid;

/// Restart-resume migration subset (identical set to
/// `restart_resume_postgres_tests::apply_schema`). The bounded boot pass only
/// touches the session queue, checkpoint, event ledger, process-lifecycle,
/// idempotency, and restart-resume-report tables these migrations create.
async fn apply_restart_resume_schema(pool: &sqlx::PgPool) {
    for stmt in [
        include_str!("../migrations/0018_kernel_event_ledger.sql"),
        include_str!("../migrations/0019_kernel_session_queue.sql"),
        include_str!("../migrations/0021_kernel_process_lifecycle.sql"),
        include_str!("../migrations/0022_role_mailbox_threads_messages.sql"),
        include_str!("../migrations/0024_session_checkpoint.sql"),
        include_str!("../migrations/0028_restart_resume_report_wiring.sql"),
    ] {
        sqlx::raw_sql(stmt)
            .execute(pool)
            .await
            .expect("apply restart-resume migration subset");
    }
}

/// Create a fresh isolated schema on the real managed cluster and return a pool
/// pinned to it plus the schema-pinned URL (for the blocker connection).
/// Returns `None` only when PostgreSQL binaries are absent (caller SKIPs).
async fn isolated_restart_resume_pool() -> Option<(sqlx::PgPool, String)> {
    let url = knowledge_pg_support::base_database_url().await?;
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect managed PostgreSQL for schema setup");
    let schema = format!("wp1_mt003_{}", Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut conn)
        .await
        .expect("create isolated schema");
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let pool = sqlx::PgPool::connect(&schema_url)
        .await
        .expect("connect isolated schema");
    apply_restart_resume_schema(&pool).await;
    Some((pool, schema_url))
}

async fn seed_resumable_session(pool: &sqlx::PgPool, session_id: Uuid) -> String {
    let session_run_id = format!("SR-{session_id}");
    sqlx::query(
        r#"
        INSERT INTO kernel_session_queue (
            session_run_id, kernel_task_run_id, adapter_id, state, claimed_by,
            lease_expires_at, attempt_count, available_at, created_at, updated_at
        )
        VALUES ($1, $2, 'wp1-mt003-adapter', 'RUNNING', 'previous-worker',
                NOW() + INTERVAL '30 minutes', 1, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_run_id)
    .bind(format!("KTR-{session_id}"))
    .execute(pool)
    .await
    .expect("seed resumable session");
    session_run_id
}

/// One open process-lifecycle row for this session so the boot pass's
/// `reclaim_orphans` UPDATE has a real row to touch (and to lock).
async fn seed_open_process(pool: &sqlx::PgPool, session_run_id: &str) -> Uuid {
    let process_uuid = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, os_pid, parent_session_id, sandbox_adapter_id,
            engine_kind, started_at, owner_role, owner_wp, metadata_jsonb
        )
        VALUES ($1, 4242, $2, 'wp1-mt003-adapter', 'helper_subprocess', NOW(),
                'coder', 'WP-1', '{}'::jsonb)
        "#,
    )
    .bind(process_uuid)
    .bind(session_run_id)
    .execute(pool)
    .await
    .expect("seed open process-lifecycle row");
    process_uuid
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn restart_resume_boot_pass_is_hard_bounded_when_a_candidate_reclaim_blocks() {
    let Some((pool, schema_url)) = isolated_restart_resume_pool().await else {
        eprintln!(
            "SKIPPED restart_resume_boot_pass_is_hard_bounded_when_a_candidate_reclaim_blocks: PostgreSQL unavailable"
        );
        return;
    };

    let session_id = Uuid::now_v7();
    let session_run_id = seed_resumable_session(&pool, session_id).await;
    let process_uuid = seed_open_process(&pool, &session_run_id).await;

    // Hold an exclusive row lock on the candidate's process-lifecycle row on a
    // second connection so the boot pass's reclaim UPDATE
    // (`... SET stopped_at = NOW() WHERE parent_session_id = $1 AND stopped_at IS NULL`)
    // blocks deterministically for the whole bound.
    let mut blocker = sqlx::PgConnection::connect(&schema_url)
        .await
        .expect("connect blocker into isolated schema");
    sqlx::query("BEGIN")
        .execute(&mut blocker)
        .await
        .expect("begin blocker transaction");
    let locked: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT process_uuid FROM kernel_process_lifecycle \
         WHERE parent_session_id = $1 AND stopped_at IS NULL FOR UPDATE",
    )
    .bind(&session_run_id)
    .fetch_all(&mut blocker)
    .await
    .expect("acquire FOR UPDATE lock on the candidate row");
    assert_eq!(
        locked,
        vec![(process_uuid,)],
        "blocker must hold the exact candidate row lock"
    );

    let bound = Duration::from_millis(750);
    let runner = PostgresRestartResumeRunner::new(pool.clone());
    let started = Instant::now();
    let outcome = runner
        .run_with_bound(bound)
        .await
        .expect("bounded boot pass returns Ok, never hangs or panics");
    let elapsed = started.elapsed();

    // Release the lock (independent of the assertions below).
    sqlx::query("ROLLBACK")
        .execute(&mut blocker)
        .await
        .expect("release blocker lock");

    // The bound MUST fire: with the reclaim UPDATE blocked, natural completion is
    // impossible, so a TimedOut outcome is proof the hard bound triggered.
    let (timeout, report, evidence_persisted) = match outcome {
        BoundedRestartResumeOutcome::TimedOut {
            timeout,
            report,
            evidence_persisted,
        } => (timeout, report, evidence_persisted),
        BoundedRestartResumeOutcome::Completed(_) => {
            panic!("a blocked candidate reclaim must not let the boot pass complete")
        }
    };
    assert_eq!(timeout, bound, "outcome must carry the configured bound");
    assert!(
        elapsed < Duration::from_secs(20),
        "the bound must fire promptly with no hang; elapsed {elapsed:?}"
    );

    // Fail-closed: the still-open resumable session was NOT falsely resumed.
    let queue_state: String =
        sqlx::query_scalar("SELECT state FROM kernel_session_queue WHERE session_run_id = $1")
            .bind(&session_run_id)
            .fetch_one(&pool)
            .await
            .expect("read session queue state after bounded abort");
    assert_eq!(
        queue_state, "RUNNING",
        "a bounded-aborted pass must leave the session resumable for the staleness reclaim task and the next boot pass"
    );

    // Durable evidence: a bounded-abort report row exists (a different table than
    // the locked lifecycle row, so this write is unaffected by the held lock).
    assert!(
        evidence_persisted,
        "the bounded-abort report must be durably persisted"
    );
    let fr_events: Value = sqlx::query_scalar(
        "SELECT fr_events_emitted FROM kernel_restart_resume_report WHERE report_id = $1",
    )
    .bind(report.report_id)
    .fetch_one(&pool)
    .await
    .expect("durable bounded-abort report row must exist");
    let events = fr_events
        .as_array()
        .expect("fr_events_emitted is a JSON array");
    assert!(
        events
            .iter()
            .any(|event| event == "FR-EVT-RESTART-RESUME-STARTED"),
        "bounded-abort evidence must record the pass started: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| event == "FR-EVT-RESTART-RESUME-COMPLETED"),
        "bounded-abort evidence must NOT claim the pass completed (Started-without-Completed is the durable incomplete-pass marker): {events:?}"
    );

    pool.close().await;
}
