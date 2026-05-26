use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    thread,
};

use bytes::Bytes;
use handshake_core::sandbox::{
    decode_wsl_output, parse_nvidia_smi_output, parse_podman_exit_code, parse_podman_rootless_info,
    parse_podman_status, parse_wsl_list_verbose, parse_wsl_status, podman_run_args,
    windows_path_to_wsl_mount_path, AdapterId, BindMode, BindSpec, Command, GpuPassthrough,
    ImageRef, NetAllowlistEntry, NetPolicy, NetProtocol, ProcessHandle, ProcessSpec, ProcessStatus,
    RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal,
    Wsl2PodmanAdapter, Wsl2PodmanConfig, WSL2_PODMAN_ADAPTER_ID,
};

#[test]
fn wsl_detection_parses_utf16_status_and_verbose_distro_list() {
    let status_text = "Default Distribution: Ubuntu\r\nDefault Version: 2\r\n";
    let status_utf16 = utf16le(status_text);
    let decoded = decode_wsl_output(&status_utf16);
    let status = parse_wsl_status(&decoded).expect("parse WSL status");

    assert_eq!(status.default_distribution.as_deref(), Some("Ubuntu"));
    assert_eq!(status.default_version, Some(2));

    let list_text = "  NAME            STATE           VERSION\r\n* Ubuntu          Running         2\r\n  docker-desktop  Running         2\r\n";
    let distros = parse_wsl_list_verbose(&decode_wsl_output(&utf16le(list_text)));
    let ubuntu = distros
        .iter()
        .find(|distro| distro.name == "Ubuntu")
        .expect("Ubuntu distro parsed");

    assert!(ubuntu.is_default);
    assert_eq!(ubuntu.state.as_deref(), Some("Running"));
    assert_eq!(ubuntu.version, Some(2));
}

#[test]
fn podman_run_args_map_process_spec_to_default_isolation_controls() {
    let drive = 'D';
    let model_host_path = PathBuf::from(format!("{drive}:\\Models\\model.gguf"));
    let expected_model_bind = format!(
        "/mnt/{}/Models/model.gguf:/models/model.gguf:ro",
        drive.to_ascii_lowercase()
    );
    let spec = ProcessSpec {
        id: AdapterId::new("spec-1"),
        image_or_root: ImageRef::new("docker.io/library/alpine:3.20"),
        cmd: vec!["sh".to_string(), "-c".to_string(), "sleep 30".to_string()],
        env: BTreeMap::from([("HANDSHAKE_MODE".to_string(), "test".to_string())]),
        cwd: Some(PathBuf::from("/workspace")),
        binds: vec![
            BindSpec {
                host_path: model_host_path,
                guest_path: PathBuf::from("/models/model.gguf"),
                mode: BindMode::ReadOnly,
            },
            BindSpec {
                host_path: PathBuf::from("fixtures/tool"),
                guest_path: PathBuf::from("/workspace/tool"),
                mode: BindMode::NoExec,
            },
        ],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits {
            memory_bytes: Some(2 * 1024 * 1024 * 1024),
            cpu_cores: Some(2),
            timeout_ms: Some(30_000),
        },
        required_capabilities: BTreeSet::from([RequiredCapability::CrossMachinePortable]),
        metadata: BTreeMap::new(),
    };

    let args = podman_run_args(&spec).expect("build podman args");

    assert_eq!(args.first().map(String::as_str), Some("--remote=false"));
    assert!(
        !args.iter().any(|arg| arg == "--rm"),
        "detached containers must remain inspectable for status()/exit_code() until kill/cleanup"
    );
    assert!(contains_pair(&args, "--userns", "keep-id"));
    assert!(args.contains(&"--read-only".to_string()));
    assert!(contains_pair(
        &args,
        "--tmpfs",
        "/tmp:rw,noexec,nosuid,nodev,mode=1777"
    ));
    assert!(contains_pair(&args, "--network", "none"));
    assert!(contains_pair(&args, "-w", "/workspace"));
    assert!(contains_pair(&args, "-e", "HANDSHAKE_MODE=test"));
    assert!(contains_pair(&args, "-v", &expected_model_bind));
    assert!(contains_pair(
        &args,
        "-v",
        "fixtures/tool:/workspace/tool:ro,noexec"
    ));
    assert!(contains_pair(&args, "--memory", "2147483648"));
    assert!(contains_pair(&args, "--cpus", "2"));
    assert!(args.ends_with(&[
        "docker.io/library/alpine:3.20".to_string(),
        "sh".to_string(),
        "-c".to_string(),
        "sleep 30".to_string()
    ]));
}

