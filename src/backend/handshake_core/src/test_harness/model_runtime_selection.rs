//! Finite, fixture-owned fault at the canonical active-selection audit append.
use crate::{
    kernel::KernelActor,
    model_runtime::{ModelRegistryPersistenceError, ModelRuntimeSelectionPurpose},
    storage::surreal::SurrealModelRegistryStore,
    swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
};

/// Arms one exact selection request. Other scopes, targets and operations cannot consume it.
/// Clones of this store share the finite fault; new production stores do not.
pub fn fail_next_selection_audit_append(
    store: &SurrealModelRegistryStore,
    scope: &ExactResourceScopeAttribution,
    purpose: ModelRuntimeSelectionPurpose,
    target: [u8; 32],
    expected_revision: u64,
    actor: &KernelActor,
    reason: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    store.arm_active_selection_audit_fault(scope, purpose, target, expected_revision, actor, reason)
}

/// Exercises the same issued-credential initializer admission used by the private activation
/// request carrier. It never constructs record authority or grants and is test-utils only.
pub async fn with_issued_registry_initializer<T>(
    state: &crate::AccountBootstrapState,
    headers: &axum::http::HeaderMap,
    expected: &ExactResourceScopeAttribution,
    operation: impl std::future::Future<Output = T>,
) -> Result<T, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    crate::api::authority::with_registry_initializer_authority(state, headers, expected, operation).await
}
/// Exact issued operator Update boundary for registry CAS/rebind component consumers.
/// This does not authorize initialization, process-runtime ownership, or arbitrary SQL.
pub async fn with_issued_registry_selection_update<T>(
    state: &crate::AccountBootstrapState,
    headers: &axum::http::HeaderMap,
    expected: &ExactResourceScopeAttribution,
    operation: impl std::future::Future<Output = T>,
) -> Result<T, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let admission = crate::api::authority::authorize_bootstrap_request(
        state, headers, "fs.write", ResourceKind::Workspace,
        expected.workspace_id.as_str(), ResourceAction::Update,
    ).await?;
    if admission.actor_kind != "operator"
        || admission.account_id != expected.owner_account_id.as_uuid().to_string()
        || admission.principal_id != expected.actor_principal_id.as_uuid().to_string()
        || admission.session_id != expected.authenticated_session_id.as_uuid().to_string()
        || admission.access_space_id != expected.access_space_id.as_uuid().to_string()
    {
        return Err(crate::api::authority::constant_denial());
    }
    Ok(state.surreal.with_record_user_scope(
        admission.record_user_scope,
        crate::storage::surreal::event_ledger::with_loom_session_receipt(
            KernelActor::Operator(admission.actor_id), expected.workspace_id.as_str().to_owned(), operation,
        ),
    ).await)
}
/// Owner of actual production-loaded fixture models. Its process runtime must be created by
/// the existing production constructor (which owns a real OS lease), never a descriptor shim.
pub struct GenuineModelSelectionFixture {
    runtime: crate::process_ledger::ProcessReclaimRuntime,
    account: crate::AccountBootstrapState,
    headers: axum::http::HeaderMap,
    scope: ExactResourceScopeAttribution,
    client: Option<std::sync::Arc<dyn crate::llm::LlmClient>>,
    selection_owner: Option<std::sync::Arc<crate::llm::local_router::LocalModelRuntimeLlmClient>>,
    current_model_id: Option<crate::model_runtime::ModelId>,
    target_model_id: Option<crate::model_runtime::ModelId>,
}

/// Failure retains the actual runtime/model owner for explicit bounded settlement.
pub struct GenuineModelFixtureFailure {
    pub reason: String,
    pub retained: Box<GenuineModelSelectionFixture>,
}

