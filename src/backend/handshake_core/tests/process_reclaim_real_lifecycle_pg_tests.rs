//! WP-1 MT-007 remediation (step c): real-process + real-PostgreSQL proof of the
//! production process-reclaim lifecycle.
//!
//! Unlike `pidless_cloud_lifecycle_tests.rs` (which proves lifecycle logic against
//! in-memory `CapturingStore`/`DelayedFirstWriteStore` mocks and a mock runtime),
//! every proof here runs against:
//!   * REAL Handshake-managed PostgreSQL (auto-started by `knowledge_pg_support`;
//!     full migration chain incl. the migration-0359 runtime-owner guard, on an
//!     isolated schema per test). SKIP loudly only when the PostgreSQL binaries
//!     are genuinely absent.
//!   * A REAL spawned long-lived Windows child process the test owns, whose
//!     ledger START row carries the real OS pid + `os_creation_time_100ns`
//!     (captured through the production `sandbox::process_creation_time_100ns`)
//!     + real executable SHA-256, and
//!   * The production reclaim path: `Reclaim` + `ProductionSandboxKill` ->
//!     `HandshakeNativeSandboxAdapter::reclaim_detached` ->
//!     `reclaim_verified_detached_process` (the exact-generation identity fence in
//!     `sandbox/handshake_native.rs`) -> `TerminateProcess`, with the STOP row
//!     persisted through the production `LedgerBatcher` + `PostgresProcessLedgerStore`.
//!
//! Proofs:
//!   (a) START durable before the process is treated as live; production reclaim
//!       kills the real child and writes a durable STOP.
//!   (b) crash/restart reconciliation: a prior-boot orphan START (durable, its
//!       in-memory writer dropped) is reconciled by a fresh boot's reclaim path,
//!       killing the orphan and writing a durable STOP.
//!   (c) concurrent-reclaimer safety: two reclaimers racing the same process
//!       produce exactly one kill + one durable STOP, no double-STOP, no error
//!       escalation.
//!   (d) PID-reuse guard: a START row whose recorded creation-generation identity
//!       does not match the live PID must NOT kill that process and must fail
//!       closed (no STOP, process still alive).
//!
//! The reclaim identity fence and `reclaim_verified_detached_process` are
//! Windows-only, so this real-kill proof is gated to Windows.
#![cfg(windows)]

#[allow(dead_code)]
mod knowledge_pg_support;

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use handshake_core::process_ledger::{
    acquire_embedded_runtime_instance_lease, production_process_sandbox_registry,
    reconcile_restart_orphans_at_boot, set_dead_owner_confirmation_gap_override_for_test,
    spawn_managed_staleness_reclaim_task_after_boot, KillOutcome, LedgerBatcher,
    LedgerBatcherConfig, NoopOverflowSink, PostgresModelLaneStaleSessionSource,
    PostgresProcessLedgerStore, ProcessLedgerStore, ProcessReclaimRuntime, ProductionSandboxKill,
    Reclaim, ReclaimKillOperationStatus, ReclaimProcessStore, ReclaimTrigger, StaleSessionSource,
    StalenessReclaimConfig, EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
    EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
};
use handshake_core::sandbox::{
    process_creation_time_100ns, AdapterId, SandboxAdapterRegistry,
    HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::{Ipv4Addr, UdpSocket};
use uuid::Uuid;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const IDENTITY_POOL_HOLD: Duration = Duration::from_millis(5_500);
const IDENTITY_PATH_COMPLETION_BOUND: Duration = Duration::from_secs(20);
const AUTHORITATIVE_RECLAIM_KILL_BOUND: Duration = Duration::from_secs(30);

/// Kill-on-drop guard so a spawned child never leaks, even if an assertion
/// panics mid-test. Only ever kills the process THIS test spawned.
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn pid(&self) -> u32 {
        self.0.as_ref().expect("child present").id()
    }

    /// Poll for the child to have exited within `deadline`.
    fn wait_exited(&mut self, deadline: Duration) -> bool {
        let child = self.0.as_mut().expect("child present");
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) if start.elapsed() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Ok(None) => return false,
                Err(_) => return false,
            }
        }
    }

    fn is_still_running(&mut self) -> bool {
        match self.0.as_mut().expect("child present").try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Resolve a real, long-lived, single-process Windows executable path. PowerShell
/// `Start-Sleep` is one process with no grandchildren, so terminating it is clean.
fn powershell_path() -> Option<PathBuf> {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:/Windows".to_string());
    let candidate =
        PathBuf::from(system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    candidate.is_file().then_some(candidate)
}

fn sha256_file(path: &PathBuf) -> String {
    let bytes = std::fs::read(path).expect("read executable for hash");
    let digest = Sha256::digest(&bytes);
    hex::encode(digest)
}

/// A real spawned child plus the durable identity the ledger would carry.
struct SpawnedChild {
    guard: ChildGuard,
    pid: u32,
    executable_sha256: String,
    os_creation_time_100ns: u64,
}

fn spawn_real_long_lived_child() -> SpawnedChild {
    let exe = powershell_path().expect(
        "REQUIRED_REAL_PROCESS_PROOF: Windows PowerShell executable is unavailable; this test must fail, not green-skip",
    );
    let child = Command::new(&exe)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 600",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .expect("spawn real long-lived child process");
    let pid = child.id();
    // Capture the exact process-generation identity through the SAME production
    // helper the sandbox spawn path uses.
    let os_creation_time_100ns =
        process_creation_time_100ns(pid).expect("read child OS creation-generation identity");
    SpawnedChild {
        guard: ChildGuard(Some(child)),
        pid,
        executable_sha256: sha256_file(&exe),
        os_creation_time_100ns,
    }
}

/// Full migration chain on an isolated schema of the real managed cluster.
async fn managed_full_chain_pool() -> (knowledge_pg_support::KnowledgePg, PgPool) {
    let kp = knowledge_pg_support::knowledge_pg().await.expect(
        "REQUIRED_REAL_POSTGRES_PROOF: Handshake-managed PostgreSQL is unavailable; this test must fail, not green-skip",
    );
    let pool = sqlx::PgPool::connect(&kp.schema_url)
        .await
        .expect("connect a reclaim pool pinned to the isolated migrated schema");
    (kp, pool)
}

/// Directly seed a durable START row carrying the real OS identity, exactly as a
/// production spawn would record it (owner_runtime_instance_id left NULL so the
/// migration-0359 runtime-owner guard is a no-op). `creation_time_override` lets
/// the PID-reuse proof record a mismatched generation identity.
async fn seed_real_process_start(
    pool: &PgPool,
    session_run_id: &str,
    process_uuid: Uuid,
    pid: u32,
    creation_time: u64,
    executable_sha256: &str,
) {
    let metadata = json!({
        "executable_sha256": executable_sha256,
        "os_creation_time_100ns": creation_time,
        "sandbox_handle_id": process_uuid.to_string(),
    });
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, os_pid, parent_session_id, sandbox_adapter_id,
            sandbox_internal_id, engine_kind, started_at, owner_role, owner_wp,
            metadata_jsonb
        )
        VALUES ($1, $2, $3, $4, $5, 'external_compat', NOW(), 'coder', 'WP-1', $6)
        "#,
    )
    .bind(process_uuid)
    .bind(i64::from(pid))
    .bind(session_run_id)
    .bind(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
    .bind(process_uuid.to_string())
    .bind(metadata)
    .execute(pool)
    .await
    .expect("seed durable START row with real OS identity");
}

fn batcher_config() -> LedgerBatcherConfig {
    LedgerBatcherConfig {
        capacity: 64,
        batch_size: 1,
        flush_interval: Duration::from_millis(5),
    }
}

/// Build a fresh production reclaim stack over the real pool: production
/// `PostgresProcessLedgerStore`, production `ProductionSandboxKill` (which
/// registers `HandshakeNativeSandboxAdapter`), and a `LedgerBatcher` STOP writer.
fn build_reclaim(
    pool: &PgPool,
) -> (
    Reclaim,
    LedgerBatcher,
    tokio::task::JoinHandle<Result<(), handshake_core::process_ledger::ProcessLedgerError>>,
) {
    build_reclaim_with_kill_pool(pool, pool)
}

/// Build the production stack with a separately constrained identity-query pool.
/// The store and STOP writer keep using `store_pool`, so exhausting `kill_pool`
/// isolates the exact identity-read boundary rather than blocking claim/STOP SQL.
fn build_reclaim_with_kill_pool(
    store_pool: &PgPool,
    kill_pool: &PgPool,
) -> (
    Reclaim,
    LedgerBatcher,
    tokio::task::JoinHandle<Result<(), handshake_core::process_ledger::ProcessLedgerError>>,
) {
    let store: Arc<dyn ProcessLedgerStore> =
        Arc::new(PostgresProcessLedgerStore::new(store_pool.clone()));
    let (ledger, join) = LedgerBatcher::spawn(store, Arc::new(NoopOverflowSink), batcher_config());
    let reclaim_store = Arc::new(PostgresProcessLedgerStore::new(store_pool.clone()));
    let killer = Arc::new(ProductionSandboxKill::new(
        kill_pool.clone(),
        tokio::runtime::Handle::current(),
    ));
    let reclaim = Reclaim::new(reclaim_store, killer, Arc::new(ledger.clone()));
    (reclaim, ledger, join)
}

/// Mirror the production composition root's store-authority preflight before
/// the writer starts, keeping the first reclaim STOP's five-second budget for
/// the mutation and durable acknowledgement rather than catalog discovery.
async fn build_preflighted_reclaim(
    pool: &PgPool,
) -> (
    Reclaim,
    LedgerBatcher,
    tokio::task::JoinHandle<Result<(), handshake_core::process_ledger::ProcessLedgerError>>,
) {
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store
        .preflight()
        .await
        .expect("preflight process-ledger writer authority");
    let store: Arc<dyn ProcessLedgerStore> = store;
    let (ledger, join) = LedgerBatcher::spawn(store, Arc::new(NoopOverflowSink), batcher_config());
    let reclaim_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    reclaim_store
        .preflight()
        .await
        .expect("preflight process-ledger reclaim authority");
    let killer = Arc::new(ProductionSandboxKill::new(
        pool.clone(),
        tokio::runtime::Handle::current(),
    ));
    let reclaim = Reclaim::new(reclaim_store, killer, Arc::new(ledger.clone()));
    (reclaim, ledger, join)
}

async fn single_connection_identity_pool(schema_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(schema_url)
        .await
        .expect("connect one-slot identity-query pool")
}

async fn wait_for_stop_reason(pool: &PgPool, process_uuid: Uuid, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let reason: Option<String> = sqlx::query_scalar(
            "SELECT stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_one(pool)
        .await
        .expect("read reclaim stop_reason while synchronizing the proof");
        if reason.as_deref() == Some(expected) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "process {process_uuid} did not reach stop_reason={expected}; observed {reason:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn stopped_at(pool: &PgPool, process_uuid: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar("SELECT stopped_at FROM kernel_process_lifecycle WHERE process_uuid = $1")
        .bind(process_uuid)
        .fetch_one(pool)
        .await
        .expect("read stopped_at")
}

async fn open_row_count(pool: &PgPool, process_uuid: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_lifecycle WHERE process_uuid = $1 AND stopped_at IS NULL",
    )
    .bind(process_uuid)
    .fetch_one(pool)
    .await
    .expect("count open rows")
}

async fn stopped_row_count(pool: &PgPool, process_uuid: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_lifecycle WHERE process_uuid = $1 AND stopped_at IS NOT NULL",
    )
    .bind(process_uuid)
    .fetch_one(pool)
    .await
    .expect("count stopped rows")
}

