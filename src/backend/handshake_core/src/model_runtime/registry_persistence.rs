//! Embedded-SurrealDB authority for durable `ModelRuntime` selections.
//!
//! The live [`ModelRegistry`] is process-local dispatch state. Durable
//! artifact identity, runtime role, adapter binding, lifecycle, active
//! defaults, revisions, and canonical EventLedger linkage are owned by the
//! injected [`SurrealModelRegistryStore`]. Every operation requires the exact
//! five-field resource scope; there is no unscoped or relational fallback.

use std::{collections::BTreeSet, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    kernel::KernelActor,
    storage::surreal::SurrealModelRegistryStore,
    swarm_orchestration::resource_scope::{ExactResourceScopeAttribution, ScopeDenied},
};

use super::{
    BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry, ModelRuntimeError,
    ModelRuntimeRole, OperatorId, ProviderKind, RuntimeBinding,
};

pub const MODEL_RUNTIME_REGISTRY_TABLE: &str = "model_runtime_registry";
pub const MODEL_RUNTIME_ACTIVE_SELECTION_TABLE: &str = "model_runtime_active_selection";
pub const MODEL_RUNTIME_REGISTRY_SCHEMA_ID: &str = "hsk.model_runtime_registry.row@2";
pub const MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID: &str = "hsk.model_runtime.capabilities@1";
pub const MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID: &str = "hsk.model_runtime.selection_event@3";
pub const MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID: &str = "hsk.model_runtime.active_selection@1";
pub const MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID: &str =
    "hsk.model_runtime.active_selection_event@1";

pub(crate) const MODEL_REGISTRY_ROW_CAP: usize = 4_096;
pub(crate) const MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP: usize = 64 * 1_024;

/// Production name retained at the model-runtime boundary while the concrete
/// implementation is explicitly embedded-SurrealDB-only.
pub type ModelRegistryStore = SurrealModelRegistryStore;

/// One exact-scope registry authority injected into boot and runtime control.
///
/// Keeping the scope beside the cloneable store prevents callers from deriving
/// scope from mutable ambient state or accidentally widening a later request.
#[derive(Clone)]
pub struct ScopedModelRegistryAuthority {
    store: ModelRegistryStore,
    scope: ExactResourceScopeAttribution,
}

impl ScopedModelRegistryAuthority {
    pub fn new(store: ModelRegistryStore, scope: ExactResourceScopeAttribution) -> Self {
        Self { store, scope }
    }

    pub fn store(&self) -> &ModelRegistryStore {
        &self.store
    }

    pub fn scope(&self) -> &ExactResourceScopeAttribution {
        &self.scope
    }
}

/// Immutable artifact selection. Host paths and boot-scoped model IDs are
/// observations and therefore deliberately excluded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeSelection {
    pub artifact_sha256: [u8; 32],
    pub runtime_binding: RuntimeBinding,
    pub runtime_role: ModelRuntimeRole,
    pub declared_capabilities: ModelCapabilities,
    pub provider: ProviderKind,
}