#[test]
fn loopback_and_allowlist_network_policies_have_explicit_spawn_modes() {
    let loopback_args =
        podman_run_args(&spec_with_net_policy(NetPolicy::LoopbackOnly)).expect("loopback args");
    assert!(contains_pair(&loopback_args, "--network", "none"));

    let allowlist_args = podman_run_args(&spec_with_net_policy(NetPolicy::Allowlist(vec![
        NetAllowlistEntry {
            host: "127.0.0.1".to_string(),
            port: Some(11434),
            protocol: NetProtocol::Tcp,
        },
    ])))
    .expect("allowlist args");
    assert!(contains_pair(&allowlist_args, "--network", "none"));

    let external_allowlist = podman_run_args(&spec_with_net_policy(NetPolicy::Allowlist(vec![
        NetAllowlistEntry {
            host: "example.com".to_string(),
            port: Some(443),
            protocol: NetProtocol::Tcp,
        },
    ])));
    assert!(
        matches!(
            external_allowlist,
            Err(SandboxAdapterError::NetPolicyApplyFailed { .. })
        ),
        "external allowlists must fail closed until firewall seeding is implemented"
    );
}

#[test]
fn status_exit_code_and_gpu_probe_parsers_are_strict_but_tolerant_of_whitespace() {
    assert_eq!(
        parse_podman_status("running\n", None),
        ProcessStatus::Running
    );
    assert_eq!(
        parse_podman_status("exited\n", Some(7)),
        ProcessStatus::Exited { code: 7 }
    );
    assert_eq!(
        parse_podman_status("configured\n", None),
        ProcessStatus::FailedToStart {
            reason: "podman container status configured".to_string()
        }
    );
    assert_eq!(parse_podman_exit_code(" 42\r\n").unwrap(), Some(42));
    assert_eq!(parse_podman_exit_code("").unwrap(), None);

    assert_eq!(
        parse_nvidia_smi_output("NVIDIA GeForce RTX 4090\n"),
        GpuPassthrough::NvidiaCuda
    );
    assert_eq!(parse_nvidia_smi_output(""), GpuPassthrough::None);
}

#[test]
fn rootless_podman_info_parser_requires_true_rootless_mode() {
    assert_eq!(parse_podman_rootless_info("true\n").unwrap(), true);
    assert_eq!(parse_podman_rootless_info("false\n").unwrap(), false);
    assert!(parse_podman_rootless_info("").is_err());
}

#[test]
fn windows_paths_are_converted_to_wsl_mount_paths_without_hardcoded_workspace_roots() {
    let drive = 'D';
    let host_path = PathBuf::from(format!("{drive}:\\Models\\foo.gguf"));
    let expected = format!("/mnt/{}/Models/foo.gguf", drive.to_ascii_lowercase());
    assert_eq!(
        windows_path_to_wsl_mount_path(host_path.as_path()),
        expected
    );
    assert_eq!(
        windows_path_to_wsl_mount_path(PathBuf::from("fixtures/input").as_path()),
        "fixtures/input"
    );
}