async fn drain(
    ledger: LedgerBatcher,
    join: tokio::task::JoinHandle<Result<(), handshake_core::process_ledger::ProcessLedgerError>>,
) {
    ledger.begin_close();
    tokio::time::timeout(Duration::from_secs(10), join)
        .await
        .expect("ledger writer drains within the bound")
        .expect("ledger writer task must not panic")
        .expect("ledger writer flushes successfully");
}

// ---------------------------------------------------------------------------
// (a) START durable -> production reclaim kills the real child -> durable STOP
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_process_reclaim_kills_child_and_writes_durable_stop() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
    )
    .await;

    // START is durable and the process is treated as live (open row, no STOP)
    // before any reclaim runs.
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(stopped_at(&pool, process_uuid).await.is_none());
    assert!(
        spawned.guard.is_still_running(),
        "child must be live pre-reclaim"
    );

    let (reclaim, ledger, join) = build_reclaim(&pool);
    let report = reclaim
        .run(&session_run_id, ReclaimTrigger::Restart)
        .await
        .expect("production reclaim run");

    assert_eq!(
        report.processes_reclaimed.len(),
        1,
        "exactly one process reclaimed"
    );
    let reclaimed = &report.processes_reclaimed[0];
    assert_eq!(reclaimed.process_uuid, process_uuid);
    assert!(
        matches!(
            reclaimed.kill_result,
            handshake_core::process_ledger::KillOutcome::Killed
        ),
        "production kill must succeed against the real child: {:?}",
        reclaimed.kill_result
    );

    // The REAL child is gone.
    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the real child process must be terminated by the production reclaim path"
    );

    drain(ledger, join).await;

    // Durable STOP row.
    assert!(
        stopped_at(&pool, process_uuid).await.is_some(),
        "reclaim must write a durable STOP row"
    );
    pool.close().await;
}

// ---------------------------------------------------------------------------
// MT-019: a production identity read may wait beyond the obsolete five-second
// sub-budget, but remains inside the authoritative 30-second reclaim bound.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_production_kill_identity_wait_uses_authoritative_reclaim_budget() {
    let (kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
    )
    .await;

    let identity_pool = single_connection_identity_pool(&kp.schema_url).await;
    let held_identity_connection = identity_pool
        .acquire()
        .await
        .expect("exhaust the exact production identity pool");
    let (reclaim, ledger, join) = build_reclaim_with_kill_pool(&pool, &identity_pool);
    let task_session = session_run_id.clone();
    let mut reclaim_task =
        tokio::spawn(async move { reclaim.run(&task_session, ReclaimTrigger::Restart).await });

    // This durable transition occurs immediately before `SandboxKill::kill`, so
    // it proves the task reached the identity-read boundary before the clock.
    wait_for_stop_reason(&pool, process_uuid, "reclaim_kill_in_progress").await;
    let identity_wait_started = Instant::now();
    tokio::time::sleep(IDENTITY_POOL_HOLD).await;
    assert!(
        !reclaim_task.is_finished(),
        "production kill returned while its exact identity pool remained exhausted for {IDENTITY_POOL_HOLD:?}; the obsolete five-second identity sub-budget is still active"
    );

    drop(held_identity_connection);
    let report = tokio::time::timeout(IDENTITY_PATH_COMPLETION_BOUND, &mut reclaim_task)
        .await
        .expect("production kill completes after the identity connection is released")
        .expect("production kill task must not panic")
        .expect("production reclaim succeeds after a >5s identity wait");
    let identity_path_elapsed = identity_wait_started.elapsed();
    assert!(identity_path_elapsed >= IDENTITY_POOL_HOLD);
    assert!(
        identity_path_elapsed < AUTHORITATIVE_RECLAIM_KILL_BOUND,
        "identity wait must remain inside the authoritative reclaim kill bound: {identity_path_elapsed:?}"
    );
    assert_eq!(report.processes_reclaimed.len(), 1);
    assert!(matches!(
        report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    ));
    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the production adapter must kill the owned real child"
    );

    drain(ledger, join).await;
    assert_eq!(open_row_count(&pool, process_uuid).await, 0);
    assert_eq!(
        stopped_row_count(&pool, process_uuid).await,
        1,
        "the delayed identity read must still produce exactly one durable STOP"
    );
    eprintln!(
        "MT019_NON_SKIP kill_identity_wait_ms={} child_pid={} durable_stops=1",
        identity_path_elapsed.as_millis(),
        spawned.pid
    );
    identity_pool.close().await;
    pool.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_crash_left_status_identity_wait_uses_authoritative_reclaim_budget() {
    let (kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
    )
    .await;

    // Simulate a crash after the durable kill-start fence but before the adapter
    // call. The fresh boot must query that crash-left operation without turning
    // open/in-progress evidence into a false STOP.
    let prior_boot_store = PostgresProcessLedgerStore::new(pool.clone());
    let mut claimed = prior_boot_store
        .active_processes_for_session(&session_run_id)
        .await
        .expect("claim the real process before the simulated crash");
    assert_eq!(claimed.len(), 1);
    let claimed_process = claimed.pop().expect("one claimed process");
    prior_boot_store
        .mark_reclaim_kill_started(process_uuid, &claimed_process.reclaim_claim)
        .await
        .expect("persist the crash-left kill-operation fence");
    assert!(spawned.guard.is_still_running());
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(stopped_at(&pool, process_uuid).await.is_none());

    let identity_pool = single_connection_identity_pool(&kp.schema_url).await;
    let held_identity_connection = identity_pool
        .acquire()
        .await
        .expect("exhaust the crash-recovery identity pool");
    let (reclaim, ledger, join) = build_reclaim_with_kill_pool(&pool, &identity_pool);
    let reclaim = Arc::new(reclaim);
    let status_reclaim = Arc::clone(&reclaim);
    let kill_operation_uuid = claimed_process.reclaim_claim.kill_operation_uuid;
    let mut status_task = tokio::spawn(async move {
        status_reclaim
            .reconcile_kill_operation(process_uuid, kill_operation_uuid)
            .await
    });

    let identity_wait_started = Instant::now();
    tokio::time::sleep(IDENTITY_POOL_HOLD).await;
    assert!(
        !status_task.is_finished(),
        "crash-left status returned while its exact identity pool remained exhausted for {IDENTITY_POOL_HOLD:?}; the obsolete five-second identity sub-budget is still active"
    );
    drop(held_identity_connection);
    let status = tokio::time::timeout(IDENTITY_PATH_COMPLETION_BOUND, &mut status_task)
        .await
        .expect("crash-left status completes after the identity connection is released")
        .expect("crash-left status task must not panic")
        .expect("crash-left status succeeds after a >5s identity wait");
    let identity_path_elapsed = identity_wait_started.elapsed();
    assert_eq!(status, ReclaimKillOperationStatus::InProgress);
    assert!(identity_path_elapsed >= IDENTITY_POOL_HOLD);
    assert!(
        identity_path_elapsed < AUTHORITATIVE_RECLAIM_KILL_BOUND,
        "crash-left identity wait must remain inside the authoritative reclaim kill bound: {identity_path_elapsed:?}"
    );
    assert!(
        spawned.guard.is_still_running(),
        "an in-progress status observation must not kill outside the retry path"
    );
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(
        stopped_at(&pool, process_uuid).await.is_none(),
        "in-progress crash recovery evidence must not fabricate STOP"
    );

    // The test knows the simulated prior boot crashed before calling its
    // adapter, so NotStarted is the truthful authoritative terminal evidence.
    // Apply it through the production store transition, then prove the normal
    // retry path owns the eventual real kill and STOP.
    prior_boot_store
        .resolve_reclaim_kill_operation(
            process_uuid,
            kill_operation_uuid,
            ReclaimKillOperationStatus::NotStarted,
        )
        .await
        .expect("release the crash-left claim from truthful not-started evidence");

    let report = reclaim
        .run(&session_run_id, ReclaimTrigger::Restart)
        .await
        .expect("retry the crash-left in-progress operation after lease expiry");
    assert_eq!(report.processes_reclaimed.len(), 1);
    assert!(matches!(
        report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    ));
    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the production retry must kill the owned real child"
    );
    drain(ledger, join).await;
    assert_eq!(open_row_count(&pool, process_uuid).await, 0);
    assert_eq!(
        stopped_row_count(&pool, process_uuid).await,
        1,
        "crash-left recovery must produce exactly one durable STOP"
    );
    eprintln!(
        "MT019_NON_SKIP crash_status_identity_wait_ms={} child_pid={} durable_stops=1",
        identity_path_elapsed.as_millis(),
        spawned.pid
    );
    identity_pool.close().await;
    pool.close().await;
}

