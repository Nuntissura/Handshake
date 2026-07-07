//! WP-1 MT-010: Dexterity ModelLane backend navigation proof.
//!
//! These tests use real PostgreSQL plus kernel_event_ledger rows and the real
//! Axum router. The navigation surface is runtime code, not UserManual prose.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::api;
use handshake_core::swarm_orchestration::model_lane::{
    model_lane_context_bundle_id_for_handoff, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCrdtHandoffMetadata, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneHandoffSelectionState, ModelLaneHandoffSourceKind, ModelLaneKind, ModelLaneLeaseScope,
    ModelLaneLeaseState, ModelLaneLocusBinding, ModelLaneLoomHandoffRef, ModelLaneMessageKind,
    ModelLaneMtRuntimeStatus, ModelLaneNavigationProjection, ModelLaneProviderKind,
    ModelLaneRecoveryEventKind, ModelLaneRecoveryState, ModelLaneRecoveryStatus,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneContextBundleHandoff,
    NewModelLaneDiagnosticTierStatus, NewModelLaneLease, NewModelLaneMessage,
    NewModelLaneMtRuntimeStatus, NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent,
    NewModelLaneRun, RuntimeBinding,
};
use handshake_core::user_manual::registry::{wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::ensure_seeded;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::BTreeSet;
use user_manual_support::{app_state_for, start_server};

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-010";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const OWNER: &str = "KERNEL_BUILDER-MT010";
const RUN_ID: &str = "run-mt010-navigation";
const LANE_ID: &str = "lane-mt010-local";
const MESSAGE_ID: &str = "msg-mt010-local";
const LOOM_BLOCK_ID: &str = "loom-block-mt010-navigation-handoff";

struct NavigationFixture {
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
    store: ModelLaneStore,
}

#[tokio::test]
async fn model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery() {
    let fx = navigation_fixture().await;

    let run = fx
        .get_projection(&format!("/swarm/model-lanes/navigation/runs/{RUN_ID}"))
        .await;
    assert_projection_header(&run, "model_lane.navigation.run", "run", RUN_ID);
    assert_eq!(run.run.as_ref().expect("run").run_id, RUN_ID);
    assert_eq!(run.lanes.len(), 1);
    assert_eq!(run.messages.len(), 1);
    assert_eq!(run.artifacts.len(), 1);
    assert_eq!(run.recovery_events.len(), 1);
    assert_eq!(run.recovery_checkpoints.len(), 1);
    assert_eq!(run.leases.len(), 1);
    assert_eq!(run.diagnostic_tiers.len(), 3);
    assert_eq!(run.mt_runtime_statuses.len(), 1);
    assert!(run
        .manual_refs
        .iter()
        .any(|value| value.contains("model-lane-navigation")));
    assert!(run
        .event_ledger_refs
        .iter()
        .any(|value| value.starts_with("eventledger://kernel/KE-")));
    assert!(run
        .flight_recorder_refs
        .iter()
        .any(|value| value.contains("locus://wp1/mt010")));
    assert!(run
        .flight_recorder_refs
        .iter()
        .any(|value| value.contains("loom://mt010")));
    assert!(run
        .flight_recorder_refs
        .iter()
        .any(|value| value.contains("fems://mt010")));
    assert!(run
        .flight_recorder_refs
        .iter()
        .any(|value| value.contains("palmistry://wp1/model-lane/mt010")));
    assert!(run
        .recovery_routes
        .iter()
        .any(|value| value.contains("recover_run_after_restart")));
    let run_event_id = run.run.as_ref().expect("run").event_ledger_event_id.clone();
    let run_event_seq = run.run.as_ref().expect("run").event_ledger_seq.to_string();
    let memory_pack_ref = run.run.as_ref().expect("run").memory_pack_ref.clone();
    let memory_pack_hash = run.run.as_ref().expect("run").memory_pack_hash.clone();

    let lane = fx
        .get_projection(&format!("/swarm/model-lanes/navigation/lanes/{LANE_ID}"))
        .await;
    assert_projection_header(&lane, "model_lane.navigation.lane", "lane", LANE_ID);
    assert_eq!(lane.lanes[0].lane_id, LANE_ID);
    assert_eq!(lane.messages[0].message_id, MESSAGE_ID);

    let message = fx
        .get_projection(&format!(
            "/swarm/model-lanes/navigation/messages/{MESSAGE_ID}"
        ))
        .await;
    assert_projection_header(
        &message,
        "model_lane.navigation.message",
        "message",
        MESSAGE_ID,
    );
    assert_eq!(message.messages[0].message_id, MESSAGE_ID);
    assert_eq!(message.artifacts[0].artifact_ref, payload_ref(MESSAGE_ID));

    let artifact = fx
        .get_projection(&format!(
            "/swarm/model-lanes/navigation/artifacts?artifact_ref={}&context_bundle_id=ctx-{RUN_ID}",
            payload_ref(MESSAGE_ID)
        ))
        .await;
    assert_projection_header(
        &artifact,
        "model_lane.navigation.artifact_context",
        "artifact_context",
        &payload_ref(MESSAGE_ID),
    );
    assert_eq!(
        artifact.artifacts[0].artifact_binding_id,
        artifact_binding_id(MESSAGE_ID)
    );
    assert_eq!(artifact.messages[0].message_id, MESSAGE_ID);
    let artifact_aliases = vec![
        ("artifact_ref", artifact.artifacts[0].artifact_ref.clone()),
        (
            "artifact_binding_id",
            artifact.artifacts[0].artifact_binding_id.clone(),
        ),
        (
            "artifact_manifest_ref",
            artifact.artifacts[0].artifact_manifest_ref.clone(),
        ),
        (
            "artifact_payload_ref",
            artifact.artifacts[0].artifact_payload_ref.clone(),
        ),
        (
            "artifact_sha256",
            artifact.artifacts[0].artifact_sha256.clone(),
        ),
        ("content_hash", artifact.artifacts[0].content_hash.clone()),
    ];
    let artifact_content_hash = artifact.artifacts[0].content_hash.clone();
    for (selector, value) in artifact_aliases {
        let artifact_by_alias = fx
            .get_projection(&format!(
                "/swarm/model-lanes/navigation/artifacts?{selector}={value}&run_id={RUN_ID}"
            ))
            .await;
        assert_eq!(
            artifact_by_alias.artifacts[0].content_hash,
            artifact_content_hash
        );
        assert_eq!(artifact_by_alias.messages[0].message_id, MESSAGE_ID);
    }

    fx.expect_error(
        &format!(
            "/swarm/model-lanes/navigation/artifacts?artifact_ref={}&content_hash={}",
            payload_ref(MESSAGE_ID),
            sample_sha256()
        ),
        reqwest::StatusCode::BAD_REQUEST,
        "bad_request",
    )
    .await;

    fx.expect_error(
        &format!(
            "/swarm/model-lanes/navigation/artifacts?artifact_ref=artifact://missing/mt010&run_id={RUN_ID}"
        ),
        reqwest::StatusCode::NOT_FOUND,
        "not_found",
    )
    .await;

    let context = fx
        .get_projection(&format!(
            "/swarm/model-lanes/navigation/artifacts?context_bundle_id=ctx-{RUN_ID}"
        ))
        .await;
    assert_projection_header(
        &context,
        "model_lane.navigation.artifact_context",
        "artifact_context",
        &format!("ctx-{RUN_ID}"),
    );
    assert_eq!(context.run.as_ref().expect("context run").run_id, RUN_ID);

    let trace = fx
        .get_projection(&format!(
            "/swarm/model-lanes/navigation/traces/trace-{RUN_ID}?span_id=span-{MESSAGE_ID}"
        ))
        .await;
    assert_projection_header(
        &trace,
        "model_lane.navigation.trace_span",
        "trace_span",
        &format!("span-{MESSAGE_ID}"),
    );
    assert_eq!(trace.messages[0].message_id, MESSAGE_ID);

    let diagnostics = fx
        .get_projection(&format!(
            "/swarm/model-lanes/navigation/diagnostics/{RUN_ID}?behavior_id=HBR-INT-009&tier=flight_recorder&mt_id={MT_ID}"
        ))
        .await;
    assert_projection_header(
        &diagnostics,
        "model_lane.navigation.diagnostic_tier",
        "diagnostic_tier",
        "HBR-INT-009",
    );
    assert_eq!(diagnostics.diagnostic_tiers.len(), 1);
    assert_eq!(diagnostics.mt_runtime_statuses[0].micro_task_id, MT_ID);

    let recovery = fx
        .get_projection(&format!("/swarm/model-lanes/navigation/recovery/{RUN_ID}"))
        .await;
    assert_projection_header(
        &recovery,
        "model_lane.navigation.recovery",
        "recovery",
        RUN_ID,
    );
    assert_eq!(
        recovery.recovery_events[0].recovery_event_id,
        "recovery-event-mt010-001"
    );
    assert_eq!(
        recovery.recovery_checkpoints[0].checkpoint_id,
        "checkpoint-mt010"
    );

    for (key, value, expected_kind) in [
        (
            "model_session_id",
            format!("model-session-{LANE_ID}"),
            "model_session_id",
        ),
        ("session_id", format!("session-{LANE_ID}"), "session_id"),
        ("wp_id", WP_ID.to_string(), "wp_id"),
        ("mt_id", MT_ID.to_string(), "mt_id"),
        ("task_board_id", TASK_BOARD_ID.to_string(), "task_board_id"),
        (
            "locus_ref",
            format!("locus://wp1/mt010/{RUN_ID}/{LANE_ID}/{MESSAGE_ID}"),
            "locus_ref",
        ),
        (
            "loom_ref",
            format!("loom://mt010/{RUN_ID}/{MESSAGE_ID}"),
            "loom_ref",
        ),
        ("loom_block_id", LOOM_BLOCK_ID.to_string(), "loom_block_id"),
        (
            "fems_ref",
            format!("fems://mt010/{RUN_ID}/{MESSAGE_ID}"),
            "fems_ref",
        ),
        (
            "memory_pack_ref",
            memory_pack_ref.clone(),
            "memory_pack_ref",
        ),
        (
            "memory_pack_hash",
            memory_pack_hash.clone(),
            "memory_pack_hash",
        ),
        (
            "event_ledger_event_id",
            run_event_id,
            "event_ledger_event_id",
        ),
        ("event_ledger_seq", run_event_seq, "event_ledger_seq"),
        ("trace_id", format!("trace-{RUN_ID}"), "trace_id"),
        ("span_id", format!("span-{MESSAGE_ID}"), "span_id"),
        ("error_code", "CX-MM-010".to_string(), "error_code"),
    ] {
        let lookup = fx.get_lookup_projection(key, &value).await;
        assert_projection_header(
            &lookup,
            "model_lane.navigation.lookup",
            expected_kind,
            &value,
        );
        assert_eq!(lookup.run.as_ref().expect("lookup run").run_id, RUN_ID);
    }

    let bad = fx
        .http
        .get(format!("{}/swarm/model-lanes/navigation/lookup", fx.base))
        .send()
        .await
        .expect("bad lookup request");
    assert_eq!(bad.status(), reqwest::StatusCode::BAD_REQUEST);
    let bad_body: Value = bad.json().await.expect("bad lookup json");
    assert_eq!(bad_body["error"], "bad_request");

    let missing = fx
        .http
        .get(format!("{}/swarm/model-lanes/navigation/lookup", fx.base))
        .query(&[("run_id", "run-mt010-missing")])
        .send()
        .await
        .expect("missing lookup request");
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);
    let missing_body: Value = missing.json().await.expect("missing lookup json");
    assert_eq!(missing_body["error"], "not_found");

    seed_navigation_ambiguous_run(&fx.store).await;
    fx.expect_error(
        &format!("/swarm/model-lanes/navigation/artifacts?content_hash={artifact_content_hash}"),
        reqwest::StatusCode::CONFLICT,
        "ambiguous_lookup",
    )
    .await;
    for (key, value) in [
        ("artifact_ref", artifact_content_hash.as_str()),
        ("wp_id", WP_ID),
        ("mt_id", MT_ID),
        ("task_board_id", TASK_BOARD_ID),
        ("memory_pack_ref", memory_pack_ref.as_str()),
        ("memory_pack_hash", memory_pack_hash.as_str()),
        ("error_code", "CX-MM-010"),
    ] {
        fx.expect_lookup_error(
            key,
            value,
            reqwest::StatusCode::CONFLICT,
            "ambiguous_lookup",
        )
        .await;
    }
}

