//! WP-1 MT-003: Dexterity launch adapter runtime proof.
//!
//! These tests use the Rust backend registry, SwarmCoordinator, PostgreSQL, and
//! EventLedger paths. They intentionally do not use frontend, Tauri, terminal,
//! or direct endpoint launch authority.

mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream;
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, LoadSpec,
    LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind,
    RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    dexterity_spawn_model_session_id, DexterityLaunchAdapterKind, DexterityLaunchAdapterRegistry,
    DexterityLaunchAdapterRequest, DexterityLaunchContract, DexterityNormalizedLaunch,
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneRecoveryState,
    ModelLaneStatus, ModelLaneStore, NewModelLaneCloudConsentReceipt,
    NewModelLaneCloudProjectionPlan, RuntimeBinding,
};
use handshake_core::swarm_orchestration::production_factory::{
    build_production_swarm_coordinator, CloudLaneFactoryConfig,
};
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, ModelSessionState,
    RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::json;

#[tokio::test]
async fn model_lane_launch_all_lane_kinds_through_rust_registry() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-003 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity launch schema");
    let store = ModelLaneStore::new(pool.clone());
    let registry = DexterityLaunchAdapterRegistry::standard();
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(DexterityLaunchProofFactory {
        ledger: ledger.clone(),
        loads: loads.clone(),
        unloads: unloads.clone(),
    });
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(8)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let mut process_backed_count = 0usize;
    let mut authority_instance_id = None;

    for (idx, adapter_kind) in supported_adapters().into_iter().enumerate() {
        let raw_launch = launch_request(adapter_kind.clone(), idx, ModelLaneStatus::Ready);
        let launch = registry
            .normalize(raw_launch.clone())
            .expect("registered Dexterity adapter normalizes");
        let (run, lane) = if adapter_uses_no_os_runtime(&adapter_kind) {
            let caller = coordinator
                .authorize_no_os_model_lane(
                    &raw_launch,
                    authority_instance_id.expect("process-backed authority session exists"),
                )
                .expect("live Dexterity authority session issues no-OS caller receipt");
            coordinator
                .launch_no_os_model_lane(raw_launch, caller)
                .await
                .expect("no-OS Dexterity lane launches through SwarmCoordinator")
        } else {
            process_backed_count += 1;
            let spawn = spawn_request_for_adapter(
                adapter_kind.clone(),
                idx,
                spawn_contract_from_normalized(&launch),
            );
            seed_cloud_launch_authority(&store, &spawn, &adapter_kind, idx).await;
            let instance_id = spawn.instance_id;
            let spawned = coordinator
                .spawn_session(spawn)
                .await
                .expect("process-backed Dexterity lane launches through SwarmCoordinator");
            assert_eq!(spawned, instance_id);
            assert!(
                coordinator.session_runtime(instance_id).is_some(),
                "runtime must not be exposed until the Dexterity launch record commits"
            );
            authority_instance_id.get_or_insert(instance_id);
            let replay = store
                .replay_run(&launch.run_id)
                .await
                .expect("spawn_session launch replay exists");
            assert_eq!(replay.lanes.len(), 1);
            (replay.run, replay.lanes.into_iter().next().unwrap())
        };
        assert!(lane.event_ledger_event_id.starts_with("KE-"));
        assert!(lane.event_ledger_seq > 0);
        assert_eq!(run.event_ledger_stream_id, launch.event_ledger_stream_id);
        assert_eq!(lane.event_ledger_stream_id, launch.event_ledger_stream_id);
        assert!(lane.capability_negotiation_ref.is_some());
        assert!(lane.provider_feature_profile_ref.is_some());
        assert!(lane.requested_execution_policy_ref.is_some());
        assert!(lane.effective_execution_policy_ref.is_some());
        assert!(lane.cancellation_ref.is_some());
        assert!(lane.reclaim_policy_ref.is_some());
        assert!(lane.terminal_status_mapping_ref.is_some());
        assert_eq!(lane.owner_session, "KERNEL_BUILDER-MT003");
        assert_eq!(lane.trace_id, format!("trace-mt003-{idx}"));
        let stream_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger \
             WHERE session_run_id = $1 \
               AND aggregate_type IN ('model_lane_run', 'model_lane')",
        )
        .bind(&launch.event_ledger_stream_id)
        .fetch_one(&pool)
        .await
        .expect("count Dexterity EventLedger stream rows");
        assert_eq!(
            stream_rows, 2,
            "run and lane events must bind to the declared EventLedger stream"
        );
        if matches!(
            lane.runtime_binding,
            RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
        ) {
            assert!(lane
                .process_ownership_ref
                .as_deref()
                .expect("process-backed lane has ownership ref")
                .starts_with("process-ledger://"));
            assert!(lane.no_os_process_reason_ref.is_none());
        } else {
            assert!(lane.process_ownership_ref.is_none());
            assert!(lane
                .no_os_process_reason_ref
                .as_deref()
                .expect("no-OS-process lane has explicit equivalent")
                .starts_with("no-os-process://"));
        }
    }

    assert_eq!(loads.load(Ordering::SeqCst), process_backed_count);
    coordinator
        .drain_all()
        .await
        .expect("drain Dexterity runtime proof");
    assert_eq!(unloads.load(Ordering::SeqCst), process_backed_count);
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
    let factory = Arc::new(CountingFactory {
        calls: calls.clone(),
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

    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for no-OS caller proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated no-OS caller schema");
    let store = ModelLaneStore::new(pool);
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let preflight_calls = Arc::new(AtomicUsize::new(0));
    let preflight_coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: preflight_calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let missing_dexterity_contract_request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 36),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003-missing-dexterity-contract",
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003");
    let err = preflight_coordinator
        .spawn_session(missing_dexterity_contract_request)
        .await
        .expect_err("ModelLaneStore-backed coordinator must reject missing Dexterity contract");
    assert!(matches!(err, SwarmError::LedgerFailed(_)), "got {err}");
    assert!(
        err.to_string().contains("with_dexterity_launch"),
        "expected missing Dexterity contract error, got {err}"
    );
    assert_eq!(
        preflight_calls.load(Ordering::SeqCst),
        0,
        "missing Dexterity contract must fail before factory creation"
    );

    let missing_wp_mt_request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 35),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003-missing-wp-mt",
    )
    .with_dexterity_launch(spawn_contract(
        "run-mt003-missing-wp-mt",
        "lane-mt003-missing-wp-mt",
    ));
    let err = preflight_coordinator
        .spawn_session(missing_wp_mt_request)
        .await
        .expect_err("missing WP/MT must fail before factory creation");
    assert!(matches!(err, SwarmError::LedgerFailed(_)), "got {err}");
    assert!(
        err.to_string().contains("wp_id"),
        "expected missing wp_id preflight error, got {err}"
    );
    assert_eq!(
        preflight_calls.load(Ordering::SeqCst),
        0,
        "Dexterity preflight must run before factory creation"
    );

    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads: loads.clone(),
            unloads: unloads.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    );
    let authority_launch = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            92,
            ModelLaneStatus::Ready,
        ))
        .expect("authority launch normalizes");
    let authority_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        92,
        spawn_contract_from_normalized(&authority_launch),
    );
    let authority_instance_id = authority_spawn.instance_id;
    coordinator
        .spawn_session(authority_spawn)
        .await
        .expect("spawn no-OS authority session");

    let mut forged_owner = launch_request(
        DexterityLaunchAdapterKind::Subagent,
        93,
        ModelLaneStatus::Ready,
    );
    forged_owner.owner_session = "FORGED-MT003".into();
    let err = coordinator
        .authorize_no_os_model_lane(&forged_owner, authority_instance_id)
        .expect_err("forged no-OS owner must fail before caller receipt");
    assert!(
        err.to_string().contains("authority owner"),
        "expected owner authorization error, got {err}"
    );

    let subagent = launch_request(
        DexterityLaunchAdapterKind::Subagent,
        94,
        ModelLaneStatus::Ready,
    );
    let caller = coordinator
        .authorize_no_os_model_lane(&subagent, authority_instance_id)
        .expect("live authority issues subagent caller receipt");
    let validator = launch_request(
        DexterityLaunchAdapterKind::Validator,
        95,
        ModelLaneStatus::Ready,
    );
    let err = coordinator
        .launch_no_os_model_lane(validator, caller)
        .await
        .expect_err("caller receipt must be bound to exact adapter/run/lane");
    assert!(
        err.to_string().contains("caller adapter") || err.to_string().contains("receipt is bound"),
        "expected bound caller receipt error, got {err}"
    );

    let stale_no_os = launch_request(
        DexterityLaunchAdapterKind::Subagent,
        96,
        ModelLaneStatus::Ready,
    );
    let stale_caller = coordinator
        .authorize_no_os_model_lane(&stale_no_os, authority_instance_id)
        .expect("live authority issues stale-receipt proof caller");
    coordinator
        .cancel_session(
            authority_instance_id,
            "retire_authority_for_stale_no_os_proof",
        )
        .await
        .expect("cancel no-OS authority session");
    let err = coordinator
        .launch_no_os_model_lane(stale_no_os, stale_caller)
        .await
        .expect_err("stale no-OS caller receipt must fail after authority removal");
    assert!(
        err.to_string().contains("UNKNOWN_INSTANCE")
            || err.to_string().contains("authority session"),
        "expected stale authority error, got {err}"
    );
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn model_lane_launch_records_factory_failure_through_swarm_coordinator() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for failed launch proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity failed-launch schema");
    let store = ModelLaneStore::new(pool.clone());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(CountingFactory {
        calls: calls.clone(),
    });
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 35),
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003-factory-failed",
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003")
    .with_dexterity_launch(spawn_contract(
        "run-mt003-factory-failed",
        "lane-mt003-factory-failed",
    ));
    let err = coordinator
        .spawn_session(request)
        .await
        .expect_err("factory failure must propagate after failed lane record");
    assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let replay = store
        .replay_run("run-mt003-factory-failed")
        .await
        .expect("factory failure records failed Dexterity run");
    assert_eq!(replay.lanes.len(), 1);
    let lane = &replay.lanes[0];
    assert_eq!(lane.status, ModelLaneStatus::Failed);
    assert_eq!(lane.recovery_state, ModelLaneRecoveryState::Reclaimable);
    assert_eq!(lane.failstate_code.as_deref(), Some("factory_failed"));
    assert!(lane.startup_failure_ref.is_some());
    assert!(lane.reason_ref.is_some());
    assert!(lane.process_ownership_ref.is_none());
    assert!(lane
        .no_os_process_reason_ref
        .as_deref()
        .expect("failed-before-process lane records no-OS equivalent")
        .starts_with("no-os-process://factory-create-failed/"));
}

