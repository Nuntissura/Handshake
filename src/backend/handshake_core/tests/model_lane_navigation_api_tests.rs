//! Restores the pinned WP-1 MT-010 selectors over the current authenticated embedded store.
//! The original @1 wire assertions remain active and conflict with the current @2 projection.

mod account_session_support;
mod surreal_test_store_support;
#[allow(dead_code)]
mod user_manual_support;

use axum::Extension;
use handshake_core::api;
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::storage::surreal::SurrealStorage;
use handshake_core::swarm_orchestration::model_lane::*;
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::SurrealAdmissibleCrdtPosture;
use handshake_core::user_manual::registry::{wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::ensure_seeded;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;
use surreal_test_store_support::IsolatedSurrealTestStore;
use user_manual_support::{app_state_for, start_server};

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-010";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const LABEL: &str = "mt010-navigation";
const OWNER: &str = "owner-mt018-mt010-navigation";
const RUN_ID: &str = "run-mt018-mt010-navigation";
const LANE_ID: &str = "lane-mt018-mt010-navigation";
const MESSAGE_ID: &str = "message-mt018-mt010-navigation";
const LOOM_BLOCK_ID: &str = "loom-block-mt010-navigation-handoff";

#[derive(serde::Deserialize)]
pub struct PinnedNavigationProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub route_id: String,
    pub lookup_kind: String,
    pub lookup_ref: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub manual_refs: Vec<String>,
    pub run: Option<ModelLaneRunRecord>,
    pub lanes: Vec<ModelLaneRecord>,
    pub messages: Vec<ModelLaneMessageRecord>,
    pub artifacts: Vec<ModelLaneContextBundleArtifactBindingRecord>,
    pub context_handoffs: Vec<ModelLaneContextBundleHandoffRecord>,
    pub recovery_checkpoints: Vec<ModelLaneRecoveryCheckpointRecord>,
    pub recovery_events: Vec<ModelLaneRecoveryEventRecord>,
    pub leases: Vec<ModelLaneLeaseRecord>,
    pub diagnostic_tiers: Vec<ModelLaneDiagnosticTierStatusRecord>,
    pub mt_runtime_statuses: Vec<ModelLaneMtRuntimeStatusRecord>,
    pub event_ledger_refs: Vec<String>,
    pub flight_recorder_refs: Vec<String>,
    pub error_codes: Vec<String>,
    pub recovery_routes: Vec<String>,
}

#[tokio::test]
async fn model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery() {
    let fx = navigation_fixture().await;
    let context_bundle_id = &fx.context_bundle_id;

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
            "/swarm/model-lanes/navigation/artifacts?artifact_ref={}&context_bundle_id={context_bundle_id}",
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
            "/swarm/model-lanes/navigation/artifacts?context_bundle_id={context_bundle_id}"
        ))
        .await;
    assert_projection_header(
        &context,
        "model_lane.navigation.artifact_context",
        "artifact_context",
        &format!("{context_bundle_id}"),
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

    seed_navigation_ambiguous_run(&fx).await;
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
        let projection: PinnedNavigationProjection =
            response.json().await.expect("navigation projection");
        assert_eq!(projection.schema_id, "hsk.model_lane_navigation@1");
        assert_eq!(projection.route_id, surface.surface_id);
    }
}

