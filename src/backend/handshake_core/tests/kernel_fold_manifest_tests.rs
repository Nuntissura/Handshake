use handshake_core::kernel::fold_manifest::{
    kernel002_fold_manifest, FoldClassification, FoldManifestError,
    LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID,
};

const WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

#[test]
fn kernel002_fold_manifest_preserves_all_source_stubs() {
    let manifest = kernel002_fold_manifest();

    assert_eq!(manifest.wp_id, WP_ID);
    assert_eq!(manifest.source_stubs.len(), 39);
    assert!(manifest.import_rule.contains("identity"));
    assert!(manifest.import_rule.contains("intent"));
    assert!(manifest.import_rule.contains("scope"));
    assert!(manifest.import_rule.contains("acceptance"));
    assert!(manifest.import_rule.contains("risks"));

    let direct = manifest
        .source_stubs
        .iter()
        .filter(|entry| entry.fold_classification == FoldClassification::Direct)
        .count();
    let transitive = manifest
        .source_stubs
        .iter()
        .filter(|entry| entry.fold_classification == FoldClassification::Transitive)
        .count();
    assert_eq!(direct, 32);
    assert_eq!(transitive, 7);

    for entry in manifest.source_stubs {
        assert!(
            entry.source_path.starts_with(".GOV/task_packets/stubs/WP-"),
            "unexpected source path: {}",
            entry.source_path
        );
        assert!(
            entry.source_path.ends_with(".md"),
            "source path must point to source markdown: {}",
            entry.source_path
        );
        assert_eq!(
            entry.pre_fold_sha256.len(),
            64,
            "pre-fold hash must be sha256 hex for {}",
            entry.stub_id
        );
        assert!(
            entry
                .pre_fold_sha256
                .chars()
                .all(|ch| ch.is_ascii_hexdigit()),
            "pre-fold hash must be hex for {}",
            entry.stub_id
        );
        assert!(
            entry.source_scope_import.contains("identity")
                && entry.source_scope_import.contains("intent")
                && entry.source_scope_import.contains("scope")
                && entry.source_scope_import.contains("acceptance")
                && entry.source_scope_import.contains("risks"),
            "source import instruction dropped required detail categories for {}",
            entry.stub_id
        );
    }

    let sqlite_boundary = manifest
        .source_stubs
        .iter()
        .find(|entry| entry.stub_id == LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID)
        .expect("SQLite boundary source must stay folded");
    assert_eq!(
        sqlite_boundary.fold_classification,
        FoldClassification::Transitive
    );
    assert_eq!(sqlite_boundary.reset_override, Some("reset_invariant"));
    assert!(sqlite_boundary
        .source_scope_import
        .contains("Postgres/EventLedger/CRDT"));

    assert!(manifest
        .source_stubs
        .iter()
        .any(|entry| entry.stub_id
            == "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1"));
}

#[test]
fn kernel002_fold_manifest_rejects_missing_or_mismatched_sources() {
    let manifest = kernel002_fold_manifest();
    let observed: Vec<(&str, &str)> = manifest
        .source_stubs
        .iter()
        .map(|entry| (entry.source_path, entry.pre_fold_sha256))
        .collect();

    assert!(manifest.verify_observed_sources(&observed).is_ok());

    let missing = &observed[..observed.len() - 1];
    let missing_errors = manifest
        .verify_observed_sources(missing)
        .expect_err("missing source must fail activation preflight");
    assert!(missing_errors.iter().any(|error| matches!(
        error,
        FoldManifestError::MissingSource { source_path }
            if *source_path == ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1.md"
    )));

    let bad_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let mut mismatched = observed;
    mismatched[0].1 = bad_hash;
    let mismatch_errors = manifest
        .verify_observed_sources(&mismatched)
        .expect_err("hash mismatch must fail activation preflight");
    assert!(mismatch_errors.iter().any(|error| matches!(
        error,
        FoldManifestError::HashMismatch {
            source_path,
            expected_sha256,
            observed_sha256,
        } if *source_path == ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.md"
            && *expected_sha256 == "f160424f7dd05647fec455d6eee7acbd0f1774d58d4b948963d4af9c58cce5a7"
            && *observed_sha256 == bad_hash
    )));
}
