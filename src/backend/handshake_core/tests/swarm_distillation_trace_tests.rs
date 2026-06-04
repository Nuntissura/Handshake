use chrono::{TimeZone, Utc};
use handshake_core::distillation::swarm_trace::{
    capture_distillation_swarm_trace_at, prepare_distillation_swarm_trace_queue_entry_at,
    SwarmTraceCandidate, SwarmTraceEligibility, SwarmTraceError, SwarmTraceIneligibleReason,
    SwarmTraceOutputSource, SwarmTraceRouteMetadata, SwarmTraceRouteOutcome,
    DISTILLATION_SWARM_TRACE_QUEUE_SCHEMA, DISTILLATION_SWARM_TRACE_VERSION,
};
use uuid::Uuid;

fn candidate() -> SwarmTraceCandidate {
    SwarmTraceCandidate::new(
        Uuid::now_v7(),
        "sample-207",
        "Explain how warm VM restore changes first-token latency.",
        "local:warm-vm-student",
        "The local model reuses a restored loaded VM and streams frames.",
        "cloud:teacher",
        "The cloud path provides a comparison answer for distillation review.",
    )
    .with_comparison_labels(vec![
        "warm-start".to_string(),
        "teacher-student".to_string(),
    ])
    .with_route_metadata(
        SwarmTraceRouteMetadata::new(
            "session://swarm/wt-207#local-cloud",
            "local confidence below configured threshold; escalated to cloud",
            SwarmTraceRouteOutcome::CloudEscalated,
            "local-model-id-warm-vm",
            "cloud-model-id-teacher",
        )
        .with_local_confidence_basis_points(7_250)
        .with_validation_labels(vec!["low-confidence".to_string(), "cloud-win".to_string()]),
    )
}

#[test]
fn swarm_trace_bundle_preserves_labeled_local_and_cloud_outputs() {
    let captured_at = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();

    let bundle = capture_distillation_swarm_trace_at(candidate(), captured_at)
        .expect("eligible trace captures");

    assert_eq!(bundle.version, DISTILLATION_SWARM_TRACE_VERSION);
    assert_eq!(bundle.sample_id, "sample-207");
    assert_eq!(bundle.outputs.len(), 2);
    assert_eq!(bundle.outputs[0].source, SwarmTraceOutputSource::Local);
    assert_eq!(bundle.outputs[0].label, "local:warm-vm-student");
    assert_eq!(bundle.outputs[1].source, SwarmTraceOutputSource::Cloud);
    assert_eq!(bundle.outputs[1].label, "cloud:teacher");
    let route = bundle.route.as_ref().expect("route metadata captured");
    assert_eq!(route.session_id, "session://swarm/wt-207#local-cloud");
    assert_eq!(
        route.route_reason,
        "local confidence below configured threshold; escalated to cloud"
    );
    assert_eq!(route.route_outcome, SwarmTraceRouteOutcome::CloudEscalated);
    assert_eq!(route.local_model_id, "local-model-id-warm-vm");
    assert_eq!(route.cloud_model_id, "cloud-model-id-teacher");
    assert_eq!(route.local_confidence_basis_points, Some(7_250));
    assert_eq!(
        route.validation_labels,
        vec!["low-confidence".to_string(), "cloud-win".to_string()]
    );
    assert_eq!(
        bundle.comparison_labels,
        vec!["warm-start".to_string(), "teacher-student".to_string()]
    );
    assert!(bundle.eligibility.distillation_allowed);
    assert_eq!(bundle.captured_at_utc, captured_at);
}

#[test]
fn swarm_trace_queue_entry_redacts_prompt_and_outputs_without_erasing_safe_context() {
    let mut raw = candidate();
    raw.prompt = "Use OPENAI_API_KEY=sk-trace-secret for user@example.com when testing warm traces"
        .to_string();
    raw.local_output =
        "Local token saw Bearer sk-local-secret and 555-123-4567 while streaming".to_string();
    raw.cloud_output =
        "Cloud token saw api_key=sk-cloud-secret while comparing teacher output".to_string();

    let queue_entry = prepare_distillation_swarm_trace_queue_entry_at(raw, Utc::now())
        .expect("redacted trace remains eligible for queue entry");
    assert_eq!(queue_entry.schema, DISTILLATION_SWARM_TRACE_QUEUE_SCHEMA);
    assert!(queue_entry.eligible_for_training_review);
    let bundle = queue_entry.bundle;

    assert!(!bundle.prompt.contains("sk-trace-secret"));
    assert!(!bundle.prompt.contains("user@example.com"));
    assert!(!bundle.outputs[0].output.contains("sk-local-secret"));
    assert!(!bundle.outputs[0].output.contains("555-123-4567"));
    assert!(!bundle.outputs[1].output.contains("sk-cloud-secret"));
    assert!(bundle.prompt.contains("testing warm traces"));
    assert!(bundle.prompt.contains("[REDACTED_ENV]"));
    assert!(bundle.prompt.contains("[REDACTED_EMAIL]"));
    assert!(bundle.outputs[0].output.contains("while streaming"));
    assert!(bundle.outputs[0].output.contains("[REDACTED_SECRET]"));
    assert!(bundle.outputs[0].output.contains("[REDACTED_PHONE]"));
    assert!(bundle.outputs[1].output.contains("teacher output"));
    assert!(bundle.outputs[1].output.contains("[REDACTED_SECRET]"));
    assert!(bundle
        .redactions_applied
        .iter()
        .any(|r| r.field == "prompt" && r.secrets_found && r.pii_found));
    assert!(bundle
        .redactions_applied
        .iter()
        .any(|r| r.field == "outputs.local.output" && r.secrets_found && r.pii_found));
    assert!(bundle
        .redactions_applied
        .iter()
        .any(|r| r.field == "outputs.cloud.output" && r.secrets_found));
}

#[test]
fn swarm_trace_refuses_ineligible_or_empty_training_examples() {
    let ineligible = candidate().with_eligibility(SwarmTraceEligibility {
        distillation_allowed: false,
        local_output_allowed: true,
        cloud_output_allowed: true,
    });
    let err = prepare_distillation_swarm_trace_queue_entry_at(ineligible, Utc::now())
        .expect_err("distillation-disabled traces must not queue");
    assert_eq!(
        err,
        SwarmTraceError::Ineligible {
            reasons: vec![SwarmTraceIneligibleReason::DistillationNotAllowed]
        }
    );

    let mut empty_local = candidate();
    empty_local.local_output.clear();
    let err = prepare_distillation_swarm_trace_queue_entry_at(empty_local, Utc::now())
        .expect_err("empty local output must not queue as a teacher/student sample");
    assert!(
        matches!(err, SwarmTraceError::Ineligible { reasons } if reasons.contains(&SwarmTraceIneligibleReason::EmptyLocalOutput))
    );
}

#[test]
fn swarm_trace_refuses_route_metadata_without_model_identity() {
    let raw = candidate().with_route_metadata(SwarmTraceRouteMetadata::new(
        "session://swarm/wt-207#bad",
        "route selected cloud",
        SwarmTraceRouteOutcome::CloudEscalated,
        " ",
        "cloud-model-id-teacher",
    ));

    let err = prepare_distillation_swarm_trace_queue_entry_at(raw, Utc::now())
        .expect_err("route metadata without a local model id must not queue");

    assert!(
        matches!(err, SwarmTraceError::Ineligible { reasons } if reasons.contains(&SwarmTraceIneligibleReason::EmptyLocalModelId))
    );
}
