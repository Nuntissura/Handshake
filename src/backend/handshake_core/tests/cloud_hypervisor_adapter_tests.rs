//! Real (non-mock) integration tests for the Tier-3 Cloud Hypervisor microVM
//! sandbox adapter.
//!
//! These tests actually boot a Cloud Hypervisor microVM inside WSL2 and run a
//! command inside it. On a host without WSL2 + KVM + the staged VM artifacts,
//! `CloudHypervisorAdapter::try_new` returns `AdapterUnavailable`; in that case
//! the test prints a clear skip message and returns so non-WSL CI does not
//! fail. On a host where the adapter IS available it MUST exercise a real boot.

use std::collections::BTreeMap;

use handshake_core::sandbox::{
    AdapterId, Command, CloudHypervisorAdapter, CloudHypervisorConfig, ImageRef, IsolationTier,
    NetPolicy, ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    Signal, TrustClass, CLOUD_HYPERVISOR_ADAPTER_ID, SANDBOX_IDLE_TIMEOUT_METADATA_KEY,
    SANDBOX_MODE_METADATA_KEY, SANDBOX_MODE_PERSISTENT,
};

fn skip_message(error: &SandboxAdapterError) -> String {
    format!(
        "SKIP cloud_hypervisor adapter test: runtime unavailable on this host ({error}). \
         This is expected on non-WSL2 / non-KVM hosts."
    )
}

/// Turn the runtime-unavailable skip into a hard failure when the operator
/// asserts the host SHOULD have the runtime by setting `HANDSHAKE_CH_REQUIRE`.
/// Without it, a regression that makes `try_new` wrongly report unavailable
/// would silently pass every test (return == PASS). With it set, such a
/// regression panics. On genuinely non-WSL hosts (env unset) it just prints the
/// skip message and lets the caller `return`.
fn require_or_skip(error: &SandboxAdapterError) {
    if std::env::var("HANDSHAKE_CH_REQUIRE").is_ok() {
        panic!(
            "HANDSHAKE_CH_REQUIRE is set but the Cloud Hypervisor adapter is unavailable: {error}"
        );
    }
    eprintln!("{}", skip_message(error));
}

fn sample_spec() -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("ch-test-spec"),
        image_or_root: ImageRef::new("initramfs"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        required_capabilities: Default::default(),
        trust_class: TrustClass::UntrustedAgent,
        metadata: BTreeMap::new(),
    }
}

fn persistent_spec() -> ProcessSpec {
    let mut spec = sample_spec();
    spec.metadata.insert(
        SANDBOX_MODE_METADATA_KEY.to_string(),
        SANDBOX_MODE_PERSISTENT.to_string(),
    );
    spec
}

fn last_tick(serial: &str) -> Option<u64> {
    serial
        .lines()
        .filter_map(|line| line.trim().strip_prefix("TICK-"))
        .filter_map(|n| n.trim().parse::<u64>().ok())
        .max()
}

