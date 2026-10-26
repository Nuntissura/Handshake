use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    flight_recorder::{duckdb::DuckDbFlightRecorder, FlightRecorder},
    model_runtime::cloud::{CliBridgeProvider, LiveCliSpawner},
    process_ledger::{
        production_process_sandbox_registry_async, resolve_embedded_runtime_host_scope,
        LedgerDrainJoinOutcome, ProcessReclaimRuntime, ReclaimResourceScope,
    },
    swarm_orchestration::{
        events::DurableSwarmFrDrain,
        model_lane::{ModelLaneError, ModelLaneResult, ModelLaneStore},
        operator_chat::OperatorChatLaunchService,
        production_factory::{
            build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle,
            CloudLaneFactoryConfig,
        },
    },
};
use crate::model_runtime::cloud::official_cli_bridge::CliSubprocessSpawner;

const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

fn prerequisite(error: impl std::fmt::Display) -> ModelLaneError {
    ModelLaneError::InvalidInput(format!("AC9 genuine role runtime prerequisite: {error}"))
}

/// Owns the same runtime, ledger and durable event bridge used by production.
pub(crate) struct Ac9RoleAuthorityRuntime {
    pub(crate) service: Arc<OperatorChatLaunchService>,
    pub(crate) provider: CliBridgeProvider,
    pub(crate) working_dir: PathBuf,
    pub(crate) model: String,
    process: ProcessReclaimRuntime,
    events: DurableSwarmFrDrain,
}

impl Ac9RoleAuthorityRuntime {
    pub(crate) async fn open(store: &ModelLaneStore) -> ModelLaneResult<Self> {
        use crate::api::model_access::{
            load_official_cli_config_from_path, OfficialCliConfigError,
            HANDSHAKE_OFFICIAL_CLI_CONFIG_PATH,
        };
        let path = std::env::var_os(HANDSHAKE_OFFICIAL_CLI_CONFIG_PATH)
            .map(PathBuf::from)
            .ok_or_else(|| prerequisite(format!("{HANDSHAKE_OFFICIAL_CLI_CONFIG_PATH} is unset")))?;
        let (provider, config) = match load_official_cli_config_from_path(&path, CliBridgeProvider::Codex) {
            Ok(config) => (CliBridgeProvider::Codex, config),
            Err(OfficialCliConfigError::Contract("provider")) => (
                CliBridgeProvider::ClaudeCode,
                load_official_cli_config_from_path(&path, CliBridgeProvider::ClaudeCode)
                    .map_err(prerequisite)?,
            ),
            Err(error) => return Err(prerequisite(error)),
        };
        let (launch, _) = config.clone().into_parts();
        let record: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).map_err(prerequisite)?)
            .map_err(prerequisite)?;
        let models = record.get("model_allowlist").and_then(serde_json::Value::as_array)
            .ok_or_else(|| prerequisite("configured CLI model allowlist is absent"))?;
        if models.len() != 1 {
            return Err(prerequisite("role proof requires one explicitly selected configured CLI model"));
        }
        let model = models[0].as_str().filter(|value| !value.is_empty())
            .ok_or_else(|| prerequisite("configured CLI model is empty"))?.to_owned();
        if !config.clone().into_parts().1.contains(&model) {
            return Err(prerequisite("CLI configuration changed while selecting its model"));
        }
        let working_dir = launch.working_dir.clone()
            .ok_or_else(|| prerequisite("configured CLI working directory is absent"))?;
        let scope = store.access().exact_read_scope().cloned()
            .ok_or_else(|| prerequisite("authenticated exact workspace scope is absent"))?;
        let storage = store.surreal_storage().clone();
        let recorder_path = storage.config().path().with_extension(format!(
            "ac9-role-{}.duckdb", uuid::Uuid::now_v7()
        ));
        let recorder: Arc<dyn FlightRecorder> = Arc::new(
            DuckDbFlightRecorder::new_on_path(&recorder_path, 7).map_err(prerequisite)?
        );
        let resource_scope = ReclaimResourceScope::from_exact(&scope).map_err(prerequisite)?;
        let registry = production_process_sandbox_registry_async().await.map_err(prerequisite)?;
        let process = ProcessReclaimRuntime::production(
            storage.clone(), resource_scope.clone(), None, registry.clone(),
            resolve_embedded_runtime_host_scope().map_err(prerequisite)?, DRAIN_TIMEOUT,
        ).await.map_err(prerequisite)?;
        let spawner = Arc::new(LiveCliSpawner::new(
            Arc::new(process.ledger().ledger()), registry.clone(),
        ).with_reclaim(process.reclaim(), resource_scope));
        if let Err(error) = spawner.pin_config(&launch) {
            let report = process.shutdown_and_drain(DRAIN_TIMEOUT).await;
            return Err(prerequisite(format!("CLI launch pin failed: {error}; cleanup: {report:?}")));
        }
        let cloud = CloudLaneFactoryConfig::unconfigured()
            .with_official_cli_provider_replay_captured(provider.id(), spawner, config, None);
        let (service, events) = build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle(
            storage.clone(), recorder, None, process.ledger(), process.reclaim(), cloud,
            Some(registry), None, storage.resource_access_lifecycle(), scope, uuid::Uuid::now_v7(),
        );
        Ok(Self { service, provider, working_dir, model, process, events })
    }

    pub(crate) async fn shutdown(self) -> ModelLaneResult<()> {
        let sessions = self.service.coordinator().drain_all().await;
        let events = self.events.drain_and_join(DRAIN_TIMEOUT).await;
        let process = self.process.shutdown_and_drain(DRAIN_TIMEOUT).await;
        if sessions.is_err() || !events || !process.reclaim_task_quiesced || !process.lease_released
            || !matches!(&process.ledger, LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained)
        {
            return Err(prerequisite(format!(
                "runtime cleanup incomplete: sessions={sessions:?}, events={events}, process={process:?}"
            )));
        }
        Ok(())
    }
}

