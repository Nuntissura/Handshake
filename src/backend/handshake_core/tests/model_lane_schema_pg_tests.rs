//! WP-1 MT-002: ModelLane schema/storage proof.
//!
//! These tests require real PostgreSQL through `knowledge_pg_support` and assert
//! lane records plus EventLedger rows. There is no SQLite, mock, or structs-only
//! fallback in this proof path.

mod knowledge_pg_support;
mod model_lane_cloud_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use futures::stream;
use handshake_core::kernel::{
    ArtifactRecord, ContextBundle, DummyEchoModelAdapter, KernelActor, KernelEventType,
    ModelAdapter, ModelAdapterRequest, ToolDecisionRecord,
};
use handshake_core::mcp::gate::{evaluate_kernel_tool_gate_decision, KernelMcpToolGateRequest};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCachePolicy, KvQuantSupport,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::swarm_orchestration::model_lane::{
    DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLaneMessageRecord, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore,
    ModelLaneTarget, NewModelLane, NewModelLaneMessage, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest,
    SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::json;

#[tokio::test]
async fn model_lane_schema_persists_and_replays_eventledger_rows() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-002 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane schema");
    let store = ModelLaneStore::new(pool.clone());

    let run = sample_run("run-mixed-001");
    let stored_run = store.record_run(run).await.expect("record lane run");
    assert!(
        stored_run.event_ledger_event_id.starts_with("KE-"),
        "run must carry EventLedger evidence"
    );
    assert!(stored_run.event_ledger_seq > 0);

    // Cloud lanes fail closed unless durable ProjectionPlan/ConsentReceipt
    // authority already exists (spec 4.3.9.2.5). Seed the cloud lane's authority
    // before recording it, matching the identity `sample_lane` stamps.
    seed_cloud_lane_authority_for(
        &store,
        "run-mixed-001",
        "lane-cloud",
        "mlane-stream-run-mixed-001",
    )
    .await;

    for lane in [
        sample_lane(
            "lane-local",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ),
        sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ),
        sample_lane(
            "lane-cli",
            ModelLaneKind::CliModel,
            RuntimeBinding::CliBridge,
            LaunchAuthority::CliBridge,
        ),
        sample_lane(
            "lane-human",
            ModelLaneKind::HumanOperator,
            RuntimeBinding::Human,
            LaunchAuthority::Operator,
        ),
        sample_lane(
            "lane-subagent",
            ModelLaneKind::Subagent,
            RuntimeBinding::Subagent,
            LaunchAuthority::SubagentManager,
        ),
        sample_lane(
            "lane-validator",
            ModelLaneKind::Validator,
            RuntimeBinding::Validator,
            LaunchAuthority::ValidatorRunner,
        ),
    ] {
        let stored = store.record_lane(lane).await.expect("record lane");
        assert!(stored.event_ledger_event_id.starts_with("KE-"));
        assert!(stored.event_ledger_seq > 0);
    }

    let message = sample_message(
        "msg-001",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    let stored_message = store
        .record_message(message)
        .await
        .expect("record model lane message");
    assert!(stored_message.event_ledger_event_id.starts_with("KE-"));
    assert!(stored_message.event_ledger_seq > stored_run.event_ledger_seq);

    let replay = store.replay_run("run-mixed-001").await.expect("replay run");
    assert_eq!(replay.run.run_id, "run-mixed-001");
    assert_eq!(replay.lanes.len(), 6);
    assert_eq!(replay.messages.len(), 1);
    assert_eq!(replay.messages[0].payload_sha256, sample_sha256());
    // Optional-reference round-trip through record_json is still proven, via a
    // reference that does not claim CRDT authority. An advisory ModelLane
    // message must replay with every CRDT authority field null; a non-null one
    // would have to dereference to real persisted Yjs bytes (proven against
    // real updates in mixed_model_lane_integration_pg_tests mt004_*/mt009_crdt_*
    // and model_lane_context_bundle_pg_tests), never to a synthetic string.
    assert_eq!(
        replay.messages[0].proposal_ref.as_deref(),
        Some("proposal://mt002/msg-001")
    );
    assert!(
        replay.messages[0].crdt_update_ref.is_none()
            && replay.messages[0].crdt_base_snapshot_ref.is_none()
            && replay.messages[0].crdt_state_vector.is_none()
            && replay.messages[0].crdt_proposal_ref.is_none()
            && replay.messages[0].crdt_stale_base_ref.is_none(),
        "an advisory ModelLane message must replay with null CRDT authority"
    );
    assert!(
        replay
            .messages
            .windows(2)
            .all(|pair| pair[0].event_ledger_seq <= pair[1].event_ledger_seq),
        "replay must be ordered by EventLedger sequence, not timestamps"
    );

    let ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type IN ('model_lane_run', 'model_lane', 'model_lane_message')",
    )
    .fetch_one(&pool)
    .await
    .expect("count EventLedger rows");
    assert_eq!(
        ledger_rows, 8,
        "run + six lanes + message must be event-backed"
    );
    let stream_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type IN ('model_lane_run', 'model_lane', 'model_lane_message')",
    )
    .bind("mlane-stream-run-mixed-001")
    .fetch_one(&pool)
    .await
    .expect("count EventLedger rows by declared ModelLane stream");
    assert_eq!(
        stream_rows, 8,
        "run, lane, and message events must bind to event_ledger_stream_id"
    );
    let stream_indexes: Vec<String> = sqlx::query_scalar(
        "SELECT indexname FROM pg_indexes \
         WHERE schemaname = current_schema() \
           AND indexname IN ( \
             'idx_model_lane_runs_stream_replay', \
             'idx_model_lanes_stream_replay', \
             'idx_model_lane_messages_stream_replay' \
           ) \
         ORDER BY indexname",
    )
    .fetch_all(&pool)
    .await
    .expect("list ModelLane stream replay indexes");
    assert_eq!(
        stream_indexes,
        vec![
            "idx_model_lane_messages_stream_replay".to_string(),
            "idx_model_lane_runs_stream_replay".to_string(),
            "idx_model_lanes_stream_replay".to_string(),
        ],
        "run, lane, and message tables must all have stream replay indexes"
    );

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    for schema_id in [
        "hsk.model_lane_run@1",
        "hsk.model_lane@1",
        "hsk.model_lane_message@1",
        "hsk.model_lane_terminal@1",
    ] {
        assert!(
            registry_rows.iter().any(|row| row.schema_id == schema_id),
            "missing schema registry row {schema_id}"
        );
    }

    let encoded = serde_json::to_value(&replay.messages[0]).expect("message serializes");
    let decoded: ModelLaneMessageRecord =
        serde_json::from_value(encoded).expect("message deserializes");
    assert_eq!(decoded.message_id, replay.messages[0].message_id);
}

