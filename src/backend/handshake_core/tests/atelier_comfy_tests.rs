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

use handshake_core::atelier::comfy::{
    comfy_event_family, scrub_provenance, DeclaredOutput, MediaKind, NewBridgeProbe,
    NewCapabilityRegistration, NewIntakeOutput, ProbeOutcome, RoutingIntent,
    SAVEIMAGE_FALLBACK_SLOT,
};
use handshake_core::atelier::AtelierStore;
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

/// Probe idempotency on `workflow_run_id`, the `fallback_reason` server-side
/// guard, and `PROBE_RECORDED` event emission (Section 6.9.2).
#[tokio::test]
async fn atelier_comfy_probe_idempotency_and_fallback_guard() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_probe_idempotency_and_fallback_guard: DATABASE_URL not set"
        );
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
    assert_eq!(fetched.probe_id, probe.probe_id, "single probe is recoverable");

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
    let grant_ref = format!("capgrant://engine.comfyui/{}", Uuid::new_v4());

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
    assert_eq!(rejects.len(), 1, "the rejected output is recorded as a typed reject row");
    assert_eq!(rejects[0].0, "PREVIEW");
    assert!(
        !reg.declared_outputs.iter().any(|d| d.output_slot == "PREVIEW"),
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
        eprintln!(
            "SKIP atelier_comfy_output_dedup_and_sidecar_binding: DATABASE_URL not set"
        );
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
    };

    // --- first record is a fresh materialization ---
    let first = store
        .record_intake_output(&new_output)
        .await
        .expect("record fresh intake output");
    assert!(!first.deduplicated, "first delivery is a fresh materialization");
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
    assert_eq!(listed.len(), 1, "exactly one output row exists after re-delivery");

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

/// SaveImage fallback marker + receipt: fallback marker idempotency, the
/// fallback sentinel slot, secret scrubbing of free-form provenance, and the
/// FALLBACK_ENGAGED / RECEIPT_PRODUCED events (6.9.5 / 6.9.6).
#[tokio::test]
async fn atelier_comfy_fallback_receipt_and_secret_scrub() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_comfy_fallback_receipt_and_secret_scrub: DATABASE_URL not set"
        );
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
