use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use handshake_core::sandbox::{
    AdapterId, BindMode, GpuPassthrough, ImageRef, IsolationStrength, NetAllowlistEntry, NetPolicy,
    NetProtocol, ProcessHandle, ProcessSpec, ProcessStatus, RequiredCapability, ResourceLimits,
    SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass, WindowsNativeJailAdapter,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID, WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

#[test]
fn windows_native_jail_capabilities_are_centralized_and_do_not_claim_portability() {
    let caps = WindowsNativeJailAdapter::target_capability_contract();

    assert_eq!(
        caps.adapter_id,
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert!(!caps.runtime_available);
    assert_eq!(
        caps.filesystem_isolation_strength,
        IsolationStrength::VeryStrong
    );
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Strong);
    assert_eq!(caps.gpu_passthrough, GpuPassthrough::VendorAgnostic);
    assert_eq!(caps.stdio_throughput_class, ThroughputClass::Medium);
    assert!(caps.win32_native_fidelity);
    assert!(!caps.cross_machine_portable);
}

#[test]
fn windows_native_jail_unavailable_adapter_does_not_present_runtime_capabilities() {
    let adapter = WindowsNativeJailAdapter::unavailable_for_current_host();
    let caps = adapter.capabilities();

    assert_eq!(
        caps.adapter_id,
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert!(!caps.runtime_available);
    assert_eq!(caps.filesystem_isolation_strength, IsolationStrength::Weak);
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Weak);
    assert_eq!(caps.gpu_passthrough, GpuPassthrough::None);
    assert_eq!(caps.stdio_throughput_class, ThroughputClass::Low);
    assert!(!caps.win32_native_fidelity);
    assert!(!caps.cross_machine_portable);
}

#[test]
fn windows_native_jail_backend_uses_approved_rappct_dependency_without_hand_rolled_win32() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read Cargo.toml");
    let cargo_lock = fs::read_to_string(manifest_dir.join("Cargo.lock")).expect("read Cargo.lock");
    let bootstrap =
        fs::read_to_string(manifest_dir.join("src/sandbox/bootstrap.rs")).expect("read bootstrap");
    let windows_native_jail_root = manifest_dir.join("src/sandbox/windows_native_jail");
    let windows_native_jail_sources = read_module_sources(&windows_native_jail_root);
    let unaudited_windows_native_jail_sources = read_module_sources_excluding(
        &windows_native_jail_root,
        &["job_object_wrap.rs", "restricted_appcontainer.rs"],
    );

    // Structural assertions below are platform-independent. The runtime
    // backend-availability boolean is asserted by a separate Windows+feature
    // gated test (`windows_native_jail_backend_approved_under_feature_gate`)
    // so that cross-platform CI runs of this test pass cleanly per MT-046
    // contract: "non-Windows callers receive typed AdapterUnavailable rather
    // than a build failure".
    assert!(
        cargo_toml.contains("rappct"),
        "MT-046 must depend on the MT-045 approved crate"
    );
    assert!(
        cargo_toml.contains("windows-sys"),
        "MT-046 must keep the audited Win32 composition layer explicit"
    );
    assert!(
        cargo_lock.contains("name = \"rappct\""),
        "Cargo.lock must pin the MT-045 approved crate"
    );
    assert!(
        !cargo_toml.contains("codex-windows-sandbox")
            && !cargo_toml.contains("codex_windows_sandbox"),
        "OpenAI codex-windows-sandbox is prior art only; it is not a published drop-in dependency"
    );

    assert!(
        bootstrap.contains("WindowsNativeJailAdapter::try_new"),
        "bootstrap must register WindowsNativeJailAdapter after MT-045 approval"
    );

    for required in [
        "CreateRestrictedToken",
        "CreateProcessAsUserW",
        "PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES",
        "PROC_THREAD_ATTRIBUTE_JOB_LIST",
        "CreateJobObjectW",
        "TerminateJobObject",
    ] {
        assert!(
            windows_native_jail_sources.contains(required),
            "WindowsNativeJailAdapter audited wrapper must include `{required}`"
        );
    }

    for banned in [
        "use windows::",
        "use winapi::",
        "windows::Win32",
        "winapi::um",
        "extern \"system\"",
        "LoadLibrary",
        "GetProcAddress",
        "DuplicateTokenEx",
        "SetTokenInformation",
        "CreateProcessWithTokenW",
        "CreateAppContainerProfile",
        "DeriveAppContainerSidFromAppContainerName",
        "CreatePrivateNamespace",
        "CreateBoundaryDescriptor",
        "Fwpm",
        "unsafe ",
        "unsafe{",
    ] {
        assert!(
            !unaudited_windows_native_jail_sources.contains(banned),
            "Win32 isolation surface `{banned}` must stay inside audited wrapper modules"
        );
    }
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[test]
fn windows_native_jail_backend_approved_under_feature_gate() {
    // Gated companion to the structural confinement test above: under a
    // Windows host built with --features win-native-integration, the
    // compile-time MT-045 approval flag must be live so the bootstrap
    // path registers the runtime adapter rather than skipping it.
    assert!(
        WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
        "MT-045 approval must remain live in Windows + win-native-integration builds"
    );
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_try_new_reports_runtime_capabilities_after_mt045_approval() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("MT-045 approved Windows-native backend should initialize on Windows");
    let caps = adapter.capabilities();

    assert_eq!(
        caps.adapter_id,
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert!(caps.runtime_available);
    assert_eq!(
        caps.filesystem_isolation_strength,
        IsolationStrength::VeryStrong
    );
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Strong);
    assert_eq!(caps.gpu_passthrough, GpuPassthrough::VendorAgnostic);
    assert_eq!(caps.stdio_throughput_class, ThroughputClass::Medium);
    assert!(caps.win32_native_fidelity);
    assert!(!caps.cross_machine_portable);
}

#[cfg(not(all(target_os = "windows", feature = "win-native-integration")))]
#[tokio::test]
async fn windows_native_jail_try_new_is_typed_adapter_unavailable_off_windows_or_no_feature() {
    let error = WindowsNativeJailAdapter::try_new()
        .await
        .expect_err("try_new must fail closed without Windows + win-native-integration");
    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(
                reason.contains("WindowsNativeJailAdapter unavailable"),
                "{reason}"
            );
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn windows_native_jail_rejects_empty_spawn_before_backend_lookup() {
    let adapter = WindowsNativeJailAdapter::unavailable_for_current_host();
    let error = adapter
        .spawn(process_spec(Vec::new()))
        .await
        .expect_err("empty command must fail as invalid caller input");

    assert_spawn_failed_reason_contains(error, "requires ProcessSpec.cmd");
}

#[tokio::test]
async fn windows_native_jail_unavailable_path_fails_closed_with_typed_error() {
    let adapter = WindowsNativeJailAdapter::unavailable_for_current_host();
    let error = adapter
        .spawn(process_spec(vec!["cmd.exe", "/C", "echo hello"]))
        .await
        .expect_err("unavailable adapter must fail closed");

    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(
                reason.contains("WindowsNativeJailAdapter unavailable"),
                "{reason}"
            );
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_spawn_runs_process_in_native_backend_with_stdout() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let probe_exe = PathBuf::from(env!("CARGO_BIN_EXE_mt046_token_probe"));
    let output_dir = tempfile::tempdir().expect("create stdout output dir");
    let output_path = output_dir.path().join("stdout-probe.txt");
    let mut spec = process_spec(vec![
        probe_exe.to_string_lossy().as_ref(),
        "--stdout-probe",
        output_path.to_string_lossy().as_ref(),
    ]);
    spec.cwd = Some(output_dir.path().to_path_buf());
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: output_dir.path().to_path_buf(),
        guest_path: PathBuf::from("C:/handshake/stdout-output"),
        mode: BindMode::ReadWrite,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: probe_exe.clone(),
        guest_path: PathBuf::from("C:/handshake/mt046-token-probe.exe"),
        mode: BindMode::ReadOnly,
    });

    let handle = adapter
        .spawn(spec)
        .await
        .expect("spawn should launch through Windows-native backend");

    assert_eq!(
        handle.adapter_id,
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert!(handle.pid.is_some(), "spawned handle should expose os pid");

    assert_eq!(wait_for_exit_code(&adapter, &handle).await, Some(0));
    assert_eq!(
        adapter.status(&handle).await.expect("query status"),
        ProcessStatus::Exited { code: 0 }
    );
    let probe = fs::read_to_string(&output_path).expect("read stdout probe receipt");
    assert!(
        probe.contains("stdout_probe_completed=true"),
        "stdout probe did not complete:\n{probe}"
    );
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_spawn_uses_appcontainer_and_restricted_token() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let output_dir = tempfile::tempdir().expect("create token probe output dir");
    let output_path = output_dir.path().join("mt046-token-probe.txt");
    let probe_exe = PathBuf::from(env!("CARGO_BIN_EXE_mt046_token_probe"));

    let mut spec = process_spec(vec![
        probe_exe.to_string_lossy().as_ref(),
        output_path.to_string_lossy().as_ref(),
    ]);
    spec.cwd = Some(output_dir.path().to_path_buf());
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: output_dir.path().to_path_buf(),
        guest_path: PathBuf::from("C:/handshake/probe-output"),
        mode: BindMode::ReadWrite,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: probe_exe.clone(),
        guest_path: PathBuf::from("C:/handshake/mt046-token-probe.exe"),
        mode: BindMode::ReadOnly,
    });

    let handle = adapter
        .spawn(spec)
        .await
        .expect("spawn should launch the token probe through the Windows-native backend");

    let mut exit_code = None;
    for _ in 0..50 {
        exit_code = adapter.exit_code(&handle).await.expect("query exit code");
        if exit_code.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(exit_code, Some(0));

    let probe = fs::read_to_string(&output_path).unwrap_or_else(|error| {
        panic!(
            "read token probe receipt {}: {error}",
            output_path.display()
        )
    });
    assert!(
        probe.contains("is_appcontainer=true"),
        "token probe did not prove AppContainer state:\n{probe}"
    );
    assert!(
        probe.contains("is_restricted=true"),
        "token probe did not prove Restricted Token state:\n{probe}"
    );
    assert!(
        probe
            .lines()
            .find_map(|line| line.strip_prefix("restricted_sid_count="))
            .and_then(|value| value.parse::<u32>().ok())
            .is_some_and(|count| count > 0),
        "token probe did not prove restricted SID material:\n{probe}"
    );
    assert!(
        probe.contains("is_lpac=true") || probe.contains("is_lpac=false"),
        "token probe did not report LPAC telemetry:\n{probe}"
    );
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_enforces_bind_acl_modes_and_outside_denial() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let probe_exe = PathBuf::from(env!("CARGO_BIN_EXE_mt046_token_probe"));
    let read_only_dir = tempfile::tempdir().expect("create read-only dir");
    let read_write_dir = tempfile::tempdir().expect("create read-write dir");
    let outside_dir = tempfile::tempdir().expect("create outside dir");
    let read_only_path = read_only_dir.path().join("allowed-ro.txt");
    let read_write_path = read_write_dir.path().join("allowed-rw.txt");
    let outside_path = outside_dir.path().join("outside.txt");
    let output_path = read_write_dir.path().join("fs-probe.txt");
    fs::write(&read_only_path, "mt046-ro-sentinel").expect("write read-only sentinel");
    fs::write(&outside_path, "mt046-outside-sentinel").expect("write outside sentinel");

    let mut spec = process_spec(vec![
        probe_exe.to_string_lossy().as_ref(),
        "--fs-probe",
        output_path.to_string_lossy().as_ref(),
        read_only_path.to_string_lossy().as_ref(),
        read_write_path.to_string_lossy().as_ref(),
        outside_path.to_string_lossy().as_ref(),
    ]);
    spec.cwd = Some(read_write_dir.path().to_path_buf());
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: read_only_path.clone(),
        guest_path: PathBuf::from("C:/handshake/allowed-ro.txt"),
        mode: BindMode::ReadOnly,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: read_write_dir.path().to_path_buf(),
        guest_path: PathBuf::from("C:/handshake/rw"),
        mode: BindMode::ReadWrite,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: probe_exe.clone(),
        guest_path: PathBuf::from("C:/handshake/mt046-token-probe.exe"),
        mode: BindMode::ReadOnly,
    });

    let handle = adapter.spawn(spec).await.expect("spawn fs probe");
    assert_eq!(wait_for_exit_code(&adapter, &handle).await, Some(0));

    let probe = fs::read_to_string(&output_path).expect("read fs probe receipt");
    for expected in [
        "allowed_ro_read=true",
        "allowed_ro_write_denied=true",
        "allowed_rw_write=true",
        "outside_existing_read_denied=true",
        "outside_existing_write_denied=true",
    ] {
        assert!(probe.contains(expected), "missing {expected} in:\n{probe}");
    }
    assert_eq!(
        fs::read_to_string(&read_write_path).expect("read rw sentinel"),
        "mt046-rw-sentinel"
    );
    assert_eq!(
        fs::read_to_string(&outside_path).expect("outside file must remain unchanged"),
        "mt046-outside-sentinel"
    );
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_job_kill_terminates_child_tree() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let probe_exe = PathBuf::from(env!("CARGO_BIN_EXE_mt046_token_probe"));
    let report_dir = tempfile::tempdir().expect("create job report dir");
    let report_path = report_dir.path().join("job-report.txt");

    let mut spec = process_spec(vec![
        probe_exe.to_string_lossy().as_ref(),
        "--job-grandchild",
        report_path.to_string_lossy().as_ref(),
    ]);
    spec.cwd = Some(report_dir.path().to_path_buf());
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: report_dir.path().to_path_buf(),
        guest_path: PathBuf::from("C:/handshake/job-report"),
        mode: BindMode::ReadWrite,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: probe_exe.clone(),
        guest_path: PathBuf::from("C:/handshake/mt046-token-probe.exe"),
        mode: BindMode::ReadOnly,
    });

    let handle = adapter.spawn(spec).await.expect("spawn job probe");
    let report = wait_for_report(&report_path).await;
    let child_pid = report
        .lines()
        .find_map(|line| line.strip_prefix("child_pid="))
        .and_then(|value| value.parse::<u32>().ok())
        .expect("job report should include child pid");

    adapter
        .kill(&handle, Signal::Kill)
        .await
        .expect("kill job handle");
    for _ in 0..50 {
        if !process_is_still_active(child_pid) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("grandchild process {child_pid} stayed active after job kill");
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_deny_all_blocks_outbound_tcp_probe() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let probe_exe = PathBuf::from(env!("CARGO_BIN_EXE_mt046_token_probe"));
    let output_dir = tempfile::tempdir().expect("create network output dir");
    let output_path = output_dir.path().join("network-probe.txt");

    let mut spec = process_spec(vec![
        probe_exe.to_string_lossy().as_ref(),
        "--network-deny-probe",
        output_path.to_string_lossy().as_ref(),
    ]);
    spec.cwd = Some(output_dir.path().to_path_buf());
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: output_dir.path().to_path_buf(),
        guest_path: PathBuf::from("C:/handshake/network-output"),
        mode: BindMode::ReadWrite,
    });
    spec.binds.push(handshake_core::sandbox::BindSpec {
        host_path: probe_exe.clone(),
        guest_path: PathBuf::from("C:/handshake/mt046-token-probe.exe"),
        mode: BindMode::ReadOnly,
    });

    let handle = adapter.spawn(spec).await.expect("spawn network probe");
    assert_eq!(wait_for_exit_code(&adapter, &handle).await, Some(0));
    let probe = fs::read_to_string(&output_path).expect("read network probe receipt");
    assert!(
        probe.contains("tcp_connect_denied=true"),
        "DenyAll network probe unexpectedly connected:\n{probe}"
    );
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_rejects_loopback_only_at_spawn_until_exemption_support_exists() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let mut spec = process_spec(vec![
        "C:/Windows/System32/cmd.exe",
        "/C",
        "echo handshake-win-native-jail",
    ]);
    spec.net_policy = NetPolicy::LoopbackOnly;

    let error = adapter
        .spawn(spec)
        .await
        .expect_err("LoopbackOnly must fail loudly until MT-046 wires loopback exemption support");

    match error {
        SandboxAdapterError::NetPolicyApplyFailed { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(reason.contains("LoopbackOnly"), "{reason}");
            assert!(reason.contains("use DenyAll"), "{reason}");
        }
        other => panic!("expected NetPolicyApplyFailed, got {other:?}"),
    }
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
#[tokio::test]
async fn windows_native_jail_rejects_allowlist_at_spawn_until_wfp_support_exists() {
    let adapter = WindowsNativeJailAdapter::try_new()
        .await
        .expect("Windows-native backend should initialize");
    let mut spec = process_spec(vec![
        "C:/Windows/System32/cmd.exe",
        "/C",
        "echo handshake-win-native-jail",
    ]);
    spec.net_policy = NetPolicy::Allowlist(vec![NetAllowlistEntry {
        host: "example.com".to_string(),
        port: Some(443),
        protocol: NetProtocol::Tcp,
    }]);

    let error = adapter
        .spawn(spec)
        .await
        .expect_err("Allowlist must fail loudly until MT-046 wires WFP allowlist support");

    match error {
        SandboxAdapterError::NetPolicyApplyFailed { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(reason.contains("allowlists"), "{reason}");
            assert!(reason.contains("use DenyAll"), "{reason}");
        }
        other => panic!("expected NetPolicyApplyFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn windows_native_jail_rejects_post_spawn_mutations_loudly() {
    let adapter = WindowsNativeJailAdapter::unavailable_for_current_host();
    let handle = ProcessHandle::new(
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        None,
        "win-native-test-handle",
    );

    let exec_error = adapter
        .exec(
            &handle,
            handshake_core::sandbox::Command {
                argv: vec![
                    "cmd.exe".to_string(),
                    "/C".to_string(),
                    "echo late".to_string(),
                ],
                env_overlay: BTreeMap::new(),
                stdin: None,
                timeout_ms: Some(1_000),
            },
        )
        .await
        .expect_err("exec-after-spawn must fail loudly");
    assert_spawn_failed_reason_contains(exec_error, "does not support exec");

    let bind_error = adapter
        .fs_bind(
            &handle,
            PathBuf::from("fixtures/host"),
            PathBuf::from("C:/sandbox/host"),
            BindMode::ReadOnly,
        )
        .await
        .expect_err("post-spawn fs_bind must fail loudly");
    assert_spawn_failed_reason_contains(bind_error, "post-spawn fs_bind unsupported");

    let net_error = adapter
        .net_policy(&handle, NetPolicy::LoopbackOnly)
        .await
        .expect_err("post-spawn net_policy must fail loudly");
    match net_error {
        SandboxAdapterError::NetPolicyApplyFailed { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(
                reason.contains("post-spawn net_policy unsupported"),
                "{reason}"
            );
            assert!(reason.contains("declare policy before spawn"), "{reason}");
        }
        other => panic!("expected NetPolicyApplyFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn windows_native_jail_rejects_stale_handles() {
    let adapter = WindowsNativeJailAdapter::unavailable_for_current_host();
    let handle = ProcessHandle::new(AdapterId::new("wsl2_podman"), None, "wrong-adapter");

    let error = adapter
        .kill(&handle, Signal::Kill)
        .await
        .expect_err("wrong-adapter handle must be rejected");
    assert!(matches!(
        error,
        SandboxAdapterError::ProcessHandleStale { .. }
    ));
}

fn process_spec(cmd: Vec<&str>) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("win-native-test"),
        image_or_root: ImageRef::new("C:/Windows/System32"),
        cmd: cmd.into_iter().map(str::to_string).collect(),
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        required_capabilities: BTreeSet::from([RequiredCapability::Win32NativeFidelity]),
        metadata: BTreeMap::new(),
    }
}

fn assert_spawn_failed_reason_contains(error: SandboxAdapterError, needle: &str) {
    match error {
        SandboxAdapterError::SpawnFailed { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(reason.contains(needle), "{reason}");
        }
        other => panic!("expected SpawnFailed, got {other:?}"),
    }
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
async fn wait_for_exit_code(
    adapter: &WindowsNativeJailAdapter,
    handle: &ProcessHandle,
) -> Option<i32> {
    let mut exit_code = None;
    for _ in 0..60 {
        exit_code = adapter.exit_code(handle).await.expect("query exit code");
        if exit_code.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    exit_code
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
async fn wait_for_report(path: &Path) -> String {
    for _ in 0..60 {
        if path.exists() {
            return fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read report {}: {error}", path.display()));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for report {}", path.display());
}

#[cfg(all(target_os = "windows", feature = "win-native-integration"))]
fn process_is_still_active(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, STILL_ACTIVE},
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return false;
    }
    let mut exit_code = 0u32;
    let ok = unsafe { GetExitCodeProcess(process, &mut exit_code) };
    unsafe {
        let _ = CloseHandle(process);
    }
    ok != 0 && exit_code == STILL_ACTIVE as u32
}

fn read_module_sources(root: &Path) -> String {
    read_module_sources_with_filter(root, &|_| true)
}

fn read_module_sources_excluding(root: &Path, excluded_files: &[&str]) -> String {
    read_module_sources_with_filter(root, &|path| {
        !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| excluded_files.contains(&name))
    })
}

fn read_module_sources_with_filter(root: &Path, include: &dyn Fn(&Path) -> bool) -> String {
    let mut combined = String::new();
    for entry in fs::read_dir(root).expect("read WindowsNativeJailAdapter module directory") {
        let entry = entry.expect("read WindowsNativeJailAdapter module entry");
        let path = entry.path();
        if path.is_dir() {
            combined.push_str(&read_module_sources_with_filter(&path, include));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") && include(&path) {
            combined.push_str(
                &fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
            );
        }
    }
    combined
}
