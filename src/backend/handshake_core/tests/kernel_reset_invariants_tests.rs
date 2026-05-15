use handshake_core::kernel::{
    fold_manifest::{kernel002_fold_manifest, LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID},
    reset_invariants::{kernel002_reset_invariant_matrix, LegacyResetTopic, ResetDisposition},
};

#[test]
fn kernel002_reset_invariants_cover_all_legacy_topics() {
    let manifest = kernel002_fold_manifest();
    let matrix = kernel002_reset_invariant_matrix();

    assert_eq!(matrix.wp_id, manifest.wp_id);
    matrix
        .verify_against_fold_manifest(&manifest)
        .expect("reset invariant matrix must stay attached to folded source manifest");

    for topic in [
        LegacyResetTopic::LegacyLocalAuthority,
        LegacyResetTopic::MarkdownAuthority,
        LegacyResetTopic::MailboxChronology,
        LegacyResetTopic::UiLocalTruth,
    ] {
        let entries = matrix.entries_for_topic(topic);
        assert!(
            !entries.is_empty(),
            "reset invariant topic must have explicit conversions: {:?}",
            topic
        );

        for entry in entries {
            assert!(
                !entry.legacy_assumption.is_empty(),
                "legacy assumption must be preserved for {}",
                entry.source_stub_id
            );
            assert!(
                !entry.kernel_semantics.is_empty(),
                "kernel reset semantics must be explicit for {}",
                entry.source_stub_id
            );
            assert!(
                entry.reset_disposition.is_allowed_kernel002_disposition(),
                "reset disposition must be one of the reset-approved outcomes for {}",
                entry.source_stub_id
            );
        }
    }
}

#[test]
fn sqlite_obligations_reset_to_postgres_eventledger_crdt_authority() {
    let matrix = kernel002_reset_invariant_matrix();
    let sqlite_entries = matrix.entries_for_topic(LegacyResetTopic::LegacyLocalAuthority);
    let sqlite_source_ids: Vec<&str> = sqlite_entries
        .iter()
        .map(|entry| entry.source_stub_id)
        .collect();

    for required_source in [
        LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID,
        "WP-1-FEMS-Write-Time-Safeguards-v1",
        "WP-1-Locus-Work-Tracking-System-Phase1-v1",
    ] {
        assert!(
            sqlite_source_ids.contains(&required_source),
            "SQLite reset coverage missing for {required_source}"
        );
    }

    for entry in sqlite_entries {
        assert_eq!(
            entry.reset_disposition,
            ResetDisposition::PostgresEventLedgerCrdtAuthority
        );
        assert!(
            entry.kernel_semantics.contains("Postgres/EventLedger/CRDT"),
            "SQLite reset must name the replacement authority stack for {}",
            entry.source_stub_id
        );
        assert!(
            !entry.kernel_semantics.contains("SQLite authority"),
            "SQLite must not remain authoritative for {}",
            entry.source_stub_id
        );
    }
}

#[test]
fn markdown_mailbox_and_ui_truth_reset_to_non_authority_or_promotion_gate() {
    let matrix = kernel002_reset_invariant_matrix();

    let markdown_entries = matrix.entries_for_topic(LegacyResetTopic::MarkdownAuthority);
    assert!(markdown_entries
        .iter()
        .any(|entry| entry.source_stub_id == "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1"));
    assert!(markdown_entries.iter().all(|entry| matches!(
        entry.reset_disposition,
        ResetDisposition::ProjectionOrAdvisory
    )));

    let mailbox_entries = matrix.entries_for_topic(LegacyResetTopic::MailboxChronology);
    assert!(mailbox_entries
        .iter()
        .any(|entry| entry.source_stub_id == "WP-1-Role-Mailbox-Message-Thread-Contract-v1"));
    assert!(mailbox_entries.iter().any(|entry| entry.source_stub_id
        == "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1"));
    assert!(mailbox_entries.iter().all(|entry| matches!(
        entry.reset_disposition,
        ResetDisposition::ProjectionOrAdvisory | ResetDisposition::PromotionGatedAction
    )));

    let ui_entries = matrix.entries_for_topic(LegacyResetTopic::UiLocalTruth);
    assert!(ui_entries
        .iter()
        .any(|entry| entry.source_stub_id == "WP-1-Dev-Command-Center-MVP-v1"));
    assert!(ui_entries.iter().all(|entry| matches!(
        entry.reset_disposition,
        ResetDisposition::ProjectionOrAdvisory | ResetDisposition::PromotionGatedAction
    )));
}
