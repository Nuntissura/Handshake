//! AC9 structural denial checks. Executable-wave acceptance additionally requires
//! canonically resolved promotion and policy-specific authority evidence.

use crate::swarm_orchestration::production_factory::execute_production_routing_wave;
use crate::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterKind, DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest,
    ModelLaneError, ModelLanePromotionOutcome, ModelLaneResult,
};
use crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionContext;
use crate::swarm_orchestration::routing::{
    ModelLaneRoutingDispatchTarget, ModelLaneRoutingGraph, ModelLaneRoutingPolicy,
    ModelLaneRoutingStageLaunchPlan, SwarmRoutingError,
};

use super::ac9_lifecycle_surreal_tests::{artifact_workspace, Ac9Harness};

fn expected_stage_ids(policy: ModelLaneRoutingPolicy) -> &'static [&'static str] {
    match policy {
        ModelLaneRoutingPolicy::LocalFirst => &["local-attempt", "cloud-escalation"],
        ModelLaneRoutingPolicy::CloudReview => &["local-candidate", "cloud-review"],
        ModelLaneRoutingPolicy::CloudPlanLocalExecute => &["cloud-plan", "local-execute"],
        ModelLaneRoutingPolicy::ParallelDebate => &["debate-local", "debate-cloud", "debate-join"],
        ModelLaneRoutingPolicy::ValidatorLane => &["validation-candidate", "validator-verdict"],
        ModelLaneRoutingPolicy::OperatorLane => &["operator-candidate", "operator-decision"],
    }
}

/// Producer-owned inputs retain their real consent, projection, process and memory
/// bindings. These request prototypes confer no routing authority by themselves.
pub(super) struct Ac9StageRequestInputs {
    pub(super) local: DexterityLaunchAdapterRequest,
    pub(super) cloud: Option<DexterityLaunchAdapterRequest>,
    pub(super) validator: Option<DexterityLaunchAdapterRequest>,
    pub(super) operator: Option<DexterityLaunchAdapterRequest>,
}

fn bind_stage_request(
    request: &mut DexterityLaunchAdapterRequest,
    context: &ModelLaneRoutingExecutionContext,
    policy: ModelLaneRoutingPolicy,
    stage_id: &str,
) {
    request.run_id = context.run_id.clone();
    request.trace_id = context.trace_id.clone();
    request.run_span_id = context.run_span_id.clone();
    request.coordinator_session_id = context.coordinator_session_id.clone();
    request.routing_policy = policy.as_str().to_string();
    request.work_packet_id = Some(context.work_packet_id.clone());
    request.micro_task_id = context.micro_task_id.clone();
    request.task_board_id = Some(context.task_board_id.clone());
    request.owner_session = context.owner_session.clone();
    request.locus_binding_ref = context.locus_ref.clone();
    request.lane_id = format!("{}:{}", context.run_id, stage_id);
    request.lane_span_id = format!("{}:{}", context.run_span_id, stage_id);
    request.role = stage_id.to_string();
}

impl Ac9StageRequestInputs {
    /// Registers the explicitly synthetic in-process local participant. Its zero
    /// numeric handle is not an OS PID, so reclamation cannot target a host process.
    pub(super) fn register_in_process_local(
        &mut self,
        context: &ModelLaneRoutingExecutionContext,
        policy: ModelLaneRoutingPolicy,
        scope: &crate::swarm_orchestration::resource_scope::ResourceScope,
        ledger: &crate::process_ledger::LedgerBatcher,
    ) -> Result<crate::process_ledger::ProcessOwnershipRecordId, crate::process_ledger::ProcessLedgerError> {
        let session = scope.authenticated_session.ok_or_else(||
            crate::process_ledger::ProcessLedgerError::Store("AC9 requires actual authenticated session".into()))?;
        let space = scope.access_space.ok_or_else(||
            crate::process_ledger::ProcessLedgerError::Store("AC9 requires actual access space".into()))?;
        let workspace = scope.workspace.as_ref().ok_or_else(||
            crate::process_ledger::ProcessLedgerError::Store("AC9 requires actual workspace".into()))?;
        let stage = ModelLaneRoutingGraph::for_policy(policy).stages.into_iter()
            .find(|stage| stage.target == ModelLaneRoutingDispatchTarget::LocalModel)
            .expect("each of the six AC9 policies has a local model stage");
        bind_stage_request(&mut self.local, context, policy, &stage.stage_id);
        let mut meta = crate::process_ledger::SpawnMeta::new(
            0,
            crate::process_ledger::ProcessEngineKind::LlamaCpp,
            "synthetic-ac9-in-process",
        );
        meta.pid_authoritative = false;
        meta.model_id = self.local.model_id.clone();
        meta.runtime_binding = Some("local".into());
        meta.session_id = Some(self.local.model_session_id.clone());
        meta.parent_session_id = Some(self.local.coordinator_session_id.clone());
        meta.trace_id = Some(self.local.trace_id.clone());
        meta.span_id = Some(self.local.lane_span_id.clone());
        meta.owner_wp = self.local.work_packet_id.clone();
        meta.wp_id = self.local.work_packet_id.clone();
        meta.mt_id = self.local.micro_task_id.clone();
        meta.role_id = Some(self.local.role.clone());
        meta.metadata_blob = serde_json::json!({
            "synthetic": true,
            "network_egress": false,
            "owner_account_id": scope.owner_account_id.to_string(),
            "actor_principal_id": scope.actor_principal_id.to_string(),
            "authenticated_session_id": session.to_string(),
            "access_space_id": space.to_string(),
            "workspace_id": workspace.as_str(),
            "run_id": self.local.run_id,
            "lane_id": self.local.lane_id,
        });
        let record_id = crate::process_ledger::record_spawn(ledger, meta)?;
        self.local.process_ownership_ref = Some(format!(
            "process-ledger://{}", record_id.as_uuid(),
        ));
        Ok(record_id)
    }