// ---------------------------------------------------------------------------
// (b) crash/restart reconciliation of a prior-boot orphan
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn boot_reconcile_kills_prior_boot_orphan_and_writes_durable_stop() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();

    // Prior boot: a durable orphan START row remains. The prior process's
    // in-memory writer state is gone (nothing else references it) — only the
    // durable PostgreSQL row survives, exactly like a hard crash.
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
    )
    .await;
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(spawned.guard.is_still_running());

    // Fresh boot: brand-new reclaim stack reconciles the orphan the way
    // `ProcessReclaimRuntime`'s boot reconcile does (in-progress sweep, then a
    // Restart-triggered reclaim of the session).
    let (reclaim, ledger, join) = build_reclaim(&pool);
    reclaim
        .reconcile_in_progress_for_session(&session_run_id)
        .await
        .expect("boot in-progress kill-operation reconcile");
    let report = reclaim
        .run(&session_run_id, ReclaimTrigger::Restart)
        .await
        .expect("boot reconcile reclaim run");

    assert_eq!(report.processes_reclaimed.len(), 1);
    assert!(
        matches!(
            report.processes_reclaimed[0].kill_result,
            handshake_core::process_ledger::KillOutcome::Killed
        ),
        "fresh boot must kill the prior-boot orphan: {:?}",
        report.processes_reclaimed[0].kill_result
    );
    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the prior-boot orphan child must be terminated on the new boot"
    );

    drain(ledger, join).await;
    assert!(
        stopped_at(&pool, process_uuid).await.is_some(),
        "boot reconcile must write a durable STOP row"
    );
    pool.close().await;
}

// ---------------------------------------------------------------------------
// (c) concurrent-reclaimer safety: exactly one kill + one durable STOP
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_reclaimers_produce_exactly_one_kill_and_one_stop() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
    )
    .await;

    // Two independent reclaimers over the same real pool + same process record.
    let (reclaim_a, ledger_a, join_a) = build_reclaim(&pool);
    let (reclaim_b, ledger_b, join_b) = build_reclaim(&pool);
    let session_a = session_run_id.clone();
    let session_b = session_run_id.clone();
    let (result_a, result_b) = tokio::join!(
        async move { reclaim_a.run(&session_a, ReclaimTrigger::Restart).await },
        async move { reclaim_b.run(&session_b, ReclaimTrigger::Restart).await },
    );

    // No error escalation: both racing reclaimers return Ok.
    let report_a = result_a.expect("reclaimer A must not error-escalate on a race");
    let report_b = result_b.expect("reclaimer B must not error-escalate on a race");

    let killed_with_stop: usize = [&report_a, &report_b]
        .iter()
        .flat_map(|report| report.processes_reclaimed.iter())
        .filter(|reclaimed| {
            reclaimed.process_uuid == process_uuid
                && matches!(
                    reclaimed.kill_result,
                    handshake_core::process_ledger::KillOutcome::Killed
                )
                && reclaimed.stop_event_kind
                    == Some(handshake_core::process_ledger::LedgerEventKind::Stop)
        })
        .count();
    // Surface WHAT the reclaimers actually reported. A bare `0 != 1` cannot
    // distinguish "neither reclaimer saw the process at all" from "both saw it
    // but declined to kill" from "one killed it but wrote no STOP" - three
    // different defects with three different fixes. The counts alone sent an
    // earlier investigation down the wrong path.
    let observed: Vec<String> = [&report_a, &report_b]
        .iter()
        .flat_map(|report| report.processes_reclaimed.iter())
        .map(|reclaimed| {
            format!(
                "{{uuid={}, kill={:?}, stop={:?}}}",
                reclaimed.process_uuid, reclaimed.kill_result, reclaimed.stop_event_kind
            )
        })
        .collect();
    assert_eq!(
        killed_with_stop, 1,
        "exactly one reclaimer may kill + STOP the shared process (no double-STOP). \
         target process_uuid={process_uuid}, real child pid={}. \
         Reclaimed entries observed across BOTH reclaimers: {observed:?} \
         (an empty list means neither reclaimer selected this process as reclaimable at all)",
        spawned.pid
    );

    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the real child must be terminated exactly once by the winning reclaimer"
    );

    drain(ledger_a, join_a).await;
    drain(ledger_b, join_b).await;

    // Exactly one durable STOP: the single lifecycle row is stopped once.
    assert_eq!(open_row_count(&pool, process_uuid).await, 0);
    assert!(stopped_at(&pool, process_uuid).await.is_some());
    pool.close().await;
}

// ---------------------------------------------------------------------------
// (d) PID-reuse guard: mismatched generation identity must NOT kill
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pid_reuse_guard_refuses_to_kill_a_mismatched_generation() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    // Record a creation-generation identity that does NOT match the live PID's
    // real generation: models a reused PID belonging to a different process
    // generation than the one the START row was written for.
    let mismatched_creation = spawned.os_creation_time_100ns.wrapping_add(1);
    seed_real_process_start(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        mismatched_creation,
        &spawned.executable_sha256,
    )
    .await;

    let (reclaim, ledger, join) = build_reclaim(&pool);
    let report = reclaim
        .run(&session_run_id, ReclaimTrigger::Restart)
        .await
        .expect("reclaim run returns Ok and fails closed at the process level");

    assert_eq!(report.processes_reclaimed.len(), 1);
    let reclaimed = &report.processes_reclaimed[0];
    assert!(
        matches!(
            reclaimed.kill_result,
            handshake_core::process_ledger::KillOutcome::Failed { .. }
        ),
        "the identity fence must fail closed on a generation mismatch, not kill: {:?}",
        reclaimed.kill_result
    );
    assert!(
        reclaimed.stop_event_kind.is_none(),
        "no STOP may be written when the identity fence refuses the kill"
    );

    // The live process with the reused PID must be untouched.
    assert!(
        spawned.guard.is_still_running(),
        "the mismatched-generation live process must NOT be killed"
    );

    drain(ledger, join).await;

    // Fail-closed: the row remains truthfully open (never falsely stopped).
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(stopped_at(&pool, process_uuid).await.is_none());

    // Teardown kills the process THIS test spawned (via ChildGuard::drop).
    pool.close().await;
}

// ---------------------------------------------------------------------------
// (e) WP-1 MT-007 V4 close-out: the COMPOSED product-boot reclaimer surfaces AND
//     reclaims a generic spawned-process (Official-CLI bridge) orphan.
//
// Proofs (a)-(d) call `reclaim.run(session_id, ...)` with a KNOWN session id,
// bypassing the surfacing step. The V4 gap was that the generic spawned-process
// Reclaim/SandboxKill path was not wired into product boot: an Official-CLI START
// row could stay OPEN after crash with no production owner to kill + STOP it.
// This proof drives the EXACT production composition product boot runs —
// `PostgresModelLaneStaleSessionSource::restart_sessions()` (PostgreSQL-authoritative
// surfacing) -> `reconcile_restart_orphans_at_boot` -> `reconcile_in_progress` +
// `run(ReclaimTrigger::Restart)` -> `ProductionSandboxKill` + durable STOP —
// against a real official-CLI orphan whose prior owning runtime instance is
// provably dead, and asserts it is surfaced, killed, and STOPped.
// ---------------------------------------------------------------------------