impl NavigationFixture {
    async fn get_projection(&self, path: &str) -> PinnedNavigationProjection {
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

    async fn get_lookup_projection(&self, key: &str, value: &str) -> PinnedNavigationProjection {
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

struct NavigationFixture {
    base: String,
    server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
    canonical: CanonicalNavigationFixture,
    source: ModelLaneMessageRecord,
    context_bundle_id: String,
    _isolated: IsolatedSurrealTestStore,
}

impl Drop for NavigationFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn navigation_fixture() -> NavigationFixture {
    let isolated = IsolatedSurrealTestStore::create()
        .await
        .expect("isolated original navigation proof");
    let storage = isolated.storage().clone();
    let canonical = CanonicalNavigationFixture::create(&storage, LABEL).await;
    let database = handshake_core::storage::surreal::SurrealDatabase::new(storage.clone());
    ensure_seeded(&database)
        .await
        .expect("seed canonical current UserManual");
    let posture = canonical.posture(LABEL).await;
    let source = canonical
        .create
        .run(canonical.store.record_message(posture.message.clone()))
        .await
        .expect("record admitted canonical CRDT source");
    canonical.context(LABEL, &posture, &source).await;
    let context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&navigation_handoff_draft(LABEL))
            .expect("canonical context identity");
    canonical
        .create
        .run(canonical.store.record_recovery_event(sample_recovery_event(
            "recovery-event-mt010-001",
            RUN_ID,
            LANE_ID,
        )))
        .await
        .expect("record original recovery event");
    canonical
        .create
        .run(
            canonical
                .store
                .record_lane_lease(sample_lease("lease-mt010-local", RUN_ID, LANE_ID)),
        )
        .await
        .expect("record original lane lease");
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
        canonical
            .create
            .run(
                canonical
                    .store
                    .record_diagnostic_tier_status(sample_tier(RUN_ID, tier, state, evidence)),
            )
            .await
            .expect("record original diagnostic tier");
    }
    canonical
        .create
        .run(
            canonical
                .store
                .record_mt_runtime_status(sample_mt_status(RUN_ID)),
        )
        .await
        .expect("record original task status");
    canonical
        .create
        .run(
            canonical
                .store
                .record_recovery_checkpoint(sample_checkpoint(source.event_ledger_seq)),
        )
        .await
        .expect("record original checkpoint");
    let exact = handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(&canonical.scope)
        .expect("actual authenticated exact tuple");
    let state = app_state_for(&database).await;
    let (base, server) = start_server(
        api::model_lane_navigation::routes(state)
            .layer(Extension(
                ProductLocalResourceScope::from_exact(exact).expect("actual native scope"),
            ))
            .layer(Extension(canonical.lifecycle.clone())),
    )
    .await;
    let http = canonical.account.client();
    NavigationFixture {
        base,
        server,
        http,
        canonical,
        source,
        context_bundle_id,
        _isolated: isolated,
    }
}

async fn seed_navigation_ambiguous_run(fixture: &NavigationFixture) {
    let canonical = &fixture.canonical;
    let posture = canonical.posture("mt010-ambiguous").await;
    let run_id = posture.run_id.as_str();
    let lane_id = posture.lane_id.as_str();
    canonical
        .create
        .run(canonical.store.record_message(posture.message.clone()))
        .await
        .expect("record admitted ambiguous CRDT source");
    let shared = NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: artifact_binding_id("msg-mt010-ambiguous-shared"),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        artifact_ref: fixture.source.payload_ref.clone(),
        artifact_sha256: fixture.source.payload_sha256.clone(),
        content_hash: fixture.source.payload_sha256.clone(),
        payload_json: json!({"schema_id":"hsk.model_lane_context_payload@1", "message_id": fixture.source.message_id, "summary": fixture.source.summary}),
        artifact_manifest_ref: format!(
            "artifact-manifest://navigation/{}",
            fixture.source.message_id
        ),
        artifact_payload_ref: fixture.source.payload_ref.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        event_ledger_stream_id: event_stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: "idem-artifact-binding-msg-mt010-ambiguous-shared".into(),
        created_at_utc: "2026-09-02T00:03:00Z".into(),
        diagnostic_payload: json!({}),
    };
    canonical
        .create
        .run(
            canonical
                .store
                .record_context_bundle_artifact_binding(shared),
        )
        .await
        .expect("record actual duplicate content binding");
    canonical
        .create
        .run(canonical.store.record_recovery_event(sample_recovery_event(
            "recovery-event-mt010-ambiguous",
            run_id,
            lane_id,
        )))
        .await
        .expect("record original ambiguous event");
}

fn event_stream_id(run_id: &str) -> String {
    let label = run_id
        .strip_prefix("run-mt018-")
        .expect("canonical admitted run ID");
    format!("model-lane://mt018/{label}")
}
fn payload_ref(message_id: &str) -> String {
    let label = message_id
        .strip_prefix("message-mt018-")
        .expect("canonical admitted message ID");
    format!("artifact://mt018/{label}/proposal")
}
fn artifact_binding_id(message_id: &str) -> String {
    format!("navigation-artifact-{message_id}")
}

fn assert_projection_header(
    projection: &PinnedNavigationProjection,
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
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
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

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

use handshake_core::storage::surreal::bootstrap_production_schemas;
use handshake_core::storage::surreal::resource_authority::{
    AuthorizationRequest, ResourceAction, ResourceGrantSpec, ResourceKind,
};
use handshake_core::swarm_orchestration::model_lane as lane;
use handshake_core::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry;
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture_authenticated_with_context,
    AuthenticatedModelLaneTestOperation,
};

