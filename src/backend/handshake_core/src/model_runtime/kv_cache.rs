use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{KvQuantSupport, ModelRuntimeError};

const KV_PREFIX_HASH_DOMAIN: &[u8] = b"handshake.kv_prefix.v1";

/// `KvCacheHandle` is the public per-model KV cache handle returned by
/// `ModelRuntime::kv_cache(model_id)`. It carries an opaque `id` (for
/// debug/audit logs) plus an `Arc<dyn KvCacheOps>` so the public
/// `kv_cache_technique::*` surface can dispatch through the trait
/// without each adapter needing a downcast. Mirrors `LoraStackHandle`.
///
/// Cloud/external adapters that do not own a real KV cache return
/// `KvCacheHandle::new(...)`, which installs an `UnsupportedKvCacheOps`
/// shim that fails closed on every mutation while still letting
/// `as_str()` work for logging.
#[derive(Clone)]
pub struct KvCacheHandle {
    id: String,
    ops: Arc<dyn KvCacheOps>,
}

impl KvCacheHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            id: value.into(),
            ops: Arc::new(UnsupportedKvCacheOps),
        }
    }

    pub fn with_ops(value: impl Into<String>, ops: Arc<dyn KvCacheOps>) -> Self {
        Self {
            id: value.into(),
            ops,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub fn quantization(&self) -> KvQuantSupport {
        self.ops.quantization()
    }

    pub fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        self.ops.set_quantization(level)
    }

    pub fn occupancy(&self) -> KvCacheStats {
        self.ops.occupancy()
    }

    pub fn prefix_commit(
        &self,
        prefix_tokens: &[u32],
    ) -> Result<KvPrefixHandle, ModelRuntimeError> {
        self.ops.prefix_commit(prefix_tokens)
    }

    pub fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.ops.prefix_restore(handle)
    }

    pub fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.ops.prefix_evict(handle)
    }

    pub fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        self.ops.evict_all()
    }
}

impl fmt::Debug for KvCacheHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KvCacheHandle")
            .field("id", &self.id)
            .field("ops", &"<dyn KvCacheOps>")
            .finish()
    }
}

impl PartialEq for KvCacheHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for KvCacheHandle {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KvPrefixHandle {
    prefix_id: Uuid,
    content_hash: [u8; 32],
    token_count: u32,
}

impl KvPrefixHandle {
    pub fn from_tokens(prefix_tokens: &[u32]) -> Result<Self, ModelRuntimeError> {
        Self::from_scoped_tokens(&[], prefix_tokens)
    }

    pub fn from_scoped_tokens(
        cache_scope: &[u8],
        prefix_tokens: &[u32],
    ) -> Result<Self, ModelRuntimeError> {
        let token_count = u32::try_from(prefix_tokens.len()).map_err(|_| {
            ModelRuntimeError::KvCacheError("prefix token count exceeds u32".to_string())
        })?;
        Ok(Self {
            prefix_id: Uuid::now_v7(),
            content_hash: Self::content_hash_for_scope_and_tokens(cache_scope, prefix_tokens),
            token_count,
        })
    }

    pub fn from_parts(
        prefix_id: Uuid,
        content_hash: [u8; 32],
        token_count: u32,
    ) -> Result<Self, ModelRuntimeError> {
        if prefix_id.get_version_num() != 7 {
            return Err(ModelRuntimeError::KvCacheError(
                "KV prefix handle prefix_id must be UUID v7".to_string(),
            ));
        }
        Ok(Self {
            prefix_id,
            content_hash,
            token_count,
        })
    }

    pub fn content_hash_for_tokens(prefix_tokens: &[u32]) -> [u8; 32] {
        Self::content_hash_for_scope_and_tokens(&[], prefix_tokens)
    }

    pub fn content_hash_for_scope_and_tokens(
        cache_scope: &[u8],
        prefix_tokens: &[u32],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(KV_PREFIX_HASH_DOMAIN);
        hasher.update((cache_scope.len() as u64).to_le_bytes());
        hasher.update(cache_scope);
        hasher.update((prefix_tokens.len() as u64).to_le_bytes());
        for token in prefix_tokens {
            hasher.update(token.to_le_bytes());
        }
        hasher.finalize().into()
    }

    pub fn prefix_id(&self) -> Uuid {
        self.prefix_id
    }

    pub fn content_hash(&self) -> &[u8; 32] {
        &self.content_hash
    }

    pub fn token_count(&self) -> u32 {
        self.token_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvCacheStats {
    pub bytes_used: u64,
    pub bytes_capacity: u64,
    pub prefix_cache_entries: u32,
    pub prefix_cache_hit_count: u64,
    pub prefix_cache_miss_count: u64,
    pub quant_level_current: KvQuantSupport,
}

pub enum KvCachePolicy {
    Default {
        quant: KvQuantSupport,
        prefix_cache_ttl_seconds: u64,
        max_bytes: Option<u64>,
    },
    Disabled,
    Custom(Box<dyn KvCacheOps + Send + Sync>),
}

impl Default for KvCachePolicy {
    fn default() -> Self {
        Self::Default {
            quant: KvQuantSupport::None,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        }
    }
}

impl fmt::Debug for KvCachePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default {
                quant,
                prefix_cache_ttl_seconds,
                max_bytes,
            } => formatter
                .debug_struct("Default")
                .field("quant", quant)
                .field("prefix_cache_ttl_seconds", prefix_cache_ttl_seconds)
                .field("max_bytes", max_bytes)
                .finish(),
            Self::Disabled => formatter.write_str("Disabled"),
            Self::Custom(_) => formatter.write_str("Custom(<dyn KvCacheOps>)"),
        }
    }
}

pub trait KvCacheOps: Send + Sync {
    fn quantization(&self) -> KvQuantSupport;

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError>;

    fn occupancy(&self) -> KvCacheStats;

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError>;

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError>;

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError>;

    fn evict_all(&self) -> Result<(), ModelRuntimeError>;
}

/// Default KV cache ops for adapters that don't own a real KV cache
/// surface (cloud BYOK adapters in particular). Returns the equivalent
/// of `KvCacheError("kv_cache_unsupported")` on every mutation and
/// reports an empty occupancy snapshot.
struct UnsupportedKvCacheOps;

impl KvCacheOps for UnsupportedKvCacheOps {
    fn quantization(&self) -> KvQuantSupport {
        KvQuantSupport::None
    }

    fn set_quantization(&self, _level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        Err(unsupported_kv_cache_error())
    }

    fn occupancy(&self) -> KvCacheStats {
        KvCacheStats {
            bytes_used: 0,
            bytes_capacity: 0,
            prefix_cache_entries: 0,
            prefix_cache_hit_count: 0,
            prefix_cache_miss_count: 0,
            quant_level_current: KvQuantSupport::None,
        }
    }

    fn prefix_commit(&self, _prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        Err(unsupported_kv_cache_error())
    }

    fn prefix_restore(&self, _handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Err(unsupported_kv_cache_error())
    }

    fn prefix_evict(&self, _handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Err(unsupported_kv_cache_error())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        Err(unsupported_kv_cache_error())
    }
}

fn unsupported_kv_cache_error() -> ModelRuntimeError {
    ModelRuntimeError::KvCacheError(
        "kv_cache_unsupported: this adapter does not expose a KV cache surface".to_string(),
    )
}