#[tokio::test]
async fn model_lane_navigation_user_manual_registry_rows_match_runtime_routes() {
    let fx = navigation_fixture().await;
    let surfaces: Vec<_> = wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::ModelLaneNavigation)
        .collect();
    assert_eq!(surfaces.len(), 8);

    let expected_ids: BTreeSet<_> = [
        "model_lane.navigation.run",
        "model_lane.navigation.lane",
        "model_lane.navigation.message",
        "model_lane.navigation.artifact_context",
        "model_lane.navigation.trace_span",
        "model_lane.navigation.diagnostic_tier",
        "model_lane.navigation.recovery",
        "model_lane.navigation.lookup",
    ]
    .into_iter()
    .collect();
    let actual_ids: BTreeSet<_> = surfaces.iter().map(|surface| surface.surface_id).collect();
    assert_eq!(actual_ids, expected_ids);

    for surface in surfaces {
        let path = seeded_navigation_path(surface.route);
        let response = fx
            .http
            .get(format!("{}{}", fx.base, path))
            .send()
            .await
            .unwrap_or_else(|err| panic!("probe {} failed: {err}", surface.route));
        assert!(
            response.status().is_success(),
            "registered navigation route {} must succeed against seeded runtime rows, got {}",
            surface.route,
            response.status()
        );
        let projection: ModelLaneNavigationProjection =
            response.json().await.expect("navigation projection");
        assert_eq!(projection.schema_id, "hsk.model_lane_navigation@1");
        assert_eq!(projection.route_id, surface.surface_id);
    }
}

