//! MT-093 — INF-2 KV cache Tauri IPC bridge.
//!
//! Wraps `handshake_core::model_runtime::techniques::kv_cache_technique::*`
//! with camelCase DTOs and the Work Profile knob bridge
//! `settings.execPolicy.kvCache.quantization`. Mirrors `commands::lora`
//! structurally: every command preflights through
//! `ModelRuntimeState::kv_cache_command_binding`, then requires a live
//! `Arc<dyn ModelRuntime>` and dispatches the technique surface.

use std::sync::Arc;

use handshake_core::model_runtime::{
    techniques::kv_cache_technique, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelId,
    ModelRuntime,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::model_runtime::{parse_model_id, KvCacheCommandGate, ModelRuntimeState};

pub const KERNEL_MODEL_RUNTIME_KV_SET_QUANTIZATION_IPC_CHANNEL: &str =
    "kernel_model_runtime_kv_set_quantization";
pub const KERNEL_MODEL_RUNTIME_KV_PREFIX_COMMIT_IPC_CHANNEL: &str =
    "kernel_model_runtime_kv_prefix_commit";
pub const KERNEL_MODEL_RUNTIME_KV_PREFIX_RESTORE_IPC_CHANNEL: &str =
    "kernel_model_runtime_kv_prefix_restore";
pub const KERNEL_MODEL_RUNTIME_KV_EVICT_ALL_IPC_CHANNEL: &str = "kernel_model_runtime_kv_evict_all";
pub const KERNEL_MODEL_RUNTIME_KV_OCCUPANCY_IPC_CHANNEL: &str = "kernel_model_runtime_kv_occupancy";
pub const KV_NOT_AVAILABLE_PREFIX: &str = "kv_not_available";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheStatsIpc {
    pub bytes_used: u64,
    pub bytes_capacity: u64,
    pub prefix_cache_entries: u32,
    pub prefix_cache_hit_count: u64,
    pub prefix_cache_miss_count: u64,
    pub quant_level_current: KvQuantSupport,
}

impl From<KvCacheStats> for KvCacheStatsIpc {
    fn from(stats: KvCacheStats) -> Self {
        Self {
            bytes_used: stats.bytes_used,
            bytes_capacity: stats.bytes_capacity,
            prefix_cache_entries: stats.prefix_cache_entries,
            prefix_cache_hit_count: stats.prefix_cache_hit_count,
            prefix_cache_miss_count: stats.prefix_cache_miss_count,
            quant_level_current: stats.quant_level_current,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvPrefixHandleIpc {
    pub prefix_id: String,
    pub content_hash_hex: String,
    pub token_count: u32,
}

impl From<KvPrefixHandle> for KvPrefixHandleIpc {
    fn from(handle: KvPrefixHandle) -> Self {
        Self {
            prefix_id: handle.prefix_id().to_string(),
            content_hash_hex: hex::encode(handle.content_hash()),
            token_count: handle.token_count(),
        }
    }
}

impl KvPrefixHandleIpc {
    pub fn try_into_handle(self) -> Result<KvPrefixHandle, String> {
        let prefix_id = Uuid::parse_str(self.prefix_id.trim())
            .map_err(|error| format!("invalid prefix_id: {error}"))?;
        let bytes = hex::decode(self.content_hash_hex.trim())
            .map_err(|error| format!("invalid content_hash_hex: {error}"))?;
        let content_hash: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            format!("content_hash must decode to 32 bytes, got {}", bytes.len())
        })?;
        KvPrefixHandle::from_parts(prefix_id, content_hash, self.token_count)
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheExecPolicyIpc {
    pub quantization: Option<KvQuantSupport>,
    pub prefix_cache_ttl_seconds: Option<u64>,
    pub max_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheCommandSettingsIpc {
    #[serde(default)]
    pub exec_policy: Option<KvCacheExecPolicyIpc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvSetQuantizationRequestIpc {
    pub model_id: String,
    pub level: Option<KvQuantSupport>,
    #[serde(default)]
    pub settings: Option<KvCacheCommandSettingsIpc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvSetQuantizationResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub previous_quantization: KvQuantSupport,
    pub current_quantization: KvQuantSupport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvPrefixCommitRequestIpc {
    pub model_id: String,
    pub prefix_tokens: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvPrefixCommitResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandleIpc,
    pub occupancy: KvCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvPrefixRestoreRequestIpc {
    pub model_id: String,
    pub prefix_handle: KvPrefixHandleIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvPrefixRestoreResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub prefix_handle: KvPrefixHandleIpc,
    pub occupancy: KvCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvEvictAllRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvEvictAllResultIpc {
    pub model_id: String,
    pub event_type: String,
    pub previous_occupancy: KvCacheStatsIpc,
    pub current_occupancy: KvCacheStatsIpc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvOccupancyRequestIpc {
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvOccupancyResultIpc {
    pub model_id: String,
    pub occupancy: KvCacheStatsIpc,
}

#[tauri::command]
pub async fn kernel_model_runtime_kv_set_quantization(
    request: KvSetQuantizationRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<KvSetQuantizationResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_KV_SET_QUANTIZATION_IPC_CHANNEL;
    kv_set_quantization(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_kv_prefix_commit(
    request: KvPrefixCommitRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<KvPrefixCommitResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_KV_PREFIX_COMMIT_IPC_CHANNEL;
    kv_prefix_commit(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_kv_prefix_restore(
    request: KvPrefixRestoreRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<KvPrefixRestoreResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_KV_PREFIX_RESTORE_IPC_CHANNEL;
    kv_prefix_restore(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_kv_evict_all(
    request: KvEvictAllRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<KvEvictAllResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_KV_EVICT_ALL_IPC_CHANNEL;
    kv_evict_all(request, state.inner()).await
}

#[tauri::command]
pub async fn kernel_model_runtime_kv_occupancy(
    request: KvOccupancyRequestIpc,
    state: State<'_, ModelRuntimeState>,
) -> Result<KvOccupancyResultIpc, String> {
    let _ = KERNEL_MODEL_RUNTIME_KV_OCCUPANCY_IPC_CHANNEL;
    kv_occupancy(request, state.inner()).await
}

pub async fn kv_set_quantization(
    request: KvSetQuantizationRequestIpc,
    state: &ModelRuntimeState,
) -> Result<KvSetQuantizationResultIpc, String> {
    let model_id =
        preflight_kv_capability(&request.model_id, state, KvCacheCommandGate::Quantization)?;
    let runtime = require_live_runtime(model_id, state)?;
    let level = effective_quantization(&request).ok_or_else(|| {
        "set_quantization requires either request.level or settings.execPolicy.kvCache.quantization"
            .to_string()
    })?;
    let receipt = kv_cache_technique::set_quantization(runtime.as_ref(), model_id, level)
        .map_err(|error| format!("kv set_quantization dispatch failed: {error}"))?;
    Ok(KvSetQuantizationResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        previous_quantization: receipt.previous_quantization,
        current_quantization: receipt.current_quantization,
    })
}

pub async fn kv_prefix_commit(
    request: KvPrefixCommitRequestIpc,
    state: &ModelRuntimeState,
) -> Result<KvPrefixCommitResultIpc, String> {
    let model_id =
        preflight_kv_capability(&request.model_id, state, KvCacheCommandGate::PrefixCache)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt =
        kv_cache_technique::prefix_commit(runtime.as_ref(), model_id, &request.prefix_tokens)
            .map_err(|error| format!("kv prefix_commit dispatch failed: {error}"))?;
    Ok(KvPrefixCommitResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        prefix_handle: receipt.prefix_handle.into(),
        occupancy: receipt.occupancy.into(),
    })
}

pub async fn kv_prefix_restore(
    request: KvPrefixRestoreRequestIpc,
    state: &ModelRuntimeState,
) -> Result<KvPrefixRestoreResultIpc, String> {
    let model_id =
        preflight_kv_capability(&request.model_id, state, KvCacheCommandGate::PrefixCache)?;
    let runtime = require_live_runtime(model_id, state)?;
    let handle = request.prefix_handle.try_into_handle()?;
    let receipt = kv_cache_technique::prefix_restore(runtime.as_ref(), model_id, &handle)
        .map_err(|error| format!("kv prefix_restore dispatch failed: {error}"))?;
    Ok(KvPrefixRestoreResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        prefix_handle: receipt.prefix_handle.into(),
        occupancy: receipt.occupancy.into(),
    })
}

pub async fn kv_evict_all(
    request: KvEvictAllRequestIpc,
    state: &ModelRuntimeState,
) -> Result<KvEvictAllResultIpc, String> {
    let model_id =
        preflight_kv_capability(&request.model_id, state, KvCacheCommandGate::PrefixCache)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = kv_cache_technique::evict_all(runtime.as_ref(), model_id)
        .map_err(|error| format!("kv evict_all dispatch failed: {error}"))?;
    Ok(KvEvictAllResultIpc {
        model_id: receipt.model_id.to_string(),
        event_type: receipt.event_type,
        previous_occupancy: receipt.previous_occupancy.into(),
        current_occupancy: receipt.current_occupancy.into(),
    })
}

pub async fn kv_occupancy(
    request: KvOccupancyRequestIpc,
    state: &ModelRuntimeState,
) -> Result<KvOccupancyResultIpc, String> {
    let model_id =
        preflight_kv_capability(&request.model_id, state, KvCacheCommandGate::Telemetry)?;
    let runtime = require_live_runtime(model_id, state)?;
    let receipt = kv_cache_technique::occupancy(runtime.as_ref(), model_id)
        .map_err(|error| format!("kv occupancy dispatch failed: {error}"))?;
    Ok(KvOccupancyResultIpc {
        model_id: receipt.model_id.to_string(),
        occupancy: receipt.occupancy.into(),
    })
}

fn preflight_kv_capability(
    model_id: &str,
    state: &ModelRuntimeState,
    gate: KvCacheCommandGate,
) -> Result<ModelId, String> {
    let model_id = parse_model_id(model_id)?;
    state.kv_cache_command_binding(model_id, gate)?;
    Ok(model_id)
}

fn require_live_runtime(
    model_id: ModelId,
    state: &ModelRuntimeState,
) -> Result<Arc<dyn ModelRuntime>, String> {
    state.live_runtime(model_id)?.ok_or_else(|| {
        format!(
            "{KV_NOT_AVAILABLE_PREFIX}: KV cache technique requires a live ModelRuntime adapter attached for model {model_id}; the adapter is not yet bound to this model in this app session"
        )
    })
}

fn effective_quantization(request: &KvSetQuantizationRequestIpc) -> Option<KvQuantSupport> {
    // Work Profile precedence: settings.execPolicy.kvCache.quantization
    // wins when present so the operator's saved policy outranks an
    // ad-hoc request value. Falls back to request.level otherwise.
    if let Some(settings) = request.settings.as_ref() {
        if let Some(policy) = settings.exec_policy.as_ref() {
            if let Some(level) = policy.quantization {
                return Some(level);
            }
        }
    }
    request.level
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::model_runtime::{
        techniques::kv_cache_technique::{
            FR_EVT_LLM_INFER_KV_EVICT, FR_EVT_LLM_INFER_KV_PREFIX_COMMIT,
            FR_EVT_LLM_INFER_KV_PREFIX_RESTORE, FR_EVT_LLM_INFER_KV_SET_QUANTIZATION,
        },
        BaseModelTag, KvCacheHandle, KvCacheOps, ModelCapabilities, ModelRegistration, OperatorId,
        ProviderKind, RuntimeBinding,
    };
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::Utc;
    use futures_util::stream;
    use handshake_core::model_runtime::{
        CancellationToken, Embedding, GenerateRequest, KvCacheStats, KvPrefixHandle, LoadSpec,
        LoraStackHandle, Score, SteeringHookHandle, TokenStream,
    };

    #[test]
    fn kv_cache_dtos_are_camel_case_and_expose_work_profile_exec_policy_knob() {
        let value = serde_json::to_value(KvSetQuantizationRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            level: Some(KvQuantSupport::Q4),
            settings: Some(KvCacheCommandSettingsIpc {
                exec_policy: Some(KvCacheExecPolicyIpc {
                    quantization: Some(KvQuantSupport::Q8),
                    prefix_cache_ttl_seconds: Some(300),
                    max_bytes: None,
                }),
            }),
        })
        .expect("serialize set_quantization request");

        assert!(value.get("modelId").is_some());
        assert!(value.get("model_id").is_none());
        assert!(
            value.pointer("/settings/execPolicy/quantization").is_some(),
            "settings.execPolicy.quantization is the Work Profile knob bridge"
        );
    }

    #[test]
    fn effective_quantization_prefers_exec_policy_over_request_level() {
        let request = KvSetQuantizationRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            level: Some(KvQuantSupport::Q4),
            settings: Some(KvCacheCommandSettingsIpc {
                exec_policy: Some(KvCacheExecPolicyIpc {
                    quantization: Some(KvQuantSupport::Q8),
                    prefix_cache_ttl_seconds: None,
                    max_bytes: None,
                }),
            }),
        };
        assert_eq!(effective_quantization(&request), Some(KvQuantSupport::Q8));
    }

    #[test]
    fn effective_quantization_falls_back_to_request_level_when_exec_policy_empty() {
        let request = KvSetQuantizationRequestIpc {
            model_id: ModelId::new_v7().to_string(),
            level: Some(KvQuantSupport::Q4),
            settings: None,
        };
        assert_eq!(effective_quantization(&request), Some(KvQuantSupport::Q4));
    }

    #[tokio::test]
    async fn kv_commands_dispatch_through_live_runtime_and_emit_canonical_event_types() {
        let model_id = ModelId::new_v7();
        let state = state_with_model(
            model_id,
            ModelCapabilities {
                supports_kv_prefix_cache: true,
                supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
                ..Default::default()
            },
        );
        let stack = Arc::new(IpcKvStack::new(KvQuantSupport::Q4));
        state
            .attach_live_runtime(model_id, Arc::new(IpcRuntime::new(model_id, stack.clone())))
            .expect("attach live runtime");

        let set = kv_set_quantization(
            KvSetQuantizationRequestIpc {
                model_id: model_id.to_string(),
                level: None,
                settings: Some(KvCacheCommandSettingsIpc {
                    exec_policy: Some(KvCacheExecPolicyIpc {
                        quantization: Some(KvQuantSupport::Q8),
                        prefix_cache_ttl_seconds: None,
                        max_bytes: None,
                    }),
                }),
            },
            &state,
        )
        .await
        .expect("set_quantization dispatches");
        assert_eq!(set.event_type, FR_EVT_LLM_INFER_KV_SET_QUANTIZATION);
        assert_eq!(set.current_quantization, KvQuantSupport::Q8);

        let commit = kv_prefix_commit(
            KvPrefixCommitRequestIpc {
                model_id: model_id.to_string(),
                prefix_tokens: vec![1, 2, 3],
            },
            &state,
        )
        .await
        .expect("prefix_commit dispatches");
        assert_eq!(commit.event_type, FR_EVT_LLM_INFER_KV_PREFIX_COMMIT);
        let restored_handle = commit.prefix_handle.clone();

        let restore = kv_prefix_restore(
            KvPrefixRestoreRequestIpc {
                model_id: model_id.to_string(),
                prefix_handle: restored_handle,
            },
            &state,
        )
        .await
        .expect("prefix_restore dispatches");
        assert_eq!(restore.event_type, FR_EVT_LLM_INFER_KV_PREFIX_RESTORE);

        let evict = kv_evict_all(
            KvEvictAllRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect("evict_all dispatches");
        assert_eq!(evict.event_type, FR_EVT_LLM_INFER_KV_EVICT);
        assert_eq!(evict.current_occupancy.prefix_cache_entries, 0);

        let occupancy = kv_occupancy(
            KvOccupancyRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect("occupancy dispatches");
        assert_eq!(occupancy.occupancy.quant_level_current, KvQuantSupport::Q8);
    }

    #[tokio::test]
    async fn kv_commands_fail_closed_when_registration_does_not_support_capability() {
        let model_id = ModelId::new_v7();
        let state = state_with_model(model_id, ModelCapabilities::default());
        let stack = Arc::new(IpcKvStack::new(KvQuantSupport::None));
        state
            .attach_live_runtime(model_id, Arc::new(IpcRuntime::new(model_id, stack.clone())))
            .expect("attach live runtime");

        let err = kv_set_quantization(
            KvSetQuantizationRequestIpc {
                model_id: model_id.to_string(),
                level: Some(KvQuantSupport::Q4),
                settings: None,
            },
            &state,
        )
        .await
        .expect_err("declared supports_kv_quantization=None must reject before runtime mutation");
        assert!(err.contains("kv_cache_quantization"), "{err}");
    }

    #[tokio::test]
    async fn kv_commands_return_typed_unavailable_when_no_live_runtime_is_attached() {
        let model_id = ModelId::new_v7();
        let state = state_with_model(
            model_id,
            ModelCapabilities {
                supports_kv_prefix_cache: true,
                supports_kv_quantization: KvQuantSupport::Q4,
                ..Default::default()
            },
        );

        let err = kv_occupancy(
            KvOccupancyRequestIpc {
                model_id: model_id.to_string(),
            },
            &state,
        )
        .await
        .expect_err("registered and loaded model still needs a live runtime");
        assert!(err.contains(KV_NOT_AVAILABLE_PREFIX), "{err}");
    }

    fn state_with_model(model_id: ModelId, capabilities: ModelCapabilities) -> ModelRuntimeState {
        let state = ModelRuntimeState::default();
        state
            .register_for_tests(ModelRegistration {
                model_id,
                artifact_path: PathBuf::from("fixtures/models/local-test.gguf"),
                sha256: [9; 32],
                runtime_binding: RuntimeBinding::Candle,
                declared_capabilities: capabilities,
                base_model_tag: BaseModelTag::new("local-test-base"),
                registered_at_utc: Utc::now(),
                registered_by: OperatorId::new("operator-ilja"),
                provider: ProviderKind::Local,
            })
            .expect("register model");
        state.mark_loaded_for_tests(model_id).expect("mark loaded");
        state
    }

    struct IpcRuntime {
        model_capabilities: std::collections::HashMap<ModelId, ModelCapabilities>,
        stack: Arc<IpcKvStack>,
    }

    impl IpcRuntime {
        fn new(model_id: ModelId, stack: Arc<IpcKvStack>) -> Self {
            Self {
                model_capabilities: std::collections::HashMap::from([(
                    model_id,
                    ModelCapabilities {
                        supports_kv_prefix_cache: true,
                        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
                        ..Default::default()
                    },
                )]),
                stack,
            }
        }
    }

    #[async_trait]
    impl ModelRuntime for IpcRuntime {
        fn adapter_name(&self) -> &'static str {
            "ipc_kv_test"
        }

        async fn load(
            &mut self,
            _spec: LoadSpec,
        ) -> Result<ModelId, handshake_core::model_runtime::ModelRuntimeError> {
            Err(handshake_core::model_runtime::ModelRuntimeError::LoadError(
                "ipc kv test runtime does not load models".to_string(),
            ))
        }

        async fn unload(
            &mut self,
            _id: ModelId,
        ) -> Result<(), handshake_core::model_runtime::ModelRuntimeError> {
            Ok(())
        }

        fn generate(&self, _req: GenerateRequest) -> TokenStream {
            Box::pin(stream::empty())
        }

        async fn score(
            &self,
            _id: ModelId,
            _sequence: Vec<u32>,
        ) -> Result<Score, handshake_core::model_runtime::ModelRuntimeError> {
            Ok(Score {
                token_logprobs: Vec::new(),
                mean_logprob: 0.0,
            })
        }

        async fn embed(
            &self,
            _id: ModelId,
            _text: &str,
        ) -> Result<Embedding, handshake_core::model_runtime::ModelRuntimeError> {
            Ok(Embedding { vector: Vec::new() })
        }

        fn capabilities(
            &self,
            id: ModelId,
        ) -> Result<&ModelCapabilities, handshake_core::model_runtime::ModelRuntimeError> {
            self.model_capabilities.get(&id).ok_or_else(|| {
                handshake_core::model_runtime::ModelRuntimeError::LoadError(format!(
                    "unknown model {id}"
                ))
            })
        }

        fn kv_cache(
            &self,
            _id: ModelId,
        ) -> Result<KvCacheHandle, handshake_core::model_runtime::ModelRuntimeError> {
            Ok(KvCacheHandle::with_ops("ipc-kv-test", self.stack.clone()))
        }

        fn lora_stack(
            &self,
            _id: ModelId,
        ) -> Result<LoraStackHandle, handshake_core::model_runtime::ModelRuntimeError> {
            Ok(LoraStackHandle::new("ipc-kv-lora"))
        }

        fn steering_hooks(
            &self,
            _id: ModelId,
        ) -> Result<SteeringHookHandle, handshake_core::model_runtime::ModelRuntimeError> {
            Ok(SteeringHookHandle::new("ipc-kv-steering"))
        }

        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    struct IpcKvStack {
        state: Mutex<IpcKvState>,
    }

    struct IpcKvState {
        quantization: KvQuantSupport,
        prefixes: std::collections::HashMap<uuid::Uuid, ([u8; 32], u32)>,
    }

    impl IpcKvStack {
        fn new(initial: KvQuantSupport) -> Self {
            Self {
                state: Mutex::new(IpcKvState {
                    quantization: initial,
                    prefixes: std::collections::HashMap::new(),
                }),
            }
        }
    }

    impl KvCacheOps for IpcKvStack {
        fn quantization(&self) -> KvQuantSupport {
            self.state.lock().unwrap().quantization
        }

        fn set_quantization(
            &self,
            level: KvQuantSupport,
        ) -> Result<(), handshake_core::model_runtime::ModelRuntimeError> {
            self.state.lock().unwrap().quantization = level;
            Ok(())
        }

        fn occupancy(&self) -> KvCacheStats {
            let state = self.state.lock().unwrap();
            KvCacheStats {
                bytes_used: 0,
                bytes_capacity: 0,
                prefix_cache_entries: state.prefixes.len() as u32,
                prefix_cache_hit_count: 0,
                prefix_cache_miss_count: 0,
                quant_level_current: state.quantization,
            }
        }

        fn prefix_commit(
            &self,
            prefix_tokens: &[u32],
        ) -> Result<KvPrefixHandle, handshake_core::model_runtime::ModelRuntimeError> {
            let handle = KvPrefixHandle::from_tokens(prefix_tokens)?;
            self.state.lock().unwrap().prefixes.insert(
                handle.prefix_id(),
                (*handle.content_hash(), handle.token_count()),
            );
            Ok(handle)
        }

        fn prefix_restore(
            &self,
            handle: &KvPrefixHandle,
        ) -> Result<(), handshake_core::model_runtime::ModelRuntimeError> {
            let state = self.state.lock().unwrap();
            match state.prefixes.get(&handle.prefix_id()) {
                Some((expected, _)) if expected == handle.content_hash() => Ok(()),
                _ => Err(
                    handshake_core::model_runtime::ModelRuntimeError::KvCacheError(format!(
                        "unknown prefix {}",
                        handle.prefix_id()
                    )),
                ),
            }
        }

        fn prefix_evict(
            &self,
            handle: KvPrefixHandle,
        ) -> Result<(), handshake_core::model_runtime::ModelRuntimeError> {
            self.state
                .lock()
                .unwrap()
                .prefixes
                .remove(&handle.prefix_id());
            Ok(())
        }

        fn evict_all(&self) -> Result<(), handshake_core::model_runtime::ModelRuntimeError> {
            self.state.lock().unwrap().prefixes.clear();
            Ok(())
        }
    }
}
