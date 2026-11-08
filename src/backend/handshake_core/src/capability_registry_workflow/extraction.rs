use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{
    registry_external_path, sha256_hex, validate_integrity, validate_schema,
    CapabilityRegistryDocument, CapabilityRegistryEntry, CapabilityRegistryWorkflowError as Error,
    CapabilityRegistryWorkflowResult as Result,
};
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::llm::{
    BoundLlmModel, CompletionRequest, CompletionResponse, InvocationDrainOutcome, LlmError,
    LlmInvocationControl, ModelTier,
};
use crate::storage::artifacts::{
    ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::surreal::resource_authority::{
    AuthorizationDecision, AuthorizationRequest, RegistryExtractionArtifact, ResourceAction,
    ResourceKind, RoleAuthorityAuthInput, WorkspaceArtifactPublicationPermit,
};
use crate::AppState;

const POINTER: &str = ".GOV/spec/SPEC_CURRENT.md";
const BASELINE: &str = "assets/capability_registry.json";
const ENGINES: &str = "src/backend/handshake_core/mechanical_engines.json";
const SCHEMA: &str = "src/backend/handshake_core/schemas/capability_registry.schema.json";
const INSTRUCTION: &str = "Extract capability entries evidenced by this exact source chunk. Return JSON only, with entries (the supplied registry entry schema) and evidence (objects with capability_id and quote). Every new entry needs an exact nonempty quotation from this chunk. Preserve existing entries verbatim; never infer grants, profile membership, issuer authority, review or publication. Return empty arrays when the chunk adds no entry. Conflicting requirements must not be guessed. The complete input set is processed separately; this response is only a proposal from the identified byte range.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    sources: BTreeMap<String, SourceArtifact>,
    max_prompt_bytes: usize,
    max_output_tokens: u32,
    invocation_timeout_ms: u64,
    output_retention_ttl_days: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceArtifact {
    artifact_id: Uuid,
    sha256: String,
}

#[derive(Default)]
pub(crate) struct InvocationRegistry {
    retained: Mutex<BTreeMap<Uuid, Arc<Invocation>>>,
}

impl std::fmt::Debug for InvocationRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.retained.lock() {
            Ok(retained) => formatter
                .debug_struct("RegistryExtractionInvocations")
                .field("retained_jobs", &retained.keys().collect::<Vec<_>>())
                .finish(),
            Err(_) => formatter.write_str("RegistryExtractionInvocations { custody: unknown }"),
        }
    }
}

struct Invocation {
    job_id: Uuid,
    control: LlmInvocationControl,
    observation: Mutex<InvocationObservation>,
}

struct InvocationObservation {
    provenance: Value,
    completion: Option<Arc<std::result::Result<CompletionResponse, LlmError>>>,
    drain: Option<InvocationDrainOutcome>,
    scope_ended_without_release: bool,
}

struct InvocationLease {
    registry: Arc<InvocationRegistry>,
    invocation: Arc<Invocation>,
    released: bool,
}

impl InvocationRegistry {
    fn retain(
        self: &Arc<Self>,
        job_id: Uuid,
        control: LlmInvocationControl,
        provenance: Value,
    ) -> Result<InvocationLease> {
        let mut retained = self
            .retained
            .lock()
            .map_err(|_| rejected("extraction_invocation_owner_unknown"))?;
        if retained.contains_key(&job_id) {
            return Err(rejected("extraction_prior_invocation_unsettled"));
        }
        let invocation = Arc::new(Invocation {
            job_id,
            control,
            observation: Mutex::new(InvocationObservation {
                provenance,
                completion: None,
                drain: None,
                scope_ended_without_release: false,
            }),
        });
        retained.insert(job_id, invocation.clone());
        Ok(InvocationLease {
            registry: self.clone(),
            invocation,
            released: false,
        })
    }
}