/// Bind an ephemeral loopback UDP port, capture it, then release it. The freed
/// port makes the seeded prior owner's loopback lease read as DEAD to the
/// restart sweep's exclusive-bind liveness probe, exactly as a crashed prior
/// Handshake instance would leave it.
///
/// MT-019 F7: the drop-then-use window lets an unrelated host process take the
/// ephemeral port, which would silently veto surfacing and make the proof pass
/// for the wrong reason. Re-probe the freed port and retry with a different one
/// until it is observed free. The residual race is unavoidable with ephemeral
/// ports; the retry loop makes it rare instead of routine.
fn free_loopback_udp_port() -> u16 {
    for _ in 0..16 {
        let socket =
            UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind ephemeral loopback UDP");
        let port = socket
            .local_addr()
            .expect("read bound loopback addr")
            .port();
        drop(socket);
        match UdpSocket::bind((Ipv4Addr::LOCALHOST, port)) {
            Ok(reprobe) => {
                drop(reprobe);
                return port;
            }
            Err(_) => continue,
        }
    }
    panic!("could not obtain a stably-free ephemeral loopback UDP port after 16 attempts");
}

/// Seed a durable Official-CLI bridge START row exactly as the production
/// cloud-lane spawn records it: `engine_kind = 'official_cli_bridge'`, the native
/// sandbox adapter, the real OS identity, and a typed runtime-owner descriptor
/// pointing at a PRIOR (crashed, distinct-instance) loopback lease. The freed
/// `prior_port` is what proves the prior owner dead to `restart_sessions()`.
#[allow(clippy::too_many_arguments)]
async fn seed_official_cli_start_with_dead_prior_owner(
    pool: &PgPool,
    session_run_id: &str,
    process_uuid: Uuid,
    pid: u32,
    creation_time: u64,
    executable_sha256: &str,
    prior_instance_id: Uuid,
    host_scope: &str,
    prior_port: u16,
) {
    let metadata = json!({
        "executable_sha256": executable_sha256,
        "os_creation_time_100ns": creation_time,
        "sandbox_handle_id": process_uuid.to_string(),
    });
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, os_pid, parent_session_id, sandbox_adapter_id,
            sandbox_internal_id, engine_kind, started_at, owner_role, owner_wp,
            owner_runtime_instance_id, owner_host_scope_id, owner_lease_schema_id,
            owner_lease_protocol, owner_lease_address, owner_lease_port,
            metadata_jsonb
        )
        VALUES (
            $1, $2, $3, $4, $5, 'official_cli_bridge', NOW(), 'coder', 'WP-1',
            $6::uuid, $7, $8, $9, $10, $11, $12
        )
        "#,
    )
    .bind(process_uuid)
    .bind(i64::from(pid))
    .bind(session_run_id)
    .bind(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
    .bind(process_uuid.to_string())
    .bind(prior_instance_id)
    .bind(host_scope)
    .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
    .bind(EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL)
    .bind("127.0.0.1")
    .bind(i32::from(prior_port))
    .bind(metadata)
    .execute(pool)
    .await
    .expect("seed durable official-CLI START row with a dead prior runtime-owner");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn boot_reconcile_via_restart_sessions_reclaims_official_cli_orphan() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    // This boot's LIVE runtime instance. Its OS liveness lease is held for the
    // whole test so its own loopback port can never be mistaken for free.
    let host_scope = "wp1-mt007-boot-reclaim-host";
    let this_boot_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire this boot's OS liveness lease");
    let this_descriptor = this_boot_lease.descriptor().clone();

    // A PRIOR (crashed) instance on the SAME host: a distinct instance id whose
    // loopback UDP lease port is now FREE (its process is gone).
    let prior_instance_id = Uuid::now_v7();
    assert_ne!(prior_instance_id, this_descriptor.instance_id);
    let prior_port = free_loopback_udp_port();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_official_cli_start_with_dead_prior_owner(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        prior_instance_id,
        host_scope,
        prior_port,
    )
    .await;

    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(stopped_at(&pool, process_uuid).await.is_none());
    assert!(
        spawned.guard.is_still_running(),
        "the official-CLI child must be live pre-reclaim"
    );

    // Fresh boot: the PostgreSQL-authoritative stale-session source + the EXACT
    // production composed boot reclaimer. No session id is passed in; the
    // official-CLI orphan must be SURFACED by `restart_sessions()` and then
    // killed + STOPped by `run(Restart)` through `ProductionSandboxKill`.
    //
    // MT-019 P-4(b) changes this proof's shape: a prior owner is only treated as
    // dead after its loopback lease has been observed free TWICE, at least one
    // confirmation gap apart. The gap is configured explicitly (rather than left
    // at the 30s production default) so the proof stays fast, and the FIRST pass
    // is asserted to reclaim nothing.
    let stale_source =
        PostgresModelLaneStaleSessionSource::new(pool.clone(), this_descriptor.clone())
            .with_dead_owner_confirmation_gap(Duration::from_millis(50));
    let (reclaim, ledger, join) = build_preflighted_reclaim(&pool).await;
    let first_pass = reconcile_restart_orphans_at_boot(&reclaim, &stale_source)
        .await
        .expect("first composed boot restart-reconcile pass");
    assert_eq!(
        first_pass.sessions_reconciled, 0,
        "one free-port sample is not liveness evidence; the first pass must reclaim nothing"
    );
    assert!(
        spawned.guard.is_still_running(),
        "a single dead-owner sample must never authorise a kill"
    );
    tokio::time::sleep(Duration::from_millis(80)).await;
    let report = reconcile_restart_orphans_at_boot(&reclaim, &stale_source)
        .await
        .expect("composed boot restart-reconcile pass");

    assert_eq!(
        report.sessions_reconciled, 1,
        "the official-CLI orphan session must be surfaced by restart_sessions() and reconciled"
    );
    let killed: usize = report
        .reclaim_reports
        .iter()
        .flat_map(|reclaim_report| reclaim_report.processes_reclaimed.iter())
        .filter(|reclaimed| {
            reclaimed.process_uuid == process_uuid
                && matches!(reclaimed.kill_result, KillOutcome::Killed)
        })
        .count();
    assert_eq!(
        killed, 1,
        "the composed boot reclaimer must kill the official-CLI orphan exactly once: {:?}",
        report.reclaim_reports
    );

    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the official-CLI orphan child must be terminated by the composed boot reclaimer"
    );

    drain(ledger, join).await;

    // Durable STOP + the START is no longer open: a production owner closed it.
    assert!(
        stopped_at(&pool, process_uuid).await.is_some(),
        "composed boot reclaim must write a durable STOP row for the official-CLI orphan"
    );
    assert_eq!(
        open_row_count(&pool, process_uuid).await,
        0,
        "the official-CLI START must no longer be open after composed boot reclaim"
    );

    drop(this_boot_lease);
    pool.close().await;
}

// ===========================================================================
// WP-1 MT-019: running-app reap, periodic restart tick, resilient boot, and the
// process-ownership safety prerequisites (P-2..P-5).
//
// Every proof below runs against REAL Handshake-managed PostgreSQL, a REAL
// spawned Windows child, and the production reclaim/kill path. The file is
// `#![cfg(windows)]` (the identity fence is Windows-only), which is called out
// explicitly where it limits coverage.
// ===========================================================================

/// Seed a durable official-CLI START row owned by the CALLER's LIVE runtime
/// instance. `parent_session_id` is optional because the real production class
/// this targets — the official-CLI auth-status/preflight probe — historically
/// wrote none, which is exactly why a session-keyed claim could not reach it.
#[allow(clippy::too_many_arguments)]
async fn seed_self_owned_official_cli_start(
    pool: &PgPool,
    parent_session_id: Option<&str>,
    process_uuid: Uuid,
    pid: u32,
    creation_time: u64,
    executable_sha256: &str,
    owner: &handshake_core::process_ledger::EmbeddedRuntimeInstanceDescriptor,
) {
    let metadata = json!({
        "executable_sha256": executable_sha256,
        "os_creation_time_100ns": creation_time,
        "sandbox_handle_id": process_uuid.to_string(),
    });
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, os_pid, parent_session_id, sandbox_adapter_id,
            sandbox_internal_id, engine_kind, started_at, owner_role, owner_wp,
            owner_runtime_instance_id, owner_host_scope_id, owner_lease_schema_id,
            owner_lease_protocol, owner_lease_address, owner_lease_port,
            metadata_jsonb
        )
        VALUES (
            $1, $2, $3, $4, $5, 'official_cli_bridge', NOW(), 'coder', 'WP-1',
            $6::uuid, $7, $8, $9, $10, $11, $12
        )
        "#,
    )
    .bind(process_uuid)
    .bind(i64::from(pid))
    .bind(parent_session_id)
    .bind(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
    .bind(process_uuid.to_string())
    .bind(owner.instance_id)
    .bind(&owner.host_scope_id)
    .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
    .bind(EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL)
    .bind(owner.loopback_address.to_string())
    .bind(i32::from(owner.loopback_port))
    .bind(metadata)
    .execute(pool)
    .await
    .expect("seed durable self-owned official-CLI START row");
}

