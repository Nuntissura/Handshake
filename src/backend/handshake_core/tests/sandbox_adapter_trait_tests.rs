use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use bytes::Bytes;
use handshake_core::sandbox::{
    default_no_op_capabilities, AdapterCapabilities, AdapterId, BindMode, BindSpec, Command,
    ExecResult, GpuPassthrough, ImageRef, IsolationStrength, NetPolicy, ProcessHandle, ProcessSpec,
    ProcessStatus, RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal,
    ThroughputClass, TrustClass,
};

#[derive(Debug, Clone)]
struct NoopAdapter {
    capabilities: AdapterCapabilities,
}

impl Default for NoopAdapter {
    fn default() -> Self {
        Self {
            capabilities: default_no_op_capabilities(),
        }
    }
}

impl NoopAdapter {
    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "noop adapter has no isolation backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for NoopAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn status(&self, _handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn exit_code(&self, _handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        Err(self.unavailable())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

#[test]
fn sandbox_adapter_trait_object_is_constructible_and_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<NoopAdapter>();

    let adapter: Box<dyn SandboxAdapter> = Box::new(NoopAdapter::default());
    let caps = adapter.capabilities();

    assert_eq!(caps.adapter_id, AdapterId::new("noop"));
    assert_eq!(caps.filesystem_isolation_strength, IsolationStrength::Weak);
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Weak);
}

#[tokio::test]
async fn noop_adapter_methods_fail_closed_without_backend() {
    let adapter = NoopAdapter::default();
    let handle = ProcessHandle::new(AdapterId::new("noop"), None, "noop-internal");

    let spec = ProcessSpec {
        id: AdapterId::new("noop"),
        image_or_root: ImageRef::new("noop-root"),
        cmd: vec!["noop".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: vec![BindSpec {
            host_path: PathBuf::from("fixtures/input"),
            guest_path: PathBuf::from("/guest/input"),
            mode: BindMode::ReadOnly,
        }],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        idle_timeout_ms: None,
        required_capabilities: BTreeSet::from([RequiredCapability::VeryStrongNetworkIsolation]),
        trust_class: TrustClass::default(),
        metadata: BTreeMap::new(),
    };
    let command = Command {
        argv: vec!["noop".to_string()],
        env_overlay: BTreeMap::new(),
        stdin: Some(Bytes::from_static(b"input")),
        timeout_ms: Some(1_000),
    };

    assert_adapter_unavailable(adapter.spawn(spec).await);
    assert_adapter_unavailable(adapter.exec(&handle, command).await);
    assert_adapter_unavailable(
        adapter
            .fs_bind(
                &handle,
                PathBuf::from("fixtures/model.gguf"),
                PathBuf::from("/models/model.gguf"),
                BindMode::ReadOnly,
            )
            .await,
    );
    assert_adapter_unavailable(adapter.net_policy(&handle, NetPolicy::LoopbackOnly).await);
    assert_adapter_unavailable(adapter.kill(&handle, Signal::Term).await);
    assert_adapter_unavailable(adapter.status(&handle).await);
    assert_adapter_unavailable(adapter.exit_code(&handle).await);
}

#[test]
fn adapter_capabilities_are_clonable_and_serde_round_trip() {
    let capabilities = default_no_op_capabilities();
    let cloned = capabilities.clone();

    assert_eq!(cloned, capabilities);
    assert_eq!(capabilities.gpu_passthrough, GpuPassthrough::None);
    assert_eq!(capabilities.stdio_throughput_class, ThroughputClass::Low);
    assert!(!capabilities.win32_native_fidelity);
    assert!(!capabilities.cross_machine_portable);

    let encoded = serde_json::to_string(&capabilities).expect("capabilities serialize");
    assert_eq!(
        serde_json::from_str::<AdapterCapabilities>(&encoded).expect("capabilities deserialize"),
        capabilities
    );
}

#[test]
fn sandbox_adapter_trait_source_has_exact_public_method_shape() {
    let adapter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sandbox")
        .join("adapter.rs");
    let source = fs::read_to_string(adapter_path).expect("read sandbox adapter source");
    let trait_source = source
        .split("pub trait SandboxAdapter")
        .nth(1)
        .and_then(|body| body.split("\n}\n").next())
        .expect("trait source is present");

    let declarations = trait_source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("async fn ") || line.starts_with("fn "))
        .collect::<Vec<_>>();

    // 8 core methods + the Master Spec v02.187 §3.5.7 additive methods:
    // snapshot/restore (#7) and copy_in/copy_out (#4) = 12.
    assert_eq!(
        declarations.len(),
        12,
        "SandboxAdapter must expose exactly the core 8 methods plus the §3.5.7 \
         additive methods snapshot/restore/copy_in/copy_out: {declarations:?}"
    );

    for method in [
        "spawn",
        "exec",
        "fs_bind",
        "net_policy",
        "kill",
        "status",
        "exit_code",
        "snapshot",
        "restore",
        "copy_in",
        "copy_out",
        "capabilities",
    ] {
        assert!(
            declarations
                .iter()
                .any(|line| line.starts_with(&format!("async fn {method}("))
                    || line.starts_with(&format!("fn {method}("))),
            "missing SandboxAdapter method {method}"
        );
    }
}

#[test]
fn sandbox_adapter_trait_has_no_adapter_specific_imports() {
    let adapter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sandbox")
        .join("adapter.rs");
    let source = fs::read_to_string(adapter_path).expect("read sandbox adapter source");
    let lower = source.to_ascii_lowercase();

    for banned in [
        "podman::",
        "bollard::",
        "docker::",
        "win32::",
        "windows::",
        "windows_sys::",
    ] {
        assert!(
            !lower.contains(banned),
            "sandbox adapter trait must not import adapter-specific crate surface `{banned}`"
        );
    }
}

fn assert_adapter_unavailable<T: std::fmt::Debug>(result: Result<T, SandboxAdapterError>) {
    let error = result.expect_err("noop adapter must fail closed");
    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new("noop"));
            assert!(reason.contains("noop adapter"));
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn copy_in_out_default_is_unsupported() {
    // Master Spec §3.5.7 #4: the trait exposes copy_in/copy_out; adapters with no
    // live per-file channel (the default) return a typed CopyUnsupported rather
    // than silently succeeding or panicking.
    let adapter = NoopAdapter::default();
    let handle = ProcessHandle::new(AdapterId::new("noop"), None, "noop-copy");
    let r_in = adapter
        .copy_in(&handle, PathBuf::from("/tmp/h"), PathBuf::from("/g"))
        .await;
    assert!(
        matches!(r_in, Err(SandboxAdapterError::CopyUnsupported { .. })),
        "default copy_in must be CopyUnsupported, got {r_in:?}"
    );
    let r_out = adapter
        .copy_out(&handle, PathBuf::from("/g"), PathBuf::from("/tmp/h"))
        .await;
    assert!(
        matches!(r_out, Err(SandboxAdapterError::CopyUnsupported { .. })),
        "default copy_out must be CopyUnsupported, got {r_out:?}"
    );
}