impl NavigationFixture {
    async fn get_projection(&self, path: &str) -> ModelLaneNavigationProjection {
        let response = self
            .http
            .get(format!("{}{}", self.base, path))
            .send()
            .await
            .unwrap_or_else(|err| panic!("GET {path} failed: {err}"));
        assert!(
            response.status().is_success(),
            "GET {path} must succeed, got {}",
            response.status()
        );
        response.json().await.expect("navigation projection json")
    }

    async fn get_lookup_projection(&self, key: &str, value: &str) -> ModelLaneNavigationProjection {
        let response = self
            .http
            .get(format!("{}/swarm/model-lanes/navigation/lookup", self.base))
            .query(&[(key, value)])
            .send()
            .await
            .unwrap_or_else(|err| panic!("GET lookup {key} failed: {err}"));
        assert!(
            response.status().is_success(),
            "GET lookup {key}={value} must succeed, got {}",
            response.status()
        );
        response.json().await.expect("lookup projection json")
    }

    async fn expect_error(&self, path: &str, status: reqwest::StatusCode, error: &str) {
        let response = self
            .http
            .get(format!("{}{}", self.base, path))
            .send()
            .await
            .unwrap_or_else(|err| panic!("GET {path} failed: {err}"));
        assert_eq!(response.status(), status, "GET {path} status");
        let body: Value = response.json().await.expect("error body");
        assert_eq!(body["error"], error);
    }

