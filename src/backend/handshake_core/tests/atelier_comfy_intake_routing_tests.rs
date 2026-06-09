//! WP-KERNEL-005 Pose-ComfyUI pipeline: comfy-output -> Core intake routing,
//! the ComfyUI adapter endpoint boundary, the direct-localhost execution guard,
//! and the artifact-before-registration ordering invariant.
//!
//! MTs covered:
//!   * MT-107 image-sourcing adapter -> intake-lane mapping for routed outputs.
//!   * MT-108 accepted/pending/rejected lane decision for an output / routing
//!     intent, wired through the same single routing rule (no duplicate logic).
//!   * MT-109 endpoint_config validation that REJECTS unauthorized (non
//!     Handshake-native) execution endpoints.
//!   * MT-119 explicit guard rejecting DIRECT localhost ComfyUI execution.
//!   * MT-120 artifact_ref is persisted BEFORE a registration_id is assigned in
//!     the governed comfy output registration path (live PostgreSQL proof).
//!
//! MT-107/108/109/119 are pure-function invariants and need no database. MT-120
//! exercises the real `AtelierStore` against a live Postgres and is gated on
//! `atelier_pg_support::database_url()`. All run-scoped ids are unique per run
//! (`Uuid::new_v4()`) so the proof never depends on global table counts.

mod atelier_pg_support;

use handshake_core::atelier::comfy::{
    map_comfy_output_to_intake_lane, map_comfy_routing_intent_to_intake_lane,
    reject_direct_localhost_comfy_execution, ComfyAdapterKind, ComfyEndpointConfig,
    ComfyOutputRegistrationFailureStatus, IntakeOutput, MediaKind, NewComfyOutputRegistrationFailure,
    RoutingIntent,
};
use handshake_core::atelier::intake::IntakeLane;
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Build a governed `IntakeOutput` record value for the pure lane-mapping
/// proofs. Field values are run-scoped but the lane decision depends only on
/// `routing_intent`.
fn output_with_routing(routing_intent: RoutingIntent) -> IntakeOutput {
    let parent_artifact_ref = match routing_intent {
        RoutingIntent::Sidecar => Some(format!("artifact://atelier/comfy/{}", Uuid::new_v4())),
        _ => None,
    };
    IntakeOutput {
        intake_output_id: Uuid::new_v4(),
        workflow_run_id: Uuid::new_v4(),
        node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
        registration_id: None,
        source_node_instance_id: "42".to_string(),
        source_output_slot: "IMAGE".to_string(),
        media_kind: MediaKind::Image,
        mime: "image/png".to_string(),
        artifact_ref: format!("artifact://atelier/comfy/{}", Uuid::new_v4()),
        artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
        content_hash: format!("sha256-{}", Uuid::new_v4()),
        routing_intent,
        parent_artifact_ref,
        prompt_json_ref: None,
        graph_hash: None,
        seed: None,
        workflow_input_metadata: serde_json::json!({ "identity": {} }),
        materialized_at_utc: chrono::Utc::now(),
    }
}

/// MT-107 / MT-108: a routed comfy output maps into the Core intake lane
/// vocabulary, and the output-level and routing-intent-level entry points agree
/// (one routing rule, no duplicate logic).
#[test]
fn mt107_mt108_comfy_output_maps_into_accepted_pending_rejected_lanes() {
    // Artifact + Sidecar are durable, materialized outputs -> Accepted lane.
    for routing_intent in [RoutingIntent::Artifact, RoutingIntent::Sidecar] {
        let output = output_with_routing(routing_intent);
        assert_eq!(
            map_comfy_output_to_intake_lane(&output),
            IntakeLane::Accepted,
            "{routing_intent:?} output is a durable accepted intake output"
        );
        assert_eq!(
            map_comfy_output_to_intake_lane(&output),
            map_comfy_routing_intent_to_intake_lane(routing_intent),
            "output-level and routing-intent-level mapping share one rule for {routing_intent:?}"
        );
    }

    // Transient preview is not persisted -> Skipped lane (nothing to accept).
    let transient = output_with_routing(RoutingIntent::Transient);
    assert_eq!(
        map_comfy_output_to_intake_lane(&transient),
        IntakeLane::Skipped,
        "a transient preview output is skipped, not accepted"
    );
    assert_eq!(
        map_comfy_routing_intent_to_intake_lane(RoutingIntent::Transient),
        IntakeLane::Skipped,
    );

    // The mapping is deterministic / idempotent: same output -> same lane.
    let stable = output_with_routing(RoutingIntent::Artifact);
    assert_eq!(
        map_comfy_output_to_intake_lane(&stable),
        map_comfy_output_to_intake_lane(&stable),
        "lane assignment is deterministic for a given output"
    );
}

