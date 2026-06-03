use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use handshake_core::model_runtime::{
    CancellationToken, FinishReason, GenPrompt, GenerateRequest, LoadSpec, ModelCapabilities,
    ModelRuntime, ProviderKind, RuntimeKind, SamplingParams, WarmAgentFrameStream,
    WarmAgentGenerateRequest, WarmAgentGuestFrame, WarmAgentHostFrame, WarmAgentTransport,
    WarmVmModelConfig, WarmVmModelRuntime, WarmVmSnapshotManifest, WarmVmTransportError,
    WARM_AGENT_PROTOCOL_ID, WARM_AGENT_PROTOCOL_VERSION,
};
use handshake_core::sandbox::{
    AdapterId, BindMode, BindSpec, CloudHypervisorAdapter, CloudHypervisorConfig, ImageRef,
    NetPolicy, ProcessSpec, ResourceLimits, SandboxAdapter, Signal, SnapshotRef, TrustClass,
    CLOUD_HYPERVISOR_ADAPTER_ID, CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY,
    CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT, SANDBOX_MODE_METADATA_KEY, SANDBOX_MODE_PERSISTENT,
};
use sha2::{Digest, Sha256};

const REQUEST_ID_PLACEHOLDER: &str = "$REQUEST_ID";

#[derive(Default)]
struct TransportObs {
    loads: Vec<WarmAgentHostFrame>,
    generates: Vec<WarmAgentGenerateRequest>,
    cancels: Vec<String>,
}

struct ScriptedWarmTransport {
    ready: WarmAgentGuestFrame,
    scripts: Mutex<VecDeque<Vec<Result<WarmAgentGuestFrame, WarmVmTransportError>>>>,
    obs: Arc<Mutex<TransportObs>>,
}

struct PendingWarmTransport {
    ready: WarmAgentGuestFrame,
    obs: Arc<Mutex<TransportObs>>,
}

#[async_trait]
impl WarmAgentTransport for ScriptedWarmTransport {
    async fn load_model(
        &self,
        frame: WarmAgentHostFrame,
    ) -> Result<WarmAgentGuestFrame, WarmVmTransportError> {
        self.obs.lock().unwrap().loads.push(frame);
        Ok(self.ready.clone())
    }

    fn generate(&self, request: WarmAgentGenerateRequest) -> WarmAgentFrameStream {
        self.obs.lock().unwrap().generates.push(request.clone());
        let mut script = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
        for item in &mut script {
            if let Ok(frame) = item {
                patch_request_id_placeholder(frame, &request.request_id);
            }
        }
        Box::pin(stream::iter(script))
    }

    async fn cancel(&self, request_id: &str) -> Result<(), WarmVmTransportError> {
        self.obs
            .lock()
            .unwrap()
            .cancels
            .push(request_id.to_string());
        Ok(())
    }
}

#[async_trait]
impl WarmAgentTransport for PendingWarmTransport {
    async fn load_model(
        &self,
        frame: WarmAgentHostFrame,
    ) -> Result<WarmAgentGuestFrame, WarmVmTransportError> {
        self.obs.lock().unwrap().loads.push(frame);
        Ok(self.ready.clone())
    }

    fn generate(&self, request: WarmAgentGenerateRequest) -> WarmAgentFrameStream {
        self.obs.lock().unwrap().generates.push(request);
        Box::pin(stream::pending())
    }

    async fn cancel(&self, request_id: &str) -> Result<(), WarmVmTransportError> {
        self.obs
            .lock()
            .unwrap()
            .cancels
            .push(request_id.to_string());
        Ok(())
    }
}

fn ready() -> WarmAgentGuestFrame {
    WarmAgentGuestFrame::Ready {
        protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
        protocol_version: WARM_AGENT_PROTOCOL_VERSION,
        agent_id: "integration-warm-agent".to_string(),
        ready_nonce: "ready-nonce".to_string(),
        loaded_model_sha256: Some("sha-warm".to_string()),
        loaded_model_guest_path: Some("/models/model.gguf".to_string()),
    }
}