#[tokio::test]
async fn production_builder_wires_model_lane_store_for_failed_dexterity_launch() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for production-builder proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated production-builder Dexterity schema");
    let store = ModelLaneStore::new(pool.clone());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let coordinator = build_production_swarm_coordinator(
        ledger,
        CloudLaneFactoryConfig::unconfigured(),
        store.clone(),
        Some(1),
        uuid::Uuid::now_v7(),
        |_ev| Ok(()),
    );
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 36),
        RuntimeAdapterBinding::Candle,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003-production-builder",
    )
    .with_local_artifact(
        "D:/__handshake_no_such_dexterity_model__/model.safetensors",
        sample_sha256(),
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003")
    .with_dexterity_launch(spawn_contract_for_adapter(
        "run-mt003-production-builder",
        "lane-mt003-production-builder",
        "candle",
    ));
    let err = coordinator
        .spawn_session(request)
        .await
        .expect_err("missing local artifact must fail after failed lane record");
    assert!(matches!(err, SwarmError::FactoryFailed(_)), "got {err}");

    let replay = store
        .replay_run("run-mt003-production-builder")
        .await
        .expect("production builder attached ModelLaneStore");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Failed);
    assert_eq!(
        replay.lanes[0].no_os_process_reason_ref.as_deref(),
        Some("no-os-process://factory-create-failed/lane-mt003-production-builder")
    );
}

