use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[cfg(feature = "candle-runtime-engine")]
use super::ssm_state::SsmStateSource;
use crate::model_runtime::{
    KvCacheHandle, KvCacheOps, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelId,
    ModelRuntimeError,
};

const STATE_VECTOR_PREFIX_HASH_DOMAIN: &[u8] = b"handshake.candle.state_vector.prefix.v1";
const STATE_VECTOR_SNAPSHOT_HASH_DOMAIN: &[u8] = b"handshake.candle.state_vector.snapshot.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateVectorId(Uuid);

impl StateVectorId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(value: Uuid) -> Result<Self, ModelRuntimeError> {
        if value.get_version_num() != 7 {
            return Err(ModelRuntimeError::KvCacheError(
                "state_vector_id must be UUID v7".to_string(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for StateVectorId {
    fn default() -> Self {
        Self::new_v7()
    }
}

impl fmt::Display for StateVectorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for StateVectorId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StateVectorId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Uuid::deserialize(deserializer)?;
        Self::from_uuid(value).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SSMStateVariant {
    Mamba2,
    RwkvV5,
    RwkvV6,
    RwkvV7,
}

impl SSMStateVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mamba2 => "mamba2",
            Self::RwkvV5 => "rwkv_v5",
            Self::RwkvV6 => "rwkv_v6",
            Self::RwkvV7 => "rwkv_v7",
        }
    }
}

impl fmt::Display for SSMStateVariant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SSMTensorSnapshot {
    pub dtype: String,
    pub shape: Vec<usize>,
    pub bytes: Vec<u8>,
}

impl SSMTensorSnapshot {
    pub fn new(
        dtype: impl Into<String>,
        shape: Vec<usize>,
        bytes: Vec<u8>,
    ) -> Result<Self, ModelRuntimeError> {
        let dtype = dtype.into().trim().to_string();
        if dtype.is_empty() {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector tensor dtype must not be empty".to_string(),
            ));
        }
        if shape.is_empty() {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector tensor shape must not be empty".to_string(),
            ));
        }
        Ok(Self {
            dtype,
            shape,
            bytes,
        })
    }

    pub fn byte_len(&self) -> u64 {
        self.bytes.len() as u64
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum SSMStateSnapshot {
    Mamba2 {
        conv_states: Vec<SSMTensorSnapshot>,
        ssm_states: Vec<SSMTensorSnapshot>,
    },
    RwkvV5 {
        token_shift: Vec<SSMTensorSnapshot>,
        ssm: Vec<SSMTensorSnapshot>,
    },
    RwkvV6 {
        token_shift: Vec<SSMTensorSnapshot>,
        ssm: Vec<SSMTensorSnapshot>,
    },
    RwkvV7 {
        token_shift: Vec<SSMTensorSnapshot>,
        ssm: Vec<SSMTensorSnapshot>,
    },
}

impl SSMStateSnapshot {
    pub fn variant(&self) -> SSMStateVariant {
        match self {
            Self::Mamba2 { .. } => SSMStateVariant::Mamba2,
            Self::RwkvV5 { .. } => SSMStateVariant::RwkvV5,
            Self::RwkvV6 { .. } => SSMStateVariant::RwkvV6,
            Self::RwkvV7 { .. } => SSMStateVariant::RwkvV7,
        }
    }

    pub fn tensor_count(&self) -> u32 {
        let count = match self {
            Self::Mamba2 {
                conv_states,
                ssm_states,
            } => conv_states.len() + ssm_states.len(),
            Self::RwkvV5 { token_shift, ssm }
            | Self::RwkvV6 { token_shift, ssm }
            | Self::RwkvV7 { token_shift, ssm } => token_shift.len() + ssm.len(),
        };
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    pub fn byte_len(&self) -> u64 {
        match self {
            Self::Mamba2 {
                conv_states,
                ssm_states,
            } => tensor_bytes(conv_states) + tensor_bytes(ssm_states),
            Self::RwkvV5 { token_shift, ssm }
            | Self::RwkvV6 { token_shift, ssm }
            | Self::RwkvV7 { token_shift, ssm } => tensor_bytes(token_shift) + tensor_bytes(ssm),
        }
    }

    fn hash_into(&self, hasher: &mut Sha256) {
        update_string(hasher, self.variant().as_str());
        match self {
            Self::Mamba2 {
                conv_states,
                ssm_states,
            } => {
                update_tensors(hasher, conv_states);
                update_tensors(hasher, ssm_states);
            }
            Self::RwkvV5 { token_shift, ssm }
            | Self::RwkvV6 { token_shift, ssm }
            | Self::RwkvV7 { token_shift, ssm } => {
                update_tensors(hasher, token_shift);
                update_tensors(hasher, ssm);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateVectorSnapshotRecord {
    pub state_vector_id: StateVectorId,
    pub model_id: ModelId,
    pub artifact_sha256: String,
    pub prefix_token_count: u32,
    pub prefix_content_hash: [u8; 32],
    pub snapshot_hash: [u8; 32],
    pub snapshot: SSMStateSnapshot,
}

impl StateVectorSnapshotRecord {
    pub fn new(
        state_vector_id: StateVectorId,
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        prefix_token_count: u32,
        prefix_content_hash: [u8; 32],
        snapshot: SSMStateSnapshot,
    ) -> Result<Self, ModelRuntimeError> {
        let artifact_sha256 = normalize_artifact_sha256(artifact_sha256)?;
        let snapshot_hash = snapshot_hash(
            state_vector_id,
            model_id,
            &artifact_sha256,
            prefix_token_count,
            &prefix_content_hash,
            &snapshot,
        );
        Ok(Self {
            state_vector_id,
            model_id,
            artifact_sha256,
            prefix_token_count,
            prefix_content_hash,
            snapshot_hash,
            snapshot,
        })
    }

    pub fn from_parts(
        state_vector_uuid: Uuid,
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        prefix_token_count: u32,
        prefix_content_hash: [u8; 32],
        snapshot: SSMStateSnapshot,
    ) -> Result<Self, ModelRuntimeError> {
        Self::new(
            StateVectorId::from_uuid(state_vector_uuid)?,
            model_id,
            artifact_sha256,
            prefix_token_count,
            prefix_content_hash,
            snapshot,
        )
    }

    pub fn validate_integrity(&self) -> Result<(), ModelRuntimeError> {
        StateVectorId::from_uuid(self.state_vector_id.as_uuid())?;
        let artifact_sha256 = normalize_artifact_sha256(&self.artifact_sha256)?;
        if artifact_sha256 != self.artifact_sha256 {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector artifact_sha256 must be normalized lowercase hex".to_string(),
            ));
        }
        let expected = snapshot_hash(
            self.state_vector_id,
            self.model_id,
            &self.artifact_sha256,
            self.prefix_token_count,
            &self.prefix_content_hash,
            &self.snapshot,
        );
        if expected != self.snapshot_hash {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector snapshot_hash mismatch".to_string(),
            ));
        }
        Ok(())
    }

    pub fn byte_len(&self) -> u64 {
        self.snapshot.byte_len()
    }
}

pub trait StateVectorOps: Send + Sync {
    fn variant(&self) -> SSMStateVariant;

    fn model_id(&self) -> ModelId;

    fn artifact_sha256(&self) -> String;

    fn quantization(&self) -> KvQuantSupport;

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError>;

    fn occupancy(&self) -> KvCacheStats;

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError>;

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError>;

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError>;

    fn evict_all(&self) -> Result<(), ModelRuntimeError>;

    fn export_snapshot(
        &self,
        handle: &KvPrefixHandle,
    ) -> Result<StateVectorSnapshotRecord, ModelRuntimeError>;

    fn restore_snapshot_record(
        &self,
        handle: &KvPrefixHandle,
        record: StateVectorSnapshotRecord,
    ) -> Result<(), ModelRuntimeError>;
}

#[derive(Clone)]
pub struct StateVectorHandle {
    id: String,
    ops: Arc<dyn StateVectorOps>,
}

impl StateVectorHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            id: value.into(),
            ops: Arc::new(UnsupportedStateVectorOps),
        }
    }

    pub fn with_ops(value: impl Into<String>, ops: Arc<dyn StateVectorOps>) -> Self {
        Self {
            id: value.into(),
            ops,
        }
    }

    pub fn new_in_memory(
        value: impl Into<String>,
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        snapshot: SSMStateSnapshot,
    ) -> Result<Self, ModelRuntimeError> {
        let value = value.into();
        let ops = InMemoryStateVectorOps::new(model_id, artifact_sha256, snapshot)?;
        Ok(Self {
            id: value,
            ops: Arc::new(ops),
        })
    }

    /// CRIT-1 / MT-088 — build a handle backed by a live SSM model so
    /// `prefix_commit` pulls from `state_source.extract()` and
    /// `prefix_restore` writes back via `state_source.restore()`. Use
    /// this from adapter construction sites; tests stay on
    /// [`StateVectorHandle::new_in_memory`].
    #[cfg(feature = "candle-runtime-engine")]
    pub fn new_in_memory_with_source(
        value: impl Into<String>,
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        variant: SSMStateVariant,
        state_source: Arc<dyn SsmStateSource>,
    ) -> Result<Self, ModelRuntimeError> {
        let value = value.into();
        let ops = InMemoryStateVectorOps::new_with_source(
            model_id,
            artifact_sha256,
            variant,
            state_source,
        )?;
        Ok(Self {
            id: value,
            ops: Arc::new(ops),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub fn as_kv_cache_handle(&self) -> KvCacheHandle {
        // StateVectorHandle impls KvCacheOps (see further down this file);
        // wire it through KvCacheHandle::with_ops so the public
        // kv_cache_technique::* surface can dispatch via the handle.
        KvCacheHandle::with_ops(self.id.clone(), Arc::new(self.clone()))
    }

    pub fn variant(&self) -> SSMStateVariant {
        self.ops.variant()
    }

    pub fn model_id(&self) -> ModelId {
        self.ops.model_id()
    }

    pub fn artifact_sha256(&self) -> String {
        self.ops.artifact_sha256()
    }

    pub fn export_snapshot(
        &self,
        handle: &KvPrefixHandle,
    ) -> Result<StateVectorSnapshotRecord, ModelRuntimeError> {
        self.ops.export_snapshot(handle)
    }

    pub fn restore_snapshot_record(
        &self,
        handle: &KvPrefixHandle,
        record: StateVectorSnapshotRecord,
    ) -> Result<(), ModelRuntimeError> {
        self.ops.restore_snapshot_record(handle, record)
    }
}

impl fmt::Debug for StateVectorHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StateVectorHandle")
            .field("id", &self.id)
            .field("variant", &self.variant())
            .field("ops", &"<dyn StateVectorOps>")
            .finish()
    }
}