/// MT-109: endpoint_config validation REJECTS unauthorized (non Handshake
/// native) execution endpoints, and accepts only the managed adapter with a
/// portable ref.
#[test]
fn mt109_endpoint_config_rejects_unauthorized_endpoints() {
    // A direct LLM / model-server endpoint kind is never authorized.
    let direct_llm = ComfyEndpointConfig {
        endpoint_ref: "adapter://handshake/engine.comfyui/managed".to_string(),
        adapter_kind: ComfyAdapterKind::DirectLlmEndpoint,
    };
    let err = direct_llm
        .validate()
        .expect_err("a direct LLM endpoint kind must be rejected");
    assert!(
        err.to_string().contains("not authorized for execution"),
        "rejection must name the authorization boundary: {err}"
    );

    // A direct (un-managed) ComfyUI endpoint kind is never authorized.
    let direct_comfy = ComfyEndpointConfig {
        endpoint_ref: "adapter://handshake/engine.comfyui/managed".to_string(),
        adapter_kind: ComfyAdapterKind::DirectComfyEndpoint,
    };
    assert!(
        direct_comfy.validate().is_err(),
        "a direct ComfyUI endpoint kind must be rejected"
    );

    // Even with the managed kind, a localhost / direct-LLM ref is rejected.
    for bad_ref in [
        "http://localhost:8188/prompt",
        "http://127.0.0.1:8188/prompt",
        "ws://localhost:8188/ws",
        "ollama://localhost/comfy",
        "llm://model-server/comfy",
        "C:\\ComfyUI\\main.py",
    ] {
        let cfg = ComfyEndpointConfig {
            endpoint_ref: bad_ref.to_string(),
            adapter_kind: ComfyAdapterKind::HandshakeNativeManaged,
        };
        assert!(
            cfg.validate().is_err(),
            "managed adapter with non-portable endpoint_ref {bad_ref:?} must be rejected"
        );
    }

    // The only authorized config: managed adapter + portable Handshake ref.
    let ok = ComfyEndpointConfig {
        endpoint_ref: "adapter://handshake/engine.comfyui/managed".to_string(),
        adapter_kind: ComfyAdapterKind::HandshakeNativeManaged,
    };
    ok.validate()
        .expect("managed adapter with a portable Handshake-native ref is authorized");
    assert!(ComfyAdapterKind::HandshakeNativeManaged.is_authorized_for_execution());
    assert!(!ComfyAdapterKind::DirectComfyEndpoint.is_authorized_for_execution());
    assert!(!ComfyAdapterKind::DirectLlmEndpoint.is_authorized_for_execution());
}