#[tokio::test]
async fn model_lane_launch_cancellation_reclaim_contracts_all_lane_kinds() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-003 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity reclaim schema");
    let store = ModelLaneStore::new(pool.clone());
    let registry = DexterityLaunchAdapterRegistry::standard();

    for (idx, adapter_kind) in supported_adapters().into_iter().enumerate() {
        let launch = registry
            .normalize(launch_request(
                adapter_kind,
                idx + 20,
                ModelLaneStatus::Ready,
            ))
            .expect("registered Dexterity adapter normalizes");
        assert!(launch
            .cancellation_ref
            .as_deref()
            .unwrap()
            .starts_with("cancel-token://"));
        assert!(launch
            .reclaim_policy_ref
            .as_deref()
            .unwrap()
            .starts_with("reclaim-policy://"));
        assert!(launch
            .terminal_status_mapping_ref
            .as_deref()
            .unwrap()
            .starts_with("terminal-status://session-broker/"));
    }

    let mut missing_cancel = registry
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            50,
            ModelLaneStatus::Ready,
        ))
        .expect("registered launch");
    missing_cancel.cancellation_ref = None;
    let err = store
        .record_normalized_launch(missing_cancel)
        .await
        .expect_err("missing cancellation ref fails closed");
    assert!(err.to_string().contains("cancellation_ref"));
    let missing_cancel_replay = store
        .replay_run("run-mt003-50")
        .await
        .expect_err("rejected lane must not leave a partial run");
    assert!(missing_cancel_replay.to_string().contains("not found"));

    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads: loads.clone(),
            unloads: unloads.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let authority_launch = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            61,
            ModelLaneStatus::Ready,
        ))
        .expect("authority launch normalizes");
    let authority_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        61,
        spawn_contract_from_normalized(&authority_launch),
    );
    let authority_instance_id = authority_spawn.instance_id;
    coordinator
        .spawn_session(authority_spawn)
        .await
        .expect("spawn no-OS reclaim authority session");
    let loads_before_no_os = loads.load(Ordering::SeqCst);
    let failed_no_os = failed_launch_request(60);
    let caller = coordinator
        .authorize_no_os_model_lane(&failed_no_os, authority_instance_id)
        .expect("live authority issues failed no-OS caller receipt");
    let (_run, lane) = coordinator
        .launch_no_os_model_lane(failed_no_os, caller)
        .await
        .expect("failed no-OS startup launch persists through SwarmCoordinator");
    assert_eq!(
        loads.load(Ordering::SeqCst),
        loads_before_no_os,
        "no-OS lane launch must not load another process runtime"
    );
    assert_eq!(lane.status, ModelLaneStatus::Failed);
    assert_eq!(lane.recovery_state, ModelLaneRecoveryState::Reclaimable);
    assert_eq!(lane.failstate_code.as_deref(), Some("startup_failed"));
    assert!(lane.startup_failure_ref.is_some());
    assert!(lane.reason_ref.is_some());
    coordinator
        .drain_all()
        .await
        .expect("drain no-OS reclaim authority session");
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn model_lane_launch_rejects_ready_transition_before_persistence_commit() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for Ready race proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Ready race schema");
    let store = ModelLaneStore::new(pool.clone());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = Arc::new(SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads,
            unloads,
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    ));
    let launch = launch_request(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        93,
        ModelLaneStatus::Ready,
    );
    let spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        93,
        spawn_contract_from_normalized(
            &DexterityLaunchAdapterRegistry::standard()
                .normalize(launch)
                .expect("launch normalizes"),
        ),
    );
    let instance_id = spawn.instance_id;

    let mut lock_conn = pool.acquire().await.expect("lock connection");
    sqlx::query("BEGIN")
        .execute(&mut *lock_conn)
        .await
        .expect("begin lock transaction");
    sqlx::query("LOCK TABLE model_lane_runs IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock_conn)
        .await
        .expect("hold model lane run insert lock");

    let spawn_coordinator = coordinator.clone();
    let spawn_task = tokio::spawn(async move { spawn_coordinator.spawn_session(spawn).await });
    for _ in 0..200 {
        if coordinator.session_state(instance_id) == Some(ModelSessionState::Loading) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Loading),
        "spawn must be in Loading while ModelLane persistence is blocked"
    );
    let err = coordinator
        .transition(instance_id, ModelSessionState::Ready)
        .expect_err("public Ready transition must fail before ModelLane persistence commits");
    assert!(
        err.to_string()
            .contains("before ModelLane persistence commits"),
        "expected persistence gate error, got {err}"
    );
    assert!(
        coordinator.session_runtime(instance_id).is_none(),
        "runtime must not be exposed while Dexterity persistence is blocked"
    );

    sqlx::query("COMMIT")
        .execute(&mut *lock_conn)
        .await
        .expect("release model lane run lock");
    let spawned = spawn_task
        .await
        .expect("spawn task joins")
        .expect("spawn succeeds");
    assert_eq!(spawned, instance_id);
    assert_eq!(
        coordinator.session_state(instance_id),
        Some(ModelSessionState::Ready)
    );
    assert!(coordinator.session_runtime(instance_id).is_some());
    coordinator
        .drain_all()
        .await
        .expect("drain Ready race session");
}

