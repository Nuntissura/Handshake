//! MT-207 warm-VM model runtime.
//!
//! This runtime is intentionally separate from [`crate::model_runtime::SandboxModelRuntime`].
//! The sandbox runtime is the honest cold path: one completed sandbox `exec`
//! per generate call, then post-hoc chunking. This module models the warm path:
//! a resident guest agent sends protocol frames while generation is still
//! running, and host code converts those frames into the public [`TokenStream`].

use std::{
    collections::BTreeSet,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use uuid::Uuid;

use super::{
    error::ModelRuntimeError, validate_ready_frame, warm_agent_guest_frame_type, CancellationToken,
    Embedding, FinishReason, GenerateRequest, GeneratedToken, KvCacheHandle, LoadSpec,
    LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, Score, SteeringHookHandle,
    TokenStream, WarmAgentGenerateRequest, WarmAgentGuestFrame, WarmAgentHostFrame,
    WarmAgentProtocolError, WarmVmSnapshotManifest,
};

pub const WARM_VM_RUNTIME_ADAPTER: &str = "warm_vm_model_runtime";
const WARM_AGENT_FRAME_POLL_MS: u64 = 50;

pub type WarmAgentFrameStream =
    Pin<Box<dyn Stream<Item = Result<WarmAgentGuestFrame, WarmVmTransportError>> + Send>>;

#[derive(Debug, thiserror::Error)]
pub enum WarmVmTransportError {
    #[error("warm-agent transport failed: {0}")]
    Transport(String),
    #[error("warm-agent protocol failed: {0}")]
    Protocol(#[from] WarmAgentProtocolError),
}

#[async_trait]
pub trait WarmAgentTransport: Send + Sync {
    async fn load_model(
        &self,
        frame: WarmAgentHostFrame,
    ) -> Result<WarmAgentGuestFrame, WarmVmTransportError>;

    fn generate(&self, request: WarmAgentGenerateRequest) -> WarmAgentFrameStream;

    async fn cancel(&self, request_id: &str) -> Result<(), WarmVmTransportError>;

    async fn shutdown(&self) -> Result<(), WarmVmTransportError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarmVmModelConfig {
    pub worktree_id: String,
    pub model_guest_path: String,
    pub model_artifact_sha256: String,
}

impl WarmVmModelConfig {
    pub fn new(
        worktree_id: impl Into<String>,
        model_guest_path: impl Into<String>,
        model_artifact_sha256: impl Into<String>,
    ) -> Self {
        Self {
            worktree_id: worktree_id.into(),
            model_guest_path: model_guest_path.into(),
            model_artifact_sha256: model_artifact_sha256.into(),
        }
    }
}

struct WarmStreamState {
    request_id: String,
    frames: WarmAgentFrameStream,
    transport: Arc<dyn WarmAgentTransport>,
    req_cancel: CancellationToken,
    runtime_cancel: CancellationToken,
    last_token_index: Option<u32>,
    cancel_sent: bool,
    done: bool,
    active_requests: Arc<Mutex<BTreeSet<String>>>,
}

impl Drop for WarmStreamState {
    fn drop(&mut self) {
        remove_active(&self.active_requests, &self.request_id);
    }
}

pub struct WarmVmModelRuntime {
    transport: Arc<dyn WarmAgentTransport>,
    cfg: WarmVmModelConfig,
    model_id: ModelId,
    declared_capabilities: ModelCapabilities,
    ready: Arc<Mutex<Option<WarmAgentGuestFrame>>>,
    active_requests: Arc<Mutex<BTreeSet<String>>>,
    runtime_cancel: CancellationToken,
}

impl std::fmt::Debug for WarmVmModelRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarmVmModelRuntime")
            .field("cfg", &self.cfg)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl WarmVmModelRuntime {
    pub fn new(transport: Arc<dyn WarmAgentTransport>, cfg: WarmVmModelConfig) -> Self {
        Self {
            transport,
            cfg,
            model_id: ModelId::new_v7(),
            declared_capabilities: ModelCapabilities::default(),
            ready: Arc::new(Mutex::new(None)),
            active_requests: Arc::new(Mutex::new(BTreeSet::new())),
            runtime_cancel: CancellationToken::new(),
        }
    }

    pub fn from_restored_manifest(
        transport: Arc<dyn WarmAgentTransport>,
        cfg: WarmVmModelConfig,
        manifest: &WarmVmSnapshotManifest,
    ) -> Result<Self, ModelRuntimeError> {
        if manifest.worktree_id != cfg.worktree_id {
            return Err(ModelRuntimeError::LoadError(format!(
                "warm snapshot worktree mismatch: expected {}, got {}",
                cfg.worktree_id, manifest.worktree_id
            )));
        }
        manifest
            .validate_for_restore(&cfg.model_artifact_sha256, &cfg.model_guest_path)
            .map_err(load_protocol_error)?;
        let runtime = Self::new(transport, cfg);
        runtime.store_ready(WarmAgentGuestFrame::Ready {
            protocol_id: manifest.protocol_id.clone(),
            protocol_version: manifest.protocol_version,
            agent_id: "restored-warm-agent".to_string(),
            ready_nonce: manifest.ready_nonce.clone(),
            loaded_model_sha256: Some(manifest.model_artifact_sha256.clone()),
            loaded_model_guest_path: Some(manifest.model_guest_path.clone()),
        })?;
        Ok(runtime)
    }

    pub fn model_id(&self) -> ModelId {
        self.model_id
    }

    pub fn config(&self) -> &WarmVmModelConfig {
        &self.cfg
    }

    fn store_ready(&self, ready: WarmAgentGuestFrame) -> Result<(), ModelRuntimeError> {
        validate_ready_frame(&ready).map_err(load_protocol_error)?;
        let (loaded_sha, loaded_path) = match &ready {
            WarmAgentGuestFrame::Ready {
                loaded_model_sha256,
                loaded_model_guest_path,
                ..
            } => (
                loaded_model_sha256.as_deref(),
                loaded_model_guest_path.as_deref(),
            ),
            _ => unreachable!("validate_ready_frame rejects non-ready frames"),
        };
        if loaded_sha != Some(self.cfg.model_artifact_sha256.as_str()) {
            return Err(load_protocol_error(
                WarmAgentProtocolError::ModelHashMismatch {
                    expected: self.cfg.model_artifact_sha256.clone(),
                    actual: loaded_sha.unwrap_or("<missing>").to_string(),
                },
            ));
        }
        if loaded_path != Some(self.cfg.model_guest_path.as_str()) {
            return Err(load_protocol_error(
                WarmAgentProtocolError::ModelGuestPathMismatch {
                    expected: self.cfg.model_guest_path.clone(),
                    actual: loaded_path.unwrap_or("<missing>").to_string(),
                },
            ));
        }
        *self.ready.lock().map_err(|error| {
            ModelRuntimeError::LoadError(format!("warm ready lock poisoned: {error}"))
        })? = Some(ready);
        Ok(())
    }

    fn loaded(&self) -> bool {
        self.ready
            .lock()
            .map(|ready| ready.is_some())
            .unwrap_or(false)
    }

    fn streaming_generate(&self, req: GenerateRequest) -> TokenStream {
        if !self.loaded() {
            return single_error_stream(ModelRuntimeError::GenerateError(
                "warm VM generate before load or restored manifest".to_string(),
            ));
        }
        let request_id = Uuid::now_v7().simple().to_string();
        let request = WarmAgentGenerateRequest {
            request_id: request_id.clone(),
            model_id: req.id.to_string(),
            model_guest_path: self.cfg.model_guest_path.clone(),
            model_artifact_sha256: self.cfg.model_artifact_sha256.clone(),
            prompt: req.prompt.text.clone(),
            max_tokens: req.max_tokens,
        };
        if let Ok(mut active) = self.active_requests.lock() {
            active.insert(request_id.clone());
        }
        let frames = self.transport.generate(request);
        Box::pin(stream::unfold(
            WarmStreamState {
                request_id,
                frames,
                transport: Arc::clone(&self.transport),
                req_cancel: req.cancel,
                runtime_cancel: self.runtime_cancel.clone(),
                last_token_index: None,
                cancel_sent: false,
                done: false,
                active_requests: Arc::clone(&self.active_requests),
            },
            next_warm_stream_item,
        ))
    }
}

#[async_trait]
impl ModelRuntime for WarmVmModelRuntime {
    fn adapter_name(&self) -> &'static str {
        WARM_VM_RUNTIME_ADAPTER
    }

    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        if !spec.sha256_expected.is_empty()
            && spec.sha256_expected != self.cfg.model_artifact_sha256
        {
            return Err(ModelRuntimeError::LoadError(format!(
                "warm VM load sha mismatch: expected {}, configured {}",
                spec.sha256_expected, self.cfg.model_artifact_sha256
            )));
        }
        let frame = WarmAgentHostFrame::Load {
            request_id: Uuid::now_v7().simple().to_string(),
            model_guest_path: self.cfg.model_guest_path.clone(),
            model_artifact_sha256: self.cfg.model_artifact_sha256.clone(),
        };
        let ready = self
            .transport
            .load_model(frame)
            .await
            .map_err(|error| ModelRuntimeError::LoadError(error.to_string()))?;
        self.store_ready(ready)?;
        Ok(self.model_id)
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.transport
            .shutdown()
            .await
            .map_err(|error| ModelRuntimeError::UnloadError(error.to_string()))?;
        if let Ok(mut ready) = self.ready.lock() {
            *ready = None;
        }
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.streaming_generate(req)
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "score (warm VM agent streams text frames only)".to_string(),
            adapter: WARM_VM_RUNTIME_ADAPTER.to_string(),
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "embed (warm VM agent has no embedding verb)".to_string(),
            adapter: WARM_VM_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.declared_capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache (warm VM snapshot owns opaque guest state)".to_string(),
            adapter: WARM_VM_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "lora_stack (warm VM agent has no LoRA verb)".to_string(),
            adapter: WARM_VM_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "steering_hooks (warm VM agent exposes no residual stream)".to_string(),
            adapter: WARM_VM_RUNTIME_ADAPTER.to_string(),
        })
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.runtime_cancel.cancel();
        let active = self
            .active_requests
            .lock()
            .map(|active| active.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        if active.is_empty() {
            return;
        }
        let transport = Arc::clone(&self.transport);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                for request_id in active {
                    let _ = transport.cancel(&request_id).await;
                }
            });
        }
    }
}

