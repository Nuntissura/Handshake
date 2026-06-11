//! WP-KERNEL-005 ComfyUI custom-node intake: live PostgreSQL round-trip proofs
//! for the `atelier::comfy` submodule (Section 6.9). Run with a live
//! DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_comfy_tests -- --nocapture
//!
//! No mocks: each test connects the real `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the comfy intake records with REAL data, and
//! asserts the load-bearing invariants from Section 6.9 (probe idempotency +
//! fallback_reason guard, capability registration with declared/reject child
//! rows, output dedup on (workflow_run_id, content_hash), SaveImage fallback
//! marker, receipt, secret scrubbing). Tables persist between runs, so all
//! workflow_run_ids / hashes are made unique per run via `Uuid::new_v4()` to
//! avoid cross-run collisions. Only `handshake_core` + `tokio` + `uuid` +
//! `serde_json` (+ std) are used; sqlx is never imported directly.

use chrono::Duration;
use handshake_core::atelier::comfy::{
    comfy_event_family, scrub_provenance, ComfyBridgeFakeAdapterV1,
    ComfyOutputRegistrationFailureStatus, ComfyWorkflowHistoryQuery, ComfyWorkflowStatus,
    DeclaredOutput, IdentityWorkflowMetadata, MediaKind, NewBridgeProbe, NewCapabilityRegistration,
    NewComfyOutputRegistrationFailure, NewComfyWorkflowReceipt, NewIntakeOutput, ProbeOutcome,
    RoutingIntent, SAVEIMAGE_FALLBACK_SLOT,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

#[tokio::test]
async fn atelier_comfy_rejects_legacy_runtime_output_refs() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_comfy_rejects_legacy_runtime_output_refs: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let err = store
        .record_intake_output(&NewIntakeOutput::saveimage_fallback(
            Uuid::new_v4(),
            "node-exec-runtime-ref",
            "save-image",
            "image/png",
            "artifact://atelier/.GOV/comfy-output.png",
            &format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            &format!("sha256-{}", Uuid::new_v4()),
        ))
        .await
        .expect_err(".GOV output refs are forbidden");
    assert!(
        err.to_string().contains("Handshake-native portable ref"),
        "unexpected comfy output error: {err}"
    );

    let err = store
        .record_intake_output(&NewIntakeOutput::saveimage_fallback(
            Uuid::new_v4(),
            "node-exec-localhost-manifest",
            "save-image",
            "image/png",
            &format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            "http://localhost:9000/manifest.json",
            &format!("sha256-{}", Uuid::new_v4()),
        ))
        .await
        .expect_err("localhost manifest refs are forbidden");
    assert!(
        err.to_string().contains("Handshake-native portable ref"),
        "unexpected comfy manifest error: {err}"
    );
}

#[tokio::test]
async fn atelier_comfy_fake_adapter_is_deterministic_and_capability_gated() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_fake_adapter_is_deterministic_and_capability_gated: DATABASE_URL not set"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let adapter = ComfyBridgeFakeAdapterV1::default();
    let run_id = Uuid::new_v4();

    let probe = store
        .record_bridge_probe(&adapter.probe(run_id))
        .await
        .expect("fake adapter probe records");
    assert_eq!(probe.workflow_run_id, run_id);
    assert_eq!(probe.probe_outcome, ProbeOutcome::BridgePresent);
    assert_eq!(
        probe.node_class_id,
        ComfyBridgeFakeAdapterV1::NODE_CLASS_ID,
        "fake adapter uses a stable deterministic node class"
    );

    let denied = adapter.capability_registration(
        run_id,
        "Analyst",
        &format!("denied-evidence-{}", Uuid::new_v4()),
    );
    let denied_err = store
        .register_bridge_capability(&denied)
        .await
        .expect_err("profile without engine.comfyui must be denied before registration");
    assert!(
        denied_err.to_string().contains("engine.comfyui"),
        "denial must name the missing engine.comfyui capability: {denied_err}"
    );
    assert!(
        store
            .get_capability_registration(run_id)
            .await
            .expect("lookup denied registration")
            .is_none(),
        "denied registration must not persist a bridge capability row"
    );

    for evidence_ref in [
        "file:///tmp/comfy-evidence.json",
        "http://localhost:9000/comfy-evidence.json",
        "artifact://atelier/.GOV/comfy-evidence.json",
        "sqlite://legacy/comfy-evidence.db",
        "C:\\Users\\operator\\comfy-evidence.json",
    ] {
        let bad_run_id = Uuid::new_v4();
        let bad = adapter.capability_registration(
            bad_run_id,
            ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            evidence_ref,
        );
        let err = store
            .register_bridge_capability(&bad)
            .await
            .expect_err("legacy runtime evidence refs must be denied before registration");
        assert!(
            err.to_string()
                .contains("capability_grant_ref evidence_ref"),
            "denial must name the grant evidence ref boundary: {err}"
        );
        assert!(
            store
                .get_capability_registration(bad_run_id)
                .await
                .expect("lookup bad registration")
                .is_none(),
            "bad capability grant evidence ref must not persist a bridge capability row"
        );
    }

    let allowed = adapter.capability_registration(
        run_id,
        ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
        &format!("allowed-evidence-{}", Uuid::new_v4()),
    );
    let registered = store
        .register_bridge_capability(&allowed)
        .await
        .expect("allowed ComfyUI worker profile registers bridge capability");
    assert_eq!(registered.workflow_run_id, run_id);
    assert_eq!(
        registered.declared_outputs, allowed.accepted_outputs,
        "fake adapter accepted outputs are deterministic"
    );

    let rejects = store
        .list_capability_rejects(run_id)
        .await
        .expect("list fake adapter capability rejects");
    assert_eq!(
        rejects, allowed.rejected_outputs,
        "fake adapter rejected outputs are deterministic"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_comfy_capability_registration",
            &registered.registration_id.to_string(),
        )
        .await
        .expect("list kernel EventLedger rows for Comfy registration");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == comfy_event_family::CAPABILITY_REGISTERED
                && event.payload["atelier_payload"]["declared_output_count"] == 2
                && event.payload["atelier_payload"]["reject_count"] == 1
        }),
        "accepted fake-adapter registration must append a canonical kernel EventLedger event"
    );
}

