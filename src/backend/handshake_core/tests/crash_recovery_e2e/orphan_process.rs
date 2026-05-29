use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn orphan_process_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::OrphanProcess);
}