impl InvocationLease {
    fn release(mut self) -> Result<()> {
        let observed = self
            .invocation
            .observation
            .lock()
            .map_err(|_| rejected("extraction_invocation_owner_unknown"))?;
        if !matches!(
            observed.drain,
            Some(InvocationDrainOutcome::NeverSubmitted)
                | Some(InvocationDrainOutcome::LocalResourcesSettled { .. })
        ) {
            return Err(rejected("extraction_invocation_unsettled"));
        }
        let mut retained = self
            .registry
            .retained
            .lock()
            .map_err(|_| rejected("extraction_invocation_owner_unknown"))?;
        if !retained
            .get(&self.invocation.job_id)
            .is_some_and(|current| Arc::ptr_eq(current, &self.invocation))
        {
            return Err(rejected("extraction_invocation_owner_changed"));
        }
        retained.remove(&self.invocation.job_id);
        self.released = true;
        Ok(())
    }
}

impl Drop for InvocationLease {
    fn drop(&mut self) {
        if !self.released {
            self.invocation.control.cancel();
            let mut observed = self
                .invocation
                .observation
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            observed.scope_ended_without_release = true;
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChunkProposal {
    entries: Vec<CapabilityRegistryEntry>,
    evidence: Vec<EntryEvidence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryEvidence {
    capability_id: String,
    quote: String,
}

struct Inputs {
    configuration_artifact: RegistryExtractionArtifact,
    configuration_hash: String,
    configuration: Configuration,
    bytes: BTreeMap<String, String>,
    source_artifacts: Vec<RegistryExtractionArtifact>,
}

fn rejected(category: &'static str) -> Error {
    Error::ApprovedInputMismatch { category }
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|source| Error::ReadFile {
        path: path.to_owned(),
        source,
    })
}

fn parse<T: serde::de::DeserializeOwned>(path: &Path, bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|source| Error::JsonParse {
        path: path.to_owned(),
        source,
    })
}

fn encode(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|source| Error::JsonSerialize { source })
}

async fn artifact_operation<T, F>(state: &AppState, workspace: &str, operation: F) -> Result<T>
where
    T: Send + 'static,
    F: for<'tx> FnOnce(
            WorkspaceArtifactPublicationPermit<'tx>,
        )
            -> crate::storage::surreal::SurrealTransactionOperation<'tx, T, Error>
        + Send
        + 'static,
{
    let scope = crate::storage::surreal::current_record_user_scope()
        .ok_or_else(|| rejected("extraction_authenticated_scope_required"))?;
    if scope.workspace_id.as_deref() != Some(workspace) {
        return Err(rejected("extraction_workspace_authority"));
    }
    let credentials = RoleAuthorityAuthInput::new(
        scope.session_token,
        scope
            .channel_binding_hash
            .ok_or_else(|| rejected("extraction_channel_binding_required"))?,
    );
    let authority = state
        .surreal
        .capture_workspace_artifact_publication_authorization(
            Uuid::now_v7(),
            &state.capability_registry,
            credentials,
            workspace,
        )
        .await?;
    state
        .surreal
        .with_workspace_artifact_publication(&authority, operation)
        .await
}