#[tokio::test]
async fn model_lane_launch_cancel_session_records_terminal_model_lane_state() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for cancellation proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated cancellation schema");
    let store = ModelLaneStore::new(pool.clone());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads,
            unloads: unloads.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let launch = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            94,
            ModelLaneStatus::Ready,
        ))
        .expect("launch normalizes");
    let spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        94,
        spawn_contract_from_normalized(&launch),
    );
    let instance_id = spawn.instance_id;
    coordinator
        .spawn_session(spawn)
        .await
        .expect("spawn cancellation proof lane");
    let ready_replay = store
        .replay_run(&launch.run_id)
        .await
        .expect("ready replay exists");
    let ready_seq = ready_replay.lanes[0].event_ledger_seq;

    coordinator
        .cancel_session(instance_id, "operator_cancelled_mt003_runtime_proof")
        .await
        .expect("cancel session records terminal lane");
    let replay = store
        .replay_run(&launch.run_id)
        .await
        .expect("cancelled replay exists");
    assert_eq!(replay.lanes.len(), 1);
    let lane = &replay.lanes[0];
    assert_eq!(lane.status, ModelLaneStatus::Cancelled);
    assert_eq!(lane.recovery_state, ModelLaneRecoveryState::Terminal);
    assert_eq!(lane.failstate_code.as_deref(), Some("cancelled"));
    assert!(lane.reason_ref.is_some());
    assert!(
        lane.event_ledger_seq > ready_seq,
        "cancelled lane must point at a newer EventLedger terminal event"
    );
    let cancel_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND source_component = 'dexterity_model_lane' \
           AND event_type = 'SESSION_CANCELLED'",
    )
    .bind(&launch.event_ledger_stream_id)
    .fetch_one(&pool)
    .await
    .expect("count cancellation EventLedger rows");
    assert_eq!(cancel_events, 1);
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn model_lane_launch_reaper_records_terminal_state_before_teardown() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for reaper terminal proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated reaper terminal schema");
    let store = ModelLaneStore::new(pool.clone());
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1))
            .with_lease_ttl(Duration::from_millis(25))
            .with_reaper_scan_interval(Duration::from_millis(10)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads,
            unloads: unloads.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store.clone(),
    );
    let launch = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            97,
            ModelLaneStatus::Ready,
        ))
        .expect("launch normalizes");
    let spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        97,
        spawn_contract_from_normalized(&launch),
    );
    let instance_id = spawn.instance_id;
    coordinator
        .spawn_session(spawn)
        .await
        .expect("spawn reaper terminal proof lane");
    coordinator.start_reaper();
    for _ in 0..80 {
        if coordinator.session_state(instance_id).is_none() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    coordinator.stop_reaper();
    assert_eq!(
        coordinator.session_state(instance_id),
        None,
        "lease reaper must reclaim expired Dexterity session"
    );

    let replay = store
        .replay_run(&launch.run_id)
        .await
        .expect("reaper cancelled replay exists");
    assert_eq!(replay.lanes.len(), 1);
    let lane = &replay.lanes[0];
    assert_eq!(lane.status, ModelLaneStatus::Cancelled);
    assert_eq!(lane.recovery_state, ModelLaneRecoveryState::Terminal);
    assert_eq!(lane.failstate_code.as_deref(), Some("cancelled"));
    assert_eq!(
        lane.reason_ref.as_deref(),
        Some("terminal-reason://dexterity/lane-mt003-97/cancelled")
    );
    let terminal_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_terminal' \
           AND event_type = 'SESSION_CANCELLED'",
    )
    .bind(&launch.event_ledger_stream_id)
    .fetch_one(&pool)
    .await
    .expect("count reaper terminal EventLedger rows");
    assert_eq!(terminal_events, 1);
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        1,
        "reaper must unload only after terminal ModelLane persistence succeeds"
    );
}