/// Probe idempotency on `workflow_run_id`, the `fallback_reason` server-side
/// guard, and `PROBE_RECORDED` event emission (Section 6.9.2).
#[tokio::test]
async fn atelier_comfy_probe_idempotency_and_fallback_guard() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_comfy_probe_idempotency_and_fallback_guard: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();
    let node_class = format!("HandshakeIntakeBridge-{}", Uuid::new_v4());

    let before = store
        .count_events(comfy_event_family::PROBE_RECORDED)
        .await
        .expect("count probe events before");

    // --- bridge_present probe needs no fallback_reason ---
    let probe = store
        .record_bridge_probe(&NewBridgeProbe {
            workflow_run_id: run_id,
            node_class_id: node_class.clone(),
            bridge_protocol_version: Some("1.0.0".to_string()),
            node_instance_ids: vec!["12".to_string(), "13".to_string()],
            probe_outcome: ProbeOutcome::BridgePresent,
            fallback_reason: None,
        })
        .await
        .expect("record bridge_present probe");
    assert_eq!(probe.workflow_run_id, run_id);
    assert_eq!(probe.probe_outcome, ProbeOutcome::BridgePresent);
    assert!(probe.detected, "bridge_present implies detected = true");
    assert_eq!(
        probe.node_instance_ids,
        vec!["12".to_string(), "13".to_string()],
        "node_instance_ids round-trip through the json column"
    );

    // --- re-probing the same run is idempotent (one probe per run) ---
    let probe_again = store
        .record_bridge_probe(&NewBridgeProbe {
            workflow_run_id: run_id,
            node_class_id: node_class.clone(),
            bridge_protocol_version: Some("1.0.0".to_string()),
            node_instance_ids: vec!["12".to_string(), "13".to_string()],
            probe_outcome: ProbeOutcome::BridgePresent,
            fallback_reason: None,
        })
        .await
        .expect("re-record same-run probe");
    assert_eq!(
        probe.probe_id, probe_again.probe_id,
        "re-probing the same workflow_run_id returns the existing probe row (ON CONFLICT)"
    );

    let fetched = store
        .get_bridge_probe(run_id)
        .await
        .expect("get bridge probe")
        .expect("probe present");
    assert_eq!(
        fetched.probe_id, probe.probe_id,
        "single probe is recoverable"
    );

    // --- INVARIANT: a non-present outcome with no fallback_reason is rejected ---
    let bad = store
        .record_bridge_probe(&NewBridgeProbe {
            workflow_run_id: Uuid::new_v4(),
            node_class_id: node_class.clone(),
            bridge_protocol_version: None,
            node_instance_ids: vec![],
            probe_outcome: ProbeOutcome::BridgeAbsent,
            fallback_reason: None,
        })
        .await;
    assert!(
        bad.is_err(),
        "fallback_reason is required when outcome != bridge_present"
    );

    // A bridge_absent probe WITH a reason is accepted.
    let absent = store
        .record_bridge_probe(&NewBridgeProbe {
            workflow_run_id: Uuid::new_v4(),
            node_class_id: node_class.clone(),
            bridge_protocol_version: None,
            node_instance_ids: vec![],
            probe_outcome: ProbeOutcome::BridgeAbsent,
            fallback_reason: Some("bridge node not found in graph".to_string()),
        })
        .await
        .expect("record bridge_absent probe with reason");
    assert_eq!(absent.probe_outcome, ProbeOutcome::BridgeAbsent);
    assert!(!absent.detected, "bridge_absent implies detected = false");

    // --- event emission increased (two successful probes recorded) ---
    let after = store
        .count_events(comfy_event_family::PROBE_RECORDED)
        .await
        .expect("count probe events after");
    assert!(
        after >= before + 2,
        "PROBE_RECORDED events increased for the successful probes"
    );
}