impl std::fmt::Debug for GenuineModelFixtureFailure {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.debug_struct("GenuineModelFixtureFailure").field("reason", &self.reason)
            .field("retained_owner", &true).finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenuineDefaultSnapshot {
    pub runtime_model_id: String,
    pub selection: crate::model_runtime::PersistedActiveModelSelection,
    pub canonical_events: Vec<crate::kernel::KernelEvent>,
}

impl GenuineModelSelectionFixture {
    pub async fn load_current(
        resolved: &crate::llm::registry::ResolvedProvider,
        target_input: &GenuineCompatibleCompletionInput,
        runtime: crate::process_ledger::ProcessReclaimRuntime,
        account: crate::AccountBootstrapState,
        headers: axum::http::HeaderMap,
        scope: ExactResourceScopeAttribution,
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
    ) -> Result<Self, GenuineModelFixtureFailure> {
        let mut fixture = Self { runtime, account, headers, scope, client: None,
            selection_owner: None, current_model_id: None, target_model_id: None };
        let local = match resolved.local_model.as_ref() {
            Some(local) => local,
            None => return Err(GenuineModelFixtureFailure {
                reason: "explicit selected local model configuration is missing".into(), retained: Box::new(fixture),
            }),
        };
        let target = match target_input.resolve(local) {
            Ok(target) => target,
            Err(reason) => return Err(GenuineModelFixtureFailure { reason, retained: Box::new(fixture) }),
        };
        let authority = crate::model_runtime::ScopedModelRegistryAuthority::new(
            SurrealModelRegistryStore::new(fixture.account.surreal.clone()), fixture.scope.clone(),
        );
        let loaded = crate::llm::boot::load_genuine_configured_fixture(
            resolved, recorder, &fixture.runtime, authority, &fixture.account, &fixture.headers, Some(&target),
        ).await;
        match loaded {
            Ok((client, owner)) => { fixture.client = Some(client); fixture.selection_owner = owner; },
            Err(error) => return Err(GenuineModelFixtureFailure {
                reason: error.to_string(), retained: Box::new(fixture),
            }),
        }
        let checked = async {
            let client = fixture.client()?;
            let concrete = fixture.selection_owner.as_ref().ok_or("actual loader did not return its concrete selection owner")?;
            let erased: std::sync::Arc<dyn crate::llm::LlmClient> = concrete.clone();
            if !std::sync::Arc::ptr_eq(&client, &erased) {
                return Err("fixture client differs from actual concrete production owner".into());
            }
            let selected = client.selected_model_id();
            let id = crate::model_runtime::ModelId::from(uuid::Uuid::parse_str(&selected).map_err(|error| error.to_string())?);
            let entry = client.model_catalog().and_then(|catalog| catalog.entry(&selected))
                .ok_or("actual configured load did not publish a current catalog entry")?;
            if !entry.ready || !entry.default_selectable
                || entry.runtime_role != crate::model_runtime::ModelRuntimeRole::Completion
                || entry.artifact_sha256 != hex::encode(local.sha256)
                || entry.runtime_binding != local.runtime_binding.adapter_id()
            {
                return Err("actual configured load failed READY/Completion/artifact/binding checks".into());
            }
            fixture.observe_ownership(id, local.sha256, local.runtime_binding).await?;
            let target_entry = client.model_catalog().and_then(|catalog| catalog.entry_for_stable_anchor(&hex::encode(target.sha256)))
                .ok_or("actual compatible Completion target is absent")?;
            if !target_entry.ready || !target_entry.default_selectable
                || target_entry.runtime_role != crate::model_runtime::ModelRuntimeRole::Completion
                || target_entry.runtime_binding != target.runtime_binding.adapter_id() {
                return Err("actual compatible target failed READY/Completion/binding checks".into());
            }
            let target_id = crate::model_runtime::ModelId::from(uuid::Uuid::parse_str(&target_entry.model_id).map_err(|error| error.to_string())?);
            if target_id == id { return Err("compatible Completion target has the same runtime identity".into()); }
            fixture.observe_ownership(target_id, target.sha256, target.runtime_binding).await?;
            fixture.target_model_id = Some(target_id);
            Ok::<_, String>(id)
        }.await;
        match checked {
            Ok(id) => { fixture.current_model_id = Some(id); Ok(fixture) },
            Err(reason) => Err(GenuineModelFixtureFailure { reason, retained: Box::new(fixture) }),
        }
    }

