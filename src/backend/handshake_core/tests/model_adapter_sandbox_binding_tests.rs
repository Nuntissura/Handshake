use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::{
    kernel::{ContextBundle, KernelActor, ModelAdapter, ModelAdapterRequest},
    model_runtime::sandbox_binding::{
        box_model_process, EngineKind, ModelBoxingError, ModelProcessSpec,
        SandboxRoutedModelAdapter,
    },
    sandbox::{
        AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough, ImageRef,
        IsolationStrength, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
        RequiredCapability, SandboxAdapter, SandboxAdapterError, SandboxAdapterRegistry,
        SandboxSelectionFailure, Signal, ThroughputClass, WindowsNativeJailAdapter,
    },
};
use serde_json::json;
use tempfile::tempdir;

#[derive(Debug, Clone)]
struct RecordingAdapter {
    capabilities: AdapterCapabilities,
    spawned: Arc<Mutex<Vec<ProcessSpec>>>,
}

impl RecordingAdapter {
    fn new(capabilities: AdapterCapabilities) -> Self {
        Self {
            capabilities,
            spawned: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn spawned(&self) -> Vec<ProcessSpec> {
        self.spawned.lock().expect("spawn log").clone()
    }

    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "recording adapter has no runtime backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for RecordingAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        let adapter_id = self.capabilities.adapter_id.clone();
        self.spawned.lock().expect("spawn log").push(spec);
        Ok(ProcessHandle::new(
            adapter_id,
            Some(1234),
            "recording-spawn",
        ))
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

#[tokio::test]
async fn missing_gguf_path_fails_before_adapter_spawn() {
    let tmp = tempdir().expect("tempdir");
    let tokenizer_cache = tmp.path().join("tokenizers");
    fs::create_dir_all(&tokenizer_cache).expect("tokenizer cache");
    let adapter = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(Arc::new(adapter.clone()));

    let error = box_model_process(
        &registry,
        model_spec(
            tmp.path().join("missing.gguf"),
            tmp.path().to_path_buf(),
            tokenizer_cache,
            BTreeSet::new(),
            None,
        ),
    )
    .await
    .expect_err("missing gguf should fail before spawn");

    assert!(matches!(error, ModelBoxingError::ModelGgufNotFound { .. }));
    assert!(adapter.spawned().is_empty());
}

#[tokio::test]
async fn work_profile_override_selection_failure_propagates_without_fallback() {
    let fixture = ModelFixture::new();
    let low_stdio = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::Medium));
    let fallback_high_stdio =
        RecordingAdapter::new(capabilities("local_process", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("local_process"));
    registry.register(Arc::new(low_stdio.clone()));
    registry.register(Arc::new(fallback_high_stdio.clone()));

    let error = box_model_process(
        &registry,
        model_spec(
            fixture.gguf_path.clone(),
            fixture.gguf_root.clone(),
            fixture.tokenizer_cache.clone(),
            BTreeSet::from([RequiredCapability::HighStdioThroughput]),
            Some(AdapterId::new("wsl2_podman")),
        ),
    )
    .await
    .expect_err("override should fail capability check");

    assert!(matches!(
        error,
        ModelBoxingError::SandboxSelection(
            SandboxSelectionFailure::OverrideCapabilityMismatch { .. }
        )
    ));
    assert!(low_stdio.spawned().is_empty());
    assert!(fallback_high_stdio.spawned().is_empty());
}

#[tokio::test]
async fn win32_native_fidelity_does_not_fallback_when_windows_jail_unavailable() {
    let fixture = ModelFixture::new();
    let wsl2 = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(Arc::new(wsl2.clone()));
    registry.register(Arc::new(
        WindowsNativeJailAdapter::unavailable_for_current_host(),
    ));

    let error = box_model_process(
        &registry,
        model_spec(
            fixture.gguf_path.clone(),
            fixture.gguf_root.clone(),
            fixture.tokenizer_cache.clone(),
            BTreeSet::from([RequiredCapability::Win32NativeFidelity]),
            None,
        ),
    )
    .await
    .expect_err("unavailable Windows native jail must fail before spawn");

    assert!(matches!(
        error,
        ModelBoxingError::SandboxSelection(SandboxSelectionFailure::CapabilityUnsatisfied { .. })
    ));
    assert!(wsl2.spawned().is_empty());
}

#[tokio::test]
async fn successful_boxing_spawns_selected_adapter_with_read_only_model_binds() {
    let fixture = ModelFixture::new();
    let adapter = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(Arc::new(adapter.clone()));

    let handle = box_model_process(
        &registry,
        model_spec(
            fixture.gguf_path.clone(),
            fixture.gguf_root.clone(),
            fixture.tokenizer_cache.clone(),
            BTreeSet::new(),
            None,
        ),
    )
    .await
    .expect("box model process");

    assert_eq!(handle.adapter_id, AdapterId::new("wsl2_podman"));
    let spawned = adapter.spawned();
    assert_eq!(spawned.len(), 1);
    let spec = &spawned[0];
    assert_eq!(spec.image_or_root, ImageRef::new("llama_cpp"));
    assert_eq!(spec.net_policy, NetPolicy::DenyAll);
    assert_eq!(spec.binds.len(), 2);
    assert!(spec
        .binds
        .iter()
        .all(|bind| bind.mode == BindMode::ReadOnly));
    assert_eq!(spec.binds[0].guest_path, PathBuf::from("/models/gguf"));
    assert_eq!(
        spec.binds[1].guest_path,
        PathBuf::from("/models/tokenizers")
    );
    assert!(spec.cmd.iter().any(|part| part.ends_with("model.gguf")));
    assert_eq!(
        spec.metadata.get("model_id").map(String::as_str),
        Some("test-model")
    );
    assert_eq!(
        spec.metadata.get("engine_kind").map(String::as_str),
        Some("llama_cpp")
    );
}

#[tokio::test]
async fn model_adapter_invoke_routes_spawn_through_sandbox_adapter() {
    let fixture = ModelFixture::new();
    let adapter = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(Arc::new(adapter.clone()));
    let routed = SandboxRoutedModelAdapter::new(
        "sandbox-model-adapter",
        Arc::new(registry),
        model_spec(
            fixture.gguf_path.clone(),
            fixture.gguf_root.clone(),
            fixture.tokenizer_cache.clone(),
            BTreeSet::new(),
            None,
        ),
    );
    let model_adapter: &dyn ModelAdapter = &routed;
    let context_bundle = ContextBundle::new(
        "KTR-MT-049",
        "SR-MT-049",
        json!({"prompt": "record sandbox process metadata"}),
    )
    .expect("context bundle");

    let output = model_adapter
        .invoke(ModelAdapterRequest::new(
            context_bundle,
            KernelActor::ModelAdapter("test-harness".to_string()),
        ))
        .await
        .expect("sandbox routed invoke");

    assert_eq!(output.adapter_id, "sandbox-model-adapter");
    assert_eq!(
        output.response_event_type.as_str(),
        "MODEL_RESPONSE_RECORDED"
    );
    assert_eq!(
        output.artifact_payload["sandbox_adapter_id"].as_str(),
        Some("wsl2_podman")
    );
    assert_eq!(output.artifact_payload["pid"].as_u64(), Some(1234));
    assert_eq!(
        output.artifact_payload["sandbox_internal_id"].as_str(),
        Some("recording-spawn")
    );
    assert_eq!(
        output.artifact_payload["model_id"].as_str(),
        Some("test-model")
    );
    assert!(output
        .artifact_payload
        .get("process_uuid")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(adapter.spawned().len(), 1);
}

#[test]
fn model_spawn_surfaces_do_not_use_bare_process_command() {
    let repo = repo_root();
    let guarded_files = [
        repo.join("src/backend/handshake_core/src/kernel/model_adapter.rs"),
        repo.join("src/backend/handshake_core/src/model_runtime/sandbox_binding.rs"),
    ];

    for path in guarded_files {
        if !path.exists() {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read guarded source");
        for banned in [
            "Command::new",
            "std::process::Command",
            "tokio::process::Command",
        ] {
            assert!(
                !source.contains(banned),
                "{} must not use bare process spawning token {banned}",
                path.display()
            );
        }
    }
}

fn model_spec(
    gguf_path: PathBuf,
    gguf_root_bind: PathBuf,
    tokenizer_cache_bind: PathBuf,
    required_capabilities: BTreeSet<RequiredCapability>,
    work_profile_override: Option<AdapterId>,
) -> ModelProcessSpec {
    ModelProcessSpec {
        model_id: "test-model".to_string(),
        gguf_path,
        engine_kind: EngineKind::LlamaCpp,
        work_profile_override,
        required_capabilities,
        env: BTreeMap::from([("RUST_LOG".to_string(), "info".to_string())]),
        gguf_root_bind,
        tokenizer_cache_bind,
    }
}

fn capabilities(adapter_id: &str, stdio: ThroughputClass) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(adapter_id),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: stdio,
        win32_native_fidelity: false,
        cross_machine_portable: true,
    }
}

struct ModelFixture {
    _tmp: tempfile::TempDir,
    gguf_root: PathBuf,
    tokenizer_cache: PathBuf,
    gguf_path: PathBuf,
}

impl ModelFixture {
    fn new() -> Self {
        let tmp = tempdir().expect("tempdir");
        let gguf_root = tmp.path().join("models");
        let tokenizer_cache = tmp.path().join("tokenizers");
        fs::create_dir_all(&gguf_root).expect("gguf root");
        fs::create_dir_all(&tokenizer_cache).expect("tokenizer cache");
        let gguf_path = gguf_root.join("model.gguf");
        fs::write(&gguf_path, b"fake gguf").expect("gguf file");
        Self {
            _tmp: tmp,
            gguf_root,
            tokenizer_cache,
            gguf_path,
        }
    }
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}