#[tokio::test]
async fn dexterity_launch_records_real_swarm_spawn_session_runtime_path() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-003 runtime proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated Dexterity runtime schema");
    let store = ModelLaneStore::new(pool.clone());

    let bundle = ContextBundle::new(
        "KTR-MT003-RUNTIME",
        "SR-MT003-RUNTIME",
        json!({
            "visible_messages": [{"role": "user", "content": "prove Dexterity launch path"}],
            "tool_grants": ["read_trace"],
            "redactions": []
        }),
    )
    .expect("real ContextBundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo-mt003");
    let adapter_output = adapter
        .invoke(ModelAdapterRequest::new(
            bundle.clone(),
            KernelActor::ModelAdapter("dummy-echo-mt003".into()),
        ))
        .await
        .expect("real ModelAdapter invocation from ContextBundle");
    let tool_decision = ToolDecisionRecord::from_mcp_gate_decision(
        evaluate_kernel_tool_gate_decision(
            "dexterity-mt003-toolgate",
            ["read_trace".to_string()],
            KernelMcpToolGateRequest {
                tool_request_id: adapter_output.tool_request.tool_request_id.clone(),
                tool_id: adapter_output.tool_request.tool_id.clone(),
                reason: adapter_output.tool_request.reason.clone(),
            },
        ),
        KernelEventType::ToolDecisionRecorded,
    );
    let artifact_workspace = tempfile::tempdir().expect("ArtifactStore workspace");
    let artifact_record = ArtifactRecord::store_adapter_output(
        artifact_workspace.path(),
        "KTR-MT003-RUNTIME",
        "SR-MT003-RUNTIME",
        &adapter_output,
    )
    .expect("model output lands in ArtifactStore");

    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let unloads = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(DexterityProofFactory {
        ledger: ledger.clone(),
        unloads: unloads.clone(),
    });
    let sink = Arc::new(RecordingSwarmSink::new());
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        factory,
        sink,
        ledger,
        store.clone(),
    );
    let instance_id = ModelInstanceId::new(ModelId::new_v7(), 42);
    let request = SpawnRequest::new(
        instance_id,
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT003",
        "coordinator-session-mt003",
    )
    .with_wp("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    .with_mt("MT-003")
    .with_dexterity_launch(DexterityLaunchContract {
        run_id: "run-runtime-001".into(),
        lane_id: "lane-runtime-local".into(),
        restart_generation: 0,
        trace_id: "trace-mt003-runtime".into(),
        run_span_id: "span-runtime-run".into(),
        lane_span_id: "span-runtime-lane".into(),
        routing_policy: "single_local_runtime_spawn".into(),
        context_bundle_id: bundle.context_bundle_id.clone(),
        event_ledger_stream_id: "mlane-stream-runtime-001".into(),
        artifact_namespace: artifact_record.artifact_payload_ref.clone(),
        task_board_id: "task-board://wp-1".into(),
        locus_binding_ref: "locus://wp1/mt003/runtime-spawn".into(),
        role: "runtime-local-lane".into(),
        backend: "llama_cpp".into(),
        adapter_id: adapter.adapter_id().into(),
        capability_token_ids: vec!["capability://mt003/read-trace".into()],
        effective_capability_snapshot_ref: "capability-snapshot://mt003/runtime".into(),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://{}", tool_decision.tool_decision_id)],
        memory_pack_ref: "memory-pack://fems/mt003/runtime".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt003/runtime".into(),
        candidate_model_ids: vec!["model://mt003/runtime-local".into()],
        procedural_review_status: "toolgate_context_artifact_boundaries_exercised".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
        run_recovery_hint_ref: Some("usermanual://model-lane-schema#runtime-launch".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-schema#lane-recovery".into()),
    });

    let spawned = coordinator
        .spawn_session(request)
        .await
        .expect("Dexterity-recorded spawn_session succeeds");
    assert_eq!(spawned, instance_id);
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        0,
        "successful spawn remains live until drained"
    );

    let replay = store
        .replay_run("run-runtime-001")
        .await
        .expect("Dexterity replay from real spawn");
    assert_eq!(replay.run.context_bundle_id, bundle.context_bundle_id);
    assert_eq!(
        replay.run.artifact_namespace,
        artifact_record.artifact_payload_ref
    );
    assert_eq!(replay.run.memory_pack_hash, sample_sha256());
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].lane_id, "lane-runtime-local");
    assert_eq!(replay.lanes[0].runtime_binding, RuntimeBinding::Local);
    assert!(replay.lanes[0]
        .last_runtime_status_ref
        .as_deref()
        .expect("process ledger ref")
        .starts_with("process-ledger://"));
    assert_eq!(
        replay.lanes[0].tool_gate_decision_refs,
        vec![format!("toolgate://{}", tool_decision.tool_decision_id)]
    );
    let runtime_stream_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type IN ('model_lane_run', 'model_lane')",
    )
    .bind("mlane-stream-runtime-001")
    .fetch_one(&pool)
    .await
    .expect("count runtime EventLedger rows by declared ModelLane stream");
    assert_eq!(
        runtime_stream_rows, 2,
        "runtime run and lane events must bind to event_ledger_stream_id"
    );
    let coordinator_bound_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type IN ('model_lane_run', 'model_lane')",
    )
    .bind("coordinator-session-mt003")
    .fetch_one(&pool)
    .await
    .expect("count accidental coordinator-bound EventLedger rows");
    assert_eq!(
        coordinator_bound_rows, 0,
        "Dexterity EventLedger rows must not use coordinator_session_id as the stream"
    );

    coordinator
        .drain_all()
        .await
        .expect("drain Dexterity proof session");
    assert_eq!(
        unloads.load(Ordering::SeqCst),
        1,
        "runtime proof factory must unload the real LiveSession"
    );
}

