use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn operator_cancel_during_recovery_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::OperatorCancelDuringRecovery);
}