    async fn expect_lookup_error(
        &self,
        key: &str,
        value: &str,
        status: reqwest::StatusCode,
        error: &str,
    ) {
        let response = self
            .http
            .get(format!("{}/swarm/model-lanes/navigation/lookup", self.base))
            .query(&[(key, value)])
            .send()
            .await
            .unwrap_or_else(|err| panic!("GET lookup {key} failed: {err}"));
        assert_eq!(response.status(), status, "lookup {key}={value} status");
        let body: Value = response.json().await.expect("lookup error body");
        assert_eq!(body["error"], error);
    }
}

async fn navigation_fixture() -> NavigationFixture {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "model_lane_navigation"
    );
    ensure_seeded(&kpg.db).await.expect("seed UserManual");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated navigation schema");
    let store = ModelLaneStore::new(pool.clone());
    seed_navigation_run(&pool, &store).await;
    let state = app_state_for(&kpg.schema_url).await;
    let (base, server) = start_server(api::routes(state)).await;
    NavigationFixture {
        base,
        _server: server,
        http: reqwest::Client::new(),
        store,
    }
}

async fn seed_navigation_run(pool: &PgPool, store: &ModelLaneStore) {
    store
        .record_run(sample_run(RUN_ID, vec![LANE_ID.to_owned()]))
        .await
        .expect("record navigation run");
    store
        .record_lane(sample_lane(LANE_ID, RUN_ID))
        .await
        .expect("record navigation lane");
    let message = sample_message(MESSAGE_ID, RUN_ID, LANE_ID, 1);
    store
        .record_message(message.clone())
        .await
        .expect("record navigation message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&message))
        .await
        .expect("record navigation artifact binding");
    store
        .record_context_bundle_handoff(sample_context_bundle_handoff_for_message(&message))
        .await
        .expect("record navigation context bundle handoff");
    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt010-001",
            RUN_ID,
            LANE_ID,
        ))
        .await
        .expect("record navigation recovery event");
    store
        .record_lane_lease(sample_lease("lease-mt010-local", RUN_ID, LANE_ID))
        .await
        .expect("record navigation lease");
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/mt010/navigation",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/navigation",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/model-lane/mt010",
        ),
    ] {
        store
            .record_diagnostic_tier_status(sample_tier(RUN_ID, tier, state, evidence))
            .await
            .expect("record navigation diagnostic tier");
    }
    store
        .record_mt_runtime_status(sample_mt_status(RUN_ID))
        .await
        .expect("record navigation MT status");
    let highwater = event_stream_high_watermark(pool, &event_stream_id(RUN_ID)).await;
    store
        .record_recovery_checkpoint(sample_checkpoint(highwater))
        .await
        .expect("record navigation checkpoint");
}