    /// Builds actual pack bytes and their canonical hash through the public memory
    /// builder. Persistence, scope acceptance and process ownership remain separate.
    pub(super) fn build_memory_pack(
        &mut self,
        retriever: &dyn crate::memory::FemsRetriever,
        context: &ModelLaneRoutingExecutionContext,
    ) -> Result<crate::memory::MemoryCapsule, crate::memory::BuilderError> {
        let policy = crate::memory::CapsulePolicyTable::default();
        let capsule = crate::memory::CapsuleBuilder::new(retriever, &policy).build(
            crate::memory::BuildContext {
                task_type: crate::memory::TaskType::SwarmHarnessSession,
                query: context.run_id.clone(),
                role_id: "ac9-policy-wave".into(),
                session_id: context.coordinator_session_id.clone(),
                override_policy: None,
            },
        )?;
        self.local.memory_pack_ref = capsule.pack.pack_id.clone();
        self.local.memory_pack_hash = capsule.pack.memory_pack_hash.clone();
        for request in [&mut self.cloud, &mut self.validator, &mut self.operator]
            .into_iter().flatten()
        {
            request.memory_pack_ref = capsule.pack.pack_id.clone();
            request.memory_pack_hash = capsule.pack.memory_pack_hash.clone();
        }
        Ok(capsule)
    }
}
struct Ac9LaunchCapabilityEvidence {
    normalized: crate::swarm_orchestration::model_lane::DexterityNormalizedLaunch,
    descriptor: crate::swarm_orchestration::model_lane::DexterityLaunchAdapterDescriptor,
    requested_tools: Vec<String>,
    scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    cloud_capabilities_declaration: Option<crate::model_runtime::ModelCapabilities>,
    manifest: crate::storage::artifacts::ArtifactManifest,
}

fn cloud_capabilities_declaration(
    kind: &DexterityLaunchAdapterKind,
) -> Option<crate::model_runtime::ModelCapabilities> {
    match kind {
        DexterityLaunchAdapterKind::ByokCloudOpenAi => Some(
            crate::model_runtime::cloud::openai_byok::OpenAiByokRuntime::cloud_capabilities()),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => Some(
            crate::model_runtime::cloud::anthropic_byok::AnthropicByokRuntime::cloud_capabilities()),
        _ => None,
    }
}

fn capability_snapshot_payload(evidence: &Ac9LaunchCapabilityEvidence) -> crate::kernel::KernelResult<serde_json::Value> {
    let mut normalized = serde_json::to_value(&evidence.normalized)
        .map_err(|error| crate::kernel::KernelError::Artifact(error.to_string()))?;
    normalized.as_object_mut().ok_or(crate::kernel::KernelError::InvalidEvent("normalized launch is not an object"))?
        .remove("effective_capability_snapshot_ref");
    Ok(serde_json::json!({
        "synthetic": true, "contract_negotiation_only": true,
        "resource_scope": evidence.scope, "descriptor": evidence.descriptor,
        "requested_tool_capability_tokens": evidence.requested_tools,
        "normalized_launch_without_self_reference": normalized,
        "cloud_capabilities_declaration_not_live_attestation": evidence.cloud_capabilities_declaration,
    }))
}