#[tokio::test]
async fn model_lane_schema_serializes_competing_terminal_updates() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for terminal race proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated terminal race schema");
    let store = Arc::new(ModelLaneStore::new(pool.clone()));

    let mut run = sample_run("run-terminal-race");
    run.lane_ids = vec!["lane-terminal-race".into()];
    run.event_ledger_stream_id = "mlane-stream-terminal-race".into();
    run.artifact_namespace = "artifact://model-lane/run-terminal-race".into();
    let mut lane = sample_lane(
        "lane-terminal-race",
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    );
    lane.run_id = run.run_id.clone();
    lane.trace_id = run.trace_id.clone();
    lane.event_ledger_stream_id = run.event_ledger_stream_id.clone();
    lane.work_packet_id = run.work_packet_id.clone();
    lane.micro_task_id = run.micro_task_id.clone();
    lane.task_board_id = run.task_board_id.clone();
    lane.owner_session = run.owner_session.clone();

    store
        .record_prepared_launch((run, lane))
        .await
        .expect("record terminal race launch");

    let cancel_store = Arc::clone(&store);
    let fail_store = Arc::clone(&store);
    let (cancelled, failed) = tokio::join!(
        async move {
            cancel_store
                .record_lane_terminal_status(
                    "lane-terminal-race",
                    ModelLaneStatus::Cancelled,
                    "operator_cancelled_terminal_race",
                )
                .await
        },
        async move {
            fail_store
                .record_lane_terminal_status(
                    "lane-terminal-race",
                    ModelLaneStatus::Failed,
                    "runtime_failed_terminal_race",
                )
                .await
        }
    );

    let mut terminal_statuses = Vec::new();
    let mut conflict_count = 0usize;
    for result in [cancelled, failed] {
        match result {
            Ok(record) => terminal_statuses.push(record.status.clone()),
            Err(err) => {
                assert!(
                    err.to_string().contains("already terminal"),
                    "expected terminal idempotency conflict, got {err}"
                );
                conflict_count += 1;
            }
        }
    }
    assert_eq!(
        terminal_statuses.len(),
        1,
        "exactly one competing terminal update may win"
    );
    assert_eq!(
        conflict_count, 1,
        "exactly one competing terminal update must fail closed"
    );
    let winning_status = terminal_statuses[0].clone();
    assert!(
        winning_status == ModelLaneStatus::Cancelled || winning_status == ModelLaneStatus::Failed,
        "winning terminal status must be one submitted by the race"
    );

    let replay = store
        .replay_run("run-terminal-race")
        .await
        .expect("replay terminal race run");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, winning_status);
    if replay.lanes[0].status == ModelLaneStatus::Failed {
        assert_eq!(
            replay.lanes[0].startup_failure_ref.as_deref(),
            Some("terminal-failure://dexterity/lane-terminal-race")
        );
    }
    let terminal_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_terminal'",
    )
    .bind("mlane-stream-terminal-race")
    .fetch_one(&pool)
    .await
    .expect("count terminal EventLedger rows");
    assert_eq!(
        terminal_events, 1,
        "competing terminal updates must append one terminal EventLedger row"
    );

    let mut failed_run = sample_run("run-terminal-failed");
    failed_run.lane_ids = vec!["lane-terminal-failed".into()];
    failed_run.event_ledger_stream_id = "mlane-stream-terminal-failed".into();
    failed_run.artifact_namespace = "artifact://model-lane/run-terminal-failed".into();
    let mut failed_lane = sample_lane(
        "lane-terminal-failed",
        ModelLaneKind::CloudModel,
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
    );
    failed_lane.run_id = failed_run.run_id.clone();
    failed_lane.trace_id = failed_run.trace_id.clone();
    failed_lane.event_ledger_stream_id = failed_run.event_ledger_stream_id.clone();
    failed_lane.work_packet_id = failed_run.work_packet_id.clone();
    failed_lane.micro_task_id = failed_run.micro_task_id.clone();
    failed_lane.task_board_id = failed_run.task_board_id.clone();
    failed_lane.owner_session = failed_run.owner_session.clone();

    seed_cloud_lane_authority_for(
        &store,
        "run-terminal-failed",
        "lane-terminal-failed",
        "mlane-stream-terminal-failed",
    )
    .await;
    store
        .record_prepared_launch((failed_run, failed_lane))
        .await
        .expect("record deterministic failed terminal launch");
    let failed_terminal = store
        .record_lane_terminal_status(
            "lane-terminal-failed",
            ModelLaneStatus::Failed,
            "runtime_failed_terminal_ref",
        )
        .await
        .expect("record deterministic failed terminal status");
    assert_eq!(failed_terminal.status, ModelLaneStatus::Failed);
    assert_eq!(
        failed_terminal.startup_failure_ref.as_deref(),
        Some("terminal-failure://dexterity/lane-terminal-failed")
    );
    let failed_terminal_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_terminal' \
           AND event_type = 'SESSION_FAILED'",
    )
    .bind("mlane-stream-terminal-failed")
    .fetch_one(&pool)
    .await
    .expect("count deterministic failed terminal EventLedger rows");
    assert_eq!(
        failed_terminal_events, 1,
        "failed terminal update must append one failed terminal EventLedger row"
    );
}