async fn seed_navigation_ambiguous_run(store: &ModelLaneStore) {
    let run_id = "run-mt010-navigation-ambiguous";
    let lane_id = "lane-mt010-ambiguous";
    let mut run = sample_run(run_id, vec![lane_id.to_owned()]);
    run.memory_pack_ref = format!("memory-pack://fems/mt010/{RUN_ID}");
    run.memory_pack_hash = sample_sha256();
    store
        .record_run(run)
        .await
        .expect("record ambiguous navigation run");
    store
        .record_lane(sample_lane(lane_id, run_id))
        .await
        .expect("record ambiguous navigation lane");
    let source_message = sample_message(MESSAGE_ID, RUN_ID, LANE_ID, 1);
    let mut shared_artifact = sample_artifact_binding_for_message(&source_message);
    shared_artifact.artifact_binding_id = artifact_binding_id("msg-mt010-ambiguous-shared");
    shared_artifact.run_id = run_id.into();
    shared_artifact.trace_id = format!("trace-{run_id}");
    shared_artifact.event_ledger_stream_id = event_stream_id(run_id);
    shared_artifact.idempotency_key = "idem-artifact-binding-msg-mt010-ambiguous-shared".into();
    store
        .record_context_bundle_artifact_binding(shared_artifact)
        .await
        .expect("record ambiguous navigation artifact binding");
    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt010-ambiguous",
            run_id,
            lane_id,
        ))
        .await
        .expect("record ambiguous navigation recovery event");
}

fn assert_projection_header(
    projection: &ModelLaneNavigationProjection,
    route_id: &str,
    lookup_kind: &str,
    lookup_ref: &str,
) {
    assert_eq!(projection.schema_id, "hsk.model_lane_navigation@1");
    assert_eq!(
        projection.surface_contract_id,
        "native_swarm_lane_diagnostics"
    );
    assert_eq!(projection.route_id, route_id);
    assert_eq!(projection.lookup_kind, lookup_kind);
    assert_eq!(projection.lookup_ref, lookup_ref);
    assert_eq!(projection.output_schema_ref, "hsk.model_lane_navigation@1");
}

fn seeded_navigation_path(route: &str) -> String {
    match route {
        "/swarm/model-lanes/navigation/runs/:run_id" => {
            format!("/swarm/model-lanes/navigation/runs/{RUN_ID}")
        }
        "/swarm/model-lanes/navigation/lanes/:lane_id" => {
            format!("/swarm/model-lanes/navigation/lanes/{LANE_ID}")
        }
        "/swarm/model-lanes/navigation/messages/:message_id" => {
            format!("/swarm/model-lanes/navigation/messages/{MESSAGE_ID}")
        }
        "/swarm/model-lanes/navigation/artifacts" => format!(
            "/swarm/model-lanes/navigation/artifacts?artifact_ref={}",
            payload_ref(MESSAGE_ID)
        ),
        "/swarm/model-lanes/navigation/traces/:trace_id" => {
            format!("/swarm/model-lanes/navigation/traces/trace-{RUN_ID}")
        }
        "/swarm/model-lanes/navigation/diagnostics/:run_id" => {
            format!("/swarm/model-lanes/navigation/diagnostics/{RUN_ID}?behavior_id=HBR-INT-009")
        }
        "/swarm/model-lanes/navigation/recovery/:run_id" => {
            format!("/swarm/model-lanes/navigation/recovery/{RUN_ID}")
        }
        "/swarm/model-lanes/navigation/lookup" => {
            format!("/swarm/model-lanes/navigation/lookup?run_id={RUN_ID}")
        }
        other => panic!("unexpected navigation route {other}"),
    }
}

async fn event_stream_high_watermark(pool: &PgPool, event_ledger_stream_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(MAX(event_sequence), 0) \
         FROM kernel_event_ledger \
         WHERE session_run_id = $1",
    )
    .bind(event_ledger_stream_id)
    .fetch_one(pool)
    .await
    .expect("query EventLedger stream high-watermark")
}