fn materialize_launch_capabilities(
    root: &std::path::Path,
    scope: &crate::swarm_orchestration::resource_scope::ResourceScope,
    request: DexterityLaunchAdapterRequest,
) -> crate::kernel::KernelResult<Ac9LaunchCapabilityEvidence> {
    use crate::kernel::KernelError;
    use crate::storage::artifacts::{ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind, write_file_artifact};
    if !request.extra_capability_token_ids.is_empty() {
        return Err(KernelError::InvalidEvent("synthetic contract snapshot does not authorize extra capability tokens"));
    }
    let exact_scope = crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
    let registry = DexterityLaunchAdapterRegistry::standard();
    let descriptor = registry.descriptor(&request.adapter_kind)
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?.clone();
    let requested_tools = request.requested_tool_capability_tokens.clone();
    let cloud_capabilities_declaration = cloud_capabilities_declaration(&request.adapter_kind);
    let normalized = registry.normalize(request)
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
    let mut evidence = Ac9LaunchCapabilityEvidence {
        normalized, descriptor, requested_tools, scope: exact_scope, cloud_capabilities_declaration,
        manifest: ArtifactManifest {
            artifact_id: uuid::Uuid::now_v7(), layer: ArtifactLayer::L2,
            kind: ArtifactPayloadKind::ToolOutput, mime: "application/json".into(),
            filename_hint: Some("ac9-launch-contract-capabilities.json".into()),
            created_at: chrono::Utc::now(), created_by_job_id: None,
            source_entity_refs: Vec::new(), source_artifact_refs: Vec::new(),
            content_hash: String::new(), size_bytes: 0, classification: ArtifactClassification::Low,
            exportable: false, retention_ttl_days: None, pinned: Some(false),
            hash_basis: Some("canonical_json_bytes(synthetic_contract_negotiation)".into()),
            hash_exclude_paths: Vec::new(),
        },
    };
    let bytes = crate::kernel::context_bundle::canonical_json_bytes(&capability_snapshot_payload(&evidence)?);
    evidence.manifest.content_hash = crate::llm::sha256_hex(&bytes);
    evidence.manifest.size_bytes = bytes.len() as u64;
    write_file_artifact(root, &evidence.manifest, &bytes)
        .map_err(|error| KernelError::Artifact(error.to_string()))?;
    evidence.normalized.effective_capability_snapshot_ref = Some(format!("{}/payload",
        crate::storage::artifacts::artifact_root_rel(ArtifactLayer::L2, evidence.manifest.artifact_id)));
    reread_launch_capabilities(root, &evidence)?;
    Ok(evidence)
}

fn reread_launch_capabilities(
    root: &std::path::Path,
    evidence: &Ac9LaunchCapabilityEvidence,
) -> crate::kernel::KernelResult<()> {
    use crate::kernel::KernelError;
    use crate::storage::artifacts::{ArtifactLayer, read_artifact_manifest, read_file_artifact_with_manifest};
    let expected_ref = format!("{}/payload",
        crate::storage::artifacts::artifact_root_rel(ArtifactLayer::L2, evidence.manifest.artifact_id));
    if evidence.normalized.effective_capability_snapshot_ref.as_deref() != Some(expected_ref.as_str()) {
        return Err(KernelError::InvalidEvent("contract snapshot reference differs from actual artifact"));
    }
    let manifest = read_artifact_manifest(root, ArtifactLayer::L2, evidence.manifest.artifact_id)
        .map_err(|error| KernelError::Artifact(error.to_string()))?;
    let actual_manifest = serde_json::to_value(&manifest).map_err(|error| KernelError::Artifact(error.to_string()))?;
    let expected_manifest = serde_json::to_value(&evidence.manifest).map_err(|error| KernelError::Artifact(error.to_string()))?;
    if actual_manifest != expected_manifest {
        return Err(KernelError::InvalidEvent("contract snapshot manifest differs before payload read"));
    }
    let bytes = read_file_artifact_with_manifest(root, &manifest)
        .map_err(|error| KernelError::Artifact(error.to_string()))?;
    let expected = crate::kernel::context_bundle::canonical_json_bytes(&capability_snapshot_payload(evidence)?);
    if bytes != expected
        || manifest.content_hash != crate::llm::sha256_hex(&bytes) || manifest.size_bytes != bytes.len() as u64 {
        return Err(KernelError::InvalidEvent("contract snapshot bytes or manifest differ"));
    }
    Ok(())
}

fn resolve_launch_capabilities(
    root: &std::path::Path,
    scope: &crate::swarm_orchestration::resource_scope::ResourceScope,
    evidence: &Ac9LaunchCapabilityEvidence,
    run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
    lane: &crate::swarm_orchestration::model_lane::ModelLaneRecord,
) -> crate::kernel::KernelResult<()> {
    use crate::kernel::KernelError;
    let exact_scope = crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
    let descriptor = DexterityLaunchAdapterRegistry::standard().descriptor(&evidence.normalized.adapter_kind)
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?.clone();
    let (expected_run, expected_lane) = evidence.normalized.clone().to_records()
        .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
    if !run.lane_ids.contains(&expected_lane.lane_id) {
        return Err(KernelError::InvalidEvent("canonical run omits snapshot lane"));
    }
    if exact_scope != evidence.scope || descriptor != evidence.descriptor
        || cloud_capabilities_declaration(&evidence.normalized.adapter_kind) != evidence.cloud_capabilities_declaration
        || run.inner != expected_run || lane.inner != expected_lane {
        return Err(KernelError::InvalidEvent("contract snapshot differs from canonical scoped run lane or Locus"));
    }
    reread_launch_capabilities(root, evidence)
}