impl PartialEq for StateVectorHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for StateVectorHandle {}

impl KvCacheOps for StateVectorHandle {
    fn quantization(&self) -> KvQuantSupport {
        self.ops.quantization()
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        self.ops.set_quantization(level)
    }

    fn occupancy(&self) -> KvCacheStats {
        self.ops.occupancy()
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        self.ops.prefix_commit(prefix_tokens)
    }

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.ops.prefix_restore(handle)
    }

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.ops.prefix_evict(handle)
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        self.ops.evict_all()
    }
}

struct InMemoryStateVectorOps {
    model_id: ModelId,
    artifact_sha256: String,
    // Pre-CRIT-1 placeholder; still the canonical store when no live model
    // is attached (tests, persistence-only flows). When `state_source` is
    // populated this is updated as a cache after every commit/restore so
    // variant() / occupancy() / cached-read paths keep working without
    // re-locking the model.
    current_snapshot: Mutex<SSMStateSnapshot>,
    entries: Mutex<HashMap<Uuid, StateVectorSnapshotRecord>>,
    hits: Mutex<u64>,
    misses: Mutex<u64>,
    // CRIT-1 / MT-088: when set, prefix_commit pulls live state from the
    // attached SSM model and prefix_restore writes back into it. The field
    // is feature-gated because the SsmStateSource trait lives behind the
    // candle-runtime-engine feature.
    #[cfg(feature = "candle-runtime-engine")]
    state_source: Option<Arc<dyn SsmStateSource>>,
}