fn sample_run(run_id: &str, lane_ids: Vec<String>) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        routing_policy: "local_navigation_probe".into(),
        context_bundle_id: format!("ctx-{run_id}"),
        lane_ids,
        event_ledger_stream_id: event_stream_id(run_id),
        artifact_namespace: format!("artifact://model-lane/mt010/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-run-{run_id}"),
        replay_order_key: format!("00000000/{run_id}/run"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-navigation#recovery".into()),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("coordinator-{run_id}"),
            &format!("model-session-coordinator-{run_id}"),
        )),
        memory_pack_ref: format!("memory-pack://fems/mt010/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt010/navigation".into(),
        selected_model_id: Some("model://mt010/local/tiny".into()),
        candidate_model_ids: vec!["model://mt010/local/tiny".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(lane_id: &str, run_id: &str) -> NewModelLane {
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: event_stream_id(run_id),
        kind: ModelLaneKind::LocalModel,
        role: "local_navigation_probe".into(),
        backend: RuntimeBinding::Local.as_str().into(),
        model_id: Some("model://mt010/local/tiny".into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec![format!("capability://mt010/{lane_id}/read")],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://mt010/{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt010/{lane_id}")),
        provider_feature_profile_ref: Some(format!("provider-feature-profile://mt010/{lane_id}")),
        requested_execution_policy_ref: Some(format!(
            "execution-policy://requested/mt010/{lane_id}"
        )),
        effective_execution_policy_ref: Some(format!(
            "execution-policy://effective/mt010/{lane_id}"
        )),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://mt010/{lane_id}/allow")],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-07-01T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt010/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt010/navigation".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt010/navigation".into()),
        process_ownership_ref: Some(format!("process-ledger://mt010/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt010/navigation".into()),
        last_runtime_status_ref: Some(format!("runtime-status://mt010/{lane_id}/ready")),
        last_recovery_event_ref: Some(format!("recovery://mt010/{lane_id}/startable")),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-navigation#lane".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
    }
}

fn sample_message(
    message_id: &str,
    run_id: &str,
    lane_id: &str,
    replay_seq: i64,
) -> NewModelLaneMessage {
    let payload_ref = payload_ref(message_id);
    let crdt_update_ref = format!("crdt-update://mt010/{message_id}");
    let locus_ref = format!("locus://wp1/mt010/{run_id}/{lane_id}/{message_id}");
    let payload_json = artifact_payload_json_parts(
        message_id,
        run_id,
        &payload_ref,
        &crdt_update_ref,
        &locus_ref,
    );
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload_json));
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{lane_id}")],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-{run_id}"),
            correlation_id: format!("corr-{run_id}-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref,
        payload_sha256,
        event_ledger_stream_id: event_stream_id(run_id),
        summary: "MT-010 navigation payload".into(),
        authority: ModelLaneAuthority::PromotionCandidate,
        promotion_decision_id: Some(format!("promotion://mt010/{message_id}")),
        promotion_gate_ref: Some(format!("promotion-gate://mt010/{message_id}")),
        promotion_receipt_ref: Some(format!("promotion-receipt://mt010/{message_id}")),
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: Some(format!("artifact://promoted/mt010/{message_id}")),
        promoted_artifact_sha256: Some(sample_sha256()),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec![format!("toolgate://mt010/{lane_id}/allow")],
        coordinator_session_id: format!("coordinator-{run_id}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: format!("idem-message-{message_id}"),
        replay_order_key: format!("{replay_seq:08}/message/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt010/{message_id}")),
        crdt_update_ref: Some(crdt_update_ref),
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt010/base".into()),
        crdt_state_vector: Some("sv:mt010:1".into()),
        crdt_proposal_ref: Some(format!("crdt-proposal://mt010/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-navigation#message".into()),
        created_at_utc: "2026-07-01T00:00:00Z".into(),
        diagnostic_payload: json!({
            "artifact_ref": format!("artifact://model-lane/messages/{message_id}"),
            "crdt_update_id": format!("crdt-update-id://mt010/{message_id}"),
            "locus_ref": locus_ref,
            "loom_ref": format!("loom://mt010/{run_id}/{message_id}"),
            "fems_ref": format!("fems://mt010/{run_id}/{message_id}"),
            "flight_recorder": "kernel_event_ledger",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "palmistry://wp1/model-lane/mt010"
        }),
    }
}

fn sample_recovery_event(event_id: &str, run_id: &str, lane_id: &str) -> NewModelLaneRecoveryEvent {
    NewModelLaneRecoveryEvent {
        recovery_event_id: event_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        trace_id: format!("trace-{run_id}"),
        span_id: format!("span-{event_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{event_id}")],
        session_id: Some(format!("session-{lane_id}")),
        model_session_id: Some(format!("model-session-{lane_id}")),
        event_kind: ModelLaneRecoveryEventKind::CheckpointRestored,
        recovery_status: ModelLaneRecoveryStatus::Observed,
        replay_order_seq: 1,
        source_event_ledger_seq: None,
        payload_refs: vec![payload_ref(MESSAGE_ID)],
        artifact_refs: vec![payload_ref(MESSAGE_ID)],
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt010/base".into()),
        crdt_state_vector: Some("sv:mt010:1".into()),
        crdt_stale_base_ref: None,
        lease_id: None,
        failure_kind: None,
        error_code: Some("CX-MM-010".into()),
        replay_hint: "Use ModelLane navigation before trusting UI state".into(),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-recovery-{event_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-navigation#recovery-event".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_checkpoint(last_event_ledger_seq: i64) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: "checkpoint-mt010".into(),
        run_id: RUN_ID.into(),
        lane_id: Some(LANE_ID.into()),
        session_id: format!("session-{LANE_ID}"),
        model_session_id: format!("model-session-{LANE_ID}"),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq,
        last_message_id: Some(MESSAGE_ID.into()),
        open_payload_refs: vec![payload_ref(MESSAGE_ID)],
        lease_id: Some("lease-mt010-local".into()),
        idempotency_scope: "model-lane-navigation:run-mt010-navigation:checkpoint".into(),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some("recovery-event://recovery-event-mt010-001".into()),
        event_ledger_stream_id: event_stream_id(RUN_ID),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: "idem-checkpoint-mt010".into(),
        created_at_utc: "2026-07-01T00:00:01Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane-navigation#checkpoint".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_lease(lease_id: &str, run_id: &str, lane_id: &str) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{run_id}/{lane_id}"),
        holder_actor_id: "actor://kernel-builder/mt010".into(),
        holder_session_id: OWNER.into(),
        lease_expires_at_utc: "2099-01-01T00:00:00Z".into(),
        takeover_policy_ref: "lease-policy://mt010/recover-or-reclaim".into(),
        state: ModelLaneLeaseState::Active,
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-lease-{lease_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-navigation#lease".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics", "mt": MT_ID}),
    }
}

fn sample_tier(
    run_id: &str,
    tier: ModelLaneDiagnosticTier,
    state: ModelLaneDiagnosticTierState,
    evidence_ref: &str,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("MT-010 navigation diagnostic posture for {run_id}"),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("palmistry://wp1/model-lane/mt010".into()),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id, "mt": MT_ID}),
    }
}

