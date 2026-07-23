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
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, PostgresProcessLedgerStore,
    ProcessLedgerStore, Reclaim, ReclaimTrigger,
};
use handshake_core::sandbox::{process_creation_time_100ns, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

fn spawn_real_long_lived_child() -> Option<SpawnedChild> {
    let exe = powershell_path()?;
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
    Some(SpawnedChild {
        guard: ChildGuard(Some(child)),
        pid,
        executable_sha256: sha256_file(&exe),
        os_creation_time_100ns,
    })
}

/// Full migration chain on an isolated schema of the real managed cluster.
async fn managed_full_chain_pool() -> Option<(knowledge_pg_support::KnowledgePg, PgPool)> {
    let kp = knowledge_pg_support::knowledge_pg().await?;
    let pool = sqlx::PgPool::connect(&kp.schema_url)
        .await
        .expect("connect a reclaim pool pinned to the isolated migrated schema");
    Some((kp, pool))
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
    let store: Arc<dyn ProcessLedgerStore> =
        Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    let (ledger, join) = LedgerBatcher::spawn(store, Arc::new(NoopOverflowSink), batcher_config());
    let reclaim_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    let killer = Arc::new(
        handshake_core::process_ledger::ProductionSandboxKill::new(
            pool.clone(),
            tokio::runtime::Handle::current(),
        ),
    );
    let reclaim = Reclaim::new(reclaim_store, killer, Arc::new(ledger.clone()));
    (reclaim, ledger, join)
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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED real_process_reclaim_kills_child_and_writes_durable_stop: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED real_process_reclaim_kills_child_and_writes_durable_stop: PowerShell not found");
        return;
    };

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
    assert!(spawned.guard.is_still_running(), "child must be live pre-reclaim");

    let (reclaim, ledger, join) = build_reclaim(&pool);
    let report = reclaim
        .run(&session_run_id, ReclaimTrigger::Restart)
        .await
        .expect("production reclaim run");

    assert_eq!(report.processes_reclaimed.len(), 1, "exactly one process reclaimed");
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
// (b) crash/restart reconciliation of a prior-boot orphan
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn boot_reconcile_kills_prior_boot_orphan_and_writes_durable_stop() {
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED boot_reconcile_kills_prior_boot_orphan_and_writes_durable_stop: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED boot_reconcile_kills_prior_boot_orphan_and_writes_durable_stop: PowerShell not found");
        return;
    };

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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED concurrent_reclaimers_produce_exactly_one_kill_and_one_stop: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED concurrent_reclaimers_produce_exactly_one_kill_and_one_stop: PowerShell not found");
        return;
    };

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
    assert_eq!(
        killed_with_stop, 1,
        "exactly one reclaimer may kill + STOP the shared process (no double-STOP)"
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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED pid_reuse_guard_refuses_to_kill_a_mismatched_generation: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED pid_reuse_guard_refuses_to_kill_a_mismatched_generation: PowerShell not found");
        return;
    };

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