pub(super) fn policy_stage_requests(
    context: &ModelLaneRoutingExecutionContext,
    policy: ModelLaneRoutingPolicy,
    inputs: &Ac9StageRequestInputs,
) -> ModelLaneResult<Vec<(ModelLaneRoutingStageLaunchPlan, DexterityLaunchAdapterRequest)>> {
    use ModelLaneRoutingDispatchTarget as Target;
    let registry = DexterityLaunchAdapterRegistry::standard();
    let mut requests = Vec::new();
    for stage in ModelLaneRoutingGraph::for_policy(policy).stages {
        let (prototype, expected_kind, provider) = match stage.target {
            Target::LocalModel => (&inputs.local, DexterityLaunchAdapterKind::LocalModelRuntime, None),
            Target::CloudModel => (
                inputs.cloud.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    format!("AC9 {} requires producer-backed cloud launch inputs", policy.as_str()),
                ))?,
                DexterityLaunchAdapterKind::ByokCloudOpenAi,
                Some(crate::model_runtime::ProviderKind::ByokCloud),
            ),
            Target::Validator => (
                inputs.validator.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    "AC9 validator lane requires producer-backed validator inputs".into(),
                ))?,
                DexterityLaunchAdapterKind::Validator,
                None,
            ),
            Target::Operator => (
                inputs.operator.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    "AC9 operator lane requires producer-backed operator inputs".into(),
                ))?,
                DexterityLaunchAdapterKind::HumanOperator,
                None,
            ),
            Target::CoordinatorJoin => continue,
        };
        if prototype.adapter_kind != expected_kind {
            return Err(ModelLaneError::InvalidInput(format!(
                "AC9 {} requires {} input, received {}",
                stage.stage_id, expected_kind.as_str(), prototype.adapter_kind.as_str(),
            )));
        }
        let mut request = prototype.clone();
        bind_stage_request(&mut request, context, policy, &stage.stage_id);
        let normalized = registry.normalize(request.clone())?;
        let (_, lane) = normalized.to_records()?;
        requests.push((ModelLaneRoutingStageLaunchPlan {
            stage_id: stage.stage_id,
            dispatch_target: stage.target,
            lane_id: Some(lane.lane_id),
            model_id: lane.model_id,
            provider,
        }, request));
    }
    Ok(requests)
}
pub(super) fn in_process_service(
    store: crate::swarm_orchestration::model_lane::ModelLaneStore,
    ledger: crate::process_ledger::LedgerBatcher,
    recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
) -> (
    std::sync::Arc<crate::swarm_orchestration::operator_chat::OperatorChatLaunchService>,
    std::sync::Arc<super::ControllableFactory>,
) {
    let factory = std::sync::Arc::new(super::ControllableFactory::new(
        super::FactoryBehavior::Succeed, std::time::Duration::from_millis(1), ledger.clone(),
    ));
    let coordinator = std::sync::Arc::new(crate::swarm_orchestration::SwarmCoordinator::new_with_model_lane_store(
        crate::swarm_orchestration::SwarmConfig::new(crate::swarm_orchestration::RunBudget::defaulted(8)),
        factory.clone(), std::sync::Arc::new(crate::swarm_orchestration::RecordingSwarmSink::new()),
        ledger, store,
    ));
    let service = std::sync::Arc::new(
        crate::swarm_orchestration::operator_chat::OperatorChatLaunchService::new(
            coordinator, None::<std::sync::Arc<crate::model_runtime::catalog::ModelCatalog>>, recorder,
        ),
    );
    (service, factory)
}
pub(super) struct Ac9CloudConfiguration {
    pub(super) access: crate::model_runtime::cloud::CloudModelAccess,
    pub(super) payload: serde_json::Value,
    pub(super) payload_sha256: String,
    pub(super) redacted: bool,
}

pub(super) fn cloud_test_configuration(payload: &[u8]) -> Ac9CloudConfiguration {
    use crate::model_runtime::cloud::{ByokProvider, CloudModelAccess, ProviderAccessStatus, enumerate_byok};
    use crate::model_runtime::cloud::secrets_vault::{InMemorySecretsVault, SecretsVault};
    use crate::terminal::redaction::{PatternRedactor, SecretRedactor};

    // This inert credential configures only an isolated in-memory test vault.
    // No runtime client, endpoint, CLI probe or network call is constructed.
    let vault: std::sync::Arc<dyn SecretsVault> = std::sync::Arc::new(InMemorySecretsVault::default());
    vault.put(ByokProvider::OpenAi.vault_lane(), "synthetic-inert-ac9-credential")
        .expect("actual public test-vault producer");
    let access = CloudModelAccess::with_vault(vault, "InMemorySecretsVault");
    let rows = enumerate_byok(&access.registry());
    assert!(rows.iter().any(|row| row.provider == ByokProvider::OpenAi.id()
        && row.status == ProviderAccessStatus::Configured));
    let redacted = PatternRedactor.redact_chunk(payload);
    let payload = serde_json::json!({
        "synthetic": true, "network_egress": false, "text": redacted.redacted,
    });
    let payload_sha256 = crate::llm::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&payload),
    );
    Ac9CloudConfiguration { access, payload, payload_sha256, redacted: redacted.matched }
}