fn supported_adapters() -> Vec<DexterityLaunchAdapterKind> {
    vec![
        DexterityLaunchAdapterKind::LocalModelRuntime,
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        DexterityLaunchAdapterKind::ByokCloudAnthropic,
        DexterityLaunchAdapterKind::OfficialCliBridge,
        DexterityLaunchAdapterKind::CliBridge,
        DexterityLaunchAdapterKind::HumanOperator,
        DexterityLaunchAdapterKind::Subagent,
        DexterityLaunchAdapterKind::Validator,
    ]
}

fn adapter_uses_no_os_runtime(adapter_kind: &DexterityLaunchAdapterKind) -> bool {
    matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::HumanOperator
            | DexterityLaunchAdapterKind::Subagent
            | DexterityLaunchAdapterKind::Validator
    )
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

fn failed_launch_request(idx: usize) -> DexterityLaunchAdapterRequest {
    let mut request = launch_request(
        DexterityLaunchAdapterKind::Subagent,
        idx,
        ModelLaneStatus::Failed,
    );
    request.startup_failure_code = Some("startup_failed".into());
    request.startup_failure_ref = Some(format!("startup-failure://mt003/{idx}"));
    request.reason_ref = Some(format!("reason://mt003/{idx}/subagent-startup"));
    request
}