#[tokio::test]
async fn model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-002 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane schema");
    let store = Arc::new(ModelLaneStore::new(pool.clone()));

    let mut missing_locus = sample_run("run-missing-locus");
    missing_locus.locus_binding = None;
    let err = store
        .record_run(missing_locus)
        .await
        .expect_err("missing Locus binding must fail closed");
    assert!(
        err.to_string().contains("locus"),
        "expected Locus validation error, got {err}"
    );

    let mut mismatched_locus = sample_run("run-mismatched-locus");
    mismatched_locus
        .locus_binding
        .as_mut()
        .expect("sample has locus")
        .work_packet_id = "WP-WRONG".into();
    let err = store
        .record_run(mismatched_locus)
        .await
        .expect_err("mismatched Locus WP must fail closed");
    assert!(
        err.to_string().contains("work_packet_id"),
        "expected Locus WP validation error, got {err}"
    );

    let err = store
        .record_prepared_launch((
            sample_run("run-prepared-mismatch"),
            sample_lane(
                "lane-prepared-mismatch",
                ModelLaneKind::LocalModel,
                RuntimeBinding::Local,
                LaunchAuthority::ModelRuntime,
            ),
        ))
        .await
        .expect_err("prepared launch must reject mismatched run/lane ids");
    assert!(
        err.to_string().contains("run_id"),
        "expected prepared pair run_id validation error, got {err}"
    );

    store
        .record_run(sample_run("run-mixed-001"))
        .await
        .expect("record valid run");
    let mut divergent_run_retry = sample_run("run-mixed-001");
    divergent_run_retry.idempotency_key = "idem-run-divergent".into();
    let err = store
        .record_run(divergent_run_retry)
        .await
        .expect_err("same run_id with divergent idempotency must fail closed");
    assert!(
        err.to_string().contains("idempotency"),
        "expected run idempotency conflict, got {err}"
    );
    store
        .record_lane(sample_lane(
            "lane-local",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record local lane");
    seed_cloud_lane_authority_for(
        &store,
        "run-mixed-001",
        "lane-cloud",
        "mlane-stream-run-mixed-001",
    )
    .await;
    store
        .record_lane(sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ))
        .await
        .expect("record cloud lane");

    let mut unsupported_provider = sample_lane(
        "lane-provider-other",
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    );
    unsupported_provider.provider_kind = ModelLaneProviderKind::Other;
    let err = store
        .record_lane(unsupported_provider)
        .await
        .expect_err("unsupported provider must fail closed");
    assert!(
        err.to_string().contains("provider_kind"),
        "expected provider validation error, got {err}"
    );

    let mut missing_capability_snapshot = sample_lane(
        "lane-missing-capability",
        ModelLaneKind::LocalModel,
        RuntimeBinding::Local,
        LaunchAuthority::ModelRuntime,
    );
    missing_capability_snapshot.effective_capability_snapshot_ref = None;
    let err = store
        .record_lane(missing_capability_snapshot)
        .await
        .expect_err("missing capability snapshot must fail closed");
    assert!(
        err.to_string()
            .contains("effective_capability_snapshot_ref"),
        "expected capability validation error, got {err}"
    );

    let mut missing_hash = sample_message(
        "msg-missing-hash",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    missing_hash.payload_sha256.clear();
    let err = store
        .record_message(missing_hash)
        .await
        .expect_err("missing payload hash must fail closed");
    assert!(
        err.to_string().contains("payload_sha256"),
        "expected payload hash validation error, got {err}"
    );

    let mut missing_crdt = crdt_posture_message(
        "msg-missing-crdt",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    missing_crdt.crdt_base_snapshot_ref = None;
    let err = store
        .record_message(missing_crdt)
        .await
        .expect_err("proposal missing CRDT base snapshot must fail closed");
    assert!(
        err.to_string().contains("crdt_base_snapshot_ref"),
        "expected CRDT validation error, got {err}"
    );

    let mut non_proposal_with_partial_crdt = crdt_posture_message(
        "msg-partial-crdt-status",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    non_proposal_with_partial_crdt.kind = ModelLaneMessageKind::Status;
    non_proposal_with_partial_crdt.crdt_base_snapshot_ref = None;
    let err = store
        .record_message(non_proposal_with_partial_crdt)
        .await
        .expect_err("every CRDT-targeting message kind must fail closed on partial metadata");
    assert!(
        err.to_string().contains("crdt_base_snapshot_ref"),
        "expected kind-independent CRDT validation error, got {err}"
    );

    // Partial-CRDT admission is refused by TWO independent fail-closed layers:
    // the synchronous completeness check in `validate_message_authority`
    // ("crdt_update_ref is required") and, for records that reach the durable
    // path, `validate_message_crdt_authority_tx` ("partial CRDT metadata cannot
    // be admitted without crdt_update_ref"). The synchronous layer legitimately
    // wins for `record_message` because it rejects before a transaction is even
    // opened. The durable layer is retained as defence in depth -- it still
    // guards stored-record revalidation, where a tampered row can present a
    // partial posture that never passed the synchronous check. The assertion is
    // therefore pinned to the invariant both layers share (the denial names the
    // missing `crdt_update_ref`) so it stays true regardless of which layer
    // fires first, instead of encoding one layer's wording.
    let mut proposal_ref_only = crdt_posture_message(
        "msg-crdt-proposal-ref-only",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    proposal_ref_only.crdt_update_ref = None;
    proposal_ref_only.crdt_base_snapshot_ref = None;
    proposal_ref_only.crdt_state_vector = None;
    proposal_ref_only.crdt_stale_base_ref = None;
    let err = store
        .record_message(proposal_ref_only)
        .await
        .expect_err("proposal-only CRDT metadata must not bypass authority resolution");
    assert!(
        err.to_string().contains("crdt_update_ref"),
        "expected proposal-only CRDT validation error naming crdt_update_ref, got {err}"
    );

    // Same two-layer contract as the proposal-only probe above: a lone
    // stale-base reference still declares CRDT authority and must be denied
    // with the missing `crdt_update_ref` named.
    let mut stale_ref_only = crdt_posture_message(
        "msg-crdt-stale-ref-only",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    stale_ref_only.crdt_update_ref = None;
    stale_ref_only.crdt_base_snapshot_ref = None;
    stale_ref_only.crdt_state_vector = None;
    stale_ref_only.crdt_proposal_ref = None;
    stale_ref_only.crdt_stale_base_ref = Some("crdt-stale-base://mt002/stale-only".into());
    let err = store
        .record_message(stale_ref_only)
        .await
        .expect_err("stale-only CRDT metadata must not bypass authority resolution");
    assert!(
        err.to_string().contains("crdt_update_ref"),
        "expected stale-only CRDT validation error naming crdt_update_ref, got {err}"
    );

    let mut advisory_proposal = sample_message(
        "msg-advisory-proposal-without-crdt",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    advisory_proposal.proposal_ref = None;
    advisory_proposal.crdt_update_ref = None;
    advisory_proposal.crdt_base_snapshot_ref = None;
    advisory_proposal.crdt_state_vector = None;
    advisory_proposal.crdt_proposal_ref = None;
    advisory_proposal.crdt_stale_base_ref = None;
    advisory_proposal.authority = ModelLaneAuthority::Advisory;
    advisory_proposal.idempotency_key = "idem-message-mt002-advisory-without-crdt".into();
    store
        .record_message(advisory_proposal)
        .await
        .expect("ordinary advisory Proposal without CRDT posture is valid");

    let mut malformed_trace = sample_message(
        "msg-malformed-trace",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    malformed_trace.parent_span_id = Some(malformed_trace.message_span_id.clone());
    let err = store
        .record_message(malformed_trace)
        .await
        .expect_err("malformed trace linkage must fail closed");
    assert!(
        err.to_string().contains("parent_span_id"),
        "expected span validation error, got {err}"
    );

    let first = sample_message(
        "msg-001",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    let first_left = first.clone();
    let first_right = first;
    let left_store = Arc::clone(&store);
    let right_store = Arc::clone(&store);
    let (left, right) = tokio::join!(
        async move { left_store.record_message(first_left).await },
        async move { right_store.record_message(first_right).await }
    );
    let left = left.expect("left concurrent message insert");
    let right = right.expect("right concurrent idempotent replay");
    assert_eq!(left.message_id, "msg-001");
    assert_eq!(right.message_id, "msg-001");
    assert_eq!(left.event_ledger_event_id, right.event_ledger_event_id);

    let message_ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_lane_message'",
    )
    .fetch_one(&pool)
    .await
    .expect("count message EventLedger rows after concurrent retry");
    assert_eq!(
        message_ledger_rows, 2,
        "the advisory proposal plus one concurrent same-key insert must produce exactly two EventLedger rows"
    );

    let retry = sample_message(
        "msg-retry",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    let replayed = store
        .record_message(retry)
        .await
        .expect("same idempotency key + same payload is idempotent");
    assert_eq!(replayed.message_id, "msg-001");

    let mut conflicting = sample_message(
        "msg-conflict",
        "lane-local",
        ModelLaneTarget::Lane("lane-cloud".into()),
    );
    conflicting.payload_sha256 =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into();
    let err = store
        .record_message(conflicting)
        .await
        .expect_err("same idempotency key + different payload must fail closed");
    assert!(
        err.to_string().contains("idempotency"),
        "expected idempotency conflict, got {err}"
    );
}

fn sample_run(run_id: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: "trace-mt002".into(),
        run_span_id: "span-run-mt002".into(),
        coordinator_session_id: "coordinator-session-mt002".into(),
        routing_policy: "mixed_local_cloud_subagent".into(),
        context_bundle_id: "ctx-bundle-mt002".into(),
        lane_ids: vec![
            "lane-local".into(),
            "lane-cloud".into(),
            "lane-cli".into(),
            "lane-human".into(),
            "lane-subagent".into(),
            "lane-validator".into(),
        ],
        event_ledger_stream_id: "mlane-stream-run-mixed-001".into(),
        artifact_namespace: "artifact://model-lane/run-mixed-001".into(),
        projection_plan_ref: Some("projection-plan://cloud/redacted-workspace".into()),
        consent_receipt_ref: Some("consent://operator/byok-cloud-001".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        idempotency_key: format!("idem-run-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#recovery".into()),
        locus_binding: Some(sample_locus()),
        memory_pack_ref: "memory-pack://fems/mt002/run".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt002/local-cloud-six-lanes".into(),
        selected_model_id: Some("model://mt002/deterministic-fake".into()),
        candidate_model_ids: vec![
            "model://mt002/deterministic-fake".into(),
            "model://mt002/cloud-critic".into(),
        ],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec!["rejection://mt002/no-unsupported-provider".into()],
    }
}

fn sample_lane(
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
) -> NewModelLane {
    let provider_kind = provider_kind_for(&runtime_binding);
    let projection_plan_ref = (runtime_binding == RuntimeBinding::Cloud)
        .then_some("projection-plan://cloud/redacted-workspace".into());
    let consent_receipt_ref = (runtime_binding == RuntimeBinding::Cloud)
        .then_some("consent://operator/byok-cloud-001".into());
    let session_id = format!("session-{lane_id}");
    let model_session_id = format!("model-session-{lane_id}");
    let process_backed = matches!(
        runtime_binding,
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
    );
    let provider_feature_profile_ref = format!(
        "provider-feature-profile://{}",
        provider_kind_for(&runtime_binding).as_str()
    );
    let requested_execution_policy_ref =
        format!("execution-policy://requested/{}", runtime_binding.as_str());
    let effective_execution_policy_ref =
        format!("execution-policy://effective/{}", launch_authority.as_str());
    let terminal_status_mapping_ref = format!(
        "terminal-status://session-broker/{}",
        runtime_binding.as_str()
    );
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: "run-mixed-001".into(),
        trace_id: "trace-mt002".into(),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: "mlane-stream-run-mixed-001".into(),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("{runtime_binding:?}").to_ascii_lowercase(),
        model_id: Some("model://mt002/deterministic-fake".into()),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding,
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt002/tool-read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt002".into()),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt002/{lane_id}")),
        provider_feature_profile_ref: Some(provider_feature_profile_ref),
        requested_execution_policy_ref: Some(requested_execution_policy_ref),
        effective_execution_policy_ref: Some(effective_execution_policy_ref),
        projection_plan_ref,
        consent_receipt_ref,
        tool_gate_decision_refs: vec!["toolgate://mt002/allow-read".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-28T22:30:00Z".into()),
        lease_expires_at_utc: Some("2026-06-28T22:40:00Z".into()),
        reclaim_after_utc: Some("2026-06-28T22:41:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt002/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt002/schema-proof".into()),
        terminal_status_mapping_ref: Some(terminal_status_mapping_ref),
        process_ownership_ref: process_backed
            .then_some(format!("process-ledger://mt002/{lane_id}")),
        no_os_process_reason_ref: (!process_backed)
            .then_some(format!("no-os-process://mt002/{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt002/bounded".into()),
        last_runtime_status_ref: Some("runtime-status://mt002/ready".into()),
        last_recovery_event_ref: Some("recovery://mt002/startable".into()),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#lane-recovery".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding: Some(sample_locus_for(&session_id, &model_session_id)),
    }
}

fn sample_message(
    message_id: &str,
    from_lane_id: &str,
    to_lane: ModelLaneTarget,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mixed-001".into(),
        trace_id: "trace-mt002".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some("span-lane-local".into()),
        linked_span_contexts: vec!["span-lane-cloud".into()],
        from_lane_id: from_lane_id.into(),
        to_lane,
        routing: Some(sample_routing(
            "corr-mt002-message",
            "coordinator",
            "coordinator-session-mt002",
        )),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: "artifact://model-lane/messages/msg-001".into(),
        payload_sha256: sample_sha256(),
        event_ledger_stream_id: "mlane-stream-run-mixed-001".into(),
        summary: "local lane proposes a typed patch for cloud critique".into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt002/allow-read".into()],
        coordinator_session_id: "coordinator-session-mt002".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding: Some(sample_locus()),
        idempotency_key: "idem-message-mt002-001".into(),
        replay_order_key: "00000002/message".into(),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some("proposal://mt002/msg-001".into()),
        // No CRDT posture by default. This MT-002 fixture proves ModelLane
        // schema persistence and EventLedger-ordered replay; it is not a CRDT
        // authority fixture. Ordinary advisory/routing messages carry null
        // crdt_* fields in production (routing_execution.rs ~1681-1756), and
        // since the MT-004/005 V5 remediation every non-null crdt_update_ref is
        // dereferenced against real kernel_crdt_updates bytes, so the previous
        // synthetic `crdt-update://mt002/msg-001` decoration is now correctly
        // denied at admission. Tests that need a CRDT posture build one
        // explicitly via `crdt_posture_message` below.
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#message-replay".into()),
        created_at_utc: "2026-06-28T22:31:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "WIRED",
            "internal_diagnostics": "DEFERRED: diagnostics surface MT-008",
            "palmistry": "DEFERRED: external watcher worktree"
        }),
    }
}

/// A message declaring a COMPLETE but synthetic CRDT posture.
///
/// Used only by validation negatives that must be rejected before any durable
/// authority resolution runs: `validate_message_authority` treats any single
/// `crdt_*` field as a CRDT authority declaration and then requires
/// `proposal_ref`, `crdt_update_ref`, `crdt_base_snapshot_ref`,
/// `crdt_state_vector` and `crdt_proposal_ref` to all be present. Because these
/// probes never reach persistence, synthetic refs are correct here; anything
/// that must be ADMITTED needs real persisted Yjs bytes instead.
fn crdt_posture_message(
    message_id: &str,
    from_lane_id: &str,
    to_lane: ModelLaneTarget,
) -> NewModelLaneMessage {
    let mut message = sample_message(message_id, from_lane_id, to_lane);
    message.crdt_update_ref = Some("crdt-update://mt002/msg-001".into());
    message.crdt_base_snapshot_ref = Some("crdt-snapshot://mt002/base".into());
    message.crdt_state_vector = Some("sv:1".into());
    message.crdt_proposal_ref = Some("crdt-proposal://mt002/msg-001".into());
    message
}

fn sample_locus() -> ModelLaneLocusBinding {
    sample_locus_for("session-lane-local", "model-session-lane-local")
}

fn sample_routing(
    correlation_id: &str,
    target_role: &str,
    target_session: &str,
) -> ModelLaneRoutingMetadata {
    ModelLaneRoutingMetadata {
        target_role: target_role.into(),
        target_session: target_session.into(),
        correlation_id: correlation_id.into(),
        requires_ack: true,
        ack_for: None,
    }
}

fn sample_locus_for(session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-002".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: "coordinator-session-mt002".into(),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding_ref: "locus://wp1/mt002/coordinator-session-mt002".into(),
    }
}

/// Persist the durable cloud ProjectionPlan/ConsentReceipt authority a cloud
/// lane built by `sample_lane(.., RuntimeBinding::Cloud, ..)` needs before it
/// can be recorded. Mirrors the identity fields that `sample_lane` stamps onto
/// a cloud lane so the fail-closed durability gate (spec 4.3.9.2.5) accepts the
/// lane without weakening the gate.
async fn seed_cloud_lane_authority_for(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    event_ledger_stream_id: &str,
) {
    model_lane_cloud_support::seed_cloud_lane_authority(
        store,
        model_lane_cloud_support::CloudLaneAuthoritySpec {
            run_id,
            lane_id,
            model_session_id: &format!("model-session-{lane_id}"),
            provider_kind: provider_kind_for(&RuntimeBinding::Cloud).as_str(),
            requested_model_id: "model://mt002/deterministic-fake",
            projection_plan_id: "projection-plan://cloud/redacted-workspace",
            consent_receipt_id: "consent://operator/byok-cloud-001",
            event_ledger_stream_id,
            work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            micro_task_id: "MT-002",
            task_board_id: "task-board://wp-1",
            owner_session: "KERNEL_BUILDER-20260628-220906",
        },
    )
    .await;
}

fn provider_kind_for(runtime_binding: &RuntimeBinding) -> ModelLaneProviderKind {
    match runtime_binding {
        RuntimeBinding::Local => ModelLaneProviderKind::LocalRuntime,
        RuntimeBinding::Cloud => ModelLaneProviderKind::OpenAi,
        RuntimeBinding::CliBridge => ModelLaneProviderKind::OfficialCli,
        RuntimeBinding::Human => ModelLaneProviderKind::Human,
        RuntimeBinding::Subagent => ModelLaneProviderKind::Subagent,
        RuntimeBinding::Validator => ModelLaneProviderKind::Validator,
    }
}

struct DexterityProofFactory {
    ledger: LedgerBatcher,
    unloads: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for DexterityProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 55000 + request.instance_id.instance;
        let start = ProcessStart::new(
            ProcessEngineKind::LlamaCpp,
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone());
        self.ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let mut owned_runtime = DexterityProofRuntime::new(self.unloads.clone());
        let model_id = owned_runtime
            .load(dexterity_load_spec())
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned_runtime = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let shared_runtime = DexterityProofRuntime::new(self.unloads.clone());
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

struct DexterityProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    unloads: Arc<AtomicUsize>,
}

impl DexterityProofRuntime {
    fn new(unloads: Arc<AtomicUsize>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("dexterity-mt003-kv"),
            lora: LoraStackHandle::new("dexterity-mt003-lora"),
            steering: SteeringHookHandle::new("dexterity-mt003-steering"),
            unloads,
        }
    }
}

#[async_trait]
impl ModelRuntime for DexterityProofRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
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
                    text: format!("dexterity-token-{i}"),
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

fn dexterity_load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: std::path::PathBuf::from("dexterity-mt003-artifact"),
        sha256_expected: sample_sha256(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn sample_sha256() -> String {
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
}