async fn next_warm_stream_item(
    mut state: WarmStreamState,
) -> Option<(Result<GeneratedToken, ModelRuntimeError>, WarmStreamState)> {
    loop {
        if state.done {
            return None;
        }
        if state.req_cancel.is_cancelled() || state.runtime_cancel.is_cancelled() {
            if !state.cancel_sent {
                state.cancel_sent = true;
                let _ = state.transport.cancel(&state.request_id).await;
                remove_active(&state.active_requests, &state.request_id);
            }
            state.done = true;
            return Some((Ok(terminal_token(FinishReason::Cancelled)), state));
        }

        let next_frame = match tokio::time::timeout(
            Duration::from_millis(WARM_AGENT_FRAME_POLL_MS),
            state.frames.as_mut().next(),
        )
        .await
        {
            Ok(next_frame) => next_frame,
            Err(_) => continue,
        };

        match next_frame {
            Some(Ok(frame)) => match frame {
                WarmAgentGuestFrame::Heartbeat { request_id }
                    if heartbeat_matches(request_id.as_deref(), &state.request_id) =>
                {
                    continue
                }
                WarmAgentGuestFrame::Token {
                    request_id,
                    token_id,
                    token_index,
                    text,
                } if request_id == state.request_id => {
                    if let Some(current) = token_index {
                        if let Some(previous) = state.last_token_index {
                            if current <= previous {
                                remove_active(&state.active_requests, &state.request_id);
                                state.done = true;
                                return Some((
                                    Err(ModelRuntimeError::GenerateError(format!(
                                        "warm-agent token index did not advance for request {}: \
                                         previous={}, current={}",
                                        state.request_id, previous, current
                                    ))),
                                    state,
                                ));
                            }
                        }
                        state.last_token_index = Some(current);
                    }
                    return Some((
                        Ok(GeneratedToken {
                            token_id,
                            text,
                            logprob: None,
                            finish_reason: None,
                        }),
                        state,
                    ));
                }
                WarmAgentGuestFrame::Complete {
                    request_id,
                    finish_reason,
                } if request_id == state.request_id => {
                    remove_active(&state.active_requests, &state.request_id);
                    state.done = true;
                    return Some((
                        Ok(terminal_token(parse_finish_reason(&finish_reason))),
                        state,
                    ));
                }
                WarmAgentGuestFrame::Error {
                    request_id,
                    code,
                    message,
                } if request_id.as_deref().is_none()
                    || request_id.as_deref() == Some(state.request_id.as_str()) =>
                {
                    remove_active(&state.active_requests, &state.request_id);
                    state.done = true;
                    return Some((
                        Err(ModelRuntimeError::GenerateError(format!(
                            "warm-agent error {code}: {message}"
                        ))),
                        state,
                    ));
                }
                other => {
                    remove_active(&state.active_requests, &state.request_id);
                    state.done = true;
                    return Some((
                        Err(ModelRuntimeError::GenerateError(format!(
                            "warm-agent {} frame did not match active request {}",
                            warm_agent_guest_frame_type(&other),
                            state.request_id
                        ))),
                        state,
                    ));
                }
            },
            Some(Err(error)) => {
                remove_active(&state.active_requests, &state.request_id);
                state.done = true;
                return Some((
                    Err(ModelRuntimeError::GenerateError(error.to_string())),
                    state,
                ));
            }
            None => {
                remove_active(&state.active_requests, &state.request_id);
                state.done = true;
                return Some((
                    Err(ModelRuntimeError::GenerateError(format!(
                        "warm-agent stream for request {} closed before terminal complete frame",
                        state.request_id
                    ))),
                    state,
                ));
            }
        }
    }
}

