use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::runtime_postgres::{
    count_rows, recovery_broker, write_artifact_json, RuntimePg, RuntimeSeed, RuntimeTestError,
};

#[derive(Debug, Serialize)]
pub struct HardKillEvidence {
    pub schema: String,
    pub artifact_report: PathBuf,
    pub child_was_hard_killed: bool,
    pub child_process_row_was_left_without_graceful_stop: bool,
    pub checkpoint_rows: i64,
    pub process_rows: i64,
    pub report_rows: i64,
    pub resumed_sessions: usize,
    pub failed_sessions: usize,
}

#[derive(Debug, Serialize)]
pub struct RealStartupRecoveryEvidence {
    pub schema: String,
    pub artifact_report: PathBuf,
    pub real_binary_was_spawned: bool,
    pub startup_recovery_only_exit: bool,
    pub process_exit_success: Option<bool>,
    pub report_rows: i64,
    pub resumed_sessions: i64,
    pub failed_sessions: i64,
    pub retry_scheduled_queue_rows: i64,
    pub post_failure_checkpoint_rows: i64,
    pub reclaimed_process_rows: i64,
    pub final_counter: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ChildReadyFile {
    session_id: Uuid,
    process_id: Uuid,
    os_pid: u32,
}

#[derive(Debug, Serialize)]
struct HardKillActionFile {
    child_pid: u32,
    child_exit: String,
    child_was_hard_killed: bool,
}

struct ProductionSeed {
    session_id: Uuid,
    session_run_id: String,
}

pub async fn run_hard_kill_child_recovery() -> HardKillEvidence {
    let pg = RuntimePg::new("hard-kill-child").await;
    let ready_file = pg.artifact_dir().join("ready.json");
    let action_file = pg.artifact_dir().join("action.json");
    let stdout_file = pg.artifact_dir().join("stdout.log");
    let stderr_file = pg.artifact_dir().join("stderr.log");

    let child_pid = spawn_runtime_child(&pg, &ready_file, &stdout_file, &stderr_file)
        .unwrap_or_else(|err| panic!("spawn MT-195 runtime child: {err}"));
    wait_for_ready(&ready_file, Duration::from_secs(15));

    let mut child = child_pid;
    let pid = child.id();
    child
        .kill()
        .unwrap_or_else(|err| panic!("hard-kill MT-195 runtime child: {err}"));
    let status = child
        .wait()
        .unwrap_or_else(|err| panic!("wait for hard-killed MT-195 runtime child: {err}"));
    let child_was_hard_killed = !status.success();
    let action = HardKillActionFile {
        child_pid: pid,
        child_exit: format!("{status:?}"),
        child_was_hard_killed,
    };
    write_artifact_json(&action_file, &action);

    mark_child_killed(&pg.pool).await;
    let broker = recovery_broker(pg.pool.clone());
    let report = handshake_core::session_checkpoint::RestartResumeOrchestrator::run(&broker);

    let checkpoint_rows = count_rows(&pg.pool, "kernel_session_checkpoint").await;
    let process_rows = count_rows(&pg.pool, "kernel_runtime_process_evidence").await;
    let report_rows = count_rows(&pg.pool, "kernel_restart_resume_report").await;
    let process_rows_without_stop = process_rows_without_graceful_stop(&pg.pool).await;

    let artifact_report = pg.artifact_dir().join("report.json");
    let evidence = HardKillEvidence {
        schema: pg.schema().to_string(),
        artifact_report: artifact_report.clone(),
        child_was_hard_killed,
        child_process_row_was_left_without_graceful_stop: process_rows_without_stop == 1,
        checkpoint_rows,
        process_rows,
        report_rows,
        resumed_sessions: report.sessions_resumed.len(),
        failed_sessions: report.sessions_recovery_failed.len(),
    };
    write_artifact_json(&artifact_report, &evidence);
    evidence
}

pub async fn run_real_handshake_core_startup_recovery() -> RealStartupRecoveryEvidence {
    let pg = RuntimePg::new_production("real-handshake-core-startup-recovery").await;
    let seed = seed_production_resume_candidate(&pg.pool).await;
    let startup_report_file = pg.artifact_dir().join("startup-recovery-report.json");
    let stdout_file = pg.artifact_dir().join("real-binary-stdout.log");
    let stderr_file = pg.artifact_dir().join("real-binary-stderr.log");

    let status = spawn_real_handshake_core_startup_recovery(
        &pg,
        &startup_report_file,
        &stdout_file,
        &stderr_file,
    )
    .unwrap_or_else(|err| panic!("spawn real handshake_core startup recovery: {err}"));

    let report_rows = count_production_report_rows(&pg.pool).await;
    let resumed_sessions = count_report_sessions(&pg.pool, "sessions_resumed").await;
    let failed_sessions = count_report_sessions(&pg.pool, "sessions_recovery_failed").await;
    let retry_scheduled_queue_rows =
        count_retry_scheduled_queue_rows(&pg.pool, &seed.session_run_id).await;
    let post_failure_checkpoint_rows =
        count_post_failure_checkpoint_rows(&pg.pool, seed.session_id).await;
    let reclaimed_process_rows = count_reclaimed_process_rows(&pg.pool, &seed.session_run_id).await;
    let final_counter = latest_checkpoint_counter(&pg.pool, seed.session_id).await;

    let artifact_report = pg.artifact_dir().join("real-binary-evidence.json");
    let evidence = RealStartupRecoveryEvidence {
        schema: pg.schema().to_string(),
        artifact_report: artifact_report.clone(),
        real_binary_was_spawned: true,
        startup_recovery_only_exit: startup_report_file.exists(),
        process_exit_success: Some(status.success()),
        report_rows,
        resumed_sessions,
        failed_sessions,
        retry_scheduled_queue_rows,
        post_failure_checkpoint_rows,
        reclaimed_process_rows,
        final_counter,
    };
    write_artifact_json(&artifact_report, &evidence);
    evidence
}

fn spawn_runtime_child(
    pg: &RuntimePg,
    ready_file: &Path,
    stdout_file: &Path,
    stderr_file: &Path,
) -> Result<std::process::Child, RuntimeTestError> {
    let exe = std::env::current_exe().map_err(RuntimeTestError::Io)?;
    let stdout = File::create(stdout_file).map_err(RuntimeTestError::Io)?;
    let stderr = File::create(stderr_file).map_err(RuntimeTestError::Io)?;
    Command::new(exe)
        .arg("--ignored")
        .arg("--exact")
        .arg("runtime_child::mt195_runtime_child_entrypoint")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("HS_MT195_RUNTIME_CHILD", "1")
        .env("HS_MT195_SCHEMA_URL", pg.schema_url())
        .env("HS_MT195_READY_FILE", ready_file)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(RuntimeTestError::Io)
}

fn spawn_real_handshake_core_startup_recovery(
    pg: &RuntimePg,
    startup_report_file: &Path,
    stdout_file: &Path,
    stderr_file: &Path,
) -> Result<std::process::ExitStatus, RuntimeTestError> {
    let exe = real_handshake_core_exe();
    let stdout = File::create(stdout_file).map_err(RuntimeTestError::Io)?;
    let stderr = File::create(stderr_file).map_err(RuntimeTestError::Io)?;
    let mut command = Command::new(exe);
    command
        .env("DATABASE_URL", pg.schema_url())
        .env("HANDSHAKE_STORAGE_MODE", "postgres_primary")
        .env("HANDSHAKE_CONTROL_PLANE_REQUIRES_POSTGRES", "1")
        .env("HANDSHAKE_STARTUP_RECOVERY_ONLY", "1")
        .env(
            "HANDSHAKE_STARTUP_RECOVERY_REPORT_FILE",
            startup_report_file,
        )
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    apply_quiet_process_flags(&mut command);
    let mut child = command.spawn().map_err(RuntimeTestError::Io)?;
    wait_for_child_exit(&mut child, Duration::from_secs(120))
}

fn real_handshake_core_exe() -> PathBuf {
    option_env!("CARGO_BIN_EXE_handshake_core")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_handshake_core").map(PathBuf::from))
        .unwrap_or_else(|| {
            panic!(
                "ENVIRONMENT_BLOCKED: CARGO_BIN_EXE_handshake_core missing; rerun with --features app-runtime"
            )
        })
}

#[cfg(windows)]
fn apply_quiet_process_flags(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn apply_quiet_process_flags(_command: &mut Command) {}

fn wait_for_child_exit(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, RuntimeTestError> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Some(status) = child.try_wait().map_err(RuntimeTestError::Io)? {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!(
        "ENVIRONMENT_BLOCKED: real handshake_core startup recovery child did not exit within {:?}",
        timeout
    );
}

fn wait_for_ready(ready_file: &Path, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if ready_file.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "ENVIRONMENT_BLOCKED: MT-195 runtime child did not write ready file within {:?}",
        timeout
    );
}

async fn mark_child_killed(pool: &PgPool) {
    sqlx::query(
        r#"
        UPDATE kernel_runtime_process_evidence
        SET killed_observed_at_utc = NOW(), state = 'hard_killed_observed'
        WHERE graceful_stop_at_utc IS NULL
        "#,
    )
    .execute(pool)
    .await
    .expect("mark hard-killed process evidence");
}

async fn process_rows_without_graceful_stop(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kernel_runtime_process_evidence
        WHERE graceful_stop_at_utc IS NULL
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("count process rows without graceful stop")
}

async fn seed_production_resume_candidate(pool: &PgPool) -> ProductionSeed {
    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let kernel_task_run_id = format!("KTR-{session_id}");
    sqlx::query(
        r#"
        INSERT INTO kernel_session_queue (
            session_run_id,
            kernel_task_run_id,
            adapter_id,
            state,
            claimed_by,
            lease_expires_at,
            attempt_count,
            available_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, 'mt195-real-binary', 'RUNNING', 'mt195-previous-worker', NOW() + INTERVAL '30 minutes', 1, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_run_id)
    .bind(&kernel_task_run_id)
    .execute(pool)
    .await
    .expect("seed production session queue");

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
        VALUES ($1, $2, $3, 0, $4, 'periodic', '[]'::jsonb, NOW(), $5, 1)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(Uuid::now_v7())
    .bind(serde_json::json!({ "counter": 0, "label": "real_handshake_core_startup" }))
    .bind(std::process::id() as i32)
    .execute(pool)
    .await
    .expect("seed production checkpoint");

    seed_production_event(pool, &kernel_task_run_id, &session_run_id, 1).await;
    seed_production_event(pool, &kernel_task_run_id, &session_run_id, 2).await;
    seed_production_orphan_process(pool, &session_run_id).await;

    ProductionSeed {
        session_id,
        session_run_id,
    }
}

async fn seed_production_event(
    pool: &PgPool,
    kernel_task_run_id: &str,
    session_run_id: &str,
    by: i64,
) {
    let event_id = format!("KE-{}", Uuid::now_v7());
    sqlx::query(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            payload_hash,
            source_component,
            payload
        )
        VALUES ($1, 'kernel_event_v1', $2, $3, 'session_run', $3, $4,
                'MODEL_RESPONSE_RECORDED', 'session_broker', 'mt195-real-binary',
                '0000000000000000000000000000000000000000000000000000000000000000',
                'mt195-real-binary', $5)
        "#,
    )
    .bind(&event_id)
    .bind(kernel_task_run_id)
    .bind(session_run_id)
    .bind(format!("mt195-real-binary-{event_id}"))
    .bind(serde_json::json!({ "by": by }))
    .execute(pool)
    .await
    .expect("seed production event");
}

async fn seed_production_orphan_process(pool: &PgPool, session_run_id: &str) {
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid,
            os_pid,
            parent_session_id,
            sandbox_adapter_id,
            engine_kind,
            started_at,
            owner_role,
            owner_wp,
            metadata_jsonb
        )
        VALUES ($1, $2, $3, 'mt195-real-binary', 'helper_subprocess', NOW(), 'coder', 'WP-KERNEL-004', '{}'::jsonb)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(std::process::id() as i64)
    .bind(session_run_id)
    .execute(pool)
    .await
    .expect("seed production orphan process");
}

async fn count_production_report_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM kernel_restart_resume_report")
        .fetch_one(pool)
        .await
        .expect("count production report rows")
}