    pub fn client(&self) -> Result<std::sync::Arc<dyn crate::llm::LlmClient>, String> {
        self.client.clone().ok_or("actual fixture client is unavailable".into())
    }

    pub fn current_model_id(&self) -> Result<crate::model_runtime::ModelId, String> {
        self.current_model_id.ok_or("actual current model load was not verified".into())
    }

    pub fn target_model_id(&self) -> Result<crate::model_runtime::ModelId, String> {
        self.target_model_id.ok_or("actual configured target load was not verified".into())
    }

    /// Verifies actual configured load and ownership, not default-selection compatibility.
    /// The current LocalRouter still requires a distinct owned full-profile GenieX route.
    pub async fn attach_configured_geniex_target(
        &mut self,
        host_configuration: std::path::PathBuf,
        expected_artifact_sha256: [u8; 32],
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
    ) -> Result<crate::model_runtime::ModelId, String> {
        #[cfg(not(feature = "app-runtime"))]
        {
            let _ = (host_configuration, expected_artifact_sha256, recorder);
            Err("configured GenieX fixture requires the actual app-runtime producer feature".into())
        }
        #[cfg(feature = "app-runtime")]
        {
            let client = self.client()?;
            let authority = client.scoped_model_registry_authority()
                .ok_or("actual current client has no registry authority")?;
            match crate::backend_runtime::load_genuine_configured_geniex_fixture(
                host_configuration, client, authority, &self.runtime, recorder, &self.account, &self.headers,
            ).await {
                Ok(client) => self.client = Some(client),
                Err(failure) => {
                    if let Some(client) = failure.retained_client { self.client = Some(client); }
                    return Err(failure.reason);
                }
            }
            let entry = self.client()?.model_catalog()
                .and_then(|catalog| catalog.entry_for_stable_anchor(&hex::encode(expected_artifact_sha256)))
                .ok_or("actual configured target is absent from current catalog")?;
            if !entry.ready || !entry.default_selectable || entry.runtime_binding != "genie_x"
                || entry.runtime_role != crate::model_runtime::ModelRuntimeRole::Completion
            {
                return Err("actual configured target did not satisfy READY/Completion/GenieX checks".into());
            }
            let id = crate::model_runtime::ModelId::from(uuid::Uuid::parse_str(&entry.model_id).map_err(|error| error.to_string())?);
            if Some(id) == self.current_model_id { return Err("configured target is not a distinct loaded model".into()); }
            self.observe_ownership(id, expected_artifact_sha256, crate::model_runtime::RuntimeBinding::GenieX).await?;
            self.target_model_id = Some(id);
            Ok(id)
        }
    }
    async fn observe<T>(&self, operation: impl std::future::Future<Output = Result<T, String>>) -> Result<T, String> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let admission = crate::api::authority::authorize_bootstrap_request(
            &self.account, &self.headers, "fs.read", ResourceKind::Workspace,
            self.scope.workspace_id.as_str(), ResourceAction::Read,
        ).await.map_err(|_| "original issued registry Read denied".to_owned())?;
        if admission.account_id != self.scope.owner_account_id.as_uuid().to_string()
            || admission.principal_id != self.scope.actor_principal_id.as_uuid().to_string()
            || admission.session_id != self.scope.authenticated_session_id.as_uuid().to_string()
            || admission.access_space_id != self.scope.access_space_id.as_uuid().to_string()
        { return Err("original issued registry Read attribution mismatch".into()); }
        self.account.surreal.with_record_user_scope(admission.record_user_scope, operation).await
    }

    pub async fn observe_ownership(
        &self, id: crate::model_runtime::ModelId, sha256: [u8; 32], binding: crate::model_runtime::RuntimeBinding,
    ) -> Result<crate::storage::surreal::ProcessOwnershipInspection, String> {
        self.observe(crate::llm::embedded_ledger::verify_genuine_fixture_ownership(
            &self.account.surreal, &self.scope, id, sha256, binding,
            &self.runtime.runtime_instance().process_runtime_owner(),
        )).await
    }