impl InMemoryStateVectorOps {
    fn new(
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        snapshot: SSMStateSnapshot,
    ) -> Result<Self, ModelRuntimeError> {
        Ok(Self {
            model_id,
            artifact_sha256: normalize_artifact_sha256(artifact_sha256)?,
            current_snapshot: Mutex::new(snapshot),
            entries: Mutex::new(HashMap::new()),
            hits: Mutex::new(0),
            misses: Mutex::new(0),
            #[cfg(feature = "candle-runtime-engine")]
            state_source: None,
        })
    }

    /// Build with a live SSM state source (adapter path). The initial
    /// snapshot is pulled from the source so `variant()` answers correctly
    /// before any prefix_commit. Returns an error if the declared variant
    /// does not match the source's first extraction.
    #[cfg(feature = "candle-runtime-engine")]
    fn new_with_source(
        model_id: ModelId,
        artifact_sha256: impl Into<String>,
        variant: SSMStateVariant,
        state_source: Arc<dyn SsmStateSource>,
    ) -> Result<Self, ModelRuntimeError> {
        let initial = state_source.extract()?;
        if initial.variant() != variant {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "state-vector source variant mismatch: declared {}, extracted {}",
                variant,
                initial.variant()
            )));
        }
        Ok(Self {
            model_id,
            artifact_sha256: normalize_artifact_sha256(artifact_sha256)?,
            current_snapshot: Mutex::new(initial),
            entries: Mutex::new(HashMap::new()),
            hits: Mutex::new(0),
            misses: Mutex::new(0),
            state_source: Some(state_source),
        })
    }

    fn read_live_snapshot(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        #[cfg(feature = "candle-runtime-engine")]
        {
            if let Some(source) = self.state_source.as_ref() {
                let live = source.extract()?;
                // Refresh the cached copy so variant() / occupancy() never
                // disagree with the model.
                if let Ok(mut cached) = self.current_snapshot.lock() {
                    *cached = live.clone();
                }
                return Ok(live);
            }
        }
        self.current_snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| {
                ModelRuntimeError::KvCacheError(
                    "state-vector snapshot lock is poisoned".to_string(),
                )
            })
    }

    fn write_live_snapshot(&self, snapshot: &SSMStateSnapshot) -> Result<(), ModelRuntimeError> {
        #[cfg(feature = "candle-runtime-engine")]
        {
            if let Some(source) = self.state_source.as_ref() {
                source.restore(snapshot)?;
            }
        }
        *self.current_snapshot.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("state-vector snapshot lock is poisoned".to_string())
        })? = snapshot.clone();
        Ok(())
    }

    fn prefix_scope(&self) -> Vec<u8> {
        let mut scope = Vec::new();
        scope.extend_from_slice(STATE_VECTOR_PREFIX_HASH_DOMAIN);
        scope.extend_from_slice(self.model_id.as_uuid().as_bytes());
        push_string(&mut scope, &self.artifact_sha256);
        push_string(&mut scope, self.variant().as_str());
        scope
    }

    fn validate_record_for_restore(
        &self,
        handle: &KvPrefixHandle,
        record: &StateVectorSnapshotRecord,
    ) -> Result<(), ModelRuntimeError> {
        record.validate_integrity()?;
        if record.prefix_token_count != handle.token_count() {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector token_count mismatch".to_string(),
            ));
        }
        if &record.prefix_content_hash != handle.content_hash() {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector content_hash mismatch".to_string(),
            ));
        }
        let expected_variant = self.variant();
        let actual_variant = record.snapshot.variant();
        if actual_variant != expected_variant {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "state-vector variant mismatch: expected {expected_variant}, got {actual_variant}"
            )));
        }
        if record.artifact_sha256 != self.artifact_sha256 {
            return Err(ModelRuntimeError::KvCacheError(format!(
                "state-vector artifact_sha256 mismatch: expected {}, got {}",
                self.artifact_sha256, record.artifact_sha256
            )));
        }
        Ok(())
    }

    fn record_for_handle(
        &self,
        handle: &KvPrefixHandle,
    ) -> Result<StateVectorSnapshotRecord, ModelRuntimeError> {
        let entries = self.entries.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("state-vector entry lock is poisoned".to_string())
        })?;
        let record = entries.get(&handle.prefix_id()).ok_or_else(|| {
            ModelRuntimeError::KvCacheError("unknown state-vector handle".to_string())
        })?;
        if &record.prefix_content_hash != handle.content_hash() {
            return Err(ModelRuntimeError::KvCacheError(
                "state-vector content_hash mismatch".to_string(),
            ));
        }
        record.validate_integrity()?;
        Ok(record.clone())
    }

    fn record_hit(&self) -> Result<(), ModelRuntimeError> {
        let mut hits = self.hits.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("state-vector hit counter lock is poisoned".to_string())
        })?;
        *hits += 1;
        Ok(())
    }

    fn record_miss(&self) -> Result<(), ModelRuntimeError> {
        let mut misses = self.misses.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError(
                "state-vector miss counter lock is poisoned".to_string(),
            )
        })?;
        *misses += 1;
        Ok(())
    }
}