fn scripted_transport(
    scripts: Vec<Vec<Result<WarmAgentGuestFrame, WarmVmTransportError>>>,
) -> (Arc<ScriptedWarmTransport>, Arc<Mutex<TransportObs>>) {
    let obs = Arc::new(Mutex::new(TransportObs::default()));
    let transport = Arc::new(ScriptedWarmTransport {
        ready: ready(),
        scripts: Mutex::new(VecDeque::from(scripts)),
        obs: Arc::clone(&obs),
    });
    (transport, obs)
}

fn pending_transport() -> (Arc<PendingWarmTransport>, Arc<Mutex<TransportObs>>) {
    let obs = Arc::new(Mutex::new(TransportObs::default()));
    let transport = Arc::new(PendingWarmTransport {
        ready: ready(),
        obs: Arc::clone(&obs),
    });
    (transport, obs)
}

fn config() -> WarmVmModelConfig {
    WarmVmModelConfig::new("wt-207", "/models/model.gguf", "sha-warm")
}

fn load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: PathBuf::from("model.gguf"),
        sha256_expected: "sha-warm".to_string(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: Default::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: Some("warm_vm".to_string()),
        external_engine_import: None,
    }
}

fn generate_request(id: handshake_core::model_runtime::ModelId) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::new("stream from the restored guest"),
        sampling: SamplingParams::default(),
        lora_overrides: vec![],
        steering_overrides: vec![],
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 32,
        stop_sequences: vec![],
        speculative_mode: None,
        structured_decoding: None,
    }
}

fn live_required() -> bool {
    std::env::var("HANDSHAKE_CH_REQUIRE").is_ok()
}

fn live_enabled() -> bool {
    std::env::var("HANDSHAKE_CH_LIVE").ok().as_deref() == Some("1")
}

fn live_skip_or_panic(reason: impl AsRef<str>) {
    let reason = reason.as_ref();
    if live_required() || live_enabled() {
        panic!("{reason}");
    }
    eprintln!("SKIP warm_vm_runtime_live: {reason}");
}

fn live_env_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            live_skip_or_panic(format!("{name} is not set"));
            None
        }
    }
}

fn file_name_no_whitespace(path: &Path, label: &str) -> Option<String> {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        live_skip_or_panic(format!(
            "{label} must have a UTF-8 file name: {}",
            path.display()
        ));
        return None;
    };
    if name.chars().any(char::is_whitespace) {
        live_skip_or_panic(format!(
            "{label} file name must not contain whitespace: {name}"
        ));
        return None;
    }
    Some(name.to_string())
}

fn sha256_file_hex(path: &Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => Some(format!("{:x}", Sha256::digest(&bytes))),
        Err(error) => {
            live_skip_or_panic(format!(
                "could not read HANDSHAKE_SBX_GGUF for sha256 proof at {}: {error}; \
                 set HANDSHAKE_SBX_GGUF_SHA256 when using a WSL-native model path",
                path.display()
            ));
            None
        }
    }
}

fn live_model_identity() -> Option<(PathBuf, String, String)> {
    let host_path = PathBuf::from(live_env_required("HANDSHAKE_SBX_GGUF")?);
    let file_name = file_name_no_whitespace(&host_path, "HANDSHAKE_SBX_GGUF")?;
    let sha256 = match std::env::var("HANDSHAKE_SBX_GGUF_SHA256") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => sha256_file_hex(&host_path)?,
    };
    Some((host_path, file_name, sha256))
}

fn live_warm_agent_root() -> Option<PathBuf> {
    let agent_path = PathBuf::from(live_env_required("HANDSHAKE_CH_WARM_AGENT_HOST_PATH")?);
    file_name_no_whitespace(&agent_path, "HANDSHAKE_CH_WARM_AGENT_HOST_PATH")?;
    agent_path.parent().map(Path::to_path_buf).or_else(|| {
        live_skip_or_panic(format!(
            "HANDSHAKE_CH_WARM_AGENT_HOST_PATH must have a parent package directory: {}",
            agent_path.display()
        ));
        None
    })
}