struct Ac9EmptyMemory;

impl crate::memory::FemsRetriever for Ac9EmptyMemory {
    fn retrieve(&self, _query: &str, _top_k: u32)
        -> Result<Vec<crate::memory::RetrievedItem>, crate::memory::FemsError>
    {
        Ok(Vec::new())
    }
}

fn local_request(context: &ModelLaneRoutingExecutionContext) -> DexterityLaunchAdapterRequest {
    DexterityLaunchAdapterRequest {
        adapter_kind: DexterityLaunchAdapterKind::LocalModelRuntime,
        run_id: context.run_id.clone(), lane_id: String::new(),
        trace_id: context.trace_id.clone(), run_span_id: context.run_span_id.clone(),
        lane_span_id: String::new(), coordinator_session_id: context.coordinator_session_id.clone(),
        routing_policy: String::new(), context_bundle_id: format!("{}:context", context.run_id),
        event_ledger_stream_id: context.run_id.clone(), artifact_namespace: context.run_id.clone(),
        work_packet_id: Some(context.work_packet_id.clone()), micro_task_id: context.micro_task_id.clone(),
        task_board_id: Some(context.task_board_id.clone()), owner_session: context.owner_session.clone(),
        locus_binding_ref: context.locus_ref.clone(), role: String::new(), backend: None, adapter_id: None,
        model_id: Some(format!("{}:synthetic-local-model", context.run_id)),
        session_id: context.coordinator_session_id.clone(),
        model_session_id: format!("{}:model-session", context.run_id),
        extra_capability_token_ids: Vec::new(), requested_tool_capability_tokens: Vec::new(),
        effective_capability_snapshot_ref: None, capability_negotiation_ref: None,
        provider_feature_profile_ref: None, requested_execution_policy_ref: None,
        effective_execution_policy_ref: None, projection_plan_ref: None, consent_receipt_ref: None,
        tool_gate_decision_refs: Vec::new(), status: None, heartbeat_at_utc: None,
        lease_expires_at_utc: None, reclaim_after_utc: None, restart_generation: 0,
        cancellation_ref: None, reclaim_policy_ref: None, terminal_status_mapping_ref: None,
        process_ownership_ref: None, no_os_process_reason_ref: None, backpressure_ref: None,
        loop_counter_ref: None, last_runtime_status_ref: None, last_recovery_event_ref: None,
        startup_failure_code: None, startup_failure_ref: None, reason_ref: None,
        run_recovery_hint_ref: None, lane_recovery_hint_ref: None,
        memory_pack_ref: String::new(), memory_pack_hash: String::new(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: format!("{}:synthetic-test-budget", context.run_id),
        selected_model_id: None, candidate_model_ids: Vec::new(),
        procedural_review_status: "synthetic_ac9_input".into(), truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ac9_local_input_producers_reread_memory_and_stop_the_pidless_lifecycle() {
    use futures::FutureExt;
    use crate::memory::MemoryCapsuleIpcStore;
    use crate::process_ledger::{LedgerEventKind, ProcessStop, ReclaimResourceScope};
    use crate::storage::surreal::SurrealProcessLedgerStore;

    let snapshot_root = artifact_workspace("local-input-producers");
    let mut harness = Ac9Harness::create("local-input-producers", snapshot_root.clone()).await;
    harness.run_with_cleanup(move |harness| Box::pin(async move {
        harness.authenticate_workspace().await;
        let scope = harness.scope().clone();
        let cloud = cloud_test_configuration(b"AC9 test context api_key=synthetic-redaction-marker");
        assert!(cloud.redacted);
        assert!(!cloud.payload["text"].as_str().expect("actual redacted text").contains("synthetic-redaction-marker"));
        assert_eq!(cloud.payload_sha256, crate::llm::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&cloud.payload),
        ));
        let label = uuid::Uuid::now_v7().to_string();
        let context = ModelLaneRoutingExecutionContext {
            run_id: format!("ac9-local-input-{label}"), trace_id: format!("ac9-trace-{label}"),
            run_span_id: format!("ac9-span-{label}"),
            coordinator_session_id: scope.authenticated_session.expect("real session").to_string(),
            locus_ref: format!("ac9-locus-{label}"), work_packet_id: "WP-KERNEL-012".into(),
            micro_task_id: Some("MT-017".into()), task_board_id: "WP-KERNEL-012".into(),
            owner_session: "synthetic-ac9-local-input".into(),
            // Input declarations are not accepted wave or artifact evidence.
            initial_input_ref: format!("ac9-input-{label}"),
            initial_input_sha256: crate::llm::sha256_hex(b"synthetic AC9 local acquisition"),
        };
        let mut inputs = Ac9StageRequestInputs {
            local: local_request(&context), cloud: None, validator: None, operator: None,
        };
        let capsule = inputs.build_memory_pack(&Ac9EmptyMemory, &context).expect("real public pack builder");
        assert_eq!(capsule.pack.compute_hash().expect("canonical pack hash"), capsule.pack.memory_pack_hash);
        let db = harness.canonical_database();
        let submitter = crate::memory::SurrealKernelActionSubmitter::with_db(db.clone());
        let record = crate::memory::CapsuleRecord::from_capsule(
            &capsule, chrono::Utc::now(), &context.coordinator_session_id, "synthetic-ac9-local-input",
        );
        crate::memory::CapsuleRecorder { action_catalog: &submitter }
            .record(record.clone()).expect("real public memory record producer");
        let memory_store = crate::memory::SurrealMemoryCapsuleStore::with_db(db);
        assert_eq!(memory_store.get_capsule_record(capsule.id).expect("canonical memory manifest readback"), Some(record));
        // This manifest readback does not assert persisted MemoryPack bytes or scope acceptance.

        let store = std::sync::Arc::new(SurrealProcessLedgerStore::open(harness.canonical_storage()).await.expect("open canonical ledger"));
        let (ledger, drain) = super::ledger_pair();
        // The existing recorder's own in-memory fixture uses seven retention days;
        // this is not the PromptPayload artifact retention policy.
        let recorder = std::sync::Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7)
                .expect("actual in-memory Flight Recorder"),
        );
        let (service, factory) = in_process_service(harness.store().clone(), ledger.clone(), recorder);
        assert_eq!(service.coordinator().model_lane_store().expect("same canonical store").access(), harness.store().access());
        assert_eq!(factory.create_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        let reclaim_scope = ReclaimResourceScope::try_from_stored(
            &scope.owner_account_id.to_string(), &scope.actor_principal_id.to_string(),
            &scope.authenticated_session.expect("real session").to_string(),
            scope.workspace.as_ref().expect("real workspace").as_str(),
            &scope.access_space.expect("real access space").to_string(),
        ).expect("actual returned five-scope");
        let id = inputs.register_in_process_local(&context, ModelLaneRoutingPolicy::LocalFirst, &scope, &ledger)
            .expect("enqueue honest pidless participant");
        let start = ledger.recorded_start(id.as_uuid()).expect("retain exact accepted START for STOP");
        let operation = std::panic::AssertUnwindSafe(async {
            drain.drain_available_to(store.clone()).await.expect("persist actual START");
            let open = store.inspect_ownership_by_process_uuid(&reclaim_scope, id.as_uuid()).await
                .expect("canonical scoped START readback").expect("canonical ownership exists");
            assert_eq!(open.lifecycle_state, LedgerEventKind::Start);
            assert_eq!(open.os_pid, None);
            let initial_payload = serde_json::json!({
                "synthetic": true, "network_egress": false,
                "task": "AC9 local acquisition", "run_id": context.run_id,
                "redacted_cloud_payload": cloud.payload,
                "redacted_cloud_payload_sha256": cloud.payload_sha256,
            });
            // Root selected ONE day for this pre-gate PromptPayload/Low fixture.
            let capabilities = materialize_launch_capabilities(&snapshot_root, &scope, inputs.local.clone())
                .expect("persist and reread actual synthetic contract negotiation");
            let (source_run, _) = capabilities.normalized.clone().to_records().expect("frozen source records");
            let initial = harness.prepare_frozen_local_initial_input(
                capabilities.normalized.clone(), source_run, initial_payload.clone(), 1,
            ).await.expect("real pre-gate file, typed Create and canonical initial readback");
            assert_eq!(initial.normalized.run_id, context.run_id);
            assert_eq!(initial.normalized.trace_id, context.trace_id);
            assert_eq!(initial.normalized.memory_pack_hash, capsule.pack.memory_pack_hash);
            assert_eq!(initial.artifact.payload_json, initial_payload);
            let message = initial.replay.messages.iter().find(|message|
                format!("model-lane-message://{}", message.message_id) == initial.context.initial_input_ref)
                .expect("initial context cites the actual canonical advisory message");
            assert_eq!(initial.context.initial_input_sha256, message.payload_sha256);
            assert_eq!(message.payload_ref, initial.artifact.artifact_ref);
            assert_eq!(message.payload_sha256, initial.artifact.artifact_sha256);
            assert_eq!(initial.replay.run.run_id, initial.context.run_id);
            assert!(initial.replay.lanes.iter().any(|lane|
                lane.lane_id == initial.normalized.lane_id && lane.trace_id == context.trace_id));
            let canonical_lane = initial.replay.lanes.iter()
                .find(|lane| lane.lane_id == capabilities.normalized.lane_id).expect("canonical snapshot lane");
            resolve_launch_capabilities(&snapshot_root, &scope, &capabilities, &initial.replay.run, canonical_lane)
                .expect("resolve actual contract snapshot against canonical scoped Locus before launch");
            let mut wrong_reference = capabilities;
            let actual_reference = wrong_reference.normalized.effective_capability_snapshot_ref.take();
            wrong_reference.normalized.effective_capability_snapshot_ref = Some("capability-snapshot://unresolved".into());
            assert!(matches!(resolve_launch_capabilities(&snapshot_root, &scope, &wrong_reference,
                &initial.replay.run, canonical_lane), Err(crate::kernel::KernelError::InvalidEvent(_))));
            wrong_reference.normalized.effective_capability_snapshot_ref = actual_reference;
            wrong_reference.normalized.run_id.push_str("-wrong-run");
            assert!(matches!(resolve_launch_capabilities(&snapshot_root, &scope, &wrong_reference,
                &initial.replay.run, canonical_lane), Err(crate::kernel::KernelError::InvalidEvent(_))));
            let mut unsupported_extra = inputs.local.clone();
            unsupported_extra.extra_capability_token_ids.push("synthetic-unapproved-extra".into());
            assert!(matches!(materialize_launch_capabilities(&snapshot_root, &scope, unsupported_extra),
                Err(crate::kernel::KernelError::InvalidEvent(_))));
            assert_eq!(factory.create_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
            // Contract negotiation evidence is not live attestation, accepted promotion or dispatch.
        }).catch_unwind().await;
        // Always settle the real lifecycle before propagating an operation panic.
        ledger.record_stop(ProcessStop::from_start(&start, Some(0)).with_stop_reason("synthetic AC9 fixture complete"))
            .expect("enqueue actual matching STOP");
        drain.drain_available_to(store.clone()).await.expect("persist actual STOP");
        let stopped = store.inspect_ownership_by_process_uuid(&reclaim_scope, id.as_uuid()).await
            .expect("canonical scoped STOP readback").expect("canonical stopped ownership exists");
        assert_eq!(stopped.lifecycle_state, LedgerEventKind::Stop);
        assert_eq!(stopped.os_pid, None);
        inputs.local.process_ownership_ref = None;
        cloud.access.remove_byok_key(crate::model_runtime::cloud::ByokProvider::OpenAi)
            .expect("remove isolated inert test credential");
        if let Err(panic) = operation { std::panic::resume_unwind(panic); }
    })).await;
}
#[tokio::test]
async fn ac9_production_rejects_unresolved_receipt_with_forged_or_cross_run_authority() {
    let mut harness = Ac9Harness::create(
        "policy-authority-denials",
        artifact_workspace("policy-authority-denials"),
    )
    .await;
    let observations = harness
        .run_with_cleanup(|harness| Box::pin(async move {
            harness.authenticate_workspace().await;
            let first = harness
                .produce_gate_evidence()
                .await
                .expect("produce and canonically reread first gate evidence");
            let second = harness
                .produce_gate_evidence()
                .await
                .expect("produce and canonically reread independent gate evidence");
            assert_ne!(first.gate.event_id, second.gate.event_id);
            assert_ne!(first.operator_review_receipt_id, second.operator_review_receipt_id);

            let mut observations = Vec::new();
            for case in [
                "unresolved_gate",
                "cross_run_gate",
                "cross_run_operator_review",
                "missing_operator",
            ] {
                let approval_retention = handshake_role_authority::domain::RetentionBinding {
                    policy_ref: ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract".into(),
                    authority_class: "Owner".into(),
                    replay_policy_ref: ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract".into(),
                    artifact_retention_ttl_days: None,
                };
                let designation_retention = handshake_role_authority::domain::RetentionBinding {
                    policy_ref: ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract".into(),
                    authority_class: "Owner".into(),
                    replay_policy_ref: ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract".into(),
                    artifact_retention_ttl_days: None,
                };
                harness.authenticate_workspace().await;
                let requested_workspace = harness.authorization_request().external_resource_id;
                let requested_task_snapshot = serde_json::json!({
                    "format_version": 1,
                    "task_id": "ac9-policy-denial-fixture-task",
                    "claimed_revision": 7,
                    "operations": [
                        {"tool": "delegate_workspace_read",
                         "resource_kind": crate::storage::surreal::resource_authority::ResourceKind::Workspace,
                         "external_resource_id": requested_workspace.clone(),
                         "action": crate::storage::surreal::resource_authority::ResourceAction::Read,
                         "capability_id": "fs.read"},
                        {"tool": "delegate_workspace_create",
                         "resource_kind": crate::storage::surreal::resource_authority::ResourceKind::Workspace,
                         "external_resource_id": requested_workspace,
                         "action": crate::storage::surreal::resource_authority::ResourceAction::Create,
                         "capability_id": "fs.write"}
                    ]
                });
                let mut candidate = harness.prepare_unresolved_receipt_candidate(
                    &first, approval_retention, designation_retention, requested_task_snapshot,
                ).await;
                // This baseline deliberately retains an unresolved promotion receipt.
                // These are combined authority denials, not gate-only causal proof or
                // a claim that a kernel event is itself a promotion-receipt object.
                match case {
                    "unresolved_gate" => {
                        candidate.promotion_gate_ref = format!("unresolved-ac9-gate-{}", uuid::Uuid::now_v7());
                    }
                    "cross_run_gate" => candidate.promotion_gate_ref = second.gate.event_id.clone(),
                    "cross_run_operator_review" => {
                        candidate.operator_authority_ref = Some(second.operator_review_receipt_id.clone());
                    }
                    "missing_operator" => candidate.operator_authority_ref = None,
                    _ => unreachable!("closed authority-denial cases"),
                }

                let before = harness
                    .store()
                    .replay_promotion_decisions(&candidate.run_id)
                    .await
                    .expect("read actual promotion authority before negative");
                let run_id = candidate.run_id.clone();
                let result = harness.record_promotion_candidate(candidate).await;
                let expected_denial = match &result {
                    Err(ModelLaneError::InvalidInput(message)) if case == "missing_operator" => {
                        message == "ROUTING_AUTHORITY_MISSING: stage_id=operator-decision; authority=OperatorAuthority"
                    }
                    Ok(record) if case != "missing_operator" => {
                        record.outcome == ModelLanePromotionOutcome::Denied
                    }
                    _ => false,
                };
                let accepted = matches!(
                    &result,
                    Ok(record) if record.outcome == ModelLanePromotionOutcome::Approved
                );
                let after = harness
                    .store()
                    .replay_promotion_decisions(&run_id)
                    .await
                    .expect("read actual promotion authority after production transition");
                observations.push((
                    case,
                    before.is_empty(),
                    expected_denial,
                    accepted,
                    after.iter().any(|record| record.outcome == ModelLanePromotionOutcome::Approved),
                    format!("{result:?}"),
                ));
            }
            observations
        }))
        .await;

    for (case, initially_empty, expected_denial, accepted, persisted_approval, result) in observations {
        assert!(initially_empty, "{case}: no prior promotion authority may mask the denial");
        assert!(expected_denial, "{case}: require a typed Denied decision or the exact missing-operator error; unrelated errors are not authority-denial proof: {result}");
        assert!(!accepted, "{case}: production must reject unresolved authority: {result}");
        assert!(
            !persisted_approval,
            "{case}: production must not persist approved authority for unresolved evidence"
        );
    }
}

#[test]
fn ac9_six_policy_graphs_reject_reordering_relabelling_and_removed_gates() {
    let _production_entrypoint_compile_anchor = execute_production_routing_wave;
    let policies = ModelLaneRoutingPolicy::all();
    assert_eq!(policies.len(), 6, "AC9 retains all six policy contracts");

    for (index, policy) in policies.iter().copied().enumerate() {
        let canonical = ModelLaneRoutingGraph::for_policy(policy);
        canonical.validate().expect("canonical policy graph validates");
        assert_eq!(
            canonical
                .stages
                .iter()
                .map(|stage| stage.stage_id.as_str())
                .collect::<Vec<_>>(),
            expected_stage_ids(policy),
            "the expected policy stages are explicit, not derived from the returned graph"
        );

        let mut reordered = canonical.clone();
        reordered.stages.reverse();
        assert!(
            matches!(reordered.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject reordered stages",
            policy.as_str()
        );

        let mut relabelled = canonical.clone();
        relabelled.policy = policies[(index + 1) % policies.len()];
        assert!(
            matches!(relabelled.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "changing the policy label cannot authorize another policy's stages"
        );

        let mut duplicate = canonical.clone();
        duplicate.stages.push(canonical.stages[0].clone());
        assert!(
            matches!(duplicate.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject a duplicated stage",
            policy.as_str()
        );

        let mut missing = canonical.clone();
        missing.stages.pop();
        assert!(
            matches!(missing.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject a missing stage",
            policy.as_str()
        );

        let mut ungated = canonical.clone();
        let gated = ungated
            .stages
            .iter_mut()
            .find(|stage| {
                stage.authority_gate
                    != crate::swarm_orchestration::routing::ModelLaneRoutingAuthorityGate::None
            })
            .expect("each policy has a protected cloud, validator, or operator stage");
        gated.authority_gate =
            crate::swarm_orchestration::routing::ModelLaneRoutingAuthorityGate::None;
        assert!(
            matches!(ungated.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject removal of its authority gate",
            policy.as_str()
        );
    }
}