async fn count_report_sessions(pool: &PgPool, column: &str) -> i64 {
    assert!(
        matches!(column, "sessions_resumed" | "sessions_recovery_failed"),
        "unexpected report session column {column}"
    );
    sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COALESCE(SUM(jsonb_array_length({column})), 0)::BIGINT FROM kernel_restart_resume_report"
    ))
    .fetch_one(pool)
    .await
    .expect("count production report sessions")
}

async fn count_retry_scheduled_queue_rows(pool: &PgPool, session_run_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kernel_session_queue
        WHERE session_run_id = $1
          AND state = 'RETRY_SCHEDULED'
          AND claimed_by IS NULL
          AND lease_expires_at IS NULL
        "#,
    )
    .bind(session_run_id)
    .fetch_one(pool)
    .await
    .expect("count retry-scheduled queue rows")
}

async fn count_post_failure_checkpoint_rows(pool: &PgPool, session_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kernel_session_checkpoint
        WHERE session_id = $1
          AND state_kind = 'post_failure'
        "#,
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .expect("count post-failure checkpoints")
}

async fn count_reclaimed_process_rows(pool: &PgPool, session_run_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kernel_process_lifecycle
        WHERE parent_session_id = $1
          AND stopped_at IS NOT NULL
          AND exit_code = -1
          AND stop_reason = 'reclaim'
        "#,
    )
    .bind(session_run_id)
    .fetch_one(pool)
    .await
    .expect("count reclaimed production process rows")
}

