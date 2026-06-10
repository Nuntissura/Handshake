//! WP-KERNEL-005 MT-104 / MT-130: real PostgreSQL round-trip proofs for the
//! Comfy-compatible Replay Input Contract and the replay event family.
//!
//! MT-104 (Replay Input Contract): a `ReplayRequest` captures the workflow
//! identity + the set of input artifact refs needed to replay a run, and
//! `resolve_replay_inputs` proves each ref is a Handshake-native portable handle
//! that resolves to a STORED artifact (an `atelier_comfy_intake_output` row for
//! the run). The tests prove a valid request resolves all refs and that a
//! missing/invalid ref is rejected.
//!
//! MT-130 (Replay event family): `request_replay` emits `REPLAY_REQUESTED` on a
//! request, `REPLAY_COMPLETED` on successful resolution, and `REPLAY_FAILED` on
//! rejection -- all run-scoped on `workflow_run_id`. The event test asserts the
//! specific event rows for THIS test's run (via `count_events_for_aggregate`),
//! never global event counts.
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite). No mocks: each
//! test connects the real `AtelierStore` to a real Postgres, ensures schema,
//! materializes a real stored intake output, and resolves/replays against it.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::comfy::{
    comfy_event_family, MediaKind, NewIntakeOutput, ReplayRequest, RoutingIntent,
};
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Record a real stored intake output for `run_id` and return its portable
/// `artifact_ref`. This is the "stored artifact" a replay input ref must resolve
/// to (Section 6.9.4 governed output). Refs use the portable
/// `artifact://atelier/comfy/<uuid>` form, never a machine-local path.
async fn store_real_output(store: &AtelierStore, run_id: Uuid) -> String {
    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let new_output = NewIntakeOutput {
        workflow_run_id: run_id,
        node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
        registration_id: None,
        source_node_instance_id: "42".to_string(),
        source_output_slot: "IMAGE".to_string(),
        media_kind: MediaKind::Image,
        mime: "image/png".to_string(),
        artifact_ref: artifact_ref.clone(),
        artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
        content_hash: format!("sha256-{}", Uuid::new_v4()),
        routing_intent: RoutingIntent::Artifact,
        parent_artifact_ref: None,
        prompt_json_ref: Some(format!("artifact://atelier/prompt/{}", Uuid::new_v4())),
        graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
        seed: Some(123_456_789),
        identity_metadata: None,
    };
    let outcome = store
        .record_intake_output(&new_output)
        .await
        .expect("record real intake output for replay input");
    assert!(!outcome.deduplicated, "first delivery is fresh");
    artifact_ref
}

/// MT-104: a valid `ReplayRequest` resolves every input artifact ref to a stored
/// artifact for the run; a request naming a non-existent ref is rejected.
#[tokio::test]
async fn mt104_replay_request_resolves_all_input_refs() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt104_replay_request_resolves_all_input_refs: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let run_id = Uuid::new_v4();

    // Two real stored outputs become the replay inputs.
    let ref_a = store_real_output(&store, run_id).await;
    let ref_b = store_real_output(&store, run_id).await;

    let valid = ReplayRequest {
        workflow_run_id: run_id,
        workflow_spec_ref: format!("artifact://atelier/spec/{}", Uuid::new_v4()),
        workflow_json_ref: format!("artifact://atelier/graph/{}", Uuid::new_v4()),
        graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
        seed: Some(42),
        input_artifact_refs: vec![ref_a.clone(), ref_b.clone()],
    };

    let resolved = store
        .resolve_replay_inputs(&valid)
        .await
        .expect("valid replay request resolves all input refs");
    assert_eq!(
        resolved.resolved_inputs.len(),
        2,
        "every declared input ref must resolve to a stored artifact"
    );
    let resolved_refs: Vec<&str> = resolved
        .resolved_inputs
        .iter()
        .map(|r| r.artifact_ref.as_str())
        .collect();
    assert!(resolved_refs.contains(&ref_a.as_str()));
    assert!(resolved_refs.contains(&ref_b.as_str()));
    for input in &resolved.resolved_inputs {
        assert!(
            !input.content_hash.trim().is_empty(),
            "resolved input must carry its stored content hash"
        );
    }

    // --- rejection: a ref that does not resolve to a stored artifact for the run ---
    let missing_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let unresolved = ReplayRequest {
        workflow_run_id: run_id,
        workflow_spec_ref: format!("artifact://atelier/spec/{}", Uuid::new_v4()),
        workflow_json_ref: format!("artifact://atelier/graph/{}", Uuid::new_v4()),
        graph_hash: None,
        seed: None,
        input_artifact_refs: vec![ref_a.clone(), missing_ref.clone()],
    };
    let err = store
        .resolve_replay_inputs(&unresolved)
        .await
        .expect_err("a missing input ref must be rejected");
    assert!(
        err.to_string().contains("does not resolve to a stored artifact"),
        "unexpected error for missing ref: {err}"
    );

    // --- rejection: a forbidden legacy/.GOV ref never resolves ---
    let legacy = ReplayRequest {
        workflow_run_id: run_id,
        workflow_spec_ref: format!("artifact://atelier/spec/{}", Uuid::new_v4()),
        workflow_json_ref: format!("artifact://atelier/graph/{}", Uuid::new_v4()),
        graph_hash: None,
        seed: None,
        input_artifact_refs: vec!["artifact://atelier/.GOV/replay-input.png".to_string()],
    };
    let legacy_err = store
        .resolve_replay_inputs(&legacy)
        .await
        .expect_err(".GOV replay input refs are forbidden");
    assert!(
        legacy_err.to_string().contains("Handshake-native portable ref"),
        "unexpected error for legacy ref: {legacy_err}"
    );
}