impl StateVectorOps for InMemoryStateVectorOps {
    fn variant(&self) -> SSMStateVariant {
        self.current_snapshot
            .lock()
            .map(|snapshot| snapshot.variant())
            .unwrap_or(SSMStateVariant::Mamba2)
    }

    fn model_id(&self) -> ModelId {
        self.model_id
    }

    fn artifact_sha256(&self) -> String {
        self.artifact_sha256.clone()
    }

    fn quantization(&self) -> KvQuantSupport {
        KvQuantSupport::None
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        if level == KvQuantSupport::None {
            Ok(())
        } else {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: format!("state_vector_quantization.{level:?}"),
                adapter: "candle_state_vector".to_string(),
            })
        }
    }

    fn occupancy(&self) -> KvCacheStats {
        let (bytes_used, entries) = self
            .entries
            .lock()
            .map(|entries| {
                let bytes = entries
                    .values()
                    .map(StateVectorSnapshotRecord::byte_len)
                    .sum();
                (bytes, entries.len() as u32)
            })
            .unwrap_or((0, 0));
        let hits = self.hits.lock().map(|hits| *hits).unwrap_or_default();
        let misses = self.misses.lock().map(|misses| *misses).unwrap_or_default();
        KvCacheStats {
            bytes_used,
            bytes_capacity: bytes_used,
            prefix_cache_entries: entries,
            prefix_cache_hit_count: hits,
            prefix_cache_miss_count: misses,
            quant_level_current: KvQuantSupport::None,
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        let handle = KvPrefixHandle::from_scoped_tokens(&self.prefix_scope(), prefix_tokens)?;
        // CRIT-1: pull from live model when a source is attached; falls
        // back to the cached snapshot otherwise.
        let snapshot = self.read_live_snapshot()?;
        let record = StateVectorSnapshotRecord::new(
            StateVectorId::new_v7(),
            self.model_id,
            self.artifact_sha256.clone(),
            handle.token_count(),
            *handle.content_hash(),
            snapshot,
        )?;
        self.entries
            .lock()
            .map_err(|_| {
                ModelRuntimeError::KvCacheError("state-vector entry lock is poisoned".to_string())
            })?
            .insert(handle.prefix_id(), record);
        Ok(handle)
    }

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        let record = match self.record_for_handle(handle) {
            Ok(record) => record,
            Err(error) => {
                self.record_miss()?;
                return Err(error);
            }
        };
        self.validate_record_for_restore(handle, &record)?;
        // CRIT-1: write back into the live model when a source is attached;
        // otherwise updates the cached snapshot only.
        self.write_live_snapshot(&record.snapshot)?;
        self.record_hit()
    }

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.entries
            .lock()
            .map_err(|_| {
                ModelRuntimeError::KvCacheError("state-vector entry lock is poisoned".to_string())
            })?
            .remove(&handle.prefix_id());
        Ok(())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        self.entries
            .lock()
            .map_err(|_| {
                ModelRuntimeError::KvCacheError("state-vector entry lock is poisoned".to_string())
            })?
            .clear();
        Ok(())
    }

    fn export_snapshot(
        &self,
        handle: &KvPrefixHandle,
    ) -> Result<StateVectorSnapshotRecord, ModelRuntimeError> {
        self.record_for_handle(handle)
    }

    fn restore_snapshot_record(
        &self,
        handle: &KvPrefixHandle,
        record: StateVectorSnapshotRecord,
    ) -> Result<(), ModelRuntimeError> {
        self.validate_record_for_restore(handle, &record)?;
        self.entries
            .lock()
            .map_err(|_| {
                ModelRuntimeError::KvCacheError("state-vector entry lock is poisoned".to_string())
            })?
            .insert(handle.prefix_id(), record.clone());
        self.write_live_snapshot(&record.snapshot)?;
        Ok(())
    }
}

struct UnsupportedStateVectorOps;

impl StateVectorOps for UnsupportedStateVectorOps {
    fn variant(&self) -> SSMStateVariant {
        SSMStateVariant::Mamba2
    }