fn live_process_spec(
    warm_agent_root: PathBuf,
    model_host_path: PathBuf,
    model_guest_path: &str,
) -> ProcessSpec {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        SANDBOX_MODE_METADATA_KEY.to_string(),
        SANDBOX_MODE_PERSISTENT.to_string(),
    );
    metadata.insert(
        CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY.to_string(),
        format!("{CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT}/hsk-warm-agent"),
    );
    let model_guest_dir = Path::new(model_guest_path)
        .parent()
        .unwrap_or_else(|| Path::new("/models"))
        .to_path_buf();
    let model_host_dir = model_host_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| model_host_path.clone());

    ProcessSpec {
        id: AdapterId::new("warm-vm-runtime-live"),
        image_or_root: ImageRef::new("llama_cpp"),
        cmd: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        binds: vec![
            BindSpec {
                host_path: warm_agent_root,
                guest_path: PathBuf::from(CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT),
                mode: BindMode::ReadOnly,
            },
            BindSpec {
                host_path: model_host_dir,
                guest_path: model_guest_dir,
                mode: BindMode::ReadOnly,
            },
        ],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        idle_timeout_ms: Some(120_000),
        required_capabilities: Default::default(),
        trust_class: TrustClass::UntrustedAgent,
        metadata,
    }
}