/// Capability registration: ordered declared outputs, typed reject rows that
/// are never routed, replay-stable idempotency on `workflow_run_id`, and
/// `CAPABILITY_REGISTERED` / `CAPABILITY_REJECTED` event emission (6.9.3).
#[tokio::test]
async fn atelier_comfy_capability_registration_declared_and_rejects() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_capability_registration_declared_and_rejects: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();
    let node_class = format!("HandshakeIntakeBridge-{}", Uuid::new_v4());
    let grant_ref = ComfyBridgeFakeAdapterV1::capability_grant_ref(
        ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
        &format!("capability-evidence-{}", Uuid::new_v4()),
    );

    let reg_before = store
        .count_events(comfy_event_family::CAPABILITY_REGISTERED)
        .await
        .expect("count register events before");
    let rej_before = store
        .count_events(comfy_event_family::CAPABILITY_REJECTED)
        .await
        .expect("count reject events before");

    let accepted = vec![
        DeclaredOutput {
            output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            expected_mime: "image/png".to_string(),
            routing_intent: RoutingIntent::Artifact,
        },
        DeclaredOutput {
            output_slot: "MASK".to_string(),
            media_kind: MediaKind::Mask,
            expected_mime: "image/png".to_string(),
            routing_intent: RoutingIntent::Sidecar,
        },
    ];
    let rejected = vec![(
        "PREVIEW".to_string(),
        "transient preview not permitted by capability profile".to_string(),
    )];

    let reg = store
        .register_bridge_capability(&NewCapabilityRegistration {
            workflow_run_id: run_id,
            node_class_id: node_class.clone(),
            bridge_protocol_version: "1.0.0".to_string(),
            accepted_outputs: accepted.clone(),
            rejected_outputs: rejected.clone(),
            capability_grant_ref: grant_ref.clone(),
            consent_decision_ref: None,
        })
        .await
        .expect("register bridge capability");
    assert_eq!(reg.workflow_run_id, run_id);
    assert_eq!(
        reg.declared_outputs.len(),
        2,
        "both accepted outputs stored as ordered child rows"
    );
    assert_eq!(
        reg.declared_outputs[0].output_slot, "IMAGE",
        "declared outputs preserve declaration order"
    );
    assert_eq!(reg.declared_outputs[1].media_kind, MediaKind::Mask);

    // --- INVARIANT: rejected outputs are recorded but never enter declared (routable) set ---
    let rejects = store
        .list_capability_rejects(run_id)
        .await
        .expect("list capability rejects");
    assert_eq!(
        rejects.len(),
        1,
        "the rejected output is recorded as a typed reject row"
    );
    assert_eq!(rejects[0].0, "PREVIEW");
    assert!(
        !reg.declared_outputs
            .iter()
            .any(|d| d.output_slot == "PREVIEW"),
        "a capability-rejected output must never appear in the routable declared set"
    );

    // --- re-registration is replay-stable: same registration_id, no child-row accretion ---
    let reg_again = store
        .register_bridge_capability(&NewCapabilityRegistration {
            workflow_run_id: run_id,
            node_class_id: node_class.clone(),
            bridge_protocol_version: "1.0.0".to_string(),
            accepted_outputs: accepted.clone(),
            rejected_outputs: rejected.clone(),
            capability_grant_ref: grant_ref.clone(),
            consent_decision_ref: None,
        })
        .await
        .expect("re-register same-run capability");
    assert_eq!(
        reg.registration_id, reg_again.registration_id,
        "re-registering the same workflow_run_id returns the existing registration (idempotent)"
    );
    assert_eq!(
        reg_again.declared_outputs.len(),
        2,
        "re-registration replaces child rows; declared count stays stable (no duplicates)"
    );
    let rejects_again = store
        .list_capability_rejects(run_id)
        .await
        .expect("list rejects after re-register");
    assert_eq!(
        rejects_again.len(),
        1,
        "reject child rows are replaced, not accreted, on re-registration"
    );

    // --- event emission increased for both register and reject families ---
    let reg_after = store
        .count_events(comfy_event_family::CAPABILITY_REGISTERED)
        .await
        .expect("count register events after");
    let rej_after = store
        .count_events(comfy_event_family::CAPABILITY_REJECTED)
        .await
        .expect("count reject events after");
    assert!(
        reg_after >= reg_before + 2,
        "CAPABILITY_REGISTERED emitted on each registration"
    );
    assert!(
        rej_after >= rej_before + 2,
        "CAPABILITY_REJECTED emitted per dropped output on each registration"
    );
}