impl From<&ModelRegistration> for ModelRuntimeSelection {
    fn from(registration: &ModelRegistration) -> Self {
        Self {
            artifact_sha256: registration.sha256,
            runtime_binding: registration.runtime_binding,
            runtime_role: ModelRuntimeRole::Completion,
            declared_capabilities: registration.declared_capabilities.clone(),
            provider: registration.provider,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoleBoundModelRegistration {
    pub registration: ModelRegistration,
    pub runtime_role: ModelRuntimeRole,
}

impl RoleBoundModelRegistration {
    pub fn completion(registration: ModelRegistration) -> Self {
        Self {
            registration,
            runtime_role: ModelRuntimeRole::Completion,
        }
    }

    pub fn embedding(registration: ModelRegistration) -> Self {
        Self {
            registration,
            runtime_role: ModelRuntimeRole::Embedding,
        }
    }

    pub(crate) fn selection(&self) -> ModelRuntimeSelection {
        ModelRuntimeSelection {
            artifact_sha256: self.registration.sha256,
            runtime_binding: self.registration.runtime_binding,
            runtime_role: self.runtime_role,
            declared_capabilities: self.registration.declared_capabilities.clone(),
            provider: self.registration.provider,
        }
    }
}

/// Operator evidence required after the runtime owner has verified unload.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitModelRuntimeRebind {
    actor: KernelActor,
    reason: String,
    expected_selection_revision: u64,
}

impl ExplicitModelRuntimeRebind {
    pub fn new(
        actor: KernelActor,
        reason: impl Into<String>,
        expected_selection_revision: u64,
    ) -> Result<Self, ModelRegistryPersistenceError> {
        let request = Self {
            actor,
            reason: reason.into().trim().to_owned(),
            expected_selection_revision,
        };
        validate_rebind_request(&request)?;
        Ok(request)
    }

    pub fn actor(&self) -> &KernelActor {
        &self.actor
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn expected_selection_revision(&self) -> u64 {
        self.expected_selection_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRegistryLifecycleState {
    Active,
    Stale,
    Revoked,
}

impl ModelRegistryLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stale => "stale",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PersistedModelRegistration {
    pub schema_id: String,
    pub registry_row_id: String,
    pub artifact_sha256: [u8; 32],
    pub artifact_locator: String,
    pub last_observed_runtime_model_id: ModelId,
    pub runtime_binding: RuntimeBinding,
    pub runtime_role: ModelRuntimeRole,
    pub capabilities_schema_id: String,
    pub declared_capabilities: ModelCapabilities,
    pub provider: ProviderKind,
    pub base_model_tag: BaseModelTag,
    pub last_observed_by: OperatorId,
    pub lifecycle_state: ModelRegistryLifecycleState,
    pub selection_revision: u64,
    pub current_selection_fingerprint: String,
    pub latest_mutation_fingerprint: String,
    pub last_rebind_request_fingerprint: Option<String>,
    pub selection_created_event_id: String,
    pub selection_updated_event_id: String,
    pub selection_created_at_utc: DateTime<Utc>,
    pub selection_updated_at_utc: DateTime<Utc>,
    pub last_observed_at_utc: DateTime<Utc>,
}

impl PersistedModelRegistration {
    pub fn selection(&self) -> ModelRuntimeSelection {
        ModelRuntimeSelection {
            artifact_sha256: self.artifact_sha256,
            runtime_binding: self.runtime_binding,
            runtime_role: self.runtime_role,
            declared_capabilities: self.declared_capabilities.clone(),
            provider: self.provider,
        }
    }

    pub fn rehydrate_with_current_runtime_model_id(
        &self,
        current_runtime_model_id: ModelId,
        artifact_path: PathBuf,
    ) -> Result<ModelRegistration, ModelRegistryPersistenceError> {
        if self.lifecycle_state != ModelRegistryLifecycleState::Active {
            return Err(ModelRegistryPersistenceError::SelectionInactive {
                artifact_sha256: hex::encode(self.artifact_sha256),
                state: self.lifecycle_state.as_str().to_owned(),
            });
        }
        if artifact_path.as_os_str().is_empty() {
            return Err(ModelRegistryPersistenceError::CorruptRow(
                "configured artifact path is empty during registry rehydration".to_owned(),
            ));
        }
        validate_artifact_locator(self.artifact_sha256, &self.artifact_locator)?;
        let registration = ModelRegistration {
            model_id: current_runtime_model_id,
            artifact_path,
            sha256: self.artifact_sha256,
            runtime_binding: self.runtime_binding,
            declared_capabilities: self.declared_capabilities.clone(),
            base_model_tag: self.base_model_tag.clone(),
            registered_at_utc: Utc::now(),
            registered_by: self.last_observed_by.clone(),
            provider: self.provider,
        };
        validate_registration(&registration)?;
        Ok(registration)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModelRuntimeSelectionPurpose {
    #[serde(rename = "application/default")]
    ApplicationDefault,
    #[serde(rename = "embeddings/default")]
    EmbeddingsDefault,
}

impl ModelRuntimeSelectionPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationDefault => "application/default",
            Self::EmbeddingsDefault => "embeddings/default",
        }
    }

    pub const fn runtime_role(self) -> ModelRuntimeRole {
        match self {
            Self::ApplicationDefault => ModelRuntimeRole::Completion,
            Self::EmbeddingsDefault => ModelRuntimeRole::Embedding,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistedActiveModelSelection {
    pub purpose: ModelRuntimeSelectionPurpose,
    pub runtime_role: ModelRuntimeRole,
    pub artifact_sha256: [u8; 32],
    pub lifecycle_state: ModelRegistryLifecycleState,
    pub selection_revision: u64,
    pub latest_mutation_fingerprint: String,
    pub last_request_fingerprint: Option<String>,
    pub selection_created_event_id: String,
    pub selection_updated_event_id: String,
    pub selection_created_at_utc: DateTime<Utc>,
    pub selection_updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum ModelRegistryPersistenceError {
    #[error("model registry persistence rejected registration: {0}")]
    InvalidRegistration(String),
    #[error("model registry embedded SurrealDB error: {0}")]
    Storage(String),
    #[error("model registry persistence serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("model registry persistence audit error: {0}")]
    Audit(String),
    #[error("model registry persistence returned corrupt row: {0}")]
    CorruptRow(String),
    #[error("model registry persistence authority is unavailable: {0}")]
    AuthorityUnavailable(String),
    #[error("model registry selection conflict: {0}")]
    SelectionConflict(String),
    #[error("model registry committed observation mismatch: {0}")]
    ObservationMismatch(String),
    #[error("model registry selection revision mismatch: expected {expected}, found {actual}")]
    SelectionRevisionMismatch { expected: u64, actual: u64 },
    #[error("model registry selection is absent for artifact {0}")]
    SelectionNotFound(String),
    #[error("model registry selection is {state} for artifact {artifact_sha256}")]
    SelectionInactive {
        artifact_sha256: String,
        state: String,
    },
    #[error("model registry explicit rebind rejected: {0}")]
    InvalidRebind(String),
    #[error("model registry resource scope denied: {0}")]
    ScopeDenied(#[from] ScopeDenied),
}

pub(crate) fn validate_selection_set(
    selections: &[ModelRuntimeSelection],
) -> Result<(), ModelRegistryPersistenceError> {
    if selections.len() > MODEL_REGISTRY_ROW_CAP {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
            "configured selection set contains {} rows, exceeding the bounded {MODEL_REGISTRY_ROW_CAP}-row limit",
            selections.len()
        )));
    }
    let mut hashes = BTreeSet::new();
    for selection in selections {
        validate_selection(selection)?;
        if !hashes.insert(selection.artifact_sha256) {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "configured selection set contains duplicate artifact SHA-256 {}",
                hex::encode(selection.artifact_sha256)
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_role_bound_registration_set(
    registrations: &[RoleBoundModelRegistration],
) -> Result<Vec<ModelRuntimeSelection>, ModelRegistryPersistenceError> {
    if registrations.len() > MODEL_REGISTRY_ROW_CAP {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
            "boot registration set contains {} rows, exceeding the bounded {MODEL_REGISTRY_ROW_CAP}-row limit",
            registrations.len()
        )));
    }
    let mut hashes = BTreeSet::new();
    let mut selections = Vec::with_capacity(registrations.len());
    for role_bound in registrations {
        validate_registration(&role_bound.registration)?;
        if !hashes.insert(role_bound.registration.sha256) {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "boot registration set contains duplicate artifact SHA-256 {}",
                hex::encode(role_bound.registration.sha256)
            )));
        }
        if role_bound.runtime_role == ModelRuntimeRole::Embedding
            && (!role_bound
                .registration
                .declared_capabilities
                .supports_embedding
                || role_bound
                    .registration
                    .declared_capabilities
                    .embedding_dimension
                    .is_none())
        {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(
                "embedding role requires embedding capability and a fixed dimension".to_owned(),
            ));
        }
        selections.push(role_bound.selection());
    }
    Ok(selections)
}

pub(crate) fn validate_selection(
    selection: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    if selection.artifact_sha256 == [0; 32] {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(
            "artifact SHA-256 must not be all zeroes".to_owned(),
        ));
    }
    if selection.runtime_role == ModelRuntimeRole::Embedding
        && (!selection.declared_capabilities.supports_embedding
            || selection
                .declared_capabilities
                .embedding_dimension
                .is_none())
    {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(
            "embedding role requires embedding capability and a fixed dimension".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_registration(
    registration: &ModelRegistration,
) -> Result<(), ModelRegistryPersistenceError> {
    for (name, value) in [
        ("base_model_tag", registration.base_model_tag.as_str()),
        ("registered_by", registration.registered_by.as_str()),
    ] {
        if value.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "{name} exceeds the bounded {MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP}-byte persistence limit"
            )));
        }
    }
    let mut registry = ModelRegistry::default();
    registry
        .register(registration.clone())
        .map_err(model_runtime_validation_error)
}