impl Inputs {
    async fn load(state: &AppState, repo_root: &Path, workspace: &str) -> Result<Self> {
        let selected = std::env::var("HANDSHAKE_CAPABILITY_REGISTRY_EXTRACTION_CONFIG_ARTIFACT_ID")
            .map_err(|_| rejected("extraction_server_configuration_required"))?;
        let selected = Uuid::parse_str(&selected)
            .map_err(|_| rejected("extraction_configuration_artifact_id_invalid"))?;
        let storage = state.surreal.clone();
        let (configuration_artifact, configuration, bytes, source_artifacts) =
            artifact_operation(state, workspace, move |permit| {
                Box::pin(async move {
                    let config = storage
                        .read_registry_extraction_artifact(&permit, selected)
                        .await?;
                    let configuration: Configuration = parse(
                        Path::new("registry-extraction-configuration"),
                        config.payload(),
                    )?;
                    if configuration.output_retention_ttl_days == 0
                        || configuration.invocation_timeout_ms == 0
                    {
                        return Err(rejected("extraction_configuration_limits_required"));
                    }
                    let mut bytes = BTreeMap::new();
                    let mut artifacts = Vec::new();
                    for (identity, source) in &configuration.sources {
                        if identity.is_empty()
                            || source.sha256.len() != 64
                            || !source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                        {
                            return Err(rejected("extraction_source_binding"));
                        }
                        let artifact = storage
                            .read_registry_extraction_artifact(&permit, source.artifact_id)
                            .await?;
                        if artifact.manifest().content_hash != source.sha256 {
                            return Err(rejected("extraction_source_drift"));
                        }
                        let text = std::str::from_utf8(artifact.payload())
                            .map_err(|_| rejected("extraction_source_not_utf8"))?
                            .to_owned();
                        bytes.insert(identity.clone(), text);
                        artifacts.push(artifact);
                    }
                    Ok((config, configuration, bytes, artifacts))
                })
            })
            .await?;
        for identity in [BASELINE, ENGINES, SCHEMA] {
            let source = configuration
                .sources
                .get(identity)
                .ok_or_else(|| rejected("extraction_dependency_missing"))?;
            if sha256_hex(&read(&registry_external_path(
                repo_root,
                Path::new(identity),
            )?)?)
                != source.sha256
            {
                return Err(rejected("extraction_product_dependency_drift"));
            }
        }
        let source_json = |identity: &str| -> Result<Value> {
            let raw = bytes
                .get(identity)
                .ok_or_else(|| rejected("extraction_spec_source_missing"))?;
            parse(Path::new(identity), raw.as_bytes())
        };
        let pointer = source_json(POINTER)?;
        let manifest_id = pointer["current_spec"]["entrypoint_path"]
            .as_str()
            .ok_or_else(|| rejected("extraction_spec_pointer"))?;
        let resolver_id = pointer["current_spec"]["resolver_index_path"]
            .as_str()
            .ok_or_else(|| rejected("extraction_spec_resolver"))?;
        source_json(resolver_id)?;
        let manifest = source_json(manifest_id)?;
        let manifest_parent = manifest_id
            .rsplit_once('/')
            .ok_or_else(|| rejected("extraction_manifest_parent"))?
            .0;
        let modules = manifest["modules"]
            .as_array()
            .filter(|modules| !modules.is_empty())
            .ok_or_else(|| rejected("extraction_spec_modules"))?;
        let mut covered = std::collections::BTreeSet::new();
        for module in modules {
            let relative = module["path"]
                .as_str()
                .ok_or_else(|| rejected("extraction_module_path"))?;
            let identity = format!("{manifest_parent}/{relative}");
            let text = bytes
                .get(&identity)
                .ok_or_else(|| rejected("extraction_complete_spec_required"))?;
            if !covered.insert(identity)
                || Some(sha256_hex(text.as_bytes()).as_str()) != module["sha256"].as_str()
            {
                return Err(rejected("extraction_manifest_module_binding"));
            }
        }
        Ok(Self {
            configuration_hash: configuration_artifact.manifest().content_hash.clone(),
            configuration_artifact,
            configuration,
            bytes,
            source_artifacts,
        })
    }

    async fn recheck(&self, state: &AppState, repo_root: &Path, workspace: &str) -> Result<()> {
        let selected = std::env::var("HANDSHAKE_CAPABILITY_REGISTRY_EXTRACTION_CONFIG_ARTIFACT_ID")
            .ok()
            .and_then(|value| Uuid::parse_str(&value).ok());
        if selected != Some(self.configuration_artifact.manifest().artifact_id) {
            return Err(rejected("extraction_configuration_selection_changed"));
        }
        let mut artifacts = self.source_artifacts.clone();
        artifacts.push(self.configuration_artifact.clone());
        let storage = state.surreal.clone();
        artifact_operation(state, workspace, move |permit| {
            Box::pin(async move {
                for artifact in &artifacts {
                    storage
                        .revalidate_registry_extraction_artifact(&permit, artifact)
                        .await?;
                }
                Ok(())
            })
        })
        .await?;
        for identity in [BASELINE, ENGINES, SCHEMA] {
            if sha256_hex(&read(&registry_external_path(
                repo_root,
                Path::new(identity),
            )?)?)
                != self.configuration.sources[identity].sha256
            {
                return Err(rejected("extraction_product_dependency_drift"));
            }
        }
        Ok(())
    }
}