async fn assert_stream_emits_text(runtime: &WarmVmModelRuntime, label: &str) -> Result<(), String> {
    let mut request = generate_request(runtime.model_id());
    request.prompt = GenPrompt::new(format!("{label}: say a short warm VM test phrase"));
    request.max_tokens = 24;
    let mut stream = runtime.generate(request);
    let mut text = String::new();
    let mut saw_terminal = false;
    let frame_timeout_secs = std::env::var("HANDSHAKE_WARM_STREAM_FRAME_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(45);
    while let Some(item) = tokio::time::timeout(
        std::time::Duration::from_secs(frame_timeout_secs),
        stream.next(),
    )
    .await
    .map_err(|_| format!("{label}: timed out waiting for warm VM stream frame"))?
    {
        let token = item.map_err(|error| format!("{label}: warm VM stream error: {error}"))?;
        if token.finish_reason.is_some() {
            saw_terminal = true;
            break;
        }
        text.push_str(&token.text);
    }
    if text.trim().is_empty() {
        return Err(format!(
            "{label}: real warm VM stream must emit token text before terminal"
        ));
    }
    if !saw_terminal {
        return Err(format!("{label}: real warm VM stream must terminate"));
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires operator WSL2/KVM/Cloud-Hypervisor, a runtime-probed hsk-warm-agent package, and a real GGUF model"]
async fn warm_vm_runtime_live_model_streams_from_restored_snapshot() {
    if !live_enabled() {
        live_skip_or_panic("HANDSHAKE_CH_LIVE must be 1 for the live warm VM model proof");
        return;
    }

    let Some(warm_agent_host_path) =
        live_env_required("HANDSHAKE_CH_WARM_AGENT_HOST_PATH").map(PathBuf::from)
    else {
        return;
    };
    let Some(warm_agent_root) = live_warm_agent_root() else {
        return;
    };
    let Some((model_host_path, model_file_name, model_sha256)) = live_model_identity() else {
        return;
    };
    let model_guest_path = format!("/models/{model_file_name}");
    let config = CloudHypervisorConfig::default().with_warm_agent_host_path(warm_agent_host_path);
    let adapter = match CloudHypervisorAdapter::try_new(config).await {
        Ok(adapter) => adapter,
        Err(error) => {
            live_skip_or_panic(format!("Cloud Hypervisor adapter unavailable: {error}"));
            return;
        }
    };
    let caps = adapter.capabilities();
    assert_eq!(caps.adapter_id, AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID));
    assert!(
        caps.supports_snapshot,
        "live proof requires CH snapshot support"
    );
    assert!(
        caps.supports_warm_agent && caps.supports_live_token_stream,
        "live proof requires runtime-probed warm-agent support; status={:?}",
        adapter.warm_agent_status()
    );

    let handle = adapter
        .spawn(live_process_spec(
            warm_agent_root,
            model_host_path.clone(),
            &model_guest_path,
        ))
        .await
        .expect("spawn live persistent warm-agent VM");
    let mut restored_handle: Option<handshake_core::sandbox::ProcessHandle> = None;
    let mut snapshot_to_delete: Option<SnapshotRef> = None;

    let result: Result<(), String> = async {
        let transport = adapter
            .warm_agent_transport(&handle)
            .await
            .map_err(|error| format!("warm-agent transport before snapshot: {error}"))?;
        let transport_probe = Arc::clone(&transport);
        let warm_cfg = WarmVmModelConfig::new(
            "warm-vm-runtime-live",
            model_guest_path.clone(),
            model_sha256.clone(),
        );
        let mut runtime = WarmVmModelRuntime::new(transport, warm_cfg.clone());
        let loaded_model_id = runtime
            .load(LoadSpec {
                artifact_path: model_host_path.clone(),
                sha256_expected: model_sha256.clone(),
                runtime_kind: RuntimeKind::LlamaCpp,
                sampling_defaults: SamplingParams::default(),
                kv_cache_policy: Default::default(),
                declared_capabilities: ModelCapabilities::default(),
                provider: ProviderKind::Local,
                engine_origin: Some("warm_vm_live".to_string()),
                external_engine_import: None,
            })
            .await
            .map_err(|error| format!("live warm load: {error}"))?;
        assert_eq!(loaded_model_id, runtime.model_id());
        let reload_probe = transport_probe
            .load_model(WarmAgentHostFrame::Load {
                request_id: "live-reload-probe".to_string(),
                model_guest_path: model_guest_path.clone(),
                model_artifact_sha256: model_sha256.clone(),
            })
            .await
            .map_err(|error| format!("warm reload probe after initial load: {error}"))?;
        match reload_probe {
            WarmAgentGuestFrame::Ready {
                loaded_model_sha256,
                loaded_model_guest_path,
                ..
            } => {
                if loaded_model_sha256.as_deref() != Some(model_sha256.as_str()) {
                    return Err(format!(
                        "warm reload probe sha mismatch: {loaded_model_sha256:?}"
                    ));
                }
                if loaded_model_guest_path.as_deref() != Some(model_guest_path.as_str()) {
                    return Err(format!(
                        "warm reload probe model path mismatch: {loaded_model_guest_path:?}"
                    ));
                }
            }
            other => return Err(format!("warm reload probe expected ready, got {other:?}")),
        }
        assert_stream_emits_text(&runtime, "before snapshot").await?;

        let snapshot = adapter
            .snapshot(&handle)
            .await
            .map_err(|error| format!("snapshot model-loaded warm VM: {error}"))?;
        snapshot_to_delete = Some(snapshot.clone());
        let manifest = runtime
            .warm_snapshot_manifest(snapshot.clone())
            .map_err(|error| format!("warm snapshot manifest: {error}"))?;

        let restored = adapter
            .restore(&manifest.snapshot)
            .await
            .map_err(|error| format!("restore model-loaded warm VM snapshot: {error}"))?;
        restored_handle = Some(restored.clone());
        let restored_transport = adapter
            .warm_agent_transport(&restored)
            .await
            .map_err(|error| format!("warm-agent transport after restore: {error}"))?;
        let restored_runtime =
            WarmVmModelRuntime::from_restored_manifest(restored_transport, warm_cfg, &manifest)
                .map_err(|error| format!("restored warm runtime from manifest: {error}"))?;
        assert_stream_emits_text(&restored_runtime, "after restore").await?;
        Ok(())
    }
    .await;

    let mut failures = Vec::new();
    if let Err(error) = result {
        failures.push(error);
        if let Some(serial_log) = adapter.read_handle_serial(&handle).await {
            failures.push(format!(
                "source warm VM console tail:\n{}",
                tail_chars(&serial_log, 4096)
            ));
        }
    }
    if let Some(restored) = restored_handle {
        if let Err(error) = adapter.kill(&restored, Signal::Kill).await {
            failures.push(format!("kill restored warm VM: {error}"));
        }
    }
    if let Err(error) = adapter.kill(&handle, Signal::Kill).await {
        failures.push(format!("kill source warm VM: {error}"));
    }
    if let Some(snapshot) = snapshot_to_delete {
        if let Err(error) = adapter.delete_snapshot(&snapshot).await {
            failures.push(format!("delete warm VM snapshot: {error}"));
        }
    }
    assert!(
        failures.is_empty(),
        "warm_vm_runtime_live proof failed:\n{}",
        failures.join("\n")
    );
}

fn tail_chars(value: &str, max_chars: usize) -> String {
    let mut chars: Vec<char> = value.chars().rev().take(max_chars).collect();
    chars.reverse();
    chars.into_iter().collect()
}

fn patch_request_id_placeholder(frame: &mut WarmAgentGuestFrame, request_id: &str) {
    match frame {
        WarmAgentGuestFrame::Token { request_id: id, .. }
        | WarmAgentGuestFrame::Complete { request_id: id, .. } => {
            if id == REQUEST_ID_PLACEHOLDER {
                *id = request_id.to_string();
            }
        }
        WarmAgentGuestFrame::Error {
            request_id: Some(id),
            ..
        }
        | WarmAgentGuestFrame::Heartbeat {
            request_id: Some(id),
        } => {
            if id == REQUEST_ID_PLACEHOLDER {
                *id = request_id.to_string();
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn restored_warm_runtime_streams_guest_frames_without_model_reload() {
    let snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap/warm");
    let manifest = WarmVmSnapshotManifest::new(
        "wt-207",
        "sha-warm",
        "/models/model.gguf",
        "ready-nonce",
        snapshot,
    );
    let (transport, obs) = scripted_transport(vec![vec![
        Ok(WarmAgentGuestFrame::Token {
            request_id: REQUEST_ID_PLACEHOLDER.to_string(),
            token_id: 11,
            token_index: Some(1),
            text: "first".to_string(),
        }),
        Ok(WarmAgentGuestFrame::Token {
            request_id: REQUEST_ID_PLACEHOLDER.to_string(),
            token_id: 12,
            token_index: Some(2),
            text: " second".to_string(),
        }),
        Ok(WarmAgentGuestFrame::Complete {
            request_id: REQUEST_ID_PLACEHOLDER.to_string(),
            finish_reason: "stop".to_string(),
        }),
    ]]);

    let runtime = WarmVmModelRuntime::from_restored_manifest(transport, config(), &manifest)
        .expect("restored manifest is valid");
    let mut stream = runtime.generate(generate_request(runtime.model_id()));

    let first = stream.next().await.expect("first frame").expect("token");
    assert_eq!(first.text, "first");
    assert_eq!(first.finish_reason, None);
    let second = stream.next().await.expect("second frame").expect("token");
    assert_eq!(second.text, " second");
    let terminal = stream.next().await.expect("terminal").expect("terminal");
    assert_eq!(terminal.finish_reason, Some(FinishReason::Stop));

    let obs = obs.lock().unwrap();
    assert!(
        obs.loads.is_empty(),
        "restored warm path must not cold-load"
    );
    assert_eq!(obs.generates.len(), 1, "generate must use guest transport");
}

#[tokio::test]
async fn restored_warm_runtime_reuses_same_guest_channel_for_multiple_generates_without_reload() {
    let snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap/warm");
    let manifest = WarmVmSnapshotManifest::new(
        "wt-207",
        "sha-warm",
        "/models/model.gguf",
        "ready-nonce",
        snapshot,
    );
    let (transport, obs) = scripted_transport(vec![
        vec![
            Ok(WarmAgentGuestFrame::Token {
                request_id: REQUEST_ID_PLACEHOLDER.to_string(),
                token_id: 31,
                token_index: Some(1),
                text: "first-run".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Complete {
                request_id: REQUEST_ID_PLACEHOLDER.to_string(),
                finish_reason: "stop".to_string(),
            }),
        ],
        vec![
            Ok(WarmAgentGuestFrame::Token {
                request_id: REQUEST_ID_PLACEHOLDER.to_string(),
                token_id: 41,
                token_index: Some(1),
                text: "second-run".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Complete {
                request_id: REQUEST_ID_PLACEHOLDER.to_string(),
                finish_reason: "stop".to_string(),
            }),
        ],
    ]);

    let runtime = WarmVmModelRuntime::from_restored_manifest(transport, config(), &manifest)
        .expect("restored manifest is valid");

    let mut first_stream = runtime.generate(generate_request(runtime.model_id()));
    let first = first_stream
        .next()
        .await
        .expect("first run token")
        .expect("first run accepted");
    assert_eq!(first.text, "first-run");
    first_stream
        .next()
        .await
        .expect("first run terminal")
        .expect("first run completed");

    let mut second_stream = runtime.generate(generate_request(runtime.model_id()));
    let second = second_stream
        .next()
        .await
        .expect("second run token")
        .expect("second run accepted");
    assert_eq!(second.text, "second-run");
    second_stream
        .next()
        .await
        .expect("second run terminal")
        .expect("second run completed");

    let obs = obs.lock().unwrap();
    assert!(obs.loads.is_empty(), "restored runtime must not reload");
    assert_eq!(
        obs.generates.len(),
        2,
        "same runtime should handle two turns"
    );
    assert_ne!(
        obs.generates[0].request_id, obs.generates[1].request_id,
        "each warm request keeps a distinct guest-frame correlation id"
    );
}

#[tokio::test]
async fn warm_stream_cancel_interrupts_pending_guest_frame_wait() {
    let (transport, obs) = pending_transport();
    let mut runtime = WarmVmModelRuntime::new(transport, config());
    let id = runtime.load(load_spec()).await.expect("warm load");
    let request = generate_request(id);
    let cancel = request.cancel.clone();
    let mut stream = runtime.generate(request);
    let request_id = obs.lock().unwrap().generates[0].request_id.clone();

    let join = tokio::spawn(async move { stream.next().await });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    cancel.cancel();
    let terminal = tokio::time::timeout(std::time::Duration::from_secs(1), join)
        .await
        .expect("pending wait should observe cancellation")
        .expect("stream task should join")
        .expect("stream should yield cancellation")
        .expect("cancel terminal is not an error");

    assert_eq!(terminal.finish_reason, Some(FinishReason::Cancelled));
    assert_eq!(obs.lock().unwrap().cancels, vec![request_id]);
}

#[tokio::test]
async fn warm_stream_fails_closed_on_non_advancing_token_indexes() {
    let (transport, _obs) = scripted_transport(vec![vec![
        Ok(WarmAgentGuestFrame::Token {
            request_id: REQUEST_ID_PLACEHOLDER.to_string(),
            token_id: 21,
            token_index: Some(1),
            text: "ok".to_string(),
        }),
        Ok(WarmAgentGuestFrame::Token {
            request_id: REQUEST_ID_PLACEHOLDER.to_string(),
            token_id: 22,
            token_index: Some(1),
            text: "duplicate".to_string(),
        }),
    ]]);
    let mut runtime = WarmVmModelRuntime::new(transport, config());
    let id = runtime.load(load_spec()).await.expect("warm load");
    let mut stream = runtime.generate(generate_request(id));

    stream
        .next()
        .await
        .expect("first token")
        .expect("first token accepted");
    let err = stream
        .next()
        .await
        .expect("second item")
        .expect_err("duplicate token index must fail closed");
    assert!(err.to_string().contains("token index did not advance"));
}

#[tokio::test]
async fn warm_stream_rejects_guest_frame_with_wrong_request_id() {
    let (transport, _obs) = scripted_transport(vec![vec![Ok(WarmAgentGuestFrame::Token {
        request_id: "other-request".to_string(),
        token_id: 51,
        token_index: Some(1),
        text: "wrong".to_string(),
    })]]);
    let mut runtime = WarmVmModelRuntime::new(transport, config());
    let id = runtime.load(load_spec()).await.expect("warm load");
    let mut stream = runtime.generate(generate_request(id));

    let err = stream
        .next()
        .await
        .expect("mismatched frame")
        .expect_err("wrong request id must fail closed");
    assert!(
        err.to_string().contains("did not match active request"),
        "unexpected error: {err}"
    );
}

#[test]
fn restored_manifest_rejects_stale_model_identity_before_transport_use() {
    let stale = WarmVmSnapshotManifest::new(
        "wt-207",
        "sha-old",
        "/models/model.gguf",
        "ready-nonce",
        SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap/stale"),
    );
    let (transport, obs) = scripted_transport(vec![]);

    let err = WarmVmModelRuntime::from_restored_manifest(transport, config(), &stale)
        .expect_err("stale model hash must reject before restore is used");

    assert!(err.to_string().contains("model hash mismatch"));
    assert!(obs.lock().unwrap().loads.is_empty());
    assert!(obs.lock().unwrap().generates.is_empty());
}
