use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::kernel::context_bundle::{canonical_json_bytes, sha256_hex};
use crate::kernel::{
    ArtifactProposalDraft, KernelError, KernelEventType, KernelResult, KernelToolRequest,
    ModelAdapter, ModelAdapterOutput, ModelAdapterRequest,
};
use crate::sandbox::{
    select, AdapterId, BindMode, BindSpec, ImageRef, NetPolicy, ProcessHandle, ProcessSpec,
    RequiredCapability, ResourceLimits, SandboxAdapterError, SandboxAdapterRegistry,
    SandboxSelectionFailure, TrustClass,
};

const GGUF_GUEST_ROOT: &str = "/models/gguf";
const TOKENIZER_CACHE_GUEST_ROOT: &str = "/models/tokenizers";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    LlamaCpp,
    Candle,
}

impl EngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama_cpp",
            Self::Candle => "candle",
        }
    }

    fn argv(self, guest_gguf_path: &str) -> Vec<String> {
        match self {
            Self::LlamaCpp => vec![
                "llama-server".to_string(),
                "--model".to_string(),
                guest_gguf_path.to_string(),
            ],
            Self::Candle => vec![
                "candle-runner".to_string(),
                "--model".to_string(),
                guest_gguf_path.to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProcessSpec {
    pub model_id: String,
    pub gguf_path: PathBuf,
    pub engine_kind: EngineKind,
    pub work_profile_override: Option<AdapterId>,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub env: BTreeMap<String, String>,
    pub gguf_root_bind: PathBuf,
    pub tokenizer_cache_bind: PathBuf,
}

#[derive(Debug, Error)]
pub enum ModelBoxingError {
    #[error("model gguf path not found: {}", path.display())]
    ModelGgufNotFound { path: PathBuf },
    #[error("model work profile invalid: {reason}")]
    WorkProfileInvalid { reason: String },
    #[error(transparent)]
    SandboxSelection(#[from] SandboxSelectionFailure),
    #[error(transparent)]
    SandboxAdapter(#[from] SandboxAdapterError),
}

struct BoxedModelAcquisitionState {
    original: Option<std::pin::Pin<Box<dyn std::future::Future<
        Output = Result<ProcessHandle, SandboxAdapterError>,
    > + Send>>>,
    invoked: bool,
    result: Option<Result<ProcessHandle, SandboxAdapterError>>,
}

struct BoxedModelAcquisition {
    operation_id: uuid::Uuid,
    _adapter: Arc<dyn crate::sandbox::SandboxAdapter>,
    _original_spec: ProcessSpec,
    _original_context: crate::storage::surreal::CapturedModelLaneOperation,
    state: tokio::sync::Mutex<BoxedModelAcquisitionState>,
}

static RETAINED_BOXED_MODEL_ACQUISITIONS: std::sync::OnceLock<
    std::sync::Mutex<BTreeMap<uuid::Uuid, Arc<BoxedModelAcquisition>>>,
> = std::sync::OnceLock::new();

impl BoxedModelAcquisition {
    async fn observe(&self) -> Result<ProcessHandle, ModelBoxingError> {
        let mut state = self.state.lock().await;
        if let Some(result) = &state.result {
            return result.clone().map_err(ModelBoxingError::SandboxAdapter);
        }
        state.invoked = true;
        let result = state.original.as_mut().ok_or_else(|| ModelBoxingError::WorkProfileInvalid {
            reason: format!("original model acquisition {} is unavailable; custody remains pending", self.operation_id),
        })?.as_mut().await;
        // Cancellation drops only this observer. The original future remains
        // in its registered owner until its exact result has been retained.
        state.result = Some(result.clone());
        state.original.take();
        result.map_err(ModelBoxingError::SandboxAdapter)
    }


}

fn begin_boxed_model_acquisition(
    registry: &SandboxAdapterRegistry,
    spec: ModelProcessSpec,
) -> Result<Arc<BoxedModelAcquisition>, ModelBoxingError> {
    let work_profile_override = spec.work_profile_override.clone();
    let process_spec = process_spec_from_model_spec(spec)?;
    let adapter = select(registry, &process_spec, work_profile_override.as_ref())?;
    adapter.validate_spawn_acquisition()?;
    let original_adapter = Arc::clone(&adapter);
    let original_spec = process_spec.clone();
    let owner = Arc::new(BoxedModelAcquisition {
        operation_id: uuid::Uuid::now_v7(),
        _adapter: adapter,
        _original_spec: process_spec,
        _original_context: crate::storage::surreal::capture_model_lane_operation(),
        state: tokio::sync::Mutex::new(BoxedModelAcquisitionState {
            original: Some(Box::pin(async move { original_adapter.spawn(original_spec).await })),
            invoked: false, result: None,
        }),
    });
    // Every invocation owns a distinct operation. Spec/context equality never
    // substitutes for ownership or coalesces concurrent callers.
    RETAINED_BOXED_MODEL_ACQUISITIONS.get_or_init(Default::default).lock()
        .unwrap_or_else(|error| error.into_inner()).insert(owner.operation_id, Arc::clone(&owner));
    Ok(owner)
}

struct RoutedAcquisition {
    owner: Arc<BoxedModelAcquisition>,
    caller_live: bool,
}

type RoutedPendingAcquisitions =
    Arc<std::sync::Mutex<BTreeMap<uuid::Uuid, RoutedAcquisition>>>;

struct BoxingCaller {
    owner: Arc<BoxedModelAcquisition>,
    routed: Option<RoutedPendingAcquisitions>,
    delivered: bool,
}

impl BoxingCaller {
    fn deliver(&mut self, handle: &ProcessHandle) -> Result<(), ModelBoxingError> {
        let invalid = || ModelBoxingError::WorkProfileInvalid {
            reason: "original boxing owner is not ready for exact delivery".into(),
        };
        let mut routed = self.routed.as_ref().map(|pending|
            pending.lock().unwrap_or_else(|error| error.into_inner()));
        if let Some(pending) = &routed {
            if !pending.get(&self.owner.operation_id).is_some_and(|current|
                current.caller_live && Arc::ptr_eq(&current.owner, &self.owner)) {
                return Err(invalid());
            }
        }
        let state = self.owner.state.try_lock().map_err(|_| invalid())?;
        if state.original.is_some() || state.result.as_ref() != Some(&Ok(handle.clone())) {
            return Err(invalid());
        }
        let mut retained = RETAINED_BOXED_MODEL_ACQUISITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if !retained.get(&self.owner.operation_id)
            .is_some_and(|current| Arc::ptr_eq(current, &self.owner)) {
            return Err(invalid());
        }
        retained.remove(&self.owner.operation_id);
        if let Some(pending) = &mut routed { pending.remove(&self.owner.operation_id); }
        self.delivered = true;
        Ok(())
    }
}

impl Drop for BoxingCaller {
    fn drop(&mut self) {
        if self.delivered { return; }
        if let Some(pending) = &self.routed {
            let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
            if let Some(current) = pending.get_mut(&self.owner.operation_id) {
                if Arc::ptr_eq(&current.owner, &self.owner) { current.caller_live = false; }
            }
        }
        // The exact unknown/undelivered owner and original future stay retained.
        // Caller loss never requests a native signal or a replacement spawn.
    }
}

pub async fn box_model_process(
    registry: &SandboxAdapterRegistry,
    spec: ModelProcessSpec,
) -> Result<ProcessHandle, ModelBoxingError> {
    let owner = begin_boxed_model_acquisition(registry, spec)?;
    let mut caller = BoxingCaller { owner: Arc::clone(&owner), routed: None, delivered: false };
    let handle = owner.observe().await?;
    caller.deliver(&handle)?;
    Ok(handle)
}

#[derive(Clone)]
pub struct SandboxRoutedModelAdapter {
    adapter_id: String,
    registry: Arc<SandboxAdapterRegistry>,
    process_spec: ModelProcessSpec,
    pending: RoutedPendingAcquisitions,
}

impl SandboxRoutedModelAdapter {
    pub fn new(
        adapter_id: impl Into<String>,
        registry: Arc<SandboxAdapterRegistry>,
        process_spec: ModelProcessSpec,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            registry,
            process_spec,
            pending: Arc::new(std::sync::Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl ModelAdapter for SandboxRoutedModelAdapter {
    fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        if self.adapter_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("adapter_id is required"));
        }

        let context_bundle = request.context_bundle;
        let acquisition = {
            let mut pending = self.pending.lock().unwrap_or_else(|error| error.into_inner());
            if pending.values().any(|owner| !owner.caller_live) {
                return Err(KernelError::ModelRuntime(
                    "an abandoned or unknown original routed acquisition retains custody; no replacement spawn".into(),
                ));
            }
            let acquisition = begin_boxed_model_acquisition(&self.registry, self.process_spec.clone())
                .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
            pending.insert(acquisition.operation_id, RoutedAcquisition {
                owner: Arc::clone(&acquisition), caller_live: true,
            });
            acquisition
        };
        let mut caller = BoxingCaller {
            owner: Arc::clone(&acquisition), routed: Some(Arc::clone(&self.pending)), delivered: false,
        };
        let handle = acquisition.observe().await
            .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
        let response_text = format!(
            "sandbox-routed-model:{}:{}:{}",
            self.adapter_id, context_bundle.context_bundle_id, handle.id
        );
        let process_uuid = handle.id.to_string();
        let artifact_payload = json!({
            "adapter_id": self.adapter_id.as_str(),
            "context_bundle_id": context_bundle.context_bundle_id.as_str(),
            "context_hash": context_bundle.context_hash.as_str(),
            "model_id": self.process_spec.model_id.as_str(),
            "engine_kind": self.process_spec.engine_kind.as_str(),
            "sandbox_adapter_id": handle.adapter_id.as_str(),
            "process_uuid": process_uuid,
            "pid": handle.pid,
            "sandbox_internal_id": handle.sandbox_internal_id.as_str(),
            "response_text": response_text.as_str(),
        });
        let output_hash = sha256_hex(&canonical_json_bytes(&artifact_payload));
        let tool_request = KernelToolRequest {
            tool_request_id: format!("TOOLREQ-{}", &output_hash[..16]),
            event_type: KernelEventType::ToolRequestRecorded,
            tool_id: "sandbox_model_process".to_string(),
            reason: "sandbox-routed model adapter process boxing".to_string(),
        };
        let artifact_proposal = ArtifactProposalDraft {
            artifact_proposal_id: format!("AP-{}", &output_hash[16..32]),
            event_type: KernelEventType::ArtifactProposed,
            artifact_kind: "sandbox_model_process_output".to_string(),
            content_hash: output_hash.clone(),
        };

        let output = ModelAdapterOutput {
            adapter_id: self.adapter_id.clone(),
            context_bundle_id: context_bundle.context_bundle_id,
            response_text,
            response_event_type: KernelEventType::ModelResponseRecorded,
            tool_request,
            artifact_proposal,
            artifact_payload,
            output_hash,
        };
        caller.deliver(&handle).map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
        Ok(output)
    }
}

pub fn process_spec_from_model_spec(
    spec: ModelProcessSpec,
) -> Result<ProcessSpec, ModelBoxingError> {
    let model_id = spec.model_id.trim();
    if model_id.is_empty() {
        return Err(ModelBoxingError::WorkProfileInvalid {
            reason: "model_id is required".to_string(),
        });
    }

    let gguf_path = canonical_existing_file(&spec.gguf_path)?;
    let gguf_root_bind = canonical_existing_dir(&spec.gguf_root_bind, "gguf_root_bind")?;
    let tokenizer_cache_bind =
        canonical_existing_dir(&spec.tokenizer_cache_bind, "tokenizer_cache_bind")?;
    let relative_gguf = gguf_path.strip_prefix(&gguf_root_bind).map_err(|_| {
        ModelBoxingError::WorkProfileInvalid {
            reason: format!(
                "gguf_path {} must live under gguf_root_bind {}",
                gguf_path.display(),
                gguf_root_bind.display()
            ),
        }
    })?;
    let guest_gguf_path = guest_path(GGUF_GUEST_ROOT, relative_gguf);

    let mut metadata = BTreeMap::new();
    metadata.insert("model_id".to_string(), model_id.to_string());
    metadata.insert(
        "engine_kind".to_string(),
        spec.engine_kind.as_str().to_string(),
    );
    if let Some(adapter_id) = &spec.work_profile_override {
        metadata.insert(
            "work_profile_override".to_string(),
            adapter_id.as_str().to_string(),
        );
    }

    Ok(ProcessSpec {
        id: AdapterId::new(format!("model-process:{model_id}")),
        image_or_root: ImageRef::new(spec.engine_kind.as_str()),
        cmd: spec.engine_kind.argv(&guest_gguf_path),
        env: spec.env,
        cwd: None,
        binds: vec![
            BindSpec {
                host_path: gguf_root_bind,
                guest_path: PathBuf::from(GGUF_GUEST_ROOT),
                mode: BindMode::ReadOnly,
            },
            BindSpec {
                host_path: tokenizer_cache_bind,
                guest_path: PathBuf::from(TOKENIZER_CACHE_GUEST_ROOT),
                mode: BindMode::ReadOnly,
            },
        ],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        idle_timeout_ms: None,
        required_capabilities: spec.required_capabilities,
        // The local inference engine is operator-trusted runtime infrastructure
        // (not agent-authored code), so Tier-1 container isolation is permitted
        // per Master Spec v02.187 §3.5.4.
        trust_class: TrustClass::Trusted,
        metadata,
    })
}

fn canonical_existing_file(path: &Path) -> Result<PathBuf, ModelBoxingError> {
    if !path.is_file() {
        return Err(ModelBoxingError::ModelGgufNotFound {
            path: path.to_path_buf(),
        });
    }
    path.canonicalize()
        .map_err(|_| ModelBoxingError::ModelGgufNotFound {
            path: path.to_path_buf(),
        })
}

fn canonical_existing_dir(path: &Path, field: &'static str) -> Result<PathBuf, ModelBoxingError> {
    if !path.is_dir() {
        return Err(ModelBoxingError::WorkProfileInvalid {
            reason: format!("{field} must exist and be a directory: {}", path.display()),
        });
    }
    path.canonicalize()
        .map_err(|error| ModelBoxingError::WorkProfileInvalid {
            reason: format!("failed to canonicalize {field} {}: {error}", path.display()),
        })
}

fn guest_path(root: &str, relative: &Path) -> String {
    let tail = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if tail.is_empty() {
        root.to_string()
    } else {
        format!("{}/{}", root.trim_end_matches('/'), tail)
    }
}
