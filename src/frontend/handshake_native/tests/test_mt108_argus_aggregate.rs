//! Dedicated post-run closure gate for the MT-108 seven-surface Argus proof.
//!
//! The proof runner executes this ignored test only after all seven surface binaries have exited.
//! Keeping it separate means six successful binaries can never silently close the aggregate.

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

#[path = "native_gui_support/argus_surface_proof.rs"]
mod argus_surface_proof;

#[test]
#[ignore = "post-run gate; invoke through tests/run_mt108_argus_proof.ps1"]
fn mt108_verify_argus_evidence_exact_seven() {
    argus_surface_proof::verify_argus_evidence_exact_seven()
        .expect("MT-108 run must correlate seven Argus rows, seven screenshot markers, seven successful surface processes, and a started verifier process");
}
