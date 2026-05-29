use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

use bytes::Bytes;
use handshake_core::sandbox::{
    docker_exec_args, docker_run_args, parse_docker_exit_code, parse_docker_status, select,
    AdapterId, BindMode, BindSpec, Command, DockerAdapter, DockerConfig, GpuPassthrough, ImageRef,
    IsolationStrength, NetAllowlistEntry, NetPolicy, NetProtocol, ProcessHandle, ProcessSpec,
    ProcessStatus, RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    SandboxAdapterRegistry, SandboxSelectionFailure, Signal, ThroughputClass, TrustClass,
    DOCKER_ADAPTER_ID,
};

#[test]
fn docker_run_args_map_process_spec_to_compat_isolation_controls() {
    let container_name = "hsk-018f4b49b2a87f1f9e1d0f3d2c1b0a9e";
    let model_host_path = PathBuf::from("D:\\Models\\model.gguf");
    let spec = ProcessSpec {
        id: AdapterId::new("spec-1"),
        image_or_root: ImageRef::new("alpine:3.20"),
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
        trust_class: TrustClass::default(),
        metadata: BTreeMap::new(),
    };

    let args = docker_run_args(&spec, container_name).expect("build docker args");

    assert_eq!(args.first().map(String::as_str), Some("run"));
    assert!(
        !args.iter().any(|arg| arg == "--rm"),
        "detached containers must remain inspectable for status()/exit_code() until kill/cleanup"
    );
    assert!(contains_pair(&args, "--name", container_name));
    assert!(args.contains(&"-d".to_string()));
    assert!(args.contains(&"--read-only".to_string()));
    assert!(contains_pair(
        &args,
        "--tmpfs",
        "/tmp:rw,noexec,nosuid,nodev,mode=1777"
    ));
    assert!(contains_pair(&args, "--pids-limit", "4096"));
    assert!(contains_pair(&args, "--network", "none"));
    assert!(contains_pair(&args, "-w", "/workspace"));
    assert!(contains_pair(&args, "-e", "HANDSHAKE_MODE=test"));
    assert!(contains_pair(
        &args,
        "-v",
        "D:/Models/model.gguf:/models/model.gguf:ro"
    ));
    assert!(contains_pair(
        &args,
        "-v",
        "fixtures/tool:/workspace/tool:ro,noexec"
    ));
    assert!(contains_pair(&args, "--memory", "2147483648"));
    assert!(contains_pair(&args, "--cpus", "2"));
    assert!(args.ends_with(&[
        "alpine:3.20".to_string(),
        "sh".to_string(),
        "-c".to_string(),
        "sleep 30".to_string()
    ]));
}

#[test]
fn docker_network_policies_fail_closed_until_firewall_seeding_exists() {
    let deny_all =
        docker_run_args(&spec_with_net_policy(NetPolicy::DenyAll), "ctr").expect("deny args");
    assert!(contains_pair(&deny_all, "--network", "none"));

    let loopback =
        docker_run_args(&spec_with_net_policy(NetPolicy::LoopbackOnly), "ctr").expect("loopback");
    assert!(contains_pair(&loopback, "--network", "none"));

    let loopback_allowlist = docker_run_args(
        &spec_with_net_policy(NetPolicy::Allowlist(vec![NetAllowlistEntry {
            host: "127.0.0.1".to_string(),
            port: Some(11434),
            protocol: NetProtocol::Tcp,
        }])),
        "ctr",
    )
    .expect("loopback allowlist");
    assert!(contains_pair(&loopback_allowlist, "--network", "none"));

    let external_allowlist = docker_run_args(
        &spec_with_net_policy(NetPolicy::Allowlist(vec![NetAllowlistEntry {
            host: "example.com".to_string(),
            port: Some(443),
            protocol: NetProtocol::Tcp,
        }])),
        "ctr",
    );
    match external_allowlist {
        Err(SandboxAdapterError::NetPolicyApplyFailed { adapter_id, reason }) => {
            assert_eq!(adapter_id, AdapterId::new(DOCKER_ADAPTER_ID));
            assert!(reason.contains("external allowlist"));
            assert!(reason.contains("fail closed"));
        }
        other => panic!("expected fail-closed external allowlist, got {other:?}"),
    }
}

#[test]
fn docker_exec_args_include_interactive_stdin_and_env_overlay() {
    let args = docker_exec_args(
        "container-123",
        &Command {
            argv: vec!["sh".to_string(), "-c".to_string(), "cat".to_string()],
            env_overlay: BTreeMap::from([("HANDSHAKE_EXEC".to_string(), "1".to_string())]),
            stdin: Some(Bytes::from_static(b"input")),
            timeout_ms: Some(1_000),
        },
    )
    .expect("build docker exec args");

    assert_eq!(args.first().map(String::as_str), Some("exec"));
    assert!(args.contains(&"--interactive".to_string()));
    assert!(contains_pair(&args, "-e", "HANDSHAKE_EXEC=1"));
    assert!(args.ends_with(&[
        "container-123".to_string(),
        "sh".to_string(),
        "-c".to_string(),
        "cat".to_string()
    ]));

    let no_stdin = docker_exec_args(
        "container-123",
        &Command {
            argv: vec!["true".to_string()],
            env_overlay: BTreeMap::new(),
            stdin: None,
            timeout_ms: None,
        },
    )
    .expect("build docker exec args without stdin");
    assert!(!no_stdin.iter().any(|arg| arg == "--interactive"));
}

