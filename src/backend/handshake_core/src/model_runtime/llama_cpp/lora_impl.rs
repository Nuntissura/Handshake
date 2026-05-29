#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{collections::HashSet, ptr::NonNull};
use std::{
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::model_runtime::{
    LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackOps, LoraStackSnapshot,
    LoraStackSnapshotEntry, LoraStrength, ModelId, ModelRuntimeError,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use super::context::NativeLlamaCppBackend;

#[derive(Clone, Debug)]
pub struct LlamaCppLoraStack {
    model_id: ModelId,
    base_model_tag: String,
    state: Arc<Mutex<LlamaCppLoraStackState>>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    native: Arc<NativeLlamaCppBackend>,
}

#[derive(Debug, Default)]
struct LlamaCppLoraStackState {
    active: Vec<MountedLora>,
}

#[derive(Debug)]
struct MountedLora {
    descriptor: LoraDescriptor,
    strength: LoraStrength,
    mounted_at_utc: DateTime<Utc>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    adapter: Arc<NativeLoraAdapter>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug)]
struct NativeLoraAdapter {
    inner: Mutex<llama_cpp_2::model::LlamaLoraAdapter>,
    _owner: Arc<NativeLlamaCppBackend>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl NativeLoraAdapter {
    fn new(
        adapter: llama_cpp_2::model::LlamaLoraAdapter,
        owner: Arc<NativeLlamaCppBackend>,
    ) -> Self {
        Self {
            inner: Mutex::new(adapter),
            _owner: owner,
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
// Safety: llama.cpp adapters are model-owned native handles. Handshake stores
// the handle behind a mutex, applies it only through llama-cpp-2's safe context
// API, and each adapter keeps the owning model alive through
// Arc<NativeLlamaCppBackend>.
unsafe impl Send for NativeLoraAdapter {}

#[cfg(feature = "llama-cpp-runtime-engine")]
// Safety: shared access is limited to locking the adapter while calling the safe
// wrapper that installs it on a fresh per-request context.
unsafe impl Sync for NativeLoraAdapter {}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl Drop for NativeLoraAdapter {
    fn drop(&mut self) {
        if let Ok(mut adapter) = self.inner.lock() {
            // llama-cpp-2 0.1.146 does not implement Drop for LlamaLoraAdapter.
            // The wrapper is repr(transparent) over the native NonNull handle.
            unsafe {
                llama_cpp_sys_2::llama_adapter_lora_free(lora_adapter_ptr(&mut adapter));
            }
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
unsafe fn lora_adapter_ptr(
    adapter: &mut llama_cpp_2::model::LlamaLoraAdapter,
) -> *mut llama_cpp_sys_2::llama_adapter_lora {
    let adapter_ptr = adapter as *mut _ as *mut NonNull<llama_cpp_sys_2::llama_adapter_lora>;
    unsafe { (*adapter_ptr).as_ptr() }
}

impl LlamaCppLoraStack {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn new(
        model_id: ModelId,
        base_model_tag: impl Into<String>,
        native: Arc<NativeLlamaCppBackend>,
    ) -> Self {
        Self {
            model_id,
            base_model_tag: base_model_tag.into(),
            state: Arc::new(Mutex::new(LlamaCppLoraStackState::default())),
            native,
        }
    }

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    pub(super) fn new(model_id: ModelId, base_model_tag: impl Into<String>) -> Self {
        Self {
            model_id,
            base_model_tag: base_model_tag.into(),
            state: Arc::new(Mutex::new(LlamaCppLoraStackState::default())),
        }
    }

    pub fn handle(&self) -> LoraStackHandle {
        LoraStackHandle::with_ops(
            format!("llama_cpp:{}:lora_stack", self.model_id),
            Arc::new(self.clone()),
        )
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn validate_request(
        &self,
        lora_overrides: &[LoraId],
        has_kv_prefix: bool,
    ) -> Result<(), ModelRuntimeError> {
        let selected = self.selected_entries(lora_overrides)?;
        if has_kv_prefix && !selected.is_empty() {
            return Err(lora_error(
                "llama.cpp KV prefix handles are not scoped by LoRA stack; disable kv_prefix_handle when LoRA overrides or mounted default LoRAs are active",
            ));
        }
        if selected.len() > 1 {
            return Err(Self::multi_lora_error(selected.len()));
        }
        Ok(())
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn apply_to_context(
        &self,
        context: &llama_cpp_2::context::LlamaContext<'_>,
        lora_overrides: &[LoraId],
        has_kv_prefix: bool,
    ) -> Result<AppliedLoraGuard, ModelRuntimeError> {
        let selected = self.selected_entries(lora_overrides)?;
        if selected.is_empty() {
            return Ok(AppliedLoraGuard::default());
        }
        if has_kv_prefix {
            return Err(lora_error(
                "llama.cpp KV prefix handles are not scoped by LoRA stack; disable kv_prefix_handle when LoRA overrides or mounted default LoRAs are active",
            ));
        }
        if selected.len() > 1 {
            return Err(Self::multi_lora_error(selected.len()));
        }
        let mounted = selected
            .into_iter()
            .next()
            .expect("selected length was checked");
        let adapter_arc = mounted.adapter.clone();
        let mut adapter = adapter_arc
            .inner
            .lock()
            .map_err(|_| lora_error("llama.cpp native LoRA adapter lock is poisoned"))?;
        context
            .lora_adapter_set(&mut adapter, mounted.strength.value())
            .map_err(|error| {
                lora_error(format!(
                    "failed to apply llama.cpp LoRA {}: {error}",
                    mounted.descriptor.id
                ))
            })?;
        drop(adapter);
        Ok(AppliedLoraGuard {
            _adapters: vec![adapter_arc],
        })
    }

    fn load_adapter(&self, desc: &LoraDescriptor) -> Result<LoadedLoraAdapter, ModelRuntimeError> {
        self.validate_descriptor(desc)?;
        validate_sha256(&desc.artifact_path, desc.sha256)?;
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let adapter = self
                .native
                .model
                .lora_adapter_init(&desc.artifact_path)
                .map_err(|error| {
                    lora_error(format!(
                        "failed to initialize llama.cpp LoRA adapter {}: {error}",
                        desc.artifact_path.display()
                    ))
                })?;
            return Ok(LoadedLoraAdapter {
                #[cfg(feature = "llama-cpp-runtime-engine")]
                adapter: Arc::new(NativeLoraAdapter::new(adapter, self.native.clone())),
            });
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "llama_cpp_lora_stack".to_string(),
                adapter: "llama.cpp native engine feature disabled".to_string(),
            })
        }
    }

    fn validate_descriptor(&self, desc: &LoraDescriptor) -> Result<(), ModelRuntimeError> {
        if desc.license_tag.as_str().trim().is_empty() {
            return Err(lora_error("LoRA descriptor license tag must not be empty"));
        }
        if desc.base_model_compat.as_str() != self.base_model_tag {
            return Err(lora_error(format!(
                "LoRA base model mismatch: descriptor={}, loaded={}",
                desc.base_model_compat.as_str(),
                self.base_model_tag
            )));
        }
        if desc.rank == 0 {
            return Err(lora_error("LoRA rank must be greater than zero"));
        }
        if desc.target_modules.is_empty() {
            return Err(lora_error(
                "LoRA descriptor target_modules must not be empty",
            ));
        }
        if !desc.artifact_path.exists() {
            return Err(lora_error(format!(
                "LoRA artifact does not exist: {}",
                desc.artifact_path.display()
            )));
        }
        Ok(())
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn selected_entries(
        &self,
        lora_overrides: &[LoraId],
    ) -> Result<Vec<MountedLoraRef>, ModelRuntimeError> {
        let state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        if lora_overrides.is_empty() {
            return Ok(state
                .active
                .iter()
                .map(MountedLoraRef::from_mounted)
                .collect());
        }

        let requested = lora_overrides.iter().copied().collect::<HashSet<_>>();
        let mounted_ids = state
            .active
            .iter()
            .map(|entry| entry.descriptor.id)
            .collect::<HashSet<_>>();
        let missing = requested
            .difference(&mounted_ids)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(lora_error(format!(
                "llama.cpp LoRA override ids are not mounted: {}",
                missing.join(", ")
            )));
        }

        Ok(state
            .active
            .iter()
            .filter(|entry| requested.contains(&entry.descriptor.id))
            .map(MountedLoraRef::from_mounted)
            .collect())
    }

    fn snapshot_locked(state: &LlamaCppLoraStackState) -> LoraStackSnapshot {
        LoraStackSnapshot {
            entries: state
                .active
                .iter()
                .map(|entry| LoraStackSnapshotEntry {
                    descriptor: entry.descriptor.clone(),
                    strength: entry.strength.clone(),
                    mounted_at_utc: entry.mounted_at_utc,
                })
                .collect(),
        }
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn multi_lora_error(count: usize) -> ModelRuntimeError {
        lora_error(format!(
            "llama-cpp-2 0.1.146 safe API exposes one active LoRA adapter per context; requested {count} active adapters"
        ))
    }

    fn multi_lora_stack_error(count: usize) -> ModelRuntimeError {
        lora_error(format!(
            "llama-cpp-2 0.1.146 safe API exposes one mounted LoRA adapter per llama.cpp model stack; requested {count} mounted adapters"
        ))
    }

    fn validate_mount_slot(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        let state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        Self::validate_mount_slot_locked(&state, id)
    }

    fn validate_mount_slot_locked(
        state: &LlamaCppLoraStackState,
        id: LoraId,
    ) -> Result<(), ModelRuntimeError> {
        if state.active.iter().any(|entry| entry.descriptor.id != id) {
            return Err(Self::multi_lora_stack_error(state.active.len() + 1));
        }
        Ok(())
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug, Default)]
pub(super) struct AppliedLoraGuard {
    _adapters: Vec<Arc<NativeLoraAdapter>>,
}

#[derive(Debug)]
struct LoadedLoraAdapter {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    adapter: Arc<NativeLoraAdapter>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug)]
struct MountedLoraRef {
    descriptor: LoraDescriptor,
    strength: LoraStrength,
    adapter: Arc<NativeLoraAdapter>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
impl MountedLoraRef {
    fn from_mounted(mounted: &MountedLora) -> Self {
        Self {
            descriptor: mounted.descriptor.clone(),
            strength: mounted.strength.clone(),
            adapter: mounted.adapter.clone(),
        }
    }
}

#[async_trait]
impl LoraStackOps for LlamaCppLoraStack {
    async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        self.validate_mount_slot(desc.id)?;
        let loaded = self.load_adapter(&desc)?;
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        let _ = loaded;
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        Self::validate_mount_slot_locked(&state, desc.id)?;
        state.active.retain(|entry| entry.descriptor.id != desc.id);
        state.active.push(MountedLora {
            descriptor: desc,
            strength,
            mounted_at_utc: Utc::now(),
            #[cfg(feature = "llama-cpp-runtime-engine")]
            adapter: loaded.adapter,
        });
        Ok(())
    }

    async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        let before = state.active.len();
        state.active.retain(|entry| entry.descriptor.id != id);
        if state.active.len() == before {
            return Err(lora_error(format!("unknown llama.cpp LoRA id {id}")));
        }
        Ok(())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        self.state
            .lock()
            .map(|state| {
                state
                    .active
                    .iter()
                    .map(|entry| LoraStackEntry {
                        id: entry.descriptor.id,
                        strength: entry.strength.clone(),
                        mounted_at_utc: entry.mounted_at_utc,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn set_strength(
        &self,
        id: LoraId,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        let Some(entry) = state
            .active
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
        else {
            return Err(lora_error(format!("unknown llama.cpp LoRA id {id}")));
        };
        entry.strength = strength;
        Ok(())
    }

    async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        if new_stack.len() > 1 {
            return Err(Self::multi_lora_stack_error(new_stack.len()));
        }
        let mut replacement = Vec::with_capacity(new_stack.len());
        for (desc, strength) in new_stack {
            let loaded = self.load_adapter(&desc)?;
            #[cfg(not(feature = "llama-cpp-runtime-engine"))]
            let _ = loaded;
            replacement.push(MountedLora {
                descriptor: desc,
                strength,
                mounted_at_utc: Utc::now(),
                #[cfg(feature = "llama-cpp-runtime-engine")]
                adapter: loaded.adapter,
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| lora_error("llama.cpp LoRA stack lock is poisoned"))?;
        let previous = Self::snapshot_locked(&state);
        state.active = replacement;
        Ok(previous)
    }
}

fn validate_sha256(
    path: &std::path::Path,
    expected_sha256: [u8; 32],
) -> Result<(), ModelRuntimeError> {
    let mut file = File::open(path).map_err(|error| {
        lora_error(format!(
            "failed to open LoRA artifact {}: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            lora_error(format!(
                "failed to read LoRA artifact {}: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual: [u8; 32] = hasher.finalize().into();
    if actual != expected_sha256 {
        return Err(lora_error(format!(
            "LoRA artifact sha256 mismatch: expected {}, got {}",
            hex::encode(expected_sha256),
            hex::encode(actual)
        )));
    }
    Ok(())
}

fn lora_error(message: impl Into<String>) -> ModelRuntimeError {
    ModelRuntimeError::LoraStackError(message.into())
}
