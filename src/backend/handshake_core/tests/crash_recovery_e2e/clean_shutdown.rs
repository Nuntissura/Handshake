use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn clean_shutdown_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::CleanShutdown);
}