pub struct Ac9RoleAuthorityFixture {
    runtime: Ac9RoleAuthorityRuntime,
    selectors: crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors,
    read_custody: Arc<crate::storage::surreal::resource_authority::InitialValidatorIssuance>,
}

type PreparedRoleInputs = (
    crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
    handshake_role_authority::domain::RunBinding,
    handshake_role_authority::domain::ArtifactSubject,
    handshake_role_authority::domain::ExpectedBase,
);

pub struct Ac9RoleAuthorityEvidence<'runtime> {
    runtime: &'runtime Ac9RoleAuthorityRuntime,
    selectors: crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors,
    read_custody: Arc<crate::storage::surreal::resource_authority::InitialValidatorIssuance>,
}

impl Ac9RoleAuthorityEvidence<'_> {
    pub async fn launch_validator_destination(
        &self,
        request: crate::swarm_orchestration::model_lane::DexterityLaunchAdapterRequest,
    ) -> ModelLaneResult<(
        crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        crate::swarm_orchestration::model_lane::ModelLaneRecord,
    )> {
        Ac9RoleAuthorityFixture::launch_validator_destination_with_runtime(
            self.runtime, &self.selectors, request,
        ).await
    }
    pub fn selectors(&self) -> crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors {
        self.selectors.clone()
    }

    pub async fn record_promotion_decision(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        operation_id: uuid::Uuid,
        input: crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
    ) -> ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord> {
        let store = self.runtime.service.coordinator().model_lane_store()
            .ok_or_else(|| prerequisite("retained actual producer store is absent"))?;
        store.record_promotion_decision_with_read_custody(
            bootstrap, authorization, operation_id, self.selectors.clone(), input,
            self.read_custody.clone(),
        ).await
    }

    #[cfg(feature = "test-utils")]
    pub async fn record_promotion_decision_with_wrong_current_operation_for_test(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        operation_id: uuid::Uuid,
        input: crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
    ) -> ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord> {
        let store = self.runtime.service.coordinator().model_lane_store()
            .ok_or_else(|| prerequisite("retained actual producer store is absent"))?;
        store.record_promotion_decision_with_wrong_current_operation(
            bootstrap, authorization, operation_id, self.selectors.clone(), input,
            self.read_custody.clone(),
        ).await
    }
}
impl Ac9RoleAuthorityFixture {
    pub async fn produce(
        bootstrap: &crate::AccountBootstrapState,
        store: ModelLaneStore,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        lane: &crate::swarm_orchestration::model_lane::ModelLaneRecord,
        artifact: &crate::swarm_orchestration::model_lane::ModelLaneContextBundleArtifactBindingRecord,
        candidate: &crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
        approval_retention: handshake_role_authority::domain::RetentionBinding,
        designation_retention: handshake_role_authority::domain::RetentionBinding,
        requested_task_snapshot: serde_json::Value,
    ) -> ModelLaneResult<Self> {
        let prepared = Self::prepare_role_inputs(&store, &authorization, run, lane, artifact, candidate)?;
        let runtime = Ac9RoleAuthorityRuntime::open(&store).await?;
        let produced = Self::produce_with_runtime(&runtime, bootstrap, &store, authorization,
            run, artifact, approval_retention, designation_retention, requested_task_snapshot, prepared).await;
        match produced {
            Ok((selectors, read_custody)) => Ok(Self { runtime, selectors, read_custody }),
            Err(error) => {
                match runtime.shutdown().await {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(prerequisite(format!("authority production failed: {error}; {cleanup}"))),
                }
            }
        }
    }