#[test]
fn docker_status_and_exit_code_parsers_are_strict_but_tolerant_of_whitespace() {
    assert_eq!(
        parse_docker_status("running\n", None),
        ProcessStatus::Running
    );
    assert_eq!(
        parse_docker_status("exited\r\n", Some(7)),
        ProcessStatus::Exited { code: 7 }
    );
    assert_eq!(parse_docker_status("dead\n", None), ProcessStatus::Orphaned);
    assert_eq!(
        parse_docker_status("created\n", None),
        ProcessStatus::FailedToStart {
            reason: "docker container status created".to_string()
        }
    );
    assert_eq!(parse_docker_exit_code(" 42\r\n").unwrap(), Some(42));
    assert_eq!(parse_docker_exit_code("").unwrap(), None);
}

#[test]
fn kill_signal_args_are_mapped_to_docker_stop_or_signal_commands() {
    assert_eq!(
        DockerAdapter::kill_args("ctr", Signal::Term),
        vec!["stop", "--timeout", "10", "ctr"]
    );
    assert_eq!(
        DockerAdapter::kill_args("ctr", Signal::Kill),
        vec!["kill", "--signal", "KILL", "ctr"]
    );
    assert_eq!(
        DockerAdapter::kill_args("ctr", Signal::Int),
        vec!["kill", "--signal", "INT", "ctr"]
    );
}

#[test]
fn adapter_capabilities_identify_docker_compat_only_adapter() {
    let adapter = DockerAdapter::with_config_and_gpu_for_tests(
        DockerConfig::new("docker"),
        GpuPassthrough::NvidiaCuda,
    );
    let caps = adapter.capabilities();

    assert_eq!(caps.adapter_id, AdapterId::new(DOCKER_ADAPTER_ID));
    assert_eq!(
        caps.filesystem_isolation_strength,
        IsolationStrength::Strong
    );
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Strong);
    assert_eq!(caps.gpu_passthrough, GpuPassthrough::NvidiaCuda);
    assert_eq!(caps.stdio_throughput_class, ThroughputClass::High);
    assert!(!caps.win32_native_fidelity);
    assert!(caps.cross_machine_portable);
}

#[tokio::test]
async fn post_spawn_fs_bind_fails_loud_because_docker_binds_are_spawn_time_only() {
    let adapter = DockerAdapter::with_config_and_gpu_for_tests(
        DockerConfig::new("docker"),
        GpuPassthrough::None,
    );
    let handle = ProcessHandle::new(AdapterId::new(DOCKER_ADAPTER_ID), None, "container-id");
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
            assert_eq!(adapter_id, AdapterId::new(DOCKER_ADAPTER_ID));
            assert!(reason.contains("post-spawn fs_bind unsupported"));
            assert!(reason.contains("ProcessSpec.binds"));
        }
        other => panic!("expected loud SpawnFailed for post-spawn bind, got {other:?}"),
    }
}

#[test]
fn docker_selection_requires_explicit_opt_in_with_real_adapter() {
    let docker = Arc::new(DockerAdapter::with_config_and_gpu_for_tests(
        DockerConfig::new("docker"),
        GpuPassthrough::None,
    ));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new(DOCKER_ADAPTER_ID));
    registry.register(docker);

    let docker_id = AdapterId::new(DOCKER_ADAPTER_ID);
    let error = match select(
        &registry,
        &spec_with_net_policy(NetPolicy::DenyAll),
        Some(&docker_id),
    ) {
        Ok(adapter) => panic!(
            "expected docker opt-in failure, selected {}",
            adapter.capabilities().adapter_id
        ),
        Err(error) => error,
    };
    assert_eq!(error, SandboxSelectionFailure::DockerNotExplicitlyOptedIn);

    registry.set_docker_explicit_opt_in(true);
    let selected = select(
        &registry,
        &spec_with_net_policy(NetPolicy::DenyAll),
        Some(&docker_id),
    )
    .expect("explicit opt-in selects docker");
    assert_eq!(selected.capabilities().adapter_id, docker_id);
}

#[tokio::test]
#[cfg_attr(not(feature = "docker-integration"), ignore)]
async fn docker_spawn_exec_network_denial_bind_readonly_and_cleanup_integration() {
    let adapter = match DockerAdapter::try_new(DockerConfig::new("docker")).await {
        Ok(adapter) => adapter,
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("docker unavailable")
                || reason.contains("failed to spawn")
                || reason.contains("Docker daemon") =>
        {
            eprintln!("skipping live Docker integration: {reason}");
            return;
        }
        Err(error) => panic!("Docker integration setup failed unexpectedly: {error:?}"),
    };

    let handle = adapter
        .spawn(ProcessSpec {
            id: AdapterId::new("integration"),
            image_or_root: ImageRef::new("alpine:3.20"),
            cmd: vec!["sleep".to_string(), "30".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            required_capabilities: BTreeSet::new(),
            trust_class: TrustClass::default(),
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
            image_or_root: ImageRef::new("alpine:3.20"),
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
            trust_class: TrustClass::default(),
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

fn contains_pair(args: &[String], flag: &str, value: &str) -> bool {
    args.windows(2)
        .any(|window| window[0] == flag && window[1] == value)
}

fn spec_with_net_policy(net_policy: NetPolicy) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("spec-net"),
        image_or_root: ImageRef::new("alpine:3.20"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy,
        resource_limits: ResourceLimits::default(),
        required_capabilities: BTreeSet::new(),
        // Selection-path fixture: keep it trusted so the Tier-1 docker adapter
        // is selectable; this test covers docker opt-in, not the trust tier.
        trust_class: TrustClass::Trusted,
        metadata: BTreeMap::new(),
    }
}
