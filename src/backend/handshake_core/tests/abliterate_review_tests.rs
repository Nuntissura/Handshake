//! MT-107 integration coverage for abliteration artifact review and
//! Skill Bank gating.

use std::{collections::HashSet, fs, path::Path};

use handshake_core::distillation::{
    abliterate::AbliterationProvenance,
    abliterate_review::{
        AbliterationOutputReview, AbliterationOutputReviewConfig, AbliterationReviewFailure,
        ReviewResult, ABLITERATION_REVIEW_EVENT_ID, SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION,
    },
};
use tempfile::TempDir;

fn provenance() -> AbliterationProvenance {
    AbliterationProvenance {
        base_model_sha256: "a".repeat(64),
        refusal_direction_sha256: "b".repeat(64),
        abliteration_tool_version: "handshake-abliterate-mt106-v1".to_string(),
        abliterated_at_utc: "2026-05-21T00:00:00Z".to_string(),
        license_tag: "MIT".to_string(),
        operator_signature: "operator:test".to_string(),
        provenance_note: "reviewed offline artifact".to_string(),
        orthogonalised_weight_keys: vec!["model.layers.0.self_attn.o_proj.weight".to_string()],
        process_ledger_record_id: Some("ledger-row-1".to_string()),
    }
}

fn write_artifact(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("write artifact");
    path
}

fn assert_moved_to_quarantine(path: &Path, quarantine_path: &Path) {
    assert!(
        !path.exists(),
        "failed review must move the original artifact path"
    );
    assert!(
        quarantine_path.exists(),
        "quarantine path must preserve the artifact"
    );
    assert!(
        quarantine_path
            .components()
            .any(|part| part.as_os_str() == ".quarantine"),
        "quarantine path must live under .quarantine: {}",
        quarantine_path.display()
    );
}

#[test]
fn mt_107_missing_provenance_field_quarantines_and_blocks_skill_bank_registration() {
    let dir = TempDir::new().expect("tempdir");
    let artifact = write_artifact(&dir, "abliterated.safetensors", b"fake weights");
    let reviewer = AbliterationOutputReview::default();
    let mut provenance = provenance();
    provenance.base_model_sha256.clear();

    let verdict = reviewer.check(&artifact, &provenance).expect("review");
    match verdict {
        ReviewResult::Fail {
            reasons,
            quarantine_path,
            event,
        } => {
            assert_eq!(event.event_id, ABLITERATION_REVIEW_EVENT_ID);
            assert_eq!(event.result, "Fail");
            assert!(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    AbliterationReviewFailure::MissingProvenanceField { field }
                        if *field == "base_model_sha256"
                )
            }));
            let quarantine_path = quarantine_path.expect("quarantine path");
            assert_moved_to_quarantine(&artifact, &quarantine_path);
        }
        other => panic!("expected Fail, got {other:?}"),
    }

    let second_artifact = write_artifact(&dir, "abliterated-2.safetensors", b"fake weights");
    let err = reviewer
        .register_abliterated_model(&second_artifact, &provenance)
        .expect_err("Skill Bank registration must be blocked");
    assert!(err.reasons().iter().any(|reason| {
        matches!(
            reason,
            AbliterationReviewFailure::MissingProvenanceField { field }
                if *field == "base_model_sha256"
        )
    }));
}