fn spawn_contract(run_id: &str, lane_id: &str) -> DexterityLaunchContract {
    spawn_contract_for_adapter(run_id, lane_id, "llama_cpp")
}

fn spawn_contract_from_normalized(launch: &DexterityNormalizedLaunch) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: launch.run_id.clone(),
        lane_id: launch.lane_id.clone(),
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

fn spawn_contract_for_adapter(
    run_id: &str,
    lane_id: &str,
    adapter_id: &str,
) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: run_id.into(),
        lane_id: lane_id.into(),
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

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

struct DexterityLaunchProofFactory {
    ledger: LedgerBatcher,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for DexterityLaunchProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 56000 + request.instance_id.instance;
        let start = ProcessStart::new(
            proof_process_engine_kind(request),
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let mut owned_runtime =
            DexterityLaunchProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let model_id = owned_runtime
            .load(dexterity_load_spec_for_request(request))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned_runtime = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let shared_runtime =
            DexterityLaunchProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let teardown: handshake_core::swarm_orchestration::SessionTeardown = Arc::new(move || {
            let owned_runtime = Arc::clone(&owned_runtime);
            Box::pin(async move {
                owned_runtime
                    .lock()
                    .await
                    .unload(model_id)
                    .await
                    .map_err(|err| SwarmError::Internal(err.to_string()))
            })
        });
        Ok(LiveSession::new(
            Arc::new(shared_runtime),
            model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        ))
    }
}

fn proof_process_engine_kind(request: &SpawnRequest) -> ProcessEngineKind {
    match request.provider {
        Some(ProviderKind::OfficialCli) => ProcessEngineKind::OfficialCliBridge,
        Some(ProviderKind::ByokCloud) => ProcessEngineKind::HelperSubprocess,
        Some(ProviderKind::ExternalCompat) => ProcessEngineKind::ExternalCompat,
        Some(ProviderKind::Local) | None => match request.runtime_binding {
            RuntimeAdapterBinding::Candle => ProcessEngineKind::Candle,
            RuntimeAdapterBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
        },
    }
}

struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "CountingFactory must not be called by missing-store preflight".into(),
        ))
    }
}

struct DexterityLaunchProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
}

impl DexterityLaunchProofRuntime {
    fn new(loads: Arc<AtomicUsize>, unloads: Arc<AtomicUsize>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("dexterity-mt003-kv"),
            lora: LoraStackHandle::new("dexterity-mt003-lora"),
            steering: SteeringHookHandle::new("dexterity-mt003-steering"),
            loads,
            unloads,
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

fn dexterity_load_spec_for_request(request: &SpawnRequest) -> LoadSpec {
    LoadSpec {
        artifact_path: "mt003-proof-model.gguf".into(),
        sha256_expected: sample_sha256(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: request.provider.unwrap_or(ProviderKind::Local),
        engine_origin: Some("dexterity-mt003-proof-runtime".into()),
        external_engine_import: None,
    }
}