fn sample_mt_status(run_id: &str) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: "mt-status-mt010-navigation".into(),
        run_id: run_id.into(),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        status: ModelLaneMtRuntimeStatus::ReadyForValidation,
        claimed_by_ref: Some(format!("session://{OWNER}")),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt010/model_lane_navigation_api_tests".into()),
        hbr_status_ref: Some("hbr-int-009://dexterity/navigation".into()),
        last_recovery_event_ref: Some("recovery-event://recovery-event-mt010-001".into()),
        last_runtime_status_ref: Some("runtime-status://mt010/ready-for-validation".into()),
        event_ledger_stream_id: event_stream_id(run_id),
        owner_session: OWNER.into(),
        idempotency_key: "idem-mt-status-mt010-navigation".into(),
        diagnostic_payload: json!({"navigation": true, "mt": MT_ID}),
    }
}

fn sample_artifact_binding_for_message(
    message: &NewModelLaneMessage,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: artifact_binding_id(&message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!(
            "artifact-store://model-lane/mt010/{}/artifact.json",
            message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json: artifact_payload_json_for_message(message),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-artifact-binding-{}", message.message_id),
        created_at_utc: "2026-07-01T00:00:02Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for MT-010 navigation",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "palmistry://wp1/model-lane/mt010"
        }),
    }
}

