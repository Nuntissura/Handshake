use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn postgres_loss_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::PostgresLoss);
}