async fn authorize(state: &AppState, workspace: &str) -> Result<AuthorizationDecision> {
    let scope = crate::storage::surreal::current_record_user_scope()
        .ok_or_else(|| rejected("extraction_authenticated_scope_required"))?;
    if scope.workspace_id.as_deref() != Some(workspace)
        || scope.capability_id != "fs.write"
        || !matches!(
            scope.action,
            ResourceAction::Create | ResourceAction::Update
        )
    {
        return Err(rejected("extraction_workspace_authority"));
    }
    let write = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: scope.session_token.clone(),
            channel_binding_hash: scope.channel_binding_hash.clone(),
            capability_id: scope.capability_id,
            resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace.to_owned(),
            action: scope.action,
        })
        .await?;
    if write.capability_profile_id != "Operator"
        || write.session_id != scope.session_id
        || write.resource_id != scope.resource_id
        || Some(write.grant_id.as_str()) != scope.grant_id.as_deref()
    {
        return Err(rejected("extraction_current_writer_required"));
    }
    let read = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: scope.session_token,
            channel_binding_hash: scope.channel_binding_hash,
            capability_id: "fs.read".into(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace.to_owned(),
            action: ResourceAction::Read,
        })
        .await?;
    if read.account_id != write.account_id
        || read.principal_id != write.principal_id
        || read.session_id != write.session_id
        || read.access_space_id != write.access_space_id
        || read.resource_id != write.resource_id
        || read.capability_profile_id != "Operator"
    {
        return Err(rejected("extraction_current_reader_required"));
    }
    for capability in state
        .capability_registry
        .required_capabilities_for_job_request("workflow_run", "capability_registry_build")
        .map_err(|_| rejected("extraction_capability_profile"))?
    {
        if !state
            .capability_registry
            .profile_can(&write.capability_profile_id, &capability)
            .map_err(|_| rejected("extraction_capability_profile"))?
        {
            return Err(rejected("extraction_capability_profile"));
        }
    }
    Ok(write)
}

async fn write_candidates(
    state: &AppState,
    workspace: &str,
    job_id: Uuid,
    inputs: &Inputs,
    outputs: Vec<(String, Vec<u8>)>,
) -> Result<BTreeMap<String, (ArtifactManifest, PathBuf)>> {
    let mut sources = inputs.source_artifacts.clone();
    sources.push(inputs.configuration_artifact.clone());
    let retention = inputs.configuration.output_retention_ttl_days;
    let storage = state.surreal.clone();
    artifact_operation(state, workspace, move |permit| {
        Box::pin(async move {
            for source in &sources {
                storage
                    .revalidate_registry_extraction_artifact(&permit, source)
                    .await?;
            }
            let mut result = BTreeMap::new();
            for (name, payload) in outputs {
                let artifact_id = Uuid::now_v7();
                let manifest = ArtifactManifest {
                    artifact_id,
                    layer: ArtifactLayer::L3,
                    kind: ArtifactPayloadKind::Report,
                    mime: "application/json".into(),
                    filename_hint: Some(name.clone()),
                    created_at: chrono::Utc::now(),
                    created_by_job_id: Some(job_id),
                    source_entity_refs: Vec::new(),
                    source_artifact_refs: sources
                        .iter()
                        .map(|source| {
                            let manifest = source.manifest();
                            crate::ace::ArtifactHandle::new(
                                manifest.artifact_id,
                                format!(
                                    "{}/payload",
                                    crate::storage::artifacts::artifact_root_rel(
                                        manifest.layer,
                                        manifest.artifact_id
                                    )
                                ),
                            )
                        })
                        .collect(),
                    content_hash: sha256_hex(&payload),
                    size_bytes: payload.len() as u64,
                    classification: ArtifactClassification::High,
                    exportable: false,
                    retention_ttl_days: Some(retention),
                    pinned: None,
                    hash_basis: None,
                    hash_exclude_paths: Vec::new(),
                };
                let persisted = permit
                    .persist_file_artifact_owned(&manifest, &payload)
                    .map_err(|error| Error::Usage {
                        usage: format!("registry candidate artifact write failed: {error}"),
                    })?;
                permit.register_persisted_artifact(&persisted).await?;
                let path = crate::storage::artifacts::artifact_root_dir(
                    permit.managed_root(),
                    manifest.layer,
                    artifact_id,
                )
                .join("payload");
                result.insert(name, (manifest, path));
            }
            for source in &sources {
                storage
                    .revalidate_registry_extraction_artifact(&permit, source)
                    .await?;
            }
            Ok(result)
        })
    })
    .await
}

