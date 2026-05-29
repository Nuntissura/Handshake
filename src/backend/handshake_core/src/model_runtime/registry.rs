use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::process_ledger::ProcessEngineKind;

use super::{
    BaseModelTag, ModelCapabilities, ModelId, ModelRuntimeError, ProviderKind, RuntimeKind,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(String);

impl OperatorId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("operator id must not be empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ModelRuntimeError> {
        let normalized = value.into().trim().to_string();
        if normalized.is_empty() {
            return Err(ModelRuntimeError::LoadError(
                "operator id must not be empty".to_string(),
            ));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBinding {
    LlamaCpp,
    Candle,
}

impl RuntimeBinding {
    pub fn runtime_kind(self) -> RuntimeKind {
        match self {
            Self::LlamaCpp => RuntimeKind::LlamaCpp,
            Self::Candle => RuntimeKind::Candle,
        }
    }

    pub fn adapter_id(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama_cpp",
            Self::Candle => "candle",
        }
    }

    pub fn process_engine_kind(self) -> ProcessEngineKind {
        match self {
            Self::LlamaCpp => ProcessEngineKind::LlamaCpp,
            Self::Candle => ProcessEngineKind::Candle,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRegistration {
    pub model_id: ModelId,
    pub artifact_path: PathBuf,
    pub sha256: [u8; 32],
    pub runtime_binding: RuntimeBinding,
    pub declared_capabilities: ModelCapabilities,
    pub base_model_tag: BaseModelTag,
    pub registered_at_utc: DateTime<Utc>,
    pub registered_by: OperatorId,
    pub provider: ProviderKind,
}

#[derive(Debug, Default)]
pub struct ModelRegistry {
    registrations: HashMap<ModelId, ModelRegistration>,
    loaded_model_ids: HashMap<ModelId, RuntimeBinding>,
}

impl ModelRegistry {
    pub fn register(&mut self, reg: ModelRegistration) -> Result<(), ModelRuntimeError> {
        Self::validate_registration(&reg)?;

        if self.registrations.contains_key(&reg.model_id) {
            return Err(ModelRuntimeError::LoadError(format!(
                "model registration already exists: {}",
                reg.model_id
            )));
        }

        self.registrations.insert(reg.model_id, reg);
        Ok(())
    }

    pub fn lookup(&self, id: ModelId) -> Option<&ModelRegistration> {
        self.registrations.get(&id)
    }

    pub fn list(&self) -> Vec<&ModelRegistration> {
        let mut registrations = self.registrations.values().collect::<Vec<_>>();
        registrations.sort_by_key(|registration| registration.model_id.to_string());
        registrations
    }

    pub fn rebind(
        &mut self,
        id: ModelId,
        new_binding: RuntimeBinding,
    ) -> Result<(), ModelRuntimeError> {
        if self.loaded_model_ids.contains_key(&id) {
            return Err(ModelRuntimeError::LoadError(format!(
                "model registration cannot be rebound while loaded: {id}"
            )));
        }

        let registration = self.registrations.get_mut(&id).ok_or_else(|| {
            ModelRuntimeError::LoadError(format!("model is not registered: {id}"))
        })?;

        Self::validate_binding(new_binding, &registration.declared_capabilities)?;
        registration.runtime_binding = new_binding;
        Ok(())
    }

    pub fn mark_loaded(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        let binding = self
            .registrations
            .get(&id)
            .map(|registration| registration.runtime_binding)
            .ok_or_else(|| {
                ModelRuntimeError::LoadError(format!("model is not registered: {id}"))
            })?;

        self.loaded_model_ids.insert(id, binding);
        Ok(())
    }

    pub fn mark_unloaded(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        if !self.registrations.contains_key(&id) {
            return Err(ModelRuntimeError::UnloadError(format!(
                "model is not registered: {id}"
            )));
        }

        self.loaded_model_ids.remove(&id);
        Ok(())
    }

    pub fn is_loaded(&self, id: ModelId) -> bool {
        self.loaded_model_ids.contains_key(&id)
    }

    fn validate_registration(reg: &ModelRegistration) -> Result<(), ModelRuntimeError> {
        Self::validate_model_id(reg.model_id)?;

        if reg.provider != ProviderKind::Local {
            return Err(ModelRuntimeError::LoadError(format!(
                "model registry accepts only local provider registrations; got {:?}",
                reg.provider
            )));
        }

        if reg.artifact_path.as_os_str().is_empty() {
            return Err(ModelRuntimeError::LoadError(
                "model registration artifact_path must not be empty".to_string(),
            ));
        }

        if reg.sha256 == [0; 32] {
            return Err(ModelRuntimeError::LoadError(
                "model registration sha256 must not be all zeroes".to_string(),
            ));
        }

        Self::validate_binding(reg.runtime_binding, &reg.declared_capabilities)
    }

    fn validate_model_id(id: ModelId) -> Result<(), ModelRuntimeError> {
        if id.as_uuid().get_version_num() != 7 {
            return Err(ModelRuntimeError::LoadError(format!(
                "model registration id must be UUID v7; got {id}"
            )));
        }
        Ok(())
    }

    fn validate_binding(
        runtime_binding: RuntimeBinding,
        capabilities: &ModelCapabilities,
    ) -> Result<(), ModelRuntimeError> {
        if runtime_binding == RuntimeBinding::LlamaCpp && capabilities.supports_activation_steering
        {
            return Err(ModelRuntimeError::LoadError(format!(
                "activation_steering requires candle binding; {} must declare supports_activation_steering=false",
                runtime_binding.adapter_id()
            )));
        }

        Ok(())
    }
}
