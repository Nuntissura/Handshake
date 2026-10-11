//! WP-1 MT-003: SurrealDB cloud launch authority and scoped CRDT proof.

use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream;
use handshake_core::model_runtime::cloud::{InMemorySecretsVault, SecretsVault};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenPrompt, GenerateRequest, KvCacheHandle, KvCachePolicy,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink,
};
use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterKind, DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest,
    DexterityLaunchContract, DexterityNormalizedLaunch, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy,
    ModelLaneStatus, ModelLaneStore, NewModelLaneCloudConsentReceipt,
    NewModelLaneCloudProjectionPlan, dexterity_spawn_model_session_id,
};
use handshake_core::swarm_orchestration::production_factory::{
    CloudLaneFactoryConfig, CloudLiveRuntime, CloudRuntimeBuilder,
    build_production_swarm_coordinator,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::json;

#[tokio::test]
async fn model_lane_cloud_launch_is_no_egress_and_fail_closed_without_authority() {
    let surreal_root = std::env::temp_dir().join(format!(
        "handshake-mt003-launch-surreal-{}",
        uuid::Uuid::now_v7().simple()
    ));
    std::fs::create_dir_all(&surreal_root).expect("create isolated MT-003 SurrealDB root");
    let surreal_config = SurrealStorageConfig::for_data_dir(&surreal_root)
        .expect("configure isolated MT-003 embedded SurrealDB");
    let storage = SurrealStorage::open(surreal_config.clone())
        .await
        .expect("open isolated MT-003 embedded SurrealDB");
    let exact_scope = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new("workspace-mt003-cloud-no-egress")
            .expect("nonblank launch workspace"),
    };
    let launch_scope =
        ResourceScope::new(exact_scope.owner_account_id, exact_scope.actor_principal_id)
            .with_session(exact_scope.authenticated_session_id)
            .with_access_space(exact_scope.access_space_id)
            .with_workspace(exact_scope.workspace_id.clone());
    let store = ModelLaneStore::new_surreal_scoped(storage.clone(), launch_scope.clone())
        .await
        .expect("bootstrap MT-003 embedded SurrealDB ModelLane authority");
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
    vault
        .put("mt003-focused-inert-openai", "mt003-focused-inert-credential")
        .expect("store generated inert credential in isolated test vault");
    let cloud_builds = Arc::new(AtomicUsize::new(0));
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let cloud = CloudLaneFactoryConfig {
        openai: Some(Arc::new(NoEgressCloudBuilder::new(
            ProviderKind::ByokCloud,
            vault.clone(),
            "mt003-focused-inert-openai",
            cloud_builds.clone(),
            provider_calls.clone(),
        ))),
        anthropic: None,
        official_cli: None,
        official_cli_by_provider: HashMap::new(),
    };
    let coordinator = build_production_swarm_coordinator(
        ledger,
        cloud,
        store.clone(),
        Some(2),
        uuid::Uuid::now_v7(),
        |_event| Ok(()),
    );
    let test_idx = 20_000 + (uuid::Uuid::now_v7().as_u128() as usize % 10_000);

    let missing_raw = launch_request(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        test_idx,
        ModelLaneStatus::Ready,
    );
    let missing_normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(missing_raw)
        .expect("missing-authority cloud launch normalizes before durable preflight");
    let missing_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        test_idx,
        spawn_contract_from_normalized(&missing_normalized),
    );
    let missing_error = coordinator
        .spawn_session(missing_spawn)
        .await
        .expect_err("cloud launch without ProjectionPlan/ConsentReceipt must fail closed");
    assert!(
        missing_error.to_string().contains("CX-MM-007"),
        "missing cloud authority must be a consent denial: {missing_error}"
    );
    assert_eq!(cloud_builds.load(Ordering::SeqCst), 0);
    assert_eq!(provider_calls.load(Ordering::SeqCst), 0);

    let valid_raw = launch_request(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        test_idx + 1,
        ModelLaneStatus::Ready,
    );
    let valid_normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(valid_raw)
        .expect("valid cloud launch normalizes through the registry");
    let valid_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        test_idx + 1,
        spawn_contract_from_normalized(&valid_normalized),
    );
    seed_cloud_launch_authority(
        &store,
        &valid_spawn,
        &DexterityLaunchAdapterKind::ByokCloudOpenAi,
        test_idx + 1,
    )
    .await;
    let expected_model_session = dexterity_spawn_model_session_id(&valid_spawn);
    let seeded_authority = store
        .replay_cloud_consent_authority(&valid_spawn.dexterity_launch.as_ref().unwrap().run_id)
        .await
        .expect("replay focused cloud authority");
    assert_eq!(seeded_authority.projection_plans.len(), 1);
    assert_eq!(
        seeded_authority.projection_plans[0].model_session_id.as_deref(),
        Some(expected_model_session.as_str())
    );
    let instance_id = valid_spawn.instance_id;
    let valid_run_id = valid_spawn
        .dexterity_launch
        .as_ref()
        .expect("valid launch retains Dexterity contract")
        .run_id
        .clone();
    coordinator
        .spawn_session(valid_spawn)
        .await
        .expect("valid inert-vault cloud launch uses the Rust registry");
    prove_live_production_generation(
        &coordinator,
        instance_id,
        &DexterityLaunchAdapterKind::ByokCloudOpenAi,
    )
    .await;
    assert_eq!(cloud_builds.load(Ordering::SeqCst), 1);
    assert_eq!(provider_calls.load(Ordering::SeqCst), 1);

    drop(coordinator);
    drop(store);
    storage
        .shutdown()
        .await
        .expect("close first MT-003 embedded SurrealDB handle");
    let reopened_storage = SurrealStorage::open(surreal_config)
        .await
        .expect("reopen MT-003 embedded SurrealDB after component restart");
    let reopened_store =
        ModelLaneStore::new_surreal_scoped(reopened_storage.clone(), launch_scope)
            .await
            .expect("rebind MT-003 ModelLane authority after restart");
    let replay_after_restart = reopened_store
        .replay_cloud_consent_authority(&valid_run_id)
        .await
        .expect("replay durable MT-003 authority after restart");
    assert_eq!(replay_after_restart.projection_plans.len(), 1);
    assert_eq!(replay_after_restart.consent_receipts.len(), 1);
    assert_eq!(
        replay_after_restart.projection_plans[0]
            .model_session_id
            .as_deref(),
        Some(expected_model_session.as_str())
    );
    drop(reopened_store);
    reopened_storage
        .shutdown()
        .await
        .expect("close reopened MT-003 embedded SurrealDB");
    std::fs::remove_dir_all(&surreal_root).expect("remove isolated MT-003 SurrealDB root");
}