async fn record(
    state: &AppState,
    trace_id: Uuid,
    job_id: Uuid,
    workflow_id: Uuid,
    workspace: &str,
    authority: &AuthorizationDecision,
    model_id: &str,
    mut payload: Value,
) -> Result<()> {
    payload["owner_account_id"] = json!(authority.account_id);
    payload["actor_principal_id"] = json!(authority.principal_id);
    payload["authenticated_session_id"] = json!(authority.session_id);
    payload["access_space_id"] = json!(authority.access_space_id);
    payload["workspace_id"] = json!(workspace);
    payload["policy_decision"] = json!(authority);
    state
        .flight_recorder
        .record_event(
            FlightRecorderEvent::new(
                FlightRecorderEventType::System,
                FlightRecorderActor::System,
                trace_id,
                payload,
            )
            .with_actor_id(super::REGISTRY_WORKFLOW_ACTOR_ID)
            .with_wsids(vec![workspace.to_owned()])
            .with_job_id(job_id.to_string())
            .with_workflow_id(workflow_id.to_string())
            .with_model_id(model_id)
            .with_policy_decision(authority.decision_id.clone()),
        )
        .await
        .map_err(|error| Error::FlightRecorderRecordEvent {
            message: error.to_string(),
        })
}

fn invocation_deadline(timeout_ms: u64) -> Result<tokio::time::Instant> {
    std::time::Instant::now()
        .checked_add(Duration::from_millis(timeout_ms))
        .filter(|_| timeout_ms > 0)
        .map(tokio::time::Instant::from_std)
        .ok_or_else(|| rejected("extraction_invocation_deadline_invalid"))
}

async fn complete_owned(
    state: &AppState,
    request: CompletionRequest,
    binding: &BoundLlmModel,
    timeout_ms: u64,
    job_id: Uuid,
    workflow_id: Uuid,
    workspace: &str,
    authority: &AuthorizationDecision,
    provenance: Value,
) -> Result<CompletionResponse> {
    let deadline = invocation_deadline(timeout_ms)?;
    let trace_id = request.trace_id;
    let model_id = request.model_id.clone();
    let (future, control) = state
        .llm_client
        .clone()
        .owned_completion_with_binding(request, binding.clone(), deadline)
        .into_parts();
    let lease =
        state
            .session_registry
            .registry_extractions
            .retain(job_id, control.clone(), provenance)?;
    let completion = Arc::new(future.await);
    lease
        .invocation
        .observation
        .lock()
        .map_err(|_| rejected("extraction_invocation_owner_unknown"))?
        .completion = Some(completion.clone());
    let drain = control.drain().await;
    let mut receipt = {
        let mut observed = lease
            .invocation
            .observation
            .lock()
            .map_err(|_| rejected("extraction_invocation_owner_unknown"))?;
        observed.drain = Some(drain.clone());
        observed.provenance.clone()
    };
    receipt["event"] = json!("capability_registry_extract_invocation_observed");
    receipt["timeout_ms"] = json!(timeout_ms);
    receipt["drain"] = json!(format!("{drain:?}"));
    receipt["resource_settlements"] = json!(control
        .resource_settlements()
        .into_iter()
        .map(|(id, settled)| json!({"activity_id": id.get(), "settled": settled.is_some()}))
        .collect::<Vec<_>>());
    receipt["completion"] = match completion.as_ref() {
        Ok(response) => json!({"response_sha256": sha256_hex(response.text.as_bytes()),
            "usage": response.usage, "latency_ms": response.latency_ms}),
        Err(error) => json!({"error": error.to_string()}),
    };
    record(
        state,
        trace_id,
        job_id,
        workflow_id,
        workspace,
        authority,
        &model_id,
        receipt,
    )
    .await?;
    if matches!(drain, InvocationDrainOutcome::Unknown { .. }) {
        return Err(rejected("extraction_invocation_unsettled"));
    }
    let late = matches!(
        drain,
        InvocationDrainOutcome::LocalResourcesSettled {
            deadline_exceeded: true,
            ..
        }
    );
    lease.release()?;
    if late {
        return Err(rejected("extraction_invocation_settled_after_deadline"));
    }
    state
        .llm_client
        .revalidate_model_binding(binding)
        .map_err(|_| rejected("extraction_model_binding_changed"))?;
    match completion.as_ref() {
        Ok(response) => Ok(response.clone()),
        Err(error) => Err(Error::Usage {
            usage: format!("registry extraction model call failed: {error}"),
        }),
    }
}

