use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn idempotency_conflict_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::IdempotencyConflict);
}