async fn seed_cloud_launch_authority(
    store: &ModelLaneStore,
    spawn: &SpawnRequest,
    adapter_kind: &DexterityLaunchAdapterKind,
    idx: usize,
) {
    let (provider_kind, cloud_model_name) = match adapter_kind {
        DexterityLaunchAdapterKind::ByokCloudOpenAi => ("openai", "gpt-4o"),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => ("anthropic", "claude-sonnet-4"),
        _ => return,
    };
    let contract = spawn
        .dexterity_launch
        .as_ref()
        .expect("cloud launch has Dexterity contract");
    let projection_plan_id = contract
        .projection_plan_ref
        .clone()
        .expect("cloud launch has ProjectionPlan ref");
    let consent_receipt_id = contract
        .consent_receipt_ref
        .clone()
        .expect("cloud launch has ConsentReceipt ref");
    let model_session_id = dexterity_spawn_model_session_id(spawn);
    let requested_model_id = format!("model://dexterity/byok_cloud/{cloud_model_name}");
    let scope_hash = sample_sha256();
    let fan_out_targets = vec![format!("provider://{provider_kind}/byok")];

    let plan = store
        .record_cloud_projection_plan(NewModelLaneCloudProjectionPlan {
            projection_plan_id: projection_plan_id.clone(),
            run_id: contract.run_id.clone(),
            trace_id: contract.trace_id.clone(),
            lane_id: Some(contract.lane_id.clone()),
            model_session_id: Some(model_session_id.clone()),
            provider_kind: Some(provider_kind.into()),
            requested_model_id: Some(requested_model_id.clone()),
            scope_hash: scope_hash.clone(),
            source_artifact_refs: vec![format!("artifact-store://mt003/{idx}/cloud-context.json")],
            payload_artifact_ref: format!("artifact-store://mt003/{idx}/cloud-payload.json"),
            payload_sha256: sample_sha256(),
            redaction_policy_ref: "redaction-policy://mt003/cloud-safe".into(),
            redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
            retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
            export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
            provider_profile_ref: contract.adapter_id.clone(),
            fan_out_targets: fan_out_targets.clone(),
            export_delegation: handshake_core::swarm_orchestration::model_lane::CloudExportDelegation {
                audience_refs: fan_out_targets.clone(),
                source_scope: handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority::from_access(store.access()),
                authorization_receipt_ref: None,
            },
            consent_scope: ModelLaneCloudConsentScope::SingleLane,
            target_bindings: vec![],
            status: ModelLaneCloudProjectionPlanStatus::Active,
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: spawn
                .wp_id
                .clone()
                .expect("cloud launch has work packet binding"),
            micro_task_id: spawn
                .mt_id
                .clone()
                .expect("cloud launch has micro-task binding"),
            task_board_id: contract.task_board_id.clone(),
            owner_session: spawn.owner_role.clone(),
            idempotency_key: format!("idem-projection-mt003-{idx}"),
            created_at_utc: "2026-07-19T00:00:00Z".into(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "provider_call_attempted": false,
                "locus": contract.locus_binding_ref,
            }),
        })
        .await
        .expect("record MT-003 cloud ProjectionPlan authority");

    store
        .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
            consent_receipt_id,
            projection_plan_id,
            projection_plan_hash: plan.projection_plan_hash,
            run_id: contract.run_id.clone(),
            trace_id: contract.trace_id.clone(),
            lane_id: Some(contract.lane_id.clone()),
            model_session_id: Some(model_session_id),
            provider_kind: Some(provider_kind.into()),
            requested_model_id: Some(requested_model_id),
            scope_hash,
            consent_scope: ModelLaneCloudConsentScope::SingleLane,
            target_bindings: vec![],
            retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
            export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
            fan_out_targets,
            approved: true,
            approver: handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority::from_access(store.access()),
            approved_by_ref: "operator://mt003/approval".into(),
            approved_at_utc: "2026-07-19T00:00:10Z".into(),
            valid_from_utc: "2026-01-01T00:00:00Z".into(),
            valid_until_utc: "2027-01-01T00:00:00Z".into(),
            revoked_at_utc: None,
            revocation_ref: None,
            revocation_input_hash: None,
            status: ModelLaneCloudConsentReceiptStatus::Approved,
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: spawn
                .wp_id
                .clone()
                .expect("cloud launch has work packet binding"),
            micro_task_id: spawn
                .mt_id
                .clone()
                .expect("cloud launch has micro-task binding"),
            task_board_id: contract.task_board_id.clone(),
            owner_session: spawn.owner_role.clone(),
            idempotency_key: format!("idem-consent-mt003-{idx}"),
            created_at_utc: "2026-07-19T00:00:15Z".into(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "provider_call_attempted": false,
                "locus": contract.locus_binding_ref,
            }),
        })
        .await
        .expect("record MT-003 cloud ConsentReceipt authority");
}