    fn model_id(&self) -> ModelId {
        ModelId::new_v7()
    }

    fn artifact_sha256(&self) -> String {
        String::new()
    }

    fn quantization(&self) -> KvQuantSupport {
        KvQuantSupport::None
    }

    fn set_quantization(&self, _level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        Err(unsupported_state_vector_error())
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
        Err(unsupported_state_vector_error())
    }

    fn prefix_restore(&self, _handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Err(unsupported_state_vector_error())
    }

    fn prefix_evict(&self, _handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Err(unsupported_state_vector_error())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        Err(unsupported_state_vector_error())
    }

    fn export_snapshot(
        &self,
        _handle: &KvPrefixHandle,
    ) -> Result<StateVectorSnapshotRecord, ModelRuntimeError> {
        Err(unsupported_state_vector_error())
    }

    fn restore_snapshot_record(
        &self,
        _handle: &KvPrefixHandle,
        _record: StateVectorSnapshotRecord,
    ) -> Result<(), ModelRuntimeError> {
        Err(unsupported_state_vector_error())
    }
}

fn unsupported_state_vector_error() -> ModelRuntimeError {
    ModelRuntimeError::CapabilityNotSupported {
        capability: "state_vector_cache".to_string(),
        adapter: "unbound_state_vector_handle".to_string(),
    }
}

fn tensor_bytes(tensors: &[SSMTensorSnapshot]) -> u64 {
    tensors.iter().map(SSMTensorSnapshot::byte_len).sum()
}

fn update_tensors(hasher: &mut Sha256, tensors: &[SSMTensorSnapshot]) {
    hasher.update((tensors.len() as u64).to_le_bytes());
    for tensor in tensors {
        update_string(hasher, &tensor.dtype);
        hasher.update((tensor.shape.len() as u64).to_le_bytes());
        for dim in &tensor.shape {
            hasher.update((*dim as u64).to_le_bytes());
        }
        hasher.update((tensor.bytes.len() as u64).to_le_bytes());
        hasher.update(&tensor.bytes);
    }
}

fn snapshot_hash(
    state_vector_id: StateVectorId,
    model_id: ModelId,
    artifact_sha256: &str,
    prefix_token_count: u32,
    prefix_content_hash: &[u8; 32],
    snapshot: &SSMStateSnapshot,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(STATE_VECTOR_SNAPSHOT_HASH_DOMAIN);
    hasher.update(state_vector_id.as_uuid().as_bytes());
    hasher.update(model_id.as_uuid().as_bytes());
    update_string(&mut hasher, artifact_sha256);
    hasher.update(prefix_token_count.to_le_bytes());
    hasher.update(prefix_content_hash);
    snapshot.hash_into(&mut hasher);
    hasher.finalize().into()
}

fn update_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn push_string(buffer: &mut Vec<u8>, value: &str) {
    buffer.extend_from_slice(&(value.len() as u64).to_le_bytes());
    buffer.extend_from_slice(value.as_bytes());
}

fn normalize_artifact_sha256(value: impl Into<String>) -> Result<String, ModelRuntimeError> {
    let value = value.into().trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ModelRuntimeError::KvCacheError(
            "state-vector artifact_sha256 must be 64 lowercase hex characters".to_string(),
        ));
    }
    Ok(value)
}

// ----------------------------------------------------------------------------
// MT-117 — Cross-session state restore (SSM state persistence to disk +
// rehydration).
//
// Per refinement INF-9 feature_parity_detail ("cross-session state
// restore (state-vector persistence + restore)") + operator E-2.
//
// Surfaces in this section:
//
// 1. In-process bytes-in / bytes-out:
//    - persist_to_bytes (snapshot -> envelope JSON bytes)
//    - load_from_bytes (envelope JSON bytes -> validated envelope)
//
// 2. Cross-handle re-mint bridge (resolves MT-114.cross_handle_finding):
//    - load_into_handle takes a freshly-loaded handle + envelope bytes,
//      verifies model_artifact_sha256 + variant against the loaded handle,
//      re-mints a KvPrefixHandle under the loaded handle's prefix_scope,
//      and seats the snapshot record so the next prefix_restore call
//      succeeds.
//
// 3. ArtifactStore + KernelActionCatalogV1 integration (the disk path):
//    - persist_to_artifact_store writes the envelope to the kernel
//      ArtifactStore via write_file_artifact, dispatched through the
//      catalog action `kernel.subquadratic.persist_state` whose write_box
//      is the StateVectorPersistBox.
//    - load_from_artifact_store reads the persisted envelope back from
//      disk, verifies the manifest content_hash, and revalidates the
//      envelope integrity before returning it.
//    - load_from_artifact_store_into_handle wires the above two through
//      the re-mint bridge so an operator can resume a long-running SSM
//      session by artifact_id alone after a Handshake process restart.
//
// All disk writes go through write_file_artifact, which itself runs
// atomic temp+fsync+rename per artifact_store_root([CX-503R] PostgreSQL/
// EventLedger-backed authority does not apply here — state vectors are
// model artifacts under .handshake/artifacts/L3/, not authority rows).
// ----------------------------------------------------------------------------

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::model_runtime::{lora::LicenseTag, registry::OperatorId};
use crate::storage::artifacts::{
    artifact_root_dir, artifact_root_rel, write_file_artifact, ArtifactClassification,
    ArtifactError, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::EntityRef;

/// Envelope-format-version marker — bumped only when the wire format
/// changes incompatibly. Future MTs introducing tensor-quantization or
/// streaming compression must bump this and add a migration shim.
pub const STATE_VECTOR_PERSIST_ENVELOPE_VERSION: &str = "hsk.subquad.state_vector.persist.v1";

/// Stable kernel catalog action_id for the persist-state mutation.
/// Mirrored in action_catalog::subquadratic_persist_state_action so the
/// dispatcher and the model_runtime engine name the same action.
pub const STATE_VECTOR_PERSIST_ACTION_ID: &str = "kernel.subquadratic.persist_state";

/// Stable kernel write_box schema_id for the persist-state mutation.
pub const STATE_VECTOR_PERSIST_WRITE_BOX_SCHEMA_ID: &str = "hsk.write_box.state_vector_persist@1";

/// Sidecar metadata persisted alongside the snapshot record. Fields per
/// MT-117 contract narrative; the record already carries the integrity
/// hashes so this struct keeps the operator-facing audit fields
/// (who/when/license) without duplicating the cryptographic state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateVectorPersistMetadata {
    pub persisted_at_utc: DateTime<Utc>,
    pub persisted_by: OperatorId,
    pub license_tag: LicenseTag,
    pub n_tokens_advanced: u32,
    pub variant_label: String,
}