    pub async fn snapshot_default(&self) -> Result<GenuineDefaultSnapshot, String> {
        let client = self.client()?;
        let authority = client.scoped_model_registry_authority().ok_or("actual registry authority unavailable")?;
        let session = crate::api::authority::authenticated_session_credentials_bootstrap(&self.account, &self.headers)
            .await.map_err(|_| "original issued session unavailable".to_owned())?;
        self.observe(async {
            let selection = authority.store().list_active_selections(authority.scope()).await
                .map_err(|error| error.to_string())?.into_iter()
                .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
                .ok_or("actual application/default is absent")?;
            let canonical_events = self.account.surreal.list_account_kernel_events_for_aggregate(
                &session.session_token, &session.channel_binding_hash,
                "model_runtime_active_selection", ModelRuntimeSelectionPurpose::ApplicationDefault.as_str(),
            ).await.map_err(|error| error.to_string())?;
            Ok(GenuineDefaultSnapshot { runtime_model_id: client.selected_model_id(), selection, canonical_events })
        }).await
    }

    pub async fn arm_selection_audit_failure(
        &self, request_id: uuid::Uuid, expected_revision: u64, target_sha256: [u8; 32], reason: &str,
    ) -> Result<(), String> {
        if request_id.is_nil() { return Err("selection request UUID is nil".into()); }
        let client = self.client()?;
        let authority = client.scoped_model_registry_authority().ok_or("actual registry authority unavailable")?;
        let target = client.model_catalog().and_then(|catalog| catalog.entry_for_stable_anchor(&hex::encode(target_sha256)))
            .ok_or("actual selection target is absent")?;
        if !target.ready || target.runtime_role != crate::model_runtime::ModelRuntimeRole::Completion {
            return Err("actual selection target is not READY Completion".into());
        }
        let admission = crate::api::authority::authorize_bootstrap_request(
            &self.account, &self.headers, "fs.write", crate::storage::surreal::resource_authority::ResourceKind::Workspace,
            self.scope.workspace_id.as_str(), crate::storage::surreal::resource_authority::ResourceAction::Update,
        ).await.map_err(|_| "original issued selection Update denied".to_owned())?;
        if admission.actor_kind != "operator" || authority.scope() != &self.scope
            || admission.account_id != self.scope.owner_account_id.as_uuid().to_string()
            || admission.principal_id != self.scope.actor_principal_id.as_uuid().to_string()
            || admission.session_id != self.scope.authenticated_session_id.as_uuid().to_string()
            || admission.access_space_id != self.scope.access_space_id.as_uuid().to_string()
        {
            return Err("actual operator selection authority mismatch".into());
        }
        fail_next_selection_audit_append(authority.store(), &self.scope,
            ModelRuntimeSelectionPurpose::ApplicationDefault, target_sha256, expected_revision,
            &KernelActor::Operator(admission.actor_id),
            &format!("selection_request_id={request_id}; {reason}"),
        ).map_err(|error| error.to_string())
    }

    pub async fn shutdown(&self, timeout: std::time::Duration) -> Result<(), String> {
        if let Some(client) = &self.client {
            client.shutdown_gracefully().await.map_err(|error| error.to_string())?;
        }
        let drained = self.runtime.shutdown_and_drain(timeout).await;
        if !drained.reclaim_task_quiesced || !drained.lease_released
            || !matches!(drained.ledger, crate::process_ledger::LedgerDrainJoinOutcome::Flushed
                | crate::process_ledger::LedgerDrainJoinOutcome::AlreadyDrained)
        {
            return Err(format!("genuine model fixture settlement remains unproven: {drained:?}"));
        }
        Ok(())
    }
}
/// Read boundary for selecting the owned snapshot and consuming its finite lifecycle
/// counterfactual. It grants no lifecycle write or canonical row/receipt mutation.
pub async fn with_issued_registry_lifecycle_counterfactual<T>(
    state: &crate::AccountBootstrapState,
    headers: &axum::http::HeaderMap,
    expected: &ExactResourceScopeAttribution,
    operation: impl std::future::Future<Output = T>,
) -> Result<T, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let admission = crate::api::authority::authorize_bootstrap_request(
        state, headers, "fs.read", ResourceKind::Workspace,
        expected.workspace_id.as_str(), ResourceAction::Read,
    ).await?;
    if admission.account_id != expected.owner_account_id.as_uuid().to_string()
        || admission.principal_id != expected.actor_principal_id.as_uuid().to_string()
        || admission.session_id != expected.authenticated_session_id.as_uuid().to_string()
        || admission.access_space_id != expected.access_space_id.as_uuid().to_string()
    { return Err(crate::api::authority::constant_denial()); }
    Ok(state.surreal.with_record_user_scope(admission.record_user_scope, operation).await)
}
impl GenuineModelSelectionFixture {
    pub fn current_selection_revision(&self) -> Result<u64, String> {
        self.selection_owner.as_ref().ok_or("actual concrete selection owner is absent")?
            .genuine_fixture_selection_revision()
    }