async fn prove_live_production_generation(
    coordinator: &SwarmCoordinator,
    instance_id: ModelInstanceId,
    adapter_kind: &DexterityLaunchAdapterKind,
) {
    let model_id = coordinator
        .session_model_id(instance_id)
        .expect("production factory publishes its runtime-minted model identity");
    let request = GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("Reply with one short token."),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 1,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    };
    let mut stream = coordinator
        .generate_session_managed(instance_id, request)
        .unwrap_or_else(|error| {
            panic!("production {adapter_kind:?} generation must start: {error}")
        });
    let mut observed_tokens = 0usize;
    while let Some(token) = stream.next().await {
        token.unwrap_or_else(|error| {
            panic!("production {adapter_kind:?} generation must complete: {error}")
        });
        observed_tokens += 1;
    }
    assert!(
        observed_tokens > 0,
        "production {adapter_kind:?} generation returned no provider/runtime output"
    );
}

fn spawn_request_for_adapter(
    adapter_kind: DexterityLaunchAdapterKind,
    idx: usize,
    contract: DexterityLaunchContract,
) -> SpawnRequest {
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 100 + idx as u32),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        format!("coordinator-session-mt003-{idx}"),
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003")
    .with_dexterity_launch(contract);
    match adapter_kind {
        DexterityLaunchAdapterKind::LocalModelRuntime => request,
        DexterityLaunchAdapterKind::ByokCloudOpenAi => request
            .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o")
            .with_byok_cloud_provider(ByokCloudProvider::OpenAi),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => request
            .with_cloud_provider(ProviderKind::ByokCloud, "claude-sonnet-4")
            .with_byok_cloud_provider(ByokCloudProvider::Anthropic),
        DexterityLaunchAdapterKind::OfficialCliBridge => request
            .with_cloud_provider(ProviderKind::OfficialCli, "claude-sonnet")
            .with_sandbox_posture(
                handshake_core::sandbox::TrustClass::Trusted,
                handshake_core::sandbox::IsolationTier::Tier1Container,
                std::collections::BTreeSet::from([
                    handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                handshake_core::sandbox::NetPolicy::HostInherited,
                // MT-003 blocker #2: official-CLI / CLI-bridge lanes MUST carry a
                // resolvable, descriptor-matching requested execution-policy ref.
                // The preflight now rejects unknown/stale refs, so the proof must
                // exercise the real cli_bridge policy authority, not a placeholder.
                handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            ),
        DexterityLaunchAdapterKind::CliBridge => request
            .with_cloud_provider(ProviderKind::OfficialCli, "generic-cli-bridge-model")
            .with_sandbox_posture(
                handshake_core::sandbox::TrustClass::Trusted,
                handshake_core::sandbox::IsolationTier::Tier1Container,
                std::collections::BTreeSet::from([
                    handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                handshake_core::sandbox::NetPolicy::HostInherited,
                handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            ),
        other => panic!("adapter {other:?} is not process-backed"),
    }
}

fn launch_request(
    adapter_kind: DexterityLaunchAdapterKind,
    idx: usize,
    status: ModelLaneStatus,
) -> DexterityLaunchAdapterRequest {
    let process_backed = matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::LocalModelRuntime
            | DexterityLaunchAdapterKind::ByokCloudOpenAi
            | DexterityLaunchAdapterKind::ByokCloudAnthropic
            | DexterityLaunchAdapterKind::OfficialCliBridge
            | DexterityLaunchAdapterKind::CliBridge
    );
    let cloud = matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::ByokCloudOpenAi
            | DexterityLaunchAdapterKind::ByokCloudAnthropic
    );
    let candidate_model_id = match &adapter_kind {
        DexterityLaunchAdapterKind::ByokCloudOpenAi => "model://dexterity/byok_cloud/gpt-4o".into(),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => {
            "model://dexterity/byok_cloud/claude-sonnet-4".into()
        }
        _ => format!("model://mt003/candidate/{idx}"),
    };
    DexterityLaunchAdapterRequest {
        adapter_kind,
        run_id: format!("run-mt003-{idx}"),
        lane_id: format!("lane-mt003-{idx}"),
        trace_id: format!("trace-mt003-{idx}"),
        run_span_id: format!("span-run-mt003-{idx}"),
        lane_span_id: format!("span-lane-mt003-{idx}"),
        coordinator_session_id: "coordinator-session-mt003".into(),
        routing_policy: "dexterity_registry_normalized".into(),
        context_bundle_id: format!("context-bundle://mt003/{idx}"),
        event_ledger_stream_id: format!("event-ledger://mt003/{idx}"),
        artifact_namespace: format!("artifact://mt003/{idx}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-003".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT003".into(),
        locus_binding_ref: format!("locus://wp1/mt003/{idx}"),
        role: format!("lane-role-{idx}"),
        backend: None,
        adapter_id: None,
        model_id: Some(format!("model://mt003/{idx}")),
        session_id: format!("session-mt003-{idx}"),
        model_session_id: format!("model-session-mt003-{idx}"),
        extra_capability_token_ids: vec![],
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: None,
        capability_negotiation_ref: None,
        provider_feature_profile_ref: None,
        requested_execution_policy_ref: None,
        effective_execution_policy_ref: None,
        projection_plan_ref: cloud.then_some(format!("projection-plan://mt003/{idx}")),
        consent_receipt_ref: cloud.then_some(format!("consent://mt003/{idx}")),
        tool_gate_decision_refs: vec![format!("toolgate://mt003/{idx}/allow-read-context")],
        status: Some(status),
        heartbeat_at_utc: Some("2026-06-29T00:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T00:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T00:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: None,
        reclaim_policy_ref: None,
        terminal_status_mapping_ref: None,
        process_ownership_ref: process_backed.then_some(format!("process-ledger://mt003/{idx}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://mt003/{idx}")),
        last_runtime_status_ref: Some(format!("runtime-status://mt003/{idx}")),
        last_recovery_event_ref: Some(format!("recovery://mt003/{idx}")),
        startup_failure_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        run_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#run".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#lane".into()),
        memory_pack_ref: "memory-pack://fems/mt003".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt003".into(),
        selected_model_id: None,
        candidate_model_ids: vec![candidate_model_id],
        procedural_review_status: "preflight_reviewed_and_registry_normalized".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn spawn_contract_from_normalized(launch: &DexterityNormalizedLaunch) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: launch.run_id.clone(),
        lane_id: launch.lane_id.clone(),
        restart_generation: launch.restart_generation,
        trace_id: launch.trace_id.clone(),
        run_span_id: launch.run_span_id.clone(),
        lane_span_id: launch.lane_span_id.clone(),
        routing_policy: launch.routing_policy.clone(),
        context_bundle_id: launch.context_bundle_id.clone(),
        event_ledger_stream_id: launch.event_ledger_stream_id.clone(),
        artifact_namespace: launch.artifact_namespace.clone(),
        task_board_id: launch
            .task_board_id
            .clone()
            .expect("MT-003 normalized launch includes task board"),
        locus_binding_ref: launch.locus_binding_ref.clone(),
        role: launch.role.clone(),
        backend: launch.backend.clone(),
        adapter_id: launch.adapter_id.clone(),
        capability_token_ids: launch.capability_token_ids.clone(),
        effective_capability_snapshot_ref: launch
            .effective_capability_snapshot_ref
            .clone()
            .expect("MT-003 normalized launch includes capability snapshot"),
        projection_plan_ref: launch.projection_plan_ref.clone(),
        consent_receipt_ref: launch.consent_receipt_ref.clone(),
        tool_gate_decision_refs: launch.tool_gate_decision_refs.clone(),
        memory_pack_ref: launch.memory_pack_ref.clone(),
        memory_pack_hash: launch.memory_pack_hash.clone(),
        determinism_mode: launch.determinism_mode.clone(),
        budget_summary_ref: launch.budget_summary_ref.clone(),
        candidate_model_ids: launch.candidate_model_ids.clone(),
        procedural_review_status: launch.procedural_review_status.clone(),
        truncation_warning_ref: launch.truncation_warning_ref.clone(),
        rejection_reason_refs: launch.rejection_reason_refs.clone(),
        run_recovery_hint_ref: launch.run_recovery_hint_ref.clone(),
        lane_recovery_hint_ref: launch.lane_recovery_hint_ref.clone(),
    }
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

/// Deterministic cloud seam for MT-003 production-lane proof. It reads only
/// generated inert credentials from an isolated test vault and returns the
/// existing controllable runtime; no HTTP client, subprocess, or provider
/// endpoint is constructed. `provider_calls` records valid-lane generation so
/// the missing-authority preflight can assert that it stays at zero.
struct NoEgressCloudBuilder {
    provider: ProviderKind,
    vault: Arc<dyn SecretsVault>,
    lane: String,
    builds: Arc<AtomicUsize>,
    provider_calls: Arc<AtomicUsize>,
}

impl NoEgressCloudBuilder {
    fn new(
        provider: ProviderKind,
        vault: Arc<dyn SecretsVault>,
        lane: impl Into<String>,
        builds: Arc<AtomicUsize>,
        provider_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            provider,
            vault,
            lane: lane.into(),
            builds,
            provider_calls,
        }
    }
}

#[async_trait]
impl CloudRuntimeBuilder for NoEgressCloudBuilder {
    fn provider(&self) -> ProviderKind {
        self.provider
    }

    async fn build_loaded(
        &self,
        model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        self.vault
            .get(&self.lane)
            .map_err(|error| format!("isolated inert test credential unavailable: {error}"))?;
        self.builds.fetch_add(1, Ordering::SeqCst);
        let mut runtime = DexterityLaunchProofRuntime::new_with_provider_calls(
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            self.provider_calls.clone(),
        );
        let model_id = runtime
            .load(LoadSpec {
                artifact_path: std::path::PathBuf::new(),
                sha256_expected: String::new(),
                runtime_kind: RuntimeKind::LlamaCpp,
                sampling_defaults: SamplingParams::default(),
                kv_cache_policy: KvCachePolicy::default(),
                declared_capabilities: ModelCapabilities::default(),
                provider: self.provider,
                engine_origin: Some(model_name.to_string()),
                external_engine_import: None,
            })
            .await
            .map_err(|error| format!("no-egress cloud runtime load failed: {error}"))?;
        Ok(CloudLiveRuntime {
            runtime: Arc::new(runtime),
            model_id,
        })
    }
}

struct DexterityLaunchProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
    provider_calls: Option<Arc<AtomicUsize>>,
}

impl DexterityLaunchProofRuntime {
    fn new(loads: Arc<AtomicUsize>, unloads: Arc<AtomicUsize>) -> Self {
        Self::new_with_optional_provider_calls(loads, unloads, None)
    }

    fn new_with_provider_calls(
        loads: Arc<AtomicUsize>,
        unloads: Arc<AtomicUsize>,
        provider_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self::new_with_optional_provider_calls(loads, unloads, Some(provider_calls))
    }

    fn new_with_optional_provider_calls(
        loads: Arc<AtomicUsize>,
        unloads: Arc<AtomicUsize>,
        provider_calls: Option<Arc<AtomicUsize>>,
    ) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("dexterity-mt003-kv"),
            lora: LoraStackHandle::new("dexterity-mt003-lora"),
            steering: SteeringHookHandle::new("dexterity-mt003-steering"),
            loads,
            unloads,
            provider_calls,
        }
    }
}

