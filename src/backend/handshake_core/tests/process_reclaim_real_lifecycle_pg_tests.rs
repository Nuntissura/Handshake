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
    PostgresProcessLedgerStore, ProcessLedgerStore, ProcessReclaimRuntime, Reclaim, ReclaimTrigger,
    StaleSessionSource, StalenessReclaimConfig, EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
    EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
};
use handshake_core::sandbox::{
    process_creation_time_100ns, AdapterId, SandboxAdapterRegistry,
    HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::net::{Ipv4Addr, UdpSocket};
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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED boot_reconcile_via_restart_sessions_reclaims_official_cli_orphan: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED boot_reconcile_via_restart_sessions_reclaims_official_cli_orphan: PowerShell not found");
        return;
    };

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
    let (reclaim, ledger, join) = build_reclaim(&pool);
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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_running_app_reclaims_same_instance_official_cli_orphan_without_reboot: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_running_app_reclaims_same_instance_official_cli_orphan_without_reboot: PowerShell not found");
        return;
    };

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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_running_app_reap_never_kills_a_live_same_instance_child: PostgreSQL unavailable");
        return;
    };
    let Some(mut sessionless) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_running_app_reap_never_kills_a_live_same_instance_child: PowerShell not found");
        return;
    };
    let Some(mut with_session) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_running_app_reap_never_kills_a_live_same_instance_child: PowerShell not found");
        return;
    };

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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_single_row_claim_leaves_sibling_reclaim_metadata_untouched: PostgreSQL unavailable");
        return;
    };
    let Some(mut target) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_single_row_claim_leaves_sibling_reclaim_metadata_untouched: PowerShell not found");
        return;
    };
    let Some(mut sibling) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_single_row_claim_leaves_sibling_reclaim_metadata_untouched: PowerShell not found");
        return;
    };

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
// P-4(b): the dead-owner probe requires TWO corroborating observations.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt019_dead_owner_probe_requires_two_corroborating_samples() {
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_dead_owner_probe_requires_two_corroborating_samples: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_dead_owner_probe_requires_two_corroborating_samples: PowerShell not found");
        return;
    };

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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_post_boot_staleness_task_resurfaces_restart_orphan: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_post_boot_staleness_task_resurfaces_restart_orphan: PowerShell not found");
        return;
    };

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
async fn mt019_production_with_lease_boot_is_fail_open_on_unreapable_orphan_and_reports_kill_failed()
{
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_production_with_lease_boot_is_fail_open_on_unreapable_orphan_and_reports_kill_failed: PostgreSQL unavailable");
        return;
    };
    let Some(mut spawned) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_production_with_lease_boot_is_fail_open_on_unreapable_orphan_and_reports_kill_failed: PowerShell not found");
        return;
    };
    let Some(mut own_child) = spawn_real_long_lived_child() else {
        eprintln!("SKIPPED mt019_production_with_lease_boot_is_fail_open_on_unreapable_orphan_and_reports_kill_failed: PowerShell not found");
        return;
    };

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
    let Some((_kp, pool)) = managed_full_chain_pool().await else {
        eprintln!("SKIPPED mt019_production_with_lease_boot_timeout_fails_closed_and_retains_lease: PostgreSQL unavailable");
        return;
    };

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
        Ok(_runtime) => panic!("a boot reconcile that exceeds the startup timeout must fail closed"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("boot reconciliation exceeded"),
        "the fail-closed boot error must name the timeout: {error}"
    );

    pool.close().await;
}