/// Output routing: fresh materialization, dedup on `(workflow_run_id,
/// content_hash)`, sidecar lineage-binding rejection, and the
/// OUTPUT_MATERIALIZED / OUTPUT_DEDUPLICATED event split (6.9.4).
#[tokio::test]
async fn atelier_comfy_output_dedup_and_sidecar_binding() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_comfy_output_dedup_and_sidecar_binding: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());

    let mat_before = store
        .count_events(comfy_event_family::OUTPUT_MATERIALIZED)
        .await
        .expect("count materialized before");
    let dedup_before = store
        .count_events(comfy_event_family::OUTPUT_DEDUPLICATED)
        .await
        .expect("count dedup before");

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
        content_hash: content_hash.clone(),
        routing_intent: RoutingIntent::Artifact,
        parent_artifact_ref: None,
        prompt_json_ref: Some(format!("artifact://atelier/prompt/{}", Uuid::new_v4())),
        graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
        seed: Some(123_456_789),
        identity_metadata: None,
    };

    // --- first record is a fresh materialization ---
    let first = store
        .record_intake_output(&new_output)
        .await
        .expect("record fresh intake output");
    assert!(
        !first.deduplicated,
        "first delivery is a fresh materialization"
    );
    assert_eq!(first.output.artifact_ref, artifact_ref);
    assert_eq!(first.output.seed, Some(123_456_789), "seed pin round-trips");

    // --- INVARIANT: re-delivery of same (run, content_hash) dedups to the existing artifact ---
    let second = store
        .record_intake_output(&new_output)
        .await
        .expect("re-record same output");
    assert!(
        second.deduplicated,
        "re-delivery with the same content_hash resolves to a dedup hit"
    );
    assert_eq!(
        first.output.intake_output_id, second.output.intake_output_id,
        "dedup returns the existing row, never a duplicate"
    );
    assert_eq!(
        second.output.artifact_ref, artifact_ref,
        "dedup resolves to the existing artifact_ref"
    );

    let listed = store
        .list_intake_outputs(run_id)
        .await
        .expect("list intake outputs");
    assert_eq!(
        listed.len(),
        1,
        "exactly one output row exists after re-delivery"
    );

    // --- INVARIANT: a sidecar without a parent_artifact_ref is rejected (lineage binding) ---
    let orphan_sidecar = NewIntakeOutput {
        workflow_run_id: run_id,
        node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
        registration_id: None,
        source_node_instance_id: "43".to_string(),
        source_output_slot: "SIDECAR".to_string(),
        media_kind: MediaKind::SidecarJson,
        mime: "application/json".to_string(),
        artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
        artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
        content_hash: format!("sha256-{}", Uuid::new_v4()),
        routing_intent: RoutingIntent::Sidecar,
        parent_artifact_ref: None,
        prompt_json_ref: None,
        graph_hash: None,
        seed: None,
        identity_metadata: None,
    };
    let sidecar_err = store.record_intake_output(&orphan_sidecar).await;
    assert!(
        sidecar_err.is_err(),
        "a sidecar output must bind to a parent_artifact_ref; orphan sidecar is rejected"
    );

    // --- event emission: one materialized, one deduplicated ---
    let mat_after = store
        .count_events(comfy_event_family::OUTPUT_MATERIALIZED)
        .await
        .expect("count materialized after");
    let dedup_after = store
        .count_events(comfy_event_family::OUTPUT_DEDUPLICATED)
        .await
        .expect("count dedup after");
    assert!(
        mat_after >= mat_before + 1,
        "OUTPUT_MATERIALIZED emitted for the fresh materialization"
    );
    assert!(
        dedup_after >= dedup_before + 1,
        "OUTPUT_DEDUPLICATED emitted for the dedup hit"
    );
}

