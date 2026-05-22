use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use uuid::Uuid;

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
    current_snapshot: Mutex<SSMStateSnapshot>,
    entries: Mutex<HashMap<Uuid, StateVectorSnapshotRecord>>,
    hits: Mutex<u64>,
    misses: Mutex<u64>,
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
        })
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
        let snapshot = self
            .current_snapshot
            .lock()
            .map_err(|_| {
                ModelRuntimeError::KvCacheError(
                    "state-vector snapshot lock is poisoned".to_string(),
                )
            })?
            .clone();
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
        *self.current_snapshot.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("state-vector snapshot lock is poisoned".to_string())
        })? = record.snapshot;
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
        *self.current_snapshot.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError("state-vector snapshot lock is poisoned".to_string())
        })? = record.snapshot;
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