fn sample_context_bundle_handoff_for_message(
    message: &NewModelLaneMessage,
) -> NewModelLaneContextBundleHandoff {
    let mut handoff = NewModelLaneContextBundleHandoff {
        handoff_id: "handoff-mt010-navigation-loom".into(),
        context_bundle_id: "CTX-placeholder".into(),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        handoff_span_id: "span-handoff-mt010-navigation-loom".into(),
        parent_span_id: Some(message.message_span_id.clone()),
        linked_span_contexts: vec![message.message_span_id.clone()],
        downstream_lane_id: message.from_lane_id.clone(),
        source_lane_id: message.from_lane_id.clone(),
        source_message_id: message.message_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        source_kind: ModelLaneHandoffSourceKind::Proposal,
        authority_state: ModelLaneAuthority::PromotionCandidate,
        selection_state: ModelLaneHandoffSelectionState::Selected,
        reason_code: "reason://mt010/navigation/loom-block".into(),
        decision_ref: Some("context-decision://mt010/navigation/select-loom-block".into()),
        reviewer_ref: Some("validator://mt010/navigation/loom-block".into()),
        replay_hint: "replay://mt010/navigation/loom-block".into(),
        crdt_payload: Some(sample_crdt_handoff_metadata_for_message(message)),
        loom_refs: vec![ModelLaneLoomHandoffRef {
            workspace_id: "workspace-mt010-navigation".into(),
            block_id: LOOM_BLOCK_ID.into(),
            source_block_id: Some("loom-block-mt010-source".into()),
            target_block_id: Some("loom-block-mt010-target".into()),
            artifact_ref: Some(message.payload_ref.clone()),
            content_hash: sample_sha256(),
            version: "1".into(),
            event_ledger_evidence_ref: format!("eventledger://mt010/loom/{LOOM_BLOCK_ID}"),
            flight_recorder_evidence_ref: format!("flight-recorder://mt010/loom/{LOOM_BLOCK_ID}"),
        }],
        memory_pack_refs: Vec::new(),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: "idem-handoff-mt010-navigation-loom".into(),
        replay_order_key: "00000050/handoff-mt010-navigation-loom".into(),
        created_at_utc: "2026-07-01T00:00:03Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ContextBundle handoff EventLedger receipt required",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "palmistry://wp1/model-lane/mt010"
        }),
    };
    handoff.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&handoff).expect("derive ContextBundle id");
    handoff
}

fn sample_crdt_handoff_metadata_for_message(
    message: &NewModelLaneMessage,
) -> ModelLaneCrdtHandoffMetadata {
    ModelLaneCrdtHandoffMetadata {
        schema_id: "hsk.model_lane_crdt_payload@1".into(),
        document_id: "doc-mt010-navigation".into(),
        workspace_id: "workspace-mt010-navigation".into(),
        actor_id: "actor-lane-mt010-local".into(),
        actor_kind: "local_model".into(),
        lane_id: message.from_lane_id.clone(),
        crdt_site_id: "site-lane-mt010-local".into(),
        update_seq: 1,
        update_bytes_ref: message
            .crdt_update_ref
            .clone()
            .expect("sample message has CRDT update ref"),
        update_sha256: sample_sha256(),
        state_vector: message
            .crdt_state_vector
            .clone()
            .expect("sample message has CRDT state vector"),
        base_snapshot_ref: message
            .crdt_base_snapshot_ref
            .clone()
            .expect("sample message has CRDT base snapshot"),
        materialized_projection_hash: sample_sha256(),
        replay_metadata: json!({
            "format": "yjs_update_v1",
            "yjs_compatible": true,
            "flight_recorder": "eventledger://mt010/crdt/msg-mt010-local"
        }),
        promotion_gate_ref: message
            .promotion_gate_ref
            .clone()
            .expect("sample message has promotion gate ref"),
        promotion_receipt_ref: message.promotion_receipt_ref.clone(),
        validation_runner_ref: "validation-runner://mt010/navigation/crdt".into(),
        authority_effect: "advisory_only".into(),
    }
}

fn sample_locus(run_id: &str, session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: Some(TASK_BOARD_ID.into()),
        coordinator_session_id: format!("coordinator-{run_id}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: OWNER.into(),
        locus_binding_ref: format!("locus://wp1/mt010/{run_id}/{session_id}"),
    }
}

fn event_stream_id(run_id: &str) -> String {
    format!("mlane-stream-{run_id}")
}

fn payload_ref(message_id: &str) -> String {
    format!("artifact://model-lane/messages/{message_id}")
}

fn artifact_binding_id(message_id: &str) -> String {
    format!("artifact-binding-{message_id}")
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

fn artifact_payload_json_for_message(message: &NewModelLaneMessage) -> Value {
    artifact_payload_json_parts(
        &message.message_id,
        &message.run_id,
        &message.payload_ref,
        message.crdt_update_ref.as_deref().unwrap_or_default(),
        message
            .diagnostic_payload
            .get("locus_ref")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )
}

fn artifact_payload_json_parts(
    message_id: &str,
    run_id: &str,
    payload_ref: &str,
    crdt_update_ref: &str,
    locus_ref: &str,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "message_id": message_id,
        "run_id": run_id,
        "payload_ref": payload_ref,
        "crdt_update_ref": crdt_update_ref,
        "locus": locus_ref,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &Value) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    ch => output.push(ch),
                }
            }
            output.push('"');
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &Value::String((*key).clone()));
                output.push(':');
                if let Some(value) = map.get(*key) {
                    write_canonical_json(output, value);
                }
            }
            output.push('}');
        }
    }
}