async fn latest_checkpoint_counter(pool: &PgPool, session_id: Uuid) -> Option<i64> {
    sqlx::query(
        r#"
        SELECT compact_state
        FROM kernel_session_checkpoint
        WHERE session_id = $1
        ORDER BY created_at_utc DESC
        LIMIT 1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .expect("load latest production checkpoint")
    .and_then(|row| {
        row.get::<serde_json::Value, _>("compact_state")
            .get("counter")
            .and_then(serde_json::Value::as_i64)
    })
}

pub async fn run_child_entrypoint_from_env() {
    if std::env::var("HS_MT195_RUNTIME_CHILD").ok().as_deref() != Some("1") {
        return;
    }

    let schema_url = std::env::var("HS_MT195_SCHEMA_URL")
        .expect("ENVIRONMENT_BLOCKED: HS_MT195_SCHEMA_URL missing for MT-195 runtime child");
    let ready_file = PathBuf::from(
        std::env::var("HS_MT195_READY_FILE")
            .expect("ENVIRONMENT_BLOCKED: HS_MT195_READY_FILE missing for MT-195 runtime child"),
    );
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&schema_url)
        .await
        .expect("connect MT-195 child to isolated Postgres schema");
    let process_id = Uuid::now_v7();
    let seed = RuntimeSeed::success("hard_kill_child");
    crate::runtime_postgres::insert_seed(&pool, seed)
        .await
        .expect("insert child checkpoint/event evidence");
    insert_process_evidence(&pool, process_id, seed.session_id).await;
    let ready = ChildReadyFile {
        session_id: seed.session_id,
        process_id,
        os_pid: std::process::id(),
    };
    write_artifact_json(&ready_file, &ready);

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn insert_process_evidence(pool: &PgPool, process_id: Uuid, session_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO kernel_runtime_process_evidence (
            process_id,
            session_id,
            os_pid,
            role,
            state,
            checkpoint_committed_at_utc
        )
        VALUES ($1, $2, $3, 'mt195-runtime-child', 'checkpoint_committed', NOW())
        "#,
    )
    .bind(process_id)
    .bind(session_id)
    .bind(std::process::id() as i32)
    .execute(pool)
    .await
    .expect("insert child process evidence");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "internal MT-195 runtime child entrypoint; parent test spawns and hard-kills this process"]
async fn mt195_runtime_child_entrypoint() {
    run_child_entrypoint_from_env().await;
}
