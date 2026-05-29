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
    TrustClass, CLOUD_HYPERVISOR_ADAPTER_ID,
};

fn skip_message(error: &SandboxAdapterError) -> String {
    format!(
        "SKIP cloud_hypervisor adapter test: runtime unavailable on this host ({error}). \
         This is expected on non-WSL2 / non-KVM hosts."
    )
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

/// Real microVM boot: spawn a handle, exec a command inside a freshly booted
/// Cloud Hypervisor microVM, and assert on the captured guest stdout + exit
/// code. Also asserts the Tier-3 capability shape.
#[tokio::test]
async fn cloud_hypervisor_boots_real_microvm_and_captures_stdout() {
    let adapter = match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("{}", skip_message(&error));
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
            eprintln!("{}", skip_message(&error));
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
            eprintln!("{}", skip_message(&error));
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
            eprintln!("{}", skip_message(&error));
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