fn heartbeat_matches(frame_request_id: Option<&str>, active_request_id: &str) -> bool {
    frame_request_id.is_none() || frame_request_id == Some(active_request_id)
}

fn remove_active(active_requests: &Arc<Mutex<BTreeSet<String>>>, request_id: &str) {
    if let Ok(mut active) = active_requests.lock() {
        active.remove(request_id);
    }
}

fn single_error_stream(err: ModelRuntimeError) -> TokenStream {
    Box::pin(stream::iter([Err(err)]))
}

fn terminal_token(reason: FinishReason) -> GeneratedToken {
    GeneratedToken {
        token_id: 0,
        text: String::new(),
        logprob: None,
        finish_reason: Some(reason),
    }
}

fn parse_finish_reason(value: &str) -> FinishReason {
    match value.trim().to_ascii_lowercase().as_str() {
        "length" => FinishReason::Length,
        "cancelled" | "canceled" => FinishReason::Cancelled,
        "error" => FinishReason::Error,
        _ => FinishReason::Stop,
    }
}

fn load_protocol_error(error: WarmAgentProtocolError) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{
        GenPrompt, ProviderKind, RuntimeKind, SamplingParams, WARM_AGENT_PROTOCOL_ID,
        WARM_AGENT_PROTOCOL_VERSION,
    };
    use crate::sandbox::{AdapterId, SnapshotRef};
    use std::path::PathBuf;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct FakeObs {
        loads: Vec<WarmAgentHostFrame>,
        generates: Vec<WarmAgentGenerateRequest>,
        cancels: Vec<String>,
        shutdowns: usize,
    }

    struct FakeWarmTransport {
        ready: WarmAgentGuestFrame,
        scripts: StdMutex<Vec<Vec<Result<WarmAgentGuestFrame, WarmVmTransportError>>>>,
        obs: Arc<StdMutex<FakeObs>>,
    }

    struct PendingWarmTransport {
        ready: WarmAgentGuestFrame,
        obs: Arc<StdMutex<FakeObs>>,
    }

    #[async_trait]
    impl WarmAgentTransport for FakeWarmTransport {
        async fn load_model(
            &self,
            frame: WarmAgentHostFrame,
        ) -> Result<WarmAgentGuestFrame, WarmVmTransportError> {
            self.obs.lock().unwrap().loads.push(frame);
            Ok(self.ready.clone())
        }

        fn generate(&self, request: WarmAgentGenerateRequest) -> WarmAgentFrameStream {
            self.obs.lock().unwrap().generates.push(request.clone());
            let mut scripts = self.scripts.lock().unwrap();
            let mut script = if scripts.is_empty() {
                Vec::new()
            } else {
                scripts.remove(0)
            };
            for item in &mut script {
                if let Ok(frame) = item {
                    patch_blank_request_id(frame, &request.request_id);
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

        async fn shutdown(&self) -> Result<(), WarmVmTransportError> {
            self.obs.lock().unwrap().shutdowns += 1;
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

        async fn shutdown(&self) -> Result<(), WarmVmTransportError> {
            self.obs.lock().unwrap().shutdowns += 1;
            Ok(())
        }
    }

    fn ready(model_hash: &str, model_path: &str) -> WarmAgentGuestFrame {
        WarmAgentGuestFrame::Ready {
            protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
            protocol_version: WARM_AGENT_PROTOCOL_VERSION,
            agent_id: "fake-warm-agent".to_string(),
            ready_nonce: "nonce-1".to_string(),
            loaded_model_sha256: Some(model_hash.to_string()),
            loaded_model_guest_path: Some(model_path.to_string()),
        }
    }

    fn fake_transport(
        script: Vec<Result<WarmAgentGuestFrame, WarmVmTransportError>>,
    ) -> (Arc<FakeWarmTransport>, Arc<StdMutex<FakeObs>>) {
        let obs = Arc::new(StdMutex::new(FakeObs::default()));
        let transport = Arc::new(FakeWarmTransport {
            ready: ready("sha-warm", "/models/model.gguf"),
            scripts: StdMutex::new(vec![script]),
            obs: Arc::clone(&obs),
        });
        (transport, obs)
    }

    fn pending_transport() -> (Arc<PendingWarmTransport>, Arc<StdMutex<FakeObs>>) {
        let obs = Arc::new(StdMutex::new(FakeObs::default()));
        let transport = Arc::new(PendingWarmTransport {
            ready: ready("sha-warm", "/models/model.gguf"),
            obs: Arc::clone(&obs),
        });
        (transport, obs)
    }

    fn cfg() -> WarmVmModelConfig {
        WarmVmModelConfig::new("wt-1", "/models/model.gguf", "sha-warm")
    }

    fn load_spec() -> LoadSpec {
        LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: "sha-warm".to_string(),
            runtime_kind: RuntimeKind::LlamaCpp,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: crate::model_runtime::KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::Local,
            engine_origin: Some("warm_vm".to_string()),
            external_engine_import: None,
        }
    }

    fn gen_req(id: ModelId, prompt: &str) -> GenerateRequest {
        GenerateRequest {
            id,
            prompt: GenPrompt::new(prompt),
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

    #[tokio::test]
    async fn load_sends_protocol_load_and_validates_ready_model_identity() {
        let (transport, obs) = fake_transport(vec![]);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load warm model");
        assert_eq!(id, runtime.model_id());
        let obs = obs.lock().unwrap();
        let load = obs.loads.first().expect("load frame recorded");
        match load {
            WarmAgentHostFrame::Load {
                model_guest_path,
                model_artifact_sha256,
                ..
            } => {
                assert_eq!(model_guest_path, "/models/model.gguf");
                assert_eq!(model_artifact_sha256, "sha-warm");
            }
            other => panic!("expected Load frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn generate_yields_token_frame_before_terminal_complete() {
        let script = vec![
            Ok(WarmAgentGuestFrame::Token {
                request_id: String::new(),
                token_id: 1,
                token_index: None,
                text: "hel".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Token {
                request_id: String::new(),
                token_id: 2,
                token_index: None,
                text: "lo".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Complete {
                request_id: String::new(),
                finish_reason: "stop".to_string(),
            }),
        ];
        let (transport, obs) = fake_transport(script);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let request = gen_req(id, "hello?");
        let mut stream = runtime.generate(request);
        let generated_request_id = {
            let obs = obs.lock().unwrap();
            obs.generates
                .first()
                .expect("generate request")
                .request_id
                .clone()
        };
        assert!(!generated_request_id.is_empty());

        let first = stream.next().await.expect("first frame").expect("token");
        assert_eq!(first.text, "hel");
        assert_eq!(first.finish_reason, None);
        let second = stream.next().await.expect("second frame").expect("token");
        assert_eq!(second.text, "lo");
        let terminal = stream
            .next()
            .await
            .expect("terminal")
            .expect("terminal token");
        assert_eq!(terminal.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn restored_manifest_runtime_generates_without_calling_load() {
        let snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap");
        let manifest = WarmVmSnapshotManifest::new(
            "wt-1",
            "sha-warm",
            "/models/model.gguf",
            "nonce-1",
            snapshot,
        );
        let script = vec![Ok(WarmAgentGuestFrame::Complete {
            request_id: String::new(),
            finish_reason: "stop".to_string(),
        })];
        let (transport, obs) = fake_transport(script);
        let runtime = WarmVmModelRuntime::from_restored_manifest(transport, cfg(), &manifest)
            .expect("manifest matches");
        let mut stream = runtime.generate(gen_req(runtime.model_id(), "p"));
        let terminal = stream.next().await.expect("terminal").expect("ok");
        assert_eq!(terminal.finish_reason, Some(FinishReason::Stop));
        assert!(
            obs.lock().unwrap().loads.is_empty(),
            "no cold load after restore"
        );

        let stale_snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap2");
        let stale = WarmVmSnapshotManifest::new(
            "wt-1",
            "sha-old",
            "/models/model.gguf",
            "nonce",
            stale_snapshot,
        );
        let (transport, _obs) = fake_transport(vec![]);
        assert!(matches!(
            WarmVmModelRuntime::from_restored_manifest(transport, cfg(), &stale),
            Err(ModelRuntimeError::LoadError(_))
        ));

        let wrong_worktree_snapshot =
            SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap3");
        let wrong_worktree = WarmVmSnapshotManifest::new(
            "wt-other",
            "sha-warm",
            "/models/model.gguf",
            "nonce",
            wrong_worktree_snapshot,
        );
        let (transport, _obs) = fake_transport(vec![]);
        let err = WarmVmModelRuntime::from_restored_manifest(transport, cfg(), &wrong_worktree)
            .expect_err("wrong worktree manifest must fail");
        assert!(format!("{err}").contains("worktree mismatch"));
    }

    #[tokio::test]
    async fn cancel_sends_warm_agent_cancel_and_terminal_cancelled() {
        let script = vec![Ok(WarmAgentGuestFrame::Heartbeat { request_id: None })];
        let (transport, obs) = fake_transport(script);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let req = gen_req(id, "cancel me");
        let cancel = req.cancel.clone();
        let mut stream = runtime.generate(req);
        let generated_request_id = obs.lock().unwrap().generates[0].request_id.clone();
        cancel.cancel();
        let terminal = stream.next().await.expect("terminal").expect("cancelled");
        assert_eq!(terminal.finish_reason, Some(FinishReason::Cancelled));
        assert_eq!(obs.lock().unwrap().cancels, vec![generated_request_id]);
    }

    #[tokio::test]
    async fn request_cancel_interrupts_pending_frame_wait() {
        let (transport, obs) = pending_transport();
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let req = gen_req(id, "cancel while pending");
        let cancel = req.cancel.clone();
        let mut stream = runtime.generate(req);
        let generated_request_id = obs.lock().unwrap().generates[0].request_id.clone();

        let next_item = tokio::spawn(async move { stream.next().await });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        cancel.cancel();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(1), next_item)
            .await
            .expect("pending stream should observe cancellation")
            .expect("stream task joins")
            .expect("stream item")
            .expect("cancelled terminal");

        assert_eq!(terminal.finish_reason, Some(FinishReason::Cancelled));
        assert_eq!(obs.lock().unwrap().cancels, vec![generated_request_id]);
    }

    #[tokio::test]
    async fn dropped_stream_removes_active_request() {
        let script = vec![
            Ok(WarmAgentGuestFrame::Token {
                request_id: String::new(),
                token_id: 1,
                token_index: None,
                text: "partial".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Complete {
                request_id: String::new(),
                finish_reason: "stop".to_string(),
            }),
        ];
        let (transport, _obs) = fake_transport(script);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let stream = runtime.generate(gen_req(id, "drop early"));
        assert_eq!(
            runtime.active_requests.lock().unwrap().len(),
            1,
            "generate registers the active request"
        );
        drop(stream);
        assert!(
            runtime.active_requests.lock().unwrap().is_empty(),
            "dropping an unpolled stream must not leak active request ids"
        );
    }

    #[tokio::test]
    async fn mismatched_or_truncated_streams_surface_generate_errors() {
        let wrong = vec![Ok(WarmAgentGuestFrame::Token {
            request_id: "other-request".to_string(),
            token_id: 1,
            token_index: None,
            text: "sensitive-token-text".to_string(),
        })];
        let (transport, _obs) = fake_transport(wrong);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let mut stream = runtime.generate(gen_req(id, "p"));
        let err = stream
            .next()
            .await
            .expect("error")
            .expect_err("mismatch must fail");
        let rendered = err.to_string();
        assert!(rendered.contains("token frame did not match"));
        assert!(!rendered.contains("other-request"));
        assert!(!rendered.contains("sensitive-token-text"));

        let (transport, _obs) = fake_transport(vec![]);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let mut stream = runtime.generate(gen_req(id, "p"));
        assert!(matches!(
            stream.next().await.expect("truncated error"),
            Err(ModelRuntimeError::GenerateError(_))
        ));
    }

    #[tokio::test]
    async fn duplicate_tokenizer_ids_are_allowed_without_token_index() {
        let script = vec![
            Ok(WarmAgentGuestFrame::Token {
                request_id: String::new(),
                token_id: 7,
                token_index: None,
                text: "first".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Token {
                request_id: String::new(),
                token_id: 7,
                token_index: None,
                text: "repeat".to_string(),
            }),
            Ok(WarmAgentGuestFrame::Complete {
                request_id: String::new(),
                finish_reason: "stop".to_string(),
            }),
        ];
        let (transport, _obs) = fake_transport(script);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let mut stream = runtime.generate(gen_req(id, "p"));
        let first = stream.next().await.expect("first token").expect("first ok");
        assert_eq!(first.token_id, 7);
        let second = stream
            .next()
            .await
            .expect("second token")
            .expect("repeat token id remains valid");
        assert_eq!(second.token_id, 7);
        let terminal = stream.next().await.expect("terminal").expect("terminal ok");
        assert_eq!(terminal.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn duplicate_or_out_of_order_token_indexes_surface_generate_errors() {
        for script in [
            vec![
                Ok(WarmAgentGuestFrame::Token {
                    request_id: String::new(),
                    token_id: 7,
                    token_index: Some(1),
                    text: "first".to_string(),
                }),
                Ok(WarmAgentGuestFrame::Token {
                    request_id: String::new(),
                    token_id: 8,
                    token_index: Some(1),
                    text: "duplicate".to_string(),
                }),
            ],
            vec![
                Ok(WarmAgentGuestFrame::Token {
                    request_id: String::new(),
                    token_id: 7,
                    token_index: Some(2),
                    text: "second".to_string(),
                }),
                Ok(WarmAgentGuestFrame::Token {
                    request_id: String::new(),
                    token_id: 8,
                    token_index: Some(1),
                    text: "older".to_string(),
                }),
            ],
        ] {
            let (transport, _obs) = fake_transport(script);
            let mut runtime = WarmVmModelRuntime::new(transport, cfg());
            let id = runtime.load(load_spec()).await.expect("load");
            let mut stream = runtime.generate(gen_req(id, "p"));
            stream
                .next()
                .await
                .expect("first token")
                .expect("first token ok");
            let err = stream
                .next()
                .await
                .expect("second item")
                .expect_err("non-advancing token index must fail closed");
            assert!(
                format!("{err}").contains("token index did not advance"),
                "{err}"
            );
        }
    }

    #[tokio::test]
    async fn mismatched_heartbeat_surfaces_generate_error() {
        let wrong = vec![Ok(WarmAgentGuestFrame::Heartbeat {
            request_id: Some("other-request".to_string()),
        })];
        let (transport, _obs) = fake_transport(wrong);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        let mut stream = runtime.generate(gen_req(id, "p"));
        assert!(matches!(
            stream.next().await.expect("heartbeat mismatch error"),
            Err(ModelRuntimeError::GenerateError(_))
        ));
    }

    #[tokio::test]
    async fn unload_calls_transport_shutdown() {
        let (transport, obs) = fake_transport(vec![]);
        let mut runtime = WarmVmModelRuntime::new(transport, cfg());
        let id = runtime.load(load_spec()).await.expect("load");
        runtime.unload(id).await.expect("shutdown");
        assert_eq!(obs.lock().unwrap().shutdowns, 1);
    }

    #[test]
    fn cancel_without_tokio_runtime_does_not_panic() {
        let (transport, _obs) = fake_transport(vec![]);
        let runtime = WarmVmModelRuntime::new(transport, cfg());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.cancel(CancellationToken::new());
        }));
        assert!(result.is_ok());
    }

    fn patch_blank_request_id(frame: &mut WarmAgentGuestFrame, request_id: &str) {
        match frame {
            WarmAgentGuestFrame::Token { request_id: id, .. }
            | WarmAgentGuestFrame::Complete { request_id: id, .. } => {
                if id.is_empty() {
                    *id = request_id.to_string();
                }
            }
            WarmAgentGuestFrame::Error {
                request_id: Some(id),
                ..
            } => {
                if id.is_empty() {
                    *id = request_id.to_string();
                }
            }
            WarmAgentGuestFrame::Heartbeat {
                request_id: Some(id),
            } => {
                if id.is_empty() {
                    *id = request_id.to_string();
                }
            }
            _ => {}
        }
    }
}