/// Seed an exact-owner lifecycle row that is deliberately outside the stale
/// source's sandbox-owned authorization class. Sharing a session with an
/// authorized stale child must never widen the later claim/recovery queries to
/// this row.
#[allow(clippy::too_many_arguments)]
async fn seed_self_owned_non_sandbox_start(
    pool: &PgPool,
    parent_session_id: &str,
    process_uuid: Uuid,
    pid: u32,
    creation_time: u64,
    executable_sha256: &str,
    owner: &handshake_core::process_ledger::EmbeddedRuntimeInstanceDescriptor,
) {
    let metadata = json!({
        "executable_sha256": executable_sha256,
        "os_creation_time_100ns": creation_time,
    });
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, os_pid, parent_session_id, sandbox_adapter_id,
            sandbox_internal_id, engine_kind, started_at, owner_role, owner_wp,
            owner_runtime_instance_id, owner_host_scope_id, owner_lease_schema_id,
            owner_lease_protocol, owner_lease_address, owner_lease_port,
            metadata_jsonb
        )
        VALUES (
            $1, $2, $3, NULL, NULL, 'official_cli_bridge', NOW(), 'coder', 'WP-1',
            $4::uuid, $5, $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(process_uuid)
    .bind(i64::from(pid))
    .bind(parent_session_id)
    .bind(owner.instance_id)
    .bind(&owner.host_scope_id)
    .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
    .bind(EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL)
    .bind(owner.loopback_address.to_string())
    .bind(i32::from(owner.loopback_port))
    .bind(metadata)
    .execute(pool)
    .await
    .expect("seed durable self-owned non-sandbox START row");
}

async fn seed_terminal_lane_for_process(pool: &PgPool, session_id: &str, process_uuid: Uuid) {
    let run_id = format!("RUN-MT019-{}", Uuid::now_v7());
    let lane_id = format!("LANE-MT019-{}", Uuid::now_v7());
    let stream_id = format!("STREAM-MT019-{}", Uuid::now_v7());
    let run_event_id = format!("EVT-MT019-RUN-{}", Uuid::now_v7());
    let lane_event_id = format!("EVT-MT019-LANE-{}", Uuid::now_v7());
    let run_sequence: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type,
            actor_kind, actor_id, payload_hash, source_component, payload
        ) VALUES ($1, '1', $2, $2, 'mt019_stale_scope', $3, $4,
                  'mt019_stale_scope', 'test', 'mt019', $5,
                  'process_reclaim_real_lifecycle_pg_tests', '{}'::jsonb)
        RETURNING event_sequence
        "#,
    )
    .bind(&run_event_id)
    .bind(session_id)
    .bind(&run_id)
    .bind(format!("IDEM-{run_event_id}"))
    .bind("0".repeat(64))
    .fetch_one(pool)
    .await
    .expect("insert MT-019 stale-scope run event");
    sqlx::query(
        r#"
        INSERT INTO model_lane_runs (
            run_id, trace_id, run_span_id, coordinator_session_id,
            work_packet_id, micro_task_id, task_board_id, owner_session,
            idempotency_key, replay_order_key, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json
        ) VALUES ($1,$2,$3,$4,'WP-1','MT-019','TB-MT019','OWNER-MT019',
                  $5,$6,$7,$8,$9,$10)
        "#,
    )
    .bind(&run_id)
    .bind(format!("TRACE-{run_id}"))
    .bind(format!("SPAN-{run_id}"))
    .bind(session_id)
    .bind(format!("IDEM-{run_id}"))
    .bind(format!("REPLAY-{run_id}"))
    .bind(&stream_id)
    .bind(&run_event_id)
    .bind(run_sequence)
    .bind(json!({"run_id": run_id.clone(), "coordinator_session_id": session_id}))
    .execute(pool)
    .await
    .expect("insert MT-019 stale-scope run");

    let lane_sequence: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type,
            actor_kind, actor_id, payload_hash, source_component, payload
        ) VALUES ($1, '1', $2, $2, 'mt019_stale_scope', $3, $4,
                  'mt019_stale_scope', 'test', 'mt019', $5,
                  'process_reclaim_real_lifecycle_pg_tests', '{}'::jsonb)
        RETURNING event_sequence
        "#,
    )
    .bind(&lane_event_id)
    .bind(session_id)
    .bind(&lane_id)
    .bind(format!("IDEM-{lane_event_id}"))
    .bind("0".repeat(64))
    .fetch_one(pool)
    .await
    .expect("insert MT-019 stale-scope lane event");
    sqlx::query(
        r#"
        INSERT INTO model_lanes (
            lane_id, run_id, trace_id, lane_span_id, kind, runtime_binding,
            launch_authority, status, work_packet_id, micro_task_id,
            task_board_id, owner_session, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json
        ) VALUES ($1,$2,$3,$4,'worker','local','session_broker','completed',
                  'WP-1','MT-019','TB-MT019','OWNER-MT019',$5,$6,$7,$8)
        "#,
    )
    .bind(&lane_id)
    .bind(&run_id)
    .bind(format!("TRACE-{run_id}"))
    .bind(format!("SPAN-{lane_id}"))
    .bind(&stream_id)
    .bind(&lane_event_id)
    .bind(lane_sequence)
    .bind(json!({
        "lane_id": lane_id,
        "run_id": run_id,
        "coordinator_session_id": session_id,
        "process_ownership_ref": format!("process-ledger://{process_uuid}"),
        "status": "completed"
    }))
    .execute(pool)
    .await
    .expect("insert MT-019 terminal lane");
}