#[test]
fn adapter_capabilities_identify_wsl2_podman_default_and_do_not_claim_win32_fidelity() {
    let adapter = Wsl2PodmanAdapter::with_config_and_gpu_for_tests(
        Wsl2PodmanConfig::for_distro("Ubuntu"),
        GpuPassthrough::NvidiaCuda,
    );
    let caps = adapter.capabilities();

    assert_eq!(caps.adapter_id, AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
    assert!(caps.cross_machine_portable);
    assert!(!caps.win32_native_fidelity);
    assert_eq!(caps.gpu_passthrough, GpuPassthrough::NvidiaCuda);
}

#[test]
fn adapter_capabilities_gpu_cache_is_concurrency_safe() {
    let adapter = Arc::new(Wsl2PodmanAdapter::with_config_and_gpu_for_tests(
        Wsl2PodmanConfig::for_distro("Ubuntu"),
        GpuPassthrough::NvidiaCuda,
    ));
    let mut workers = Vec::new();

    for _ in 0..16 {
        let adapter = Arc::clone(&adapter);
        workers.push(thread::spawn(move || adapter.capabilities()));
    }

    for worker in workers {
        let capabilities = worker.join().expect("capability worker should not panic");
        assert_eq!(
            capabilities.adapter_id,
            AdapterId::new(WSL2_PODMAN_ADAPTER_ID)
        );
        assert_eq!(capabilities.gpu_passthrough, GpuPassthrough::NvidiaCuda);
    }
}

#[tokio::test]
async fn post_spawn_fs_bind_fails_loud_because_podman_binds_are_spawn_time_only() {
    let adapter = Wsl2PodmanAdapter::with_config_and_gpu_for_tests(
        Wsl2PodmanConfig::for_distro("Ubuntu"),
        GpuPassthrough::None,
    );
    let handle = ProcessHandle::new(AdapterId::new(WSL2_PODMAN_ADAPTER_ID), None, "container-id");
    let result = adapter
        .fs_bind(
            &handle,
            PathBuf::from("fixtures/host"),
            PathBuf::from("/workspace/host"),
            BindMode::ReadOnly,
        )
        .await;

    match result {
        Err(SandboxAdapterError::SpawnFailed { adapter_id, reason }) => {
            assert_eq!(adapter_id, AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
            assert!(reason.contains("post-spawn fs_bind unsupported"));
            assert!(reason.contains("ProcessSpec.binds"));
        }
        other => panic!("expected loud SpawnFailed for post-spawn bind, got {other:?}"),
    }
}

#[test]
fn kill_signal_args_are_mapped_to_podman_stop_or_signal_commands() {
    assert_eq!(
        Wsl2PodmanAdapter::kill_args("ctr", Signal::Term),
        vec!["--remote=false", "stop", "--time", "10", "ctr"]
    );
    assert_eq!(
        Wsl2PodmanAdapter::kill_args("ctr", Signal::Kill),
        vec!["--remote=false", "kill", "--signal", "KILL", "ctr"]
    );
    assert_eq!(
        Wsl2PodmanAdapter::kill_args("ctr", Signal::Int),
        vec!["--remote=false", "kill", "--signal", "INT", "ctr"]
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "wsl2-integration"), ignore)]
async fn wsl2_podman_spawn_exec_and_cleanup_integration() {
    let adapter = match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::for_distro("Ubuntu")).await {
        Ok(adapter) => adapter,
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("podman unavailable")
                || reason.contains("not registered")
                || reason.contains("WSL") =>
        {
            eprintln!("skipping live WSL2 Podman integration: {reason}");
            return;
        }
        Err(error) => panic!("WSL2 Podman integration setup failed unexpectedly: {error:?}"),
    };
    let handle = adapter
        .spawn(ProcessSpec {
            id: AdapterId::new("integration"),
            image_or_root: ImageRef::new("docker.io/library/alpine:3.20"),
            cmd: vec!["sleep".to_string(), "30".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            required_capabilities: BTreeSet::new(),
            metadata: BTreeMap::new(),
        })
        .await
        .expect("spawn alpine sleep");

    let exec = adapter
        .exec(
            &handle,
            Command {
                argv: vec!["sh".to_string(), "-c".to_string(), "echo hello".to_string()],
                env_overlay: BTreeMap::new(),
                stdin: Some(Bytes::new()),
                timeout_ms: Some(10_000),
            },
        )
        .await
        .expect("exec echo");
    assert_eq!(exec.exit_code, 0);
    assert_eq!(exec.stdout, Bytes::from_static(b"hello\n"));

    let dns = adapter
        .exec(
            &handle,
            Command {
                argv: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "nslookup example.com >/tmp/dns.out 2>&1".to_string(),
                ],
                env_overlay: BTreeMap::new(),
                stdin: Some(Bytes::new()),
                timeout_ms: Some(10_000),
            },
        )
        .await
        .expect("exec dns denial probe");
    assert_ne!(
        dns.exit_code, 0,
        "NetPolicy::DenyAll must block outbound DNS"
    );

    adapter
        .kill(&handle, Signal::Kill)
        .await
        .expect("cleanup container");

    let readonly_dir = tempfile::tempdir().expect("temp read-only bind dir");
    std::fs::write(readonly_dir.path().join("seed.txt"), b"seed").expect("seed file");
    let readonly_handle = adapter
        .spawn(ProcessSpec {
            id: AdapterId::new("readonly-bind"),
            image_or_root: ImageRef::new("docker.io/library/alpine:3.20"),
            cmd: vec!["sleep".to_string(), "30".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: vec![BindSpec {
                host_path: readonly_dir.path().to_path_buf(),
                guest_path: PathBuf::from("/readonly"),
                mode: BindMode::ReadOnly,
            }],
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            required_capabilities: BTreeSet::new(),
            metadata: BTreeMap::new(),
        })
        .await
        .expect("spawn readonly bind container");

    let write_probe = adapter
        .exec(
            &readonly_handle,
            Command {
                argv: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "echo x > /readonly/seed.txt".to_string(),
                ],
                env_overlay: BTreeMap::new(),
                stdin: Some(Bytes::new()),
                timeout_ms: Some(10_000),
            },
        )
        .await
        .expect("exec readonly write probe");
    assert_ne!(
        write_probe.exit_code, 0,
        "read-only bind write must fail inside guest"
    );
    adapter
        .kill(&readonly_handle, Signal::Kill)
        .await
        .expect("cleanup readonly container");
}

fn utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect()
}

fn contains_pair(args: &[String], flag: &str, value: &str) -> bool {
    args.windows(2)
        .any(|window| window[0] == flag && window[1] == value)
}

fn spec_with_net_policy(net_policy: NetPolicy) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("spec-net"),
        image_or_root: ImageRef::new("docker.io/library/alpine:3.20"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy,
        resource_limits: ResourceLimits::default(),
        required_capabilities: BTreeSet::new(),
        metadata: BTreeMap::new(),
    }
}