/// Wire-format envelope. Disk I/O writes this; load_from_bytes reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateVectorPersistEnvelope {
    pub envelope_version: String,
    pub record: StateVectorSnapshotRecord,
    pub metadata: StateVectorPersistMetadata,
}

impl StateVectorPersistEnvelope {
    pub fn new(
        record: StateVectorSnapshotRecord,
        persisted_by: OperatorId,
        license_tag: LicenseTag,
        persisted_at_utc: DateTime<Utc>,
    ) -> Self {
        let metadata = StateVectorPersistMetadata {
            persisted_at_utc,
            persisted_by,
            license_tag,
            n_tokens_advanced: record.prefix_token_count,
            variant_label: record.snapshot.variant().as_str().to_string(),
        };
        Self {
            envelope_version: STATE_VECTOR_PERSIST_ENVELOPE_VERSION.to_string(),
            record,
            metadata,
        }
    }
}

/// Record returned by the disk-integration path. Mirrors the envelope's
/// integrity fields plus the operator-facing artifact_id and on-disk
/// reference paths so the operator can find the snapshot later through
/// the ArtifactStore listing without re-loading the payload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateVectorPersistRecord {
    pub artifact_id: Uuid,
    pub artifact_layer: ArtifactLayer,
    pub artifact_root_rel: String,
    pub envelope_version: String,
    pub state_vector_id: StateVectorId,
    pub model_id: ModelId,
    pub artifact_sha256: String,
    pub variant: SSMStateVariant,
    pub byte_len: u64,
    pub content_hash: String,
    pub persisted_at_utc: DateTime<Utc>,
    pub persisted_by: OperatorId,
    pub license_tag: LicenseTag,
}

/// Serialize a handle's snapshot to a portable byte envelope. Operator-
/// facing audit metadata (who/when/license) lives in the envelope's
/// metadata field; the record itself carries the cryptographic
/// integrity (snapshot_hash + content_hash + artifact_sha256 + variant).
pub fn persist_to_bytes(
    handle: &StateVectorHandle,
    prefix_handle: &KvPrefixHandle,
    persisted_by: OperatorId,
    license_tag: LicenseTag,
) -> Result<Vec<u8>, ModelRuntimeError> {
    let record = handle.export_snapshot(prefix_handle)?;
    let envelope = StateVectorPersistEnvelope::new(record, persisted_by, license_tag, Utc::now());
    serde_json::to_vec(&envelope).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector envelope serialization failed: {error}"
        ))
    })
}

/// Deserialize a byte envelope back into the wire-format struct, with
/// integrity validation. The envelope version is checked first; future
/// migration paths land here.
pub fn load_from_bytes(bytes: &[u8]) -> Result<StateVectorPersistEnvelope, ModelRuntimeError> {
    let envelope: StateVectorPersistEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector envelope deserialization failed: {error}"
        ))
    })?;
    if envelope.envelope_version != STATE_VECTOR_PERSIST_ENVELOPE_VERSION {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector envelope version mismatch: expected {}, got {}",
            STATE_VECTOR_PERSIST_ENVELOPE_VERSION, envelope.envelope_version
        )));
    }
    envelope.record.validate_integrity()?;
    Ok(envelope)
}