pub(crate) fn validate_rebind_request(
    request: &ExplicitModelRuntimeRebind,
) -> Result<(), ModelRegistryPersistenceError> {
    if !matches!(request.actor, KernelActor::Operator(_)) {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "selection rebind requires an explicit operator actor".to_owned(),
        ));
    }
    if request.actor.actor_id().trim().is_empty()
        || request.actor.actor_id().len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        || request.reason.trim().is_empty()
        || request.reason.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        || request.expected_selection_revision == 0
    {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "rebind requires bounded actor/reason text and a nonzero expected revision".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn ensure_selection_matches(
    persisted: &PersistedModelRegistration,
    attempted: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    require_active_registration(persisted)?;
    if persisted.selection() == *attempted {
        return Ok(());
    }
    Err(ModelRegistryPersistenceError::SelectionConflict(format!(
        "artifact {} is already revision {} with adapter `{}` and a different immutable selection",
        hex::encode(attempted.artifact_sha256),
        persisted.selection_revision,
        persisted.runtime_binding.adapter_id()
    )))
}

pub(crate) fn ensure_runtime_binding_matches(
    persisted: &PersistedModelRegistration,
    attempted: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    require_active_registration(persisted)?;
    if persisted.artifact_sha256 == attempted.artifact_sha256
        && persisted.runtime_binding == attempted.runtime_binding
        && persisted.provider == attempted.provider
        && persisted.runtime_role == attempted.runtime_role
    {
        return Ok(());
    }
    Err(ModelRegistryPersistenceError::SelectionConflict(format!(
        "artifact {} has a different persisted adapter/provider/role identity",
        hex::encode(attempted.artifact_sha256)
    )))
}

pub(crate) fn require_active_registration(
    persisted: &PersistedModelRegistration,
) -> Result<(), ModelRegistryPersistenceError> {
    if persisted.lifecycle_state == ModelRegistryLifecycleState::Active {
        Ok(())
    } else {
        Err(ModelRegistryPersistenceError::SelectionInactive {
            artifact_sha256: hex::encode(persisted.artifact_sha256),
            state: persisted.lifecycle_state.as_str().to_owned(),
        })
    }
}

pub(crate) fn artifact_locator_for_sha256(sha256: [u8; 32]) -> String {
    format!("artifact://sha256/{}", hex::encode(sha256))
}

pub(crate) fn validate_artifact_locator(
    artifact_sha256: [u8; 32],
    artifact_locator: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    let expected = artifact_locator_for_sha256(artifact_sha256);
    if artifact_locator == expected {
        Ok(())
    } else {
        Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "artifact locator does not bind to persisted SHA-256 {}",
            hex::encode(artifact_sha256)
        )))
    }
}

