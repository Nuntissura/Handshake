//! Dedicated post-run closure gate for the MT-108 manifest-driven Argus proof.
//!
//! The proof runner executes this ignored test only after every manifest row has exited.
//! Keeping it separate means a partial matrix can never silently close the aggregate.

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

#[path = "native_gui_support/mt108_matrix_verifier.rs"]
mod mt108_matrix_verifier;

#[test]
fn mt108_screenshot_identity_survives_pid_reuse() {
    let first =
        screenshot_harness::screenshot_process_identity(Some("cargo-scenario-a-111"), 76348);
    let second =
        screenshot_harness::screenshot_process_identity(Some("cargo-scenario-b-222"), 76348);

    assert_ne!(first, second);
    assert_eq!(
        screenshot_harness::screenshot_process_identity(None, 76348),
        "pid76348",
        "standalone tests retain a deterministic process-local fallback"
    );
}

#[test]
#[ignore = "post-run gate; invoke through tests/run_mt108_argus_proof.ps1"]
fn mt108_verify_argus_evidence_manifest() {
    mt108_matrix_verifier::verify().expect(
        "MT-108 run must correlate every manifest row to committed source, bounded process identity, canonical inspect/action/receipt/reinspection trace, and material captured PNG",
    );
}
