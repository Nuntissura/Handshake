#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::model_runtime::{
    KvCacheHandle, KvCacheOps, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelRuntimeError,
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use super::context::NativeLlamaCppBackend;

const LLAMA_CPP_KV_PREFIX_SCOPE_DOMAIN: &[u8] = b"handshake.llama_cpp.kv_prefix.v1";

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug)]
struct PrefixEntry {
    content_hash: [u8; 32],
    token_count: u32,
    created_at: Instant,
    state: Vec<u8>,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[derive(Debug)]
struct LlamaCppKvCacheState {
    quantization: KvQuantSupport,
    prefixes: HashMap<uuid::Uuid, PrefixEntry>,
    lru_order: VecDeque<uuid::Uuid>,
    hits: u64,
    misses: u64,
}

#[derive(Debug)]
pub struct LlamaCppKvCache {
    handle: KvCacheHandle,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    native: Arc<NativeLlamaCppBackend>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    scope: Vec<u8>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    max_bytes: Option<u64>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    prefix_cache_ttl: Option<Duration>,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    supported_quantization: KvQuantSupport,
    #[cfg(feature = "llama-cpp-runtime-engine")]
    state: Mutex<LlamaCppKvCacheState>,
}

impl LlamaCppKvCache {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn new(
        handle: KvCacheHandle,
        native: Arc<NativeLlamaCppBackend>,
        initial_quantization: KvQuantSupport,
        supported_quantization: KvQuantSupport,
        prefix_cache_ttl_seconds: u64,
        max_bytes: Option<u64>,
        scope: Vec<u8>,
    ) -> Self {
        Self {
            handle,
            native,
            scope,
            max_bytes,
            prefix_cache_ttl: (prefix_cache_ttl_seconds > 0)
                .then(|| Duration::from_secs(prefix_cache_ttl_seconds)),
            supported_quantization,
            state: Mutex::new(LlamaCppKvCacheState {
                quantization: initial_quantization,
                prefixes: HashMap::new(),
                lru_order: VecDeque::new(),
                hits: 0,
                misses: 0,
            }),
        }
    }

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    pub(super) fn new(handle: KvCacheHandle) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> KvCacheHandle {
        self.handle.clone()
    }

    pub(super) fn scope_for_model(
        model_id: crate::model_runtime::ModelId,
        sha256: &str,
    ) -> Vec<u8> {
        let mut scope = Vec::new();
        scope.extend_from_slice(LLAMA_CPP_KV_PREFIX_SCOPE_DOMAIN);
        scope.extend_from_slice(model_id.as_uuid().as_bytes());
        scope.extend_from_slice((sha256.len() as u64).to_le_bytes().as_slice());
        scope.extend_from_slice(sha256.as_bytes());
        scope
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn validate_prompt_prefix(
        &self,
        handle: &KvPrefixHandle,
        prompt_tokens: &[llama_cpp_2::token::LlamaToken],
    ) -> Result<usize, ModelRuntimeError> {
        let token_count = usize::try_from(handle.token_count()).map_err(|error| {
            ModelRuntimeError::KvCacheError(format!(
                "prefix token count does not fit usize: {error}"
            ))
        })?;
        if token_count == 0 || token_count > prompt_tokens.len() {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "prefix handle token_count {} is not valid for prompt token count {}",
                handle.token_count(),
                prompt_tokens.len()
            )));
        }
        let prefix_ids = prompt_tokens[..token_count]
            .iter()
            .copied()
            .map(|token| {
                u32::try_from(token.0).map_err(|error| {
                    ModelRuntimeError::KvCacheError(format!(
                        "llama.cpp prompt token id does not fit u32: {error}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected = KvPrefixHandle::content_hash_for_scope_and_tokens(&self.scope, &prefix_ids);
        if &expected != handle.content_hash() {
            return Err(ModelRuntimeError::KvCacheError(
                "prefix handle content_hash mismatch for prompt prefix".to_string(),
            ));
        }
        Ok(token_count)
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(super) fn restore_into_context(
        &self,
        handle: &KvPrefixHandle,
        context: &mut llama_cpp_2::context::LlamaContext<'_>,
    ) -> Result<(), ModelRuntimeError> {
        let state = {
            let mut state = self.lock_state()?;
            self.purge_expired_locked(&mut state);
            let prefix_id = handle.prefix_id();
            match state.prefixes.get(&handle.prefix_id()) {
                Some(entry) if &entry.content_hash == handle.content_hash() => {
                    if entry.token_count != handle.token_count() {
                        state.misses = state.misses.saturating_add(1);
                        return Err(ModelRuntimeError::KvCacheError(
                            "prefix handle token_count mismatch".to_string(),
                        ));
                    }
                    let state_bytes = entry.state.clone();
                    touch_prefix_locked(&mut state, prefix_id);
                    state_bytes
                }
                Some(_) => {
                    state.misses = state.misses.saturating_add(1);
                    return Err(ModelRuntimeError::KvCacheError(
                        "prefix handle content_hash mismatch".to_string(),
                    ));
                }
                None => {
                    state.misses = state.misses.saturating_add(1);
                    return Err(ModelRuntimeError::KvCacheError(
                        "unknown prefix handle".to_string(),
                    ));
                }
            }
        };

        let restored = unsafe { context.set_state_data(&state) };
        if restored == 0 {
            self.record_miss()?;
            return Err(ModelRuntimeError::KvCacheError(
                "llama.cpp state restore returned zero bytes".to_string(),
            ));
        }
        self.record_hit()
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn capture_prefix_state(&self, prefix_tokens: &[u32]) -> Result<Vec<u8>, ModelRuntimeError> {
        use llama_cpp_2::{llama_batch::LlamaBatch, token::LlamaToken};

        if prefix_tokens.is_empty() {
            return Err(ModelRuntimeError::KvCacheError(
                "prefix_commit requires at least one prefix token".to_string(),
            ));
        }

        let mut context = self.native.new_context(self.quantization())?;
        let mut batch = LlamaBatch::new(prefix_tokens.len(), 1);
        let last_index = prefix_tokens.len().saturating_sub(1);
        for (position, token_id) in prefix_tokens.iter().copied().enumerate() {
            let token_id = i32::try_from(token_id).map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "prefix token id does not fit i32 for llama.cpp: {error}"
                ))
            })?;
            let position_i32 = i32::try_from(position).map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "prefix token position does not fit i32 for llama.cpp: {error}"
                ))
            })?;
            batch
                .add(
                    LlamaToken::new(token_id),
                    position_i32,
                    &[0],
                    position == last_index,
                )
                .map_err(|error| {
                    ModelRuntimeError::KvCacheError(format!(
                        "failed to add prefix token to llama.cpp batch: {error}"
                    ))
                })?;
        }
        context.decode(&mut batch).map_err(|error| {
            ModelRuntimeError::KvCacheError(format!("llama.cpp prefix decode failed: {error}"))
        })?;

