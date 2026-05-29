use handshake_core::session_checkpoint::CrashRecoveryScenario;

#[test]
fn event_seq_gap_matches_golden_evidence() {
    crate::assert_scenario_matches_golden(CrashRecoveryScenario::EventSeqGap);
}