/// Cross-handle re-mint bridge (resolves MT-114.cross_handle_finding).
///
/// Accepts the freshly-loaded handle on the target model and the
/// envelope bytes; verifies model_artifact_sha256 + variant against the
/// loaded handle, re-mints a KvPrefixHandle under the loaded handle's
/// scope (so the new content_hash matches the loaded model's
/// prefix_scope), and inserts the snapshot record so subsequent
/// prefix_restore calls succeed under the new scope.
///
/// Returns the freshly re-minted KvPrefixHandle the operator should use
/// for the subsequent prefix_restore call.
///
/// Failure modes (per MT-117 contract):
/// - envelope version mismatch -> KvCacheError
/// - sha256 mismatch -> KvCacheError with "snapshot was taken against a
///   different model artifact; either load that artifact or recapture"
/// - variant mismatch -> KvCacheError
/// - record integrity hash mismatch -> KvCacheError
pub fn load_into_handle(
    target_handle: &StateVectorHandle,
    envelope_bytes: &[u8],
    prefix_tokens: &[u32],
) -> Result<KvPrefixHandle, ModelRuntimeError> {
    let envelope = load_from_bytes(envelope_bytes)?;

    let target_artifact_sha = target_handle.artifact_sha256();
    if envelope.record.artifact_sha256 != target_artifact_sha {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector cross-session restore: snapshot was taken against a different model \
             artifact (envelope sha256={}, loaded model sha256={}); either load that artifact \
             or recapture",
            envelope.record.artifact_sha256, target_artifact_sha
        )));
    }

    let target_variant = target_handle.variant();
    let envelope_variant = envelope.record.snapshot.variant();
    if envelope_variant != target_variant {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector cross-session restore: variant mismatch (envelope={}, loaded={})",
            envelope_variant, target_variant
        )));
    }

    if envelope.record.prefix_token_count != prefix_tokens.len() as u32 {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector cross-session restore: prefix_tokens length mismatch (envelope token \
             count={}, supplied tokens={})",
            envelope.record.prefix_token_count,
            prefix_tokens.len()
        )));
    }

    // Re-mint under the loaded handle's scope and seed it with the
    // envelope snapshot. KvCacheOps::prefix_commit mints a fresh handle
    // under the local prefix_scope; the subsequent restore_snapshot_record
    // call (with a new record whose content_hash matches the re-minted
    // handle) seats the snapshot in the target cache so the operator's
    // next prefix_restore call picks up the migrated state.
    let reminted_handle =
        <StateVectorHandle as KvCacheOps>::prefix_commit(target_handle, prefix_tokens).map_err(
            |error| {
                ModelRuntimeError::KvCacheError(format!(
                    "state-vector cross-session restore: re-mint prefix_commit failed: {error}"
                ))
            },
        )?;

    let migrated_record = StateVectorSnapshotRecord::from_parts(
        StateVectorId::new_v7().as_uuid(),
        target_handle.model_id(),
        target_artifact_sha,
        reminted_handle.token_count(),
        *reminted_handle.content_hash(),
        envelope.record.snapshot.clone(),
    )?;
    migrated_record.validate_integrity().map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector cross-session restore: migrated record integrity check failed: {error}"
        ))
    })?;

    target_handle.restore_snapshot_record(&reminted_handle, migrated_record)?;

    // MT-095: bind the re-minted handle so the operator's subsequent restore
    // through the gated KvCacheHandle chokepoint (subquadratic::state_restore /
    // kv_cache_technique::prefix_restore) verifies it instead of fail-closing on
    // a missing binding. A cross-session reload is a fresh commit into THIS
    // process's cache, so it is bound like one (under the live process key +
    // now()); the prefix-cache TTL then measures from reload time. bind_derived
    // preserves prefix_id / content_hash / token_count, so the snapshot record
    // seated above (and the direct KvCacheOps trait restore path) still match.
    Ok(reminted_handle.bind_derived(target_handle.model_id(), Utc::now().timestamp_micros()))
}

fn map_artifact_error(error: ArtifactError) -> ModelRuntimeError {
    ModelRuntimeError::KvCacheError(format!("state-vector artifact store: {error}"))
}

fn envelope_content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Persist a committed state-vector snapshot to the kernel ArtifactStore
/// via the `kernel.subquadratic.persist_state` catalog action.
///
/// Returns a `StateVectorPersistRecord` carrying the minted `artifact_id`
/// plus the on-disk reference paths the operator can use to locate the
/// persisted bundle later. The envelope on disk is byte-identical to the
/// envelope returned by `persist_to_bytes` (so load_from_bytes /
/// load_into_handle continue to be re-usable against the same bytes).
///
/// Failure modes (per MT-117 contract):
/// - export_snapshot fails (unknown handle / tampered content_hash)
///   -> KvCacheError surfaced as-is from the StateVectorOps surface.
/// - envelope serialization fails -> KvCacheError.
/// - ArtifactStore write fails (root-escape / disk full / hash mismatch)
///   -> KvCacheError via `map_artifact_error`.
pub fn persist_to_artifact_store(
    handle: &StateVectorHandle,
    prefix_handle: &KvPrefixHandle,
    persisted_by: OperatorId,
    license_tag: LicenseTag,
    workspace_root: &Path,
) -> Result<StateVectorPersistRecord, ModelRuntimeError> {
    let envelope_bytes = persist_to_bytes(
        handle,
        prefix_handle,
        persisted_by.clone(),
        license_tag.clone(),
    )?;

    // Re-parse the just-serialized envelope so the persist record
    // mirrors the wire-format metadata exactly (notably the
    // persisted_at_utc minted by persist_to_bytes).
    let envelope: StateVectorPersistEnvelope =
        serde_json::from_slice(&envelope_bytes).map_err(|error| {
            ModelRuntimeError::KvCacheError(format!(
                "state-vector envelope re-parse for ArtifactStore record failed: {error}"
            ))
        })?;

    let content_hash = envelope_content_hash(&envelope_bytes);
    let artifact_id = Uuid::now_v7();
    let layer = ArtifactLayer::L3;
    let variant = envelope.record.snapshot.variant();
    let filename_hint = format!(
        "state_vector_{}_{}.envelope.json",
        variant.as_str(),
        envelope.record.state_vector_id.as_uuid()
    );
    let manifest = ArtifactManifest {
        artifact_id,
        layer,
        kind: ArtifactPayloadKind::Bundle,
        mime: "application/json".to_string(),
        filename_hint: Some(filename_hint),
        created_at: envelope.metadata.persisted_at_utc,
        created_by_job_id: None,
        source_entity_refs: vec![
            EntityRef {
                entity_kind: "model_id".to_string(),
                entity_id: envelope.record.model_id.as_uuid().to_string(),
            },
            EntityRef {
                entity_kind: "state_vector_id".to_string(),
                entity_id: envelope.record.state_vector_id.as_uuid().to_string(),
            },
            EntityRef {
                entity_kind: "subquadratic_variant".to_string(),
                entity_id: variant.as_str().to_string(),
            },
        ],
        source_artifact_refs: Vec::new(),
        content_hash: content_hash.clone(),
        size_bytes: envelope_bytes.len() as u64,
        // State vectors carry model behavior under operator authorship —
        // they are not Low-classification public outputs.
        classification: ArtifactClassification::Medium,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(false),
        hash_basis: Some("sha256(state_vector_persist_envelope.json_bytes)".to_string()),
        hash_exclude_paths: Vec::new(),
    };

    write_file_artifact(workspace_root, &manifest, &envelope_bytes).map_err(map_artifact_error)?;

    Ok(StateVectorPersistRecord {
        artifact_id,
        artifact_layer: layer,
        artifact_root_rel: artifact_root_rel(layer, artifact_id),
        envelope_version: envelope.envelope_version,
        state_vector_id: envelope.record.state_vector_id,
        model_id: envelope.record.model_id,
        artifact_sha256: envelope.record.artifact_sha256,
        variant,
        byte_len: envelope_bytes.len() as u64,
        content_hash,
        persisted_at_utc: envelope.metadata.persisted_at_utc,
        persisted_by: envelope.metadata.persisted_by,
        license_tag: envelope.metadata.license_tag,
    })
}