struct CanonicalNavigationFixture {
    account: account_session_support::AccountFixture,
    scope: ResourceScope,
    lifecycle: ResourceAccessLifecycleRegistry,
    store: ModelLaneStore,
    create: AuthenticatedModelLaneTestOperation,
    read: AuthenticatedModelLaneTestOperation,
    nav: AuthenticatedModelLaneTestOperation,
    document_update: AuthenticatedModelLaneTestOperation,
    document_id: String,
    model_id: String,
    catalog: std::sync::Arc<handshake_core::model_runtime::ModelCatalog>,
}
impl CanonicalNavigationFixture {
    async fn create(storage: &SurrealStorage, label: &str) -> Self {
        bootstrap_production_schemas(storage)
            .await
            .expect("bootstrap canonical navigation producers");
        let mut account = account_session_support::AccountFixture::install(storage).await;
        account.owner = account_session_support::OwnerSession::provision_with_capabilities(
            storage,
            account.binding_token(),
            &["model_lane.navigation.read"],
        )
        .await;
        let state = app_state_for(&handshake_core::storage::surreal::SurrealDatabase::new(
            storage.clone(),
        ))
        .await;
        let workspace_id = account.create_workspace(&state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("owned document fixture listener");
        let address = listener.local_addr().expect("document fixture address");
        let app = handshake_core::api::knowledge_documents::routes(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("owned document fixture server");
        });
        let response = account
            .apply(reqwest::Client::new().post(format!("http://{address}/knowledge/documents")))
            .header("x-hsk-actor-id", &account.actor_id)
            .header("x-hsk-actor-kind", "operator")
            .header(
                "x-hsk-kernel-task-run-id",
                format!("navigation-fixture-{label}"),
            )
            .header("x-hsk-session-run-id", &account.session_id)
            .json(
                &json!({"workspace_id": workspace_id, "title": format!("Navigation {label}"),
                "content_json": {"type": "doc", "content": []}}),
            )
            .send()
            .await
            .expect("real authenticated document producer");
        let status = response.status();
        let body: serde_json::Value = response.json().await.expect("canonical document response");
        server.abort();
        let _ = server.await;
        assert_eq!(
            status,
            reqwest::StatusCode::CREATED,
            "document producer: {body}"
        );
        let document_id = body["document"]["rich_document_id"]
            .as_str()
            .expect("actual document ID")
            .to_owned();
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(
                uuid::Uuid::parse_str(&account.account_id).expect("actual account"),
            ),
            ActorPrincipalId::from_uuid(
                uuid::Uuid::parse_str(&account.principal_id).expect("actual principal"),
            ),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(
            uuid::Uuid::parse_str(&account.session_id).expect("actual session"),
        ))
        .with_access_space(AccessSpaceRef::from_uuid(
            uuid::Uuid::parse_str(&account.access_space_id).expect("actual Space"),
        ))
        .with_workspace(WorkspaceScopeRef::new(workspace_id.clone()).expect("actual workspace"));
        let exact = handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("actual exact tuple");
        let lifecycle = storage.resource_access_lifecycle();
        lifecycle
            .register_active(exact.clone())
            .expect("register actual fixture lifecycle");
        let request = navigation_request(
            &account,
            ResourceKind::Workspace,
            &workspace_id,
            ResourceAction::Read,
            "fs.read",
        );
        let actual = storage
            .authorize_protected_resource(request)
            .await
            .expect("resolve canonical owning workspace grant");
        storage
            .grant_resource(
                &account.account_id,
                &account.access_space_id,
                ResourceGrantSpec {
                    principal_id: account.principal_id.clone(),
                    resource_id: actual.resource_id,
                    actions: vec![ResourceAction::Read],
                    capability_ids: vec!["model_lane.navigation.read".into()],
                    expires_at: None,
                    delegation_chain: actual.delegation_chain,
                },
            )
            .await
            .expect("canonical exact navigation grant producer");
        // Component catalog input only: no artifact loading, readiness, or model invocation proof.
        let model = handshake_core::model_runtime::ModelId::new_v7();
        let model_id = model.to_string();
        let mut registry = handshake_core::model_runtime::ModelRegistry::default();
        registry
            .register(handshake_core::model_runtime::ModelRegistration {
                model_id: model.clone(),
                artifact_path: "models/navigation-component-fixture.gguf".into(),
                sha256: [0x5a; 32],
                runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
                declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
                base_model_tag: handshake_core::model_runtime::BaseModelTag::new(
                    "navigation-component-fixture",
                ),
                registered_at_utc: chrono::Utc::now(),
                registered_by: handshake_core::model_runtime::OperatorId::new(
                    "navigation-component-fixture",
                ),
                provider: handshake_core::model_runtime::ProviderKind::Local,
            })
            .expect("register component catalog identity");
        registry
            .mark_loaded(model)
            .expect("mark component catalog identity loaded");
        let catalog =
            handshake_core::model_runtime::ModelCatalog::from_registry(Arc::new(registry));

