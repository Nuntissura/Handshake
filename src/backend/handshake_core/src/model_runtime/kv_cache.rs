use std::fmt;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{KvQuantSupport, ModelId, ModelRuntimeError};

const KV_PREFIX_HASH_DOMAIN: &[u8] = b"handshake.kv_prefix.v1";

// MT-095: BLAKE3 keyed-hash derivation domain. Domain separation keeps
// the keyed hash collision-free against any other BLAKE3-derived ids
// in the same process (e.g., if we add a `derive_session_id` later).
const KV_PREFIX_DERIVE_DOMAIN: &[u8] = b"handshake.kv_prefix.derive.v1";

// Per-process keyed hash key. Generated lazily from OS entropy via two
// uuid::Uuid::new_v4() draws (16 bytes each → 32-byte key). The key is
// NEVER written to disk; a captured KvPrefixHandle cannot be replayed
// across process restarts because the next process derives a different
// key and validation `handle.derived_id == derive_id(...)` fails. This
// is the replay-resistance invariant for MT-095 + AC-INFER-LAB-8-TECHNIQUES.b.
static KV_PREFIX_KEY: OnceLock<[u8; 32]> = OnceLock::new();

fn kv_prefix_key() -> &'static [u8; 32] {
    KV_PREFIX_KEY.get_or_init(|| {
        let a = *Uuid::new_v4().as_bytes();
        let b = *Uuid::new_v4().as_bytes();
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(&a);
        key[16..].copy_from_slice(&b);
        key
    })
}

/// Test-only accessor for the per-process KV prefix key. `#[doc(hidden)]`
/// signals this is not part of the public API contract. Used by the
/// MT-095 security test sweep to assert the OnceLock initialized.
#[doc(hidden)]
pub fn __kv_prefix_key_for_tests() -> &'static [u8; 32] {
    kv_prefix_key()
}

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
    // MT-095 replay-resistance binding: 16-byte BLAKE3 keyed hash over
    // (model_id, content_hash, registered_at_utc). `None` for handles
    // created via the legacy `from_tokens` / `from_scoped_tokens` /
    // `from_parts` paths (which predate MT-095). The cache stack's
    // restore path is responsible for re-deriving and comparing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    derived_id: Option<[u8; 16]>,
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
            derived_id: None,
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
            derived_id: None,
        })
    }

    /// MT-095 replay-resistant constructor. Computes a BLAKE3 keyed hash
    /// over (model_id, content_hash, registered_at_utc_micros) using
    /// the per-process `KV_PREFIX_KEY`, truncates the digest to 16 bytes,
    /// and binds that to the returned handle. The cache stack records
    /// the same `derived_id` against the cached entry; restore validates
    /// the handle's `derived_id` matches the live re-derivation.
    pub fn from_derived(
        model_id: ModelId,
        content_hash: [u8; 32],
        registered_at_utc_micros: i64,
        token_count: u32,
    ) -> Self {
        let derived_id = Self::derive_id(model_id, &content_hash, registered_at_utc_micros);
        Self {
            // prefix_id stays UUID v7 for compatibility with v7-invariant
            // tests; the replay-resistance binding lives in `derived_id`.
            prefix_id: Uuid::now_v7(),
            content_hash,
            token_count,
            derived_id: Some(derived_id),
        }
    }

    /// Compute the BLAKE3 keyed-hash binding for replay resistance.
    /// Truncated to 16 bytes — collision-resistance reasoning: 2^64
    /// preimage / 2^128 second-preimage bounds well exceed any
    /// in-process key/cache lifetime, and the per-process key further
    /// prevents cross-process replay (red_team minimum control).
    pub fn derive_id(
        model_id: ModelId,
        content_hash: &[u8; 32],
        registered_at_utc_micros: i64,
    ) -> [u8; 16] {
        let key = kv_prefix_key();
        let mut hasher = blake3::Hasher::new_keyed(key);
        hasher.update(KV_PREFIX_DERIVE_DOMAIN);
        hasher.update(model_id.as_uuid().as_bytes());
        hasher.update(content_hash);
        hasher.update(&registered_at_utc_micros.to_le_bytes());
        let full = hasher.finalize();
        let mut truncated = [0u8; 16];
        truncated.copy_from_slice(&full.as_bytes()[..16]);
        truncated
    }

    pub fn derived_id(&self) -> Option<&[u8; 16]> {
        self.derived_id.as_ref()
    }

    /// MT-095 restore-time validation helper. Returns Ok when the
    /// handle's `derived_id` matches `derive_id(model_id, content_hash,
    /// registered_at_utc_micros)` re-computed under the live process
    /// key. Surfaces typed errors for every adversarial path: missing
    /// binding, mismatch (which covers cross-model, cross-process,
    /// tampered content_hash, and stale-snapshot once paired with TTL
    /// enforcement by the caller).
    pub fn verify_derived_against(
        &self,
        model_id: ModelId,
        content_hash: &[u8; 32],
        registered_at_utc_micros: i64,
    ) -> Result<(), ModelRuntimeError> {
        let Some(handle_derived) = self.derived_id else {
            return Err(ModelRuntimeError::KvCacheError(
                "KV prefix handle has no MT-095 derived_id binding".to_string(),
            ));
        };
        if &self.content_hash != content_hash {
            return Err(ModelRuntimeError::KvCacheError(
                "KV prefix handle content_hash mismatch (tampered)".to_string(),
            ));
        }
        let recomputed = Self::derive_id(model_id, content_hash, registered_at_utc_micros);
        if handle_derived != recomputed {
            return Err(ModelRuntimeError::KvCacheError(
                "KV prefix handle derived_id mismatch (cross-model, cross-process, or stale snapshot)".to_string(),
            ));
        }
        Ok(())
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