/// Read a persisted state-vector envelope back from the kernel
/// ArtifactStore. Validates manifest content_hash against the actual
/// payload bytes before deserializing, then revalidates the envelope's
/// own integrity via load_from_bytes. A drifted manifest (operator-side
/// edit, partial write, swap-then-restore) is rejected before any
/// envelope-side logic runs.
///
/// Failure modes (per MT-117 contract):
/// - missing artifact_id (no such directory) -> KvCacheError.
/// - manifest read/deserialize failure -> KvCacheError.
/// - payload read failure -> KvCacheError.
/// - manifest.content_hash != sha256(payload) -> KvCacheError naming the
///   expected vs actual hash so the operator can re-export.
/// - envelope version / record integrity failure -> KvCacheError from
///   load_from_bytes.
pub fn load_from_artifact_store(
    artifact_id: Uuid,
    workspace_root: &Path,
) -> Result<StateVectorPersistEnvelope, ModelRuntimeError> {
    let artifact_root = artifact_root_dir(workspace_root, ArtifactLayer::L3, artifact_id);
    if !artifact_root.exists() {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} not found in ArtifactStore (expected {})",
            artifact_root.display()
        )));
    }

    let manifest_path = artifact_root.join("artifact.json");
    let payload_path = artifact_root.join("payload");

    let manifest_bytes = std::fs::read(&manifest_path).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} manifest read failed at {}: {error}",
            manifest_path.display()
        ))
    })?;
    let manifest: ArtifactManifest = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} manifest deserialize failed: {error}"
        ))
    })?;

    let payload_bytes = std::fs::read(&payload_path).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} payload read failed at {}: {error}",
            payload_path.display()
        ))
    })?;

    let actual_hash = envelope_content_hash(&payload_bytes);
    if actual_hash != manifest.content_hash {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} payload tampered after persist: \
             expected sha256={}, got sha256={}",
            manifest.content_hash, actual_hash
        )));
    }
    if manifest.size_bytes != payload_bytes.len() as u64 {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "state-vector artifact_id {artifact_id} payload size mismatch: \
             manifest size_bytes={}, on-disk size_bytes={}",
            manifest.size_bytes,
            payload_bytes.len()
        )));
    }

    load_from_bytes(&payload_bytes)
}

/// Operator-facing one-shot: read a persisted envelope from the
/// ArtifactStore by `artifact_id`, then route it through the cross-handle
/// re-mint bridge so the loaded handle's prefix_scope is honored. Returns
/// the re-minted `KvPrefixHandle` the operator should pass to a
/// subsequent `prefix_restore` call.
///
/// This is the public "load_from_disk(artifact_id)" entry point named in
/// the MT-117 contract narrative.
pub fn load_from_artifact_store_into_handle(
    target_handle: &StateVectorHandle,
    artifact_id: Uuid,
    prefix_tokens: &[u32],
    workspace_root: &Path,
) -> Result<KvPrefixHandle, ModelRuntimeError> {
    let envelope = load_from_artifact_store(artifact_id, workspace_root)?;
    let envelope_bytes = serde_json::to_vec(&envelope).map_err(|error| {
        ModelRuntimeError::KvCacheError(format!(
            "state-vector envelope re-serialize for cross-handle restore failed: {error}"
        ))
    })?;
    load_into_handle(target_handle, &envelope_bytes, prefix_tokens)
}