    pub async fn produce_for_candidate<'runtime>(
        &'runtime self,
        bootstrap: &crate::AccountBootstrapState,
        store: ModelLaneStore,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        lane: &crate::swarm_orchestration::model_lane::ModelLaneRecord,
        artifact: &crate::swarm_orchestration::model_lane::ModelLaneContextBundleArtifactBindingRecord,
        candidate: &crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
        approval_retention: handshake_role_authority::domain::RetentionBinding,
        designation_retention: handshake_role_authority::domain::RetentionBinding,
        requested_task_snapshot: serde_json::Value,
    ) -> ModelLaneResult<Ac9RoleAuthorityEvidence<'runtime>> {
        let prepared = Self::prepare_role_inputs(&store, &authorization, run, lane, artifact, candidate)?;
        let (selectors, read_custody) = Self::produce_with_runtime(&self.runtime, bootstrap, &store,
            authorization, run, artifact, approval_retention, designation_retention,
            requested_task_snapshot, prepared).await?;
        Ok(Ac9RoleAuthorityEvidence { runtime: &self.runtime, selectors, read_custody })
    }
    fn prepare_role_inputs(
        store: &ModelLaneStore,
        authorization: &crate::storage::surreal::resource_authority::AuthorizationRequest,
        run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        lane: &crate::swarm_orchestration::model_lane::ModelLaneRecord,
        artifact: &crate::swarm_orchestration::model_lane::ModelLaneContextBundleArtifactBindingRecord,
        candidate: &crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
    ) -> ModelLaneResult<PreparedRoleInputs> {
        use handshake_role_authority::domain as role;
        use crate::storage::surreal::resource_authority::RoleAuthorityAuthInput;
        use sha2::{Digest, Sha256};
        if candidate.run_id != run.run_id || candidate.trace_id != run.trace_id
            || lane.run_id != run.run_id || artifact.run_id != run.run_id
            || artifact.trace_id != run.trace_id
        { return Err(prerequisite("candidate, lane and artifact do not share the persisted run")); }
        let scope = store.access().exact_read_scope().cloned()
            .ok_or_else(|| prerequisite("canonical Owner scope missing"))?;
        let auth = RoleAuthorityAuthInput::new(authorization.session_token.clone(),
            authorization.channel_binding_hash.clone().filter(|value| !value.is_empty())
                .ok_or_else(|| prerequisite("canonical Owner channel missing"))?);
        let locus = run.locus_binding.as_ref()
            .ok_or_else(|| prerequisite("canonical run locus binding missing"))?;
        let required = |value: &Option<String>| value.clone()
            .filter(|value| !value.is_empty()).ok_or_else(|| prerequisite("canonical work attribution missing"));
        let binding = role::RunBinding {
            run_id: run.run_id.clone(), trace_id: run.trace_id.clone(),
            coordinator_session_id: run.coordinator_session_id.clone(), owner_session: run.owner_session.clone(),
            work_packet_id: required(&run.work_packet_id)?, micro_task_id: required(&run.micro_task_id)?,
            task_board_id: required(&run.task_board_id)?, locus_binding_ref: locus.locus_binding_ref.clone(),
            launch_plan_hash: role::ContentSha256::from_hex(&format!("{:x}",
                Sha256::digest(serde_json::to_vec(&candidate.routing_launch_plan).map_err(prerequisite)?)))
                .map_err(prerequisite)?,
        };
        let subject = role::ArtifactSubject {
            artifact_ref: artifact.artifact_ref.clone(), manifest_ref: artifact.artifact_manifest_ref.clone(),
            sha256: role::ContentSha256::from_hex(&artifact.artifact_sha256).map_err(prerequisite)?,
            version: artifact.payload_json.get("artifact_version").and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty()).ok_or_else(|| prerequisite("canonical artifact version absent"))?.into(),
        };
        let expected = role::ExpectedBase {
            aggregate_id: candidate.expected_event_ledger_aggregate_id.clone(),
            aggregate_version: candidate.expected_event_ledger_version.try_into().map_err(prerequisite)?,
            base_ref: Some(candidate.base_snapshot_ref.clone()),
            base_vector_ref: Some(candidate.state_vector.clone()), schema_id: candidate.schema_id.clone(),
        };
        Ok((scope, auth, binding, subject, expected))
    }

    async fn produce_with_runtime(
        runtime: &Ac9RoleAuthorityRuntime,
        bootstrap: &crate::AccountBootstrapState,
        store: &ModelLaneStore,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        artifact: &crate::swarm_orchestration::model_lane::ModelLaneContextBundleArtifactBindingRecord,
        approval_retention: handshake_role_authority::domain::RetentionBinding,
        designation_retention: handshake_role_authority::domain::RetentionBinding,
        requested_task_snapshot: serde_json::Value,
        prepared: PreparedRoleInputs,
    ) -> ModelLaneResult<(
        crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors,
        Arc<crate::storage::surreal::resource_authority::InitialValidatorIssuance>,
    )> {
        use handshake_role_authority::domain as role;
        use crate::swarm_orchestration::model_lane::{
            ArtifactOperatorApprovalRequest, FinalPromotionAuthoritySelectors, RoleEvidenceConsumeSelector,
        };
        use crate::swarm_orchestration::operator_chat::{OperatorChatLaneKind, OperatorChatSelection};
        let (scope, auth, binding, subject, expected) = prepared;
        let produced = async {
            let action = || {
                let id = uuid::Uuid::now_v7().to_string();
                role::ActionKey { action_id: id.clone(), request_id: id.clone(), idempotency_key: id }
            };
            let approval = store.approve_artifact(bootstrap, authorization, ArtifactOperatorApprovalRequest {
                operation_id: uuid::Uuid::now_v7(), approval: role::ApproveOperatorArtifact {
                    action: action(), scope: scope.clone(), run: binding.clone(), subject: subject.clone(),
                    expected: expected.clone(), retention: approval_retention,
                },
            }).await?;
            let operator = RoleEvidenceConsumeSelector {
                evidence_ref: role::RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}",
                    approval.receipt().event.event_id)).map_err(prerequisite)?,
                expectation: role::ResolutionExpectation { receipt_scope: scope.clone(), role: role::Role::Operator,
                    run: binding.clone(), subject: subject.clone(), purpose: role::Purpose::PriorSelection,
                    expected: expected.clone() }, designation_ref: None, producer: None,
            };
            let prompt = format!("Independently review this persisted artifact and its expected base. Report your actual findings; do not assume approval.\n{}",
                serde_json::to_string(&serde_json::json!({"artifact": artifact.payload_json,
                    "subject": subject, "run": binding, "expected": expected})).map_err(prerequisite)?);
            let selection = OperatorChatSelection {
                engine_profile: None,
                engine_profile_session: None,
                lane_kind: OperatorChatLaneKind::Cli, model_id: runtime.model.clone(), cloud_provider: None,
                cli_provider: Some(runtime.provider.id().into()),
                working_dir: runtime.working_dir.to_str().ok_or_else(|| prerequisite("CLI directory is not UTF-8"))?.into(),
                worktree_id: None, prompt: prompt.clone(), owner_session: run.owner_session.clone(),
                parent_session_id: run.coordinator_session_id.clone(),
                work_packet_id: run.work_packet_id.clone(), micro_task_id: run.micro_task_id.clone(),
            };
            let instance = runtime.service.prepare_designated_validator_runtime(
                &selection, &scope, bootstrap, auth.clone(), run).await.map_err(prerequisite)?;
            let runner = runtime.service.validator_runner_runtime_selection(instance)
                .map_err(prerequisite)?;
            let designation = runtime.service.designate_authenticated_validator(
                bootstrap, auth.clone(),
                crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationInput {
                    operation_id: uuid::Uuid::now_v7(),
                    action: action(),
                    scope: scope.clone(),
                    run: binding,
                    subject,
                    expected,
                    retention: designation_retention,
                    runner,
                    requested_task_snapshot,
                },
            ).await.map_err(prerequisite)?;
            let (validator, read_custody) = runtime.service.review_designated_validator_with_custody(bootstrap, auth, &designation,
                operator.clone(), crate::model_runtime::GenerateRequest {
                    id: instance.model_id, prompt: prompt.into(), sampling: Default::default(),
                    lora_overrides: vec![], steering_overrides: vec![], kv_prefix_handle: None,
                    cancel: Default::default(), max_tokens: 2048, stop_sequences: vec![],
                    speculative_mode: None, structured_decoding: None,
                }, 256 * 1024).await?;
            Ok::<_, ModelLaneError>((FinalPromotionAuthoritySelectors { operator, validator }, read_custody))
        }.await;
        produced
    }

    pub async fn record_promotion_decision(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        operation_id: uuid::Uuid,
        input: crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
    ) -> ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord> {
        let store = self.runtime.service.coordinator().model_lane_store()
            .ok_or_else(|| prerequisite("retained actual producer store is absent"))?;
        store.record_promotion_decision_with_read_custody(
            bootstrap, authorization, operation_id, self.selectors.clone(), input,
            self.read_custody.clone(),
        ).await
    }


    pub fn selectors(&self) -> crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors {
        self.selectors.clone()
    }

    pub async fn launch_validator_destination(
        &self,
        request: crate::swarm_orchestration::model_lane::DexterityLaunchAdapterRequest,
    ) -> ModelLaneResult<(
        crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        crate::swarm_orchestration::model_lane::ModelLaneRecord,
    )> {
        Ac9RoleAuthorityFixture::launch_validator_destination_with_runtime(
            &self.runtime, &self.selectors, request,
        ).await
    }
    async fn launch_validator_destination_with_runtime(
        runtime: &Ac9RoleAuthorityRuntime,
        selectors: &crate::swarm_orchestration::model_lane::FinalPromotionAuthoritySelectors,
        request: crate::swarm_orchestration::model_lane::DexterityLaunchAdapterRequest,
    ) -> ModelLaneResult<(
        crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
        crate::swarm_orchestration::model_lane::ModelLaneRecord,
    )> {
        use crate::swarm_orchestration::model_lane::DexterityLaunchAdapterKind;
        let store = runtime.service.coordinator().model_lane_store()
            .ok_or_else(|| prerequisite("retained actual producer store is absent"))?;
        let exact = store.access().exact_read_scope()
            .ok_or_else(|| prerequisite("retained actual producer scope is incomplete"))?;
        let original = crate::storage::surreal::current_record_user_scope()
            .ok_or_else(|| prerequisite("Validator destination requires original authenticated Create"))?;
        let subject = crate::storage::surreal::current_model_lane_subject()
            .ok_or_else(|| prerequisite("Validator destination requires original workspace subject"))?;
        if original.action != crate::storage::surreal::resource_authority::ResourceAction::Create
            || original.capability_id != "fs.write"
            || original.session_id != exact.authenticated_session_id.to_string()
            || original.workspace_id.as_deref() != Some(exact.workspace_id.as_str())
            || original.grant_id.as_deref().is_none_or(str::is_empty)
            || subject.kind != crate::storage::surreal::resource_authority::ResourceKind::Workspace
            || subject.external_id != exact.workspace_id.as_str()
        {
            return Err(prerequisite("Validator destination differs from original authenticated workspace Create"));
        }
        let run = &selectors.validator.expectation.run;
        if request.adapter_kind != DexterityLaunchAdapterKind::Validator
            || request.run_id != run.run_id || request.trace_id != run.trace_id
            || request.coordinator_session_id != run.coordinator_session_id
            || request.owner_session != run.owner_session
            || request.work_packet_id.as_deref() != Some(run.work_packet_id.as_str())
            || request.micro_task_id.as_deref() != Some(run.micro_task_id.as_str())
            || request.task_board_id.as_deref() != Some(run.task_board_id.as_str())
            || request.locus_binding_ref != run.locus_binding_ref
        {
            return Err(prerequisite("Validator destination differs from the actual reviewed run"));
        }
        let producer = selectors.validator.producer.as_ref()
            .ok_or_else(|| prerequisite("retained actual Validator registration is absent"))?;
        let (model, ordinal) = producer.producer_instance_ref.rsplit_once('#')
            .ok_or_else(|| prerequisite("actual Validator instance reference is malformed"))?;
        let model = uuid::Uuid::parse_str(model).map_err(prerequisite)?;
        let ordinal = ordinal.parse::<u32>().map_err(prerequisite)?;
        let instance = crate::swarm_orchestration::ModelInstanceId::new(model.into(), ordinal);
        let coordinator = runtime.service.coordinator();
        let caller = coordinator.authorize_no_os_model_lane(&request, instance)
            .map_err(prerequisite)?;
        coordinator.launch_no_os_model_lane(request, caller).await.map_err(prerequisite)
    }
    pub async fn shutdown(self) -> ModelLaneResult<()> {
        let Self { runtime, selectors: _, read_custody } = self;
        drop(read_custody);
        runtime.shutdown().await
    }
}