/// Poll the persistent VM's serial console until it emits a TICK strictly
/// greater than `above` (up to ~40s), returning the highest TICK-N seen. The
/// `above` bound matters for the restored VM: it appends to the SAME serial log
/// that already holds the pre-snapshot ticks, so we must wait for a genuinely
/// NEW tick (proof the VM resumed and kept counting) rather than re-reading a
/// stale one. On timeout, dump the raw serial so a failure is diagnosable.
async fn wait_for_tick(
    adapter: &CloudHypervisorAdapter,
    handle: &handshake_core::sandbox::ProcessHandle,
    above: u64,
    label: &str,
) -> u64 {
    let mut last = String::new();
    for _ in 0..40 {
        last = adapter.read_handle_serial(handle).await.unwrap_or_default();
        if let Some(n) = last_tick(&last) {
            if n > above {
                return n;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    panic!(
        "{label}: persistent VM emitted no TICK greater than {above} within ~40s. Raw serial follows:\n{last}"
    );
}

/// Real snapshot/restore: boot a persistent idle microVM, snapshot it, restore
/// into a fresh VM, and prove the in-RAM running state (a serial TICK counter)
/// CONTINUED across the restore — i.e. the VM was resumed from captured memory,
/// not rebooted. Backs the validate-then-promote flow (spec v02.187 §3.5.7 #7).
#[tokio::test]
async fn cloud_hypervisor_snapshot_restore_preserves_running_state() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };
    assert!(
        adapter.capabilities().supports_snapshot,
        "Tier-3 microVM adapter must declare supports_snapshot"
    );

    let handle = adapter
        .spawn(persistent_spec())
        .await
        .expect("spawn persistent microVM");
    // The guest boots a few seconds after spawn returns (spawn only waits for
    // the API socket), so poll for the first TICK rather than guessing a sleep.
    let n_before = wait_for_tick(&adapter, &handle, 0, "before snapshot").await;

    let snapshot = adapter
        .snapshot(&handle)
        .await
        .expect("snapshot the persistent microVM");
    let restored = adapter
        .restore(&snapshot)
        .await
        .expect("restore a fresh microVM from the snapshot");
    // Require a TICK strictly greater than n_before: proof the restored VM
    // resumed the in-RAM counter and kept incrementing (not a reboot, not a
    // stale read of the pre-snapshot ticks in the shared serial log).
    let n_after = wait_for_tick(&adapter, &restored, n_before, "after restore").await;

    eprintln!("--- SNAPSHOT/RESTORE: TICK before={n_before}, after={n_after} ---");
    assert!(
        n_after > n_before,
        "restored microVM must CONTINUE the in-RAM counter (state preserved, resumed not rebooted): {n_before} -> {n_after}"
    );

    let _ = adapter.kill(&handle, Signal::Kill).await;
    let _ = adapter.kill(&restored, Signal::Kill).await;
}

/// A snapshot must be RE-RESTORABLE: restoring it twice yields two independent
/// VMs, and tearing one down must not destroy the snapshot or the other VM's
/// console (regression guard for the restore-owns-snapshot-dir clobber bug).
#[tokio::test]
async fn cloud_hypervisor_snapshot_restores_twice_independently() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let handle = adapter
        .spawn(persistent_spec())
        .await
        .expect("spawn persistent microVM");
    let n0 = wait_for_tick(&adapter, &handle, 0, "before snapshot").await;
    let snapshot = adapter.snapshot(&handle).await.expect("snapshot");

    // First restore from the snapshot.
    let r1 = adapter
        .restore(&snapshot)
        .await
        .expect("first restore from snapshot");
    let r1_tick = wait_for_tick(&adapter, &r1, n0, "first restore").await;

    // Second restore FROM THE SAME snapshot must still succeed (the snapshot was
    // not consumed/destroyed by the first restore).
    let r2 = adapter
        .restore(&snapshot)
        .await
        .expect("second restore from the same snapshot (must not be consumed)");
    let r2_tick = wait_for_tick(&adapter, &r2, n0, "second restore").await;

    eprintln!("--- DOUBLE RESTORE: n0={n0}, r1={r1_tick}, r2={r2_tick} ---");

    // Tearing down the first restored VM must not affect the second: r2 keeps
    // ticking on its OWN console after r1 is killed.
    adapter.kill(&r1, Signal::Kill).await.expect("kill r1");
    let r2_after = wait_for_tick(&adapter, &r2, r2_tick, "r2 after r1 kill").await;
    assert!(
        r2_after > r2_tick,
        "second restored VM must survive the first's teardown: {r2_tick} -> {r2_after}"
    );

    let _ = adapter.kill(&r2, Signal::Kill).await;
    let _ = adapter.kill(&handle, Signal::Kill).await;
}

/// A restored VM must keep its OWN serial console alive after the ORIGINAL VM is
/// torn down (regression guard for the shared-serial-log ordering hazard, where
/// killing the original deleted the serial the restored VM was still writing).
#[tokio::test]
async fn cloud_hypervisor_restored_survives_original_teardown() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let handle = adapter
        .spawn(persistent_spec())
        .await
        .expect("spawn persistent microVM");
    let n0 = wait_for_tick(&adapter, &handle, 0, "before snapshot").await;
    let snapshot = adapter.snapshot(&handle).await.expect("snapshot");
    let restored = adapter.restore(&snapshot).await.expect("restore");
    let r_tick = wait_for_tick(&adapter, &restored, n0, "restored").await;

    // Tear down the ORIGINAL; the restored VM's console must survive and keep
    // advancing (it writes to its own restore-owned serial log).
    adapter.kill(&handle, Signal::Kill).await.expect("kill original");
    let r_after = wait_for_tick(&adapter, &restored, r_tick, "restored after original kill").await;
    assert!(
        r_after > r_tick,
        "restored VM console must survive original teardown: {r_tick} -> {r_after}"
    );

    let _ = adapter.kill(&restored, Signal::Kill).await;
}