    pub async fn project_current_catalog_role(
        &self,
    ) -> Result<crate::model_runtime::CatalogRoleProjectionGuard, String> {
        self.project_current_catalog_role_at_revision(self.current_selection_revision()?).await
    }

    /// Expected revision is a real prior observation. A queued production swap makes it stale.
    pub async fn project_current_catalog_role_at_revision(
        &self, expected_selection_revision: u64,
    ) -> Result<crate::model_runtime::CatalogRoleProjectionGuard, String> {
        use crate::llm::LlmClient;
        let owner = self.selection_owner.as_ref().ok_or("actual concrete selection owner is absent")?;
        let _serial = owner.lock_genuine_fixture_selection().await;
        let client = self.client()?;
        let id = self.current_model_id()?;
        let catalog = client.model_catalog().ok_or("actual selected catalog is absent")?;
        let owner_catalog = owner.model_catalog().ok_or("actual inner catalog is absent")?;
        let runtime_owner = self.runtime.runtime_instance().process_runtime_owner();
        let valid = || -> Result<(), String> {
            if client.selected_model_id() != id.to_string() || owner.selected_model_id() != id.to_string()
                || !std::sync::Arc::ptr_eq(&catalog, &owner_catalog)
                || owner.genuine_fixture_runtime_owner().as_ref() != Some(&runtime_owner)
                || owner.genuine_fixture_selection_revision()? != expected_selection_revision {
                return Err("actual selected model/catalog/owner/revision changed".into());
            }
            Ok(())
        };
        valid()?;
        let entry = catalog.entry(&id.to_string()).ok_or("actual selected catalog entry is absent")?;
        let hash: [u8; 32] = hex::decode(&entry.artifact_sha256).map_err(|error| error.to_string())?
            .try_into().map_err(|_| "actual selected artifact hash length is invalid")?;
        if entry.runtime_binding != "candle" { return Err("current genuine fixture requires its configured Candle binding".into()); }
        self.observe_ownership(id, hash, crate::model_runtime::RuntimeBinding::Candle).await?;
        // Both checks and the awaited canonical ownership observation hold the production serial.
        valid()?;
        catalog.project_selected_completion_as_embedding(id, runtime_owner, expected_selection_revision)
    }

