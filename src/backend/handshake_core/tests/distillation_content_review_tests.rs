//! MT-120 integration smoke for the distillation content-review
//! pipeline. Exhaustive coverage lives in the inline tests in
//! `distillation::content_review::tests` and
//! `distillation::pii_patterns::tests`; this file pins the cross-crate
//! API surface and the three MT-120 red_team minimum_controls.

use handshake_core::distillation::{
    content_review::{
        ContentReview, ContentReviewConfig, QuarantineReason, ReviewVerdict,
        FR_EVT_DISTILL_PII_DETECT,
    },
    corpus_extractor::TrainingTurn,
    pii_patterns::{scan, PiiKind},
};
use handshake_core::flight_recorder::fr_event_registry::FrEventId;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};
use serde_json::json;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct MemoryFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for MemoryFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("recorder lock").clone())
    }
}

fn turn(id: &str, prompt: &str, completion: &str, license: &str) -> TrainingTurn {
    TrainingTurn {
        id: id.to_string(),
        session_id: "session".to_string(),
        model_id: "model".to_string(),
        prompt: prompt.to_string(),
        completion: completion.to_string(),
        finish_reason: Some("stop".to_string()),
        license_tag: license.to_string(),
        source_event_ids: vec!["e1".to_string()],
        sourced_at_utc: "2026-05-20T03:00:00Z".to_string(),
    }
}

#[test]
fn content_review_explicit_sexual_content_not_flagged_as_pii() {
    // MT-120 red_team minimum_controls[0]: GLOBAL-PRODUCTION discipline.
    let hits = scan("operator-authored: pussy, tits, cock, monster dick verbatim");
    assert!(
        hits.is_empty(),
        "PII scanner must not treat explicit operator content as PII; got {hits:?}"
    );

    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let verdict = reviewer
        .review(&turn(
            "t1",
            "operator prompt: pussy, tits, cock",
            "operator completion: verbatim",
            "custom_internal",
        ))
        .expect("review");
    assert!(matches!(verdict, ReviewVerdict::Pass { .. }));
}

#[test]
fn content_review_quarantine_path_is_recorded_never_deleted() {
    // MT-120 red_team minimum_controls[1]: Quarantine moves never
    // delete. The reviewer records the quarantine_path; the caller
    // performs the file move. Verify the path is well-formed and
    // contains the turn id so a downstream mover has a stable target.
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let verdict = reviewer
        .review(&turn("t-quarantine-1", "ok", "ok", "ProprietaryX"))
        .expect("review");
    match verdict {
        ReviewVerdict::Quarantine {
            turn_id,
            quarantine_path,
            reasons,
        } => {
            assert_eq!(turn_id, "t-quarantine-1");
            assert!(
                quarantine_path.contains("t-quarantine-1"),
                "quarantine_path {quarantine_path} must reference the turn id"
            );
            assert!(reasons
                .iter()
                .any(|r| matches!(r, QuarantineReason::LicenseNotAllowed { .. })));
        }
        other => panic!("expected Quarantine, got {other:?}"),
    }
}

#[test]
fn content_review_license_allowlist_is_operator_configurable() {
    // MT-120 red_team minimum_controls[2]: operator can swap the
    // allowlist (e.g. an internal-only operator may exclude MIT).
    let mut cfg = ContentReviewConfig::defaults();
    cfg.license_allowlist.clear();
    cfg.license_allowlist.insert("ProprietaryX".to_string());
    let mut reviewer = ContentReview::new(cfg);

    // MIT no longer passes.
    let verdict = reviewer
        .review(&turn("t1", "ok", "ok", "MIT"))
        .expect("review");
    assert!(matches!(verdict, ReviewVerdict::Quarantine { .. }));

    // ProprietaryX now passes (assuming no PII / no dedup hit).
    let verdict = reviewer
        .review(&turn(
            "t2",
            "novel prompt",
            "novel completion",
            "ProprietaryX",
        ))
        .expect("review");
    assert!(matches!(verdict, ReviewVerdict::Pass { .. }));
}

#[test]
fn content_review_near_duplicate_non_exact_pair_is_quarantined() {
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let first = reviewer
        .review(&turn(
            "near-dup-base",
            "Explain why distillation candidates need review gates.",
            "Candidates need review gates so raw trace fragments do not become Skill Bank entries automatically.",
            "MIT",
        ))
        .expect("first review");
    assert!(matches!(first, ReviewVerdict::Pass { .. }));

    let second = reviewer
        .review(&turn(
            "near-dup-reordered",
            "Explain why distillation candidates need review gates.",
            "Candidates need review gates so raw trace fragments do not automatically become Skill Bank entries.",
            "MIT",
        ))
        .expect("second review");

    match second {
        ReviewVerdict::Quarantine { reasons, .. } => {
            assert!(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    QuarantineReason::NearDuplicateOfTurn {
                        existing_turn_id,
                        detector,
                        similarity_milli
                    } if existing_turn_id == "near-dup-base"
                        && detector == "lexical_token_cosine_v1"
                        && *similarity_milli >= 950
                )
            }));
        }
        other => panic!("expected near duplicate quarantine, got {other:?}"),
    }
}