/// §3.5.7 #6: a persistent VM with an idle timeout must self-reap when left idle
/// (no owner kill, no activity) — orphaned-sandbox auto-reaping.
#[tokio::test]
async fn cloud_hypervisor_persistent_vm_idle_auto_kills() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let mut spec = persistent_spec();
    spec.metadata.insert(
        SANDBOX_IDLE_TIMEOUT_METADATA_KEY.to_string(),
        "4000".to_string(),
    );
    let handle = adapter
        .spawn(spec)
        .await
        .expect("spawn persistent microVM with idle timeout");
    assert!(
        adapter.live_persistent_handle_ids().contains(&handle.id),
        "freshly spawned persistent handle must be live"
    );

    // No activity at all: the reaper must auto-kill within idle_timeout + slack.
    let mut reaped = false;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if !adapter.live_persistent_handle_ids().contains(&handle.id) {
            reaped = true;
            break;
        }
    }
    assert!(reaped, "idle persistent VM must self-reap via the idle reaper");
    match adapter.status(&handle).await {
        Ok(ProcessStatus::Killed { .. }) => {}
        other => panic!("expected Killed after idle reap, got {other:?}"),
    }
}

/// §3.5.7 #8/#9: the adapter must enumerate its live persistent handles and
/// reclaim on-disk orphans from a crashed/restarted prior run — without ever
/// removing a VM it currently owns.
#[tokio::test]
async fn cloud_hypervisor_enumerate_and_reclaim_orphans() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let handle = adapter
        .spawn(persistent_spec())
        .await
        .expect("spawn persistent microVM");
    assert!(
        adapter.live_persistent_handle_ids().contains(&handle.id),
        "spawned persistent handle must be enumerated as live"
    );

    // Fabricate an on-disk orphan the adapter does NOT own (simulating a VM left
    // by a crashed prior run).
    let wd = adapter.config().work_dir().to_string();
    let orphan = format!("{wd}/persistent-orphan-test-{}", uuid::Uuid::now_v7().simple());
    let mk = std::process::Command::new("wsl.exe")
        .args([
            "-d",
            "Ubuntu",
            "-e",
            "sh",
            "-c",
            &format!("mkdir -p '{orphan}' && : > '{orphan}/ch.sock'"),
        ])
        .status()
        .expect("create orphan dir");
    assert!(mk.success(), "fabricating orphan dir must succeed");

    let found = adapter.discover_orphan_vm_dirs().await;
    assert!(
        found.iter().any(|d| d == &orphan),
        "fabricated orphan must be discovered; got {found:?}"
    );

    let reclaimed = adapter.reclaim_orphan_vm_dirs().await;
    assert!(reclaimed >= 1, "must reclaim at least the fabricated orphan");
    assert!(
        !adapter.discover_orphan_vm_dirs().await.iter().any(|d| d == &orphan),
        "orphan must be gone after reclaim"
    );
    // Reclaim must NOT have touched the live VM.
    assert!(
        adapter.live_persistent_handle_ids().contains(&handle.id),
        "reclaim must not remove the live VM this adapter owns"
    );

    adapter.kill(&handle, Signal::Kill).await.expect("kill live handle");
    assert!(
        !adapter.live_persistent_handle_ids().contains(&handle.id),
        "killed handle must no longer be enumerated"
    );
}

/// Real microVM boot: spawn a handle, exec a command inside a freshly booted
/// Cloud Hypervisor microVM, and assert on the captured guest stdout + exit
/// code. Also asserts the Tier-3 capability shape.
#[tokio::test]
async fn cloud_hypervisor_boots_real_microvm_and_captures_stdout() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    // Tier-3 capability shape is asserted only when the adapter is available
    // (it is only constructed when the runtime is available).
    let caps = adapter.capabilities();
    assert_eq!(caps.adapter_id, AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID));
    assert_eq!(caps.isolation_tier, IsolationTier::Tier3Microvm);
    assert!(
        caps.requires_nested_virt,
        "Tier-3 microVM must declare requires_nested_virt"
    );
    assert!(caps.runtime_available);

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn cloud_hypervisor handle");

    let cmd = Command {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo handshake-microvm-ok; uname -s".to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: None,
    };

    let result = adapter
        .exec(&handle, cmd)
        .await
        .expect("exec inside real microVM");

    let stdout = String::from_utf8_lossy(&result.stdout);
    eprintln!("--- REAL MICROVM STDOUT BEGIN ---");
    eprintln!("{stdout}");
    eprintln!("--- REAL MICROVM STDOUT END (exit_code={}, {} ms) ---", result.exit_code, result.duration_ms);

    assert!(
        stdout.contains("handshake-microvm-ok"),
        "captured stdout must contain the echoed marker; got: {stdout:?}"
    );
    assert!(
        stdout.contains("Linux"),
        "captured stdout must contain `Linux` from `uname -s`; got: {stdout:?}"
    );
    assert_eq!(result.exit_code, 0, "successful command must report exit 0");

    // The ephemeral model reports Exited after a completed exec.
    match adapter.status(&handle).await.expect("status after exec") {
        ProcessStatus::Exited { code } => assert_eq!(code, 0),
        other => panic!("expected Exited after completed exec, got {other:?}"),
    }
    assert_eq!(
        adapter.exit_code(&handle).await.expect("exit_code after exec"),
        Some(0)
    );
}

