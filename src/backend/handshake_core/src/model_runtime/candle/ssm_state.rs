#![cfg(feature = "candle-runtime-engine")]

//! CRIT-1 / MT-088 remediation surface.
//!
//! Provides the bridge between a live Candle SSM model's mutable state and
//! the typed [`SSMStateSnapshot`] used by [`StateVectorHandle`]. The
//! pre-remediation path stored an empty placeholder snapshot inside
//! `InMemoryStateVectorOps` that was never updated from the live model, so
//! `prefix_commit` captured nothing and `prefix_restore` restored nothing.
//! After this change:
//!
//! * Each SSM model (`CandleMamba2Model`, `CandleRwkvV5Model`,
//!   `CandleRwkvV6Model`, `CandleRwkvV7Model`) implements [`SsmModel`] —
//!   extracting / restoring its mutable `state` field as a typed
//!   [`SSMStateSnapshot`].
//! * The adapter wires an [`Arc<dyn SsmStateSource>`] into the state-vector
//!   handle. Internally the source holds a clone of the model's
//!   `Arc<Mutex<Box<…>>>`, so the handle can pull live state on commit and
//!   write live state on restore without copying the model.
//!
//! Limitations:
//!
//! * RWKV v7 DEA (delta-attention) state cannot be round-tripped today —
//!   it carries non-tensor fields (`Vec<u32>` token ids) that the existing
//!   [`SSMStateSnapshot`] enum does not model. `extract_snapshot` returns
//!   `CapabilityNotSupported` when DEA is present; the per-layer attention
//!   path is unaffected.
//! * Only F32, F16, and BF16 tensor dtypes are supported (the dtypes the
//!   in-tree SSM constructors actually produce). Other dtypes return
//!   `CapabilityNotSupported`; add a branch here if a new SSM dtype lands.

use std::sync::{Arc, Mutex};

use candle_core::{DType, Device, Tensor};

use super::state_vector::{SSMStateSnapshot, SSMTensorSnapshot};
use super::transformer::TransformerModel;
use crate::model_runtime::ModelRuntimeError;

/// Type-erased state source that [`super::state_vector::InMemoryStateVectorOps`]
/// holds in place of the prior placeholder snapshot. Concrete
/// implementations delegate to a live model's
/// [`TransformerModel::extract_ssm_snapshot`] /
/// [`TransformerModel::restore_ssm_snapshot`].
pub trait SsmStateSource: Send + Sync {
    fn extract(&self) -> Result<SSMStateSnapshot, ModelRuntimeError>;
    fn restore(&self, snapshot: &SSMStateSnapshot) -> Result<(), ModelRuntimeError>;
}

/// Mutex-guarded wrapper around the adapter's
/// `Arc<Mutex<Box<dyn TransformerModel>>>`. The state-vector handle clones
/// this Arc so it shares the same model instance the `forward()` path
/// mutates — extract pulls from live state, restore writes live state.
pub struct LockedSsmStateSource {
    inner: Arc<Mutex<Box<dyn TransformerModel>>>,
}

impl LockedSsmStateSource {
    pub fn new(inner: Arc<Mutex<Box<dyn TransformerModel>>>) -> Self {
        Self { inner }
    }
}

impl SsmStateSource for LockedSsmStateSource {
    fn extract(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
        let guard = self.inner.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError(
                "ssm model lock poisoned during state_vector extract".to_string(),
            )
        })?;
        guard.extract_ssm_snapshot()
    }

    fn restore(&self, snapshot: &SSMStateSnapshot) -> Result<(), ModelRuntimeError> {
        let mut guard = self.inner.lock().map_err(|_| {
            ModelRuntimeError::KvCacheError(
                "ssm model lock poisoned during state_vector restore".to_string(),
            )
        })?;
        guard.restore_ssm_snapshot(snapshot)
    }
}

// ===== Tensor <-> SSMTensorSnapshot helpers =====

/// Serialize a Candle [`Tensor`] to the wire format the state-vector
/// surface persists (dtype label + shape + LE-packed bytes).
pub fn tensor_to_snapshot(tensor: &Tensor) -> Result<SSMTensorSnapshot, ModelRuntimeError> {
    let dtype = tensor.dtype();
    let shape = tensor.shape().dims().to_vec();
    let contiguous = tensor.contiguous().map_err(|error| {
        ModelRuntimeError::KvCacheError(format!("ssm tensor contiguous failed: {error}"))
    })?;
    let flat = contiguous.flatten_all().map_err(|error| {
        ModelRuntimeError::KvCacheError(format!("ssm tensor flatten_all failed: {error}"))
    })?;
    let bytes = match dtype {
        DType::F32 => {
            let vec = flat.to_vec1::<f32>().map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm tensor to_vec1::<f32> failed: {error}"
                ))
            })?;
            let mut out = Vec::with_capacity(vec.len() * 4);
            for value in vec {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        }
        DType::F16 => {
            let vec = flat.to_vec1::<half::f16>().map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm tensor to_vec1::<f16> failed: {error}"
                ))
            })?;
            let mut out = Vec::with_capacity(vec.len() * 2);
            for value in vec {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        }
        DType::BF16 => {
            let vec = flat.to_vec1::<half::bf16>().map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm tensor to_vec1::<bf16> failed: {error}"
                ))
            })?;
            let mut out = Vec::with_capacity(vec.len() * 2);
            for value in vec {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        }
        other => {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: format!("ssm_tensor_dtype.{other:?}"),
                adapter: "candle_ssm_state".to_string(),
            });
        }
    };
    SSMTensorSnapshot::new(dtype_label(dtype), shape, bytes)
}