    pub async fn restore_current_catalog_role(
        &self, guard: &mut crate::model_runtime::CatalogRoleProjectionGuard,
    ) -> Result<(), String> {
        use crate::llm::LlmClient;
        let owner = self.selection_owner.as_ref().ok_or("actual concrete selection owner is absent")?;
        let _serial = owner.lock_genuine_fixture_selection().await;
        let client = self.client()?;
        let id = self.current_model_id()?;
        let catalog = client.model_catalog().ok_or("actual selected catalog is absent")?;
        let owner_catalog = owner.model_catalog().ok_or("actual inner catalog is absent")?;
        let runtime_owner = self.runtime.runtime_instance().process_runtime_owner();
        let revision = owner.genuine_fixture_selection_revision()?;
        let valid = || -> Result<(), String> {
            if client.selected_model_id() != id.to_string() || owner.selected_model_id() != id.to_string()
                || !std::sync::Arc::ptr_eq(&catalog, &owner_catalog)
                || owner.genuine_fixture_runtime_owner().as_ref() != Some(&runtime_owner)
                || owner.genuine_fixture_selection_revision()? != revision {
                return Err("actual selected model/catalog/owner/revision changed".into());
            }
            Ok(())
        };
        valid()?;
        let entry = catalog.entry(&id.to_string()).ok_or("actual selected model is absent")?;
        let hash: [u8; 32] = hex::decode(&entry.artifact_sha256).map_err(|error| error.to_string())?
            .try_into().map_err(|_| "actual selected artifact hash length is invalid")?;
        self.observe_ownership(id, hash, crate::model_runtime::RuntimeBinding::Candle).await?;
        valid()?;
        guard.restore(&catalog, &runtime_owner, id, revision)
    }

    /// A successful explicit restore already supplies its proof. Drop alone never qualifies.
    pub async fn settle_current_catalog_role(
        &self, guard: &mut crate::model_runtime::CatalogRoleProjectionGuard,
    ) -> Result<(), String> {
        if guard.was_restored() { return Ok(()); }
        self.restore_current_catalog_role(guard).await
    }
}
/// Proposed nonsecret runtime input contract. Source authors do not bind its values.
/// Root/NONAUTHOR must independently pin the supplied document and provenance before execution.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenuineCompatibleCompletionInput {
    pub artifact_path: std::path::PathBuf,
    pub artifact_sha256: String,
    pub runtime_binding: crate::model_runtime::RuntimeBinding,
    pub display_name: String,
    pub artifact_source_ref: String,
    pub host_executable_sha256: String,
    pub source_review_ref: String,
}

impl GenuineCompatibleCompletionInput {
    pub fn read_explicit_input() -> Result<Self, String> {
        let path = std::env::var_os("HANDSHAKE_TEST_COMPATIBLE_COMPLETION_INPUT")
            .ok_or("UNBOUND compatible Completion input: HANDSHAKE_TEST_COMPATIBLE_COMPLETION_INPUT is required")?;
        let path = std::fs::canonicalize(std::path::PathBuf::from(path)).map_err(|error| error.to_string())?;
        let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
        let mut input: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if input.artifact_path.is_relative() {
            input.artifact_path = path.parent().ok_or("explicit input file has no parent")?.join(&input.artifact_path);
        }
        Ok(input)
    }

    fn resolve(&self, current: &crate::llm::registry::LocalModelConfig) -> Result<crate::llm::registry::LocalModelConfig, String> {
        use sha2::Digest;
        let sha256: [u8; 32] = hex::decode(&self.artifact_sha256).map_err(|error| error.to_string())?
            .try_into().map_err(|_| "compatible target artifact SHA-256 must have 32 bytes")?;
        if sha256 == [0; 32] || sha256 == current.sha256 || self.artifact_path.as_os_str().is_empty()
            || self.display_name.trim().is_empty() || self.artifact_source_ref.trim().is_empty()
            || self.source_review_ref.trim().is_empty()
            || self.runtime_binding == current.runtime_binding
            || !matches!(self.runtime_binding, crate::model_runtime::RuntimeBinding::Candle | crate::model_runtime::RuntimeBinding::LlamaCpp) {
            return Err("explicit distinct compatible Completion config/provenance is invalid".into());
        }
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let executable_bytes = std::fs::read(executable).map_err(|error| error.to_string())?;
        if hex::encode(sha2::Sha256::digest(&executable_bytes)) != self.host_executable_sha256 {
            return Err("actual fixture host executable differs from supplied provenance pin".into());
        }
        // Text provenance is context only; actual runtime ownership comes from canonical START ACK.
        Ok(crate::llm::registry::LocalModelConfig {
            artifact_path: self.artifact_path.clone(), sha256, runtime_binding: self.runtime_binding,
            display_name: self.display_name.clone(), embedding_dimension: None,
        })
    }
}