        let size = context.get_state_size();
        if size == 0 {
            return Err(ModelRuntimeError::KvCacheError(
                "llama.cpp state size is zero after prefix decode".to_string(),
            ));
        }
        let mut state = vec![0_u8; size];
        let copied = unsafe { context.copy_state_data(state.as_mut_ptr()) };
        if copied == 0 || copied > state.len() {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "llama.cpp state copy returned invalid byte count {copied}"
            )));
        }
        state.truncate(copied);
        Ok(state)
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, LlamaCppKvCacheState>, ModelRuntimeError> {
        self.state.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("llama.cpp KV cache state lock is poisoned".to_string())
        })
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn record_hit(&self) -> Result<(), ModelRuntimeError> {
        let mut state = self.lock_state()?;
        state.hits = state.hits.saturating_add(1);
        Ok(())
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn record_miss(&self) -> Result<(), ModelRuntimeError> {
        let mut state = self.lock_state()?;
        state.misses = state.misses.saturating_add(1);
        Ok(())
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn purge_expired_locked(&self, state: &mut LlamaCppKvCacheState) {
        let Some(ttl) = self.prefix_cache_ttl else {
            return;
        };
        let now = Instant::now();
        let expired = state
            .prefixes
            .iter()
            .filter_map(|(prefix_id, entry)| {
                (now.duration_since(entry.created_at) >= ttl).then_some(*prefix_id)
            })
            .collect::<Vec<_>>();
        for prefix_id in expired {
            remove_prefix_locked(state, prefix_id);
        }
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    fn prune_to_budget_locked(&self, state: &mut LlamaCppKvCacheState) {
        self.purge_expired_locked(state);
        let Some(max_bytes) = self.max_bytes else {
            return;
        };
        while prefix_bytes_locked(state) > max_bytes && state.prefixes.len() > 1 {
            let Some(oldest) = state.lru_order.pop_front() else {
                break;
            };
            state.prefixes.remove(&oldest);
        }
    }
}

impl KvCacheOps for LlamaCppKvCache {
    fn quantization(&self) -> KvQuantSupport {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            return self
                .state
                .lock()
                .map(|state| state.quantization)
                .unwrap_or(KvQuantSupport::None);
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            KvQuantSupport::None
        }
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            if !quantization_supported(level, self.supported_quantization) {
                return Err(ModelRuntimeError::CapabilityNotSupported {
                    capability: format!("llama_cpp_kv_cache_quantization.{level:?}"),
                    adapter: "llama_cpp".to_string(),
                });
            }
            let mut state = self.lock_state()?;
            if state.quantization != level {
                state.prefixes.clear();
                state.lru_order.clear();
                state.quantization = level;
            }
            return Ok(());
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            if level == KvQuantSupport::None {
                Ok(())
            } else {
                Err(ModelRuntimeError::CapabilityNotSupported {
                    capability: format!("llama_cpp_kv_cache_quantization.{level:?}"),
                    adapter: "llama_cpp_native_feature_disabled".to_string(),
                })
            }
        }
    }

    fn occupancy(&self) -> KvCacheStats {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let Ok(mut state) = self.state.lock() else {
                return KvCacheStats {
                    bytes_used: 0,
                    bytes_capacity: 0,
                    prefix_cache_entries: 0,
                    prefix_cache_hit_count: 0,
                    prefix_cache_miss_count: 0,
                    quant_level_current: KvQuantSupport::None,
                };
            };
            self.purge_expired_locked(&mut state);
            let bytes_used = prefix_bytes_locked(&state);
            let bytes_capacity = self
                .max_bytes
                .unwrap_or_else(|| self.native.estimated_kv_capacity_bytes(state.quantization))
                .max(bytes_used);
            return KvCacheStats {
                bytes_used,
                bytes_capacity,
                prefix_cache_entries: state.prefixes.len() as u32,
                prefix_cache_hit_count: state.hits,
                prefix_cache_miss_count: state.misses,
                quant_level_current: state.quantization,
            };
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            KvCacheStats {
                bytes_used: 0,
                bytes_capacity: 0,
                prefix_cache_entries: 0,
                prefix_cache_hit_count: 0,
                prefix_cache_miss_count: 0,
                quant_level_current: KvQuantSupport::None,
            }
        }
    }

    /// MT-095: expose the policy TTL so the `KvCacheHandle` restore chokepoint
    /// enforces expiry uniformly. (The native path also purges expired entries
    /// internally via `purge_expired_locked`; this surfaces the same TTL as a
    /// typed expiry error at the public restore gate.)
    fn prefix_cache_ttl_seconds(&self) -> u64 {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            return self.prefix_cache_ttl.map(|d| d.as_secs()).unwrap_or(0);
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            0
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let handle = KvPrefixHandle::from_scoped_tokens(&self.scope, prefix_tokens)?;
            let state_bytes = self.capture_prefix_state(prefix_tokens)?;
            let mut state = self.lock_state()?;
            state.prefixes.insert(
                handle.prefix_id(),
                PrefixEntry {
                    content_hash: *handle.content_hash(),
                    token_count: handle.token_count(),
                    created_at: Instant::now(),
                    state: state_bytes,
                },
            );
            touch_prefix_locked(&mut state, handle.prefix_id());
            self.prune_to_budget_locked(&mut state);
            return Ok(handle);
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            let _ = prefix_tokens;
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "llama_cpp_kv_cache_prefix_commit".to_string(),
                adapter: "llama_cpp_native_feature_disabled".to_string(),
            })
        }
    }

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let mut context = self.native.new_context(self.quantization())?;
            return self.restore_into_context(handle, &mut context);
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            let _ = handle;
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "llama_cpp_kv_cache_prefix_restore".to_string(),
                adapter: "llama_cpp_native_feature_disabled".to_string(),
            })
        }
    }

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let mut state = self.lock_state()?;
            remove_prefix_locked(&mut state, handle.prefix_id());
            return Ok(());
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            let _ = handle;
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "llama_cpp_kv_cache_prefix_evict".to_string(),
                adapter: "llama_cpp_native_feature_disabled".to_string(),
            })
        }
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        #[cfg(feature = "llama-cpp-runtime-engine")]
        {
            let mut state = self.lock_state()?;
            state.prefixes.clear();
            state.lru_order.clear();
            drop(state);
            let mut context = self.native.new_context(self.quantization())?;
            context.clear_kv_cache();
            return Ok(());
        }
        #[cfg(not(feature = "llama-cpp-runtime-engine"))]
        {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "llama_cpp_kv_cache_evict_all".to_string(),
                adapter: "llama_cpp_native_feature_disabled".to_string(),
            })
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn prefix_bytes_locked(state: &LlamaCppKvCacheState) -> u64 {
    state
        .prefixes
        .values()
        .map(|entry| entry.state.len() as u64)
        .sum::<u64>()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn touch_prefix_locked(state: &mut LlamaCppKvCacheState, prefix_id: uuid::Uuid) {
    state.lru_order.retain(|existing| *existing != prefix_id);
    if state.prefixes.contains_key(&prefix_id) {
        state.lru_order.push_back(prefix_id);
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn remove_prefix_locked(state: &mut LlamaCppKvCacheState, prefix_id: uuid::Uuid) {
    state.prefixes.remove(&prefix_id);
    state.lru_order.retain(|existing| *existing != prefix_id);
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn quantization_supported(requested: KvQuantSupport, supported: KvQuantSupport) -> bool {
    match (requested, supported) {
        (KvQuantSupport::None, _) => true,
        (KvQuantSupport::Q4, KvQuantSupport::Q4 | KvQuantSupport::Q4Q8Mix) => true,
        (KvQuantSupport::Q8, KvQuantSupport::Q8 | KvQuantSupport::Q4Q8Mix) => true,
        (KvQuantSupport::Q4Q8Mix, KvQuantSupport::Q4Q8Mix) => true,
        _ => false,
    }
}