#[tokio::test]
async fn atelier_comfy_identity_metadata_survives_workflow_receipt_inputs() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_identity_metadata_survives_workflow_receipt_inputs: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();
    let metadata = IdentityWorkflowMetadata {
        landmarks: Some(serde_json::json!({
            "left_eye": [210.5, 224.25],
            "right_eye": [304.75, 223.5],
            "mouth_center": [257.0, 350.0]
        })),
        measurements: Some(serde_json::json!({
            "inter_eye_px": 94.25,
            "face_crop_px": [512, 512]
        })),
        pose_metadata: Some(serde_json::json!({
            "yaw_deg": -4.5,
            "pitch_deg": 1.25,
            "roll_deg": 0.5,
            "source": "mt-100"
        })),
    };

    let output = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "identity-conditioning".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: Some(format!("artifact://atelier/prompt/{}", Uuid::new_v4())),
            graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
            seed: Some(42),
            identity_metadata: Some(metadata.clone()),
        })
        .await
        .expect("record output with identity workflow metadata")
        .output;

    assert_eq!(
        output.workflow_input_metadata["identity"]["landmarks"]["left_eye"],
        serde_json::json!([210.5, 224.25]),
        "identity landmarks survive output storage"
    );
    assert_eq!(
        output.workflow_input_metadata["identity"]["measurements"]["face_crop_px"],
        serde_json::json!([512, 512]),
        "identity measurements survive output storage"
    );
    assert_eq!(
        output.workflow_input_metadata["identity"]["pose_metadata"]["yaw_deg"],
        serde_json::json!(-4.5),
        "pose metadata survives output storage"
    );

    let absent = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "identity-conditioning-absent".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: None,
            identity_metadata: None,
        })
        .await
        .expect("record output with absent identity metadata")
        .output;
    assert_eq!(
        absent.workflow_input_metadata,
        serde_json::json!({ "identity": {} }),
        "absent identity metadata serializes as an explicit empty identity object"
    );

    let receipt = store
        .produce_intake_receipt(run_id)
        .await
        .expect("produce receipt with identity workflow inputs");
    assert_eq!(receipt.workflow_inputs.len(), 2);
    assert_eq!(
        receipt.workflow_inputs[0]["artifact_ref"], output.artifact_ref,
        "receipt workflow input is tied to the output artifact"
    );
    assert_eq!(
        receipt.workflow_inputs[0]["identity"]["landmarks"]["mouth_center"],
        serde_json::json!([257.0, 350.0]),
        "receipt preserves identity landmarks in workflow inputs"
    );
    assert_eq!(
        receipt.workflow_inputs[1]["identity"],
        serde_json::json!({}),
        "receipt preserves absent identity metadata without failure"
    );

    let bad_metadata = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id: Uuid::new_v4(),
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "identity-conditioning-bad".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: None,
            identity_metadata: Some(IdentityWorkflowMetadata {
                landmarks: Some(serde_json::json!("not structured")),
                measurements: None,
                pose_metadata: None,
            }),
        })
        .await;
    assert!(
        bad_metadata.is_err(),
        "identity metadata fields must be structured JSON objects or arrays"
    );
}