#[test]
fn mt_107_empty_license_quarantines_as_untaggable_license() {
    let dir = TempDir::new().expect("tempdir");
    let artifact = write_artifact(&dir, "abliterated.safetensors", b"fake weights");
    let reviewer = AbliterationOutputReview::default();
    let mut provenance = provenance();
    provenance.license_tag = "   ".to_string();

    let verdict = reviewer.check(&artifact, &provenance).expect("review");
    match verdict {
        ReviewResult::Fail {
            reasons,
            quarantine_path,
            ..
        } => {
            assert!(reasons
                .iter()
                .any(|reason| matches!(reason, AbliterationReviewFailure::UntaggableLicense)));
            assert_moved_to_quarantine(&artifact, &quarantine_path.expect("quarantine"));
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn mt_107_license_not_in_allowlist_quarantines_artifact() {
    let dir = TempDir::new().expect("tempdir");
    let artifact = write_artifact(&dir, "abliterated.safetensors", b"fake weights");
    let reviewer = AbliterationOutputReview::default();
    let mut provenance = provenance();
    provenance.license_tag = "Proprietary-X".to_string();

    let verdict = reviewer.check(&artifact, &provenance).expect("review");
    match verdict {
        ReviewResult::Fail {
            reasons,
            quarantine_path,
            ..
        } => {
            assert!(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    AbliterationReviewFailure::LicenseNotAllowed { license_tag }
                        if license_tag == "Proprietary-X"
                )
            }));
            assert_moved_to_quarantine(&artifact, &quarantine_path.expect("quarantine"));
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn mt_107_complete_provenance_allowed_license_returns_skill_bank_gate_token() {
    let dir = TempDir::new().expect("tempdir");
    let artifact = write_artifact(&dir, "abliterated.safetensors", b"fake weights");
    let reviewer = AbliterationOutputReview::default();
    let provenance = provenance();

    let verdict = reviewer.check(&artifact, &provenance).expect("review");
    match verdict {
        ReviewResult::Pass {
            artifact_path,
            event,
            skill_bank_registration,
        } => {
            assert_eq!(artifact_path, artifact);
            assert_eq!(event.event_id, ABLITERATION_REVIEW_EVENT_ID);
            assert_eq!(event.result, "Pass");
            assert_eq!(
                skill_bank_registration.action,
                SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION
            );
            assert_eq!(skill_bank_registration.artifact_path, artifact);
            assert_eq!(
                skill_bank_registration.base_model_sha256,
                provenance.base_model_sha256
            );
            assert_eq!(
                skill_bank_registration.refusal_direction_sha256,
                provenance.refusal_direction_sha256
            );
            assert_eq!(skill_bank_registration.license_tag, "MIT");
        }
        other => panic!("expected Pass, got {other:?}"),
    }
    assert!(
        artifact.exists(),
        "passing review must not move the artifact"
    );
}

#[test]
fn mt_107_metadata_pii_quarantines_but_weight_bytes_are_not_scanned() {
    let dir = TempDir::new().expect("tempdir");
    let reviewer = AbliterationOutputReview::default();

    let artifact_with_pii_bytes = write_artifact(
        &dir,
        "weights-contain-email.safetensors",
        b"binary-ish fake weights alice@example.com",
    );
    let clean_provenance = provenance();
    assert!(matches!(
        reviewer
            .check(&artifact_with_pii_bytes, &clean_provenance)
            .expect("review"),
        ReviewResult::Pass { .. }
    ));
    assert!(
        artifact_with_pii_bytes.exists(),
        "review scans metadata text, not weight bytes"
    );

    let metadata_pii_artifact = write_artifact(&dir, "metadata-pii.safetensors", b"fake weights");
    let mut metadata_pii = provenance();
    metadata_pii.provenance_note = "operator metadata C:\\Users\\Ilja".to_string();
    let verdict = reviewer
        .check(&metadata_pii_artifact, &metadata_pii)
        .expect("review");
    match verdict {
        ReviewResult::Fail {
            reasons,
            quarantine_path,
            ..
        } => {
            assert!(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    AbliterationReviewFailure::PiiDetected { kind, .. }
                        if kind == "windows_user_path"
                )
            }));
            assert_moved_to_quarantine(
                &metadata_pii_artifact,
                &quarantine_path.expect("quarantine"),
            );
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn mt_107_operator_configurable_allowlist_can_allow_internal_license() {
    let dir = TempDir::new().expect("tempdir");
    let artifact = write_artifact(&dir, "abliterated.safetensors", b"fake weights");
    let config = AbliterationOutputReviewConfig {
        license_allowlist: HashSet::from(["Operator-Internal".to_string()]),
        quarantine_root: None,
    };
    let reviewer = AbliterationOutputReview::new(config);
    let mut provenance = provenance();
    provenance.license_tag = "Operator-Internal".to_string();

    assert!(matches!(
        reviewer.check(&artifact, &provenance).expect("review"),
        ReviewResult::Pass { .. }
    ));
}
