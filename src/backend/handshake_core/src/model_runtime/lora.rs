use std::{fmt, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelRuntimeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoraId(Uuid);

impl LoraId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn try_from_uuid(value: Uuid) -> Result<Self, ModelRuntimeError> {
        if value.get_version_num() != 7 {
            return Err(ModelRuntimeError::LoraStackError(format!(
                "LoRA id must be UUID v7; got {value}"
            )));
        }
        Ok(Self(value))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for LoraId {
    fn default() -> Self {
        Self::new_v7()
    }
}

impl fmt::Display for LoraId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone)]
pub struct LoraStackHandle {
    id: String,
    ops: Arc<dyn LoraStackOps>,
}

impl LoraStackHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            id: value.into(),
            ops: Arc::new(UnsupportedLoraStackOps),
        }
    }

    pub fn with_ops(value: impl Into<String>, ops: Arc<dyn LoraStackOps>) -> Self {
        Self {
            id: value.into(),
            ops,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        self.ops.mount(desc, strength).await
    }

    pub async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        self.ops.unmount(id).await
    }

    pub fn list_active(&self) -> Vec<LoraStackEntry> {
        self.ops.list_active()
    }

    pub async fn set_strength(
        &self,
        id: LoraId,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        self.ops.set_strength(id, strength).await
    }

    pub async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        self.ops.swap(new_stack).await
    }
}

impl fmt::Debug for LoraStackHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoraStackHandle")
            .field("id", &self.id)
            .field("ops", &"<dyn LoraStackOps>")
            .finish()
    }
}

impl PartialEq for LoraStackHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LoraStackHandle {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStrength(f32);

impl LoraStrength {
    pub fn try_new(value: f32) -> Result<Self, ModelRuntimeError> {
        if !value.is_finite() || !(0.0..=2.0).contains(&value) {
            return Err(ModelRuntimeError::LoraStackError(format!(
                "LoRA strength must be finite and in range 0.0..=2.0; got {value}"
            )));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BaseModelTag(String);

impl BaseModelTag {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("base model tag must not be empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ModelRuntimeError> {
        let normalized = value.into().trim().to_string();
        if normalized.is_empty() {
            return Err(ModelRuntimeError::LoraStackError(
                "base model tag must not be empty".to_string(),
            ));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BaseModelTag {
    fn default() -> Self {
        Self::new("unknown")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LicenseTag(String);

impl LicenseTag {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("license tag must not be empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ModelRuntimeError> {
        let normalized = value.into().trim().to_string();
        if normalized.is_empty() {
            return Err(ModelRuntimeError::LoraStackError(
                "license tag must not be empty".to_string(),
            ));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoraDescriptor {
    pub id: LoraId,
    pub artifact_path: PathBuf,
    pub sha256: [u8; 32],
    pub rank: u32,
    pub target_modules: Vec<String>,
    pub base_model_compat: BaseModelTag,
    pub license_tag: LicenseTag,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStackEntry {
    pub id: LoraId,
    pub strength: LoraStrength,
    pub mounted_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoraStackSnapshot {
    pub entries: Vec<LoraStackSnapshotEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoraStackSnapshotEntry {
    pub descriptor: LoraDescriptor,
    pub strength: LoraStrength,
    pub mounted_at_utc: DateTime<Utc>,
}

#[async_trait]
pub trait LoraStackOps: Send + Sync {
    async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError>;

    async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError>;

    fn list_active(&self) -> Vec<LoraStackEntry>;

    async fn set_strength(
        &self,
        id: LoraId,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError>;

    async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError>;
}

struct UnsupportedLoraStackOps;

#[async_trait]
impl LoraStackOps for UnsupportedLoraStackOps {
    async fn mount(
        &self,
        _desc: LoraDescriptor,
        _strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        Err(unsupported_lora_error())
    }

    async fn unmount(&self, _id: LoraId) -> Result<(), ModelRuntimeError> {
        Err(unsupported_lora_error())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        Vec::new()
    }

    async fn set_strength(
        &self,
        _id: LoraId,
        _strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        Err(unsupported_lora_error())
    }

    async fn swap(
        &self,
        _new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        Err(unsupported_lora_error())
    }
}

fn unsupported_lora_error() -> ModelRuntimeError {
    ModelRuntimeError::CapabilityNotSupported {
        capability: "lora_stack".to_string(),
        adapter: "unbound_lora_stack_handle".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_runtime_lora_unit_surface_is_filter_visible() {
        fn assert_object_safe(_: Box<dyn LoraStackOps>) {}

        assert_object_safe(Box::new(UnsupportedLoraStackOps));
        assert!(LoraStrength::try_new(f32::INFINITY).is_err());
        assert!(BaseModelTag::try_new("").is_err());
        assert!(LicenseTag::try_new("").is_err());
        assert_eq!(LoraId::new_v7().as_uuid().get_version_num(), 7);
    }
}
