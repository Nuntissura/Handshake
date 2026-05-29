use std::{path::PathBuf, sync::Arc};

use handshake_core::model_runtime::{
    techniques::lora_hotswap, BaseModelTag, LicenseTag, LoraDescriptor, LoraId, LoraStackEntry,
    LoraStackSnapshot, LoraStackSnapshotEntry, LoraStrength, ModelId, ModelRuntime,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::model_runtime::{parse_model_id, ModelRuntimeState};

pub const KERNEL_MODEL_RUNTIME_LORA_MOUNT_IPC_CHANNEL: &str = "kernel_model_runtime_lora_mount";
pub const KERNEL_MODEL_RUNTIME_LORA_UNMOUNT_IPC_CHANNEL: &str = "kernel_model_runtime_lora_unmount";
pub const KERNEL_MODEL_RUNTIME_LORA_SWAP_IPC_CHANNEL: &str = "kernel_model_runtime_lora_swap";
pub const KERNEL_MODEL_RUNTIME_LORA_LIST_IPC_CHANNEL: &str = "kernel_model_runtime_lora_list";
pub const LORA_NOT_AVAILABLE_PREFIX: &str = "lora_not_available";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraDescriptorIpc {
    pub lora_id: String,
    pub artifact_path: String,
    pub sha256: String,
    pub rank: u32,
    pub target_modules: Vec<String>,
    pub base_model_compat: String,
    pub license_tag: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraStackItemIpc {
    pub descriptor: LoraDescriptorIpc,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraExecPolicyIpc {
    #[serde(default)]
    pub lora_stack: Vec<LoraStackItemIpc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraCommandSettingsIpc {
    #[serde(default)]
    pub exec_policy: Option<LoraExecPolicyIpc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraMountRequestIpc {
    pub model_id: String,
    pub descriptor: LoraDescriptorIpc,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraUnmountRequestIpc {
    pub model_id: String,
    pub lora_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraSwapRequestIpc {
    pub model_id: String,
    #[serde(default)]
    pub stack: Vec<LoraStackItemIpc>,
    #[serde(default)]
    pub settings: Option<LoraCommandSettingsIpc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraListRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraStackEntryIpc {
    pub lora_id: String,
    pub strength: f32,
    pub mounted_at_utc: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraStackSnapshotEntryIpc {
    pub descriptor: LoraDescriptorIpc,
    pub strength: f32,
    pub mounted_at_utc: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraMutationResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub active_stack: Vec<LoraStackEntryIpc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraSwapResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub previous_stack: Vec<LoraStackSnapshotEntryIpc>,
    pub active_stack: Vec<LoraStackEntryIpc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoraListResultIpc {
    pub model_id: String,
    pub active_stack: Vec<LoraStackEntryIpc>,
}

#[tauri::command]
pub async fn kernel_model_runtime_lora_mount(
    request: LoraMountRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<LoraMutationResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_LORA_MOUNT_IPC_CHANNEL;
    lora_mount(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_lora_unmount(
    request: LoraUnmountRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<LoraMutationResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_LORA_UNMOUNT_IPC_CHANNEL;
    lora_unmount(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_lora_swap(
    request: LoraSwapRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<LoraSwapResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_LORA_SWAP_IPC_CHANNEL;
    lora_swap(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_lora_list(
    request: LoraListRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<LoraListResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_LORA_LIST_IPC_CHANNEL;
    lora_list(request, state.inner()).await
}

pub async fn lora_mount(
    request: LoraMountRequestIpc,
    state: &ModelRuntimeState,
) -> Result<LoraMutationResultIpc, String> {
    let model_id = preflight_lora_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state)?;
    let descriptor = descriptor_from_ipc(request.descriptor)?;
    let strength = LoraStrength::try_new(request.strength).map_err(|error| error.to_string())?;
    let receipt = lora_hotswap::mount(runtime.as_ref(), model_id, descriptor, strength)
        .await
        .map_err(|error| format!("lora mount dispatch failed: {error}"))?;
    Ok(LoraMutationResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        active_stack: active_stack_to_ipc(receipt.active_stack),
    })
}

pub async fn lora_unmount(
    request: LoraUnmountRequestIpc,
    state: &ModelRuntimeState,
) -> Result<LoraMutationResultIpc, String> {
    let model_id = preflight_lora_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state)?;
    let lora_id = parse_lora_id(&request.lora_id)?;
    let receipt = lora_hotswap::unmount(runtime.as_ref(), model_id, lora_id)
        .await
        .map_err(|error| format!("lora unmount dispatch failed: {error}"))?;
    Ok(LoraMutationResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        active_stack: active_stack_to_ipc(receipt.active_stack),
    })
}

pub async fn lora_swap(
    request: LoraSwapRequestIpc,
    state: &ModelRuntimeState,
) -> Result<LoraSwapResultIpc, String> {
    let model_id = preflight_lora_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state)?;
    let stack_items = effective_stack_items(request);
    let new_stack = stack_items
        .into_iter()
        .map(stack_item_from_ipc)
        .collect::<Result<Vec<_>, _>>()?;
    let receipt = lora_hotswap::swap(runtime.as_ref(), model_id, new_stack)
        .await
        .map_err(|error| format!("lora swap dispatch failed: {error}"))?;
    Ok(LoraSwapResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        previous_stack: snapshot_to_ipc(receipt.previous_stack),
        active_stack: active_stack_to_ipc(receipt.active_stack),
    })
}

pub async fn lora_list(
    request: LoraListRequestIpc,
    state: &ModelRuntimeState,
) -> Result<LoraListResultIpc, String> {
    let model_id = preflight_lora_capability_and_loaded(&request.model_id, state)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = lora_hotswap::list(runtime.as_ref(), model_id)
        .map_err(|error| format!("lora list dispatch failed: {error}"))?;
    Ok(LoraListResultIpc {
        model_id: receipt.model_id.to_string(),
        active_stack: active_stack_to_ipc(receipt.active_stack),
    })
}

fn preflight_lora_capability_and_loaded(
    model_id: &str,
    state: &ModelRuntimeState,
) -> Result<ModelId, String> {
    let model_id = parse_model_id(model_id)?;
    state.lora_command_binding(model_id)?;
    Ok(model_id)
}

fn require_live_runtime(
    model_id: ModelId,
    state: &ModelRuntimeState,
) -> Result<Arc<dyn ModelRuntime>, String> {
    state.live_runtime(model_id)?.ok_or_else(|| {
        format!(
            "{LORA_NOT_AVAILABLE_PREFIX}: LoRA hot-swap requires a live ModelRuntime adapter attached for model {model_id}; the adapter is not yet bound to this model in this app session"
        )
    })
}

fn effective_stack_items(request: LoraSwapRequestIpc) -> Vec<LoraStackItemIpc> {
    request
        .settings
        .and_then(|settings| settings.exec_policy)
        .map(|policy| policy.lora_stack)
        .filter(|stack| !stack.is_empty())
        .unwrap_or(request.stack)
}

fn stack_item_from_ipc(item: LoraStackItemIpc) -> Result<(LoraDescriptor, LoraStrength), String> {
    Ok((
        descriptor_from_ipc(item.descriptor)?,
        LoraStrength::try_new(item.strength).map_err(|error| error.to_string())?,
    ))
}

fn descriptor_from_ipc(value: LoraDescriptorIpc) -> Result<LoraDescriptor, String> {
    Ok(LoraDescriptor {
        id: parse_lora_id(&value.lora_id)?,
        artifact_path: PathBuf::from(non_empty(value.artifact_path, "artifact_path")?),
        sha256: parse_sha256(&value.sha256)?,
        rank: value.rank,
        target_modules: non_empty_vec(value.target_modules, "target_modules")?,
        base_model_compat: BaseModelTag::try_new(value.base_model_compat)
            .map_err(|error| error.to_string())?,
        license_tag: LicenseTag::try_new(value.license_tag).map_err(|error| error.to_string())?,
    })
}

fn descriptor_to_ipc(value: LoraDescriptor) -> LoraDescriptorIpc {
    LoraDescriptorIpc {
        lora_id: value.id.to_string(),
        artifact_path: value.artifact_path.to_string_lossy().to_string(),
        sha256: hex::encode(value.sha256),
        rank: value.rank,
        target_modules: value.target_modules,
        base_model_compat: value.base_model_compat.as_str().to_string(),
        license_tag: value.license_tag.as_str().to_string(),
    }
}

fn parse_lora_id(value: &str) -> Result<LoraId, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("lora_id must not be empty".to_string());
    }
    let uuid = Uuid::parse_str(trimmed).map_err(|error| format!("invalid lora_id: {error}"))?;
    LoraId::try_from_uuid(uuid).map_err(|error| error.to_string())
}

fn parse_sha256(value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim();
    let bytes = hex::decode(trimmed).map_err(|error| format!("invalid sha256 hex: {error}"))?;
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("sha256 must decode to 32 bytes, got {}", bytes.len()))
}

fn non_empty(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(trimmed.to_string())
}

fn non_empty_vec(values: Vec<String>, field: &str) -> Result<Vec<String>, String> {
    let normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return Err(format!("{field} must contain at least one value"));
    }
    Ok(normalized)
}

fn active_stack_to_ipc(entries: Vec<LoraStackEntry>) -> Vec<LoraStackEntryIpc> {
    entries
        .into_iter()
        .map(|entry| LoraStackEntryIpc {
            lora_id: entry.id.to_string(),
            strength: entry.strength.value(),
            mounted_at_utc: entry.mounted_at_utc.to_rfc3339(),
        })
        .collect()
}

fn snapshot_to_ipc(snapshot: LoraStackSnapshot) -> Vec<LoraStackSnapshotEntryIpc> {
    snapshot
        .entries
        .into_iter()
        .map(snapshot_entry_to_ipc)
        .collect()
}

fn snapshot_entry_to_ipc(entry: LoraStackSnapshotEntry) -> LoraStackSnapshotEntryIpc {
    LoraStackSnapshotEntryIpc {
        descriptor: descriptor_to_ipc(entry.descriptor),
        strength: entry.strength.value(),
        mounted_at_utc: entry.mounted_at_utc.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::Utc;
    use futures_util::stream;
    use handshake_core::model_runtime::{
        techniques::lora_hotswap::{
            FR_EVT_LLM_INFER_LORA_MOUNT, FR_EVT_LLM_INFER_LORA_SWAP, FR_EVT_LLM_INFER_LORA_UNMOUNT,
        },
        BaseModelTag, CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec,
        LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackOps, LoraStackSnapshot,
        LoraStackSnapshotEntry, LoraStrength, ModelCapabilities, ModelId, ModelRegistration,
        ModelRuntime, ModelRuntimeError, OperatorId, ProviderKind, RuntimeBinding, Score,
        SteeringHookHandle, TokenStream,
    };

    use crate::commands::{
        lora::{
            lora_list, lora_mount, lora_swap, lora_unmount, LoraCommandSettingsIpc,
            LoraDescriptorIpc, LoraExecPolicyIpc, LoraListRequestIpc, LoraMountRequestIpc,
            LoraStackItemIpc, LoraSwapRequestIpc, LoraUnmountRequestIpc, LORA_NOT_AVAILABLE_PREFIX,
        },
        model_runtime::ModelRuntimeState,
    };

    #[test]
    fn lora_commands_dtos_are_camel_case_and_expose_work_profile_exec_policy_knob() {
        let item = lora_stack_item("adapter", "local-test-base", 0.5);
        let value = serde_json::to_value(LoraSwapRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            stack: Vec::new(),
            settings: Some(LoraCommandSettingsIpc {
                exec_policy: Some(LoraExecPolicyIpc {
                    lora_stack: vec![item],
                }),
            }),
        })
        .expect("serialize swap request");

        assert!(value.get("modelId").is_some());
        assert!(value.get("model_id").is_none());
        assert!(
            value
                .pointer("/settings/execPolicy/loraStack/0/descriptor/artifactPath")
                .is_some(),
            "settings.execPolicy.loraStack is the Work Profile knob bridge"
        );
    }

    #[tokio::test]
    async fn lora_commands_mount_list_swap_and_unmount_dispatch_through_live_runtime() {
        let model_id = ModelId::new_v7();
        let stack = Arc::new(RecordingLoraStack::with_base_model("local-test-base"));
        let state = state_with_model(model_id, true);
        state
            .attach_live_runtime(
                model_id,
                Arc::new(RecordingRuntime::new(
                    model_id,
                    ModelCapabilities {
                        supports_lora: true,
                        ..Default::default()
                    },
                    stack.clone(),
                )),
            )
            .expect("attach live runtime");
        let first = descriptor_ipc("story", "local-test-base");
        let first_id = first.lora_id.clone();

        let mounted = lora_mount(
            LoraMountRequestIpc {
                model_id: model_id.to_string(),
                descriptor: first,
                strength: 0.75,
            },
            &state,
        )
        .await
        .expect("mount dispatches");
        assert_eq!(mounted.event_type, FR_EVT_LLM_INFER_LORA_MOUNT);
        assert_eq!(mounted.active_stack.len(), 1);
        assert_eq!(mounted.active_stack[0].lora_id, first_id);

        let listed = lora_list(
            LoraListRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect("list dispatches");
        assert_eq!(listed.active_stack.len(), 1);
        assert_eq!(listed.active_stack[0].lora_id, first_id);

        let second = lora_stack_item("domain", "local-test-base", 0.5);
        let second_id = second.descriptor.lora_id.clone();
        let swapped = lora_swap(
            LoraSwapRequestIpc {
                model_id: model_id.to_string(),
                stack: Vec::new(),
                settings: Some(LoraCommandSettingsIpc {
                    exec_policy: Some(LoraExecPolicyIpc {
                        lora_stack: vec![second],
                    }),
                }),
            },
            &state,
        )
        .await
        .expect("settings.execPolicy.loraStack swap dispatches");
        assert_eq!(swapped.event_type, FR_EVT_LLM_INFER_LORA_SWAP);
        assert_eq!(swapped.previous_stack.len(), 1);
        assert_eq!(swapped.active_stack[0].lora_id, second_id);

        let unmounted = lora_unmount(
            LoraUnmountRequestIpc {
                model_id: model_id.to_string(),
                lora_id: second_id,
            },
            &state,
        )
        .await
        .expect("unmount dispatches");
        assert_eq!(unmounted.event_type, FR_EVT_LLM_INFER_LORA_UNMOUNT);
        assert!(unmounted.active_stack.is_empty());
    }

    #[tokio::test]
    async fn lora_commands_fail_closed_when_registration_does_not_support_lora() {
        let model_id = ModelId::new_v7();
        let stack = Arc::new(RecordingLoraStack::with_base_model("local-test-base"));
        let state = state_with_model(model_id, false);
        state
            .attach_live_runtime(
                model_id,
                Arc::new(RecordingRuntime::new(
                    model_id,
                    ModelCapabilities {
                        supports_lora: true,
                        ..Default::default()
                    },
                    stack.clone(),
                )),
            )
            .expect("attach live runtime");

        let error = lora_mount(
            LoraMountRequestIpc {
                model_id: model_id.to_string(),
                descriptor: descriptor_ipc("story", "local-test-base"),
                strength: 1.0,
            },
            &state,
        )
        .await
        .expect_err("declared supports_lora=false must reject before runtime mutation");

        assert!(error.contains("lora"), "{error}");
        assert!(stack.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn lora_commands_return_typed_unavailable_when_no_live_runtime_is_attached() {
        let model_id = ModelId::new_v7();
        let state = state_with_model(model_id, true);

        let error = lora_list(
            LoraListRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect_err("registered and loaded model still needs a live runtime");

        assert!(error.contains(LORA_NOT_AVAILABLE_PREFIX), "{error}");
    }

    #[tokio::test]
    async fn lora_commands_reject_invalid_strength_before_runtime_mutation() {
        let model_id = ModelId::new_v7();
        let stack = Arc::new(RecordingLoraStack::with_base_model("local-test-base"));
        let state = state_with_model(model_id, true);
        state
            .attach_live_runtime(
                model_id,
                Arc::new(RecordingRuntime::new(
                    model_id,
                    ModelCapabilities {
                        supports_lora: true,
                        ..Default::default()
                    },
                    stack.clone(),
                )),
            )
            .expect("attach live runtime");

        let error = lora_mount(
            LoraMountRequestIpc {
                model_id: model_id.to_string(),
                descriptor: descriptor_ipc("story", "local-test-base"),
                strength: 3.0,
            },
            &state,
        )
        .await
        .expect_err("invalid strength fails before mount");

        assert!(error.contains("LoRA strength"), "{error}");
        assert!(stack.calls.lock().unwrap().is_empty());
    }

    fn state_with_model(model_id: ModelId, supports_lora: bool) -> ModelRuntimeState {
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(ModelRegistration {
                model_id,
                artifact_path: PathBuf::from("fixtures/models/local-test.gguf"),
                sha256: [9; 32],
                runtime_binding: RuntimeBinding::Candle,
                declared_capabilities: ModelCapabilities {
                    supports_lora,
                    ..Default::default()
                },
                base_model_tag: BaseModelTag::new("local-test-base"),
                registered_at_utc: Utc::now(),
                registered_by: OperatorId::new("operator-ilja"),
                provider: ProviderKind::Local,
            })
            .expect("register model");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");
        state
    }

    struct RecordingRuntime {
        model_capabilities: HashMap<ModelId, ModelCapabilities>,
        stack: Arc<RecordingLoraStack>,
    }

    impl RecordingRuntime {
        fn new(
            model_id: ModelId,
            capabilities: ModelCapabilities,
            stack: Arc<RecordingLoraStack>,
        ) -> Self {
            Self {
                model_capabilities: HashMap::from([(model_id, capabilities)]),
                stack,
            }
        }
    }

    #[async_trait]
    impl ModelRuntime for RecordingRuntime {
        fn adapter_name(&self) -> &'static str {
            "recording"
        }

        async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
            Err(ModelRuntimeError::LoadError(
                "recording runtime does not load models".to_string(),
            ))
        }

        async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
            Ok(())
        }

        fn generate(&self, _req: GenerateRequest) -> TokenStream {
            Box::pin(stream::empty())
        }

        async fn score(
            &self,
            _id: ModelId,
            _sequence: Vec<u32>,
        ) -> Result<Score, ModelRuntimeError> {
            Ok(Score {
                token_logprobs: Vec::new(),
                mean_logprob: 0.0,
            })
        }

        async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
            Ok(Embedding { vector: Vec::new() })
        }

        fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
            self.model_capabilities
                .get(&id)
                .ok_or_else(|| ModelRuntimeError::LoadError(format!("unknown model {id}")))
        }

        fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
            Ok(KvCacheHandle::new("recording-kv"))
        }

        fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
            Ok(LoraStackHandle::with_ops(
                "recording-lora-stack",
                self.stack.clone(),
            ))
        }

        fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
            Ok(SteeringHookHandle::new("recording-steering"))
        }

        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    struct RecordingLoraStack {
        base_model: BaseModelTag,
        active: Mutex<Vec<LoraStackSnapshotEntry>>,
        calls: Mutex<Vec<&'static str>>,
    }

    impl RecordingLoraStack {
        fn with_base_model(base_model: impl Into<String>) -> Self {
            Self {
                base_model: BaseModelTag::new(base_model),
                active: Mutex::new(Vec::new()),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl LoraStackOps for RecordingLoraStack {
        async fn mount(
            &self,
            desc: LoraDescriptor,
            strength: LoraStrength,
        ) -> Result<(), ModelRuntimeError> {
            self.calls.lock().unwrap().push("mount");
            if desc.base_model_compat != self.base_model {
                return Err(ModelRuntimeError::LoraStackError(
                    "base model mismatch".to_string(),
                ));
            }
            self.active.lock().unwrap().push(LoraStackSnapshotEntry {
                descriptor: desc,
                strength,
                mounted_at_utc: Utc::now(),
            });
            Ok(())
        }

        async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
            self.calls.lock().unwrap().push("unmount");
            self.active
                .lock()
                .unwrap()
                .retain(|entry| entry.descriptor.id != id);
            Ok(())
        }

        fn list_active(&self) -> Vec<LoraStackEntry> {
            self.active
                .lock()
                .unwrap()
                .iter()
                .map(|entry| LoraStackEntry {
                    id: entry.descriptor.id,
                    strength: entry.strength.clone(),
                    mounted_at_utc: entry.mounted_at_utc,
                })
                .collect()
        }

        async fn set_strength(
            &self,
            _id: LoraId,
            _strength: LoraStrength,
        ) -> Result<(), ModelRuntimeError> {
            Err(ModelRuntimeError::LoraStackError(
                "set_strength not used".to_string(),
            ))
        }

        async fn swap(
            &self,
            new_stack: Vec<(LoraDescriptor, LoraStrength)>,
        ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
            self.calls.lock().unwrap().push("swap");
            let previous = LoraStackSnapshot {
                entries: self.active.lock().unwrap().clone(),
            };
            *self.active.lock().unwrap() = new_stack
                .into_iter()
                .map(|(descriptor, strength)| LoraStackSnapshotEntry {
                    descriptor,
                    strength,
                    mounted_at_utc: Utc::now(),
                })
                .collect();
            Ok(previous)
        }
    }

    fn descriptor_ipc(name: &str, base_model: &str) -> LoraDescriptorIpc {
        LoraDescriptorIpc {
            lora_id: LoraId::new_v7().to_string(),
            artifact_path: format!("loras/{name}.safetensors"),
            sha256: "0707070707070707070707070707070707070707070707070707070707070707".to_string(),
            rank: 8,
            target_modules: vec!["q_proj".to_string()],
            base_model_compat: base_model.to_string(),
            license_tag: "operator-local".to_string(),
        }
    }

    fn lora_stack_item(name: &str, base_model: &str, strength: f32) -> LoraStackItemIpc {
        LoraStackItemIpc {
            descriptor: descriptor_ipc(name, base_model),
            strength,
        }
    }
}