async fn lifecycle_row_state(
    pool: &PgPool,
    process_uuid: Uuid,
) -> (Option<String>, serde_json::Value) {
    sqlx::query_as(
        "SELECT stop_reason, metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(pool)
    .await
    .expect("read lifecycle row state")
}

// ---------------------------------------------------------------------------
// F1 + P-2: the RUNNING app reaps its own mid-run official-CLI orphan, without
// a reboot and without a coordinator session id.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_running_app_reclaims_same_instance_official_cli_orphan_without_reboot() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    // The LIVE instance keeps its OS liveness lease for the whole test, exactly
    // as a running app does.
    let host_scope = "wp1-mt019-running-app-host";
    let live_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire this instance's OS liveness lease");
    let live_descriptor = live_lease.descriptor().clone();

    // The exact production shape of an official-CLI auth-status/preflight child:
    // adapter-owned, self-owned, and carrying NO parent_session_id.
    let process_uuid = Uuid::now_v7();
    seed_self_owned_official_cli_start(
        &pool,
        None,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        &live_descriptor,
    )
    .await;
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(spawned.guard.is_still_running());

    // Neither session-level surfacing path can reach this row: `stale_sessions`
    // filters `parent_session_id IS NOT NULL`, and `restart_sessions` requires it
    // too AND vetoes a live self-owned instance. This is P-2 stated as evidence,
    // not as an argument: without the process-scoped path the row is unreachable.
    let stale_source =
        PostgresModelLaneStaleSessionSource::new(pool.clone(), live_descriptor.clone());
    assert!(
        stale_source
            .stale_sessions(Duration::from_secs(1))
            .await
            .expect("stale scan must not fail on a NULL parent_session_id row")
            .is_empty(),
        "the session-scoped stale scan must not surface a session-less row"
    );
    assert!(
        stale_source
            .restart_sessions()
            .await
            .expect("restart scan")
            .is_empty(),
        "the restart scan must not surface a session-less, live-self-owned row"
    );

    // The running-app reap: owner-scoped, keyed on process_uuid alone.
    let (reclaim, ledger, join) = build_reclaim(&pool);
    let report = reclaim
        .run_owned_process(
            process_uuid,
            live_descriptor.instance_id,
            ReclaimTrigger::Failure,
        )
        .await
        .expect("running-app owner-scoped reclaim");

    let killed = report
        .processes_reclaimed
        .iter()
        .filter(|reclaimed| {
            reclaimed.process_uuid == process_uuid
                && matches!(reclaimed.kill_result, KillOutcome::Killed)
        })
        .count();
    assert_eq!(
        killed, 1,
        "the EXACT process_uuid must appear in processes_reclaimed with KillOutcome::Killed: {:?}",
        report.processes_reclaimed
    );

    assert!(
        spawned.guard.wait_exited(Duration::from_secs(10)),
        "the running-app reap must terminate the real orphan child"
    );

    drain(ledger, join).await;
    assert!(
        stopped_at(&pool, process_uuid).await.is_some(),
        "the running-app reap must write a durable STOP row"
    );
    assert_eq!(open_row_count(&pool, process_uuid).await, 0);

    drop(live_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// F1 negative proof: the running-app reap must never kill a LIVE same-instance
// child.
//
// This is asserted at the SURFACING layer on purpose. The identity fence matches
// a live child perfectly (it proves process generation, not liveness), so a proof
// written against the claim path would legitimately kill the child. The property
// that actually protects a live child is that no automatic pass ever surfaces it.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_running_app_reap_never_kills_a_live_same_instance_child() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut sessionless = spawn_real_long_lived_child();
    let mut with_session = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-live-child-host";
    let live_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire this instance's OS liveness lease");
    let live_descriptor = live_lease.descriptor().clone();

    let sessionless_uuid = Uuid::now_v7();
    seed_self_owned_official_cli_start(
        &pool,
        None,
        sessionless_uuid,
        sessionless.pid,
        sessionless.os_creation_time_100ns,
        &sessionless.executable_sha256,
        &live_descriptor,
    )
    .await;
    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let with_session_uuid = Uuid::now_v7();
    seed_self_owned_official_cli_start(
        &pool,
        Some(&session_run_id),
        with_session_uuid,
        with_session.pid,
        with_session.os_creation_time_100ns,
        &with_session.executable_sha256,
        &live_descriptor,
    )
    .await;

    let stale_source =
        PostgresModelLaneStaleSessionSource::new(pool.clone(), live_descriptor.clone())
            .with_dead_owner_confirmation_gap(Duration::from_millis(1));
    assert!(
        stale_source
            .stale_sessions(Duration::from_secs(1))
            .await
            .expect("stale scan")
            .is_empty(),
        "a live same-instance child carries no liveness evidence in its row; the stale scan must not surface it"
    );
    assert!(
        stale_source
            .restart_sessions()
            .await
            .expect("restart scan")
            .is_empty(),
        "the restart scan must veto sessions owned by THIS live instance"
    );

    // Drive the exact composed pass the boot path and the periodic tick both run,
    // twice, so even a corroborated dead-owner probe cannot help it surface.
    let (reclaim, ledger, join) = build_reclaim(&pool);
    for _ in 0..2 {
        let report = reconcile_restart_orphans_at_boot(&reclaim, &stale_source)
            .await
            .expect("composed restart-reconcile pass");
        assert_eq!(report.sessions_reconciled, 0);
        assert_eq!(report.processes_reclaimed, 0);
        assert_eq!(report.processes_kill_failed, 0);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert!(
        sessionless.guard.is_still_running(),
        "a LIVE session-less same-instance child must never be killed by an automatic pass"
    );
    assert!(
        with_session.guard.is_still_running(),
        "a LIVE same-instance child with a session id must never be killed by an automatic pass"
    );
    assert_eq!(open_row_count(&pool, sessionless_uuid).await, 1);
    assert_eq!(open_row_count(&pool, with_session_uuid).await, 1);

    drain(ledger, join).await;
    drop(live_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// P-3: the single-row claim must not touch sibling rows of the same session.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_single_row_claim_leaves_sibling_reclaim_metadata_untouched() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut target = spawn_real_long_lived_child();
    let mut sibling = spawn_real_long_lived_child();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let target_uuid = Uuid::now_v7();
    let sibling_uuid = Uuid::now_v7();
    seed_real_process_start(
        &pool,
        &session_run_id,
        target_uuid,
        target.pid,
        target.os_creation_time_100ns,
        &target.executable_sha256,
    )
    .await;
    seed_real_process_start(
        &pool,
        &session_run_id,
        sibling_uuid,
        sibling.pid,
        sibling.os_creation_time_100ns,
        &sibling.executable_sha256,
    )
    .await;

    let before = lifecycle_row_state(&pool, sibling_uuid).await;

    let (reclaim, ledger, join) = build_reclaim(&pool);
    let report = reclaim
        .run_process(&session_run_id, target_uuid, ReclaimTrigger::Failure)
        .await
        .expect("exact-process reclaim");
    assert_eq!(
        report.processes_reclaimed.len(),
        1,
        "an exact-process reclaim must act on exactly one row: {:?}",
        report.processes_reclaimed
    );
    assert_eq!(report.processes_reclaimed[0].process_uuid, target_uuid);
    assert!(target.guard.wait_exited(Duration::from_secs(10)));

    let after = lifecycle_row_state(&pool, sibling_uuid).await;
    assert_eq!(
        before, after,
        "the sibling row's stop_reason and reclaim_claim metadata must be byte-identical across an exact-process reclaim of a DIFFERENT process in the same session"
    );
    assert!(
        sibling.guard.is_still_running(),
        "the healthy sibling lane must still be running"
    );
    assert_eq!(open_row_count(&pool, sibling_uuid).await, 1);

    drain(ledger, join).await;
    pool.close().await;
}

// ---------------------------------------------------------------------------
// MT-019: stale selection and the atomic claim keep the same owner boundary.
// A foreign live process sharing the coordinator session must survive even when
// two stale reclaimers race the selected self-owned process.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_concurrent_stale_reclaim_preserves_foreign_same_session_process() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let (reclaim, ledger, join) = build_preflighted_reclaim(&pool).await;
    let reclaim = Arc::new(reclaim);
    let mut owned_child = spawn_real_long_lived_child();
    let mut foreign_child = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-stale-owner-scope-host";
    let owned_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire stale source owner lease");
    let foreign_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire foreign live owner lease");
    let owned = owned_lease.descriptor().clone();
    let foreign = foreign_lease.descriptor().clone();
    let session_id = format!("SR-{}", Uuid::now_v7());
    let owned_uuid = Uuid::now_v7();
    let foreign_uuid = Uuid::now_v7();

    seed_self_owned_official_cli_start(
        &pool,
        Some(&session_id),
        owned_uuid,
        owned_child.pid,
        owned_child.os_creation_time_100ns,
        &owned_child.executable_sha256,
        &owned,
    )
    .await;
    seed_self_owned_official_cli_start(
        &pool,
        Some(&session_id),
        foreign_uuid,
        foreign_child.pid,
        foreign_child.os_creation_time_100ns,
        &foreign_child.executable_sha256,
        &foreign,
    )
    .await;
    seed_terminal_lane_for_process(&pool, &session_id, owned_uuid).await;

    let source = PostgresModelLaneStaleSessionSource::new(pool.clone(), owned.clone());
    let candidates = source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("owner-scoped stale selection");
    assert_eq!(
        candidates.len(),
        1,
        "the owned terminal lane must surface its coordinator session"
    );
    let candidate = &candidates[0];
    assert_eq!(candidate.session_id, session_id);
    assert_eq!(candidate.authorized_process_uuids, vec![owned_uuid]);

    let reclaim_a = Arc::clone(&reclaim);
    let reclaim_b = Arc::clone(&reclaim);
    let (result_a, result_b) = tokio::join!(
        reclaim_a.run_stale_owned_session(
            &session_id,
            owned.instance_id,
            host_scope,
            &candidate.authorized_process_uuids,
        ),
        reclaim_b.run_stale_owned_session(
            &session_id,
            owned.instance_id,
            host_scope,
            &candidate.authorized_process_uuids,
        ),
    );
    let reports = [
        result_a.expect("first scoped stale reclaimer"),
        result_b.expect("second scoped stale reclaimer"),
    ];
    let owned_kills = reports
        .iter()
        .flat_map(|report| report.processes_reclaimed.iter())
        .filter(|process| {
            process.process_uuid == owned_uuid && matches!(process.kill_result, KillOutcome::Killed)
        })
        .count();
    assert_eq!(
        owned_kills, 1,
        "the owned process must be killed exactly once"
    );
    assert!(owned_child.guard.wait_exited(Duration::from_secs(10)));

    assert!(
        foreign_child.guard.is_still_running(),
        "a foreign live owner in the same session must never be killed by stale reclaim"
    );
    assert_eq!(open_row_count(&pool, foreign_uuid).await, 1);
    assert!(
        stopped_at(&pool, foreign_uuid).await.is_none(),
        "foreign same-session lifecycle must not receive a false STOP"
    );
    let foreign_state = lifecycle_row_state(&pool, foreign_uuid).await;
    assert_eq!(
        foreign_state.0, None,
        "foreign row must not even be claimed"
    );
    assert!(foreign_state.1.get("reclaim_claim").is_none());

    drain(ledger, join).await;
    eprintln!(
        "MT019_NON_SKIP stale_owner_scope owned_pid={} foreign_pid={} owned_kills=1 foreign_stops=0",
        owned_child.pid, foreign_child.pid
    );
    drop(foreign_lease);
    drop(owned_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// MT-019: the stale source's sandbox-adapter authorization predicate must be
// preserved by both the atomic claim and crash-left in-progress recovery. A
// terminal sandbox child may authorize its session, but that cannot widen into
// a same-owner/session/host non-sandbox child.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_stale_reclaim_preserves_same_owner_non_sandbox_same_session_process() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let (reclaim, ledger, join) = build_preflighted_reclaim(&pool).await;
    let mut selected_child = spawn_real_long_lived_child();
    let mut non_sandbox_child = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-stale-adapter-scope-host";
    let owner_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire stale source owner lease");
    let owner = owner_lease.descriptor().clone();
    let session_id = format!("SR-{}", Uuid::now_v7());
    let selected_uuid = Uuid::now_v7();
    let non_sandbox_uuid = Uuid::now_v7();

    seed_self_owned_official_cli_start(
        &pool,
        Some(&session_id),
        selected_uuid,
        selected_child.pid,
        selected_child.os_creation_time_100ns,
        &selected_child.executable_sha256,
        &owner,
    )
    .await;
    seed_self_owned_non_sandbox_start(
        &pool,
        &session_id,
        non_sandbox_uuid,
        non_sandbox_child.pid,
        non_sandbox_child.os_creation_time_100ns,
        &non_sandbox_child.executable_sha256,
        &owner,
    )
    .await;
    seed_terminal_lane_for_process(&pool, &session_id, selected_uuid).await;

    let source = PostgresModelLaneStaleSessionSource::new(pool.clone(), owner.clone());
    let candidates = source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("sandbox-owned stale selection");
    assert_eq!(
        candidates.len(),
        1,
        "the terminal sandbox child must authorize its coordinator session"
    );
    let candidate = &candidates[0];
    assert_eq!(candidate.session_id, session_id);
    assert_eq!(candidate.authorized_process_uuids, vec![selected_uuid]);

    let report = reclaim
        .run_stale_owned_session(
            &session_id,
            owner.instance_id,
            host_scope,
            &candidate.authorized_process_uuids,
        )
        .await
        .expect("adapter-scoped stale reclaim");
    assert_eq!(
        report
            .processes_reclaimed
            .iter()
            .filter(|process| {
                process.process_uuid == selected_uuid
                    && matches!(process.kill_result, KillOutcome::Killed)
            })
            .count(),
        1,
        "the selected sandbox process must be killed exactly once: {:?}",
        report.processes_reclaimed
    );
    assert!(selected_child.guard.wait_exited(Duration::from_secs(10)));
    assert!(
        non_sandbox_child.guard.is_still_running(),
        "same-owner/session non-sandbox process must not be claimed or killed"
    );
    assert_eq!(open_row_count(&pool, non_sandbox_uuid).await, 1);
    assert!(stopped_at(&pool, non_sandbox_uuid).await.is_none());
    let untouched_state = lifecycle_row_state(&pool, non_sandbox_uuid).await;
    assert_eq!(
        untouched_state.0, None,
        "non-sandbox row must not be claimed"
    );
    assert!(untouched_state.1.get("reclaim_claim").is_none());

    // Simulate a legacy/unscoped crash-left kill operation on the same healthy
    // non-sandbox row. The stale-owner recovery sweep must preserve the source
    // authorization boundary and ignore it entirely.
    let store = PostgresProcessLedgerStore::new(pool.clone());
    let claimed = store
        .active_process_for_session(&session_id, non_sandbox_uuid)
        .await
        .expect("seed unscoped legacy claim")
        .expect("non-sandbox row remains open for recovery-boundary proof");
    store
        .mark_reclaim_kill_started(non_sandbox_uuid, &claimed.reclaim_claim)
        .await
        .expect("seed crash-left non-sandbox kill operation");
    let before_recovery = lifecycle_row_state(&pool, non_sandbox_uuid).await;
    let sweep = reclaim
        .reconcile_in_progress_for_stale_owner(
            &session_id,
            owner.instance_id,
            host_scope,
            &candidate.authorized_process_uuids,
        )
        .await
        .expect("adapter-scoped in-progress recovery");
    assert!(
        sweep.operations.is_empty(),
        "stale-owner recovery must not resume a non-sandbox operation: {sweep:?}"
    );
    assert!(sweep.reclaim_report.is_none());
    assert!(sweep.reclaim_error.is_none());
    assert_eq!(
        lifecycle_row_state(&pool, non_sandbox_uuid).await,
        before_recovery,
        "scoped recovery must leave the non-sandbox lifecycle byte-identical"
    );
    assert!(non_sandbox_child.guard.is_still_running());
    assert!(stopped_at(&pool, non_sandbox_uuid).await.is_none());

    drain(ledger, join).await;
    eprintln!(
        "MT019_NON_SKIP stale_adapter_scope selected_pid={} non_sandbox_pid={} selected_kills=1 non_sandbox_stops=0",
        selected_child.pid, non_sandbox_child.pid
    );
    drop(owner_lease);
    pool.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_stale_reclaim_rejects_same_scope_process_set_drift_before_claim() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let (reclaim, ledger, join) = build_preflighted_reclaim(&pool).await;
    let reclaim = Arc::new(reclaim);
    let mut selected_child = spawn_real_long_lived_child();
    let mut inserted_child = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-stale-set-drift-host";
    let owner_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire stale source owner lease");
    let owner = owner_lease.descriptor().clone();
    let session_id = format!("SR-{}", Uuid::now_v7());
    let selected_uuid = Uuid::now_v7();
    let inserted_uuid = Uuid::now_v7();
    seed_self_owned_official_cli_start(
        &pool,
        Some(&session_id),
        selected_uuid,
        selected_child.pid,
        selected_child.os_creation_time_100ns,
        &selected_child.executable_sha256,
        &owner,
    )
    .await;
    seed_terminal_lane_for_process(&pool, &session_id, selected_uuid).await;
    let source = PostgresModelLaneStaleSessionSource::new(pool.clone(), owner.clone());
    let candidates = source
        .stale_session_process_sets(Duration::from_secs(300))
        .await
        .expect("capture exact stale process set");
    assert_eq!(candidates.len(), 1);
    let candidate = candidates.into_iter().next().expect("one stale candidate");
    assert_eq!(candidate.authorized_process_uuids, vec![selected_uuid]);

    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let insert_barrier = Arc::clone(&barrier);
    let insert_pool = pool.clone();
    let insert_session = session_id.clone();
    let insert_owner = owner.clone();
    let inserted_pid = inserted_child.pid;
    let inserted_creation_time = inserted_child.os_creation_time_100ns;
    let inserted_sha256 = inserted_child.executable_sha256.clone();
    let insert_task = tokio::spawn(async move {
        seed_self_owned_official_cli_start(
            &insert_pool,
            Some(&insert_session),
            inserted_uuid,
            inserted_pid,
            inserted_creation_time,
            &inserted_sha256,
            &insert_owner,
        )
        .await;
        insert_barrier.wait().await;
    });
    let reclaim_barrier = Arc::clone(&barrier);
    let reclaim_task = tokio::spawn(async move {
        reclaim_barrier.wait().await;
        reclaim
            .run_stale_owned_session(
                &candidate.session_id,
                owner.instance_id,
                host_scope,
                &candidate.authorized_process_uuids,
            )
            .await
    });
    insert_task.await.expect("concurrent insertion task joins");
    let report = reclaim_task
        .await
        .expect("concurrent reclaim task joins")
        .expect("set-drift claim fails closed without a store error");
    assert!(
        report.processes_reclaimed.is_empty(),
        "atomic process-set drift guard must claim nothing: {report:?}"
    );
    for (process_uuid, child, label) in [
        (selected_uuid, &mut selected_child, "previously selected"),
        (inserted_uuid, &mut inserted_child, "concurrently inserted"),
    ] {
        assert!(child.guard.is_still_running(), "{label} child must survive");
        assert_eq!(open_row_count(&pool, process_uuid).await, 1);
        assert!(stopped_at(&pool, process_uuid).await.is_none());
        let state = lifecycle_row_state(&pool, process_uuid).await;
        assert_eq!(state.0, None, "{label} row must not be claimed");
        assert!(state.1.get("reclaim_claim").is_none());
    }

    drain(ledger, join).await;
    eprintln!(
        "MT019_NON_SKIP stale_set_drift selected_pid={} inserted_pid={} kills=0 stops=0",
        selected_child.pid, inserted_child.pid
    );
    drop(owner_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// P-4(b): the dead-owner probe requires TWO corroborating observations.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_dead_owner_probe_requires_two_corroborating_samples() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-two-sample-host";
    let this_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire this boot's OS liveness lease");
    let this_descriptor = this_lease.descriptor().clone();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let prior_instance_id = Uuid::now_v7();
    let prior_port = free_loopback_udp_port();
    seed_official_cli_start_with_dead_prior_owner(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        prior_instance_id,
        host_scope,
        prior_port,
    )
    .await;

    // Inside the gap: a production-length confirmation window can never elapse
    // between two back-to-back scans, however slow the host is.
    let inside_gap_source =
        PostgresModelLaneStaleSessionSource::new(pool.clone(), this_descriptor.clone())
            .with_dead_owner_confirmation_gap(Duration::from_secs(3600));
    assert!(
        inside_gap_source
            .restart_sessions()
            .await
            .expect("first dead-owner sample")
            .is_empty(),
        "one free-port observation must not authorise a restart reclaim"
    );
    assert!(
        inside_gap_source
            .restart_sessions()
            .await
            .expect("second dead-owner sample, too soon")
            .is_empty(),
        "a second observation INSIDE the confirmation gap must not authorise a restart reclaim"
    );

    // Across the gap: a fresh source (its own observation state) needs its own
    // first sample, then surfaces the orphan once the gap has genuinely elapsed.
    let source = PostgresModelLaneStaleSessionSource::new(pool.clone(), this_descriptor.clone())
        .with_dead_owner_confirmation_gap(Duration::from_millis(50));
    assert!(
        source
            .restart_sessions()
            .await
            .expect("first dead-owner sample on a fresh source")
            .is_empty(),
        "a fresh scanner must take its own first sample before it may reclaim"
    );
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        source
            .restart_sessions()
            .await
            .expect("corroborated dead-owner sample"),
        vec![session_run_id.clone()],
        "two observations at least one confirmation gap apart must surface the restart orphan"
    );

    // An owner that is protecting its lease is never surfaced, no matter how many
    // samples are taken: this is the property that stops a LIVE parallel instance
    // from being declared dead.
    let live_other = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire a second live instance lease");
    let live_session = format!("SR-{}", Uuid::now_v7());
    let live_process = Uuid::now_v7();
    seed_official_cli_start_with_dead_prior_owner(
        &pool,
        &live_session,
        live_process,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        live_other.descriptor().instance_id,
        host_scope,
        live_other.descriptor().loopback_port,
    )
    .await;
    let live_source = PostgresModelLaneStaleSessionSource::new(pool.clone(), this_descriptor)
        .with_dead_owner_confirmation_gap(Duration::from_millis(1));
    for _ in 0..3 {
        let surfaced = live_source
            .restart_sessions()
            .await
            .expect("live-owner restart scan");
        assert!(
            !surfaced.contains(&live_session),
            "a live instance still holding its loopback lease must never be surfaced as a dead owner"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // The kill-on-drop guard reaps the child this test spawned.
    assert!(spawned.guard.is_still_running());
    drop(live_other);
    drop(this_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// F2: the post-boot staleness task's periodic RESTART tick re-surfaces a
// restart orphan the boot pass skipped.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_post_boot_staleness_task_resurfaces_restart_orphan() {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();

    let host_scope = "wp1-mt019-periodic-restart-host";
    let this_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire this boot's OS liveness lease");
    let this_descriptor = this_lease.descriptor().clone();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_official_cli_start_with_dead_prior_owner(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        Uuid::now_v7(),
        host_scope,
        free_loopback_udp_port(),
    )
    .await;
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);

    // `spawn_managed_staleness_reclaim_task_after_boot` is the exact production
    // wiring: it does NOT run an immediate restart pass, because boot already ran
    // one inline. Before MT-019 its periodic tick only ran the STALE pass, so a
    // skipped or timed-out boot pass meant the orphan waited for the next reboot.
    let (reclaim, ledger, join) = build_reclaim(&pool);
    let reclaim = Arc::new(reclaim);
    let stale_source: Arc<dyn StaleSessionSource> = Arc::new(
        PostgresModelLaneStaleSessionSource::new(pool.clone(), this_descriptor)
            .with_dead_owner_confirmation_gap(Duration::from_millis(100)),
    );
    let task = spawn_managed_staleness_reclaim_task_after_boot(
        Arc::clone(&reclaim),
        stale_source,
        StalenessReclaimConfig {
            ttl: Duration::from_secs(300),
            scan_interval: Duration::from_millis(250),
        },
    );

    assert!(
        spawned.guard.wait_exited(Duration::from_secs(30)),
        "the periodic restart tick must reclaim a restart orphan without a reboot"
    );
    assert!(task.shutdown_and_join(Duration::from_secs(10)).await);

    drain(ledger, join).await;
    assert!(
        stopped_at(&pool, process_uuid).await.is_some(),
        "the periodic restart tick must write a durable STOP row"
    );
    drop(this_lease);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// F3 + F5 + F7 + P-4(a): `production_with_lease` boot is FAIL-OPEN on an
// un-reapable orphan, counts it as kill-failed (not reclaimed), surfaces it, and
// retains the liveness lease while this instance still owns open rows.
//
// COVERAGE LIMIT (stated, not implied): this file is `#![cfg(windows)]`, so the
// non-Windows "reclaim adapter unavailable" path named in F3 cannot be executed
// on this host. The Windows analogue used here is `ProductionSandboxKill`
// composed with a registry that does NOT contain HandshakeNative, so
// `owning_adapter` errors and yields the identical `KillOutcome::Failed` shape
// with the row left open — the same shape the non-Windows adapter always returns.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_production_with_lease_boot_is_fail_open_on_unreapable_orphan_and_reports_kill_failed(
) {
    let (_kp, pool) = managed_full_chain_pool().await;
    let mut spawned = spawn_real_long_lived_child();
    let mut own_child = spawn_real_long_lived_child();

    // `production_with_lease` composes its own stale-session source, so the only
    // seam to shorten the P-4(b) corroboration window is the test-utils override.
    // The two-sample gate itself is proven separately by
    // `mt019_dead_owner_probe_requires_two_corroborating_samples`.
    set_dead_owner_confirmation_gap_override_for_test(Some(Duration::ZERO));

    let host_scope = "wp1-mt019-fail-open-host";
    let boot_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire boot lease");
    let boot_descriptor = boot_lease.descriptor().clone();

    let session_run_id = format!("SR-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    seed_official_cli_start_with_dead_prior_owner(
        &pool,
        &session_run_id,
        process_uuid,
        spawned.pid,
        spawned.os_creation_time_100ns,
        &spawned.executable_sha256,
        Uuid::now_v7(),
        host_scope,
        free_loopback_udp_port(),
    )
    .await;

    // A live child owned by THIS boot instance. The unreapable orphan above
    // belongs to a prior instance, and the lease only speaks for its own
    // instance's rows, so P-4(a) needs a self-owned open row to be meaningful.
    let own_process_uuid = Uuid::now_v7();
    seed_self_owned_official_cli_start(
        &pool,
        None,
        own_process_uuid,
        own_child.pid,
        own_child.os_creation_time_100ns,
        &own_child.executable_sha256,
        &boot_descriptor,
    )
    .await;

    // A registry with NO reclaim-capable adapter registered.
    let empty_registry = Arc::new(SandboxAdapterRegistry::new(AdapterId::new(
        HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
    )));
    let runtime = ProcessReclaimRuntime::production_with_lease(
        pool.clone(),
        Arc::new(PostgresProcessLedgerStore::new(pool.clone())),
        None,
        empty_registry,
        boot_lease,
        Duration::from_secs(30),
    )
    .await
    .expect("boot must COMPLETE (fail-open) when an orphan cannot be reaped");

    let report = runtime.boot_reconcile_report();
    assert_eq!(
        report.sessions_reconciled, 1,
        "the orphan session must still be surfaced: {report:?}"
    );
    assert_eq!(
        report.processes_reclaimed, 0,
        "a Failed kill must NOT be counted as reclaimed: {report:?}"
    );
    assert_eq!(
        report.processes_kill_failed, 1,
        "a Failed kill must be counted as kill-failed: {report:?}"
    );

    // Truthful fail-open: the child is untouched, the row is still open, and no
    // STOP was fabricated.
    assert!(
        spawned.guard.is_still_running(),
        "an unreapable orphan's process must not be reported as reclaimed nor be killed"
    );
    assert_eq!(open_row_count(&pool, process_uuid).await, 1);
    assert!(stopped_at(&pool, process_uuid).await.is_none());
    // The live self-owned child was never a restart candidate.
    assert!(own_child.guard.is_still_running());
    assert_eq!(open_row_count(&pool, own_process_uuid).await, 1);

    // P-4(a): the liveness lease must NOT be released while this instance still
    // owns open rows, because releasing it advertises a live process as dead and
    // authorises another instance to kill its healthy children.
    let drained = runtime.shutdown_and_drain(Duration::from_secs(10)).await;
    assert!(
        !drained.lease_released,
        "the liveness lease must be retained while this instance owns open lifecycle rows"
    );
    assert!(
        drained
            .lease_retained_reason
            .as_deref()
            .unwrap_or_default()
            .contains("still open"),
        "the retention reason must name the open rows: {:?}",
        drained.lease_retained_reason
    );

    // ...and it IS released once this instance provably owns no open row.
    sqlx::query(
        "UPDATE kernel_process_lifecycle SET stopped_at = NOW(), exit_code = 0 WHERE process_uuid = $1",
    )
    .bind(own_process_uuid)
    .execute(&pool)
    .await
    .expect("close this instance's own lifecycle row");
    let drained_again = runtime.shutdown_and_drain(Duration::from_secs(10)).await;
    assert!(
        drained_again.lease_released,
        "the liveness lease must be released once this instance owns zero open lifecycle rows: {:?}",
        drained_again.lease_retained_reason
    );

    set_dead_owner_confirmation_gap_override_for_test(None);
    pool.close().await;
}

// ---------------------------------------------------------------------------
// F7: `production_with_lease`'s startup TIMEOUT wrapper is real and fails closed.
//
// Documented leak: on the fail-closed path `production_with_lease` deliberately
// `mem::forget`s the runtime lease when the writer's terminal state was not
// observed, so this test may leak one UDP socket for the remainder of the test
// process. It uses its own host scope so no other proof can be affected.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_production_with_lease_boot_timeout_fails_closed_and_retains_lease() {
    let (_kp, pool) = managed_full_chain_pool().await;

    let host_scope = "wp1-mt019-boot-timeout-host";
    let boot_lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), host_scope)
        .expect("acquire boot lease");

    let boot = ProcessReclaimRuntime::production_with_lease(
        pool.clone(),
        Arc::new(PostgresProcessLedgerStore::new(pool.clone())),
        None,
        production_process_sandbox_registry(),
        boot_lease,
        Duration::from_nanos(1),
    )
    .await;
    let error = match boot {
        Ok(_runtime) => {
            panic!("a boot reconcile that exceeds the startup timeout must fail closed")
        }
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("boot reconciliation exceeded"),
        "the fail-closed boot error must name the timeout: {error}"
    );

    pool.close().await;
}