#[test]
fn content_review_same_prompt_distinct_completion_is_not_near_duplicate() {
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let first = reviewer
        .review(&turn(
            "answer-a",
            "How should the operator handle a failed validation?",
            "Record the validator failure, preserve the evidence, and route the same work packet back for repair.",
            "MIT",
        ))
        .expect("first review");
    assert!(matches!(first, ReviewVerdict::Pass { .. }));

    let second = reviewer
        .review(&turn(
            "answer-b",
            "How should the operator handle a failed validation?",
            "Open the calendar panel, inspect tomorrow's meetings, and draft a scheduling note for the team.",
            "MIT",
        ))
        .expect("second review");
    assert!(matches!(second, ReviewVerdict::Pass { .. }));
}

#[test]
fn content_review_different_prompt_same_boilerplate_completion_is_not_near_duplicate() {
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let first = reviewer
        .review(&turn(
            "prompt-a",
            "Summarize the sandbox lifecycle evidence for MT-046.",
            "This item should be reviewed by the operator before promotion because the evidence is incomplete.",
            "MIT",
        ))
        .expect("first review");
    assert!(matches!(first, ReviewVerdict::Pass { .. }));

    let second = reviewer
        .review(&turn(
            "prompt-b",
            "Summarize the memory retrieval calibration evidence for MT-157.",
            "This item should be reviewed by the operator before promotion because the evidence is incomplete.",
            "MIT",
        ))
        .expect("second review");
    assert!(matches!(second, ReviewVerdict::Pass { .. }));
}

#[test]
fn content_review_pii_kinds_have_stable_string_labels() {
    // Telemetry filters key on these labels; pin them so refactors are
    // explicit.
    assert_eq!(PiiKind::Email.label(), "email");
    assert_eq!(PiiKind::Phone.label(), "phone");
    assert_eq!(PiiKind::CreditCard.label(), "credit_card");
    assert_eq!(PiiKind::ApiKey.label(), "api_key");
    assert_eq!(PiiKind::WindowsUserPath.label(), "windows_user_path");
    assert_eq!(PiiKind::MacAddress.label(), "mac_address");
    assert_eq!(PiiKind::Ipv4.label(), "ipv4");
}

#[test]
fn content_review_pii_outcome_builds_privacy_preserving_flight_recorder_event() {
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let outcome = reviewer
        .review_with_events(&turn(
            "pii-review-1",
            "Contact alice@example.com about the run.",
            "Call +1 555 123 4567 once the trace is ready.",
            "MIT",
        ))
        .expect("review with events");

    let job_id = "job-content-review-pii";
    let events = outcome.flight_recorder_events(Uuid::now_v7(), job_id);
    assert_eq!(
        events.len(),
        1,
        "PII detections should aggregate into one FR event"
    );

    let event = &events[0];
    assert_eq!(event.job_id.as_deref(), Some(job_id));
    assert_eq!(event.actor, FlightRecorderActor::System);
    assert_eq!(
        event.event_type,
        FlightRecorderEventType::DistillPiiDetected
    );
    assert!(event.validate().is_ok(), "{:?}", event.validate());
    assert_eq!(event.payload["type"], "distill.pii_detected");
    assert_eq!(
        event.payload["fr_event_id"],
        FrEventId::DistillPiiDetect.as_str()
    );
    assert_eq!(event.payload["fr_event_id"], FR_EVT_DISTILL_PII_DETECT);
    assert_eq!(event.payload["turn_id"], "pii-review-1");
    assert_eq!(event.payload["severity"], "Medium");
    assert_eq!(event.payload["pii_kinds"], json!(["email", "phone"]));
    assert!(event.payload.get("job_id").is_none());

    let payload_text = serde_json::to_string(&event.payload).expect("payload serializes");
    assert!(!payload_text.contains("alice@example.com"));
    assert!(!payload_text.contains("555 123 4567"));
    assert!(!payload_text.contains("Contact "));
    assert!(!payload_text.contains("Call "));
}

#[test]
fn content_review_pii_flight_recorder_event_rejects_inline_sensitive_content_fields() {
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::DistillPiiDetected,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({
            "type": "distill.pii_detected",
            "fr_event_id": FR_EVT_DISTILL_PII_DETECT,
            "turn_id": "pii-review-raw",
            "pii_kinds": ["email"],
            "severity": "Medium",
            "raw_prompt": "alice@example.com"
        }),
    );

    assert!(
        event.validate().is_err(),
        "PII telemetry must reject raw prompt/completion/PII payload fields"
    );
}

#[tokio::test]
async fn content_review_pii_outcome_records_through_flight_recorder_trait() {
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let outcome = reviewer
        .review_with_events(&turn(
            "pii-review-record",
            "Contact alice@example.com about the run.",
            "No raw source text may be logged.",
            "MIT",
        ))
        .expect("review with events");
    let recorder = MemoryFlightRecorder::default();

    let recorded = outcome
        .record_flight_recorder_events(&recorder, Uuid::now_v7(), "job-content-review-record")
        .await
        .expect("record content review FR events");
    assert_eq!(recorded, 1);

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_type,
        FlightRecorderEventType::DistillPiiDetected
    );
    assert_eq!(
        events[0].job_id.as_deref(),
        Some("job-content-review-record")
    );
    let payload_text = serde_json::to_string(&events[0].payload).expect("payload serializes");
    assert!(!payload_text.contains("alice@example.com"));
    assert!(!payload_text.contains("Contact "));
}