pub(crate) const fn runtime_binding_token(binding: RuntimeBinding) -> &'static str {
    match binding {
        RuntimeBinding::LlamaCpp => "llama_cpp",
        RuntimeBinding::Candle => "candle",
    }
}

pub(crate) fn parse_runtime_binding(
    token: &str,
) -> Result<RuntimeBinding, ModelRegistryPersistenceError> {
    match token {
        "llama_cpp" => Ok(RuntimeBinding::LlamaCpp),
        "candle" => Ok(RuntimeBinding::Candle),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown runtime_binding `{token}`"
        ))),
    }
}

pub(crate) fn parse_runtime_role(
    token: &str,
) -> Result<ModelRuntimeRole, ModelRegistryPersistenceError> {
    match token {
        "completion" => Ok(ModelRuntimeRole::Completion),
        "embedding" => Ok(ModelRuntimeRole::Embedding),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown runtime_role `{token}`"
        ))),
    }
}

pub(crate) const fn provider_token(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ExternalCompat => "external_compat",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
    }
}

pub(crate) fn parse_provider(token: &str) -> Result<ProviderKind, ModelRegistryPersistenceError> {
    match token {
        "local" => Ok(ProviderKind::Local),
        "external_compat" => Ok(ProviderKind::ExternalCompat),
        "byok_cloud" => Ok(ProviderKind::ByokCloud),
        "official_cli" => Ok(ProviderKind::OfficialCli),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown provider `{token}`"
        ))),
    }
}

