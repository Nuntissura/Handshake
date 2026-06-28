//! WP-KERNEL-012 MT-095 — the HBR-INT-009 three-tier diagnostic evidence format, end-to-end proofs.
//!
//! - AC-015-1 / PT-015-A: emit a well-formed `test_run_with_three_tier_diagnostic_wiring_record.json`
//!   to the EXTERNAL artifact root (atomic write), then parse it back into the typed struct.
//! - AC-015-3 / PT-015-C: the serialized tier/status spellings are congruent with the
//!   `hbr_int_009_tier_obligations` contract vocabulary.
//! - AC-015-4 / PT-015-C: the emitted record is a REAL example grounded in this WP's actual tiers
//!   (the UI-thread heartbeat: FLIGHT_RECORDER not-applicable, INTERNAL_DIAGNOSTICS wired to MT-084,
//!   PALMISTRY wired to MT-091) and passes `validate()`.
//!
//! Artifact hygiene (CX-212E, HARD): every artifact is written ONLY to the external
//! `Handshake_Artifacts/handshake-test` root; `assert_no_local_artifact_dir` fails the test if a
//! repo-local `test_output/` or `tests/screenshots/` directory ever appears.
//!
//! The pure-logic pass/fail/duplicate/empty-evidence cases (PT-015-B) live in the in-crate unit tests
//! in `src/three_tier_evidence.rs`; these integration tests cover the filesystem + grounding ACs.

use std::path::{Path, PathBuf};

use handshake_diag_ring::three_tier_evidence::congruence;
use handshake_diag_ring::{
    run_at_now, DiagTier, ThreeTierDiagnosticWiringRecord, TierWiring, EVIDENCE_FILE_NAME,
};

/// Crate-relative path to the external artifacts root (CX-212E), disk-agnostic. This crate sits at
/// `<repo>/src/frontend/handshake_native/diag_ring`, so FIVE `..` reach `<repo>/..` where
/// `Handshake_Artifacts` is a sibling of the repo worktree. (The handshake_native crate one level up
/// uses FOUR `..`; this nested crate uses FIVE — matching `.cargo/config.toml`.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Fail if any repo-local test-artifact directory exists under the crate (CX-212E artifact hygiene).
/// Artifacts go to the external root ONLY; a stray `test_output/` or `tests/screenshots/` is a
/// hygiene regression and a tracked one under `src/` is a committed-artifact failure.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let path = Path::new(local);
        assert!(
            !path.exists(),
            "CX-212E: no repo-local `{local}` dir may exist — three-tier evidence artifacts go to \
             the external Handshake_Artifacts/handshake-test root only (found {})",
            path.display()
        );
    }
}

/// The REAL grounded example record (AC-015-4): the UI-thread heartbeat observable behavior.
///
/// Grounding in this WP's actual tiers:
/// - FLIGHT_RECORDER = NOT_APPLICABLE — the per-frame heartbeat is a Tier-2/Tier-3 liveness signal,
///   deliberately NOT written to the Tier-1 business-event ledger (§5.8.6: internal_diagnostics
///   SUPPLEMENTS the Flight Recorder, it does not push per-frame noise into it).
/// - INTERNAL_DIAGNOSTICS = WIRED — MT-084 records `DiagEvent::heartbeat` every egui frame via
///   `diagnostics::recorder::heartbeat`; proven by `test_heartbeat.rs`.
/// - PALMISTRY = WIRED — the external watcher reads the heartbeat from the MT-081 ring under a bounded
///   seqlock read to detect a UI freeze (MT-091 freeze detection).
fn heartbeat_example_record() -> ThreeTierDiagnosticWiringRecord {
    ThreeTierDiagnosticWiringRecord::new(
        "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        "MT-095",
        "ui_thread_heartbeat",
        run_at_now(),
        vec![
            TierWiring::not_applicable(
                DiagTier::FlightRecorder,
                "Per-frame UI heartbeat is a Tier-2/Tier-3 liveness signal; deliberately not written \
                 to the Tier-1 Flight Recorder business-event ledger (§5.8.6 supplements FR).",
            ),
            TierWiring::wired(
                DiagTier::InternalDiagnostics,
                "MT-084: diagnostics::recorder::heartbeat writes DiagEvent::heartbeat every egui \
                 frame; proven by test_heartbeat.rs.",
            ),
            TierWiring::wired(
                DiagTier::Palmistry,
                "MT-091: Palmistry reads the heartbeat from the MT-081 ring under a bounded seqlock \
                 read to detect a UI freeze.",
            ),
        ],
    )
}

#[test]
fn three_tier_evidence_emit_to_external_root_and_parse_back() {
    // AC-015-1 / AC-015-4 / PT-015-A.
    let out_dir = external_artifact_dir("MT-095");
    let record = heartbeat_example_record();

    // The real example record must be well-formed evidence.
    record
        .validate()
        .expect("AC-015-4: the grounded heartbeat example must pass validate()");

    // Atomic emit to the EXTERNAL root.
    let written = record
        .emit(&out_dir)
        .expect("AC-015-1: emit the evidence record to the external artifact root");
    assert!(
        written.ends_with(EVIDENCE_FILE_NAME),
        "emitted file should be the canonical {EVIDENCE_FILE_NAME}, got {}",
        written.display()
    );
    assert!(written.exists(), "emitted evidence file must exist on disk");
    assert!(
        written.starts_with(Path::new("../../../../../Handshake_Artifacts")),
        "evidence must be written under the external Handshake_Artifacts root, got {}",
        written.display()
    );

    // Parse it back into the typed struct and assert it round-trips exactly.
    let reloaded = ThreeTierDiagnosticWiringRecord::load(&written)
        .expect("AC-015-1: parse the emitted JSON back into the typed struct");
    assert_eq!(reloaded, record, "emitted record must round-trip byte-for-byte");
    reloaded
        .validate()
        .expect("the parsed-back record must still validate");

    // The on-disk JSON must carry the congruent contract spellings (AC-015-3 on the real artifact).
    let raw = std::fs::read_to_string(&written).expect("read emitted JSON");
    for tier_spelling in congruence::CONTRACT_TIERS {
        assert!(
            raw.contains(tier_spelling),
            "emitted JSON must contain tier spelling {tier_spelling}"
        );
    }
    assert!(raw.contains("WIRED"), "emitted JSON must contain a WIRED status");
    assert!(
        raw.contains("NOT_APPLICABLE"),
        "emitted JSON must contain the NOT_APPLICABLE status"
    );

    assert_no_local_artifact_dir();
}

#[test]
fn three_tier_evidence_serde_vocabulary_is_congruent_with_contract_array() {
    // AC-015-3 / PT-015-C: the machine vocabulary must equal the hbr_int_009_tier_obligations array.
    for (tier, spelling) in congruence::tier_pairs() {
        let json = serde_json::to_string(&tier).expect("serialize tier");
        assert_eq!(json, format!("\"{spelling}\""));
    }
    for (status, spelling) in congruence::status_pairs() {
        let json = serde_json::to_string(&status).expect("serialize status");
        assert_eq!(json, format!("\"{spelling}\""));
    }
    // Belt-and-braces: the published constant arrays are exactly the contract vocabulary.
    assert_eq!(
        congruence::CONTRACT_TIERS,
        ["FLIGHT_RECORDER", "INTERNAL_DIAGNOSTICS", "PALMISTRY"]
    );
    assert_eq!(
        congruence::CONTRACT_STATUSES,
        ["WIRED", "NOT_APPLICABLE", "DEFERRED"]
    );

    assert_no_local_artifact_dir();
}