pub(crate) async fn prepare(
    state: &AppState,
    repo_root: &Path,
    job_id: Uuid,
    workflow_id: Uuid,
    trace_id: Uuid,
    workspace: &str,
) -> Result<Value> {
    let authority = authorize(state, workspace).await?;
    let inputs = Inputs::load(state, repo_root, workspace).await?;
    let binding = state
        .llm_client
        .bind_selected_model(invocation_deadline(
            inputs.configuration.invocation_timeout_ms,
        )?)
        .await
        .map_err(|error| Error::Usage {
            usage: format!("registry extraction model binding unavailable: {error}"),
        })?;
    let model_id = binding.profile().model_id.clone();
    if model_id.trim().is_empty() {
        return Err(rejected("extraction_configured_model_required"));
    }
    if binding.profile().model_tier == ModelTier::Cloud {
        return Err(rejected(
            "extraction_cloud_classification_projection_and_consent_required",
        ));
    }
    let context = binding.profile().max_context_tokens as usize;
    if inputs.configuration.max_output_tokens == 0
        || inputs.configuration.max_prompt_bytes == 0
        || inputs
            .configuration
            .max_prompt_bytes
            .checked_add(inputs.configuration.max_output_tokens as usize)
            .map_or(true, |budget| budget > context)
    {
        return Err(rejected("extraction_context_budget"));
    }
    let schema: Value = parse(Path::new(SCHEMA), inputs.bytes[SCHEMA].as_bytes())?;
    let baseline: CapabilityRegistryDocument =
        parse(Path::new(BASELINE), inputs.bytes[BASELINE].as_bytes())?;
    validate_schema(
        &schema,
        &serde_json::to_value(&baseline).map_err(|source| Error::JsonSerialize { source })?,
    )?;
    validate_integrity(&baseline)?;
    let mut entries: BTreeMap<_, _> = baseline
        .entries
        .iter()
        .map(|entry| (entry.capability_id.clone(), entry.clone()))
        .collect();
    let dependencies: BTreeMap<_, _> = inputs
        .configuration
        .sources
        .iter()
        .map(|(identity, source)| (identity.clone(), source.sha256.clone()))
        .collect();
    let common = format!(
        "{INSTRUCTION}\nSchema:\n{}\nExisting registry:\n{}\nMechanical engines:\n{}\n",
        inputs.bytes[SCHEMA], inputs.bytes[BASELINE], inputs.bytes[ENGINES]
    );
    if common.len() >= inputs.configuration.max_prompt_bytes {
        return Err(rejected(
            "extraction_model_context_too_small_for_complete_dependencies",
        ));
    }
    let mut receipts = Vec::new();
    for (identity, text) in &inputs.bytes {
        let mut offset = 0;
        while offset < text.len() {
            let current = authorize(state, workspace).await?;
            if current.account_id != authority.account_id
                || current.principal_id != authority.principal_id
                || current.session_id != authority.session_id
                || current.access_space_id != authority.access_space_id
                || current.grant_id != authority.grant_id
                || current.resource_id != authority.resource_id
            {
                return Err(rejected("extraction_authority_changed"));
            }
            inputs.recheck(state, repo_root, workspace).await?;
            state
                .llm_client
                .revalidate_model_binding(&binding)
                .map_err(|_| rejected("extraction_model_binding_changed"))?;
            let header = format!(
                "{common}Source identity: {}\nSource SHA256: {}\nStart byte: {offset}\nChunk:\n",
                serde_json::to_string(identity)
                    .map_err(|source| Error::JsonSerialize { source })?,
                dependencies[identity]
            );
            let available = inputs
                .configuration
                .max_prompt_bytes
                .checked_sub(header.len())
                .filter(|size| *size > 0)
                .ok_or_else(|| rejected("extraction_chunk_budget"))?;
            let mut end = offset.saturating_add(available).min(text.len());
            while end > offset && !text.is_char_boundary(end) {
                end -= 1;
            }
            if end == offset {
                return Err(rejected("extraction_chunk_utf8_budget"));
            }
            let chunk = &text[offset..end];
            let prompt = format!("{header}{chunk}");
            let prompt_hash = sha256_hex(prompt.as_bytes());
            let sequence = receipts.len();
            let mut receipt = json!({
                "event": "capability_registry_extract_started", "sequence": sequence,
                "model_id": model_id, "prompt_sha256": prompt_hash,
                "model_binding": binding.provenance(),
                "job_id": job_id, "workflow_id": workflow_id, "trace_id": trace_id,
                "source_identity": identity, "source_sha256": dependencies[identity],
                "start_byte": offset, "end_byte": end, "input_hashes": dependencies,
                "configuration_sha256": inputs.configuration_hash, "published": false,
            });
            record(
                state,
                trace_id,
                job_id,
                workflow_id,
                workspace,
                &current,
                &model_id,
                receipt.clone(),
            )
            .await?;
            let response = complete_owned(
                state,
                CompletionRequest::new(trace_id, prompt, model_id.clone())
                    .with_max_tokens(inputs.configuration.max_output_tokens)
                    .with_temperature(0.0),
                &binding,
                inputs.configuration.invocation_timeout_ms,
                job_id,
                workflow_id,
                workspace,
                &current,
                receipt.clone(),
            )
            .await?;
            let current = authorize(state, workspace).await?;
            inputs.recheck(state, repo_root, workspace).await?;
            let response_name = format!("response-{sequence:06}.json");
            let response_artifacts = write_candidates(
                state,
                workspace,
                job_id,
                &inputs,
                vec![(response_name.clone(), response.text.as_bytes().to_vec())],
            )
            .await?;
            let (response_manifest, response_path) = &response_artifacts[&response_name];
            receipt["event"] = json!("capability_registry_extract_completed");
            receipt["response_sha256"] = json!(sha256_hex(response.text.as_bytes()));
            receipt["response_artifact_id"] = json!(response_manifest.artifact_id);
            receipt["token_usage"] = json!(response.usage);
            receipt["latency_ms"] = json!(response.latency_ms);
            record(
                state,
                trace_id,
                job_id,
                workflow_id,
                workspace,
                &current,
                &model_id,
                receipt.clone(),
            )
            .await?;
            let proposal: ChunkProposal = parse(&response_path, response.text.as_bytes())?;
            let document = CapabilityRegistryDocument {
                entries: proposal.entries,
            };
            validate_schema(
                &schema,
                &serde_json::to_value(&document)
                    .map_err(|source| Error::JsonSerialize { source })?,
            )?;
            validate_integrity(&document)?;
            for entry in document.entries {
                if let Some(previous) = entries.get(&entry.capability_id) {
                    if previous != &entry {
                        return Err(rejected("extraction_conflicting_entry"));
                    }
                    continue;
                }
                let evidence: Vec<_> = proposal
                    .evidence
                    .iter()
                    .filter(|e| e.capability_id == entry.capability_id)
                    .collect();
                if evidence.len() != 1
                    || evidence[0].quote.trim().is_empty()
                    || !chunk.contains(&evidence[0].quote)
                {
                    return Err(rejected("extraction_source_evidence_required"));
                }
                if entry
                    .required_capabilities
                    .iter()
                    .any(|capability| !state.capability_registry.is_valid(capability))
                {
                    return Err(rejected("extraction_unknown_required_capability"));
                }
                entries.insert(entry.capability_id.clone(), entry);
            }
            receipts.push(receipt);
            offset = end;
        }
    }
    let current = authorize(state, workspace).await?;
    inputs.recheck(state, repo_root, workspace).await?;
    let draft = CapabilityRegistryDocument {
        entries: entries.into_values().collect(),
    };
    validate_integrity(&draft)?;
    let draft_bytes = encode(&draft)?;
    let draft_hash = sha256_hex(&draft_bytes);
    let added: Vec<_> = draft
        .entries
        .iter()
        .filter(|entry| !baseline.entries.contains(entry))
        .collect();
    let diff = json!({
        "schema": "wp012.auth_mapping_registry_proposal_diff@1",
        "baseline": { "registry_path": BASELINE, "sha256": dependencies[BASELINE], "entry_count": baseline.entries.len() },
        "draft": { "path": "capability_registry_draft.json", "sha256": draft_hash, "entry_count": draft.entries.len() },
        "mechanical_engines": { "path": ENGINES, "sha256": dependencies[ENGINES] },
        "registry_schema": { "path": SCHEMA, "sha256": dependencies[SCHEMA] },
        "added": added, "removed": [], "changed": [],
    });
    let diff_bytes = encode(&diff)?;
    let provenance = json!({
        "model_id": model_id, "configuration_sha256": inputs.configuration_hash,
        "model_binding": binding.provenance(),
        "input_hashes": dependencies, "chunks": receipts,
        "draft_sha256": draft_hash, "diff_sha256": sha256_hex(&diff_bytes),
    });
    let provenance_bytes = encode(&provenance)?;
    let artifacts = write_candidates(
        state,
        workspace,
        job_id,
        &inputs,
        vec![
            ("capability_registry_draft.json".into(), draft_bytes),
            ("capability_registry_diff.json".into(), diff_bytes.clone()),
            (
                "capability_registry_extraction.json".into(),
                provenance_bytes.clone(),
            ),
        ],
    )
    .await?;
    let (draft_manifest, draft_path) = &artifacts["capability_registry_draft.json"];
    let (diff_manifest, diff_path) = &artifacts["capability_registry_diff.json"];
    let (provenance_manifest, provenance_path) = &artifacts["capability_registry_extraction.json"];
    let outcome = json!({
        "event": "capability_registry_extraction_prepared", "model_id": model_id,
        "draft_path": draft_path, "diff_path": diff_path, "provenance_path": provenance_path,
        "draft_artifact_id": draft_manifest.artifact_id, "diff_artifact_id": diff_manifest.artifact_id,
        "provenance_artifact_id": provenance_manifest.artifact_id,
        "draft_sha256": draft_hash, "diff_sha256": sha256_hex(&diff_bytes),
        "capability_registry_version": draft_hash, "configuration_sha256": inputs.configuration_hash,
        "input_hashes": dependencies, "chunk_count": receipts.len(),
        "provenance_sha256": sha256_hex(&provenance_bytes),
        "human_actual_diff_reviewed": false, "published": false, "publication_receipt_confirmed": false,
    });
    record(
        state,
        trace_id,
        job_id,
        workflow_id,
        workspace,
        &current,
        &model_id,
        outcome.clone(),
    )
    .await?;
    Ok(outcome)
}

#[cfg(test)]
mod tests;