        let store = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            scope.clone(),
            lifecycle.clone(),
        )
        .with_model_catalog(catalog.clone());
        let create = navigation_operation(
            storage,
            &account,
            &scope,
            ResourceKind::Workspace,
            &workspace_id,
            ResourceAction::Create,
            "fs.write",
        )
        .await;
        let read = navigation_operation(
            storage,
            &account,
            &scope,
            ResourceKind::Workspace,
            &workspace_id,
            ResourceAction::Read,
            "fs.read",
        )
        .await;
        let nav = navigation_operation(
            storage,
            &account,
            &scope,
            ResourceKind::Workspace,
            &workspace_id,
            ResourceAction::Read,
            "model_lane.navigation.read",
        )
        .await;
        let document_update = navigation_operation(
            storage,
            &account,
            &scope,
            ResourceKind::RichDocument,
            &document_id,
            ResourceAction::Update,
            "fs.write",
        )
        .await;
        bootstrap_production_schemas(storage)
            .await
            .expect("administratively reconcile exact actual fixture metadata");
        Self {
            account,
            scope,
            lifecycle,
            store,
            create,
            read,
            nav,
            document_update,
            document_id,
            model_id,
            catalog,
        }
    }
    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        // Draft producer input: only its known canonical run/locus/session identity
        // participates in the production context-ID derivation before record_run.
        let draft = navigation_handoff_draft(label);
        let context_id = lane::model_lane_context_bundle_id_for_handoff(&draft)
            .expect("production context identity before run creation");
        let mut posture = build_surreal_admissible_crdt_posture_authenticated_with_context(
            &self.store,
            self.scope.workspace.as_ref().expect("workspace").as_str(),
            label,
            &self.model_id,
            &self.document_id,
            &self.create,
            &self.document_update,
            &context_id,
        )
        .await
        .expect("actual authenticated CRDT producer chain");
        use sha2::{Digest, Sha256};
        let payload = json!({"schema_id": "hsk.model_lane_context_payload@1", "message_id": posture.message.message_id, "summary": posture.message.summary});
        posture.message.payload_sha256 = hex::encode(Sha256::digest(
            handshake_storage_support::canonical_json::canonical_json_bytes(&payload),
        ));
        posture
    }
    async fn context(
        &self,
        label: &str,
        posture: &SurrealAdmissibleCrdtPosture,
        source: &lane::ModelLaneMessageRecord,
    ) {
        let artifact = lane::NewModelLaneContextBundleArtifactBinding {
            artifact_binding_id: format!("navigation-artifact-{}", source.message_id),
            run_id: source.run_id.clone(),
            trace_id: source.trace_id.clone(),
            artifact_ref: source.payload_ref.clone(),
            artifact_sha256: source.payload_sha256.clone(),
            content_hash: source.payload_sha256.clone(),
            artifact_kind: "model_lane_message_payload".into(),
            artifact_manifest_ref: format!("artifact-manifest://{}", source.message_id),
            artifact_payload_ref: source.payload_ref.clone(),
            payload_json: json!({"schema_id":"hsk.model_lane_context_payload@1", "message_id":source.message_id,"summary":source.summary}),
            event_ledger_stream_id: source.event_ledger_stream_id.clone(),
            work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
            micro_task_id: "MT-018".into(),
            task_board_id: "task-board://wp-1".into(),
            owner_session: source.owner_session.clone(),
            idempotency_key: format!("navigation-artifact-{}", source.message_id),
            created_at_utc: "2026-09-02T00:03:00Z".into(),
            diagnostic_payload: json!({}),
        };
        self.create
            .run(self.store.record_context_bundle_artifact_binding(artifact))
            .await
            .expect("canonical artifact producer");
        let binding = source
            .crdt_authority_binding
            .as_ref()
            .expect("canonical CRDT message binding");
        let mut input = navigation_handoff_draft(label);
        input.context_bundle_id = lane::model_lane_context_bundle_id_for_handoff(&input)
            .expect("independent actual context identity");
        input.source_message_id = source.message_id.clone();
        input.source_lane_id = source.from_lane_id.clone();
        input.artifact_ref = source.payload_ref.clone();
        input.artifact_sha256 = source.payload_sha256.clone();
        input.content_hash = source.payload_sha256.clone();
        input.parent_span_id = Some(source.message_span_id.clone());
        input.linked_span_contexts = vec![source.trace_id.clone()];
        input.crdt_payload = Some(lane::ModelLaneCrdtHandoffMetadata {
            schema_id: "hsk.model_lane_crdt_payload@1".into(),
            document_id: binding.document_id.clone(),
            workspace_id: binding.workspace_id.clone(),
            actor_id: binding.actor_id.clone(),
            actor_kind: binding.actor_kind.clone(),
            lane_id: binding.lane_id.clone(),
            crdt_site_id: binding.crdt_site_id.clone(),
            update_seq: binding.update_seq,
            update_bytes_ref: binding.update_bytes_ref.clone(),
            update_sha256: posture.update.update_sha256.clone(),
            state_vector: binding.state_vector.clone(),
            base_snapshot_ref: binding.base_snapshot_ref.clone(),
            materialized_projection_hash: binding.materialized_projection_hash.clone(),
            replay_metadata: json!({"format":"yjs_update_v1", "yjs_compatible":true,"replay_order_key":posture.update.replay_order_key,
                "dependency_update_ids":posture.update.dependency_update_ids,"schema_version":posture.update.replay_schema_version}),
            promotion_gate_ref: format!(
                "promotion-gate://model-lane-message/{}",
                source.message_id
            ),
            promotion_receipt_ref: None,
            validation_runner_ref: format!(
                "eventledger://{}",
                posture.update.event_ledger_event_id
            ),
            authority_effect: "advisory_only".into(),
        });
        self.create
            .run(self.store.record_context_bundle_handoff(input))
            .await
            .expect("actual authorized canonical handoff producer");
    }
}
fn navigation_request(
    account: &account_session_support::AccountFixture,
    kind: ResourceKind,
    id: &str,
    action: ResourceAction,
    capability: &str,
) -> AuthorizationRequest {
    use sha2::{Digest, Sha256};
    AuthorizationRequest {
        session_token: account.session_token.clone(),
        channel_binding_hash: Some(hex::encode(Sha256::digest(
            account.channel_binding_token.as_bytes(),
        ))),
        capability_id: capability.into(),
        resource_kind: kind,
        external_resource_id: id.into(),
        action,
    }
}
async fn navigation_operation(
    storage: &SurrealStorage,
    account: &account_session_support::AccountFixture,
    scope: &ResourceScope,
    kind: ResourceKind,
    id: &str,
    action: ResourceAction,
    capability: &str,
) -> AuthenticatedModelLaneTestOperation {
    AuthenticatedModelLaneTestOperation::authorize(storage.clone(), navigation_request(account, kind, id, action, capability),
        handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("same real tuple"))
        .await.expect("actual canonical fixture operation")
}
fn navigation_handoff_draft(label: &str) -> lane::NewModelLaneContextBundleHandoff {
    lane::NewModelLaneContextBundleHandoff {
        handoff_id: format!("navigation-handoff-{label}"),
        context_bundle_id: String::new(),
        run_id: format!("run-mt018-{label}"),
        trace_id: format!("trace-mt018-{label}"),
        handoff_span_id: format!("navigation-handoff-span-{label}"),
        parent_span_id: None,
        linked_span_contexts: Vec::new(),
        downstream_lane_id: format!("lane-mt018-{label}"),
        source_lane_id: String::new(),
        source_message_id: String::new(),
        artifact_ref: String::new(),
        artifact_sha256: String::new(),
        content_hash: String::new(),
        source_kind: lane::ModelLaneHandoffSourceKind::Proposal,
        authority_state: lane::ModelLaneAuthority::PromotionCandidate,
        selection_state: lane::ModelLaneHandoffSelectionState::Selected,
        reason_code: "navigation_source_selected".into(),
        decision_ref: None,
        reviewer_ref: None,
        replay_hint: "read canonical authorized source".into(),
        crdt_payload: None,
        loom_refs: Vec::new(),
        memory_pack_refs: Vec::new(),
        event_ledger_stream_id: format!("model-lane://mt018/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-018".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-mt018-{label}"),
        idempotency_key: format!("navigation-handoff-{label}"),
        replay_order_key: format!("0040-{label}"),
        created_at_utc: "2026-09-02T00:04:00Z".into(),
        diagnostic_payload: json!({}),
    }
}
