//! MT-120 integration smoke for the distillation content-review
//! pipeline. Exhaustive coverage lives in the inline tests in
//! `distillation::content_review::tests` and
//! `distillation::pii_patterns::tests`; this file pins the cross-crate
//! API surface and the three MT-120 red_team minimum_controls.

use handshake_core::distillation::{
    content_review::{ContentReview, ContentReviewConfig, QuarantineReason, ReviewVerdict},
    corpus_extractor::TrainingTurn,
    pii_patterns::{scan, PiiKind},
};

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
        .review(&turn(
            "t-quarantine-1",
            "ok",
            "ok",
            "ProprietaryX",
        ))
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
        .review(&turn("t2", "novel prompt", "novel completion", "ProprietaryX"))
        .expect("review");
    assert!(matches!(verdict, ReviewVerdict::Pass { .. }));
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