fn model_runtime_validation_error(error: ModelRuntimeError) -> ModelRegistryPersistenceError {
    ModelRegistryPersistenceError::InvalidRegistration(error.to_string())
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileSnapshotV1 {
    pub profile: handshake_dexterity::EngineProfile,
    pub fingerprint: String,
}

impl EngineProfileSnapshotV1 {
    pub fn new(profile: handshake_dexterity::EngineProfile) -> Result<Self, EngineProfileError> {
        let resolved = handshake_dexterity::Registry::new(vec![profile.clone()])
            .and_then(|registry| registry.resolve(&profile.id))
            .map_err(|_| EngineProfileError::InvalidProfile)?;
        Ok(Self { profile, fingerprint: resolved.fingerprint().to_owned() })
    }

    pub fn validate(&self) -> Result<(), EngineProfileError> {
        if Self::new(self.profile.clone())?.fingerprint != self.fingerprint {
            return Err(EngineProfileError::FingerprintMismatch);
        }
        Ok(())
    }

    pub fn identity(&self) -> EngineProfileIdentityV1 {
        EngineProfileIdentityV1 {
            profile_id: self.profile.id.clone(),
            revision: self.profile.revision,
            fingerprint: self.fingerprint.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileIdentityV1 {
    pub profile_id: String,
    pub revision: u64,
    pub fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileDefinitionV1 {
    pub profile_id: String,
    pub revisions: Vec<EngineProfileSnapshotV1>,
    pub head_generation: u64,
    pub tombstoned: bool,
}

impl EngineProfileDefinitionV1 {
    pub fn head(&self) -> Result<&EngineProfileSnapshotV1, EngineProfileError> {
        self.revisions.last().ok_or(EngineProfileError::Integrity)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileRegistryV1 {
    pub registry_generation: u64,
    pub profiles: Vec<EngineProfileDefinitionV1>,
    pub active_generation: u64,
    pub active: Option<EngineProfileIdentityV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineProfileMutationV1 {
    Create { snapshot: EngineProfileSnapshotV1 },
    NewRevision {
        expected_head: EngineProfileIdentityV1,
        expected_head_generation: u64,
        snapshot: EngineProfileSnapshotV1,
    },
    Tombstone {
        expected_head: EngineProfileIdentityV1,
        expected_head_generation: u64,
    },
    Activate {
        expected_head: EngineProfileIdentityV1,
        expected_head_generation: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileMutationRequestV1 {
    pub request_id: uuid::Uuid,
    pub reason: String,
    pub expected_registry_generation: u64,
    pub expected_active_generation: u64,
    pub mutation: EngineProfileMutationV1,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum EngineProfileError {
    #[error("engine_profile_invalid")]
    InvalidProfile,
    #[error("engine_profile_fingerprint_mismatch")]
    FingerprintMismatch,
    #[error("engine_profile_conflict")]
    Conflict,
    #[error("engine_profile_unavailable")]
    ProfileUnavailable,
    #[error("engine_profile_scope_denied")]
    ScopeDenied,
    #[error("engine_profile_integrity")]
    Integrity,
    #[error("engine_profile_capacity_exceeded")]
    CapacityExceeded,
    #[error("engine_profile_storage_unavailable")]
    StorageUnavailable,
}

impl EngineProfileMutationRequestV1 {
    pub fn validate(&self) -> Result<(), EngineProfileError> {
        if self.request_id.is_nil() || self.reason.trim().is_empty()
            || self.reason.len() > 512
            || self.reason.chars().any(char::is_control)
            || self.expected_registry_generation >= i64::MAX as u64
            || self.expected_active_generation >= i64::MAX as u64
        {
            return Err(EngineProfileError::InvalidProfile);
        }
        if let EngineProfileMutationV1::Create { snapshot }
            | EngineProfileMutationV1::NewRevision { snapshot, .. } = &self.mutation
        {
            snapshot.validate()?;
        }
        Ok(())
    }
}

impl EngineProfileRegistryV1 {
    pub fn validate(&self) -> Result<(), EngineProfileError> {
        if serde_json::to_vec(self).map_err(|_| EngineProfileError::Integrity)?.len() > 1_048_576 {
            return Err(EngineProfileError::CapacityExceeded);
        }
        if self.registry_generation > i64::MAX as u64
            || self.active_generation > i64::MAX as u64
            || self.profiles.len() > MODEL_REGISTRY_ROW_CAP
        {
            return Err(EngineProfileError::Integrity);
        }
        let mut ids = BTreeSet::new();
        for definition in &self.profiles {
            if !ids.insert(definition.profile_id.as_str()) || definition.revisions.is_empty() {
                return Err(EngineProfileError::Integrity);
            }
            for (index, snapshot) in definition.revisions.iter().enumerate() {
                snapshot.validate()?;
                if snapshot.profile.id != definition.profile_id
                    || snapshot.profile.revision != index as u64 + 1
                {
                    return Err(EngineProfileError::Integrity);
                }
            }
            if definition.head_generation != definition.revisions.len() as u64
                + u64::from(definition.tombstoned)
            {
                return Err(EngineProfileError::Integrity);
            }
        }
        if let Some(active) = &self.active {
            let definition = self.profiles.iter()
                .find(|profile| profile.profile_id == active.profile_id && !profile.tombstoned)
                .ok_or(EngineProfileError::Integrity)?;
            if definition.head()?.identity() != *active {
                return Err(EngineProfileError::Integrity);
            }
        }
        Ok(())
    }

    pub fn apply(&self, request: &EngineProfileMutationRequestV1) -> Result<Self, EngineProfileError> {
        self.validate()?;
        request.validate()?;
        if request.expected_registry_generation != self.registry_generation
            || request.expected_active_generation != self.active_generation
        {
            return Err(EngineProfileError::Conflict);
        }
        let mut next = self.clone();
        if let EngineProfileMutationV1::Create { snapshot } = &request.mutation {
            if snapshot.profile.revision != 1 || next.profiles.len() >= MODEL_REGISTRY_ROW_CAP
                || next.profiles.iter().any(|profile| profile.profile_id == snapshot.profile.id)
            {
                return Err(EngineProfileError::Conflict);
            }
            next.profiles.push(EngineProfileDefinitionV1 {
                profile_id: snapshot.profile.id.clone(),
                revisions: vec![snapshot.clone()],
                head_generation: 1,
                tombstoned: false,
            });
        } else {
            let (expected, generation) = match &request.mutation {
                EngineProfileMutationV1::NewRevision { expected_head, expected_head_generation, .. }
                | EngineProfileMutationV1::Tombstone { expected_head, expected_head_generation }
                | EngineProfileMutationV1::Activate { expected_head, expected_head_generation } =>
                    (expected_head, *expected_head_generation),
                EngineProfileMutationV1::Create { .. } => unreachable!(),
            };
            let definition = next.profiles.iter_mut()
                .find(|profile| profile.profile_id == expected.profile_id && !profile.tombstoned)
                .ok_or(EngineProfileError::ProfileUnavailable)?;
            if definition.head_generation != generation || definition.head()?.identity() != *expected {
                return Err(EngineProfileError::Conflict);
            }
            match &request.mutation {
                EngineProfileMutationV1::NewRevision { snapshot, .. } => {
                    if snapshot.profile.id != expected.profile_id
                        || snapshot.profile.revision != expected.revision.checked_add(1)
                            .ok_or(EngineProfileError::Conflict)?
                    {
                        return Err(EngineProfileError::Conflict);
                    }
                    definition.revisions.push(snapshot.clone());
                    definition.head_generation = definition.head_generation.checked_add(1)
                        .ok_or(EngineProfileError::Conflict)?;
                    if next.active.as_ref() == Some(expected) {
                        next.active = None;
                        next.active_generation += 1;
                    }
                }
                EngineProfileMutationV1::Tombstone { .. } => {
                    definition.tombstoned = true;
                    definition.head_generation = definition.head_generation.checked_add(1)
                        .ok_or(EngineProfileError::Conflict)?;
                    if next.active.as_ref() == Some(expected) {
                        next.active = None;
                        next.active_generation += 1;
                    }
                }
                EngineProfileMutationV1::Activate { .. } => {
                    next.active = Some(expected.clone());
                    next.active_generation += 1;
                }
                EngineProfileMutationV1::Create { .. } => unreachable!(),
            }
        }
        next.registry_generation += 1;
        if serde_json::to_vec(&next).map_err(|_| EngineProfileError::Integrity)?.len() > 1_048_576 {
            return Err(EngineProfileError::CapacityExceeded);
        }
        next.validate()?;
        Ok(next)
    }
}