#[tokio::test]
async fn atelier_comfy_output_registration_failure_is_retryable_without_losing_saved_image() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_output_registration_failure_is_retryable_without_losing_saved_image: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let run_id = Uuid::new_v4();
    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let artifact_manifest_ref = format!("manifest://atelier/comfy/{}", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());

    let failure_before = store
        .count_events(comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED)
        .await
        .expect("count failure events before");
    let retry_before = store
        .count_events(comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED)
        .await
        .expect("count retry events before");

    let failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: Some(Uuid::new_v4()),
            source_node_instance_id: "saveimage-late-register".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: artifact_ref.clone(),
            artifact_manifest_ref: artifact_manifest_ref.clone(),
            content_hash: content_hash.clone(),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: Some(format!("artifact://atelier/prompt/{}", Uuid::new_v4())),
            graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
            seed: Some(707),
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "capability registration unavailable after image save".to_string(),
            evidence: serde_json::json!({
                "saveimage_path_ref": format!("artifact://atelier/comfy/saveimage/{}", Uuid::new_v4()),
                "Authorization": "Bearer should-not-persist"
            }),
        })
        .await
        .expect("record retryable output registration failure");

    assert_eq!(failure.workflow_run_id, run_id);
    assert_eq!(failure.artifact_ref, artifact_ref);
    assert_eq!(failure.artifact_manifest_ref, artifact_manifest_ref);
    assert_eq!(failure.content_hash, content_hash);
    assert_eq!(
        failure.status,
        ComfyOutputRegistrationFailureStatus::Retryable
    );
    assert_eq!(failure.resolved_intake_output_id, None);
    assert_eq!(
        failure.evidence["Authorization"],
        serde_json::json!("[REDACTED]"),
        "failure evidence is scrubbed while preserving saved-image refs"
    );

    let outputs_before_retry = store
        .list_intake_outputs(run_id)
        .await
        .expect("list outputs before retry");
    assert!(
        outputs_before_retry.is_empty(),
        "recording failure evidence preserves the generated output refs without pretending registration succeeded"
    );

    let listed = store
        .list_comfy_output_registration_failures(run_id)
        .await
        .expect("list output registration failures");
    assert_eq!(listed, vec![failure.clone()]);

    let adapter = ComfyBridgeFakeAdapterV1::default();
    let registration = store
        .register_bridge_capability(&adapter.capability_registration(
            run_id,
            ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
        ))
        .await
        .expect("record capability registration before retry");

    let retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await
        .expect("retry saved output registration");
    assert!(
        !retry.deduplicated,
        "the first retry registers the saved image as a fresh intake output"
    );
    assert_eq!(retry.output.artifact_ref, artifact_ref);
    assert_eq!(
        retry.output.registration_id,
        Some(registration.registration_id)
    );
    assert_eq!(retry.output.seed, Some(707));

    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("get failure after retry")
        .expect("failure still queryable");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered
    );
    assert_eq!(
        resolved.resolved_intake_output_id,
        Some(retry.output.intake_output_id)
    );
    assert_eq!(resolved.retry_count, 1);

    let second_retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await;
    assert!(
        second_retry.is_err(),
        "registered failure evidence is no longer retryable"
    );

    let local_path_failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: Uuid::new_v4(),
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: None,
            source_node_instance_id: "saveimage-local".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: "C:\\ComfyUI\\output\\image.png".to_string(),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: None,
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "local path should be rejected".to_string(),
            evidence: serde_json::json!({}),
        })
        .await;
    assert!(
        local_path_failure.is_err(),
        "saved-image failure rows must preserve portable artifact refs, not machine-local paths"
    );

    let failure_after = store
        .count_events(comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED)
        .await
        .expect("count failure events after");
    let retry_after = store
        .count_events(comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED)
        .await
        .expect("count retry events after");
    assert!(failure_after >= failure_before + 1);
    assert!(retry_after >= retry_before + 1);
}

#[tokio::test]
async fn atelier_comfy_workflow_receipt_schema_preserves_outputs_status_and_evidence() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_workflow_receipt_schema_preserves_outputs_status_and_evidence: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let run_id = Uuid::new_v4();

    let first = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "saveimage-primary".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: Some(format!(
                "artifact://atelier/workflow-json/{}",
                Uuid::new_v4()
            )),
            graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
            seed: Some(1001),
            identity_metadata: Some(IdentityWorkflowMetadata {
                landmarks: Some(serde_json::json!({ "left_eye": [210.5, 224.25] })),
                measurements: None,
                pose_metadata: None,
            }),
        })
        .await
        .expect("record first workflow output")
        .output;
    let second = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "saveimage-secondary".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
            seed: Some(1002),
            identity_metadata: None,
        })
        .await
        .expect("record second workflow output")
        .output;

    let receipt = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: run_id,
            workflow_spec_ref: format!("workflow-spec://{}", Uuid::new_v4()),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({
                "queue_id": format!("queue-{}", Uuid::new_v4()),
                "executor": "fake-adapter"
            }),
        })
        .await
        .expect("record durable workflow receipt");

    assert_eq!(receipt.system_id, "comfyui");
    assert_eq!(receipt.workflow_run_id, run_id);
    assert_eq!(receipt.status, ComfyWorkflowStatus::Succeeded);
    assert_eq!(
        receipt.outputs.len(),
        2,
        "receipt captures every output row"
    );
    assert_eq!(receipt.outputs[0]["artifact_ref"], first.artifact_ref);
    assert_eq!(receipt.outputs[1]["artifact_ref"], second.artifact_ref);
    assert_eq!(
        receipt.outputs[0]["workflow_input_metadata"]["identity"]["landmarks"]["left_eye"],
        serde_json::json!([210.5, 224.25]),
        "receipt keeps workflow input metadata beside the output it belongs to"
    );
    assert_eq!(
        receipt.receipt_json["schema"],
        "hsk.atelier.comfy.workflow_receipt@1"
    );
    assert_eq!(
        receipt.receipt_json["refs"]["workflow_spec_ref"],
        receipt.workflow_spec_ref
    );
    assert_eq!(receipt.receipt_json["status"], "succeeded");
    assert_eq!(receipt.receipt_json["outputs"].as_array().unwrap().len(), 2);
    assert_eq!(
        receipt.receipt_json["evidence"]["executor"],
        serde_json::json!("fake-adapter")
    );

    let fetched = store
        .get_comfy_workflow_receipt(run_id)
        .await
        .expect("get workflow receipt")
        .expect("workflow receipt present");
    assert_eq!(fetched, receipt);

    let failed_without_error = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: format!("workflow-spec://{}", Uuid::new_v4()),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Failed,
            error_ref: None,
            evidence: serde_json::json!({ "executor": "fake-adapter" }),
        })
        .await;
    assert!(
        failed_without_error.is_err(),
        "failed workflow receipts must carry an error_ref"
    );

    let local_ref = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: "C:\\operator\\workflow.json".to_string(),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({ "executor": "fake-adapter" }),
        })
        .await;
    assert!(
        local_ref.is_err(),
        "machine-local workflow receipt refs are rejected"
    );
}