/// Inverse of [`tensor_to_snapshot`] — reconstitutes a Candle tensor on
/// `device` from a typed snapshot.
pub fn snapshot_to_tensor(
    snapshot: &SSMTensorSnapshot,
    device: &Device,
) -> Result<Tensor, ModelRuntimeError> {
    let dtype = parse_dtype_label(&snapshot.dtype)?;
    let expected_bytes = snapshot
        .shape
        .iter()
        .copied()
        .fold(1usize, |acc, dim| acc.saturating_mul(dim))
        .saturating_mul(dtype.size_in_bytes());
    if snapshot.bytes.len() != expected_bytes {
        return Err(ModelRuntimeError::KvCacheError(format!(
            "ssm tensor byte length mismatch: dtype={} shape={:?} expected={} got={}",
            snapshot.dtype,
            snapshot.shape,
            expected_bytes,
            snapshot.bytes.len()
        )));
    }
    match dtype {
        DType::F32 => {
            let mut vec = Vec::with_capacity(snapshot.bytes.len() / 4);
            for chunk in snapshot.bytes.chunks_exact(4) {
                vec.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            Tensor::from_vec(vec, snapshot.shape.clone(), device).map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm f32 tensor reconstruction failed: {error}"
                ))
            })
        }
        DType::F16 => {
            let mut vec = Vec::with_capacity(snapshot.bytes.len() / 2);
            for chunk in snapshot.bytes.chunks_exact(2) {
                vec.push(half::f16::from_le_bytes([chunk[0], chunk[1]]));
            }
            Tensor::from_vec(vec, snapshot.shape.clone(), device).map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm f16 tensor reconstruction failed: {error}"
                ))
            })
        }
        DType::BF16 => {
            let mut vec = Vec::with_capacity(snapshot.bytes.len() / 2);
            for chunk in snapshot.bytes.chunks_exact(2) {
                vec.push(half::bf16::from_le_bytes([chunk[0], chunk[1]]));
            }
            Tensor::from_vec(vec, snapshot.shape.clone(), device).map_err(|error| {
                ModelRuntimeError::KvCacheError(format!(
                    "ssm bf16 tensor reconstruction failed: {error}"
                ))
            })
        }
        other => Err(ModelRuntimeError::CapabilityNotSupported {
            capability: format!("ssm_tensor_dtype.{other:?}"),
            adapter: "candle_ssm_state".to_string(),
        }),
    }
}

fn dtype_label(dtype: DType) -> &'static str {
    // candle_core::DType is #[non_exhaustive]; the catch-all keeps this
    // compiling if upstream adds a dtype. In practice dtype_label is only
    // reached for F32/F16/BF16 because tensor_to_snapshot rejects other
    // dtypes before labelling.
    match dtype {
        DType::F32 => "f32",
        DType::F16 => "f16",
        DType::BF16 => "bf16",
        DType::F64 => "f64",
        DType::U8 => "u8",
        DType::U32 => "u32",
        DType::I64 => "i64",
        _ => "unsupported",
    }
}

fn parse_dtype_label(label: &str) -> Result<DType, ModelRuntimeError> {
    match label.trim().to_ascii_lowercase().as_str() {
        "f32" => Ok(DType::F32),
        "f16" => Ok(DType::F16),
        "bf16" => Ok(DType::BF16),
        other => Err(ModelRuntimeError::KvCacheError(format!(
            "unsupported ssm tensor dtype label: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_snapshot_roundtrip_f32() {
        let device = Device::Cpu;
        let original = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (2, 2), &device).unwrap();
        let snap = tensor_to_snapshot(&original).unwrap();
        assert_eq!(snap.dtype, "f32");
        assert_eq!(snap.shape, vec![2, 2]);
        let restored = snapshot_to_tensor(&snap, &device).unwrap();
        let restored_vec = restored.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert_eq!(restored_vec, vec![1.0, 2.0, 3.0, 4.0]);
        // round-trip again to prove the snapshot is byte-stable
        let snap2 = tensor_to_snapshot(&restored).unwrap();
        assert_eq!(snap, snap2);
    }

    #[test]
    fn snapshot_to_tensor_rejects_byte_length_mismatch() {
        let device = Device::Cpu;
        let bad = SSMTensorSnapshot::new("f32".to_string(), vec![2, 2], vec![0u8; 8]).unwrap();
        // 2x2 f32 wants 16 bytes; we passed 8 → must error.
        let err = snapshot_to_tensor(&bad, &device).unwrap_err();
        let message = format!("{err:?}");
        assert!(message.contains("byte length mismatch"), "got: {message}");
    }

    #[test]
    fn parse_dtype_label_rejects_unknown() {
        let err = parse_dtype_label("complex64").unwrap_err();
        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported ssm tensor dtype label"),
            "got: {message}"
        );
    }
}
