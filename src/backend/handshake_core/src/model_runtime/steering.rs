use std::{collections::BTreeMap, fmt, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelRuntimeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SteeringVectorId(Uuid);

impl SteeringVectorId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SteeringVectorId {
    fn default() -> Self {
        Self::new_v7()
    }
}

impl From<Uuid> for SteeringVectorId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl fmt::Display for SteeringVectorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone)]
pub struct SteeringHookHandle {
    id: String,
    ops: Arc<dyn SteeringHookOps>,
}

impl SteeringHookHandle {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            id: value.into(),
            ops: Arc::new(UnsupportedSteeringHookOps),
        }
    }

    pub fn with_ops(value: impl Into<String>, ops: Arc<dyn SteeringHookOps>) -> Self {
        Self {
            id: value.into(),
            ops,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        self.ops.capture(spec).await
    }

    pub async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        self.ops.register_vector(vector).await
    }

    pub fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        self.ops.list_vectors()
    }

    pub async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        self.ops.set_active(ids).await
    }

    pub async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        self.ops.unregister(id).await
    }
}

impl fmt::Debug for SteeringHookHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SteeringHookHandle")
            .field("id", &self.id)
            .field("ops", &"<dyn SteeringHookOps>")
            .finish()
    }
}

impl PartialEq for SteeringHookHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SteeringHookHandle {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LayerIndex(u32);

impl LayerIndex {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    ResidStream,
    MlpOut,
    AttnOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteeringVectorValues {
    values: Vec<f32>,
    intensity: f32,
}

impl SteeringVectorValues {
    pub fn try_new(values: Vec<f32>, intensity: f32) -> Result<Self, ModelRuntimeError> {
        if values.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "steering vector values must not be empty".to_string(),
            ));
        }
        if !intensity.is_finite() || !(-10.0..=10.0).contains(&intensity) {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "steering vector intensity must be finite and in range -10.0..=10.0; got {intensity}"
            )));
        }
        Ok(Self { values, intensity })
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContrastiveTechnique {
    CAA,
    RefusalVector,
    RepE,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteeringProvenance {
    Manual {
        author: String,
        notes: String,
    },
    Contrastive {
        positive_prompts: Vec<String>,
        negative_prompts: Vec<String>,
        technique: ContrastiveTechnique,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteeringVector {
    pub id: SteeringVectorId,
    pub name: String,
    pub layer: LayerIndex,
    pub hook_point: HookPoint,
    pub values: SteeringVectorValues,
    pub description: String,
    pub derivation_provenance: SteeringProvenance,
}

impl SteeringVector {
    pub fn try_new(
        id_proposed: Option<SteeringVectorId>,
        name: impl Into<String>,
        layer: LayerIndex,
        hook_point: HookPoint,
        values: SteeringVectorValues,
        description: impl Into<String>,
        derivation_provenance: Option<SteeringProvenance>,
    ) -> Result<Self, ModelRuntimeError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "steering vector name must not be empty".to_string(),
            ));
        }
        let description = description.into().trim().to_string();
        if description.is_empty() {
            return Err(ModelRuntimeError::SteeringHookError(
                "steering vector description must not be empty".to_string(),
            ));
        }
        let derivation_provenance = derivation_provenance.ok_or_else(|| {
            ModelRuntimeError::SteeringHookError(
                "steering vector provenance is required".to_string(),
            )
        })?;
        if let SteeringProvenance::Manual { author, .. } = &derivation_provenance {
            if author.trim().is_empty() {
                return Err(ModelRuntimeError::SteeringHookError(
                    "steering vector manual provenance author is required".to_string(),
                ));
            }
        }
        Ok(Self {
            id: id_proposed.unwrap_or_default(),
            name,
            layer,
            hook_point,
            values,
            description,
            derivation_provenance,
        })
    }

    /// True when this vector is a refusal-direction ablation marker
    /// (`Contrastive { technique: RefusalVector, .. }`). The runtime apply
    /// path keys on this to switch from additive steering
    /// (`base + steering * intensity`) to Arditi et al. 2024 directional
    /// ablation (`resid - (resid·dir / dir·dir) * dir`). The intensity on
    /// such a vector is a marker (`REFUSAL_ABLATION_INTENSITY`), not an
    /// additive scale.
    pub fn is_refusal_ablation(&self) -> bool {
        matches!(
            &self.derivation_provenance,
            SteeringProvenance::Contrastive {
                technique: ContrastiveTechnique::RefusalVector,
                ..
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteeringVectorMeta {
    pub id: SteeringVectorId,
    pub name: String,
    pub layer: LayerIndex,
    pub hook_point: HookPoint,
    pub intensity: f32,
    pub description: String,
}

impl From<&SteeringVector> for SteeringVectorMeta {
    fn from(value: &SteeringVector) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            layer: value.layer,
            hook_point: value.hook_point,
            intensity: value.values.intensity(),
            description: value.description.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSpec {
    pub prompts: Vec<String>,
    pub layers: Vec<LayerIndex>,
    pub hook_point: HookPoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptureResult {
    pub activations: BTreeMap<LayerIndex, Vec<Vec<f32>>>,
    pub tokens_seen: u32,
}

#[async_trait]
pub trait SteeringHookOps: Send + Sync {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError>;

    async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError>;

    fn list_vectors(&self) -> Vec<SteeringVectorMeta>;

    async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError>;

    async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError>;
}

struct UnsupportedSteeringHookOps;

#[async_trait]
impl SteeringHookOps for UnsupportedSteeringHookOps {
    async fn capture(&self, _spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        Err(unsupported_steering_error())
    }

    async fn register_vector(
        &self,
        _vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        Err(unsupported_steering_error())
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        Vec::new()
    }

    async fn set_active(&self, _ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        Err(unsupported_steering_error())
    }

    async fn unregister(&self, _id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        Err(unsupported_steering_error())
    }
}

fn unsupported_steering_error() -> ModelRuntimeError {
    ModelRuntimeError::CapabilityNotSupported {
        capability: "activation_steering".to_string(),
        adapter: "unbound_steering_hook_handle".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_runtime_steering_unit_surface_is_filter_visible() {
        fn assert_object_safe(_: Box<dyn SteeringHookOps>) {}

        assert_object_safe(Box::new(UnsupportedSteeringHookOps));
        assert!(SteeringVectorValues::try_new(Vec::new(), 1.0).is_err());
        assert!(SteeringVectorValues::try_new(vec![1.0], f32::INFINITY).is_err());
        assert_eq!(SteeringVectorId::new_v7().as_uuid().get_version_num(), 7);
    }
}
