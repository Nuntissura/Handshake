use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use handshake_core::sandbox::{
    AdapterId, BindMode, BindSpec, Command, ExecResult, ImageRef, NetAllowlistEntry, NetPolicy,
    NetProtocol, ProcessHandle, ProcessSpec, ProcessStatus, RequiredCapability, ResourceLimits,
    SandboxAdapterError, Signal, TrustClass,
};

#[test]
fn sandbox_types_tests_process_handle_new_mints_v7_uuid() {
    let handle = ProcessHandle::new(AdapterId::new("wsl2_podman"), Some(4242), "podman-abc");

    assert_eq!(handle.id.get_version_num(), 7);
    assert_eq!(handle.adapter_id.as_str(), "wsl2_podman");
    assert_eq!(handle.pid, Some(4242));
    assert_eq!(handle.sandbox_internal_id, "podman-abc");
}

#[test]
fn sandbox_types_tests_net_policy_round_trips_and_rejects_unknown_variants() {
    let deny = NetPolicy::DenyAll;
    let loopback = NetPolicy::LoopbackOnly;
    assert_ne!(deny, loopback);

    let allowlist = NetPolicy::Allowlist(vec![NetAllowlistEntry {
        host: "127.0.0.1".to_string(),
        port: Some(11434),
        protocol: NetProtocol::Tcp,
    }]);

    for policy in [deny, loopback, allowlist] {
        let encoded = serde_json::to_string(&policy).expect("net policy serializes");
        let decoded: NetPolicy = serde_json::from_str(&encoded).expect("net policy deserializes");
        assert_eq!(decoded, policy);
    }

    let unknown = serde_json::from_str::<NetPolicy>(r#""allow_all""#);
    assert!(
        unknown.is_err(),
        "unknown net policy variants must fail closed"
    );
}

#[test]
fn sandbox_types_tests_required_capabilities_are_deterministic_sets() {
    let mut left = BTreeSet::new();
    left.insert(RequiredCapability::NvidiaCudaPassthrough);
    left.insert(RequiredCapability::VeryStrongFilesystemIsolation);
    left.insert(RequiredCapability::HighStdioThroughput);

    let mut right = BTreeSet::new();
    right.insert(RequiredCapability::HighStdioThroughput);
    right.insert(RequiredCapability::NvidiaCudaPassthrough);
    right.insert(RequiredCapability::VeryStrongFilesystemIsolation);

    assert_eq!(left, right);
    assert_eq!(
        serde_json::to_string(&left).expect("left set serializes"),
        serde_json::to_string(&right).expect("right set serializes")
    );
}

#[test]
fn sandbox_types_tests_process_spec_command_and_exec_result_serde_round_trip() {
    let spec = ProcessSpec {
        id: AdapterId::new("windows_native_jail"),
        image_or_root: ImageRef::new("local-root"),
        cmd: vec!["python".to_string(), "-m".to_string(), "pytest".to_string()],
        env: BTreeMap::from([("RUST_LOG".to_string(), "info".to_string())]),
        cwd: Some(PathBuf::from("/workspace")),
        binds: vec![BindSpec {
            host_path: PathBuf::from("fixtures/input"),
            guest_path: PathBuf::from("/workspace/input"),
            mode: BindMode::ReadOnly,
        }],
        net_policy: NetPolicy::LoopbackOnly,
        resource_limits: ResourceLimits {
            memory_bytes: Some(4 * 1024 * 1024 * 1024),
            cpu_cores: Some(2),
            timeout_ms: Some(30_000),
        },
        required_capabilities: BTreeSet::from([
            RequiredCapability::Win32NativeFidelity,
            RequiredCapability::VeryStrongNetworkIsolation,
        ]),
        trust_class: TrustClass::Reviewed,
        metadata: BTreeMap::from([("wp".to_string(), "WP-KERNEL-004".to_string())]),
    };

    let command = Command {
        argv: vec!["cargo".to_string(), "test".to_string()],
        env_overlay: BTreeMap::from([("NO_COLOR".to_string(), "1".to_string())]),
        stdin: Some(Bytes::from_static(b"stdin")),
        timeout_ms: Some(60_000),
    };

    let result = ExecResult {
        exit_code: 0,
        stdout: Bytes::from_static(b"ok"),
        stderr: Bytes::new(),
        duration_ms: 123,
    };

    let status = ProcessStatus::Killed {
        by_signal: Signal::Term,
    };

    assert_eq!(
        serde_json::from_str::<ProcessSpec>(&serde_json::to_string(&spec).unwrap()).unwrap(),
        spec
    );
    assert_eq!(
        serde_json::from_str::<Command>(&serde_json::to_string(&command).unwrap()).unwrap(),
        command
    );
    assert_eq!(
        serde_json::from_str::<ExecResult>(&serde_json::to_string(&result).unwrap()).unwrap(),
        result
    );
    assert_eq!(
        serde_json::from_str::<ProcessStatus>(&serde_json::to_string(&status).unwrap()).unwrap(),
        status
    );
}

#[test]
fn sandbox_types_tests_error_display_strings_are_stable() {
    assert_eq!(
        SandboxAdapterError::ImageMissing {
            image_or_root: ImageRef::new("missing-root")
        }
        .to_string(),
        "sandbox image or root missing: missing-root"
    );
    assert_eq!(
        SandboxAdapterError::SpawnFailed {
            adapter_id: AdapterId::new("wsl2_podman"),
            reason: "permission denied".to_string(),
        }
        .to_string(),
        "sandbox adapter wsl2_podman failed to spawn process: permission denied"
    );
    assert_eq!(
        SandboxAdapterError::CapabilityUnsatisfied {
            required: BTreeSet::from([RequiredCapability::VendorAgnosticGpu]),
            available: BTreeSet::from([RequiredCapability::CrossMachinePortable]),
        }
        .to_string(),
        "sandbox capability unsatisfied: required=[vendor_agnostic_gpu], available=[cross_machine_portable]"
    );
}

#[test]
fn sandbox_types_tests_shared_types_do_not_import_adapter_specific_crates() {
    let types_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sandbox")
        .join("types.rs");
    let source = fs::read_to_string(types_path).expect("read sandbox shared types source");
    let lower = source.to_ascii_lowercase();

    for banned in [
        "podman::",
        "bollard::",
        "win32::",
        "windows::",
        "windows_sys::",
    ] {
        assert!(
            !lower.contains(banned),
            "sandbox shared types must not import adapter-specific crate surface `{banned}`"
        );
    }
}