#[tokio::test]
async fn atelier_comfy_workflow_history_and_stats_filter_receipts_including_failures() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_workflow_history_and_stats_filter_receipts_including_failures: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let character_a = format!("character://atelier/{}", Uuid::new_v4());
    let character_b = format!("character://atelier/{}", Uuid::new_v4());
    let spec_a = format!("workflow-spec://pose-comfy/{}", Uuid::new_v4());
    let spec_b = format!("workflow-spec://pose-comfy/{}", Uuid::new_v4());

    let success = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: spec_a.clone(),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({
                "character_ref": &character_a,
                "executor": "fake-adapter",
                "stats_case": "success"
            }),
        })
        .await
        .expect("record successful workflow receipt");
    let failed = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: spec_a.clone(),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Failed,
            error_ref: Some(format!(
                "artifact://atelier/workflow-error/{}",
                Uuid::new_v4()
            )),
            evidence: serde_json::json!({
                "character_ref": &character_a,
                "executor": "fake-adapter",
                "stats_case": "failed"
            }),
        })
        .await
        .expect("record failed workflow receipt");
    let _other_character = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id: Uuid::new_v4(),
            workflow_spec_ref: spec_b,
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({
                "character_ref": &character_b,
                "executor": "fake-adapter",
                "stats_case": "other"
            }),
        })
        .await
        .expect("record other character receipt");

    assert_eq!(success.character_ref, Some(character_a.clone()));
    assert_eq!(failed.character_ref, Some(character_a.clone()));

    let query = ComfyWorkflowHistoryQuery {
        character_ref: Some(character_a.clone()),
        workflow_spec_ref: Some(spec_a.clone()),
        status: None,
        from_utc: Some(success.created_at_utc - Duration::seconds(1)),
        to_utc: Some(failed.created_at_utc + Duration::seconds(1)),
    };
    let history = store
        .list_comfy_workflow_history(&query)
        .await
        .expect("list workflow history");
    assert_eq!(history.len(), 2);
    assert!(history
        .iter()
        .any(|receipt| receipt.workflow_run_id == success.workflow_run_id));
    assert!(
        history
            .iter()
            .any(|receipt| receipt.workflow_run_id == failed.workflow_run_id),
        "history includes failed receipts, not only successful outputs"
    );

    let failed_only = store
        .list_comfy_workflow_history(&ComfyWorkflowHistoryQuery {
            status: Some(ComfyWorkflowStatus::Failed),
            ..query.clone()
        })
        .await
        .expect("list failed workflow history");
    assert_eq!(failed_only.len(), 1);
    assert_eq!(failed_only[0].workflow_run_id, failed.workflow_run_id);

    let stats = store
        .comfy_workflow_stats(&query)
        .await
        .expect("aggregate workflow stats");
    assert_eq!(stats.total_count, 2);
    assert_eq!(stats.status_counts.get("succeeded"), Some(&1));
    assert_eq!(stats.status_counts.get("failed"), Some(&1));
    assert_eq!(
        stats.failure_count, 1,
        "aggregate stats count failed receipts in the same filtered range"
    );
}