#[async_trait]
impl ModelRuntime for DexterityLaunchProofRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.unloads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        if let Some(provider_calls) = &self.provider_calls {
            provider_calls.fetch_add(1, Ordering::SeqCst);
        }
        let cancel = req.cancel.clone();
        let items = (0..req.max_tokens.min(2)).map(move |i| {
            if cancel.is_cancelled() {
                Err(ModelRuntimeError::Cancelled)
            } else {
                Ok(handshake_core::model_runtime::GeneratedToken {
                    token_id: i,
                    text: format!("dexterity-mt003-token-{i}"),
                    logprob: None,
                    finish_reason: None,
                })
            }
        });
        Box::pin(stream::iter(items.collect::<Vec<_>>()))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: vec![],
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: vec![] })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(self.kv.clone())
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(self.lora.clone())
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(self.steering.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

#[test]
fn model_lane_crdt_surreal_queries_scope_the_selected_row_on_all_five_fields() {
    let provider = include_str!("../src/storage/surreal/model_lane.rs");
    let scope_predicates = [
        "owner_account_id = $owner_account_id",
        "actor_principal_id = $actor_principal_id",
        "authenticated_session_id = $authenticated_session_id",
        "access_space_id = $access_space_id",
        "workspace_id = $workspace_id",
    ];
    let crdt_query_lines = provider.lines().filter(|line| {
        line.contains("FROM kernel_crdt_updates WHERE")
            || line.contains("FROM kernel_crdt_snapshots WHERE")
            || line.contains("FROM knowledge_crdt_agent_lane_leases WHERE")
            || line.contains("FROM knowledge_crdt_ai_edit_proposals WHERE")
    });
    let mut query_count = 0;
    for query in crdt_query_lines {
        query_count += 1;
        for predicate in scope_predicates {
            assert!(
                query.contains(predicate),
                "CRDT query does not scope its selected durable row with {predicate}: {query}"
            );
        }
    }
    assert!(
        query_count >= 11,
        "expected every CRDT read and guarded subquery"
    );
    assert!(
        !provider.contains("array::len((SELECT VALUE id FROM model_lane_authority"),
        "an unrelated exact-scoped lane must never stand in for CRDT row scope"
    );
    assert!(
        !provider.contains("OR scope_kind = 'document')"),
        "historical document leases must bind the requested document identity"
    );

    let requested = (
        "account-a",
        "actor-a",
        "session-a",
        "space-a",
        "workspace-a",
    );
    for mismatched in [
        (
            "account-b",
            "actor-a",
            "session-a",
            "space-a",
            "workspace-a",
        ),
        (
            "account-a",
            "actor-b",
            "session-a",
            "space-a",
            "workspace-a",
        ),
        (
            "account-a",
            "actor-a",
            "session-b",
            "space-a",
            "workspace-a",
        ),
        (
            "account-a",
            "actor-a",
            "session-a",
            "space-b",
            "workspace-a",
        ),
    ] {
        assert_ne!(
            requested, mismatched,
            "same-workspace partial scope must be denied"
        );
    }
}

#[tokio::test]
async fn model_lane_launch_rejects_direct_endpoint_frontend_tauri_and_terminal_bypass() {
    let registry = DexterityLaunchAdapterRegistry::standard();
    for adapter_kind in [
        DexterityLaunchAdapterKind::DirectEndpoint,
        DexterityLaunchAdapterKind::FrontendAppSrc,
        DexterityLaunchAdapterKind::AppSrcTauri,
        DexterityLaunchAdapterKind::TerminalOnly,
        DexterityLaunchAdapterKind::ExternalCompat,
    ] {
        let err = registry
            .normalize(launch_request(
                adapter_kind.clone(),
                90,
                ModelLaneStatus::Ready,
            ))
            .expect_err("bypass adapter must fail before persistence");
        assert!(
            err.to_string().contains("bypass") || err.to_string().contains("external_compat"),
            "unexpected bypass error: {err}"
        );
    }

    let mut unsupported = launch_request(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        91,
        ModelLaneStatus::Ready,
    );
    unsupported.requested_tool_capability_tokens =
        vec!["tool-capability://unsupported-shell".into()];
    let err = registry
        .normalize(unsupported)
        .expect_err("unsupported tool capability must fail closed");
    assert!(err.to_string().contains("unsupported tool capability"));

    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let calls = Arc::new(AtomicUsize::new(0));
    let compensations = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(CountingFactory {
        calls: calls.clone(),
        compensations: compensations.clone(),
    });
    let coordinator = SwarmCoordinator::new(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 34),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003-no-store",
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003")
    .with_dexterity_launch(spawn_contract("run-mt003-no-store", "lane-mt003-no-store"));
    let err = coordinator
        .spawn_session(request)
        .await
        .expect_err("Dexterity launch without ModelLaneStore must fail before factory");
    assert!(matches!(err, SwarmError::LedgerFailed(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn spawn_contract(run_id: &str, lane_id: &str) -> DexterityLaunchContract {
    spawn_contract_for_adapter(run_id, lane_id, "llama_cpp")
}

fn spawn_contract_for_adapter(
    run_id: &str,
    lane_id: &str,
    adapter_id: &str,
) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: run_id.into(),
        lane_id: lane_id.into(),
        restart_generation: 0,
        trace_id: "trace-mt003-runtime".into(),
        run_span_id: "span-run-mt003-runtime".into(),
        lane_span_id: "span-lane-mt003-runtime".into(),
        routing_policy: "swarm_coordinator_model_runtime".into(),
        context_bundle_id: "context-bundle://mt003/runtime".into(),
        event_ledger_stream_id: "event-ledger://mt003/runtime".into(),
        artifact_namespace: "artifact://mt003/runtime".into(),
        task_board_id: "task-board://wp-1".into(),
        locus_binding_ref: "locus://wp1/mt003/runtime".into(),
        role: "local-runtime-lane".into(),
        backend: "model_runtime".into(),
        adapter_id: adapter_id.into(),
        capability_token_ids: vec!["capability://dexterity/local-generate".into()],
        effective_capability_snapshot_ref: "capability-snapshot://mt003/runtime".into(),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt003/runtime/allow-read-context".into()],
        memory_pack_ref: "memory-pack://fems/mt003/runtime".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt003/runtime".into(),
        candidate_model_ids: vec!["model://mt003/runtime-local".into()],
        procedural_review_status: "runtime_spawn_preflighted".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
        run_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#runtime".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#reclaim".into()),
    }
}

struct CountingFactory {
    calls: Arc<AtomicUsize>,
    compensations: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "CountingFactory must not be called by missing-store preflight".into(),
        ))
    }

    async fn cancel_pending_create(&self, _request: &SpawnRequest) -> Result<(), SwarmError> {
        self.compensations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