/// MT-130: `request_replay` emits REPLAY_REQUESTED + REPLAY_COMPLETED on a
/// successful replay and REPLAY_REQUESTED + REPLAY_FAILED on a rejected one.
/// Assertions are run-scoped: they count only the events for THIS test's
/// workflow run id, never global event totals.
#[tokio::test]
async fn mt130_replay_event_family_emitted_on_request_complete_fail() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt130_replay_event_family_emitted_on_request_complete_fail: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // The replay event family is registered for parity/coverage.
    assert!(
        handshake_core::atelier::event_family::ALL.contains(&comfy_event_family::REPLAY_REQUESTED)
            && handshake_core::atelier::event_family::ALL
                .contains(&comfy_event_family::REPLAY_COMPLETED)
            && handshake_core::atelier::event_family::ALL
                .contains(&comfy_event_family::REPLAY_FAILED),
        "replay event family must be folded into the aggregate ALL list"
    );

    const AGG: &str = "atelier_comfy_intake_output";

    // --- success path: REPLAY_REQUESTED + REPLAY_COMPLETED, no REPLAY_FAILED ---
    let ok_run = Uuid::new_v4();
    let ok_agg = ok_run.to_string();
    let ok_ref = store_real_output(&store, ok_run).await;
    let ok_request = ReplayRequest {
        workflow_run_id: ok_run,
        workflow_spec_ref: format!("artifact://atelier/spec/{}", Uuid::new_v4()),
        workflow_json_ref: format!("artifact://atelier/graph/{}", Uuid::new_v4()),
        graph_hash: None,
        seed: None,
        input_artifact_refs: vec![ok_ref],
    };
    store
        .request_replay(&ok_request)
        .await
        .expect("valid replay request completes");

    let requested = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_REQUESTED, AGG, &ok_agg)
        .await
        .expect("count REPLAY_REQUESTED for ok run");
    let completed = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_COMPLETED, AGG, &ok_agg)
        .await
        .expect("count REPLAY_COMPLETED for ok run");
    let failed = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_FAILED, AGG, &ok_agg)
        .await
        .expect("count REPLAY_FAILED for ok run");
    assert_eq!(requested, 1, "REPLAY_REQUESTED emitted once for the ok run");
    assert_eq!(completed, 1, "REPLAY_COMPLETED emitted once for the ok run");
    assert_eq!(failed, 0, "no REPLAY_FAILED for the successful ok run");

    // --- failure path: REPLAY_REQUESTED + REPLAY_FAILED, no REPLAY_COMPLETED ---
    let bad_run = Uuid::new_v4();
    let bad_agg = bad_run.to_string();
    let missing_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let bad_request = ReplayRequest {
        workflow_run_id: bad_run,
        workflow_spec_ref: format!("artifact://atelier/spec/{}", Uuid::new_v4()),
        workflow_json_ref: format!("artifact://atelier/graph/{}", Uuid::new_v4()),
        graph_hash: None,
        seed: None,
        input_artifact_refs: vec![missing_ref],
    };
    store
        .request_replay(&bad_request)
        .await
        .expect_err("replay request with an unresolved ref fails");

    let bad_requested = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_REQUESTED, AGG, &bad_agg)
        .await
        .expect("count REPLAY_REQUESTED for bad run");
    let bad_completed = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_COMPLETED, AGG, &bad_agg)
        .await
        .expect("count REPLAY_COMPLETED for bad run");
    let bad_failed = store
        .count_events_for_aggregate(comfy_event_family::REPLAY_FAILED, AGG, &bad_agg)
        .await
        .expect("count REPLAY_FAILED for bad run");
    assert_eq!(bad_requested, 1, "REPLAY_REQUESTED emitted once for the bad run");
    assert_eq!(bad_completed, 0, "no REPLAY_COMPLETED for the failed bad run");
    assert_eq!(bad_failed, 1, "REPLAY_FAILED emitted once for the bad run");
}
