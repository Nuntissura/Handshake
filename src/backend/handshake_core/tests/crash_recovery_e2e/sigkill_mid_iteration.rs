use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn sigkill_mid_iteration_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::SigkillMidIteration);
}