/// SaveImage fallback marker + receipt: fallback marker idempotency, the
/// fallback sentinel slot, secret scrubbing of free-form provenance, and the
/// FALLBACK_ENGAGED / RECEIPT_PRODUCED events (6.9.5 / 6.9.6).
#[tokio::test]
async fn atelier_comfy_fallback_receipt_and_secret_scrub() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_comfy_fallback_receipt_and_secret_scrub: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();

    let fb_before = store
        .count_events(comfy_event_family::FALLBACK_ENGAGED)
        .await
        .expect("count fallback before");
    let receipt_before = store
        .count_events(comfy_event_family::RECEIPT_PRODUCED)
        .await
        .expect("count receipt before");

    // --- a bridge_absent probe so the receipt records the fallback path ---
    store
        .record_bridge_probe(&NewBridgeProbe {
            workflow_run_id: run_id,
            node_class_id: format!("HandshakeIntakeBridge-{}", Uuid::new_v4()),
            bridge_protocol_version: None,
            node_instance_ids: vec![],
            probe_outcome: ProbeOutcome::BridgeAbsent,
            fallback_reason: Some("no bridge node in graph".to_string()),
        })
        .await
        .expect("record absent probe");

    // --- INVARIANT: secret keys in free-form provenance are scrubbed before persistence ---
    let raw_provenance = serde_json::json!({
        "filename_hint": "front_yaw_01.png",
        "Authorization": "Bearer super-secret-token",
        "api_key": "sk-do-not-store",
        "model": "sdxl",
    });
    let scrubbed = scrub_provenance(&raw_provenance);
    assert_eq!(
        scrubbed["Authorization"],
        serde_json::json!("[REDACTED]"),
        "bearer credential is redacted, never stored raw"
    );
    assert_eq!(scrubbed["api_key"], serde_json::json!("[REDACTED]"));
    assert_eq!(
        scrubbed["filename_hint"],
        serde_json::json!("front_yaw_01.png"),
        "benign provenance keys survive scrubbing"
    );
    assert_eq!(scrubbed["model"], serde_json::json!("sdxl"));

    // --- record a SaveImage-fallback output via the helper constructor ---
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let fb_output = NewIntakeOutput::saveimage_fallback(
        run_id,
        format!("nodeexec-{}", Uuid::new_v4()),
        "saveimage-99",
        "image/png",
        format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
        format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
        content_hash.clone(),
    );
    let recorded = store
        .record_intake_output(&fb_output)
        .await
        .expect("record saveimage-fallback output");
    assert_eq!(
        recorded.output.source_output_slot, SAVEIMAGE_FALLBACK_SLOT,
        "fallback output carries the stable fallback sentinel slot"
    );
    assert_eq!(
        recorded.output.registration_id, None,
        "a SaveImage-fallback output has no capability registration"
    );

    // --- mark fallback engaged; idempotent on workflow_run_id ---
    store
        .mark_saveimage_fallback(run_id, "no bridge node in graph")
        .await
        .expect("mark fallback engaged");
    store
        .mark_saveimage_fallback(run_id, "no bridge node in graph (rescan)")
        .await
        .expect("re-mark fallback (idempotent)");
    let fb_reason = store
        .get_saveimage_fallback(run_id)
        .await
        .expect("get fallback marker")
        .expect("fallback marker present");
    assert_eq!(
        fb_reason, "no bridge node in graph (rescan)",
        "re-marking updates the single fallback row in place (one marker per run)"
    );

    // --- produce the receipt; it reconstructs what intake produced ---
    let receipt = store
        .produce_intake_receipt(run_id)
        .await
        .expect("produce intake receipt");
    assert_eq!(receipt.workflow_run_id, run_id);
    assert_eq!(
        receipt.probe_outcome,
        Some(ProbeOutcome::BridgeAbsent),
        "receipt reflects the recorded probe outcome"
    );
    assert!(
        receipt.fallback_engaged,
        "receipt records that the SaveImage fallback was engaged"
    );
    assert_eq!(
        receipt.materialized_artifact_refs.len(),
        1,
        "receipt lists the one fallback-materialized artifact"
    );
    assert_eq!(
        receipt.materialized_artifact_refs[0], recorded.output.artifact_ref,
        "receipt artifact ref matches the recorded output"
    );

    // --- event emission increased for fallback + receipt families ---
    let fb_after = store
        .count_events(comfy_event_family::FALLBACK_ENGAGED)
        .await
        .expect("count fallback after");
    let receipt_after = store
        .count_events(comfy_event_family::RECEIPT_PRODUCED)
        .await
        .expect("count receipt after");
    assert!(
        fb_after >= fb_before + 2,
        "FALLBACK_ENGAGED emitted on each mark"
    );
    assert!(
        receipt_after >= receipt_before + 1,
        "RECEIPT_PRODUCED emitted when the receipt was produced"
    );
}