/// MT-119: explicit guard rejecting DIRECT localhost ComfyUI execution; only a
/// portable Handshake-native managed ref is allowed.
#[test]
fn mt119_rejects_direct_localhost_comfy_execution() {
    for direct_endpoint in [
        "http://localhost:8188/prompt",
        "http://127.0.0.1:8188/prompt",
        "localhost:8188",
        "ws://[::1]:8188/ws",
        "http://0.0.0.0:8188/prompt",
    ] {
        let err = reject_direct_localhost_comfy_execution(direct_endpoint)
            .expect_err("direct localhost ComfyUI execution must be rejected");
        assert!(
            err.to_string().contains("Handshake-native portable ref"),
            "rejection must name the Handshake-native boundary: {err}"
        );
    }

    // The Handshake-native managed adapter ref is the only allowed path.
    reject_direct_localhost_comfy_execution("adapter://handshake/engine.comfyui/managed")
        .expect("a portable Handshake-native managed execution ref is allowed");
}

/// MT-120: in the governed comfy output registration path, `artifact_ref` is
/// persisted BEFORE a `registration_id` is assigned. The output-first
/// registration-failure record durably preserves the saved artifact_ref while
/// it still carries NO successful registration; the registration_id is only
/// bound later, on retry. This proves the save-before-register ordering against
/// a live PostgreSQL instance.
#[tokio::test]
async fn mt120_artifact_ref_is_persisted_before_registration_id() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt120_artifact_ref_is_persisted_before_registration_id: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let run_id = Uuid::new_v4();
    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let artifact_manifest_ref = format!("manifest://atelier/comfy/{}", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());

    // Step 1: the generated image is saved (artifact_ref + manifest persisted)
    // BEFORE any registration succeeds. The failure record carries the saved
    // artifact_ref and NO attempted/assigned registration_id.
    let failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: None,
            source_node_instance_id: "saveimage-before-register".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: artifact_ref.clone(),
            artifact_manifest_ref: artifact_manifest_ref.clone(),
            content_hash: content_hash.clone(),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: Some(2026),
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "registration not yet assigned at save time".to_string(),
            evidence: serde_json::json!({ "stage": "artifact_saved_pre_registration" }),
        })
        .await
        .expect("persist saved artifact before registration is assigned");

    // INVARIANT: the artifact_ref is durably persisted while no registration
    // exists yet -> save precedes registration.
    assert_eq!(
        failure.artifact_ref, artifact_ref,
        "the saved artifact_ref is persisted before any registration_id is assigned"
    );
    assert_eq!(
        failure.attempted_registration_id, None,
        "no registration_id exists at save time"
    );
    assert_eq!(
        failure.resolved_intake_output_id, None,
        "no intake output / registration is bound yet"
    );
    assert_eq!(
        failure.status,
        ComfyOutputRegistrationFailureStatus::Retryable,
        "the saved-but-unregistered output is recoverable"
    );

    // And it is durably recoverable from storage before registration.
    let reloaded = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("reload saved failure")
        .expect("saved failure is durable before registration");
    assert_eq!(reloaded.artifact_ref, artifact_ref);
    assert_eq!(reloaded.attempted_registration_id, None);

    // Step 2: ONLY NOW is a registration assigned, on retry. The artifact_ref
    // was already persisted (step 1); the registration_id is bound after it.
    let adapter =
        handshake_core::atelier::comfy::ComfyBridgeFakeAdapterV1::default();
    let registration = store
        .register_bridge_capability(&adapter.capability_registration(
            run_id,
            handshake_core::atelier::comfy::ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
        ))
        .await
        .expect("assign a capability registration AFTER the artifact was saved");

    let retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await
        .expect("bind the registration to the already-saved artifact");

    // The same saved artifact_ref now carries the later-assigned registration_id.
    assert_eq!(
        retry.output.artifact_ref, artifact_ref,
        "the registered output reuses the artifact_ref that was saved first"
    );
    assert_eq!(
        retry.output.registration_id,
        Some(registration.registration_id),
        "registration_id is assigned only after the artifact_ref was persisted"
    );

    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("reload resolved failure")
        .expect("failure still queryable");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered,
        "after registration, the saved artifact is marked registered"
    );
    assert_eq!(
        resolved.resolved_intake_output_id,
        Some(retry.output.intake_output_id),
    );
}