/// Negative path: a command that exits non-zero must surface the real guest
/// exit code (not 0, not a host-side error).
#[tokio::test]
async fn cloud_hypervisor_propagates_nonzero_exit_code() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn cloud_hypervisor handle");

    let cmd = Command {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo before-failure; exit 7".to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: None,
    };

    let result = adapter
        .exec(&handle, cmd)
        .await
        .expect("exec non-zero command inside real microVM");

    let stdout = String::from_utf8_lossy(&result.stdout);
    eprintln!("--- REAL MICROVM (nonzero) STDOUT ---\n{stdout}\n--- exit_code={} ---", result.exit_code);

    assert!(stdout.contains("before-failure"));
    assert_eq!(
        result.exit_code, 7,
        "non-zero guest exit code must be propagated verbatim"
    );
}

/// `fs_bind` must still fail closed for an unsafe guest mount point (one that
/// collides with the kernel/synthetic filesystems the init script owns), even
/// though real binds are now supported.
#[tokio::test]
async fn cloud_hypervisor_fs_bind_rejects_reserved_guest_path() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn cloud_hypervisor handle");

    let err = adapter
        .fs_bind(
            &handle,
            std::path::PathBuf::from("D:/host"),
            std::path::PathBuf::from("/proc"),
            handshake_core::sandbox::BindMode::ReadOnly,
        )
        .await
        .expect_err("fs_bind must reject reserved guest paths");
    assert!(
        matches!(err, SandboxAdapterError::BindGuestPathInvalid { .. }),
        "fs_bind must return a typed invalid-guest-path error, got {err:?}"
    );
}

/// REAL read-write filesystem bind. Bakes a host directory into the per-exec
/// microVM at `/work`, runs a command that reads the baked-in file AND writes a
/// new file, then asserts the new file is written back to the *host* directory
/// (the genuine write-back path — no mock anywhere).
#[tokio::test]
async fn cloud_hypervisor_fs_bind_read_write_writes_back_to_host() {
    use std::fs;

    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            require_or_skip(&error);
            return;
        }
    };

    // Unique Windows host temp dir with a baked-in input file.
    let host_dir = std::env::temp_dir().join(format!(
        "hsk-ch-fsbind-{}",
        uuid::Uuid::now_v7().simple()
    ));
    fs::create_dir_all(&host_dir).expect("create host bind dir");
    fs::write(host_dir.join("input.txt"), "hello-from-host").expect("write input.txt");

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn cloud_hypervisor handle");

    adapter
        .fs_bind(
            &handle,
            host_dir.clone(),
            std::path::PathBuf::from("/work"),
            handshake_core::sandbox::BindMode::ReadWrite,
        )
        .await
        .expect("fs_bind ReadWrite /work");

    let cmd = Command {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            "cat /work/input.txt; echo; echo MODIFIED-IN-VM > /work/output.txt; echo wrote"
                .to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: None,
    };

    let result = adapter
        .exec(&handle, cmd)
        .await
        .expect("exec inside real microVM with RW bind");

    let stdout = String::from_utf8_lossy(&result.stdout);
    eprintln!("--- REAL MICROVM (fs_bind RW) STDOUT BEGIN ---");
    eprintln!("{stdout}");
    eprintln!(
        "--- REAL MICROVM (fs_bind RW) STDOUT END (exit_code={}, {} ms) ---",
        result.exit_code, result.duration_ms
    );

    assert!(
        stdout.contains("hello-from-host"),
        "guest must read the baked-in host file via /work; got: {stdout:?}"
    );
    assert_eq!(result.exit_code, 0, "command must exit 0; got {stdout:?}");

    // The genuine write-back: the host temp dir must now contain output.txt
    // with the content the guest wrote.
    let written = fs::read_to_string(host_dir.join("output.txt"))
        .expect("output.txt must be written back to the host bind dir");
    let written_trimmed = written.trim_end_matches(['\r', '\n']);
    eprintln!("--- HOST WRITE-BACK output.txt = {written_trimmed:?} ---");
    assert_eq!(
        written_trimmed, "MODIFIED-IN-VM",
        "host output.txt must contain the value written inside the microVM"
    );

    // Clean up the host temp dir.
    let _ = fs::remove_dir_all(&host_dir);
}
